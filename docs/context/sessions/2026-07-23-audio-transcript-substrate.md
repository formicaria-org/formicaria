# 2026-07-23 — Audio→transcript, end-to-end (laptop v1)

First multimodal specialist, built in the plan's mandated order: **the shared safety substrate lands
before any specialist can write**, then the whole path is wired — command → runner → whisper leaf →
proposal → UI. The substrate + glue are hermetic (no model/network/runtime) and ride `pixi run ci`;
the real whisper runtime is manual-only (laptop). See "honest gaps" for what has *not* been run.

## What shipped (crate `fm-agent`, new file `src/transcribe.rs`)

- **`Transcribe` trait** — `fn transcribe(&self, audio: &[u8], mime: &str) -> Result<String>`. The
  seam mirrors `LlmStep`/`WebSearch`. The signature *is* the load-bearing invariant: a specialist
  receives audio **by value** and nothing else — no blob path, no store handle — so a wedged or
  malicious specialist **cannot corrupt the device's only copy** of a content-addressed blob. The
  blob layer hands out paths (`fm_app::commands::blob_path`); the *orchestrator* reads bytes
  read-only and passes them in.
- **`Provenance { specialist, model, blob_hash }`** + `key()` = the idempotency key
  `(blob-hash|specialist|model)`.
- **`transcript_block`** — a provenance-marked **callout** adjunct (`> [!note]`) that *references*
  the source (`asset:sha256-<hash>`, which also keeps the blob alive under the on-demand refcount
  scan), fenced by HTML-comment markers. Untrusted transcript text is defanged (any `fm:transcript`
  marker neutralized) and block-quoted so it can't forge the fence or escape the callout.
- **`insert_or_supersede`** — pure splice: same-key re-run **supersedes in place**, a different model
  version **appends a second** adjunct, all other content untouched (insertion-only, as a string
  transform).
- **`transcribe_into`** — the whole flow as pure orchestration over the seam; returns the new body
  for the caller to turn into a proposal via `create_proposal`. Creates nothing, deletes nothing,
  touches no blob.
- **`WhisperServer`** (the real leaf) — POSTs the audio as `multipart/form-data` to a loopback
  `whisper-server` `/inference`, parses `{"text":…}`. Same hand-rolled TLS-free HTTP as the other
  seams; the server never sees a vault path (invariant holds across the process boundary too).

## Tests (8, all hermetic, in `transcribe.rs`)

provenance+source-reference present · insertion-only preserves host content · same-key supersede
(no duplicate) · different-model appends · **forged-fence transcript can't truncate the note** ·
**a wedged specialist cannot corrupt the source blob** (real temp file stands in) · `parse_transcript`
· `WhisperServer` posts multipart audio over a real loopback socket.

## Wired end-to-end (laptop v1) — all CI-green

- **Command**: `/transcribe <asset-ref>` in `convo::Intent` (new `transcribe: Option<String>` +
  `asset_ref` recogniser). Routed in `Agent::handle` to a new `transcribe_turn` (fm-agent-run/lib.rs)
  that reads the blob **by value** via the seam, runs `WhisperServer`, and lands the transcript as a
  **proposal** on the host note — the same PR cycle as `/research`. Degrades gracefully (no asset →
  asks; no runtime → "not on this device"; non-audio → says so).
- **Vault seam**: `VaultAccess::blob_bytes(reference) -> (Vec<u8>, String)`. Desktop `FmServe` does a
  binary-safe `GET /api/blob/<ref>`; in-process `DispatchVault` (mobile) resolves `dispatch::blob_path`
  + reads bytes. The runner reads; the specialist only ever gets bytes.
- **Runtime (desktop)**: `agent-serve --whisper-port <p> [--whisper-model ggml-base.en]` launches a
  second supervised `whisper-server` beside `llama-server`, killed on exit (no orphan). `models.toml`
  gained a `[whisper]` block; `agents/fetch-whisper.sh` + `pixi run fetch-whisper` stage the weights.
- **UI**: `transcribeCommand()` helper (tested) + a **Transcribe** action in the asset options menu,
  shown only when `props.mime` is `audio/*`; it composes `@agent /transcribe <ref>` and sends it.
- **Tests**: fm-agent transcribe (9), convo (+3), fm-agent-run runner→fake-whisper-socket→proposal (2),
  UI agentCommands (+2). Full `pixi run ci` green.

## Real run — DONE (gap closed)

