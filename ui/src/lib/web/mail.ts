// **A passage of a mail conversation, for a study room** (workspace-design §12, #2; `decisions.md`
// 2026-11-16, night: *mail stays in its section*). What the person selected in the conversation, or
// else its latest message, quoted, ending with a link back to the conversation and the date of the
// message. The conversation itself stays in the Mail section.
import { appToday } from '../appTime';

interface MailNote {
  id: string;
  title?: string | null;
  body: string;
  props?: Record<string, unknown>;
}

/** The latest message of a conversation note: the text after its last message heading. */
export function latestMessage(body: string): string {
  const parts = body.split(/\n(?=#{2,4} )/);
  const last = parts[parts.length - 1] ?? body;
  return last.replace(/<!--[\s\S]*?-->/g, '').trim();
}

export function mailPassage(n: MailNote, selection = ''): string {
  const text = (selection.trim() || latestMessage(n.body)).slice(0, 8000);
  const quoted = text
    .split('\n')
    .map((l) => (l.trim() ? `> ${l.trimEnd()}` : '>'))
    .join('\n');
  const title = (n.title ?? 'a conversation').replace(/[[\]]/g, '');
  const when = typeof n.props?.mail_last === 'string' ? n.props.mail_last.slice(0, 10) : appToday();
  return `${quoted}\n>\n> — [Mail: ${title}](note:${n.id}), ${when}`;
}
