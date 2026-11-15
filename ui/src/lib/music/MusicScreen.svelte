<script lang="ts">
  // **The Music view** (`decisions.md#ui`, 2026-11-13, *a sixth group, Music*; owner, 2026-11-14:
  // *"where we can browse for the songs and playlists and play them, also seeing it running …
  // exactly like an mp3"*): the deck on top and the browser under it:
  // **Songs · Albums · Artists · Playlists**, one search over all four.
  //
  // **One column, the player on top** (owner, 2026-11-15: *"why the play is on the left and the
  // rest on the right? … a vertical layout"*). Where the deck has room it is a strip, pinned with
  // the tabs and the search while the songs scroll under it; on a phone it is stacked, scrolls
  // away, and the tabs and search stay pinned while the slim bar takes over the keys.
  //
  // The library and the playlists are asked for once each; searching, grouping and sorting happen
  // here, so a keystroke never waits on the server (the `perf.rs` rule). **Pressing a song plays
  // it**, and the list it was pressed in becomes the queue. Its ⋯ holds the rest: play next, add
  // to the queue, add to a playlist, open its note — and inside a playlist, move and remove.
  //
  // **Only the song rows in view are drawn**, so a library of thousands scrolls as lightly as one
  // of ten. Every row is the same height, which is what makes that exact.
  import Icon from '../Icon.svelte';
  import { assetUrl, musicLibrary, musicPlaylists, type Playlist } from '../ipc';
  import * as nav from '../nav.svelte';
  import type { ObjectMeta } from '../types';
  import Deck from './Deck.svelte';
  import * as player from './player.svelte';
  import * as pl from './playlist';

  interface Props {
    /** Changes whenever something was written: the library is asked for again. */
    signal?: unknown;
    onopen?: (id: string) => void;
    onadd: () => void;
  }
  let { signal, onopen, onadd }: Props = $props();

  type Tab = 'songs' | 'albums' | 'artists' | 'playlists';
  const TABS: [Tab, string][] = [
    ['songs', 'Songs'],
    ['albums', 'Albums'],
    ['artists', 'Artists'],
    ['playlists', 'Playlists'],
  ];

  let songs = $state<ObjectMeta[] | null>(null);
  let lists = $state<Playlist[]>([]);
  let failed = $state('');
  let q = $state('');
  let tab = $state<Tab>('songs');
  /** An album, artist or playlist opened from its tab. */
  let open = $state<{ kind: 'album' | 'artist' | 'playlist'; key: string } | null>(null);

  let asked = 0;
  function load() {
    const n = ++asked;
    Promise.all([musicLibrary(), musicPlaylists()])
      .then(([s, p]) => {
        if (n !== asked) return;
        songs = s;
        lists = p;
        failed = '';
      })
      .catch((e) => n === asked && (failed = String(e)));
  }
  $effect(() => {
    void signal;
    load();
  });
  /** After a playlist edit made here: ask again. */
  const reload = () => load();

  // ── the songs, folded once for searching ────────────────────────────────────────────────────
  const text = (s: ObjectMeta, key: string) => {
    const v = s.props[key];
    return typeof v === 'string' ? v : '';
  };
  const num = (s: ObjectMeta, key: string) => {
    const v = s.props[key];
    return typeof v === 'number' ? v : null;
  };
  /** Lower case and without accents, so "beyonce" finds "Beyoncé". */
  const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

  interface Row {
    song: ObjectMeta;
    title: string;
    artist: string;
    album: string;
    by: string;
    hay: string;
    length: number | null;
  }
  const rows = $derived<Row[]>(
    (songs ?? []).map((s) => {
      const artist = text(s, 'artist');
      const album = text(s, 'album');
      return {
        song: s,
        title: s.title ?? '',
        artist,
        album,
        by: [artist, album].filter(Boolean).join(' · '),
        hay: fold(
          [s.title ?? '', artist, album, text(s, 'album_artist'), text(s, 'genre')].join('\n'),
        ),
        length: num(s, 'duration_s'),
      };
    }),
  );
  const byId = $derived(new Map(rows.map((r) => [r.song.id.toUpperCase(), r])));
  const words = $derived(fold(q).split(/\s+/).filter(Boolean));
  const shown = $derived(
    words.length ? rows.filter((r) => words.every((w) => r.hay.includes(w))) : rows,
  );

  /** Disc, then track, then title: an album in the order it was made. */
  const inOrder = (a: Row, b: Row) =>
    (num(a.song, 'disc') ?? 1) - (num(b.song, 'disc') ?? 1) ||
    (num(a.song, 'track') ?? 999) - (num(b.song, 'track') ?? 999) ||
    a.title.localeCompare(b.title);

  interface Album {
    key: string;
    name: string;
    artist: string;
    year: number | null;
    cover: string | null;
    songs: Row[];
  }
  const albums = $derived.by<Album[]>(() => {
    const m = new Map<string, Album>();
    for (const r of shown) {
      const name = r.album || 'Unknown album';
      const artist = text(r.song, 'album_artist') || r.artist || 'Unknown artist';
      const key = `${fold(artist)}\u0000${fold(name)}`;
      let a = m.get(key);
      if (!a) m.set(key, (a = { key, name, artist, year: null, cover: null, songs: [] }));
      a.songs.push(r);
      a.year ??= num(r.song, 'year');
      a.cover ??= text(r.song, 'cover') || null;
    }
    for (const a of m.values()) a.songs.sort(inOrder);
    return [...m.values()].sort((x, y) => x.name.localeCompare(y.name));
  });

  interface Artist {
    key: string;
    name: string;
    albums: Album[];
    count: number;
    /** The first of their albums' covers, to know them by at a glance. */
    cover: string | null;
  }
  const artists = $derived.by<Artist[]>(() => {
    const m = new Map<string, Artist>();
    for (const a of albums) {
      // An artist is who made the songs; a compilation's songs go to each of theirs.
      const names = new Set(a.songs.map((r) => r.artist || a.artist));
      for (const name of names) {
        const key = fold(name);
        let x = m.get(key);
        if (!x) m.set(key, (x = { key, name, albums: [], count: 0, cover: null }));
        x.cover ??= a.cover;
        const theirs = a.songs.filter((r) => (r.artist || a.artist) === name);
        x.albums.push({ ...a, songs: theirs });
        x.count += theirs.length;
      }
    }
    return [...m.values()].sort((x, y) => x.name.localeCompare(y.name));
  });

  const playlists = $derived(
    words.length ? lists.filter((p) => words.every((w) => fold(p.title).includes(w))) : lists,
  );
  /** A playlist's songs that are in the library, in order, each with its place in the list. */
  const playlistRows = (p: Playlist) =>
    p.songs.flatMap((id, i) => {
      const r = byId.get(id.toUpperCase());
      return r ? [{ row: r, at: i }] : [];
    });

  const openAlbum = $derived(
    open?.kind === 'album' ? (albums.find((a) => a.key === open!.key) ?? null) : null,
  );
  const openArtist = $derived(
    open?.kind === 'artist' ? (artists.find((a) => a.key === open!.key) ?? null) : null,
  );
  const openList = $derived(
    open?.kind === 'playlist' ? (lists.find((p) => p.id === open!.key) ?? null) : null,
  );
  $effect(() => {
    if (open) nav.openLayer('music-open', () => (open = null));
    else nav.closeLayer('music-open');
  });
  $effect(() => () => nav.closeLayer('music-open'));

  function choose(t: Tab) {
    tab = t;
    open = null;
  }

  // ── playing ──────────────────────────────────────────────────────────────────────────────────
  const tracks = (list: Row[]) =>
    list.map((r) => player.trackOf(r.song)).filter((t): t is player.Track => t !== null);
  /** Play `list` from the one at `i`: the list it was pressed in becomes the queue. */
  function playFrom(list: Row[], i: number) {
    const all = list.map((r) => player.trackOf(r.song));
    const from = all.slice(0, i).filter(Boolean).length;
    player.playList(tracks(list), from);
  }
  const shuffleAll = (list: Row[]) => player.shuffleList(tracks(list));
  const clock = (secs: number) =>
    `${Math.floor(secs / 60)}:${String(Math.round(secs % 60)).padStart(2, '0')}`;
  const total = (list: Row[]) => {
    const s = list.reduce((n, r) => n + (r.length ?? 0), 0);
    return s >= 3600
      ? `${Math.floor(s / 3600)} h ${Math.round((s % 3600) / 60)} min`
      : `${Math.round(s / 60)} min`;
  };
  const isOn = (r: Row) => player.now()?.id === r.song.id;

  // ── the ⋯ sheet: everything else that can be done with songs ───────────────────────────────
  interface Menu {
    title: string;
    rows: Row[];
    /** Set when the songs were picked inside a playlist: where, so it can move or drop them. */
    inList?: { id: string; at: number; last: number };
    picking?: boolean;
  }
  let menu = $state<Menu | null>(null);
  let newName = $state('');
  let busy = $state(false);
  let note = $state('');
  $effect(() => {
    if (menu) nav.openLayer('music-menu', () => (menu = null));
    else nav.closeLayer('music-menu');
  });
  $effect(() => () => nav.closeLayer('music-menu'));
  function more(title: string, list: Row[], inList?: Menu['inList']) {
    newName = '';
    menu = { title, rows: list, inList };
  }
  function say(s: string) {
    note = s;
    setTimeout(() => note === s && (note = ''), 2500);
  }
  async function act(fn: () => Promise<unknown> | unknown, done?: string) {
    if (busy) return;
    busy = true;
    try {
      await fn();
      menu = null;
      if (done) say(done);
    } catch (e) {
      say(String(e instanceof Error ? e.message : e));
    } finally {
      busy = false;
    }
  }
  const entriesOf = (list: Row[]): pl.Entry[] =>
    list.map((r) => ({ id: r.song.id, title: r.title }));
  const addTo = (p: Playlist, list: Row[]) =>
    act(async () => {
      await pl.edit(p.id, (e) => [...e, ...entriesOf(list)]);
      reload();
    }, `added to “${p.title}”`);
  const makeWith = (list: Row[]) =>
    act(
      async () => {
        await pl.create(newName, entriesOf(list));
        reload();
      },
      `made the playlist “${newName.trim() || 'New playlist'}”`,
    );
  const moveIn = (id: string, from: number, to: number) =>
    act(async () => {
      await pl.edit(id, (e) => pl.move(e, from, to));
      reload();
    });
  const dropFrom = (id: string, at: number) =>
    act(async () => {
      await pl.edit(id, (e) => e.filter((_, i) => i !== at));
      reload();
    }, 'taken out of the playlist');

  // ── new playlists from the Playlists tab ─────────────────────────────────────────────────────
  let naming = $state<'empty' | 'queue' | null>(null);
  async function makeNew() {
    const fromQueue = naming === 'queue';
    const list: pl.Entry[] = fromQueue
      ? player.queue().map((t) => ({ id: t.id, title: t.title }))
      : [];
    await act(
      async () => {
        const id = await pl.create(newName, list);
        naming = null;
        newName = '';
        reload();
        open = { kind: 'playlist', key: id };
      },
      fromQueue ? 'the queue is saved as a playlist' : undefined,
    );
  }

  // ── drawing only the song rows in view ──────────────────────────────────────────────────────
  /** A row's height in pixels: `.row`'s, which is fixed so the arithmetic below is exact. */
  const ROW = 56;
  const MARGIN = 10;
  let box = $state<HTMLElement | null>(null);
  let listEl = $state<HTMLElement | null>(null);
  let scrolled = $state(0);
  let tall = $state(0);
  const first = $derived(Math.max(0, Math.floor(scrolled / ROW) - MARGIN));
  /// With no height to go by (before the first layout, and in a test), a generous first screen.
  const count = $derived(tall ? Math.ceil(tall / ROW) + 2 * MARGIN : 200);
  const drawn = $derived(shown.slice(first, first + count));
  function onscroll() {
    if (box) scrolled = Math.max(0, box.scrollTop - (listEl?.offsetTop ?? 0));
  }
  // The height in view, again whenever it changes. Measured by hand rather than with
  // `bind:clientHeight`, which needs a `ResizeObserver` an older WebView may not have.
  $effect(() => {
    const el = box;
    if (!el) return;
    tall = el.clientHeight;
    if (typeof ResizeObserver === 'undefined') return;
    const watch = new ResizeObserver(() => (tall = el.clientHeight));
    watch.observe(el);
    return () => watch.disconnect();
  });
  // A new search starts at the top of what it found.
  $effect(() => {
    void words;
    scrolled = 0;
    if (box && listEl && box.scrollTop > listEl.offsetTop) box.scrollTop = 0;
  });

  // ── the keys of a player, while the Music view is on screen ─────────────────────────────────
  // Only when nothing that takes typing has the focus, and never with a modifier held: Space, the
  // arrows and a letter mean what they always mean everywhere else.
  function onkey(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented) return;
    const el = e.target as HTMLElement | null;
    if (el && el !== document.body && !el.classList?.contains('music')) return;
    if (document.querySelector('[role="dialog"]')) return;
    const keys: Record<string, () => void> = {
      ' ': player.toggle,
      ArrowLeft: () => player.skip(-5),
      ArrowRight: () => player.skip(5),
      n: player.next,
      p: player.previous,
      m: player.toggleMute,
      s: player.toggleShuffle,
      r: player.cycleRepeat,
      '+': () => player.setVolume(player.volume() + 0.05),
      '-': () => player.setVolume(player.volume() - 0.05),
    };
    const f = keys[e.key];
    if (!f) return;
    e.preventDefault();
    f();
  }
