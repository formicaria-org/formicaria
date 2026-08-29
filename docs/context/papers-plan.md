# A papers app for formicaria — plan, revised after adversarial review

_This is the on-demand topic doc for the papers/PDF-annotation direction, reached from
[overview.md](./overview.md)'s router. It lived outside the repo until 2026-08-29, which meant a
cold session could reconstruct the shipped bug fixes and none of the direction — see the note on
the record at the end of Part 4._

How this got here, and what three audits found underneath it:
[`sessions/2026-08-29-papers-and-what-three-audits-found.md`](./sessions/2026-08-29-papers-and-what-three-audits-found.md).

**Owner's rulings (2026-08-29), which govern everything below:**
- **The papers tool is a separate app** over the existing command seam, not a feature inside the
  notebook. The counter-argument is in Part 5, and it was overruled — read it before revisiting.
- **Phase A first, and only.** Phases B and C-groundwork were authorised afterwards, in order.

_Written 2026-08-29. **Revision 2**, after four adversarial reviews (efficiency, maintainability,
non-technical usability, Zotero portability). Revision 1's design is superseded; what it got wrong is
recorded in Part 5 rather than deleted, because the errors are instructive._

---

## Context

The owner is a researcher who tracks papers in Zotero and wants:

> download a paper → highlight words, sentences, **figures**, **formulas** → attach notes *anchored
> to a location in the paper* → and have those notes be **formicaria notes**.

Then, in the owner's own reframing:

> *"a separate tool from formicaria that can be attached to the core… our own Zotero-like app…
> replicating its functions but following our fm philosophy and API"*, plus a browser extension.

And a hard requirement added afterwards: **"it should eventually allow to port entire libs from
Zotero."**

**The review's headline: this plan found more shipped bugs than plan flaws.** Six defects exist in
the current codebase today, three of them silently lying to the user, and the plan quietly assumed
all six were absent. They are worth fixing whether or not a papers app is ever built.

---

## Part 1 — The landscape (validated; unchanged in substance)

### 1.1 Zotero

- **Zotero 9** (2026-04-10), **Zotero 10** (2026-08-17; 10.0.1 on 08-24). Both dates confirmed.
- **Reader**: PDF + EPUB + HTML snapshots. Annotation types: highlight, underline, note,
  **image/area**, **ink**, text.
- **Annotations are items**, not PDF content: `annotationType`, `annotationText`,
  `annotationComment`, `annotationColor` (an 8-colour palette), `annotationPageLabel`,
  `annotationSortIndex`, `annotationPosition`. **These fields are not documented in the Web API v3
  docs** — any importer is reverse-engineering the client and must pin the shape in tests.
- **Position is an unbounded structure**: `{ pageIndex, rects?: number[][], paths?: number[][],
  nextPageRects?: number[][] }` for PDFs; a **W3C `FragmentSelector` carrying an EPUB CFI** for
  EPUBs; a `CssSelector` + `TextPositionSelector` for snapshots.
- **Local HTTP API** `http://localhost:23119/api/` — offline, no rate limits, **off by default**
  (Settings → Advanced). **Writes are new in Zotero 10** (`POST /api/local/authorize`,
  `Zotero-Server-ID` echoed back). **Results are not paginated by default.**
- **Attachment bytes arrive as a `302` to a `file://` URL** — the importer is a filesystem reader,
  not an HTTP client.
- **The Connector** (~700 site translators) is the real moat.
- **Direct SQLite**: read-only only; schema changes between releases; Zotero 10 turned on WAL.

### 1.2 The PKM tools that tried this

**Obsidian PDF++** is the closest analogue — highlight → writes a markdown link with a selection
fragment, then **renders every backlink as a highlight**. Author warns of breakage on any Obsidian
update. **Obsidian Annotator** (hypothes.is in a box) is broken on modern iOS. **Logseq** uses an
`.edn` sidecar per PDF that is opaque and has lost highlights in the field. **Heptabase**'s
highlight-becomes-a-card-on-a-whiteboard is the best product idea in the space. **Zotero Better
Notes** proves two-way markdown sync is achievable. **~10 more Obsidian plugins** are all export
pipelines. **Qiqqa, Docear, Polar** — all-in-ones that owned metadata *and* reading *and* notes —
are all dead.

