// **One piece: content moving from anything into a note** (`docs/context/workspace-design-2026-11-15.md`
// §13.3; `decisions.md` 2026-11-16, *one library of notes*). Every way content moves (a web page's
// selection or snip, the phone's Share and selection menu, the phone's page, a mail's passage, a
// passage of any other note, a discussion's reply) produces the same thing: the note text it becomes,
// and where it came from. One consumer puts pieces into a note, through that note's own pane, as one
// edit (`App.svelte`'s `put`).
import type { Brought, SharedText } from './share.svelte';
import { broughtText } from './web/brought';
import { latestMessage } from './web/mail';
import { sharedPassage } from './web/shared';

export interface PieceSource {
  /** The kind of what it came from: `web`, `phone`, `note`, `conversation`, `discussion`… */
  kind: string;
  /** Its exact address: a page's URL (at the passage), `note:<id>`. */
  address: string;
  title: string;
}

export interface Piece {
  /** What it becomes in the note, its source line included. */
  md: string;
  source: PieceSource;
}

const quote = (text: string) =>
  text
    .split('\n')
    .map((l) => (l.trim() ? `> ${l.trimEnd()}` : '>'))
    .join('\n');
const clean = (s: string) => s.replace(/[[\]]/g, '').trim();

/** Text from another app (Share, the selection menu). */
export function fromShared(t: SharedText, when: string): Piece {
  return {
    md: sharedPassage(t.text, t.subject, when),
    source: { kind: 'phone', address: '', title: t.subject },
  };
}

/** What the phone's page brought back. */
export function fromBrought(b: Brought, when: string): Piece {
  return {
    md: broughtText(b, when),
    source: { kind: 'web', address: b.pageUrl, title: b.pageTitle },
  };
}

interface NoteLike {
  id: string;
  title?: string | null;
  body: string;
  kind_key?: string;
  props?: Record<string, unknown>;
}

/** A passage of a note, for another note: the selection quoted with a link back to it (and when);
 *  with nothing selected, a mail's latest message, or else a link to the note itself. */
export function fromNote(n: NoteLike, selection: string, when: string): Piece {
  const title = clean(n.title || 'a note');
  const conversation = n.kind_key === 'conversation' || !!n.props?.gmail_thread;
  const label = conversation ? `Mail: ${title}` : title;
  const at =
    conversation && typeof n.props?.mail_last === 'string' ? n.props.mail_last.slice(0, 10) : when;
  const source = {
    kind: n.kind_key || (conversation ? 'conversation' : 'study'),
    address: `note:${n.id}`,
    title,
  };
  const text = selection.trim() || (conversation ? latestMessage(n.body) : '');
  if (!text) return { md: `[${title}](note:${n.id})`, source };
  return { md: `${quote(text.slice(0, 8000))}\n>\n> — [${label}](note:${n.id}), ${at}`, source };
}

/** Pieces as one block of note text, in the order they came. */
export function piecesMarkdown(pieces: Piece[]): string {
  return pieces.map((p) => p.md.trim()).join('\n\n');
}
