<script lang="ts">
  // **Songs** (`decisions.md#ui`, 2026-11-13, *a sixth group, Music*): the search first, then every
  // song. The library is asked for once; typing filters it here, so a keystroke never waits on the
  // server. Pressing a song opens its note, which plays it where it is opened.
  import Icon from '../Icon.svelte';
  import { musicLibrary } from '../ipc';
  import type { ObjectMeta } from '../types';

  interface Props {
    /** Changes whenever something was written: the library is asked for again. */
    signal?: unknown;
    onopen?: (id: string) => void;
    onadd: () => void;
  }
  let { signal, onopen, onadd }: Props = $props();

  let songs = $state<ObjectMeta[] | null>(null);
  let failed = $state('');
  let q = $state('');

  let asked = 0;
  $effect(() => {
    void signal;
    const mine = ++asked;
    musicLibrary()
      .then((s) => {
        if (mine !== asked) return;
        songs = s;
        failed = '';
      })
      .catch((e) => mine === asked && (failed = String(e)));
  });

  const text = (s: ObjectMeta, key: string) => {
    const v = s.props[key];
    return typeof v === 'string' ? v : '';
  };
  /** Lower case and without accents, so "beyonce" finds "Beyoncé". */
  const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

  /// What a song is searched by, folded once per library rather than once per keystroke.
  const rows = $derived(
    (songs ?? []).map((s) => ({
      song: s,
      title: s.title ?? '',
      by: [text(s, 'artist'), text(s, 'album')].filter(Boolean).join(' · '),
      hay: fold(
        [
          s.title ?? '',
          text(s, 'artist'),
          text(s, 'album'),
          text(s, 'album_artist'),
          text(s, 'genre'),
        ].join('\n'),
      ),
      length: typeof s.props.duration_s === 'number' ? s.props.duration_s : null,
    })),
  );
  const words = $derived(fold(q).split(/\s+/).filter(Boolean));
  const shown = $derived(
    words.length ? rows.filter((r) => words.every((w) => r.hay.includes(w))) : rows,
  );

  const clock = (secs: number) =>
    `${Math.floor(secs / 60)}:${String(Math.round(secs % 60)).padStart(2, '0')}`;
</script>

<div class="songs">
  {#if songs && songs.length}
    <label class="find">
      <Icon name="search" size={16} />
      <input
        type="search"
        placeholder="Search your music"
        aria-label="search songs"
        autocomplete="off"
        bind:value={q}
      />
    </label>
  {/if}

  {#if failed}
    <p class="failed" role="alert">{failed}</p>
  {:else if !songs}
    <p class="quiet">Loading…</p>
  {:else if !songs.length}
    <div class="empty">
      <p>No songs yet.</p>
      <button type="button" class="go" onclick={onadd}>Add a song</button>
    </div>
  {:else if !shown.length}
    <p class="quiet">Nothing matches “{q.trim()}”.</p>
  {:else}
    <ul class="list" aria-label="songs">
      {#each shown as r (r.song.id)}
        <li>
          <button type="button" class="row" onclick={() => onopen?.(r.song.id)}>
            <span class="name">
              <span class="title">{r.title}</span>
              {#if r.by}<span class="by">{r.by}</span>{/if}
            </span>
            {#if r.length !== null}<span class="length">{clock(r.length)}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .songs {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-6);
    min-height: 0;
    overflow-y: auto;
  }
  .find {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2.75rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text-muted);
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    font: inherit;
    color: var(--text);
    outline: none;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 3rem;
    padding: var(--space-2) var(--space-1);
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title,
  .by {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .by,
  .length,
  .quiet {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .length {
    font-variant-numeric: tabular-nums;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .empty p,
  .quiet,
  .failed {
    margin: 0;
  }
  .failed {
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }
  .go {
    min-height: 2.75rem;
    padding: 0 var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--accent-contrast);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
</style>
