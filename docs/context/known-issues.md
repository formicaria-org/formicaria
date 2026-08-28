# Known issues, gaps & traps — what is *not* working

Honest status of rough edges, deferred work, and things that will bite you.
Keep this current: when you fix something, delete its entry; when you hit a new
trap, add one. Newest concerns first within each section.

## Study assistant — current gaps (2026-07-22; agent shipped on-device, see overview.md)

- **`/research` on the phone covers 3 sources, not the general web (yet).** Mobile web search was
  wired 2026-07-24 (`crates/fm-agent-run/src/websearch.rs`, `DirectSearch`): the phone can't run the
  desktop's `search-proxy.py`, so it does the HTTPS in-process (`ureq`/rustls) over three stable
  keyless APIs — **Wikipedia · arXiv · GitHub** — text-only, fail-soft per source. `agent.rs` sets
  `web_direct: true`. **Deliberately omitted: DuckDuckGo (general web)** — it needs brittle HTML
  scraping and blocks scrapers; adding it is the remaining step. (Desktop still uses the local proxy:
  `pixi run search-proxy` + `agent-serve --searxng-port 8888`, which does include DDG.)
- **No stop button.** A thinking turn can't be cancelled from the UI (discussions or note threads) —
  deferred; needs a cancel signal threaded to the in-flight model call.
- **RAG is one hop only** (host note + its `note:`-linked notes, text only). Deliberate: vault-wide
  retrieval fed a tiny model unrelated fragments it parroted. Broader RAG is deferred pending its own
  research (owner: bad RAG is worse than none).
- **Multi-round proposal refinement — context notes** (surfaced 2026-07-22 while testing the
  propose→refine→accept cycle; mechanics proven in `fm-app/tests/proposal_cycle.rs`):
  1. **The agent can't see its own previous draft** (still open, deliberately — kept out for
     simplicity). Context is the *current* host note (still the pre-proposal text until accept) + linked
     notes + discussion; the previous proposal's body lives on a branch, not in any of those. So each
     round **re-generates from prose feedback** rather than *editing* the last draft — a weak model can
     drop things it got right before. Fix when we act on it: feed the open proposal's body back in as
     "your current draft." Larger change (new vault query for the host's open proposal), so not bundled
     with the history fix.
  - **Cross-device angle (owner, 2026-07-22):** the cycle is fully agent-/device-agnostic *mechanically*
    (a round can be qwen on the laptop, the next lfm2.5 on the phone — proven by the cross-device
    assertions in `a_user_iterates_with_the_agent_over_several_rounds_before_accepting`). But the phone
    model has a **smaller context window and weaker instruction-following**, so a thread that refines
    cleanly on the laptop can still regress mid-cycle on the phone, and alternating models can mix
    formatting styles. `history_budget`/`ctx` are per-model in `models.toml`; the both-ends pin above
    matters most on the phone's tighter budget.
- **Deleting a first-class discussion root orphans its replies** (they keep a `thread_of` to a gone
  note). Pre-existing; per-message Delete now exists but there's no "delete the whole thread".
- **`pixi run build` does NOT rebuild `agent-serve`** (only fm-serve + UI). After changing the agent,
  `cargo build --release -p fm-agent-run`, or the desktop keeps running the old agent binary.
- **Math normalisation is a blunt delimiter replace** (`\[`→`$$` etc.) on the agent's reply — fine for
  model output, but would mis-convert a literal escaped `\[` if a model ever emitted prose brackets.
- **Mobile foreground-service / jniLibs / manifest edits live in generated `gen/android/`** (gitignored);
  they are re-applied by committed scripts (`ci/android-{stage-runtime,inject-service}.sh`) on each
  build, not stored in git — a fresh `tauri android init` needs those scripts re-run.
- **The launcher's stale-check includes `ui/dist`**, but fm-serve serves the *embedded* UI, so a
  `ui/dist`-only change (without rebuilding fm-serve) triggers a restart that changes nothing. Harmless;
  the fm-serve binary mtime is the real UI signal.

_Last verified: 2026-07-22 — the **study assistant shipped on-device on both platforms** (see the
"Study assistant — current gaps" section above and `overview.md`); the **"the agent goes out when
formicaria does" teardown is now unit-tested** on both seams (watch-loop `!alive()→stop_model`, and the
shared `PR_SET_PDEATHSIG` backstop moved into `SupervisedModel::launch` — see
`sessions/2026-07-22-lifecycle-ci-tests.md`). Prior: 2026-07-20 (evening) —
**four defects found by an adversarial review of this repo's own code were fixed**, and a second adversarial pass over those fixes found four of them
incomplete; see `sessions/2026-07-20-four-bugs-a-review-found.md`. The durable lessons are in
`decisions.md`. Also fixed: **`verify` was checking a hardcoded `notes/`**, so a Track-V vault whose
`vault.json` puts notes in `docs/` got "verified 0 note(s): 0 error(s)" and exit 0 — an
integrity checker passing a vault it never opened. `inspect_path` had the same hardcode and
would preview such a folder as empty. Both now ask `Descriptor::notes_dir`, as `backup` always
did. Standing traps that came out of it: a `copy_note`-style "strip user text" fix
must enumerate the **typed** `Object` fields (`status`, `tags`) and not only `extra`, because a
map-shaped fix silently misses them; and a git merge driver must **never exit non-zero**, since
git reads that as a conflict while leaving the file clean.

Prior: 2026-07-20 — **media on Android works end to end**: attach a file, it lands in the
vault, the note renders it (`sessions/2026-07-20-capture-on-a-real-phone.md`). The bug that hid
this for days was that **Android delivers no request body to a custom-scheme handler**, so every
photo was stored as zero bytes while ingest reported success; the giveaway was that every capture
produced the same reference — the SHA-256 of the empty string. Ingest now goes through the
`fm_ingest` IPC command as base64, which is the only binary transport the platform leaves open.
Prior: 2026-07-19, the Android byte path for media
(`sessions/2026-07-19-media-on-the-phone.md`) — blobs stream over a `fmblob://` protocol handler,
because a phone has no HTTP server and media worked in neither direction before. Prior: after git-over-HTTPS started working on Android
(`sessions/2026-07-19-git-on-the-phone.md`): the trust store is loaded into libgit2 from memory,
because `openssl-src` builds every Android target with `no-stdio` and no file-based certificate
loading can work there at all. Prior: multi-method vault acquisition and the stale-merge-driver fix
(`sessions/2026-07-19-acquiring-a-vault.md`). Prior: 2026-07-18, after the dispatch extraction,
the blob route, the sync loop and the
scene merge (`sessions/2026-07-18-dispatch-and-blob-route.md`,
`sessions/2026-07-18-sync-loop-and-scene-merge.md`). Prior: the mobile-port code audit
(`sessions/2026-07-18-mobile-port-plan.md`); 2026-07-16 (assets/status/kanban/slash-menu/
edit-gesture)._

> **Looking for what to work on?** This file is the honest *description* of rough edges,
> including ones we have decided to live with. The ranked **work queue** — with what "done"
> looks like for each — is [outstanding.md](./outstanding.md).

## Android startup/shutdown — what two adversarial audits found and this pass did NOT fix (2026-07-31)

The gray-screen fix and its tests are in
`sessions/2026-07-31-the-gray-screen-on-first-open.md` and `decisions.md#track-m`. These are the
*other* findings from auditing the launch and teardown paths — all anchored, none fixed:

- **A kill mid-`pull` leaves a vault mid-merge — now recoverable from the app, which it was not.**
  Fixed 2026-07-31 (`decisions.md#git`): both backends' `commit_all` refuse while anything is unmerged
  and, once the index is settled, commit the merge with **both parents** and call `cleanup_state()`.
  Previously `cleanup_state` had exactly one call site and `commit_all` never inspected `repo.state()`,
  so a resolution committed with the second parent silently dropped and `push_squashed` /
  `merge_proposal_branch` refused **forever** — reached by a `SIGKILL` with no user action at all, and
  on Android a `SIGKILL` is how the app is normally closed.
  **What remains:** the window *inside* `pull` (between `repo.merge()` and its commit) still leaves
  `MERGE_HEAD` on a hard kill — deliberately, exactly as an interrupted `git merge` does. The
  difference is that the conflict surface now finds it, explains it, and can settle it.
