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

/** How the panes are arranged. **Two named layouts, and deliberately only two.**
 *
 *  - `auto`   — the arrangement follows the available space (the default).
 *  - `tiled`  — the pane grid, always. What the desktop has always done.
 *  - `single` — one view at a time with a switcher, always.
 *
 *  This is a *layout*, not a platform. `single` is reachable on a desktop and `tiled` on a
 *  tablet, which is the point: if the narrow arrangement only worked on a phone it would be a
 *  fork with extra steps, and nothing would exercise it during ordinary desktop work.
 *
 *  Two and no more. A general "customisable frontend" is unbounded and lands on the plugin API
 *  `docs/context/plan.md` already rejects. */
export type Layout = 'auto' | 'tiled' | 'single';

export interface Workspace {
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

/** The default workspace: one board, two columns to arrange into. */
export function defaultWorkspace(): Workspace {
  return { cols: 2, panes: [newPane('board')] };
}

/** Give every pane a fresh, unique id. The id counter (`paneId`) resets on each page load, so
 *  ids persisted by an older session can collide across loads — and a duplicate key crashes the
 *  keyed `{#each}` that renders the grid (a blank window). Ids are only `{#each}` keys within one
 *  session and are never persisted meaningfully, so re-minting them on load is safe and always
 *  unique. Call this on whatever comes back from storage. */
export function reidentify(panes: Pane[]): Pane[] {
  return panes.map((p) => ({ ...p, id: paneId() }));
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
