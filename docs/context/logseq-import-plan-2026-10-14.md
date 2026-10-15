# Logseq import: a test framework first, then the fixes (plan, 2026-10-14)

_Status: **Part 1 (the test framework) and Part 2 (the fixes) are both built (2026-10-15)**; every
judged check passes on the owner's graph (scorecard below).
Read before touching `fm-core/src/import.rs`, `import/logseq.rs`, or the import surface. The ruling
this works inside is `decisions.md#data`, *An import converts; adoption renders* (2026-09-02);
nothing below reverses it unless it says so._

## Where the framework is, and how to run it

| Piece | File | Runs in |
|---|---|---|
| Oracle (independent inventory of a graph) | `crates/fm-app/tests/import_conformance/oracle.rs` | — |
| Checks C1–C13, scorecard | `crates/fm-app/tests/import_conformance/mod.rs` | — |
| Mini-graph (public fixture, one page per construct) | `crates/fm-app/tests/fixtures/logseq-mini/` | — |
| Oracle self-test + mini-graph conformance (**ratcheted** by `KNOWN_FAILING`) | `crates/fm-app/tests/import_corpus.rs` | `pixi run test` → `ci` |
| Private corpus conformance | same file, `#[ignore]`d | `FM_IMPORT_CORPUS=~/logseq-phd pixi run import-corpus` |
| Render contract (what the importer emits must render) | `ui/src/lib/render.import.test.ts` | `pixi run test-ui` → `ci` |
| Real-browser pass + contact sheet over the imported vault | `ci/import-render.{sh,py}` | `pixi run import-render` (after `import-corpus`) |

Outputs (private, gitignored): `target/import-corpus/{scorecard.json,report.md}`,
`target/import-corpus/shots/{index.html,render-report.json}`.

**The ratchet.** `KNOWN_FAILING` lists the checks the importer fails on the mini-graph today. The
test fails if any *other* check fails, and also if a listed one starts passing — so a fix must
take its id out of the list, and the list can only shrink.

**One deviation from the plan below:** the oracle does not use `regex`. A separately written
hand scanner with *different algorithms* (it balances `[]`/`()`; the importer does not) gives the
same independence without a new dev-dependency, so `Cargo.toml` is unchanged.

## After Part 2 (2026-10-15)

The owner's answers: implicit pages become **tags** (Q1); nothing is skipped by default, and the
person sets the rules — size and kinds — and what comes over is **copied** (Q2: hard links were
built and reversed the same day — "we cannot have part of a vault outside the vault",
`decisions.md#vault`; a move was rejected for the read-only promise); an import can **replace** an
earlier one, never over an edit, and a second import of the same graph is caught (Q3); search reads
**tags and property values** (Q4, `decisions.md#seams`). Measured on the PhD graph, same harness:

| Check | Before | After |
|---|---|---|
| C2 titles | 99.2% | **100%** |
| C3 links to existing pages | 41.3% | **100%** |
| C5 attachments referenced | 86.1% | **100%** |
| C5b highlights → PDF page | 0% | **100%** |
| C6 missing files still named | 99.8% | **100%** |
| C7 no Logseq syntax visible | 57.1% | **100%** |
| C9 search finds each page | 88.4% | **100%** |
| C11b one asset note per file (new) | — | **100%** (re-imports had duplicated them) |
| C13 Excalidraw → whiteboard | 0% | **100%** |
| C14 replace rewrites an unedited import (new) | — | **100%** |
| C15 an earlier import is recognised (new) | — | **100%** |

C1, C8, C10, C11 stayed green throughout. Bugs the harness found *in the course of Part 2*, each now
a unit test: LaTeX's `{{` read as a macro (and a macro taking the next `}}` lines away); inline code
or `$` matched across bullets; a referenced block's words pasted unconverted; nested `[[…]]`, `|`
and lone `[` in Logseq page names; two identical files in one graph getting two asset notes;
`ingest` reading a whole file into memory to sniff it, and an unrecognised binary read whole to
test it as text (a 500 MB ROS bag). The browser pass then found `#[[two words]]` tags left in their brackets in
705 places (the Rust debris check had no `#[[` pattern; it has now) — they read `#two words`, as in
Logseq.