### 1.3 The "why": the pain is the seam

One-way import overwrites prose; corrections never flow back; volume is unmanageable ("a single book
may have over 100 highlights"); even zendron concedes only one-way sync works.

### 1.4 Anchoring

W3C's model (quote + position selectors) with hypothes.is fuzzy repair is the general answer.
**Formicaria needs none of it**: a PDF is a content-addressed immutable blob, so a changed PDF is a
*different blob* and geometry cannot drift. Keep the quote as provenance, not repair.

### 1.5 Embedded vs sidecar

Writing highlights into the PDF **breaks content addressing** — every highlight changes the hash.
Ruled out. An opaque sidecar loses plain-text readability. **The note is the store.**

### 1.6 Ingestion without a connector

arXiv Atom (already in-repo, plain HTTP), Crossref/OpenAlex (free for single DOI lookups), the DOI in
the PDF's own text, and `citation_*` meta tags. `hayagriva` (Rust, CSL, 2,600 styles) is named and
deferred.

---

## Part 2 — Six live bugs the plan assumed were absent

**These are defects in the shipped app today.** Each was found independently during review and
verified in code. They are listed first because several are worth fixing on their own merits.

| # | Bug | Evidence | Why it matters here |
|---|---|---|---|
| **B1** | **PDFs do not render, and two user-facing strings claim they do.** `frame-src 'none'` blocks every frame source including `'self'`; the PDF path is an `<iframe>` | `fm-serve/src/main.rs:1168`; `mobile/src-tauri/tauri.conf.json:21` (same); `ui/src/lib/render.ts:385-391`. False claims at `ui/src/lib/SettingsPanel.svelte:699-701` and `packaging/README-release.txt:110-111`. Absent from `known-issues.md`. `render.test.ts:110` asserts the iframe **in jsdom, where no CSP applies** | The exact class `decisions.md:80-92` was written to close. The whole feature starts from a dead viewer |
| **B2** | **`assets` has no write path and is silently discarded.** `apply_property` has no `assets` arm → falls to `extra` as a scalar `Text` → `from_file`'s `as_string_seq` returns an empty vec on the next load | `fm-core/src/edit.rs:52-57`; `frontmatter.rs:174, 235-240`. `obj.assets` is written in exactly one place: `commands.rs:1294`, by `ingest`, on the asset note it creates | **Silent data loss today.** And it means the plan's central link — paper note → PDF blob — *has no write path at all*. `code` has the same gap |
| **B3** | **Every planning view calls `load_all()`.** `candidates` pushes only `Predicate::Text` to SQL; `Kind`, `Prop`, `NoteRef` are in-memory residuals | `fm-core/src/file.rs:696-701`, `892-915` | Harmless at 1 KB/note prose. Fatal at 2.5 KB/page of PDF text — see Part 3 |
| **B4** | **The phone's blob route has no `Range` and buffers whole files, while its own comment says it streams** | `mobile/src-tauri/src/lib.rs:233-234` (the comment) vs `:236-247` (the code). Desktop does it correctly at `fm-serve/src/blob.rs` | pdf.js's partial-load design is off on the phone; a 40 MB scan is a 40 MB `Vec` plus wry's copy |
| **B5** | **Bulk ingest holds the global app lock across two subprocess spawns per file** (`pdftotext`, `vipsthumbnail`) | `fm-app/src/dispatch.rs:1618-1620`; `ingest` is absent from the fast list at `:455-478`, and unlike `run_backup` (`:2158-2161`) never drops the guard | A 5,000-PDF import freezes every tab for 25–40 minutes |
| **B6** | **No bundle-size gate and no npm licence gate exist.** `ci/checks.sh`'s only size budget is on `docs/context/` line count; `deny.toml`/`third-party.sh` are keyed on `Cargo.lock` and never see npm | `ci/checks.sh:376`, `:127-155`; `ci/third-party.sh:24-27` | The eager JS payload today is **~70 KB gz**; one stray top-level import moves 700+ KB into it, green. And `marked`, `katex`, `mermaid`, `dompurify`, `excalidraw`, `react` are **already** baked into the binary and absent from `THIRD-PARTY.md` |

---

## Part 3 — What the reviews broke in the plan

### 3.1 The anchor format was wrong (found twice, independently)

`[p.4](asset:sha256-…#page=4&rect=68,246,174,267)` carries **one** rect. Zotero's is `rects` —
`number[][]`. **A highlight crossing a line break is 2+ rects, which in a two-column paper is the
common case.** Revision 1 quoted the plural form in its own research and then designed the singular.

It also has no home for: `nextPageRects` (a highlight spanning a page break), `paths` (ink),
EPUB CFIs, `annotationSortIndex` (three distinct formats), colour, per-annotation tags, or
`pageLabel` (printed, e.g. `"iv"`, `"S3"`) as distinct from `pageIndex` (0-based) — which revision 1
conflated. And Zotero itself splits oversized annotations into multiple items
(`ANNOTATION_POSITION_MAX_SIZE = 65000`), so one user highlight can arrive as N items to re-join.

### 3.2 "The existing code already handles this ref" was false in three places

- **Nothing resolves an `<a href="asset:…">`.** `resolveAssets` walks `img` only
  (`render.ts:315-318`); `resolveNotes` matches `href^="note:"` (`:441-443`). `URI_ALLOWED`
  *preserves* the `asset:` scheme through DOMPurify, so the anchor survives as a **live link to a
  dead scheme** — `render.ts:474-476` documents this exact trap for `note:` and solves it with a
  `<button>`, warning that "a real href would send the webview to a dead scheme."
- **`parse_ref` rejects the fragment outright** — `commands.rs:1099` requires all-hex. `asset_status`,
  `resolve_asset_bytes` and `GET /api/blob` all route through it.
- **An anchored *image* ref breaks today**: `![fig](asset:sha256-…#page=3&rect=…)` reaches
  `parse_ref`, fails, and renders as an `.asset-missing-inline` placeholder.

Only `refs.rs` — the one call site revision 1's verification plan tested — actually works.

### 3.3 Stage 0 was not shippable, in both its deliverables

- **`save_view` has no filter parameter.** `views.rs:460-490`; its own doc at `:455-459` says
  *"editing a filter stays a file-level job"*. A "Papers view" can only be hand-written in a text
  editor — the exact `outstanding.md §2.6b` failure the owner has already ruled on.
- **No metadata editor exists.** The props panel is six fixed fields — Status, Start, Due, Hard,
  Title, Tags (`NotePanel.svelte:2037-2112`, `:157-164`). The open props map is read in exactly two
  places, neither a display. `authors`/`year`/`venue`/`doi` would be written and then **invisible**.

### 3.4 Three data-model traps

- **Search returns the wrong note.** Extracted PDF text is the *asset* note's body, titled by
  filename (`commands.rs:1292-1294`), so a phrase search returns `1706.03762v7.pdf`, not the paper.
  Revision 1's verification asserted the search passes — it does, and proves nothing.
- **`year` sorts into two disjoint blocks.** Typed in the app → `Text` (`edit.rs:53-58`); parsed from
  YAML → `Int` (`frontmatter.rs:259-263`). `Ord` compares **variant before value**
  (`fm-model/src/lib.rs:48-57`, which documents this trap by name).
- **Tags split on spaces** (`edit.rs:31`), so a "Machine Learning" collection ports as two unrelated
  tags. Multi-word tags are unrepresentable through the only write path.

### 3.5 The satellite argument was weaker than stated, and miscited

Revision 1 attributed *"native browser elements… no JS media libraries"* to `decisions.md:2102`.
**That is the v1-editor entry and says no such thing.** The rule's only home repo-wide is
`MASTERPLAN.md:341`; no `decisions.md` entry records it.

And the three properties that made the `fm-agent-run` ruling safe are all absent: that ruling is a
**Cargo feature, off by default** (`Cargo.toml:32-33`), on a **separate binary that drags in no
fm-core**, gated by `deny.toml` + `third-party.sh`. Here there is one `package.json`, no JS feature
gate, no npm licence gate (B6), and `fm-serve/build.rs:25,70` `include_bytes!`-bakes `ui/dist` into
the **one** shipped binary — `decisions.md:1760-1767`: *"the binary alone **is** the app."* "The
notebook never loads pdf.js" is true at runtime and false at ship time.

**`/manual/` is a category error as a precedent**: static mdBook output with no `ipc`, no components
and no design system, whose separate CSP exists *precisely because it is not the app*.

The general principle "declare a new route, admit any dependency" does not distinguish pdf.js from
`zotero/reader`, video.js, Monaco or tesseract.js — and Part 6 already queues `zotero/reader` behind
the same door. Under `CLAUDE.md`'s four questions this **contradicts** and owes a dated reversal with
a `> SUPERSEDED` banner, not a scoping entry.

### 3.6 Promotion-as-designed floods every view and taxes every poll

`thread.rs:10-15` records the trap verbatim: *"the founding ruling enumerated three surfaces… and by
the time it was built there were five… and `activity` was duly forgotten"*; `ci/checks.sh:30-45`
guards `thread::notes_base()` having one definition. Revision 1 promised promoted annotations were
"taggable, boardable, linkable" — which floods every board, `.view`, `recent`, `timeline`, `activity`
and search result with highlight fragments, reproducing by construction the volume disease it
diagnosed in competitors.

It also costs: `path_for` is one flat notes directory, and `reindex` does `read_dir` + a `stat` per
entry **every 15 s while a tab is visible** (`file.rs:744-756`), against a 500 ms budget at 10k.

And the `annotates` refusal argument was backwards. The real reason at `edit.rs:40-46` is that *the
presence of the key hides the note*, so a board drag erases it from every view. `thread.rs:41-47`
proves the corollary: `proposes`/`targets`/`declined` are **not** in the refusal list because
proposal notes are excluded from every board. **So the refusal is only needed if annotation notes are
board-visible — and if they are, §3.6's flooding bites.** Revision 1 wanted both.

### 3.7 The browser extension design does not work

`Scope` is `All | Only(Vec<String>)` — **vault-scoped only** (`fm-app/src/scope.rs:22-30`). A
command-level scope does not exist and would be a new authorization axis through the one door.
Worse, an extension service worker on `127.0.0.1` takes the **`peer.loopback`** branch, which checks
origin membership only (`main.rs:371-373`) — **pairing tokens are the non-loopback path**, so "pair it
like a tablet" never reaches the code that rejects it. And `share::pair` is *"the only unauthenticated
write in the server"*, living on the **shared TLS listener on its own port**. Saving a PDF would
require turning on LAN sharing.

**Replacement, and it is better anyway: a paste box.** "Paste a DOI, arXiv ID, or URL." No install,
no pairing, no daemon, no new authorization axis.

### 3.8 Scale: the binding constraint is bytes per note, and every budget is blind to it

Measured with this repo's pinned poppler: 1.7–2.2 KB of extracted text per page (dense two-column
runs 2–4 KB). **5,000 papers × 12 pp × 2.5 KB ≈ 150 MB of note body**, hydrated by `load_all()` (B3)
on every view switch → ~300 MB transient, 0.5–1.5 s of pure memcpy per request. `perf.rs` seeds
55-byte notes, so it pins note *count* and cannot see a 30× change in bytes.

