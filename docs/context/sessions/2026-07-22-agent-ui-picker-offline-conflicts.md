# 2026-07-22 — agent UI: @-picker, offline warning, conflicts under Collaboration

The UI half of the agent-interaction work. Owner asks addressed: typing `@` showed no agents; a
mention to a dead agent went silent instead of warning; a conflicted note the "waiting on you"
warning named never appeared under Collaboration.

## Shipped (built on the presence + activity endpoints from the prior batch)

- **@-picker** (`NotePanel.svelte`): the reply composer now has an `oninput`/keyboard-driven popup
  (mirroring the slash menu) that opens on an `@token`, listing the **live** agents from
  `GET /api/agents` (`ipc.onlineAgents`). Arrow/Enter/Tab/Esc + click; inserts `@name `. Handles the
  `.`/`-` in model names (`lfm2.5-230m`). Placeholder now hints "type @ to call an assistant".
- **Offline warning**: on send, any `@name` the draft addresses that isn't in `agentsOnline` produces
  a gentle notice ("`@name` isn't running, so it won't answer — turn it on in Settings"). Never blocks
  sending. Honest, because a mention posted while the agent is off is seeded as already-seen and won't
  be auto-answered.
- **Conflicts under Collaboration** (`dispatch.rs` `"conflicts"`): now **unions git's unmerged paths**
  (`vcs::conflicts` per vault — the authoritative source the warning reads) with the existing
  body-marker scan. Each unmerged `<dir>/<id>.md` maps to its indexed note, or a **stub** ObjectMeta
  when unindexed. Verified against the owner's real conflict: `01KY1TK58N…` is a **delete/modify
  (`UD`)** conflict — no text markers, unindexed — so the old marker-scan missed it entirely; it now
  shows as "conflict — resolve in git". `mock.ts` gains an `agents` case (the enable toggle stands in
  for a live agent, so the picker is demoable in dev).

## Still open — per-message author labels (agent vs person)

Not shipped. Message authorship is a **git fact** (`lastEditFor` ← the `activity` feed ← git commit
author), and the agent's `reply` does `store.put` with **no distinct author** — the commit is the
vault's batched auto-commit under the owner's identity, so agent messages are indistinguishable from
the owner's. Making them distinct needs the agent's writes committed under the **model git identity**
(as `create_proposal` already does for proposals) — a commit-flow change, and currently moot anyway
because the owner's vault has commits **blocked by the `UD` conflict above**. Do this after the
conflict is resolved, as a focused change.

## State

Built (release, UI re-embedded), `pixi run test`/`test-ui` green, `/api/conflicts` + `/api/agents`
verified live. Stack left running on :8765. A process-hygiene note: background fm-serve must be
started with `setsid … < /dev/null & disown` or the tool shell's exit kills it (the source of this
session's repeated restart churn).
