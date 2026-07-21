# 2026-07-21 — Study/research assistant agent: plan-only, refined + gated

**Nothing built.** This session produced a plan and preserved research; no feature code, no
dependencies, no commits. Gated behind MASTERPLAN's "core boring & stable first."

## What changed on disk
- `docs/context/ai-agents-plan.md` — added **Part III**: the re-scope from a *coding* agent to a
  *study/research* assistant, a table of what it supersedes in Parts I–II, and two new **cited**
  research passes (process governance + on-chip model execution). This is the durable record.
- Working plan file (outside the repo): `~/.claude/plans/staged-drifting-shamir.md` — the full staged
  roadmap.

## The decisions that stick (owner rulings, 2026-07-21)
- **Scope:** web research + faithful summarize + tidy notes/agenda. NOT coding. Math reasoning is a
  must-have but *not* carried by the default model. PDF/math-OCR deferred (a `parse_pdf` tool concern,
  model-agnostic).
- **Models — super-light on both devices.** Web research is *tool-orchestration, not knowledge*
  (1.2–1.7B + search ≈ 78–83% agentic-SimpleQA). Default = **Lucy 1.7B (Qwen3-1.7B)** on both;
  **LFM2.5-1.2B** = can't-crash fallback; **Qwen3-4B-Thinking** = hand-invoked math step-up, laptop-only.
- **Containment is in the tool set, not an OS jail:** **tool-calling-only (NO code execution)** +
  **propose-only branch** + **text-only/no-loopback fetch**. This holds identically on the phone, so
  the phone can research too. OS sandbox (`srt`/bwrap) drops to optional laptop defense-in-depth.
- **The core app never knows the agent exists** — its only output is a git branch + a `proposes:` note
  (a class `crates/fm-app/src/thread.rs` already ships). Zero agent code in fm-serve/fm-app/fm-core;
  `rm -rf` the agent dirs → core byte-identical. Off-switch = don't launch the separate process (no
  in-app toggle).
- **Structure:** no `fm-agents` repo yet — `crates/fm-agent-mcp/` (Rust MCP + preflight) + `agents/`
  (harness/searxng/sandbox) behind an `agent` pixi env; default build never compiles it.
- **Honest privacy:** notes stay local; research inherently touches the net (SearXNG anonymizes the
  source, not the queries). Not "nothing phones home."
- **Resource-efficiency contract:** opt-in, zero idle footprint, minimal resources for minimal time —
  the model is *one greedy process* under cgroup/nice/RLIMIT/timeout admission-control + teardown.

## Sequencing (the executable roadmap)
- **Gate 0 (unmet):** §2.5 thread backend rebuilt+committed, §1.1 UI seen by eye, §1.2 phone-media
  storage answered. §2.5 is the real unblock for the `propose-branch` phase.
- **Stage 0–2 (hermetic, `pixi run ci`):** dirs + fail-closed `preflight` → read-only `notes` MCP over
  `dispatch` → `propose-branch` (create-only).
- **Track A — laptop first:** plain `search` MCP → governance harness + **LFM2.5-1.2B single-shot
  dry-run = the measurement phase** (the numbers become the phone's floor) → swap in Lucy 1.7B for
  quality → hardened full loop.
- **Track B — phone later:** port read-only MCP + preflight → Android governance (lean on the OS:
  WorkManager/Thermal API, idempotent/resumable because a kill is normal) → LFM2.5-1.2B re-measured
  against the laptop floor.

## Five numbers to measure on-device before any "deploy" (the verify gate)
cold-load latency · `-t` thread-scaling sweep on the i7-11800H · Dimensity-7300 throughput ·
Android mmap-thrash behaviour · compositor stutter.
