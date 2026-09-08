// The flexible workspace: a set of panes the user opens, arranges, and resizes freely in a
// grid. Deliberately NOT a fixed set of preset splits — the user asked to *open views and
// rearrange them where they want*. A pane is `(kind + its params + a grid span)`; the
// workspace is a column count and an ordered list of panes that flow into that grid.
//
// Pure data + helpers here (unit-testable); the rendering lives in Pane.svelte and App.svelte.

import type { ObjectMeta, Board, DiscussionSummary, Renderer, ConflictInfo } from './types';

/** What a pane shows. The built-in renderers (including the git `activity` stream), a specific
 *  note (which also covers a whiteboard — a board-note), or a saved `.view`. */
export type PaneKind =
  | 'board'
  | 'agenda'
  | 'timeline'
  | 'search'
  | 'activity'
  | 'collaboration'
  | 'discussions'
  | 'note'
  | 'view';

export interface Pane {
  id: string;
  kind: PaneKind;
  groupBy: string; // board
  agendaMode: 'month' | 'week' | 'list'; // agenda
  /** timeline — how much of each note a row shows. See `Timeline.svelte`. */
  timelineMode: 'feed' | 'compact';
  query: string; // search
  noteId: string | null; // note / whiteboard
  viewName: string | null; // a saved .view
  colSpan: number; // grid columns occupied (>=1)
  rowSpan: number; // grid rows occupied (>=1)
}

/** How the panes are arranged. **Two named layouts, and now actually only two.**
 *
 *  - `single` — one view at a time, on every screen. **The default.**
 *  - `tiled`  — the pane grid. What the desktop used to do by default.
 *
 *  This is a *layout*, not a platform. `single` is reachable on a desktop and `tiled` on a
 *  tablet, which is the point: if the narrow arrangement only worked on a phone it would be a
 *  fork with extra steps, and nothing would exercise it during ordinary desktop work.
 *
 *  **`auto` was removed on 2026-08-31.** It followed the available space, which sounded free and
 *  was not: because it was the *default*, a phone never matched a `[data-layout='single']` rule,
 *  so every narrow CSS fact had to be written twice — silently, and it shipped half-written
 *  twice. It also synced nothing (the preference is per-browser `localStorage`), so its pitch —
 *  tile on the laptop, one view on the phone — is what two per-device settings already do. See
 *  `decisions.md`, 2026-08-31. */
export type Layout = 'single' | 'tiled';

/** Schema version stamped into a persisted workspace. Bumped when a stored record needs
 *  interpreting differently — the point being to tell "the user chose this" apart from "this was
 *  the default at the time", which is exactly what the `auto` removal turned on. */
export const SCHEMA = 2;

export interface Workspace {
  /** Schema version (`SCHEMA`). Absent on anything written before 2026-08-31, which is precisely
   *  what `migrateWorkspace` needs to know: an unstamped record's `layout` may be a default
   *  rather than a choice. */
  v?: number;
  cols: number; // grid column count (>=1) — tiled only
  /** How `cols` is decided. `auto` tracks the pane count so opening a view widens the grid and
   *  closing one lets the rest reclaim the space; a number pins it. Absent means `auto`, so a
   *  workspace persisted before this existed keeps working. */
  colMode?: 'auto' | 'fixed';
  panes: Pane[]; // flow into the grid in order
  /** Optional for back-compat: a workspace persisted before layouts existed has neither, and
   *  `loadWorkspace` must keep accepting it. */
  layout?: Layout;
  /** Which pane is showing in `single`. Promotes `focused` from a border-colour hint to real,
   *  persisted state — it already meant "the active pane" everywhere else (`closePane` clamps
   *  it, `openNoteInPane` sets it), it just never decided what was visible. */
  active?: number;
}

/** A soft ceiling so a runaway loop or a fat-fingered "add" can't spawn hundreds of heavy
 *  panes (each fetches and, for a whiteboard, mounts a React root). Not a hard layout limit —
 *  the user still arranges the ones they have however they like. */
export const MAX_PANES = 8;

let seq = 0;
/** A stable id without Math.random/Date (kept deterministic-friendly): a monotonic counter,
 *  which is all a `{#each}` key needs within one session. */
export function paneId(): string {
  seq += 1;
  return `p${seq}`;
}

