import type { ObjectMeta } from './types';

// What to call a note where there is room for one line: its header, and its row in the list of open
// windows. Its title when it has one; otherwise its first line, the way a card names it
// (`title ?? preview`), with a Markdown heading's `#` marks taken off — an untitled note's first line
// is usually its heading, and "# Reading notes" is not a name. A `#tag` keeps its mark: no space, no
// heading. A board's first line is its canvas scene, never a name, so an untitled board has none.
// `null` when there is nothing to call it by.
export function noteName(n: Pick<ObjectMeta, 'title' | 'preview' | 'props'>): string | null {
  const title = n.title?.trim();
  if (title) return title;
  if (n.props?.view === 'board') return null;
  return n.preview?.replace(/^#{1,6}\s+/, '').trim() || null;
}
