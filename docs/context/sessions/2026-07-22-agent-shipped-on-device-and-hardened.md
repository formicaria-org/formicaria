# 2026-07-22 — the study agent shipped on-device on both platforms, then hardened

Continues `2026-07-22-agent-on-the-phone-and-the-last-mile.md` (the FFI-one-path decision was
**reversed** — `forward-plan-review-2026-07-22.md` + `agent-backed-apps-runtime-2026-07-22.md`: keep
the model **out-of-process behind the OpenAI/HTTP seam**, FFI at most a feature-gated option). This
session took that decision and **shipped a working agent on the real phone and the real laptop**, then
fixed a long tail of real-usage bugs.

## The shape that shipped — one runner, two thin launchers

**`fm-agent-run` (`serve_loop` + `Agent::handle`) is the single shared runner**, used verbatim by the
desktop `agent-serve` binary and the mobile `mobile/src-tauri/src/agent.rs`. The only platform-specific
code is the launch/transport seam (`VaultAccess`: `FmServe` HTTP vs `DispatchVault` in-process). There
is **no forked feature logic** — an owner concern, checked and confirmed.

- **Phone (in-process):** the arm64 `llama-server` + ggml/llama libs are **bundled in the APK's
  `jniLibs`** (renamed `libllama-server.so`, `useLegacyPackaging=true` so it extracts and is
  exec'able), staged by `ci/android-stage-runtime.sh` (fetch pinned b10081 → transitive NEEDED closure
  → llvm-strip 235 MB→25 MB → arch-check). The runner finds it via `/proc/self/maps` (pure Rust, no JVM
  shim). The **model is fetched on first enable** (`fm_agent_run::fetch`, `ureq`+`rustls`, resumable +
  checksum, behind the `download` feature). A **foreground service** (`ci/android-inject-service.sh`)
  keeps the app's network alive (Doze was stalling the 150–730 MB download). Proven end-to-end.
- **Laptop (subprocess):** `agent-serve` spawns `llama-server` under the watchdog. Runtime is the
  **Vulkan build on the RTX 3050** (llama.cpp ships no CUDA binary; Vulkan uses the proprietary
  driver's ICD, no CUDA toolkit). Measured: qwen3-4b **14→52 tok/s** and off system RAM onto ~2.5 GB
  VRAM.

## Config-driven runtime — `agents/models.toml` is the one config

One manifest, read by `fm_agent_run::manifest`, drives both platforms:
- `default` (laptop pick) + `default_mobile` (phone pick); `threads` + `threads_mobile`.
- `gpu = "auto"` → **GPU by default, CPU fallback**: `fetch.sh` takes the Vulkan runtime when a Vulkan
  loader is present, else the CPU build; `serve` passes `-ngl 99` (harmless on a CPU build). `on`/`off`.
- Per-model `ctx`/`threads` overrides; `max_reply_chars` (default raised 600→2000).

## The models — evidence-based, per device

`model-benchmarks-2026-07-22.md` (real llama-bench + per-process RSS/CPU) and
`model-selection-research-2026-07-22.md` (leaderboards, weighted BFCL×IFEval, code penalised) →
**phone `lfm2.5-1.2b`** (~16 tok/s, ~0.9 GB, ~4 of 8 cores) and **laptop `qwen3-4b-2507`** (GPU).
Key finding: on the phone **4 threads beat 8** (big.LITTLE) — fastest *and* leaves the UI cores.

## Minimal pre/post (owner: pre/post are for SAFETY only; show the LLM as it is)

- **Context** = the host note + the notes it **links to** (`[..](note:id)`, one hop, text only, capped)
  + the discussion. **No vault-wide RAG** — it fed a tiny model unrelated fragments it parroted.
- **Post** = math-delimiter normalisation (`\[..\]`→`$$..$$`, `\(..\)`→`$..$` — the note renders KaTeX)
  + the length cap. **Echo-stripping removed** (cosmetic, and it cleaned real answers to nothing).

## Real-usage bugs fixed (each found on-device)

- **"No reply in a note's discussion."** The watch loop enumerated `discussions()` = first-class
  discussion roots only; a note's comment thread has an ordinary-note root, never listed. New
  `commands::thread_roots` (+ dispatch) returns **every** thread with messages; the agent's
  `discussions()` maps to it. The Discussions *view* still uses `discussions`.
- **Empty replies / "couldn't get a usable answer".** The tiny model was fine (direct call answered);
  post-processing emptied it. Simplified to cap-only (later + math). Empty-reply guard kept.
- **`/proc/loadavg` denied to apps (Android).** Preflight fail-closed on it; now tolerated (memory is
  the governor). `available_parallelism()`=1 on Android → count `/proc/cpuinfo`.
- **@-picker** (owner spec): show **every** collaborator (git authors, incl. past agents) + online
  agents, each with a **green (active) / red (inactive)** dot; on mobile the shell answers
  `agents`/`agent_status`/`set_agent` (transport, never dispatched into the core). The poll that fills
  it was gated on `isDiscussion` → empty in note threads; now gated on `discOpen`.
- **Stale desktop server served the old model** after a config switch. The launcher reused *any*
  running server; now it **restarts a stale one** (running started before the current
  binaries/`ui/dist`/`models.toml`). `FM_AUTO_SHUTDOWN` (close tab → stop) + agent self-stop (~6 s)
  already give clean shutdown. (Gotcha: `pixi run build` rebuilds fm-serve + UI but **not**
  `agent-serve` — `cargo build -p fm-agent-run` for that.)
- **Message delete in threads.** A message is a note, so `deleteNote` works; added a per-message Delete
  (two-click) in the thread view — works in note comment threads too, leaves the host note.

## Mobile lifecycle — on/off + clean shutdown

`agent.json` on/off (Settings toggle), **default on** on the phone (an update must not silently kill a
working agent). Off stops the model now (trips the watchdog off-switch). **App exit** stops it
(`RunEvent::Exit`) and **`PR_SET_PDEATHSIG`** on the child kills it with the app process — no orphan.
Backgrounded-but-alive keeps running (the foreground service). A control state guards double-launch;
`agent.json` cleaned old weights when the default changed.

## State at session end

Both devices run their pick, config-driven, one shared runner. All committed and pushed
(`f334ae4`…`6efa676`, plus the launcher/model/GPU commits). **Deferred:** a **stop button** to cancel a
thinking turn; **broader RAG** (beyond linked notes) — owner: it needs its own research, bad RAG is
worse than none. Re-launch to activate a new build (phone: relaunch; desktop: double-click → the
launcher restarts the stale server).
