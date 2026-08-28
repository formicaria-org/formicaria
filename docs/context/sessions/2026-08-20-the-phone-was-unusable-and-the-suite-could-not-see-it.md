# 2026-08-20 — the phone was unusable, and the suite could not see it

**The report.** *"I have tried for a few weeks the mobile app and it is not usable: freezes with big
notes, freezes when asking to add a photo, lagging. This happens because we do not have a proper test
suite that recreates from the simplest functions to the most expected uses of the app."*

The diagnosis was right in substance and wrong in one detail worth recording, because the detail is
the reusable part.

## There was a test suite. It had never executed the phone.

39 UI test files, 55 Rust test files, wall-clock perf budgets in `fm-core/tests/perf.rs`, and a green
`pixi run ci` throughout the four weeks the app was unusable. **Four independent things each made the
mobile path unreachable from CI, and not one of them looked like a hole:**

1. `ipc.ts` chose its backend from a module-scope `const isTauri = … '__TAURI_INTERNALS__' in window`.
   A `const` is evaluated at import, so no test could ever take the Tauri branch without resetting
   the module registry and re-importing everything that transitively pulls `ipc.ts` in — which for
   `App.svelte` is all of it.
2. jsdom reports a fine pointer, so every `(pointer: coarse)` branch was dead code. `App.svelte`
   said so in a comment: *"Verified at a narrow viewport and by `pointer: coarse`, NOT on a device."*
3. `mock.ts`'s `ingest` arm hashed the **filename** and discarded the bytes. The zero-byte-photo class
   of bug — which had already shipped once — was structurally invisible to every UI test.
4. `mobile/src-tauri` is workspace-excluded, so `b64_decode`, which every phone photo passes through,
   had never been compiled by a test, let alone run by one.

**None of these produced a failing test. They produced an absence, and an absence looks exactly like
coverage.** That is the lesson to carry to the next platform: when a platform's defects are not
reproducing, check first whether any test executes that platform's branches *at all*.

Two smaller instances of the same shape turned up while fixing it: `asyncUtilTimeout` had been raised
to 5000 while `testTimeout` stayed at vitest's default 5000, so a query that needed to retry killed
the test first and reported `Test timed out` with no element name; and the mock's fixture dates were
literals (`due: '2026-07-11'`), so `App.flow.test.ts` had gone red when the calendar rolled into
August — a test failing on the clock rather than on the code.

## Why it froze: one structural cause, then five multipliers

**The structural one.** Three facts nobody had composed:

1. Android never gets Tauri's async custom-protocol IPC —
   `tauri/scripts/ipc-protocol.js`: `const canUseCustomProtocol = osName !== 'android'`. It falls
   back to `window.ipc.postMessage`.
2. That is an `@JavascriptInterface` method, and wry runs the handler **inline**
   (`wry/src/android/binding.rs`: `(ipc.handler)(request)` on the calling thread). A JS→Java bridge
   call is synchronous, so the page's JS thread is parked until Rust returns.
3. A plain `#[tauri::command]` is `ExecutionContext::Blocking` — the body runs before the call
   returns.

Net: **the UI could not paint for the duration of any command.** Not a slow one — any one. That is
why the phone presented as *freezing* rather than as slow, and why every other cost on the platform
showed up as the app locking up instead of as latency. The desktop never saw it, because `fm-serve`
is thread-per-connection. One word — `(async)` — fixes it, and `ci/checks.sh` now greps for it.

**The multipliers**, each of which is a freeze only because of the above:

| What | Where | Cost |
|---|---|---|
| A full-corpus `backlinks` scan on **every 500 ms autosave** | `NotePanel.svelte` — two `$effect`s read `note?.id`, and `save()` reassigns `note` | `load_all()` per keystroke pause |
| The 1.5 s discussion poll running `proposal_for` every tick | `NotePanel.svelte` `pollAgent` | full-corpus scan + a libgit2 open, 40×/min |
| 40 guaranteed-failing IPC round trips per minute | `agent_activity_poll` exists only in `fm-serve`; the phone dispatched it and got *unknown command* | plus a `log::error!` each |
| A per-vault network `ls-remote` every 45 s **and on every return to the app** | `App.svelte` gated on `PROD`, and a Tauri build **is** `PROD` | seconds of dead UI on mobile data |
| A full FTS table scan on every save | `fts` is `id UNINDEXED`, so `DELETE FROM fts WHERE id = ?` cannot seek | grows with the vault |
| ~10 copies of every photo across the bridge | `ipc.ts` → JSON → JNI marshal → serde → decode | ~600 MB peak at the old 48 MB ceiling → renderer kill |
| A full index rebuild on **every** cold start | `FileStore::open` was unconditionally `Reindex::Full`; Android kills backgrounded apps constantly | the common path, not the rare one |
| Serial, undeduplicated resolution of every image and note chip | `render.ts` `for … await` | one blocking round trip each |