**For the owner's existing vault** (imported with the old importer, so no `import_hash`): import the
folder again into that vault with **Replace** ticked — the preview offers it. Notes are rewritten
only where `updated` still equals the source file's time, i.e. never saved in the app; anything
edited is kept and named.

**Not done from the list:** F9 (warning which attachments exceed `git_assets_max`) — blobs are
outside git regardless, and the preview now states sizes by kind, which answers the same question.

## Baseline on the PhD graph (2026-10-15, before Part 2)

| Check | Result | Bar |
|---|---|---|
| C1 every page is one note, journals dated | 1181/1181 ✅ | 100% |
| C2 titles as Logseq shows them | 1171/1181 ❌ — `%23`, `%22`, `%5C`, `%3A` left in titles | 100% |
| C3 links to an existing page reach it | **478/1158 (41%)** ❌ | 100% |
| C4 links to implicit pages (reported) | 4,261 of 5,419 links point at a page with no file | — |
| C5 attachments in the graph become referenced blobs | 583/677 (86%) ❌ | 100% |
| C5b PDF highlights link to their page | 0/20 ❌ | 100% |
| C6 attachments not in the graph are still named | 639/639 ✅ (after an oracle fix: `../../../assets/x`) | 100% |
| C7 no Logseq syntax visible | 674/1181 (57%) ❌ | 100% |
| C8 words survive | 99.93% ✅ — but the misses are real (F11) | 99.5% |
| C9 **search finds each page by its rarest words** | **3002/3397 (88%)** ❌ | 98% |
| C10 source untouched · C11 re-import adds nothing | ✅ · ✅ | exact |
| C13 Excalidraw drawing → whiteboard note | 0/1 ❌ | 100% |

Import: 57 s, 350 MB peak. Browser pass (185 notes: all 125 with attachments, plus a sample;
identical on two runs): of 569 images, 13 do not load (file names with parentheses, and a few more
to read off the contact sheet); 132 notes fail a check, 125 of them by showing Logseq syntax (`[[` 118 — the dangling implicit links; `{:height` 27;
`{{` 26); no asset *links* at all, because no non-image file was ever imported (F2).

**Findings the plan did not have** (each now a Part 2 item):

- **F10 — search never sees properties or tags.** `Object::searchable_text` is title + body only,
  so every page property — the authors, venues and item types on 400+ Zotero pages — is
  unsearchable. This is most of C9's failures, and the likeliest cause of "could not find many
  things". It is an *app* change, not an importer one: it touches the FTS index (bump
  `INDEX_SCHEMA`) and wants a `decisions.md#seams` entry.
