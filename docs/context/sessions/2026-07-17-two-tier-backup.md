# 2026-07-17 — Two-tier backup: push notes by default, restic media on request

**Outcome:** the **Back up** button opens `BackupPanel.svelte` instead of firing
restic. The panel sets the vault's **git remote in-app**, pushes the **notes**
(squashing the unpushed window into one `backup:` commit), and offers **restic
with media** as an unchecked box. It states what each tier will and will not carry
*before* you act, and afterwards reports each tier's real outcome — including
whether the data actually left the machine. `pixi run ci` green; both tiers
verified end-to-end over the live API.

New: `fm-core/src/git.rs` gains `remote`/`set_remote`/`unpushed`/`push_squashed`;
`fm-serve` gains `backup_status`/`set_git_remote`/`push` (**17 → 20 commands**) and
a `BackupStatus` type; `ui/src/lib/destination.ts` (+tests) and `BackupPanel.svelte`.

## Why

Two data classes with nothing in common. Notes are small, plain, mergeable → git
carries them anywhere, authenticated by the user's own ssh-agent/credential helper,
so **the app holds no secret**. Blobs are heavy and git-ignored → only restic sees
them. And the old default was *broken by default*: restic needs `FM_RESTIC_REPO` +
`RESTIC_PASSWORD`, which `packaging/formicarium.sh` never sets — every desktop-icon
launch errored. The two `decisions.md` entries carry the full rationale and the
prior art (Zotero, git-annex, photo managers).

The owner's framing drove the shape: end users must be able to set the vault's
remote **from the app**, and the vault's remote has nothing to do with the app
source's remote. Storage is the vault's own `.git/config` — no new config file, and
decoupled by construction.

## The audit that changed the design

The previous plan asserted *"local git already auto-commits, so history is never at
risk"*. The owner asked for that to be checked. **It is half false**, and the
correction is now in `known-issues.md`: auto-commit swallows every error, its timer
dies with the tab (and closing the tab *is* how you quit), and CLI/Vim writes never
commit. Files are never at risk (atomic temp+rename, and `add -A` sweeps misses
into the next commit) — **commits are**. That is precisely why the panel reports
what happened instead of assuming.

## Load-bearing constraints (don't "simplify" these away)

- **Never squash the first push.** No tracking ref ⇒ "unpushed" means the *entire*
  history; squashing history that has never left the machine destroys its only
  copy. First push goes whole; every later one is a single commit.
- **Never fetch in the push flow.** The tracking ref is stale *on purpose*: a
  remote another machine moved stays ahead of it, so our push is **rejected**
  (verified). Fetch first and `reset --soft` would rebase onto their tip and
  silently overwrite their content with our tree.
- **`GIT_TERMINAL_PROMPT=0` + `GIT_SSH_COMMAND=ssh -o BatchMode=yes`** in `git()`.
  fm-serve is thread-per-connection with no TTY: a credential prompt would hang the
  request *forever* rather than fail.
- **A local destination is not "off this machine".** `reachOf` classifies both a git
  URL and a restic repo; the panel must keep saying which.

## Found while verifying

`git remote add origin ""` **succeeds**, and the resulting remote reports its own
*name* as its URL — so the vault would claim a destination it hasn't got. Surfaced
via `fm-serve`'s `serde_json::from_slice(body).unwrap_or(Value::Null)` +
`s(k).unwrap_or("")`, which turns malformed JSON into empty-string args (now noted
in `known-issues.md`; other commands are still exposed). `set_remote` refuses a
blank URL, with a test.

## Verified

Against a live `fm-serve` + a local bare repo: blank URL refused · push with no
remote gives *our* message, not git's stderr · first push sent 3 commits unsquashed
· 4 later auto-commits → **squashed to 1**, all notes present in the pushed tree ·
**0 blobs** in the notes repo · a diverged remote **rejected** our push and their
file survived · restic snapshot contains the blob git left behind and excludes
`index.sqlite`. Rust: 6 → 9 tests in `tests/git.rs` (bare repo, no network, no
credentials). UI: 134 tests, incl. `destination.test.ts`.

## Left not-working

- **Ambient auth against a real GitHub remote is unverified** — no credentials in
  this environment. Everything was proven against a local bare repo. The owner
  should confirm one real push.
- **The panel's UI was never clicked** (no display): its logic is thin glue over
  the API paths above, which were exercised directly, but the markup is unproven.
- **No auto-push, deliberately.** An auto-push that swallows errors — as the
  existing auto-commit does — is a backup that silently isn't. The panel shows
  "N commits not pushed" instead. Revisit only with visible failure.
- Restic remains env-var-only and unusable from the desktop launcher; making it
  first-class for end users needs config UI **and a password store**, which
  `roadmap.md:64` rejects. The blob-mirror tier (now in `roadmap.md`) is the more
  promising direction.