- **The phone's `fmblob` handler buffers whole blobs and honours no `Range`**
  (`mobile/src-tauri/src/lib.rs`, the `blob_response` arm). Ruling 7's streaming/seeking path exists
  only in `fm-serve/src/blob.rs`; the handler's own doc comment claims otherwise. On a device already
  swapping 3 GB, opening a large blob is a multi-hundred-MB transient — i.e. an invitation to the
  renderer kill described below.
- **The model fetcher verifies neither length nor hash** (`fm-agent-run/src/fetch.rs:111-149`;
  `agents/models.toml` pins no `sha256`, so both verification branches are dead code). A body that
  closes early is renamed as complete, and `ensure_model` then returns that file forever — nothing in
  the tree ever deletes a bad `.gguf`. The only self-heal is uninstalling the app, which also
  destroys the vault.
- **A poisoned vault mutex bricks every command for the life of the process**
  (`fm-app/src/dispatch.rs:186`, `lock().map_err(...)` with no recovery). The agent thread and the
  webview both `dispatch`, so a panic in either poisons for both.
- **`activity` holds the vault mutex across a per-vault libgit2 revwalk**, and it is among the first
  things the first `refresh()` fires (`fm-app/src/dispatch.rs`, the `activity` arm; `App.svelte`
  fires it before awaiting the feeds). Every other command queues behind a year of git history. This
  is the arm the module's own "five arms drop the lock before slow I/O" note does not cover.
- **Android reclaiming the WebView renderer kills the app, silently**, because `RustWebViewClient`
  has no `onRenderProcessGone` override and the framework default is to kill the process. So a
  memory-tight phone can make formicaria vanish with nothing saying why. **A logging override cannot
  be injected from `ci/android-inject-service.sh`** — tried 2026-07-31: anything under `gen/android/
  .../generated/` is rewritten by tauri's own codegen *during* `tauri android build`, i.e. after the
  pre-build hook has run, so the patch reappeared and vanished on every build with no error anywhere
  (`AgentService.kt`/`MainActivity.kt` are patchable only because they live outside `generated/`).
  Needs a Gradle source-transform or an upstream hook; not worth it for a log line, but **do not
  spend the afternoon rediscovering why the awk block "didn't work"**.
- **Turning the assistant off leaves its foreground notification up.** `agent::set_running(false)`
  kills the model but nothing calls `stopSelf`/`stopForeground`, so "formicaria study assistant —
  Running on your device" persists over an app that is running nothing. (The worse half of this — a
  `START_STICKY` service resurrecting a *hollow* process with no Rust in it at all — was fixed.)
- **The `unsafe set_var` calls' stated justification is false.** `mobile/src-tauri/src/lib.rs`'s
  `configure_paths` and `fm-app/src/secrets.rs` both say "single-threaded, before any vault is
  opened"; the webview is already loaded and invoking by then, and bionic's `setenv` is not
  thread-safe. The window is small and the fix is not obvious — but do not trust the comment.
- **A `configure_paths` early return produces a *wrong* screen, not a blank one.** If
  `app_data_dir()` fails, `FM_CONFIG_DIR`/`FM_VAULT` stay unset, `App::load` **succeeds** with zero
  vaults, and the user is shown the first-run form — whose `create_vault` then cannot persist
  anything.

## Known gaps / not fully working

- **A paired device's microphone needs the certificate installed, and there is deliberately no
  way around that.** `getUserMedia` requires a secure context, so it works over the TLS listener
  and not over the plain-HTTP fallback (`--no-default-features`, or a machine where no
  certificate could be minted). **No click-through path is offered** — see `decisions.md`'s TLS
  exception for why. **Photo/video capture never needed any of this**: it is a plain
  `<input type="file" capture="environment">`, a picker rather than `getUserMedia`. `/transcribe`
  also treats an *attached* clip identically to a recorded one, so the fallback is real.
- **Nothing verifies the certificate for the user.** There is no CA, no OCSP, no CRL — trust is
  the human comparing the SHA-256 fingerprint Settings prints against what the device shows,
  once. Consequently **"Disconnect all devices" revokes tokens, not trust**: a device that
  installed the certificate still trusts this computer until the profile is deleted there.
- **The share cookie is not port-scoped, because no cookie is** (RFC 6265). A token set on the
  share port is sent to every port on that host. `Secure` keeps it off plain-http listeners
  entirely, which is most of the exposure; what remains is another *https* service on the same
  machine. `__Host-` would not fix ports either — nothing does.
- **A blob hash is still an existence oracle across the boundary.** The Origin check is POST-only,
  so a page that already knows a sha256 can put it in an `<img src>` and learn from
  `onload`/`onerror` whether this machine holds it. It cannot read the bytes (no CORS headers, and
  do not add any). Known and accepted: the hash has to be known first, and the alternative is
  authenticating subresource loads, which cookies are the only mechanism for.
- **A refused cross-vault write answers `500`, not `403`.** `api()` maps every `dispatch` error to
  500, so "no vault named 'personal'" arrives with the wrong status. The *message* is right and
  is deliberately identical to a nonexistent vault (a caller must not be able to probe for names),
  but the status code is sloppy. Fixing it means classifying `dispatch`'s error type, which is a
  larger change than this feature.
- **Sharing applies at the next launch**, deliberately (the `agent.rs` precedent) — starting and
  stopping a listener under live connections buys nothing a reopen does not. The Settings toggle
  says so; if it ever stops saying so, that is the bug.
- **Android can carry bytes to the app only as base64 in a JSON string**, and that is a platform
  limit with no workaround at this layer. Both binary doors are shut: Tauri states *"On Android,
  `InvokeBody::Raw` is not supported"*, and wry intercepts through
  `WebViewClient.shouldInterceptRequest(view, request: WebResourceRequest)` — Android's
  `WebResourceRequest` has **no body accessor**, so a POST to a custom scheme arrives with its
  body silently dropped. That silence stored every phone photo as zero bytes for days. Ingest
  therefore goes through `fm_ingest` with the file base64-encoded, capped at `MAX_INGEST`
  (**16 MB since 2026-08-20**, down from 48 — see `decisions.md#track-m`: the old number counted
  the string and not the ~10 copies the transport makes, so it permitted a ~600 MB transient and
  an unexplained renderer kill) — above any phone photo, below video. **Video on Android is
  refused with an explanation**; chunked ingest would lift it and is **still not built**. It is now
  the *named* next step rather than a vague one: `fm_ingest_chunk` + `fm_ingest_finish` over
  `BlobStore::put_file`, which already streams and hashes in 64 KB chunks, with `commands::
  asset_note` already factored out for exactly this second byte-arrival path. Bounds the transient
  at one chunk regardless of file size. The one new hazard is orphan sessions after a `SIGKILL`
  mid-upload — sweep the session dir at boot.
  **Both ends of the bridge are now tested** (they were not, which is how the zero-byte photo
  shipped): `fm-app/src/wire.rs` for the decoder, `ui/src/lib/ingest.phone.test.ts` for the
  encoder and the ceiling, `fm-app/tests/mobile_workload.rs` for a real multi-MB photo.