Storage compounds: `objects.content` holds the file **and** the fts5 shadow table holds the text
again (not `content=''`), so `index.sqlite` reaches ~400–500 MB — rebuilt from files on **every**
desktop start (`dispatch.rs:160`; `known-issues.md:344-355` forbids extending `TrustIndex` to
desktop), and `FM_AUTO_SHUTDOWN` makes "every start" mean "every time you close the tab."

**Verdict: ~500 papers is indistinguishable from today; ~1,500 puts every view switch at 150–400 ms;
~3,000 is a visible stall; past ~3,000 the phone is out entirely** — below the owner's stated floor.

**`-bbox-layout` is dropped.** Measured at **19–22× the plain text** (~42 KB/page; 2.5 GB for 5,000
papers), with nowhere to store it — and redundant, because pdf.js supplies the text layer
client-side. Revision 1 sold it as "no new dependency", which was true and irrelevant.

**pdf.js's real cost**: ~330 KB gz + ~410 KB gz worker **plus ~2.8 MB of `cmaps/` and
`standard_fonts/`** that must ship or CJK and non-embedded-font PDFs render blank — against a 26 MB
Android package. Revision 1's "~2 MB gz" was wrong.

### 3.9 Blobs do not travel — so a ported library syncs its text, not its papers

`ensure_repo` gitignores `blobs/` unconditionally (`git.rs:737`); `git_assets_max` defaults to `None`
= *"notes travel, media does not"* (`descriptor.rs:39-51`). Turn it on and the 5-second debounced
`commit_all` walks and `stat`s **every blob** then `git add`s them all (`git.rs:752-766, 872-880`) —
at 20,000 blobs that is ~1.6 MB of argv against a typical 2 MB `ARG_MAX`, presenting as *"commit
silently stopped working."* Restic is the only tier carrying blobs, its first snapshot of 10–50 GB
runs 20 minutes to hours **inside one blocking HTTP request with no progress and no resume**
(`backup.rs:74-104`), and **restic does not exist on Android**.

