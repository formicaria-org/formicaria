<script lang="ts">
  import { tick } from 'svelte';
  import Board from './renderers/Board.svelte';
  import Agenda from './renderers/Agenda.svelte';
  import Calendar from './renderers/Calendar.svelte';
  import Timeline from './renderers/Timeline.svelte';
  import Search from './renderers/Search.svelte';
  import Icon from './lib/Icon.svelte';
  import { arrange, orderColumns, moveValue, placeValue } from './lib/boardOrder';
  import {
    getBoard,
    getAgenda,
    recent,
    capture,
    setProperty,
    search as ipcSearch,
    commit,
    ping,
  } from './lib/ipc';
  import type { Board as BoardData, ObjectMeta } from './lib/types';

  type View = 'board' | 'agenda' | 'timeline' | 'search';
  let view = $state<View>('board');
  let groupBy = $state('status');
  let agendaMode = $state<'month' | 'week' | 'list'>('month');
  let board = $state<BoardData | null>(null);
  let cards = $state<ObjectMeta[] | null>(null);
  let results = $state<ObjectMeta[]>([]);
  let searchQuery = $state('');
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  // The open notes, left to right: a trail, not a single note. Opening from a
  // view starts a fresh one; following a note reference pushes onto it, so the
  // note you came from stays on screen. Empty = nothing open.
  let openIds = $state<string[]>([]);
  let startEditing = $state(false);
  let trailEl = $state<HTMLElement | undefined>(undefined);

  // Full screen by default (the preferred reading/writing mode); the toggle
  // shrinks to a docked side-sheet, and the choice is remembered per-browser like
  // the theme. Anything other than the stored '0' (incl. unset) means full screen.
  // Guard storage access — it's absent in the test env and in private mode.
  function readWidePref(): boolean {
    try {
      return localStorage.getItem('fm-note-wide') !== '0';
    } catch {
      return true;
    }
  }
  let wide = $state(readWidePref());
  function toggleWide() {
    wide = !wide;
    try {
      localStorage.setItem('fm-note-wide', wide ? '1' : '0');
    } catch {
      /* private mode / storage disabled — the default (full screen) still applies */
    }
  }
  // Distinct status values seen so far — feeds the note panel's status datalist,
  // so the picker is data-driven (no hardcoded status literal anywhere).
  let knownStatuses = $state<string[]>([]);

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let commitTimer: ReturnType<typeof setTimeout> | undefined;

  let theme = $state(document.documentElement.dataset.theme ?? 'dark');
  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('fm-theme', theme);
  }

  // User-chosen column order, per group-by, persisted client-side (like the
  // theme). A view preference, not note data — so it lives in localStorage, not
  // the vault. Reconciled against live columns by orderColumns (new columns
  // appear, deleted ones are ignored).
  function loadOrders(): Record<string, string[]> {
    try {
      return JSON.parse(localStorage.getItem('fm-board-order') ?? '{}');
    } catch {
      return {};
    }
  }
  let orders = $state<Record<string, string[]>>(loadOrders());

  // Card order *within* a column — where you dropped it, not when it was created.
  // Same reasoning and same storage as the column order above: a view preference,
  // per group-by, keyed by column value → the note ids in the order you chose.
  function loadCardOrders(): Record<string, Record<string, string[]>> {
    try {
      return JSON.parse(localStorage.getItem('fm-card-order') ?? '{}');
    } catch {
      return {};
    }
  }
  let cardOrders = $state<Record<string, Record<string, string[]>>>(loadCardOrders());

  // Which audiences to show. A **view preference**, like the column order — it lives in
  // localStorage and never in a vault, because what you are currently looking at is not
  // knowledge and would embarrass you in five years. Hiding a vault hides its notes from
  // the views; it does not, and must not, mean anything about who can see them. That is
  // decided by which repo holds the file, and nothing in this browser can change it.
  let hiddenVaults = $state<string[]>(
    (() => {
      try {
        return JSON.parse(localStorage.getItem('fm-hidden-vaults') ?? '[]');
      } catch {
        return [];
      }
    })(),
  );
  // Derived from the *unfiltered* data, so hiding the last vault does not also hide the
  // chip you would use to bring it back.
  const allVaults = $derived(
    [
      ...new Set(
        [...(board?.columns.flatMap((c) => c.cards) ?? []), ...(cards ?? [])]
          .map((n) => n.vault)
          .filter(Boolean),
      ),
    ].sort(),
  );
  const shown = (n: ObjectMeta) => !n.vault || !hiddenVaults.includes(n.vault);
  const visibleCards = $derived(cards?.filter(shown) ?? null);

  function toggleVault(name: string) {
    hiddenVaults = hiddenVaults.includes(name)
      ? hiddenVaults.filter((v) => v !== name)
      : [...hiddenVaults, name];
    try {
      localStorage.setItem('fm-hidden-vaults', JSON.stringify(hiddenVaults));
    } catch {
      // A browser that won't remember the preference still honours it this session.
    }
  }

  // The board with columns arranged by the saved order for the current grouping,
  // and each column's cards arranged by the saved drop order.
  let displayBoard = $derived(
    board
      ? {
          ...board,
          columns: orderColumns(board.columns, orders[groupBy] ?? []).map((c) => ({
            ...c,
            cards: arrange(
              c.cards.filter(shown),
              cardOrders[groupBy]?.[c.value] ?? [],
              (n) => n.id,
            ),
          })),
        }
      : null,
  );
  function persistOrders() {
    try {
      localStorage.setItem('fm-board-order', JSON.stringify(orders));
      localStorage.setItem('fm-card-order', JSON.stringify(cardOrders));
    } catch {
      /* private mode / quota — order just won't persist */
    }
  }
  function onReorder(fromValue: string, toValue: string, before: boolean) {
    if (!board) return;
    const current = orderColumns(board.columns, orders[groupBy] ?? []).map((c) => c.value);
    orders = { ...orders, [groupBy]: moveValue(current, fromValue, toValue, before) };
    persistOrders();
  }

  let railCollapsed = $state(false);
  let paletteOpen = $state(false);
  let backupOpen = $state(false);
  let searchEl = $state<HTMLInputElement | undefined>(undefined);
  const VIEW_TITLES: Record<View, string> = {
    board: 'Board',
    agenda: 'Agenda',
    timeline: 'Timeline',
    search: 'Search',
  };
  let viewTitle = $derived(VIEW_TITLES[view]);

  // Commands surfaced in the ⌘K palette (label + action).
  let commands = $derived([
    { label: 'Go to Board', run: () => (view = 'board') },
    { label: 'Go to Agenda', run: () => (view = 'agenda') },
    { label: 'Go to Timeline', run: () => (view = 'timeline') },
    { label: 'Search notes', run: () => { view = 'search'; searchEl?.focus(); } },
    { label: 'New note', run: onNew },
    { label: 'New board', run: onNewBoard },
    { label: 'Toggle theme', run: toggleTheme },
    { label: 'Back up the vault', run: onBackup },
  ]);

  // Keyboard map: ⌘K palette · ⌘\ toggle rail · 1–3 views · / search · c new note · Esc close.
  function onGlobalKey(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      paletteOpen = !paletteOpen;
      return;
    }
    if (mod && e.key === '\\') {
      e.preventDefault();
      railCollapsed = !railCollapsed;
      return;
    }
    if (paletteOpen && e.key === 'Escape') {
      paletteOpen = false;
      return;
    }
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return; // don't hijack typing
    if (openIds.length) return; // the note panel owns keys while open
    const views: View[] = ['board', 'agenda', 'timeline'];
    if (e.key === '/') {
      e.preventDefault();
      view = 'search';
      searchEl?.focus();
    } else if (e.key === 'c') {
      e.preventDefault();
      void onNew();
    } else if (e.key >= '1' && e.key <= '3') {
      view = views[Number(e.key) - 1];
    }
  }

  function learnStatuses(list: ObjectMeta[]) {
    const set = new Set(knownStatuses);
    let changed = false;
    for (const c of list) {
      if (c.status && !set.has(c.status)) {
        set.add(c.status);
        changed = true;
      }
    }
    if (changed) knownStatuses = [...set];
  }

  async function refresh() {
    try {
      if (view === 'board') {
        board = await getBoard(groupBy);
        learnStatuses(board.columns.flatMap((c) => c.cards));
      } else if (view === 'timeline') {
        cards = await recent();
        learnStatuses(cards);
      } else if (view === 'agenda') {
        cards = await getAgenda();
        learnStatuses(cards);
      }
      // 'search' is driven by the query input, not by the view switch.
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  // Reload when the view switches, or (in board view) when the grouping changes.
  $effect(() => {
    void view;
    if (view === 'board') void groupBy;
    void refresh();
  });

  // Liveness heartbeat, and the local poll — one beat, two jobs.
  //
  // Liveness: while this tab is open, ping the server every few seconds so its
  // auto-shutdown watchdog knows someone is here. When the tab closes the pings stop
  // and the server exits — closing the tab closes the app, with no background process
  // left over. Only in the served build (the mock has no server); a reload's brief gap
  // stays under the server's idle window.
  //
  // The poll: the same request tells us whether the vault moved under us. It has to,
  // because the views are served from SQLite, so an edit made by anything else — a
  // `git pull`, a merge driver, Vim — is otherwise invisible until the app restarts.
  // Refresh only when something actually changed: re-running the query every 3s would
  // fight the user's own scrolling and drag state for no reason.
  $effect(() => {
    if (!import.meta.env.PROD) return;
    const beat = async () => {
      const r = await ping().catch(() => null);
      if (r?.changed) await refresh();
    };
    void beat();
    const id = setInterval(() => void beat(), 3000);
    return () => clearInterval(id);
  });

  // "New note": create a blank note and open it straight in the editor (property
  // form + empty body), Obsidian/Notion style. Differentiate with tags, not type.
  async function onNew() {
    try {
      const meta = await capture('');
      startEditing = true;
      openIds = [meta.id];
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // A board is a note whose body is an Excalidraw scene, flagged `view: board`.
  // Create it empty, mark it, give it a title, then open it (the panel renders the
  // canvas for board notes). No new command or Kind — just a property.
  async function onNewBoard() {
    try {
      const scene =
        // `source` stays `formicarium` on purpose — do NOT rename it to match the
        // wordmark. Excalidraw writes this field into every board's JSON **on disk**, so
        // changing it rewrites every board file the next time it is saved: churn in git,
        // for a string no user ever sees. Migrate deliberately or leave it.
        '{"type":"excalidraw","version":2,"source":"formicarium","elements":[],"appState":{},"files":{}}';
      const meta = await capture(scene);
      await setProperty(meta.id, 'view', 'board');
      await setProperty(meta.id, 'title', 'Untitled board');
      startEditing = false;
      openIds = [meta.id];
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // Open an existing note in read mode (never inherit a stale startEditing).
  // Opening from a view starts a new trail — the old one was a different thought.
  function openNote(id: string) {
    startEditing = false;
    openIds = [id];
  }

  // Follow a note reference: push the target onto the trail. Re-following a note
  // already open truncates back to it rather than opening a second copy — two
  // panes of one note would be two editors over one file.
  async function pushNote(id: string) {
    const at = openIds.indexOf(id);
    openIds = at === -1 ? [...openIds, id] : openIds.slice(0, at + 1);
    await tick(); // let the new pane mount before scrolling to it
    // Guarded like the localStorage access above: scrollTo is absent in jsdom,
    // and failing to scroll must never break the navigation itself.
    trailEl?.scrollTo?.({ left: trailEl.scrollWidth, behavior: 'smooth' });
  }

  // Close one pane and everything downstream of it — the trail past it was
  // reached *through* it, so it no longer has a path.
  function closeFrom(i: number) {
    if (i === 0) return closeNote();
    openIds = openIds.slice(0, i);
  }

  async function onMove(id: string, value: string, beforeId: string | null) {
    // The drag write-back: set the grouped property to the target column's value.
    // The card's place *within* the column is a view preference, so it is saved
    // client-side rather than written to the note. Save it first: it is keyed by
    // id, so the refresh below re-reads it and the card lands where it was
    // dropped — including when the drag crossed into a different column.
    const column = displayBoard?.columns.find((c) => c.value === value);
    const ids = column?.cards.map((n) => n.id) ?? [];
    cardOrders = {
      ...cardOrders,
      [groupBy]: { ...cardOrders[groupBy], [value]: placeValue(ids, id, beforeId) },
    };
    persistOrders();
    try {
      await setProperty(id, groupBy, value);
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // Rotate a note's status from a card or the open note's header. Same write path
  // as a board drag, but always on `status` — the board may be grouped by anything.
  async function onSetStatus(id: string, value: string | null) {
    try {
      await setProperty(id, 'status', value ?? '');
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  function onSearchInput() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(runSearch, 250);
  }
  async function runSearch() {
    try {
      results = await ipcSearch(searchQuery);
      learnStatuses(results);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  // Debounced auto-commit after any successful write. Best-effort — a clean tree
  // is a no-op and a missing git binary must never block editing.
  function scheduleCommit() {
    clearTimeout(commitTimer);
    commitTimer = setTimeout(() => {
      commit(`auto: ${new Date().toISOString()}`).catch(() => {});
    }, 5000);
  }

  // Backing up is a conversation, not a fire-and-forget: the panel owns setting
  // the remote, choosing whether media rides along, and reporting what actually
  // left the machine.
  function onBackup() {
    backupOpen = true;
  }

  // Editing a note's properties can change its type/status, so refresh the
  // current view when the panel closes.
  function closeNote() {
    openIds = [];
    startEditing = false;
    void refresh();
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

<div class="app" class:rail-collapsed={railCollapsed}>
  <nav class="sidebar" aria-label="navigation">
    <div class="brand">
      <button
        class="icon-btn rail-toggle"
        onclick={() => (railCollapsed = !railCollapsed)}
        aria-label="toggle sidebar"
        title="Toggle sidebar (Ctrl+\)"
      >
        <Icon name={railCollapsed ? 'chevronRight' : 'chevronLeft'} />
      </button>
      <span class="wordmark">formicaria</span>
    </div>

    <div class="create">
      <label class="searchfield">
        <Icon name="search" size={16} />
        <input
          bind:this={searchEl}
          class="sidebar-search"
          type="search"
          placeholder="Search notes…"
          bind:value={searchQuery}
          oninput={onSearchInput}
          onfocus={() => (view = 'search')}
          spellcheck="false"
          aria-label="search notes"
        />
      </label>
      <button type="button" class="new-btn" onclick={onNew} title="Create a note and open the editor">
        <Icon name="plus" size={15} /> <span class="label">New note</span>
      </button>
      <button type="button" class="new-btn secondary" onclick={onNewBoard} title="Create a whiteboard (Excalidraw canvas)">
        <Icon name="pen" size={15} /> <span class="label">New board</span>
      </button>
    </div>

    <ul class="nav">
      <li>
        <button class="nav-item" class:active={view === 'board'} aria-current={view === 'board' ? 'page' : undefined} onclick={() => (view = 'board')}>
          <Icon name="board" /> <span class="label">Board</span>
        </button>
      </li>
      <li>
        <button class="nav-item" class:active={view === 'agenda'} aria-current={view === 'agenda' ? 'page' : undefined} onclick={() => (view = 'agenda')}>
          <Icon name="calendar" /> <span class="label">Agenda</span>
        </button>
      </li>
      <li>
        <button class="nav-item" class:active={view === 'timeline'} aria-current={view === 'timeline' ? 'page' : undefined} onclick={() => (view = 'timeline')}>
          <Icon name="timeline" /> <span class="label">Timeline</span>
        </button>
      </li>
    </ul>

    <!-- Only when there is a boundary to draw. One vault means no audiences to tell
         apart, and a filter with one chip is furniture. -->
    {#if allVaults.length > 1 && !railCollapsed}
      <div class="vaults">
        <span class="vaults-label">Vaults</span>
        {#each allVaults as v (v)}
          <button
            class="vault-chip"
            class:off={hiddenVaults.includes(v)}
            aria-pressed={!hiddenVaults.includes(v)}
            onclick={() => toggleVault(v)}
            title={hiddenVaults.includes(v) ? `Show ${v}` : `Hide ${v}`}
          >
            {v}
          </button>
        {/each}
      </div>
    {/if}

    <div class="sidebar-foot">
      <button class="icon-btn" onclick={() => (paletteOpen = true)} aria-label="command palette" title="Command palette (Ctrl+K)">
        <Icon name="command" />
      </button>
      <button class="icon-btn" onclick={toggleTheme} aria-label="toggle theme" title="Toggle light/dark">
        <Icon name={theme === 'dark' ? 'sun' : 'moon'} />
      </button>
      <button class="backup" onclick={onBackup} title="Push your notes; optionally snapshot media">
        <Icon name="backup" size={15} /> <span class="label">Back up</span>
      </button>
    </div>
  </nav>

  <div class="main">
    <header class="content-header">
      <h1 class="view-title">{viewTitle}</h1>
      <div class="header-controls">
        {#if view === 'board'}
          <label class="group">
            <span>group by</span>
            <input list="props" bind:value={groupBy} spellcheck="false" />
            <datalist id="props">
              <option value="status"></option>
              <option value="project"></option>
              <option value="tags"></option>
            </datalist>
          </label>
        {:else if view === 'agenda'}
          <div class="seg" role="group" aria-label="agenda layout">
            <button class:active={agendaMode === 'month'} aria-pressed={agendaMode === 'month'} onclick={() => (agendaMode = 'month')}>Month</button>
            <button class:active={agendaMode === 'week'} aria-pressed={agendaMode === 'week'} onclick={() => (agendaMode = 'week')}>Week</button>
            <button class:active={agendaMode === 'list'} aria-pressed={agendaMode === 'list'} onclick={() => (agendaMode = 'list')}>List</button>
          </div>
        {/if}
      </div>
    </header>

    {#if error}<p class="banner error">{error}</p>{/if}
    {#if notice}<p class="banner notice">{notice}</p>{/if}

    <div class="stage">
      {#if view === 'board'}
        {#if displayBoard}
          <Board
            board={displayBoard}
            onmove={onMove}
            onreorder={onReorder}
            onopen={openNote}
            statuses={knownStatuses}
            onstatus={onSetStatus}
          />
        {:else}
          <p class="empty">Loading…</p>
        {/if}
      {:else if view === 'search'}
        <Search cards={results} query={searchQuery} onopen={openNote} />
      {:else if !visibleCards}
        <p class="empty">Loading…</p>
      {:else if view === 'timeline'}
        <Timeline cards={visibleCards} onopen={openNote} statuses={knownStatuses} onstatus={onSetStatus} />
      {:else if agendaMode === 'list'}
        <Agenda cards={visibleCards} onopen={openNote} />
      {:else}
        <Calendar cards={visibleCards} range={agendaMode === 'week' ? 'week' : 'month'} onopen={openNote} />
      {/if}
    </div>
  </div>

  {#if openIds.length}
    <!-- The trail is modal as a whole: one overlay and one backdrop for all of
         it, however many panes deep it runs. Lazy: the read view (marked +
         KaTeX + Mermaid) only loads when a note is opened, so the core bundle
         stays small — and the import resolves once, not once per pane. -->
    <div class="overlay" class:wide>
      <button class="backdrop" aria-label="close note" onclick={closeNote}></button>
      <div class="trail" bind:this={trailEl}>
        {#await import('./lib/NotePanel.svelte') then { default: NotePanel }}
          {#each openIds as id, i (id)}
            <NotePanel
              {id}
              {wide}
              solo={openIds.length === 1}
              startEditing={i === openIds.length - 1 && startEditing}
              statuses={knownStatuses}
              onclose={() => closeFrom(i)}
              onnavigate={pushNote}
              ontogglewide={toggleWide}
              onsaved={scheduleCommit}
            />
          {/each}
        {/await}
      </div>
    </div>
  {/if}

  {#if paletteOpen}
    {#await import('./lib/CommandPalette.svelte') then { default: CommandPalette }}
      <CommandPalette {commands} onclose={() => (paletteOpen = false)} />
    {/await}
  {/if}

  {#if backupOpen}
    {#await import('./lib/BackupPanel.svelte') then { default: BackupPanel }}
      <BackupPanel onclose={() => (backupOpen = false)} />
    {/await}
  {/if}
</div>

<style>
  /* ── The note trail: the modal surface holding one or more open panes ── */
  .overlay {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: flex-end;
    z-index: 50;
  }
  .overlay.wide {
    justify-content: stretch;
    align-items: stretch;
    padding: 0;
  }
  /* A full-area button behind the panes: clicking outside closes, with no
     stopPropagation and no listeners on non-interactive elements. */
  .backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgb(0 0 0 / 0.5);
    cursor: default;
  }
  /* Panes sit left-to-right in the order they were opened. The row scrolls
     rather than squeezing them, so a long trail stays readable; each pane snaps
     so you land on a note, not between two. `justify-content: flex-end` keeps a
     short trail docked right, where the single sheet has always been. */
  .trail {
    position: relative;
    display: flex;
    justify-content: flex-end;
    margin-left: auto;
    max-width: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-snap-type: x proximity;
    overscroll-behavior-x: contain;
  }

  /* ── Shell: recessive left rail + main column (Linear/Things-style) ── */
  .app {
    display: grid;
    grid-template-columns: 15rem 1fr;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
    transition: grid-template-columns var(--dur-med) var(--ease);
  }
  .app.rail-collapsed {
    grid-template-columns: 3.25rem 1fr;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-height: 0;
    padding: var(--space-3);
    background: var(--surface);
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: var(--header-h);
    padding-left: var(--space-1);
  }
  .wordmark {
    font-weight: 700;
    font-size: var(--text-md);
    letter-spacing: 0.01em;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
  }
  .rail-toggle {
    flex: none;
  }

  /* Create surface: the primary Search field over the two create buttons. */
  .create {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .searchfield {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-elevated);
    color: var(--text-muted);
  }
  .searchfield:focus-within {
    border-color: var(--accent);
    color: var(--accent);
  }
  .sidebar-search {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: var(--text-sm);
    outline: none;
  }
  .sidebar-search::placeholder {
    color: var(--text-muted);
  }
  .new-btn {
    width: 100%;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .new-btn:hover {
    background: var(--accent-hover);
  }
  .new-btn.secondary {
    background: var(--surface-elevated);
    color: var(--text);
    border-color: var(--border);
  }
  .new-btn.secondary:hover {
    background: var(--accent-subtle);
    border-color: var(--accent);
    color: var(--accent);
  }

  /* Nav items: quiet by default, tinted when active. */
  .nav {
    list-style: none;
    margin: var(--space-1) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    box-sizing: border-box;
    padding: var(--space-2) var(--space-2);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .nav-item :global(svg) {
    flex: none;
  }
  .nav-item .label {
    overflow: hidden;
    white-space: nowrap;
  }
  .nav-item:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--accent-subtle);
    color: var(--accent);
    font-weight: 600;
  }
  /* Hiding a vault is a view preference and nothing more — it changes what is on
     screen, never who can see a note. The chips are quiet on purpose: they must not
     read like a permission control. */
  .vaults {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding: 0 var(--space-3) var(--space-3);
    align-items: center;
  }
  .vaults-label {
    width: 100%;
    font-size: var(--text-xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 0.2rem;
  }
  .vault-chip {
    font-size: 0.68rem;
    padding: 0.1rem 0.45rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    cursor: pointer;
    letter-spacing: 0.02em;
  }
  .vault-chip.off {
    color: var(--text-muted);
    opacity: 0.5;
    text-decoration: line-through;
  }
  .sidebar-foot {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }
  .backup {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-left: auto;
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    font-size: var(--text-xs);
    white-space: nowrap;
  }
  .backup:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .icon-btn {
    display: grid;
    place-items: center;
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition:
      background var(--dur-fast) var(--ease),
      color var(--dur-fast) var(--ease);
  }
  .icon-btn:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  /* Collapsed rail: icons only. */
  .rail-collapsed .wordmark,
  .rail-collapsed .create,
  .rail-collapsed .label {
    display: none;
  }
  .rail-collapsed .nav-item {
    justify-content: center;
    padding-inline: 0;
  }
  .rail-collapsed .sidebar-foot {
    flex-direction: column;
  }
  .rail-collapsed .backup {
    margin-left: 0;
    padding: var(--space-2);
  }

  /* ── Main column ── */
  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
  }
  .content-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    height: var(--header-h);
    padding: 0 var(--space-5);
    border-bottom: 1px solid var(--border);
  }
  .view-title {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 650;
    color: var(--text);
  }
  .header-controls {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .group input {
    width: 7rem;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-elevated);
    color: var(--text);
  }
  /* Segmented control for the agenda layout. */
  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .seg button {
    padding: var(--space-1) var(--space-3);
    border: none;
    border-radius: calc(var(--radius-sm) - 2px);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .seg button.active {
    background: var(--surface-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .stage {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .banner {
    margin: 0;
    padding: var(--space-2) var(--space-5);
    font-size: var(--text-sm);
  }
  .banner.error {
    background: var(--danger-bg);
    color: var(--danger-fg);
  }
  .banner.notice {
    background: var(--ok-bg);
    color: var(--ok-fg);
  }
  .empty {
    padding: var(--space-7);
    color: var(--text-muted);
  }

  /* Below tablet, force the collapsed icon rail. */
  @media (max-width: 900px) {
    .app,
    .app.rail-collapsed {
      grid-template-columns: 3.25rem 1fr;
    }
    .wordmark,
    .create,
    .nav-item .label,
    .backup .label {
      display: none;
    }
    .nav-item {
      justify-content: center;
      padding-inline: 0;
    }
    .sidebar-foot {
      flex-direction: column;
    }
  }
</style>
