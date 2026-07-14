# Formicarium — design set

**As of 2026-07-14. No code written yet.**

---

## Read in this order

| # | File | Why |
|---|---|---|
| 1 | **`PLAN.md`** | **Start here.** The consolidated working plan. Supersedes everything below as the thing you build from. |
| 2 | **`ADR-007-validity-review.md`** | **Read second.** Adversarial review of the plan. Contains **open findings that must be resolved before writing code.** |
| 3 | `ADR-001` … `ADR-006` | The evidence trail. Reference material — read when you want to know *why* a decision was made, or when you're tempted to reverse one. |
| 4 | `REQUIREMENTS.md` | Historical. The first draft, before scope was cut. Kept to show what was removed and why. |

---

## Status

| Document | Status |
|---|---|
| `PLAN.md` | **Working plan** — needs the ADR-007 amendments applied |
| `ADR-001-source-of-truth.md` | Accepted — atom = file, files canonical, SQLite disposable |
| `ADR-002-artifacts.md` | Accepted — reference artifacts, never ingest them |
| `ADR-003-asset-store.md` | Accepted — assets are a media subsystem on their own volume |
| `ADR-004-asset-pipeline.md` | Accepted — extract text, delegate viewing to the OS, build no viewers |
| `ADR-005-interaction-and-ingest.md` | Accepted — capture-first, no save button, one ingest function |
| `ADR-006-views-not-plugins.md` | Accepted — kanban/calendar are core views; **no plugin API, ever** |
| `ADR-007-validity-review.md` | **Open** — 9 findings, 2 critical, 4 action items |
| `REQUIREMENTS.md` | Superseded by `PLAN.md` |

---

## Blocked on two questions

Neither needs research. Both need an honest answer.

**1. Do you edit notes outside the app?** (Vim, `git pull`, Syncthing)
→ If **no**: the file watcher does not exist. The app is the sole writer and updates the index in the same transaction. This removes the single largest risk in the entire design.
→ *(ADR-007, Finding 1)*

**2. Do you actually have deadlines?** Name five things in your work with real due dates.
→ If you can't: **cut the calendar permanently.** It's cargo-culted from Notion and you will never open it.
→ *(PLAN §11)*

---

## The three ideas everything else hangs on

**One model, many views.** Objects with typed properties; kanban, calendar and gallery are a *query plus a renderer*, not features. Logseq spent three years refactoring toward this. Obsidian shipped it as Bases. If the second renderer costs more than a weekend, the architecture is wrong — stop and fix it there, while it's cheap.

**Guard the seams.** The query engine must never touch the filesystem. Enforce it with a test suite that passes against `MemoryStore` with zero I/O. That test isn't checking correctness — it's the architecture defending itself against you, in eight months, when you're tired and in a hurry. It is the difference between swapping a storage backend and Logseq's three-year rewrite.

**Every hard problem is someone else's solved problem.** git for history. restic for durability and dedup. SQLite FTS5 for search. `pdftotext` for reading inside papers. `xdg-open` for viewing. You write the catalog. You write nothing else.

---

## Before the first commit

Highest-return unwritten item in this entire set: **run SilverBullet for two weeks.** It's close enough to this spec that knowing *precisely* why it isn't enough would sharpen every document here — and if it turns out to be enough, you've saved six months and gone back to doing Meta-RL.
