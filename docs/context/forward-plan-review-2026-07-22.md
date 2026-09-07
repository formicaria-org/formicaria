# Forward-plan adversarial review — FFI vs. the alternatives

> **Dated review, 2026-07-22 — a snapshot, not maintained.** Read for the argument, not for
> status. Current rulings: `decisions.md#agent`.

*Adversarial multi-agent review (4 lenses + synthesis, 2026-07-22) pressure-testing the
"one path = in-process FFI llama.cpp, delete the subprocess path" migration against the owner's
principles. Supersedes the FFI-single-path decision recorded in
`sessions/2026-07-22-agent-on-the-phone-and-the-last-mile.md`.*

## VERDICT

**The plan does not hold up as written. The two irreversible commitments at its center — delete the
subprocess path, and run llama.cpp in-process on the phone — should be removed.** Everything else is
fine and worth doing.

The migration's justifying argument is a false dichotomy. It claimed "subprocess-on-desktop +
FFI-on-mobile = two paths, therefore unify on in-process FFI." But the subprocess/HTTP path is
*already* a single code path that ran unchanged on desktop-x64 **and** android-arm64 this session —
the only per-platform difference is which prebuilt binary sits in `runtime/`, i.e. data selected at
deploy time (the repo's own sanctioned pattern, same as `VaultAccess` having `FmServe` +
`DispatchVault` behind one trait). So "one path" does not select FFI; applied honestly it points at
*keeping the proven subprocess path*. The real argument for FFI is "match the chat-app shipping
standard" — legitimate but weak once you notice those apps have no notes-first user to protect.

And the cost is the load-bearing one the owner's tenets care most about. Today the model is a
separate child process; the safety harness (`SupervisedModel` → `Watchdog` → `child.kill()`+reap)
delivers an **unconditional, un-prompt-injectable OS-level off-switch** and a **crash domain that
cannot touch the notebook**. In-process FFI deletes both: "stop" becomes a cooperative
`drop(llama_context)` that cannot interrupt a wedged `llama_decode`, and a segfault / GGML
`assert()→abort()` / OOM inside libllama now faults the notes-only user's process. The plan booked
FFI purely as a packaging win and never recorded this robustness loss — that undocumented trade is
the core defect.

There is a clean way to keep almost everything the plan wanted (unification, a bundled lib, one
seam) **without** paying that price: keep the model out-of-process, and if the goal is to stop
shipping a fat prebuilt `llama-server`, bundle `libllama` but launch it in a **thin child process**
(the "subprocess-bundled-lib" option). Adopt the seam, keep two impls, delete nothing until FFI is
proven by real inference on a real phone.

## Ranked findings

### F1 — In-process FFI collapses the model's crash-domain into the notes app and downgrades the hard off-switch to a cooperative flag *(HIGH)*
`Watchdog::kill()` does an unconditional `child.kill()`+`wait()` on any breach/timeout/stop/unreadable
device — a terminator that works even if the model is wedged, looping, or mid-segfault, blast radius a
disposable child. In-process, `libllama` runs in the app's own address space (on mobile, alongside the
live `Arc<App>` / FileStore / SQLite handle). "Stop = drop the context" only fires *between*
generation steps if the native loop polls the flag; a long `llama_decode`, a multi-second prefill, a
GGML threadpool deadlock, or a native crash/OOM now takes down the vault and unsaved edits.
**Better path:** keep a real OS process boundary as the containment unit; if FFI is wanted, run it in
a child process so `SupervisedModel`/`Watchdog` keep their external SIGKILL. Never in-process as the
phone's *sole* path.

### F2 — "One path" is inverted: subprocess/HTTP is already the single portable code path; FFI-everywhere is N per-platform native builds *(HIGH)*
`OpenAiStep` (190 lines, zero heavy deps) + `SupervisedModel` is *one* code path that ran on both
platforms; the per-OS variation is a prebuilt binary (data). FFI-everywhere is a distinct from-source
toolchain integration per platform — desktop cmake+bindgen (already needed a manual
`BINDGEN_EXTRA_CLANG_ARGS` stdbool.h hack), an unproven NDK cross-compile with 16 KB-page `.so`, plus
macOS/Metal + Windows/MSVC if "longevity means all platforms." The plan's headline justification is
wrong. **Better path:** the `LlmStep` seam already gives one orchestrator over swappable runtimes —
two impls is the seam working (like `FmServe`+`DispatchVault`), not a fork.

### F3 — The migration deletes the only phone-proven runtime *before* the hard long-pole (Android FFI cross-build) is de-risked *(HIGH)*
Step 5 deletes the subprocess path; step 6 (bundle `libllama`, first-run fetch, foreground service,
actually run inference on-device) is where FFI *first executes on a phone*. The delete lands before
the on-device proof. The desktop de-risk ran no inference, no cross-compile, never touched the device.
If the NDK wall is hit after step 5, the phone has no working agent and desktop was switched off a
lighter path that already worked. **Better path:** any deletion strictly *after* a full generation is
proven on desktop **and** end-to-end on the actual phone; keep subprocess behind a feature as a
fallback through at least one release.

