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
  scraping and blocks scrapers; adding it is the remaining step. (**The desktop searches in-process too**
  since 2026-09-02, when the launch path dropped its shell — so DuckDuckGo is a loss on *both*
  now, not a desktop advantage. `pixi run search-proxy` + `--searxng-port 8888` remains as an
  explicit override that restores it.)
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
- **`pixi run build` rebuilds `agent-serve`** as of 2026-09-02 — it ships in the archive, so it has
  to. Before that it built only fm-serve + UI, and a changed agent kept running the old binary.
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
- **The assistant installs itself, and the desktop gaps this section described are closed**
  (2026-09-02, `decisions.md#agent`). `fm-serve` depends on `fm-agent-run/download`, `pixi run
  build` builds that crate, and the archive carries `agent-serve` + `models.toml`; support is
  decided by a live memory reading rather than an OS list. Audio transcription self-installs on
  Linux and Windows. What follows is the record of the state before that, kept because the *chain*
  is the value — not because any of it still holds.
- **What was missing on the desktop was `fm-agent-run/download`, not `fm-serve/agent`** (corrected
  2026-09-02). The `agent` feature was always on in the shipped binary; the `download` feature was
  not, `pixi run build` did not build `fm-agent-run`, and `release.yml` staged nothing AI-related —
  so a downloaded copy reported "the assistant is not on this machine yet" **even on Linux**. All
  four are now false.
- **macOS still has no upstream speech-to-text build.** whisper.cpp v1.9.1 publishes Linux and
  Windows binaries and none for macOS, so transcription is unavailable there — a missing *artifact*,
  not a missing implementation, and the app says so in those words. Adding it is a URL and a hash on
  the day upstream publishes one.
- **Superseded 2026-09-02, kept for the chain:** *The assistant runs on Linux and Android only* — `fm_agent`'s resource monitor reads `/proc` and
  **fails closed** everywhere else, so on Windows and macOS `preflight::admit` refuses before a model
  is spawned. Since 2026-08-29 the settings row says so instead of offering the switch, so this is a
  stated boundary rather than a silent one; it stops being a gap when those platforms get a monitor
  (`GlobalMemoryStatusEx` / `host_statistics64`), not by relaxing the gate. **Residual:** the
  capability check covers the OS, the stack and the tools it shells out to, but **not the weights** —
  a machine with `agents/` and no GGUF still turns the assistant on and fails at the model server.
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
- ~~**A `configure_paths` early return produces a *wrong* screen, not a blank one.**~~
  **Fixed 2026-09-04** (`decisions.md`, *a shell that cannot configure its paths refuses*): the
  failure is logged as a sentence and recorded in a `OnceLock` that `boot` consults first, so the
  app says what happened instead of presenting a first-run form that cannot save. `configure_paths`
  also logs `vault root:`, and `ci/android-smoke.sh` asserts that line exists **and precedes**
  `vaults ready` — proven red on the emulator by removing it.
- **A very large import may not land as one commit.** `vcs::commit_all` hands every path to `git`
  in a single argv, three times (`ls-files`, `add -A`, `commit --only`). Every other caller commits
  a handful; an import commits thousands, and somewhere past roughly fifty thousand paths that
  exceeds `ARG_MAX` and the commit fails. **The notes are on disk and in the index either way** —
  only the "undo it in one step" property is lost — and the import report says so in those words
  rather than claiming success. Fixing it means batching the staging while keeping one commit,
  which changes a function every write path shares; not worth doing until someone actually has a
  graph that size.
- **An import runs `vipsthumbnail` once per attachment, under the vault lock.** The genuinely heavy
  parts (hashing, `pdftotext`) already happen with the guard released, but `commands::asset_note`
  thumbnails inside the write loop and is reused deliberately rather than reimplemented. A few
  hundred images is seconds; a few thousand is not. See `papers-plan.md` B5 for the shape.

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
  an unexplained renderer kill).
  **Chunked ingest lifted the ceiling on 2026-09-04** (`decisions.md`, *a file is sliced, so its
  size stops being a memory limit*), so a file over 16 MB is no longer refused — it is sent in 2 MB
  slices through `fm_ingest_chunk` + `fm_ingest_finish` over `BlobStore::put_file`, and **video on
  Android works**. 16 MB is now the threshold between the cheap single-shot path and the chunked
  one, not a refusal. The orphan-session hazard the plan named is handled by an age-based sweep at
  boot (`fm_core::chunked::sweep`).
  **Both ends of the bridge are now tested** (they were not, which is how the zero-byte photo
  shipped): `fm-app/src/wire.rs` for the decoder, `ui/src/lib/ingest.phone.test.ts` for the
  encoder and the ceiling, `fm-app/tests/mobile_workload.rs` for a real multi-MB photo.

- ~~**The phone's `fmblob` handler holds a whole blob in memory, honours no `Range`, and ships none
  of the desktop's security headers.**~~ **Fixed 2026-09-04** (`decisions.md`, *the phone's blob
  route answers a `Range`, and stops lying about it*) — `parse_range`/`inline_safe`/`blob_reply` are
  in `fm_app::wire` and tested in the gate, the handler seeks and reads one window, and it sets
  `nosniff`, a CSP and the disposition allowlist. **One thing it does NOT close**, and it matters:
  an actual ranged fetch has never run on a device. Verified by compilation and by the shared tests;
  a negative control on the emulator proved the handler is not reached from any screen
  `android-smoke` drives. That belongs to `outstanding.md` §1.1. Original entry, for the chain:
  *(Moved here 2026-09-04 from `papers-plan.md` B4, which is where it had been recorded and is not
  where anyone looks.)* `blob_response` (`mobile/src-tauri/src/lib.rs`) calls
  `dispatch("resolve_asset", …)` and returns `out.into_bytes()`: no `Accept-Ranges`, no `Range`
  parsing, a hardcoded `application/octet-stream`, and `Access-Control-Allow-Origin: *`. Its comment
  says *"a GET streams, and `<video>` can seek without the file ever being held whole in memory"* —
  **none of which is true**, and the *"streams"* half is not even achievable: Tauri's
  `register_uri_scheme_protocol` returns `Response<Vec<u8>>` and has no streaming body type. What is
  achievable is `Range`, which bounds the peak to one slice.
  **Worse since 2026-09-04**: chunked ingest removed the upload ceiling, so the files this path must
  serve are now unbounded — a 200 MB video is a 200 MB `Vec` plus wry's copy, on the device with the
  least memory.
  **And separately**: `fm-serve/src/blob.rs` sets `X-Content-Type-Options: nosniff`, a CSP, and an
  `inline_safe` allowlist forcing `Content-Disposition: attachment` for anything not known-safe. Its
  header argues that a blob reachable as a same-origin URL is a navigation hazard **because blobs
  arrive from collaborators through the merge driver** — an argument that is platform-independent,
  and the phone has none of the mitigations.

- ~~**Ingest holds the global vault lock across two subprocess spawns per file.**~~
  **Fixed 2026-09-04** (`decisions.md`, *the vault lock is not held across a subprocess or a
  revwalk*), together with `activity`'s revwalk. Both are pinned by a rendezvous test in
  `fm-app/tests/dispatch_concurrency.rs` — 5 s blocked versus 0.06 s free, measured both ways.
  Original entry, for the chain:
  *(Moved here 2026-09-04 from `papers-plan.md` B5.)* The `ingest` **and** `ingest_finish` arms in
  `dispatch.rs` take the guard and keep it through `fm_core::ingest`'s `pdftotext` and
  `vipsthumbnail` spawns, so a bulk import freezes every tab — the estimate on the record is 25–40
  minutes for 5,000 PDFs. Several arms in the same file already drop the guard before slow I/O
  (`run_backup`, `run_import`, `open_skipped`) and `run_import`'s comment states the rule the fix
  must follow: **re-resolve the vault after re-taking the guard**, because it may have been
  forgotten or moved while the work ran.
  **`ingest_finish` is new (2026-09-04, chunked ingest) and inherited this** — which is the worse
  half, since it is the path built specifically for large, slow files.

- **The read view asks for a thumbnail now; *generating* one on Android is still missing.**
  *(Half fixed 2026-09-04 — `decisions.md`, *the read view draws the small copy*.* `render.ts` uses
  `asset.thumb ?? asset.url` for images, and `resolve_asset_bytes` falls back to the full blob so a
  phone-ingested image without a derivative still renders. What remains is M8: nothing on Android
  can *make* a thumbnail, so on a phone the fallback is the normal path and the full-resolution
  decode is unchanged there.)* Original entry: Easy to mis-diagnose as an
  Android gap, and it is not. `ingest::thumbnail` shells `vipsthumbnail` and writes
  `derived/<hash>/thumb.webp`, but the only readers are `fm-cli` and `asset_status`'s `has_thumb`
  flag; `NotePanel`'s resolver calls `assetUrl(ref)` without a `kind`, so every inline image is the
  **full blob** on desktop and phone alike. *(Corrected 2026-09-04: `assetUrl` does take a `kind` —
  `ipc.ts:564` — and `Timeline.svelte:155` passes `'thumb'`, so the feed is fine. It is the read
  view that never asks.)* The Gallery view that used to consume thumbnails was removed. So a note with five 12 MP photos decodes ~200 MB of bitmap
  everywhere — the desktop simply has the memory to survive it.
  **Do not "fix" this by passing `kind: "thumb"` from the phone.** `commands.rs`'s
  `resolve_asset_bytes` hard-errors on a missing thumb (there is no fallback to the full blob), and
  `vipsthumbnail` does not exist on Android — so every image in every note would become a 404
  placeholder. **The fallback has to land first**, and the correct implementation already exists
  next door: `commands::blob_path_of_kind` documents both the fallback *and* why the blob must be
  resolved before the thumb (a `derived/` file must not answer for a vault whose blob the caller was
  never entitled to). The remaining half — *generating* a derivative on Android — is M8 (pure-Rust
  media extraction), deliberately unsequenced in `mobile-design.md`. Mitigated 2026-08-20 with `loading="lazy"` +
  `decoding="async"` in `render.ts`, so off-screen images cost nothing; the on-screen ones still
  decode at full camera resolution.

