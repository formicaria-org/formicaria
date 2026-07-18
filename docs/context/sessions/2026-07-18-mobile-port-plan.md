# 2026-07-18 — Mobile port: adversarial review, then the corrected plan into the repo

**A planning session, not shipped code.** The owner brought a large plan for putting
formicaria **on the phone itself** (overriding `MASTERPLAN:57`'s "phone = thin client to the
laptop's server"), already claiming an internal adversarial hardening, and asked for a *critical*
review against the project's principles — open/user-owned/not-vendored, easy free-remote sync,
low-latency, corruption-robust, multi-user/multi-project, maintainability, longevity, efficiency —
then to fix the contradictions and land the corrected picture in the repo.

## What the review did
Three parallel code audits (git/merge, the UI/`ipc.ts` layer, `fm-serve`/`fm-app`/`vaults.rs`/
`blob.rs`/`ingest.rs`) checked the plan's line-anchored claims against the actual tree, plus a
pass against `decisions.md`. The plan was strong and self-aware, but rested on **one false
premise**, **silently picked the more invasive of two architectures**, and **reversed two carried
decisions without saying so**. The load-bearing findings:

- **"One command library, nothing forks" is false today.** `fm-cli` does **not** front `fm-app`
  (no dep; it reimplements against `fm-core` — `fm-cli/src/main.rs:131-146`). `fm-serve::api()`
  is a second surface; a Tauri bridge would be a third. (New `known-issues.md` trap.)
- **`diffy` is unnecessary and breaks "reuse mature tools" on the corruption-critical path.**
  `git2` — already being added — exposes `merge_file`/`MergeFileOptions` with our/their/ancestor
  labels + marker size (libgit2's own `git merge-file`). Verified against the git2-rs docs.
- **Two carried decisions reversed unflagged:** a Keystore-held token vs *"the app stores no
  secret of its own"*; `git2`-in-`fm-core` vs *"git is a capability, not a dependency."*
- **Multi-user provenance gap:** the plan wired the token but never `set_identity` on clone, so
  every phone commit would be the `PLACEHOLDER_EMAIL` "you" — the hole Phase 0 closed.
- **`auto-push after commit` wedges silently** against `push_squashed`'s deliberate
  reject-on-moved-remote (the best-effort-and-silent known-issue, on the sync path).
- **The "planned streaming `GET /api/blob`" does not exist** — blobs still buffer whole into RAM.
- Factual: several "critical files" mis-pathed (`NotePanel`/`Whiteboard`/`Pane` are in
  `ui/src/lib/`, not `renderers/`); "only 3 functions touch the network" understates a port where
  **all ~13 git ops shell out**.

## What landed in the repo (docs only — no product code)
The corrected plan, in the project's own working-memory shape (`plan.md` sequence +
a `*-design.md` receipts file + reconciled `decisions.md`/`overview.md`/`known-issues.md`):

- **`docs/context/mobile-design.md`** (new) — the full corrected design + code audit behind
  Track M: the `fm_app::dispatch` extraction (ruling 1), the `git2`/HTTPS-only port and
  `git2::merge_file` engine, PAT-first auth + injected per-URL `CredentialSource` + `set_identity`
  on clone, the streaming blob route, the explicit auto-push loop, the `SyncProvider` seam
  (sole impl, `history` as a queried optional capability), the free-serverless backend menu, the
  staged spikes→M0–M8 sequence, and re-weighted risks (NDK-outside-pixi #1).
- **`plan.md`** — a compact **Track M** pointing at the receipts; `_Last updated_` bumped; the
  `MASTERPLAN:57` override noted under stale-lines.
- **`decisions.md`** — a new top entry recording the rulings, the two reversals (owned in
  writing), and the rejections (PWA, CRDT, global-closure creds, killing the merge driver, Path A
  as the end state).
- **`README.md`** index, **`overview.md`** (`_Last verified_` + a pointer), **`known-issues.md`**
  (the `fm-cli`≠`fm-app` fork trap + bumped verified line).

## The corrected architecture, in one breath
Extract `fm_app::dispatch` so there is genuinely **one** command surface; `fm-serve` and the
mobile shell are thin over it (Tauri bridge primary; `fm-serve`-on-device documented as the
lower-fork fallback, since a PROD `ipc.ts` already speaks HTTP). Port `git.rs` to in-process
`git2` (HTTPS-only, `gix` push isn't shipped) — the single desktop backend too; body merge via
`git2::merge_file`, desktop driver kept installed, differential-test gated. PAT-first auth,
Keystore-scoped; `set_identity` on clone. Build the real streaming `GET /api/blob/<hash>` for both
platforms. Lifecycle-driven reindex + incremental cold-start. Sync-as-a-seam with `GitSyncProvider`
as the only impl, free-serverless backends default, self-hosted first-class — "no CRDT / no sync
framework" intact. **Path B (on-device git) is the destination; Path A (transport-only) a
low-risk early demo, not the end state.**

## Not done / carried
- **No product code changed** — this is the plan + the reconciled context, nothing mobile exists.
  The spikes (extract `dispatch`; `git2::merge_file` + differential test; a git2 clone/push spike)
  are the first real work when execution starts on the owner's machine.
- The `fm-cli`↔`fm-app` fork is now a documented trap; ruling 1 is what collapses it.
- Standing order still in force: this is a remote session — **no push/PR**; handed back as a
  patch for the owner to apply locally.
