// **The names of places come from one place, for every screen on every system**
// (`decisions.md#ui`, 2026-10-12, *the layout may differ; the names never do*).
//
// There is one UI: the desktop serves `ui/dist` and the phone's shell embeds the same folder
// (`mobile/src-tauri/tauri.conf.json`), so a name can only differ between systems if it is
// written in two places here. These pin the places it is written:
//
//   - a built-in view's name: `BUILTIN_PANES` in `panes.ts`, and nowhere else;
//   - the two groups of built-in views: `BUILTIN_SPACES` in `spaces.svelte.ts`;
//   - the groups and views that ship with the app: the `.view` files in
//     `crates/fm-app/src/shipped/`, which every system's backend reads. That the in-memory
//     backend and Settings copy them exactly is pinned from the other side, where the files
//     are (`crates/fm-app/tests/log.rs`, *the front end names the shipped groups…*).
import { describe, expect, it } from 'vitest';
import { BUILTIN_PANES, paneLabel, paneTitle, type Pane } from './panes';
import { BUILTIN_SPACES } from './spaces.svelte';

describe('the names of places', () => {
  it('a group’s tab is called what the view is called everywhere else', () => {
    for (const s of BUILTIN_SPACES)
      for (const t of s.tabs) {
        expect(t.label, `${s.name} → ${t.kind}`).toBe(paneLabel(t.kind));
        expect(BUILTIN_PANES.some((b) => b.kind === t.kind && b.label === t.label)).toBe(true);
      }
  });

  it('a window’s title is the same name', () => {
    for (const b of BUILTIN_PANES) {
      const title = paneTitle({ kind: b.kind, groupBy: 'status' } as Pane);
      expect(title.startsWith(b.label), `${b.kind}: ${title}`).toBe(true);
    }
  });

  it('no built-in view is in two groups, and no two groups share a name', () => {
    const kinds = BUILTIN_SPACES.flatMap((s) => s.tabs.map((t) => t.kind));
    expect(new Set(kinds).size).toBe(kinds.length);
    const names = BUILTIN_SPACES.map((s) => s.name);
    expect(new Set(names).size).toBe(names.length);
  });
});
