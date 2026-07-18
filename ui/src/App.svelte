<script lang="ts">
  import Icon from './lib/Icon.svelte';
  import Pane from './lib/Pane.svelte';
  import {
    newPane,
    reidentify,
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
    alive,
    listVaults,
    listViews,
    runView,
    activity as fetchActivity,
    backupStatus,
  } from './lib/ipc';
  import { setActivity, lastEditFor, contributors } from './lib/activity.svelte';
  import { pullVault, syncFor } from './lib/sync.svelte';
  import { hashHue } from './lib/vaultColor';
  import type { ObjectMeta, VaultInfo, ViewInfo } from './lib/types';
  import NewVault from './lib/NewVault.svelte';

  // The flexible workspace: panes the user opens, arranges, and resizes. `feeds` holds the
  // fetched data keyed by feed (panes sharing a feed share one fetch). `focused` is the pane
  // keyboard/新-pane actions target.
  function loadWorkspace(): Workspace {
    try {
      const w = JSON.parse(localStorage.getItem('fm-workspace') ?? 'null');
      if (w && Array.isArray(w.panes) && w.panes.length && typeof w.cols === 'number') {
        // Re-mint pane ids: the counter resets each load, so ids persisted by an older session
        // can collide and crash the keyed {#each}. Fresh ids are always unique.
        return { ...w, panes: reidentify(w.panes) };
      }
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
  // Whether the current `error` is one `refresh()` raised (a feed that failed to load, which
  // the next successful refresh genuinely resolves) or one it must not touch — a failed
  // sync, a refused save. Those are about the user's data and stay until dismissed.
  let errorIsTransient = $state(true);
  function report(message: string) {
    error = message;
    errorIsTransient = false;
  }

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

  // Which vault a new note/board is created in. A view preference (persisted like the theme),
  // not a permission — it only picks where the file lands; the picker only shows with >1 vault.
  // Empty = the default vault. Asset drops are unaffected (they join the dropped-on note's vault).
  let newVaultTarget = $state<string>(
    (() => {
      try {
        return localStorage.getItem('fm-create-vault') ?? '';
      } catch {
        return '';
      }
    })(),
  );
  function setCreateVault(name: string) {
    newVaultTarget = name;
    try {
      localStorage.setItem('fm-create-vault', name);
    } catch {
      /* private mode — the choice just won't persist */
    }
  }

  // Distinct status values seen so far — feeds the note panel's status datalist,
  // so the picker is data-driven (no hardcoded status literal anywhere).
  let knownStatuses = $state<string[]>([]);

  // Does this machine have git? `null` until the first heartbeat answers — unknown is not
  // "no", so nothing is claimed before we know. Git is optional (see `scheduleCommit`);
  // this exists so its absence is *stated once* rather than swallowed every 5 seconds.
  let gitAvailable = $state<boolean | null>(null);
  let saidNoGit = false;
  // Same discipline for the other half: git present but *failing*. Said once per run, so a
  // vault that cannot commit is stated rather than swallowed — and stated rather than
  // repeated every five seconds.
  let saidCommitFailed = false;

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
  // The create destination, clamped to a vault that still exists (a removed/renamed one
  // falls back to the default rather than erroring on the next capture). Empty = default vault.
  const createTarget = $derived(allVaults.includes(newVaultTarget) ? newVaultTarget : '');
  const defaultVault = $derived((vaults ?? []).find((v) => v.default)?.name ?? allVaults[0] ?? '');

  // Saved `.view` files (query + a renderer), authored in the vault. Each becomes a choice in
  // a pane's view picker; opening one adds/retargets a pane.
  let views = $state<ViewInfo[]>([]);

  // Contributor filter — the git-authorship twin of the vault filter. `contributors()` (reactive,
  // from the activity module) drives the chips; `hiddenAuthors` is a HIDE list like `hiddenVaults`,
  // persisted the same way. A note whose last editor is hidden is filtered out; a note git knows
  // nothing about (no author) is always shown, so the filter never hides un-attributed notes.
  let hiddenAuthors = $state<string[]>(
    (() => {
      try {
        return JSON.parse(localStorage.getItem('fm-hidden-authors') ?? '[]');
      } catch {
        return [];
      }
    })(),
  );
  const allContributors = $derived(contributors());
  // Reads `hiddenVaults`, `hiddenAuthors` and `lastEditFor` — all reactive — so a pane's
  // `.filter(shown)` re-runs when any of them change (the reads happen inside the derived).
  const shown = (n: { id: string; vault: string }) => {
    if (n.vault && hiddenVaults.includes(n.vault)) return false;
    const author = lastEditFor(n.id)?.author;
    return !author || !hiddenAuthors.includes(author);
  };

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
  function toggleAuthor(name: string) {
    hiddenAuthors = hiddenAuthors.includes(name)
      ? hiddenAuthors.filter((a) => a !== name)
      : [...hiddenAuthors, name];
    try {
      localStorage.setItem('fm-hidden-authors', JSON.stringify(hiddenAuthors));
    } catch {
      // Honoured this session even if it can't be remembered.
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
    { label: 'Open Activity', run: () => addPane('activity') },
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
    // Git authorship, in parallel and non-blocking: it enriches (the "edited by" labels, the
    // activity stream, the contributor filter) but must never hold up the notes themselves, and
    // an old/absent git is not an error here.
    void fetchActivity()
      .then(setActivity)
      .catch(() => {});
    const keys = distinctFeeds(workspace.panes);
    try {
      const entries = await Promise.all(
        keys.map(async (k) => [k, await loadFeed(k).catch(() => ({}) as Feed)] as const),
      );
      feeds = Object.fromEntries(entries);
      // Clear only what *this* function put there. It used to clear unconditionally, which
      // meant a sync failure the user had not read yet was wiped by unrelated background
      // activity — and `refresh()` runs on every pane change and every `changed` beat, so
      // "unrelated" was most of the time. A message about losing work has to outlive a poll.
      if (errorIsTransient) error = null;
    } catch (e) {
      error = String(e);
      errorIsTransient = true;
    }
  }

  // Reload when the set of distinct feeds the workspace needs changes (a pane added, closed,
  // regrouped, or its search edited). Fetches only the distinct feeds, so N panes over M
  // feeds cost M requests, not N.
  $effect(() => {
    void distinctFeeds(workspace.panes).join('|');
    void refresh();
  });

  // Automatic "someone pushed" awareness. `backup_status` computes `remote_moved` per vault (a
  // no-write `ls-remote`), so this is a *network* poll — far slower than the 3 s heartbeat, and
  // only when the tab is visible. The chip it feeds turns the deferred "you only find out if you
  // open the backup panel" into a passive nudge; the pull uses the same command the panel does.
  let movedVaults = $state<string[]>([]);
  async function checkRemotes() {
    if (!vaults?.length || document.hidden) return;
    try {
      const st = await backupStatus();
      movedVaults = st.vaults.filter((v) => v.remote_moved).map((v) => v.name);
    } catch {
      /* offline / no remote — nothing to nudge about */
    }
  }
  // Bring their work down. Through `pullVault` rather than `pull` directly, so a merge that
  // genuinely disagrees comes back as *named notes* instead of a thrown string — the markers
  // are in the body, so those notes still open, and telling the user which ones is the whole
  // difference between a conflict and a mystery.
  //
  // Pull only. Nothing here publishes: pulling is a thing worth doing on a nudge, and
  // pushing is a thing worth doing on purpose.
  async function getTheirChanges() {
    const targets = movedVaults;
    movedVaults = [];
    for (const v of targets) {
      const phase = await pullVault(v, refresh);
      if (phase === 'conflicts') {
        const s = syncFor(v);
        notice =
          `'${v}': ${s.conflicts.length} note(s) came back with conflicting edits — ` +
          `they still open, with both versions marked in the body: ${s.conflicts.join(', ')}`;
      } else if (phase === 'failed') {
        report(syncFor(v).error ?? `could not pull '${v}'`);
      }
    }
  }
  $effect(() => {
    if (!import.meta.env.PROD) return; // network poll: production only (the mock has no remote)
    void checkRemotes();
    const id = setInterval(() => void checkRemotes(), 45000);
    const onFocus = () => void checkRemotes();
    window.addEventListener('focus', onFocus);
    return () => {
      clearInterval(id);
      window.removeEventListener('focus', onFocus);
    };
  });

  // Liveness. **One beat, one job** — it used to be one beat doing two, which is why it
  // had to fire every 3 seconds and why every heartbeat took the vault lock and reindexed.
  //
  // While this tab is open, tell the server someone is here so its auto-shutdown watchdog
  // does not quit. When the tab closes the beats stop and the server exits — closing the
  // tab closes the app, with no background process left over. `alive` reads nothing and
  // locks nothing, so it stays cheap enough to fire from a hidden tab, which is the point:
  // a backgrounded tab must keep the app alive without making it work.
  //
  // 15s against the server's 90s idle window. Browsers throttle a hidden tab's timers to
  // about once a minute, so the window is sized against the *throttled* rate — the old
  // 3s-beat/10s-window pairing killed the app out from under anyone who left it in a
  // background tab for a minute.
  $effect(() => {
    if (!import.meta.env.PROD) return; // the mock has no server to keep alive
    void alive();
    const id = setInterval(() => void alive(), 15000);
    return () => clearInterval(id);
  });

  // Has the vault moved under us? Separate beat, separate job.
  //
  // It has to exist at all because the views are served from SQLite, so an edit made by
  // anything else — a `git pull`, the merge driver, Vim — is otherwise invisible until the
  // app restarts. `ping` answers it, at the cost of an incremental reindex.
  //
  // **Only while the tab is visible.** Nobody is reading a hidden tab, so re-reading the
  // vault for it is pure waste (and on a battery device, waste that costs something). A
  // visible tab still notices an outside edit within ~15s.
  //
  // Deliberately *not* fully lifecycle-driven, which is where Track M's efficiency ruling
  // was heading: a visible-but-never-refocused window — formicaria tiled beside Vim, or on
  // a second monitor — fires no lifecycle event at all, and would simply stop updating. The
  // battery argument that motivates dropping the poll is a phone argument; on a desktop it
  // buys nothing and costs that.
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
    // has no server, and a repeating interval under Vitest is its own bug.
    void beat();
    if (!import.meta.env.PROD) return;

    // Coming back to the tab refreshes **unconditionally**, rather than asking `changed`
    // first. `ping` reindexes, and a reindex writes the new mtimes back — so with two tabs
    // open the first one to ask consumes the answer and the second is told "nothing
    // changed" forever. Gating the foreground refresh on that flag is how a tab you just
    // returned to shows you stale notes.
    const onVisible = () => {
      if (document.hidden) return;
      void refresh();
    };
    document.addEventListener('visibilitychange', onVisible);

    const id = setInterval(() => {
      if (document.hidden) return;
      void beat();
    }, 15000);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisible);
    };
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
      const meta = await capture('', createTarget);
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
      const meta = await capture(scene, createTarget);
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

  // Debounced auto-commit after any successful write. A clean tree is a no-op and a
  // missing git binary must never block editing.
  // Git is a **capability, not a dependency**. Your notes are Markdown files and the
  // whole notebook — capture, board, agenda, search, edit — works with no git installed
  // at all. What git adds is history: local undo that outlives this session, and the
  // backup and collaboration built on it.
  //
  // So when there's no git, don't schedule: firing this every 5s at a binary that isn't
  // there, and swallowing the failure, is how a vault ends up quietly unversioned and you
  // find out on the day you needed the history.
  //
  // **And when there IS git, say so when it fails.** This used to end in
  // `.catch(() => {})`, which is the same outcome by a different route: a vault that
  // stopped versioning — because a merge is half-finished, or the identity is missing —
  // looks exactly like one that is fine. Once per run, so a persistent failure does not
  // become a notification every five seconds.
  // **Every vault, not just the default.** This used to call `commit()` with no vault
  // argument, which means `list[0]` — so on a multi-vault install exactly one repo was
  // auto-committed and the others only ever got a commit when someone opened the backup
  // panel and pressed a button. A vault you write to daily and never back up by hand had
  // no history at all, which is the same failure as having no git, arrived at quietly.
  // A clean vault is a no-op, so committing all of them costs nothing.
  function scheduleCommit() {
    if (gitAvailable === false) return;
    clearTimeout(commitTimer);
    commitTimer = setTimeout(() => {
      const stamp = new Date().toISOString();
      for (const v of vaults ?? []) {
        commit(`auto: ${stamp}`, v.name).catch((e) => {
          if (saidCommitFailed) return;
          saidCommitFailed = true;
          notice =
            `Your notes are saved as files, but git could not record a change in '${v.name}': ${e}. ` +
            `History and backup are paused until that is fixed.`;
        });
      }
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

    {#if allVaults.length > 1}
      <!-- Where new notes/boards land. A destination, not a permission — it only picks the
           folder the file is written to. -->
      <label class="tb-cols tb-create" title="Create new notes in this vault">
        in
        <select
          value={createTarget || defaultVault}
          onchange={(e) => setCreateVault((e.currentTarget as HTMLSelectElement).value)}
          aria-label="create in vault"
        >
          {#each allVaults as v (v)}<option value={v}>{v}</option>{/each}
        </select>
      </label>
    {/if}

    <span class="tb-sep"></span>

    <!-- Open a view into a new pane. -->
    <div class="tb-add" role="group" aria-label="open a view">
      <span class="tb-add-label"><Icon name="plus" size={14} /> view</span>
      <button class="tb-chip" onclick={() => addPane('board')}>Board</button>
      <button class="tb-chip" onclick={() => addPane('agenda')}>Agenda</button>
      <button class="tb-chip" onclick={() => addPane('timeline')}>Timeline</button>
      <button class="tb-chip" onclick={() => addPane('search')}>Search</button>
      <button class="tb-chip" onclick={() => addPane('activity')} title="Who changed what, from git">Activity</button>
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

    {#if allContributors.length > 1}
      <!-- Contributor filter — the git-authorship twin of the vault filter. One click hides a
           person's notes everywhere; the coloured dot matches their "edited by" label. -->
      <div class="vaults" aria-label="contributor filter">
        {#each allContributors as who (who)}
          <button
            class="vault-chip contrib"
            class:off={hiddenAuthors.includes(who)}
            style="--ch:{hashHue(who)}"
            aria-pressed={!hiddenAuthors.includes(who)}
            onclick={() => toggleAuthor(who)}
            title={hiddenAuthors.includes(who) ? `Show ${who}'s notes` : `Hide ${who}'s notes`}
          >
            <span class="contrib-dot" aria-hidden="true"></span>{who}
          </button>
        {/each}
      </div>
    {/if}

    {#if movedVaults.length}
      <!-- Someone pushed work you don't have — a passive nudge with one-click pull. -->
      <button class="tb-chip moved" onclick={getTheirChanges} title="Someone pushed — get their changes">
        <Icon name="inbox" size={14} />
        {movedVaults.length === 1 ? movedVaults[0] : `${movedVaults.length} vaults`}: get changes
      </button>
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
    {#if error}
      <p class="banner error">
        {error}
        <button
          class="banner-dismiss"
          onclick={() => ((error = null), (errorIsTransient = true))}
          aria-label="dismiss">✕</button>
      </p>
    {/if}
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
            vaults={allVaults}
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
  /* The "someone pushed" nudge: filled with the accent so it reads as an invitation to act. */
  .tb-chip.moved {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
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
  /* The create destination reads at a glance: a touch larger, full-contrast text, and an
     accent-tinted box so it stands out as *where new things land* rather than a quiet setting. */
  .tb-create {
    font-size: var(--text-sm);
    color: var(--text);
  }
  .tb-create select {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text);
    background: var(--accent-subtle);
    border-color: var(--accent);
    padding: 2px 6px;
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
  /* Contributor chips carry the person's colour (a dot), matching their "edited by" labels. */
  .vault-chip.contrib {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }
  .contrib-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: hsl(var(--ch) 55% 48%);
    box-shadow: 0 0 0 1px hsl(var(--ch) 55% 32%);
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

  /* ── Phone ────────────────────────────────────────────────────────────────────
     A media-query reflow of the components that already exist, and deliberately
     nothing more: no new stateful layout, no phone-only component tree, no second
     set of behaviours to keep in step with the desktop's. The pane workspace is
     already a grid of independent panes, so "one at a time" is a column count.

     `--cols` is a *desktop* preference (the pane-count control writes it), so it is
     overridden rather than read here — a phone has no room to honour it, and a
     workspace saved on a laptop must not arrive on a phone as four 4rem columns.

     Verified at a narrow viewport and by `pointer: coarse`, NOT on a device — see
     known-issues. Chrome's touch emulation is actively misleading for the drag half
     of this, which is why the touch path is a real button with a real test rather
     than something only a phone can exercise. */
  @media (max-width: 40rem) {
    .workspace {
      grid-template-columns: 1fr;
      grid-auto-rows: minmax(60vh, auto);
      padding: var(--space-1);
      gap: var(--space-1);
      /* Panes stack, so the page scrolls vertically and never sideways. */
      overflow-x: hidden;
    }
    .topbar {
      flex-wrap: wrap;
      row-gap: 0.4rem;
      padding: 0.4rem 0.5rem;
    }
  }

  /* Finger-sized targets wherever the pointer is coarse — which is the honest test,
     not the viewport width: a tablet is wide and still has no mouse. 2.75rem is the
     ~44px both platform guidelines ask for; several of these were 3px of padding. */
  @media (pointer: coarse) {
    .tb-btn,
    .tb-chip {
      min-height: 2.75rem;
      padding-inline: 0.75rem;
    }
  }
</style>
