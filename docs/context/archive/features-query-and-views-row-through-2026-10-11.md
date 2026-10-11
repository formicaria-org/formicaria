# The *Query & views* row of `features.md`, as it stood through 2026-10-11

> **Archived, not maintained.** Moved out verbatim on 2026-10-11 to keep the always-read layer under
> its byte budget. Every claim in it is recorded, with its reasons, in `decisions.md#ui`. Where this
> disagrees with the code, the code wins.

| **Query & views** | ✅ | pure `fm-query` engine; board/agenda/timeline/search, generic literal-free renderers — **the feed is a place you read and discuss since 2026-08-31**: a post carries a char-capped
`excerpt` (several lines, markdown stripped, filled only by `recent()`) faded out with a mask, is an
`<article>` whose title+picture is the one button that opens the note, and carries a reply count
from a single `thread_roots` call — tap it and one discussion expands with the last three messages
and a box, flat and reply-only. Editing stays in the pane (one editor per note), bodies are never
Markdown-rendered in a list, and there is no double-tap; **the timeline has two densities since
2026-08-30**: a *feed* (default) showing each note as a post with its picture, its first line, its vault and its last editor, which is what makes a shared vault legible as one stream; and the original one-line *list*, switched per pane like the agenda's month/week/list. `.view` files — **a filtered view names what it leaves out and offers the unfiltered one; the board carries a column rail** (2026-08-24); **saveable from the app** — an arrangement you keep, not a query you write (2026-08-28), **deletable and renameable, recorded in git** (2026-08-30). **The whole authoring surface was withdrawn on 2026-08-31** — no save, rename, delete or board group-by anywhere in the UI, because it was reachable before anyone could learn what a view is; the commands and their tests stay, and a `.view` file written by hand or arriving over git still lists and opens, so the promised design plan is cheap to land. **The board's column rail went with it**, against advice: a filtered view still says what it leaves out, but a board that scrolls no longer says so | `overview.md` · `decisions.md#ui` |
