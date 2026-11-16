// **A passage of a mail conversation, for a study room** (workspace-design §12, #2; `decisions.md`
// 2026-11-16, night: *mail stays in its section*). What the person selected in the conversation, or
// else its latest message, quoted, ending with a link back to the conversation and the date of the
// message. The conversation itself stays in the Mail section.
/** The latest message of a conversation note: the text after its last message heading. */
export function latestMessage(body: string): string {
  const parts = body.split(/\n(?=#{2,4} )/);
  const last = parts[parts.length - 1] ?? body;
  return last.replace(/<!--[\s\S]*?-->/g, '').trim();
}
