# 2026-07-22 — executing the adversarial-review plan (tier-1 + portability seams)

Acting on `docs/context/agent-review-2026-07-22.md` (the 5-agent adversarial review). Sequenced
narrow-and-deep, one green commit per item, per the review's own guidance.

## Done and pushed

**First tier — the cheap, high-value wins (all of it):**
- **3.1 / 3.3 / 3.4** (`44d1a69`) — `Limits::resident()` kills the 300 s watchdog SIGKILL that was
  ending the assistant every 5 minutes (the intermittent "model exited on its own"); adaptive poll
  backoff (1→30 s idle); heartbeat decoupled to ~5 s; presence expiry 8→20 s; force-scan 15→60 s.
- **4.1 / 4.2** (`af2f4ce`) — one `heading::*` source shared by the assemblers and the echo-stripper
  (no more silent drift), plus one `normalize_answer` seam; guard test added.
- **2.2 / 2.3** (`91a887d`) — extracted `AgentRegistry` (TTL boards, unit-tested), one `AppState::new`,
  typed endpoint parsing.

**Second tier — portability (the real project):**
- **1.1 `VaultAccess`** (`f26ce37`) — the keystone. `Agent<V: VaultAccess>`; `FmServe` is the desktop
  HTTP impl; a mobile in-process `fm_app::dispatch` impl is now just a second implementor. First
  `FakeVault` unit test proves the decoupling.
- **1.4** (`095953a`) — the one-shot proposal binary now writes through the seam (single-writer, no
  SQLite race, no git binary); **`fm-agent-run` dropped `fm-core` + `fm-app` entirely** — the runner
  drags no git/FileStore/SQLite, exactly what lets it target Android.
- **2.1** (`357ac3f`) — all of fm-serve's agent machinery (registry, six `/api/agent_*` endpoints,
  spawn) moved into an `agent` module behind a **default-on `agent` cargo feature**. `cargo build
  --no-default-features` now compiles a provably agent-free core — "`rm -rf agents/` is byte-identical"
  is compiler-enforced, the way `fm-query`'s purity is. Six endpoints smoke-tested live; auto-start
  still fires.

Every increment green under `pixi run test`; the running stack is on the fixed build (no 5-min death).

## Remaining seams (interlock — one green commit at a time)

- **1.5 + 4.3** — a Rust launcher + `AgentLauncher` seam replacing the bash/awk/python chain, and a
  single manifest (`models.toml`) reader (the three awk copies already diverge).
- **1.3** — a real HTTPS-capable `WebSearch` so the Python `search-proxy.py` + the third port go away.
- **1.6** — an Android-aware `ResourceMonitor` (thermal headroom + PSI, cancellation not SIGKILL)
  behind the `ResourceMonitor` trait, so preflight doesn't fail-closed on a phone that can't read
  `/proc` load-average.

## The one large, deferrable lift

- **1.2** — a `ModelRuntime` trait split out of the subprocess launch, with an in-process
  llama.cpp/LFM runtime for the phone (the only on-device model shape). Validate the desktop-side
  seams first; the same generic `Agent` exercises them there.

## Trap re-confirmed this session

Background `fm-serve` for testing MUST be launched `setsid nohup … < /dev/null & disown`, or the
Bash tool's shell exit kills it (the session's repeated restart churn).
