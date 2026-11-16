// **Text brought in from another app on the phone** (`decisions.md` 2026-11-16, *the phone's Bring
// in*): what Android's Share hands over as text, or what a person selected in any app and chose
// *formicaria* for. It arrives as the web's pieces do: a link as a link, a passage as a quote with
// where it came from when that is known (Share often adds the page's address), and when.

const URL_RE = /https?:\/\/[^\s<>"']+/g;
const clean = (s: string) => s.replace(/[[\]]/g, '').replace(/\s+/g, ' ').trim();

/** The note text for a shared `text` (and the subject Share may give with it). */
export function sharedPassage(text: string, subject = '', when = ''): string {
  const t = text.trim();
  const urls = t.match(URL_RE) ?? [];
  const stamp = when ? `, ${when}` : '';
  // Only an address (often with the page's title before it): a link.
  const rest = urls.reduce((s, u) => s.replace(u, ''), t).trim();
  // A title reads like one (short, no closing punctuation); a sentence ends like one.
  const titleLike =
    !rest ||
    rest === subject.trim() ||
    (rest.length <= 120 && !rest.includes('\n') && !/[.!?…"”]$/.test(rest));
  if (urls.length === 1 && titleLike) {
    const title = clean(rest || subject) || urls[0];
    return `[${title}](${urls[0]})${stamp}`;
  }
  const quoted = t
    .split('\n')
    .map((l) => (l.trim() ? `> ${l.trimEnd()}` : '>'))
    .join('\n');
  const from = urls.length
    ? `[${clean(subject) || 'the source'}](${urls[urls.length - 1]})`
    : clean(subject);
  return from
    ? `${quoted}\n>\n> — ${from}${stamp}`
    : `${quoted}\n>\n> — shared from another app${stamp}`;
}
