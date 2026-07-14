# formicarium — v1 brief (decisions resolved)

**2026-07-14 · resolves the open questions in README / PLAN / ADR-007 · this is what we build**

This supersedes the README's "Blocked on two questions" and PLAN §11. `PLAN.md` and the ADRs remain the
evidence trail; where this doc disagrees with them, **this doc wins**. Kept deliberately short — it is a
decision record, not a design essay. The reasoning already lives in the ADRs.

---

## Purpose (unchanged)

A local tool where you capture ideas fast, retrieve them years later, see the closest deadline, and keep every
figure, deck, paper and recording attached to the thought that produced it.

**It can only fail two ways: capture is slow, or retrieval doesn't work.** Everything else is decoration.
**Success:** you use it daily for three months without wanting to leave.

---

## Decisions resolved this session

| Question | Answer | Consequence |
|---|---|---|
| Build vs. adopt | **Build**, with a real editor | (Do a 2-evening SilverBullet/Obsidian trial anyway — cheap insurance.) |
| Editor | **Live-preview Markdown** (CodeMirror 6 + inline KaTeX) | Markdown stays the source of truth. **No WYSIWYG/block editor** — it would corrupt notes you hand-edit in Vim. |
| External writers | **Yes** — Vim, `git pull`, Syncthing all touch the notes dir | The index is *advisory*. Reindex-on-startup + periodic **mtime polling**. **No inotify watcher.** |
| Deadlines | Hard **and** soft; **priority is derived from `due`** | Agenda-first. **No `priority` field.** Month-grid calendar is optional, not the daily driver. |
| Access | **Desktop-only for v1** | Deletes the entire auth / HTTP-server / Tailscale / range-serving subsystem. |

The two decisions that shrink scope most: **desktop-only** removes auth and networking wholesale, and
**Markdown-as-truth** removes the WYSIWYG editor and its round-tripping problems.

---

## Data model (ADR-001/002, plus the deadline refinement)

One object = one note = **one Markdown file**. Structure in YAML frontmatter. Files canonical, SQLite disposable.
**Atom = the file, never the block.** ULID `id`; links point at ids, never filenames.

```yaml
---
id: 01J8ZQK4XN9P
type: note          # note | task | meeting | asset
status: doing       # enum, EXCLUSIVE — a property, never a tag
due: 2026-07-20     # working target date, nullable — SOFT by default
hard: true          # present only for fixed external commitments (submission, talk, grant)
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T11:03:00Z
tags: [meta-rl]     # multi-valued, unordered — NOT status
assets: [sha256:9f2a…]
code:   [meta-rl@a3f9c2e]
---
```

**Deadlines, done right:**
- **No `priority` field, ever.** Urgency is *computed*: `overdue > due-soon > due-later > no-due`, with `hard`
  breaking ties and flagged so a fixed deadline never hides behind slipping soft ones.
- **The soft `due` field IS the priority dial.** Nudging it forward = reprioritizing. So editing `due` must be a
  one-keystroke affordance — you'll do it daily.
- A **meeting** is just a note with `type: meeting` and a date; it appears in the agenda alongside tasks.

---

## Views — generic renderers, not features (ADR-006, unchanged)

| View | Query | Note |
|---|---|---|
| **Stream** | all, `created` desc | home |
| **Board** | group by any enum property | drag writes the value back |
| **Gallery** | `type = asset`, thumbnails | find the figure |
| **Agenda** | `status != done AND due != null`, sort `due asc` | **the "closest deadline" view — zero renderer code** |
| Calendar | place by any date property | available, *not* the daily driver — build if you miss it |

**The falsifiable test stays:** the board renderer must not know what `status` is. CI greps renderer source for
`todo`/`doing`/`done` and **fails the build if found**.

---

## Scope — v1

**Build:** always-focused capture box (no save button) · live-preview Markdown editor · files + git auto-commit ·
`Store` seam + `MemoryStore` first · query engine · 4 renderers (stream/board/gallery/agenda) · sha256
content-addressed blobs on their own volume · ingest via paste/drag **and** `fm add` (`cp --reflink=auto`) ·
`pdftotext` → FTS5 · libvips thumbnails · `verify` · restic backup.

**Cut:** auth / phone / Tailscale / range-serving *(desktop-only)* · inotify watcher · whiteboard · graph view ·
**plugin API (never)** · OCR · wikilinks/backlinks *(v2)* · month-grid calendar *(optional)* ·
saved-view editor UI *(ship 4 hand-written `.view` files)*.

