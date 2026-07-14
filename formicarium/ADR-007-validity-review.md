# ADR-007: Validity review of PLAN.md and its tooling

**Status:** Review — findings require decisions before S0
**Date:** 2026-07-14
**Mode:** Adversarial. Reviewing my own plan, looking for what's wrong.

---

## Summary

The plan is **structurally sound and tactically under-specified**. The seams (Store, Blobs, OS) are right and evidenced. But three things were hand-waved, one of which is the seam the whole architecture rests on, and one of which — if answered a particular way — **deletes the single largest source of pain in the entire design.**

Two tool choices were made for the wrong reasons and survive on better ones. Two should change.

| # | Finding | Severity |
|---|---|---|
| 1 | The file watcher may not be needed **at all** | **Critical — could remove the #1 risk** |
| 2 | `Store.query()` is undesigned, and it's the seam | **Critical — blocks S0** |
| 3 | CodeMirror forces a build step, which destroys HTMX's rationale | **Major** |
| 4 | Go + SQLite implies cgo, which breaks the static binary | **Major — undisclosed trap** |
| 5 | Go was justified on the weakest argument available | Moderate |
| 6 | blake3 → should be sha256 | Moderate |
| 7 | The cold-start budget is measuring the wrong thing | Moderate |
| 8 | Auto-commit gives you undo, **not** readable history | Minor — honesty |
| 9 | The 4-weekend kill criterion is probably optimistic | Minor — but it matters |

---

## Finding 1 — You may not need a file watcher. Ask the question.

ADR-001 named the file watcher as the tax you'd pay for choosing files: inotify limits, atomic-rename semantics, partial writes, index drift. I then said "the index is disposable, just reindex" and moved on without ever asking **when reindex is triggered.**

The watcher exists for exactly one reason: **something other than the app writes to the notes directory.**

> **Do you actually edit notes outside the app?**

**If no** — the app is the sole writer. It updates the index **in the same transaction as the write**. There is no drift, because there is no second writer. **The file watcher does not exist.** The largest identified risk in the entire project evaporates.

**If yes** (Vim, `git pull` from another machine, Syncthing) — you need to detect external changes. But even then, **inotify is not the answer**:

| Approach | Cost |
|---|---|
| inotify watcher | Real pain: watch limits, rename semantics, partial writes, debouncing |
| **Reindex on startup + a manual "reindex" button** | ~20 lines. A 10-second operation you run when you know you edited externally. |
| Poll mtimes every 30s | ~30 lines, no inotify, catches everything eventually |

**For a single user, a reindex button is entirely sufficient.** You *know* when you edited a file in Vim. You press the button. The full-fidelity file watcher solves a problem that only exists for software that must be invisible to strangers.

**Recommendation: no file watcher in v1.** Reindex on startup, plus a manual reindex command. Revisit only if you find yourself pressing it constantly.

This is the single biggest simplification available in the plan, and I missed it for six documents.

---

## Finding 2 — `Store.query()` is the seam, and I never designed it

Every ADR rests on: *the query engine talks to `Store`; swapping storage is therefore cheap.* But `query(filter, sort, group)` was never specified, and it is not obvious.

The tension: **property filtering** (`status = doing`) and **full-text search** (`"trust region"`) are different kinds of query, and both must cross the same interface.

| Option | Verdict |
|---|---|
| `query()` takes **SQL** | **Seam is dead on arrival.** `MemoryStore` cannot execute SQL. This defeats the entire architecture. |
| `query()` takes **a DSL you invent** | You now maintain a language. Deferred complexity, not avoided. |
| **`query()` takes a typed filter struct** | ✅ Bounded, designable, expressive enough for board / gallery / search and nothing more. |

### Concrete proposal — design this before S0

```go
type Filter struct {
    Type     []string             // note | task | asset
    Status   []string             // enum values
    Tags     []string             // AND semantics
    DateRange map[string][2]time.Time  // property name → [from, to]
    Text     string               // full-text predicate
}

type Query struct {
    Filter  Filter
    SortBy  []SortKey   // property + direction
    GroupBy string      // any enum property — see ADR-006
    Limit   int
}
```

**The critical trick:** `Text` is *just another predicate*.
- `FileStore` implements it with **FTS5**.
- `MemoryStore` implements it with a **naive substring scan** — correct, slow, and irrelevant on 50 test fixtures.

Both satisfy the same contract. The seam holds. Search is not special.

**This must be written before any storage code exists.** It is the load-bearing interface, and it is currently a comment in a diagram.

---

## Finding 3 — CodeMirror silently destroys the case for HTMX

The frontend was specified as *server-rendered HTML + HTMX + three JS islands (CodeMirror, SortableJS, File API)*.

HTMX's entire pitch is **no build step**. But CodeMirror 6 is a modular npm package that **requires a bundler**. The moment you add it, you have `package.json`, a bundler, and a `node_modules` tree — and HTMX's core advantage is gone. At that point Svelte is no more expensive than HTMX, and better at the interactive parts.

So the editor choice quietly determines the entire frontend stack. It deserved a decision and got an aside.

| Option | Build step? | Verdict |
|---|---|---|
| **Plain `<textarea>` + preview toggle** | **None** | ✅ **v1.** Markdown in a textarea is what Memos ships. For *capture*, syntax highlighting is decoration. |
| CodeMirror 6 | Bundler required | Then use Svelte, not HTMX — HTMX has lost its reason to exist |
| TipTap / WYSIWYG | Bundler + a much bigger surface | No. It fights Markdown-as-truth. |

**Recommendation: plain textarea, HTMX, SortableJS and HTMX both vendored as single files, zero npm.** No `node_modules` anywhere in the project. Add CodeMirror only if writing in a textarea genuinely hurts after a month of real use — and accept that adopting it means adopting a build step.

