// The flexible workspace: a set of panes the user opens, arranges, and resizes freely in a
// grid. Deliberately NOT a fixed set of preset splits — the user asked to *open views and
// rearrange them where they want*. A pane is `(kind + its params + a grid span)`; the
// workspace is a column count and an ordered list of panes that flow into that grid.
//
// Pure data + helpers here (unit-testable); the rendering lives in Pane.svelte and App.svelte.

import type { ObjectMeta, Board, Renderer } from './types';

/** What a pane shows. The built-in renderers (including the git `activity` stream), a specific
 *  note (which also covers a whiteboard — a board-note), or a saved `.view`. */
export type PaneKind = 'board' | 'agenda' | 'timeline' | 'search' | 'activity' | 'note' | 'view';

export interface Pane {
  id: string;
  kind: PaneKind;
  groupBy: string; // board
  agendaMode: 'month' | 'week' | 'list'; // agenda
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

/** Whatever a feed holds: a grouped board, or a flat card list. */
export interface Feed {
  board?: Board;
  cards?: ObjectMeta[];
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
    case 'view':
      return p.viewName ?? 'View';
    case 'note':
      return 'Note';
  }
}

/** Map a saved-`.view` renderer onto the pane kind that draws it (they share renderers). */
export function rendererKind(r: Renderer): PaneKind {
  return r === 'board' ? 'board' : r === 'agenda' ? 'agenda' : 'timeline';
}
