<script lang="ts">
  import Board from './renderers/Board.svelte';
  import Agenda from './renderers/Agenda.svelte';
  import Calendar from './renderers/Calendar.svelte';
  import Timeline from './renderers/Timeline.svelte';
  import Search from './renderers/Search.svelte';
  import Icon from './lib/Icon.svelte';
  import { orderColumns, moveValue } from './lib/boardOrder';
  import {
    getBoard,
    getAgenda,
    recent,
    capture,
    setProperty,
    search as ipcSearch,
    commit,
    backup,
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
  let draft = $state('');
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let openId = $state<string | null>(null);
  let startEditing = $state(false);
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
  // The board with columns arranged by the saved order for the current grouping.
  let displayBoard = $derived(
    board ? { ...board, columns: orderColumns(board.columns, orders[groupBy] ?? []) } : null,
  );
  function onReorder(fromValue: string, toValue: string, before: boolean) {
    if (!board) return;
    const current = orderColumns(board.columns, orders[groupBy] ?? []).map((c) => c.value);
    orders = { ...orders, [groupBy]: moveValue(current, fromValue, toValue, before) };
    try {
      localStorage.setItem('fm-board-order', JSON.stringify(orders));
    } catch {
      /* private mode / quota — order just won't persist */
    }
  }

  let railCollapsed = $state(false);
  let paletteOpen = $state(false);
  let captureEl = $state<HTMLInputElement | undefined>(undefined);
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
    { label: 'Search notes', run: () => (view = 'search') },
    { label: 'New note', run: onNew },
    { label: 'New board', run: onNewBoard },
    { label: 'Capture a note', run: () => captureEl?.focus() },
    { label: 'Toggle theme', run: toggleTheme },
    { label: 'Back up the vault', run: onBackup },
  ]);

  // Keyboard map: ⌘K palette · ⌘\ toggle rail · 1–4 views · / search · c capture · Esc close.
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
    if (openId) return; // the note panel owns keys while open
    const views: View[] = ['board', 'agenda', 'timeline', 'search'];
    if (e.key === '/') {
      e.preventDefault();
      view = 'search';
    } else if (e.key === 'c') {
      e.preventDefault();
      captureEl?.focus();
    } else if (e.key >= '1' && e.key <= '4') {
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

  // Liveness heartbeat: while this tab is open, ping the server every few seconds
  // so its auto-shutdown watchdog knows someone is here. When the tab closes the
  // pings stop and the server exits — closing the tab closes the app, with no
  // background process left over. Only in the served build (the mock has no
  // server); a reload's brief gap stays under the server's idle window.
  $effect(() => {
    if (!import.meta.env.PROD) return;
    void ping().catch(() => {});
    const id = setInterval(() => void ping().catch(() => {}), 3000);
    return () => clearInterval(id);
  });

  async function onCapture(e: Event) {
    e.preventDefault();
    const body = draft.trim();
    if (!body) return;
    draft = '';
    try {
      await capture(body);
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // "New note": create a blank note and open it straight in the editor (property
  // form + empty body), Obsidian/Notion style. Differentiate with tags, not type.
  async function onNew() {
    try {
      const meta = await capture('');
      startEditing = true;
      openId = meta.id;
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
        '{"type":"excalidraw","version":2,"source":"formicarium","elements":[],"appState":{},"files":{}}';
      const meta = await capture(scene);
      await setProperty(meta.id, 'view', 'board');
      await setProperty(meta.id, 'title', 'Untitled board');
      startEditing = false;
      openId = meta.id;
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // Open an existing note in read mode (never inherit a stale startEditing).
  function openNote(id: string) {
    startEditing = false;
    openId = id;
  }

  async function onMove(id: string, value: string) {
    // The drag write-back: set the grouped property to the target column's value.
    try {
      await setProperty(id, groupBy, value);
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

  async function onBackup() {
    notice = null;
    try {
      await backup();
      notice = 'Backed up.';
      setTimeout(() => (notice = null), 3000);
    } catch (e) {
      error = String(e);
    }
  }

  // Editing a note's properties can change its type/status, so refresh the
  // current view when the panel closes.
  function closeNote() {
    openId = null;
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
      <span class="wordmark">formicarium</span>
    </div>

    <form class="composer" onsubmit={onCapture}>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:this={captureEl} placeholder="Capture a note…" bind:value={draft} autofocus />
      <div class="composer-row">
        <button type="button" class="new-btn" onclick={onNew} title="Create a note and open the editor">
          <Icon name="plus" size={15} /> <span class="label">New note</span>
        </button>
        <button type="button" class="new-btn" onclick={onNewBoard} title="Create a whiteboard (Excalidraw canvas)">
          <Icon name="pen" size={15} /> <span class="label">New board</span>
        </button>
      </div>
    </form>

    <button class="nav-item find" class:active={view === 'search'} onclick={() => (view = 'search')}>
      <Icon name="search" /> <span class="label">Search</span>
    </button>

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

    <div class="sidebar-foot">
      <button class="icon-btn" onclick={() => (paletteOpen = true)} aria-label="command palette" title="Command palette (Ctrl+K)">
        <Icon name="command" />
      </button>
      <button class="icon-btn" onclick={toggleTheme} aria-label="toggle theme" title="Toggle light/dark">
        <Icon name={theme === 'dark' ? 'sun' : 'moon'} />
      </button>
      <button class="backup" onclick={onBackup} title="Snapshot the vault with restic">
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
        {:else if view === 'search'}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="searchbox"
            placeholder="Search notes, tasks, documents…"
            bind:value={searchQuery}
            oninput={onSearchInput}
            spellcheck="false"
            autofocus
          />
        {/if}
      </div>
    </header>

    {#if error}<p class="banner error">{error}</p>{/if}
    {#if notice}<p class="banner notice">{notice}</p>{/if}

    <div class="stage">
      {#if view === 'board'}
        {#if displayBoard}
          <Board board={displayBoard} onmove={onMove} onreorder={onReorder} onopen={openNote} />
        {:else}
          <p class="empty">Loading…</p>
        {/if}
      {:else if view === 'search'}
        <Search cards={results} query={searchQuery} onopen={openNote} />
      {:else if !cards}
        <p class="empty">Loading…</p>
      {:else if view === 'timeline'}
        <Timeline {cards} onopen={openNote} />
      {:else if agendaMode === 'list'}
        <Agenda {cards} onopen={openNote} />
      {:else}
        <Calendar {cards} range={agendaMode === 'week' ? 'week' : 'month'} onopen={openNote} />
      {/if}
    </div>
  </div>

  {#if openId}
    <!-- Lazy: the read view (marked + KaTeX + Mermaid) only loads when a note is
         opened, so the core bundle stays small. -->
    {#await import('./lib/NotePanel.svelte') then { default: NotePanel }}
      <NotePanel id={openId} {startEditing} statuses={knownStatuses} onclose={closeNote} onsaved={scheduleCommit} />
    {/await}
  {/if}

  {#if paletteOpen}
    {#await import('./lib/CommandPalette.svelte') then { default: CommandPalette }}
      <CommandPalette {commands} onclose={() => (paletteOpen = false)} />
    {/await}
  {/if}
</div>

<style>
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

  /* Composer: the create surface, visually distinct from Search (owner's #1 ask). */
  .composer {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-elevated);
  }
  .composer input {
    width: 100%;
    box-sizing: border-box;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-size: var(--text-sm);
  }
  .composer-row {
    display: flex;
    gap: var(--space-2);
  }
  .new-btn {
    flex: 1;
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
  .find {
    margin-top: var(--space-1);
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
  .rail-collapsed .composer,
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
  .searchbox {
    width: min(28rem, 40vw);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-elevated);
    color: var(--text);
    font-size: var(--text-sm);
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
    .composer,
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