---

## Architecture — three seams (ADR-001, the crown jewels — untouched)

```
views  →  query engine (NEVER touches the filesystem)  →  ┌ Store seam:  MemoryStore | FileStore
                                                          ├ Blobs seam: content-addressed, own volume
                                                          └ OS seam:    pdftotext · libvips · xdg-open · restic · git
```

The `Store` interface (`get/put/delete/query/reindex`) and a `MemoryStore` that passes the whole query-engine
test suite **with zero filesystem access** is the insurance policy. Build the fake store *first*.

---

## Recommended stack (dependable + cutting-edge, mid-2026)

A **native shell** (not a browser tab) so we get a real window, a global capture shortcut, and a small footprint
with no bundled browser.

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri v2** *(recommended)* | Mature system-webview shell; tiny binaries; no bundled Chromium; strong ecosystem; Rust ≈ 10-yr longevity. Alt: **Wails (Go)** — see fork below. |
| Frontend | TypeScript + Vite + **Svelte 5** | Compiles away to a tiny runtime; dependable-cutting-edge; swappable. |
| Editor | **CodeMirror 6** | Best-in-class Markdown-source editor; Obsidian-style inline preview via its decoration / `WidgetType` API. |
| Math | **KaTeX** | Fast, synchronous LaTeX render inline. (MathLive only if you later want a visual equation editor.) |
| Drag & drop | **Pragmatic drag-and-drop** (framework-agnostic) | For the board; small and well-supported. |
| Index / search | **SQLite FTS5** | Over note bodies + frontmatter + `pdftotext` output — one search box for everything you've read. |
| Hash / assets | **sha256** content-addressed blobs (ADR-007); **libvips** thumbnails via subprocess | Verifiable with `sha256sum` forever; shell out for media. |
| History | **git** auto-commit | Buys *undo*, not a readable history. Don't try to fix that. |
| Backup | **restic** | Dedup / encrypt / integrity / remote — config, not code. |
| Text extraction | **pdftotext** now; `pandoc` / `tesseract` later | The single highest-value feature in the system. |

Exact versions get pinned at first commit (verify each is current before adding it).

---

## The one open fork — backend language

Both satisfy every requirement above. This is the last thing to decide before S0.

- **Tauri v2 / Rust** *(recommended)* — best native-shell ecosystem, excellent asset & SQLite libraries
  (`rusqlite` bundled = SQLite + FTS5; `sha2`; `image`/vips; `notify` in poll mode; `ulid`), smallest binaries.
  Cost: Rust is harder to write than Go — but this app is CRUD + files + SQLite + subprocess calls, not
  algorithmically hard, and most of the "looks nice" work lives in the TS frontend regardless of backend.
- **Wails / Go** *(keeps the ADR investment)* — matches ADR-004/007's Go + `modernc.org/sqlite` (pure Go, FTS5
  included) decision; easier language; near-static binary. Cost: smaller ecosystem than Tauri; v3 not yet stable
  (use v2).

**Recommendation: Tauri v2.** Choose Wails/Go instead if you'd rather write the backend yourself in a language
you're already comfortable in.

---

## Build order — S4 is the checkpoint that matters

| | Slice | Proves |
|---|---|---|
| S0 | type → disk → reload → still there | the pipeline exists |
| S1 | search box → FTS5 → results w/ timestamps | retrieval works |
| S2 | properties editable; `status` settable | the typed model holds |
| S3 | **Board:** group by `status`, drag writes back | the thesis is alive |
| **S4** | **Gallery:** a *second* renderer over the same query layer | **the thesis is proven — or dead** |
| S5 | assets (hash / store / `pdftotext` / thumbnails) + agenda `.view` | the media library + deadlines |
| S6 | `verify` + restic + **a tested restore** | durability |

**If S4 hurts, stop** — the query layer isn't real and every later view only gets more expensive.

---

## First three commits (ADR-005, unchanged)

1. `Store` interface + `MemoryStore` + the query-engine test suite that passes **with zero filesystem access**.
2. `.githooks/pre-commit` — the 5 MB text-only guard; `git config core.hooksPath .githooks`.
3. Perf budgets as CI assertions, *now*: capture < 200 ms · search < 100 ms @ 10k · keystroke → paint < 16 ms ·
   reindex < 10 s @ 10k.
