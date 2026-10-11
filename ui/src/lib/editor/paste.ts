/// What pasting does when it is not just "put this text here".
///
/// **An address pasted over selected words links them** (the owner, 2026-10-11): select a word or
/// a sentence, paste a link, and the words become the link's text. It is what the selection and
/// the clipboard say together, and it saves typing the brackets by hand.
///
/// Only when it is unambiguous:
/// - what is pasted is one address and nothing else: a web or mail address, or a note's;
/// - what is selected is words on one line, and is not itself an address (pasting one address
///   over another is replacing it).
/// Anything else is an ordinary paste, and this answers `null`.
const ADDRESS = /^(https?:\/\/|mailto:|note:)\S+$/i;

export function linkOnPaste(selected: string, pasted: string): string | null {
  const address = pasted.trim();
  if (!ADDRESS.test(address)) return null;
  if (!selected.trim() || selected.includes('\n') || ADDRESS.test(selected.trim())) return null;
  // A `]` in the words would end the link's text early; a `)` in the address would end the
  // address. Both are written so that they do not.
  const words = selected.replace(/([[\]])/g, '\\$1');
  const target = /[()\s]/.test(address) ? `<${address}>` : address;
  return `[${words}](${target})`;
}
