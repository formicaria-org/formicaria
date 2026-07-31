<script lang="ts">
  import { tick } from 'svelte';
  import Icon from './lib/Icon.svelte';
  import Pane from './lib/Pane.svelte';
  import {
    newPane,
    reidentify,
    distinctFeeds,
    feedKey,
    reorder,
    rendererKind,
    BUILTIN_PANES,
    MAX_PANES,
    type Workspace,
    type Layout,
    type Pane as PaneT,
    type PaneKind,
    type Feed,
  } from './lib/panes';
  import {
    getBoard,
    getAgenda,
    recent,
    proposals,
    conflicts,
    createDiscussion,
    discussions as fetchDiscussions,
    templates as fetchTemplates,
    getNote,
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
    resolveConflict,
    unrecorded,
    recordUnrecorded,
  } from './lib/ipc';
  import { setActivity, lastEditFor, contributors, authorKey } from './lib/activity.svelte';
  import { pullVault, syncFor, syncVault } from './lib/sync.svelte';
  import { conflictLabels } from './lib/conflictLabel';
  import { hashHue } from './lib/vaultColor';
  import { labelFor, setVaultLabels } from './lib/vaultLabels.svelte';
  import type { ObjectMeta, VaultInfo, ViewInfo, ConflictInfo, Unrecorded } from './lib/types';
  import NewVault from './lib/NewVault.svelte';
  import Pairing from './lib/Pairing.svelte';
  import Starting from './lib/Starting.svelte';
  import { isRemote } from './lib/remote';
  import * as ipc from './lib/ipc';
  import ViewBar from './lib/ViewBar.svelte';
  import * as keys from './lib/keys';

  // The flexible workspace: panes the user opens, arranges, and resizes. `feeds` holds the
  // fetched data keyed by feed (panes sharing a feed share one fetch). `focused` is the pane
  // keyboard/new-pane actions target.
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
    return { cols: 2, layout: 'auto', panes: [newPane('board')] };
  }
  let workspace = $state<Workspace>(loadWorkspace());
  let feeds = $state<Record<string, Feed>>({});
  // Which pane is showing in `single`, and the focus ring in `tiled`. Seeded from the
  // persisted workspace so reopening the app lands where you left it.
  let focused = $state(loadWorkspace().active ?? 0);
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
      localStorage.setItem('fm-workspace', JSON.stringify({ ...workspace, active: focused }));
    } catch {
      /* private mode — the workspace just won't persist this session */
    }
  }
  function addPane(kind: PaneKind, over: Partial<PaneT> = {}) {
    if (workspace.panes.length >= MAX_PANES) {
      notice = `That's the most panes at once (${MAX_PANES}). Close one to open another.`;
      return;
    }
    const panes = [...workspace.panes, newPane(kind, over)];
    workspace = { ...workspace, cols: autoCols(panes.length, workspace), panes };
    focused = workspace.panes.length - 1;
    persistWorkspace();
    void refresh();
  }
  function closePane(id: string) {
    const filtered = workspace.panes.filter((p) => p.id !== id);
    const panes = filtered.length ? filtered : [newPane('board')];
    // **The remaining views reclaim the space.** Growing on open without shrinking on close
    // left an empty column behind — the grid got wider and never got narrower, so closing two
    // of three notes left one note in a third of the screen.
    workspace = { ...workspace, cols: autoCols(panes.length, workspace), panes };
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
  // Changing arrangement is a view preference, like the theme — it never touches a vault.
  function setLayout(layout: Layout) {
    workspace = { ...workspace, layout };
    persistWorkspace();
  }

  /** The column count for `n` panes: tracks the pane count unless it has been pinned.
   *
   *  Capped at 4 — beyond that a pane is too narrow to read, and wrapping to a second row is
   *  the honest answer rather than eight slivers. */
  function autoCols(n: number, w: Workspace): number {
    if (w.colMode === 'fixed') return w.cols;
    return Math.max(1, Math.min(n, 4));
  }

  /** Pin the column count, or hand it back to `auto`. A view preference, like the layout. */
  function setCols(cols: number | 'auto') {
    workspace =
      cols === 'auto'
        ? { ...workspace, colMode: 'auto', cols: autoCols(workspace.panes.length, { ...workspace, colMode: 'auto' }) }
        : { ...workspace, colMode: 'fixed', cols: Math.max(1, Math.min(cols, 4)) };
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
  // The same discipline for restic. Unlike git it is never mentioned unprompted — a machine
  // without it is not missing anything until you try to add a vault from a backup — so this
  // exists only to decide whether that route is offered at all. `false` on every phone.
  let resticAvailable = $state<boolean | null>(null);
  let saidNoGit = false;
  // Same discipline for the other half: git present but *failing*. Said once per run, so a
  // vault that cannot commit is stated rather than swallowed — and stated rather than
  // repeated every five seconds.
  let saidCommitFailed = false;
  // The last set of unreadable notes we told the user about, joined. Compared rather than
  // counted so that "one conflict resolved, a different one appeared" is still announced.
  let lastSkipped = '';

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

  // The notes tagged `template`, surfaced in the palette as "New from …". Same cadence as views:
  // a template is authored (tagged) rarely, so this is fetched on load and when Settings (which
  // hosts the "New from …" actions) opens — not polled. Declared here, above `commands`, because
  // the palette reads it.
  let templates = $state<ObjectMeta[]>([]);
  function reloadTemplates() {
    void fetchTemplates()
      .then((t) => (templates = t))
      .catch(() => (templates = []));
  }

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
    // **Keyed by identity, not by name spelling.** The chips deduplicate one person by email; this
    // used to compare the author *name*, so hiding a chip hid only the spelling it was labelled
    // with — notes last edited under another spelling of the same person stayed visible while the
    // chip read "hidden". One vault here had 534 commits as `singhbal-baljinder` and 77 as
    // `Baljinder`, one email (2026-07-31).
    const last = lastEditFor(n.id);
    return !last || !hiddenAuthors.includes(authorKey(last));
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
  /// Takes the contributor **key** (email-based), not a display name — see `shown` above.
  function toggleAuthor(key: string) {
    hiddenAuthors = hiddenAuthors.includes(key)
      ? hiddenAuthors.filter((a) => a !== key)
      : [...hiddenAuthors, key];
    try {
      localStorage.setItem('fm-hidden-authors', JSON.stringify(hiddenAuthors));
    } catch {
      // Honoured this session even if it can't be remembered.
    }
  }
  // What the palette should be pre-filtered to when it opens. The toolbar's two "+" buttons
  // are the only callers; Ctrl+K clears it, because a shortcut that silently narrowed the
  // list would be a trap.
  let settingsSection = $state('');
  /// **There is no command palette.** It was a second menu that drifted from Settings — it
  /// carried preferences Settings also owned, it reopened stuck on whatever filter a "+" button
  /// had left behind, and it meant two places to look for one thing. Settings is the single
  /// surface now: preferences *and* the actions that used to live in the palette, under one
  /// gear. `section` scrolls it to a heading, so "+ New" still lands where it meant to.
  function openSettings(section = '') {
    settingsSection = section;
    settingsOpen = true;
    reloadTemplates(); // the "New from …" actions live here — a note tagged since load may be one
  }
  let settingsOpen = $state(false);
  let backupOpen = $state(false);
  let newVaultOpen = $state(false);
  // Kept as state rather than read off the last ping, because the panel has to survive
  // the beat that follows opening it — and because a note staying broken must not make
  // the list flicker.
  let skippedOpen = $state(false);
  /// The last-reported set of unopenable vaults, so the notice fires on change and not per beat.
  let lastUnopened = '';
  let skippedNotes = $state<import('./lib/ipc').SkippedNote[]>([]);
  /// This device has not been let in — see the `{#if}` at the top of the markup for why that
  /// replaces the whole app rather than showing beside it.
  let needsPairing = $state(false);
  let searchEl = $state<HTMLInputElement | undefined>(undefined);
  let createOpen = $state(false);
  let searchOpen = $state(false);

  /** Open the collapsed search and put the caret in it. */
  async function openSearch() {
    searchOpen = true;
    await tick(); // the input does not exist until the branch renders
    searchEl?.focus();
  }

  /// What the red plus offers: **three things you can make**, and nothing else.
  ///
  /// A note and a board are documents. A **window** is a place to look at them — and it opens on
  /// the board and is then *rotated* to whatever view you want, rather than being chosen from a
  /// list of seven at the moment of creation. That is the whole simplification: listing every
  /// view here made the menu grow with the app and asked you to decide before you could see
  /// anything, when changing your mind afterwards costs one scroll or one swipe.
  ///
  /// **New vault is not here** either. You make a vault a handful of times ever; it lives in
  /// Settings, which is where rare configuration belongs.
  const CREATE_MENU: { group: string; label: string; run: () => void }[] = [
    { group: 'Create', label: 'New note', run: onNew },
    { group: 'Create', label: 'New board', run: onNewBoard },
    { group: 'Create', label: 'New discussion', run: onNewDiscussion },
    { group: 'Create', label: 'New window', run: () => addPane('board') },
  ];

  // The `＋` menu, plus one "New from …" per template. Templates belong here — the `＋` is literally
  // "make something new", and making a note from a template is exactly that — not only buried in the
  // searchable palette. A template is a note you tagged `template`; this is its own group so it gets
  // a separator, and the list is refreshed each time the menu opens (see the `＋` button).
  const createItems = $derived([
    ...CREATE_MENU,
    ...(templates ?? []).map((t) => ({
      group: 'Template',
      label: `New from “${t.title || t.preview || 'Untitled'}”`,
      run: () => onNewFromTemplate(t.id),
    })),
  ]);

  // Commands surfaced in the ⌘K palette (label + action). "Open …" adds a pane.
  let commands = $derived([
    // **Grouped, and deliberately not a second Settings.** The palette used to carry
    // `Columns: 1..4` and a theme toggle — the same controls Settings owns — so the two drifted
    // into two half-menus saying different things. Anything that is a *preference* now lives in
    // Settings, and the palette's job is *actions*: open something, make something, do
    // something to a vault. `Settings` is here as the door to the other half, not a copy of it.
    // **The plus menu is spread in, not restated.** Its three items were written out a second
    // time here with different labels, which is exactly how the last palette drifted into a
    // second Settings that disagreed with the first.
    ...CREATE_MENU,
    // Only here: rare enough to be clutter in a menu reached dozens of times a day.
    { group: 'Create', label: 'New vault', run: () => (newVaultOpen = true) },
    // **Templates appear in the `＋` menu (discoverable) and here (searchable).** A template is a
    // note you tagged `template`; "New from …" spins a fresh note off its body. Kept in the palette
    // too so you can filter to one by name when there are many, the same as every saved view.
    ...(templates ?? []).map((t) => ({
      group: 'Create',
      label: `New from “${t.title || t.preview || 'Untitled'}”`,
      run: () => onNewFromTemplate(t.id),
    })),
    // **Every view, by name — and only in the palette.** The plus deliberately stops at "new
    // window" because a window is rotated after it opens, but the palette is the *searchable*
    // surface: typing "timeline" should land on a timeline without knowing that a window is the
    // thing that holds one. Opening a window already on the right view is strictly less work.
    ...BUILTIN_PANES.map((b) => ({
      group: 'Open',
      label: `Open ${b.label}`,
      run: () => addPane(b.kind),
    })),
    ...(views ?? [])
      .filter((v) => !v.error)
      .map((v) => ({
        group: 'Open',
        label: `Open “${v.name}”`,
        run: () => addPane('view', { viewName: v.name }),
      })),
    { group: 'Vault', label: 'Back up the vault', run: onBackup },
    { group: 'This view', label: 'Close this view', run: () => run('closePane') },
    { group: 'App', label: 'Settings', run: () => (settingsOpen = true) }
  ]);

  // Keyboard map: ⌘K palette · / focus global search · c new note · Esc close palette.
  // Every binding in force, defaults plus whatever Settings changed. Re-read when Settings
  // saves, so a rebind takes effect without a reload.
  let keymap = $state(keys.load());
  function reloadKeys() {
    keymap = keys.load();
  }

  /** Move the active pane by `step`, wrapping. The switcher the editor was missing. */
  function cyclePane(step: number) {
    const n = workspace.panes.length;
    if (n < 2) return;
    focused = (focused + step + n) % n;
    persistWorkspace();
  }

  function run(cmd: keys.Command) {
    switch (cmd) {
      case 'palette':
        openSettings();
        break;
      case 'nextPane':
        cyclePane(1);
        break;
      case 'prevPane':
        cyclePane(-1);
        break;
      case 'newNote':
        void onNew();
        break;
      case 'newView':
        openSettings('commands');
        break;
      case 'focusSearch':
        searchEl?.focus();
        break;
      case 'closePane':
        if (workspace.panes[focused]) closePane(workspace.panes[focused].id);
        break;
      case 'backup':
        backupOpen = true;
        break;
    }
  }

  function onGlobalKey(e: KeyboardEvent) {
    // **Typing wins, except for the commands that are not text.** A bare `c` must type a `c`,
    // so the editor is protected — but protecting it wholesale is why a note, once open, could
    // not be left without closing it: no navigation reached the handler at all. `WHILE_TYPING`
    // is the exemption, and every member of it requires a modifier, so none can eat a keystroke
    // someone meant to type.
    const tag = (e.target as HTMLElement | null)?.tagName;
    const typing = tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';

    for (const cmd of Object.keys(keymap) as keys.Command[]) {
      if (typing && !keys.WHILE_TYPING.has(cmd)) continue;
      if (!keys.matches(e, keymap[cmd])) continue;
      e.preventDefault();
      run(cmd);
      return;
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
    if (type === 'collaboration') {
      // Two things that need a person: proposals (branch + note) and notes still in conflict (both
      // versions marked in the body). Both are derived scans — no git read here. Conflicts render
      // above proposals as "needs resolution", so an unresolved merge is findable, not just a toast.
      const [cards, confl] = await Promise.all([proposals(), conflicts()]);
      learnStatuses(cards);
      return { cards, conflicts: confl };
    }
    if (type === 'discussions') {
      // The ongoing discussions, most-active-first, each with its participants. Its own feed field
      // because a discussion summary is richer than an ObjectMeta card.
      return { discussions: await fetchDiscussions() };
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
        const names = (await conflictLabels(s.conflicts)).join(', ');
        notice =
          `'${v}': ${s.conflicts.length} note(s) came back with conflicting edits — ` +
          `they still open, with both versions marked in the body. Open ${names} and merge the two.`;
      } else if (phase === 'failed') {
        report(syncFor(v).error ?? `could not pull '${v}'`);
      }
    }
  }
  $effect(() => {
    if (!import.meta.env.PROD) return; // network poll: production only (the mock has no remote)
    // **Not at t=0.** This shells out to `git ls-remote` *per vault*, so firing it as the app
    // opens puts a network round trip per vault against the first paint. It never blocked
    // rendering — `backup_status` drops the vault lock before the network, deliberately — but it
    // competes for CPU and IO at the one moment the user is waiting. Nobody needs to know within
    // three seconds that a collaborator pushed; they need their notes on screen.
    const first = setTimeout(() => void checkRemotes(), 3000);
    const id = setInterval(() => void checkRemotes(), 45000);
    const onFocus = () => void checkRemotes();
    window.addEventListener('focus', onFocus);
    return () => {
      clearTimeout(first);
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
    // **A hidden tab on a paired device stops beating.** The watchdog counts any authenticated
    // peer as "someone is using this" — it has to, or closing the desktop tab would kill the
    // server out from under someone working on the tablet. The other half of that bargain is
    // here: a tablet left face-up on a table must not hold the app open all day. The desktop
    // keeps beating regardless, because on the machine itself an open tab *is* the app being
    // open, which is exactly what `FM_AUTO_SHUTDOWN` means.
    const id = setInterval(() => {
      if (isRemote() && document.hidden) return;
      void alive();
    }, 15000);
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
    // A device that has not paired gets 401 on everything, so the whole app is replaced by the
    // pairing screen. Driven by a callback rather than polled: the very first command fails, and
    // waiting a beat to notice would show a broken app for that beat.
    ipc.onNeedsPairing(() => (needsPairing = true));

    // How far this client has caught up. Sent as `since`, replaced by whatever comes back —
    // including when nothing changed, so a client that was away does not re-run the same query
    // on every beat forever.
    let generation = 0;
    const beat = async () => {
      const r = await ping(generation).catch(() => null);
      if (r) {
        generation = r.generation;
        gitAvailable = r.git;
        resticAvailable = r.restic;
        // Once, not every beat. The notebook is fine; be accurate about what isn't.
        if (!r.git && !saidNoGit) {
          saidNoGit = true;
          notice =
            'git isn\'t installed — your notes are saved as files, but not versioned. ' +
            'Install git for history, backup and sharing.';
        }
      }
      // Notes the vault could not read — usually a conflicted merge. They are absent from
      // every view, so saying nothing means they have simply vanished as far as anyone
      // using the app can tell. This used to go to stderr, which in a browser is nowhere.
      //
      // Keyed on the set itself, so it is stated when it changes and not on every beat: a
      // conflict that persists must not become a notification every fifteen seconds, and a
      // *new* one must not be swallowed because an older one is already showing.
      if (r) {
        // A configured vault that would not open. Stated the same way and for the same reason as the
        // unreadable notes below: the app now opens the *others* instead of refusing everything, and
        // that is only an improvement if the missing one is named — a vault silently absent reads as
        // lost notes. Keyed on the set, so it is said when it changes rather than every 15 seconds.
        const vk = (r.unopened_vaults ?? []).join(String.fromCharCode(0));
        if (vk !== lastUnopened) {
          lastUnopened = vk;
          if (r.unopened_vaults?.length) {
            report(
              `${r.unopened_vaults.length} vault(s) could not be opened and their notes are not ` +
                `showing: ${r.unopened_vaults.join('; ')}. Everything else is unaffected.`,
            );
          }
        }
        skippedNotes = r.skipped;
        const key = r.skipped.map((s) => `${s.vault}/${s.name}`).join(String.fromCharCode(0));
        if (key !== lastSkipped) {
          lastSkipped = key;
          if (r.skipped.length) {
            // Names the count and points at the place, rather than pasting every
            // filename and its parser error into a banner that scrolls away. The
            // list -- and the one action available on it -- live in the panel.
            report(
              `${r.skipped.length} note(s) could not be read and are missing from every ` +
                `view — usually a conflicted merge. Open "Unreadable notes" to see ` +
                `which ones, and to fix them.`,
            );
          }
        }
      }
      if (r?.changed) await refresh();
    };
    // One beat unconditionally, in every backend: it is what answers `gitAvailable`, which
    // the first-run screen shows. Only the repeating timer is production-only — the mock
    // has no server, and a repeating interval under Vitest is its own bug.
    void beat();
    if (!import.meta.env.PROD) return;

    // Coming back to the tab beats immediately rather than waiting out the interval, so a tab
    // you just returned to is current before you have finished looking at it.
    //
    // This used to call `refresh()` **unconditionally**, because `changed` could not be
    // trusted: it was derived from whether this reindex found drift, and a reindex writes the
    // fresh mtimes back, so the first client to ask consumed the answer and the rest were told
    // "nothing changed" forever. `changed` is now `generation > since` — a comparison every
    // client can make independently — so asking is enough, and a quiet return costs one
    // heartbeat instead of a full re-query of every open view.
    const onVisible = () => {
      // Going *away* is the half that used to be ignored, and on a phone it is the half that
      // costs something: the app is usually left by being killed, so a commit owed "in five
      // seconds" is a commit that never happens. Spend the notice we get.
      if (document.hidden) {
        flushPendingCommit();
        return;
      }
      void beat();
    };
    document.addEventListener('visibilitychange', onVisible);
    // `pagehide` because a WebView teardown does not reliably fire `visibilitychange` first.
    window.addEventListener('pagehide', flushPendingCommit);

    const id = setInterval(() => {
      if (document.hidden) return;
      void beat();
    }, 15000);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisible);
      window.removeEventListener('pagehide', flushPendingCommit);
    };
  });

  // Which vaults exist. Answered once at startup and again whenever one is created —
  // it changes about as often as you start a new project, so it does not ride the 3s beat.
  //
  // **A refusal is not an empty vault list, and this used to conflate them.** `.catch(() => [])`
  // sent every failure to the first-run screen — offering to create your first vault to someone
  // who has ten, because one command did not answer. On the phone that is not hypothetical: the
  // shell answers `fm` only once it has opened the vaults, so a launch can legitimately refuse for
  // a moment ("still opening your vaults") and can refuse permanently if the open failed. So:
  // record the reason, keep `vaults` at `null` — which now renders `Starting`, not nothing — and
  // ask again.
  //
  // **The retry is not PROD-gated**, though the first draft of it was. A retry that runs only in
  // the shipped bundle is a retry no test can reach, on the one path that only ever fails on a
  // phone — the same shape of hole this whole fix exists to close. The mock answers immediately
  // unless a test asks it not to (`mock.faults`), so nothing repeats in the suite.
  // **Time-driven, not rejection-driven — because the symptom was a call that never settled.**
  // The first draft retried in `.catch`, which a promise that never resolves *or* rejects never
  // reaches: `bootError` stayed null, no retry was ever scheduled, and the phone showed a bare
  // "Opening your vaults…" forever. That is the exact geometry of the bug (Tauri's event loop, which
  // delivers the reply, does not start until the shell's setup hook returns — so an invoke issued
  // before that can simply never come back). A watchdog re-asks on elapsed time, so a silent
  // backend, a refusing one and a slow one all converge on the same recovering path.
  let bootError = $state<string | null>(null);
  let bootAttempts = $state(0);
  let bootWaited = $state(0);
  // The counters live in plain variables and are *assigned* into the `$state` ones, never
  // incremented through them: `bootAttempts += 1` reads what it writes, and this runs inside an
  // `$effect`, so that read makes the effect its own dependency — an infinite re-run
  // (`effect_update_depth_exceeded`, which took out 36 tests on the way in).
  let bootTries = 0;
  let bootTimer: ReturnType<typeof setInterval> | undefined;
  const BOOT_BEAT_MS = 1200;
  function loadVaults() {
    bootAttempts = ++bootTries;
    void listVaults()
      .then((v) => {
        vaults = v;
        // What each vault is *called*, for display only. Fed here because this is where the answer
        // lands; the rest of the app keeps passing names.
        setVaultLabels(v);
        bootError = null;
        // Nothing left to watch: a boot that succeeded must not keep polling `list_vaults`
        // behind a working app for the rest of the session.
        clearInterval(bootTimer);
        bootTimer = undefined;
      })
      .catch((e) => {
        bootError = String((e as { message?: string })?.message ?? e);
      });
  }
  $effect(() => {
    loadVaults();
    // One beat, doing both jobs: it re-asks (covering the never-settles case that no `.catch` can
    // see) and it counts elapsed time, which is what lets the screen escalate its wording instead
    // of saying "opening…" identically at 1 second and at 3 minutes.
    bootTimer = setInterval(() => {
      if (vaults) {
        clearInterval(bootTimer);
        bootTimer = undefined;
        return;
      }
      bootWaited += BOOT_BEAT_MS;
      loadVaults();
    }, BOOT_BEAT_MS);
    return () => clearInterval(bootTimer);
  });

  // The saved views the sidebar lists. Same cadence reasoning as vaults — a `.view` file
  // changes when you author one, not every few seconds.
  //
  // **Keyed on `vaults` so an early refusal is not permanent.** This fires at t≈0, concurrently
  // with `list_vaults`, and on the phone that is before the shell can answer anything — so a
  // `.catch(() => [])` here meant the user's saved views were missing from the sidebar and from
  // every pane's view picker for the whole session, from one badly-timed call. Reading `vaults`
  // makes the effect re-run once the backend is actually up.
  // What the app forgot it wrote. Same cadence as the saved views (and the same `vaults` key, so an
  // early refusal is not permanent) — plus after anything that could change it.
  $effect(() => {
    if (!vaults) return;
    void loadUnrecorded();
  });

  $effect(() => {
    if (!vaults) return;
    void listViews()
      .then((v) => (views = v))
      .catch(() => (views = []));
  });

  $effect(reloadTemplates);

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

  // "New discussion": a first-class discussion — a note that is the root of its own thread. It is
  // created through `create_discussion` (not capture + setProperty, because setProperty refuses the
  // `thread_of` self-anchor), given a default title, then opened. Being self-rooted it is an
  // `is_message` note, so it never lands on the board/timeline — it lives in the Discussions view.
  async function onNewDiscussion() {
    try {
      const meta = await createDiscussion('Untitled discussion', createTarget);
      openNoteInPane(meta.id);
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

  // "New from template": start a note pre-filled with a template's body. A template is just a
  // note tagged `template`, so this copies its **body** (the scaffold — headings, checklists,
  // callouts) into a fresh note and opens it in the editor. Only the body travels: the copy is
  // its own untitled note, not another template, so the `template` tag (and the rest of the
  // frontmatter) is deliberately left behind rather than cloned.
  async function onNewFromTemplate(id: string) {
    try {
      const tpl = await getNote(id);
      const meta = await capture(tpl?.body ?? '', createTarget);
      openNoteInPane(meta.id, { editing: true });
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
    commitPending = true;
    commitTimer = setTimeout(commitNow, 5000);
  }

  /// Whether a debounced commit is still owed. Tracked so leaving the app can settle it: the timer
  /// is a browser `setTimeout`, and on Android the ordinary way to leave is a process kill (Back
  /// exits, swipe-away and LMKD `SIGKILL`), so "in five seconds" can simply never arrive. Files are
  /// still safe — writes are atomic temp+rename — it is the *history* that would silently lag.
  let commitPending = false;
  function flushPendingCommit() {
    if (!commitPending) return;
    clearTimeout(commitTimer);
    commitNow();
  }

  function commitNow() {
    commitPending = false;
    {
      const stamp = new Date().toISOString();
      for (const v of vaults ?? []) {
        commit(`auto: ${stamp}`, v.name)
          .then((r) => {
            // **A commit that committed nothing is not automatically fine.** `commit_all`
            // refuses outright while the vault is mid-merge — correctly, since staging
            // conflict markers would publish them as content — but it does not *throw*, so
            // this used to fall through the `.catch` and say nothing at all. Every write
            // after the conflict is then saved to disk and never committed, for as long as
            // the conflict sits there. This is the path that matters: `git.rs::commit_all`
            // notes the auto-commit is debounced 5s after any write, "so a pull that
            // conflicts hits this within seconds — it is the default path, not an edge case".
            if (r?.committed || !r?.conflicts?.length || saidCommitFailed) return;
            saidCommitFailed = true;
            notice =
              `'${v.name}' has ${r.conflicts.length} note${r.conflicts.length === 1 ? '' : 's'} ` +
              `waiting on you: ${r.conflicts.join(', ')}. Nothing in this vault is being ` +
              `committed until they are settled — open each one, both versions are marked in ` +
              `the text.`;
          })
          .catch((e) => {
            if (saidCommitFailed) return;
            saidCommitFailed = true;
            notice =
              `Your notes are saved as files, but git could not record a change in '${v.name}': ${e}. ` +
              `History and backup are paused until that is fixed.`;
          });
      }
    }
  }

  /// Resolve a conflict by keeping a side — the only resolution that exists when there are no
  /// markers to edit (one device deleted the note, the other edited it).
  ///
  /// **This is the button whose absence froze a vault for a week.** Git's own answers are `add` and
  /// `rm`; the app offered neither, and every message told the user to open the note and keep the
  /// text they wanted — text that does not exist in this case. Refreshing afterwards matters as much
  /// as the resolution: the note reappears (or goes), and the vault starts committing again, so the
  /// surfaces that were stuck must be re-read rather than left showing the frozen state.
  async function onResolveConflict(c: ConflictInfo, keep: 'theirs' | 'mine' | 'edited') {
    try {
      await resolveConflict(c.vault, c.path, keep);
      await refresh();
      // Recording what was waiting is the point of unfreezing: the commit that had been refused for
      // as long as the conflict stood is now possible, so take it rather than waiting for the next
      // edit to trigger the debounce.
      scheduleCommit();
      await loadUnrecorded();
      notice =
        keep === 'edited'
          ? 'Resolved, and the merge is finished — this vault can commit again.'
          : `Resolved. Keeping the ${keep === 'theirs' ? "other device's" : "this device's"} version.`;
    } catch (e) {
      error = String(e);
    }
  }

  /// Notes on disk that git does not have — per vault, refreshed on load and after anything that
  /// could change it. See `ipc.unrecorded`: the app could forget it wrote a note and then never
  /// stage it, silently, forever.
  let unrecordedList = $state<Unrecorded[]>([]);
  async function loadUnrecorded() {
    unrecordedList = await unrecorded().catch(() => []);
  }
  const unrecordedTotal = $derived(unrecordedList.reduce((n, u) => n + u.count, 0));
  /// Record every vault's forgotten notes. One click, because the answer is never "some of them".
  ///
  /// **Reports once, over all of them.** The first version set `notice` inside the loop, so with two
  /// vaults the *last* one won: recording 146 notes in one vault and then finding nothing in the next
  /// announced "Nothing left to record in 'notes'" — telling the user their click did nothing while it
  /// had just committed 146 notes. A per-item message inside a loop over items is a report of the last
  /// item, not of the work.
  async function recordAllUnrecorded() {
    const done: { vault: string; notes: number }[] = [];
    try {
      for (const u of [...unrecordedList]) {
        const r = await recordUnrecorded(u.vault);
        if (r.committed && r.notes > 0) done.push({ vault: u.vault, notes: r.notes });
      }
    } catch (e) {
      error = String(e);
    }
    await loadUnrecorded();
    const total = done.reduce((n, d) => n + d.notes, 0);
    if (total === 0) {
      notice = 'Nothing left to record — git already has every note.';
    } else {
      const where = done.map((d) => `${d.notes} in “${d.vault}”`).join(', ');
      // Says the next step too: a commit is not a backup, and on a phone the vault may be the only
      // copy until it is pushed.
      notice =
        `Recorded ${total} note${total === 1 ? '' : 's'} that had never been committed (${where}). ` +
        `They are in this device's history now — back up to send them to a remote.`;
    }
  }

  // Backing up is a conversation, not a fire-and-forget: the panel owns setting
  // the remote, choosing whether media rides along, and reporting what actually
  // left the machine.
  function onBackup() {
    backupOpen = true;
  }

  let backupMenuOpen = $state(false);
  /** Non-null while a backup runs — doubles as the button's label and its disabled flag. */
  let savingLabel = $state<string | null>(null);

  /// **The default action: commit and push notes.**
  ///
  /// Every vault, because "back up" with several vaults open meaning only the first one is the
  /// bug this panel already had once. Assets ride along only where that vault's `vault.json` sets
  /// `git_assets_max` — a per-vault rule, so one device cannot decide what lands in shared
  /// history for everyone.
  async function backUpNotes() {
    if (savingLabel) return;
    savingLabel = 'Backing up…';
    error = null;
    const stamp = new Date().toISOString().slice(0, 16).replace('T', ' ');
    try {
      const results = await Promise.all(
        allVaults.map((v) => syncVault(v, `backup: ${stamp}`, () => refresh())),
      );
      const failed = allVaults.filter((_, i) => results[i] === 'failed');
      const conflicted = allVaults.filter((_, i) => results[i] === 'conflicts');
      // **A vault with no remote is not a failure**, and lumping it in with one made the whole
      // action look blocked. Each vault is pushed independently, so the ones that *can* back up
      // always do — but the old message could not say that, and reasonably read as "nothing went"
      // (reported 2026-07-31). Now the sentence leads with what left the machine.
      const local = allVaults.filter((_, i) => results[i] === 'local');
      const sent = allVaults.filter((_, i) => results[i] === 'synced');
      const parts: string[] = [];
      if (sent.length) {
        parts.push(
          allVaults.length === 1
            ? 'Notes backed up'
            : `Notes backed up (${sent.length} of ${allVaults.length} vaults)`,
        );
      }
      if (local.length) {
        // Stated as the fact it is, with the fix: no remote yet. Never "failed".
        parts.push(
          `${local.map((v) => `“${v}”`).join(', ')} ${local.length === 1 ? 'has' : 'have'} no ` +
            `remote yet — committed here, but nowhere to send. Add one in backup options.`,
        );
      }
      if (failed.length || conflicted.length) {
        // Named, and pointed at the panel that can actually resolve it — a toolbar button is the
        // wrong place to explain a merge conflict.
        parts.push(
          `${[...failed, ...conflicted].map((v) => `“${v}”`).join(', ')} need you: open backup ` +
            `options for detail.`,
        );
      }
      if (failed.length || conflicted.length) {
        report(parts.join(' '));
      } else {
        notice = parts.join(' ') || 'Nothing to back up';
      }
    } catch (e) {
      report(`Backup failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      savingLabel = null;
    }
  }

  /// The variants behind the chevron. Short on purpose: the button already does the common thing,
  /// and everything here is either rarer or needs a screen of its own to be honest about.
  let BACKUP_MENU = $derived([
    {
      label: 'Back up notes',
      note: 'commit and push — what the button does',
      run: backUpNotes,
    },
    {
      label: 'Get their changes',
      note: 'pull what others pushed',
      run: getTheirChanges,
    },
    {
      label: 'Backup options…',
      note: 'media, remotes, per-vault detail',
      run: onBackup,
    },
    {
      label: 'Attachment settings…',
      note: 'which assets travel with your notes',
      run: () => openSettings(),
    },
  ] as { label: string; note: string; run: () => void }[]);

</script>

<svelte:window onkeydown={onGlobalKey} />

<!-- The gate. Three states, and the middle one is the point:
     null - we have not asked yet, or the backend has not answered. `Starting` renders nothing
            for its first 700ms — unknown is not "no", and flashing a first-run screen at
            someone who has ten vaults is a lie we would tell for 40ms — and only then says so.
            **A `null` that never resolves used to render nothing forever**, which on a phone is
            an unreportable blank window (2026-07-31); it now shows the reason and a Retry.
     []   - the first run. The vault form IS the app: no rail, no views, no palette,
            because there is genuinely nothing else to do and offering it would be a menu
            of things that all fail.
     [..] - the app.

     Pairing comes **before** all of it: a device that has not been let in gets 401 on every
     command, so "no vaults" and "the first run" are not things it can distinguish — it would
     render the new-vault form at someone who has ten vaults they simply cannot see yet, and
     whose attempt to create an eleventh would also be refused. -->
{#if needsPairing}
  <Pairing />
{:else if vaults?.length === 0}
  <NewVault
    firstRun
    git={gitAvailable}
    restic={resticAvailable}
    oncreated={(v) => {
      vaults = v;
      void refresh();
    }}
  />
{:else if !vaults}
  <Starting
    error={bootError}
    attempts={bootAttempts}
    waitedMs={bootWaited}
    onretry={loadVaults}
  />
{:else}
<div class="app" data-layout={workspace.layout ?? 'auto'}>
  <!-- The nav is a horizontal top bar now (it was a tall left rail that wasted vertical
       space). Everything the rail held lives here in one row; the workspace gets the full
       height and width below it. -->
  <header class="topbar">
    <!-- **One plus, one gear, and a lens.** `New` and `View` were two buttons that ran the
         *same* line of code — `openSettings('commands')` — so the toolbar spent three controls
         and a wordmark saying one thing. The wordmark went too: the app does not need to tell
         you its name on every screen of its own window, and on a phone that space is the
         difference between the search field fitting and not.

         This is a real anchored menu, which the codebase previously avoided on the grounds that
         `.topbar` is a scroll container that would clip one. It is `position: fixed` and
         measured off the button, so no ancestor's overflow can clip it — and the alternative,
         routing every creation through a full-screen palette, is what made "new note" feel like
         a settings trip. -->
    <div class="create-wrap">
      <button
        type="button"
        class="plus-btn"
        onclick={() => {
          createOpen = !createOpen;
          if (createOpen) reloadTemplates(); // a note tagged since load may now be a template
        }}
        aria-expanded={createOpen}
        aria-haspopup="menu"
        title="Make something new (Ctrl+K)"
        aria-label="make something new">
        <Icon name="plus" size={18} />
      </button>
      {#if createOpen}
        <!-- Click-away on a backdrop rather than a document listener: it also blocks the stray
             tap that would otherwise land on whatever is behind the menu. -->
        <div class="menu-backdrop" role="presentation" onclick={() => (createOpen = false)}></div>
        <ul class="create-menu" role="menu">
          {#each createItems as item, i (item.label)}
            <!-- The rule falls where "make something" turns into "look at something", worked out
                 from the groups rather than flagged by hand — so it stays right when an item is
                 added on either side of it. -->
            {#if i > 0 && item.group !== createItems[i - 1].group}
              <li class="menu-sep" role="separator"></li>
            {/if}
            <li role="none">
              <button
                type="button"
                role="menuitem"
                onclick={() => ((createOpen = false), item.run())}>{item.label}</button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Collapsed to its lens until wanted. A search field is the widest thing in the bar and
         is used a fraction as often as it occupies space; open it and it takes the room it
         needs. `searchOpen` starts false on every load, deliberately — a bar that remembers
         being open is a bar that is usually open. -->
    {#if searchOpen}
      <label class="searchfield">
        <Icon name="search" size={15} />
        <input
          bind:this={searchEl}
          class="topbar-search"
          type="search"
          placeholder="Search…"
          bind:value={searchQuery}
          oninput={onSearchInput}
          onblur={() => { if (!searchQuery.trim()) searchOpen = false; }}
          spellcheck="false"
          aria-label="search notes"
        />
      </label>
    {:else}
      <!-- **Its own class, not `.icon-btn`.** Narrow layouts hide every `.icon-btn` in the top
           bar, because those controls also live in the bottom `ViewBar` where the thumb is.
           Search does not, so reusing that class would have made the search button disappear on
           exactly the screen this collapsing is for. -->
      <button
        type="button"
        class="search-btn"
        onclick={openSearch}
        aria-label="search notes"
        aria-expanded={false}
        title="Search">
        <Icon name="search" size={16} />
      </button>
    {/if}

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

    <span class="tb-spacer"></span>

    {#if allVaults.length > 1}
      <div class="vaults" aria-label="vault filter">
        <!-- Keyed and toggled on the vault **name** (the identity, and what `hiddenVaults` persists),
             labelled with what the repository is called — see `vaultLabels.svelte.ts`. -->
        {#each allVaults as v (v)}
          <button
            class="vault-chip"
            class:off={hiddenVaults.includes(v)}
            aria-pressed={!hiddenVaults.includes(v)}
            onclick={() => toggleVault(v)}
            title={hiddenVaults.includes(v) ? `Show ${labelFor(v)}` : `Hide ${labelFor(v)}`}
          >
            {labelFor(v)}
          </button>
        {/each}
      </div>
    {/if}

    {#if allContributors.length > 1}
      <!-- Contributor filter — the git-authorship twin of the vault filter. One click hides a
           person's notes everywhere; the coloured dot matches their "edited by" label. -->
      <div class="vaults" aria-label="contributor filter">
        {#each allContributors as who (who.key)}
          <button
            class="vault-chip contrib"
            class:off={hiddenAuthors.includes(who.key)}
            style="--ch:{hashHue(who.key)}"
            aria-pressed={!hiddenAuthors.includes(who.key)}
            onclick={() => toggleAuthor(who.key)}
            title={hiddenAuthors.includes(who.key)
              ? `Show ${who.label}'s notes`
              : `Hide ${who.label}'s notes`}
          >
            <span class="contrib-dot" aria-hidden="true"></span>{who.label}
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

    {#if skippedNotes.length}
      <!-- Notes that are on disk but absent from every view because they do not parse.
           A chip rather than only a banner: the banner is dismissible and this condition
           is not transient — it persists until a human resolves the file. -->
      <button
        class="tb-chip moved"
        onclick={() => (skippedOpen = true)}
        title="Notes that could not be read — usually a conflicted merge">
        {skippedNotes.length} unreadable
      </button>
    {/if}

    {#if unrecordedTotal}
      <!-- **Notes that exist and are not in history.** A chip for the same reason "unreadable" is
           one: this condition is not transient, and its whole failure mode was silence. `commit_all`
           stages only the paths the app remembers writing, and that memory dies with the process —
           so a note written before the last restart could never be staged by it, and nothing said
           so. Ninety-five had accumulated over a week before anyone noticed (2026-07-31). -->
      <button
        class="tb-chip moved"
        onclick={() => recordAllUnrecorded()}
        title="Notes on disk that git does not have yet — click to record them">
        {unrecordedTotal} not in history
      </button>
    {/if}

    <!-- **Back up is a split button**, because "save" had become a question nobody could answer
         from the screen. The wide half does the ordinary thing — commit and push **notes** — and
         the narrow half opens the variants. That keeps one obvious action at one click while the
         rarer choices stay reachable without a trip to Settings.
         Pushes are user-triggered rather than on a timer: a push is a visible act with a remote
         audience, and a cadence that fires on its own makes it one nobody chose. -->
    <div class="create-wrap">
      <button
        type="button"
        class="save-btn"
        onclick={backUpNotes}
        disabled={savingLabel !== null}
        title="Commit and push your notes"
        aria-label="back up notes">
        <Icon name="backup" size={14} />
        <span class="save-label">{savingLabel ?? 'Back up'}</span>
      </button>
      <button
        type="button"
        class="save-more"
        onclick={() => (backupMenuOpen = !backupMenuOpen)}
        aria-expanded={backupMenuOpen}
        aria-haspopup="menu"
        aria-label="other backup options"
        title="Other backup options">
        <Icon name="chevron-down" size={12} />
      </button>
      {#if backupMenuOpen}
        <div class="menu-backdrop" role="presentation" onclick={() => (backupMenuOpen = false)}></div>
        <ul class="create-menu right" role="menu">
          {#each BACKUP_MENU as item (item.label)}
            <li role="none">
              <button type="button" role="menuitem" onclick={() => ((backupMenuOpen = false), item.run())}>
                <span class="mi-label">{item.label}</span>
                <span class="mi-note">{item.note}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <button class="icon-btn" onclick={() => openSettings()} aria-label="settings" title="Settings — what this install is configured as">
      <Icon name="gear" size={16} />
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
        <div
          class="cell"
          class:active={i === focused}
          style="grid-column: span {Math.min(pane.colSpan, workspace.cols)}; grid-row: span {pane.rowSpan};">
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
            onresolve={onResolveConflict}
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

    <!-- Navigation for the single-pane arrangement. Always rendered, shown by CSS only when
         one pane is visible — the same no-conditional-component-tree discipline as the rest. -->
    <ViewBar
      onsettings={() => openSettings()}
      panes={workspace.panes}
      active={focused}
      onselect={(i) => {
        focused = i;
        persistWorkspace();
      }}
      onclose={closePane} />
  </div>

  {#if settingsOpen}
    {#await import('./lib/SettingsPanel.svelte') then { default: SettingsPanel }}
      <SettingsPanel
        layout={workspace.layout ?? 'auto'}
        onlayout={setLayout}
        onclose={() => (settingsOpen = false)}
        onkeyschanged={reloadKeys}
        {commands}
        section={settingsSection}
        columns={workspace.colMode === 'fixed' ? workspace.cols : 'auto'}
        oncolumns={setCols}
        {theme}
        ontheme={toggleTheme}
        onbackup={() => {
          settingsOpen = false;
          backupOpen = true;
        }} />
    {/await}
  {/if}

  {#if skippedOpen}
    {#await import('./lib/SkippedPanel.svelte') then { default: SkippedPanel }}
      <SkippedPanel skipped={skippedNotes} onclose={() => (skippedOpen = false)} />
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
        restic={resticAvailable}
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
    /* `dvh`, not `vh`: on a phone `100vh` is the tallest the viewport ever gets, so with a
       retracting URL bar or an on-screen keyboard the shell is taller than what you can see
       and the bottom of the app is unreachable. `dvh` tracks the *current* viewport. */
    height: 100dvh;
    overflow: hidden;
    background: var(--bg);
  }
  .topbar {
    grid-row: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--header-h);
    /* Pay back `viewport-fit=cover`. Without this the toolbar paints under the status bar and
       the clock sits on top of the search field — observed on a real device, invisible on the
       emulator until the screen was taller. `max()` so a desktop with no insets is unchanged. */
    padding: max(var(--safe-top), 0px) max(var(--safe-right), var(--space-3)) 0
      max(var(--safe-left), var(--space-3));
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    /* **Never `overflow-x: auto` here.** It looks like graceful degradation and behaves as
       hiding: on a phone the trailing controls — the command palette and Settings — scrolled
       off the right edge with nothing indicating the bar could scroll. That made Settings, the
       only diagnostic surface on a device whose logcat is suppressed, reachable solely by
       swiping a bar nobody would think to swipe; and it put "Back up" (i.e. push) out of reach
       entirely, since that is reached through the palette. Wrapping keeps every control on
       screen; the search field gives up its width first. */
    flex-wrap: wrap;
    row-gap: var(--space-1);
  }
  /* The search box is the one elastic item: it may shrink to nothing before any button is
     pushed to a second line, because a button you cannot see is a feature you do not have. */
  .topbar :global(.search) {
    flex: 1 1 6rem;
    min-width: 0;
  }
  /* ---------------------------------------------------------------------------------------
     Two arrangements, one shell.

     `single` shows one pane and lets a switcher move between them; `tiled` is the grid.
     `auto` asks the space. All of it is CSS keyed off `data-layout`, so there is **no
     viewport-tracking TypeScript** — the only new state is a preference string, exactly like
     the theme. That keeps the promise made when the phone CSS first landed: no new stateful
     layout, no phone-only component tree.

     Every pane stays mounted and fetched, so switching is instant and the feed layer is
     untouched. `display: none` rather than unmounting is the point.
     --------------------------------------------------------------------------------------- */
  [data-layout='single'] .cell:not(.active) {
    display: none;
  }
  /* **The inset floor, hung off a condition proven to match on the device.**
     `env(safe-area-inset-top)` is 0 in wry's Android WebView (Android 15 forces edge-to-edge
     for targetSdk 35, but the insets are never handed to the page), and a `pointer: coarse`
     floor did not take either — measured on the owner's phone: the CSS shipped, compiled
     correctly, and the toolbar still landed on the clock. The narrow-layout query *is*
     matching, because the single-pane bar appears. So the floor rides that instead of a
     capability query nothing here can confirm.
     `max()` so a platform that does report a real inset still wins. */
  [data-layout='single'] .topbar {
    padding-top: max(var(--safe-top), 1.75rem);
  }
  [data-layout='single'] :global(.viewbar) {
    display: flex;
  }
  /* Their home in `single` is the bottom bar, which has the room and the thumb reach. Leaving
     them here too would wrap the top bar onto a second row for controls that are already on
     screen — which is the cramping this replaced the `overflow-x` hiding with. */
  [data-layout='single'] .topbar .icon-btn {
    display: none;
  }
  [data-layout='single'] .topbar .save-label {
    display: none;
  }
  /* With one pane filling the screen there is nothing to drag it against, nothing to resize it
     relative to, and no ambiguity about which pane a close button means — so the container
     chrome goes and the content gets the room. Closing moved to the view bar. The pane's own
     *content* controls (the view rotator, group-by, search box) stay: those configure what you
     are looking at, not where it sits. */
  [data-layout='single'] :global(.grip),
  [data-layout='single'] :global(.pane-close),
  [data-layout='single'] :global(.resize-grip) {
    display: none;
  }
  [data-layout='single'] .workspace {
    grid-template-columns: 1fr;
    grid-auto-rows: 1fr;
  }
  /* The narrow default. 60rem, not the 40rem used elsewhere: two panes side by side need room
     for two *readable* columns, which runs out well before a phone's width. */
  @media (max-width: 60rem) {
    [data-layout='auto'] .cell:not(.active) {
      display: none;
    }
    /* Same floor, same reason — see the note on the `single` rule above. */
    [data-layout='auto'] .topbar {
      padding-top: max(var(--safe-top), 1.75rem);
    }
    [data-layout='auto'] :global(.viewbar) {
      display: flex;
    }
    [data-layout='auto'] .workspace {
      grid-template-columns: 1fr;
      grid-auto-rows: 1fr;
    }
    /* **The `single` twins of these live above, and both halves are required.**
       `auto` is the default, so a phone never matches a `[data-layout='single']` rule — writing
       only that half means the rule silently does nothing on the device it was written for.
       Caught by screenshotting the emulator: "＋ New" and "＋ View" still had their labels and
       the top bar still carried the utility buttons, because both hides were single-only. */
    [data-layout='auto'] .topbar .icon-btn {
      display: none;
    }
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
  /* The one create control. Round and filled so it reads as *the* action in the bar rather than
     one button among several, and sized to the 2.75rem touch target the phone work settled on. */
  .create-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .plus-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-contrast);
    cursor: pointer;
  }
  .plus-btn:hover {
    filter: brightness(1.08);
  }
  /* `fixed`, not `absolute`: `.topbar` scrolls horizontally, and an absolutely-positioned menu
     inside a scroll container is clipped by it. This is the constraint that kept the codebase
     on full-screen palettes; anchoring to the viewport is what lifts it. */
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .create-menu {
    position: fixed;
    top: calc(var(--header-h) + env(safe-area-inset-top, 0px) - 2px);
    left: var(--space-2);
    z-index: 41;
    min-width: 11rem;
    max-height: 70vh;
    overflow-y: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-2, 6px);
    box-shadow: var(--shadow-lg);
  }
  .create-menu button {
    display: block;
    width: 100%;
    text-align: left;
    /* The touch target both platform guidelines ask for. */
    min-height: 2.75rem;
    padding: var(--space-2);
    background: none;
    border: none;
    border-radius: var(--radius-2, 6px);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .create-menu button:hover {
    background: var(--surface-hover);
  }
  .menu-sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border);
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
  /* **One control, two halves.** They sit flush and share an outline so the pair reads as a
     single thing with a default action, which is the point of a split button: the wide half is
     what you almost always want, the narrow half admits there are alternatives. Separating them
     into two buttons would ask a question on every backup. */
  .save-btn,
  .save-more {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: inherit;
    font-size: var(--text-sm);
    padding: 5px 10px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
  }
  .save-btn {
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
    border-right-color: transparent;
    white-space: nowrap;
  }
  .save-more {
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    padding-inline: 6px;
    color: var(--text-muted);
  }
  .save-btn:hover:not(:disabled),
  .save-more:hover {
    background: var(--surface-hover);
  }
  .save-btn:disabled {
    cursor: default;
    color: var(--text-muted);
  }
  /* The menu hangs off the right-hand control, so it aligns to that edge rather than the
     viewport's left the way the create menu does. */
  .create-menu.right {
    left: auto;
    right: var(--space-2);
  }
  .create-menu .mi-label {
    display: block;
  }
  /* The second line is what makes the menu answerable without opening anything: "commit and
     push" says what backing up *is*, which was the actual question. */
  .create-menu .mi-note {
    display: block;
    font-size: var(--text-xs, 0.75rem);
    color: var(--text-muted);
  }
  .create-menu button {
    line-height: 1.3;
  }

  /* Styled like `.icon-btn` but exempt from the narrow-layout hide — see the markup. */
  .search-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 5px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }
  .search-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
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

  /* ── Narrow shell ─────────────────────────────────────────────────────────────
     A media-query reflow of components that already exist: no viewport-tracking
     TypeScript, no phone-only component tree, no second set of behaviours to keep
     in step. That promise still holds — the layout modes above are `data-layout`
     plus CSS, and the one new component (ViewBar) is always rendered.

     **Superseded in part (2026-07-19):** "one at a time is a column count" was the
     old answer, and it only *stacked* panes — you scrolled past whole views to
     reach the next. One-at-a-time is now a real arrangement (see the `data-layout`
     block above); what remains here is the shell reflow, which is genuinely about
     the window rather than about any pane.

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

    /* Measured on a device (2026-07-19, `sessions/2026-07-19-the-ui-on-android.md`): the
       toolbar wrapped to FOUR rows and the board began below the halfway mark of a 2400px
       screen. Everything below is about getting that back — the content is the product, the
       chrome is not. */

    .tb-chip {
      flex: 0 0 auto;
    }

    /* The workspace-columns control is **ignored** at this width — `.workspace` above is
       forced to `1fr`. It was still rendered, still said "2", and still did nothing: a
       control that lies about the state is worse than one that is absent. `:not(.tb-create)`
       because the vault selector shares the class and is genuinely useful here. */
    .tb-cols:not(.tb-create) {
      display: none;
    }

    /* The row that used to overflow — wordmark, search field, "＋ New", "＋ View" — is now a
       plus, a lens and a gear, so nothing here needs hiding to make it fit. What remains is the
       search field *once opened*: it is the only elastic thing in the bar, and 6rem still shows
       a legible placeholder while leaving room for the vault chips beside it. */
    .topbar-search {
      width: 6rem;
    }
    /* The icon says "back up" well enough at this width, and the word is the widest thing left
       in the bar. Written here *and* under `[data-layout='single']` below, because `auto` is the
       default and a phone matches only the media query — writing one half is silent. */
    .save-label {
      display: none;
    }
  }

  /* Finger-sized targets wherever the pointer is coarse — which is the honest test,
     not the viewport width: a tablet is wide and still has no mouse. 2.75rem is the
     ~44px both platform guidelines ask for; several of these were 3px of padding. */
  @media (pointer: coarse) {
    .search-btn,
    .tb-chip {
      min-height: 2.75rem;
      padding-inline: 0.75rem;
    }
    /* **Both axes, or it stops being a circle.** The rule above sets a min-height and horizontal
       padding, which is right for a pill-shaped chip and wrong for a round button: the width
       stayed 2rem while the height grew to 2.75rem, and the plus shipped as a visible ellipse.
       Caught by screenshotting the emulator, not by any test. */
    .plus-btn {
      width: 2.75rem;
      height: 2.75rem;
      padding: 0;
    }
  }
</style>