The audio→transcript path is proven with the **real** whisper.cpp binary, not just the fake:
- `models.toml` `whisper_runtime_url` now points at whisper.cpp **v1.9.1 `whisper-bin-ubuntu-x64.tar.gz`**
  — a prebuilt, CPU-only, portable `whisper-server` (~24 MB, no CUDA, links only stock system libs;
  it also bundles a `parakeet-cli`). `pixi run fetch-whisper` stages it + the `ggml-base.en.bin`
  weights, exactly like the llama.cpp runtime — **no build-from-source** (my earlier "no portable
  Linux server tarball" claim was wrong; see [audio-asr-research-2026-07-23.md](../audio-asr-research-2026-07-23.md)).
- Verified end-to-end: the **shipped `WhisperServer` Rust client** drove a live `whisper-server` on
  `jfk.wav` and returned the correct transcript ("And so my fellow Americans, ask not what your
  country can do for you…"). Harness: `crates/fm-agent/tests/live_whisper.rs` (`#[ignore]`d manual
  test, `WHISPER_PORT` + `WHISPER_WAV`), the model-leaf equivalent of `live_research.rs`.

## Direction (principled audit, 2026-07-23)

Scored the ASR options against the owner's principles (stdlib-over-pip, no native blob in the app,
simplicity/least-machinery, tiny edge, unified devices): **whisper.cpp sidecar now** (reuses the
llama-server pattern + shipped client, zero Python, zero build), **pure-Rust candle as the eventual
end-state** (only option with no native blob on *both* devices and no Python). The Python-ONNX routes
(sherpa-onnx, useful-moonshine-onnx) are **rejected** — Moonshine is the nicer model but its only clean
runtime needs pip+onnxruntime, which the stdlib/simplicity principles disfavor for too small a size win.

## Real end-to-end run — DONE, and it caught a bug

Drove the whole flow with **real components**: an isolated fm-serve (scratch vault via `FM_VAULTS=`,
so the real config was untouched — note: `FM_CONFIG_DIR` is Android-only on Linux, the list lives at
`~/.config/formicaria/vaults.json`) + the real prebuilt `whisper-server`, through the shipped `FmServe`
client (harness `crates/fm-agent-run/tests/live_transcribe.rs`). Proven: `FmServe::blob_bytes` over the
real `/api/blob` route, real whisper, and a provenance-marked **insertion-only proposal** whose branch
holds the original note + the adjunct with the correct transcript (verified via `git show` on the
`proposal/…` branch; `master` untouched).

