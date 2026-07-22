# 2026-07-22 — device-agnostic CI tests for "the agent goes out with formicaria"

Continues `2026-07-22-agent-shipped-on-device-and-hardened.md`. The owner asked for **a few
device-agnostic CI tests to catch the lifecycle behaviour** — "if formicaria is out, the agent should
have gone out after a while without connection" — **especially on the Android device**, where an
orphaned `llama-server` pinning RAM is the worst case. The earlier session *fixed* the orphan/VRAM bug
(launcher restart kills the whole stack); this session *pins the guarantee with tests* so it can't
silently regress on either platform.

## Two seams, two tests — both hermetic (no model, no network, no device)

There are two independent ways the model is supposed to go away when formicaria does, so there are two
tests:

1. **The vault vanished → the watch loop stops the model.**
   `fm-agent-run` `serve_loop` polls `agent.fm.alive()`; when it returns false (fm-serve / the app is
   gone) it must call `stop_model()` and **return**, not keep polling a dead server. This is the exact
   seam that failed when a *restarted* fm-serve was mistaken for the original and the old agent-serve
   never noticed its parent died. `FakeVault` gained a configurable `alive` field (was hardcoded
   `true`); the new test `serve_loop_stops_the_model_when_formicaria_is_gone`
   (`crates/fm-agent-run/src/lib.rs`) drives the loop with `alive: false` and asserts `stop_model` ran.
   This is the **desktop** path (over HTTP, fm-serve can genuinely disappear).

2. **The supervisor process died → the kernel kills the model (`PR_SET_PDEATHSIG`).**
   This is the **phone** path: a swiped-away / LMKD-reaped app runs no clean shutdown, so the backstop
   must be behaviour-independent. The `prctl(PR_SET_PDEATHSIG, SIGKILL)` was **mobile-only**, in
   `agent.rs`, and untestable on the CI host. **Moved into the shared `SupervisedModel::launch`**
   (`crates/fm-agent/src/launch.rs`, via a `die_with_supervisor(&mut cmd)` helper, no-op off Unix) — so
   it is now **one code path for desktop and phone** and exercised on the Linux CI host by
   `the_model_dies_when_the_thread_that_launched_it_goes_away`: spawn `sleep` under the same helper from
   a thread, let the thread exit, assert the child is gone or a zombie (read `/proc/<pid>/stat` state —
   `kill -0` can't tell a zombie from a live process). As a bonus, **desktop `agent-serve` now inherits
   the backstop too** (both platforms launch through `SupervisedModel::launch`), a second layer under
   the launcher's whole-stack kill for the orphaned-VRAM case.

`libc` moved accordingly: added to `fm-agent` as a `cfg(unix)` dep (one `prctl` call), removed from
`mobile/src-tauri/Cargo.toml` (now unused there — it reaches libc transitively through fm-agent).

## What is *not* covered (honest scope)

- The Android in-process `DispatchVault.alive()` is always true (the vault can't disappear from a
  process it lives in), so seam #1 never fires on the phone — the phone relies on seam #2 + the app-exit
  hook (`RunEvent::Exit → agent::stop()`, which trips the same watchdog stop that
  `it_launches_supervised_and_the_stop_flag_stops_it` already covers). All three mechanisms are now
  tested at the unit level; an end-to-end "kill the app, watch RSS drop" check stays a manual on-device
  step.
- `pixi run cargo test -p fm-agent -p fm-agent-run` green (42 + 11). Mobile needs the usual
  rebuild+install to pick up the moved backstop; behaviour is unchanged, only the code path is shared.
