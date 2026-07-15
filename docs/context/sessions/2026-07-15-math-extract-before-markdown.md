# 2026-07-15 — LaTeX in the read view: extract math *before* Markdown

**Symptom (owner):** `$$ P(\theta|u,m)P(u|m ) $$` doesn't render — "the theta
command isn't rendering and the backslash is in red." Recurring; the earlier
"GAE math not rendering" report was the same class.

## Investigation (what we ruled out)

- **Not the file/JSON round-trip.** A live `fm-serve` capture→get→disk check
  shows a *single* backslash everywhere (`$$ P(\theta|u,m)P(u|m ) $$`). fm-core
  writes the body verbatim; fm-serve uses `serde_json` (correct escaping both ways).
- **Not the render logic for that exact string.** Running the *real* pipeline
  (`marked` → real KaTeX auto-render) in jsdom produced a correct `.katex-display`,
  no error. That's why it passed every headless test — **the tests mock KaTeX**, and
  the real pipeline happens to handle the isolated example.
- **Assets/CSP fine.** All 59 KaTeX fonts + the CSS chunk are in `dist`, Vite's
  `mapDeps` injects the stylesheet `<link>`, fm-serve serves `.css/.woff2` with
  correct MIME, no CSP.

The red is KaTeX's own error color — it *ran* and failed on the owner's real note,
whose content differs from the pared-down example (subscripts, a stray `$`, …).

## Root cause (architectural)

Markdown ran **before** KaTeX, and auto-render then greedily walked text nodes.
Two failure modes this creates, both reproduced:

1. **Markdown mangles the formula** — `_`/`*`/`\` inside `$…$` are Markdown syntax
   (emphasis, escapes). Subscripts and commands get eaten before KaTeX sees them.
2. **A stray `$` mis-pairs the delimiters** — one lone `$` (currency: "costs $5")
   shifts every following delimiter, so real math renders as garbage or errors red.

## Fix — `ui/src/lib/render.ts`

Lift math out of the Markdown **source** first, so the parser can't touch it:

- **`extractMath(src)`** (pure, exported, unit-tested): replaces each `$$…$$`
  (display, matched first as unambiguous pairs) and `$…$` (inline) with an empty
  `<span data-math=i>` placeholder that survives `marked` untouched. Inline `$` is
  conservative — won't open on whitespace/digit (so `$5` stays literal), won't
  cross a `$$`, won't span a blank line; `\$` and code spans/fences are copied
  through so a `$` inside them is never math.
- **`renderMath(el, spans)`** now imports **`katex`** directly (not the
  `auto-render` contrib) and fills each placeholder via `renderToString`. On a
  parse error it keeps the **raw source** with `class="math-error"` + the KaTeX
  message as a `title` tooltip **and** a `console.warn` — visible and diagnosable,
  never a silent red blob. `.math-error` styled in `app.css` (dotted underline,
  `cursor: help`).

## Tests

- `render.test.ts`: mock is now `vi.mock('katex', …)` (was the auto-render path);
  new `extractMath` block pins subscripts, currency (`$5`, `$5–$10`), stray-`$`,
  code-fence `$`, escaped `\$`, and unterminated `$`. Plus a "broken formula keeps
  its source + tooltip" case. Same mock swap in `App.flow`/`App.features` tests.
- Green: svelte-check 0 errors, **69/69** vitest, prod build (auto-render no longer
  bundled; KaTeX still lazy-loaded as its own chunk).

## Still to confirm

The canvas/browser render can't be verified headless. If a formula still shows the
dotted-underline fallback, **hover it** — the tooltip now carries the exact KaTeX
message (or check the console `KaTeX could not render:` warning). That's the
ground truth for any remaining case.
