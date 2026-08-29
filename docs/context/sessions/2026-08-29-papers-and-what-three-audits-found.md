# 2026-08-29 — papers, and what three audits found underneath them

**The ask.** *"Researchers taking notes need to track papers. I always used Zotero — maybe we can do
something formicaria-specific… if I download a paper I might want to highlight things in it (words,
sentences, figures, formulas) and add notes connected to particular paper locations."* Then, after
the research: *"a separate tool from formicaria that can be attached to the core… our own Zotero-like
app, following our fm philosophy and API"*, and a hard requirement — *"it should eventually allow to
port entire libs from Zotero."*

Full direction, with the reviews behind it: [`papers-plan.md`](../papers-plan.md).

## The research said the pain is the seam, not the features

Zotero is not standing still — 9 in April, **10 on 2026-08-17**, which added a *write*-capable local
API for the first time. Its reader does highlight, underline, note, **area** (a box over a figure or
a formula) and ink; annotations are items with `rects`, colours and sort indices, and they live in
Zotero's database, invisible in any other reader.

Every alternative — PDF++, Annotator, Logseq, ZotLit, zendron, ten more Obsidian plugins — is an
**export pipeline**, and the forums say the same three things about all of them: re-importing
overwrites the prose you wrote, corrections never flow back, and volume is unmanageable. Qiqqa,
Docear and Polar, the all-in-ones that owned metadata *and* reading *and* notes, are all dead.

That is the whole argument for doing this here: if the highlight **is** a note, there is no seam to
go wrong. And formicaria has an advantage none of them has — a PDF is a content-addressed immutable
blob, so an annotation's geometry cannot drift under it.

## Four adversarial reviews of the plan, before any code

They found **more shipped bugs than plan flaws**, and killed several of my own ideas:
`-bbox-layout` (measured at 19–22× the plain text, and redundant since pdf.js supplies the text
layer), the browser extension (an extension on 127.0.0.1 takes the *loopback* branch, and pairing
tokens are the non-loopback path — replaced by a paste box), and a single-rect anchor format (a
highlight is `rects`, **plural**; one crossing a line break has several).

The owner ruled: **a separate app**, and **Phase A only** — fix what is broken first.

## What shipped

**Phase A — six live defects, three of them telling the user something false.** No PDF had ever
rendered (`frame-src 'none'` versus the read view's own iframe, silent because a blocked frame is a
placeholder and the only test ran in jsdom, where no CSP applies) while Settings said *"PDFs are
stored, opened and shown"*. `assets`/`code` had no write path and were silently discarded. Every
planning view hydrated the whole corpus. Plus the three §2.6b repairs: a details panel for arbitrary
frontmatter, a filter a saved view can actually carry, and a *Save view* button — `newView` was a
labelled command with an empty default key in an app whose palette had been removed.

**Phase B — the library, entirely offline.** A paper is a `Kind::Note` tagged `paper`. It comes from
a pasted BibTeX entry, a DOI, an arXiv link or a title; the identifier a PDF prints on itself is read
at ingest, free, because `pdftotext` has already run. BibTeX back out. No network, because
`fm-agent-run`'s ruling says the core links no HTTP client — so a DOI is *recognised*, never
resolved.

**Phase C groundwork — an anchored reference**, `[p. 4](asset:sha256-…#page=4)`. `#page=N` is the
standard PDF open parameter, verified in a real browser against our own blob route.

## Then three audits, and they were right

**A schema bump is not a migration.** `INDEX_SCHEMA` went 1→2 and the commit claimed an old index
would rebuild. `Reindex::Full` does `DELETE FROM objects`, never `DROP TABLE`, and
`CREATE TABLE IF NOT EXISTS` is a no-op — so the v2 insert met a v1 table. `MultiStore::open_with`
files a failed vault under `unopened`, so **every** vault failed, `list_vaults` answered `[]`, and
`[]` is the first-run signal: every v0.2.x upgrade would have offered *"create your first vault"*
with the whole library intact and unreachable. The migration pattern already existed three lines
above, for `fts_rowid`. **The tell was the missing version-history line**, which is why that doc
block now says every bump owes a line *and* a column migration.

**Scan a `&str` by characters, not bytes.** `identifier_in_text` sliced `&text[..4000]`;
`pdftotext` emits ’ “ — ﬁ routinely, and the panic fired inside `asset_note` with the global
`Mutex` held and nothing catching unwinds — one dropped paper, a poisoned lock, a dead app.

**Scope a capability claim to the device you tested.** *"PDFs render again on every platform"* was
written off a headless desktop browser. Android cannot render a PDF inline at all — `mobile-design.md`
had said so all along — the phone serves every blob as `octet-stream`, and `frame-src 'self'` cannot
match `fmblob:` anyway, with the counter-evidence two clauses away in the string being edited. The
always-read layer was made to contradict an on-demand doc, which is the exact failure the two-layer
split exists to prevent.

**A changed default is a decision even when it arrives inside a bug fix.** The `tags` separator
changed with a long comment, a long commit message and **no entry** — while the entry written ninety
minutes earlier still asserted the old behaviour, in an append-only log. Worse, the read side had not
moved with the write side, so a hand-written `tags: alpha, beta` read as one tag and split into two
on the first save: *opening a note and saving it changed what it was tagged with.*

**A guard that does not mention what it guards proves nothing.** `filestore_matches_memorystore` is
the test that holds seam 2, and it ran three queries — none containing a `Kind` predicate, the one
new thing pushed into SQL. The pushdown was correct; nothing would have noticed if it were not. Now
eleven cases with an asset in the corpus, verified by breaking the pushdown and watching it fail.

**And the record has to live in the repo.** The plan sat in a scratchpad, `features.md` pointed at a
file that did not exist, `outstanding.md` knew nothing about the reader, and the owner's two rulings
were written in no repo file at all. A cold session could have reconstructed the bug fixes and none
of the direction. That is fixed; it should not have needed an auditor to say so.

## Where this stops, and what is next

**Nothing of the actual ask exists yet** — no reader, no highlight of a word or a figure, no note
anchored to a place. `#page=N` is the human half of the anchor and deliberately not the machine half.
Queued in [`outstanding.md`](../outstanding.md) §2.0, in order:

1. The **anchor format's machine half** — multi-rect, versioned, explicit about its coordinate
   space, in a structured block in an annotation note. **Designed before a line of reader code.**
2. The **reader**: pdf.js with `isEvalSupported: false` (the CSP has no `'unsafe-eval'`, by ruling)
   plus its `cmaps`/`standard_fonts`. Desktop only, and say so.
3. **Annotations as notes from the start**, excluded from planning views — a prose block can never
   hold multi-rect, ink, colour or per-annotation tags.
4. The **Zotero port**. Local API only: every export route drops annotations, and area-annotation
   images often do not exist until the PDF has been opened in Zotero's own reader, so the importer
   must render crops itself and therefore depends on the reader.

**Before any of that**, the three obligations the *separate app* ruling incurs — the dated
`MASTERPLAN.md:341` reversal, widened `ci/checks.sh` sink and renderer greps, and an npm licence gate
(`deny.toml` is keyed on `Cargo.lock` and will never see pdf.js). They are cheap now and become an
argument later.
