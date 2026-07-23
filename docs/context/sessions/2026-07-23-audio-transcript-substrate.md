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

## Honest gaps (manual-only, not run here)

- **The real `whisper-server` binary is not auto-staged** — whisper.cpp ships no portable Linux server
  tarball like llama.cpp's, so `whisper_runtime_url` in `models.toml` is empty and `fetch-whisper`
  fetches only the weights + prints how to place the binary (build from whisper.cpp). `agent-serve`
  errors clearly if it's missing. **A real audio→transcript run has NOT been executed** (needs the
  binary + network; real model leaves are manual-only by house rule) — only the hermetic path is proven.
- **Phone audio** stays a later spike (Android admission gate + mid-run-kill consistency). Mobile got
  only the uniform `blob_bytes` accessor; `whisper_port` is `None` there.
- **Whisper is eager-loaded** when `--whisper-port` is set (RAM for the session). On-demand spawn /
  kill-to-unload is a refinement; fine on the laptop, required before the phone.

## Also this session

- Fixed a repo bug: `mobile/src-tauri/icons/icon.icns` showed as a perpetual phantom change because
  `.gitattributes` line-ending-normalized the binary (it was missing from the `*.png/*.ico/…` binary
  opt-out). Added `*.icns binary` + a comment; renormalized. Commit `2166b2a`.
