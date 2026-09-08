<script lang="ts">
  import { tick, untrack } from 'svelte';
  import Icon from './lib/Icon.svelte';
  import Pane from './lib/Pane.svelte';
  import {
    newPane,
    distinctFeeds,
    feedKey,
    reorder,
    rendererKind,
    autoCols,
    matchesTarget,
    migrateWorkspace,
    BUILTIN_PANES,
    OPENABLE_PANES,
    RAIL_PANES,
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
    readTheme,
    createPaper,
    runView,
    activity as fetchActivity,
    threadRoots,
    backupStatus,
    duplicates,
    pruneDuplicates,
    resolveConflict,
    unrecorded,
    demoted,
    keptNotes,
    lastCommits,
    recordUnrecorded,
  } from './lib/ipc';
  import { setActivity, lastEditFor } from './lib/activity.svelte';
  import { pullVault, syncFor, syncVault } from './lib/sync.svelte';
  import { conflictLabels } from './lib/conflictLabel';
  import { hashHue } from './lib/vaultColor';
  import { labelFor, setVaultLabels } from './lib/vaultLabels.svelte';
  import type {
    ObjectMeta,
    VaultInfo,
    ViewInfo,
    ConflictInfo,
    DuplicateFamily,
    Unrecorded,
  } from './lib/types';
  import NewVault from './lib/NewVault.svelte';
  import Welcome from './lib/Welcome.svelte';
  import * as appearance from './lib/appearance';
  import Pairing from './lib/Pairing.svelte';
  import Starting from './lib/Starting.svelte';
  import { isRemote } from './lib/remote';
  import { isPhone } from './lib/platform';
  import { pollIntervalMs, foregroundCheckDue } from './lib/remotePoll';
  import { quietLabel, quietTitle, quietVaults } from './lib/quietVaults';
  import * as ipc from './lib/ipc';
  import ViewBar from './lib/ViewBar.svelte';
  import * as keys from './lib/keys';

  // The flexible workspace: panes the user opens, arranges, and resizes. `feeds` holds the
  // fetched data keyed by feed (panes sharing a feed share one fetch). `focused` is the pane
  // keyboard/new-pane actions target.
  /// Read the persisted workspace **once**. The parsing, the id re-minting, the clamping and the
  /// layout migration all live in `migrateWorkspace` (pure, and therefore testable without
  /// rendering the whole app); this only supplies the string and reports whether anything moved.
  ///
  /// It used to be called *twice* — once for `workspace` and again for `focused` — which parsed
  /// `localStorage` twice and, worse, re-minted two independent sets of pane ids.
  function loadWorkspace(): { workspace: Workspace; migrated: boolean } {
    let raw: unknown = null;
    try {
      raw = JSON.parse(localStorage.getItem('fm-workspace') ?? 'null');
    } catch {
      /* fall through to the default */
    }
    return migrateWorkspace(raw);
  }
  const loaded = loadWorkspace();
  let workspace = $state<Workspace>(loaded.workspace);
  let feeds = $state<Record<string, Feed>>({});
  /// How many messages each note's discussion holds — the feed's reply badges. Read like
  /// `lastEditFor`: one fetch, a map, every row a lookup.
  let threadCounts = $state<Record<string, number>>({});
  // Which pane is showing in `single`, and the focus ring in `tiled`. Seeded from the
  // persisted workspace so reopening the app lands where you left it — already clamped into
  // range by `migrateWorkspace`, because an out-of-range active pane renders a blank app.
  let focused = $state(loaded.workspace.active ?? 0);
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
  // **Stamp a migrated record straight away.** Otherwise the one-time layout move re-runs on
  // every load, and a `tiled` chosen *after* the migration would be undone by the next one.
  if (loaded.migrated) persistWorkspace();
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
  /** **Reuse before you add.** With one view at a time as the default, tapping a view's name
   *  means "show me that" — appending a window answers a question nobody asked, and on a phone
   *  it also spends a feed and whatever scroll position the old one had.
   *
   *  This generalises two retargets the code had already grown by hand for exactly this reason:
   *  `openNoteInPane` never opened a note twice, and `onSearchInput` re-pointed the single search
   *  pane. `matchesTarget` owns what counts as "the same thing". See `decisions.md`, 2026-08-31 —
   *  it reverses the previous entry's "a window can no longer be re-pointed".
   *
   *  The `changed` guard is not an optimisation detail: `changePane` calls `refresh()`, so
   *  re-focusing an already-correct board would otherwise cost a fetch every time you tapped its
   *  name. `App.phone.test.ts` budgets exactly that. */
  function focusOrOpen(kind: PaneKind, over: Partial<PaneT> = {}) {
    const at = workspace.panes.findIndex((p) => matchesTarget(p, kind, over));
    if (at === -1) {
      addPane(kind, over);
      return;
    }
    const pane = workspace.panes[at];
    const changed = Object.entries(over).filter(([k, v]) => pane[k as keyof PaneT] !== v);
    focused = at;
    if (changed.length) changePane(pane.id, Object.fromEntries(changed));
    else persistWorkspace(); // mounted and already fetched — nothing to refresh
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

  /** Pin the column count, or hand it back to `auto`. A view preference, like the layout. */
  function setCols(cols: number | 'auto') {
    workspace =
      cols === 'auto'
        ? {
            ...workspace,
            colMode: 'auto',
            cols: autoCols(workspace.panes.length, { ...workspace, colMode: 'auto' }),
          }
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

  // ---- The user's own theme (`<vault>/themes/*.css`), and getting back out of one ----
  //
  // The file lives in the vault so it arrives on the other machine; *wearing* it is this device's
  // business, so the selection is `localStorage` like the dark/light choice above. A collaborator
  // who pulls your vault gets your theme and is not forced into it.
  /// **"Something in a vault changed."** The beat's `generation` is a function-local (it is sent as
  /// `since` and replaced by the reply), so nothing reactive can depend on it. This is bumped
  /// beside the `refresh()` it already triggers, which is what lets a theme edited on another
  /// machine — or in the other pane — land without a timer of its own.
  let vaultTick = $state(0);
  let userTheme = $state<appearance.Selection | null>(null);
  /// Set when a stored theme was refused at boot. Deliberately plain text in the top bar rather
  /// than a styled panel: the situation it reports is "the styling may be the problem".
  let themeRefused = $state('');
  /// The theme is applied but nobody has yet proved they can reach anything. While this is true the
  /// escape control is on screen; the first click, key, wheel or touch takes it away.
  let themeUnproven = $state(false);

  // **Decided before anything is applied.** If the last run put a theme on the page and the user
  // never managed to click or type, do not put it back — that is the one failure a person cannot
  // style their way out of, and closing and reopening the app is the gesture that fixes it on
  // Android, where there is no address bar, and on a phone, where there is no keyboard.
  {
    const stored = appearance.readSelection();
    if (stored && appearance.wasArmed()) {
      appearance.writeSelection(null);
      appearance.disarm();
      themeRefused =
        `The appearance “${stored.name}” was switched off. Last time it was on, nothing on ` +
        `screen was clicked or typed — which usually means it made the app unusable. ` +
        `You can turn it back on in Settings.`;
    } else {
      userTheme = stored;
    }
  }

  /// Which selection the boot guard is currently armed for. **Arming is once per theme, not once
  /// per run of the effect below** — the effect re-runs on every `vaultTick`, and `vaultTick` bumps
  /// whenever `ping` sees the vault move, *including the user's own writes*. So every edit used to
  /// re-arm the guard and put the escape button back on screen seconds after it was dismissed:
  /// "I continuously see the Turn off … in the bottom right", reported 2026-08-31. Each re-arm also
  /// added five more capture-phase document listeners without removing the previous set.
  let armedFor = $state<string | null>(null);

  /// Fetch and apply whatever is selected. Re-runs when the selection changes and when the
  /// generation moves, so an edit on another machine (or in the other pane) lands without a timer.
  /// **Re-applying the CSS on a tick is the feature; re-arming was the bug.**
  $effect(() => {
    const sel = userTheme;
    void vaultTick;
    if (!sel) {
      appearance.clear();
      appearance.disarm();
      themeUnproven = false;
      armedFor = null;
      return;
    }
    const key = `${sel.vault}\u0000${sel.name}`;
    void readTheme(sel.name, sel.vault)
      .then((css) => {
        // A theme that is already proven stays proven: the question the guard asks is "can anyone
        // reach anything with this theme on", and that was answered the first time.
        if (armedFor !== key) {
          armedFor = key;
          appearance.arm(() => (themeUnproven = false));
          themeUnproven = true;
        }
        appearance.apply(css);
      })
      .catch(() => {
        // The theme's vault is gone, or the file is. Say so once and fall back to the built-in
        // look rather than leaving someone on a blank screen wondering.
        appearance.clear();
        appearance.writeSelection(null);
        themeRefused = `The appearance “${sel.name}” is not in this vault any more.`;
        userTheme = null;
      });
  });

  function turnOffTheme() {
    appearance.writeSelection(null);
    userTheme = null;
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

  // **Has this person told git who they are?** Read from `list_vaults`, which already spawns git
  // locally for each vault's label — never from `backup_status`, which runs a network `ls-remote`
  // per vault and would make the first screen wait on the network (the 2026-07-17 ruling).
  //
  // Gated on the *default* vault, the one every fresh note lands in. Asking per vault would mean a
  // welcome screen that reappears whenever someone clones a second notebook, which is not what a
  // welcome is.
  //
  // `welcomeDone` is a per-browser dismissal: skipping has to stick, or "skip" means "ask me again
  // next launch". It is a convenience, not state — losing it re-asks a question that is cheap to
  // answer and skippable again, which is why `localStorage` is the right home for it.
  let welcomeDone = $state(true);
  const needsWelcome = $derived(
    !welcomeDone && gitAvailable === true && !!vaults?.length && !vaults[0].identity,
  );

  const allVaults = $derived((vaults ?? []).map((v) => v.name).sort());
  // The create destination, clamped to a vault that still exists (a removed/renamed one
  // falls back to the default rather than erroring on the next capture). Empty = default vault.
  const createTarget = $derived(allVaults.includes(newVaultTarget) ? newVaultTarget : '');
  /// **List order, not alphabetical.** `allVaults` is sorted for display, and falling back to
  /// `allVaults[0]` meant this disagreed with the backend, whose default is index 0 of the caller's
  /// *config* list (`infos`). Latent rather than live — `infos` always marks one — but two places
  /// deciding "which vault is default" by different rules is how they drift.
  const defaultVault = $derived(
    (vaults ?? []).find((v) => v.default)?.name ?? (vaults ?? [])[0]?.name ?? '',
  );

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

  /// **The contributor filter was removed on 2026-08-31.** It was a row of collaborator names in
  /// the chrome, the twin of the vault filter, and the owner asked for it gone from the desktop
  /// and the phone alike: it filtered something nobody filtered by, and on a small screen it was
  /// a third of the chrome. Nothing about *identity* changed — every "edited by" label, its
  /// colour hash, and the Activity stream are untouched, and the single `activity` fetch that fed
  /// all three keeps its other two consumers. `fm-hidden-authors` is simply no longer read, so a
  /// stale entry in someone's storage is inert. See `decisions.md`, 2026-08-31.
  ///
  /// Reads `hiddenVaults`, and is called from inside a `$derived`, so a pane's `.filter(shown)`
  /// re-runs when the filter changes.
  /// **Only names that still exist can hide anything.** A vault removed from the list leaves its
  /// name behind in `localStorage`, where it is inert — and counting it made the "N of M" label lie
  /// and kept a filter chip on screen for a vault nobody has. Derived, so the filter, the label and
  /// the trigger's own visibility can never disagree about what "hidden" means.
  const hidden = $derived(hiddenVaults.filter((n) => allVaults.includes(n)));
  const shown = (n: { id: string; vault: string }) => !(n.vault && hidden.includes(n.vault));

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
  let unrecordedOpen = $state(false);
  let demotedOpen = $state(false);
  /// The last-reported set of unopenable vaults, so the notice fires on change and not per beat.
  let lastUnopened = '';
  let skippedNotes = $state<import('./lib/ipc').SkippedNote[]>([]);
  /// This device has not been let in — see the `{#if}` at the top of the markup for why that
  /// replaces the whole app rather than showing beside it.
  let needsPairing = $state(false);
  let searchEl = $state<HTMLInputElement | undefined>(undefined);
  let createOpen = $state(false);
  let searchOpen = $state(false);
  let vaultMenuOpen = $state(false);
  /// **The filter states itself.** A row of chips said "something is hidden" only in colour, which
  /// is how a vault hidden weeks ago comes to read as notes that have gone missing. The trigger
  /// says how many of how many, so the answer to "where is that note?" is on the button.
  const vaultFilterLabel = $derived(
    hidden.length === 0
      ? 'All vaults'
      : `${allVaults.length - hidden.length} of ${allVaults.length} vaults`,
  );

  /** Open the collapsed search and put the caret in it.
   *
   *  **It expands the panel first, because otherwise this button makes search disappear.** In a
   *  collapsed rail `.app.panel-collapsed .topbar .searchfield` (four classes) hides the field,
   *  outranking `.search-slot.open .searchfield` (three) — while `.search-slot.open .search-btn`
   *  hides the lens you just pressed. So `searchOpen = true` on a collapsed panel hid *both*
   *  halves and the search was simply gone from the chrome. Widening the rail is also the honest
   *  answer to the gesture: you asked for a field, so give it somewhere to be. */
  async function openSearch() {
    if (!panelOpen) togglePanel();
    searchOpen = true;
    await tick(); // the input does not exist until the branch renders
    searchEl?.focus();
  }

  /// What the red plus offers: **things you can make**, and nothing else.
  ///
  /// This said "three things" and had said it since before `New discussion` was added; `New paper`
  /// made five. The number was never the rule — the rule is the sentence after it, and it still
  /// holds: a **document** belongs here, a *view* does not.
  ///
  /// A note and a board are documents. A **window** is a place to look at them — and it opens on
  /// the board and is then *rotated* to whatever view you want, rather than being chosen from a
  /// list of seven at the moment of creation. That is the whole simplification: listing every
  /// view here made the menu grow with the app and asked you to decide before you could see
  /// anything, when changing your mind afterwards costs one scroll or one swipe.
  ///
  /// **New vault is not here** either. You make a vault a handful of times ever; it lives in
  /// Settings, which is where rare configuration belongs.
  const CREATE_MENU: { group: string; label: string; run: () => void; command?: keys.Command }[] = [
    { group: 'Create', label: 'New note', run: onNew, command: 'newNote' as keys.Command },
    { group: 'Create', label: 'New board', run: onNewBoard },
    { group: 'Create', label: 'New discussion', run: onNewDiscussion },
    { group: 'Create', label: 'New paper', run: onNewPaper },
    // **The one deliberate `addPane`.** Everything else re-points an existing window; this is
    // where someone explicitly asks for another, and the way into a `tiled` arrangement.
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

  // Commands surfaced in the ⌘K palette (label + action). "Open …" shows that view.
  /// **Every view you can open, in one place.** The panel lists these down the left and the action
  /// list offers the same set as "Open …" — computed once so the two cannot say different things.
  /// Two half-menus disagreeing is exactly how the old command palette drifted into a second
  /// Settings, which is recorded as the reason it was removed.
  ///
  /// A saved view borrows the icon of the renderer it draws through, so it reads as the kind of
  /// thing it is rather than as an anonymous entry.
  const targetsFrom = (kinds: typeof BUILTIN_PANES) => [
    ...kinds.map((b) => ({
      key: b.kind as string,
      label: b.label,
      icon: b.icon,
      saved: false,
      // **Show it, do not stack it.** One edit covers the rail, the bar's Views menu and the
      // palette, because all three are built from this one list.
      run: () => focusOrOpen(b.kind),
    })),
    ...(views ?? [])
      .filter((v) => !v.error)
      .map((v) => ({
        key: `view:${v.name}`,
        label: v.name,
        icon:
          BUILTIN_PANES.find((b) => b.kind === rendererKind(v.renderer ?? 'timeline'))?.icon ??
          'timeline',
        saved: true,
        run: () => focusOrOpen('view', { viewName: v.name }),
      })),
  ];

  /// The rail: what you are invited to open, minus the rare one.
  let viewTargets = $derived(targetsFrom(RAIL_PANES));
  /// The palette: the same, plus the rare one. A filterable list can afford it; a column cannot.
  let paletteTargets = $derived(targetsFrom(OPENABLE_PANES));

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
    ...paletteTargets.map((t) => ({ group: 'Open', label: `Open ${t.label}`, run: t.run })),
    {
      group: 'Vault',
      label: 'Back up the vault',
      run: onBackup,
      command: 'backup' as keys.Command,
    },
    {
      group: 'This view',
      label: 'Close this view',
      run: () => run('closePane'),
      command: 'closePane' as keys.Command,
    },
    {
      group: 'App',
      label: 'Settings',
      run: () => (settingsOpen = true),
      command: 'palette' as keys.Command,
    },
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
    // **The one menu that needs Escape.** The others close on their next click, because a click is
    // the only thing you do in them. This one stays open across clicks by design, so without this
    // the only way out is a click on the backdrop — and a keyboard user has none.
    if (e.key === 'Escape' && vaultMenuOpen) {
      vaultMenuOpen = false;
      return;
    }
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
      // What the view leaves out travels with what it returned — a saved view draws through the
      // same renderer as the built-in it shadows, so the pane needs this to explain a column (or a
      // note) that a filter removed. Carried here rather than looked up in `views`, which is
      // fetched once per vault change and can arrive late or not at all.
      const view = { filters: r.filters ?? [], renderer: r.renderer, group_by: r.group_by };
      return r.board ? { board: r.board, view } : { cards: r.rows ?? [], view };
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
    // **One call for every post's reply count.** `thread()` is a whole-corpus read, so a count per
    // row would be thirty of those behind one mutex; `thread_roots` returns every root's count in
    // one pass and is already what the study agent watches. Fired here, beside `activity`, and for
    // the same reasons: it enriches the feed and must never hold up the notes.
    void threadRoots()
      .then((rs) => (threadCounts = Object.fromEntries(rs.map((r) => [r.id, r.count]))))
      .catch(() => {});
    const keys = distinctFeeds(workspace.panes);
    try {
      // **A feed that failed is not a feed that is empty.** This swallowed every error to `{}`, so
      // `cards` became `[]` and the timeline drew "No notes yet" — the app telling someone their
      // notebook was empty because a read had failed. There is no loading state either, so an
      // unresolved feed looked identical. Reported 2026-09-08, after a vault filter emptied every
      // view and the screen said exactly that.
      const failed: string[] = [];
      const entries = await Promise.all(
        keys.map(
          async (k) =>
            [
              k,
              await loadFeed(k).catch((e) => {
                failed.push(e instanceof Error ? e.message : String(e));
                return {} as Feed;
              }),
            ] as const,
        ),
      );
      feeds = Object.fromEntries(entries);
      // Clear only what *this* function put there. It used to clear unconditionally, which
      // meant a sync failure the user had not read yet was wiped by unrelated background
      // activity — and `refresh()` runs on every pane change and every `changed` beat, so
      // "unrelated" was most of the time. A message about losing work has to outlive a poll.
      if (errorIsTransient) error = null;
      // Left **transient**, unlike `report()`: the next successful refresh should clear it. A read
      // that failed once is not news worth outliving the poll that fixes it — but a screen that
      // says nothing at all is how "the backend is down" reads as "you have written nothing".
      if (failed.length) {
        error =
          failed.length === 1
            ? `Could not load this view: ${failed[0]}`
            : `Could not load ${failed.length} views: ${failed[0]}`;
      }
    } catch (e) {
      error = String(e);
      errorIsTransient = true;
    }
  }

  // Reload when the set of distinct feeds the workspace needs changes (a pane added, closed,
  // regrouped, or its search edited). Fetches only the distinct feeds, so N panes over M
  // feeds cost M requests, not N.
  //
  // **The key must be a `$derived`, not computed and discarded inside the effect.** It used to be
  // `void distinctFeeds(workspace.panes).join('|')` right here — which computes the key, throws it
  // away, and leaves the effect subscribed to the whole `workspace.panes` proxy. So it did not
  // fire "when the set of distinct feeds changes" as the comment claimed; it fired on *any* pane
  // mutation — focus, resize, reorder, a note pane opening — and each firing reloads every
  // distinct feed, and every feed query is a full-corpus `load_all()` on the server. On a phone,
  // where each of those parks the UI thread, that is a large share of the general lag.
  //
  // As a memo it does what it says: same key, no reload.
  //
  // **`untrack` is load-bearing and not decoration.** The memo alone achieved nothing, because
  // `refresh()` is called from inside this effect and its own first lines read
  // `distinctFeeds(workspace.panes)` synchronously — which re-subscribes the effect to the whole
  // proxy, exactly as before. The dependency has to be narrowed at *both* ends: `feedKeys` says
  // what this effect should watch, and `untrack` stops the work it triggers from widening that
  // again. Verified by an adversarial review that caught the memo being a no-op.
  //
  // **And the dependencies are then named explicitly, because `untrack` removes real ones too.**
  // `refresh()` returns early until `vaults` has loaded, and it reads `vaults` to do so — so the
  // effect had been depending on the vault list *by accident*, and that accident is what reloaded
  // the board once the list arrived. Untracking `refresh()` without saying so took the board out
  // with it (twelve tests, all "the board never rendered"). So both real triggers are listed here
  // where they can be read, and `untrack` keeps everything else `refresh()` happens to touch —
  // `errorIsTransient`, `feeds` — from silently becoming a trigger too.
  let feedKeys = $derived(distinctFeeds(workspace.panes).join('|'));
  $effect(() => {
    void feedKeys; // the set of feeds the workspace needs
    void vaults; // …and the vault list they are read from
    untrack(() => void refresh());
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
      // **Independent of the phase**: a pull can keep a note *and* still conflict on another, and
      // the keep is a decision the app made on the user's behalf. Said here as well as in the
      // Backup panel, because this chip is the other door onto the same operation.
      const kept = syncFor(v).kept;
      if (kept.length) {
        const names = (await conflictLabels(kept)).join(', ');
        notice =
          `'${v}': ${kept.length} note(s) came back — the other device had deleted ` +
          `${kept.length === 1 ? 'it' : 'them'} while this one was editing, and there was no text ` +
          `to merge. Kept rather than leaving this vault unable to record anything: ${names}. ` +
          `Delete ${kept.length === 1 ? 'it' : 'them'} again here if that is what you meant.`;
      }
    }
    // **A pull is the event that creates both of these, so it is the moment to re-read them.**
    // Until now `loadDemoted` ran at boot and from its own panel and nowhere else, so the chip for
    // a disagreement this very pull produced did not appear until the app restarted — a persistent
    // surface that was, in practice, less prompt than the banner it was meant to outlast. The same
    // 2026-08-24 lesson as the "not in history" count: the number is a fact about git, and this is
    // the moment git changed.
    await Promise.all([loadDemoted(), loadKept()]);
  }
  $effect(() => {
    if (!import.meta.env.PROD) return; // network poll: production only (the mock has no remote)
    // **`PROD` is not a statement about which device this is** — a Tauri build is `PROD` too, so
    // this ran on the phone: a per-vault `ls-remote` every 45 s *and* on every return to the app,
    // on mobile data, on a battery. `pollIntervalMs`/`foregroundCheckDue` carry the policy (and
    // are unit-tested there); this keeps only the wiring.
    const phone = isPhone();
    // **Not at t=0.** This shells out to `git ls-remote` *per vault*, so firing it as the app
    // opens puts a network round trip per vault against the first paint. It never blocked
    // rendering on the desktop — `backup_status` drops the vault lock before the network,
    // deliberately — but it competes for CPU and IO at the one moment the user is waiting. Nobody
    // needs to know within three seconds that a collaborator pushed; they need their notes on
    // screen.
    // On a phone the foreground *is* the schedule, rate-limited — Android fires this every time
    // the screen wakes or a shade is dismissed, which is not the same thing as "the user came
    // back to work". Seeded by the startup check below so the two never fire back to back.
    let lastCheck = 0;
    const first = setTimeout(() => {
      lastCheck = Date.now();
      void checkRemotes();
    }, 3000);
    const every = pollIntervalMs(phone);
    const id = every === null ? undefined : setInterval(() => void checkRemotes(), every);
    const onFocus = () => {
      const now = Date.now();
      if (!foregroundCheckDue(phone, now, lastCheck)) return;
      lastCheck = now;
      void checkRemotes();
    };
    window.addEventListener('focus', onFocus);
    return () => {
      clearTimeout(first);
      if (id !== undefined) clearInterval(id);
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

    // **Say goodbye on the way out**, so the server does not have to wait out its 90-second
    // window guessing whether this tab is still here. `pagehide` rather than `beforeunload`: the
    // latter is unreliable on mobile and blocks the bfcache. A reload fires this too, which is
    // fine — the new page registers on its first command, long before the short grace elapses.
    addEventListener('pagehide', () => ipc.sayGoodbye());

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
            "git isn't installed — your notes are saved as files, but not versioned. " +
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
      if (r?.changed) {
        vaultTick++;
        await refresh();
      }
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
  /// The welcome screen's dismissal, remembered per browser. Wrapped because every storage
  /// accessor can throw — a private window, cleared site data, a browser set to block it — and a
  /// welcome screen that crashes the app on the way past would be worse than no welcome screen.
  /// Failing to read means "already done": never trap someone on a screen we cannot dismiss.
  const WELCOME_KEY = 'fm-welcome-done';
  function dismissWelcome() {
    welcomeDone = true;
    try {
      localStorage.setItem(WELCOME_KEY, '1');
    } catch {
      /* nothing to do: the screen is dismissed for this session either way */
    }
  }
  $effect(() => {
    try {
      welcomeDone = localStorage.getItem(WELCOME_KEY) === '1';
    } catch {
      welcomeDone = true;
    }
  });

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
    void loadDuplicates();
    void loadDemoted();
    void loadKept();
    void loadLastSaves();
  });

  $effect(() => {
    if (!vaults) return;
    void listViews()
      .then((v) => (views = v))
      .catch(() => (views = []));
  });

  /// **Save the arrangement you are looking at, under a name.**
  ///
  /// This command existed, was labelled "New view", and opened the Settings list — a label promising
  /// a capability that did not exist. A `.view` could be neither written nor deleted from the app at
  /// all, so the only documented way to have one was to author YAML in a text editor, for a headline
  /// feature, in an app whose owner works only through the UI.
  ///
  /// Deliberately *not* a filter builder: the filter grammar is nine kinds of predicate, and a UI
  /// for it is a query builder nobody non-technical would use. What a person actually does is
  /// arrange a board or an agenda and want to keep it — so that is what this saves.
  // The save dialog's state. This was a `window.prompt()` until 2026-08-29 — genuinely in-app, but
  // not an affordance this app would otherwise ship, and it could ask exactly one question.
  /// **The chrome is a panel on a wide screen and a bottom bar on a narrow one** — the same
  /// controls, placed where the space and the hand are. Open by default; collapsing narrows it to
  /// its icons so the width goes back to the work. Per browser, like the theme and the layout,
  /// because it is a view preference and touches no vault (`decisions.md`, 2026-08-30).
  let panelOpen = $state(
    (() => {
      try {
        return localStorage.getItem('fm-panel') !== 'collapsed';
      } catch {
        return true;
      }
    })(),
  );
  function togglePanel() {
    panelOpen = !panelOpen;
    try {
      localStorage.setItem('fm-panel', panelOpen ? 'open' : 'collapsed');
    } catch {
      /* a private window still gets a working panel, just not a remembered one */
    }
  }

  /// **Where a dropdown actually goes.**
  ///
  /// These menus are `position: fixed`, and they have to be: the chrome scrolls, so a child
  /// positioned against it would be clipped. But their coordinates were *hardcoded* — `top:
  /// header-height; left: space-2` for the create menu, `right: space-2` for backup — which were
  /// the corners of a horizontal bar across the top of the window. The chrome is a column down the
  /// left now, or a bar along the bottom, so the backup menu opened in the opposite corner from its
  /// own button.
  ///
  /// Measured on open instead, which is what `StatusChip`'s picker already does. **Opens upward
  /// when the button is in the lower half** — anchoring the menu's bottom to the button's top,
  /// which needs no guess about how tall the menu is — and that is every menu opened from the foot
  /// of the panel or from the bottom bar. Nudged inward so a menu near an edge stays on screen.
  let menuAnchor = $state('');
  function anchorTo(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const left = Math.max(8, Math.min(r.left, window.innerWidth - 11 * 16 - 8));
    menuAnchor =
      r.bottom > window.innerHeight / 2
        ? `left:${Math.round(left)}px;bottom:${Math.round(window.innerHeight - r.top + 4)}px;top:auto;`
        : `left:${Math.round(left)}px;top:${Math.round(r.bottom + 4)}px;`;
  }

  let viewsOpen = $state(false);
  let helpOpen = $state(false);
  /// **Saving, renaming and deleting a view left the UI on 2026-08-31.** The dialog, its state and
  /// the three functions that drove it are gone: *"views are basically fixed for now and view
  /// customization will need its own design plan."* `saveView`/`renameView`/`deleteView` remain in
  /// `ipc.ts` and in `fm-app`, and `listViews`/`runView` below are untouched — a `.view` file still
  /// lists in the rail and still opens. See `decisions.md`, 2026-08-31.

  $effect(reloadTemplates);

  // Open a note as a pane, deduped by its id: a note already in a pane is *focused*, never
  // opened a second time — two panes over one file would be two editors racing `updateBody`
  // and losing writes (the invariant the old trail's truncation protected). This is the one
  // way a note reaches the screen now: a card click, a followed `note:` chip, or a fresh note.
  function openNoteInPane(id: string, opts: { editing?: boolean } = {}) {
    if (opts.editing) editingId = id;
    // The hand-written version of `focusOrOpen`, which is now the shared one. Still load-bearing
    // rather than merely tidy: two editors open on one note race each other through
    // `update_body`, so a note must never be opened twice.
    focusOrOpen('note', { noteId: id });
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
  // **A paper is made from something you already have.** The one thing a researcher always has is
  // the citation — a BibTeX entry off the publisher page, a DOI, an arXiv link — so the dialog
  // takes any of them, or a bare title, in one box rather than asking which kind it is.
  //
  // Nothing here reaches the network: the core links no HTTP client (`fm-agent-run`'s ruling), so
  // an identifier is *recognised* and recorded, never resolved. A pasted BibTeX entry is what
  // carries a full record offline.
  let paperOpen = $state(false);
  let paperInput = $state('');

  function onNewPaper() {
    paperInput = '';
    paperOpen = true;
  }
  async function confirmNewPaper() {
    // Nothing typed is not a paper. Without this, submitting an empty box `put`s a titleless,
    // bodyless note tagged `paper`, which then shows up in the Papers view as a blank card.
    if (!paperInput.trim()) return;
    try {
      const meta = await createPaper(paperInput, createTarget);
      paperOpen = false;
      openNoteInPane(meta.id);
      scheduleCommit();
    } catch (err) {
      error = String(err);
    }
  }

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
      // **And it becomes the visible one.** The old version re-pointed the search pane but never
      // moved `focused`, which was invisible while every pane was on screen and a dead keypress
      // the moment one view at a time became the default: a second query changed nothing you
      // could see. `focusOrOpen` cannot express that bug — focusing is what it does.
      if (searchQuery.trim() || workspace.panes.some((p) => p.kind === 'search')) {
        focusOrOpen('search', { query: searchQuery });
      }
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
  /// How long after the last write to commit — and, second, the longest a commit may ever be deferred.
  const COMMIT_QUIET_MS = 5000;
  const COMMIT_MAX_WAIT_MS = 15000;
  /// When the oldest pending write happened. `0` = nothing pending.
  let commitSince = 0;

  function scheduleCommit() {
    if (gitAvailable === false) return;
    clearTimeout(commitTimer);
    commitPending = true;
    // **A debounce with a ceiling.** Plain `clearTimeout` + 5 s means every write pushes the deadline
    // out, so a *burst* of writes defers the commit indefinitely — it does not fire late, it does not
    // fire at all. That is not hypothetical: 142 notes were written in one minute on the owner's phone
    // (2026-07-31), each one resetting this timer, and when the process ended the pending commit died
    // with the tab and the in-memory write-record died with the process. The notes were then
    // unstageable for good.
    //
    // So the quiet period still applies, but never past `COMMIT_MAX_WAIT_MS` after the *first* pending
    // write. Under a burst that turns "never" into "every 15 seconds", which is what makes a burst
    // survivable at all.
    if (commitSince === 0) commitSince = Date.now();
    const remaining = Math.max(0, commitSince + COMMIT_MAX_WAIT_MS - Date.now());
    commitTimer = setTimeout(commitNow, Math.min(COMMIT_QUIET_MS, remaining));
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
    commitSince = 0;
    {
      const stamp = new Date().toISOString();
      const runs: Promise<unknown>[] = [];
      for (const v of vaults ?? []) {
        const run = commit(`auto: ${stamp}`, v.name)
          .then(async (r) => {
            // **A conflicted note is announced whether or not the commit succeeded.** This used
            // to fire only on `!r.committed`, because `commit_all` refused outright while the
            // vault was mid-merge and said so by committing nothing at all. Since 2026-09-07 it
            // commits everything *except* the conflicted paths (`decisions.md`, *a conflict
            // blocks its own notes and nothing else*), so the usual outcome is a successful
            // commit with notes still stuck — and a message conditioned on failure would be
            // silent in exactly that case, which is how two notes went unnoticed for 39 days.
            if (!r?.conflicts?.length || saidCommitFailed) return;
            saidCommitFailed = true;
            const n = r.conflicts.length;
            const names = (await conflictLabels(r.conflicts)).join(', ');
            notice =
              `'${v.name}': ${n} note${n === 1 ? '' : 's'} ${n === 1 ? 'is' : 'are'} waiting on ` +
              `you — ${names}. ${n === 1 ? 'It has' : 'They have'} both versions in the text and ` +
              `git will not record ${n === 1 ? 'it' : 'them'} until you pick one. Everything ` +
              `else in this vault is being committed as usual.`;
          })
          .catch((e) => {
            if (saidCommitFailed) return;
            saidCommitFailed = true;
            notice =
              `Your notes are saved as files, but git could not record a change in '${v.name}': ${e}. ` +
              `History and backup are paused until that is fixed.`;
          });
        runs.push(run);
      }
      // The same reason as the backup path: this commit is exactly what moves a note *into*
      // history, so the count of what is outside it is stale the moment this settles.
      void Promise.allSettled(runs).then(loadUnrecorded);
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
  /// Identical-note families, loaded beside the outstanding list because the panel shows both and the
  /// order between them matters (record, then prune).
  let duplicateList = $state<DuplicateFamily[]>([]);
  async function loadDuplicates() {
    duplicateList = await duplicates().catch(() => []);
  }
  async function onPruneDuplicates(vault: string) {
    try {
      const r = await pruneDuplicates(vault);
      notice = `Removed ${r.removed} extra cop${r.removed === 1 ? 'y' : 'ies'} across ${r.kept} note${r.kept === 1 ? '' : 's'} in “${labelFor(vault)}”. They stay in git history, so this can be undone.`;
      await Promise.all([loadDuplicates(), loadUnrecorded()]);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function loadUnrecorded() {
    unrecordedList = await unrecorded().catch(() => []);
  }

  /// Fields the two devices set differently, where the merge kept both.
  ///
  /// **The condition the demotion ruling attaches to itself** (`decisions.md`, 2026-09-07): the
  /// loser is only *not* resolution by fiat for as long as it is visible and one tap from winning.
  /// Loaded on the same cadence as the outstanding list — it is a scan, not a per-render read — and
  /// stateless, so it is right after a restart and on the other device.
  let demotedList = $state<import('./lib/ipc').DemotedField[]>([]);
  async function loadDemoted() {
    demotedList = await demoted().catch(() => []);
  }
  /// Counted by **note**, not by row: two diverged fields on one note is one thing to look at, and a
  /// chip that says "2" for one note reads as two notes.
  const demotedNotes = $derived(new Set(demotedList.map((d) => d.id)).size);

  /// **Notes a merge brought back after the other device deleted them.**
  ///
  /// The second half of the same category as `demotedList`, and it shares the chip with it: both
  /// mean *the two devices disagreed, the merge chose, nothing is blocked*. A separate chip would
  /// have been the sixth in this toolbar, and the 2026-07-19 device note below records what six
  /// costs on a phone — a bar wrapped to four rows with the board starting halfway down the screen.
  ///
  /// Stateless in the same way, and that is the part `outstanding.md` §2.12 got wrong when it said
  /// a chip here "needs its own state": the resurrection is recorded in the merge commit that
  /// caused it, so this is a read of history exactly as *not in history* is a read of `git status`.
  let keptList = $state<import('./lib/ipc').KeptNote[]>([]);
  async function loadKept() {
    keptList = await keptNotes().catch(() => []);
  }
  /// Counted by note across both halves, because the chip opens one panel: two diverged fields and
  /// a resurrection on the same note is one thing to go and look at.
  const settledNotes = $derived(
    new Set([...demotedList.map((d) => d.id), ...keptList.map((k) => k.id ?? k.path)]).size,
  );

  /// **How long each vault has been quiet** — the one alert that reports an absence.
  ///
  /// Every other chip here names something it found: a note that will not parse, a note git does
  /// not have, a field two devices disagree about. Each of those depends on a detector working.
  /// This one depends on nothing: it asks git when the vault last saved anything and says so, which
  /// is why it is the one that would have caught the thirty-nine-day freeze — where every specific
  /// detector was either missing or blocked by the very conflict it existed to report.
  ///
  /// Local and cheap (`git log -1` per vault), so unlike `movedVaults` it is **not** behind the
  /// production-only network poll: the device that most needs to be told its notes have not gone
  /// anywhere is a phone that is often offline, and this must be right at the first paint.
  let lastSaves = $state<import('./lib/ipc').LastCommit[]>([]);
  async function loadLastSaves() {
    lastSaves = await lastCommits().catch(() => []);
  }
  /// The ages are derived from the instants rather than sent as numbers, so the backend never has
  /// to have an opinion about "now" — and so a reload is all it takes to be current. `Date.now()`
  /// is not a reactive dependency, so this is as fresh as `lastSaves` is: the vault-list cadence,
  /// plus after every backup. At a threshold measured in weeks that is ample, and the alternative
  /// — a ticking clock behind a chip — is a re-render every second to change nothing.
  const quiet = $derived(quietVaults(lastSaves, Date.now()));
  const unrecordedTotal = $derived(unrecordedList.reduce((n, u) => n + u.count, 0));
  /// Record every vault's forgotten notes. One click, because the answer is never "some of them".
  ///
  /// **Reports once, over all of them.** The first version set `notice` inside the loop, so with two
  /// vaults the *last* one won: recording 146 notes in one vault and then finding nothing in the next
  /// announced "Nothing left to record in 'notes'" — telling the user their click did nothing while it
  /// had just committed 146 notes. A per-item message inside a loop over items is a report of the last
  /// item, not of the work.
  /// Record one vault's forgotten notes — the panel's per-vault button.
  ///
  /// Reports over the whole action rather than per item: the first version set `notice` inside a loop
  /// over vaults, so recording 146 in one and finding none in the next announced the *nothing*.
  async function onRecordUnrecorded(vault: string) {
    try {
      const r = await recordUnrecorded(vault);
      if (r.committed) {
        notice =
          `Recorded ${r.notes} note${r.notes === 1 ? '' : 's'} in “${labelFor(vault)}”. ` +
          `They are in this device's history now — back up to send them to a remote.`;
      } else if (r.notes > 0) {
        // **A refusal is not a success.** This used to say "Nothing left to record" whenever the
        // backend answered `committed: false` — including when it had found notes and git had
        // *declined* to commit them. The owner tapped Record with 147 outstanding and was told there
        // was nothing to record. Anything the backend can explain goes on screen verbatim, and this
        // is an `error`, not a `notice`, because the action did not happen.
        //
        // **Through `report()`, not a bare assignment.** A direct `error = …` leaves
        // `errorIsTransient` at whatever the last poll set it to, and `refresh()` clears the banner
        // whenever that flag is true — so the one message explaining why 147 notes are still
        // unrecorded could vanish on the next 15 s beat, before it had been read. `report()` is
        // what marks a message as worth outliving a poll, which is exactly what this one is.
        report(
          `Could not record the ${r.notes} note${r.notes === 1 ? '' : 's'} in “${labelFor(vault)}”. ` +
            (r.reason ?? 'Git declined, and gave no reason.'),
        );
      } else {
        notice = `Nothing left to record in “${labelFor(vault)}”.`;
      }
      await loadUnrecorded();
      await loadDuplicates();
    } catch (e) {
      // Same reasoning as the refusal above: a failure the user asked for by tapping Record must
      // outlive the next poll, so it goes through `report()`.
      report(String(e));
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
      // **Split by whether anything was actually committed, because the sentence says so.**
      // "committed here, but nowhere to send" was printed for every remoteless vault, including
      // ones with nothing new in them — so a phone reported a vault as just-committed in the same
      // breath as the quiet-vault chip said "38 days since a save" (2026-09-08). The chip had read
      // `git log`; the sentence had read nothing.
      const localSaved = local.filter((v) => syncFor(v).committed);
      const localQuiet = local.filter((v) => !syncFor(v).committed);
      if (localSaved.length) {
        // Stated as the fact it is, with the fix: no remote yet. Never "failed".
        parts.push(
          `${localSaved.map((v) => `“${labelFor(v)}”`).join(', ')} ` +
            `${localSaved.length === 1 ? 'has' : 'have'} no remote yet — committed here, but ` +
            `nowhere to send. Add one in backup options.`,
        );
      }
      if (localQuiet.length) {
        parts.push(
          `${localQuiet.map((v) => `“${labelFor(v)}”`).join(', ')} ` +
            `${localQuiet.length === 1 ? 'has' : 'have'} nothing new to save, and no remote to ` +
            `send it to. Add one in backup options.`,
        );
      }
      if (failed.length || conflicted.length) {
        // Named, and pointed at the panel that can actually resolve it — a toolbar button is the
        // wrong place to explain a merge conflict.
        parts.push(
          `${[...failed, ...conflicted].map((v) => `“${labelFor(v)}”`).join(', ')} need you: ` +
            `open backup options for detail.`,
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
      // **Backup commits, so the "not in history" chip must be re-read here.** It was loaded once
      // per `vaults` change and after the panel's own Record button — nowhere else. So backing up
      // recorded the notes and left the chip saying the old number, which reads as "Backup did not
      // clear them" (reported 2026-08-24 with a count of 1). The count is a fact about git, and
      // this is the moment git changed.
      await loadUnrecorded();
      // Same argument, and the chip it feeds is the one that says "nothing has been saved here in
      // N days": a backup is precisely the event that answers it, so leaving the old number on
      // screen would have the alert survive the act that resolved it — which is how an alert stops
      // being read at all.
      await loadLastSaves();
      // Backing up goes through `syncVault`, which pulls before it pushes — so it too can be the
      // thing that resurrects a note or demotes a field.
      await Promise.all([loadDemoted(), loadKept()]);
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
{:else if needsWelcome}
  <!-- Before the app, after pairing, and only when there is something to ask: the archive now
       ships a ready vault with a welcome note in it, so `NewVault`'s first-run branch below no longer fires for most
       people and this is what greets them instead. Hidden entirely where git is absent — every
       field on it would be a control that cannot do anything — and hidden the moment a committer
       exists, which is also what makes it stop appearing after it is answered. -->
  <Welcome
    vault={vaults?.[0]?.name ?? ''}
    ondone={() => {
      dismissWelcome();
      void loadVaults();
    }}
  />
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
  <Starting error={bootError} attempts={bootAttempts} waitedMs={bootWaited} onretry={loadVaults} />
{:else}
  <div class="app" data-layout={workspace.layout ?? 'single'} class:panel-collapsed={!panelOpen}>
    <!-- **One set of controls, two placements.** Wide: a vertical panel down the left, holding
       everything. Narrow: the same element as a bar along the bottom, where a thumb can reach it.
       Nothing is duplicated and nothing is platform-branched — it is the container that changes,
       which is why this is markup that does not know where it is.

       This reverses the 2026-08-28 note that lived here ("it was a tall left rail that wasted
       vertical space"). That rail sat *beside* a top bar and displaced nothing; this one replaces
       the top bar, so the workspace gets that row's height back — and it collapses, which the old
       one could not. Reasoning in `decisions.md`, 2026-08-30. -->
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
          onclick={(e) => {
            anchorTo(e);
            createOpen = !createOpen;
            if (createOpen) reloadTemplates(); // a note tagged since load may now be a template
          }}
          aria-expanded={createOpen}
          aria-haspopup="menu"
          title="Make something new (Ctrl+K)"
          aria-label="make something new"
        >
          <Icon name="plus" size={18} />
        </button>
        {#if createOpen}
          <!-- Click-away on a backdrop rather than a document listener: it also blocks the stray
             tap that would otherwise land on whatever is behind the menu. -->
          <div class="menu-backdrop" role="presentation" onclick={() => (createOpen = false)}></div>
          <ul class="create-menu" role="menu" style={menuAnchor}>
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
                  onclick={() => ((createOpen = false), item.run())}>{item.label}</button
                >
              </li>
            {/each}
            <!-- **Where it lands, asked where you decide it** (2026-08-31). This was a permanent
               `in <select>` in the chrome: a control on screen at all times for a choice you make
               only while creating something, and on a narrow bar it cost a whole row. It belongs
               to the ＋ menu, so it lives in the ＋ menu.

               **`menuitemradio`, and the click does not close.** One-of-many, unlike the vault
               *filter* beside it — and you almost always pick the destination and then pick what
               to make, so closing here would mean opening the menu twice for one note. -->
            {#if allVaults.length > 1}
              <li class="menu-sep" role="separator"></li>
              <li class="menu-head" role="presentation">Create in</li>
              {#each allVaults as v (v)}
                <li role="none">
                  <button
                    type="button"
                    role="menuitemradio"
                    aria-checked={(createTarget || defaultVault) === v}
                    onclick={() => setCreateVault(v)}
                  >
                    <span class="tick" aria-hidden="true"
                      >{(createTarget || defaultVault) === v ? '✓' : ''}</span
                    >
                    {labelFor(v)}
                  </button>
                </li>
              {/each}
            {/if}
          </ul>
        {/if}
      </div>

      <!-- Collapsed to its lens until wanted. A search field is the widest thing in the bar and
         is used a fraction as often as it occupies space; open it and it takes the room it
         needs. `searchOpen` starts false on every load, deliberately — a bar that remembers
         being open is a bar that is usually open. -->
      <div class="search-slot" class:open={searchOpen}>
        <label class="searchfield">
          <Icon name="search" size={15} />
          <input
            bind:this={searchEl}
            class="topbar-search"
            type="search"
            placeholder="Search…"
            bind:value={searchQuery}
            oninput={onSearchInput}
            onblur={() => {
              if (!searchQuery.trim()) searchOpen = false;
            }}
            spellcheck="false"
            aria-label="search notes"
          />
        </label>
        <!-- **Its own class, not `.icon-btn`.** Narrow layouts hide every `.icon-btn` in the top
           bar, because those controls also live in the bottom `ViewBar` where the thumb is.
           Search does not, so reusing that class would have made the search button disappear on
           exactly the screen this collapsing is for.

           **Both forms are always in the DOM now**, and CSS decides. The collapsing was justified
           by "a search field is the widest thing in the bar" — true of a bar, and meaningless in a
           panel of fixed width, where the field costs one row of height. Choosing between them in
           script would mean measuring the viewport, which the layout ruling forbids outright. -->
        <button
          type="button"
          class="search-btn"
          onclick={openSearch}
          aria-label="search notes"
          aria-expanded={searchOpen}
          title="Search"
        >
          <Icon name="search" size={16} />
        </button>
      </div>

      <!-- **The same list, for when there is no panel to put a rail in.** Below 60rem the chrome is
         a bar, so the rail is hidden and this opens the identical `viewTargets` in a menu. Without
         it a narrow window has no way to open a view at all, now that a pane header names its
         window instead of switching it.

         Not `.icon-btn`: the twin rules hide every `.icon-btn` in the bar, because those controls
         move to `ViewBar` there — the same trap `.search-btn` carries a comment about. This one
         has to be visible in exactly the place that rule would hide it. -->
      <div class="create-wrap views-wrap">
        <button
          type="button"
          class="views-btn"
          onclick={(e) => (anchorTo(e), (viewsOpen = !viewsOpen))}
          aria-expanded={viewsOpen}
          aria-haspopup="menu"
          title="Open a view"
          aria-label="open a view"
        >
          <Icon name="board" size={16} />
        </button>
        {#if viewsOpen}
          <div class="menu-backdrop" role="presentation" onclick={() => (viewsOpen = false)}></div>
          <ul class="create-menu" role="menu" style={menuAnchor}>
            {#each viewTargets as t (t.key)}
              <li role="none">
                <button type="button" role="menuitem" onclick={() => ((viewsOpen = false), t.run())}
                  >{t.label}</button
                >
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <!-- **The views you can open** — the rail this panel was asked for. A fixed list, in a fixed
         order, so it can be learned: every built-in, then every saved view. It is the same set the
         action list offers under "Open …", computed once (`viewTargets`) so the two cannot drift.
         Panel only: the bottom bar already has `ViewBar`, which answers a different question —
         which of your open windows to look at, rather than which view to open. -->
      <nav class="panel-views" aria-label="views">
        {#each viewTargets as t (t.key)}
          <button
            type="button"
            class="view-item"
            class:saved={t.saved}
            onclick={t.run}
            title={t.saved ? `Open “${t.label}” — a view you saved` : `Open ${t.label}`}
            aria-label={`open ${t.label}`}
          >
            <Icon name={t.icon} size={16} />
            <span class="lbl">{t.label}</span>
          </button>
        {/each}
      </nav>

      <!-- **Also whenever something is actually hidden, however few vaults there are.** This was
         `allVaults.length > 1` alone, and the hazard the label's own comment names — "how a vault
         hidden weeks ago comes to read as notes that have gone missing" — was reachable by exactly
         that gate: hide one of two vaults, remove the other, and the control that undoes it stops
         rendering while the filter keeps filtering. The app then looks like an empty notebook with
         no way back. Reported by the owner, 2026-09-08. -->
      {#if allVaults.length > 1 || hidden.length}
        <!-- **A menu, not a row of chips** (2026-08-31). One chip per vault does not survive a long
           list: it wrapped the bar onto extra rows and, collapsed, showed slivers of names. A menu
           costs one control whatever the list does.

           **The trigger says what the filter is doing.** The chip row only said it in colour, so a
           vault hidden weeks ago read as notes that were missing — the same class of silence
           `decisions.md` rules against for a filtered view. "All vaults" or "2 of 5" is the state
           on its face.

           Keyed and toggled on the vault **name** (the identity, and what `hiddenVaults`
           persists), labelled with what the repository is called — see `vaultLabels.svelte.ts`. -->
        <div class="create-wrap vaults-wrap">
          <button
            type="button"
            class="tb-chip vaults-btn"
            class:filtering={hiddenVaults.length > 0}
            onclick={(e) => (anchorTo(e), (vaultMenuOpen = !vaultMenuOpen))}
            aria-expanded={vaultMenuOpen}
            aria-haspopup="menu"
            title="Which vaults to show"
            aria-label="which vaults to show"
          >
            <span class="lbl">{vaultFilterLabel}</span>
            <Icon name="chevron-down" size={12} />
          </button>
          {#if vaultMenuOpen}
            <div
              class="menu-backdrop"
              role="presentation"
              onclick={() => (vaultMenuOpen = false)}
            ></div>
            <ul class="create-menu" role="menu" style={menuAnchor}>
              {#each allVaults as v (v)}
                <li role="none">
                  <!-- **`menuitemcheckbox`, and the click does not close.** Every other menu in this
                     app is single-shot because it runs one action; a filter is many-of-many and
                     closing after each vault would make setting two of them a chore. That is the
                     one place this diverges from the shared skeleton, so it is stated here. -->
                  <button
                    type="button"
                    role="menuitemcheckbox"
                    aria-checked={!hiddenVaults.includes(v)}
                    onclick={() => toggleVault(v)}
                  >
                    <span class="tick" aria-hidden="true"
                      >{hiddenVaults.includes(v) ? '' : '✓'}</span
                    >
                    {labelFor(v)}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}

      <span class="tb-spacer"></span>

      {#if movedVaults.length}
        <!-- Someone pushed work you don't have — a passive nudge with one-click pull. -->
        <button
          class="tb-chip moved alert"
          onclick={getTheirChanges}
          title="Someone pushed — get their changes"
          aria-label={`${
            movedVaults.length === 1 ? movedVaults[0] : `${movedVaults.length} vaults`
          }: get changes`}
        >
          <Icon name="inbox" size={14} />
          <span class="lbl"
            >{movedVaults.length === 1 ? movedVaults[0] : `${movedVaults.length} vaults`}: get
            changes</span
          >
        </button>
      {/if}

      {#if skippedNotes.length}
        <!-- Notes that are on disk but absent from every view because they do not parse.
           A chip rather than only a banner: the banner is dismissible and this condition
           is not transient — it persists until a human resolves the file. -->
        <button
          class="tb-chip moved alert"
          onclick={() => (skippedOpen = true)}
          title="Notes that could not be read — usually a conflicted merge"
          aria-label={`${skippedNotes.length} unreadable`}
        >
          {skippedNotes.length}{' '}<span class="lbl">unreadable</span>
        </button>
      {/if}

      {#if unrecordedTotal}
        <!-- **Notes that exist and are not in history.** A chip for the same reason "unreadable" is
           one: this condition is not transient, and its whole failure mode was silence. `commit_all`
           stages only the paths the app remembers writing, and that memory dies with the process —
           so a note written before the last restart could never be staged by it, and nothing said
           so. Ninety-five had accumulated over a week before anyone noticed (2026-07-31). -->
        <!-- **Opens the panel rather than committing blind.** It used to record on one click, which is
           the right *action* and the wrong *first* step: the count alone cannot say whether these are
           notes that exist nowhere else or notes something is rewriting, and those want opposite
           responses. The panel says which, then offers the button. -->
        <button
          class="tb-chip moved alert"
          onclick={() => (unrecordedOpen = true)}
          title="Notes on disk that git does not have yet — click to see which, and why"
          aria-label={`${unrecordedTotal} not in history`}
        >
          {unrecordedTotal}{' '}<span class="lbl">not in history</span>
        </button>
      {/if}

      {#if quiet.length}
        <!-- **Nothing has been saved here in weeks.** The only chip that reports an *absence*, and
           the reason it exists: for thirty-nine days a vault could not commit, and the screen it
           was on looked exactly like a notebook nobody had opened. Every other alert here names
           something a detector found, so every other alert is silent when the detector is the
           thing that broke. This one asks git one question — when did this vault last save
           anything — and cannot be blocked by the answer.
           **Filled like the "someone pushed" chip**, because unlike "both answers" this is an
           invitation to act, and the action is one button to its right.
           It opens the panel rather than backing up on the click: weeks of silence has more than
           one cause — a merge waiting on a person, a remote that never got set, or simply nobody
           writing — and the panel is where those are told apart. -->
        <button
          class="tb-chip moved alert"
          onclick={onBackup}
          title={quietTitle(quiet, labelFor)}
          aria-label={quietLabel(quiet, labelFor)}
        >
          <Icon name="clock" size={14} />
          <span class="lbl">{quietLabel(quiet, labelFor)}</span>
        </button>
      {/if}

      {#if settledNotes}
        <!-- **Everything the merge settled on your behalf**, in one chip and one panel: a field the
           two devices set differently (both answers kept, `decisions.md` 2026-09-07) and a note one
           device deleted while the other was editing it (the note kept, same date). Two shapes, one
           category — *they disagreed, the merge chose, nothing is blocked* — and the panel keeps
           them in separate sections because the answers differ.
           **One chip and not two.** The kept-note surface was owed as a chip of its own
           (`outstanding.md` §2.12); it would have been the sixth here, and the measured cost of that
           is in the media query below — a toolbar wrapped to FOUR rows on a real phone, with the
           board starting past halfway. The content is the product.
           **Deliberately the quietest chip** (no `moved` class): "unreadable" means a note is
           missing from every view and "not in history" means a note exists in one place only. This
           one means everything is fine and a choice is waiting. -->
        <button
          class="tb-chip alert"
          onclick={() => (demotedOpen = true)}
          title="Notes the two devices disagreed about — the merge chose, and you can change it"
          aria-label={`${settledNotes} decided for you`}
        >
          {settledNotes}{' '}<span class="lbl">decided for you</span>
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
          aria-label="back up notes"
        >
          <Icon name="backup" size={14} />
          <!-- **No idle label.** Help and Settings beside it carry none, and one labelling rule per
             state is what makes a column of controls read as a column rather than a list of
             exceptions. The word returns while it is *working*, because "is anything happening?"
             is the one moment an icon alone cannot answer (heuristic 1, visibility of system
             status); the tooltip carries the meaning the rest of the time. -->
          {#if savingLabel}<span class="save-label">{savingLabel}</span>{/if}
        </button>
        <button
          type="button"
          class="save-more"
          onclick={(e) => (anchorTo(e), (backupMenuOpen = !backupMenuOpen))}
          aria-expanded={backupMenuOpen}
          aria-haspopup="menu"
          aria-label="other backup options"
          title="Other backup options"
        >
          <Icon name="chevron-down" size={12} />
        </button>
        {#if backupMenuOpen}
          <div
            class="menu-backdrop"
            role="presentation"
            onclick={() => (backupMenuOpen = false)}
          ></div>
          <ul class="create-menu" role="menu" style={menuAnchor}>
            {#each BACKUP_MENU as item (item.label)}
              <li role="none">
                <button
                  type="button"
                  role="menuitem"
                  onclick={() => ((backupMenuOpen = false), item.run())}
                >
                  <span class="mi-label">{item.label}</span>
                  <span class="mi-note">{item.note}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <!-- The manual, one click from anywhere in the app.
         The first non-technical tester's verdict was that it was "difficult to find and click on"
         — it shipped in the archive as a folder, and nothing in the running app ever mentioned
         it. It is baked into the binary, so this works offline and still works when someone has
         copied out just the executable.

         **Not on the phone.** There the UI is served by the Tauri shell, not `fm-serve`, so
         `/manual/` resolves to nothing; a Help button that 404s is worse than no Help button. -->
      <button
        class="icon-btn help"
        onclick={() => (helpOpen = true)}
        aria-label="help"
        title="Help — how this works"
      >
        <Icon name="help" size={16} />
      </button>

      <button
        class="icon-btn"
        onclick={() => openSettings()}
        aria-label="settings"
        title="Settings — what this install is configured as"
      >
        <Icon name="gear" size={16} />
      </button>

      <!-- Last, with the other system controls, rather than above the ＋ where it started: the first
         thing in a panel should be the thing you came to do. Hidden where the chrome is a bar,
         because a bar is already as small as it gets. -->
      <button
        type="button"
        class="icon-btn panel-toggle"
        onclick={togglePanel}
        aria-expanded={panelOpen}
        aria-label={panelOpen ? 'collapse the panel' : 'expand the panel'}
        title={panelOpen ? 'Collapse — give the width back to your notes' : 'Expand the panel'}
      >
        <Icon name="chevron-down" size={16} />
      </button>
    </header>

    <div class="body">
      {#if error}
        <p class="banner error">
          {error}
          <button
            class="banner-dismiss"
            onclick={() => ((error = null), (errorIsTransient = true))}
            aria-label="dismiss">✕</button
          >
        </p>
      {/if}
      {#if notice}
        <p class="banner notice">
          {notice}
          <button class="banner-dismiss" onclick={() => (notice = null)} aria-label="dismiss"
            >✕</button
          >
        </p>
      {/if}
      {#if themeRefused}
        <p class="banner notice">
          {themeRefused}
          <button class="banner-dismiss" onclick={() => (themeRefused = '')} aria-label="dismiss"
            >✕</button
          >
        </p>
      {/if}

      <!-- **The way out of a theme that hides everything.** On screen only while the theme is still
         unproven — the first click, key, wheel or touch anywhere takes it away, because that is the
         moment we learn the app is reachable. A permanent floating button would tax every session
         to insure against a rare one.

         `all: revert` first, then literal values with `!important`, so it does not inherit the
         theme it exists to escape. **This is convenience, not a guarantee**: a theme carrying its
         own `!important` at equal specificity still wins. The layer that actually rescues the app
         is the armed-boot guard in `appearance.ts`, which is JS and cannot be styled away. -->
      {#if userTheme && themeUnproven}
        <button class="theme-escape" onclick={turnOffTheme}>
          Turn off “{userTheme.name}”
        </button>
      {/if}

      <!-- **The chrome, when one view fills the window**: which views are open, and the controls
         belonging to the one you are looking at. At the *top*, which reverses a 2026-08-30 ruling
         about a phone's thumb reach — see `decisions.md` 2026-08-31: only the tabs moved, the
         actions bar stays at the bottom, and this row replaces the pane header rather than
         joining it.

         **Rendered, not merely hidden, on a layout branch.** Everywhere else the arrangement is
         pure CSS; here it cannot be. `ViewControls` inside this bar and the copy inside a tiled
         pane header would both be in the DOM, giving two elements labelled `group by` — ambiguous
         to a screen reader and to `getByLabelText`. This branches on the layout *preference*, not
         on the viewport, so the rule it has to respect is untouched. -->
      {#if (workspace.layout ?? 'single') === 'single'}
        <ViewBar
          panes={workspace.panes}
          active={focused}
          feed={feedKey(workspace.panes[focused])
            ? feeds[feedKey(workspace.panes[focused]) ?? '']
            : undefined}
          onselect={(i) => {
            focused = i;
            persistWorkspace();
          }}
          onchange={(patch) => changePane(workspace.panes[focused].id, patch)}
          onclose={closePane}
        />
      {/if}

      <!-- The flexible workspace: a CSS grid of panes. `cols` sets the column count; each pane
         spans some columns; panes flow into rows. The renderers are pure and height:100%, so
         each drops into its cell unchanged. -->
      <div class="workspace" style="--cols:{workspace.cols}">
        {#each workspace.panes as pane, i (pane.id)}
          <div
            class="cell"
            class:active={i === focused}
            style="grid-column: span {Math.min(
              pane.colSpan,
              workspace.cols,
            )}; grid-row: span {pane.rowSpan};"
          >
            <Pane
              {pane}
              index={i}
              cols={workspace.cols}
              feed={feedKey(pane) ? feeds[feedKey(pane) ?? ''] : undefined}
              statuses={knownStatuses}
              {shown}
              focused={i === focused}
              startEditing={pane.kind === 'note' && pane.noteId === editingId}
              vaults={allVaults}
              onopen={openNote}
              onresolve={onResolveConflict}
              onmove={onMove}
              onstatus={onSetStatus}
              counts={threadCounts}
              onnavigate={openNoteInPane}
              onsaved={scheduleCommit}
              onchange={(patch) => changePane(pane.id, patch)}
              onreorder={movePane}
              onresize={(patch) => resizePane(pane.id, patch)}
              onclose={() => closePane(pane.id)}
              onfocus={() => (focused = i)}
              headed={workspace.layout === 'tiled'}
            />
          </div>
        {/each}
      </div>
    </div>

    {#if settingsOpen}
      {#await import('./lib/SettingsPanel.svelte') then { default: SettingsPanel }}
        <SettingsPanel
          layout={workspace.layout ?? 'single'}
          onlayout={setLayout}
          onclose={() => (settingsOpen = false)}
          onkeyschanged={reloadKeys}
          {commands}
          section={settingsSection}
          columns={workspace.colMode === 'fixed' ? workspace.cols : 'auto'}
          oncolumns={setCols}
          {theme}
          ontheme={toggleTheme}
          {userTheme}
          onusertheme={(sel) => {
            appearance.writeSelection(sel);
            userTheme = sel;
            themeRefused = '';
          }}
          onbackup={() => {
            settingsOpen = false;
            backupOpen = true;
          }}
        />
      {/await}
    {/if}

    {#if helpOpen}
      {#await import('./lib/HelpPanel.svelte') then { default: HelpPanel }}
        <HelpPanel onclose={() => (helpOpen = false)} />
      {/await}
    {/if}

    {#if unrecordedOpen}
      {#await import('./lib/UnrecordedPanel.svelte') then { default: UnrecordedPanel }}
        <UnrecordedPanel
          unrecorded={unrecordedList}
          duplicates={duplicateList}
          onrecord={onRecordUnrecorded}
          onprune={onPruneDuplicates}
          onclose={() => (unrecordedOpen = false)}
        />
      {/await}
    {/if}

    {#if skippedOpen}
      {#await import('./lib/SkippedPanel.svelte') then { default: SkippedPanel }}
        <SkippedPanel skipped={skippedNotes} onclose={() => (skippedOpen = false)} />
      {/await}
    {/if}

    {#if demotedOpen}
      {#await import('./lib/KeptPanel.svelte') then { default: KeptPanel }}
        <KeptPanel
          rows={demotedList}
          kept={keptList}
          onclose={() => (demotedOpen = false)}
          onchanged={async () => {
            await loadDemoted();
            await loadKept();
            await refresh();
          }}
        />
      {/await}
    {/if}

    {#if backupOpen}
      {#await import('./lib/BackupPanel.svelte') then { default: BackupPanel }}
        <BackupPanel
          onclose={() => (backupOpen = false)}
          onvaults={() => void loadVaults()}
          onnewvault={() => {
            backupOpen = false;
            newVaultOpen = true;
          }}
        />
      {/await}
    {/if}

    {#if paperOpen}
      <div class="sheet-backdrop" role="presentation" onclick={() => (paperOpen = false)}></div>
      <div
        class="sheet"
        role="dialog"
        aria-modal="true"
        aria-label="Add a paper"
        tabindex="-1"
        onkeydown={(e) => e.key === 'Escape' && (paperOpen = false)}
      >
        <div class="save-view">
          <h2>Add a paper</h2>
          <label class="sv-field">
            <span>Paste a citation, a DOI, an arXiv link — or just type the title</span>
            <!-- svelte-ignore a11y_autofocus -->
            <textarea
              aria-label="paper citation or identifier"
              bind:value={paperInput}
              rows="6"
              autofocus
              spellcheck="false"
              placeholder={'@article{…}\n\nor  10.48550/arXiv.1706.03762\nor  https://arxiv.org/abs/1706.03762\nor  Attention Is All You Need'}
            ></textarea>
          </label>
          <p class="sv-hint">
            Read on this computer — nothing is looked up online. A pasted citation fills in the
            author, year and journal; an identifier on its own is recorded, and you can fill in the
            rest from the note itself.
          </p>
          <div class="sv-actions">
            <button class="sv-cancel" onclick={() => (paperOpen = false)}>Cancel</button>
            <button class="sv-save" onclick={confirmNewPaper} disabled={!paperInput.trim()}
              >Add paper</button
            >
          </div>
        </div>
      </div>
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
  .save-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-5);
    width: min(26rem, calc(100vw - 2 * var(--space-5)));
  }
  .save-view h2 {
    margin: 0;
    font-size: var(--text-md);
  }
  .sv-field textarea {
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--text-xs);
    resize: vertical;
  }
  .sv-hint {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--text-muted);
  }
  .sv-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  .sv-actions button {
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
  }
  .sv-actions .sv-save:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
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
    /* Centre inside the safe area, not inside the raw window: `viewport-fit=cover` means
       `inset: 0` reaches under the status bar and the navigation bar. `border-box` + an
       explicit `100dvh` so the child can say `max-height: 100%` rather than restate this
       padding in a calc that can fall out of step with it. */
    box-sizing: border-box;
    height: 100dvh;
    padding: var(--safe-top) var(--safe-right) var(--safe-bottom) var(--safe-left);
    z-index: 41;
    pointer-events: none;
  }
  /* The navigation-bar floor — `--bar-floor`, never a hand-copied number. */
  @media (pointer: coarse) {
    .sheet {
      padding-bottom: max(var(--safe-bottom), var(--bar-floor));
    }
  }
  .sheet > :global(*) {
    pointer-events: auto;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-3, 10px);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
    /* `100%` of `.sheet`'s content box — the visible viewport less both insets. Never
       `90vh`: `100vh` is the tallest the viewport ever gets, so with the URL bar out or the
       keyboard up a `vh` cap is taller than what you can see. */
    max-height: 100%;
    overflow: auto;
    overscroll-behavior: contain;
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
     Two arrangements, one shell — and now actually two.

     `single` shows one pane; `tiled` is the grid. **`auto` is gone** (`decisions.md`,
     2026-08-31): it was the *default*, so a phone never matched a `[data-layout='single']`
     rule and every narrow fact had to be written twice. Writing one half was silent, and it
     shipped half-written twice.

     **The arrangement is decided here and nowhere else.** Everything downstream reads a
     *value*, never a selector, so an arrangement-dependent rule can no longer be half-written
     — there is one place to change it and the components inherit. That also retires every
     `[data-layout=…] :global(…)` rule, one of which tied on specificity with `ViewBar`'s own
     scoped `.viewbar` and was decided by bundle order: the same defect `ci/checks.sh` polices
     for `.panel-views`, sitting unnoticed in a second component.

     Still pure CSS keyed off `data-layout` — **no viewport-tracking TypeScript**, exactly as
     promised when the phone CSS first landed. Every pane stays mounted and fetched, so
     switching is instant and the feed layer is untouched: `display: none`, never unmounting.
     --------------------------------------------------------------------------------------- */
  .app {
    --cell: none; /* a pane that is not the active one */
    --ws-cols: minmax(0, 1fr);
    --ws-rows: minmax(0, 1fr);
    /* With one pane filling the screen there is nothing to drag it against and nothing to
       resize it relative to. The pane's own *content* controls stay — those configure what you
       are looking at, not where it sits. */
    --pane-grip: none;
    --pane-resize: none;
    --view-name: 1.05rem; /* one view on screen: its name *is* the chrome */
  }
  .app[data-layout='tiled'] {
    --cell: flex;
    /* `var(--cols)` resolves at the *use* site — `.workspace`, which carries the inline
       `style="--cols:…"`. Substitution is lazy, which is what lets a value defined up here
       read one that is only set further down the tree. */
    --ws-cols: repeat(var(--cols, 1), minmax(0, 1fr));
    --ws-rows: minmax(8rem, 1fr);
    --pane-grip: inline-flex;
    --pane-resize: block;
    --view-name: 0.85rem;
  }
  /* `tiled` on a phone still stacks and scrolls — the pre-existing behaviour, expressed as a
     value. Must sit *after* the `[data-layout='tiled']` rule above: equal specificity, so
     source order decides. */
  @media (max-width: 40rem) {
    .app[data-layout='tiled'] {
      --ws-cols: 1fr;
      --ws-rows: minmax(60vh, auto);
    }
  }
  .cell:not(.active) {
    display: var(--cell);
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
  /* See the comment at the markup. Literal values on purpose: every one of these read from a
     custom property would be a property the theme can redefine. */
  .theme-escape {
    all: revert;
    position: fixed !important;
    /* Clear of the navigation bar and any cutout. `calc` rather than a bare literal because this
       is the escape hatch: a button you cannot reach is the same as no escape hatch, and it sits
       at the one screen edge the system chrome always occupies. */
    right: calc(12px + var(--safe-right, 0px)) !important;
    bottom: calc(12px + var(--safe-bottom, 0px)) !important;
    z-index: 2147483647 !important;
    display: block !important;
    visibility: visible !important;
    opacity: 1 !important;
    pointer-events: auto !important;
    min-width: 44px !important;
    min-height: 44px !important;
    padding: 10px 14px !important;
    border: 2px solid #000 !important;
    border-radius: 8px !important;
    background: #fff !important;
    color: #000 !important;
    font:
      500 14px/1.2 system-ui,
      sans-serif !important;
    cursor: pointer !important;
  }
  /* ================= WHERE THE CHROME SITS =====================================
     The same `<header>` in two placements. Wide: a column down the left, so the workspace gets
     the full height — the objection that removed the old rail was that it displaced nothing, and
     this replaces the top bar rather than joining it. Narrow: a bar along the bottom, because a
     top bar on a phone holds actions the thumb cannot reach.

     Driven by width alone. `pointer: coarse` is *not* consulted here: a wide touch tablet has
     room for the panel and a narrow desktop window does not, which is the same
     branch-on-space-never-on-platform rule the layout arrangements already follow. */
  @media (min-width: 60rem) {
    .app {
      grid-template-columns: auto minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr);
    }
    .topbar {
      grid-column: 1;
      grid-row: 1;
      width: 13.5rem;
      height: 100%;
      min-height: 0;
      flex-direction: column;
      align-items: stretch;
      flex-wrap: nowrap;
      /* The panel is the one place a scroll is right: it is a list of controls, it is obviously
         vertical, and unlike the old top bar nothing is hidden off an edge nobody would swipe. */
      overflow-y: auto;
      overflow-x: hidden;
      border-bottom: 0;
      border-right: 1px solid var(--border);
      padding: max(var(--safe-top), var(--space-3)) var(--space-3) var(--space-3)
        max(var(--safe-left), var(--space-3));
      gap: var(--space-2);
    }
    /* Collapsed: down to its icons. The width goes back to the notes, and one click returns it —
       which the rail this replaces could never do.

       **Hide the words; never crop them.** The first version set this width with
       `overflow: hidden` and left every label laid out, so the rail showed a sliver of "Back up"
       and a sliver of the vault name — text cut mid-word, which reads as broken rather than as
       compact. A collapsed rail is icons and nothing else, and each one keeps the `title` it
       already had, so the name is a hover away rather than gone. */
    .app.panel-collapsed .topbar {
      width: 3.5rem;
      padding-left: var(--space-2);
      padding-right: var(--space-2);
      align-items: center;
    }
    .app.panel-collapsed .topbar .lbl,
    .app.panel-collapsed .topbar .save-label {
      display: none;
    }
    /* These are *only* their words — a vault picker with no name, or a filter chip with nothing
       in it, would be a control that cannot say what it does. They come back on expand. */
    .app.panel-collapsed .topbar .vaults-wrap,
    .app.panel-collapsed .topbar .searchfield {
      display: none;
    }
    .app.panel-collapsed .topbar .create-wrap,
    .app.panel-collapsed .topbar .view-item,
    .app.panel-collapsed .topbar .tb-chip {
      justify-content: center;
    }
    .app.panel-collapsed .topbar .view-item {
      padding-inline: 0;
    }
    .body {
      grid-column: 2;
      grid-row: 1;
      min-height: 0;
    }
    /* In a column these should fill the panel's width rather than shrink to their text. */
    .topbar .create-wrap {
      width: 100%;
    }

    /* **The search field is open in the panel.** Its collapsing was justified by "a search field
       is the widest thing in the bar" — a statement about a bar. Here the panel's width is fixed
       and a field costs one row of height, so there is nothing to reclaim by hiding it. Both forms
       are in the DOM and CSS picks, because deciding in script would mean measuring the viewport,
       which the layout ruling forbids. */
    .app:not(.panel-collapsed) .topbar .searchfield {
      display: flex;
      width: 100%;
    }
    .app:not(.panel-collapsed) .topbar .search-btn {
      display: none;
    }
    .topbar .searchfield :global(input) {
      width: 100%;
    }

    /* The trailing controls line up with the rail above them rather than centring in a column
       that is otherwise all left-aligned — proximity is doing the grouping, so alignment should
       not fight it. */
    .topbar .icon-btn,
    .topbar .save-btn {
      justify-content: flex-start;
    }
    .topbar .icon-btn {
      display: flex;
      align-items: center;
      padding-left: var(--space-2);
    }
    /* The spacer earns its keep in both placements: it pushes the trailing controls to the far
       edge, which is the right-hand end of a bar and the bottom of a panel. */
    .topbar .tb-spacer {
      width: 100%;
    }
    .panel-toggle {
      align-self: flex-end;
      transform: rotate(90deg);
    }
    .app.panel-collapsed .panel-toggle {
      align-self: center;
      transform: rotate(-90deg);
    }
  }

  /* Narrow: the same element, along the bottom. Everything the hand needs is on one edge. */
  /* The bar's way into the same list. Hidden by default because the panel has the rail; shown
     only where the chrome is a bar. Declared before its override, for the reason spelled out
     immediately below. */
  .views-wrap {
    display: none;
  }
  .views-btn {
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
  .views-btn:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }

  /* **The rail, and the bug that made it invisible.**
     
     This was two rules: `display: flex` inside `@media (min-width: 60rem)`, and `display: none`
     here. **A media query adds no specificity**, so at wide widths they tied and the later one —
     this one — won. The rail never rendered, at any width, from the day it was added; and nothing
     caught it, because jsdom applies no CSS, so the test that finds these buttons cannot tell you
     they are hidden.
     
     So visibility is decided in exactly one place now: shown by default, hidden only in the
     narrow query, which is exclusive with the panel. Nothing to tie, and the answer no longer
     depends on where in this file the blocks happen to sit. `ci/checks.sh` fails the build if a
     bare `display: none` comes back. */
  .panel-views {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
  }
  .view-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 2rem;
    padding: var(--space-1) var(--space-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    font: inherit;
    font-size: var(--text-sm);
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  .view-item:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  /* A saved view is one of yours, and reads as a name rather than a fixture. */
  .view-item.saved .lbl {
    font-style: italic;
  }

  @media (max-width: 59.999rem) {
    .app {
      grid-template-rows: minmax(0, 1fr) auto;
    }
    .topbar {
      grid-row: 2;
      border-bottom: 0;
      border-top: 1px solid var(--border);
      /* **The insets swap ends with the bar, and this is the only rule that says so.** `env()` is
         0 in the Android WebView until the shell hands the real values over, so the floor in
         `app.css` is the fallback that clears the navigation bar. Any narrower rule that touches
         padding must use longhands and leave the bottom alone — a shorthand here is how this bar
         became untappable for a day. `ci/checks.sh` guards it. */
      padding: var(--space-1) max(var(--safe-right), var(--space-3))
        max(var(--safe-bottom), var(--space-2)) max(var(--safe-left), var(--space-3));
    }
    .body {
      grid-row: 1;
      min-height: 0;
    }
    /* Only a panel can be collapsed; a bar is already as small as it gets. */
    .panel-toggle {
      display: none;
    }
    /* No room for a rail along a bar — the Views button opens the same list instead. */
    .panel-views {
      display: none;
    }
    /* **Help goes, Settings stays.** This used to hide every `.icon-btn`, which was survivable
       only because `ViewBar` carried a second gear on the phone; that gear is gone (the strip
       is desktop-only now), so hiding the class outright would leave a phone with no way into
       Settings at all — and on a device with no logcat that is the only diagnostic surface
       there is. Help is the one that can afford to go: it opens a manual that 404s on the
       phone anyway. */
    .topbar .help {
      display: none;
    }
    .views-wrap {
      display: flex;
    }
  }

  .search-slot {
    display: flex;
    align-items: center;
    min-width: 0;
  }
  /* In a bar the field appears on demand, exactly as before. */
  .search-slot .searchfield {
    display: none;
  }
  .search-slot.open .searchfield {
    display: flex;
  }
  .search-slot.open .search-btn {
    display: none;
  }

  .workspace {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: var(--ws-cols);
    grid-auto-rows: var(--ws-rows);
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
    /* No coordinates here — `anchorTo` measures the button that opened it and supplies them
       inline. A fixed corner was only ever right while the chrome was a bar across the top. */
    z-index: 41;
    min-width: 11rem;
    /* `dvh`, not `vh` — see the overlay note in app.css. */
    max-height: 70dvh;
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
  /* The vault filter's trigger. A `tb-chip` like the alert chips beside it, plus a state it is
     allowed to shout about: a filter that is hiding something should not look idle. */
  .vaults-btn {
    gap: var(--space-1);
  }
  .vaults-btn.filtering {
    border-color: var(--accent);
    color: var(--text);
  }
  /* A fixed-width tick column, so the names line up whether or not they are checked — a list that
     shifts sideways as you toggle it is hard to aim at. */
  .create-menu button .tick {
    display: inline-block;
    width: 1rem;
    color: var(--accent);
  }
  /* Names the group below it. Not a `menuitem` — it is a label, and making it focusable would
     put a dead stop in the keyboard walk through the menu. */
  .menu-head {
    padding: 4px var(--space-2) 2px;
    color: var(--text-muted);
    font-size: var(--text-xs);
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
  .tb-spacer {
    flex: 1;
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
      /* The columns and rows are the arrangement's to decide (`--ws-cols`/`--ws-rows`, set
         once at the top); this rule keeps only what is genuinely about a narrow screen. */
      padding: var(--space-1);
      gap: var(--space-1);
      /* Panes stack, so the page scrolls vertically and never sideways. */
      overflow-x: hidden;
    }
    /* **Longhands, never the `padding` shorthand.** This rule and the safe-area one in the
       59.999rem block above both match a phone, media queries add no specificity, and Svelte
       scopes both the same — so source order decides and this one wins. As a shorthand it reset
       all four sides, discarding `max(var(--safe-bottom), …)` and leaving the bar's content
       6.4px above the screen edge. On a device with a 47px navigation bar and 44px controls,
       that put Back up, "get their changes" and Settings *inside* the system strip, where the
       taps never reached the app (reported 2026-08-31).

       It was written on 2026-07-18, when `.topbar` was a **top** bar and a symmetric 0.4rem was
       harmless; the bar moved to the bottom on 2026-08-30 and nobody re-read this. The bottom is
       deliberately not set here at all — it belongs to the rule that knows about the inset. */
    .topbar {
      flex-wrap: wrap;
      row-gap: 0.4rem;
      padding-top: 0.4rem;
      padding-inline: 0.5rem;
    }

    /* Measured on a device (2026-07-19, `sessions/2026-07-19-the-ui-on-android.md`): the
       toolbar wrapped to FOUR rows and the board began below the halfway mark of a 2400px
       screen. Everything below is about getting that back — the content is the product, the
       chrome is not. */

    .tb-chip {
      flex: 0 0 auto;
    }

    /* The row that used to overflow — wordmark, search field, "＋ New", "＋ View" — is now a
       plus, a lens and a gear, so nothing here needs hiding to make it fit. What remains is the
       search field *once opened*: it is the only elastic thing in the bar, and 6rem still shows
       a legible placeholder while leaving room for the vault chips beside it. */
    .topbar-search {
      width: 6rem;
    }
    /* The icon says "back up" well enough at this width, and the word is the widest thing left
       in the bar. **Written once now.** It used to need a `[data-layout='single']` twin, because
       `auto` was the default and a phone matched only the media query; with `auto` gone this is
       a fact about the window and one rule states it. */
    .save-label {
      display: none;
    }
    /* **The alert chips shed their words at this width, and keep their counts.** Measured on the
       owner's phone (2026-09-08): five chips of prose wrapped the toolbar and pushed the board
       down, which is the same failure the four-row note above records — and the reason a sixth
       chip was refused rather than added. The icon and the number are the alert; the sentence is
       detail, and detail belongs in the panel each chip opens. Every one of them carries an
       explicit `aria-label` with the full wording, so hiding the text costs the button no name —
       to a screen reader, and to the tests, nothing here changed.
       **`.alert` and not `.tb-chip`**: the vault filter is a `tb-chip` too, and it is *only* its
       words. The rule above about `.vaults-wrap` is the same lesson, learned on the same bar. */
    .tb-chip.alert .lbl {
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