export function newPane(kind: PaneKind, over: Partial<Pane> = {}): Pane {
  return {
    id: paneId(),
    kind,
    groupBy: 'status',
    agendaMode: 'month',
    // **Feed by default.** A timeline of titles answers "what did I write"; a feed answers "what
    // has been happening", which is the question a shared vault raises — every collaborator's
    // notes in one stream, each badged with its audience and its last editor. The dense list is a
    // click away for when you are hunting rather than catching up.
    timelineMode: 'feed',
    query: '',
    noteId: null,
    viewName: null,
    colSpan: 1,
    rowSpan: 1,
    ...over,
  };
}

/** How many columns the grid should have for `n` panes. `fixed` pins it to whatever the user
 *  chose; `auto` tracks the pane count, so opening a view widens the grid and closing one lets
 *  the rest reclaim the space. Capped at 4 — past that a column is too narrow to read.
 *
 *  **This lives here, not in `App.svelte`, because `defaultWorkspace` and `migrateWorkspace`
 *  both need it** and because `panes.test.ts` used to re-implement it by hand with a comment
 *  admitting it was a mirror. A mirror is a second definition that can drift. */
export function autoCols(n: number, w: Pick<Workspace, 'colMode' | 'cols'>): number {
  if (w.colMode === 'fixed') return w.cols;
  return Math.max(1, Math.min(n, 4));
}

/** The default workspace: one board, one view at a time.
 *
 *  **`cols` is derived, never stated.** Both copies of this default used to say `cols: 2` while
 *  starting with a single pane, and `cols` was only ever recomputed inside `addPane`/`closePane`
 *  — so a fresh session on a wide screen laid one pane into a two-column grid and left the second
 *  column blank. `panes.test.ts` pinned that as a feature. */
export function defaultWorkspace(): Workspace {
  const panes = [newPane('board')];
  return {
    v: SCHEMA,
    cols: autoCols(panes.length, { colMode: 'auto', cols: 1 }),
    layout: 'single',
    panes,
    active: 0,
  };
}

/** Give every pane a fresh, unique id. The id counter (`paneId`) resets on each page load, so
 *  ids persisted by an older session can collide across loads — and a duplicate key crashes the
 *  keyed `{#each}` that renders the grid (a blank window). Ids are only `{#each}` keys within one
 *  session and are never persisted meaningfully, so re-minting them on load is safe and always
 *  unique. Call this on whatever comes back from storage. */
export function reidentify(panes: Pane[]): Pane[] {
  return panes.map((p) => ({ ...p, id: paneId() }));
}

/** Does this pane already show what is being asked for?
 *
 *  The question behind "open the Board": with one view at a time, tapping a name means *show me
 *  that*, so a pane already showing it should be brought forward rather than duplicated. What
 *  makes two panes of a kind different *things* is one parameter, and only one:
 *
 *  - `view` — its name. Two saved views are two views.
 *  - `note` — its id. This one predates the rule (`openNoteInPane` never opened a note twice) and
 *    it is load-bearing: two editors open on one note race each other through `update_body`.
 *  - everything else — nothing. **A board grouped differently is deliberately the same pane**:
 *    group-by is a control *inside* the window, not another view. Likewise a search: matching any
 *    search regardless of query is what lets a second query re-point the one results pane, which
 *    is what `onSearchInput` already did by hand. */
export function matchesTarget(p: Pane, kind: PaneKind, over: Partial<Pane> = {}): boolean {
  if (p.kind !== kind) return false;
  if (kind === 'view') return p.viewName === (over.viewName ?? null);
  if (kind === 'note') return p.noteId === (over.noteId ?? null);
  return true;
}

/** Read whatever is in storage and return a workspace that is safe to render.
 *
 *  **Pure, so it can be unit-tested.** This used to be inline in `App.svelte`'s `loadWorkspace`,
 *  where the only way to exercise it was to render the whole app.
 *
 *  It does four jobs beyond parsing:
 *
 *  1. **Clamps `active`.** `closePane` clamped it and load did not. Under `single` an out-of-range
 *     active pane means no `.cell.active` at all, and `.cell:not(.active)` hides the rest — a
 *     blank app, from a stored number. Under the old tiled default every cell was visible, so the
 *     same bad record was survivable and nobody saw it.
 *  2. **Recomputes `cols`**, which was only ever recomputed on add/close (see `defaultWorkspace`).
 *  3. **Migrates the layout, once.** Missing or `auto` becomes `single`; `tiled` is left alone.
 *  4. **Stamps `SCHEMA`**, so step 3 cannot run twice and undo a later choice.
 *
 *  `migrated` means *"this differs from what was stored — write it back"*, which includes the
 *  first run, where nothing was stored at all.
 *
 *  **Why an unstamped `auto` is treated as "never chose".** `layout` is written into storage by
 *  the very first `persistWorkspace()`, so a stored `auto` is overwhelmingly the old default
 *  rather than a decision — and `auto` no longer exists to honour. `tiled` was never a default,
 *  so it is always a real choice and survives. */
