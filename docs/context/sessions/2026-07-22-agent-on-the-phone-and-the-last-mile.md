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
