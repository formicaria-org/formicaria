# 2026-07-21 — Note customization, reuse features, and the single-＋ header

Continued from the same day's collaboration/discussions/header-declutter work. The owner asked for
a lightweight interface with "crisp customization" (colored text, diagrams, mindmaps) and then to
"implement the remaining wants". Grounded in multi-agent adversarial reviews (Obsidian/Logseq/Roam
failures, Obsidian/Quarto/JSON-Canvas successes, Mermaid/DOMPurify XSS CVEs). Everything below
shipped with `pixi run ci` green and was built+installed on **both** desktop and the real phone.

## What shipped

### Body decorations — closed-vocabulary, semantic-token, sanitize-safe
- New `ui/src/lib/render-vocab.ts` is the single source of truth: `TEXT_TOKENS`
  (`accent/info/ok/warn/muted`) and `CALLOUT_TYPES` (`note/tip/info/warning/danger/quote`).
- Three **marked v18 extensions** registered at module scope in `render.ts`, running *inside*
  `marked.parse` (upstream of the single DOMPurify pass): `==highlight==` → `<mark>`,
  `[text]{.token}` → `<span class="tk-token">`, `> [!type]` callouts → `<div class="callout …">`.
  Unknown token/type degrades to literal text / a plain blockquote.
- **Colors are semantic theme tokens, never hex/color names** — a note written today still reads
  right under a theme years from now, and a body can never carry arbitrary HTML/style (portability
  *and* the shared-vault XSS hole, closed at once). Mindmaps/trees are a Mermaid `graph TD` — no
  plugin, no bundle. Rule written into `docs/src/dev/design-system.md`.
- **XSS guardrails:** sinks tagged `// sink-ok` with the reason; a CI grep in `ci/checks.sh` fails
  on any new untagged `innerHTML`/`{@html}`/`insertAdjacentHTML` write, plus a Mermaid-never-`loose`
  grep. Mermaid stays `securityLevel:'strict'`; sanitize runs last.

### Whole-note embeds (transclusion)
- `![](note:<id>)` renders the target note **inline in a card** instead of a chip — a hub note can
  pull several others onto one screen. `resolveEmbeds` in `render.ts` recurses via `renderInto`
  with a `seen` cycle-guard and `MAX_EMBED_DEPTH=3`; a missing target degrades to a placeholder, a
  self/loop embed stops with a marker. Always the whole file (the file is the unit).

### Backlinks — "what links here", no reverse index
- `commands::backlinks(id)` scans note bodies via `refs::references` (the `copy_note` primitive);
  a note is a backlink when it references the target (a `note:` link *or* an embed), never itself.
  Same O(corpus) cost as `recent`/`thread` — **no index to keep true**, the files stay the truth.
- `NotePanel` shows a **Linked from** panel below the note when non-empty; a chip click opens the
  linker in a pane.

### Templates — "New from …", a template is just a tagged note
- `commands::templates()` = `notes_base().and(TagsAll(["template"]))`. **No new Kind, no reserved
  property** — tag any note `template` to make it a starting point, untag to unmake it; it stays an
  ordinary searchable note. Surfaced in the palette (Settings) as `New from "…"`; picking it reads
  the template body via `get` and captures a fresh untitled note with it. Only the **body** travels
  — the copy is not itself a template.

### The single-＋ options window + intuitive edit-exit (supersedes the morning's `⋯` overflow)
- The morning's `⋯` overflow was replaced (owner's later ask) by a **single `＋`** in the header
  opening an options *window* (a card, not a dropdown) holding Edit / Copy to… / Delete, dismissed
  by outside-click / Escape / ✕ (`clickOutside` capture-phase pointerdown — touch + mouse).
- **Bug the owner hit twice on the phone:** the window opened "in weird places, not close to the
  ＋". Root cause: it was `position:absolute` against the scrollable `.panel`, so once a note was
  scrolled it drifted to the panel's unscrolled top; a first fix moved it into the sticky header but
  still pinned it to the row's right edge while `＋` sits mid-row. **Final fix:** nest the window in
  a `position:relative` wrapper around the `＋` (the `＋ Media` shape) at `left:0; top:100%`, so it
  opens directly beside the button at any scroll offset.
- **Intuitive edit→read:** clicking the header chrome (title / identity line — anything but a
  control) now flushes the draft and returns to the read view, the twin of Ctrl+S / Escape. An
  accent underline + pointer cursor mark the header as "done" while editing. `onHeaderClick` excludes
  `button, input, a, select, .capture, .options-window`.

## Decision recorded

**Canvas note-type: not built (redundant).** A whole-note canvas already exists — **boards**
(`view: board`, Excalidraw in the body) with *structural 3-way merge* on desktop and phone
(`scene.rs`), which Obsidian Canvas / Miro can't do without a server. A second JSON-Canvas format
would duplicate it for no gain and violate "resist adding kinds". The owner chose **Stop — wants
done**: embeds + backlinks + templates are the substantive wants; richer tables/queries is small
incremental `.view` polish to do on demand, not a speculative build.

## Commits
`c62c1b6` header window + outside-click · `7b9403b` customization + XSS guards · `6ea1f98` embeds ·
`86bc69b` backlinks · `75e2446` header options-window (first fix) + click-header-to-finish ·
`ccd51ac` options-window anchored to the ＋ wrapper (final fix) · `908ef5d` templates.
