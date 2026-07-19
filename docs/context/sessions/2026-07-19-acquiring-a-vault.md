# 2026-07-19 — Acquiring a vault from elsewhere (git and restic), and a silent data-loss fix

The owner's ask: *"I shall be able to pull a known vault in a device with formicaria using
formicaria… not only git. Any of the currently supported methods is ok (restic too)."* Then two
clarifications that changed the design, and one that changed it back:

1. Auth happens once per method per device, delegated to each tool's own store.
2. It must be **general enough that new methods are easy to add**.
3. **p2p eventually**, and *"these vaults can be git init or not, that's not a core necessary
   requirement of formicaria."*

## Two framings discarded before writing anything

**"The carrier is a git bundle."** Elegant — a p2p layer would only ever move one file, and
history/merge semantics stay untouched. Rejected because it promotes git from *an optional
capability* to *the definition of a vault*, contradicting what the app already promises
(`SettingsPanel.svelte`: "git: not installed — notes are still files and still safe";
`dispatch.rs:915-918` deliberately not calling `ensure_repo`). Non-git vaults would have become
unshareable second-class citizens.

**"A vault is a directory, so any directory copy moves it."** The first half is true and
deliberately defended. The second half is **false in both directions**, and that finding is the
whole design.

## What a vault actually is: three tiers

| Tier | What | Where |
|---|---|---|
| **Portable** | `notes/`, `blobs/`, `manifest.json`, `vault.json`, `.gitattributes`, `.gitignore`, `.git` | in the vault |
| **Machine-local — must NOT travel** | `index.sqlite`, `.git/config`'s merge driver, `.git/config`'s committer identity | in the vault |
| **Registration** | name + **absolute** path + restic repo | `vaults.json`, *outside* the vault |

**No transport moved exactly the right set.** git moves notes minus media (yet `manifest.json`
*is* tracked, so a clone asserts blobs it lacks). restic moves notes + media minus history,
identity and descriptor — and restores to `dest/home/ada/vault/…`. A verbatim copy moves
everything *including tier 2*.

## The prerequisite: a live data-loss path, fixed

`install_merge_driver` (`git.rs`) registered `merge.fm.driver` as an **absolute** path to `fm`,
and its guard was one-directional — `let Some(exe) = merge_command() else { return Ok(()) }`
left a stale entry in place. There was no `git config --unset` anywhere in the workspace.

Every way that path goes stale is real: reinstall to a different prefix, a dev build where a
release one ran, a package shipping `fm-serve` without `fm`, mobile (which has no `fm` beside
it at all), or a vault directory copied between machines — `.git/config` travels with a *copy*
even though git declines to carry it in a *clone*. Git then reads the driver's non-zero exit as
"conflict" and hands back `%A` untouched, which the module's own comment already spelled out:
*"The user sees a conflict, opens a file that looks completely normal, resolves it, and has
silently deleted their collaborator's edit."*