### 3.10 The Zotero port: what actually survives

**Bibliographic metadata ports at ~85–90% of field *content* and near 0% of field *structure*.
Annotations port at ~25–35% with the anchor format as written. EPUB, snapshot and ink annotations
port at 0%. Collection hierarchy ports at 0%.**

The decisive finding: **every export route drops annotations.**
`Zotero.Translate.ItemGetter.setAll` has a literal `case 'annotation': return false` upstream of
*every* translator — Zotero RDF, CSL JSON, BibTeX, RIS. The "Include Annotations" checkbox is a
*file* option that **bakes annotations into a rewritten PDF**, which changes its bytes and therefore
its sha256 — the migration would break content addressing by itself. **The local API is the only
viable path.**

Second decisive finding: **area-annotation images may not exist to copy.** They are cached at
`<dataDir>/cache/…/<key>.png` and, per Zotero staff, are not synced and *"won't generate images for
those on a given computer until you open the PDF in the reader"*. **The importer must render the crop
itself** — so the port depends on the reader's render pipeline, not the other way round.

Three more blockers, all in current code: `created`/`updated` are refused by `apply_property`
(so all 5,000 papers are dated import day, breaking `recent`, `stale`, timeline and ULID order
permanently); there is **no home for the Zotero key**, so a second import duplicates everything; and
`capture` takes only `(body, vault)`, so an import is `capture` + ~12 `set_property` + `update_body`
per item ≈ **65,000 dispatch calls**, each a full file rewrite plus index update, not restartable.

