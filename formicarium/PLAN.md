# PLAN — Personal research workspace

**v1.0 · 2026-07-14 · supersedes REQUIREMENTS.md and ADR-001…006 as the working document**
Those remain as the evidence trail. This is the thing you build from.

---

## 1. What this is

A local tool where you capture ideas fast, retrieve them years later, see what you're working on, and keep every figure, deck, paper and recording attached to the thought that produced it.

**Success:** you use it daily for three months without wanting to leave.
**It will fail from one of two things only:** capture is slow, or retrieval doesn't work. Optimize against those two above everything else. Every other feature is decoration.

### Kill criteria — decide these now, honour them later

| If… | Then |
|---|---|
| The board (S3) isn't working after **4 weekends** | Stop. Install SilverBullet. You've learned what you needed. |
| Adding the *second* view (S4) is painful | The architecture is wrong. Stop and fix it — do not continue building on it. |
| You haven't opened it for **2 weeks** during the build | It isn't solving your problem. Find out why before writing more code. |

A plan without an exit is a trap. This is the exit.

---

## 2. Constraints and load — honest numbers

| | Reality |
|---|---|
| Users | **1** |
| Notes, 10-year horizon | ~10k files, ~20 MB text |
| Assets, 10-year horizon | **100 GB – 1 TB** (videos and decks dominate; text is a rounding error) |
| Writes | ~50/day |
| Reads | ~100/day |
| Concurrency | One writer. Maybe two browser tabs. |

**This is not a distributed systems problem.** One process, one SQLite file (WAL mode), one disk. No queue broker, no object store cluster, no cache layer, no S3, no CDN. Anything that looks like infrastructure is a mistake.

**Search is not a scale problem.** `ripgrep` scans 20 MB in ~30 ms. SQLite exists here for **queryability** (kanban and calendar are relational queries with filters and joins), not for volume. Never let "what if it gets big" justify complexity.

---

## 3. Architecture — three seams, and nothing else matters

```
   views          stream · board · gallery              ← cheap, swappable
     │
   query engine   filter · sort · group                 ← THE STABLE CORE
     │                                                     touches no filesystem, ever
   ── Store ──────────────────────────────────────────  ← SEAM 1: guard this
     │
     ├─ MemoryStore   (tests, zero I/O)
     └─ FileStore     markdown files (truth) + SQLite index (disposable)

   ── Blobs ──────────────────────────────────────────  ← SEAM 2
     └─ content-addressed, own volume, outside the repo

   ── OS ─────────────────────────────────────────────  ← SEAM 3
     └─ pdftotext · libvips · xdg-open · restic · git
```

**Seam 1 is the whole insurance policy.** MediaWiki learned it in 2005 (decouple revision metadata from text storage); Logseq relearned it in 2023 and paid three years for it. If the query engine ever imports a path helper, the seam is gone and storage becomes irreversible.

**Seam 3 is the whole simplicity policy.** Every hard format problem is somebody else's solved problem. You shell out.

---

## 4. Data model

One object = one note = **one Markdown file**. Structure in frontmatter. Nothing else.

```yaml
---
id: 01J8ZQK4XN9P          # ULID. Links point at IDs, never filenames.
type: note                # note | task | asset
status: doing             # enum, EXCLUSIVE — a property, never a tag
due: 2026-07-20           # date | null
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T11:03:00Z
tags: [meta-rl]           # multi-valued, unordered — NOT status
assets: [blake3:9f2a…]    # content hashes
code:  [meta-rl@a3f9c2e]  # git refs — immutable, cannot rot
---
```

**Three rules that are load-bearing:**

1. **The atom is the file, never the block.** No per-bullet IDs, no per-bullet timestamps. That is precisely what forced Logseq's rewrite. Want per-entry timestamps? Make the entries *files* — capture creates a new small file, and file mtime is entry mtime. Free.
2. **Status is a property. Tags are tags.** A kanban column is exclusive; a tag is not. Conflating them puts one task in three columns and you'll hack around it forever.
3. **The notes repo is text only.** Enforced by a pre-commit hook rejecting anything over 5 MB. Assets live on a different volume entirely.

---

## 5. Scope — cuts made

