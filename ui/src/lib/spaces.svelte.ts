// The four groups everything in the app belongs to — Library, Tasks, Food, Fitness — and whether
// this device shows them (`decisions.md#ui`, 2026-10-12, *four groups over notes*).
//
// **The built-in views join a group by this table**, not by becoming `.view` files: as files the
// agenda would lose its *Show as* and search its typed query. Food and Fitness are groups already,
// by the `group:` key of the views that ship with the app.
//
// **Phase 0: this device only.** On in `pnpm dev`, off everywhere else until the owner has tried
// it on the phone. With it off the chrome is exactly what it was.
import type { PaneKind } from './panes';
import type { ViewGroup } from './types';

export type SpaceTab = { key: string; label: string; kind: PaneKind; viewName?: string };
export type Space = { name: string; icon: string; tabs: SpaceTab[] };

export const BUILTIN_SPACES: Space[] = [
  {
    name: 'Library',
    icon: 'timeline',
    tabs: [
      { key: 'timeline', label: 'Notes', kind: 'timeline' },
      { key: 'search', label: 'Search', kind: 'search' },
    ],
  },
  {
    name: 'Tasks',
    icon: 'board',
    tabs: [
      { key: 'board', label: 'Board', kind: 'board' },
      { key: 'agenda', label: 'Agenda', kind: 'agenda' },
    ],
  },
];

/** A group of views that ship with the app, as the same shape. */
export const spaceOfGroup = (g: ViewGroup): Space => ({
  name: g.name,
  icon: 'chart',
  tabs: g.views.map((v) => ({ key: `view:${v}`, label: v, kind: 'view', viewName: v })),
});

const KEY = 'fm-spaces';

function read(): boolean {
  try {
    const v = localStorage.getItem(KEY);
    if (v !== null) return v === '1';
  } catch {
    /* no storage: the default */
  }
  return import.meta.env.DEV && import.meta.env.MODE !== 'test';
}

const state = $state({ shown: read() });

export const spacesShown = () => state.shown;

/** Read the stored choice again. For tests, which swap storage between cases. */
export function reload() {
  state.shown = read();
}