- **Anything that only happens on the vault heartbeat is untestable in jsdom.** The repeating
  `ping` interval is behind `if (!import.meta.env.PROD) return` (`App.svelte`, deliberately — a
  repeating interval under Vitest is its own bug), so exactly one beat runs at mount and `vaultTick`
  never bumps. Found 2026-08-31 while trying to pin the theme-escape re-arm bug: the test passed
  identically with and without the fix. **Check that a new test fails without its fix**; for this
  class it cannot, and the honest answer is to say so rather than ship a green tick that means
  nothing.

- **A `.view` file can no longer be removed from inside the app, on any device.** Save, Rename and
  Delete view were withdrawn on 2026-08-31 (*"views are basically fixed for now and view
  customization will need its own design plan"*). The commands and their tests survive; only the
  buttons went. So a stale view — one a collaborator pushed, or one made before the withdrawal —
  is removed by deleting `<vault>/views/<name>.view` and letting git carry it. **Accepted
  knowingly**, when the first instance came up the same day; the design plan is where it gets a
  real answer.

- **Each device has its own vault clone, so deleting a file on the laptop changes nothing on the
  phone until it is pushed *and* pulled.** Obvious once stated, and stated because it was not:
  "Active" was deleted from the laptop's vault, reported as done, and was still on the phone —
  the two are separate clones synced by git, exactly as designed. The tell is
  `git -C vault rev-list --count origin/main..HEAD`. On 2026-08-31 that was **144 commits, three
  days**, because Back up had not been pressed since 2026-08-28 — so the phone was missing far
  more than the one file anybody was looking at. **Check that number before concluding a vault
  change did not work.**

- **A UI change is not on the owner's screen until `pixi run build` — `ui/dist` freshness proves
  nothing.** `fm-serve` serves the copy of the UI **baked into the binary** unless `FM_UI_DIST` is
  set (`crates/fm-serve/src/main.rs:1109-1111`), and the desktop icon — which is how the owner
  actually launches it — sets only `FM_OPEN`/`FM_AUTO_SHUTDOWN` (`packaging/formicaria.sh`).
  `FM_UI_DIST` is the **dev** path (`pixi run serve`), and only there does a rebuilt `ui/dist` plus
  a reload show new work. On 2026-08-31 this cost a full review round: two rounds of UI work were
  finished, `ui/dist` was verified fresh, the owner was told to reload, and they reported three
  already-fixed faults because the binary was four hours old. **Check `target/release/fm-serve`'s
  mtime against `ui/src`, never `ui/dist`'s.** The failure is silent in both directions — nothing
  warns, and the result looks like a broken change rather than an old one.

- **The phone's only reliable diagnostic channel is the one the shell forwards** (half-closed
  2026-09-03; the finding below is not history — it is why the forwarding exists): `eprintln!`/stdout never reaches logcat from a
  Tauri Android shell, and — found the hard way on 2026-07-20 — the WebView routed **no `console.*`
  output there either**: a signed, installed, MD5-verified build full of `console.warn` produced
  zero lines while the native `ca-bundle:` log from the same run came through fine. **That cost a
  full build/sign/install/ask-the-owner round trip.** The shell now injects a script forwarding
  `console.error`, `console.warn`, `window.onerror` and `unhandledrejection` to `fm_log`, which
  writes them through the same `log` sink tagged `web:` (`decisions.md`, *the WebView gets a
  voice*). **`console.log` is still not forwarded, by design** — only failures — so anything you
  want to read off a device that is not an error must still be rendered on screen. **Unverified on
  hardware**: it cross-compiles for `aarch64-linux-android` and nothing here has run it on a phone
  or a Simulator.

- **A phone vault holds the only copy of its media.** App-private storage is wiped on uninstall,
  `blobs/` is gitignored so a push does not carry it, and restic — the one thing that does — is a
  binary Android does not have. Losing notes is bad; losing the only copy of a photo is worse, and
  capture makes that materially more likely. No answer yet.

- **A phone surface must not invent its own inset number.** The shell reads `WindowInsetsCompat`
  and writes the real values into `--safe-*` (2026-08-31), and `app.css` keeps a floor beneath them
  for the moment before that fires. Both of those are *shared*. What still bites is a surface that
  reads raw `env(safe-area-inset-*)` or hard-codes a pixel offset: it gets neither the real value
  nor the fallback. Three did — `.board-exit`, `.theme-escape`, and `UnrecordedPanel`'s local
  `3.25rem` guess — and each was found on the owner's device, by a screenshot, never by a test.
  **Read the tokens.**

  **And the bridge published a zero that defeated the fallback** (fixed 2026-09-08). This is the
  sharper trap, and it is not the one it first looked like. `addDocumentStartJavaScript` runs the
  apply script at *document start*, when the inset listener has not fired and `top` is `0f` — so the
  page opened with `--safe-top: 0px` set as an **inline property on `:root`**, which outranks the
  `@media (pointer: coarse)` floor completely. The floor was not too small on Android; it was
  **never in force there**. The owner's phone has a cutout of `DisplayCutout insets=Rect(0, 130 - 0,
  0)` / density 3.25 = 40 CSS px, and the top control rendered at `y = 0`; reported as *"we cannot
  use top pixels."*
  **The lesson, which is general:** a layered design — real value, else fallback — only holds if the
  upper layer stays *silent* until it knows. Publishing `0` is not falling back, it is overriding
  with the worst answer through the very mechanism (an inline property) chosen to win. Either say
  nothing until you know, or say something true: `MainActivity.fallbackTop()` now answers with the
  device's own `status_bar_height` unioned with `safeInsetTop`, so there is no zero state. The CSS
  floor was raised to 2.75rem and pinned by `ci/checks.sh` as well, but that guards only a touch
  device with *no* bridge — it is not what fixed this.
  Settings → *This machine* now prints the resolved values, because on this phone
  (logcat suppressed, above) there was no other way to tell a broken bridge from a short floor.

- **A surface that covers the screen must not size itself in `vh`, and must say what scrolls.**
  The sibling of the rule above, and it bit harder. Every dialog is `position: fixed` inside
  `.app`, which is `height: 100dvh; overflow: hidden` — **the document never scrolls**, so an
  overlay that outgrows the screen has no fallback at all. `BackupPanel` shipped with no
  `max-height` and no `overflow` and lost its own primary button; three siblings capped in `vh`,
  which is *the tallest the viewport ever gets*, so a retracting URL bar or the keyboard made them
  taller than the screen; and `HelpPanel` had **no overlay CSS whatever**, because its markup
  copied `class="sheet"` from `App.svelte` and Svelte scopes that rule to App's own elements.
  Fixed 2026-09-01 (`decisions.md#ui`) and now CI-guarded — but the guard checks units and
  scrollports, not duplication, so a sixth hand-written copy of the block would still pass.
  The nav-bar floor is `--bar-floor`; do not retype `3.25rem`.

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

