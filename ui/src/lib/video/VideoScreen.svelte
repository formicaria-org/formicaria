<script lang="ts">
  import { library } from '../libraryChanged.svelte';
  import { untrack } from 'svelte';
  // **Videos** (`decisions.md` 2026-11-17, *media from any site*; the owner: *"organizable in
  // collections? Just like songs"*): every video note (`log: video`), its collections (notes tagged
  // `collection` linking their videos, like playlists), one search, and a player on top. The file is
  // the vault's own: played from it, never copied; *Open with…* hands it to another player.
  import {
    assetUrl,
    createNotes,
    ingestFile,
    openWith,
    videoCollections,
    videoLibrary,
  } from '../ipc';
  import type { Playlist } from '../ipc';
  import type { ObjectMeta } from '../types';

  let {
    signal,
    addSignal = 0,
    onopen,
  }: {
    /** Changes whenever something was written: the library is asked for again. */
    signal?: unknown;
    /** Changes when the group's add control is pressed: *Add a video*. */
    addSignal?: number;
    onopen?: (id: string) => void;
  } = $props();

  let videos = $state<ObjectMeta[]>([]);
  let collections = $state<Playlist[]>([]);
  let q = $state('');
  let within = $state('');
  let playing = $state<ObjectMeta | null>(null);
  let problem = $state('');
  let picker = $state<HTMLInputElement>();

  $effect(() => {
    void signal;
    void library.n;
    void videoLibrary()
      .then((v) => (videos = v))
      .catch((e) => (problem = String(e)));
    void videoCollections()
      .then((c) => (collections = c))
      .catch(() => (collections = []));
  });
  let seenAdd = untrack(() => addSignal);
  $effect(() => {
    if (addSignal !== seenAdd) {
      seenAdd = addSignal;
      picker?.click();
    }
  });

  const text = (v: ObjectMeta, k: string) =>
    typeof v.props[k] === 'string' ? (v.props[k] as string) : '';
  const shown = $derived.by(() => {
    const words = q.toLowerCase().split(/\s+/).filter(Boolean);
    const inList = collections.find((c) => c.id === within);
    const pool = inList
      ? inList.songs
          .map((id) => videos.find((v) => v.id === id))
          .filter((v): v is ObjectMeta => !!v)
      : videos;
    return pool.filter((v) => {
      const hay = `${v.title ?? ''} ${text(v, 'channel')}`.toLowerCase();
      return words.every((w) => hay.includes(w));
    });
  });
  const length = (v: ObjectMeta) => {
    const s = typeof v.props.duration_s === 'number' ? v.props.duration_s : null;
    return s == null ? '' : `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, '0')}`;
  };
  const file = (v: ObjectMeta) => text(v, 'video') || v.assets[0] || '';

  /** *Add a video*: a file of this device, uploaded like any attachment, made a video note. */
  async function added(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = '';
    if (!f) return;
    problem = '';
    try {
      const asset = await ingestFile(f);
      const ref = asset.assets[0];
      const title = f.name.replace(/\.[a-z0-9]{2,5}$/i, '');
      await createNotes([
        {
          title,
          body: `![${title.replace(/[[\]\\]/g, '\\$&')}](asset:${ref.replace(/^sha256:/, 'sha256-')})\n`,
          tags: [],
          start: '',
          props: { log: 'video', video: ref },
        },
      ]);
      videos = await videoLibrary();
    } catch (err) {
      problem = `The video could not be added: ${String(err)}`;
    }
  }
</script>

<div class="videos">
  {#if playing}
    <div class="player">
      <!-- svelte-ignore a11y_media_has_caption -->
      <video src={assetUrl(file(playing))} controls autoplay aria-label={`playing ${playing.title}`}
      ></video>
      <div class="now">
        <strong>{playing.title}</strong>
        <span class="muted">{text(playing, 'channel')}</span>
        <button type="button" class="binding" onclick={() => onopen?.(playing!.id)}>Its note</button
        >
        <button
          type="button"
          class="binding"
          onclick={() => void openWith(file(playing!), 'video/*')}>Open with…</button
        >
      </div>
    </div>
  {/if}

  <div class="bar">
    <input
      type="search"
      placeholder="Search your videos"
      aria-label="search videos"
      bind:value={q}
    />
    <select aria-label="collection" bind:value={within}>
      <option value="">All videos</option>
      {#each collections as c (c.id)}
        <option value={c.id}>{c.title}</option>
      {/each}
    </select>
    <input
      bind:this={picker}
      type="file"
      accept="video/*"
      hidden
      aria-label="a video file"
      onchange={(e) => void added(e)}
    />
  </div>

  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}

  {#if !videos.length}
    <p class="muted empty">
      No videos yet. On a web page, <strong>Save media</strong> keeps a video here; or add a file with
      the group's ＋.
    </p>
  {:else}
    <ul class="list" aria-label="videos">
      {#each shown as v (v.id)}
        <li class:on={playing?.id === v.id}>
          <button type="button" class="row" onclick={() => (playing = v)}>
            <span class="title">{v.title}</span>
            <span class="muted">{[text(v, 'channel'), length(v)].filter(Boolean).join(' · ')}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .videos {
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 0.5rem);
    height: 100%;
    overflow: auto;
    padding: var(--space-2, 0.5rem);
  }
  .player video {
    width: 100%;
    max-height: 60dvh;
    background: #000;
    border-radius: 6px;
  }
  .now {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: baseline;
  }
  .bar {
    display: flex;
    gap: 0.5rem;
    position: sticky;
    top: 0;
    background: var(--bg, Canvas);
    padding: 0.25rem 0;
  }
  .bar input[type='search'] {
    flex: 1;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    padding: 0.5rem 0.4rem;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border, #8883);
    text-align: left;
    cursor: pointer;
  }
  li.on .row {
    background: var(--panel, #8881);
  }
  .title {
    font-weight: 600;
  }
  .muted {
    opacity: 0.7;
  }
  .problem {
    color: var(--danger, #c33);
  }
</style>
