import { describe, expect, it } from 'vitest';
import {
  newPane,
  reidentify,
  defaultWorkspace,
  feedKey,
  distinctFeeds,
  reorder,
  clampSpan,
  paneTitle,
  rendererKind,
  autoCols,
  matchesTarget,
  migrateWorkspace,
  SCHEMA,
  type Pane,
} from './panes';

describe('newPane', () => {
  it('defaults to a 1×1 pane with sane per-kind params', () => {
    const p = newPane('board');
    expect(p.kind).toBe('board');
    expect(p.groupBy).toBe('status');
    expect(p.colSpan).toBe(1);
    expect(p.rowSpan).toBe(1);
    expect(p.id).toMatch(/^p\d+$/);
  });
  it('gives every pane a distinct id', () => {
    expect(newPane('board').id).not.toBe(newPane('board').id);
  });
  it('applies overrides (e.g. a saved view name)', () => {
    expect(newPane('view', { viewName: 'Lab' }).viewName).toBe('Lab');
  });
});

describe('feedKey', () => {
  it('keys a board by its group-by, so two boards grouped differently do not share a feed', () => {
    expect(feedKey(newPane('board', { groupBy: 'status' }))).toBe('board:status');
    expect(feedKey(newPane('board', { groupBy: 'project' }))).toBe('board:project');
  });
  it('keys search by its query', () => {
    expect(feedKey(newPane('search', { query: 'mito' }))).toBe('search:mito');
  });
  it('has no feed for a note pane (NotePanel self-fetches by id)', () => {
    expect(feedKey(newPane('note', { noteId: '01K' }))).toBeNull();
  });
  it('is null for a view pane with no name yet', () => {
    expect(feedKey(newPane('view'))).toBeNull();
  });
});

describe('distinctFeeds', () => {
  it('dedups so two panes over the same feed fetch once', () => {
    const panes: Pane[] = [
      newPane('board', { groupBy: 'status' }),
      newPane('board', { groupBy: 'status' }), // same feed as the first
      newPane('agenda'),
      newPane('note', { noteId: 'x' }), // no feed
    ];
    expect(distinctFeeds(panes).sort()).toEqual(['agenda', 'board:status']);
  });
  it('is empty when nothing needs fetching', () => {
    expect(distinctFeeds([newPane('note', { noteId: 'x' })])).toEqual([]);
  });
});

describe('reorder', () => {
  const ids = (ps: Pane[]) => ps.map((p) => p.id);
  it('moves a pane from one slot to another', () => {
    const a = newPane('board');
    const b = newPane('agenda');
    const c = newPane('timeline');
    expect(ids(reorder([a, b, c], 0, 2))).toEqual([b.id, c.id, a.id]);
  });
  it('is a no-op for an out-of-range or identity move', () => {
    const a = newPane('board');
    const b = newPane('agenda');
    expect(reorder([a, b], 1, 1)).toEqual([a, b]);
    expect(reorder([a, b], 5, 0)).toEqual([a, b]);
  });
});

describe('clampSpan', () => {
  it('never exceeds the column count and never drops below 1', () => {
    expect(clampSpan(3, 2)).toBe(2);
    expect(clampSpan(0, 4)).toBe(1);
    expect(clampSpan(2, 4)).toBe(2);
  });
});

describe('paneTitle / rendererKind', () => {
  it('labels a board with its grouping', () => {
    expect(paneTitle(newPane('board', { groupBy: 'project' }))).toBe('Board · project');
  });
  it('maps a saved view renderer onto the pane kind that draws it', () => {
    expect(rendererKind('board')).toBe('board');
    expect(rendererKind('agenda')).toBe('agenda');
    expect(rendererKind('gallery')).toBe('timeline'); // gallery shows as a flat list
  });
});

describe('reidentify', () => {
  it('gives every pane a unique id even when the inputs collide', () => {
    // Simulates a workspace persisted across sessions where the reset id counter minted the
    // same id twice — the exact shape that crashed the keyed {#each} and blanked the window.
    const colliding: Pane[] = [
      { ...newPane('board'), id: 'p1' },
      { ...newPane('note'), id: 'p1' },
      { ...newPane('agenda'), id: 'p1' },
    ];
    const out = reidentify(colliding);
    expect(new Set(out.map((p) => p.id)).size).toBe(3);
    // Kind and params are preserved — only the id changes.
    expect(out.map((p) => p.kind)).toEqual(['board', 'note', 'agenda']);
  });
});

describe('defaultWorkspace', () => {
  // **It used to say two columns for one pane**, and this test pinned that as a feature. `cols`
  // was only ever recomputed inside `addPane`/`closePane`, never on load, so a fresh session on
  // a wide screen laid the single board into a two-column grid and left the second one blank.
  it('is one board, in exactly as many columns as it needs', () => {
    const w = defaultWorkspace();
    expect(w.cols).toBe(1);
    expect(w.panes).toHaveLength(1);
    expect(w.panes[0].kind).toBe('board');
  });

  it('opens one view at a time, and says which schema wrote it', () => {
    const w = defaultWorkspace();
    expect(w.layout).toBe('single');
    expect(w.active).toBe(0);
    expect(w.v).toBe(SCHEMA);
  });
});