---

## Finding 4 — Go + SQLite means cgo, and cgo breaks the static binary

Undisclosed trap. The standard driver (`mattn/go-sqlite3`) is a **cgo** binding. cgo compromises the exact property Go was chosen for: cross-compilation becomes painful, and the binary is no longer cleanly static.

**Fix:** use **`modernc.org/sqlite`** — a pure-Go transpilation of SQLite. No cgo. Genuinely static binary. **FTS5 is included**, because it transpiles the real C source.

It is roughly 2–3× slower than the cgo driver. At 10k rows and one user, that is invisible. **Take the pure-Go driver. Never think about it again.**

Had this gone unnoticed until S1, it would have meant reworking the build.

---

## Finding 5 — Go was right, for the wrong reason

I justified Go on **"single static binary, easy distribution."** That argument is weak here: **you are deploying to one machine that you own.** `apt install python3` is not a hardship. The distribution argument only matters when shipping to strangers, and you have none — the same logic I used to kill the plugin API.

**The real argument is dependency rot, and it's much stronger.**

This tool is meant to hold a decade of your work. A Go binary compiled in 2026 still runs in 2036. A Python project with fifteen `pip` dependencies **will not install cleanly in 2036** — transitive pins break, wheels vanish, the ecosystem moves. For a tool whose entire value proposition is *longevity*, that is disqualifying.

| Option | Complexity | Longevity | Speed to write |
|---|---|---|---|
| **Go + `modernc.org/sqlite`** | Low | **Excellent** — static, no runtime, no deps | Medium |
| Rust + `rusqlite` (bundled) | Medium-high | Excellent | Slow |
| Python + FastAPI | **Lowest** | **Poor** — venv/dependency rot over a decade | **Fastest** |

**Go stands. The reasoning in PLAN.md §10 should be corrected**, because a decision defended by a weak argument gets overturned the first time someone attacks the weak argument.

---

## Finding 6 — Use sha256, not blake3

blake3 is ~5× faster. On a 2 GB video: ~0.3 s versus ~2 s. **Both are fine.**

sha256 buys something blake3 does not: **universality**. You can verify any blob with `sha256sum`, from any machine, forever, with no tooling. For a content-addressed store meant to outlive the application, **debuggability beats 1.7 seconds.**

**Reverse the decision. sha256.**

---

## Finding 7 — The cold-start budget measures the wrong thing

PLAN §2 inherits *"cold start < 500 ms."* But the server is a **systemd user service**. It starts once, at login, and then runs for weeks. **Cold start is nearly irrelevant.**

What actually determines whether the tool feels fast:

| Replace | With |
|---|---|
| ~~Cold start < 500 ms~~ | **Time to interactive capture box < 200 ms** |
| — | **Search results rendered < 100 ms** @ 10k objects |
| — | **Keystroke → paint < 16 ms** (no typing lag, ever) |
| — | **Reindex < 10 s** @ 10k |

The Logseq complaint you're actually reacting to was never RAM. It was **8-second cold starts and input lag**. Since you're running a daemon, only the second one can happen to you — so measure *that*.

---

## Finding 8 — Auto-commit gives you undo, not history. Say so.

I sold git auto-commit as "Wikipedia's revision model." Partly true. But committing every 30 seconds produces **thousands of commits titled `auto:`**, and `git log` becomes noise. You will not read it as a narrative.

**That's fine — but be honest about what you bought:**
- ✅ **Undo, at any granularity, forever.** Crash-safety. `git checkout` recovery. Real value.
- ❌ **A readable history.** You will not have one. Do not try to fix this — do not build "commit management." The notes themselves are the narrative.

Minor risk: `git add -A` scans the tree on each commit. Fine at 10k files; measure if it grows.

---

## Finding 9 — The kill criterion may fire falsely

*"The board isn't working after 4 weekends → stop."*

For an experienced web developer, plausible. **You are a Meta-RL researcher, not a web developer.** S0 through S3 involves Go, SQLite, HTTP, HTMX, drag-and-drop, and atomic file writes. Four weekends is optimistic, and a kill criterion that fires falsely is worse than none — it kills a healthy project.

**Recommendation: 8 weekends to a working board.** Keep the *other* two criteria unchanged — especially "two weeks without opening it," which is the honest one, because it measures whether the tool is solving your problem rather than whether you're fast at Go.

---

## Consequences

**What becomes easier**
- No file watcher (pending your answer to Finding 1) — removes the largest identified risk in the project.
- No npm, no bundler, no `node_modules` — plain textarea + vendored HTMX + vendored SortableJS.
- Pure-Go SQLite → genuinely static binary, trivially reproducible build.

**What becomes harder**
- `Store.query()` must be designed *properly*, before S0. This is real work that was previously invisible.

**What to revisit**
- If writing in a textarea hurts after a month of real use → CodeMirror, and accept a build step.
- If you press "reindex" more than once a week → then, and only then, build a watcher.

---

## Action items — before writing any code

1. [ ] **Answer Finding 1: do you edit notes outside the app?** If no, delete the file watcher from the plan entirely.
2. [ ] Write the `Store` interface and `Query` struct. Nothing else compiles until this exists.
3. [ ] Amend PLAN.md: sha256 not blake3; textarea not CodeMirror; `modernc.org/sqlite`; corrected perf budgets; 8-weekend kill criterion; Go justified on **longevity**, not distribution.
4. [ ] Confirm `modernc.org/sqlite` has FTS5 enabled in a five-line spike. **Do this first** — if it doesn't, Finding 4's fix collapses and the language decision reopens.