export function migrateWorkspace(raw: unknown): { workspace: Workspace; migrated: boolean } {
  const w = raw as Partial<Workspace> | null | undefined;
  if (!w || !Array.isArray(w.panes) || !w.panes.length || typeof w.cols !== 'number') {
    // `migrated` means "this differs from what was stored, write it back" — and nothing stored
    // differs from the default. Stamping a first run makes the arrangement explicit in storage
    // rather than implicit in whatever this version happens to default to.
    return { workspace: defaultWorkspace(), migrated: true };
  }
  const panes = reidentify(w.panes as Pane[]);
  // `as string` because a pre-migration record can legitimately hold 'auto', which `Layout` no
  // longer admits — reading it is the whole point of this function.
  const stored = w.layout as string | undefined;
  const layout: Layout = stored === 'tiled' ? 'tiled' : 'single';
  const migrated = w.v !== SCHEMA;
  return {
    workspace: {
      ...w,
      v: SCHEMA,
      layout,
      panes,
      cols: autoCols(panes.length, { colMode: w.colMode, cols: w.cols }),
      active: Math.min(Math.max(w.active ?? 0, 0), panes.length - 1),
    } as Workspace,
    migrated,
  };
}

/** The data source a pane draws from. Two panes with the *same* key share one fetch — so a
 *  workspace of five panes over three distinct feeds costs three requests, not five, and the
 *  3 s heartbeat refreshes the distinct set. `note` has no feed (NotePanel self-fetches by id). */
export function feedKey(p: Pane): string | null {
  switch (p.kind) {
    case 'board':
      return `board:${p.groupBy}`;
    case 'agenda':
      return 'agenda';
    case 'timeline':
      return 'timeline';
    case 'search':
      return `search:${p.query}`;
    case 'collaboration':
      // The proposals feed — a flat `ObjectMeta` list like `timeline`, fetched once and shared.
      return 'collaboration';
    case 'discussions':
      return 'discussions';
    case 'view':
      return p.viewName ? `view:${p.viewName}` : null;
    case 'note':
    case 'activity':
      // No feed: `note` self-fetches by id; `activity` reads the reactive activity module,
      // which App refreshes alongside the feeds.
      return null;
  }
}

/** The distinct feed keys a workspace needs — what App actually fetches. */
export function distinctFeeds(panes: Pane[]): string[] {
  return [...new Set(panes.map(feedKey).filter((k): k is string => k !== null))];
}

/** Whatever a feed holds: a grouped board, a flat card list, or the discussions summary list. */
export interface Feed {
  board?: Board;
  cards?: ObjectMeta[];
  discussions?: DiscussionSummary[];
  /** Conflicted notes (Collaboration surface) — shown above proposals as "needs resolution".
   *  Carries the conflict *kind*, because only one kind can be resolved by editing the note. */
  conflicts?: ConflictInfo[];
  /** The saved view this feed came from, when it came from one — what it filters out, in words,
   *  plus the built-in renderer it shadows so the pane can offer the unfiltered version.
   *
   *  **It rides with the feed, not with the view list.** The words describe *this* payload, and a
   *  pane rendering a view before (or without) a successful `list_views` would otherwise show a
   *  filtered board with nothing to explain it — which is the bug, reintroduced through the fix's
   *  own data source. `list_views` is fetched once per `vaults` change and has failed on the phone
   *  before (`App.boot.test.ts`); `run_view` arrives with the board itself and cannot disagree
   *  with it. */
  view?: { filters: string[]; renderer: Renderer; group_by: string | null };
}

/** Move the pane at `from` to `to`, returning a new array (drag-to-reorder). Out-of-range or
 *  no-op moves return the list unchanged. */