describe('matchesTarget — what counts as the same thing', () => {
  // The rule behind `focusOrOpen`: tapping a view's name shows that view rather than stacking
  // another window beside it.
  it('treats a board as a board however it is grouped', () => {
    // group-by is a control *inside* the window, not a different view.
    expect(matchesTarget({ ...newPane('board'), groupBy: 'tag' }, 'board')).toBe(true);
    expect(matchesTarget(newPane('board'), 'agenda')).toBe(false);
  });

  it('separates saved views by name and notes by id', () => {
    const v = newPane('view', { viewName: 'Papers' });
    expect(matchesTarget(v, 'view', { viewName: 'Papers' })).toBe(true);
    expect(matchesTarget(v, 'view', { viewName: 'Reading' })).toBe(false);
    const n = newPane('note', { noteId: 'abc' });
    expect(matchesTarget(n, 'note', { noteId: 'abc' })).toBe(true);
    // Load-bearing: two editors on one note race each other through `update_body`.
    expect(matchesTarget(n, 'note', { noteId: 'xyz' })).toBe(false);
  });

  it('matches any search regardless of query, so a second query re-points one pane', () => {
    expect(matchesTarget(newPane('search', { query: 'alpha' }), 'search', { query: 'beta' })).toBe(
      true,
    );
  });
});

describe('migrateWorkspace', () => {
  it('falls back to the default for anything unusable', () => {
    for (const bad of [null, undefined, {}, { panes: [] }, { panes: [newPane('board')] }]) {
      expect(migrateWorkspace(bad).workspace.layout).toBe('single');
    }
  });

  it('reads an unstamped `auto` as the old default, not as a choice', () => {
    // `layout` was written by the very first persist, so a stored `auto` is overwhelmingly the
    // default of the day rather than a decision — and `auto` no longer exists to honour.
    const { workspace, migrated } = migrateWorkspace({
      cols: 2,
      layout: 'auto',
      panes: [newPane('board')],
    });
    expect(workspace.layout).toBe('single');
    expect(migrated).toBe(true);
    expect(workspace.v).toBe(SCHEMA);
  });

  it('leaves a deliberate `tiled` alone, and does not re-migrate a stamped record', () => {
    const tiled = migrateWorkspace({ cols: 2, layout: 'tiled', panes: [newPane('board')] });
    expect(tiled.workspace.layout).toBe('tiled');
    const stamped = migrateWorkspace({
      v: SCHEMA,
      cols: 1,
      layout: 'tiled',
      panes: [newPane('board')],
    });
    expect(stamped.workspace.layout).toBe('tiled');
    expect(stamped.migrated).toBe(false);
  });

  it('clamps an active pane that is past the end', () => {
    // Under one-view-at-a-time an out-of-range active pane means no `.cell.active` at all, and
    // every other cell is hidden — a blank app, from a stored number. `closePane` clamped; load
    // did not, and the old tiled default hid the consequence because every cell was visible.
    const { workspace } = migrateWorkspace({
      cols: 2,
      layout: 'single',
      panes: [newPane('board'), newPane('agenda')],
      active: 5,
    });
    expect(workspace.active).toBe(1);
    expect(
      migrateWorkspace({ cols: 1, panes: [newPane('board')], active: -3 }).workspace.active,
    ).toBe(0);
  });

  it('recomputes the column count, which load never used to do', () => {
    expect(migrateWorkspace({ cols: 2, panes: [newPane('board')] }).workspace.cols).toBe(1);
    // A pinned width is the user's, and survives.
    expect(
      migrateWorkspace({ cols: 3, colMode: 'fixed', panes: [newPane('board')] }).workspace.cols,
    ).toBe(3);
  });
});

// ── The grid reclaims space, and defaults stay typeable on a non-US keyboard ──
import * as k from './keys';

describe('columns follow the pane count', () => {
  // **The real function, imported.** This used to be a hand-written mirror of `App.svelte`'s
  // copy, with a comment saying so — which is a second definition that can drift from the one
  // that ships. The property is still the point: opening a view widened the grid and closing
  // one did NOT narrow it, so closing two of three notes left one in a third of the screen.
  const cols = (n: number, mode: 'auto' | 'fixed', cols: number) =>
    autoCols(n, { colMode: mode, cols });

  it('grows as views open and shrinks as they close', () => {
    expect(cols(1, 'auto', 2)).toBe(1);
    expect(cols(3, 'auto', 2)).toBe(3);
    expect(cols(1, 'auto', 3)).toBe(1); // the reclaim
  });

  it('never exceeds four, because a fifth pane is a sliver', () => {
    expect(cols(8, 'auto', 2)).toBe(4);
  });

  it('leaves a pinned width alone', () => {
    expect(cols(4, 'fixed', 2)).toBe(2);
  });
});

describe('default shortcuts are typeable on a non-US layout', () => {
  // `Ctrl+[` / `Ctrl+]` shipped first and are unreachable on an Italian keyboard, where both
  // brackets need AltGr. Anything behind AltGr, or that moves between layouts, is barred.
  const NEEDS_ALTGR_OR_MOVES = ['[', ']', '\\', ';', "'", '`', '@', '#', '{', '}'];

  it('binds no key that an Italian layout puts behind AltGr', () => {
    for (const cmd of Object.keys(k.DEFAULTS) as k.Command[]) {
      expect(NEEDS_ALTGR_OR_MOVES, `${cmd} is not reachable on an Italian keyboard`).not.toContain(
        k.DEFAULTS[cmd].key,
      );
    }
  });

  it('matches on the physical key when a rebind recorded one', () => {
    // Captured on one layout, pressed on another: the character differs, the position does not.
    const b: k.Binding = { key: 'z', code: 'KeyY', mod: true };
    const ev = (o: Partial<KeyboardEvent>) =>
      ({
        key: 'y',
        code: 'KeyY',
        ctrlKey: true,
        metaKey: false,
        altKey: false,
        shiftKey: false,
        ...o,
      }) as KeyboardEvent;
    expect(k.matches(ev({}), b)).toBe(true);
    expect(k.matches(ev({ code: 'KeyZ' }), b)).toBe(false);
  });
});
