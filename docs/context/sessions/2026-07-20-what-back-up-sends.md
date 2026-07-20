# 2026-07-20 — Making "save" answerable, and letting small attachments travel

The owner asked what saving does twice across two days. That is the finding: the behaviour was
right and **unsayable from the screen**.

## What Back up is now

A **split button** in the top bar. The wide half commits and pushes **notes**, across every vault.
The narrow half opens the variants: *Back up notes*, *Get their changes*, *Backup options…*,
*Attachment settings…* — each with a second line saying what it does, because "commit and push"
is the answer to the question that kept being asked.

**Pushes stay user-triggered, not on a timer.** A push has a remote audience; a cadence that fires
on its own makes it an act nobody chose. Auto-commit remains local and unchanged.

## Attachments can travel, per vault, by size

`vault.json` gains `git_assets_max`. Absent — the default — means **notes only**, exactly as
before. Set it to `"2MB"` and blobs at or under that are committed and pushed with the notes.

**It lives in the vault, not in Settings**, and that was the owner's call when asked. The setting
decides what enters *shared, permanent history*: a per-device value would let the loosest machine
choose for every collaborator, and a pushed commit cannot be un-pushed — a 50 MB video committed
once is in every clone forever and removing it means rewriting history people have already pulled.
In the vault's own file, the rule travels with the vault.

Three implementation notes worth keeping:

- **`git add -f`, one path at a time.** `ensure_repo` puts `blobs/` in `.gitignore`, and git cannot
  filter by size, so the selection happens in `blobs_within` and each chosen file is named
  explicitly. The ignore rule is untouched — pinned by a test, because "un-ignore blobs/" would be
  the obvious wrong fix and would let everything through.
- **Zero cost when off.** No descriptor opinion → no walk. `commit_all` runs on a 5-second
  debounce, so the default path must not grow a directory scan.
- **Lowering the limit does not untrack what already travelled.** The bytes are in history the
  moment they are pushed, so a deletion commit would cost noise and buy nothing. The limit governs
  what travels *next*, which is the only thing it can honestly govern.

`Descriptor::set_git_assets_max` is the **narrow exception** to `write_new`'s never-overwrite rule.
It re-reads the file as raw JSON and replaces one key, so a description someone wrote and keys this
version has never heard of both survive — `read` keeps only the four it knows, so a naive rewrite
would silently eat the rest. Pinned by a test with a `future_key` in it.

## What the mock caught

`mock.ts` is bound to the real DTOs, and adding a field to `VaultInfo` failed `check-ui` in four
places immediately. That is the design working: a shape the Rust never sends cannot compile. The
mock now models the setting round-tripping, with one vault on and one off so both states are
visible in `pnpm dev`.

## Still open

- **Video on Android.** Bytes cross as base64 in a JSON string, so `MAX_INGEST` caps an attachment
  at 48 MB. Chunked ingest would lift it — and is deliberately queued *behind* the question of
  where a phone's blobs survive at all, since a phone vault is app-private storage with no restic.
- **Nothing marks an attachment as local-only in the note yet.** The owner asked for that when
  large media is kept on the device; the size rule now exists to drive it, and the marking does
  not.
