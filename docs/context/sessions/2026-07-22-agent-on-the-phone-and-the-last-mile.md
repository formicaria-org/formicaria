# 2026-07-22 — the study agent, proven on the phone; and the decided last mile

The marathon session that took the agent from "works on desktop" to **proven end-to-end on a real
arm64 Android phone** (via adb-shell), then decided how to ship it inside the app. Follows the
adversarial review (`agent-review-2026-07-22.md`) and its execution
(`2026-07-22-executing-the-agent-review.md`).

## Proven on the phone (adb-shell, `/data/local/tmp`, cleaned up after)

The **entire** stack ran on the device and answered an `@`-mention end to end: `fm-serve` (vault
backend + SQLite, cross-compiled for android), `agent-serve` (orchestrator + watchdog + presence),
the prebuilt **android-arm64 `llama.cpp`** server + the 230M model, mention-detection → pipeline →
reply. The two "hard blockers" the review named both dissolved:
- the **portable runner** cross-compiles (after 1.4 dropped fm-core/git/SQLite);
- the **model runtime** is a *prebuilt* `llama-b10081-bin-android-arm64` — **not** the large FFI lift
  it was feared to be.

Each blocker surfaced exactly where predicted and was fixed live (committed `b045ad7` = **1.6**):
- `ResourceMonitor` gated to `target_os="linux"`; Android is `"android"` but has `/proc` → now both.
- `available_parallelism()` returns **1** on this Android → load-per-core read 8× high → watchdog
  killed the model. Now counts `/proc/cpuinfo`.
- Load-average is the wrong governor for a phone running the model (the model *is* the load) —
  `Limits::resident()` now gates on **memory**, not load. Thermal is the noted next refinement.
- Empty model reply → 500 (store rejects empty) → now a plain fallback (all-platform robustness).

## The in-process shipping keystone (committed `4fcea3b`)

The watch loop was extracted to `fm_agent_run::watch::serve_loop` (generic over `VaultAccess`); the
desktop binary and the mobile app share it. `mobile/src-tauri/src/agent.rs` adds **`DispatchVault`** —
an in-process `VaultAccess` that calls `fm_app::dispatch` directly (no localhost server) — and
`start()` which launches the model + runs the loop on a background thread. **Compiles for
aarch64-android.** Presence/activity (the mobile wheel) and web search over the shell's HTTPS are
noted follow-ups.

## Decisions (owner, 2026-07-22)

- **Core vs agent split.** The **core stays minimal-deps + super-light** for notes-only users, and is
  gated so a notes-only build compiles *none* of the agent (the `agent` cargo feature; the **mobile**
  agent must be gated the same way — TODO). The **agent may carry its own heavier deps** — FFI is
  acceptable when it buys robustness. Platform-specific choices (libgit2 for Android) are fine.