The previous plan added scope in every round and cut nothing. Simplicity is a budget; here it finally gets paid.

### v1 — build this

| | Why it survives |
|---|---|
| Capture box, always focused, no save button | The success criterion |
| Markdown + frontmatter, git auto-commit | Storage + free revision history |
| SQLite FTS5 over notes **and extracted PDF text** | The other success criterion |
| Query engine behind `Store`, plus `MemoryStore` | The seam. Build the fake store *first*. |
| **3 renderers: stream, board, gallery** | Stream = home. Board = "what am I doing". Gallery = find the figure. |
| Assets: content-addressed blobs, own volume | The media library |
| Ingest: drag/paste + `fm add` CLI | Two doors, not three |
| `pdftotext` extraction | **Highest-value feature in the system:** search inside every paper you've read |
| `libvips` image thumbnails | Required by gallery |
| `verify` | Makes reference-not-ingest safe |
| `restic` backup | Config, not code |

### Cut — and the honest reason

| Cut | Reason |
|---|---|
| **Calendar** | Does your work actually have *due dates*? Research mostly doesn't. If you can't name five things with real deadlines, this is cargo-culted from Notion. **Answer honestly before building it.** |
| **Wikilinks / backlinks** | Genuinely useful. Not needed to use the tool daily. v2. |
| **Whiteboard** | **Kill, don't defer.** Logseq built whiteboards and effectively abandoned them. It cannot be derived from the data model, so it's a second product wearing your product's clothes. Highest risk, lowest proven value. Use Excalidraw and link the file. |
| **Graph view** | Everyone builds it. Nobody uses it after week two. Cut permanently. |
| **OCR (`tesseract`)** | Slow, niche. Add when you actually hit a scanned PDF you care about. |
| **`pandoc` extraction** | pptx/docx text search is nice. PDFs are 90% of what you read. v2. |
| **Video thumbnails (`ffmpeg`)** | Add with your first video. |
| **Watched inbox folder** | A file watcher, with all its inotify pain, for a batch-import convenience. v2. |
| **GC** | You will have no garbage for a year. |
| **Auth / phone / Tailscale** | v2 — *but keep the auth hook in the request path from day one.* Retrofitting auth is miserable. |
| **Saved `.view` editor UI** | v1 ships three `.view` files written by hand. The renderers stay generic; only the editor is deferred. |
| **Semantic search, table view, timeline view** | v3, v2, and never (the stream *is* the timeline). |
| **Plugin API** | **Never.** See §7. |

---

## 6. Interaction

**Capture:** always-focused box. `⌘Space` from anywhere. Type. Done. No "new note" button — a button is already too slow.

**Save:** none. Typing stops → 500 ms → atomic write (temp + rename). Idle 30 s or blur → `git commit`. History accrues silently; this is Wikipedia's append-only model, using a binary you already have.

**Media in, three ways, one function:**
- **paste / drag** — small things. Server streams to temp while hashing; if the hash already exists, discard and return it. Dedup for free.
- **`fm add <path>`** — big things. Pushing 2 GB through a browser to a server on the *same machine* is absurd. Use `cp --reflink=auto`: instant and **zero extra bytes** on btrfs/XFS. Never hardlink — an in-place edit would mutate an immutable blob.
- All transports call **the same ingest function.** Only the door differs.

**Media out:** you wrote no viewer. Browser renders images, PDF, mp4, audio natively. Everything else → `xdg-open` on Ubuntu, download-and-open on a phone. **Every OS already ships a better viewer than you could build.**

---

## 7. Views are core. There is no plugin API.

Both competitors ran this experiment and converged.

Obsidian's original Kanban plugin kept boards in **its own format** — a silo, invisible to every other query. The Projects plugin people relied on for boards **was abandoned**. The community's replacement generates columns from *any property*. Obsidian is now pulling kanban and calendar into core. Logseq's DB rewrite made Kanban/Calendar/Gallery core views. **Two projects, different architectures, same destination.**

### The test that decides whether you got it right

> **The board renderer must not know what `status` is.**

Point it at `type`. You should get a board of notes / tasks / assets. If the strings `todo`, `doing` or `done` appear anywhere in renderer code, you built a *feature*, not a *renderer* — and you'll rebuild it the day you want to group by priority.

