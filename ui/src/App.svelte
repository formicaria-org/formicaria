<script lang="ts">
  import Icon from './lib/Icon.svelte';
  import Pane from './lib/Pane.svelte';
  import {
    newPane,
    distinctFeeds,
    feedKey,
    reorder,
    rendererKind,
    MAX_PANES,
    type Workspace,
    type Pane as PaneT,
    type PaneKind,
    type Feed,
  } from './lib/panes';
  import {
    getBoard,
    getAgenda,
    recent,
    capture,
    setProperty,
    search as ipcSearch,
    commit,
    ping,
    listVaults,
    listViews,
    runView,
  } from './lib/ipc';
  import type { ObjectMeta, VaultInfo, ViewInfo } from './lib/types';
  import NewVault from './lib/NewVault.svelte';

  // The flexible workspace: panes the user opens, arranges, and resizes. `feeds` holds the
  // fetched data keyed by feed (panes sharing a feed share one fetch). `focused` is the pane
  // keyboard/新-pane actions target.
  function loadWorkspace(): Workspace {
    try {
      const w = JSON.parse(localStorage.getItem('fm-workspace') ?? 'null');
      if (w && Array.isArray(w.panes) && w.panes.length && typeof w.cols === 'number') return w;
    } catch {
      /* fall through to default */
    }
    return { cols: 2, panes: [newPane('board')] };
  }
  let workspace = $state<Workspace>(loadWorkspace());
  let feeds = $state<Record<string, Feed>>({});
  let focused = $state(0);
  let searchQuery = $state(''); // the top-bar global search box
  let error = $state<string | null>(null);

  function persistWorkspace() {
    try {
      localStorage.setItem('fm-workspace', JSON.stringify(workspace));
    } catch {
      /* private mode — the workspace just won't persist this session */
    }
  }
  function addPane(kind: PaneKind, over: Partial<PaneT> = {}) {
    if (workspace.panes.length >= MAX_PANES) {
      notice = `That's the most panes at once (${MAX_PANES}). Close one to open another.`;
      return;
    }
    workspace = { ...workspace, panes: [...workspace.panes, newPane(kind, over)] };
    focused = workspace.panes.length - 1;
    persistWorkspace();
    void refresh();
  }
  function closePane(id: string) {
    const panes = workspace.panes.filter((p) => p.id !== id);
    workspace = { ...workspace, panes: panes.length ? panes : [newPane('board')] };
    if (focused >= workspace.panes.length) focused = workspace.panes.length - 1;
    persistWorkspace();
    void refresh();
  }
  function changePane(id: string, patch: Partial<PaneT>) {
    workspace = {
      ...workspace,
      panes: workspace.panes.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    };
    persistWorkspace();
    void refresh();
  }
  function setCols(cols: number) {
    workspace = { ...workspace, cols: Math.max(1, Math.min(cols, 4)) };
    persistWorkspace();
  }
  function movePane(from: number, to: number) {
    workspace = { ...workspace, panes: reorder(workspace.panes, from, to) };
    if (focused === from) focused = to;
    persistWorkspace();
  }
  // Resize a pane's span. Unlike changePane this does NOT refresh: a span change moves no
  // feed, so refetching would be pure waste (and would fight the drag).
  function resizePane(id: string, patch: Partial<PaneT>) {
    workspace = {
      ...workspace,
      panes: workspace.panes.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    };
    persistWorkspace();
  }
  let notice = $state<string | null>(null);
  // A note is a pane now (kind:'note'), not a separate side-trail. `editingId` is the one note
  // that should open straight in the editor — set by "New note", read once by that pane on mount.
  // It is transient (never persisted), so a reload reopens note panes in read mode.
  let editingId = $state<string | null>(null);

  // Distinct status values seen so far — feeds the note panel's status datalist,
  // so the picker is data-driven (no hardcoded status literal anywhere).
  let knownStatuses = $state<string[]>([]);

  // Does this machine have git? `null` until the first heartbeat answers — unknown is not
  // "no", so nothing is claimed before we know. Git is optional (see `scheduleCommit`);
  // this exists so its absence is *stated once* rather than swallowed every 5 seconds.
  let gitAvailable = $state<boolean | null>(null);
  let saidNoGit = false;

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let commitTimer: ReturnType<typeof setTimeout> | undefined;

  let theme = $state(document.documentElement.dataset.theme ?? 'dark');
  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('fm-theme', theme);
  }

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
  // The configured vaults, from `list_vaults` — authoritative, and `null` until the first
  // answer arrives, because unknown is not the same as none (the `gitAvailable`
  // discipline). `[]` is the first run and gates the whole app below.
  //
  // This used to be derived from the notes we happened to have *fetched*, which meant a
  // vault with nothing in it did not exist as far as the sidebar was concerned — so "empty
  // vault" and "no vault" looked identical. That is precisely the confusion the first-run
  // screen exists to end, and it must not inherit it: the vault you just made is the one
  // most likely to be empty.
  let vaults = $state<VaultInfo[] | null>(null);
  const allVaults = $derived((vaults ?? []).map((v) => v.name).sort());

  // Saved `.view` files (query + a renderer), authored in the vault. Each becomes a choice in
  // a pane's view picker; opening one adds/retargets a pane.
  let views = $state<ViewInfo[]>([]);
  const shown = (n: ObjectMeta) => !n.vault || !hiddenVaults.includes(n.vault);

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

  let paletteOpen = $state(false);
  let backupOpen = $state(false);
  let newVaultOpen = $state(false);
  let searchEl = $state<HTMLInputElement | undefined>(undefined);

  // Commands surfaced in the ⌘K palette (label + action). "Open …" adds a pane.
  let commands = $derived([
    { label: 'Open Board', run: () => addPane('board') },
    { label: 'Open Agenda', run: () => addPane('agenda') },
    { label: 'Open Timeline', run: () => addPane('timeline') },
    { label: 'Open Search', run: () => addPane('search') },
    ...views
      .filter((v) => !v.error)
      .map((v) => ({ label: `Open “${v.name}”`, run: () => addPane('view', { viewName: v.name }) })),
    { label: 'New note', run: onNew },
    { label: 'New board', run: onNewBoard },
    { label: 'New vault', run: () => (newVaultOpen = true) },
    { label: 'Toggle theme', run: toggleTheme },
    { label: 'Back up the vault', run: onBackup },
  ]);

  // Keyboard map: ⌘K palette · / focus global search · c new note · Esc close palette.
  function onGlobalKey(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      paletteOpen = !paletteOpen;
      return;
    }
    if (paletteOpen && e.key === 'Escape') {
      paletteOpen = false;
      return;
    }
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return; // don't hijack typing
    if (e.key === '/') {
      e.preventDefault();
      searchEl?.focus();
    } else if (e.key === 'c') {
      e.preventDefault();
      void onNew();
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

  // Fetch one feed by its key. Panes sharing a key share this result.
  async function loadFeed(key: string): Promise<Feed> {
    const sep = key.indexOf(':');
    const type = sep === -1 ? key : key.slice(0, sep);
    const arg = sep === -1 ? '' : key.slice(sep + 1);
    if (type === 'board') {
      const board = await getBoard(arg);
      learnStatuses(board.columns.flatMap((c) => c.cards));
      return { board };
    }
    if (type === 'agenda') {
      const cards = await getAgenda();
      learnStatuses(cards);
      return { cards };
    }
    if (type === 'timeline') {
      const cards = await recent();
      learnStatuses(cards);
      return { cards };
    }
    if (type === 'search') {
      const cards = arg ? await ipcSearch(arg) : [];
      learnStatuses(cards);
      return { cards };
    }
    if (type === 'view') {
      const r = await runView(arg);
      return r.board ? { board: r.board } : { cards: r.rows ?? [] };
    }
    return {};
  }

  async function refresh() {
    // Nothing to ask about, and asking anyway paints an error banner *behind* the
    // first-run screen — the app's first impression being a failure it caused itself.
    if (!vaults?.length) return;
    const keys = distinctFeeds(workspace.panes);
    try {
      const entries = await Promise.all(
        keys.map(async (k) => [k, await loadFeed(k).catch(() => ({}) as Feed)] as const),
      );
      feeds = Object.fromEntries(entries);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  // Reload when the set of distinct feeds the workspace needs changes (a pane added, closed,
  // regrouped, or its search edited). Fetches only the distinct feeds, so N panes over M
  // feeds cost M requests, not N.
  $effect(() => {
    void distinctFeeds(workspace.panes).join('|');
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
    const beat = async () => {
      const r = await ping().catch(() => null);
      if (r) {
        gitAvailable = r.git;
        // Once, not every beat. The notebook is fine; be accurate about what isn't.
        if (!r.git && !saidNoGit) {
          saidNoGit = true;
          notice =
            'git isn\'t installed — your notes are saved as files, but not versioned. ' +
            'Install git for history, backup and sharing.';
        }
      }
      if (r?.changed) await refresh();
    };
    // One beat unconditionally, in every backend: it is what answers `gitAvailable`, which
    // the first-run screen shows. Only the repeating timer is production-only — the mock
    // has no server to keep alive, and a 3s interval under Vitest is its own bug.
    void beat();
    if (!import.meta.env.PROD) return;
    const id = setInterval(() => void beat(), 3000);
    return () => clearInterval(id);
  });

  // Which vaults exist. Answered once at startup and again whenever one is created —
  // it changes about as often as you start a new project, so it does not ride the 3s beat.
  $effect(() => {
    void listVaults()
      .then((v) => (vaults = v))
      .catch(() => (vaults = []));
  });

  // The saved views the sidebar lists. Same cadence reasoning as vaults — a `.view` file
  // changes when you author one, not every few seconds.
  $effect(() => {
    void listViews()
      .then((v) => (views = v))
      .catch(() => (views = []));
  });

  // Open a note as a pane, deduped by its id: a note already in a pane is *focused*, never
  // opened a second time — two panes over one file would be two editors racing `updateBody`
  // and losing writes (the invariant the old trail's truncation protected). This is the one
  // way a note reaches the screen now: a card click, a followed `note:` chip, or a fresh note.
  function openNoteInPane(id: string, opts: { editing?: boolean } = {}) {
    if (opts.editing) editingId = id;
    const at = workspace.panes.findIndex((p) => p.kind === 'note' && p.noteId === id);
    if (at !== -1) {
      focused = at;
      return;
    }
    addPane('note', { noteId: id });
  }

  // "New note": create a blank note and open it straight in the editor (property
  // form + empty body), Obsidian/Notion style. Differentiate with tags, not type.
  async function onNew() {
    try {
      const meta = await capture('');
      openNoteInPane(meta.id, { editing: true });
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // A board is a note whose body is an Excalidraw scene, flagged `view: board`.
  // Create it empty, mark it, give it a title, then open it (the pane renders the
  // canvas for board notes — so "whiteboard in a pane" is free). No new command or Kind.
  async function onNewBoard() {
    try {
      const scene =
        '{"type":"excalidraw","version":2,"source":"formicaria","elements":[],"appState":{},"files":{}}';
      const meta = await capture(scene);
      await setProperty(meta.id, 'view', 'board');
      await setProperty(meta.id, 'title', 'Untitled board');
      openNoteInPane(meta.id);
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // Open an existing note (from a card click) — as a pane, deduped.
  function openNote(id: string) {
    openNoteInPane(id);
  }

  // The drag write-back: set the property THIS pane groups by to the target column's value.
  // `groupBy` is passed by the pane, not read from a global — so a drag in one board pane
  // writes its own property, never another pane's. (Within-column ordering is dropped in the
  // pane model for now; the card lands in the column, position by the server's sort.)
  async function onMove(groupBy: string, id: string, value: string, _beforeId: string | null) {
    try {
      await setProperty(id, groupBy, value);
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // Rotate a note's status from a card or the open note's header. Same write path
  // as a board drag, but always on `status` — a board may be grouped by anything.
  async function onSetStatus(id: string, value: string | null) {
    try {
      await setProperty(id, 'status', value ?? '');
      await refresh();
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // The top-bar global search: debounced, it opens (or retargets) a single search pane so a
  // quick search doesn't require adding a pane by hand first.
  function onSearchInput() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      const existing = workspace.panes.find((p) => p.kind === 'search');
      if (existing) changePane(existing.id, { query: searchQuery });
      else if (searchQuery.trim()) addPane('search', { query: searchQuery });
    }, 250);
  }

  // Debounced auto-commit after any successful write. Best-effort — a clean tree
  // is a no-op and a missing git binary must never block editing.
  // Git is a **capability, not a dependency**. Your notes are Markdown files and the
  // whole notebook — capture, board, agenda, search, edit — works with no git installed
  // at all. What git adds is history: local undo that outlives this session, and the
  // backup and collaboration built on it.
  //
  // So when there's no git, don't schedule: firing this every 5s at a binary that isn't
  // there, and swallowing the failure, is how a vault ends up quietly unversioned and you
  // find out on the day you needed the history.
  function scheduleCommit() {
    if (gitAvailable === false) return;
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

</script>

<svelte:window onkeydown={onGlobalKey} />

<!-- The gate. Three states, and the middle one is the point:
     null - we have not asked yet. Show NOTHING; unknown is not "no", and flashing an
            empty board or a first-run screen at someone who has ten vaults is a lie we
            would tell for 40ms.
     []   - the first run. The vault form IS the app: no rail, no views, no palette,
            because there is genuinely nothing else to do and offering it would be a menu
            of things that all fail.
     [..] - the app. -->
{#if vaults?.length === 0}
  <NewVault
    firstRun
    git={gitAvailable}
    oncreated={(v) => {
      vaults = v;
      void refresh();
    }}
  />
{:else if vaults}
<div class="app">
  <!-- The nav is a horizontal top bar now (it was a tall left rail that wasted vertical
       space). Everything the rail held lives here in one row; the workspace gets the full
       height and width below it. -->
  <header class="topbar">
    <span class="wordmark">formicaria</span>

    <label class="searchfield">
      <Icon name="search" size={15} />
      <input
        bind:this={searchEl}
        class="topbar-search"
        type="search"
        placeholder="Search…"
        bind:value={searchQuery}
        oninput={onSearchInput}
        spellcheck="false"
        aria-label="search notes"
      />
    </label>

    <button type="button" class="tb-btn" onclick={onNew} title="Create a note and open the editor">
      <Icon name="plus" size={15} /> New note
    </button>
    <button type="button" class="tb-btn ghost" onclick={onNewBoard} title="Create a whiteboard">
      <Icon name="pen" size={15} /> New board
    </button>

    <span class="tb-sep"></span>

    <!-- Open a view into a new pane. -->
    <div class="tb-add" role="group" aria-label="open a view">
      <span class="tb-add-label"><Icon name="plus" size={14} /> view</span>
      <button class="tb-chip" onclick={() => addPane('board')}>Board</button>
      <button class="tb-chip" onclick={() => addPane('agenda')}>Agenda</button>
      <button class="tb-chip" onclick={() => addPane('timeline')}>Timeline</button>
      <button class="tb-chip" onclick={() => addPane('search')}>Search</button>
      {#each views.filter((v) => !v.error) as v (v.name)}
        <button class="tb-chip saved" onclick={() => addPane('view', { viewName: v.name })} title="Saved view">{v.name}</button>
      {/each}
    </div>

    <label class="tb-cols" title="Workspace columns">
      cols
      <select value={workspace.cols} onchange={(e) => setCols(Number((e.currentTarget as HTMLSelectElement).value))}>
        {#each [1, 2, 3, 4] as n (n)}<option value={n}>{n}</option>{/each}
      </select>
    </label>

    <span class="tb-spacer"></span>

    {#if allVaults.length > 1}
      <div class="vaults" aria-label="vault filter">
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

    <button class="icon-btn" onclick={() => (paletteOpen = true)} aria-label="command palette" title="Command palette (Ctrl+K)">
      <Icon name="command" />
    </button>
    <button class="icon-btn" onclick={toggleTheme} aria-label="toggle theme" title="Toggle light/dark">
      <Icon name={theme === 'dark' ? 'sun' : 'moon'} />
    </button>
    <button class="tb-btn ghost" onclick={onBackup} title="Push your notes; optionally snapshot media">
      <Icon name="backup" size={15} /> Back up
    </button>
  </header>

  <div class="body">
    {#if error}<p class="banner error">{error}</p>{/if}
    {#if notice}
      <p class="banner notice">
        {notice}
        <button class="banner-dismiss" onclick={() => (notice = null)} aria-label="dismiss">✕</button>
      </p>
    {/if}

    <!-- The flexible workspace: a CSS grid of panes. `cols` sets the column count; each pane
         spans some columns; panes flow into rows. The renderers are pure and height:100%, so
         each drops into its cell unchanged. -->
    <div class="workspace" style="--cols:{workspace.cols}">
      {#each workspace.panes as pane, i (pane.id)}
        <div class="cell" style="grid-column: span {Math.min(pane.colSpan, workspace.cols)}; grid-row: span {pane.rowSpan};">
          <Pane
            {pane}
            index={i}
            cols={workspace.cols}
            feed={feedKey(pane) ? feeds[feedKey(pane) ?? ''] : undefined}
            statuses={knownStatuses}
            savedViews={views}
            {shown}
            focused={i === focused}
            startEditing={pane.kind === 'note' && pane.noteId === editingId}
            onopen={openNote}
            onmove={onMove}
            onstatus={onSetStatus}
            onnavigate={openNoteInPane}
            onsaved={scheduleCommit}
            onchange={(patch) => changePane(pane.id, patch)}
            onreorder={movePane}
            onresize={(patch) => resizePane(pane.id, patch)}
            onclose={() => closePane(pane.id)}
            onfocus={() => (focused = i)}
          />
        </div>
      {/each}
    </div>
  </div>

  {#if paletteOpen}
    {#await import('./lib/CommandPalette.svelte') then { default: CommandPalette }}
      <CommandPalette {commands} onclose={() => (paletteOpen = false)} />
    {/await}
  {/if}

  {#if backupOpen}
    {#await import('./lib/BackupPanel.svelte') then { default: BackupPanel }}
      <BackupPanel
        onclose={() => (backupOpen = false)}
        onnewvault={() => {
          backupOpen = false;
          newVaultOpen = true;
        }}
      />
    {/await}
  {/if}

  {#if newVaultOpen}
    <div class="sheet-backdrop" role="presentation" onclick={() => (newVaultOpen = false)}></div>
    <div class="sheet" role="dialog" aria-modal="true" aria-label="New vault">
      <NewVault
        git={gitAvailable}
        oncreated={(v) => {
          vaults = v;
          newVaultOpen = false;
          void refresh();
        }}
        oncancel={() => (newVaultOpen = false)}
      />
    </div>
  {/if}
</div>
{/if}

<style>
  /* The new-vault dialog. Genuinely modal — a form, not a peer view — so it keeps the
     fixed backdrop the note trail gave up. */
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 40;
  }
  .sheet {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 41;
    pointer-events: none;
  }
  .sheet > :global(*) {
    pointer-events: auto;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-3, 10px);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
    max-height: 90vh;
    overflow: auto;
  }

  /* ── Shell: a horizontal top bar over the workspace. ──
     A note is a pane now, so there is no separate trail column: one column, a header row
     over the workspace row. */
  .app {
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
  }
  .topbar {
    grid-row: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--header-h);
    padding: 0 var(--space-3);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    overflow-y: hidden;
  }
  .body {
    grid-row: 2;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  /* The flexible pane grid. `--cols` is the user's column count; panes flow into it and
     each spans some columns. min-height:0 on the grid and cells lets a pane scroll
     internally instead of the page. */
  .workspace {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(var(--cols, 2), minmax(0, 1fr));
    grid-auto-rows: minmax(8rem, 1fr);
    gap: var(--space-2);
    padding: var(--space-2);
    overflow: auto;
  }
  .cell {
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .cell > :global(.pane) {
    flex: 1;
    min-width: 0;
  }

  /* Top-bar controls. */
  .wordmark {
    font-weight: 700;
    font-size: var(--text-md);
    color: var(--text);
    white-space: nowrap;
    padding-right: var(--space-1);
  }
  .searchfield {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: 3px var(--space-2);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    color: var(--text-muted);
  }
  .topbar-search {
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    width: 9rem;
    outline: none;
  }
  .tb-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
    font: inherit;
    font-size: var(--text-sm);
    padding: 4px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-contrast);
    cursor: pointer;
  }
  .tb-btn.ghost {
    background: transparent;
    border-color: var(--border);
    color: var(--text);
  }
  .tb-sep {
    width: 1px;
    align-self: stretch;
    margin: 6px 2px;
    background: var(--border);
  }
  .tb-add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .tb-add-label {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-xs);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .tb-chip {
    font: inherit;
    font-size: var(--text-sm);
    white-space: nowrap;
    padding: 3px 9px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
  }
  .tb-chip:hover {
    border-color: var(--accent);
  }
  .tb-chip.saved {
    border-style: dashed;
    color: var(--text-muted);
  }
  .tb-cols {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-xs);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .tb-cols select {
    font: inherit;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 4px;
  }
  .tb-spacer {
    flex: 1;
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
  .banner-dismiss {
    margin-left: var(--space-2);
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
  }
  .banner-dismiss:hover {
    opacity: 1;
  }
</style>
