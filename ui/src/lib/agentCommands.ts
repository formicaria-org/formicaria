// The assistant slash-commands offered as tappable chips in a discussion — discoverable and one-tap
// on a phone, still typeable on a laptop (the owner: commands must be easily selectable on both
// devices). `/research` leads: the grounded web-research profile whose only output is a cited proposal.
//
// The chip-insert logic is a pure string transform so it is unit-tested without the (huge) NotePanel
// component: given the current draft, the tapped command, and who to address, it returns the new draft.

export const AGENT_COMMANDS = [
  { cmd: '/research', hint: 'Grounded web research → a cited proposal on this note' },
  { cmd: '/search', hint: 'Ground the reply in a web search' },
  { cmd: '/propose', hint: 'Propose an edit to this note' },
  { cmd: '/transcribe', hint: "Recordings and handwriting in this note → text, as a proposal" },
] as const;

/**
 * Build the draft after a command chip is tapped. Ensures the message addresses an assistant
 * (`@name`) so the command actually reaches one — prepending `agent` only when nothing is mentioned
 * yet — then appends the command **idempotently** (never twice). Returns with a trailing space so the
 * user types the question next. With no `agent` and no existing mention, the command is still inserted
 * (unaddressed): the send path surfaces the "no assistant" notice.
 */
export function withCommand(draft: string, cmd: string, agent?: string): string {
  let d = draft.trim();
  const hasMention = /(^|\s)@\S+/.test(d);
  if (!hasMention && agent) d = `@${agent} ${d}`.trim();
  // Escape any regex metacharacters in the command (the leading `/` is literal in a RegExp body).
  const esc = cmd.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  if (!new RegExp(`(^|\\s)${esc}(\\s|$)`).test(d)) d = `${d} ${cmd}`.trim();
  return `${d} `;
}