- **The read view never asks for a thumbnail — on any platform.** Easy to mis-diagnose as an
  Android gap, and it is not. `ingest::thumbnail` shells `vipsthumbnail` and writes
  `derived/<hash>/thumb.webp`, but the only readers are `fm-cli` and `asset_status`'s `has_thumb`
  flag; `NotePanel`'s resolver uses `assetUrl(ref)`, which has no `kind` parameter at all, so every
  inline image is the **full blob** on desktop and phone alike. The Gallery view that used to
  consume thumbnails was removed. So a note with five 12 MP photos decodes ~200 MB of bitmap
  everywhere — the desktop simply has the memory to survive it.
  **Do not "fix" this by passing `kind: "thumb"` from the phone.** `commands.rs`'s `resolve_asset`
  hard-errors on a missing thumb (there is no fallback to the full blob), and `vipsthumbnail` does
  not exist on Android — so every image in every note would become a 404 placeholder. The real fix
  is a downscale path that works on both platforms, which is M8 (pure-Rust media extraction),
  deliberately unsequenced in `mobile-design.md`. Mitigated 2026-08-20 with `loading="lazy"` +
  `decoding="async"` in `render.ts`, so off-screen images cost nothing; the on-screen ones still
  decode at full camera resolution.

- **The owner's phone has no diagnostic channel except the app's own UI.** `eprintln!`/stdout
  never reaches logcat from a Tauri Android shell, and — found the hard way on 2026-07-20 — the
  WebView routes **no `console.*` output there either**: a signed, installed, MD5-verified build
  full of `console.warn` produced zero lines while the native `ca-bundle:` log from the same run
  came through fine. Anything you need to read off that device must be rendered on screen. This
  cost a full build/sign/install/ask-the-owner round trip.

- **A phone vault holds the only copy of its media.** App-private storage is wiped on uninstall,
  `blobs/` is gitignored so a push does not carry it, and restic — the one thing that does — is a
  binary Android does not have. Losing notes is bad; losing the only copy of a photo is worse, and
  capture makes that materially more likely. No answer yet.

- **`env(safe-area-inset-*)` is 0 in the Android WebView, so every full-screen surface needs a floor.**
  wry does not forward the window insets into the page, and Android 15 forces edge-to-edge — so a
  full-screen panel paints *under* the status bar and the navigation bar. `app.css` floors the top inset
  on coarse pointers; the bottom floor is 0.5rem, which clears a gesture pill but **not a 3-button
  navigation bar**. Found on the owner's phone 2026-07-31: `UnrecordedPanel`'s primary button was half
  under the system bar and the line below it invisible, which one screenshot showed and no test could.
  Each full-screen surface therefore floors its own bottom padding (see that panel) rather than raising
  the global and shifting every other surface sight-unseen. **The real fix is named and unbuilt**: the
  shell should read `WindowInsetsCompat` and hand the values to the page, the way it hands over
  `FM_CONFIG_DIR` — a platform fact only the platform has.
- **Every narrow-layout CSS rule in `App.svelte` must be written twice.** Once for
  `[data-layout='single']` and once inside `@media (max-width: 60rem)` for `[data-layout='auto']`,
  because `auto` is the default and a phone therefore never matches a `single` rule. Writing only
  one half is **silent** — it shipped twice on 2026-07-19. There is no way to express "narrow
  right now" in one place without viewport-tracking TypeScript, which the layout design
  deliberately avoids.

- **The Android git token is app-private storage, not the Keystore.** The owner chose
  hardware-backed; what shipped is a 0600 file in the app's private directory. The kernel
  isolates it per-UID so no other app can read it — genuinely stronger than the plaintext
  `~/.git-credentials` that `credential.helper store` leaves on a Linux desktop — but it does
  not survive root, does not stop someone holding the unlocked phone, and is not hardware-backed.
  Closing it needs a Tauri Android plugin with a Kotlin/JNI layer this repo does not have.
  Mitigated meanwhile by the UI telling the user to scope a fine-grained token to one repo with
  an expiry, which is what actually bounds a leak (a PAT is a bearer token, **not** device-bound).

- **A gesture control must not remember what you did last.** The pane-header wheel rotator shipped
  two direction-dependent bugs within an hour on 2026-07-20, and both were the same mistake: what
  a step *cost* depended on history rather than on the gesture in front of it. First a blocked
  accumulator was *held* at the threshold, pre-charging the direction you were already going;
  then a cooldown that a **reversal cleared and continuing did not**, so after any step one way
  was throttled and the other fired instantly. Each passed a single-example test while being
  visibly lopsided in use. It now has no direction memory at all — a detent turns one view
  immediately either way, and only smooth scrolling accumulates. Pinned by a test that drives an
  arbitrary lopsided sequence of notches and requires the view to land exactly where the
  arithmetic says, which no direction-dependent rule can satisfy.

- **A round button needs both axes set, and the coarse-pointer rule only sets one.**
  `@media (pointer: coarse)` gives toolbar controls `min-height: 2.75rem` and horizontal padding
  — right for a pill-shaped chip, wrong for a circle, whose width comes from its own rule. The
  red plus shipped to the emulator as a visible ellipse on 2026-07-20. `svelte-check` cannot see
  this; one screenshot can.

- **Narrow layouts hide every `.icon-btn` in the top bar** (`[data-layout='single']` *and* the
  `auto` media query), because those controls live in the bottom `ViewBar` where the thumb is.
  Reusing that class for anything that must stay visible on a phone makes it silently vanish
  there — nearly shipped for the collapsed search button on 2026-07-20.

- **"Save token" is a separate action from "Join".** Typing a token and pressing Join silently
  discards it; the token only reaches the backend via its own button. Reported from real use.

- **Diagnostics cannot be read from a Xiaomi/MIUI device.** Rust's stderr is not routed to logcat
  on Android at all, and MIUI suppresses app logcat output besides — `android_logger` output is
  visible on the emulator and silent on the owner's phone. Anything that must be diagnosable on a
  real device has to surface in the app's own UI (which is why `config` reports `ca_bundle`).

- **Two transport tiers, one warning, two meanings — decide before the next bulk transport.**
  `notes/` travels by git and `blobs/` out-of-band, so `verify` grades a referenced-but-missing
  blob a *Warning* because it is expected to be transient. A transport that moves the directory
  **whole** inverts that: those vaults have atomic note/blob completeness, and the same warning
  would mean "permanently lost". `vault.json` (`descriptor.rs`) has no field recording which
  kind a vault is. Not urgent — restic acquisition does not create this population, because it
  restores the same two tiers separately — but it lands the moment rclone/p2p/zip does.

- **No "Send a copy", and no auth UI.** Acquisition is in (git clone, restic restore); the
  outbound half is only `git push`. Deferred because the honest non-git form today is "write a
  folder or a zip", which the OS already does. Auth is delegated to each tool's own store, which
  holds on desktop — **but on Android none of those stores exist** (no terminal, no credential
  helper, no `rclone config`), so the phone forces a real Keystore decision the moment a second
  transport needs a secret.