export function reorder(panes: Pane[], from: number, to: number): Pane[] {
  if (from === to || from < 0 || to < 0 || from >= panes.length || to >= panes.length) {
    return panes;
  }
  const next = panes.slice();
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

/** Clamp a span to something sane for the current column count. */
export function clampSpan(span: number, cols: number): number {
  return Math.max(1, Math.min(span, cols));
}

/** The label shown in a pane's header and its picker. `view`/`note` carry their own name. */
export function paneTitle(p: Pane): string {
  switch (p.kind) {
    case 'board':
      return `Board · ${p.groupBy}`;
    case 'agenda':
      return 'Agenda';
    case 'timeline':
      return 'Timeline';
    case 'search':
      return p.query ? `Search · ${p.query}` : 'Search';
    case 'activity':
      return 'Activity';
    case 'collaboration':
      return 'Collaboration';
    case 'discussions':
      return 'Discussions';
    case 'view':
      return p.viewName ?? 'View';
    case 'note':
      return 'Note';
  }
}

/** The built-in panes, in the order they appear everywhere they are enumerated. **One list, so
 *  the three surfaces that list them cannot drift** — the ⌘K "Open …" commands, the pane
 *  view-picker, and the bottom-bar icons all derive from this, so a new kind added here shows up
 *  in all three at once (the "load-bearing literal spread across files" this exists to prevent).
 *  `note`/`view` are deliberately absent: a note opens by being clicked, a saved view from its own
 *  list. */
export interface BuiltinPane {
  kind: PaneKind;
  label: string;
  /** An `Icon.svelte` name; the bottom bar falls back to a dot for anything it does not know. */
  icon: string;
}
/** The icon for the control that *opens* this list, which is deliberately **not** any pane's own.
 *
 *  It lives here, beside them, because that is the only place the collision is visible: the button
 *  hard-coded `board` and so wore the same picture as the Board view it offers — on a phone, the
 *  bottom bar's picker and the Board tab directly above it were identical (reported 2026-09-08,
 *  *"the icon of open view and open board are the same, should be different"*). A constant here,
 *  and one test over this file, is what keeps the next icon choice from re-colliding. */
export const VIEWS_MENU_ICON = 'views';

export const BUILTIN_PANES: BuiltinPane[] = [
  { kind: 'board', label: 'Board', icon: 'board' },
  { kind: 'agenda', label: 'Agenda', icon: 'calendar' },
  { kind: 'timeline', label: 'Timeline', icon: 'timeline' },
  { kind: 'search', label: 'Search', icon: 'search' },
  { kind: 'activity', label: 'Activity', icon: 'inbox' },
  { kind: 'collaboration', label: 'Collaboration', icon: 'merge' },
  { kind: 'discussions', label: 'Discussions', icon: 'chat' },
];

/// **What is offered, as distinct from what exists.** `BUILTIN_PANES` above stays the full
/// registry — kind to label and icon — because a pane of *any* kind still has to render, still
/// needs its icon in the bottom bar, and a workspace saved before today may hold one. These two
/// lists are only about what the app *invites* you to open.
///
/// **`search` is offered nowhere.** Opening an empty search pane does nothing: the search box makes
/// one when you type into it (`onSearchInput`), which is the only way in that has ever made sense.
/// Listing it invited a click that produces a blank pane.
export const OPENABLE_PANES: BuiltinPane[] = BUILTIN_PANES.filter((b) => b.kind !== 'search');

/// **What the rail shows.** The palette is filterable, so a long list there costs nothing; the rail
/// is a column you look at, where every entry is paid for in attention. `activity` comes off it —
/// it is the git edit history, genuinely useful and genuinely rare — and stays in the palette,
/// which is progressive disclosure rather than removal. It is *not* the same view as the timeline:
/// the timeline is your notes by the day you wrote them, activity is every edit including other
/// people's. If it should go entirely, deleting it from `OPENABLE_PANES` is the one-line version.
export const RAIL_PANES: BuiltinPane[] = OPENABLE_PANES.filter((b) => b.kind !== 'activity');

/** Map a saved-`.view` renderer onto the pane kind that draws it (they share renderers). */
export function rendererKind(r: Renderer): PaneKind {
  return r === 'board' ? 'board' : r === 'agenda' ? 'agenda' : 'timeline';
}
