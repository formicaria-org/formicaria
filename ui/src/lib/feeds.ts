// **What a view shows, read from the server** — one place for the app and every feature page
// (`decisions.md` 2026-11-17, *one server, many views*: one codebase, many views). A feed key
// (`panes.ts`' `feedKey`) names what to read; the answer is what the renderers draw.
import {
  conflicts,
  discussions as fetchDiscussions,
  getAgenda,
  getBoard,
  proposals,
  recent,
  runView,
} from './ipc';
import { BUILTIN_SPACES } from './spaces.svelte';
import type { Feed } from './panes';
import type { ObjectMeta, ViewGroup, ViewInfo } from './types';

export async function loadFeed(key: string): Promise<Feed> {
  const sep = key.indexOf(':');
  const type = sep === -1 ? key : key.slice(0, sep);
  const arg = sep === -1 ? '' : key.slice(sep + 1);
  if (type === 'board') return { board: await getBoard(arg) };
  if (type === 'agenda') return { cards: await getAgenda() };
  if (type === 'timeline') return { cards: await recent() };
  if (type === 'collaboration') {
    // Two things that need a person: proposals (branch + note) and notes still in conflict (both
    // versions marked in the body). Conflicts render above proposals as "needs resolution".
    const [cards, confl] = await Promise.all([proposals(), conflicts()]);
    return { cards, conflicts: confl };
  }
  if (type === 'discussions') return { discussions: await fetchDiscussions() };
  if (type === 'view') {
    const r = await runView(arg);
    // What the view leaves out travels with what it returned, so a pane can explain a column (or a
    // note) a filter removed.
    const view = {
      filters: r.filters ?? [],
      renderer: r.renderer,
      group_by: r.group_by,
      totals: r.totals,
      group: r.group,
    };
    return r.board ? { board: r.board, view } : { cards: r.rows ?? [], view };
  }
  return {};
}

/** The cards a feed carries, wherever it keeps them. */
export function cardsOf(feed: Feed | undefined): ObjectMeta[] {
  if (!feed) return [];
  return feed.board ? feed.board.columns.flatMap((c) => c.cards) : (feed.cards ?? []);
}

/** The statuses a feed's notes use, for the status controls. */
export function statusesOf(feed: Feed | undefined): string[] {
  return [...new Set(cardsOf(feed).flatMap((c) => (c.status ? [c.status] : [])))];
}

/** The groups of views (Food, Fitness, …) in the order they were listed. A shipped tracker group is
 *  left out where this device hides the trackers; the built-in groups every device has stay. */
export function groupsOf(list: ViewInfo[], trackersShown: boolean): ViewGroup[] {
  const out: ViewGroup[] = [];
  for (const v of list) {
    if (v.error || !v.group) continue;
    const builtIn = BUILTIN_SPACES.some((s) => s.name === v.group);
    if (v.shipped && !trackersShown && !builtIn) continue;
    const g = out.find((x) => x.name === v.group);
    if (g) g.views.push(v.name);
    else out.push({ name: v.group, views: [v.name] });
  }
  return out;
}