- **F11 — a `date::` that does not parse is silently dropped.** `apply_page_property` routes
  `date`/`created` into `created`; `[[Nov 30th, 2012]]` (a paper's *publication* date) does not
  parse, and the value vanishes. Publication date is not creation date either — keep it as a
  property.
- **F12 — a label holding brackets breaks the wikilink scanner.** `[[Draft]_notes.pdf](../assets/…)`
  is read as a `[[` link running across the following lines (see the mini-graph's dangling names).
- **F13 — aliases vanish from the note.** `alias::` is used to resolve links but is not kept, so a
  reader cannot see or search a page's other names.

## Why

The owner imported their PhD graph and "could not find many things", and wants every linked image
and file to be visible after the import. The importer's own tests are unit tests over hand-written
snippets. Nothing has ever measured it against a real graph, so there was no way to say whether it
works. **So the first deliverable is a measuring instrument; the fixes come second and are judged
by it.**

## What the corpus actually contains (census, 2026-10-14)

The graph is the owner's private one at `~/logseq-phd`: 2.5 GB, 688 pages, 493 journals,
765 files under `assets/` (2.4 GB, 1.9 GB of it ROS `.bag` files, 4 files over 100 MB), 4 tldraw
whiteboards (`.edn`), 1 `draws/*.excalidraw`. `:file/name-format :triple-lowbar`; the journal
title format is the default (`MMM do, yyyy`).

| Construct | Files | Hits | What the importer does today |
|---|---|---|---|
| `[[wikilink]]` | 830 | 6,092 | **only ~481 (8%) resolve**, by an offline simulation of `page_name` |
| `[[Oct 14th, 2026]]` (journal by display date) | — | 832 | dangle: journals are registered as `2026_10_14` only |
| page filename with `%3A` etc. (186 pages) | — | — | `page_name` decodes `%2F` only, so `[[A: B]]` misses `A%3A B.md` |
| `title::` (page-level and block-level) | 423 | 602 | overrides the title but is **not registered as a name**, so links to it dangle |
| `![img](../assets/…)` | 126 | 591 | ingested and rewritten ✅ |
| `…){:height 530, :width 748}` after an image | 29 | 57 | the suffix is left as **visible debris** |
| `[file](../assets/x.pdf/.pptx/.zip/.bag/.ipynb/…)` (not an image) | 58 | 86 | **never ingested** (`attachments` is filled from images only), so it stays a dead `../assets/` link |
| asset path containing `(` `)` | 8 | 10 | `md_span` stops at the first `)`, so the link breaks |
| page property holding a link (`file::` on `hls__` pages, `links::`) | — | — | stored as a scalar string: not rewritten, not ingested |
| `{{zotero-imported-file KEY, "x.pdf"}}` | 406 | 635 | raw macro text left in the note |
| `{{video https://youtube…}}` | 16 | 20 | raw macro text |
| `{{renderer …}}`, `{{query …}}` | 4 | 4 | raw macro text |
| `((uuid))` block reference | 65 | 148 | inlined ✅ (includes a `[label](((uuid)))` form, which is not handled) |
| `hl-page::` PDF highlights (`hls__` pages + `.edn` sidecars in `assets/`) | 3 | 21 | text only; not linked to the PDF page |
| `#tag` / `#[[multi word]]` | 142 / 309 | 191 / 676 | hoisted ✅ |
| `$$…$$` math | 27 | 492 | passed through; `render.ts` renders it with KaTeX ✅ |
| `[x](https://…)` / bare URL | 532 / 579 | 1,516 / 1,170 | passed through ✅ (rendering of bare URLs to be checked) |
| implicit pages (property values and authors: `[[journalArticle]]`, `[[Abstract]]`, people) | — | ~2,000+ | dangle **by design** (*Dangling links stay as text*), but this is likely a large part of "could not find" |

The census scripts are throwaway Python; Layer 1 below replaces them with something the gate runs.

## Part 1: the test framework

**What it must answer:** *for each thing a reader could see or follow in Logseq, can they still see
or follow it in formicaria, and can they find it with search?* Every answer is a number with a
threshold, so "does the port work" has a yes/no answer.

### Constraint: the corpus is private and the repo is public

The PhD graph **never enters the repo**: no file content, page titles or report in git. So there are
two corpora:

- **The private corpus**: the real graph, passed as `FM_IMPORT_CORPUS=/home/baljinder/logseq-phd`.
  The test is skipped when the variable is unset. Reports go to `target/import-corpus/`, which is
  already gitignored. This run is the **acceptance test**.
- **A synthetic mini-graph** in `crates/fm-core/tests/fixtures/logseq-mini/`: one small page per
  construct in the census table, written by hand rather than copied, with a golden expected output.
  This runs in `pixi run ci` and is the **regression guard**. Rule: every defect the private run
  finds gets a mini-graph case *before* it is fixed.

### Layer 1: the oracle (an independent inventory)

The inventory must be built **without the importer's own scanners**. If the oracle shared
`scan_wikilinks`, it would share that function's bugs and report 100%. So it gets its own small,
separately written parser (*as built*: hand-written with different algorithms, no `regex`; see the
deviation note above). From the source graph it lists:

- **pages**: every `.md` under `pages/` and `journals/`, with every name a link could use to
  reach it: decoded filename, `title::`, `alias::`, and for journals the display title in the
  configured format;
- **links**: every `[[X]]`, `[l](X.md)` and `[l](((uuid)))`, each classified as *page exists*,
  *journal day*, *implicit page* (property value or tag with no file), or *missing*;
- **attachments**: every local target in an image, a link, a page property, a `{{zotero-imported-file}}`
  or an `hl-page` sidecar. Each is tagged *present in the graph* or *not in the graph* (Zotero
  storage is not on this machine: `:zotero-data-directory ""`);
- **block refs**, **macros by name**, **tags**, and **searchable text**: per page, the words left
  once markup and bookkeeping properties are stripped.

### Layer 2: conversion conformance (Rust: `crates/fm-core/tests/import_corpus.rs`)

The test runs the **real path**, `fm_app::dispatch("run_import", …)`, into a temporary vault. It
does not call `convert` alone, because the bugs may sit in the write, the index or the commit. It
then checks the vault against the oracle:

| # | Check | Pass bar |
|---|---|---|
| C1 | every source `.md` is exactly one note (`source_key` sets are equal); journals dated by their filename | 100% |
| C2 | each note's **title** is the name Logseq shows (decoded, `title::` honoured) | 100% |
| C3 | every link whose target *exists* (page or journal) becomes a `note:` reference to **that** note | 100% |
| C4 | links to *implicit* pages: counted, and handled the way the owner decides (see Q1) | reported |
| C5 | every attachment *present in the graph* becomes an `asset:sha256-…` reference, and the blob store holds those exact bytes (re-hashed from the source file) | 100% |
| C6 | attachments *not* in the graph leave a readable, non-broken mention (filename, no raw macro) | 100% |
| C7 | **no debris** in any body: `../assets/`, `{:height`, `{{` macros, raw `((uuid))`, `%3A` in titles, an unbalanced `](` | 0 hits |
| C8 | **text preservation**: every searchable word of the source page appears in its note | ≥ 99.5% of words, with misses listed |
| C9 | **findability**: for each page, 3 distinctive words (the rarest across the corpus) and its title, run through the `search` command, find that note in the top 10 | ≥ 98% |
| C10 | the source folder is unchanged (fingerprint before and after; an existing invariant, re-asserted at scale) | exact |
| C11 | a second import adds 0 notes and 0 blobs | exact |
| C12 | budget: wall time and peak memory, logged. Run inside a `MemoryMax` scope per the one-heavy-job rule | logged; ceiling to be set from the baseline |

The output is `target/import-corpus/scorecard.json` plus `report.md`: each check's number, pass or
fail, and up to 20 concrete failures per check. Those are private and stay local. It runs as
**`pixi run import-corpus`**, which is **not** part of `ci`.

### Layer 3: render conformance ("can I *see* it")

Conversion can be perfect and the note still render broken, so the rendered DOM is checked as well:

- **Vitest (in `ci`)**: the mini-graph's golden bodies go through `render.ts`, asserting that every
  `asset:` becomes `<img>`/`<video>`/`<iframe>`/`<a>`, math produces KaTeX, a `{{video}}`
  replacement renders as intended, and no `[[`, `{{` or `](../` survives as visible text.
- **A real browser (private run)**: a sibling of `ci/shots.py` drives headless Chromium over CDP.
  It serves the imported temporary vault on a spare port (never 8765) and opens **every note that
  has an attachment** plus a stratified sample of the rest. It asserts:
  - every `<img>` has `naturalWidth > 0`;
  - every asset `<a>`/`<iframe>` answers 200 from `/api/blob`;
  - no KaTeX error spans appear;
  - the visible text has no debris patterns;
  - nothing overflows at 320 px (the popups rule).

  It also writes a **contact sheet** of screenshots so the owner can look at the result directly.
  Runs as `pixi run import-corpus-ui`.

### Order of work, Part 1

1. Mini-graph fixture and golden outputs (public, small).
2. Oracle and Layer 2 test; run on the private corpus to get the **baseline scorecard** of today's
   importer, which is expected to fail C2, C3, C5, C7 and probably C9.
3. Layer 3 (Vitest first, then the browser pass).
4. Commit the framework while the scorecard is still red, so the fixes show up as numbers moving.

## Part 2: the fixes, ranked by what the census says they unlock

Each fix is one mini-graph case and one scorecard delta.

| # | Fix | Unlocks |
|---|---|---|
| F1 | **Names**: percent-decode the whole filename stem (not only `%2F`); register `title::` and the journal display title (format read from `config.edn`, default `MMM do, yyyy`) as names | ~1,000+ links (C2, C3) |
| F2 | **Non-image attachments**: collect every local Markdown link target, not only `![…]`; resolve, ingest, rewrite | 86 links to pdf/pptx/zip/ipynb/bag (C5) |
| F3 | **Page properties that hold links** (`file::`, `links::`): rewrite their values through `rewrite` and ingest what they point at; `hls__` pages get their PDF | C5, C7 |
| F4 | **Span parsing**: balanced parentheses inside a link target; drop or convert the `{:height …, :width …}` suffix | 10 broken + 57 debris (C5, C7) |
| F5 | **Macros**: `{{video url}}` → a plain link to that URL (the CSP forbids framing YouTube); `{{zotero-imported-file K, "f.pdf"}}` → link to the asset if `f.pdf` is in the graph, else readable text naming the file and that it lives in Zotero; any other macro → its readable text, never raw braces | 659 (C6, C7) |
| F6 | `[label](((uuid)))` → the label, linked to the note holding that block | C3 |
| F7 | `hl-page::` highlights → `[p. N](asset:sha256-…#page=N)`, the form `render.ts` already opens at a page | 21 (C5) |
| F8 | `draws/*.excalidraw` → a board note (`view: board`, the scene as body). Whiteboards stay counted as left behind | 1 drawing |
| F9 | Large files: say which attachments exceed the vault's `git_assets_max`, so the user knows they will not travel with a backup (blobs are already gitignored; nothing new ships) | honesty |

**Re-import caveat.** Imports are **add-only** (`source_key`), so fixing the importer does nothing
to the vault the owner already imported. Testing the fixed version means importing into a **fresh**
vault, then forgetting the old one from the UI. Whether to offer a UI path that replaces an earlier
import is Q3.

## Open questions for the owner (before Part 2, not before Part 1)

- **Q1: implicit pages.** About 2,000 links point at pages Logseq creates implicitly: property
  values like `[[journalArticle]]`, section names like `[[Abstract]]`, and author names. The
  standing ruling keeps them as plain text, and offers stubs only as an opt-in, because empty notes
  flood every view. Options: (a) keep that ruling; (b) turn them into **tags** on the linking note,
  so you can find every paper by an author without creating new notes; (c) default stubs on, for
  Logseq only. (b) is the likeliest fit with the ruling, but it extends it, so it needs a dated
  `decisions.md` entry.
- **Q2: `.bag` and zip files (2.1 GB).** Copy them into the blob store, or link only the ones a note
  references (F2 already does only that) and skip anything over a size the user picks?
- **Q3: replacing an earlier import.** Add a "re-import, replacing notes I have not edited" path, or
  keep the import add-only and rely on a fresh vault?

## Rulings this touches (the four questions)

- F1 to F7 are permitted: they make the existing *conversion* ruling true for constructs it did not
  know about, and they write nothing new into what ships.
- The framework itself adds no dependency and nothing to what ships (tests, two `ci/` scripts, two
  pixi tasks outside `ci`).
- Q1(b) or (c), and Q3, would **extend or reverse** *Dangling links stay as text* or *Add-only*.
  Each one gets its dated `decisions.md` entry first, with a SUPERSEDED banner if it reverses.
- Agents and cost: none needed. The work is sequential and local.
