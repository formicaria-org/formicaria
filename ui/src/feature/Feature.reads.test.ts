// **A feature page reads its data once** (`decisions.md` 2026-11-17). Guards the loop found by
// `browser-check`'s memory budget: a feature whose first view is the timeline set itself to the
// timeline again and again, and a tab of it grew until the browser ran out of memory.
import { render } from '@testing-library/svelte';
import { afterEach, expect, it, vi } from 'vitest';
import Feature from './Feature.svelte';
import * as mock from '../lib/mock';

afterEach(() => {
  history.replaceState(null, '', '/');
  vi.restoreAllMocks();
});

for (const [path, reads] of [
  ['/f/group/Library', ['recent']],
  ['/f/group/Tasks', ['board']],
  ['/f/group/Food', ['run_view']],
  ['/f/agenda', ['agenda']],
] as const) {
  it(`${path} reads what it shows once, and nothing it does not show`, async () => {
    mock.reset();
    const spy = vi.spyOn(mock, 'handle');
    history.replaceState(null, '', path);
    const r = render(Feature);
    await new Promise((res) => setTimeout(res, 1200));
    const calls = spy.mock.calls.map((c) => c[0]);
    for (const read of reads) expect(calls.filter((c) => c === read)).toHaveLength(1);
    for (const other of ['recent', 'board', 'run_view', 'agenda'].filter(
      (c) => !(reads as readonly string[]).includes(c),
    ))
      expect(
        calls.filter((c) => c === other),
        `${path} read ${other}`,
      ).toHaveLength(0);
    expect(calls.length).toBeLessThan(15);
    r.unmount();
  });
}