</script>

<svelte:window onkeydown={onkey} />

{#snippet songRow(r: Row, list: Row[], i: number, inList?: Menu['inList'], numbered?: boolean)}
  <li class:on={isOn(r)}>
    <button
      type="button"
      class="row"
      onclick={() => playFrom(list, i)}
      aria-label={`play ${r.title}`}
      aria-current={isOn(r) ? 'true' : undefined}
    >
      {#if numbered}<span class="n">{isOn(r) && player.playing() ? '▶' : i + 1}</span>
      {:else}{@render cover(text(r.song, 'cover') || null, r.title)}{/if}
      <span class="name">
        <span class="title">{r.title}</span>
        {#if r.by}<span class="by">{r.by}</span>{/if}
      </span>
      {#if r.length !== null}<span class="length">{clock(r.length)}</span>{/if}
    </button>
    <button
      type="button"
      class="more"
      onclick={() => more(r.title, [r], inList)}
      aria-label={`more for ${r.title}`}
      title="More"
    >
      <Icon name="more" size={16} />
    </button>
  </li>
{/snippet}

{#snippet bulk(title: string, list: Row[])}
  <div class="bulk">
    <button type="button" class="go" onclick={() => playFrom(list, 0)} disabled={!list.length}>
      <Icon name="play" size={14} /> Play
    </button>
    <button type="button" class="ghost" onclick={() => shuffleAll(list)} disabled={!list.length}>
      <Icon name="shuffle" size={14} /> Shuffle
    </button>
    <button
      type="button"
      class="ghost"
      onclick={() => more(title, list)}
      disabled={!list.length}
      aria-label={`more for ${title}`}
    >
      <Icon name="more" size={14} />
    </button>
  </div>
{/snippet}

{#snippet cover(src: string | null, name: string, big = false)}
  {#if src}
    <img class="cover" class:big src={assetUrl(src, 'thumb')} alt="" loading="lazy" />
  {:else}
    <span class="cover none" class:big aria-hidden="true"
      >{(name.trim()[0] ?? '♪').toUpperCase()}</span
    >
  {/if}
{/snippet}

{#snippet back(label: string)}
  <button type="button" class="back" onclick={() => (open = null)}>‹ {label}</button>
{/snippet}

<div class="music-box">
  <div class="music" bind:this={box} {onscroll}>
    <!-- The player and the ways in, on top: pinned together where the deck is a strip. -->
    <div class="top">
      <div class="deck-col">
        <Deck {onopen} />
      </div>
      <div class="head">
        <div class="tabs" role="tablist" aria-label="browse music">
          {#each TABS as [k, label] (k)}
            <button
              type="button"
              role="tab"
              aria-selected={tab === k}
              class:sel={tab === k}
              onclick={() => choose(k)}>{label}</button
            >
          {/each}
        </div>
        <label class="find">
          <Icon name="search" size={16} />
          <input
            type="search"
            placeholder="Search your music"
            aria-label="search music"
            autocomplete="off"
            bind:value={q}
          />
        </label>
      </div>
    </div>

    <div class="browse">
      {#if note}<p class="note" role="status">{note}</p>{/if}

      {#if failed}
        <p class="failed" role="alert">{failed}</p>
      {:else if !songs}
        <p class="quiet">Loading…</p>
      {:else if !songs.length && tab !== 'playlists'}
        <div class="empty">
          <p>No songs yet.</p>
          <button type="button" class="go" onclick={onadd}>Add a song</button>
        </div>
      {:else if tab === 'songs'}
        {#if !shown.length}
          <p class="quiet">Nothing matches “{q.trim()}”.</p>
        {:else}
          <div class="count">
            <span>{shown.length} {shown.length === 1 ? 'song' : 'songs'} · {total(shown)}</span>
            {@render bulk(words.length ? `songs matching “${q.trim()}”` : 'all songs', shown)}
          </div>
          <ul
            class="list"
            aria-label="songs"
            bind:this={listEl}
            style:padding-top="{first * ROW}px"
            style:padding-bottom="{Math.max(0, shown.length - first - count) * ROW}px"
          >
            {#each drawn as r, k (r.song.id)}
              {@render songRow(r, shown, first + k)}
            {/each}
          </ul>
        {/if}
      {:else if tab === 'albums'}
        {#if openAlbum}
          <div class="detail">
            {@render back('Albums')}
            <div class="hero">
              {@render cover(openAlbum.cover, openAlbum.name, true)}
              <div class="hero-text">
                <h2>{openAlbum.name}</h2>
                <p>
                  {openAlbum.artist}{#if openAlbum.year}
                    · {openAlbum.year}{/if}
                </p>
                <p class="by">
                  {openAlbum.songs.length}
                  {openAlbum.songs.length === 1 ? 'song' : 'songs'} · {total(openAlbum.songs)}
                </p>
                {@render bulk(openAlbum.name, openAlbum.songs)}
              </div>
            </div>
            <ul class="list short" aria-label={`songs of ${openAlbum.name}`}>
              {#each openAlbum.songs as r, i (r.song.id)}
                {@render songRow(r, openAlbum.songs, i, undefined, true)}
              {/each}
            </ul>
          </div>
        {:else if !albums.length}
          <p class="quiet">Nothing matches “{q.trim()}”.</p>
        {:else}
          <ul class="grid" aria-label="albums">
            {#each albums as a (a.key)}
              <li>
                <button
                  type="button"
                  class="tile"
                  onclick={() => (open = { kind: 'album', key: a.key })}
                  aria-label={`album ${a.name}`}
                >
                  {@render cover(a.cover, a.name)}
                  <span class="title">{a.name}</span>
                  <span class="by">{a.artist}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if tab === 'artists'}
        {#if openArtist}
          {@const all = openArtist.albums.flatMap((a) => a.songs)}
          <div class="detail">
            {@render back('Artists')}
            <div class="hero">
              <div class="hero-text">
                <h2>{openArtist.name}</h2>
                <p class="by">
                  {openArtist.albums.length}
                  {openArtist.albums.length === 1 ? 'album' : 'albums'} · {openArtist.count}
                  {openArtist.count === 1 ? 'song' : 'songs'}
                </p>
                {@render bulk(openArtist.name, all)}
              </div>
            </div>
            {#each openArtist.albums as a (a.key)}
              <h3 class="section">
                {@render cover(a.cover, a.name)}
                <span
                  >{a.name}{#if a.year}
                    · {a.year}{/if}</span
                >
              </h3>
              <ul class="list short" aria-label={`songs of ${a.name}`}>
                {#each a.songs as r (r.song.id)}
                  {@render songRow(r, all, all.indexOf(r), undefined, false)}
                {/each}
              </ul>
            {/each}
          </div>
        {:else if !artists.length}
          <p class="quiet">Nothing matches “{q.trim()}”.</p>
        {:else}
          <ul class="list short" aria-label="artists">
            {#each artists as a (a.key)}
              <li>
                <button
                  type="button"
                  class="row"
                  onclick={() => (open = { kind: 'artist', key: a.key })}
                  aria-label={`artist ${a.name}`}
                >
                  {@render cover(a.cover, a.name)}
                  <span class="name">
                    <span class="title">{a.name}</span>
                    <span class="by"
                      >{a.albums.length}
                      {a.albums.length === 1 ? 'album' : 'albums'} · {a.count}
                      {a.count === 1 ? 'song' : 'songs'}</span
                    >
                  </span>
                  <span class="chev" aria-hidden="true">›</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if tab === 'playlists'}
        {#if openList}
          {@const items = playlistRows(openList)}
          {@const list = items.map((x) => x.row)}
          <div class="detail">
            {@render back('Playlists')}
            <div class="hero">
              <div class="hero-text">
                <h2>{openList.title}</h2>
                <p class="by">
                  {list.length}
                  {list.length === 1 ? 'song' : 'songs'} · {total(list)}
                  {#if openList.songs.length > list.length}
                    · {openList.songs.length - list.length} not in your music{/if}
                </p>
                <div class="bulk">
                  <button
                    type="button"
                    class="go"
                    onclick={() => playFrom(list, 0)}
                    disabled={!list.length}
                  >
                    <Icon name="play" size={14} /> Play
                  </button>
                  <button
                    type="button"
                    class="ghost"
                    onclick={() => shuffleAll(list)}
                    disabled={!list.length}
                  >
                    <Icon name="shuffle" size={14} /> Shuffle
                  </button>
                  {#if onopen}
                    <button type="button" class="ghost" onclick={() => onopen(openList.id)}>
                      <Icon name="pen" size={14} /> Open its note
                    </button>
                  {/if}
                </div>
              </div>
            </div>
            {#if !list.length}
              <p class="quiet">
                Empty so far. Use ⋯ on any song, album or artist and choose <em>Add to playlist</em
                >.
              </p>
            {:else}
              <ul class="list short" aria-label={`songs of ${openList.title}`}>
                {#each items as x, i (x.row.song.id + ':' + x.at)}
                  {@render songRow(
                    x.row,
                    list,
                    i,
                    { id: openList.id, at: x.at, last: openList.songs.length - 1 },
                    true,
                  )}
                {/each}
              </ul>
            {/if}
          </div>
        {:else}
          <div class="bulk">
            <button
              type="button"
              class="go"
              onclick={() => {
                naming = 'empty';
                newName = '';
              }}><Icon name="plus" size={14} /> New playlist</button
            >
            <button
              type="button"
              class="ghost"
              disabled={!player.queue().length}
              onclick={() => {
                naming = 'queue';
                newName = '';
              }}>Save the queue as a playlist</button
            >
          </div>
          {#if naming}
            <form
              class="naming"
              onsubmit={(e) => {
                e.preventDefault();
                void makeNew();
              }}
            >
              <input
                placeholder="Name of the playlist"
                aria-label="name of the new playlist"
                bind:value={newName}
              />
              <button type="submit" class="go" disabled={busy}>Make it</button>
              <button type="button" class="ghost" onclick={() => (naming = null)}>Cancel</button>
            </form>
          {/if}
          {#if !playlists.length}
            <p class="quiet">
              {q.trim() ? `No playlist matches “${q.trim()}”.` : 'No playlists yet.'}
            </p>
          {:else}
            <ul class="list short" aria-label="playlists">
              {#each playlists as p (p.id)}
                {@const n = playlistRows(p).length}
                <li>
                  <button
                    type="button"
                    class="row"
                    onclick={() => (open = { kind: 'playlist', key: p.id })}
                    aria-label={`playlist ${p.title}`}
                  >
                    <span class="cover none small" aria-hidden="true"
                      ><Icon name="list" size={16} /></span
                    >
                    <span class="name">
                      <span class="title">{p.title}</span>
                      <span class="by">{n} {n === 1 ? 'song' : 'songs'}</span>
                    </span>
                    <span class="chev" aria-hidden="true">›</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
</div>

{#if menu}
  {@const m = menu}
  <div class="overlay">
    <button
      type="button"
      class="scrim"
      onclick={() => (menu = null)}
      aria-label="close"
      tabindex="-1"
    ></button>
    <div class="sheet" role="dialog" aria-modal="true" aria-label={m.title}>
      <header>
        <h2>{m.title}</h2>
        <button type="button" class="x" onclick={() => (menu = null)} aria-label="close">
          <Icon name="close" size={18} />
        </button>
      </header>
      {#if !m.picking}
        <div class="acts">
          <button
            type="button"
            onclick={() => act(() => player.playNext(tracks(m.rows)), 'plays next')}
          >
            Play next
          </button>
          <button
            type="button"
            onclick={() => act(() => player.enqueue(tracks(m.rows)), 'added to the queue')}
          >
            Add to queue
          </button>
          <button type="button" onclick={() => (menu = { ...m, picking: true })}
            >Add to playlist…</button
          >
          {#if m.inList}
            {@const w = m.inList}
            <button
              type="button"
              disabled={w.at === 0}
              onclick={() => moveIn(w.id, w.at, w.at - 1)}
            >
              Move up
            </button>
            <button
              type="button"
              disabled={w.at >= w.last}
              onclick={() => moveIn(w.id, w.at, w.at + 1)}
            >
              Move down
            </button>
            <button type="button" class="danger" onclick={() => dropFrom(w.id, w.at)}>
              Take out of this playlist
            </button>
          {/if}
          {#if m.rows.length === 1 && onopen}
            <button
              type="button"
              onclick={() => {
                menu = null;
                onopen(m.rows[0].song.id);
              }}>Open its note</button
            >
          {/if}
        </div>
      {:else}
        <div class="acts" role="group" aria-label="add to playlist">
          {#each lists as p (p.id)}
            <button type="button" onclick={() => addTo(p, m.rows)}>{p.title}</button>
          {/each}
          <form
            class="naming"
            onsubmit={(e) => {
              e.preventDefault();
              void makeWith(m.rows);
            }}
          >
            <input
              placeholder="New playlist"
              aria-label="name of the new playlist"
              bind:value={newName}
            />
            <button type="submit" class="go" disabled={busy}>Make it</button>
          </form>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .music-box {
    container: music / inline-size;
    height: 100%;
    min-height: 0;
  }
  .music {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3) var(--space-6);
    min-height: 0;
    height: 100%;
    box-sizing: border-box;
    overflow-y: auto;
  }
  .deck-col {
    flex: none;
  }
  .browse {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  /* A phone: the stacked deck scrolls away, the tabs and the search stay pinned. The wrapper
     steps out of the layout so that the head can stick to the scroller itself. */
  .top {
    display: contents;
  }
  /* Room for the deck as a strip: it and the head are pinned together, the songs scroll under. */
  @container music (min-width: 40rem) {
    .music {
      padding: 0 var(--space-4) var(--space-6);
    }
    .top {
      position: sticky;
      top: 0;
      z-index: 3;
      display: flex;
      flex-direction: column;
      gap: var(--space-2);
      padding-top: var(--space-3);
      background: var(--bg, var(--surface));
    }
    .top .head {
      position: static;
      flex-direction: row;
      align-items: center;
      padding: 0 0 var(--space-2);
    }
    .top .tabs {
      flex: 0 1 28rem;
    }
    .top .find {
      flex: 1;
    }
  }
  .head {
    position: sticky;
    top: calc(-1 * var(--space-3));
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) 0;
    background: var(--bg, var(--surface));
  }
  /* One segmented control: the four ways in, side by side, the chosen one lit. */
  .tabs {
    display: flex;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
  }
  .tabs button {
    flex: 1;
    min-width: 0;
    min-height: 2.5rem;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    font: inherit;
    font-size: var(--text-sm);
    color: inherit;
    cursor: pointer;
    white-space: nowrap;
  }
  .tabs button.sel {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
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
  .count {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    flex-wrap: wrap;
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list li {
    display: flex;
    align-items: center;
    height: 56px; /* ROW: fixed, so only the rows in view need drawing */
    box-sizing: border-box;
    border-bottom: 1px solid var(--border);
  }
  .list li.on .title,
  .list li.on .n {
    color: var(--accent);
    font-weight: 600;
  }
  .more {
    flex: none;
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .more:hover {
    background: var(--surface-hover);
  }
  .row {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-1);
    border: 0;
    background: none;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .n {
    width: 1.5rem;
    text-align: right;
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
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
  .quiet,
  .chev {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }
  .length {
    font-variant-numeric: tabular-nums;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
    gap: var(--space-3) var(--space-2);
  }
  .tile {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .tile .cover {
    width: 100%;
    height: auto;
    aspect-ratio: 1;
    font-size: 2rem;
  }
  .tile .title {
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .tile .by {
    font-size: var(--text-xs, 0.75rem);
  }
  @container music (min-width: 40rem) {
    .grid {
      grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
    }
  }
  .tile:hover .cover {
    filter: brightness(1.08);
  }
  .cover {
    flex: none;
    width: 2.5rem;
    height: 2.5rem;
    object-fit: cover;
    border-radius: var(--radius-md, 0.5rem);
    background: var(--surface-hover);
  }
  .cover.none {
    display: grid;
    place-items: center;
    color: var(--text-muted);
    font-weight: 700;
  }
  .cover.big {
    width: 8rem;
    height: 8rem;
    font-size: 2.5rem;
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .back {
    align-self: flex-start;
    min-height: 2.25rem;
    padding: 0 var(--space-2);
    border: 0;
    background: none;
    font: inherit;
    color: var(--accent);
    cursor: pointer;
  }
  .hero {
    display: flex;
    align-items: flex-end;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .hero-text {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }
  .hero h2 {
    margin: 0;
    font-size: var(--text-lg);
  }
  .hero p {
    margin: 0;
  }
  .section {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-2) 0 0;
    font-size: var(--text-base);
  }
  .bulk {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .go,
  .ghost {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border-radius: var(--radius-pill);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
  }
  .go {
    border: 0;
    background: var(--accent);
    color: var(--accent-contrast);
  }
  .ghost {
    border: 1px solid var(--border);
    background: none;
    color: inherit;
  }
  .go:disabled,
  .ghost:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .naming {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .naming input {
    flex: 1;
    min-width: 10rem;
    min-height: 2.5rem;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 0.5rem);
    background: var(--surface);
    font: inherit;
    color: var(--text);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .empty p,
  .quiet,
  .failed,
  .note {
    margin: 0;
  }
  .note {
    font-size: var(--text-sm);
    color: var(--accent);
  }
  .failed {
    font-size: var(--text-sm);
    color: var(--danger-fg);
  }

  .overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    justify-content: center;
    align-items: flex-end;
    box-sizing: border-box;
    height: var(--app-h, 100dvh);
    padding-top: calc(var(--overlay-inset, 2rem) + var(--safe-top, 0px));
  }
  .scrim {
    position: fixed;
    inset: 0;
    border: 0;
    background: rgb(0 0 0 / 0.45);
    cursor: default;
  }
  .sheet {
    position: relative;
    width: min(28rem, 100%);
    max-height: min(100%, 80dvh);
    overflow-y: auto;
    overscroll-behavior: contain;
    box-sizing: border-box;
    padding: var(--space-4) var(--space-4) max(var(--safe-bottom, 0px), var(--space-4));
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--surface-elevated);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    box-shadow: var(--shadow-lg);
  }
  .sheet header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .sheet h2 {
    margin: 0;
    font-size: var(--text-lg);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .x {
    width: 2.75rem;
    height: 2.75rem;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: var(--radius-pill);
    background: none;
    color: inherit;
    cursor: pointer;
  }
  .acts {
    display: flex;
    flex-direction: column;
  }
  .acts > button {
    min-height: 3rem;
    padding: 0 var(--space-2);
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .acts > button:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .acts > button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .acts .danger {
    color: var(--danger-fg);
  }
  .acts .naming {
    margin-top: var(--space-2);
  }
</style>