- **Two conflict traps, both fixed 2026-07-31 — the lessons, because the shape recurs.**
  A vault sat **mid-merge for 7 days** on one `DU` ("deleted by us") path — a template deleted on
  the laptop and edited on the phone — with **95 new notes never committed**, because `commit_all`
  refuses while a vault is mid-merge. Two independent defects, and the pairing is what made it
  costly:
  1. The app held **two different definitions of "conflict"**. `commit` asked git for unmerged
     paths (all seven codes), while the list the UI showed was derived by **scanning note bodies for
     `<<<<<<<`**. A delete/modify conflict has no markers and never will — one side has no blob, so
     the `.md` driver is not even called — so the app warned about a conflict it could not show, and
     every message said *"open each one, both versions are marked in the text"*, which for that kind
     is advice nobody can follow. Git's own resolutions (`add` = theirs, `rm` = ours) had **no UI at
     all**, and the owner has no terminal. Now `vcs::conflicted` carries the kind, `resolve_conflict`
     keeps a side *and finishes the merge*, and the Collaboration surface offers both.
  2. **`commit_all` could permanently skip a note, not merely lag.** It stages exactly the paths
     `FileStore::put`/`delete` recorded — correct, so that a vault which is also a project repo never
     has its owner's staged work swept into an `auto:` commit — but that record is **per-process
     memory**. Every note written before the last restart was unstageable *forever*, and nothing said
     so. Now `vcs::unrecorded` lists them and a toolbar chip counts them.
  **The durable lessons:** a surface derived from a *symptom* (markers in text) will silently miss
  every case that lacks the symptom, so derive it from the *authority* (git) and keep the symptom as
  the union; and "history can lag" is a very different claim from "history can be skipped" — the
  second one needs a visible count, because nobody audits git by hand.
  3. A third defect, found the same day by the new differential test and worse than both: **editing a
     note resolves nothing as far as git is concerned** (all three index stages remain), so the
     documented resolution settled nothing, and the note left the UI as soon as the markers were
     tidied. Both backends' `commit_all` were also broken on that path in opposite directions — the
     phone silently committing markers as note content, the laptop erroring with *"cannot do a partial
     commit during a merge"*. Fixed; see `decisions.md#git`, "A backend that cannot finish a merge must
     refuse to commit".
  **Still true:** resolving a marker conflict by *editing* remains the only way to keep both sides;
  `keep theirs`/`keep mine` on a `UU` would discard one, which is why the UI offers **Open the note**
  + **Mark resolved** there instead.
  **And the gate to remember:** a compiler that checks signatures says nothing about behaviour, and
  the suite that catches backend divergence (`pixi run test-native-git`) is **opt-in** — run it
  whenever `commit_all` or `git_native.rs` changes.
- **A conflicted note is surfaced only by name** (current behaviour, not a bug): the `.md` driver
  puts the markers in the note *body*, so it opens and resolves in the ordinary editor. `commit`
  answers `CommitResult { committed, conflicts }` and all three callers (the 5 s auto-commit in
  `App.svelte`, `sync.svelte.ts`, `BackupPanel.svelte`) **stop and name the notes** rather than
  silently freezing every commit in a mid-merge vault — the trap being that `commit_all` once
  reported "clean tree" and "I refuse, mid-merge" with the same `Ok(false)`.
- **Reindex still stats every file, on every beat.** `Reindex::Incremental` re-*reads* only
  what moved (Phase 1), but the scan itself is still O(n) `stat`s, and the `ping` heartbeat
  runs it on every beat (15 s, and only while the tab is visible). Now gated by a perf-budget test at 10k notes
  (`fm-core/tests/perf.rs`) — which is what caught the deletion sweep being O(n²) against a
  `Vec` (453 ms → 50 ms per quiet beat, 2026-07-18). Before reaching for a watcher (inotify)
  note that a watcher is a dependency and a per-platform behaviour, which is why polling won
  on the way in.
  **The cold-start half of this is done (2026-08-20), on Android only.** `FileStore::open` is still
  a full rebuild and remains the default everywhere; the phone opens through
  `open_incremental`/`ColdStart::TrustIndex` (see `decisions.md#track-m`). The three
  preconditions this entry named are met: a schema gate (`PRAGMA user_version` vs `INDEX_SCHEMA`,
  doubling as a completion marker written *after* the reindex commits — so marker-present implies
  rows-durable, which is what matters when `SIGKILL` leaves no shutdown hook), the
  `objects(path)` index (which already existed) **plus** the `fts_rowid` seek that removed the real
  cause of "Incremental can be slower than Full" — `forget_path` and `index_object` were both
  scanning the whole FTS table, so incremental was O(2·k·n) — and cold-start tests, now
  `fm-core/tests/cold_start.rs`.
  **What is unchanged is the hazard**: mtime-only detection is blind to every mtime-preserving
  writer (`restic restore`, `rsync -a`, `cp -p`, `tar -x`), which is exactly why this is a
  frontend claim and not a default. `cold_start.rs` asserts that divergence as a *failing* case on
  purpose. Do not extend `TrustIndex` to the desktop.
- **A stale editor can no longer overwrite a merge — but only where `updated` moves.**
  Fixed 2026-07-18: `update_body` takes the **version** the caller last saw — the sha256 of
  the body it loaded, carried on `NoteDetail.version` — and returns the new one; a mismatch is
  `StoreError::Conflict`, and `NotePanel` reloads and puts the unsaved draft back *below* the
  merged text with markers rather than discarding it.
  **Why the existing mtime guard could not cover this:** `FileStore::put` refuses a write
  whose file moved since we indexed it — but `pull` merges and then *reindexes* (a merge is
  invisible until it does), which records the post-merge mtime and stands the guard down
  exactly when it was needed. The staleness lives in the client, so the client declares its
  base.
  **The residual gap is closed too** (2026-07-18): the token is the **sha256 of the body**,
  not `updated`, so an edit that never bumps a timestamp — Vim — still moves it. Measured at
  1.6 ms in release for a 2.8 MB whiteboard body, against a 600 ms save debounce, and pinned
  by a perf budget.
- **On Android the app is normally left by being killed — so "commits can lag" is worse there than
  the desktop framing says.** Swipe-away and an LMKD reclaim are a `SIGKILL`: no unwinding, no
  destructors, no flush. *Back* can do the same — `TauriActivity` sets `handleBackNavigation = false`,
  so a Back that reaches the Activity finishes it and tao's event loop then calls
  `std::process::exit` — **but measured on the owner's phone 2026-07-31 it did not**: this frontend
  pushes history (`NotePanel` handles `onpopstate`), so the WebView had somewhere to go back to and
  absorbed the key. Treat Back as *may* exit, depending on the page's history; the kills that always
  happen are the swipe and the reclaim. The 500 ms save debounce and the 5 s auto-commit
  are browser `setTimeout`s, so *the ordinary way of leaving the app* can skip both. Since 2026-07-31
  `pagehide`/`visibilitychange` flush them — **best-effort only**: a WebView is not guaranteed to
  deliver either event before the Activity is torn down, and nothing in wry/tauri wires Android's
  `onPause` to the page. Files themselves are never at risk (atomic temp+rename); up to 500 ms of
  *typed text* and a pending commit are.
  **Weakened slightly on 2026-08-20, deliberately.** The flush calls `save()` without awaiting it,
  and while the IPC commands were *blocking* that made the write effectively synchronous —
  `postMessage` did not return until the temp+rename+`sync_all` had completed. Now that the
  commands are `#[tauri::command(async)]` (they had to be; see `decisions.md#track-m`), the write
  starts immediately on a worker instead of finishing inline. Sub-millisecond, and a clear net win
  against removing multi-second freezes from every command — but it *is* a change to what this
  entry promised, and it is written down rather than discovered.
- **The phone shell has never run on a phone.** The reflow and the tap→move menu are verified
  at a narrow viewport, by `pointer: coarse`, and by component tests — not on a device. Chrome's
  touch emulation is *actively misleading* here: it synthesises PointerEvents but does not
  reproduce Android's `dragstart` suppression, so emulation can hide the very bug the menu
  exists to work around. One real device is needed once, to confirm card drag is genuinely dead
  there, the menu is reachable, and the targets are hittable.

- **The suite could not see the phone at all, until 2026-08-20 — and that is why four weeks of
  unusable app produced no red test.** Worth keeping as a *shape of gap*, because it will recur
  for the next platform. Four separate things each made the mobile path unreachable from CI, and
  none of them looked like a hole:
  1. `ipc.ts` chose its backend from a module-scope `const isTauri`, so no test could ever take the
     Tauri branch (it is `isPhone()` in `platform.ts` now, read at call time).
  2. jsdom reports a fine pointer, so every `(pointer: coarse)` branch was dead code
     (`test-setup.ts` supplies a settable stub).
  3. `mock.ts`'s `ingest` hashed the **filename** and discarded the bytes, so no byte-path defect
     could be observed — the zero-byte-photo class of bug was structurally invisible.
  4. `mobile/src-tauri` is workspace-excluded, so `b64_decode` — which every photo passes
     through — had never been compiled by a test (the pure half now lives in `fm_app::wire`).
  Plus two smaller ones found on the way: `asyncUtilTimeout` equalled `testTimeout`, so a slow
  query failed as an opaque *"Test timed out"* with no element name; and the mock's fixture dates
  were literals, so `App.flow.test.ts` went red when the calendar rolled past them.
  **The lesson to carry:** none of these produced a *failing* test. They produced an absence, and
  an absence looks exactly like coverage. When a platform's defects are not reproducing, check
  first whether any test executes that platform's branches at all.

