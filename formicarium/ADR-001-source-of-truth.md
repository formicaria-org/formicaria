# ADR-001: Source of truth and object granularity

**Status:** Proposed
**Date:** 2026-07-14
**Blocks:** everything else. Write nothing until this is decided.

---

## Context

The v0.1 requirements assumed "Markdown files + rebuildable index" without interrogating it. Research into projects that actually shipped (or died) shows the framing was wrong.

**The question is not "files or database." It is "what is the atom."**

Once granularity is fixed, the storage answer largely follows.

---

## Evidence from projects that already made this bet

### Logseq — chose block granularity, was forced to abandon files

Logseq's founder (tienson) states the reasons plainly:
https://discuss.logseq.com/t/database-version-too-drastic-choice/20346

> Using Markdown files as the true facts brings some challenges:
> 1. Bad sync experience due to file system complexity; renaming a page is slow because every file that referenced it must be rewritten and re-synced.
> 2. Building proper and fast real-time collaboration on top of Markdown files is almost impossible.
> 3. Limited structured data support compared to a DB — **not every block has a persistent id, no timestamps**.

**Reason 3 is the load-bearing one.** Logseq's atom is the *block* — every bullet is an addressable object requiring an ID and timestamps. Markdown cannot carry per-block metadata without ceasing to be standard Markdown. That is an unwinnable fight, and they lost it.

The payoff of the rewrite is precisely our target feature set — Kanban view, Calendar view, Gallery view, properties, tag/class:
https://discuss.logseq.com/t/logseq-og-markdown-vs-logseq-db-sqlite/34608

Outcome: the project **split into two products**. Files version is now in maintenance mode; the DB is canonical and Markdown is an export.
https://github.com/logseq/docs/blob/master/db-version.md

**Cost of the rebuild: roughly three years and a user base split in half.** This is exactly the outcome we are trying to avoid.

### Obsidian — chose file granularity, kept files, shipped the same views

Obsidian Bases (core plugin) is a saved query configuration layered over Markdown + YAML frontmatter. Table, cards, list, map views. The data stays in the notes; the `.base` file stores only *which notes, which columns, how to filter and sort*.

https://deepwiki.com/obsidianmd/obsidian-help/5.1-introduction-to-bases

This is a **working existence proof of "one model, many views" on plain files.** It did not require abandoning the file format.

Its stated, deliberate constraints — read these as the price of admission:
https://chughkabir.com/guide-obsidian-bases/

- **Bases does not read or query the note body.** All queryable data must live in YAML frontmatter. Called out explicitly as a performance/design choice, not an oversight.
- **Inline fields are invisible** to the engine (the Dataview `Rating:: 5` style).
- **No native relation type.** Plain text has no foreign keys. Relations must be simulated with wikilinks inside a list property, with no referential integrity.

### Dendron — chose files, *skipped the query layer*, metadata died

Dendron shut down for business reasons, not technical ones — no product-market fit for a venture-backed company; the founder pivoted:
https://github.com/dendronhq/dendron/discussions/3890

But its own FAQ contains the diagnosis that matters:
https://wiki.dendron.so/notes/683740e3-70ce-4a47-a1f4-1f140e80b558/

> Most PKM tools help you create notes but slam into a wall retrieving them once your knowledge base reaches a certain size threshold... virtually everything stops working past 10k notes.

> **Metadata is currently under-utilized because we don't have a built-in way of easily querying by it.**

Dendron had files. It had frontmatter. It had hierarchy. **It had no query layer, so the metadata was inert.** This is the negative control for our thesis: structured properties without a query engine are decoration.

**Also note:** because Dendron was local-first and open source, abandonment did not destroy anyone's data or workflow. Users kept working. For a solo project whose largest risk is the author's own burnout, **file-based storage is bus-factor insurance.** This is a real argument, and v0.1 underweighted it.

### SilverBullet — files + Lua query engine, actively maintained

https://silverbullet.md/ · https://lwn.net/Articles/1030941/
Plain Markdown files, self-hosted single binary, Lua scripting for queries and custom views. Approaching 2.0. Another data point that files + query layer is viable.

---

## The decision

> **Atom = the file. One object = one note = one Markdown file. Structure lives in YAML frontmatter. No per-block IDs, no per-block timestamps, ever.**
>
> Files are canonical. SQLite is a disposable index.

### Why this follows from the evidence

Obsidian proves this configuration supports exactly the views we want (kanban, calendar, filtered boards). Logseq's failure came from block granularity, which we are explicitly refusing. Dendron's failure came from omitting the query layer, which we are explicitly building.

We are taking the intersection of what worked and avoiding both documented failure modes.

### The trap this creates — read it before you agree

**"Every message should have a timestamp, like a GitHub comment" is a granularity landmine.**

- If *message* = a note (one file) → safe. Frontmatter carries `created`/`updated` natively.
- If *message* = a bullet inside a note → **you have just specified Logseq's atom** and inherited reason #3 of their rewrite.