**Put that grep in CI.** It is the cheapest architectural guarantee in the project.

### And no plugin API, ever

A plugin API lets *strangers* extend your software without touching your source. **You have no strangers. You have git and an editor.** What it costs a solo project: a permanent API surface, refactors that break plugins (so you stop refactoring), a security hole into your entire private corpus, and dependencies that rot. Extensibility comes from CSS themes and declarative `.view` files — **no code execution**.

---

## 8. Failure modes — the pressure test, resolved

| Failure | Answer |
|---|---|
| Crash mid-write | Atomic write (temp + rename). The canonical file is never partially written. |
| Index drifts from files | **The index is disposable.** Reindex. Budget: <10 s @ 10k. If deleting the index feels scary, you built it wrong. |
| Two tabs edit one note | Last-write-wins + `mtime` check → warn the user. **Do not build CRDTs.** You are one person. |
| GC deletes a talk you needed | GC is **report + quarantine only**. Never auto-delete. Deferred to v2 anyway. |
| `verify` finds 200 dangling refs | Expected — mostly moved data paths. It **reports, never fixes**. Same contract as `git fsck`. |
| Disk full mid-ingest | Temp write fails; the blob never lands. Safe by construction. |
| `pdftotext` missing | Feature degrades (no PDF search). System runs. Every OS dependency is optional. |
| Notes git repo corrupts | It's git. `git fsck`, plus restic, plus (once you add one) a remote. |
| **Restic restore has never been tested** | **The real risk.** An untested backup is not a backup. Restore to a scratch dir in month one. |
| Reindex blows the 10 s budget at 50k | Cache by mtime+size. But measure before you optimize — you probably never hit 50k. |

---

## 9. Build order — S4 is the checkpoint that matters

| | Slice | Proves |
|---|---|---|
| S0 | Type a note → disk → reload → still there | The pipeline exists |
| S1 | Search box → FTS5 → results with timestamps | Retrieval works |
| S2 | Properties editable; `status` settable | The typed model holds |
| S3 | **Board:** group by `status`, drag writes back | The thesis is alive |
| **S4** | **Gallery:** a *second* renderer over the same query layer | **The thesis is proven — or dead** |
| S5 | Assets: hash, store, `pdftotext`, thumbnails | The media library |
| S6 | `verify` + restic + **a tested restore** | Durability |

**S4 is the whole gamble.** Anyone can build one board. If the second renderer costs a weekend, ADR-001 was right and every future view is cheap forever. **If it hurts, stop** — the query layer isn't real, you're building features, and it only gets more expensive from here.

---

## 10. Trade-offs, stated plainly

| Chosen | Given up |
|---|---|
| Files as truth | No atomic multi-object transactions; you *will* fight the file watcher; body text isn't queryable (structure must live in frontmatter — same constraint Obsidian shipped deliberately) |
| File-level atom | No block references, no per-bullet timestamps. **Forever.** If you ever want them, this plan is void. |
| Reference, don't ingest | Data-path refs can rot. `verify` is the price. |
| No plugin API | No ecosystem. You are the ecosystem. |
| Shell out to the OS | Portability beyond Ubuntu is not free. Acceptable — you use Ubuntu. |
| Go, single binary | Slower to write than Python. Bought: `scp` it and it runs, with a runtime nobody has to install. |

---

## 11. Open — exactly two questions

1. **Do you actually have deadlines?** Name five things with real due dates. If you can't, cut the calendar permanently and stop pretending.
2. **Have you run SilverBullet for two weeks?** Still the highest-return unwritten item in this plan. It's close enough to this spec that knowing *precisely* why it isn't enough would sharpen everything above — and if it turns out to be enough, you just saved six months and went back to doing Meta-RL.

---

## 12. First three commits

1. `Store` interface — five methods. Then `MemoryStore`. Then the query-engine test suite that passes **with zero filesystem access**. That test is not checking correctness; it is the seam, defending itself against you in eight months when you're tired and in a hurry.
2. `.githooks/pre-commit` — the 5 MB guard. `git config core.hooksPath .githooks`.
3. Perf budgets as CI assertions — *now*, while they're trivially easy to pass. A budget added after it's already violated gets deleted, not fixed.