---

## Part 4 — THE PLAN: Phase A only

**Owner's rulings, 2026-08-29:** the papers tool will be a **separate app** (Part 5's counter-argument
noted and overruled — it is recorded, not deleted), and **the work now is Phase A only: fix what is
broken.** Phases B–E below are the retained forward direction, not this plan's scope.

Ordered so each step is independently committable and independently defensible.

### A1 — A PDF must render, or the app must stop saying it does *(bug B1)*

**Two candidate blockers; settle empirically before choosing a fix.**
1. `frame-src 'none'` (`fm-serve/src/main.rs:1168`; `mobile/src-tauri/tauri.conf.json:21`).
2. Blob responses carry `Content-Security-Policy: default-src 'none'; sandbox`
   (`fm-serve/src/blob.rs:88,116`), and `sandbox` applies to the **framed document** — which may
   break the browser's built-in viewer on its own. `render.ts:350-364` already notes that viewer
   needs `allow-scripts allow-same-origin`.

**Relaxing `frame-src` to `'self'` grants note bodies nothing.** `sanitize()` overrides only
`ALLOWED_URI_REGEXP` (`render.ts:110-115`), so tags fall back to DOMPurify's defaults, which exclude
`<iframe>`. The only iframe on the page is the one `render.ts:386` builds **after** sanitizing.
State that reasoning in the commit; do not let a future reader assume the clause was loosened
casually.

