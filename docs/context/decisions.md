# Decisions — the *why*

Condensed, load-bearing decisions and reversals. Each entry is: **decision —
why — consequence**. The canonical, fuller spec is
[`formicaria/MASTERPLAN.md`](../../formicaria/MASTERPLAN.md); this is the
quick-recall version. Newest first.

**This is an append-only log — a decision is superseded, never edited away.** A reversal is added
as a new dated entry and the old one gets a `> SUPERSEDED …` banner pointing to it, so the *chain*
survives (the value of "we tried X, then Y" is the whole chain). Prune only exact duplication.

## Subject index — grep a subject, jump to the decision(s)

The [overview.md](./overview.md) router sends you here by **subject**; find it below, then grep the
heading. Retrieval is per-decision, never "load the whole 1,300-line log."

- **`#seams`** (the compile-time invariants): *`fm-query` may never touch fs/db* · *Generic,
  literal-free renderers* · *Files-as-truth; the atom is the file* · *`fm-cli` shares the command
  library; does not route through `dispatch`* · *Vaults are audiences* (the `candidates` seam).
- **`#git` / `#sync`** (git, merge, collaboration): ***A backend that cannot finish a merge must
  refuse to commit*** (read before touching either `commit_all` — the two backends had opposite bugs
  here) · *The in-process sync path: the app merges* ·
  *Git is a capability, not a dependency* · *`git2` is rejected* **⟶ + The libgit2 exception**
  (read the pair — it is a reversal chain) · *Notes merge through a driver that shells out* ·
  *Collaboration is git, exposed* · *Squash-on-push — a deliberate reversal* · *A commit that
  committed nothing must say why* · *Acquiring a vault: `naturalise` is the seam* · *Backup is two
  tiers*. **On-device proposal lifecycle:** `sessions/2026-07-24-proposals-on-the-phone.md`.
- **`#track-m`** (mobile/phone): *The owner's five Track M rulings* · *The Track M record drifted* ·
  *Mobile is the app on the phone, not a thin client* · *Android TLS: trust store from memory* ·
  *`fm-serve` sends a CSP* · ***Startup is a contract*** (read before touching the shell's `setup`
  hook or the render gate) · *An emulator may be installed to; the owner's phone may only be looked
  at*.
- **`#ui`** (workspace/views/render): *A contributor is an email, everywhere* · *One shell, two
  arrangements* · *`.view` files parsed
  server-side* · *The read view sanitizes* · *The note trail is a peer column* · *Browser is the
  product* · *Whiteboard = embedded Excalidraw* · *Board images strip to the blob store* · *Assets
  query-layer-excluded from planning views* · *Status rotates; card order is a view preference* ·
  *`start`/`due` are a `Stamp`* · *Tauri was the light choice; native-GUI rewrite rejected* · *v1
  editor = textarea + read view* · *Markdown→HTML is `marked`*.
- **`#vault`** (audience/cross-vault): *A vault is labelled by its remote, identified by its local
  name* · *A vault can be forgotten, and forgetting never deletes* ·
  *A caller is a member of some audiences, not all* (`Scope`
  — read before adding a read path or touching `find_blob`/`Vaults::config`) · *Vaults are audiences* · *Every entity shows its vault badge* ·
  *Cross-vault copy is restrictive* · *A vault is created, not invented* · *A vault gains identity
  when it gains an audience* · *formicaria: three pillars, one atom (the rename)* · *Which
  attachments travel: per-vault size limit* · *Content-addressed blobs*.
- **`#data`**: *The auto-commit stages what we wrote* (**+ the explicit catch-up**: it could
  *permanently skip* a note, not merely lag) · *A conflict surface is derived from git, not from
  markers* · *The lost-update token is a content hash* ·
  *The poll answers a comparison, not a report* (the generation counter — read this before
  touching `ping` or assuming one client).
- **`#toolchain`**: *The core ships as one file; pixi is the only package manager* · *The TLS
  exception: a self-signed leaf, share-only* (read before touching `rustls`/`rcgen` — the `ring`
  pin is a licence gate) · *Every external
  tool is an optional feature* · *No plugin API*.
- **`#agent`**: *Inline meeting actions become their own note* · *The study agent's model warm-up is
  deferred a few seconds after launch*. (Model/agent decisions that are not yet folded up live in
  `ai-agents-plan.md`.)

## A conflict surface must be derived from git, not from markers in the text (2026-07-31, `#git` `#data`)

**Decision.** `conflicts` is answered from **`vcs::conflicted`** — git's unmerged paths, each with its
kind — unioned with the old body-marker scan, and every entry says whether there are markers to edit.
`resolve_conflict(vault, path, keep)` keeps a side for the kinds that have none, **and finishes the
merge when it was the last one**. Both git backends implement it, and a test asserts they agree
byte-for-byte (`fm-cli/tests/conflict_resolution_both_devices.rs`).

