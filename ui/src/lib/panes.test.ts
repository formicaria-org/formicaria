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
  it('is one board in a two-column grid', () => {
    const w = defaultWorkspace();
    expect(w.cols).toBe(2);
    expect(w.panes).toHaveLength(1);
    expect(w.panes[0].kind).toBe('board');
  });
});

// ── The grid reclaims space, and defaults stay typeable on a non-US keyboard ──
import * as k from './keys';

describe('columns follow the pane count', () => {
  // Mirrors App.svelte's `autoCols`. Kept here because the property is the point: opening a
  // view widened the grid and closing one did NOT narrow it, so closing two of three notes
  // left one note sitting in a third of the screen.
  const autoCols = (n: number, mode: 'auto' | 'fixed', cols: number) =>
    mode === 'fixed' ? cols : Math.max(1, Math.min(n, 4));

  it('grows as views open and shrinks as they close', () => {
    expect(autoCols(1, 'auto', 2)).toBe(1);
    expect(autoCols(3, 'auto', 2)).toBe(3);
    expect(autoCols(1, 'auto', 3)).toBe(1); // the reclaim
  });

  it('never exceeds four, because a fifth pane is a sliver', () => {
    expect(autoCols(8, 'auto', 2)).toBe(4);
  });

  it('leaves a pinned width alone', () => {
    expect(autoCols(4, 'fixed', 2)).toBe(2);
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
      ({ key: 'y', code: 'KeyY', ctrlKey: true, metaKey: false, altKey: false, shiftKey: false, ...o }) as KeyboardEvent;
    expect(k.matches(ev({}), b)).toBe(true);
    expect(k.matches(ev({ code: 'KeyZ' }), b)).toBe(false);
  });
});