- **A reload does not recover the app, and a first-time user found that out. Cause not
  established.** From the only outside report there is
  ([#2](https://github.com/formicaria-org/formicaria/issues/2), macOS, v0.2.1, 2026-08-30):
  *"I was taking my first note and clicked something, then this happened. Reloading the page
  didn't help. I had to rerun the start file. In general, I can't reload the page, I need to
  restart it to reload it."* The screenshot shows the chrome intact and the Activity pane holding
  real data, while the note pane's whole body reads **`TypeError: Load failed`** — Safari's wording
  for a failed `fetch()`, i.e. a network-layer failure rather than an HTTP error status.

  **Ruled out so far** (each checked against the v0.2.1 tree, which is what they ran):
  - *The reload tripped auto-shutdown.* v0.2.1's watchdog was `IDLE = 90s` / `STARTUP = 60s` with
    no `pagehide`/goodbye path at all — those came later. A one-second reload cannot cross a
    ninety-second window.
  - *A poisoned mutex wedged the server.* `fm-serve`'s only `.lock().unwrap()` calls are inside
    `#[cfg(test)]`; the request path uses `if let Ok(...)`, and `fm-app`'s vault lock recovers
    explicitly via `into_inner()`.

  **This entry first claimed the cause was `localStorage` surviving the reload. That was wrong**
  and is left recorded rather than deleted, because the reasoning error is the reusable part: a
  reload re-fetches everything the server holds, so "a reload did not fix it" does narrow the
  cause to *something durable* — but persisted client state is only one branch of that, and the
  other is durable state in the still-running server. `TypeError: Load failed` points at the
  second, and the guess was made without opening the screenshot that says so.

  **What was actually fixed (2026-09-09), since the cause is still open.** Whatever stopped the
  server, the app had *nothing to say about it*: every background failure is swallowed, so a raw
  `String(e)` in one note pane was the only witness a user got. `invoke` now tells the two failure
  shapes apart — `fetch` rejects with a `TypeError` when nothing answers and resolves non-ok when
  the server refuses — and two consecutive network-level failures raise a banner saying the app has
  stopped, that the notes are safe, and that reloading first cannot work. That does not fix the
  disappearance; it stops a stopped server presenting as a wedged interface, which is the part the
  reporter could not get past.

  **And a search for the persisted-state wedge found none.** All ten `localStorage` keys were
  checked for a value that leaves the app unusable with no control on screen to undo it, and every
  candidate has an in-app exit (`fm-panel`'s toggle stays live, `fm-theme` falls back to a complete
  palette, `fm-board-order` appends anything unnamed, `fm-workspace`'s `active` is clamped, a note
  pane keeps its own ✕). The `fm-hidden-vaults` trap was real **at v0.2.1** and was fixed on
  2026-09-08 by `8845283`; it was never reachable on a first run, because with one vault the filter
  never renders. So there is no known wedge to fix — a reset control would be insurance against the
  class, and should be argued for in those words rather than by pointing at a trap that exists.

- **A large attachment pushed from the phone can die mid-transfer, and the cause is not
  established.** Reported 2026-09-09, on the first backup after the phone learned to send
  attachments at all: `SSL error: error:80000020: system library::Broken pipe` — OpenSSL's
  `ERR_LIB_SYS` / `EPIPE`, i.e. the far end closed the socket while libgit2 was still writing.
  **What is known.** The same phone had pushed successfully minutes earlier, so the token and the
  Android CA path are both fine; every blob that has ever reached the remote is ≤ 0.43 MB, and a
  phone photo is an order of magnitude larger. Two variables moved at once — size, and the device
  doing the pushing — so neither is isolated. `push_squashed` uses default `PushOptions` with only
  credential callbacks: no transfer tuning, and libgit2 has no `http.postBuffer` equivalent.
  **What is not a risk.** The rollback is correct: `push_squashed` captures `head_before`, and every
  failure path soft-resets to it, so a failed push leaves the vault's history exactly as it was.
  Verified by reading the error path — nothing was lost in the reported incident.
  **The transport, established 2026-09-09 and worth not re-deriving.** The phone links libgit2
  **1.9.4** (git2 0.21.0 / libgit2-sys 0.18.5+1.9.4) with vendored OpenSSL 3.6.3. A push is git
  smart-HTTP **protocol v0**: `GET /info/refs?service=git-receive-pack`, then one
  `POST /git-receive-pack` carrying the ref pktline *and* the pack in a single body, with
  **`Transfer-Encoding: chunked`** — a compile-time constant for receive-pack, no `Content-Length`,
  no option. The pack is streamed in ≤1 MiB deflate chunks, not buffered whole. **There is no SSH
  transport in the binary at all** (`libssh2-sys` is in neither lockfile). Chunked is *not* the
  anomaly — real git pushes chunked above `http.postBuffer` too, and `http.postBuffer` itself
  appears nowhere in the vendored libgit2 tree, so it is inert here.

  **A named upstream bug, still open, and this version is affected.** libgit2
  [#6385](https://github.com/libgit2/libgit2/issues/6385), *"Broken pipe error while pushing over
  HTTPS (again)"* — open since 2022-08-17, reported as reproducible only on big repositories. Its
  ancestor [#6205](https://github.com/libgit2/libgit2/pull/6205) states the mechanism plainly:
  *"github.com will terminate a git-receive-pack command over http if it is idle for more than 10
  seconds. This is easily exceeded for a large push."* That fix **is** in our tree, so the
  pre-#6205 form is not ours — but the structural gap remains: **one keep-alive connection carries
  the advertisement and the pack**, sits idle for the whole single-threaded
  `git_packbuilder__prepare` in between (`pb_parallelism` defaults to 1), and is then reused with
  no liveness check. Real git sends a `0000` probe first whenever a request outgrows
  `http_post_buffer` (~1 MiB); libgit2 has the same `send_probe`, but `needs_probe` fires only for
  **NTLM and Negotiate**, and we authenticate with Basic — so it never runs, and there is no option
  to force it.

  **MEASURED 2026-09-09, and size was never the variable.** The instrumented push reported:

      after 83304 ms: 0 of 0 objects, 0 bytes of pack sent; packing Deltafication 150/150

  Pack preparation *finished* — 150 of 150 objects deltified — and then the first write to the
  socket got `EPIPE`. **Nothing had gone on the wire at all.** So the connection was already dead
  when the body began: it had carried the ref advertisement and then idled for 83 seconds while
  packing ran, and GitHub closes an idle `git-receive-pack` after about ten. That is #6385 exactly,
  and it settles hypothesis (a) against (b) and (c) — a mid-upload drop would have shown megabytes
  sent, and a flaky link would not have been deterministic four times over.

  **What costs the 83 seconds is delta-searching photographs**, which cannot succeed: incompressible
  bytes compared against each other to discover that every pair is unrelated. Fixed by capping the
  delta search at 512 KB, so notes still delta against their own history and media does not, plus
  `packbuilder_parallelism(4)` for what remains — see `git_native::keep_packing_quick` for why the
  only reachable lever is `pack.deltaCacheSize` (libgit2 reads that one key into
  `big_file_threshold` as well) and what happens if upstream ever fixes that.

  **The batching built for this was aimed at the wrong quantity** and is kept anyway: it bounds
  packing time as a side effect of bounding bytes, and it makes a failure resumable. But the budget
  is bytes where the constraint is seconds, and that mismatch is now understood rather than
  guessed. The old paragraph below is left for its reasoning about what could not be distinguished
  before the measurement:

  The two anchors were tens of KB (works) and ~16 MB (fails, 4/4) with nothing measured between. Across that
  gap bytes, object count, pack-preparation CPU, upload duration and per-object memory all move
  together. The desktop's small successes constrain nothing: it shells out to the `git` binary and
  is a different implementation entirely.

  **So the push is instrumented rather than guessed at** (`git_native::push`): the two callbacks
  `git2` has always exposed and this repo never used — `push_transfer_progress` and
  `pack_progress` — now append *"after N ms: X of Y objects, Z bytes of pack sent"* to the error.
  Nothing sent and a failure time ≈ prepare time means the socket was already dead (#6385, and the
  lever is the idle window). Megabytes sent before it dies means the connection dropped mid-upload,
  and the lever is batch size or the link. One Wi-Fi run then separates the link from both.

  **Ruled out as first moves, with reasons**: SSH (no transport compiled in, no upstream evidence,
  widens what ships), `http.postBuffer` (provably inert — zero occurrences in the vendored tree,
  and tried at three sizes on #6385 with no effect), and a bundled `git` binary, which contradicts
  Track M ruling 2 verbatim — *"no design may assume an executable subprocess on device"* — and so
  would need a written dated reversal before it could even be proposed.

  Also found, incidentally: libgit2 reads `pack.deltaCacheSize` into **both** `max_delta_cache_size`
  and `big_file_threshold`, so `core.bigFileThreshold` is inert and the only reachable lever also
  resizes the delta cache.

  What was already fixed is the reporting — a hexadecimal error code was the headline; `plainError`
  now says the connection dropped, that nothing was lost, and that it is safe to retry, keeping the
  raw text as the detail that tells the causes apart.

- **A media query adds no specificity, so a width-scoped hide can lose to a utility class.**
  `.panel-toggle { display: none }` inside `@media (max-width: 59.999rem)` and
  `.icon-btn { display: grid }` at the top level are both (0,1,0). They tie, and **source order
  decides** — `.icon-btn` sits ~300 lines lower, so it won: the collapse chevron rendered on every
  phone and did nothing there, because a bar has no panel to collapse. Reported 2026-09-08.
  **This was the second instance**, and the first one's lesson was already written down three rules
  above it (the `.panel-views` rail, invisible at every width from the day it was added) — which is
  why it is now a check rather than a third comment: `ci/checks.sh` fails a bare one-class hide
  inside a `@media` on any element that also carries `.icon-btn`. Qualify the selector
  (`.topbar .panel-toggle`) so specificity decides rather than position. A bare hide on a class
  with no rival (`.save-label`) is fine — the rule is *outrank your rival*, not *always qualify*.
  jsdom applies no CSS, so the tests that find these buttons cannot tell you they are visible.

- **Narrow layouts hide every `.icon-btn` in the top bar** (`[data-layout='single']` *and* the
  `auto` media query), because those controls live in the bottom `ViewBar` where the thumb is.
  Reusing that class for anything that must stay visible on a phone makes it silently vanish
  there — nearly shipped for the collapsed search button on 2026-07-20.

- **A handler that clears its own trigger removes the only sign it ran.** `getTheirChanges`
  empties `movedVaults` as its first statement, so the "get changes" chip vanishes the instant it
  is pressed — and before 2026-09-08 nothing replaced it for the seconds or minutes the pull then
  took. The user cannot tell that from a dead button. Any control that starts slow work must leave
  something behind that says it started.

- **A helper with no callers here is usually an unfinished feature, not dead code.** `syncing()`
  carried the docstring *"what a global 'syncing…' indicator reads"* and had zero consumers for as
  long as it existed; `needsAttention()` still has none. Deleting such a function as unused loses
  the design note attached to it — check what it was *for* before assuming nothing wants it.

- **A width `@media` placed above the rules it overrides loses, silently.** Both settings sheets
  define `.k`, `.line`, `.assets input`, `.vault` and `.row input` *below* their first `@media`
  block, at equal specificity — so a narrow rule written next to the existing `(pointer: coarse)`
  one compiles, warns nothing, and does nothing. Cost one full screenshot round on 2026-09-08.
  Width blocks go last in the sheet.

- **`.caps` is eight lists of two different kinds, and `.k` labels both.** Only *This machine* is
  key/value; the other seven are choice rows where `.k` is the option's name. A rule written for
  one shape hits all eight — the first narrow attempt drew a left border around every radio in
  Settings. `.facts` now marks the key/value one.

- **`min-width: 0` lets a flex item shrink past its own longest word.** Dropping `.choice .k`'s
  5.5rem reserve to widen the description made "Audio transcription on" print *on top of* that
  description at 390px. The fix is not a smaller reserve but taking the competitor off the row —
  the description wraps to its own line, so the label has no one to be squeezed by. `svelte-check`
  and the whole jsdom suite are blind to this; one screenshot at a stated width is not.

- **A vault that has a destination and has *never* sent raises nothing.** `unpushed` returns
  `None` in that state (there is no tracking ref to count against) and `last_sent` is `None` for
  the same reason, so the backup half of the quiet chip cannot fire — however many notes are
  waiting. Deliberate, and the same ruling that keeps a never-saved vault out of it: *"never" is
  not an age*, and a first run must not meet the loudest alert in the app for having done nothing
  wrong. The Backup panel ("Nowhere to send yet") and the welcome screen own that case. The
  reversal condition is a *count* without a moment — if a surface ever wants "200 notes have never
  left this device", it needs `rev-list --count HEAD` and a different sentence, not this chip.

- **An indicator that a successful auto-save also resets cannot report a backup failure.** The
  standing trap behind `decisions.md`, *an alert that measures saving cannot see sending*: "not in
  history" is `git status`, the quiet chip was `git log -1`, and both are *emptied* by the very
  loop that runs every fifteen seconds. Before adding any alert here, ask what routine success does
  to its input — an alert anti-correlated with its own failure mode is worse than none, because it
  reads as coverage.

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
  whenever `commit_all` or `git_native.rs` changes. *(Run 2026-09-04, for the first time: **green**,
  54 tests across 9 binaries. It was framed here as a CI gap and it was never one — it is a pixi
  task that runs on this machine in seconds. The gap was the habit.)*
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
- **Updating is a manual step, and three parts of it are only as good as the user's care.**
  The archive ships `Update from an older folder.{sh,command,bat}` (`decisions.md#toolchain`), and
  what it cannot do is worth knowing. **The sibling search is a convenience, not a guarantee**: it
  looks one directory up for `formicaria-*` folders with notes, so an old folder kept somewhere
  else is found only when dragged onto the script. **Vaults outside the app folder lose their
  registration** — `vaults.json` is not copied because it stores absolute paths, so the script
  lists those vaults and the user re-adds them; the notes are never touched. **The `.md` merge
  driver is stale until the next commit**: `.git/config` holds an absolute path to the `fm` binary
  beside the old folder's executable, and `ensure_repo` re-points it (or `clear_merge_driver`
  unsets it) the next time `commit_all` runs. And the **Windows `.bat` has been executed by
  nobody**, like `formicaria.vbs` before it.
- **A self-hosted remote may accept attachments larger than the ceiling, and there is no way to
  say so.** `GIT_ASSETS_CEILING` is 100MB decimal, chosen just inside GitHub's 100 MiB wall
  (`decisions.md#vault`) — but Gitea, GitLab and a plain SSH remote have no such rule, and a user
  running one is refused a limit their host would have taken. Accepted deliberately: one
  documented constant beats a second `vault.json` key for one audience, and the restic tier
  carries attachments of any size. The reversal condition is in the decision entry. Note the
  number lives **twice** — `crates/fm-core/src/descriptor.rs` is the authority,
  `ui/src/lib/size.ts` mirrors it so the form can warn first, and `ci/checks.sh` fails when they
  disagree.
- **The restic tier's coverage is narrower than "the vault", and nothing in the code says it
  twice.** `backup()` takes the notes dir + `blobs/` only, so `views/`, `themes/`, `manifest.json`
  and `vault.json` — all vault-root files — are in **git only**. A user restoring from restic alone
  gets notes and attachments with no saved views, no theme and no history. Stated in
  `docs/src/user/backup.md` as of 2026-09-02; if the snapshot's path list ever changes, that page
  is the thing that goes stale.
- **Board column *and card* order are client-side, and column reorder is currently inert
  in the pane workspace** (`localStorage['fm-board-order']`
  and `['fm-card-order']`, both keyed by group-by; card order additionally by
  column value) — view preferences, per-browser, **not** in the vault, so they
  don't sync across machines. Intentional; the vault-side `.view` file would
  change that (still deferred). Column DnD is mouse-only (like card DnD).
- **A `.view` file cannot be *edited* from the UI — created and deleted, yes; re-filtered, no.**
  `save_view` (2026-08-28/29) writes a renderer, a group-by and at most one tag; "Delete view" is
  wired as of 2026-08-30. **Changing what an existing view filters is still not possible from the
  app**, and that is deliberate — a UI over the nine-predicate grammar is a query builder, rejected
  twice (`decisions.md#ui`). A hand-written filter richer than one tag is refused rather than
  flattened. Whoever revisits this: the words for the filter already exist
  (`views::describe_pred`), which is most of an editor's read side. **Rename landed 2026-08-30** as
  its own command (`views::rename_view`) precisely because save-then-delete never trips the
  refuse-don't-flatten guard and would silently destroy a hand-written filter.
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

## Self-update: signed and published, not yet run from a real release (2026-09-11)

The app can check, download, verify and install, and on the desktop go back (`decisions.md#toolchain`).
What stands between that and anyone relying on it is not one gap but several, of different kinds.

- **Releases are signed from v0.5.2**, and that signature has been checked against the listed key. **The
  keys have one copy each, on the maintainer's laptop**: until the signing key has an offline backup and
  the recovery key has left that machine, one lost laptop strands every installed copy.
- **A failed `manifest-sign` still publishes the release, unsigned.** Re-run the failed jobs; do not tag
  again (`decisions.md`, *a failed signature is re-run, not re-tagged*). The step's annotation says which
  of five things went wrong.
- **v0.5.2 carries the updater, so nobody could update into it.** On the desktop the rescue lives in the
  launcher, so a folder gains it only by being updated once — `FM_LAUNCHER` unset refuses. On a phone the
  installed APK must already contain the updater, so 0.5.2 is installed by hand. **The first in-app
  update anyone can take is 0.5.2 to 0.5.3**, and until one has run, the chain is built, not proven.
- **The release page still says the Android app has no auto-update** (`release.yml`'s `body:`). Leave it
  until a device has taken an update, then change it.
- **The Android path has never run on a device.** The bridge's permission screen, the positive-mismatch
  signature check and the `FileProvider` hand-off are reasoned from the API and checked against Tauri's
  source, not observed. The owner's phone is the first place any of it runs.
- **Nothing here has run on Windows or macOS** either — the same standing caveat as `Start
  formicaria.vbs`. The zip extraction *is* checked against the published Windows archive.
- **Going back is desktop-only**, by necessity: Android installs an older version only after an
  uninstall, which deletes the notes kept inside the app.
- **iOS has no update rows at all.** Notify-only is possible and unbuilt.
- **A version that is subtly wrong and noticed late** has no way back once the next update supersedes
  its backup; past that point going back means downloading an older release by hand, which
  `discard_an_index_from_the_future` makes safe.
- **`models.toml` in `program/` is a no-op** on any machine that has enabled the assistant:
  `manifest_path()` prefers `<config>/formicaria/tools/models.toml` forever, so a release cannot change
  the catalogue for exactly the users who have models. Android sidesteps this by rewriting it from
  `EMBEDDED_MANIFEST` on every start.
- **The trust residual:** a CI-held signing key defends against transport tampering, not a compromised
  repository or pipeline.

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

- **"It passes locally" was the input, not the output — and `sh ci/like-a-runner.sh` is the
  answer.** When `ci.yml` was finally dispatched on 2026-09-10 after eight weeks, it failed **eight
  times for eight different reasons**, every one a check that passed only because of something the
  developer's machine already had: a global `git config user.name`; a leftover
  `target/debug/deps/fm` hiding a race in the fixture that copies it; a fast CPU under a 100 ms
  budget that needs 113 ms on a 4-core runner; a `/usr/include` that agrees with conda's compiler
  where the runner's does not; a warm `rust-cache` that meant `libgit2-sys` was never compiled at
  all; a single timing sample read as signal when it was scheduling noise; and a locale, where
  `en_US.UTF-8` sorts `windows_aarch64` before `windows-link` and C collation does not. **Not one
  was a regression** — all were written after the last remote run.

  The script reproduces what a runner does not have. **Blind precisely**: its own first version
  pointed `HOME` at an empty directory and broke `pnpm`, inventing a failure a runner would never
  see — the same mistake as blinding `git(1)` without `libgit2`, which produces a false failure in
  `git_differential`. It now links `.local` and `.cache` through and withholds only the git
  configuration.

- **A warm cache hid a broken build for two months.** `libz-sys` probes for a *system* zlib and
  exports where it found it as `DEP_Z_INCLUDE`; `libgit2-sys`'s build script turns that straight
  into an `-I` (its `build.rs:294`). On a GitHub runner that is `/usr/include`, so Ubuntu's glibc
  headers reach a compile driven by **conda's gcc**, whose sysroot carries an older glibc — and the
  errors appear *inside the system headers*, which reads like a broken machine rather than a
  configuration mistake:

      /usr/include/stdlib.h:725:35: error: expected ',' or ';' before '__attribute_alloc_align__'
      /usr/include/stdint.h:41:10: fatal error: bits/stdint-least.h: No such file or directory

  **Four CI runs in a row restored `libgit2-sys` from `rust-cache` and never compiled it.** The
  fifth ran while GitHub's cache service was answering 400s, rebuilt from scratch, and failed. It
  does not reproduce on a developer machine, whose `/usr/include` agrees with conda's compiler.
  Fixed 2026-09-10 with `LIBZ_SYS_STATIC = "1"` in the default activation env, so zlib is built
  in-tree like `vendored-libgit2` and `vendored-openssl` beside it, and no system include directory
  can reach the compile. Left at `"0"` for the Android environment, which cross-compiles against
  the NDK's zlib and works.

  **The general lesson: a cache can hide a build failure indefinitely, and the first cold run after
  an image or toolchain change is where it surfaces.** If a CI build breaks for no reason you
  changed, check whether that run rebuilt something the previous ones restored.

- **The merge-driver fixture raced against itself, and a leftover file hid it for eight weeks.**
  Three `fm-cli` suites copy the built `fm` beside their test binary so `install_merge_driver` can
  resolve `current_exe().parent()/fm`; without it the driver is *cleared* and every "desktop"
  assertion silently measures bare git, which conflicts on the `updated:` line of any two-sided
  edit. The copy went to `fm.{process::id()}.tmp` — **constant within a process**, while the tests
  in a binary are threads — so they raced on one path: `Text file busy` when one execs it as another
  writes, `No such file or directory` when one renames it out from under another. Every error was
  `let _ = …`, so the losers carried on driverless.

  **It passed on every developer machine because a *stamped leftover* `deps/fm` from an earlier run
  made all threads return early**, so no copy was attempted and no race occurred. A fresh runner has
  no leftover, all threads try at once, and exactly the tests that ran before the winning `rename`
  fail — deterministically, twice out of four, identically on repeat.

  Fixed 2026-09-10: a `std::sync::Once` (so late threads *wait* rather than race), a temp name
  unique per process **and** thread (two test binaries share `deps/`), and every step checked
  instead of discarded. **Reproduce a fresh machine by deleting the leftover**, not by trusting a
  green local run:

      rm -f target/debug/deps/fm target/debug/deps/fm.stamp && pixi run ci

- **Three tests passed only because the developer had run `git config --global user.name` once.**
  `set_remote` refuses without an identity — deliberately, because from that point every commit
  carries a name into somebody else's clone. `identity()` reads `git config user.name` **in the
  vault**, and `git config` falls through to the *global* file, so a fixture that never set one
  silently inherited the developer's. On a fresh clone or a runner there is none, and `pixi run ci`
  — the gate `CONTRIBUTING.md` tells every contributor to run — failed on tests that have nothing
  to do with identity. Found 2026-09-10, the first time `ci.yml` had been dispatched in weeks.
  Fixed by giving each fixture its own identity.

  **How to check for more of these:** run the gate with the global config hidden. And hide it from
  **both** git implementations — `GIT_CONFIG_GLOBAL=/dev/null` is a git(1) feature that **libgit2
  ignores**, so blinding only git(1) makes the two backends disagree about the world and produces a
  *false* failure in `git_differential::ensure_repo_leaves_both_vaults_in_the_same_state` (which is
  correct, and was nearly "fixed" on the strength of it). `HOME` must move too:

      pixi run -- env HOME=$(mktemp -d) GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
        cargo test --workspace --features native-git

- **The release notes were thrown away for five releases, and reading them back locally worked
  perfectly.** `release.yml` takes the release message from the annotated tag
  (`git tag -l --format='%(contents)'`) — documented, deliberate, *"impossible to forget, because
  writing it is part of cutting the tag"*. But `actions/checkout` fetches
  `+<commit-sha>:refs/tags/<tag>` — **the commit, not the tag** — so the runner's tag is
  **lightweight**, and `%(contents)` on a lightweight tag returns the *commit's* message.
  v0.3.0 through v0.5.1 each published their `chore(release):` commit, `Co-Authored-By` trailers
  and all, where the notes should have been. `fetch-depth: 0` does not help: it deepens history, it
  does not turn a lightweight tag into an annotated one.

  **Why nobody noticed.** The mechanism was documented, the annotation really was written every
  time, and `git tag -l --format='%(contents)' vX.Y.Z` on a laptop prints it correctly — the bug
  exists only on the runner, and the published page was never read back. Fixed 2026-09-10 by
  fetching the tag object explicitly and refusing to use a lightweight tag's contents at all
  (it now degrades to *no* notes, which is visibly wrong, rather than to a developer commit
  message, which looks deliberate). `ci/checks.sh` guards both halves. The four affected release
  pages were repaired by hand from the annotations, which were still sitting in the tags.

  **The general lesson: verifying the input is not verifying the output.** Every check here was on
  the tag, and the tag was always right.

- **`git tag -a -F` eats the release message's headings**, and the release publishes without them.
  The message lives in the annotated tag (`release.yml:303-310` reads it back with
  `%(contents)`), and the house format opens `### What's new in X.Y.Z`. `-F` defaults to
  `--cleanup=strip`, which discards every line starting with `#` — so all three `###` headings
  vanished from the v0.5.0 tag, silently, exit 0. **Use `--cleanup=verbatim`.** Caught 2026-09-09
  only because the tag was read back before being pushed; pushing is what publishes, so there is no
  second chance. Verify with `git tag -l --format='%(contents)' vX.Y.Z | grep -c '^### '` — that
  reads it exactly as the workflow will.

- **A test fixture cached by *existence* expires silently, and the test keeps passing.** Three
  `fm-cli` suites copy the built `fm` beside the test binary so `git::ensure_repo` can find the merge
  driver — without it `install_merge_driver` *clears* the driver and the "desktop" half of every
  two-device test measures **bare git**, not our code. The copy was `if !dst.exists()`, so whatever a
  previous run left in `target/debug/deps/fm` answered for ever: found 2026-09-07 to be **six weeks
  old**. Nothing failed. `a_clean_merge_is_byte_identical_on_both_devices` even carried a doc comment
  explaining that a `merge_texts` mutation leaving it green was *correct rather than a gap* — and
  once the fixture was fresh, that mutation failed it, and so did a real cross-device divergence
  (`decisions.md`, 2026-09-07). Anywhere a test writes a fixture into `target/`, ask what happens on
  the thousandth run rather than the first.

  **And "stale" cannot be decided by mtime here, which is the second half of the same trap.**
  `pixi run ci` runs `test` (no `native-git`) and `test-native-git` as separate tasks, so cargo
  alternates two different `fm` builds through `target/debug/fm` — and restoring a *cached* artifact
  moves its mtime **backwards**. Measured the same day: a 102 MB native build stamped 18:59 sitting
  beside the 57 MB plain one stamped 18:57, in that order. An "older than the source" test therefore
  reports fresh for ever. The fix is a **stamp** of (length, mtime) compared for *equality*, plus
  executing the new copy before renaming it into place — the two tasks can also be mid-swap on the
  source while a test reads it, and a truncated driver is the quietest failure in the tree: git takes
  the failed exec as "conflict" and hands back `%A` untouched, so the merge yields one side, no
  markers, and no error anywhere.

- **`pixi run ci` still flakes under load, and raising the timeout again is not the answer.**
  Observed 2026-09-07: a full gate run failed with two UI tests timing out at 20 s
  (`ingest.phone.test.ts`'s oversized-file case and `App.features.test.ts`'s editor open), and the
  run reported **280 s of test time against ~100 s** for the green runs either side of it. Both
  files passed alone; `pixi run test-ui` passed clean; the next full gate passed. So it is
  contention, not logic.

  **The project has already been round this loop.** `ui/vitest.config.ts` records the first pass:
  both budgets sat at 5000, a `findBy*` that needed to retry consumed the whole test budget, and
  the failure read `Test timed out in 5000ms` at the `it(...)` line with no element name and no
  DOM. That was fixed by ordering the two budgets and raising `testTimeout` to 20 s. **It has now
  been exceeded too**, which says the budget was never the variable — machine load is, and there
  is no number that outruns it.
  What would actually help is bounding the concurrency (vitest's `poolOptions`/`maxWorkers`) so a
  gate run costs wall-clock instead of a coin toss, or splitting the two `App.*` suites that mount
  the whole app. **Do not simply raise 20 s to 40 s**: that is the treadmill this entry exists to
  stop, and a gate that fails on a train is a gate people learn to ignore (`fetch.rs`).

- **The app has no URL routing, so a screenshot tool cannot ask for a view.** Which view is open
  lives in `localStorage`, and the app correctly sets `frame-ancestors 'none'` — so
  `chromium --screenshot` reaches the default view and nothing else, and the obvious iframe trick is
  closed. That is why `ci/shots.py` is a ~150-line CDP driver rather than a one-line invocation. Its
  own trap, which ate three attempts: **`--virtual-time-budget` does not survive a redirect.**

- **A hand-written mock can contradict itself, and `tsc` will not say a word.** `ui/src/lib/mock.ts`
  answers ninety-odd arms through a bare `as T`, so two arms may report different values for one
  fact and still typecheck. It happened: the `config` arm hardcoded `repo: null` and
  `restic_password_set: false` while `set_restic_repo`/`set_restic_password` wrote the mock's state
  and `backup_status` read it — you could save a repository in the backup panel under `pnpm dev`
  and watch Settings go on reporting none. **A type annotation would not have caught it**: both
  shapes are `string | null`, so `satisfies Config` is satisfied by the wrong answer. What catches
  it is asserting two arms answer the same question the same way after a write
  (`mock.contract.test.ts`). Fixed 2026-09-07; the shape is what to watch for, not the instance.

- **A count in a comment is a claim, and it rots silently.** `dispatch.rs` said *"five arms exist
  precisely to drop the lock before doing slow I/O"* and named them. It was true when written. Then
  `activity` turned out to be a sixth that did **not** drop it — and the comment's confident count
  is part of why nobody looked, because a list that names five reads as exhaustive. `ingest_unlocked`
  later became a seventh, and by 2026-09-05 the same sentence had been copied verbatim into two
  test files, so the wrong number was asserted in prose in three places and matched by none of them.
  The same shape bit `ci/checks.sh`, whose header claimed *"97 tests skip … 72 on git, 7 on restic"*
  when the real figures were 143 and 9 on the day it was written — and 72 + 7 is not 97.
  **Describe the discipline, not the tally.** If a number is genuinely load-bearing, make CI count
  it: `commands.md`'s arm count is checked against `dispatch.rs` for exactly this reason, after
  saying 81 for as long as there were 85.

- **A test that does not touch a process-global still races on it.** `fm-core/tests/backup.rs` had
  one test setting `RESTIC_CACHE_DIR` to a `TempDir` and four that set nothing — which was fine,
  because with a single setter there was no race and the other four quietly used the real
  `~/.cache/restic`. Adding a *second* setter on 2026-09-04 broke **the four that had not
  changed**: they inherited a path whose `TempDir` had already been dropped, and restic failed on
  a cache directory that no longer existed. The failures moved between runs, which reads like four
  unrelated flaky tests rather than one shared variable.
  **Every restic test now takes `restic_cache()`**, which holds a mutex and hands back its own
  directory. The cost is stated in its doc comment: the file serialises, ~11 s → ~34 s. Same shape
  as `secrets.rs`'s `ENV` lock and `backup_records_everything.rs`'s `FM_VAULTS` one — this is the
  third instance, so treat *"a process-global in a test"* as needing a lock by default.


- **`adb shell` re-parses your command on the device, and a guard written without a negative
  control cannot tell you.** `ci/android-smoke.sh` was run for the first time on 2026-09-04 and
  failed at the seed step. Two bugs, stacked, and the second hid the first:
  1. It looked for the vault at `files/vaults/notes/notes`. There is no `files/`: Tauri's
     `app_data_dir()` on Android **is** `/data/data/<pkg>`, and `configure_paths`
     (`mobile/src-tauri/src/lib.rs`) puts the root at `<that>/vaults`. `run-as` lands there too.
  2. The test was `adb shell run-as $PKG sh -c "[ -d $dir ]"`. `adb shell` **joins its arguments
     and hands the string to the device's shell, which re-parses it** — so the brackets arrive as
     separate words and it dies with `[: missing ]`, **exit 2, for every path including a correct
     one**. The seeding write had the same disease one layer worse: `>` was applied by the *outer*
     shell, whose working directory is `/` and not the sandbox, so it wrote nothing and could not
     have. The fix is to quote the whole command as one string:
     `adb shell "run-as $PKG sh -c 'printf ... > path'"`.

  So the wrong path in (1) was undiagnosable from the failure message, because (2) guaranteed the
  same message either way. **Verify a device-side guard in both directions** — `test -d` on a
  directory that exists *and* on one that does not — before believing it. `android-smoke` now
  passes: four launches, all painted, deviation ~25.8 against a threshold of 10.


- **A test can pass because of the machine's ambient git configuration, and the maintainer's
  machine is the one least able to notice.** `acquire.rs` asserted *"a typo is not an auth
  problem"* — that `probe_remote` on a nonexistent GitHub URL returns `unreachable`, not
  `needs_auth`. It passed here for seven weeks and fails on a fresh machine, because **GitHub
  answers an anonymous `git-upload-pack` request with `401 WWW-Authenticate: Basic` for a private
  repository and for one that was never created, identically** — deliberately, so a 404 cannot be
  used to enumerate private repos (measured 2026-09-04 against both). With a credential helper git
  authenticates, gets a real 404, and the typo is named correctly. **With no helper the two cases
  are genuinely indistinguishable**, and `needs_auth` is the honest answer. So the assertion is a
  claim about the *machine*, not about the code. It is now conditioned on
  `fm_core::git::credential_helper().is_some()` and skips with a reason otherwise.
  **The general form: run the suite once with `GIT_CONFIG_GLOBAL=/dev/null
  GIT_CONFIG_SYSTEM=/dev/null GIT_TERMINAL_PROMPT=0` before believing anything it says about a
  remote.** The same run caught a second one — the online clone test had been pointed at this
  project's own (then private) repository and was passing through the maintainer's credential
  helper, so it was not testing an anonymous clone at all. It uses `octocat/Hello-World` now,
  overridable with `FM_TEST_PUBLIC_REPO`.


- **`ci/ios-smoke.sh` was written blind, and rung 3 has still never been dispatched.** Rungs 2 and
  4 *have* run (2026-09-03: built, installed, launched, painted — `decisions.md` carries the job
  number), so this entry's original *"never executed anywhere"* no longer holds; what does hold is
  everything below about why a first run of any new rung is a coin toss, and rung 3 — the one that
  would prove the editor, the whiteboard and `fmblob:` — is that first run. It was
  written on Linux against the pinned tauri-cli v2.11.4 source, not against a run: there is no Mac
  here, and `mobile/src-tauri` cannot even be `cargo check`ed on this machine (a Linux check dies in
  `libdbus-sys`, and an iOS target needs Xcode for the vendored C). `sh -n`, a read-through, and the
  Darwin refusal path are the whole of the local verification — except the parsers, which **are**
  covered: `sh ci/ios-smoke.sh --self-test` runs the three `simctl` text filters against captured
  fixtures, needs no Mac or network, and `ci/checks.sh` runs it on every commit. So: **expect the first `rung2` dispatch to fail on something
  mechanical** — a `simctl` format, a path — and read that as the script being new, not as iOS being
  closed. An adversarial review already found four such defects before the first dispatch, including
  a blank-screen check that would have *passed* on the home screen. The kill criterion is about the
  *app* (blank screen **and** an empty pty), not about the harness. Everything the script needs is
  discovered at runtime rather than hardcoded, precisely because of this.
- **`std::env::set_var` in a test is process-global, and cargo runs a binary's tests in parallel.**
  `pixi run ci` — the single gate — failed roughly **1 run in 8** for as long as
  `crates/fm-core/tests/git_transport.rs` had two tests. One sets `GIT_CONFIG_COUNT`/`KEY_0`/
  `VALUE_0` to simulate a permissive `~/.gitconfig` and removes them afterwards; the sibling's
  `git init` would land in the window where the count and key were still set and the value was
  gone, and git refuses that outright (`missing config value GIT_CONFIG_VALUE_0`). It reads like a
  git or environment bug and is neither. **Measured 2026-09-03: 1/8 failures parallel, 0/4 with
  `--test-threads=1`, 0/20 after the fix.**
  **The instructive part is how it got there.** The file's header already carried the invariant —
  *"One test, not several: it manipulates process-wide environment, so it must not race a sibling"*
  — and a second test was added anyway. A comment cannot stop that; a `static ENV_LOCK: Mutex<()>`
  both tests take can, and now does (poison-recovering, so a failed assertion reports itself rather
  than cascading). **Any future test in that file must take the lock**, and any *other* test binary
  that touches the environment needs its own.
- **A rust-cache can report `full match: true` and save you nothing — check *which* directory it
  hit.** Measured from run **91415007453** (rung 3, cancelled at ~17 min), and this table is the
  antidote to guessing at CI cost, which was done twice here before anyone read a log:

  | Step | Time |
  |---|---|
  | `setup-pixi` (**cache miss**) | 59s |
  | `rust-cache` restore — **full hit**, 2s to restore | 6s |
  | the frontend (three `pnpm install`s) | **15s** |
  | `boot it, twice` — of which: init+brew 26s, **build 464s**, sim boot 86s, install 87s, two launches 38s | **786s** |
  | upload-artifact + job teardown | 161s |

  **The build is 60% of the job and it was uncached.** `mobile/src-tauri/Cargo.toml:6` — *"Outside
  the workspace on purpose"* — so the iOS app compiles into `mobile/src-tauri/target`, while
  `Swatinem/rust-cache` was told only `workspaces: . -> target`. It hit perfectly on a directory the
  iOS build scarcely touches, and vendored OpenSSL, libgit2 and SQLite were rebuilt for the
  simulator triple every dispatch. Fixed by listing both workspaces in rungs 2 and 5 (rungs 1 and 4
  do not build the mobile crate). **Watch the cache size**: that target dir is multi-GB, GitHub
  evicts LRU past 10 GB per repo, and an evicted cache is indistinguishable from no cache.

  **The corrections this forced**, recorded because both were asserted here before being measured:
  the pixi pin below is worth about **50 seconds**, not minutes; and the missing pnpm store cache
  was worth about **15 seconds**, not the "minutes per run" it was called when it was found. Both
  fixes are right and both were oversold. **Read the per-step timings before naming a cost driver.**
- **Every pixi cache miss since this repo had CI was one unpinned input.** Reported by the owner as
  *"I always see pixi cache misses"* (2026-09-03) — correctly, and after it had been waved off once.
  `setup-pixi@v0.8.1` (`src/cache.ts`) keys on
  `sha256( sha256(pixi.lock) + sha256(environments) + sha256(the pixi binary) + path + cwd )`
  and calls `restoreCache(paths, key, undefined, …)` — **the third argument is `restoreKeys`, so
  there is no prefix fallback**: exact match, or a full miss and a full reinstall. `pixi-version`
  was unset in all eight blocks, so the action fetched **latest**, and every pixi release (roughly
  fortnightly) rotated the key for **every workflow at once**. Manually-dispatched workflows fare
  worst: the gap between two `ios.yml` dispatches exceeds the gap between two pixi releases, so they
  essentially never hit. Now pinned to `v0.72.2` everywhere and CI-guarded (one version repo-wide;
  every block pinned). **Raising it is a deliberate, all-blocks-together edit that costs one cold
  install per workflow** — that is the price of the pin, and it is worth paying.
  Two related facts worth keeping in the same place, because they make misses look mysterious:
  a **cancelled** job saves no cache at all (the post-step never runs), and `Swatinem/rust-cache`
  defaults `cache-on-failure` to **false**, which is why the iOS jobs set it explicitly.
- **A simulator that cannot boot is indistinguishable from one that is slow, and `simctl` will wait
  forever.** The first `rung=3` dispatch (2026-09-03) was **cancelled by the owner past 20 minutes**
  with no end in sight, against a measured band of 2.8–7.7 min. It was not the cache and not the
  build — the log shows `formicaria.app` built and both launches painted. The sweep resolved
  `FM_IOS_SIZES` against **`simctl list devicetypes`**, which is what Xcode *knows about*: Xcode 26
  still lists `iPhone-SE-3rd-generation`, so `simctl create` succeeded, no installed iOS 26 runtime
  would pair with it, and `bootstatus -b` — **which has no timeout** — sat. The guard above it only
  ever handled *"no such device type"*. Then the second size created and cold-booted another fresh
  device, so the cost compounded.
  **Fixed three ways, all in `ci/lib/simctl.sh`:** sizes now resolve against `simctl list devices
  available` (the list that has already paired device *and* runtime, so a match boots and a miss is
  an instant skip); the runner's own pre-created devices are reused, removing a create and a cold
  boot per size; and **every** boot goes through `boot_with_deadline` — including the primary one,
  which was equally unbounded. `pick_device_by` is self-tested against the real device list from
  that job, including the case that bit: `iPhone-SE` must come back **empty**, not as some other
  device. **The generalisation worth keeping:** on a billed runner, every wait needs an upper bound,
  and so does the give-up path — `simctl shutdown` has no timeout either, so it is fired and
  forgotten rather than waited on.
- **`＋ Media` would have crashed the app on iOS, and nothing would have said why.** Found by audit
  on 2026-09-03, before anyone was asked to install anything. On iOS a missing `NS*UsageDescription`
  is **not a denied permission — the system terminates the app** the moment the API is touched, and
  `NotePanel.svelte` renders `＋ Media` on every note being edited with **no platform gate**: Record
  audio reaches `getUserMedia`, Take a photo and Record a video reach the camera, and the file items
  reach the photo library. Four keys, none declared. Two taps from a tester, and it would have read
  as "your app is broken" — correctly. `ci/ios-inject-plist.sh` injects them; `ci/checks.sh` guards
  the injection on Linux; `ci/ios-package.sh` asserts they survived into the built `.ipa`.
  **The wording of each is user-visible** — it is what the system prompt shows — so it says what is
  accessed and when, rather than being a string that merely satisfies the linker.
- **The iOS `.ipa` is the first artifact here meant for users that has never run on its target.**
  `ci/ios-package.sh` (rung 5) is written blind against tauri-cli v2.11.4's source, exactly as
  `ci/ios-smoke.sh` was, and expects the same first-dispatch mechanical failures. Its parsers are
  self-tested (`--self-test`, run by `ci/checks.sh`); nothing else in it is. **What no job here can
  ever answer:** whether the `.ipa` re-signs under a free Apple ID, installs, launches, or syncs on
  a physical iPhone. Do not let "rung 5 is green" become "iOS works" — it means the file is the
  right shape. Three consequences follow, and none of them is fixed:
  - **G5 — container-relative vault paths. Fixed 2026-09-03, hardware-unverified.** A managed
    vault persists as `@root/<name>` and resolves against the current root at read time, and a
    stale absolute container path is healed on read for anyone on a pre-marker build
    (`decisions.md`, *a managed vault persists as `@root/<name>`*). **Still unverified on hardware
    like everything else here** — the tests prove the resolution, not that iOS moves a container
    the way this assumes.
  - **LAN pairing — key added 2026-09-03, and the entry it replaced was partly wrong.** It claimed `NSBonjourServices` was needed too. Reading the code says
    otherwise: `fm-serve/src/share.rs:357-370` advertises `<hostname>.local` and the phone
    **resolves** that name. `NSBonjourServices` is required for *browsing* services
    (`NWBrowser`/`NSNetServiceBrowser`), which nothing here does — and inventing a service type to
    satisfy a key we do not need is the exact class of guess this project keeps paying for.
    `NSLocalNetworkUsageDescription` is injected by `ci/ios-inject-plist.sh`. **Still unverified on
    hardware**: a Simulator does not enforce the permission, so no rung can prove the prompt appears
    or that resolution works behind it. git-over-HTTPS sync is unaffected (rung 4 proved it).
  - **The app must never acquire a free-team-forbidden entitlement** — App Groups, keychain
    sharing, push, iCloud, associated domains, Sign in with Apple, Apple Pay. Ticking a capability
    box writes one, and it would make the artifact unsignable by every user it exists for.
    `ci/ios-package.sh` asserts their absence; that assertion is the only guard.
- **`ci/ios-logs.sh` has never completed a real fetch** — only its refusal paths are exercised (no
  credentials, and a bad token: both exit 1 with a usable message and leave no directory behind). It
  is *not* billed and needs no Mac; it needs `gh` logged in or a `GH_TOKEN`/`GITHUB_TOKEN` with
  `actions:read`, neither of which exists in an agent session here. If it misbehaves, downloading the
  run's zip from the Actions page by hand is the fallback and costs nothing.

- **A component test cannot tell you a thing is visible, and neither can grepping the bundle.**
  Demonstrated expensively on 2026-08-30: the panel's view rail shipped `display: none` at every
  width and survived two commits. It had `display: flex` inside `@media (min-width: 60rem)` and
  `display: none` in a base rule *later* in the file — **a media query adds no specificity**, so the
  two tied and source order decided. The component test found the `<nav>` and its buttons and passed
  throughout, because **jsdom applies no CSS**; the "verification" then grepped the built bundle for
  the class name, which proves the markup ships and says nothing about whether it renders.
  **There is no headless browser wired into this project**, so anything whose failure mode is
  *invisible* — a layout, a collapsed state, a media query — is eyes-on-a-browser or it is unverified.
  Say "not verified" rather than describing a string match as one. `ci/checks.sh` now fails on a bare
  `display: none` for `.panel-views` specifically; the general hazard has no guard.

- **"Is the app I am looking at the app I just built?" — two ways it is not, and neither looks like
  it.** Cost an hour on 2026-08-30.
  1. **A release binary serves the UI compiled into it.** `FM_UI_DIST` is the dev loop only
     (`main.rs:103`); every release launch serves `UI_ASSETS`. So **`pnpm -C ui build` alone can
     never change what a release binary shows** — only `pixi run build`, and the order matters
     because the embed reads `ui/dist` at compile time. The misleading part is that the process
     start time is *later* than the `ui/dist` build, which makes it look current.
  2. **Relaunching within ~90 s of closing the tab used to hand you back the previous binary.** The
     server keeps the port for up to 90 s after its last tab goes (the watchdog's allowance for a
     throttled beat), and the `AddrInUse` arm handed the browser to whatever was already there.
     Fixed 2026-08-30: `/api/alive` reports the running server's build and a different one is
     refused with instructions rather than silently served.
  **The honest check**, which a timestamp cannot give you: fetch the served asset and grep it for a
  string only the new build contains — `curl -s localhost:8765/` for the hashed `index-*.js` name,
  then `curl` that and look for e.g. `timelineMode` or `kind=thumb`. A timestamp says when a file
  was written; a string says what is inside it.

- **`tauri icon` is nondeterministic for `.icns` only** — still true, no longer a hazard. The
  release script regenerates every icon from `icon-source.svg`, and that one file comes back the
  same 44312 bytes with ~43k of them reordered while every other generated icon is byte-identical
  (measured 2026-08-30, re-measured across four builds 2026-09-09). **Since 2026-09-09
  `ci/android-release.sh` restores it immediately after generating**, so a release build leaves the
  tree clean and there is nothing to sweep up.

  **Why it needed the script and not the warning.** This entry used to end *"do not sweep it into a
  commit with `git add -A` without looking, which is exactly how it got into `9a48a1f`"* — and it
  then got into `94442c5` and `4a3d3dc` the same way, one of them while the script's own header was
  being edited to describe the problem. Three times, against a warning that was already written and
  already correct. Worth keeping as the example: a hazard a person must remember at exactly the
  wrong moment is not mitigated by documenting it.

  Only `icon.icns` is restored. Restoring the whole directory would silently revert a real edit to
  `icon-source.svg`. Whether these should be generated at build time rather than tracked is still
  open.

- **The visual pass is part-done, and these are the parts that are not.** Shipped 2026-08-30:
  hue-derived status colour, `EmptyState` wired into all six renderers that had hand-rolled text
  (Board had none at all), cards showing title *and* preview with a two-line clamp, and the chrome
  in two placements. Then the timeline feed, which brought the **first thumbnail consumer** —
  `?kind=thumb` now exists on both the desktop blob route and the Android `fmblob://` handler (both
  had a parsed-and-discarded query string; the phone's was the `unused variable: query` warning our
  release build printed) — and gave `AssetMissing.svelte` its first caller.
  **Still open:** **the read view still points at full blobs**, so `render.ts:407`'s renderer-kill
  is *narrowed to the feed, not closed* — converting the read view is the larger Android win and
  wants its own measurement; the calendar's urgency is still a colour with no second carrier;
  `data-type` is still inert, so a note and a discussion look identical; and five of seven renderers
  still hand-pick spacing near the token scale without matching it.
  **The landmine for the rest:** there is still no virtualisation anywhere. The feed covers itself
  with a visible 30-post window and a button; anything else added per-card has no such cover and
  multiplies by the whole result set.

- **`libwhisper-server.so` is not 16 KB aligned, and every other bundled library is.** Measured on
  the 2026-08-30 release APK: our own `libformicaria_mobile_lib.so` and all fifteen llama/ggml libs
  report `LOAD align 0x4000`; the prebuilt whisper binary reports `0x1000`. `ci/android-release.sh`
  warns and does not fail. Google states 16 KB page size as a *device* property, so on a device that
  uses it the loader can refuse that library — which would take out **audio transcription only**,
  leaving the rest of the app working, and would look like whisper silently never starting. Not
  investigated: the fix is in how that binary is produced (`ci/android-stage-whisper.sh`), not here.

- **Changing a `VaultAccess` method signature breaks the phone, and nothing local tells you.** The
  trait lives in `crates/fm-agent-run/src/fmserve.rs` (a workspace member, so it compiles in `pixi
  run ci`); its second implementation lives in `mobile/src-tauri/src/agent.rs`, in a crate
  **deliberately excluded from the workspace** so a contributor with no Android toolchain still gets
  a green gate. So `cargo test --workspace` compiles the trait and never the impl, and `pixi run ci`
  passes while the phone will not build. Demonstrated 2026-08-30: `792d819` added `origin: &Origin`
  to `create_proposal` and the mobile impl went unfixed for three commits, surfacing only when
  someone actually built an APK. **After touching that trait, run `pixi run -e android
  android-check`** (or a full `android-release`). The same hazard applies to any trait a workspace
  crate declares and the excluded mobile crate implements.

- **Excalidraw reads `--border-radius-md` / `--border-radius-lg`, which `app.css` does not define**
  (we use `--radius-*`). A user theme that sets the `--border-radius-*` spelling will silently
  restyle the whiteboard and nothing else — verified by reading
  `ui/node_modules/@excalidraw/excalidraw/dist/prod/index.css`, which declares no `--bg`/`--text`/
  `--accent`/`--surface` of its own. This is the concrete reason the theme surface is a *documented
  list of names* (`appearance.ts`'s `SUPPORTED`) rather than "any custom property you like".


- **A jsdom test can never see a Content-Security-Policy, so a policy can forbid a shipped feature
  and every test still passes.** Found 2026-08-29: the app policy carried `frame-src 'none'` while
  `ui/src/lib/render.ts` renders every PDF in an `<iframe>` at `/api/blob/…`, so **no PDF had ever
  rendered** — desktop or phone, both policies had the clause. The symptom is a broken-document
  placeholder: nothing throws, nothing logs where a user looks. `render.test.ts` asserted the
  `<iframe>` element and passed the whole time, because Vitest's jsdom applies no CSP; meanwhile two
  user-facing strings said PDFs were *"stored, opened and shown"*. Fixed and pinned by
  `the_policy_lets_the_read_view_frame_its_own_pdf` in `crates/fm-serve/src/main.rs`
  (`decisions.md#ui` — *the read view may frame its own blob*).

  **The lesson: a CSP clause is a claim about what the app is permitted to do, so it has to be
  tested against the real response header, server-side.** Any UI test that renders an element the
  policy governs — a frame, a worker, a `blob:` URL, a font — is testing the element, not the
  permission. The rest of the policy is *only* covered by
  `every_response_carries_a_policy_that_stops_a_note_phoning_home` and this new sibling; anything
  they do not assert is unverified.

  Two clauses are load-bearing and currently unasserted anywhere: **`worker-src`** (Excalidraw's
  pica resize worker) and **`font-src`** (KaTeX's bundled woff2). Both fail the same silent way.

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

- **The manual tab does not keep the server alive, and that is deliberate.** With
  `FM_AUTO_SHUTDOWN` (which the shipped launchers set), the watchdog quits ~90 s after the last
  authenticated request. The manual makes none — `/manual/*` is served under `MANUAL_CSP`, whose
  `connect-src 'none'` provably forbids it calling anything. So closing the *app* tab while leaving
  the *manual* tab open stops the server, and the manual tab then breaks on its next navigation.
  **Do not "fix" this by injecting a heartbeat into the book**: that would make the manual a live
  client of the vault API, which is exactly what `connect-src 'none'` exists to prevent. Reopen the
  app instead.

- **The busy-port probe identifies formicaria, not *which* formicaria.** `serving_formicaria`
  asks `/api/alive` and trusts a `HTTP/1.1 200`. With two unpacked copies on one machine, launching
  the second opens a browser at the first — which is right for the common case (a second
  double-click) and wrong for the rare one (two installs, two vaults). The message says "if that is
  a different copy, quit it first", which is the honest half-fix; a real one needs `/api/alive` to
  carry something identifying, and it must not be the vault path — `/api/config` is in
  `REMOTE_DENIED` precisely because that discloses the host's absolute paths to a paired device.

- **`fm-app`'s `who_left_a_message_is_read_from_git` goes red intermittently under load.** Seen once
  in a full `pixi run ci` on 2026-08-28 and green on every re-run since. It is not the test in
  isolation: **0 of 40** sequential runs of the binary fail, but a burst of 8x8 concurrent runs
  produced **8 failures (~12%)** — and a second identical burst produced none, so it tracks machine
  load (that first burst followed an Android build) rather than concurrency by itself. The panic is
  always `discussions.rs:95`, *"the discussion has participants"*: `fm_core::vcs::activity(vault,
  "@0")` yielded nothing for a commit `commit_all` had just returned `true` for. **`--since=@0` is
  git's epoch syntax — "everything since 1970" — so this is NOT a time-window race**, which was the
  obvious first guess and is wrong. The mechanism is **unidentified**; it is recorded here so the
  next red is not misread as a regression from whatever was being changed at the time. Re-run the
  binary alone before believing it.

  **Seen again 2026-09-10**, on Dependabot's setup-pixi pull request — the same line, and *not*
  caused by the bump. Two things learned. The assertion now **prints what git actually had**: the
  paths handed to `commit_all`, the participants map, `git log --since=@0 --name-only`, and
  `git status --porcelain`. It said only "the discussion has participants" before, which is why
  three sightings produced no diagnosis. The next red should identify the mechanism by itself.

  And the rate is **much lower than "under load" implies**: reproduced once in 24 parallel runs,
  then **zero in 192** — 96 of them pinned to four cores to mimic a runner. So do not expect to
  reproduce it on demand, and do not conclude from a quiet loop that it is fixed.

  Repro:
  `for i in $(seq 1 8); do target/debug/deps/discussions-* who_left_a_message_is_read_from_git --exact >/dev/null 2>&1 || echo FAIL & done; wait`

- **`tauri icon` rewrites `icon.icns` non-deterministically** — recorded once, above, with what
  the script now does about it. (This was the second copy of the same finding; the two drifted, and
  the shorter one was the one people read.)

- **Line endings are LF, and both halves matter.** `from_file` tolerates CRLF because a
  Windows editor produces it; the repo's `.gitattributes` (`* text=auto eol=lf`) stops git
  producing it in the first place; and `ensure_repo` writes `*.md merge=fm text eol=lf`
  into every vault for the same reason. Remove any one and Windows breaks *silently*: the
  loader is deliberately tolerant, so unparseable notes don't error — they vanish, and the
  vault opens empty. That is exactly how CI found it (all eight `e2e_vault` tests at once,
  because they're the only ones reading committed fixtures rather than writing their own).

- **`.desktop` has no relative `Exec`** — it must be absolute, so the entry cannot be a
  static file in the repo. It was one, carrying `/home/<the author>/...`, which meant every
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
- **`fm-serve` is only partly tested.** The blob route now has real response-path tests
  (a listener on port 0, a live socket: sniffed type, the `nosniff`/attachment allowlist,
  `Range`/206/416, 404) and the query-args split has unit tests. Everything else — the CSRF
  guard, the `Host` guard, static serving, the watchdog — is still exercised only by hand,
  and every UI test runs against `mock.ts`. Narrower than it was; not closed.
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
- **A NUL byte makes a source file binary, and `grep` skips a binary file in silence.**
  `SkippedPanel.svelte` carried a literal NUL inside a template literal, so `file(1)` called it
  `data` and **every `ci/checks.sh` sweep over `ui/src` passed over it without a word** — hiding a
  `76vh` from the guard written specifically to catch it. Escaped to `\0` on 2026-09-01 (same
  value), and every sweep in that section now passes `-a`. The general rule: a repo-wide `grep`
  guard that omits `-a` is making a claim it has not checked, and it fails *open*. Same class as
  the `fm-query` filter that a trailing comment used to disarm.

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

- **`macos-latest` is macOS 26 (Tahoe) as of 2026-09-09.** Read off a rung-5 log: Homebrew poured
  `arm64_tahoe` bottles. It matters because that image's **awk enforces POSIX on `-v`
  assignments** where gawk, mawk and busybox awk do not — which is what killed rung 5's second
  ever run (`ci/ios-inject-plist.sh`, fixed the same day, and `ci/checks.sh` now refuses the
  construct textually because no awk here can refuse it behaviourally). Rung 5's only green run,
  2026-09-03, was on the previous image. **Anything an iOS rung "proved" before 2026-09-09 was
  proved against a different macOS**, and the runner image moves without notice.

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

### `has_conflict_markers` hardcodes seven, and a vault can ask git for fewer

`merge::has_conflict_markers` — the **one** definition of "still conflicted", used by the conflict
list and by the guard that refuses to stage marked-up text — tests `starts_with("<<<<<<<")`. Git
takes `conflict-marker-size` from `.gitattributes` and honours it, and `write_gitattributes` only
ever *appends*, by design, because Track V's whole case is a repo you already own. So on a vault
carrying `* conflict-marker-size=3`, the driver is handed `%L=3`, git writes `<<< ours`, and
`has_conflict_markers` answers **false**: `commands::conflicts` omits the note, and
`resolve_conflict(_, _, Keep::Edited)` stages a body containing `<<< ours` as the note's content —
the exact failure that definition exists to prevent.

**Pre-existing**, found by the §2.14 audit (2026-09-08). Not fixed there because the honest fix is a
run-length rule (`<{3,}`), and loosening the tolerance is a decision of its own: the doc on that
function deliberately requires both an opening *and* a closing marker so that a note **writing
about** merges is not flagged, and this repo's own notes do that. `>>> ` is also a Python prompt.
Whoever fixes it should extend `sentence_merge.rs`'s `a_marker_of_any_size_still_owns_its_line` to a
size **below** seven — it currently runs 7/12/32, so its closing assertion passes for the wrong
reason.

### `settle_the_paths_libgit2_merged_itself` stages before it checks the outcome

`git_native.rs`'s second merge loop calls `index.add_path` unconditionally and only then tests the
outcome, where its sibling 120 lines above puts the `add_path` inside the `Clean` branch. On a
conflicted outcome the path is staged at stage 0 with its markers, so `conflicts()` — index-derived
— reports nothing for it and the next auto-commit could commit `<<<<<<<` as a note's content.

**Not simply the sibling's fix.** That loop handles paths libgit2 merged *itself*, so the index has
no conflict stages to leave standing; skipping `add_path` would leave the index holding libgit2's
answer while the working tree holds ours. Marking it unmerged means *creating* conflict stages,
which is a merge-index change and not one to smuggle in beside a formatting feature. The audit could
not construct a reaching input (a clean whole-file libgit2 merge implies a clean body merge on these
inputs), which is why this is recorded rather than urgent.