### F4 — Deleting `OpenAiStep` + `models.toml runtime_url` forecloses the remote-provider seam the design deliberately built *(HIGH)*
`OpenAiStep` (~190 lines pure `std::net`, unit-tested) is the OpenAI `/v1/chat/completions` shape that
`ai-agents-plan §1/§3` and Phase-2 reserve for a remote provider (OpenRouter/Cerebras), a LAN laptop
the phone talks to, ollama, or LM Studio — `runtime_url` is what makes a provider remote. `LlamaStep`
speaks llama.cpp's native C API, so deleting `OpenAiStep` amputates the seam the plan's own future
mode depends on. Cost of keeping ≈ 0. **Better path:** keep `OpenAiStep` and `runtime_url` regardless;
add `LlamaStep` *alongside*.

### F5 — Base-APK `libllama` tax + an ungated mobile agent violates "notes-only pays nothing," and gating is scheduled *after* bundling *(HIGH)*
(1) On-demand delivery was deferred "because one APK is simplest" — dev-simplicity over the
low-resource principle — so ~30–50 MB of native `libllama` ships in the *base* APK for every
notes-only user. (2) The mobile agent is not feature-gated at all: `mobile/src-tauri/Cargo.toml`
depends on `fm-agent`/`fm-agent-run` unconditionally, `lib.rs` has an unconditional `mod agent;` +
`agent::start` in setup with no `cfg`. Desktop was forced to gate its agent behind the `agent` cargo
feature (CI-enforced byte-identical `--no-default-features`); mobile has no equivalent, and the plan
lists "gate the mobile agent" as step 5 — *after* bundling libllama. So the light notes-only APK is
not the default artifact and doesn't yet exist. **Better path:** gate the mobile agent behind a cargo
feature **first**, CI-enforced like `fm-serve`'s; ship two flavors (notes-only / agent) from one
codebase. Worth doing regardless of the FFI decision.

### F6 — Reworking `SupervisedModel`/`Watchdog` from "supervise a Child" to "drop a context" forks or dilutes the clean safety harness and throws away its test suite *(MEDIUM)*
`Watchdog::supervise` is built end-to-end around `std::process::Child` (`try_wait()`, `child.kill()`,
`Outcome::Completed(ExitStatus)`), with a deterministic test suite spawning `sleep 30` as a child.
None maps onto an in-process `llama_context`: no pid, no `ExitStatus`, no external kill, tests
undriveable. Step 3's "the watchdog/preflight seams adapt" hand-waves a rewrite of the containment
core into weaker cooperative primitives. **Better path:** keep `SupervisedModel`/`Watchdog` as the
*process*-supervision harness; if in-process is added, give it its own honestly-named lifecycle +
tests — don't claim the Child-based harness merely "adapts."

### F7 — Idle/unload memory reclamation regresses for exactly the low-resource devices the owner prioritises *(MEDIUM)*
Subprocess "unload" is a clean process exit that returns **all** pages (weights + pre-allocated `-c`
KV cache + GGML arenas) to the OS instantly and unfragmented, and LMKD can reap *just the model* while
the notebook survives. In-process, "drop the context" frees to bionic/glibc, which for large arenas
commonly does *not* return pages to the OS — the notebook's RSS stays inflated while idle. The
footprint table treats FFI/subprocess as memory-equivalent while running but ignores the reclamation
asymmetry when *off* — the metric that matters on a phone. **Better path:** keep the model
out-of-process; if in-process anyway, show *measured post-unload host RSS* on a low-RAM device before
claiming the low-resource story holds.

### F8 — cmake + bindgen/libclang + NDK-sysroot cross-compile is the least-reproducible step in the repo, and becomes the *mandatory* agent build path everywhere once subprocess is deleted *(MEDIUM)*
The current runtime is a prebuilt binary — zero C++ compilation in the build graph. `llama-cpp-2`
compiles ~300k LOC of C++ via cmake and regenerates bindings against a fast-moving C ABI; the easy
(desktop) target already needed a hand-injected clang-args workaround. Bus factor: essentially one
vendor's bindgen wrapper — if it lags, you maintain bindgen against a moving header yourself, the
fork/patch the owner forbids. *Partly accepted* — the owner allowed heavier agent-only deps, so not
disqualifying. **Better path:** keep subprocess/prebuilt as fallback so a broken FFI/NDK build
degrades to a working agent; pin `llama-cpp-2` **and** llama.cpp by SHA; keep cmake out of the default
`pixi run ci` gate; confirm the vendored llama.cpp/GGML licence is caught by `ci/third-party.sh`
(cargo-deny is blind to vendored C — the libgit2 hole).