**Why.** A surface derived from a *symptom* misses every case without the symptom. Markers are the
symptom of exactly one of git's seven conflict codes; a delete/modify (`DU`/`UD`) has none, because
one side has no blob and the `.md` driver is never called. So the app warned about a conflict it could
not display, and told the user in every message to *"open each one, both versions are marked in the
text"* — impossible advice, with no other action offered, while `commit_all`'s refusal to commit
mid-merge froze the whole vault. Measured cost on the owner's laptop: **7 days, 95 notes never
committed**, from one invisible note (`sessions/2026-07-31-the-gray-screen-on-first-open.md` covers
the same day's Android work; this one is in `known-issues.md`).

**Consequence.** Finishing the merge is part of `resolve_conflict`, not the caller's job: `commit_all`
stages only paths the app remembers writing, so it can answer "nothing of ours changed" and return
*without* committing — leaving `MERGE_HEAD` standing, which is itself what makes it refuse. Resolving
the last conflict and staying frozen would have been the same bug wearing a different hat.
**`keep theirs`/`keep mine` is deliberately not offered for a marker conflict**: there, both sides'
text exists and picking one discards the other, so the editor is the honest tool.

## `commit_all` may lag, but it must never *silently skip* — an explicit catch-up exists (2026-07-31, `#data`)

**Decision.** `vcs::unrecorded(vault, notes_rel)` lists notes on disk that git does not have, a
toolbar chip counts them, and `record_unrecorded` stages exactly those and commits. The debounced
auto-commit is **unchanged** — still only the paths this process recorded writing.

**Why.** The precision was right and its scope was wrong. Staging only recorded paths is what keeps a
vault that is also a project repo from having its owner's carefully staged work swept into an `auto:`
commit every five seconds — but that record is **per-process memory**, so a note written before the
last restart could never be staged by it. Not "history lags", which is what `known-issues.md` claimed:
history was *permanently missing* those notes, and nothing surfaced it. 95 had accumulated.

**Consequence.** The catch-up is **explicit and human-initiated**, never on a timer — the same
reasoning as *"auto-push is explicit, never silent"*: it stages files the app does not remember
writing, which is a judgement a person should make. And the count is a **persistent chip**, not a
banner, for the reason the "unreadable notes" chip is: the condition lasts until someone acts, and its
entire failure mode was silence.

## A vault is labelled by its remote, and identified by its local name (2026-07-31, `#vault` `#ui`)

**Decision.** `list_vaults` carries a `label`: the repository behind the vault's remote
(`…/formicarium-vault.git` → `formicarium-vault`), `null` when there is no remote. The UI shows the
label — badge, vault filter, tooltips, and the colour hue — while **every key stays the local `name`**.
`ui/src/lib/vaultLabels.svelte.ts` resolves one from the other, so the twenty-odd components that
already pass a vault name did not each have to learn about labels.

**Why.** The same repository cloned on two devices can carry two different local names — the owner's
laptop said `vault` where the phone said `notes` — so one *audience* looked like two different vaults
depending on which screen you were on. The remote is the thing both devices agree about.

**Why not simply rename the vault.** A vault's name is not a label: it is the write routing key
(`MultiStore::route`), the argument seventeen dispatch arms take, and the key behind the persisted view
preferences (`hiddenVaults`, `fm-board-order`, `fm-card-order`, keyed per vault *and* per group-by).
Renaming to match the remote would silently reset all of those and break any command in flight. So the
split is identity versus display — the same shape as `authorKey`/label for contributors, decided the
same day, for the same reason.

**Consequence.** The hue follows the *label*, so one repository is one colour on every device; keying
it on the folder name gave one audience two colours. **Two vaults cloned from the same remote fall
back to their local names — both of them**, not just the second: two identical labels make the vault
filter ambiguous, and hiding notes from the wrong audience is worse than showing a folder name.
`remote_label` reads local `git config` only — no `ls-remote`, no network — so the vault list does not
inherit the slowness that keeps `backup_status` off the heartbeat.

## A vault can be forgotten, and forgetting never deletes (2026-07-31, `#vault`)

**Decision.** `forget_vault` unregisters a vault: dropped from the live `MultiStore` and from
`vaults.json`. **It never touches a file.** The answer reports how many notes were left behind and
where, and the UI (Backup panel, two-step) repeats it — *"Removed 'x'. Its 209 notes are still on disk
at …"* / *"it was empty. Nothing was deleted."* Removing the last vault is allowed and lands on the
first-run screen, which is already a state the UI knows (`list_vaults` → `[]`).

**Why.** Three commands brought a vault into being — `create_vault`, `clone_vault`, `restore_vault` —
and **none took one away.** On a desktop that is a papercut you can fix by editing `vaults.json`; for
this owner it is permanent, because the product is the only way in. And the phone *manufactures* the
problem: `configure_paths` auto-creates an empty default vault on first launch, so the owner had one
they never asked for, could not use, and could not remove (2026-07-31: *"creates only confusion"*).

**Consequence.** "Forget" and "destroy" stay different verbs, and only the reversible one is built: a
vault dropped from the list is re-added by pointing at the same directory, so the worst case of a
mistaken click is retyping a path. That is what makes it safe behind one button — and why the message
must state the count, since "removed from the list" would otherwise read as "erased". The list is
saved to disk **before** the vault leaves memory: if the save fails nothing has changed, which is the
recoverable order. There is deliberately no "and delete the files" option; when someone wants that,
it is a separate decision with a separate confirmation, not a checkbox next to this one.

## A contributor is an email, everywhere — the name is only a label (2026-07-31, `#git` `#ui`)

**Decision.** `activity.svelte.ts` exports `authorKey(e)` = the lowercased email, falling back to the
name — and **both** the contributor chips and the note filter use it. `contributors()` returns
`{key, label}` so the key travels with the label and the two cannot drift.

**Why.** They had drifted. The chips already deduplicated by email, with a comment naming the exact
case (*"a vault signed `singhbal-baljinder` on one machine and `Baljinder Singh` on another is still
one human"*), while `shown()` hid notes by comparing the author **name**. So one chip represented one
person and hid only the spelling it happened to be labelled with. Measured in the owner's own vault:
**534 commits as `singhbal-baljinder`, 77 as `Baljinder`, 3 as `baljinder` in another vault — one
email.** Clicking the chip left 77 notes on screen while reporting "hidden".

**Which name counts, since three things are called one:** the **email** is the identity — it is what
git carries as the stable half, what GitHub attributes commits by, and now what this app groups by. A
git `user.name` is a display string. A **GitHub username** counts for almost nothing here: it is the
username field beside a PAT, and with a token that field is nearly free-form. And a **vault name** is a
local label for an audience, unrelated to any of them. Recording this because the owner reasonably
assumed the GitHub username was the load-bearing one; it is the least.

**Consequence.** `hiddenAuthors` in `localStorage` now holds keys rather than names. A stale entry from
before this change simply matches nothing — the filter fails *open* (the note is shown), which is the
right direction for a preference: a filter that silently hides notes after an upgrade would look like
data loss. The agent identities (`<model>@fm-agents.local`) are separate emails and stay separate
contributors, which is the intent — a model's commits are labelled as the model's.

## A backend that cannot finish a merge must refuse to commit — and "I edited it" is a resolution (2026-07-31, `#git`)

**Decision.** Three rulings, all forced by one differential test:

1. **Neither backend may commit while anything is unmerged.** The subprocess one always refused; the
   libgit2 one had **no such guard**, and `index.add_path` clears a path's conflict stages — so a
   debounced auto-commit five seconds after a conflicting pull committed the note **with its
   `<<<<<<<` markers as content**, dropped the merge's second parent, and pushed it.
2. **A merge in flight is committed as a merge**: `MERGE_HEAD` as the second parent, then
   `cleanup_state()`. A single-parent commit silently drops the incoming history; a `MERGE_HEAD` left
   standing freezes the vault for good (`push_squashed`, `merge_proposal_branch` and the next `pull`
   all refuse over an unfinished merge) — unrecoverable on a phone, which has no shell.
   The subprocess side had the mirror-image bug: it committed with `--only <paths>`, which git refuses
   outright mid-merge (*"cannot do a partial commit during a merge"*), so the ordinary resolution
   failed on every attempt and the vault stayed frozen behind a scary banner.
3. **`Keep::Edited` exists**, because **editing a note resolves nothing as far as git is concerned** —
   verified against real git: clean text over a `UU` path leaves all three index stages, so the path
   stays unmerged. The app's own instruction ("open each one, both versions are marked in the text")
   therefore settled nothing, and since the conflict *list* was derived from markers in the body, the
   note left the UI the moment the markers were tidied — taking the only sign of trouble with it while
   the vault silently stopped recording history. Git's verb for "I reconciled this" is `add`, and
   nothing called it. It is **refused while markers remain** (`merge::has_conflict_markers`, now the
   single definition shared with the conflict list): staging a marked-up file is what git reads as
   "resolved", and it would publish `<<<<<<<` to every collaborator.

**Why it was invisible.** `vcs.rs`'s own comment says the `#[cfg]` inside each routed function means
"the two backends cannot drift in shape without the compiler saying so" — true, and only for a build
that *has* the feature. `pixi run ci` does not build `native-git`, so shape drift reaches only the
phone. Behaviour drift is not checked by any compiler at all: both backends had `commit_all`, and both
were wrong, in opposite directions, on the same path.

**Consequence.** `ci/checks.sh` now greps that every `route!`d name exists in **both** backends (shape,
cheap, in the default gate), and the parity assertions live in `pixi run test-native-git` — where they
were written to fail first and did, catching both bugs on the first run. The check's failure message
says explicitly that a green grep is not evidence the backends agree.

## Startup is a contract: the store is reachable first, and no state renders a blank screen (2026-07-31, `#track-m` `#ui`)

**Decision.** Three rules, all now enforced rather than intended.

1. **The Android `setup` hook may not fail upward, and nothing slow may precede the store.** Tauri
   builds the webview *before* the hook runs (`tauri/src/app.rs:2521`), and the event loop that
   delivers an IPC reply does not start until the hook returns — so the hook's duration is a window
   in which the page is asking questions nobody can answer. Startup therefore lives in one
   `boot()`: `catch_unwind` around `App::load`, `manage` the instant the store exists, then the CA
   bundle and the model on a spawned thread. A failure is recorded in `BOOT` and **every command
   answers with it**, because a `?` there panics the shell's thread and leaves a live webview with
   no backend — which is not a crash anyone can report, it is a screen that never paints.
2. **`BOOT` is a `Mutex`, never a `OnceLock`**, and `boot()` is re-attempted by the first command
   that finds no store. The first version froze the first failure for the life of the process, so
   the UI's "Try again" re-asked, got the same stale sentence, and could not possibly help. A retry
   that cannot retry is worse than no button.
3. **No state of the render gate may paint an empty document.** `vaults === null` renders
   `Starting` — silent for 700 ms so a fast launch never flashes it, then a status, then at 8 s an
   escalation with the backend's verbatim reason and a retry. And a *refusal* is not an empty vault
   list: `.catch(() => (vaults = []))` offered to create a first vault to someone who has ten.

**Why.** The owner's phone opened to a gray screen on the first launch and worked on the second
(`sessions/2026-07-31-the-gray-screen-on-first-open.md`). Every ingredient was ordinary; the
combination was unreportable, because a phone has no console, no stdout in logcat, and (on MIUI) not
even our own tag. **The identical first-paint contention bug had been fixed on the desktop a week
earlier** — "the study agent's model warm-up is deferred", below — and nobody asked whether the phone
had the same shape. It did, and worse: there the *vault open* was in front of the first frame too.

**Consequence.** The boot poll is **time-driven, not rejection-driven** — the actual symptom was an
invoke that never settled, which no `.catch` can observe. `ci/checks.sh` greps the shell structurally
(no `?`/`unwrap`/`expect`/`panic` in the hook or in `boot`; the store managed before
`install_ca_bundle`/`agent::start`), because the mobile crate cannot compile in `pixi run ci` at all
and a rule nothing checks is a rule that lasts one refactor. `mock.ts` grew a fault surface
(`reject`/`hang`/`delay`) so the UI's refusal, stall and never-answers paths are testable — the
general form of a lesson `known-issues.md` had already recorded about one hand-written guard.

## An emulator may be installed to and force-stopped; the owner's phone may only be looked at (2026-07-31, `#track-m`)

**Decision.** Automated device tests target an emulator, and `ci/android-smoke.sh` **refuses to run
against any serial that is not `emulator-*`** — before it checks anything else, including whether its
own tools are present. The real-phone pass stays observation only (screencap via `exec-out`, which
writes nothing on the device; `dumpsys`; `ps`), with the owner doing the installing and the tapping.

**Why.** The test has to install, force-stop, `am kill` and uninstall to be worth anything, and the
device this project is developed against is the owner's personal phone. A guard that is a sentence in
a header is not a guard; this one is the first executable line and was verified to refuse with the
phone attached.

**Consequence.** `pixi run android-smoke` is opt-in, in the `android` feature, and deliberately **not**
part of `pixi run ci` — which must stay green for a contributor with no NDK. What it asserts is shaped
by the bug it exists to catch: **two consecutive cold launches must both paint**, because "fine the
second time" was the whole signature and a one-launch test would have gone green throughout. "Painted"
is `vips deviate` over a screenshot (a flat surface is ~0, a real screen is tens), with every measured
number written to `stats.txt` so the threshold stays grounded rather than guessed.

## The TLS exception: a self-signed **leaf**, share-only, and only for the microphone (2026-07-26)

**Decision.** `fm-serve` links `rustls` + `rcgen` behind a `tls` feature (default on;
`--no-default-features` still builds the std-only server, now CI-enforced). The shared listener
gets its own port and a **self-signed leaf certificate**, not a local CA.

**Why, in the shape of the libgit2 exception.** Sharing over plain HTTP was built first and
carries the whole feature — touch editing, photo and video capture (`<input capture>` is a
picker, not `getUserMedia`), ingest, blobs, whiteboard. The exception buys exactly one thing:
`getUserMedia`, which browsers disable outside a secure context and which no amount of
server-side care can grant over http. **You cannot hand-roll TLS**, which is the one place
`decisions.md`'s "40 lines against a dependency" stance does not reach. Terminating TLS in a
subprocess would have been more consistent with "invoke, don't link" and is the route to revisit
if a terminator ever resolves on all four `pixi.toml` platforms.

**Narrowly limited to:** the `tls` feature, `fm-serve` alone (`fm-core`/`fm-app`/`fm-query` link
nothing), and **`ring` backends pinned on both crates** — the default is `aws-lc-rs`, whose
licence includes `OpenSSL`, which `deny.toml` does not allow and which also wants cmake.
`ci/checks.sh` now fails if `aws-lc-rs` appears in `Cargo.lock`, because a transitive
default-feature flip is silent.

**A leaf, not a CA — the security call.** A private CA is the *convenient* design: re-mint the
leaf when DHCP moves and no paired device notices. Rejected. A root CA in a tablet's trust store
signs **any name for that device's entire browsing life**; its key would sit in a config
directory, be swept into whatever backs that directory up (including this app's own restic
backup), and outlive uninstalling formicaria. Leaked leaf key → one notebook server. The DHCP
problem it solved is answered instead by putting `<hostname>.local` in the SANs and advertising
*that*: mDNS is resolved by both iOS and Android, already advertised by avahi/Bonjour, and a name
survives a lease change where a bookmarked IP does not. The certificate is re-minted whenever the
machine's name set changes.

**No click-through path is documented, anywhere.** The 2026-07-19 Android TLS entry rejects
`certificate_check → CertificateOk` emphatically, for skipping hostname verification; telling a
user to dismiss an interstitial is the same act with the user as the actor, and under this design
a warning only ever appears on a certificate that is *not* the expected one. Instead the
certificate is a **file to carry across**, and its SHA-256 fingerprint is printed on the desktop
and shown in Settings so it can be compared against what the device displays before installing.
That comparison *is* the verification, performed by a human because a home network offers no
other root of trust. A device that will not trust it simply has no in-app microphone — a
supported state, with the message already written in `record.ts`. **Also rejected: HSTS** (it
would pin the address to https permanently; when DHCP hands that IP to a printer the user has an
error they cannot clear) and **fetching the certificate over the connection it authenticates**
(trust-on-first-use with no verification step at all).

**Consequence for the cookie.** `Secure` + a week's `Max-Age` on TLS; a **session** cookie on the
plain-HTTP fallback, because a long-lived bearer token in clear on a LAN is precisely the
`userpass_plaintext`-over-an-unauthenticated-connection shape the Android entry refuses.

## A caller is a member of some audiences, not all of them: `Scope` (2026-07-26)

**Decision.** `dispatch_as(.., &Scope)` narrows a caller to named vaults; `dispatch` keeps its old
signature and means `Scope::All`, so `fm-cli`, the phone, the study agent and the desktop's own
browser are untouched. Enforcement is `fm_core::Scoped`, a **view** of `MultiStore` over a subset
of its vaults, reached through `Vaults::store(scope)`.

**Why.** *Vaults are audiences* has always been the model, but every caller was the person at the
keyboard, so "sees everything" was correct by construction. It stops being correct the instant a
device can pair. Three specific ways the boundary leaked, each fixed at its own choke point:

- **Reads** federate through `Store::candidates`, so board/agenda/search/recent/activity/`.view`
  are all scoped by narrowing the set the read *runs over*. Filtering results afterwards would not
  be a filter but a redaction — the other audience's notes would already have been read.
- **`find_blob` searched every vault**, and its doc comment *justified* that: "whichever vault
  answers, the bytes hash to the reference." True, and exactly the hazard — content-addressing
  deduplicates, so a hash learned legitimately from a shared vault resolves against a private one.
  `GET /api/blob` is not a command, so it takes the scope explicitly.
- **`Vaults::config("")` resolved the default to `list[0]` *before* the store saw it**, so a
  capture from a scoped caller would have been filed into an audience it cannot read. The default
  is now the caller's first *reachable* vault. Found by a test, not by reading.

**Consequence.** An unknown vault and an out-of-scope vault give the identical error, so a caller
cannot probe for names it was not given; `list_vaults` is filtered, because the switcher and the
copy-to menu are built from it and a vault's *name* discloses. An empty scope grants nothing —
that inversion is how a device whose vaults were revoked would silently gain all of them.
`Scope::All` is a variant, not "a list of every vault", so a vault created later is included
rather than silently denied to the machine's own user.

**Where the halves live:** mechanism in `fm-core` (it needs the vault list, and a check that can
be forgotten is not a check); policy in `fm-app::scope`. Deliberately not a permission system —
no roles, no verbs. Location is the permission, exactly as on disk. Tests:
`fm-core/tests/scoped.rs` (mechanism), `fm-app/tests/scoped_dispatch.rs` (wiring — the half that
rots, since a new read path reaching `Vaults::all` would leave every mechanism test passing).

## The poll answers a comparison, not a report: `ping` carries a generation (2026-07-26)

**Decision.** `ping` takes the `since` the client last saw and answers `changed = generation >
since`, where `generation` is a monotonic per-process counter on `App`. It is bumped by any
command not on `dispatch::READ_ONLY`, and by a reindex that finds drift.

**Why — two separate defects, one cause: the answer was a *report*, and a report can only be
made once.**

- **Drift is consumed by whoever asks first.** `changed` was "did *this* incremental reindex find
  moved mtimes?" — but that same reindex writes the fresh mtimes back. The first client to ask
  got the news; every other client was told "nothing changed" indefinitely. `App.svelte` had
  carried a comment describing this since it was written, mitigated by an unconditional refresh
  on `visibilitychange` — a mitigation that **does not exist on a tablet**, which is held in the
  hand and never backgrounded.
- **A write through `dispatch` produced no drift at all** — the larger half, and it was not
  known. `FileStore::put` indexes the file it just wrote, mtime included (`file.rs::index_object`),
  so afterwards the index and the disk agree and an incremental reindex finds *nothing*. The poll
  therefore only ever saw **out-of-band** edits — a `git pull`, the merge driver, Vim. That was
  sufficient for exactly as long as there was one client, and it is why a second client would
  never have seen the first one's notes at all, not merely seen them late.

**Consequence.** N clients each hold their own cursor, so the server needs no identity, no
per-client eviction, and no definition of "gone" — and a client that misses a beat catches up on
the next one. `READ_ONLY` is an **allowlist**, so the failure mode of forgetting to classify a new
command is "one redundant re-query" and never "silently invisible to every other screen"; `ping`
is on it and bumps from inside its own arm instead, or the poll would report a change on every
beat forever. A `since` of 0 (a client with no cursor) is told `changed` deliberately: its initial
queries are not atomic with its first beat, so assuming it is behind is the recoverable error.
Refused writes do **not** bump — a stale `base` on a debounced editor would otherwise fan a
re-query out to every client on every keystroke.

**Consequence for the reader:** `ping` is no longer "is this vault dirty?" — it is "have I fallen
behind?", and those differ the moment there are two clients. Tests: `tests/poll_generation.rs`.

## The in-process sync path: the app merges, because libgit2 cannot (2026-07-19)

**Decision.** `git_native` now covers the whole collaboration loop — `clone`, `commit_all`,
`pull`, `push`, `unpushed`, `conflicts` — so a phone can share a vault with a desktop.

**Extended 2026-07-24 to the whole proposal lifecycle** (create / review / revise / accept /
reject), which had been left calling `crate::git` directly and therefore did not work on a phone
at all. The same ruling applies for the same reason — `merged_text()` is now the shared decision
both `pull` and `merge_proposal_branch` route through, so one vault cannot hold two merge
semantics. Two accept-path specifics are recorded because they are counter-intuitive and both
cost notes if reversed:

- **Move `HEAD` last.** Decide in memory → write working tree and index → commit. Committing
  first and checking out second leaves an *undetectable* half-state on a kill, which the next
  auto-commit silently converts into a revert of the accepted proposal. A detectable half-state
  (git's own `MERGE_HEAD`) is strictly better than a silent one.
- **Never `checkout_head(force)`; scope the checkout to the merged paths.** Unscoped, it reverts
  every uncommitted edit in the vault — including notes the proposal never mentions.

See `sessions/2026-07-24-proposals-on-the-phone.md`.

**The load-bearing part is the merge.** libgit2 contains no process spawn, so it can never
invoke the `.md` driver that makes two people editing one note a non-event rather than a
conflict on the `updated:` line the app rewrites on every save. Porting `pull` naively would
have silently disabled that: a collaborator running `git pull` in a terminal would still get the
structural merge while the app quietly did a worse one — **two merge semantics in one vault, and
ours the wrong one.** That was the single strongest argument in the original `git2` rejection.

So the app resolves conflicted paths **itself**, by calling `merge::merge_texts` — the *same*
engine `fm merge-md` calls. One engine, two call sites, which is what makes them unable to
diverge. This is why `merge_texts` had to be extracted from its path-shaped wrapper first: none
of this was safe until the engine could be called with three strings.

**Proven, not asserted.** `crates/fm-cli/tests/git_native_merge.rs` runs the same divergence
through both backends and compares: a concurrent edit to different lines of one note comes back
**byte-identical**, cleanly merged, on both. A genuine disagreement conflicts on both, with
markers **in the body** so the note still parses, and the path named rather than swallowed.

**That test lives in `fm-cli`, not `fm-core`, and the reason is a trap worth remembering:**
`ensure_repo` points the driver at the `fm` binary beside the running one, and only `fm-cli`
builds one. Run from `fm-core`'s harness the subprocess side silently falls back to git's plain
text merge and conflicts — so the comparison would grade two broken things against each other.

**Consequences and the things deliberately left out:**
- `repo.merge`, not `merge_trees`. The latter returns a standalone in-memory index that cannot
  be written or `add_path`'d into, so a conflicted pull would be invisible to every later call.
  `repo.merge` leaves a repo-backed index and files on disk, which is what real `git merge` does.
- **The squash is not ported.** `push_squashed` collapses history, guarded by an ancestry check
  and a rollback that verifies the remote ref actually moved. Reimplementing that on a backend
  that has never run against a real remote is exactly the half-shipping this project forbids on
  the path that must never corrupt. **A phone pushes what it has.**
- **Credentials are a PAT from the environment** (`FM_GIT_TOKEN`), and the callback **fails after
  one attempt** — libgit2 retries while it keeps receiving credentials, so a bad token is a hang
  rather than an error. The Keystore-backed source is the shell's job, the same way it supplies
  `FM_CONFIG_DIR`.
- Still unported: `activity` (git log) and `remote_moved`. Neither is on the corruption path;
  both degrade to "unknown" rather than to a wrong answer.

**Reversal condition:** the differential test cannot stay green → stop, because that is the
signal that a phone and a desktop have started disagreeing about what a merged note is.

## One shell, two arrangements — layout adapts by space, never by platform (2026-07-19)

**Decision.** The UI has **exactly two named layouts**, `tiled` and `single`, chosen by a
`layout: 'auto' | 'tiled' | 'single'` preference on the workspace. `auto` is the default and
follows available space. **Both are reachable on every platform** — `single` on a desktop,
`tiled` on a tablet.

**Why not a mobile frontend.** Putting the app on a phone made the tiled grid untenable: today's
answer stacks panes vertically, so you scroll past whole views to reach the next. The tempting
fix is a phone-specific UI. Two decades of cross-platform work says that is the expensive
mistake — `m.example.com`, a separate mobile app team, `Platform.select()` through a codebase:
all converge on content drift and two things to change per feature. What survived is **one
content layer inside an adaptive shell**, and **branching on space and input capability, never
on platform**, because platforms multiply forever while space is a continuum with two or three
thresholds testable at any window width.

This codebase already held that principle and under-implemented it: `App.svelte` argues pointer
beats width because *"a tablet is wide and still has no mouse"*, and explicitly rejected "a
phone-only component tree". This finishes the thought rather than reversing it.

**Consequences:**
- **No viewport-tracking TypeScript.** The whole mechanism is `data-layout` on the app root plus
  CSS; the only new state is a preference string, exactly like `fm-theme`. `single` hides
  non-active panes with `display: none` — every pane stays **mounted and fetched**, so switching
  is instant and the feed layer is untouched.
- **Desktop opt-in is the test, not a courtesy.** A narrow layout only a phone could run is a
  second frontend wearing a setting, and nothing would exercise it during ordinary desktop work.
  If `single` ever stops working in a desktop browser, the design has failed.
- **Hard stop at two.** A general "customisable frontend" is unbounded and lands on the plugin
  API `plan.md` already rejects. Two arrangements are bounded and both are tested.
- **The one setting Settings is allowed to own.** Everything else there is a read-only mirror
  (`vaults::save` is append-only). Layout is a *view* preference like the theme: this browser,
  no vault, nothing on disk.
- **Renderers moved to container queries.** They were querying the *window* while living in a
  pane sized `viewport ÷ cols` — a `colSpan:1` pane on a wide monitor got desktop-width board
  columns it could not fit. `Pane` is now `container-type: inline-size`; `Board`/`Card`'s width
  rules are `@container`, while `pointer: coarse` stays a *media* query because it is a
  capability, not a size. **Container queries are why this advice differs from 2010's** — a
  component can now ask its own box, which is what makes one renderer correct at any width.
- **Safe areas are paid for.** `viewport-fit=cover` plus `env(safe-area-inset-*)` on the shell,
  and `100dvh` rather than `100vh`. On a real phone the toolbar was painting under the status
  bar with the clock on top of the search field — invisible on the emulator.

**Deliberately not done:** swipe between panes. `Board` already uses `scroll-snap-type: x
mandatory` for its columns on narrow screens, so a horizontal pane swipe would compete with a
horizontal column swipe and make both feel broken. If it is ever wanted it must be an edge
gesture, judged on a device. And **no lazy per-pane fetching** — `MAX_PANES` is 8 and feeds
dedupe by `feedKey`, so it would trade instant switching for a loading flash.

**Reversal condition:** a third arrangement is genuinely needed (a tablet rail, say) → that is
the moment to check whether this has become the customisation system it refuses to be, not to
add a fourth.

## The libgit2 exception, and the discovery that `cargo deny` cannot enforce it (2026-07-19)

> **Reversal chain (`#git`):** narrows *"`git2` is rejected"* (2026-07-18, below) — which still
> holds **for the desktop**. This is the mobile-only exception. Later realized in code: the
> `native-git` backend + the `vcs` router now carry the full history *and* proposal lifecycle on the
> phone (`sessions/2026-07-24-proposals-on-the-phone.md`). **This entry is current.**

**Decision — one named exception, vendored, mobile only.** `fm-core` gains an optional
`native-git` feature (**off by default**) pulling `git2` with `vendored-libgit2`. The desktop
keeps shelling out to `git`; *"git is a capability, not a dependency"* still holds there, and
the `.md` merge driver keeps working for a collaborator's terminal `git pull`. **Why an
exception at all:** a phone has no `git` binary — Android ships none, iOS forbids executing
one. **Why it is safe:** libgit2 is GPL-2.0-only *with its own linking exception*, read
verbatim from the vendored `COPYING` at the pinned version; linking is precisely what that
exception permits. What survives is the **notice** obligation and the duty to publish any
modification to libgit2 — we make none.

**`git2 >= 0.21` is a floor, not a preference.** 0.20.4 — the version GitSync ships — carries
**RUSTSEC-2026-0183** and **-0184**, both `unsound`. The first is squarely on our path:
`Remote::list()` passes a null pointer to `slice::from_raw_parts` when a remote advertises no
refs, which is exactly the empty-remote case a first push meets. The advisories gate caught it
on the first resolve.

**The discovery, and it is the part that matters: `cargo deny` cannot enforce this, and never
will.** `libgit2-sys` declares `license = "MIT OR Apache-2.0"` — true of its Rust wrapper,
silent about the ~230k lines of GPL C it vendors — and cargo-deny treats a valid `license`
field as authoritative. `[[licenses.clarify]]` is **silently inert** in that case regardless of
the path or hash it is given (tested against cargo-deny 0.20.2; a paired
`[[licenses.exceptions]]` reports `license-exception-not-encountered`). Both were written,
tested, found ineffective, and **removed rather than left in place looking effective** — dead
config that appears to enforce something is worse than none.

This *vindicates* the 2026-07-18 `git2` rejection's sharpest line — that the gate "clears on a
metadata technicality" — and goes further: it is not fixable within `deny.toml`.

**Consequence — the enforcement moved to where it can actually run.** `ci/third-party.sh`
carries a licence **override** so the shipped notice states libgit2's real licence, and
`ci/checks.sh` **fails if `libgit2-sys` enters `Cargo.lock` without that override**, so the
dependency cannot arrive without the notice that legally must travel with it. Keyed on the
lockfile, so it fires for an optional dependency too. Both were verified by breaking them.
`deny.toml` now carries the finding in full, pointing the next reader at the real gate rather
than implying it does a job it cannot do.

**Reversal condition:** cargo-deny gains the ability to override a declared `license` field →
move the assertion back and delete the `checks.sh` entry. A maintained permissive pure-Rust
git with working push appears → drop libgit2 entirely.

## The owner's five Track M rulings (2026-07-19)

Taken by the owner after the drift review, and binding. They close the questions the review left
open; the receipts for each are in `sessions/2026-07-19-mobile-drift-review.md`.

1. **libgit2 is the git backend — and the *intended* end state is one git dependency on both
   platforms, desktop included.** *Why:* it solves the problem whole, and two shipping apps
   already vindicate it (PuppyGit via JNI; GitSync, which migrated *off* JGit onto Rust `git2`
   and ships Android **and** iOS from one Rust core). *Consequence:* `deny.toml` gains **one named
   exception** for libgit2, citing its GPL-2.0 **linking exception**; the linked-vs-invoked ratio
   itself is **not** rewritten (it still correctly protects a future `.deb` shipping `pdftotext`),
   and `ci/third-party.sh` must emit the notice that exception requires. This **reverses** the
   2026-07-18 `git2` rejection on its licence ground.
   > **Sequencing caveat, recorded so it is not lost.** The desktop half is the *destination*, not
   > the next commit, and two findings gate it. (a) **`git merge-file` is the permanent oracle** —
   > it is the only grader the pure-Rust body engine will ever have, so it must survive **in the
   > test harness** even after production stops shelling out; "one git dep" is satisfied by one
   > dep *in the shipped binary*, not by deleting the oracle. (b) **libgit2 cannot invoke external
   > merge drivers**, so an app-side pull must call `merge_files` itself at each conflicted path —
   > the driver stays installed for collaborators running terminal `git pull`, and both routes
   > call the same engine, which is what makes them unable to diverge. **Build the engine and the
   > differential harness first (steps 1–2); swap the backend after.** Swapping first would
   > silently disable the frontmatter merge on the one path that must never corrupt.
2. **iOS eventually; Android now.** *Why:* it costs nothing today and permanently forecloses the
   ship-a-binary temptation, since iOS forbids `fork`/`exec` outright. *Consequence:* no design
   may assume an executable subprocess on device, on either platform.
3. **Non-pixi dependencies are accepted — but they are *project-local*, never a system
   requirement.** *Why:* the Android NDK/SDK have no conda packaging and never will. But
   "outside pixi" must not become "outside the project": a checkout that asks a contributor to
   `sudo apt install` something has moved the dependency into a place the repo cannot pin,
   cannot version, and cannot uninstall — which is the *reproducibility* the pixi rule exists to
   protect, lost by another route. **Refined by the owner 2026-07-19, after a `sudo apt install
   adb` instruction was correctly rejected as a system-wide requirement.**
   *Consequence:* the first written exception to *"pixi is the only package manager"*, and it is
   paid for with:
   - **Everything under a gitignored `.android/` in the repo.** Every piece Google ships —
     `platform-tools` (adb), the NDK, `cmdline-tools` — is a **standalone zip, not an
     installer**, so all of it unzips into the tree and runs from there. Nothing is installed,
     so nothing needs uninstalling, and two checkouts can hold different versions.
   - **One-command bootstrap** (`pixi run android-init`), reading a committed
     `android/toolchain.lock` that pins each artifact by **our own SHA-256** — the durable,
     project-owned assertion. `sdkmanager` and anything it resolves stay quarantined in an
     opt-in environment **declared non-hermetic**.
   - **`pixi run ci` gains no NDK dependency, ever.** A contributor with no Android toolchain
     must still get a green `pixi run ci`.
   - **No system-level device setup either.** USB `adb` on Linux wants udev rules or `plugdev`
     membership, which is exactly the system requirement this rules out — so the documented
     path is **Android 11+ wireless debugging** (`adb pair` with a code over Wi-Fi). No USB, no
     udev rule, no sudo. `adb reverse` is transport-agnostic, so the loopback tunnel that makes
     `fm-serve`'s Host guard pass by construction works identically over Wi-Fi.
   - `openjdk`, `gradle` and the four `rust-std-*-linux-android` targets are conda-forge-native
     and stay in `pixi.toml` — the non-pixi surface is **NDK + SDK + platform-tools only**.
4. **A divergent frontmatter field keeps its current "loud-and-absent" behaviour; the fix is the
   missing UI surface, not a semantic change.** *Why:* it never loses data, and the obvious
   alternative is forbidden in writing at `merge.rs:32-35`. *Consequence:* a characterization test
   locks the behaviour, and a "needs attention" place lists skipped notes with a raw editor. Any
   future semantic change is **its own ruling with its own adversarial pass**.
5. **The MVP cut line is steps 0–5** of the corrected sequence — everything that needs no git
   backend and no NDK. *Why:* it is demoable in days, fixes real bugs, and stakes nothing on a
   decision whose consequences are not yet observed. *Consequence:* re-cut the line **after step
   0** puts the real UI on real glass.

## The Track M record drifted from the Track M rulings — and the body-merge engine is its own decision (2026-07-19)

**Decision — five rulings, from an adversarial drift review** (receipts:
`sessions/2026-07-19-mobile-drift-review.md`; four rulings were each attacked by three skeptics
and **all four were revised under attack**).

1. **The judgement never drifted; the record did.** Every ruling taken since 2026-07-18 traces to
   a stated principle and stands. But the sequence was never re-derived after its foundation was
   rejected: **9 of 12 sequenced items — 7 of 8 inside the MVP cut line — were still written in
   terms of the rejected `git2` backend.** A cold-read test of seven questions a fresh session
   would ask answered **six wrong**. *Consequence:* refuted **arguments** stay verbatim and marked
   (they teach); refuted **instructions** are deleted (they recruit).
2. **The body-merge engine is a decision distinct from the git backend**, and nobody had named it.
   `merge_files` shells `git merge-file` (`merge.rs:204`, `:233`) and takes driver-shaped path
   inputs, so **no engine exists that a phone can call** — under *any* backend. *Consequence:* a
   differential harness against `git merge-file` is a **standing precondition** on any change to
   `merge_files`, and the desktop keeps shelling out **permanently, as the oracle**. Swapping the
   desktop engine would destroy the only grader we will ever have.
3. **Ship-a-git-binary-in-the-APK is struck permanently.** Not on the licence ratio in `deny.toml`
   — which is correct and stays as written — but on dated facts: a bundled binary produces **no
   row** in `ci/third-party.sh`'s `cargo tree` walk, so we become the licence *and* CVE
   distributor of a TLS stack invisible to both gates. That is structurally worse than the defect
   `git2` was rejected for. It is also Android-only forever: iOS forbids `fork`/`exec` outright.
4. **We will not build a backend for a platform that does not exist — but we have evaluated what
   exists.** The option set *as of 2026-07-19* (never "evaluated and closed"): `gix` (permissive,
   but **push is unimplemented**, so it means hand-writing `send-pack` — which the skeptics
   correctly called more sync machinery than the blob mirror already rejected as *"a sync
   framework by another name"*); `git2`/libgit2 (mature push, and **two shipping apps vindicate
   it** — PuppyGit via JNI, and GitSync, which migrated *off* JGit onto Rust `git2` and ships
   Android **and** iOS from one Rust core — but it enters under a wrong licence declaration and
   needs a conscious `deny.toml` exception naming libgit2's linking exception); Path A
   (transport-only, **demo-only, never the end state**, in both places it is mentioned).
   *Consequence:* the merge-driver objection **no longer discriminates** — neither engine can
   invoke an external driver and neither needs to, because the app calls its own `merge_files`
   during the pull. That is GitSync's shipped pattern and is better than a driver: one engine,
   both platforms.
5. **A phone with no git backend is not a failure state.** `git::available()` already returns
   false gracefully (`git.rs:53-58`); such a device is a compliant degraded notebook that reports
   `git: false` and hides the collaboration surfaces. What is constitutive on mobile is **sync**,
   not git — record that rather than suspending the capability model.

**Also found, and it is not a mobile problem:** the invariant at *"Frontmatter merges
structurally"* below is **asserted, not held** — see the ⛔ note there. It reproduces today, on
desktop, in shipped collaboration code.

**Why this entry exists at all:** the 2026-07-18 review's own finding was that the draft *"applied
the project's rules to others and exempted its own proposals"* — and then did the same thing
itself. So: **before any ruling ships, grep this file for the thing it is about, and quote what
you find.** Cite rulings **by subject, never by number** — `mobile-design.md` and `plan.md` number
them differently, and the hybrid propagated a wrong number into this file.

## Mobile is the app on the phone, not a thin client — one core, git-coordinated (2026-07-18)
**Why:** the owner overrode `MASTERPLAN:57`, which deferred mobile as *"a server + auth
decision"* (the phone as a thin client to the laptop's `fm-serve`). The goal is an app that runs
**on the phone itself** — collaborate with yourself/others across devices over the *same* git-repo
vaults, feature-parity of every view including the whiteboard, editable and merged on both
platforms — under one constraint: **minimal decade-scale maintenance.** That constraint plus
*"not two parallel workflows / seen seamlessly"* both point to **one shared Rust core reused on
both platforms**, not a second implementation (a PWA would re-implement `merge`/`query` in JS and
could diverge on the exact concurrent edits sync must reconcile — silent loss on the one operation
that matters). **Recommended:** Tauri v2 mobile, Android first, embedding `fm-core`/`fm-query`/
`fm-model`/`fm-app`; the phone is a *thin frontend*, nothing in the core forks. Full design +
audit in [`mobile-design.md`](./mobile-design.md); sequence in [`plan.md`](./plan.md) Track M.
**Consequence** — the rulings, each a fix an adversarial review + three code audits forced:

- **One command surface (the load-bearing fix).** The premise *"`fm-app` is already fronted by
  `fm-serve` and `fm-cli`, so nothing forks"* is **false**: `fm-cli` reimplements against
  `fm-core` (no `fm-app` dep), `fm-serve::api()` is a second surface, a Tauri bridge would be a
  third. Extract `api()`'s `match` + the single-lock `Vaults{MultiStore + Vec<VaultConfig>}`
  discipline into `fm_app::dispatch`; both transports go thin over it. This is what makes "one
  command library" *true*, and it precedes every milestone.
- **Two reversals, owned in writing — and reversal (1) is now ⛔ BLOCKED (audited 2026-07-18).**
  (1) **`git2` in `fm-core` would reverse "git is a capability, not a dependency"** — linking
  libgit2 compiles a git implementation into the core *always*, even for a git-less desktop user;
  in exchange in-process git beats "hope `git` is on PATH". **It must not be built as written**,
  for two reasons the ruling never weighed: **libgit2 cannot invoke external merge drivers** (no
  process-spawn exists in it; `git_merge_driver_register` is unbound), so porting `pull()` would
  *silently disable* the `.md` frontmatter merge while a collaborator's terminal `git pull` still
  honours it — two merge semantics in one vault; and **`deny.toml` forbids linking GPL code**,
  which libgit2 is (GPL-2.0-with-linking-exception) — `cargo deny` passes it only because
  `libgit2-sys` under-declares as `MIT OR Apache-2.0`, and `ci/third-party.sh` reads the same
  field, so we would ship binaries omitting a required notice. Needs an owner decision, not a
  silent pass. `gix` stays the documented pure-Rust swap once its push ships.
  (2) **A Keystore-held token reverses "the app stores no secret of its own"** — scoped to
  the fact a phone has no ambient credential-helper; encrypted under an Android Keystore key,
  never in prefs or the remote URL. Untouched (no mobile shell exists yet).
- **Auth is PAT-first (user-owned, un-vendored), OAuth device flow optional.** A pasted
  fine-grained token is host-agnostic (Gitea included) and needs no vendored OAuth-App
  registration on the critical path; OAuth is the convenience. A **per-URL injected
  `CredentialSource`** (not a global `OnceLock<Fn>`) serves multi-repo + refresh + tests;
  **`set_identity` on clone** keeps the `PLACEHOLDER_EMAIL` provenance sentinel honest on mobile.
- **Merge stays in the libgit2 family — ⛔ BLOCKED: the named function does not exist**
  (audited 2026-07-18). `git2` 0.20.4 exposes only `Repository::merge_file_from_index`, which
  needs index entries and would pollute the ODB. The buffer-shaped `git_merge_file` is bound in
  `libgit2-sys` but `git2` imports that crate **privately**, so this needs a direct `libgit2-sys`
  dependency plus unsafe FFI — a different decision, inheriting the licence question above. The
  rejection of a niche crate (`diffy`) on the one path that must never corrupt still stands. If
  ever built: the desktop `.md` driver **stays installed**; driver + both app-pulls call the same
  `merge_files`; a differential test (vs `git merge-file`) gates the swap.
- **Whiteboards merge element-wise, in pure Rust — ✅ SHIPPED 2026-07-18.** `fm-core/src/scene.rs`,
  called from `merge_body` before the text merge. **Why not Excalidraw's own
  `reconcileElements`:** it takes two scenes and *no base*, so it cannot tell "you deleted this"
  from "I added this" and keeps the element either way — every deleted shape returns on the next
  sync. A 3-way merge has the base and honours the deletion. Higher `version` wins, lower
  `versionNonce` breaks ties (deterministic on both machines, which is what stops the next sync
  diverging), fractional `index` keeps the z-order, file maps unite. **There is no `.excalidraw`
  file and no second driver** — `FileStore` writes `<ulid>.md` and the existing `*.md merge=fm`
  attribute already routes board notes into `merge_files`.
- **Sync is a seam, git one provider — but build nothing extra.** A thin `SyncProvider` trait with
  `GitSyncProvider` as the *sole* impl; design against Syncthing's profile *on paper*; make
  `history`/authorship a **queried optional capability** (that is the test the seam isn't
  git-shaped). Backends default free & serverless (rclone → ~70 backends; a free private GitHub/
  GitLab repo is the zero-server option), self-hosted first-class. **Keeps "no CRDT / no sync
  framework" intact** — extends "a bare git remote is the coordinator" to "coordinator is a role."
- **Auto-push is explicit, never silent** (`reject → pull → merge → re-push`), or the naive
  "auto-push after commit" wedges into the documented best-effort-and-silent failure loop against
  `push_squashed`'s deliberate reject-on-moved-remote. **Build the real streaming `GET
  /api/blob/<hash>`** — ✅ SHIPPED 2026-07-18 as `GET /api/blob/<reference>` (`fm-serve/src/blob.rs`) — for both platforms. **Mobile shell stays trivially CSS**
  (the pane-grid "unverifiable by CI" precedent applied, so it earns no e2e).

**Rejected:** a mobile PWA (a second, divergent merge/query implementation — the exact silent-loss
risk); CRDT/per-block ids (voids "the atom is the file", doesn't merge media, for a real-time we
don't need); a global-closure credential source (can't serve N repos, can't rotate on 401,
untestable); killing the desktop merge driver (reintroduces frontmatter corruption on terminal
`git pull`); **Path A (transport-only) as the *end state*** — it retreats toward the
satellite-of-desktop model this decision overrides and fails a phone-only collaborator, so it is a
low-risk *early demo*, not where Track M lands. **Top risk carried:** the Android SDK/NDK are not
conda-packaged, so the toolchain escapes the pixi-only house rule — pin the whole matrix in CI
(≠ `pixi.lock` reproducibility).

## The auto-commit stages what we wrote, not where we wrote it (2026-07-18)

**Why:** `commit_all` ran `git add -A` every five seconds. In a vault that is also a project
repo — the direction Track V is heading — that is a second author: it staged half-written code
and destroyed a curated index. Scoping it to the vault's *directories* fixed the worst of that
and was still wrong, because in a project vault the notes directory may well be `docs/`, so a
note being hand-edited in Vim was committed mid-sentence.

**Consequence:** `FileStore::put`/`delete` record every path they touch; `commit_all` stages
exactly that list and nothing else. Cleared only once a commit lands, so a failed commit does
not forget what it owed.

**The write-list is deliberately not on the `Store` trait.** That seam carries no paths, no
mtimes and no directory handles, and that is precisely what makes a storage swap a backend
change rather than a rewrite. `Vaults.store` is a concrete `MultiStore`, so it reaches the list
without widening the seam — the same reasoning that keeps `find_blob` off the trait.

**The trade, owned:** a note edited outside the app is now never committed *by* the app. That
is the intent — it is your edit, in your repo, and yours to commit — and it makes true what
`known-issues.md` already claimed about `fm-cli`/Vim writes. **Rejected:** committing anything
we did not write, on the reasoning that "it rides along anyway"; riding along is exactly how a
half-finished sentence becomes a commit.

## The lost-update token is a content hash, not a timestamp (2026-07-18)

**Why:** `update_body`'s `base` was the `updated` stamp, which only moves for writers that bump
it. The app does. The `.md` merge driver does. **Vim does not** — and `FileStore::put`'s mtime
guard is disarmed a few seconds later by the poll's own reindex, which writes the new mtime
into the index. So the single writer the guard could not see was the one it most needed to.

**Consequence:** the token is the sha256 of the body, carried on `NoteDetail.version` and
returned by `update_body`. Content cannot lie about whether the body moved.

**Measured before committing to it**, because it sits on the whiteboard save path: **1.6 ms in
release**, ~41 ms in debug, for a 2.8 MB body — against a 600 ms save debounce that then writes
and fsyncs that same body, so it is comparable to the write it precedes rather than a new cost.
Pinned by a perf budget, set in debug terms because that is what `pixi run ci` runs, and sized
to catch an algorithmic regression rather than to police the constant factor.

**Rejected:** having the client hash the body itself via Web Crypto — it is available on
localhost, but it makes every save path async for no gain when the server is already holding
the bytes.

## `fm-cli` shares the command library; it does not route through `dispatch` (2026-07-18)

**Why:** the standing debt was recorded as *"migrate `fm-cli` onto `fm_app::dispatch`"*, and
that turns out to be the wrong shape. `dispatch` is a **wire** surface — JSON in, JSON out —
built so a transport can frame it. A CLI wants typed values to print, so `fm show` would have
to serialize and re-parse its own answer. Only 7 of the CLI's 13 commands even have an arm;
`verify`/`manifest`/`restore`/`check`/`reindex` are CLI-only by design, and `merge-md` runs
*before* a store is opened because git invokes it as the merge driver.

**Consequence:** `fm-cli` depends on `fm-app` and calls the **typed command functions**. The
real debt was duplicated *logic*: `Cmd::Add` rebuilt the asset note — title, blob hash, MIME,
put, thumbnail — in five lines that already existed in `commands::ingest`, and the copies had
drifted exactly as duplicated logic does: the CLI's never set `obj.vault`, so a file added from
the command line was stamped with no audience. Both call `commands::asset_note` now. The CLI
keeps streaming from a path (a large file is never held whole) where `ingest` takes bytes from
an upload — that difference is real and kept; the note is not.

**So "one command library" means one implementation of each command, not one entry point.**
`dispatch` is the one *door* for frontends that speak a wire; `commands` is the one *library*
for everything.

## `git2` is rejected; git stays a subprocess capability (2026-07-18)

> **PARTIALLY SUPERSEDED (`#git`)** by *The libgit2 exception* (2026-07-19, above): the rejection
> **stands for the desktop** (still shells out to `git`), but the phone — which has no `git` binary —
> now links vendored libgit2 behind the off-by-default `native-git` feature. Read this entry for
> *why linking was rejected as the default*; read the exception for *why the phone is the one place
> it is allowed*. The reversal is deliberate and the chain is the point — do not delete either.

**Decision made under the project's own principles**, after the audit found `mobile-design.md`'s
rulings 2/3/6 rest on wrong premises. The earlier "reversal, owned in writing" is itself
reversed: **"git is a capability, not a dependency" stands.**

Every principle in this repo points the same way, which is why this is a decision rather than
a preference:

- **"Bounded, replaceable dependencies — lean on tools that already solve a problem whole and
  shell out."** Linking libgit2 is the exact opposite move. Shelling out to `git` is the
  stance that makes `git` swappable at all.
- **The licence gate.** `deny.toml` says any GPL crate that is *linked* must fail the build,
  and names pdftotext/libvips as the pattern: GPL tools are **invoked**, never linked. libgit2
  is GPL-2.0-with-linking-exception. `cargo deny` would pass it only because `libgit2-sys`
  declares `MIT OR Apache-2.0` while vendoring ~230k lines of GPL C — clearing a gate on a
  metadata technicality is not satisfying it, and `ci/third-party.sh` would then ship binaries
  omitting a notice the exception requires.
- **Do not ruin what works.** libgit2 cannot invoke external merge drivers. Porting `pull()`
  silently disables the `.md` frontmatter merge while a collaborator's terminal `git pull`
  still honours it — Phase 1 undone, quietly, in the one place that must never corrupt.
- **Minimal decade-scale maintenance.** Ruling 3's actual shape is a direct `libgit2-sys`
  dependency plus unsafe FFI around a function `git2` deliberately keeps private
  (`git2::merge_file` does not exist). That is a maintenance liability, not a simplification.

**What this costs:** mobile has no `git` binary, so the phone port cannot use `git.rs` as-is.
That is a real constraint and it is *not* solved here, deliberately — mobile is blocked on the
Android toolchain anyway, and pre-committing to the wrong backend to unblock something that
does not exist is how the wrong backend gets built. When it is live, the options are: ship a
git binary with the app, revisit `gix` once its push ships (the documented pure-Rust escape),
or the transport-only Path A. **Rejected:** doing it now "so mobile is ready".

## Whiteboard images: git-track them, and do not strip yet (2026-07-18)

Two questions, answered separately.

**Which storage — decided: git-track whiteboard-embedded blobs, a scoped exception.**
`mobile-design.md` offered (a) that, or (b) a blob mirror over rclone/S3. (a) wins on the
stated principles: it is one `.gitignore` exception against a whole new subsystem; it stays
fully offline; content-addressing already dedups it; and (b) is a sync framework by another
name, which *"no CRDT library, no sync framework"* rules out. Recorded so nobody has to
re-litigate it.

**When to strip — decided: not yet, and this is the honest reason.** Boards sync *today*,
un-stripped, because the element merge (`fm-core/src/scene.rs`) shipped without needing the
strip — so the plan's "must land before boards are shared" was already false. What remains is
churn, not correctness: a 2 MB screenshot is ~2.7 MB rewritten per stroke. That cost is felt
on a phone (flash wear, battery) and barely on a desktop, so its beneficiary is the platform
that does not exist yet. Against that: the change is in the whiteboard save path, it has three
known traps (`onDestroy` flushes synchronously, so an async upload on save loses the last
stroke — upload eagerly on paste instead; `lastSerialized` compares the *raw* serialization
and must switch to the stripped text; and storing bytes needs either a thin `put_blob` command
or an asset note per screenshot), and **a canvas cannot be verified without eyes on it**.
Shipping a blind change to the one view whose failure mode is "your drawing is gone" fails
*do not ruin what works*. It lands when someone can watch it happen.

## Collaboration is git, *exposed* — not reimplemented (2026-07-18)
**Why:** the machinery (per-vault git, `.md` merge driver, push/pull, signed identity) already
shipped; git knows who changed what and when, but nothing surfaced it. The user's framing: *use
git and expose it*, don't build features on top. **Consequence:** one read-only
`git::activity` = `git log --name-only` over `notes/*.md` (a note file's stem *is* its ULID, so no
mapping) yields, walking newest-first, each note's **last editor** (`--no-merges`, since a merge's
author is the merger). That single command powers **all** collaboration visualisation:
- **`EditedBy` labels** on every card and the open note ("● name · 5m ago"), person-coloured by
  the same `hashHue` as vault badges, delivered to renderers via a runes-in-module store
  (`activity.svelte.ts`) — no prop-drilling.
- An **Activity pane** (git's log as a first-class workspace view), and a **contributor filter**
  (chips like the vault filter; `App.shown` gained an author check, so one click hides a person
  everywhere).
- **Automatic "someone pushed" awareness** — a slow, visibility-gated `remote_moved` network poll
  (not the 15 s heartbeat) feeding a one-click-pull chip, wiring the deferred item with existing
  commands.
**Rejected / deferred:** storing any authorship (git already knows — the app writes nothing);
"created by" and per-commit logs (last-editor map is the MVP); anchored comments (need the
deferred backlinks index); live presence (needs the descoped peer — git knows only *pushed*
state). The placeholder committer (`formicaria@localhost`) displays as "you".

## Every entity shows its vault, as a name-coloured badge, in every view (2026-07-18)
**Why:** with a set of vaults, "who can see this?" is a property you must be able to read off any
note, board or asset at a glance — but the badge existed only on board cards, and was a neutral
outlined chip whose colour the *theme* was meant to assign per vault (keyed off a `data-vault`
attribute). A theme can't colour a vault it has never heard of, so arbitrary vault names got no
colour. **Consequence:** one shared `ui/src/lib/VaultBadge.svelte` (+ pure `vaultColor.ts`:
`vaultHue(name)` — a deterministic hash → hue) is used by **Card (board), Agenda, Timeline,
Search, Calendar, and the open NotePanel**, so the same vault reads the same colour everywhere.
The badge fixes saturation/lightness so white-on-hue stays legible on every hue in both themes;
Calendar bars are too small for a name, so they use the compact `dot` form with the name in the
tooltip. Still shown only when `vault` is set (a single-vault install has no boundary), and the
colour still tells the truth about audience because `vault` is derived from location, never from
the file's content. No vault name lives in a renderer — the colour is derived generically.

## Cross-vault copy is restrictive by default; create picks a vault (2026-07-18)
**Why:** with multiple vaults you couldn't choose where a new note was born, and porting a
note to another audience had no path. Files-as-truth makes the copy "just a copy" — but a
naive one leaks: a copied note keeping its `note:`/`asset:` references would, once its new
vault is pushed, expose ids/hashes/filenames from *another* vault and leave its audience with
dangling pointers. **Consequence:** create-in-vault threads one `vault` string through
`capture` (mirroring `ingest`; routing/refusal already lived in `MultiStore::route`) and a
top-bar destination picker. Copy is governed by one principle — **ideas flow, artifacts do
not**:

- **Restrictive default.** `copy_note` copies *only the prose*: a fresh ULID (a copy is a new
  note — same id in two vaults makes one unreachable via `get`), body rewritten by
  `fm-app/refs.rs::strip_cross_vault` to drop every `note:` link and (unless opted in) every
  `asset:`/`sha256:`/local-image reference — whole Markdown span, label included — replaced by
  a fixed marker, and `assets`/`code` cleared. **Every field a human can type into is stripped
  the same way** — `title`, `status`, `tags`, and both the keys and values of `extra`
  (`refs::strip_value`, recursing into lists). A 2026-07-20 review found the strip was body-only;
  the fix for *that* then missed `status` and `tags`, because they are **typed `Object` fields
  rather than `extra` entries**, so a fix reasoning about the property map never saw them — and
  the test written alongside built its haystack from `title` + `extra` and stayed green. Both are
  free-form user text (`board` groups by any `status` string; a tag is any string). Enumerate the
  fields, do not iterate a map. So a copy can never point outside its new vault.
- **Opt-in carries into the target, never points out.** `with_assets` copies the first-degree
  blobs *into* the target (`BlobStore`, content-addressed dedup; manifest refreshed) so it is
  self-contained; note links stay stripped (the linked-notes tier is deferred, and by the same
  principle each copied child would itself be prose-only).
- **Sensitive → warn + undo.** The "Copy to…" popover states the write is permanent in the
  target's git history; every copy leaves an `uncopy_note` Undo that also reclaims the blobs it
  newly wrote (only those nothing else there still references).
- **Rejected:** a same-ULID "move-as-copy" (ambiguous reads), and vault-scoping references to
  make a copy self-describing (re-couples a note to a location — the same rejection `.view`
  and the blob store already made). `Object.vault` stays derived-from-location, never a field.

## `.view` files are parsed server-side; the wire carries a name, never a query (2026-07-17)
**Why:** the user asked for a customizable multi-pane workspace; three designs + an
adversarial critic found the ask was already `MASTERPLAN.md:323` — *"five generic renderers =
query + a renderer, `.view` config files remain planned"* — and that the tempting route
(put `Query`/`Filter` on the wire so the UI builds filters) has two traps. **Consequence:** a
`.view` is a YAML file in `vault/views/`, parsed **server-side** (`fm-app/views.rs`) into an
`fm_query::Query`; the UI sends only a **name** (`list_views`/`run_view`). Four properties are
load-bearing:

**No `Query` on the wire.** Putting it there would force serde onto `fm-query`/`fm-model`,
where `Object.vault` is *never serialized on purpose* (else the permission is forgeable by a
typo), and would hand a client `PropertyValue`'s variant-order `Ord` trap. A name crossing the
wire has neither risk. (Note the CI grep would **not** have caught serde on those crates — it
greps `rusqlite|sqlx|std::fs`, so this is enforced by *not writing the derive*, deliberately.)

**The filter DSL has no ordered `prop` comparison.** `prop:` supports `eq`/`ne`/`exists` only;
date windows go through `date:` (a real `DateRange` over parsed `Date`s). So the `Ord` trap —
comparing a `Text` against a `Stamp` and getting a confident wrong answer — is *structurally
unreachable* from a `.view`, not merely discouraged.

**A `.view` extends a preset; it never replaces one.** `Filter { all }` is a top-level AND, so
user conjuncts compose onto the renderer's base by `Vec::extend`. The base for
board/agenda/timeline is `Kind(Note)`, written once in Rust — so the assets-exclusion decision
survives: a `.view` cannot widen a board to include assets, only narrow within notes.

**A broken `.view` is named, never dropped.** `list_views` includes an unparseable file with
its error (and the filename stem as name); `run_view` returns the parse error as the response.
The parse-error discipline is the whole reason a saved query is safe to hand a non-programmer.

**Consequence for the UI:** the sidebar lists views under the built-in nav; a selected view
owns the stage, rendering through the *same* renderers as the built-ins (a custom board
supports cross-column status drag but not within-column ordering — kept small). YAML not TOML
(a deliberate deviation from `MASTERPLAN:132`): `serde_yaml_ng` already parses frontmatter, so
zero new deps, and a `.view` reads like the top of a note. **Rejected:** the pane grid (see the
de-modalize entry); `Query` on the wire (both traps above); a query-builder UI / DSL grammar
(the file *is* the language, and it is already YAML); moving presets to the client (re-opens
the "filtering forgotten per view" bug `decisions.md` closed).

## The read view sanitizes untrusted note bodies (2026-07-17)
**Why:** `render.ts` assigns `marked.parse()` straight to `innerHTML`, and a note body is no
longer only the author's own text — collaboration made bodies arrive from other people through
the `.md` merge driver. `fm-serve`'s CSRF guard allows no-Origin requests, so a hostile
`<img onerror>` running in our origin can call any `/api/*`: read every note, delete them, or
set a remote and push a private vault off the machine. This was the top item in
`known-issues.md`, and its "single-user, low-risk" excuse expired the day two people could
share a vault. **Consequence:** DOMPurify runs on `marked`'s output before the DOM sees it
(`sanitize()` in `render.ts`), so scripts and event handlers are stripped. **The load-bearing
subtlety:** the sanitizer must not eat our *own* pipeline. Three URI schemes are ours —
`note:` (a reference chip), `asset:`/`sha256:` (inline blobs) — and they are **not** in
DOMPurify's default allow-list, so a naive call strips them and every asset image and note
chip silently vanishes. The config widens the URI regexp by exactly those three (they are
inert in a browser and fully replaced before display, so they add no sink) and nothing else.
Math (`span[data-math]`) and Mermaid (`code.language-mermaid`) placeholders survive because
the resolve passes run *after* sanitize; tests pin both the stripping and the survival.
Mermaid's SVG sink keeps relying on its own `securityLevel: 'strict'` — layering DOMPurify on
it risks dropping the `foreignObject` it uses for text, a regression headless CI cannot see.
**Rejected:** hand-rolling a sanitizer (the one thing worse than none); sanitizing Mermaid's
output (its strict mode is the designed control).

## The note trail is a peer column, not a modal overlay (2026-07-17)
**Why:** the trail (`openIds` + `NotePanel`) was `position:fixed; z-index:50` with a backdrop
that closed it on an outside click — i.e. reading a note was a **mode**. That contradicts the
founding thesis that a note *is* the task *is* the board card (`plan.md`): if a note is just
another view of the same file, seeing it should not dim and disable the board. The user asked
for a customizable multi-pane workspace; exploring it (three designs + an adversarial critic)
found the real want underneath — *"my agenda visible while I write the note it's about"* — and
that the layout ask was already `MASTERPLAN.md:323`'s own deferred `.view` design, not a new
feature. **Consequence:** de-modalize instead of building a layout engine. `.trail` becomes
the **third column** of the `.app` grid, a peer of `.main`; the `.overlay`/`.backdrop` are
deleted; `NotePanel` loses its `100vh`/`100vw` viewport-locking for `100%`. The grid is three
custom-property columns (`--rail`/`--main`/`--trail`) so rail-collapse and the trail compose
without a combinatorial explosion of `grid-template-columns` rules. `wide` (already persisted)
stops meaning "modal vs less modal" and starts meaning "the note takes the whole content area
vs docks beside the view" — which is what a user always thought it meant. **Rejected, and this
is the load-bearing part — REVERSED 2026-07-18; the pane grid shipped:** a **pane grid**
(pick N splits, any view in any cell). The objections, and what became of each:

- *"Configurability standing in for design"* — the owner asked for it directly, which is the
  one thing that settles this class of argument.
- *"A second, incompatible pane concept beside the trail"* — resolved by **deleting the
  trail**: a note became a pane kind (`kind:'note'`), so there is one pane concept, not two.
  That is what made the reversal safe rather than additive.
- *"Unverifiable by `pixi run ci`"* — **this one stood.** The layout is still not verified by
  CI: `panes.ts` is a pure, tested core (pane list, spans, feed-key dedup) and the geometry is
  not. The precedent still holds — the phone reflow is media-query-only for the same reason.
- *"A saved arrangement is what `.view` files are for"* — wrong, and usefully so. A `.view` is
  *a query plus a renderer*; an arrangement is which panes are on screen. Different things,
  and `.view` shipped separately.
- Also rejected at the time: threading `groupBy` through `onMove`/`onReorder`. That bug is
  reachable exactly when two boards are on screen — i.e. under the pane grid — so it became
  real the moment this reversed, and was fixed with it.

**The stopping line, as it now stands:** a flat pane list plus spans, **not** a recursive
split tree. A pane still cannot contain a pane — the part of the original ruling that
survived, because the split tree is the one shape with no natural stopping point.

## A vault is created, not invented; the vault list gains its first writer (2026-07-17)
**Why:** `load_vaults()` read `vaults.json` and **nothing wrote it** — hand-edited JSON, so
there was no path from "I want a vault" to a configured, opened, listed one. And it could
never return empty: `FM_VAULT` defaulted to the *relative* `"vault"`, so a typo, or the
launcher started from a different cwd, silently `create_dir_all`ed a working empty vault
named after the mistake while your notes appeared to have vanished. Configuration
masquerading as capability — the `restic_ready` shape. **Consequence:** the default is gone
(`FM_VAULT` set explicitly still works; unset means **zero vaults**, a real state that gates
the whole UI on a first-run screen); `MultiStore::open(&[])` is legal and `route` returns
`StoreError::NoVaults`, so reads over zero are honestly empty and **writes are loud**;
`check_path`/`create_vault`/`list_vaults` (`[]` is *the* first-run signal — not
`backup_status`, which shells out per vault including a network `ls-remote`, and making the
screen shown when nothing exists depend on the slowest git command is backwards). Six rules
are load-bearing:

**`vaults::save` merges into the parsed `Value` tree and appends only.** Never a typed serde
round-trip: `#[serde(flatten)] extra` would reformat a human's whole file and turn "I don't
understand this" into "I silently dropped it". It refuses a file it could not parse —
overwriting a hand-edited list is the loss the malformed-JSON warning exists to shout about
— and **writes the whole live list**, because `FM_VAULT` set with no `vaults.json` plus a
second vault created means `load` starts preferring the file, ignores `FM_VAULT`, and
**vault #1 vanishes on the next start**.

**JSON before memory: the config write is the commit point.** Reverse it and a failed write
leaves an in-memory vault that vanishes on restart *while the user captures notes into it*.
A failed open never deletes the directory (it may have pre-existed; this codebase does not
delete user data on a failure path), and partial success is reported as partial.

**Creation does not `git init`.** `commit_all` already calls `ensure_repo` on the first
auto-commit, gated by `ping.git`. Eager init buys an empty `.git` five seconds early,
imposes structure at the moment we promise not to, and — if the path sits inside a repo the
user owns — `ensure_repo` probes only `<path>/.git`, finds none, and `git init`s a **nested
repo shadowing theirs**, writing the placeholder identity into it. `blobs/` and `derived/`
*are* created eagerly: no git needed, and git cannot track an empty directory anyway.

**One mutex over the store and the list, never two.** `api()` already took them in opposite
orders (`commit`/`push` lock-then-resolve; `ingest` the reverse), so two would be AB/BA. The
trap that survives: with one, `lock(state)` + a separate `state.vault()` is a
**self-deadlock** — `std::sync::Mutex` is not reentrant — so those arms resolve through the
guard they already hold. `config()` returns **owned**, which is also what lets `ingest`
borrow `&mut store` at the same time. And the guard is dropped before I/O everywhere except
`commit`/`push`/`pull`: `backup_status` shells out per vault including a network
`ls-remote`, and holding it across that would stall every 15 s `ping`.

**`writable` is probed, never inferred from mode bits.** Create and remove a temp entry.
Configuration is not capability — the `restic_ready` rule applied verbatim. Likewise the
verdict (`ok`) is computed server-side: duplicating the policy in Svelte is how a button
enables and then fails.

**`allVaults` derives from `list_vaults`, not from loaded notes.** It used to come from
fetched cards, so a vault with nothing in it did not exist as far as the sidebar was
concerned — "empty vault" and "no vault" were indistinguishable, which is the exact
confusion the first-run screen exists to end. The vault you just made is the one most likely
to be empty.

**Rejected:** a native folder dialog (a browser cannot pick a server's directory, and
shelling out to zenity means the core spawns a process to do its own first run — so: a typed
path, validated server-side per keystroke); moving the config into `fm-core` (it is env by
definition, and `fm-app/src/vaults.rs:3` already rules that fm-core stays free of environment and
configuration concerns); an `open_lossy` for the startup panic (real, but pre-existing and
uncoupled — see `known-issues.md`).

## formicaria: three pillars, one atom; renamed when the plural became true (2026-07-17)
**Why:** the tool grows into *knowledge management + task scheduling + collaboration*
without becoming three products. **Consequence:** those are three **views of one Markdown
file** — a task is a note with a `due`, a message a note with a target, a shared note a
note in a different repo — so *resist adding a fourth thing*. The full sequenced program
lives in [plan.md](./plan.md) (Track S single-user + Track C collaboration), with
[collaboration-design.md](./collaboration-design.md) as its code audit. **The name changed
only once it was true:** `formicarium`→`formicaria` (the plural = a *set* of vaults) waited
for multi-vault to ship, and landed with it on 2026-07-17 as its own commit — a rename
should read as "only strings moved". No code identifiers moved (`fm-*`/`fm` fit either
name). Nothing carries the old name now: the two literals initially held back (the
placeholder identity, Excalidraw's `source`) were held back *for backward compatibility*,
and there was none to keep — the author is the only user and no vault ran on the
placeholder. **The rule that outlives the rename:** `PLACEHOLDER_EMAIL` is a sentinel
matched **by value**, and a vault's `.git/config` is per-machine, so changing it again
hands every vault still on the old value a "real" identity and reopens the hole Phase 0
closed. Safe once, while every vault was the author's. Not twice.

## The core ships as one file; pixi is the only package manager; deps come from wherever (2026-07-17)
**Why:** the owner's ruling. *pixi is the only package manager, including for generating
the executables. CI emits a binary per OS as an artifact, and those binaries work with the
optional deps installed however the user prefers — pixi being one way.* **Consequence:**
`fm-serve/build.rs` bakes `ui/dist` into the binary (hand-rolled `include_bytes!` table —
~40 lines against a dependency, in a crate that is deliberately std-only networking), so
the binary alone *is* the app. Verified: alone in an empty directory, empty `PATH`, no
`ui/dist`, no pixi — it serves the UI, its JS, the SPA fallback and the API. `FM_UI_DIST`
has **no default**: set explicitly it reads from disk (the dev loop keeps its speed — a
`.svelte` edit needs no Rust rebuild), unset it serves itself. The Linux-only assumptions
went with it (`open_native()` knows macOS's `open` and Windows' `cmd /C start ""`;
`config_dir()`/`home()` know `Application Support`, `%APPDATA%`, `USERPROFILE`) — hand-rolled
over the `dirs` crate, since it is three env lookups. `pixi.toml` covers linux-64 /
osx-64 / osx-arm64 / win-64, and **all of it resolves**, including restic, poppler, libvips
and vs2022 for rusqlite's bundled SQLite — so the default env stays flat and the `media`
feature split was not needed. CI gates on all three OSes (not fail-fast: the point is to
learn *which* is broken) and uploads `fm-serve` + `fm` per platform; a tag attaches a zip
each. **`fm` is never optional beside `fm-serve`** — `ensure_repo` installs the merge driver
by pointing git at the binary beside the running one. **The binary does not care how the
optional tools got onto `PATH`** — pixi, apt, brew — which is what makes one artifact serve
every user. **Rejected:** installing tools system-wide as a prerequisite (that is the
user's choice, not ours); baking the pixi env's path into the launcher (re-couples the very
thing this removed); musl-static (the binary already runs with no pixi env; glibc 2.34 is
met by any 2021+ distro).

## Every external tool is an optional feature that declares itself (2026-07-17)
**Why:** the owner's ruling, and it settles a class of question rather than one case: *the
core should let you take notes and schedule tasks on a local PC with nothing installed. If
you want PDFs rendered nicely, you install that dependency. A missing dep means that
feature doesn't work — the core still does.* **Consequence:** the core is `FileStore` over
Markdown files and **spawns nothing**. Verified against a machine with an empty `PATH` — no
git, no restic, no pdftotext, no vipsthumbnail: capture, `due` scheduling, agenda, board,
search and edit all work. Everything external is a *feature* with a *declared capability*:

| tool | feature it buys | without it |
|---|---|---|
| **git** | history (local undo past this session), backup, collaboration | `git::available()` → the tab stops the 5s auto-commit and says so once; the panel says git isn't installed |
| **restic** | media (blob) backup | `backup::available()` → `restic_ready` is false, the checkbox is off and says why |
| **pdftotext** | a PDF's text is searchable | blob still stored and rendered; extraction returns `None` — no text, no error |
| **vipsthumbnail** | gallery thumbnails | tile falls back to the missing-asset placeholder |
| **xdg-open** | "open in the OS app" | that one action errors |

**The rule that generalises:** a capability must mean *"this will work"*, never *"this is
configured"*. `restic_ready` broke it — it meant "repo set + password set", so on a machine
with no restic the checkbox enabled, you ticked it, and it failed. Configuration is not
capability. **And absence must be stated, not swallowed:** the notebook working while a
feature silently doesn't is how you discover on the day you need it. **Rejected:** treating
any of these as hard requirements (the core demonstrably needs none); bundling them into
the binary (they are subprocesses — see the packaging note in `plan.md`).

## Git is a capability, not a dependency (2026-07-17)
**Why:** the owner pushed back that formicaria runs on vaults on a single PC and should not
be tightly coupled to git — git is for backup and collaboration. Tested: **true, and the
code already agreed.** With no git on the machine at all (empty `PATH`), the server starts
and capture / board / agenda / search / edit all work. Files-as-truth means the notebook is
a directory of Markdown files; `FileStore` never spawns git. **But it was optional in fact
and not in design**: the 5 s auto-commit spawned git, failed, and the UI swallowed it —
forever — while `backup_status` reported `remote: null, identity: null`, which is
indistinguishable from "you haven't set it up yet". A vault quietly unversioned, discovered
on the day you need the history. **Consequence:** `git::available()` (cached — git does not
appear mid-run), reported on the heartbeat (`ping.git`) and in `backup_status.git`. The tab
**stops scheduling** the auto-commit when there is no git and says so **once**; the backup
panel says *"git isn't installed — your notes are safe, they're files on disk, but nothing
on this panel can run"* instead of inviting you to type a remote into a tier that cannot
run. **One nuance the framing missed:** git is not *only* backup and collaboration — it is
also **local history**, the undo that outlives the session, on a single PC with no remote.
So: optional, and worth having. Which is exactly why its absence must be stated rather than
swallowed. **Rejected:** making git a hard requirement (the notebook demonstrably doesn't
need it); reimplementing versioning (that is what shelling out to git buys us).

## Vaults are audiences: git is per-vault, blobs are searched, hiding is only a view (2026-07-17)
**Why:** multi-vault forced three questions the plan had collapsed into "wiring", and each
has a wrong answer that loses data quietly. **Consequences, in order of how badly the
wrong answer bites:**

**Git is per-vault, so the backup panel is a list, not a form.** One vault = one repo =
one remote = one collaborator list, so `commit`/`push`/`pull`/`set_git_remote`/
`set_identity` each take a vault, and `backup_status` returns one entry per vault: N
remotes, N identities, N unpushed counts, N "someone pushed". There is no honest way to
collapse those — a single "unpushed" across a set of vaults is a number about nothing. The
tempting shortcut, letting `push` mean "the first vault", is exactly the overstatement
`destination.ts` was written to prevent: a backup that silently skips the lab vault. The
verdict names the vaults that did **not** make it, because "your notes are backed up"
while one sat still is the one sentence this panel must never say. An **unknown** vault
name is an error, never a fallback: writing a lab note into personal is a disclosure git
history makes permanent, and the reverse loses it. *Identity is per-vault too* — a vault is
an audience, so the name on a lab repo need not be the one on your personal notes.

**Blob resolution searches every vault.** A `sha256:` reference deliberately does not say
which vault holds the bytes, and it must not: that is what keeps a cross-vault `note:`/
`asset:` link free and lets ULIDs stay the only identifier anyone needs. Searching is not
a shortcut, it is *correct* — content-addressing means whichever vault answers, the bytes
hash to the reference, so they are the same bytes. **Rejected:** vault-scoping references,
which would re-couple a note to a location and break the links Phase 2 gets for nothing.
*Ingest is the exception and takes an explicit vault*: a blob should land beside the notes
that will reference it, never in an audience that shouldn't have it.

**restic is per vault too — a restic repo *is* per repository.** *(Corrected 2026-07-17:
the first cut made it one repo for the whole set, on the theory that a snapshot is
disaster recovery rather than sharing. That was wrong on its own terms — restic has no
notion of "part of a repo", so backing up a set of vaults means a repo each, and one repo
holding several vaults is not a design choice, it is a merge of things that were kept
apart on purpose.)* So each vault carries an optional `restic` in the vault list, and a
vault without one simply has nowhere to put its media — which is **not** an error: you may
well want the lab's notes shared over git and its media backed up by the lab, not by you.
"Include media" backs up the vaults that have a repo and **names the ones that don't**,
per vault, in the report and in the verdict. Silently skipping them would be the exact
overstatement `destination.ts` exists to prevent. `RESTIC_PASSWORD` is still one password
for every repo: a per-vault password has to live somewhere, and the one place it must
never live is the config file sitting next to the paths. The single-vault install's
`FM_RESTIC_REPO` still works and becomes that vault's repo.

**An asset joins the audience of the note it was dropped on.** `ingest` takes the vault's
**path and its name**, because they answer different things — the blob store takes a path,
the `Store` routes by name — and they must agree. The first cut passed only the path:
bytes went to the lab vault while the asset note went to the default, so the people who
could see the file could not see the note describing it, and the note pointed at bytes its
own vault never had. Caught by running it, not by a test.

**Hiding a vault is a view preference, not a permission.** The filter lives in
localStorage next to the column order, and filters client-side. It changes what is on
screen and nothing else; who can see a note is decided by which repo holds the file, and
nothing in a browser can change that. The chips are styled quiet so they never read like
an access control.

## Notes merge through a driver that shells out for the body — and is never installed unless it can run (2026-07-17)
**Why:** `updated:` is rewritten on every save, so *any* two concurrent edits to one note
collide on that line even when the two people touched different paragraphs — and git's
markers land inside the YAML fence, where `from_file` rightly refuses them and the note
drops out of the vault. Every concurrent edit, by construction, for a reason that is
entirely our own doing. **Consequence:** `fm-core/src/merge.rs` + `fm merge-md`, wired up
by `.gitattributes` (`*.md merge=fm`) and `merge.fm.driver`. It resolves structurally what
is mechanically resolvable — `updated` = the later reading (a clock, not an opinion),
`id`/`created` = base, `tags`/`assets`/`code` = union, any field only one side touched
takes that side — and **hands the body to `git merge-file`**: a 3-way text merge is a
solved problem and shelling out is the house rule (no diff3 to get wrong, no new dep).
*The property that pays for the whole thing:* frontmatter is always emitted whole and
valid, so **a conflict lands in the body** — the note still parses, still indexes, and
still opens in the editor with the markers in the textarea. That is what makes conflict
surfacing possible at all, and it is why "resolve markers in the textarea" is a feature
rather than a wish. A genuinely divergent *field* (both sides set `status` differently)
falls back to a whole-file merge rather than picking a winner: resolving by fiat is the
silent loss this phase exists to stop.

> **⛔ The italicised invariant above is asserted, not held (found 2026-07-19).** The two
> sentences contradict each other, and the second one is what the code does. `merge_objects`
> returns `None` on a divergent `status` (`merge.rs:104-122`), `merge_files` then calls
> `whole_file`, and that line-merges the **entire file including the YAML fence**
> (`merge.rs:232`). Markers land *inside* the frontmatter, `frontmatter::from_file` rightly
> rejects it, and the note **disappears from every view** — `file.rs:417-420` already names this
> exact cause. Trigger: two people drag one card to different columns. `crates/fm-cli/tests/
> merge.rs` has five tests and **none covers it**.
>
> It fails *loud-and-absent* — the note is collected with a reason (`file.rs:426`, `:471`) and
> surfaced by name (`App.svelte`, "N note(s) could not be read"). That is the right failure, so
> this is a **missing test and a missing UI surface, not a redesign**. Do **not** "fix" it by
> letting ours win the field: `merge.rs:32-35` forbids exactly that in writing, and it would be
> worse — bodies are identical in the card-drag case, so the merge returns **Clean**, auto-commit
> fires at 5s and the sync loop pushes it. Loud-and-absent would become quiet-and-wrong.
>
> Any change to this behaviour is **its own ruling with its own adversarial pass**. Acceptance:
> both values survive in the file, the file parses, and the result is `Conflicted` — never Clean,
> or each machine keeps its own value by fiat and re-derives the conflict forever.

**Two traps, both load-bearing.** (1) The
`merge.fm.driver` definition lives in `.git/config` and deliberately does **not** travel —
git will not let a repo ship a command that runs on your machine — so `ensure_repo`
installs it on every open, exactly as it writes `.gitignore` and the identity; only
`.gitattributes` travels. (2) **Never install a driver we cannot point at.** Found the
hard way: pointing at a bare `fm` and hoping PATH would answer meant git ran a
nonexistent command, took the non-zero exit as "conflict", and handed back `%A`
*untouched* — i.e. ours, with **no markers** — so the user resolves a normal-looking file
and silently deletes their collaborator's edit. `merge_command()` returns `None` unless an
`fm` binary really sits beside the running one, and no driver at all degrades safely to
git's built-in text merge. **The corollary bit later:** `pixi run build` shipped only
`fm-serve`, so in a release install there *was* no `fm` beside it and the driver silently
never installed — the centrepiece of Phase 1, absent, with every test green (the tests
build `fm-cli` themselves). Found by a clean release build, not by CI. The build task now
builds both and says why. **Rejected:** a bare `fm` on PATH (PATH at `git pull` time is
not PATH now, and being wrong is data loss); resolving field conflicts by `updated`
last-writer-wins (fiat, i.e. the CRDT mistake decision 1 rules out).

## A vault gains an identity when it gains an audience, not before (2026-07-17)
**Why:** `ensure_identity` wrote a placeholder committer (`formicaria@localhost`) whenever
`user.email` was unset — the default state of a researcher who never configured git. In a
shared vault that attributes *everyone's* commits to the same fake name, gutting the
provenance that "awareness over enforcement" (no locks; "Ravi pushed 2 min ago" is a `git
log` query) is built on. But simply deleting the fallback is worse, not better: git cannot
invent an identity on a host without a FQDN, so a fresh vault would fail to commit at all
— and the auto-commit swallows its errors, so the notes would silently stop being
versioned. **Consequence:** the placeholder stays, scoped to what it is honest for — a
vault **nobody else can see**. `git::identity()` reports that exact literal as `None`
("nobody real signs this"), and `git::set_remote` refuses while it stands, because a
remote is precisely the moment a name starts travelling into someone else's clone and git
history is forever. So the question is asked **once**, in the backup panel, at the only
moment the answer matters — and anyone whose git is already configured never sees it. Two
consequences worth keeping: the sentinel is **load-bearing**, so renaming
`PLACEHOLDER_EMAIL` would turn every vault running on it into a "real" identity and
silently reopen the hole; and detection is by-value, so a vault that has been on the
placeholder for months **heals itself** the moment the user answers. Identity is written
**repo-locally** — a vault is an audience, so the name on a lab repo need not be the one
on your personal notes, and this app has no business editing anyone's global git config.
Enforced at `set_remote` only: `commit_all` cannot refuse (a silent stop is worse than a
fake name on a private commit), so an already-remote'd vault is nudged by the panel, which
shows the question whenever `backup_status.identity` is null.

## Inline meeting actions become their own note, never a per-block atom (2026-07-17)
**Why:** an owner types `- [ ] Ravi to send the draft` mid-meeting and wants it to show up
in the agenda — but making inline checkboxes first-class agenda items needs **per-block
identity**, which *the atom is the file* forbids (`MASTERPLAN.md:456`: it "voids this
plan"). **Consequence:** a checkbox stays **plain Markdown in the body** (an interactive
toggle that rewrites the body bytes, no id), and does **not** auto-appear in any planning
view. A deliberate gesture — a `/promote` slash entry or a per-line button — **extracts the
line into its own note file** (a task note with `status`/`due`, back-linked via
`[Title](note:<ulid>)`), which then flows into board/agenda as a normal atom. Friction stays
at "type `- [ ]`"; scheduling is an explicit promotion. **Rejected:** a body-scan
"checkboxes → agenda" pass — it manufactures second-class items that don't round-trip and
re-pollutes the exact views the assets decision below cleaned up. **Rejected:** per-block
ids/timestamps — voids the plan.

## Board images strip to the content-addressed blob store on save (2026-07-17)
**Why:** Excalidraw's `serializeAsJSON(…, 'local')` embeds a pasted image as a **base64
data URL inside the note body** (`BinaryFileData.dataURL`), and `Whiteboard.svelte`
re-serializes the whole scene on every debounced `onChange` — so a 2 MB screenshot becomes
~2.7 MB of **git churn per pointer move**, routing bulk binary through the note body and
breaking the "blobs are already out of git" premise (the two-tier-backup decision). This is
latent today and unbounded the moment boards are shared. **Consequence:** on board save,
**strip the scene's inline `files` into the blob store** (reuse `BlobStore::put_bytes` +
ingest's MIME sniff, dedup by sha256), leaving only blob references in the `.excalidraw`
JSON; **rehydrate on load** via `resolve_asset` / the planned `GET /api/blob/<hash>`. Reuses
the existing blob seam rather than adding one. **⛔ BLOCKED (2026-07-18), and the ordering
claim was wrong:** the board merge already shipped without this (`fm-core/src/scene.rs`, under
the existing `*.md merge=fm` driver — there is no separate `.excalidraw` driver), so boards sync
today *un-stripped*. The strip cannot ship until one question is answered: **`blobs/` is
gitignored**, so once images live there instead of in the scene body, a shared board shows no
images on a collaborator's clone. Pick (a) git-track whiteboard-embedded blobs as a scoped
exception, or (b) a blob mirror — `mobile-design.md` names both and decides neither. Two further
traps found while auditing: `onDestroy` flushes *synchronously*, so an async upload on save loses
the last stroke (upload eagerly on paste instead), and `lastSerialized` compares the **raw**
serialization, so it must switch to the stripped text or the no-op-save guard breaks.

## Assets are query-layer-excluded from the planning views (2026-07-16)
**Why:** an asset is a blob a note *references*, not a thing you plan; a PDF
getting its own board card and timeline entry was noise. Filtering in each
renderer was rejected — it must be repeated per view and silently forgotten by
the next one. **Consequence:** `board`/`agenda`/`recent` carry
`Predicate::Kind(vec![Kind::Note])`; `FileStore` delegates structured predicates
to `fm_query::run`, so this cost no storage-layer change. **`search` and `gallery`
still see assets on purpose** — search (which indexes extracted PDF text) is the
only way to *find* an asset, and it is what backs the `/` menu's asset insertion,
so filtering it too would make ingested files unreachable. Two knock-ons: a board
grouped by `type` can now only answer `note` (the old
"falsifies-the-thesis" test was rewritten to pin the new rule; the generic
group-by claim still lives in `board_by_a_custom_property_…`), and the timeline's
type pill became dead — the status chip took its slot.

## Status rotates through the vault's own values; card order is a view preference (2026-07-16)
**Why:** setting a status meant entering edit mode and *typing* it. The obvious
fix — a `<select>` of `todo/doing/done` — would hardcode one workflow's enum into
the UI, which "generic, literal-free renderers" exists to prevent.
**Consequence:** `status.ts`'s `nextStatus(current, known)` cycles the statuses
`App.learnStatuses` already collects from fetched data, plus unset (so rotating
can always clear, with no separate control) — the chip renders any workflow and
the CI literal grep stays green. Typing a *new* value stays the Details editor's
job; it joins the cycle once a note carries it.
Separately, a card's **position within a column** is remembered in
`localStorage['fm-card-order']`, not in the note. **Rejected:** an `order`
property in frontmatter — it would rewrite a note file on every drag, and where a
card sits on your board is a view preference, not knowledge. Same reasoning, and
the same per-browser no-sync caveat, as the existing column order.

## `start`/`due` are a `Stamp` (day + OPTIONAL time), not a Date/DateTime pair (2026-07-15)
**Why:** the owner needs meeting times ("a time option beyond the date"), but an
all-day deadline must stay expressible and must not churn on disk. Two obvious
designs were rejected:
1. **Reuse `PropertyValue::Date` + `DateTime`.** `PropertyValue` derives `Ord`
   from **variant order first**, so every all-day item would sort before every
   timed item regardless of the actual day — silently wrecking agenda order,
   `Op::Lt/Gt`, and group bucketing. This is a trap, not a preference.
2. **Make `due` an `OffsetDateTime`.** RFC 3339 demands an offset; a wall-clock
   intention doesn't have one. It would also force a time on every deadline.

**Consequence:** `fm_model::Stamp { date, time: Option<Time> }` — naive (no
offset: 14:30 means 14:30 where you are, and the file is the truth), minute
granular, one `PropertyValue::Stamp` variant. `Display`/`FromStr` are **inverses**,
which is load-bearing twice over: it keeps `to_file` byte-idempotent (a bare date
in, a bare date out — no vault-wide churn), and it keeps the board's drag
write-back lossless (`value_string` → `display()` → `apply_property`; the old
`DateTime` display dropped the time, so a drag would have erased it). `Stamp` owns
the format on both sides, so the date literal is no longer duplicated across
crates. **Urgency stays day-granular** — a time is presentation, not priority.
`created`/`updated` are unchanged: they are *instants*, so they stay
`OffsetDateTime`/RFC 3339, and the UI's `parseStamp` is anchored so it can never
match one and hand back a UTC day.

## Whiteboard = embedded Excalidraw, lazy-loaded (2026-07-15)
**Why:** the owner wanted a real "drawio but simpler" freeform canvas, not a
diagrams-as-code stand-in, and chose full-featured-fast over build-it-minimal.
**Consequence:** a deliberate reversal of "no new UI runtime dependency" —
`@excalidraw/excalidraw` + `react`/`react-dom` are now deps. Mitigations keep the
base app lean: the editor is **lazily imported** in `Whiteboard.svelte` (React
root mounted via `createElement`, no JSX → no build plugin), so it's a separate
~744 KB-gz chunk that downloads **only when a board opens** — base `index.js`
stays ~34 KB gz (same discipline as KaTeX/Mermaid). **A board is just a note**
with a `view: board` property whose **body is the Excalidraw scene JSON** — no new
`Kind`, no backend change, files-as-truth intact (the `.excalidraw` JSON is plain
text on disk). **Caveats:** Excalidraw fetches fonts from a CDN unless
`EXCALIDRAW_ASSET_PATH` is set — offline it degrades to fallback fonts (local-font
bundling deferred); and the canvas only renders in a real browser, so it's
**unverified in headless CI** (build + code-split + round-trip are verified).

## Browser is the product; the native window is removed (2026-07-15)
**Why:** the Tauri/WebKitGTK window never painted reliably on the developer's
box (blank/gray; mutter/X11 with no compositor). The same SPA rendered correctly
in a real browser, so the UI logic was sound — the webview was the problem.
**Consequence:** `fm-serve` (std-only HTTP) serves the built UI to the default
browser; `fm-app` became a **library only** (no `[[bin]]`, no tauri deps); the
pixi `gui` env, the `e2e/` WebDriver tree, and the deny.toml Tauri-RUSTSEC
waivers are gone. Modern CSS is now fine (real browser, not WebKitGTK). The old
"WebKitGTK blank-window" risk is retired.

## Files-as-truth; the atom is the file (foundational)
**Why:** durability and ownership — a note must survive as plain text without
this app. **Consequence:** one Markdown note = one file; frontmatter is a
generic YAML mapping so unknown/custom props survive read→edit→write with no
silent loss; key order is fixed so `to_file` is byte-idempotent. No per-block
ids/timestamps — block-level structure is explicitly out of scope.

## `fm-query` may never touch fs/db (the insurance policy)
**Why:** a pure query engine is what keeps the whole system testable, portable,
and honest about the seam between "data" and "storage." **Consequence:**
enforced by a compile-time boundary *and* a CI grep. Do not add `rusqlite`/
`std::fs`/path handling to `fm-query`, ever. Search works by: `Text` predicate →
FTS5 prefix `MATCH` loads only the hit ids → the pure engine applies the rest.

## Generic, literal-free renderers
**Why:** a theme/renderer must not encode a specific workflow's enum values, so
arbitrary property values (any status, any type) render + tint without code
changes. **Consequence:** no `todo/doing/done` in `ui/src/renderers/`; column
tints keyed by `[data-value]`, urgency by `[data-urgency]`, labels sourced from
helpers (`urgency.ts`). CI greps enforce it.

## v1 editor = plain `<textarea>` + rendered read view
**Why:** a live-preview CodeMirror 6 editor was assessed as the single biggest
build risk. **Consequence:** editing is a textarea over the literal bytes
(`update_body`, byte round-trip tested) with a separate rendered read view
(`render.ts`, `marked`→HTML, lazy KaTeX/Mermaid). CM6 live-preview is **deferred
to v2**.

## Content-addressed blobs, extracted text in the note body
**Why:** dedup + integrity (bit-rot = a blob no longer hashing to its filename),
and searchability before the git-ignored blob syncs. **Consequence:** blobs are
sha256 with `ab/cd` fan-out; ingest sniffs MIME (`infer`), runs `pdftotext` →
the **asset note's body** (git-tracked, FTS-indexed), and makes a thumbnail.
`put_bytes` dedups before writing.

## Markdown→HTML is JS `marked`, not Rust pulldown-cmark
**Why:** it shipped that way; the MASTERPLAN's "pulldown-cmark→HTML" line never
materialized (the crate is absent). **Consequence:** the only HTML assembly is
one `marked.parse()` in `render.ts`. *(That output is now sanitized with DOMPurify before the DOM sees it — see the sanitize entry.)* The then-unsanitized-innerHTML gap in
[known-issues.md](./known-issues.md).

## No plugin API
**Why:** plugin APIs rot and become a compatibility burden. **Consequence:**
extend via modular Rust (add a renderer / store / extractor), declarative
declarative `.view` files, a one-file theme (design tokens), and an optional mlua
hatch — never a stable plugin surface.

## Tauri was the light choice; a native-GUI rewrite is rejected
**Why:** even before removal, Tauri used the system webview (no bundled
Chromium; ~9.6 MB release binary) — lighter than Electron (Logseq/Obsidian run
1–2 GB). **Consequence:** the only genuinely-lighter path (egui/Slint/iced) would
mean rewriting the entire Svelte + KaTeX + Mermaid read view — not worth it.
Now moot (browser), but do not propose a framework rewrite.

## Which attachments travel is a per-vault size limit, in `vault.json`
**Why:** the two-tier split below is right by default and too absolute in practice — a screenshot
in a note is part of the note, and a 200 KB PNG has nothing in common with a 60 MB video except
living in `blobs/`. So `git_assets_max` opts a vault in: blobs at or under it are committed and
pushed with the notes; everything else stays local and restic's.

**In the vault's own file, never in per-device Settings.** This decides what enters *shared,
permanent history*. A per-device value would let the loosest machine decide for every
collaborator, and git cannot take it back: a large file committed once is in every clone forever,
and removing it means rewriting history others have pulled. In `vault.json` the rule travels with
the vault, so everyone pushing to a repo obeys one limit. **Default is absent = notes only**, so
pointing formicaria at someone's existing repo never starts writing binaries into it.

**Consequence:** `git add -f`, per file — `blobs/` stays in `.gitignore` (git cannot filter by
size, and un-ignoring the directory would let everything through). The walk is skipped entirely
when the vault has no opinion, because `commit_all` runs on a 5-second debounce. Lowering the
limit does not untrack what already travelled; the bytes are already in history, so it governs
what travels next. `Descriptor::set_git_assets_max` is the one narrow exception to `write_new`'s
never-overwrite rule, and preserves unknown keys.

## Backup is two tiers: git push (default) + restic (opt-in)
**Why:** the vault holds two data classes with nothing in common. Notes are
small, plain, mergeable → git carries them anywhere, authenticated by the user's
own ssh-agent/credential-helper, so **the app stores no secret**. Blobs are heavy
and git-ignored → only restic sees them. The button used to run restic
unconditionally, which was *broken-by-default*: it needs `FM_RESTIC_REPO` +
`RESTIC_PASSWORD` and `packaging/formicaria.sh` never sets them, so every
desktop-icon launch errored. This is the settled split elsewhere (Zotero syncs
metadata and file attachments as separate tiers; git-annex/git-LFS put a pointer
in git and content in special remotes; photo managers back up a small catalog and
hand bulk originals to a file tool). **Consequence:** `BackupPanel.svelte` owns
the vault's remote (stored in the vault's own `.git/config` — no new config
file, and decoupled from the app's source remote by construction). **A push
carries notes only**: media is off-sited only when the box is ticked, stated in
the panel rather than tracked. `manifest.json` is git-tracked, so a git-only
restore still knows its blob inventory and `fm verify` names what is missing —
and because it is tracked *and* staged on every commit, it needs its own merge
driver (`manifest.json merge=fm-manifest` → `fm merge-manifest`). Text-
merged it produced `<<<<<<<` inside a JSON file no user wrote or can resolve,
which then froze commits vault-wide.

**The merge is purely additive — `base ∪ ours ∪ theirs` — and the absence of a
deletion rule is the whole decision.** The obvious 3-way rule ("in base, gone
from one side ⇒ deleted") is wrong here: `blobs/` is gitignored and every writer
is a rebuild from local disk, so this is a git-tracked inventory of a
**per-machine** store and each clone legitimately holds a different subset.
"Absent on their side" means *they never received those bytes*. Applying the rule
made the manifest converge on the **intersection** of what each machine happened
to hold, erasing the record of blobs that exist — destroying the one thing the
file is for. The cost of being additive is an entry outliving its blob, which
`verify` reports and `fm manifest` clears: visible and recoverable, versus silent
and permanent.

**The driver must never exit non-zero**, which is a correctness rule, not tidiness:
git reads any non-zero exit as "I left you a conflict in %A" while %A is untouched
and marker-free — the trap `install_merge_driver` documents — and here a conflicted
`manifest.json` freezes commits for the whole vault. So every side is read
leniently (an unparseable manifest is *no information*, not a failure) and a failed
write keeps ours. Android has no driver at all (libgit2 cannot spawn one), so
`git_native::pull` routes the path by name to the same `Manifest::merge`.
`destination.ts` classifies both a git URL and a restic repo as local/remote —
a local path is a legitimate destination but must never be reported as "off this
machine".

## Squash-on-push — a deliberate reversal of "don't build commit management"
> **Reversal (`#git`), current.** Reverses `MASTERPLAN.md`'s *don't build commit management* — for
> the **remote** only; the local repo still keeps every `auto:` commit for undo.

**Why:** `MASTERPLAN.md:411,447` accept thousands of `auto:` commits as the price
of undo and say *don't build commit management*. That holds for the **local**
repo, but the remote is a different audience: auto-commit fires every few seconds
of editing, so pushing raw would make the GitHub history unreadable.
**Consequence:** `git::push_squashed` collapses the unpushed window into one
`backup:` commit (`reset --soft <base>` + commit). Three constraints are
load-bearing:
- **Only *our* commits are the window** (added 2026-07-18, Track V). The
  justification above — auto-commit fires every few seconds — justifies collapsing
  what *we* wrote and nothing else. `newest_foreign` walks `tracking..HEAD` and
  stops the squash at the first commit whose subject is not `auto:`/`backup:`, so
  three hand-written manuscript commits in a vault that is also a project repo stay
  three commits. Discriminated by **message prefix, never author**: we commit as the
  user's own identity, so an author test classifies everything as ours.
- **Never squash the first push.** With no tracking ref, "unpushed" means the
  *entire* history — destroying history that has never left the machine is
  exactly backwards. First push sends it whole; every later push is one commit.
- **Never fetch in this flow.** The tracking ref is stale on purpose: a remote
  another machine moved then stays ahead of it, so our push is *rejected* and the
  user is told (verified). Fetch first and the `reset --soft` would rebase onto
  their tip and silently overwrite their content with our tree.

**Accepted cost:** granular undo only reaches back to the last push; before that
each push is one step. This narrows (does not remove) the undo auto-commit buys.

## Acquiring a vault: `naturalise` is the seam, not a transport trait
*(2026-07-19, `sessions/2026-07-19-acquiring-a-vault.md`)*

**Why:** the owner wants vaults acquirable from elsewhere by several methods —
git and restic now, p2p later — and explicitly *"these vaults can be git init or
not, that's not a core necessary requirement of formicaria."* The tempting seam
("a vault is a directory, so any directory copy moves one") is **false**: vault
state has three tiers, and one of them must *not* travel (`index.sqlite`, the
`.git/config` merge driver, the committer identity) while a third
(`vaults.json`) lives outside the vault entirely.

**Consequence:** `fm_core::acquire::naturalise` owns tier 2 in one place, and
every acquisition path routes through it. A transport moves bytes and nothing
else, so **a new transport cannot corrupt a vault** — it touches neither the
merge path nor the safety. There is deliberately **no `Transport` trait and no
registry**: the extension point is the filesystem, which is also why a folder
synced by other means is already adoptable with no code.

**The rule that bounds it: `sync` requires git, `copy` works with anything.**
There is exactly one merge engine (`merge::merge_texts`) and it is git-shaped, so
only git can be a two-way relationship; every other transport hands you a copy.
A non-git vault can therefore be *shared* but not *collaborated on*, and the UI
says that in those words rather than implying otherwise. This is what keeps p2p
from needing a CRDT layer (`plan.md:67`): p2p moves a copy for any vault, and can
carry packs for git ones.

**Accepted cost:** restic acquisition returns notes and media with **no history**,
because `backup` snapshots the vault's own directories and not its root. It is a
*recovery*, not a *join*, and stating that before the button was preferred to
widening what restic snapshots.


## Android TLS: the trust store is loaded from memory, never from a file
*(2026-07-19, `sessions/2026-07-19-git-on-the-phone.md`)*

**Why:** `openssl-src` passes `no-stdio` to OpenSSL's configure on **every** Android target, so
the vendored build has no `BIO_s_file`. `X509_load_cert_file` therefore fails with
`X509_R_BIO_LIB` on a file that is present, correct and readable by the same process — measured
on a device. `SSL_CERT_FILE`, `SSL_CERT_DIR` and `GIT_OPT_SET_SSL_CERT_LOCATIONS` all end in a
file BIO, so **none of them can ever work there**. Five separate diagnoses were spent producing
better files before this was found.

**Consequence:** `fm_core::git_native::add_certs_from_pem` parses the bundle through a *memory*
BIO and hands each `X509 *` to libgit2's `GIT_OPT_ADD_SSL_X509_CERT` (option 45, which
`libgit2-sys` does not bind, hence the two extra `-sys` dependencies — both resolving to the
libraries `git2` already links). `fm_app::ca_bundle` supplies the bytes from the device's own
Conscrypt store. Two ordering rules are load-bearing and non-obvious:

- It must run **before the first `git2` call in the process**, because `libgit2-sys` never
  defines `GIT_OPENSSL_DYNAMIC` and OpenSSL is initialised **eagerly** inside
  `git_libgit2_init()`.
- It must **initialise libgit2 itself** first. Calling `git_libgit2_opts` raw skips the `init()`
  that every `git2::opts::*` wrapper performs, and `git_openssl__add_x509_cert` then dereferences
  a NULL `git__ssl_ctx` — a launch crash, not an error code.

**Rejected:** shipping Mozilla's `cacert.pem` as an asset (what PuppyGit does). It works, but it
is a trust store that goes stale the day it ships and can only be refreshed by a release. Reading
the platform's means the app follows the device's own trust decisions and OS updates.

**Also rejected, emphatically:** `certificate_check` returning `CertificateOk`. That is what
GitSync does, and it skips hostname verification while sending `userpass_plaintext` over an
unauthenticated connection. The leaf certificate libgit2 hands the callback has no chain, so real
verification is not possible there either.

## `fm-serve` sends a Content-Security-Policy, and it is the "nothing phones home" guard
**Why:** a note is Markdown, and Markdown renders remote images. A single
`![](https://attacker/p.png?leak=…)` in a note that arrived by merge, by copy, or in a shared
vault fetches the attacker's URL **on render, from the user's machine**. That is not an XSS
chain: it needs no script, it survives DOMPurify (whose allow-list permits `https:`), and it
survives a human reading the diff. Until 2026-07-20 there was no CSP anywhere in the server —
the app self-hosted Excalidraw's fonts precisely to avoid this class, and then left the door
open for note content. **Consequence:** `write_response` sends one policy on every response
(`main.rs::CSP`), plus `nosniff` and `Referrer-Policy: no-referrer`; blobs get
`default-src 'none'; sandbox` behind their existing `Content-Disposition: attachment`.
`frame-ancestors 'none'` is separate from `default-src` and load-bearing: the server sits on a
fixed localhost port with no auth, so any page the user visits could otherwise frame it.
**Accepted losses, both real:** `'unsafe-eval'` is refused, so Excalidraw's harfbuzz font
*subsetter* throws and an exported SVG embeds a font URL rather than the bytes (in-app boards are
fine); and no CSP directive stops a top-level navigation the user clicks, which
`Referrer-Policy: no-referrer` only blunts. **The Android shell now sets its own CSP in
`tauri.conf.json` — it was `null`, i.e. the phone was the only wholly unguarded surface — but
that policy was verified only as far as *launching clean* on a real phone — installed, started,
zero CSP violations in logcat, and the embedded frontend confirmed free of inline `<script>`. It
admits `http://fmblob.localhost` (wry rewrites the custom scheme on Android) and Tauri's `ipc:`
origin, but neither has been exercised: opening a note with an image and a whiteboard is what
would prove them, and getting either wrong shows up as broken images rather than a crash.**

The two clauses that shape the app's own code: `img-src 'self' data: blob:` is the beacon fix
(`data:`/`blob:` are local bytes, so they carry no request off the machine), and **`script-src
'self'` with no `'unsafe-inline'`** — which is why Excalidraw's `EXCALIDRAW_ASSET_PATH` line
moved out of `index.html` into `ui/public/excalidraw-asset-path.js`. Keeping inline scripts out
is what stops a sanitiser bypass from being code execution. `style-src 'unsafe-inline'` is
unavoidable and low-risk: Mermaid injects `<style>` and KaTeX/Excalidraw set `style=` on
everything they draw. The Android shell serves through its own custom scheme and is unaffected.

## A commit that committed nothing must say *why*
**Why:** `commit_all` returns `Ok(false)` both for "clean tree, nothing to do" and for "this
vault is mid-merge, so I refuse" — and those are opposites. The refusal is right (staging
conflict markers would publish them as content), but read as success it means **every write
after the conflict is saved to disk and never committed**, for as long as the conflict sits
there, while the sync loop reports `synced`. **Consequence:** the `commit` dispatch arm answers
`CommitResult { committed, conflicts }`, filling `conflicts` from `vcs::conflicts` whenever it
committed nothing, and **all three callers** stop at the `conflicts` phase, which the UI already knows how to
show. The third one is the point: `App.svelte`'s 5 s debounced auto-commit is what
`commit_all` itself calls "the default path, not an edge case", and the first version of this
fix touched only `sync.svelte.ts` — so the dominant caller kept swallowing it. The extra
`vcs::conflicts` call is gated on `.git/MERGE_HEAD` existing, so the common case (clean tree)
costs a `stat`, not a second `git status` spawn. The `git.rs`/`git_native.rs` signatures were
deliberately **not** changed: the distinction is only needed where a human is told about it,
and widening the seam would have touched both backends and forty call sites for nothing.

## The study agent's model warm-up is deferred a few seconds after launch (2026-07-24, `#agent`)
**Why:** with the agent enabled, `fm-serve` auto-spawns the whole stack at launch — and the model
server reads a **multi-GB GGUF off disk and loads it onto the GPU the instant it starts**. Since
adopting Qwen3-VL-4B (2.4 GB) that load began *the moment the browser is told to open*, and on the
owner's laptop (4 GB GPU shared Intel iGPU + NVIDIA via Vulkan, Firefox compositing on the same GPU)
it starves exactly the disk and GPU the browser needs to paint the app's first frame. Measured:
`fm-serve` binds in ~57 ms and fires `xdg-open` at ~33 ms, but the *window* crawled in because the
2.4 GB read + Vulkan load hit at t≈0. That is what "double-click takes forever now" was — not the
launcher or the server, which were never slow. **Consequence:** `agent::spawn_at_launch` now claims
the run slot immediately (so a concurrent settings toggle still can't double-spawn) but sleeps
`AGENT_WARMUP_DELAY_SECS` (5 s) on a background thread before the actual spawn, releasing the slot if
that spawn fails. The page gets an uncontended window to render; the assistant warms a beat after the
app is already usable. This is *only* about launch contention — the `/api/set_agent` "turn it on now"
path stays immediate, because there the user is asking for the model, not for the app to open.
Verified after the change: browser-open at +0.06 s, model spawn at +5.09 s.
