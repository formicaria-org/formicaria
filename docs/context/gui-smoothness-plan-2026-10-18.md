# Smoothness on the phone — findings and the order to fix them (2026-10-18)

> **Status: built 2026-10-18** except fix 10 (dropped) and proposal pushes (left) — see
> `decisions.md` 2026-10-18. The data-loss bug below is fixed. Kept for the reasoning.
> Originally **a plan.** Read-only research: a scout, an analyst, an adversarial
> check (three agents, ~440k tokens). Where this disagrees with the code, the code wins — every
> line reference here was true on 2026-10-18 at `0429600` (v0.8.7).

**The ask.** The owner sees the interface glitch on the phone "when multiple things are going on,
like backup and opening something else", and wants it smooth **keeping exactly the current setup and
features**. So every fix below is judged by one test: *does a person notice anything except that it
is smoother?* Where the answer was yes, the fix was changed or dropped.

## The finding that comes first: a pull on the phone can undo a save (data loss, today)

Not a smoothness item — found on the way, and it gates two of the fixes below.

- `fm-core/src/git_native.rs:1139-1143`: a **fast-forward** moves the ref, then
  `checkout_head(force())`. Force makes the working tree match the new HEAD — every tracked note with
  changes not yet recorded is put back, **including notes the incoming commits never touched**. The
  real-merge path does the same after its merge commit (`~:1270`). No dirty check before either.
- The desktop's `git merge` (`git.rs:2084`) refuses when a dirty file would be overwritten and
  leaves the others alone. So: **phone, iOS and Windows-without-git only** (the libgit2 backend).
- Reached because `syncOnce` sends `commit` and `pull` as two commands with no lock between them
  (`ui/src/lib/sync.svelte.ts:417-441`); a save landing in that gap is reverted on disk. 2026-10-17's
  *a backup when you start editing* makes the gap coincide with typing. Today the editor usually
  survives it (the next save's `base` mismatches → "both versions"), which is why it has not been
  seen as loss.
- **Fix (C):** in the `pull` arm, under the lock, record `written` + adoptable files exactly as the
  `commit` arm does, before `vcs::pull` (both backends). And check out with SAFE, not FORCE, before
  moving the ref and after a merge. **Test:** under `test-native-git`, two clones of a bare repo; D2
  edits Y and sends; D1 writes X unrecorded and pulls; X must keep D1's bytes (fails today). Second
  case with a local commit on D1 to cover the merge path.

## Why it glitches — the causes, verified

| # | Cause | Where | Verdict |
|---|---|---|---|
| 1 | **`push` and `pull` hold the one vault lock across the network** (and `commit` across `git status`). Opening a note (`get`), every feed, `ping`, `asset_status` and the `fmblob://` image handler all queue behind a backup, then land together: the stall-then-jump | `fm-app/src/dispatch.rs:1746-1777`, `:1605-1656`, `:2550`; `mobile/src-tauri/src/lib.rs:1092` | Confirmed. A **deliberate exception**: 2026-07-17 `#vault` (*the guard is dropped before I/O everywhere except commit/push/pull*), cost already noted 2026-09-10 |
| 2 | Opening, closing or changing a pane calls `refresh()` — every whole-corpus feed, `activity`, `thread_roots` — and a view change then refreshes **again** through the `feedKeys` effect | `ui/src/App.svelte:223, 272, 280`, `:1108-1113` | Confirmed (double fetch) |
| 3 | Every refresh **replaces** `feeds`, `threadCounts` and activity wholesale, so the visible view and every row reading `lastEditFor` re-render on identical data. Commit/push/pull bump the generation, so a refresh follows every backup and every save (twice per edit) | `App.svelte:1059`, `:1032`; `activity.svelte.ts:16-21`; `dispatch.rs:350-353` | Confirmed. The bump itself is **load-bearing** — keep it, make the redundant refresh paint nothing |
| 4 | Theme CSS re-written on every `vaultTick` even when unchanged → whole-page style recalc (only with a custom theme) | `App.svelte:432-466`; `appearance.ts:160-168` | Confirmed |
| 5 | An arrival re-adopts the open note → read view rebuilt, asset URLs revoked, images re-requested, scroll lost | `NotePanel.svelte:413-458`; `render.ts:169-191` | Partly (only on a real change, but then the whole view) |
| 6 | Board builds the column list once **per card** → every card's props change per refresh | `renderers/Board.svelte:~157` | Confirmed (small) |
| 7 | Commands queued behind the lock each occupy a tokio worker (`#[tauri::command(async)]`), so even lock-free commands stall | `mobile/src-tauri/src/lib.rs:480-520` | Inferred, plausible |
| 8 | The phone's fast-forward reports `incoming: 0` though it computed the count; the desktop returns it. So on the phone arrivals are not announced and the open note is not refreshed until the next beat | `git_native.rs:1130`, `:1143` vs `git.rs:2082-2086` | Confirmed — a *same on every OS* bug |

Checked and **left alone** because fixing them would change what a person sees: no windowing of
the board or the compact timeline (scroll and find-in-page change); the busy chip's width (the
resting-toolbar ruling, 2026-08-31); the 60 s `lastSaves` reassignment (it **is** the clock of the
"not sent" chip); `JSON.parse` of replies (inherent to Tauri on Android — fewer calls is the lever).