**Do first, unconditionally, even if the render fix is deferred:** correct the two false strings —
`ui/src/lib/SettingsPanel.svelte:699-701` and `packaging/README-release.txt:110-111`, both claiming
*"PDFs are still stored, opened and shown"* — and add a `known-issues.md` entry. A capability the app
declares and does not have is the exact failure `decisions.md:80-92` exists to prevent.

**Test:** the current `render.test.ts:110` asserts the iframe in jsdom, **where no CSP applies** — it
passed throughout. Whatever the fix, it needs a test that would have failed.

### A2 — `assets` and `code` are silently discarded on write *(bug B2)*

`apply_property` (`fm-core/src/edit.rs:52-57`) has no arm for either, so a value falls through to
`extra`, `to_file` writes it as a **scalar**, and `from_file`'s `as_string_seq`
(`frontmatter.rs:174, 235-240`) returns an empty vec on the next load. No error anywhere.

Add both arms. They are `Vec<String>`, so follow the `tags` arm's shape — **but do not copy its
space-splitting** (`edit.rs:31`): an asset reference cannot contain a space, and splitting on space
is itself the bug that breaks multi-word tags later. Split on comma only.

**Test:** `set_property(id, "assets", "sha256:…")` → reload → the value is still there. This is a
live data-loss bug, so it earns a regression test regardless of the papers work.

### A3 — A Details panel that shows and edits arbitrary frontmatter keys *(the §2.6b repair)*

Today the props editor is six fixed fields (`ui/src/lib/NotePanel.svelte:2037-2112`, state at
`:157-164`), and the open props map is read in exactly two places, neither a display. Any other
frontmatter key is written to the file and then **invisible in the app** — reachable only by opening
the `.md` in a text editor, which `outstanding.md §2.6b` rules out.

**Types must be preserved on the round trip.** `year` typed in the app becomes `Text`
(`edit.rs:53-58`) while `year` parsed from YAML becomes `Int` (`frontmatter.rs:259-263`), and `Ord`
compares **variant before value** (`fm-model/src/lib.rs:48-57`, which documents this trap by name) —
so a mixed corpus sorts into two disjoint blocks. The panel must round-trip a value to the same
variant it read, or this ships as a silent sorting bug.

### A4 — `save_view` cannot write a filter *(the other §2.6b repair)*

`views.rs:460-490` takes `(vault, name, view, group_by)` and nothing else; its own doc at `:455-459`
says *"editing a filter stays a file-level job"*, and it **refuses** to overwrite a hand-written
filtered view rather than drop the filter. So any filtered view is a hand-edited YAML file.

