<script lang="ts">
  // **Save media** (`decisions.md` 2026-11-17; the owner: *"extensions to get multimedia contents from
  // ANY website"*, *"extract audio of decent quality (MP3) from any video"*, *"we do not keep multiple
  // copies of heavy stuff"*, *"organizable in collections, just like songs"*). What the page holds;
  // for the one chosen, the video, its sound as the site serves it, or its sound as MP3; into Music
  // (a playlist), Videos (a collection) or a note. Only the chosen file is fetched, and only the final
  // file is kept. The same sheet on the phone and the computer.
  import { onDestroy, onMount } from 'svelte';
  import * as ipc from './ipc';
  import * as run from './mediaRun';
  import { touched as touchedLibrary } from './libraryChanged.svelte';
  import type { Candidate } from './web/media';

  let {
    page,
    candidates,
    note = null,
    onopen,
  }: {
    page: string;
    candidates: Candidate[];
    /** The note looked at last, offered as a place to put it. */
    note?: { id: string; title: string } | null;
    /** Open the note a saved file became. */
    onopen?: (id: string) => void;
  } = $props();

  let chosen = $state<Candidate | null>(null);
  // **Several tracks at once** (2026-11-20, a Bandcamp album): a page's own tracks (the ones it
  // names, `meta`) are ticked, two or more of them; everything else stays one choice.
  const tracks = $derived(candidates.filter((c) => c.meta));
  const several = $derived(tracks.length >= 2);
  let picked = $state<string[]>([]);
  let waiting = $state(0);
  let probe = $state<ipc.MediaProbe | null>(null);
  let probing = $state(false);
  let problem = $state('');
  let what = $state<'video' | 'audio' | 'mp3'>('audio');
  let quality = $state(1080);
  let mp3 = $state('v2');
  let into = $state<'music' | 'videos' | 'note'>('music');
  let list = $state('');
  let newList = $state('');
  let lists = $state<{ id: string; title: string }[]>([]);
  let tools = $state<ipc.MediaTools | null>(null);
  let jobs = $state<ipc.MediaJob[]>([]);
  let mine = $state<number[]>([]);
  let timer: ReturnType<typeof setInterval> | undefined;

  const host = $derived.by(() => {
    try {
      return new URL(page).hostname.replace(/^www\./, '');
    } catch {
      return page;
    }
  });
  const heights = $derived((probe?.heights ?? []).filter((h) => h >= 240 && h <= 2160));
  const shown = $derived(jobs.filter((j) => mine.includes(j.id)));
  const active = $derived(shown.some((j) => !['done', 'failed', 'cancelled'].includes(j.state)));

  /** **Not into a song or a playlist** (2026-11-21: an album saved *into the note* went into a
   *  song's note, and Music played only its first track): those notes are Music's own, and a track
   *  goes to Music, or into a playlist from the list below. */
  let noteIsMusic = $state(false);
  $effect(() => {
    const id = note?.id;
    noteIsMusic = false;
    if (!id) return;
    void ipc
      .getNote(id)
      .then((n) => {
        const props = (n as { props?: Record<string, unknown> } | null)?.props ?? {};
        const tags = (n as { tags?: string[] } | null)?.tags ?? [];
        noteIsMusic =
          props.log === 'music' ||
          props.log === 'video' ||
          tags.some((t) => /^(playlist|collection)$/i.test(t));
        if (noteIsMusic && into === 'note') into = what === 'video' ? 'videos' : 'music';
      })
      .catch(() => {});
  });

  onMount(() => {
    void run
      .tools()
      .then((t) => (tools = t))
      .catch(() => {});
    if (candidates.length === 1) void choose(candidates[0]);
  });
  onDestroy(() => clearInterval(timer));

  /** What the ticked tracks are, from the page: nothing to look up. */
  function trackProbe(): ipc.MediaProbe | null {
    const on = tracks.filter((t) => picked.includes(t.url));
    if (!on.length) return null;
    const m = on[0].meta ?? {};
    return {
      title: on.length === 1 ? (m.title ?? on[0].label) : `${on.length} tracks`,
      uploader: m.artist || null,
      duration: null,
      site: m.album || null,
      live: false,
      video: false,
      audio: true,
      heights: [],
    };
  }

  function tick(url: string, on: boolean) {
    picked = on ? [...picked.filter((u) => u !== url), url] : picked.filter((u) => u !== url);
    chosen = null;
    problem = '';
    probe = trackProbe();
    if (what === 'video') what = 'mp3';
  }

  function tickAll(on: boolean) {
    picked = on ? tracks.map((t) => t.url) : [];
    chosen = null;
    problem = '';
    probe = trackProbe();
    if (what === 'video') what = 'mp3';
  }

  async function choose(c: Candidate) {
    picked = [];
    chosen = c;
    probe = null;
    problem = '';
    probing = true;
    try {
      probe = await run.probe(c.url);
      // A file whose address says nothing (a Bandcamp stream): the title the page gave it.
      if (c.meta?.title) probe = { ...probe, title: c.meta.title };
      if (!probe.video && what === 'video') what = 'audio';
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    } finally {
      probing = false;
    }
  }

  // Where it goes follows what is kept, until the person says otherwise.
  $effect(() => {
    if (into !== 'note') into = what === 'video' ? 'videos' : 'music';
  });
  $effect(() => {
    const want = into;
    list = '';
    if (want === 'note') return void (lists = []);
    void (want === 'music' ? ipc.musicPlaylists() : ipc.videoCollections())
      .then((l) => (lists = l.map((x) => ({ id: x.id, title: x.title }))))
      .catch(() => (lists = []));
  });

  async function poll() {
    try {
      const before = new Set(jobs.filter((j) => j.state === 'done').map((j) => j.id));
      jobs = await run.jobs();
      if (jobs.some((j) => j.state === 'done' && !before.has(j.id))) touchedLibrary();
    } catch {
      /* asked again in a second */
    }
    if (!active && timer) {
      clearInterval(timer);
      timer = undefined;
    }
  }

  /** Save every ticked track, one after another, into one playlist (made first when new). */
  async function saveTracks() {
    problem = '';
    const on = tracks.filter((t) => picked.includes(t.url));
    let target = list;
    try {
      if (into !== 'note' && list === '+') {
        const name = newList.trim() || on[0].meta?.album || '';
        target = '';
        if (name) {
          const [made] = await ipc.createNotes([
            {
              title: name,
              body: '',
              tags: [into === 'music' ? 'playlist' : 'collection'],
              props: {},
            },
          ]);
          target = made.id;
          list = made.id;
          lists = [...lists, { id: made.id, title: name }];
          newList = '';
        }
      }
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
      return;
    }
    const asks: ipc.MediaAsk[] = on.map((t) => ({
      url: t.url,
      page,
      what,
      quality: 0,
      mp3,
      into,
      list: into === 'note' ? '' : target,
      note: into === 'note' ? (note?.id ?? '') : '',
      ...(t.meta ? { meta: t.meta } : {}),
    }));
    waiting = asks.length;
    void run
      .saveEach(asks, (id) => {
        mine = [...mine, id];
        waiting = run.waitingCount();
        void poll();
        if (!timer) timer = setInterval(() => void poll(), 1000);
      })
      .finally(() => (waiting = 0));
  }

  async function save() {
    if (picked.length) return saveTracks();
    if (!chosen) return;
    problem = '';
    const target = list === '+' ? (newList.trim() ? `new:${newList.trim()}` : '') : list;
    try {
      const { id } = await run.save({
        url: chosen.url,
        page,
        what,
        quality: what === 'video' ? quality : 0,
        mp3,
        into,
        list: into === 'note' ? '' : target,
        note: into === 'note' ? (note?.id ?? '') : '',
        ...(chosen.meta ? { meta: chosen.meta } : {}),
      });
      mine = [...mine, id];
      await poll();
      if (!timer) timer = setInterval(() => void poll(), 1000);
    } catch (e) {
      problem = e instanceof Error ? e.message : String(e);
    }
  }

  const STATE: Record<ipc.MediaJob['state'], string> = {
    tools: 'Getting the media tools…',
    downloading: 'Downloading…',
    converting: 'Converting…',
    filing: 'Putting it in your library…',
    done: 'Saved',
    failed: 'Not saved',
    cancelled: 'Cancelled',
  };
  const where = (j: ipc.MediaJob) =>
    j.what === 'video' && into !== 'note' ? 'Videos' : into === 'note' ? 'your note' : 'Music';
  const minutes = (s: number | null) =>
    s == null ? '' : `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, '0')}`;
