# Decisions — the *why*

Condensed, load-bearing decisions and reversals. Each entry is: **decision —
why — consequence**. The canonical, fuller spec is
[`formicarium/MASTERPLAN.md`](../../formicarium/MASTERPLAN.md); this is the
quick-recall version. Newest first.

## Assets are query-layer-excluded from the planning views (2026-07-16)
**Why:** an asset is a blob a note *references*, not a thing you plan; a PDF
getting its own board card and timeline entry was noise. Filtering in each
renderer was rejected — it must be repeated per view and silently forgotten by
the next one. **Consequence:** `board`/`agenda`/`recent` carry
`Predicate::Kind(vec![Kind::Note])`; `FileStore` delegates structured predicates
to `fm_query::run`, so this cost no storage-layer change. **`search` and `gallery`
still see assets on purpose** — search (which indexes extracted PDF text) is the
only way to *find* an asset, and it is what backs the `/` menu's asset insertion,
so filtering it too would make ingested files unreachable. Two knock-ons: a board
grouped by `type` can now only answer `note` (the old
"falsifies-the-thesis" test was rewritten to pin the new rule; the generic
group-by claim still lives in `board_by_a_custom_property_…`), and the timeline's
type pill became dead — the status chip took its slot.

## Status rotates through the vault's own values; card order is a view preference (2026-07-16)
**Why:** setting a status meant entering edit mode and *typing* it. The obvious
fix — a `<select>` of `todo/doing/done` — would hardcode one workflow's enum into
the UI, which "generic, literal-free renderers" exists to prevent.
**Consequence:** `status.ts`'s `nextStatus(current, known)` cycles the statuses
`App.learnStatuses` already collects from fetched data, plus unset (so rotating
can always clear, with no separate control) — the chip renders any workflow and
the CI literal grep stays green. Typing a *new* value stays the Details editor's
job; it joins the cycle once a note carries it.
Separately, a card's **position within a column** is remembered in
`localStorage['fm-card-order']`, not in the note. **Rejected:** an `order`
property in frontmatter — it would rewrite a note file on every drag, and where a
card sits on your board is a view preference, not knowledge. Same reasoning, and
the same per-browser no-sync caveat, as the existing column order.

## `start`/`due` are a `Stamp` (day + OPTIONAL time), not a Date/DateTime pair (2026-07-15)
**Why:** the owner needs meeting times ("a time option beyond the date"), but an
all-day deadline must stay expressible and must not churn on disk. Two obvious
designs were rejected:
1. **Reuse `PropertyValue::Date` + `DateTime`.** `PropertyValue` derives `Ord`
   from **variant order first**, so every all-day item would sort before every
   timed item regardless of the actual day — silently wrecking agenda order,
   `Op::Lt/Gt`, and group bucketing. This is a trap, not a preference.
2. **Make `due` an `OffsetDateTime`.** RFC 3339 demands an offset; a wall-clock
   intention doesn't have one. It would also force a time on every deadline.

**Consequence:** `fm_model::Stamp { date, time: Option<Time> }` — naive (no
offset: 14:30 means 14:30 where you are, and the file is the truth), minute
granular, one `PropertyValue::Stamp` variant. `Display`/`FromStr` are **inverses**,
which is load-bearing twice over: it keeps `to_file` byte-idempotent (a bare date
in, a bare date out — no vault-wide churn), and it keeps the board's drag
write-back lossless (`value_string` → `display()` → `apply_property`; the old
`DateTime` display dropped the time, so a drag would have erased it). `Stamp` owns
the format on both sides, so the date literal is no longer duplicated across
crates. **Urgency stays day-granular** — a time is presentation, not priority.
`created`/`updated` are unchanged: they are *instants*, so they stay
`OffsetDateTime`/RFC 3339, and the UI's `parseStamp` is anchored so it can never
match one and hand back a UTC day.

## Whiteboard = embedded Excalidraw, lazy-loaded (2026-07-15)
**Why:** the owner wanted a real "drawio but simpler" freeform canvas, not a
diagrams-as-code stand-in, and chose full-featured-fast over build-it-minimal.
**Consequence:** a deliberate reversal of "no new UI runtime dependency" —
`@excalidraw/excalidraw` + `react`/`react-dom` are now deps. Mitigations keep the
base app lean: the editor is **lazily imported** in `Whiteboard.svelte` (React
root mounted via `createElement`, no JSX → no build plugin), so it's a separate
~744 KB-gz chunk that downloads **only when a board opens** — base `index.js`
stays ~34 KB gz (same discipline as KaTeX/Mermaid). **A board is just a note**
with a `view: board` property whose **body is the Excalidraw scene JSON** — no new
`Kind`, no backend change, files-as-truth intact (the `.excalidraw` JSON is plain
text on disk). **Caveats:** Excalidraw fetches fonts from a CDN unless
`EXCALIDRAW_ASSET_PATH` is set — offline it degrades to fallback fonts (local-font
bundling deferred); and the canvas only renders in a real browser, so it's
**unverified in headless CI** (build + code-split + round-trip are verified).

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

## Backup is two tiers: git push (default) + restic (opt-in)
**Why:** the vault holds two data classes with nothing in common. Notes are
small, plain, mergeable → git carries them anywhere, authenticated by the user's
own ssh-agent/credential-helper, so **the app stores no secret**. Blobs are heavy
and git-ignored → only restic sees them. The button used to run restic
unconditionally, which was *broken-by-default*: it needs `FM_RESTIC_REPO` +
`RESTIC_PASSWORD` and `packaging/formicarium.sh` never sets them, so every
desktop-icon launch errored. This is the settled split elsewhere (Zotero syncs
metadata and file attachments as separate tiers; git-annex/git-LFS put a pointer
in git and content in special remotes; photo managers back up a small catalog and
hand bulk originals to a file tool). **Consequence:** `BackupPanel.svelte` owns
the vault's remote (stored in the vault's own `.git/config` — no new config
file, and decoupled from the app's source remote by construction). **A push
carries notes only**: media is off-sited only when the box is ticked, stated in
the panel rather than tracked. `manifest.json` is git-tracked, so a git-only
restore still knows its blob inventory and `fm verify` names what is missing.
`destination.ts` classifies both a git URL and a restic repo as local/remote —
a local path is a legitimate destination but must never be reported as "off this
machine".

## Squash-on-push — a deliberate reversal of "don't build commit management"
**Why:** `MASTERPLAN.md:411,447` accept thousands of `auto:` commits as the price
of undo and say *don't build commit management*. That holds for the **local**
repo, but the remote is a different audience: auto-commit fires every few seconds
of editing, so pushing raw would make the GitHub history unreadable.
**Consequence:** `git::push_squashed` collapses the unpushed window into one
`backup:` commit (`reset --soft <tracking-ref>` + commit). Two constraints are
load-bearing:
- **Never squash the first push.** With no tracking ref, "unpushed" means the
  *entire* history — destroying history that has never left the machine is
  exactly backwards. First push sends it whole; every later push is one commit.
- **Never fetch in this flow.** The tracking ref is stale on purpose: a remote
  another machine moved then stays ahead of it, so our push is *rejected* and the
  user is told (verified). Fetch first and the `reset --soft` would rebase onto
  their tip and silently overwrite their content with our tree.

**Accepted cost:** granular undo only reaches back to the last push; before that
each push is one step. This narrows (does not remove) the undo auto-commit buys.
