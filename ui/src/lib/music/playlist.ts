// **A playlist is a note** (`music-plan-2026-11-13.md`, *Playlists*): tagged `playlist`, its body a
// numbered list of links to song notes, `1. [Title](note:<id>)`. The order is the list's order, so
// the playlist reads and edits as an ordinary note, and two devices adding to one merge like any
// note.
//
// Every change here is a body edit through `updateBody` against the version read, recorded for
// Undo like any other edit to a note's text. Lines that are not entries — a sentence about the
// playlist, a heading — are kept where they are.
import { getNote, updateBody } from '../ipc';
import { createNotesUndoable, recordTextEdit } from '../undo.svelte';

export interface Entry {
  id: string;
  title: string;
}

/** A list line that links a note: `3. [Title](note:<ulid>)`, with anything after it. */
const LINE = /^\s*\d+[.)]\s+\[((?:\\.|[^\]\\])*)\]\(note:([0-9A-Za-z]{26})\)/;

/** Escape the label half of a Markdown link. */
const label = (t: string) => t.replace(/\\/g, '\\\\').replace(/[[\]]/g, '\\$&');
const unlabel = (t: string) => t.replace(/\\(.)/g, '$1');

/** The playlist's entries, in order. */
export function entries(body: string): Entry[] {
  const out: Entry[] = [];
  for (const line of body.split('\n')) {
    const m = LINE.exec(line);
    if (m) out.push({ id: m[2].toUpperCase(), title: unlabel(m[1]) });
  }
  return out;
}

/** The numbered list of `list`, one line each. */
export function render(list: Entry[]): string {
  return list.map((e, i) => `${i + 1}. [${label(e.title)}](note:${e.id})`).join('\n');
}

/**
 * `body` with its entries replaced by `list`, renumbered. The new list stands where the first
 * entry stood; every other line is kept. A body with no entries gets the list at its end.
 */
export function rewrite(body: string, list: Entry[]): string {
  const lines = body.split('\n');
  const at = lines.findIndex((l) => LINE.test(l));
  const rest = lines.filter((l) => !LINE.test(l));
  const block = list.length ? [render(list)] : [];
  if (at < 0) {
    const kept = body.replace(/\s+$/, '');
    return (kept ? `${kept}\n\n` : '') + (block[0] ?? '') + '\n';
  }
  // The index among the kept lines where the first entry used to be.
  const before = lines.slice(0, at).filter((l) => !LINE.test(l)).length;
  const out = [...rest.slice(0, before), ...block, ...rest.slice(before)].join('\n');
  return out.endsWith('\n') ? out : out + '\n';
}

/** Make a playlist of `list`, as one undoable step. Answers the new note's id. */
export async function create(title: string, list: Entry[]): Promise<string> {
  const name = title.trim() || 'New playlist';
  const [made] = await createNotesUndoable(
    [{ title: name, tags: ['playlist'], props: {}, body: list.length ? render(list) + '\n' : '' }],
    `made the playlist “${name}”`,
  );
  return made.id;
}

/** Change a playlist's entries, as one undoable edit to its note. */
export async function edit(id: string, change: (list: Entry[]) => Entry[]): Promise<void> {
  const n = await getNote(id);
  if (!n) throw new Error('that playlist no longer exists');
  const after = rewrite(n.body, change(entries(n.body)));
  if (after === n.body) return;
  await updateBody(id, after, n.version);
  recordTextEdit(id, n.title ?? 'playlist', n.body, after);
}

/** Move the entry at `from` to `to`. */
export function move(list: Entry[], from: number, to: number): Entry[] {
  if (from === to || !list[from] || to < 0 || to >= list.length) return list;
  const out = list.slice();
  const [e] = out.splice(from, 1);
  out.splice(to, 0, e);
  return out;
}
