# 2026-07-22 — the study assistant, run live against the real vault (three bugs fixed)

Ran the study assistant **end-to-end against the owner's real vault** (the `Test agent`
discussion, `01KY2W4AGS…`, in the `vault` vault) at their explicit request — "create a note or
modify my Test agent one to see directly if it works." Started the freshly-built
`target/release/fm-serve` from the repo root; with `~/.config/formicaria/agent.json` =
`{"enabled": true}` it **auto-launched the whole stack from one process** — `search-proxy.py`
(:8888), `agent-serve` (watcher), and `llama-server` (:8081) — no terminal, exactly the
double-click-the-icon flow. Killing fm-serve tore the model down cleanly with **no orphans**
(the "model dies with formicaria" contract). Posting `@lfm2.5-230m …` into the discussion made
the agent **write a reply message straight into the vault** (uncommitted — the vault has an
unresolved merge conflict, `01KY1TK58N…`, that blocks commits, so replies are visible in the app
but not in git until it's settled).

**It works** — auto-start, mention detection, retrieval, web search grounding (`Paris`, `Tokyo`,
`Rome` all came through `/search`), and a single clean reply. Three real defects surfaced and were
fixed at the root; the model's *answer quality* remains the documented 230M limit (it answered
"Spain" with "Rome" — confused by the prior turn in a growing history).

## The three bugs (all in the agent runner/orchestrator, not the core)

1. **Self-reply infinite loop (critical).** The watcher answers the last message of each
   discussion; the agent's own reply becomes that last message, and because the tiny model *echoes
   the prompt* (which contains `@lfm2.5-230m`), `convo::addressed` matched its own output and it
   replied again, forever — it spammed ~6 messages before I killed it. **Fix:** `Agent::handle`
   now returns `(reply, Option<reply_id>)`; `agent-serve`'s loop inserts that id into `handled` so
   the agent never answers itself (`fm-agent-run/src/{lib,serve,chat}.rs`).

2. **Backlog reply on startup.** The watcher didn't seed `handled`, so on its first poll it
   answered the last message of *every* discussion (stale history). **Fix:** seed `handled` with
   every existing message id before the loop — it now answers only what arrives *while it watches*.
   (Introduces a ~1s startup race: a mention posted between "listening" and the end of seeding is
   swallowed. Irrelevant in real use — the user mentions the agent long after launch — but it bit
   the test until I waited a few seconds after model-ready before posting.)

3. **Prompt echo in the reply.** The 230M model ignores "don't repeat the question" and parrots
   the prompt back around the real answer, in **three different shapes** across runs: answer under
   an invented `# Answer` heading (answer-last); answer first then a trailing bullet-echo of the
   question; answer first then a trailing echo of a `# Notes and search results` section heading.
   **Fix:** `strip_echoed_prompt(reply, question)` in `fm-agent/src/lib.rs` cleans all three with
   signals we can *trust exactly* rather than guessing at prose: keep text after the last invented
   `Answer` heading, then cut at the first line that is one of the **section headings we injected**
   (`# Conversation so far`, `# Notes and search results`, `# Question`, `# Task`, `# Provided
   notes`, `# Search results`) or that repeats the **exact question** — all strings the model was
   *given*, never asked to produce (the system prompt forbids headings and repeating the question),
   so a clean reply contains none of them and is returned whole. Unit-tested for all three shapes.

## State

- All fixes **built (`target/release`) and `pixi run test` green** (incl. the new echo-stripper
  test). **Not committed** — awaiting the owner's go (house rule: commit only when asked).
- The icon flow picks these up for free: `agent-serve.sh` prefers `target/release/agent-serve`,
  now rebuilt with all three fixes.
- Left in the owner's `Test agent` discussion: the demonstration messages (Paris/Tokyo/Rome +
  the earlier loop-echo artifacts), **uncommitted**. Owner can delete them in the app.
- Machine left as found — formicaria was not running when the session started, and every test
  process was stopped at the end.

## Honest limit (unchanged, now seen live)

Plumbing is solid; **LFM2.5-230M answer quality is unreliable** — it got Paris/Tokyo/Rome right
but said "Rome" for Spain (context confusion) and earlier "Yes" to "does mRNA change DNA". Quality
is meant to come from RAG + a bigger model later, not this bring-up model. The formatting is now
clean regardless of what the model says.