- **Runtime = in-process FFI llama.cpp**, not the subprocess `llama-server`. Research (two agents,
  sourced) shows every shipping Android app (llama.cpp official, SmolChat, PocketPal, ChatterUI)
  JNI/FFI-binds `libllama` in-process; subprocess-from-`nativeLibraryDir` is the "hacker path" (W^X
  rename-to-`lib*.so` + `extractNativeLibs=true`, and llama.cpp has **no supported background-daemon
  story**, so we'd own orphaned-process/port cleanup). Subprocess *is* proven working on the phone but
  is the fallback.
- **Packaging = one APK (~+30 MB native code, dormant when the agent is off).** On-demand feature-
  module delivery (to keep the base truly tiny) was considered and **deferred** — one APK is simplest.

## Measured phone footprint (why low-resource users are fine)

- **Native llama code in the APK: ~30–50 MB** (a *custom text-only arm64* build trims to ~15–25 MB;
  the prebuilt `libllama.so` is 34 MB because it bundles vision/tools we don't need). Same for FFI or
  subprocess; must be in the APK (W^X blocks loading fetched native code).
- **Model: ~146 MB** (230M Q4 — the *smallest*), **fetched on first enable**, not in the APK,
  deletable, unloaded when off.
- **RAM while running: ~300 MB** — a different, far safer class than the research's "1–2 GB model"
  OOM/thermal warnings.
- **Agent OFF: zero runtime cost.** The agent is opt-in.

## The last mile (option 2, FFI) — the actionable plan

1. **NDK r27d → r28+** — 16 KB-page `.so`s (Android 15+; Play mandates after May 2026). A 4 KB-aligned
   lib SIGSEGVs on new devices.
2. **A trimmed text-only arm64 `libllama`** built with the NDK, FFI-linked behind the **mobile `agent`
   feature** (a new `LlmStep` impl calling llama.cpp directly — replaces `OpenAiStep`+subprocess on
   mobile). Keep models mmap'd, Q4, threads capped to big-cores.
3. **First-run model fetch** from Hugging Face into app-scoped storage — **resumable + checksum-
   verified**, `mkdirs()` before the temp file (the AI Edge Gallery `ENOENT` trap), re-verify after
   updates (storage wiped on uninstall).
4. **Foreground service + notification** around active generation; inference **off the main thread**
   (ANR); **unload on background** (LMKD).
5. **Gate the mobile agent behind a feature** so the notes-only build stays light.
6. **`tauri android build`** — watch the `extractNativeLibs`/manifest gotchas — then deploy + test.

## State at session end

Desktop agent fully working (offline-warning + instant-wheel fixes live — reload the browser). Phone
cleaned + disconnected. Repo fully committed at `4fcea3b`. The mobile in-process agent code is done
and compiles for the phone; the last mile above is scoped and decided, not yet built.

## UPDATE — runtime decided and de-risked: **one path, FFI** (owner, later 2026-07-22)

The owner insisted on **one maintainable path, no forks/patches, portable, longevity, and it must not
break the other platforms.** That *resolves* the subprocess-vs-FFI fork: subprocess-on-desktop +
FFI-on-mobile would be **two** paths, so the single path is **in-process FFI llama.cpp everywhere**,
and the subprocess/`OpenAiStep`-local path gets **removed** (once FFI is proven on both). This
supersedes the earlier "subprocess, one path" argument — FFI is the shipping standard (both research
agents) *and* the one that stays single across platforms.

- **Binding = `llama-cpp-2`** (utilityai), MIT/Apache, the maintained standard Rust wrapper — no fork,
  no patch.
- **De-risked on desktop (proven this session):** `llama-cpp-2` v0.1.152 builds here (cmake compiles
  llama.cpp, bindgen generates bindings, ~72s) and **runs** (backend inits). The *only* wrinkle was a
  one-line fix: bindgen's clang couldn't find `stdbool.h`, solved with
  `BINDGEN_EXTRA_CLANG_ARGS='-I<pixi>/lib/gcc/x86_64-conda-linux-gnu/14.3.0/include'`.
- **`cmake` + a C++ toolchain go in an agent-only pixi feature env** (like `media`/`android`), **not**
  the default env — the core stays light; a notes-only contributor never pays for them.
- **Two last-mile fears already handled by the repo:** 16 KB page alignment is *already* forced for
  Android (`-Wl,-z,max-page-size=16384` in the android rustflags), and `android-apk`/`android-release`
  (signed)/`android-install` tasks already exist.

### The migration (incremental, desktop never breaks)

1. Agent pixi feature env with `cmake`/`cxx-compiler`; `llama-cpp-2` behind an `ffi-model` cargo
   feature on `fm-agent` (the core links none of it).
2. A new **`LlmStep` FFI impl** (`LlamaStep`) — load the GGUF, tokenize, sample, generate — *alongside*
   the existing `OpenAiStep` (nothing breaks).
3. Rework the model **lifecycle**: `SupervisedModel` supervises a *subprocess*; in-process FFI has no
   subprocess — "stop" = drop the `llama_context`, preflight before load, memory-monitor while
   resident. The `watchdog`/`preflight` seams adapt; `serve_loop` is unchanged.
4. Prove the FFI runner on **desktop**, then cross-build `llama-cpp-2` for **android** (the next
   de-risk: bindgen + cmake against the NDK sysroot — same `stdbool.h` class of fix expected).
5. Switch `run_agent` to the FFI model and **delete** the subprocess path (`OpenAiStep`-local,
   `llama-server` spawn, `models.toml` `runtime_url`).
6. Bundle `libllama` in `jniLibs` (it's a normal `.so` now — no W^X exec trick), first-run GGUF fetch,
   foreground service, `tauri android build` → install.

The biggest unknown — *does the maintained binding build in this toolchain* — is now **answered yes**.
What remains is the migration work above, done in verified increments.

## UPDATE — the FFI-one-path decision is REVERSED (owner, later 2026-07-22)

Two evidence passes (both ≤5 agents, ≤1M tokens) overturned the FFI-everywhere plan above; the plan's
migration steps 5–6 (delete the subprocess path, run llama.cpp in-process on the phone) are **dropped**.

- **Adversarial review of the forward plan** (`forward-plan-review-2026-07-22.md`) found the "one path"
  argument *inverted*: the subprocess/HTTP path (`OpenAiStep` + `SupervisedModel`) is *already* one
  code path that ran unchanged on desktop-x64 **and** android-arm64 this session — the per-platform
  difference is a prebuilt binary (data in `runtime/`), the repo's own sanctioned pattern. FFI-
  *everywhere* is what fragments into N per-platform native builds. And in-process FFI silently trades
  away the load-bearing property: today the model is a child process, so `Watchdog::kill()` is an
  **unconditional, un-prompt-injectable OS off-switch** and a model crash/OOM **cannot touch the
  notebook**; in-process, "stop" becomes a cooperative `drop(llama_context)` and a libllama
  segfault/OOM faults the notes-only user's process.
- **Field evidence — apps like us** (`agent-backed-apps-runtime-2026-07-22.md`): of ~18 primary-app
  analogs (notes/KB apps with an LLM agent — Reor, Obsidian+Copilot, AppFlowy, Trilium, Khoj,
  AnythingLLM, Joplin+Jarvis, Standard Notes, Continue.dev…), **every one keeps the generative model
  out-of-process behind an OpenAI/Ollama-compatible HTTP seam**; **zero** embed a generative LLM in the
  app process. The only in-process-FFI examples are chatbots/demos (PocketPal, ChatterUI, SmolChat,
  Google AI Edge Gallery) — cited for the exact whole-app SIGSEGV/OOM/LMKD-kill failures we feared. On
  Android no serious notes app embeds a model in the vault-bearing process.

**The decided path now:** keep the model **out-of-process behind the OpenAI-compatible HTTP seam as the
default, load-bearing path** (`OpenAiStep` + `models.toml runtime_url` + `SupervisedModel`/`Watchdog`
stay — they are also the remote/LAN-provider seam). Make the runtime a `ModelRuntime`/`LlmStep` seam
with the proven subprocess as the sole shipping impl; any FFI/in-process is at most a feature-gated,
desktop-only, degraded-isolation *option*, **never** the phone's sole path and **never** a deletion of
the subprocess path. If a turnkey single-artifact is wanted, follow **Reor's playbook**: bundle
`libllama` but **spawn it as a supervised child process** — isolation *and* one build. Model "no model
available" as a **typed, non-fatal state** (AppFlowy's pattern).

**First actionable step — DONE.** The mobile agent was **not feature-gated at all**; now it is, mirroring
`fm-serve`'s `agent` feature: `mobile/src-tauri/Cargo.toml` gains `default = ["agent"]` +
`agent = ["dep:fm-agent", "dep:fm-agent-run"]` with both crates `optional = true`; `lib.rs` gates
`mod agent;` and the `agent::start` block behind `#[cfg(feature = "agent")]`; and `ci/checks.sh` grows a
structural guard (no Android toolchain in `pixi run ci`, so a grep: the two crates must be `optional`,
`default` must keep `"agent"`, and `mod agent;` must carry the cfg). **Verified on the aarch64 target:**
both configs `cargo check` clean; `cargo tree --no-default-features -i fm-agent` → "did not match any
packages" (agent-free), default graph lists both. So a notes-only APK provably links neither agent crate.

**On plan steps 2–3 — the tree is already in the recommended end-state.** The decided path is "keep the
model out-of-process behind `OpenAiStep`/`SupervisedModel`; make FFI at most a feature-gated option;
delete nothing." The tree already matches that: desktop *and* mobile already run a subprocess
`llama-server` talked to via `OpenAiStep`; there is **no FFI code in the tree** (it was only de-risked in
a throwaway project). So there is nothing to unify or delete. A `ModelRuntime` selection seam over a
**single** subprocess impl would be speculative generality (the `LlmStep` seam already abstracts the
model call) — deferred until a real second runtime is actually on the table. Mobile already treats a
missing model as a **typed, non-fatal state** (`agent::start` returns `Err`, `lib.rs` logs "study agent
not started" and the app runs on as a notebook) — the AppFlowy pattern, already satisfied. Step 3
(on-device FFI de-risk) needs the physical phone and stays deferred behind the feature.

## Phone last-mile — EXECUTED on-device (owner "Go", later 2026-07-22)

Owner chose **on-device**: bundle the arm64 llama runtime in the APK, fetch the ~150 MB model on first
enable. Built in verified increments (commits `0669b29`→`1fc85d9`):

- **In-app model downloader** (`0669b29`) — `fm_agent_run::fetch` behind a `download` feature:
  resumable (HTTP Range), SHA-256-verified, `<dest>.part`→rename. `ureq`+`rustls` (pure-Rust TLS,
  cross-compiles to android; `webpki-roots`' CDLA-Permissive-2.0 added to deny). 3 hermetic localhost
  tests, in `pixi run ci` via `test-agent-download`.
- **Manifest → HF URL + provisioning** (`ca9ab1c`) — manifest parses `repo`/`sha256`, builds the
  `resolve/main` URL; `fetch::ensure_model` fetches on first enable, idempotent/network-free when the
  file is already present (sideload-friendly).
- **Runner wired to the bundled runtime** (`3f47879`) — `fm_agent_run::nativelib` finds the native-lib
  dir from `/proc/self/maps` (our own cdylib's dir) in **pure Rust, no JVM shim**; `agent::start` execs
  `<nativeLibraryDir>/libllama-server.so`, downloads weights on a background thread, ships `models.toml`
  embedded (`include_str!`). Every failure ⇒ a log line + "run on as a notebook".
- **Runtime staged into the APK** (`1fc85d9`) — `ci/android-stage-runtime.sh` (a `pixi` task; a dep of
  `android-apk`/`android-release`): fetches pinned llama.cpp **b10081** android-arm64, renames
  `llama-server`→`libllama-server.so`, copies its **transitive NEEDED closure** (readelf BFS) + the
  dlopen'd ggml-cpu backends, **llvm-strips 235 MB→25 MB**, arch-checks AArch64, and sets
  `useLegacyPackaging=true` (extract-native-libs, so the binary is exec'able). **Verified: an arm64
  debug APK builds with all 14 runtime libs in `lib/arm64-v8a` (+25 MB; the 206 MB in the debug APK is
  the debug Rust cdylib, which release strips).**

**Remains — needs the physical phone (owner's step; assistant's adb is `/data/local/tmp`-only, no
installs):** install a release APK and confirm the exec-from-`nativeLibraryDir` path works from an
installed APK (proven earlier only via adb-shell in `/data/local/tmp`); verify **16 KB page alignment**
of the prebuilt libs (Android 15+ SIGSEGVs a 4 KB-aligned `.so`); a **foreground service** so the model
isn't LMKD-reaped mid-generation; a real **`sha256`** per model in `models.toml`; and a notes-only APK
**flavor** that skips staging.
