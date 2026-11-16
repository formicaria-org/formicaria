// **Where a feature page is** (`decisions.md` 2026-11-17, *one server, many views*): every feature and
// every note has an address on formicaria's server, so a browser tab can hold it.
//
//   /f/note/<id>          a note
//   /f/group/<name>       a feature: its views as a strip (Notes is the group `Library`)
//   /f/view/<name>        one saved or shipped view
//   /f/<kind>             a built-in view (agenda, board, timeline, discussions, collaboration)
import type { PaneKind } from '../lib/panes';

export type Route =
  | { kind: 'note'; id: string }
  | { kind: 'group'; name: string }
  | { kind: 'view'; name: string }
  | { kind: 'builtin'; pane: PaneKind };

const BUILTIN: PaneKind[] = ['agenda', 'board', 'timeline', 'discussions', 'collaboration'];
const ID = /^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/;

export function parseRoute(path: string): Route {
  const parts = path
    .replace(/^\/f\/?/, '')
    .split('/')
    .map(decodeURIComponent);
  const [head, rest] = [parts[0] ?? '', parts.slice(1).join('/')];
  if (head === 'note' && ID.test(rest)) return { kind: 'note', id: rest.toUpperCase() };
  if (head === 'group' && rest) return { kind: 'group', name: rest };
  if (head === 'view' && rest) return { kind: 'view', name: rest };
  if ((BUILTIN as string[]).includes(head)) return { kind: 'builtin', pane: head as PaneKind };
  return { kind: 'group', name: 'Library' };
}

/** The address of an item, as formicaria's tabs and links know it (`note:<id>`, `view:…`). */
export function itemUrl(r: Route): string {
  switch (r.kind) {
    case 'note':
      return `/f/note/${r.id}`;
    case 'group':
      return `/f/group/${encodeURIComponent(r.name)}`;
    case 'view':
      return `/f/view/${encodeURIComponent(r.name)}`;
    case 'builtin':
      return `/f/${r.pane}`;
  }
}

/** formicaria's older addresses (`note:<id>`, `view:group:<name>`, `view:saved:<name>`, `view:<kind>`)
 *  as a feature page's. */
export function addressUrl(address: string): string {
  if (address.startsWith('note:')) return itemUrl({ kind: 'note', id: address.slice(5) });
  const v = address.startsWith('view:') ? address.slice(5) : address;
  if (v.startsWith('group:')) return itemUrl({ kind: 'group', name: v.slice(6) });
  if (v.startsWith('saved:')) return itemUrl({ kind: 'view', name: v.slice(6) });
  if ((BUILTIN as string[]).includes(v)) return itemUrl({ kind: 'builtin', pane: v as PaneKind });
  return itemUrl({ kind: 'group', name: 'Library' });
}
