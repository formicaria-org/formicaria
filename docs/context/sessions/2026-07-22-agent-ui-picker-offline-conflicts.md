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

## Per-message author labels (agent vs person) — shipped

Message authorship is a **git fact** (`lastEditFor` ← the `activity` feed ← git commit author), and
the owner's insight was right: every collaborator already has a git identity, so reuse it. Added
`git::commit_all_as` / `vcs::commit_all_as` (the same `GIT_AUTHOR_*`/`GIT_COMMITTER_*` env trick
`create_proposal_branch` already uses), and the `"reply"` dispatch now, **when an author is given**,
commits *just that one message file* under it immediately — leaving the user's other pending writes
untouched, idempotent against the later batch. `FmServe::reply_as` sends the agent's model identity;
`handle` and the timeout-notice use it, while plain `reply` (recording a *user's* message) stays
unattributed. Verified live in the clean `notes` vault: the agent's reply is authored
`lfm2.5-230m <lfm2.5-230m@fm-agents.local>`, distinct from the owner's `baljinder` commits, so the
discussion's `EditedBy` now distinguishes them. Caveat: during a conflicted merge `commit_all` refuses
(correctly), so in the owner's currently-`UD`-conflicted `vault` the attribution defers until the
conflict is resolved.

## State

Built (release, UI re-embedded), `pixi run test`/`test-ui` green, `/api/conflicts` + `/api/agents`
verified live. Stack left running on :8765. A process-hygiene note: background fm-serve must be
started with `setsid … < /dev/null & disown` or the tool shell's exit kills it (the source of this
session's repeated restart churn).
