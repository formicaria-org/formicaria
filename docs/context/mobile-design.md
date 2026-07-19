# Mobile design — the receipts behind Track M

The line-by-line design + code audit behind [`plan.md`](./plan.md)'s **Track M** (formicaria
on the phone), the same way [`collaboration-design.md`](./collaboration-design.md) sits behind
Track C. `plan.md` carries the sequence-defining *rulings*; this file carries the *why*, the
*why-not*, the exact code the port touches, **and the staged sequence** — the same shape as
`collaboration-design.md`'s `## Sequencing (do not reorder)`.

**Cite rulings by subject, never by number.** This file and `plan.md` number them differently and
always have; the hybrid that used to sit here propagated a wrong number into `decisions.md`.

**Status (verified 2026-07-19).** Shipped: the one command surface (`fm_app::dispatch`), the
streaming blob route, the explicit sync loop, the liveness/reindex split, the whiteboard scene
merge, the phone CSS shell, the board touch fallback. **Rejected** (not blocked — the foundation
is not coming back): the **`git2` backend** and the **`git2::merge_file` body engine**. The
**transport ruling — Tauri bridge primary, `fm-serve`-on-device as the documented retreat — is
untouched, not rejected.**

**Four things block a phone build, not one:**

1. **Toolchain** — the Android NDK + SDK are not conda-packaged (the four
   `rust-std-*-linux-android` targets *are*; see `known-issues.md`).
2. **Git backend** — deliberately open; the `git2` rejection left it so on purpose.
3. **Body-merge engine** — `merge_files` still shells `git merge-file` (`merge.rs:204`, `:233`),
   so there is no engine a phone can call. This is a *separate decision* from the backend, and it
   is the one nobody had named.
4. **Transport** — decided in principle, unbuilt.

**`git.rs` has no `clone`.** M1 is new code on every possible backend, not a port. Nothing in the
sequence below should be read as "port the existing primitive."

This file supersedes the owner's first mobile draft, which an adversarial
review (three code audits + a pass against `decisions.md`) found to rest on one false premise,
silently pick the more invasive of two architectures, and reverse two carried decisions without
saying so. Those are fixed below; each fix is marked **(corrected)**. A later round of building
refuted three more of its rulings — those are marked too, which is the point of keeping the
file rather than deleting it.

> **The override.** `MASTERPLAN.md:57` deferred mobile and pre-framed it as *"a server + auth
> decision"* — the phone as a thin client to the laptop's server. **The owner overrides that:**
> the app runs **on the phone itself**, so you can collaborate with yourself (and others) across
> devices over git-repo vaults, with feature parity of today's views (notes, Board, Agenda/
> Calendar, Timeline, Search, and the Excalidraw whiteboard), editable and merged on both
> platforms. Multiple repos are supported (each repo = one audience = one remote — the desktop
> `MultiStore` model, unchanged). The overriding constraint is **minimal decade-scale
> maintenance**. Recorded as a decision in `decisions.md`.

## The one-line thesis

*"Not two parallel workflows / seen seamlessly"* + *"minimal maintenance"* both point away from a
second implementation and toward **one shared core, reused on both platforms, with git
coordinating the same vaults across devices.** The codebase already has the seam for this — but
the first draft mis-named where it is. Fixing that is ruling 1.

---

## Ruling 1 — one command surface (the load-bearing correction)

**SHIPPED 2026-07-18.** `fm_app::dispatch` exists; `fm-serve` is an HTTP shell over it. Three
things the extraction taught, beyond the plan below:

1. **The lock could not move.** The plan's proposed `dispatch(..., vaults: &mut Vaults)` would
   have held the lock for the whole command — but five arms exist precisely to *drop* it
   before slow I/O (`asset_status`, `resolve_asset`, `open_external`, `backup`,
   `backup_status`). So `App` owns the `Mutex` and each arm takes it exactly as long as it
   did before. (The old comment claiming a held lock could starve the *auto-shutdown
   watchdog* was stale and is now corrected in place: liveness is refreshed before dispatch,
   so a slow command cannot make the app quit. It does stall the reindex poll, which is the
   real cost.)
2. **`vaults.rs` moved up too**, from `fm-serve` to `fm-app`. The vault list is state the
   command surface owns; HTTP was only the first caller. A second transport re-reading
   `vaults.json` would have been the fork the extraction exists to prevent.
3. **`open_external` is the one genuinely platform-bound arm**, so it became a seam — the
   `Host` trait, one method, implemented by the shell. `#[cfg(target_os)]` inside `fm-app`
   would compile a wrong answer for Android. Note `open_native` has a *second* caller in
   `fm-serve::main` (the `FM_OPEN` browser launch), which independently keeps it there.

The original finding, for the record:

