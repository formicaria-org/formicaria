# 2026-07-22 — stress-testing the agent interaction: a live progress wheel + concurrency fixes

Owner asked to stress-test the user↔agent interaction (rapid messages, repeats), give the user a
**turning wheel that shows the whole pipeline working** (not just the LLM — "say it is doing the web
search"), that goes down only on an answer or a timeout, and to **research popular GitHub LLM tools
and learn from their failures**. Efficiency, transparency, ease-of-use are the priorities.

## What the stress test found (live, against the real vault)

The `agent-serve` watcher polled every 3s, examined only `msgs.last()` per discussion, and ran the
whole turn **inline and single-threaded**:
- **Rapid messages → only the last answered.** Fired 3 quick questions, got 1 reply. The other two
  were silently dropped (only `last` was ever inspected).
- **One slow turn stalls every discussion** for up to ~150s (30s search + 120s LLM timeouts, all
  blocking the single loop).
- **Zero feedback** during the wait — and, worse, **no polling at all in the UI**: the thread only
  reloaded on note-change and right after the user sent, so the agent's async reply didn't even
  appear live until you navigated away and back.

## Research (2 subagents, ~18 tools, sourced to GitHub issues)

Full findings folded into `ai-agents-plan.md`. The convergent lesson across every tool: **the one bug
they all share is treating cancel/timeout/progress as a UI concern instead of backend state.** What
works: backend is the source of truth for per-stage state, the UI polls and renders it (polling beats
a WebSocket SPOF — gpt-researcher #1284); emit one typed event per pipeline stage and **label the
non-LLM stages** (LangGraph's `get_stream_writer()` exists precisely to fill the silent tool-call
gap); per-stage wall-clock timeout that actually cancels; show elapsed so "slow" ≠ "hung"; clear the
spinner on completion; serialize/coalesce rapid + duplicate messages.

## What was built

**Activity channel (fm-serve, in-memory, core stays agent-agnostic).** `AppState.agent_activity:
Mutex<HashMap<disc, Activity>>` + two transport endpoints beside `/api/alive` and `/api/agent_status`
(NOT dispatch, so fm-core/fm-app never learn the agent exists, and it never touches disk):
`POST /api/agent_activity` (agent writes `{discussion, stage, question}` or `{done:true}`) and
`POST /api/agent_activity_poll` (browser reads `{active, stage, question, elapsed_secs}`). A stale
entry (agent died mid-turn without clearing) auto-expires after 180s so the wheel never spins forever.

**Orchestrator as an FSM (fm-agent / fm-agent-run).** `Agent::handle` takes an `on_stage: &dyn Fn`
and reports each stage — `reading the conversation → reading your notes → searching the web →
thinking → writing the reply`. `serve.rs` wires `on_stage` to `fm.activity(id, …)` and clears with
`fm.activity_done(id)` on completion *or* error; `chat.rs` prints the stages in the REPL.

**Concurrency fixes (serve.rs).** Answer **every** pending mention in order (scan all messages past
the watermark, not just the last), and **coalesce identical resends within a poll** (a double-tap on
send → one answer). A timeout/failure now posts a **visible** notice (`⚠️ I couldn't finish that
one …`) and clears the wheel, instead of a silent stall.

**UI (NotePanel.svelte).** While a discussion is open, a 1.5s poll drives a turning-wheel row
(`.disc-working` + CSS spinner, `prefers-reduced-motion` aware) showing the live stage + elapsed
seconds, and **also refreshes the thread** so the reply appears the instant it lands (fixing the
no-live-updates gap). `ipc.agentActivity()` degrades to inactive where the endpoint is absent (mobile
shell), so it never throws.

## Verified live

- Activity endpoints: idle→`active:false`, set→`active:true` with rising `elapsed_secs`, stage
  updates keep elapsed counting from turn start, done→cleared. ✓
- Agent publishes stages during a real `/search` turn (`thinking` caught + cleared). The pre-LLM
  stages fly by (<150ms each on this tiny-fast model + local search) — the wheel shows each stage for
  its *real* duration, so slow stages (the owner's actual concern) stay visible; fast ones flash.
- Concurrency: 3 rapid distinct questions → **3 replies** (was 3→1). ✓
- `pixi run test` (57 suites) + `test-ui` (275) green; new unit tests for the echo-stripper's third
  shape.

## Open observations (flagged, not caused by this work)

- **llama-server exited on its own once** (`ExitStatus 1`) after a turn; agent-serve correctly saw
  `model.finished()` and shut down. Intermittent (it answered 3 turns in a row in the next run). A
  llama.cpp stability matter to watch, separate from the orchestrator.
- **Web search silently no-ops if the proxy is down** — `web()` is best-effort, so the turn continues
  and the model answers from memory. Fine for resilience, but the "searching the web" stage then does
  nothing; consider surfacing "web search unavailable" when the proxy is unreachable.
- **Pickup latency = poll interval (3s).** Kept at 3s for efficiency (owner priority); the wheel
  appears within ~3s of sending. Lower `--poll-secs` for snappier pickup at more idle polling.
- 230M reply quality remains the documented limit; the echo-stripper now handles three echo shapes
  but a chaotic echo can still leak. Quality is for RAG + a bigger model, not this bring-up model.

## State

Built + tested; **not committed** (awaiting the owner's go). Machine left clean — all test processes
stopped, both scratch discussions deleted.