**Two were correctness bugs, not merely cost.** Typing in a plain note's body silently **closed an
open comment thread** — the `:1006` effect's `else` branch sets `discOpen = false`, and it re-ran on
every autosave. And the phone was logging 40 errors a minute at a command that does not exist there.

## Two things I was wrong about, kept because the reasoning teaches

- **Thumbnails.** I read "no thumbnails on Android" as an Android gap. It is not: the read view
  never requests a thumbnail on **any** platform — `has_thumb` has no consumer and `assetUrl` has no
  `kind`, so every inline image is the full blob on desktop too. And the cheap-looking partial —
  pass `kind: "thumb"` from the phone — would have **404'd every image**, because `resolve_asset`
  hard-errors on a missing thumb and `vipsthumbnail` does not exist there. Deferred to M8 and
  recorded; mitigated with `loading="lazy"`.
- **Chunking the base64 encode.** It removes one copy out of ten; the other nine are on the
  transport. Lowering `MAX_INGEST` to 16 MB was the honest interim, and chunked *ingest* (over
  `BlobStore::put_file`, which already streams) is the real fix and is a transport change.

## The three defects I put in myself, and how they were caught

Worth more than the fixes, because they are a class:

1. **`reindex` called `forget_path` (which drops the `objects` row) before `index_object`**, so the
   remembered `fts_rowid` was already gone and the delete fell through to the `id` scan. The
   per-save budget was green the whole time, because `put` does not call `forget_path` — so the fix
   worked on the path I had a test for and not on the path I did not.
2. **`Store::delete` still deleted from `fts` by `id`.** I simply never looked at it.
3. **`DELETE … WHERE rowid IN (subquery)` is a full scan on an fts5 virtual table.** SQLite pushes
   `rowid = ?` down to `xBestIndex`, which fts5 answers as a lookup; it does **not** push down
   `IN`. Same for `WHERE id IN (...)` on an `UNINDEXED` column — evaluated per row *even when the
   subquery is empty*. Both forms were written during the seek work and both silently reintroduced
   the exact scan they were meant to remove.

(1) and (2) were found by re-reading one interaction after declaring the work done. (3) was found
only because a new budget failed — and then the budget itself was wrong twice before it was right:
it first divided the **whole** incremental pass by the changed-note count, which is mostly the
inherent O(n) stat sweep `perf.rs` already budgets separately, and read 3.7× on correct code; and at
40 changed notes the subtraction was noisier than the signal, swinging 1.07→1.59 run to run.

**A perf budget can fail on correct code, and can pass on broken code. Calibrate it against both
states, measured, before trusting either answer.** Every threshold added here has both ranges
recorded in the test:

| Budget | fixed | broken | threshold |
|---|---|---|---|
| per-`put` growth over a 4× vault | 1.17 – 1.20× | 2.16 – 2.24× | 1.6 |
| per-changed-note incremental poll | 1.19 – 1.43× | 2.82 – 2.89× | 2.0 |

## What shipped

**Fixes:** `(async)` on both Android commands (+ a `cancelled` stale-response guard the blocking
bridge had been hiding, + a `ci/checks.sh` grep); `noteId`/`feedKeys` memos so autosave stops
re-running effects; the discussion poll refreshes proposals on edges and backs off to 4 s when idle;
`agent_activity_poll` shell-answered; `checkRemotes` off the timer on a phone; `fts_rowid` seeks in
`index_object`/`forget_path`/`delete`/`reindex`; `MAX_INGEST` 48 → 16 MB; `loading="lazy"`; dedupe +
`Promise.all` in `render.ts`; `ColdStart::TrustIndex` on Android only.

**The suite that would have caught them:** `platform.ts` (a lazily-read platform predicate),
`harness.ts` (`asPhone()` — a fake `__TAURI_INTERNALS__` modelling the real bridge, plus a counting
ledger), a settable `matchMedia` in `test-setup.ts`, `mock.ts` gaining `seed()`/`reset()` and a
byte-consuming `ingest`, `fm_app::wire` (the pure encodings moved into the workspace so
`cargo test --workspace` reaches them), and new budgets in `big_notes.rs`, `cold_start.rs`,
`mobile_workload.rs`, `App.phone.test.ts`, `ingest.phone.test.ts`,
`NotePanel.bigNote.svelte.test.ts`, `NotePanel.poll.svelte.test.ts`, `pollBeat.test.ts`,
`remotePoll.test.ts`.

**Verification.** `pixi run ci` green; 77 Rust test targets; 45 UI files / 363 tests; the mobile
crate compiles clean for `aarch64-linux-android` (`cargo check --target aarch64-linux-android` from
`mobile/src-tauri`, which is the *only* way anything in that crate is compile-checked — `pixi run ci`
deliberately has no Android toolchain). **Every budget was watched failing before its fix landed.**

**What CI still cannot prove:** the synchronous JNI bridge is the mechanism behind the freezes and
nothing host-side reproduces it. The `(async)` fix is verified structurally and by compiling for the
real target, not by running. The acceptance test is the owner's: open a big note and type, add a
photo, leave the app and come back.
