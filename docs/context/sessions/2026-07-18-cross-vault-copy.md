# 2026-07-18 — Choose a vault on create, and copy a note between vaults

Two cross-vault gestures that multi-vault made necessary but that didn't exist:
picking **where a new atom is created**, and **porting a note into another vault**.
Because formicaria is files-as-truth, the second is "just a copy" — but a copy done
carelessly leaks another vault's contents, so the default is deliberately restrictive.

## Create-in-vault (mirror the `ingest` path)
`capture` was the only creation command with no `vault` argument, so every new
note/board landed in the default vault. Added one string end-to-end — exactly as
`ingest` already carried it: `commands::capture(store, body, vault)` sets `obj.vault`
(routing + unknown-name refusal already live in `MultiStore::route`); the server
`capture` arm validates via `config()`; `ipc.capture(body, vault)`; the mock honours
it. UI: a compact **destination `<select>`** in the top bar beside New note/New board
(`newVaultTarget`, persisted to `localStorage['fm-create-vault']`), shown only with
`>1 vault`, clamped to a still-existing vault. Asset drops are unchanged (they still
join the dropped-on note's vault).

## Copy a note — restrictive by default, sensible escalation
The governing principle (the user's, and it reshaped the design mid-plan): **a copy
carries only the prose. Vaults stay separated — ideas flow, artifacts do not.** A
copied note pushed to the target vault's remote must reveal nothing about the source
vault (no leaked hashes, ids, filenames, or dangling pointers).

- **`crates/fm-app/src/refs.rs`** (new, pure, no `regex` dep — hand-scanned) is the
  reusable core: `references(body) -> (asset_hashes, note_ids)` and
  `strip_cross_vault(body, keep_assets)`. Strip removes every `note:` link **always**
  and every `asset:`/`sha256:`/local-image reference unless `keep_assets`, replacing the
  **whole Markdown span** (label and all — a filename is itself a leak) with a fixed
  `⟨removed on copy⟩` marker. 6 unit tests.
- **`commands::copy_note`** — a copy is a **new note** (fresh ULID; same-id-in-two-vaults
  would make one unreachable via `MultiStore::get`). Prose-only by default: body stripped,
  `assets`/`code` cleared. `with_assets` opts in to carrying the first-degree blobs *into*
  the target (`BlobStore::put_file`, content-addressed → auto-dedup; target `manifest.json`
  refreshed) and keeps the asset refs — note links are still stripped (the linked-notes
  tier is deferred). Returns `CopyResult { meta, new_blobs }`. Refuses the note's own
  vault and an unknown target.
- **`commands::uncopy_note`** — recede a copy: delete the copied note, then remove each
  blob it newly wrote **only if no note remaining in the target still references it**
  (content-addressed, so a survivor may now be shared).
- Server arms `copy_note`/`uncopy_note` build the `(name, path)` list from `configs()` and
  best-effort commit the **target** vault.

## Sensitive → warn + undo (also the user's steer)
Writing into another vault is permanent in its git history, so the **NotePanel "Copy
to…"** popover (gated on `>1 vault`) states that plainly, defaults the "also copy the
files" opt-in **off**, and every copy leaves an **Undo** strip that calls `uncopyNote`.
Reuses the existing header-button/confirm-strip styling — no new modal system.

Two follow-on refinements the user asked for:
- **A second, warning-coloured confirm** (`--danger` tokens) when the copy is *sharper
  than plain prose*: carrying the files, and/or replacing an existing copy. A plain
  first-time prose copy still runs on one click; anything sharper arms a Cancel/confirm
  strip first (`requestCopy` → `copyStatus` pre-check → `copyConfirm`).
- **Override, never duplicate.** Re-copying the same note into a vault **replaces** its
  prior copy instead of accumulating a second. Provenance is a **one-way fingerprint of
  the source id** stamped as `copy_of: sha256(id)` (`fm_core::blob::sha256_hex`) — it lets
  a re-copy find and delete the old copy, and reveals neither the id nor the origin vault
  (embedding the raw `note:<id>` is exactly what we strip). `copy_note` returns `replaced`;
  `copy_status(id, vault)` is the pre-check behind the "will replace" warning. Undo receds
  the new copy (the replaced one is intentionally gone — the source is always intact).

## Invariants respected
`Object.vault` is still never serialized (the copy sets audience by *writing into* the
target `FileStore`, not by any field); unknown target = hard error; blobs travel so the
target is self-contained for its own audience; the UI never implies permission.

## Verified
- `pixi run ci` green: new `crates/fm-app/tests/copy.rs` (capture routing, prose-only
  leak-guard, `with_assets` blob+manifest+dedup, `uncopy` remove-vs-keep, error cases),
  `refs` units, and a UI test driving the Copy-to popover + Undo. 165 UI tests total.
- One gotcha found + fixed in the test seed: blobs are content-addressed, so `put_file`
  re-hashes — the seeded bytes must actually hash to the expected name (`b"abc"`).

## Deferred (unchanged from the plan)
- Fine-grained **copy first-degree linked notes** (`with_links`): reuses `references()`;
  each copied child is itself prose-only so no outward pointer survives at any depth.
- **V2 co-tenancy hygiene** (`plan.md`) still underlies writing into a real co-tenant
  project repo (the `git add -A` auto-commit etc.) — latent for a solo user.
- Standing order still in force: no remote CI until the owner lifts it.
