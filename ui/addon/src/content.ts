// **The page side of formicaria's browser add-on** (`decisions.md` 2026-11-15). Injected only when
// the person presses *Keep* (`activeTab`), never into every page. It reads; it decides nothing and
// sends nothing — the background script does both.
//
// What it reads: the address and title, the structured data the page publishes for machines (JSON-LD,
// `<meta>`), the article text Mozilla's Readability finds, turned into Markdown by Turndown, what the
// person selected, and **a copy of the page as it is now** — the document with a `<base>` so its
// relative links still resolve, and its scripts left out: it is a copy to read, not a program to run.
import type { PageFacts } from '../../src/lib/web/clip';
import { readPage } from '../../src/lib/web/read';

/** What the background script asks for: everything it needs, read once. Called by the background
 *  right after injecting this file (`formicaria_content.collect()`), so no message can race it. */
export function collect(): PageFacts & { snapshot: string } {
  return readPage(document, location.href, window.getSelection()?.toString() ?? '');
}