Scope this carefully against `decisions.md`'s ruling — *"A saved view is an arrangement you keep, not
a query you write"* — which is a standing decision about **not** shipping a query builder. Writing
back the filter the user already has (from the current view's state) is not a query builder; read the
entry before designing the surface.

### A5 — Push `Predicate::Kind` into `FileStore::candidates` *(bug B3)*

`candidates` pushes only `Predicate::Text` to SQL via `fts_match_expr`; `Kind`, `Prop` and `NoteRef`
fall through to `load_all()` (`fm-core/src/file.rs:696-701`, `892-915`). Add a `Kind` pushdown beside
the existing one — roughly ten lines — so a `notes_base()` view never hydrates an asset note.

Measured payoff: extracted PDF text is 1.7–2.2 KB/page and lives in the asset note's **body**
(`commands.rs:1292`), so this is the difference between hydrating a library's full text on every view
switch and not touching it.

### A6 — A perf budget that can see bytes *(makes A5 provable)*

`crates/fm-core/tests/perf.rs` seeds `format!("routine note number {i} about gradients and
estimators")` — ~55 bytes. It pins note **count** and is structurally blind to body size.
Add a budget seeded with realistic bodies that **fails before A5 and passes after**. Without this,
A5 is an unverifiable claim and the next change silently undoes it.

### A7 *(optional, decide when the others land)*

Each is a real defect, none blocks the papers work, and each is defensible alone:
- **B4** — the phone blob route buffers whole files with no `Range`, while `mobile/src-tauri/src/lib.rs:233-234`
  comments that it streams. Fix the code or the comment; a comment that conceals a bug is worse than neither.
- **B5** — `ingest` holds the global lock across `pdftotext` and `vipsthumbnail`
  (`dispatch.rs:1618-1620`); it is absent from the fast list at `:455-478` and never drops the guard
  the way `run_backup` does at `:2158-2161`.
- **B6** — no bundle-size gate (the eager JS payload is **~70 KB gz** today) and no npm licence gate.
  The second matters more now that a **separate app** is the agreed direction: `deny.toml` and
  `third-party.sh` are keyed on `Cargo.lock` and will never see pdf.js.

---

## Retained forward direction (not this plan's scope)

### Phase B — the library

Paper = `Note` (forced by `views.rs:148-150`), flat metadata written through A3 with consistent
types. Tag handling must not split on spaces, or collections need a path-shaped convention. Board by
`status`, agenda by `due`, tags, git, phone, backup — all free. BibTeX copy is a `format!`.
**Acquisition is a paste box**: DOI / arXiv ID / URL, plus the DOI regex over extracted text.

**One structural change belongs here, not in Phase A:** move extracted PDF text out of the asset
note's **body** into `derived/<hash>/text.txt`, indexed into FTS from there. `derived/` already holds
thumbnails, and this is derived data by definition. It fixes 150 MB note bodies, git size, launch
time, and search returning `1706.03762v7.pdf` instead of the paper. It is **not** in Phase A because
today's behaviour is deliberate (`overview.md:196-201` — search sees assets on purpose, and that is
how you find a PDF); it only becomes wrong once a separate paper note exists. It needs a migration.

### Phase C — the reader *(desktop only, stated as such)*

pdf.js with `isEvalSupported: false`, **and** the `cmaps`/`standard_fonts` assets. The anchor format
must be **multi-rect, versioned, and explicit about its coordinate space** before a line is written.
An `<a href="asset:…">` resolver and a fragment-tolerant `parse_ref` are prerequisites (3.2).

### Phase D — annotations as notes *from the start*

Per the portability review: a highlight-as-prose-block can never hold multi-rect, ink, CFI, colour,
tags or `sortIndex`. So an annotation is its **own note** with a fenced block carrying the verbatim
position JSON, and the human-readable anchored link *derived* from it for graceful degradation.
**They must be excluded from planning views via `thread::notes_base`** (3.6) — which then means
`annotates` needs no `apply_property` refusal, and the "boardable" promise is withdrawn.

### Phase E — the Zotero port *(PDF-only; say so)*

Local API only. Needs: an atomic, idempotent **`import_item`** command (title + body + tags as a
*list* + properties + asset hashes in one call); a reserved **`source_key`/`source_library`**; and an
import path permitted to set `created`/`updated` from `dateAdded`/`dateModified`. Explicit `limit`/
`start` — the local API does not paginate by default. Attachments arrive as `file://` redirects.
**Depends on Phase C's crop renderer** for area annotations.

### Dropped
The browser extension (3.7 — replaced by the paste box), `-bbox-layout` (3.8), area-select on touch
(HTML5 drag never fires on touch — `Board.svelte:174-177` already learned this and needed a tap menu).