- **Double-click-to-edit could splice an insertion *inside* a reference, corrupting the note.**
  Found on the owner's phone 2026-08-20, in real data: a note had been rewritten to
  `asset:sha256-a52bb![JPEG_….jpg](asset:sha256-cd63457f…)` — a second image reference spliced five
  characters into the first one's hash. The app logged `asset_status: parse error: not an asset
  reference` on every startup and the image was permanently broken, with nothing on screen saying
  why.
  **Mechanism:** `clickedOffset` (`NotePanel.svelte`) maps a double-clicked word to a source offset
  by *ordinal* — count the word in the rendered text, find that occurrence in the source. `locate.ts`
  always said that answer is a hint, but the caller used it as a caret. Rendered and source text
  disagree by much more than "syntax the reader never sees": an image's alt text is an attribute and
  contributes nothing to `textContent` **unless the blob is missing**, when the placeholder renders
  it as words; a `note:` chip renders a title where the source has an id; an embed renders a whole
  other note's body. So the drift depends on which blobs happen to be present.
  **Fixed** by `locate.ts::outsideDestination` — the caret is pushed out of any `](…)` destination
  before it is used. That is a floor, not a mapping: the caret can still be a few words off, which
  is an annoyance, where landing inside a reference was silent data loss. **An exact mapping needs
  source positions threaded through `marked` and `extractMath`** and is still not built.
  **The existing corrupted note is not repaired by this** — the fix prevents recurrence only.

- **Trap: `DELETE … WHERE rowid IN (subquery)` is a full scan on an fts5 table.** SQLite pushes a
  `rowid = ?` constraint down to a virtual table's `xBestIndex`, which fts5 answers as a lookup; it
  does **not** push down `rowid IN (...)`, so that form walks the entire full-text table however
  small the subquery is. Same for `WHERE id IN (...)` — `fts.id` is `UNINDEXED`, so the predicate
  is evaluated per row, *including when the subquery is empty*. Both forms were written during the
  2026-08-20 seek work and both silently reintroduced the scan they were meant to remove. Resolve
  the rowid in Rust and delete by equality; guard any legacy `id`-matched delete behind a cheap
  `objects_path` existence check. Costs measured at 8k notes: 1.6× residual growth from the `IN`
  form, flat once it was an equality seek.

- **Trap: a perf budget can measure the wrong denominator and fail on correct code.** The first
  version of `an_incremental_poll_stays_linear_in_what_changed` divided the *whole* incremental
  pass by the number of changed notes, and read 3.7× on a correct implementation — because an
  incremental poll has an inherent O(n) component (`SELECT path, mtime_ns` over every row, a
  `read_dir`, a `stat` per file) that `perf.rs` already budgets separately as the quiet case.
  Subtract the quiet pass at the same `n` and divide what is left. Related: keep the changed-note
  count well above the noise in that subtraction — at 40 the ratio swung 1.07→1.59 run to run and
  looked like a signal; at 200 it is stable.

- **`ping` under heavy contention is not fast.** Measured ~0.5 s worst case while another thread
  hammered `asset_status`/`recent` on a 300-note vault (`fm-app/tests/mobile_workload.rs`). That
  is an artificial worst case and it is inside the 1 s budget the test asserts, so it is not a
  defect — but it is the number to beat if "the app feels sticky while syncing" is ever reported,
  and it is why the budget is a ceiling rather than a target.
- **No per-view object cache.** Board/Agenda/Timeline each YAML-parse the whole
  corpus via `load_all` per request. Same scale caveat as above.
- **Missing media is a warning, never a crash** — by design. A missing blob
  renders the `.asset-missing-inline` placeholder; don't "fix" it into an error.
- **The mock now mirrors one guard deliberately.** `mock.ts`'s `update_body` throws the same
  conflict the server does, because a mock that quietly accepts a write the backend would
  refuse is how the UI's rejection path stays untested until a user finds it.
- **The mock can drift from the real contract silently.** `mock.ts` returns `… as T`,
  which casts the type check away — so it kept a top-level `restic_repo` long after restic
  became per-vault, and `tsc` said nothing. `backup_status` now builds a typed
  `BackupStatus` first; the other arms are still bare casts. If a UI test passes against a
  shape the Rust doesn't send, this is why.
- **Auto-commit is best-effort; commits can lag** (audited 2026-07-17, half-fixed
  2026-07-18). `scheduleCommit` debounces 5s and every GUI write reaches it (4 App call
  sites + `onsaved` from NotePanel's five write paths). **No longer silent, and no longer
  default-vault-only**: it commits *every* vault (it used to call `commit()` with no vault
  argument, so on a multi-vault install exactly one repo had a history) and it states the
  first failure in the notice banner instead of `.catch(() => {})`. What is still true: the
  timer is a browser `setTimeout` that **dies with the tab** — and with `FM_AUTO_SHUTDOWN`
  closing the tab *is* how you quit, so "edit, then close" can skip that commit — and
  **`fm-cli`/Vim writes never commit** (no `fm commit` subcommand). Nothing surfaces "you
  have uncommitted edits". **And since 2026-07-18 a Vim edit is never committed by us at
  all**: `commit_all` stages exactly the paths `put`/`delete` recorded, so a note you are
  hand-editing is not swept in mid-sentence — the flip side being that it is not versioned by
  the app either, and is yours to commit. Files are still never at risk (atomic temp+rename);
  what lags is *our* commits of *our* writes.
  So: **files are never at risk; commits can lag.** Don't restate this as "history is
  always safe" — it isn't. Also, the spec (`MASTERPLAN.md:350`) says "500 ms→disk,
  30 s/blur→commit": the code is 5 s with **no blur handler**, and `MASTERPLAN.md:391`
  still lists auto-commit as *not built* while `:426` lists it as shipped.
- **A backup destination may be local and that is not an error.** A git remote can
  be a path/`file://`, and `FM_RESTIC_REPO` is a bare path when local. `reachOf`
  (`destination.ts`) classifies both; the panel must keep saying which. Never
  report a local destination as "off this machine".
- **Board column *and card* order are client-side, and column reorder is currently inert
  in the pane workspace** (`localStorage['fm-board-order']`
  and `['fm-card-order']`, both keyed by group-by; card order additionally by
  column value) — view preferences, per-browser, **not** in the vault, so they
  don't sync across machines. Intentional; the vault-side `.view` file would
  change that (still deferred). Column DnD is mouse-only (like card DnD).
- **A `.view` file cannot be written, edited or deleted from the UI — only stepped out of.**
  `list_views`/`run_view` are the only commands; there is no `save_view`, and no CLI subcommand
  either, so a saved view is authored by hand in `<vault>/views/*.view`. Since 2026-08-24 a
  filtered view at least *declares* what it leaves out and offers one click to the unfiltered
  built-in renderer (`decisions.md#ui`) — but the owner works only through the UI, so **changing
  what a view filters is still not something they can do**. Whoever adds view authoring: the
  words for the filter already exist (`views::describe_pred`), which is most of an editor's
  read side.
- **The caret-anchored `/` menu is unverified in headless.** `caret.ts` measures
  with a mirror div, and **jsdom has no layout** — `caretXY` returns zeros there,
  so the menu degrades to the editor's top-left and the tests can't see the real
  placement. Only a real browser proves it; re-check by eye after touching the
  editor's font/padding, since the mirror clones exactly those properties.