</script>

<section class="save-media" aria-label="save media">
  <p class="muted">On {host}</p>

  {#if tools && !tools.supported}
    <p class="problem">Save media does not run on this kind of computer yet.</p>
  {:else if tools && (!tools.ytdlp || !tools.ffmpeg)}
    <p class="muted note">
      The first save fetches the media tools (yt-dlp {tools.ytdlp_version} and ffmpeg
      {tools.ffmpeg_version}, about 70–150 MB once), checked against this release of formicaria.
    </p>
  {/if}

  <fieldset>
    <legend>What to keep from</legend>
    <ul class="candidates">
      {#if several}
        <li>
          <label>
            <input
              type="checkbox"
              checked={picked.length === tracks.length}
              onchange={(e) => tickAll(e.currentTarget.checked)}
            />
            <span>All tracks ({tracks.length})</span>
          </label>
        </li>
      {/if}
      {#each candidates as c (c.url)}
        <li>
          <label>
            {#if several && c.meta}
              <input
                type="checkbox"
                name="track"
                checked={picked.includes(c.url)}
                onchange={(e) => tick(c.url, e.currentTarget.checked)}
              />
            {:else}
              <input
                type="radio"
                name="candidate"
                checked={chosen?.url === c.url}
                onchange={() => void choose(c)}
              />
            {/if}
            <span>{c.label}</span>
          </label>
        </li>
      {/each}
    </ul>
  </fieldset>

  {#if probing}
    <p class="muted">Looking at it…</p>
  {:else if probe}
    <div class="probe">
      <strong>{probe.title ?? 'Untitled'}</strong>
      <span class="muted"
        >{[probe.uploader, minutes(probe.duration), probe.site].filter(Boolean).join(' · ')}</span
      >
      {#if probe.live}<span class="problem">A live stream: it cannot be kept while it runs.</span
        >{/if}
    </div>

    <fieldset>
      <legend>Keep</legend>
      <label
        ><input type="radio" name="what" value="video" bind:group={what} disabled={!probe.video} />
        The video</label
      >
      {#if what === 'video' && heights.length}
        <select aria-label="video quality" bind:value={quality}>
          {#each [...heights].reverse() as h (h)}
            <option value={h}>{h}p</option>
          {/each}
        </select>
      {/if}
      <label
        ><input type="radio" name="what" value="audio" bind:group={what} /> The sound, as the site
        serves it <span class="muted">(best quality, no conversion)</span></label
      >
      <label
        ><input type="radio" name="what" value="mp3" bind:group={what} /> The sound as MP3</label
      >
      {#if what === 'mp3'}
        <select aria-label="MP3 quality" bind:value={mp3}>
          <option value="v2">Good (about 190 kbps, variable)</option>
          <option value="192">192 kbps</option>
          <option value="256">256 kbps</option>
          <option value="320">320 kbps</option>
        </select>
      {/if}
    </fieldset>

    <fieldset>
      <legend>Into</legend>
      <label
        ><input
          type="radio"
          name="into"
          value="music"
          bind:group={into}
          disabled={what === 'video'}
        /> Music</label
      >
      <label
        ><input
          type="radio"
          name="into"
          value="videos"
          bind:group={into}
          disabled={what !== 'video'}
        /> Videos</label
      >
      {#if note && !noteIsMusic}
        <label
          ><input type="radio" name="into" value="note" bind:group={into} /> The note “{note.title ||
            'Untitled'}”</label
        >
      {/if}
      {#if into !== 'note'}
        <select aria-label={into === 'music' ? 'playlist' : 'collection'} bind:value={list}>
          <option value="">{into === 'music' ? 'No playlist' : 'No collection'}</option>
          {#each lists as l (l.id)}
            <option value={l.id}>{l.title}</option>
          {/each}
          <option value="+">{into === 'music' ? 'A new playlist…' : 'A new collection…'}</option>
        </select>
        {#if list === '+'}
          <input
            aria-label="the new list's name"
            placeholder={picked.length
              ? (tracks.find((t) => picked.includes(t.url))?.meta?.album ?? '') ||
                (into === 'music' ? 'Playlist name' : 'Collection name')
              : into === 'music'
                ? 'Playlist name'
                : 'Collection name'}
            bind:value={newList}
          />
        {/if}
      {/if}
    </fieldset>

    <button
      type="button"
      class="primary"
      onclick={() => void save()}
      disabled={probe.live || waiting > 0}
      >{picked.length > 1 ? `Save ${picked.length} tracks` : 'Save'}</button
    >
    {#if waiting > 0}
      <p class="muted" aria-live="polite">
        {waiting} more waiting: each starts when the one before has finished.
        <button type="button" class="binding" onclick={() => run.stopEach()}>Start no more</button>
      </p>
    {/if}
  {/if}

  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}

  {#if shown.length}
    <ul class="jobs" aria-label="saving">
      {#each shown as j (j.id)}
        <li>
          <span class="title"
            >{j.title ||
              candidates.find((c) => c.url === j.url)?.meta?.title ||
              chosen?.label ||
              j.url}</span
          >
          <span class="state"
            >{j.state === 'done' ? `Saved in ${where(j)}` : STATE[j.state]}{j.error
              ? `: ${j.error}`
              : ''}</span
          >
          {#if !['done', 'failed', 'cancelled'].includes(j.state)}
            <progress value={j.progress ?? undefined} max="1"></progress>
            <button type="button" class="binding" onclick={() => void run.cancel(j.id)}
              >Cancel</button
            >
          {:else if j.state === 'done' && j.note && onopen}
            <button type="button" class="binding" onclick={() => onopen?.(j.note!)}>Open</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <p class="muted small">
    Only what you choose is downloaded, and only the final file is kept. Protected media (DRM)
    cannot be kept; what you may keep from a site is yours to judge.
  </p>
</section>

<style>
  .save-media {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  fieldset {
    border: 1px solid var(--border, #8884);
    border-radius: 6px;
    padding: 0.5rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .candidates {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .candidates li {
    padding: 0.2rem 0;
    overflow-wrap: anywhere;
  }
  .probe {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .muted {
    opacity: 0.7;
  }
  .small {
    font-size: 0.85em;
  }
  .problem {
    color: var(--danger, #c33);
  }
  .jobs {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .jobs li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.2rem 0.6rem;
    align-items: center;
  }
  .jobs progress {
    width: 100%;
  }
  .jobs .title {
    font-weight: 600;
  }
  .primary {
    align-self: flex-start;
  }
</style>