If you ever find yourself wanting per-bullet timestamps, per-bullet IDs, or block-level backlinks: **stop.** That is not a feature request, it is a request to change the atom, and it invalidates this ADR. Either refuse it, or reopen this document and accept a rewrite.

---

## Honest limits of one-file-per-object

Accept these now, in writing, so they are not discovered as bugs later.

| Limit | Consequence | Mitigation |
|---|---|---|
| **No atomic multi-object transactions** | Moving 5 tasks between columns = 5 separate file writes. A crash mid-operation leaves inconsistent state. | Write-temp-then-rename per file. Accept that bulk ops are not atomic. Add a `verify` command (like `git fsck`). |
| **No referential integrity** | Delete a note that others link to → dangling links. The filesystem will not stop you. | `verify` command detects dangling refs. Never auto-delete. |
| **No native relations** | Links are wikilinks in a list property. No foreign keys, no cascade, no join guarantees. | Same as Obsidian. Live with it. Resolve links through the index. |
| **File watcher is unreliable** | inotify has watch limits. Editors write via atomic-rename → you see delete+create, not modify. Syncthing writes partials. | Debounce. Treat the index as advisory. Make reindex cheap (<10s) so drift is recoverable, not fatal. |
| **Rename cost** | Logseq's reason #1. Renaming a page rewrites every file that references it. | **Stable ID in frontmatter, links by ID, not filename.** This kills the rename problem outright — it is the single most important mitigation on this list. |
| **Body content is not queryable** | Structure must be in frontmatter. If it is in the body, it is invisible to kanban/calendar. | Same constraint Obsidian shipped deliberately. Enforce it: if you want to filter on it, it is a property. |
| **YAML parse cost on reindex** | 10k files × parse. | Measured, not feared. Budget: full reindex <10s @ 10k. If violated, cache by mtime+size. |
| **Filesystem quirks** | Case-insensitivity (macOS), path length, unicode normalization, illegal filename chars. | Filenames are *display only*. ID is truth. Sanitize aggressively. |
| **No real-time collaboration, ever** | Logseq's reason #2. | Already a non-goal. Do not revisit. |

---

## The modularity mechanism — how to not rebuild

Requirements said "scale modularly without rebuilding continuously." That does not happen by accident, and it does not happen by choosing the right storage. It happens by **putting a seam in the right place.**

```
  Views  (kanban, calendar, timeline)   ← swappable, cheap
    │
    ▼
  Query engine  (filter / sort / group)  ← THE STABLE CORE
    │
    ▼
  Store interface  ← THE SEAM. Guard it.
    │
    ├── FileStore     (markdown + SQLite index)   ← v1
    ├── MemoryStore   (tests, no I/O)
    └── SqliteStore   (if ADR-001 is ever reversed) ← escape hatch
```

**Rules that make the seam real:**

1. The query engine may **never** touch the filesystem. Not once. Not for a shortcut. If it imports a path or an `fs` call, the seam is already broken.
2. The store interface exposes only: `get(id)`, `put(object)`, `delete(id)`, `query(filter, sort, group)`, `reindex()`. Nothing filesystem-shaped leaks through — no paths, no mtimes, no directory handles.
3. **Build `MemoryStore` on day one**, before `FileStore`. Every query-engine test runs against it. If the tests need a filesystem, the seam is fake.
4. Views may never touch the store. Views talk to the query engine only.

**This is the insurance policy.** If in eighteen months you conclude Logseq was right and files must go, the migration is: implement `SqliteStore`, write a one-shot importer, swap. The query engine and every view survive untouched. That is the difference between a swap and Logseq's three-year rewrite.

The seam is worth more than the storage decision behind it. **The storage decision is reversible if and only if the seam is real.**

---

## Consequences

**Easier**
- Backup, sync, git, grep, external editors — all free, all inherited.
- Survives abandonment (Dendron's users still work today).
- Kanban/calendar are proven achievable in this configuration (Obsidian).
- Migration to DB-as-truth stays cheap, *if the seam holds*.

**Harder**
- File watcher will cost real time. Budget for it. It is not a bug, it is the tax.
- No atomic bulk operations. Design UI around this — no operation should need to move 50 objects atomically.
- Body content stays unqueryable. Discipline required: if you want to filter on it, it goes in frontmatter.

**Revisit when**
- You want per-block timestamps or IDs → the atom is changing → this ADR is void.
- Reindex breaks the 10s budget at your real corpus size.
- You find yourself building conflict-resolution logic → you are reinventing sync → stop and reconsider DB-as-truth.

---

## Action items

1. [ ] Answer, in one sentence: **is a "message" a note or a bullet?** Everything above depends on it.
2. [ ] Write the `Store` interface (5 methods) before any storage code exists.
3. [ ] Implement `MemoryStore` first. Query-engine tests must pass with no filesystem.
4. [ ] Add reindex-time and search-latency assertions to CI now, while the corpus is small and they're easy to pass.
5. [ ] Before S3 (kanban), install Obsidian and build one Base. It is the reference implementation of your architecture — use it to steal the constraints, not the code.