---

## Part 5 — The packaging decision, and the argument it overruled

**Decided by the owner, 2026-08-29: a separate app.** Recorded here in full because the reviews
argued the other way and a decision is worth more with its counter-argument attached.

The maintainability review's verdict was to **cut the satellite** and put the reader in the notebook
as a fourth lazy chunk beside KaTeX, Mermaid and Excalidraw — costing one narrow reversal ("the read
view uses native elements except where the native element provides no text layer") and buying zero
duplicated UI and no second design system. Its measured case: `NotePanel.svelte` is **142 KB**,
`App.svelte` **106 KB**, `ipc.ts` **43 KB**, plus `render.ts`'s sanitizer with three audited HTML
sinks and `app.css`'s tokens. Import them and the satellite inherits marked + katex + mermaid +
dompurify — the weight the framing existed to avoid. Fork them and `ci/checks.sh:102-114` ("no
unaudited HTML sink in the UI") plus the literal-free-renderer grep must be widened by hand, or the
second app is unguarded against the stored XSS the first is guarded against — from note bodies that
arrive from collaborators through the merge driver in **both** apps.
`docs/src/dev/design-system.md` is **not** CI-enforced. It also established that `/manual/` is not a
valid precedent (static mdBook output, not a frontend) and that pdf.js ships inside the one binary
either way, since `fm-serve/build.rs:25,70` bakes `ui/dist` in.

**What the owner's decision therefore obliges, when Phase C is reached:**
1. A **dated reversal** of `MASTERPLAN.md:341` with a `> SUPERSEDED` banner — not the "scoping entry"
   revision 1 proposed. The `fm-agent-run` ruling does not cover this: that one is a **Cargo feature
   off by default** on a **separate binary that drags in no fm-core**, gated by `deny.toml`. None of
   those three properties holds here.
2. **Widening `ci/checks.sh`'s sink and renderer guards** to cover the second app, in the same commit
   that creates it — not afterwards.
3. **An npm licence gate** (A7/B6), because `deny.toml` will never see pdf.js, and the existing
   omission of marked/katex/mermaid/dompurify/excalidraw/react from `THIRD-PARTY.md` is already a
   live gap this would deepen.

---

## Part 6 — Verification

**For this plan (Phase A):**
- **A1** — a test that would have failed: `render.test.ts:110` asserts the iframe **in jsdom, where
  no CSP applies**, which is why the bug shipped. Whatever the fix, cover it against a real policy.
  And the string corrections land whether or not the render fix does.
- **A2** — `set_property(id, "assets", "sha256:…")` → reload → still there. Same for `code`.
- **A3** — a value read as `Int` and written back is still `Int` (the `Ord` variant-order trap).
- **A4** — a `save_view` round trip that carries a filter, and does not clobber a hand-written one.
- **A5 + A6** — the new `perf.rs` budget, seeded with realistic body sizes, **fails before A5 and
  passes after**. Without that ordering the pushdown is an unverifiable claim.
- **Throughout:** `pixi run ci`. Per `outstanding.md §1.1`, layout and CSP claims are not provable
  from CI — A1 needs eyes on a browser, and on the phone.

**Retained for later phases:** a test pinning `isEvalSupported: false`; parse/serialize round-trips
for the multi-rect anchor; a `refs.rs` test that an anchored ref still scans as a reference to the
bare blob; **a bundle-size assertion pinning the eager payload** (~70 KB gz today) — revision 1
pinned the clause that costs nothing and skipped the one that costs 10×; annotation notes appearing
in no planning view; and a second Zotero import creating zero duplicates.

---

## Part 7 — Out of scope, stated

Two-way Zotero sync · translator parity · CSL styles / Word integration · **EPUB and snapshot
annotations, which have no landing site at all — so "an entire library" honestly means "the PDF part
of a library", and that belongs in Phase E's description, not buried here** · writing annotations
into the PDF · a second `FileStore`.