- **Whiteboard (Excalidraw) caveats.** (1) Fonts are self-hosted (`copy-excalidraw-fonts.mjs`,
  build-time; `index.html` sets `window.EXCALIDRAW_ASSET_PATH`) so nothing phones home; Xiaolai
  (13 of 14 MB, CJK) is skipped, so CJK whiteboard text falls back to a system font. **Durable
  lesson: a deferral justified by a single number deserves the number re-measured** — the "~14 MB"
  that deferred this was one font; everything else was 362 KB. (2) The canvas renders only in a real
  browser, so
  it's **unverified in headless CI** (build, code-split, and the board round-trip
  are tested; the visual editor is not). (3) Whole-note **embeds** shipped
  2026-07-21 (`![](note:id)`), but a **board** embedded renders its raw Excalidraw
  *scene JSON* as a body, not the canvas — the embed path is the Markdown renderer,
  which never invokes the board widget. So a board is still effectively open-on-its-own;
  canvas-inside-an-embed is the remaining gap. Planned in [plan.md](./plan.md) (Track S #4).

## Deferred (intentionally not built yet)

- Global capture hotkey (was window-only; needs rethinking for the browser).
- Optional mlua scripting hatch.
- **v2:** CM6 live-preview editor, backlinks panel, watched inbox, OCR, video
  posters, semantic search. (Forward note references + the sliding-pane trail
  **shipped 2026-07-16**; only the *backlinks* half is still deferred, and it
  needs a link index — see [plan.md](./plan.md) (Track C, Phase 3).)
- **Calendar sync, whiteboard-in-a-note + PDF export, and the `Source`/local-model
  ingest module** are *planned, with the design decided* — see
  [plan.md](./plan.md) (Track S) rather than re-deriving them.

## Historical / no longer relevant

- The entire "desktop window rendering / blank WebKitGTK / gui-env / global
  shortcut / custom-protocol / screenshot recipe" saga is **dead** — the window
  was removed 2026-07-15. Ignore those notes for how-to-run; kept only as the
  reason behind the browser pivot ([decisions.md](./decisions.md)).

## Traps for whoever works here next

- **A quadratic hides at the size you develop at.** The full index rebuild was O(n²) from the
  start and nobody saw it, because at a few hundred notes it is milliseconds. Measured 2026-07-20
  at the size the project claims to support: **2 500 notes 2.2 s, 5 000 notes 10.3 s, 10 000 notes
  54.7 s** — ~5x per doubling — and 169 s for three vaults, against a MASTERPLAN budget of
  "reindex < 10 s @ 10k". Cause: two unindexed scans per note (`forget_path` deleting by
  `objects.path`, which had no index, plus a `DELETE FROM fts` against an `UNINDEXED` id).
  After: **0.29 s at 10k, 0.87 s for 3x10k**, with a flat per-note cost.

  The lesson for the next one: **assert the shape of the curve, not a duration.** A wall-clock
  budget at 10k would have caught this only if someone had thought to write it at 10k; the
  linearity test (`crates/fm-core/tests/perf.rs`) compares per-note cost at 2k against 8k and
  fails at ~4x, which is what quadratic looks like at any absolute speed.

- **Before optimising, measure — then measure again after the first fix.** The first fix here
  (skipping the redundant FTS delete) was correct, reasoned from the code, and moved 10k from
  54.7 s to 38.1 s while leaving the curve quadratic. It looked like progress and was not the
  cause. Three separate profiling attempts were wrong before reading the loop body settled it:
  SQLite itself does the same 10 000 inserts in **0.03 s**, which is what proved the problem was
  never the database.

- **`pixi run` costs ~1.4s whenever its activation cache is cold**, against 0.07s warm — and
  `--frozen` does *not* avoid it. Measured 2026-07-20 while hunting startup time. This is why
  `packaging/formicaria.sh` execs `target/release/fm-serve` directly with the env's `bin` on PATH
  instead of going through `pixi run app`: the server itself binds in 35–43ms, so pixi was ~97% of
  a cold launch. **Beware measuring this by alternating the two forms** — the first attempt
  "proved" `--frozen` was faster purely because plain always ran first and warmed the cache for
  it. Time each form cold, separately, or the answer is an artefact.

  What activation actually supplies at runtime is `PATH` and nothing else that matters: no
  `LD_LIBRARY_PATH` is set at all, `git` comes from `/usr/bin`, and `pdftotext`/`vipsthumbnail`/
  `restic` run from the env's `bin` with PATH alone (conda binaries carry their own RPATH). The
  rest is the conda *build* toolchain, which a running server has no use for. The dev tasks still
  go through `pixi run`, and must — they build.

- **You cannot verify a UI change by grepping the APK — Tauri brotli-compresses the embedded
  frontend.** There is no `assets/*.js` in the APK at all: `frontendDist` is compiled into
  `libformicaria_mobile_lib.so` and compressed, so `strings` finds *zero* UI text (checked
  2026-07-20 — `asset-missing-inline`, `From the library` and even `svelte` all return 0 hits in a
  binary that certainly contains them). The `.so`-grep that caught a phantom fix on 2026-07-19
  worked because that string was **Rust**, and generalising it to UI strings would silently
  "prove" every frontend change missing. Verify instead that the string is in `ui/dist/assets/*.js`
  **and** that `ui/dist` is older than the APK — that pair is what shows the bundle was embedded.

- **`pixi run ci | tail` reports the exit code of `tail`, not of CI.** A piped gate always looks
  green: on 2026-07-20 a failing test (221 tests, 1 red) was reported as "exit code 0" because the
  pipeline's status is its *last* command's. Run `pixi run ci` unpiped, or append
  `; echo "exit: $?"`, and read the count — never trust the exit status of a pipe.

- **`tauri icon` rewrites `mobile/src-tauri/icons/*` non-deterministically.** Every
  `ci/android-release.sh` run re-encodes all platforms' icons, so an *Android* build leaves the
  *macOS* `icon.icns` dirty with 43k of 44k bytes changed and no semantic difference. It is
  committed, so it shows up in `git status` after any APK build. Discard it
  (`git checkout -- mobile/src-tauri/icons/`) rather than committing the churn; whether these
  should be generated at build time instead of tracked is an open design call, not a papercut fix.

- **Line endings are LF, and both halves matter.** `from_file` tolerates CRLF because a
  Windows editor produces it; the repo's `.gitattributes` (`* text=auto eol=lf`) stops git
  producing it in the first place; and `ensure_repo` writes `*.md merge=fm text eol=lf`
  into every vault for the same reason. Remove any one and Windows breaks *silently*: the
  loader is deliberately tolerant, so unparseable notes don't error — they vanish, and the
  vault opens empty. That is exactly how CI found it (all eight `e2e_vault` tests at once,
  because they're the only ones reading committed fixtures rather than writing their own).

- **`.desktop` has no relative `Exec`** — it must be absolute, so the entry cannot be a
  static file in the repo. It was one, carrying `/home/baljinder/...`, which meant every
  clone got a launcher into a stranger's home *and* a rename silently rewrote the path to
  somewhere that didn't exist (the file looked right and launched nothing).
  `packaging/install.sh` now **generates** it from the real checkout path. Moving the repo
  = re-run `install.sh`; there is no fixing it from inside the file.

- **`fm-cli` does NOT front `fm-app` — the command logic is forked in two.** Despite the
  overview's "one command library behind two frontends" framing, `crates/fm-cli/Cargo.toml` has
  **no `fm-app` dependency** and `fm-cli/src/main.rs` reimplements the commands against `fm-core`
  directly (e.g. `Cmd::Add` at `main.rs:131-146` rebuilds ingest instead of calling
  `commands::ingest`). Only **`fm-serve`** is a thin frontend over `fm-app::commands`; and
  `fm-serve::api()` itself is not "thin over `MultiStore`" — it dispatches over `Mutex<Vaults{
  MultiStore + Vec<VaultConfig>}>` with a documented single-lock discipline (`main.rs:31-37`) and
  a dozen arms that reach around the `Store` trait to per-vault paths. So there are **two** command
  surfaces today, and any new frontend (the planned mobile bridge) is a **third**. *(Half
  fixed 2026-07-18: `fm_app::dispatch` shipped and `fm-serve` is now a transport shell over
  it — so there are **two** surfaces, not three-in-waiting. `fm-cli` now depends on `fm-app`
  and shares its command *functions* rather than rebuilding them; it deliberately does not
  route through `dispatch`, which is a JSON wire surface — see `decisions.md`.)* Track M's
  ruling 1 (extract `fm_app::dispatch`) exists to collapse these. **This trap has no expiry —
  corrected 2026-07-19.** It previously read "until it lands", which invites a reader to wait for
  a convergence that was ruled *against*: `decisions.md` settled on one command **library**, not
  one command **door**, so `fm-cli` sharing `commands::*` while not routing through `dispatch` is
  the intended end state. What remains permanently true is the operational half: a change to a
  command's behaviour must be made where both callers see it, or they drift. Found by the
  2026-07-18 mobile-port audit. *(`crates/fm-cli/src/main.rs:7-8` still asserts the retired debt
  in source — fix it there too, or a cold reader who checks the code finds it reasserted.)*
- **`pixi run build` must build `fm`, not just `fm-serve`.** `ensure_repo` installs the
  `.md` merge driver by pointing git at the `fm` binary **beside the running one**, and
  deliberately installs nothing when it can't find one. So a build task that ships only
  `fm-serve` makes the merge driver silently never install — every concurrent edit then
  conflicts on the `updated:` line, i.e. the single thing Phase 1 exists to prevent, with
  no error anywhere. It was like this for a whole phase and every test passed, because the
  tests build `fm-cli` themselves. **Only a clean release build finds this.** If you ever
  split the workspace or trim the build task, this is what breaks first and quietest.

- **Sandboxed Bash fails** in this environment with a seccomp/`setgroups`
  error. Run shell commands with `dangerouslyDisableSandbox: true`.
- **The desktop launcher runs a PREBUILT binary and never recompiles.**
  `pixi run app` execs `target/release/fm-serve` + the built `ui/dist` as they are
  on disk — that's what makes the icon start instantly. So a backend change you
  just made is **invisible** to the owner until someone runs `pixi run build`
  (this bit us on 2026-07-16: a query-layer filter looked unimplemented for an
  hour). `pixi run serve` rebuilds debug and hides the trap. **Run `pixi run build`
  before asking the owner to verify anything**, and check
  `stat target/release/fm-serve` against your edit when a fix "didn't work".
- **Toolchain is not on PATH.** `cargo/node/pnpm/mdbook/restic` live in
  `.pixi/envs/default/bin`. Use `pixi run <task>` or
  `pixi run -e default <cmd>`; a bare `cargo`/`pnpm` in a background shell will
  be "command not found".
- **`fm-serve` env vars:** `FM_VAULT` (no default — unset means the first-run screen), `FM_UI_DIST` (default
  `ui/dist`), `FM_ADDR` (default `127.0.0.1:8765`), `FM_OPEN` (xdg-open the
  browser), `FM_RESTIC_REPO` / `RESTIC_PASSWORD` (the **media** backup tier only —
  the notes tier needs neither).
- **An unreadable `.md` disappears from the app with only a stderr line.** Since
  Phase 0 the vault opens and serves the rest (`FileStore::skipped()`, warned about
  at `fm-serve` startup), which is the right trade — but a user in the browser sees
  the note **silently missing**, and the terminal is the only place that says why.
  The in-app list is Phase 1's "conflict surfacing" (`plan.md`).
- **`fm-serve` is only partly tested.** The blob route now has real response-path tests
  (a listener on port 0, a live socket: sniffed type, the `nosniff`/attachment allowlist,
  `Range`/206/416, 404) and the query-args split has unit tests. Everything else — the CSRF
  guard, the `Host` guard, static serving, the watchdog — is still exercised only by hand,
  and every UI test runs against `mock.ts`. Narrower than it was; not closed.
- **A backgrounded tab can shut the app down.** The heartbeat is 3s
  (`App.svelte:229`) but browsers throttle background timers to ~1/min, while the
  watchdog idles out at 10s (`main.rs:91`). Only bites with `FM_AUTO_SHUTDOWN`
  (i.e. the desktop launcher). Found by the 2026-07-17 audit.
- **SQLite has no `busy_timeout`/WAL**, and every `fm` CLI command takes an
  exclusive write lock — so the CLI races a running server. Found by the
  2026-07-17 audit.
- **The API silently accepts malformed JSON.** `api()` does
  `serde_json::from_slice(body).unwrap_or(Value::Null)` (`fm-serve/src/main.rs:164`)
  and `s(k)` then `unwrap_or("")`, so a client bug arrives as an **empty-string
  arg**, not an error. This bit during verification (a bad test body became
  `set_git_remote("")`, and `git remote add origin ""` *succeeds*, leaving a
  remote whose `get-url` reports its own name). `set_remote` now refuses a blank
  URL; other commands are still exposed to this.
- **Path traversal** on the static route is rejected with 400 — verify with
  `curl --path-as-is` (plain curl normalizes `../` client-side and hides it).
- **Renderers:** no `todo/doing/done`, no scheduling literals — CI grep
  (`ci/checks.sh`) will fail the build.
- **`fm_core::git::<fn>` outside `fm-core` fails the build** (`ci/checks.sh`) — call
  `fm_core::vcs::` instead. `git.rs` shells out and there is no `git` binary on Android, so
  naming a backend at a call site is how the phone ends up reporting "git not installed" while
  carrying a working libgit2. It happened **twice** (history, then the whole proposal
  lifecycle), because a unit test cannot catch a caller that never calls. Shared *types*
  (`Accepted`, `Identity`, `Pulled`, `Probe`, `HelperAdvice`) are UpperCamel and pass; the
  genuine "is there a git **binary**" questions (`available`, the credential helpers) are
  allowlisted by name.
- **Every NotePanel pane mounts its own `<svelte:window onkeydown>`**, so a key
  press is heard by *all* panes in the trail. Pane-scoped shortcuts must go
  through `ownsKeys()` (focus inside some pane → only that pane acts), or you get
  the bug Escape-exits-edit had: pane 1 closing the trail while you leave pane 2's
  editor.
- **The note trail is now a peer grid column, not a modal overlay** (de-modalized
  2026-07-17). The board stays live beside an open note and no backdrop dismisses it —
  close with the ✕ button or Escape (`NotePanel.onPaneKey`). Deliberately **left
  conservative**: `App.onGlobalKey`'s `if (openIds.length) return` still suppresses the
  app-level `1/2/3`/`c`/`/` shortcuts while a note is open, so those don't drive the view
  beside it. Making them focus-aware is the *pane-grid* rabbit hole the workspace design
  explicitly rejected — do not open it without a reason the two-region layout can't meet.
- **Assets are excluded from `board`/`agenda`/`recent`** (`Predicate::Kind`), so a
  board grouped by `type` has only a `note` column *by design* — don't "fix" it.
  Keep `search`/`gallery` seeing assets: search is the only way to find a PDF by
  its extracted text, and the `/` menu's asset insertion rides on it.
- **`mock.ts` state leaks across tests in a file.** `bodyOverrides` and the `seq`
  counter are module-scope, and vitest isolates per *file*, not per test — so
  `App.flow.test.ts`'s edit walk rewrites the GAE note's body for every test
  after it. Anchor a later test to a note the walk doesn't touch, or add a reset.
- **jsdom has no layout.** `scrollTo`/`getBoundingClientRect`/`IntersectionObserver`
  are absent or stubs, so guard them (`el?.scrollTo?.(…)`) the way the
  `localStorage` reads are guarded — an unguarded call is an unhandled rejection
  in the suite and a real crash in any browser that lags the API.
- **Do not add fs/db to `fm-query`** — compile-time + CI enforced.
- **Commit discipline:** solo repo, work on `main`, no branch/PR ceremony — but
  commit **only when the user asks**.
- **`vault/` is gitignored** (the knowledge vault is its own repo). Test
  fixtures under `crates/*/tests/fixtures/` are NOT the root `/vault/` and are
  tracked.

## External facts (dated — re-verify, never trust the date alone)

**The rule that created this section: any external claim gets a date and a re-verify command, or
it does not go in.** The corpus previously carried *"once `gix` push ships"* as though it were a
schedule, for an upstream issue that has been open for years.

All verified **2026-07-19**.

0. **VERIFIED ON DEVICE 2026-07-19: there is no `git` binary on Android.** Previously sourced
   from platform docs and asserted throughout `docs/context/`; now confirmed by running
   `command -v git` on the owner's phone (`git: inaccessible or not found`). formicaria's core,
   cross-compiled and pushed to `/data/local/tmp`, captured/listed/searched notes there with no
   git present and created no `.git` — so the capability model degrades exactly as designed.
   See `sessions/2026-07-19-the-core-on-the-phone.md`. This is the premise the whole libgit2
   decision rests on, and it is no longer an inference.
1. **`gix`/gitoxide push is still unimplemented.** Re-verify via gitoxide's `crate-status.md`,
   **never** by the existence of a `gix::push` module — that name is the `push.default` config
   enum and will fool the next checker. Consequence: the pure-Rust escape is closed, and any
   `gix` backend means hand-writing `send-pack`.
2. **conda-forge ships all four `rust-std-*-linux-android` targets, and no NDK/SDK.** Re-verify:
   `curl -s "https://api.anaconda.org/search?name=rust-std" | grep android`. The only `android-*`
   hits on anaconda.org are unmaintained personal channels (`rodgomesc`, `kivyschool`) — not
   something to stake a decade on. So the pixi gap is **NDK + SDK only**.
3. **`bundled` rusqlite (`crates/fm-core/Cargo.toml:18`) needs an NDK sysroot** regardless of any
   git decision. "Avoid C cross-compilation" was never a live argument for any backend option.
4. **Inside a pixi env, a misconfigured Android toolchain produces GREEN builds.** `c-compiler`'s
   activation always sets `CC`, so `cargo check -p fm-core --target aarch64-linux-android` exits
   **0 with no NDK installed** and deposits an **x86-64** `sqlite3.o` into the `aarch64` tree.
   `cargo build` does not catch it either — no workspace crate declares `crate-type`, so these are
   rlibs and rlibs never link. **Assert an artifact fact, never an exit code:** build a linking
   target (`-p fm-cli`) and check `readelf -h` reports AArch64.
5. **conda's `c-compiler` activation breaks Android cross-compilation, and the per-target
   override does not rescue it.** Found 2026-07-19 while getting the first ARM build green.
   The activation exports host flags — `CFLAGS=-march=nocona -mtune=haswell … -isystem
   <env>/include`, plus `CPPFLAGS`/`LDFLAGS` pointing into the x86-64 environment — and the
   `cc` crate applies `CFLAGS` and then *appends* `CFLAGS_<target>`, so setting the specific
   one does **not** win. The Android clang receives an x86 `-march` and refuses to compile
   SQLite, with a wall of flags that look like ours and are not. **Fix: empty `CFLAGS`,
   `CPPFLAGS` and `LDFLAGS` in the android environment** (`pixi.toml`,
   `[feature.android.activation.env]`), which is safe because every C compile in that build
   targets the phone. Same root cause as trap 4 below — conda's C activation is host-shaped —
   but the opposite symptom: this one fails loudly, that one passes green and wrong.
6. **`git2`'s `https` feature needs `vendored-openssl` on Android.** There is no system
   OpenSSL to link against, so `openssl-sys` fails outright (`$TARGET =
   aarch64-linux-android, openssl-sys = 0.9.117`). Building it from source is what GitSync
   ships for the same reason — and it puts a *second* vendored C blob under a licence its
   `-sys` crate does not declare, which is why `ci/checks.sh` guards `openssl-src` too.
7. **Android loopback is not sandboxed.** Any app holding `INTERNET` can reach a localhost
   listener. `fm-serve`'s Host/Origin guards are a *browser* threat model and do not apply.
   **Never ship `fm-serve` as a TCP listener on a phone** — the on-device-server fallback needs a
   per-launch bearer token before it is even a candidate.
6. **`config_dir()` does not compile for Android.** `crates/fm-app/src/vaults.rs:239-257` has
   three `#[cfg]` arms — `linux`, `macos`, `windows` — and no fallback; Android's `target_os` is
   `"android"`, so every arm is skipped. The first Android build fails in the file the dispatch
   extraction created. Precondition, not a follow-up.
7. **`git.rs` has no `clone`.** Thirteen public fns, none of them clone. Every plan document that
   frames M1 as a *port* is wrong: it is new code on every possible backend.
8. **Executing a bundled binary is possible on Android, impossible on iOS.** Android 10+ blocks
   `exec()` from the app's writable home directory (SELinux drops `execute_no_trans` for
   `targetSdk ≥ 29`), but `nativeLibraryDir` under `/data/app` stays executable — hence the
   `lib*.so` naming trick plus `useLegacyPackaging = true`. iOS forbids it outright (no
   `fork`/`exec`, mandatory code signing, W^X). Consequence: any binary-shipping design is
   **Android-only forever**. Struck as an option anyway — see `decisions.md` 2026-07-19.
9. **A bare remote whose `HEAD` names a branch it does not have clones to a silently empty
   vault.** Found 2026-07-19 by making the mistake in a verification script: `git init --bare`
   sets `HEAD` from this machine's `init.defaultBranch` (`master` here), so pushing to `main`
   leaves `HEAD` dangling. `git clone` then checks out an empty `master`, `origin/main` exists
   but is not checked out, and **the user gets a registered vault with no notes and no error** —
   git behaved correctly, and nothing in our stack is wrong. Hosted remotes set `HEAD`
   properly, so this bites **self-hosted** ones, which this project treats as first-class. Fix
   on the server (`git symbolic-ref HEAD refs/heads/main`). Worth a check in `clone` if it ever
   bites someone real: a clone that checks out nothing while the remote *has* branches is
   reportable, and today it is indistinguishable from cloning a legitimately empty repo.
10. **Syncthing is not a viable mobile backend.** Syncthing-Android was discontinued 2024-10-20
   (a Google Play storage-permission fight); the surviving fork went through an opaque
   signing-key handover that triggered an F-Droid security investigation. iOS never had an
   official app and **persistent background sync is structurally impossible there**. This does
   not retire the "design against Syncthing's profile on paper" hedge — a paper target has no bus
   factor — it **validates git as the coordinator**: fetch/merge/push is a discrete resumable job
   that fits every budget both OSes grant; a P2P mesh fits none.

- **The phone's 41 duplicate copies have no identified trigger.** 20 message bodies exist 2–6 times
  each in `formicarium-vault`, every copy a `role: message` agent command (`… /search`,
  `… /transcribe sha256:…`). The reply path is therefore implicated, but *what* re-sent them is
  unknown — a user retrying an unanswered command and a resend loop in the send path look identical in
  the notes themselves. `created` on each copy is the next evidence: identical `created` across a
  family means one write fanned out; distinct `created` means separate sends.

- **Testing Library's async default was shorter than a loaded machine needs.** Adding one test file made
  an unrelated one fail: `findBy*` retries for 1 s, and a full `App` mount in jsdom under parallel load
  exceeds that. It reads as cross-file state leakage, which vitest already isolates per file — so the
  hunt goes to the wrong place. `ui/src/test-setup.ts` sets `asyncUtilTimeout: 5000`. Suspect this
  first when a test fails only in a full run.