## The fixes, in the order to ship them

Each adjusted by the adversarial pass; the adjustments are part of the fix, not options.

| Order | Fix | Risk | Gain | Must keep |
|---|---|---|---|---|
| 1 | **C** — record before pull; SAFE checkout (above) | low | — (correctness) | a SAFE refusal surfaces as the desktop's error does |
| 2 | **Theme**: write `<style>.textContent` only when it differs | none | med (theme users) | `clear()` leaves an empty sheet, so a re-apply still writes |
| 3 | **Board**: one `$derived` column list, not one per card | none | low-med | name and comment free of the renderer CI's banned words |
| 4 | **Read view**: skip the rebuild when (element, id, body, `adoptCount`) are unchanged; otherwise keep `scrollTop` around `renderInto`. Skip before `revokeAssets()`; record the key only after success | low | med | the `.read` div is new after each edit — keying on id+body alone leaves it **blank** |
| 5 | **Unchanged reply, same object**: compare the raw reply text per command+args (both transports already hold it, `ipc.ts:95, :204`) and reuse the parsed object; for `view` feeds compare at the feed level | low | med-high | not `JSON.stringify` — on the phone that costs what it saves |
| 6 | **Pane refreshes**: drop `changePane`'s (the effect covers it); keep `addPane`'s and `closePane`'s but only when a note was saved since the last refresh | low | high | closing a note is what puts its new title on the board |
| 7 | **B** — the phone's fast-forward returns `incoming`. **Only after C**: with C unfixed, B turns a visible "both versions" into a silent adopt of the reverted file | low | med | "N changes arrived" now appears on the phone, as on the desktop |
| 8 | **The lock split** (cause 1) — its own change, `decisions.md` reversal of the 2026-07-17 exception written first. Detail below | med | **high** | everything in the list below |
| 9 | Mobile shell runs the command body in `spawn_blocking` (cause 7) | low | med | short `decisions.md` note: the doc comment there argues against `async fn` |
| 10 | Optional: per-session cache of positive `asset_status`, cleared on a vault-list change and every `changed` beat | low-med | low after 8 | a forgotten vault still shows "not available" |

### The lock split, as it must be built

A global `App.git: Mutex<()>` (per-vault later). `push`: resolve the path under `vaults`, drop it,
take `git`, run `push_squashed`. `pull`: take `git`, resolve, drop `vaults`, **fetch**; re-take
`vaults`, re-resolve (abort if the vault was forgotten or moved), **record unrecorded saves**, then
merge + `keep_notes` + `reindex` under both. `vcs::pull` splits into `fetch` / `merge_fetched` in
both backends. Invariants:

1. **C is fixed first.** With fetch outside `vaults`, a save during a phone's multi-second fetch is
   otherwise reverted every time, not rarely.
2. Order is always `git` → `vaults`. Many arms hold `vaults` and then call git (`reply`,
   `create_proposal`, accept/reject, `copy_note`, `resolve_conflict`, `commands.rs` proposal
   functions): `git` is taken at the **top** of each, enforced by a thread-local "vaults held"
   assertion in `App::git()`.
3. Every `.git` writer takes `git`: commit (incl. the agent's `commit_all_as`), push, pull,
   `resolve_conflict`, `mark_kept_seen`, proposals, `set_remote`/`set_identity`, clone/import,
   `move_vault`/`forget_vault`/`restore_vault`, `ensure_repo`. Same poison recovery as `App::lock`.
4. Squash + push + verify + rollback stay atomic under `git`. **The desktop squash must stop asking
   `git status`** (`git.rs:1902` sees unsaved files, then `git commit` fails with no rollback): use
   `git diff --cached --quiet <base>` as native does, and roll back on a failed commit.
5. Read-only status calls (`conflicted`, `unmerged_paths`, `unrecorded`, `backup_status`) run with
   `GIT_OPTIONAL_LOCKS=0`, so they cannot hold `index.lock` under a push's commit.
6. Tests reuse `dispatch_concurrency.rs`'s rendezvous stub: a blocked push must **not** block
   `get`/`ping`/`asset_status`/`blob_path_of_kind`, and **must** block `commit`; the same pair for a
   blocked fetch; forgetting the vault mid-fetch aborts cleanly.

Proposal pushes (`push_branch`, `commands.rs:1055/1092/1156`) have the same shape; fold them in or
leave them for a second pass — they are rare.

## Measuring

Nothing here was measured on the device. Before and after 1–6 and again after 8, a WebView trace on
the phone (backup started, then a note opened) is the check that the stall is gone — following
`phone-is-not-a-test-machine` (formicaria only, nothing installed).