**Bug it caught (fixed, `90f7506`):** the Transcribe action was on the *asset note*, but a proposal can
only target a note → `create_proposal` 500'd. Fix: offer **Transcribe on a regular note that embeds an
audio asset** (resolve each `asset:sha256` ref's MIME, take the first audio one), proposing into that
note; removed it from the asset-note menu. This is the plan's real "note with a recorded sound" scenario.

## In-app recording — the full loop, any device (`a6500c8`)

The record→transcribe→proposal→(edit/reject/accept) loop is now doable **entirely from the UI**, and
device-general by construction (not a laptop-only hack):
- `ui/src/lib/record.ts`: getUserMedia + an `AudioContext` pinned to 16 kHz → raw PCM → a **16-bit mono
  WAV `File`**. WAV, not `MediaRecorder`'s webm/opus, because that's what whisper.cpp reads directly
  (no server-side ffmpeg). Pure `encodeWav` unit-tested (`record.test.ts`).
- `NotePanel`: a "Record audio" item + a live indicator (Stop & add / Cancel). The clip flows through
  `ingestAll` — the **same embed path as attach/drop** — so a recorded and an attached clip are
  identical downstream; Transcribe turns either into a proposal `ProposalReview` can edit/reject/accept.
- **Phone**: `ci/android-inject-service.sh` now injects `RECORD_AUDIO` + `MODIFY_AUDIO_SETTINGS`; wry's
  `RustWebChromeClient.onPermissionRequest` already requests them at runtime and grants the WebView's
  `AUDIO_CAPTURE`. Verified: rebuilt APK requests `RECORD_AUDIO`, installed + running on the phone. So
  the *same web code* records on browser and phone; only whisper transcription stays laptop-staged
  (phone whisper = the acknowledged later spike). getUserMedia failure degrades to a plain message.

## Shipped + proven live (record → transcribe → proposal), and the lean redesign

The whole loop was exercised on the real desktop stack and iterated to the owner's taste:
- **In-app recording** (`ui/src/lib/record.ts`) → 16 kHz mono WAV; hardened (native-rate + `resume()` +
  JS resample, insecure-origin guard). Works browser + phone WebView (RECORD_AUDIO via the manifest
  inject; wry grants AUDIO_CAPTURE).
- **`/transcribe` is a command chip, not a per-note button** (owner: keep it lean). A bare
  `@name /transcribe` transcribes **every** un-transcribed audio clip the note embeds, each its own
  provenance block, and **skips clips whose accepted transcript is already in the note**
  (`already_transcribed` keys off the block marker) — so multiple recordings need one command and an
  accepted transcript is never re-done. `AudioWork::{Clips,AllDone,None}` drives the reply.
- **"Audio transcription" is a Settings toggle** beside the assistant on/off. It persists to
  `agent.json` (`{enabled, transcribe}`); fm-serve exports `FM_TRANSCRIBE=1` when spawning the agent;
  `agent-serve.sh` adds `--whisper-port 8082` when that env is set **and** the runtime is staged. Takes
  effect at the next assistant start (whisper is a launch flag on the separate agent process — the
  "close the tab, reopen" restart, matching the "agent dies with formicaria" lifecycle).

### Traps hit + fixed (worth remembering)
- `cargo` is **not on PATH outside pixi** — a raw `cargo build` silently no-ops (masked by a pipe).
  Rebuild release binaries with `pixi run -e default cargo build --release …`, or the desktop keeps
  running stale `fm-serve`/`agent-serve` (this cost hours: whisper "on" but no `--whisper-port`).
- The desktop agent is launched **by fm-serve** (`agents/start-agent.sh`) from the Settings toggle, not
  a manual terminal — so a code change needs the *release* binaries rebuilt AND a relaunch.
- The Transcribe UI must address a **running** assistant (fresh `onlineAgents()`); an unaddressed
  `/transcribe` posts silently and nothing answers.

## Phone-transcription SPIKE — feasibility PROVEN on-device

whisper.cpp ships no prebuilt Android binary, so `whisper-server` must be **cross-compiled**. Done and
verified end-to-end on the real phone (Dimensity 7300):
- **Build (arm64):** `pixi exec --spec cmake --spec ninja` (the env has no cmake; `pixi exec` is
  ephemeral, no permanent toolchain bloat) + the NDK r27d toolchain file:
  `cmake -S whisper.cpp -B build-android -G Ninja
   -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake -DANDROID_ABI=arm64-v8a
   -DANDROID_PLATFORM=android-24 -DWHISPER_BUILD_TESTS=OFF -DGGML_OPENMP=OFF` then
  `--target whisper-server`. Outputs `whisper-server` + `libwhisper.so`/`libggml{,-base,-cpu}.so`
  (AArch64; only libc/libm/libdl external). `llvm-strip` → whisper-server ~800 KB.
- **On device (via `/data/local/tmp`, per the phone standing order):** it executes, loads
  `ggml-base.en.bin` on-device, listens on :8082, and **transcribed jfk.wav correctly in ~5 s** for an
  11 s clip (~2× real-time). Cleaned up after.

### Full integration — BUILT + on the phone
- **`ci/android-stage-whisper.sh`** (a depends-of `android-release`): cross-compiles whisper-server
  **static** (single self-contained binary, no ggml `.so` to collide with llama's) → strips → stages
  `jniLibs/arm64-v8a/libwhisper-server.so`. Idempotent (version-stamped). `android-stage-runtime` now
  preserves it. Verified in the APK (2.3 MB, beside the intact llama libs — note `libllama-server.so`
  is a 4.6 KB *thin launcher*; `libllama-server-impl.so` is the 8 MB real code).
- **`mobile/.../agent.rs`**: when Audio transcription is on (`agent.json {transcribe}`, default off) it
  fetches `ggml-tiny.en` (~75 MB, the phone whisper pick — `whisper_mobile` in models.toml) and launches
  whisper-server beside llama, setting `whisper_port`. **The admission gate is the `SupervisedModel`
  memory preflight** (`Need`): on a phone too tight for both, it fails → transcription stays off, chat +
  notebook unaffected. Its off-switch joins the control state (Settings-off / app-exit frees it). Mid-run
  kill is safe by the substrate: the proposal is only created *after* transcription completes, so a kill
  leaves no partial state.
- **`lib.rs`**: the phone answers `set_transcribe` / `agent_status.transcribe` (transport, like `set_agent`).
- Opt-in by setting (no download/RAM unless enabled); applies at the next assistant start.

**Not yet exercised in-app:** flipping the phone's "Audio transcription" toggle can't be automated from
adb (release APK is non-debuggable), so the final in-app click-through (enable → restart assistant →
first-run downloads tiny.en → record → /transcribe) is a manual step. The binary itself is proven
on-device and the launch path mirrors the proven llama one.

## Still open
- **Standalone audio asset note** (opened directly, not embedded) has no host to propose into → no
  Transcribe there for now; a "companion transcript note" flow could cover it later.
- **A live browser click-through** (vs. the API-level harness) not done; the backend path is fully real.
- **Phone audio** stays a later spike (arm64 `whisper-server` cross-compile + admission gate +
  mid-run-kill consistency); mobile has only the `blob_bytes` accessor, `whisper_port: None`.
- On-demand spawn / kill-to-unload (currently eager when `--whisper-port` is set).
- **Phone audio** stays a later spike (Android admission gate + mid-run-kill consistency). Mobile got
  only the uniform `blob_bytes` accessor; `whisper_port` is `None` there.
- **Whisper is eager-loaded** when `--whisper-port` is set (RAM for the session). On-demand spawn /
  kill-to-unload is a refinement; fine on the laptop, required before the phone.

## Also this session

- Fixed a repo bug: `mobile/src-tauri/icons/icon.icns` showed as a perpetual phantom change because
  `.gitattributes` line-ending-normalized the binary (it was missing from the `*.png/*.ico/…` binary
  opt-out). Added `*.icns binary` + a comment; renormalized. Commit `2166b2a`.