**(corrected — the first draft's central premise was false.)** The draft said the durable
boundary is `fm-app`, *"already fronted by two independent frontends (`fm-serve` and `fm-cli`),
so nothing forks."* Audited: **`fm-cli` does not front `fm-app`.** `crates/fm-cli/Cargo.toml` has
no `fm-app` dependency *(true when audited; it depends on `fm-app` and shares
`commands::asset_note` since 2026-07-18)*; `fm-cli/src/main.rs` called `fm-core` directly and **re-implemented** the
command logic (e.g. `Cmd::Add`, `main.rs:131-146`, rebuilds the ingest flow rather than calling
`commands::ingest`). Only **`fm-serve`** fronts `fm-app::commands`. So the seam **already forks
once**, and a naive Tauri command layer would be a **third** dispatch surface parallel to
`fm-serve::api()` and `fm-cli` — the exact maintenance debt the plan claims to avoid.

Worse, `fm-serve::api()` is not "thin over `MultiStore`" (audited): it dispatches over
`Mutex<Vaults{ store: MultiStore, list: Vec<VaultConfig> }>` (`main.rs:38-48`) with a
**documented single-lock discipline** that exists to avoid an AB/BA deadlock between the
lock-then-resolve arms (`commit`/`push`) and the resolve-then-lock arm (`ingest`)
(`main.rs:31-37`, `362-367`, `476-478`). At least a dozen arms reach around the `Store` trait to
per-vault filesystem paths (`resolve_asset`/`asset_status`/`find_blob` walk every vault's blob
dir directly; `check_path`/`create_vault` write `vaults.json`). A Tauri port using `tauri::State`
naively reintroduces that deadlock and reimplements all of it.

**Fix — extract dispatch into the shared library.** Factor the `api()` `match` (`main.rs:278-529`)
plus the `Vaults` state and its lock discipline into a shared entry point, e.g.

```rust
// fm-app (new): the ONE command surface
pub fn dispatch(cmd: &str, query: &Query, args: &Value, vaults: &mut Vaults)
    -> Result<Vec<u8>, String>;
```

Then:
- `fm-serve::api()` becomes a thin HTTP shell: parse the URL/body → `dispatch(...)` → HTTP
  framing (the two non-JSON paths — raw-body `ingest`, bytes-out `resolve_asset` — stay in the
  HTTP shell where they belong).
- The mobile shell (ruling 2) calls the **same** `dispatch`.
- `fm-cli` **can** migrate onto it later (out of scope for Track M, but the extraction is what
  makes "one command library" finally true instead of aspirational).

This is a contained refactor of shipped desktop code, not a rewrite: the `match` already exists;
we move it behind a function and give both transports one door. It is the honest fix for the
contradiction the review found, and it makes every later mobile milestone thin.

---

## Ruling 2 — transport: thin bridge primary, server-on-device documented

**(corrected — the draft never weighed the cheaper architecture.)** Two ways to put the shared
core behind a webview on Android:

- **(A) Native command bridge (Tauri v2 mobile).** `@tauri-apps/api`'s `invoke('cmd', args)`
  maps 1:1 onto the camelCase contract; Rust registers `#[tauri::command]` wrappers that call
  `fm_app::dispatch`. Buys the plugin ecosystem the port needs anyway (Android Keystore,
  `tauri-plugin-opener`, foreground-service). With ruling 1 done, the wrappers are genuinely
  thin.
- **(B) `fm-serve` on the device.** Run the std-only HTTP server bound to `127.0.0.1` inside the
  app and point a WebView at it. `ui/src/lib/ipc.ts` **already speaks HTTP in this case** — the
  backend fork is on `import.meta.env.PROD` (`ipc.ts:23`), and a built Android bundle *is* `PROD`,
  so it would already take the `fetch('/api/...')` branch. Reuses raw-body `ingest`, bytes-out
  `resolve_asset`, the deadlock-safe lock, and the streaming blob route (ruling 7) with **zero
  transport reimplementation**, and keeps *"the browser is the product"* literally true on the
  phone — the escape hatch from the Tauri single-vendor bet that **already burned this project
  once** (`decisions.md`: the Tauri/WebKitGTK window "never painted reliably" → browser pivot).

**Ruling: (A) primary, (B) documented as the lower-fork fallback.** (A) wins on the plugin
ecosystem (Keystore/opener/foreground-service are Tauri-shaped and needed regardless), and with
ruling 1 the "third dispatch surface" objection is gone. (B)'s genuine cost: a localhost TCP
listener is reachable by other apps on the device, so it needs the existing Origin/CSRF guard
kept **plus** a per-launch bearer token — more security surface than in-process IPC. Keep (B) as
the retreat if a Tauri webview won't paint (the M0 kill criterion below). Either transport now
sits over the one `dispatch`, so the choice is reversible configuration, not a rewrite.

**Bridge specifics for (A):** insert the native branch in `ipc.ts` **ahead** of the `PROD` fork
(a Tauri build is `PROD`, so it would otherwise hit HTTP). Three spots need touching, not one:
`invoke` (`ipc.ts:23`), the binary `resolve_asset` return (`ipc.ts:40`), and `ingestFile`
(`ipc.ts:103`, which today POSTs a raw `File` to `/api/ingest?name=…&vault=…` — query params +
binary body, *not* JSON args). The upload direction passes a `Uint8Array` to a command; the
download direction goes over `blob://` (ruling 7), not an IPC byte-array.

---

## Ruling 3 — merge: `git2::merge_file`, not `diffy` (⛔ **refuted 2026-07-18 — that function does not exist**)

> **Audited against the real crate, not the docs.** `git2` 0.20.4 exposes only
> `Repository::merge_file_from_index(&IndexEntry, &IndexEntry, &IndexEntry, …)` — which needs
> index entries and so would pollute the ODB, contradicting `merge.rs`'s own design. The
> buffer-shaped `git_merge_file`, which *is* the right API, is bound in `libgit2-sys`
> (`lib.rs:3854`) but `git2` imports that crate **privately** (`use libgit2_sys as raw;`, no
> `pub`). So this ruling needs a **direct `libgit2-sys` dependency plus ~40 lines of unsafe
> FFI**, and it inherits ruling 6's licence question. The claim below that it was "verified
> against the git2-rs docs" is the error to learn from: the docs describe `MergeFileOptions`,
> which exists; the function taking it does not, at this layer.
>
> **What shipped instead, and needed none of it:** `crates/fm-core/src/scene.rs` — the
> `.excalidraw` half of this ruling — merges whiteboard scenes element-wise from `merge_body`,
> in pure Rust with `serde_json`. Also corrected: **there is no `.excalidraw` file.**
> `FileStore` writes `<ulid>.md`, and the existing `*.md merge=fm` attribute already routes
> board notes into `merge_files`, so this is a branch inside `merge.rs` and never a second
> driver. Verified through real git in `crates/fm-cli/tests/merge.rs`.

The superseded reasoning, kept because the `diffy` rejection still stands:

The draft swapped the body 3-way merge from `git merge-file` to the third-party `diffy` crate,
then worried in-text whether `diffy` even exposes a marker size and the three conflict labels.
**`git2` — already being added for the git port — exposes `merge_file` with `MergeFileOptions`
carrying `ancestor_label`/`our_label`/`their_label` and `marker_size`** (verified against the
git2-rs docs). That is libgit2's in-tree port of the *same* `git merge-file` algorithm.

**Use it as the single body engine.** Zero new dependencies (the plan's own *"bounded,
replaceable dependencies / reuse mature tested tools"* rule — reaching for a niche crate on the
one path that must never corrupt is the move the plan forbids everyone else); the engine stays in
the libgit2 family, maximally faithful to the on-disk conflict format; and both merge shapes the
audit found — `merge_body` (`merge.rs:184`, `-p`/stdout with `-L ours/base/theirs`) and
`whole_file` (`merge.rs:212`, label-less, **in-place**, so a genuine fallback conflict is
currently labelled with the real note *paths*) — fall out of one `merge_file` call with options.

**Keep the desktop driver installed (as the draft's own correction already said).** `fm-core`'s
frontmatter merge (`merge_objects`/`three_way`/`union`, `merge.rs:74-144`) is already pure and
untouched; only the body engine changes. Route **both** the desktop driver (`fm merge-md`, the
`install_merge_driver`/`.gitattributes` `*.md merge=fm text eol=lf` mechanism, still installed on
desktop) **and** the in-process pull-merge through the same `merge_files` whose body is now
`git2::merge_file`. Mobile installs no driver (no terminal git) and calls `merge_files` directly.
One engine, three call sites, cannot diverge.

**Gate the swap with a differential test:** new engine vs the old `git merge-file` over many
random `(base, ours, theirs)` triples, asserting agreement on clean-vs-conflict **and** on
conflict-region bytes (or consciously freezing the accepted differences). `crates/fm-cli/tests/
merge.rs` drives real `git merge` through the installed driver, so it stays meaningful only while
the driver is alive — do not kill it.

**A second merge type — `.excalidraw`** (whiteboards): an element-level 3-way merge over git's
base tree (honour deletions via the base, higher `version` wins, tie-break lower `versionNonce`,
watch the fractional `index` z-order) — **not** Excalidraw's base-less `reconcileElements`, which
resurrects deleted shapes. New shared-core Rust (~80 lines, `plan.md` Track C Phase 3), benefits
desktop too. Same guarantee: divergence falls back to a conflict, never a silent drop.

---

## Ruling 4 — auth: PAT-first, OAuth optional (corrected)

Once SSH is dropped (below), every host authenticates git over HTTPS with username + token, fed
to libgit2's `Cred::userpass_plaintext`. Whatever obtains the token plugs into the same callback,
so this is a UX choice, not an architecture one — and the draft had the default backwards.

- **Default: paste a fine-grained PAT.** It is the user's *own* credential, host-agnostic (Gitea
  and any git host, not just GitHub/GitLab), and needs no vendored OAuth-App registration on the
  critical path. This fits *"open, user-owned, not vendored"* best.
- **Optional: OAuth 2.0 Device Authorization Grant** as a nicer phone UX (tap "Sign in," approve
  a short code in a browser, poll for a user token — how the GitHub CLI authenticates). Note it
  **vendors** you to each host's OAuth-App program: if the registration lapses or a host changes
  policy, every user's default sign-in breaks — which is exactly why it is the *convenience*, not
  the default. GitHub: OAuth App + `repo` scope (**not** a GitHub *App* installation token —
  fine-grained-permission collisions, "can't push" reports). GitLab: device grant needs 17.1+,
  store the refresh token.

- **Storage — a deliberate, scoped reversal (logged).** The desktop backup design holds that
  *"the app stores no secret of its own"* — notes are authenticated by the user's own
  ssh-agent/credential-helper (`decisions.md`, two-tier backup). **A phone has no ambient
  credential-helper**, so the app *must* hold the token. Scope the reversal to exactly that:
  store the token/refresh token **encrypted under an Android Keystore key** (a Tauri keystore/
  `keyring` plugin), optionally biometric-gated; **never** plaintext prefs, **never** in the
  remote URL (it leaks into `.git/config`). On a 401 mid-sync, refresh (OAuth) or re-prompt
  (PAT), and **fail after one bad attempt** — git2-rs hangs if the credentials callback keeps
  returning bad creds. Logged in `decisions.md`.

- **Inject a `CredentialSource`, not a global (the draft's own correction, kept).** Only three
  functions touch the network — `pull`, `push_squashed`, `remote_moved` — so give exactly those a
  `creds: &dyn CredentialSource` whose `for_url(url, username) -> Cred` maps onto libgit2's own
  per-URL `RemoteCallbacks::credentials`. Per-URL dispatch serves multiple repos (vault A on
  github/alice, vault B on gitlab/bob — a headline goal); re-reading the keystore per call handles
  refresh; a fake impl makes it testable. A process-global `OnceLock<Box<dyn Fn() -> Cred>>` fails
  all three (no per-repo arg, set-once, untestable) — rejected. The trait lives in `fm-core::git`
  (environment-free); the concrete Keystore/OAuth impl lives in the mobile crate; `fm-serve` owns
  a desktop impl that shells `git credential fill` (preserving ambient auth — see below).

---

## Ruling 5 — `set_identity` on clone (multi-user correctness, was dropped)

The provenance model — `EditedBy` labels, the Activity pane, the contributor filter — rides on
`git::activity` reading a **real** committer name/email, and the `PLACEHOLDER_EMAIL` sentinel is
load-bearing: a shared vault on the placeholder attributes *everyone's* commits to one fake name
(the hole Phase 0 closed; `decisions.md`, "A vault gains an identity when it gains an audience").
OAuth/PAT gives a *login*, not necessarily an email. The draft wired the token but **never wired
the identity**. **Fix:** the mobile clone flow calls `git::set_identity(vault, name, email)`
before the first commit, sourced from the signed-in account (prompt for the email if the token
scope doesn't yield one). Otherwise every phone commit is `formicaria@localhost` = "you" = the
provenance hole reopened, precisely in the multi-user case the owner cares about.

---

## Ruling 6 — `git2` becomes the single in-process backend (⛔ **REJECTED 2026-07-18**, not merely blocked — see `decisions.md`, "`git2` is rejected; git stays a subprocess capability")

> **1. libgit2 cannot invoke external merge drivers.** Verified in the vendored C: only
> text/union/binary are registered, and libgit2 contains no process-spawn anywhere.
> `git_merge_driver_register` is unbound in both `libgit2-sys` and `git2`, and even wired up it
> would not affect anyone else's git. So porting `pull()` **silently disables the `.md`
> frontmatter merge** — the thing Track C Phase 1 exists to provide — while a collaborator
> running `git pull` in a terminal still gets it. Two merge semantics in one vault, and the
> app's is the worse one. There is no workaround; this alone blocks the ruling.
>
> **2. It violates `deny.toml`, and passes only on a metadata technicality.** The policy is
> explicit: *"any GPL/AGPL/SSPL/BUSL/source-available crate that is **linked** (not shelled out
> to) must fail the build. GPL tools like pdftotext/libvips are invoked as subprocesses and
> never appear in this graph."* libgit2 is GPL-2.0-with-linking-exception. `cargo deny` goes
> green because `libgit2-sys` declares `MIT OR Apache-2.0` while vendoring ~230k lines of GPL C
> — the gate clears because the metadata under-declares, not because the policy is satisfied.
> Worse, `ci/third-party.sh` reads that same field, so we would ship binaries **omitting a
> notice the linking exception requires**, and fixing that means hand-editing a file whose
> header says it is never hand-maintained. This is an owner policy call.
>
> Two smaller corrections while here: the Android C build is **not** the obstacle this ruling
> assumed (`libgit2-sys` builds via `cc::Build`; the vendored CMakeLists are inert, and pixi's
> `c-compiler` suffices). And `graph_descendant_of(X, X)` is `false` where
> `merge-base --is-ancestor X X` is `true`, which would silently break the in-sync no-op push.

The superseded reasoning, kept because the `gix`-vs-`git2` comparison still holds:

**(corrected — the draft under-owned this.)** `fm-core` has **no** `git2`/`gix` today (verified:
the only `git2` string in the workspace is the plan proposing it); git is invoked purely as a
subprocess (`Command::new("git")`, `git.rs:78`) and is genuinely optional — *"Git is a
capability, not a dependency"* (`decisions.md`): empty `PATH`, everything still works. **All ~13
public git ops shell out** through one helper — `ensure_repo`, `commit_all`, `identity`,
`set_identity`, `remote`, `set_remote`, `unpushed`, `activity`, `conflicts`, plus the three
network ops — so the port surface is *all of them*, not the "only 3 functions touch the network"
the draft framed (that count is a red herring for a subprocess→library port). `merge.rs` also
shells `git merge-file` at `184`/`213`, outside the `git(vault)` helper — ruling 3 replaces those.

Reimplement every `git.rs` body against `git2`, preserving signatures (the three network fns gain
`&dyn CredentialSource`). **Why `git2` not `gix`:** the defining feature is push over HTTPS;
`gix` push is not shipped upstream (fetch/clone are), while `git2`/libgit2 has mature HTTPS
clone/fetch/push/merge. Its cost — C cross-compile under the Android NDK — is the *same class* of
problem the project already solves for `rusqlite bundled` SQLite (`fm-core/Cargo.toml:18`). Keep
`gix` as a documented future swap for when its push matures (then the Android build gets much
simpler). **HTTPS-only, no SSH:** enable git2-rs's `https` feature, not `ssh` (SSH → libssh2 →
OpenSSL per-ABI NDK work + host-key/`known_hosts` UX for zero capability gain — every target host
authenticates over HTTPS+token). This retires the `GIT_SSH_COMMAND`/ssh-agent assumption
(`git.rs:85`).

**The reversal to own in writing:** linking `git2` compiles a git implementation (and its C
cross-compile) into `fm-core` **always**, for every desktop user who never wanted git — a bigger
binary and a mandatory C build. In exchange, in-process git is strictly *better* than "hope `git`
is on PATH." So the capability model shifts: `git::available()` (`git.rs:66`) becomes effectively
always-true, so **repoint its callers** (the `BackupStatus.git` field, the copy/uncopy
commit-gates) at what actually matters — a configured identity/remote — so "this machine has no
history yet" stays an honest signal. Desktop also loses ambient ssh-agent auth and `git@…:` SSH
remotes; mitigation: the desktop `CredentialSource` shells `git credential fill` (ambient
credential-helper auth preserved *through* git2), and SSH remotes migrate to HTTPS. Logged as
reversing *"git is a capability, not a dependency."* The one-backend call is deliberate: a
`#[cfg]` two-implementations-of-one-seam is the maintenance trap "minimal maintenance" rejects.

---

## Ruling 7 — build the real streaming blob route (it never existed)

**SHIPPED 2026-07-18 (the desktop half).** `GET /api/blob/<reference>` exists in
`fm-serve/src/blob.rs`: streamed from disk in 64 KB chunks, sniffed `Content-Type`,
`Accept-Ranges`, real `Range` (206/416), and the UI's inline media points at it instead of
minting object URLs. The mobile `blob://` protocol handler is still to build, but it now has
a working reference rather than an imagined one. What remains of the original finding, for
the record:

**(corrected — the draft mis-cited it as "planned/existing.")** Audited: there was **no** `GET
/api/blob/<hash>` route; blob bytes travelled via `POST /api/resolve_asset`, which buffers the
whole file into RAM (`commands.rs:206`, `std::fs::read`) with no `Range`. (The draft also
attributed a "512 MB cap" to it — wrong: that cap is on inbound `Content-Length` and guards
*uploads*; the blob **read** path was uncapped.) The draft leaned on this route as the
reference for the mobile `blob://` protocol.

Two things the build found that the design did not: `resolve_asset` **threw the sniffed MIME
away** and answered `application/octet-stream` for everything, so the caller had to ask
`asset_status` separately and build a typed `Blob` by hand — the route sends the real type. And
serving blobs from a *navigable* same-origin URL is a security change, not just a performance
one: see the `Content-Disposition` allowlist in `blob.rs`.

**Build it once, for both platforms:** a streaming `GET /api/blob/<hash>` in `fm-serve` with the
sniffed `Content-Type` (call `ingest::sniff_mime` — `resolve_asset` currently throws the MIME away
for `application/octet-stream`, `main.rs:534`), `Accept-Ranges`, and honouring `Range`. On the
Tauri bridge, register a v2 **async URI-scheme protocol** (`blob://<hash>`) served straight to
`<img>/<video>/<iframe src>` honouring `Range`; on the server-on-device transport (ruling 2B) the
same HTTP route serves it directly. Either way, retire the object-URL buffering in
`NotePanel.svelte` (`lib/`, not `renderers/` — see paths note). This fixes desktop efficiency and
unblocks mobile media at once. Upload stays a one-shot `Uint8Array` to a command.

---

## Ruling 8 — auto-push is explicit, never silent

**(corrected — the draft's "auto-push after auto-commit" wedges silently.)** `push_squashed` is
*designed* to **reject** when the remote moved and **tell the user**, and to **never fetch** in
that flow (`decisions.md`, "Squash-on-push": fetch-first would `reset --soft` onto their tip and
silently overwrite their tree). Automating the push removes the human who was told, so a moved
remote becomes a **silently-rejected push loop** — the documented "auto-commit is best-effort and
**silent**" known-issue, now on the sync-critical path. **Fix:** the seamless loop is an explicit
`commit → push; on reject → pull (merge via ruling 3) → re-push`, with a visible sync-status
affordance and a surfaced failure — never a swallowed `.catch(() => {})`. This is the *only*
sync orchestration Track M adds, and it is minimal (a state machine over the existing ops, not a
framework — the *"no sync framework"* rule holds). Desktop companion (small, parallel): auto-pull
on `remote_moved` + this explicit auto-push, so desktop↔mobile is bidirectionally seamless.

**Seamless ≠ real-time (set honestly):** git is the substrate, so sync is fast-async — auto-pull
on foreground/`remote_moved`, debounced auto-commit, auto-push on background — not live
co-editing. A true concurrent conflict lands **in the note body** (frontmatter stays whole/valid),
so the note still parses, indexes, and opens — never a silent last-writer-wins.

---

## Ruling 9 — Path A is an early demo, not the destination

The draft left one fork open — does mobile run git itself (**Path B**: on-device `git2`), or stay
**transport-only** while the desktop records (**Path A**: a friendly transport — Google Drive via
rclone, or Syncthing — replicates files; the desktop heals conflict-copies and records to git)?
The `SyncProvider` seam makes this per-install configuration, so it need not be settled now. **But
name the trade honestly:** Path A retreats toward the satellite-of-desktop model the owner
**overrode**, and it **fails a phone-only user or collaborator** (no desktop = no recorder),
hurting the multi-user goal. So: **Path B is the destination** — the real "the app runs on the
phone." Path A is a legitimate *low-risk early demo* that collapses the hardest milestones
(M1/M3/M4 become "desktop records / mobile syncs files"), useful to prove the shell before the
on-device git2 lands — not the shipped end state.

---

## Forward-looking: sync is a seam, git is one provider (YAGNI-bounded)

A vault is just a folder of files; git+restic sync/back-up *for now*, and nothing should
hard-wire them. Structure coordination as a **pluggable seam** so a friendlier cross-platform tool
can implement it later — by **reusing mature tools** (git, restic, Syncthing, rclone), never a
home-grown sync engine (*"no sync framework"* stands). Four roles are fused today; the seam pulls
them apart:

| Role | Today | The seam lets it be |
|---|---|---|
| **Transport / coordination** | git remote | git **or** Syncthing/LAN, rclone→cloud |
| **Recording / versioning** | git commit | git (the constant) — a change is one more commit over whatever a transport delivered |
| **Conflict-merge** | `.md`/`.excalidraw` merge | **app-owned `merge_files`** — identical for every transport, *once it is in-process* (it is not yet) |
| **Backup / durability** | restic | restic/borg/kopia — a **separate lane**, never inside the sync seam |

**⛔ Corrected 2026-07-19 — this section previously claimed the load-bearing move was already
done. It is not.** The app does **not** own the merge in-process: `merge_files` shells
`git merge-file` (`merge.rs:204`, `:233`) and takes **driver-shaped inputs** — three paths on
disk, writing over `ours` (`merge.rs:58`, `:75`) — which on desktop exist only because git
materialized `%O %A %B`. A phone caller cannot produce those without an ODB reader, i.e. the
backend it does not have.

So the safety case for a dumb transport is **contingent, not established**. The design is still
right: Syncthing renames the loser to `*.sync-conflict-*`, a watcher pairs it with the current
file (+ the git merge-base as ancestor) and runs **our** merge, and a friendly transport thereby
gains git-like safety. But it requires work that does not exist — extract
`merge_texts(base, ours, theirs, marker_size)` from the path-shaped wrapper, then supply a
pure-Rust body engine. **Until then this table describes an intention, not the code.**

**What this costs now — minimal.** Introduce the seam as a thin `SyncProvider` trait with
`GitSyncProvider` as the **sole** implementation (route the UI's `commit`/`push`/`pull`/
`remote_moved`/`activity` through it, not `git::` directly — an interface extraction over the same
code, folded into the git2 work). **Build no second provider now**; designing the trait against
Syncthing's capability profile *on paper* is enough to keep it general. The discipline that proves
the seam isn't git-shaped: make **`history`/authorship a *queried optional capability*** (git
reports `history: true`; Syncthing reports none, and the EditedBy/Activity/contributor surfaces
just hide). Mandatory capabilities: `replicate`, and `surface_conflicts() -> {path, mine, theirs,
base?}`. Everything else optional.

**Backends: default free & serverless, self-hosted first-class.** A *provider* moves bytes; a
*backend* is where they land. Guiding rule: **a user should never have to buy a server.** The
reusable unlock is **rclone** (~70 backends: Drive/Dropbox/OneDrive/WebDAV/S3), which plugs free
cloud into three roles — transport (rclone syncs the vault; desktop records to git; the app-owned
merge heals Drive's conflict-copies), git remote (a free GitHub/GitLab private repo is the cleanest
zero-server option), and restic backend (restic→rclone→Drive = free offsite media backup). A menu,
not a fork to resolve now:

| Strategy | Free/no-server remote | Records git |
|---|---|---|
| **Free & easy (default)** | free GitHub/GitLab private repo + Drive (media via rclone) | desktop (mobile optional) |
| **All-Google** | vault ↔ Drive via rclone; restic→Drive | desktop |
| **P2P, no account** | Syncthing device-to-device | desktop |
| **Fully owned** | self-hosted Gitea + own Syncthing + restic to own NAS | any |

**Non-interference contract** (so nothing corrupts anything): exactly one tool owns writes to any
path class; every tool ignores every other's internals + the per-machine disposable index
(`.git/`, `.stfolder`, the restic repo, `index.sqlite*`, `derived/`) via `.stignore`/`.gitignore`/
restic `--exclude`; the restic repo lives **outside** the synced root; writes are atomic
temp+rename (already true, `file.rs:106`); dumb-transport conflict-copies funnel into the app
merge, not left as litter. (Syncthing replicating an un-excluded `.git/` or a live SQLite index is
documented data-loss.)

**The recorder is swappable too — git today, not locked in.** Researched: keep **git** as default
(nothing else clears *both* "embeddable Rust library" and "git interop" yet). Watch **Jujutsu
(jj)** — Rust `jj-lib`, git-native (preserves history), first-class conflicts — but no stable API
yet and still pulls libgit2. Watch **gitoxide (`gix`)** as the pure-Rust escape from libgit2's
Android C build — but its **push is not shipped**, which is why ruling 6 picks git2. **Fossil**
(external process, not a library) and **Pijul** (bus-factor ~1, no git interop) are watch-items,
not plan-items. Net: git default; seam capability-based so jj/Fossil can slot in; build no second
recorder now; watch `jj-lib` stabilization + libgit2→gitoxide + `gix` push.

**Reconciled with the carried decisions:** this *extends* "a bare git remote is the coordinator"
to "**coordinator is a role** — git fills it by default, a friendlier tool can fill it for mobile"
and keeps "**no CRDT / no home-grown sync framework**" intact. CRDT (Yjs/Automerge) stays out — it
changes the atom from the file to per-block ids and doesn't merge media, reopening the very "no
per-block ids" invariant the project is built on, for a real-time we've said we don't need.

---

## The views + the whiteboard across devices

- **Most views come nearly free — with one touch gap.** Board/Agenda/Calendar/Timeline/Search are
  generic renderer components; on mobile they are the same components in a single-column shell,
  and their data already syncs. **But** (audited) the renderers are *not* fully platform-neutral:
  `Board.svelte`/`Card.svelte` import `@atlaskit/pragmatic-drag-and-drop`'s **element (HTML5-drag)
  adapter, which does not fire on touch** — so drag-a-card-to-a-column does nothing on a phone
  (and `Pane.svelte` reorder uses the same adapter, also touch-hostile; pane resize is
  `PointerEvent`, works). And every renderer imports the **global `activity.svelte.ts` singleton**
  — a hidden coupling to account for, not a props-only component. The existing fallback is
  **StatusChip click-to-rotate** (`lib/NotePanel.svelte:631`), which only cycles *status*; for
  arbitrary-column moves add a **tap → move-to-column menu**. That is the *only* option —
  "swap in pragmatic-DnD's pointer adapter" was wrong and is struck: version 2.0.1 ships
  element, external and text-selection adapters and **no pointer one**, and none of its 12
  companion packages provides one (audited 2026-07-18). The alternative is hand-rolling over the
  package's unversioned `make-adapter/` internals, which is writing a drag engine, not swapping
  an adapter. The menu is cheap anyway: `Board` already exposes `columns: {value,label}[]` and
  `onmove(id, value, beforeId)`, so it reuses the desktop write path with **no new command** —
  and stays literal-free, because column names are runtime data. Watch `ci/checks.sh`: it greps
  `ui/src/renderers` case-insensitively for a whole-word `todo|doing|done`, so a "Done" label
  fails the build. Card/column order stays `localStorage` per-device (don't sync it).
- **Whiteboard is the one view with real cross-device work.** A board note is `view: board` with
  an Excalidraw scene JSON body; the same `Whiteboard.svelte` (React/Excalidraw, lazily loaded)
  runs in a mobile webview with usable touch. Two shared-core pieces make it *sync well* (both
  improve desktop): the scene 3-way merge — **✅ shipped 2026-07-18** as
  `crates/fm-core/src/scene.rs`, a branch inside `merge_body` and **not** a second driver, since
  there is no `.excalidraw` file (`FileStore` writes `<ulid>.md`; `*.md merge=fm` already routes
  boards there) — and **Ruling B — strip embedded
  images out of the scene body into the blob store on save** (⛔ **blocked**: `blobs/` is
  gitignored, so after the strip a shared board's images stop appearing on a collaborator's
  clone. The two options below are still undecided, and shipping the strip before deciding is a
  visible regression. A carried decision, not yet coded;
  `saveBoard` at `lib/NotePanel.svelte:249` re-serializes the whole scene on the debounced
  `onChange`, `Whiteboard.svelte`). Without the strip, a 2 MB screenshot is a multi-MB body
  `fsync`'d per stroke on a phone — flash wear + battery. **Ruling B must land before boards are
  *editable* on mobile.** After it, whiteboard images live in the git-ignored blob store, so image
  bytes don't travel with the git-synced scene JSON — make them appear cross-device via **(a)
  git-tracking whiteboard-embedded blobs** (a scoped exception; simplest, fully offline,
  hash-deduped — recommended for the minimal cut) or **(b) a blob mirror** (rclone/S3, the scale
  path, deferred).

---

## Efficiency on a battery device (don't port desktop's polling)

Both harmful desktop defaults have the right machinery already in the tree:
- **Drop the 3 s reindex poll; go lifecycle-driven.** The UI's `setInterval(beat, 3000)`
  (`App.svelte:422`, PROD-gated) makes `ping` reindex every beat; on mobile the app is the sole
  writer, `pull` reindexes inline (`main.rs:407`), and there's no Vim — so the poll's job
  evaporates. Reindex on foreground/resume and after each `pull`; drop the periodic beat. Convert
  the 45 s `remote_moved` poll (`App.svelte:380`, also PROD-gated) to "check on foreground."
  **Preserve `ping`'s `{changed}` refresh signal** so auto-pull-on-foreground still repaints.
- **Cold-start with `Incremental`, not `Full`.** `FileStore::named` unconditionally rebuilds the
  whole FTS index on open (`file.rs:60`, `Reindex::Full`) — and Android kills backgrounded apps,
  so this runs on *every* relaunch across *all* vaults; `index.sqlite` persistence buys nothing at
  startup. The mtime incremental path already exists (`file.rs:309-334`). Open `Incremental`
  (schema-gated), falling back to `Full` on mismatch/corruption — O(changed) launches.
- **WebView memory:** land Ruling B so scenes aren't MB of base64; **unmount the React root when
  leaving a board** (`Whiteboard.svelte` already `root.unmount`s on destroy — confirm the
  bottom-nav shell *destroys* it, not CSS-hides). Budget-test on a low-RAM emulator profile.

---

## Media extraction — pure-Rust, shrink desktop's deps too

Don't drop `ingest`'s text/thumbnails on mobile — that silently regresses search-inside-PDF +
thumbnails, and *asymmetrically*: a PDF ingested on desktop keeps its extracted text in the note
body (git-tracked, FTS-indexed, `ingest.rs:6`); the same blob ingested on the phone (no
`pdftotext`) has an empty body → **divergent git-tracked note content for identical bytes**, and
its text is unsearchable. (Narrower than "invisible": the filename still indexes via the title,
and plain-text/markdown/SVG ingest needs no subprocess.) Pure-Rust replacements exist and **shed
desktop's GPL subprocesses** (the project's "bounded, replaceable dependencies" ethos):
- **PDF text:** `pdf-extract`/`lopdf` behind `extract_text` (`ingest.rs:85`) — retires
  `pdftotext`.
- **Raster thumbnails:** the `image` crate; `resvg` for SVG — retires most of `vipsthumbnail`.
- **Narrow honest deferral:** only **PDF-page raster** thumbnails (needs pdfium/C) and **video
  posters** (ffmpeg) degrade.

---

## Paths note (the draft mis-pathed several "critical files")

Audited corrections — `NotePanel`, `Whiteboard`, `Pane` are in **`ui/src/lib/`**, not
`ui/src/renderers/`, and are **not** renderers. `renderers/` holds only Activity, Agenda, Board,
Calendar, Card, Search, Timeline. Line anchors otherwise verified.

## Device safety — the phone is not a test rig

**The Android device used for this work is the owner's personal phone.** Everything below is a
hard rule, asked for directly after a `rm -rf "$V"` built from a shell variable — scoped to a
scratch path, but the wrong shape of command to point at someone's phone.

- **Write only under `/data/local/tmp/`.** Never `/sdcard`, `/storage`, `/data/data`,
  `/system`, or any app's directory — that is photos, messages, and every installed app.
- **Never build a destructive command from a variable.** Literal paths only, and only paths
  this session created; an unset variable turns a scoped delete into an unscoped one.
- **Never `adb install`/`uninstall`/`pm`/`settings put`/`su`.** No device state changes.
- **Do not read personal data at all**, even read-only. `logcat` filtered to our own output is
  fine; anything else is not ours to look at.
- **Clean up, then `adb disconnect`** when done, so nothing can reach the phone until the owner
  re-pairs. Wireless debugging pairing is theirs to revoke whenever they like.

**Why `/data/local/tmp` is both sufficient and the limit:** `adb shell` runs as the *shell*
user, outside the app sandbox, so a pushed binary can execute there — the W^X restriction that
stops an *app* exec'ing from its own data directory does not apply. That is exactly what made
`sessions/2026-07-19-the-core-on-the-phone.md` possible with no SDK, no APK and no shell. It is
a test fixture, never a product path, and it is the reason the port still needs in-process git
rather than a bundled `git` binary.

## Corrected sequence (2026-07-19) — steps 0–5 need no backend and no NDK

**Why this replaced the old spike list.** Spikes (ii) and (iii) were `git2` spikes and are
**void**; the phases below them were written against the same rejected mechanism. The corrected
order front-loads everything that pays off *under every possible backend*, so that no work is
staked on a decision nobody has taken yet. Receipts:
`sessions/2026-07-19-mobile-drift-review.md`.

- **STEP 0 — ✅ RUN 2026-07-19, and it already reordered what follows.** The toolchain and the
  tunnel hold (`sessions/2026-07-19-the-phone.md`); the finding is that **the current GUI is not
  good for small screens**, which puts the single-column touch work (M6) **ahead of M0 and step
  8** — an APK wrapping a shell that does not work on a phone is worse than no APK, because it
  makes a layout problem look like a platform problem. The per-path observations were not
  recorded, so `outstanding.md` §1.1 is only *partly* retired and the MVP line is **not** re-cut
  yet. The procedure, kept because it is how you do it again:
  ```
  pixi run serve            # builds the PROD bundle: real dispatch, real blob.rs, real Range

  pixi run android-init     # unpacks platform-tools into .android/, SHA-256 verified.
                            # Nothing installed system-wide; re-running is a no-op.

  # Phone: Developer options -> Wireless debugging -> Pair device with pairing code.
  ADB=.android/platform-tools/adb
  $ADB pair <phone-ip>:<pair-port>      # the PAIRING DIALOG's port; no USB, no udev, no sudo
  $ADB devices                          # the phone is already listed — see below
  $ADB reverse tcp:8765 tcp:8765        # phone's localhost -> this host; NO listener on the phone
  # phone browser: http://127.0.0.1:8765
  ```
  **There is no `adb connect` step, and adding one breaks it.** adb discovers the device over
  mDNS and connects it itself the moment pairing succeeds. The port in the pairing dialog is
  *single-use* and dies on success — different from the one on the main Wireless-debugging
  screen — so a hand-run `connect` fails, leaves a dead second entry, and every later command
  then answers `more than one device/emulator`. `adb disconnect` clears the strays.
  Wireless rather than USB on purpose: USB `adb` on Linux needs udev rules or `plugdev`
  membership, and a system requirement is exactly what ruling 3 rules out. `adb reverse` is
  transport-agnostic, so the tunnel — and the guard argument below — is unchanged.
  The device's browser sends `Host:`/`Origin: 127.0.0.1:8765`, so every guard in `fm-serve` passes
  **by construction, zero lines changed**, and nothing listens on the phone (see the loopback trap
  in `known-issues.md`). Because `pixi run serve` runs `pnpm -C ui build`, `import.meta.env.PROD`
  is true — this exercises real `dispatch` and real `blob.rs`, **not `ui/src/lib/mock.ts`**.
  Record observations into `sessions/`: the single-column workspace, 2.75rem coarse-pointer
  targets, Board scroll-snap vs finger drag, the tap→move menu, Excalidraw under a finger, an
  inline image, and a `<video>` seeking mid-file — the only thing that exercises `blob.rs`'s
  `Range` path on real hardware. **This is the only gate that can retire `outstanding.md` §1.1**
  (unobserved UI, *"the largest single risk in the project"*), because Chrome touch emulation
  provably cannot reproduce Android's `dragstart` suppression. `adb` is outside pixi and that is
  acceptable — it enters neither the build graph nor the artifact — but say so rather than letting
  it pass as hermetic.
- **STEP 1 — `merge.rs`, desktop-only, no new deps. — ✅ SHIPPED 2026-07-19.** `merge_texts` is
  extracted and is now the one engine: text in, text out, callable from a driver, an in-process
  pull, or a platform with no `git` binary. `merge_files` is the `%O %A %B` driver ABI and
  nothing else. The whole-file fallback moved from a path-shaped, in-place `git merge-file` onto
  the same labelled text merge the body already used — which **removed a leak**: fallback
  conflict markers used to be labelled with real and temp *paths*, and are now `ours`/`base`/
  `theirs` like every other conflict (locked by a test asserting no `fm-merge-`/`.merge_file`/
  `/tmp/` reaches a note). The temp-file race is fixed too. **What remains before any engine can
  be swapped is STEP 2, the differential harness** — `text_3way` is now the single place that
  shells out, so it is exactly what a phone must replace, and it stays as the permanent oracle.
  The original text, for the record:
  `merge_texts(base, ours, theirs, marker_size)` from the path-shaped wrapper; keep `merge_files`
  as the thin driver-ABI shim (it is git's `%O %A %B` contract). **Fix the PID-only temp naming**
  (`merge.rs:192`): `blob.rs` already uses PID + `AtomicU64`, `merge.rs` uses PID alone and is
  safe today *only* because its sole caller is a one-shot subprocess. The moment merges run
  in-process in a threaded server, two concurrent merges write the same three `/tmp` paths and
  produce a note whose body came from **another note** — clean exit, silent. Required under every
  resolution including "wait" and including Path A. **No semantic change.**
- **STEP 2 — ✅ SHIPPED 2026-07-19.** `crates/fm-core/tests/merge_differential.rs`: fixed-seed
  xorshift (no `rand` dep — a *reproducible* failure matters more here than statistical quality),
  ~400 generated triples across marker sizes 7/12/32 and both line endings, asserting
  **byte-for-byte and verdict-for-verdict** against `git merge-file` invoked independently.
  Plain text rather than notes on purpose, so it exercises the engine and nothing else — no
  frontmatter rules, no scene merge, no fast paths. A second test covers the invariant the
  collaboration design actually promises: **a `Clean` note merge always parses.** Three things
  learned building it:
  - **The generator is the test.** The first version conflicted 378/400 times — which sounds
    like a stress test and is a worse one, because it barely reached the clean path, and the
    clean path is where a wrong engine loses content *silently* rather than marking it. Now
    ~50/50, with a third of cases one-sided (the commonest real shape: pulling work you have no
    local edits against).
  - **The balance is asserted** (`clean > 100 && conflicted > 100`). Without it a future
    generator change could make every other assertion vacuous and still pass green.
  - **Graded against today's engine first, deliberately.** With one engine it is nearly a
    tautology — that is the point: it proves the *harness* while a known-good answer still
    exists. A harness first exercised on the day it is needed is one nobody trusts.

  **This is now the gate on any engine swap**, and risk #2 (*"wrong markers = silent data
  loss"*) finally has one. Marker-byte divergence may be waived per-case, in writing, with the
  case recorded in the file — never by loosening an assertion until it passes. As planned:
- **STEP 2 (as planned) — the differential harness, green against *today's* engine first.** Property test over
  randomized `(base, ours, theirs)` — near-adjacent hunks, CRLF, marker sizes 1 and 255, markers
  left over from a prior bad merge — asserting **verdict identity** (clean vs conflict) and the
  parse invariant. Run it against the existing subprocess implementation *before* touching
  anything: that proves the harness, not the engine. **Risk 2 (*"wrong markers = silent data
  loss"*) currently has no gate at all**, because its only mitigation was parented to a refuted
  ruling.
- **STEP 3 — the skipped-note surface — ✅ SHIPPED 2026-07-19.** `ui/src/lib/SkippedPanel.svelte`,
  reached from a persistent toolbar chip (a banner is dismissible; this condition is not
  transient). `ReindexStats.skipped` became `Vec<SkippedNote>` — `{vault, name, path, reason}` —
  because the old formatted string said what was wrong and then made it impossible to act:
  no path could be recovered from it. Two things worth keeping in mind:
  - **The wire form omits the path.** The panel names the note back to a new `open_skipped`
    arm, which resolves the path from the indexer's *own current* skipped set. So the set is
    an **allowlist**, there is no path a caller can supply, and a panel left open across a fix
    fails closed rather than opening something else. Guard tested in `fm-app/tests/skipped.rs`
    (traversal, absolute, wrong-vault, already-fixed — none reach the OS).
  - **Opening in the OS editor is the only action, and that is not an apology.** The note does
    not parse, so no editor of ours can load it.
- **STEP 4 — ✅ SHIPPED 2026-07-19.** `git::clone` (new code, not a port — `git.rs` never had
  one), a `clone_vault` dispatch arm, and a **Start empty / Clone a shared one** toggle in
  `NewVault.svelte`, so the step ships visible UI rather than only Rust. Three design points
  worth keeping: `clone` calls `ensure_repo` **itself** rather than leaving it to the caller
  (the `merge.fm.driver` definition deliberately does not travel in a repo, so a clone that
  skips it silently falls back to git's text merge and conflicts on every `updated:` line —
  making that the caller's job is how it gets forgotten); the identity is **validated before
  anything is fetched**, so a mistyped email cannot leave a real repo on disk that is not a
  registered vault, in a directory too non-empty to retry into; and the identity is
  **required**, because a cloned vault has an audience by definition. Verified end-to-end
  against a live server, not only by unit test: their note arrives, is indexed and searchable,
  the committer is a real person, and a bad email refuses while writing nothing. As planned:
- **STEP 4 (as planned) — write `clone` in `git.rs`**, on the subprocess backend where it is testable today.
  `ensure_repo` → `set_identity` **before** the first commit, which finally makes the identity
  ruling real rather than theoretical. New `dispatch` arm beside `set_git_remote`, plus a
  "clone a vault" affordance in the desktop vault picker — so the step ships visible UI, not only
  Rust. Test: offline clone of a `file://` remote, asserting the first commit's author is **not**
  `PLACEHOLDER_EMAIL`. **Prerequisite on every backend.**
- **STEP 5 — ✅ SHIPPED 2026-07-19.** After a squashing push reports success, `push_squashed`
  now asks the **remote** — `ls-remote` against the branch it just pushed — instead of trusting
  the pusher's exit code. Three judgements inside it:
  - **Only when something was squashed.** With `squashed == 0` nothing was collapsed, so a
    false success costs an unpushed vault (which `backup_status` already surfaces) rather than
    lost history — not worth a round trip on the common path.
  - **A definite mismatch, and nothing else, rolls back.** If the remote cannot be asked, the
    answer is *unknown*, and unknown must not roll back: undoing a push that actually landed
    leaves local behind a remote that already has the work, and every later push is then
    rejected as divergent. **Failing to verify is not the same as failing to push** — that
    distinction is the whole safety of this check.
  - **It never fires today, on purpose.** Subprocess git's exit code is trustworthy. This is
    insurance for whatever replaces it, where "success" becomes our own parser's opinion and a
    false success would charge the user their granular undo for a backup that never happened.
  As planned:
- **STEP 5 (as planned) — harden `push_squashed`'s rollback.** It currently fires only on
  `!out.status.success()`. Make it independent of the client's own verdict — confirm the remote
  ref actually moved via a separate `ls-remote` — because with any in-process client "success"
  becomes our own parser's opinion, and a false success destroys granular history for a backup
  that never happened. Pays under every option; a precondition for any future push client.
- **STEP 6 — the record.** Reconcile this file, `plan.md`, `decisions.md`, `known-issues.md`
  (done 2026-07-19), and re-run the cold-read test at the bottom of `README.md`.
- **STEP 7 — body engine as an *experiment*, feature-gated, mobile-only.** Run the candidate
  against the STEP 2 harness and **publish the divergence list before deciding anything**. Treat
  any engine's "auto-resolved conflict" outcome as an automatic **stop** until someone
  demonstrates what content it discards — it is resolution-by-fiat baked into the engine, and
  neither mapping is safe (report `Conflicted` and you send the user to resolve a file with no
  markers; report `Clean` and you publish the fiat). Decide any new licence allowlist entry **up
  front, deliberately**.
- **STEP 8 — ◐ MOSTLY SHIPPED 2026-07-19. The core compiles for a phone.**
  `[feature.android]` + an `android` environment carrying the two conda-forge
  `rust-std-*-linux-android` targets; NDK **r27d** pinned by our own SHA-256 in
  `android/toolchain.lock` and fetched by `pixi run android-init`; the NDK toolchain wired
  through `[feature.android.activation.env]` with `$PIXI_PROJECT_ROOT` so every path stays
  repo-relative; `pixi run android-check` builds and **asserts the ELF machine of every
  vendored C blob** plus a `CC`-provenance guard that refuses a compiler from outside
  `.android/`. Verified: `fm-core` with `native-git` builds for `aarch64-linux-android`, and
  SQLite, libgit2, libcrypto and libssl are all `AArch64`.
  Two traps found by walking into them, both now in `known-issues.md`: conda's `c-compiler`
  activation exports host `CFLAGS` that the `cc` crate *appends to* rather than lets you
  override, so they must be emptied in the android env; and `git2`'s `https` needs
  `vendored-openssl` because Android has no system OpenSSL.
  **Still open:** `x86_64-linux-android` (the emulator target) is wired but unexercised, the
  SDK/cmdline-tools are not in the lock, and `pixi run ci` deliberately gains no NDK
  dependency — a contributor without the toolchain still gets a green gate.
- **STEP 9 — M0**, only after step 8 is green *and* the transport is decided.

**Not sequenced, deliberately: M8** (pure-Rust media extraction). Replacing mature `pdftotext`
and `vipsthumbnail` with niche crates is structurally the same move the `diffy` rejection
forbids. The divergence hazard it would close is real — identical PDF bytes yielding extracted
text on desktop and an empty body on a phone, both committed, both hitting the text merge — but
the minimal correct fix is two lines of policy: **declare the extractors as capabilities on the
heartbeat** (today `ingest.rs:101`'s `.ok()?` cannot tell a missing binary from a text-free PDF),
and **rule that a device without an extractor writes no body rather than an empty one** — *an
empty body merges cleanly into a lie.*

### The superseded spike list, kept because the reasoning teaches

- ~~(i) extract `fm_app::dispatch` and prove `fm-serve` still green over it~~ — **SHIPPED.**
- ~~(ii) unify `merge_files`'s body onto `git2::merge_file`~~ — **void:** that function does not
  exist at that layer. The *instruction* is dead; the *argument* it carried (do not reach for a
  niche crate on the one path that must never corrupt) stands and is why STEP 7 is gated.
- ~~(iii) a ~30-line binary that `git2`-clones over HTTPS+PAT~~ — **void with the backend.** What
  it was really trying to prove — the `CredentialSource` *trait shape* — survives, and STEP 4
  proves it more cheaply on the backend that already works.
- **M0 — Shell paints + lists notes (read-only, no git).** `cargo tauri android init`; native
  branch for read commands; a sandbox vault; open `Reindex::Incremental`; `rusqlite bundled` +
  libgit2 cross-compiled for `x86_64`/`aarch64-linux-android`. Inline images via the streaming
  blob route / `blob://` (ruling 7). **A non-painting WebView is a kill criterion** — fall back to
  server-on-device (ruling 2B) rather than fight it.
- **M1 — Clone a repo** (`git2` read + `CredentialSource` + **`set_identity` on clone**, ruling 5;
  PAT paste, ruling 4).
- **M2 — Edit + commit + open externally** (`update_body`/`set_property`/`capture`/`delete`;
  `git2 commit_all`; `open_external` via `tauri-plugin-opener`/FileProvider — the only way to view
  a PDF on Android, since the System WebView can't render PDFs inline).
- **M3 — Push + awareness** (`push_squashed`/`unpushed`; `backup_status`'s git half kept alive —
  the seamless nudge rides on `remote_moved`).
- **M4 — Pull + in-process frontmatter merge** through the same `merge_files`; the foreground
  refresh signal. **Two-device demo** (the acceptance test): A edits para 1, B edits para 2 → clean
  merge; a same-line edit → markers in the body, note still opens.
- **M5 — Multiple repos** (a second cloned vault; `MultiStore` federation; per-URL
  `CredentialSource`).
- **M6 — All views + collaboration surfaces** (single-column touch shell; **Board touch
  fallback**; `git::activity` via `git2` `revwalk`+tree-diff so EditedBy/Activity/contributor
  aren't dark).
- **M7 — Whiteboard across devices** (Ruling B image-strip **before editing** → `.excalidraw`
  merge → git-tracked whiteboard blobs).
- **M8 — Pure-Rust media extraction** (`pdf-extract`/`image`/`resvg`; desktop sheds
  poppler/libvips).
- **Seam extraction** folded into M1–M4 (`GitSyncProvider`, sole impl). **Desktop companion**
  (parallel, small): auto-pull on `remote_moved` + the explicit auto-push (ruling 8); desktop
  `CredentialSource` shelling `git credential fill`.

**MVP cut line (Path B) = end of M4:** clone/pull/edit/commit/push round-trips across two devices
with frontmatter-aware merge, external-open, and the sync nudge. M5–M8 complete the owner's full
stated scope (multiple repos, all views, collaboration surfaces, whiteboard, PDF-search/thumbs) —
the committed next band, each a code-anchored milestone, not "someday."

## Toolchain (Android-first)

Tauri v2 mobile (`cargo tauri android`, `cargo-mobile2`), Android SDK+NDK, JDK/Gradle, Rust
android targets (`x86_64` emulator + `aarch64` phone). **Honest hermeticity gap (the top
longevity risk, ruling-level):** pixi (conda-forge) can pin `rust`/`nodejs`/`pnpm`/`c-compiler`/
`openjdk`, but the **Android SDK/NDK are not cleanly conda-packaged** — so this bolts a non-pixi
provisioning step onto the one thing that gives the project reproducibility, violating the hard
"pixi is the only package manager" house rule.

**Narrowed twice since this was written (2026-07-19).** First: the gap is **NDK + SDK +
platform-tools only** — `openjdk`, `gradle` and all four `rust-std-*-linux-android` targets are
conda-forge-native, so most of the list above is already pixi-pinned. Second, and the load-bearing
part: *outside pixi* must not become *outside the project*. Every piece Google ships is a
**standalone zip, not an installer**, so all of it unpacks into a gitignored **`.android/`** in
the tree, fetched by `pixi run android-init` against a committed `android/toolchain.lock` that
pins each artifact by **our own SHA-256**. Nothing is installed system-wide, so nothing needs
uninstalling, two checkouts can differ, and the thing the pixi rule actually protects —
reproducibility the repo can assert — survives. A checkout that says `sudo apt install` has moved
the dependency somewhere the repo cannot pin or remove, which fails the rule by another route
(`decisions.md`, owner's ruling 3). `ANDROID_HOME`/`ANDROID_NDK_HOME` point *into* `.android/`;
`sdkmanager` and whatever it resolves stay quarantined in an opt-in environment declared
non-hermetic.

**Device access is subject to the same rule.** USB `adb` on Linux wants udev rules or `plugdev`
membership — a system requirement — so the documented path is **Android 11+ wireless debugging**
(`adb pair` with a code over Wi-Fi): no USB, no udev, no sudo. `adb reverse` is
transport-agnostic, so the loopback tunnel works identically.

Pin the rest of the matrix — AGP/Gradle/`cargo-mobile2` — as a known-good set, and note that
**"pin it in CI" names no executor here**: every workflow is `workflow_dispatch`-only by standing
order, so the lock file and `pixi run ci` are the controls that actually run. Ship per-ABI split
APKs (arm64-v8a release, x86_64 emulator/CI). `pixi.lock` alone will not give a hermetic 2031
Android build; `pixi.lock` **plus** `android/toolchain.lock` is the honest claim.

## Risks (re-weighted)

1. **Toolchain/NDK hermeticity outside pixi** — the decade-scale maintenance sink; pin the whole
   matrix, resist the `targetSdk` treadmill (Play Store deferred).
2. **In-process merge fidelity (M4)** — wrong markers = silent data loss; the differential test
   (ruling 3) gates it.
3. **`fm-cli`/`fm-app` already forked + the "third dispatch surface"** — ruling 1 removes it;
   track that the extraction actually lands before the bridge.
4. **Tauri-mobile single-vendor bet, no browser fallback** — M0 paint kill criterion; ruling 2B
   is the retreat.
5. **git2 push over HTTPS+token / cross-compiling libgit2+rusqlite for Android** — spike (iii) +
   an `aarch64` compile check.
6. **Mobile committer identity** (ruling 5) — verify a phone commit shows the real author, not
   the placeholder.
7. **`.excalidraw` merge** (z-order/deletion-intent) — host-side unit test before on-device.
8. **Android background/large-repo limits** — foreground service; shallow clone + metered-data
   warning; don't commit attachments into the notes repo.