### F9 — A hand-written `LlamaStep` must re-derive determinism, the `max_tokens` hard bound, and truncation detection against the raw C API *(MEDIUM)*
`OpenAiStep`/`parse_completion` get load-bearing guarantees "for free" from `llama-server`: fixed seed
(llama.cpp's own default is *random*), `max_tokens` as a sharp never-berserk bound, low temperature,
and `finish_reason == "length"` → `truncated()` (the runner relies on this to never ship a silently
truncated study summary). A `LlamaStep` on the C API must hand-roll tokenize→sample→detokenize→stop
and re-derive each guarantee — more code, more risk. **Better path:** if FFI is pursued, prefer a thin
server shim reusing llama.cpp's own OpenAI-compatible handler over hand-rolling sampling; property-test
any `LlamaStep` for byte-comparable determinism + correct truncation *before* it may replace subprocess.

**Honest counterweight:** FFI genuinely does buy lower per-call latency (no HTTP round-trip, no server
warmup), a single build artifact, and the industry-standard mobile shape. Those are real and are why
the plan was tempting — but latency is not in the owner's priority list, and the desktop de-risk never
quantified a UX gap.

## Head-to-head (scored against the owner's principles)

**Status-quo** = spawn prebuilt `llama-server`, talk HTTP via `OpenAiStep` (proven both platforms).
**Subprocess-bundled-lib** = link `libllama` yourself but run it in a *thin child process* the
watchdog supervises. **In-process FFI** = the plan.

| Principle | In-process FFI | Subprocess-bundled-lib | Status-quo |
|---|---|---|---|
| Core robustness (notes user undisturbed) | ✗ native crash faults notebook | ✓ isolated to child | ✓ isolated to child |
| One path | ✓ one runtime, via N native builds | ✓ one lib, one supervise path | ✓ one code path; binary is data |
| Portability | ✗ per-OS cmake/bindgen/NDK | ~ one build, still from source | ✓ swap a prebuilt per OS |
| Longevity | ✗ tight C-ABI, fast-churn dep, bus-factor-1 | ~ same dep, HTTP seam retainable | ✓ frozen OpenAI wire contract |
| Low-resource (idle reclaim) | ✗ freed pages fragment heap; LMKD reaps notebook | ✓ clean exit reclaims | ✓ clean exit reclaims |
| Containment / off-switch | ✗ cooperative drop; no hard kill | ✓ external SIGKILL | ✓ external SIGKILL |
| Simplicity | ✗ rewrite watchdog; hand-roll sampling | ~ reuse watchdog; thin exe | ✓ nothing to build |
| Latency / mobile-standard shape | ✓ no HTTP hop | ~ small hop | ~ HTTP hop |

In-process FFI wins on exactly one principle (latency, not in the priority list) and loses or ties on
every principle the owner ranks first. **Subprocess-bundled-lib and status-quo dominate it on the
owner's own scoring.**

## Recommended path forward

**Adopt the seam, keep the boundary, delete nothing yet.**

1. **Model the runtime as one seam with swappable impls**, exactly like `VaultAccess` —
   `LlmStep`/`ModelRuntime` with `Subprocess` (proven, default) and, later, an optional in-process or
   bundled-lib impl behind an `ffi-model` feature. This *is* "one path" by the repo's own standard and
   requires deleting nothing.
2. **Keep `OpenAiStep`, `models.toml runtime_url`, and the `Watchdog`/`SupervisedModel` process
   harness** — the portable default, the remote/LAN seam, and the un-prompt-injectable off-switch.
3. **If unifying on one small artifact is the real goal, choose subprocess-bundled-lib over
   in-process** — bundle `libllama` but launch it in a thin child process so the OS kill and crash
   isolation survive. In-process, if ever adopted, stays a feature-gated, desktop-only,
   degraded-isolation option — never the phone's sole path.

**First 3 steps, in order:**

1. **Gate the mobile agent behind a cargo feature, CI-enforced** (mirror `fm-serve`'s `agent`
   feature so `--no-default-features` is byte-identical); make the **notes-only APK the default
   artifact**. Owner's top principle, currently violated; independent of the FFI decision; lands first.
2. **Introduce the `ModelRuntime`/`LlmStep` selection seam** with subprocess as the sole shipping impl
   — no behaviour change, no deletion — so future runtimes plug in and step-5 "delete" is unnecessary.
3. **De-risk Android FFI *behind the feature, on a real phone*** to a full tokenize→sample→generate
   under the watchdog with an equivalent hard-stop (and measure post-unload host RSS on a low-RAM
   device). Only if it beats subprocess-bundled-lib on measured latency by enough to justify the
   containment loss, reconsider *demoting* (never deleting) the subprocess path. If not, ship
   subprocess-bundled-lib and stop.
