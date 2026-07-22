# 2026-07-22 — agent observability: logging, per-stage timing, presence, self-healing poll

Stress-testing the live agent surfaced that when a mention went unanswered there was **no way to see
why** (all agent output went to `/dev/null`), and the owner asked for a **timing breakdown** ("where
is it most expensive?"), a **warning when the agent isn't alive** (not silence), and cheaper polling.
This batch makes the agent observable and its interaction reliable — all orchestrator/transport only,
the vault core stays agent-agnostic.

## What "never answered" actually was

Mostly **the owner and the assistant fighting over the same fm-serve on :8765** — repeated
kill/restart cycles during testing meant no agent was live at the moment of some mentions. With a
stable stack + the log + presence below, it is now reliable and diagnosable. Verified: `@lfm2.5-230m
what is bayesian model selection? /search` answers in ~4s pipeline (well-formed answer).

## Shipped

- **Persistent log — `agents/runtime/agent.log`** (`start-agent.sh`). Captures the watcher's
  turn/timing lines, the search proxy, AND the model server's own stdout/stderr (inherited), so a
  model crash is finally visible. Rolls to `.1` past ~5 MB. (`agent-serve.sh` output is `tee`'d so the
  dev console still shows it.)
- **Per-stage timing — `agents/runtime/timing.jsonl`** (`serve.rs::log_timing`, `fm-agent-run`). One
  JSON line per turn with each stage's ms + total, plus a `⏱ turn Nms (slowest: …)` line in the log.
  The stage boundaries come free from the existing `on_stage` FSM events. First real data:
  `thinking 2889ms`, `searching the web 879ms`, everything else <15ms — **the LLM and web search are
  the entire cost**, retrieval/assembly are noise.
- **Presence heartbeat** — the agent pings `POST /api/agent_present {name}` each poll; fm-serve keeps
  `name → last_seen` (`AppState.agent_present`, in-memory), and `POST /api/agents` returns the names
  seen in the last 8s. Transport-shaped like `/api/alive`. This is the source the UI's @-picker and
  offline-warning will read (not yet wired to the UI).
- **Self-healing poll** — keeps the count-skip optimization (skip a discussion whose message `count`
  is unchanged) but forces a **full rescan every 15s**, so a message can never be *permanently*
  missed if a count ever fails to reflect it. `poll_secs` default lowered 3→1 (the skip makes an idle
  poll one cheap `discussions()` call).
- **Graceful web-unavailable** (`fm-agent-run` `handle`) — if the search proxy is unreachable, the
  turn no longer fails; it answers from notes/memory and **prefixes the reply** with a note that web
  search was unavailable (a wrong answer that looks researched is worse than a flagged one).

## Still open (the UI half — Explore-mapped, not yet built)

1. **@-picker**: the reply composer (`NotePanel.svelte:~1895`, `onReplyKeydown` at ~912) has *zero*
   `@` handling. Mirror the existing slash-menu popup, fed by `GET /api/agents`.
2. **Offline warning**: `@`-mention while `/api/agents` is empty → warn instead of silence.
3. **Per-message author labels** (agent vs person, by the vault badge): the agent's `reply` posts with
   **no distinct author**, so its messages are indistinguishable from the user's. Needs a backend
   touch to attribute agent messages to the model identity (like `create_proposal` already does).
4. **Conflict under Collaboration**: the "waiting on you" warning reads **git unmerged paths**
   (`vcs::conflicts`, `git.rs:1562`), but the Collaboration list (`commands::conflicts`,
   `commands.rs:742`) only marker-scans note *bodies* and **excludes discussion messages/proposals**
   (`notes_base`, `thread.rs:110`). A conflicted discussion message thus never appears. Fix: feed the
   Collaboration list from git's unmerged paths (the authoritative source), per `dispatch.rs:419`.

## State

Built + `pixi run test`/`test-ui` green; stack left running on :8765 for the owner to test.
Committed this session.
