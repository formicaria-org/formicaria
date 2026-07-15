# Decisions — the *why*

Condensed, load-bearing decisions and reversals. Each entry is: **decision —
why — consequence**. The canonical, fuller spec is
[`formicarium/MASTERPLAN.md`](../../formicarium/MASTERPLAN.md); this is the
quick-recall version. Newest first.

## Browser is the product; the native window is removed (2026-07-15)
**Why:** the Tauri/WebKitGTK window never painted reliably on the developer's
box (blank/gray; mutter/X11 with no compositor). The same SPA rendered correctly
in a real browser, so the UI logic was sound — the webview was the problem.
**Consequence:** `fm-serve` (std-only HTTP) serves the built UI to the default
browser; `fm-app` became a **library only** (no `[[bin]]`, no tauri deps); the
pixi `gui` env, the `e2e/` WebDriver tree, and the deny.toml Tauri-RUSTSEC
waivers are gone. Modern CSS is now fine (real browser, not WebKitGTK). The old
"WebKitGTK blank-window" risk is retired.

## Files-as-truth; the atom is the file (foundational)
**Why:** durability and ownership — a note must survive as plain text without
this app. **Consequence:** one Markdown note = one file; frontmatter is a
generic YAML mapping so unknown/custom props survive read→edit→write with no
silent loss; key order is fixed so `to_file` is byte-idempotent. No per-block
ids/timestamps — block-level structure is explicitly out of scope.

## `fm-query` may never touch fs/db (the insurance policy)
**Why:** a pure query engine is what keeps the whole system testable, portable,
and honest about the seam between "data" and "storage." **Consequence:**
enforced by a compile-time boundary *and* a CI grep. Do not add `rusqlite`/
`std::fs`/path handling to `fm-query`, ever. Search works by: `Text` predicate →
FTS5 prefix `MATCH` loads only the hit ids → the pure engine applies the rest.

## Generic, literal-free renderers
**Why:** a theme/renderer must not encode a specific workflow's enum values, so
arbitrary property values (any status, any type) render + tint without code
changes. **Consequence:** no `todo/doing/done` in `ui/src/renderers/`; column
tints keyed by `[data-value]`, urgency by `[data-urgency]`, labels sourced from
helpers (`urgency.ts`). CI greps enforce it.

## v1 editor = plain `<textarea>` + rendered read view
**Why:** a live-preview CodeMirror 6 editor was assessed as the single biggest
build risk. **Consequence:** editing is a textarea over the literal bytes
(`update_body`, byte round-trip tested) with a separate rendered read view
(`render.ts`, `marked`→HTML, lazy KaTeX/Mermaid). CM6 live-preview is **deferred
to v2**.

## Content-addressed blobs, extracted text in the note body
**Why:** dedup + integrity (bit-rot = a blob no longer hashing to its filename),
and searchability before the git-ignored blob syncs. **Consequence:** blobs are
sha256 with `ab/cd` fan-out; ingest sniffs MIME (`infer`), runs `pdftotext` →
the **asset note's body** (git-tracked, FTS-indexed), and makes a thumbnail.
`put_bytes` dedups before writing.

## Markdown→HTML is JS `marked`, not Rust pulldown-cmark
**Why:** it shipped that way; the MASTERPLAN's "pulldown-cmark→HTML" line never
materialized (the crate is absent). **Consequence:** the only HTML assembly is
one `marked.parse()` in `render.ts`. Note the unsanitized-innerHTML gap in
[known-issues.md](./known-issues.md).

## No plugin API
**Why:** plugin APIs rot and become a compatibility burden. **Consequence:**
extend via modular Rust (add a renderer / store / extractor), declarative
`.view` files (planned), a one-file theme (design tokens), and an optional mlua
hatch — never a stable plugin surface.

## Tauri was the light choice; a native-GUI rewrite is rejected
**Why:** even before removal, Tauri used the system webview (no bundled
Chromium; ~9.6 MB release binary) — lighter than Electron (Logseq/Obsidian run
1–2 GB). **Consequence:** the only genuinely-lighter path (egui/Slint/iced) would
mean rewriting the entire Svelte + KaTeX + Mermaid read view — not worth it.
Now moot (browser), but do not propose a framework rewrite.