**Fix:** the `None` branch now calls `clear_merge_driver`, so the definition is removed rather
than left dangling — degrading to git's built-in text merge, which is uglier and *visible*.
Pinned by `a_stale_merge_driver_is_removed_rather_than_left_pointing_at_nothing`, written to
assert the real invariant (*never names a binary that isn't there*) so it holds whether or not
an `fm` happens to sit beside the test binary.

## The model: two axes, four verbs

|  | **copy** (one-shot) | **sync** (ongoing, two-way) |
|---|---|---|
| **in** | **Get a copy** — restic, and later rclone/p2p/zip | **Join** — git clone |
| **out** | **Send a copy** — later | **Publish** — attach a git remote |

**`sync` requires git; `copy` works with anything.** Not a preference: there is exactly one
merge engine (`merge::merge_texts`), it is git-shaped, and `plan.md:67` rejects a CRDT/sync
framework. Two-way exchange without a merge engine is last-writer-wins.

So a non-git vault *can* be shared — what you shared is a copy, not a collaboration, and the UI
says so in those words.

## The seam

Not a transport registry and no `Transport` trait. Two shared functions plus a deliberately
stupid contract:

- **`fm_core::acquire::naturalise`** (new module) — tier 2, in one place. Drops an arriving
  `index.sqlite`, forgets an inherited committer (`git::forget_identity`, also new), re-runs
  `ensure_repo` so the merge driver points at *this* machine or at nothing, creates
  `blobs/`+`derived/`. Deliberately never `git init`s a vault that has no history.
- **a transport** — moves bytes. That is all.

Every acquisition path routes through `naturalise`, so **a new transport cannot corrupt a
vault**: it touches neither the merge path nor the safety. That property is what makes "easy to
add methods" safe rather than alarming, and it is why no plugin API is needed — the extension
point is the filesystem, which is also why a folder synced by other means is already adoptable.

## What shipped

- **`backup::restore_vault`** — restores the latest **`fm`-tagged** snapshot (tag-filtered
  because a restic repo is frequently not ours) and *lifts the tree out of the source's
  absolute path*, which plain `restore` recreates. Stages inside `dest` so the move is a rename
  on one filesystem; **refuses before moving anything** if a name already exists.
- **`backup::latest`** — the snapshot's own recorded paths, which are the last surviving record
  of a custom notes dir. `common_parent` is pure and unit-tested; a test caught it returning
  `/` for unrelated paths, which would have lifted the filesystem root.
- **`Descriptor::write_new`** — writes `vault.json` back when the notes were in `docs/`. Never
  overwrites (`read` keeps only three keys, so a rewriter would delete the user's description
  and any key a newer version added). Without it a restored `docs/` vault opens looking in
  `notes/` and shows nothing, with no reason given.
- **`restore_vault` dispatch arm** — same shape as `clone_vault`; validates everything pure
  before touching the network. **No identity required**, and the asymmetry is deliberate: a
  clone has an audience by definition, a restore has one user by definition. Records the restic
  repo in the vault entry, since it is the only thing connecting a restored vault to anywhere.
- **`ping.restic`** and **`config.restic_installed`** — a capability, like `git`. Three restic
  questions (installed / configured for this vault / unlocked) were answerable only as one.
- **NewVault: three modes, one component** — Start empty / Join a shared one / Restore a backup.
  Routes the machine cannot take are **absent**, not present-and-failing, so Android shows two.
  The restore caveat is stated *before* the button: notes and attachments come back, history
  does not.

## Verified

`pixi run ci` green (workspace tests, `cargo-deny`, the four architectural greps, mdBook).
`pixi run test-ui` 197 passing. `pixi run android-check` exit 0. New: 5 restic round-trip tests
(including "refuses over existing content, changes nothing" and "a repo with no formicaria
snapshot says so"), 2 merge-driver/identity tests, 3 descriptor-writer tests, 5 pure unit tests.

## Deliberately not done

- **No rclone, no p2p, no "Send a copy" yet.** The out-row's honest form today is "write a
  folder or a zip", which the OS already does; its value appears once a transport is attached.
- **restic's snapshot set is unchanged** — it still excludes the vault root by design, so
  restore remains a *recovery*, not a *join*.
- **No auth UI.** Delegation to each tool's own store holds on desktop. **On Android none of
  those stores exist** — no terminal, no credential helper — so the phone forces a real
  Keystore decision; it should be made against a working second transport, not in the abstract.

## The risk to record before the next transport

`notes/` and `blobs/` are two independently-transported tiers with no shared transaction:
`verify` grades a referenced-but-missing blob a *Warning* because it is expected to be
transient. **A transport that moves the directory whole inverts that** — those vaults have
atomic completeness — and `vault.json` has no field recording which kind a vault is. The same
warning would mean "still in flight" for one population and "permanently lost" for the other.
Decide that when the second bulk transport lands, not after.
