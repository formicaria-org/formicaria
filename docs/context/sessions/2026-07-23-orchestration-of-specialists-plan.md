# 2026-07-23 — plan: orchestration of tiny specialists (command-routed), grounded research first

The owner asked to "improve the orchestration" before returning to the collaboration-matrix work, then
steered a broad vision over several turns. Seven web-research passes + four adversarial audits (all
honoring the ≤5-agent/1M-token cap) produced an **approved lean plan**:
`~/.claude/plans/staged-drifting-shamir.md`. This note is the durable pointer; the plan file is the
working design (deliberately **not** persisted as a big canon doc here). The collaboration **matrix is
on the backlog** (owner). After the plan was approved, the owner said "go" and the **v1 lead —
grounded research — was built** (see "Built" below).

## Built this session — the grounded research profile core (v1 lead), all hermetic

- **`crates/fm-agent/src/grounding.rs` (new):** the deterministic grounding heart — `pack_sources`
  (numbered `[n]` pack), `GROUNDED_WRITE_INSTRUCTION` (quote-first contract), and `verify()`:
  substring-verify each claim's verbatim quote against its cited source (unicode/whitespace-normalized),
  **drop the unsupported**, assemble the note + a `## Sources` list built by us (un-hallucinable). Pure,
  no model/network. 12 tests incl. realistic well-known-knowledge scenarios (photosynthesis; a "visible
  from space" myth the model believes is rejected; honest "not supported").
- **`search.rs`:** `with_engines()` + `DEFAULT_ENGINES = [wikipedia, duckduckgo, github, arxiv]` — one
  loopback SearXNG, multi-engine; user-added sources = engine selection + `site:` filters. No new dep,
  no TLS client (research: arXiv/GitHub are just engines on the same rails; full-page fetch via a local
  reader proxy is a later spike).
- **`lib.rs`:** `research()` grounded pipeline (refine → multi-engine search → number sources →
  quote-first write → `verify`) returning `Research { draft, grounded }`.
- **`fm-agent-run`:** `/research` command (`convo::parse`) → `Agent::research_turn` — searches, writes,
  verifies, and **appends** (additive, never clobbers) the cited note to the host note as a
  **proposal** (insertion-only, human-merged); web-off handled.
- **`crates/fm-agent/tests/live_research.rs`:** network-gated `#[ignore]` end-to-end test against a real
  local SearXNG + model (checks format + real links); CI-safe. Run:
  `FM_SEARXNG_PORT=… FM_MODEL_PORT=… pixi run cargo test -p fm-agent --test live_research -- --ignored --nocapture`.
- **Live `/research` wiring:** `fm-agent-run::research_turn` runs the pipeline on an `@name /research`
  mention and **appends** (additive, never clobbers) the cited note to the host note as a proposal.
- **Multi-source proxy:** rewrote `agents/search-proxy.py` to fan out across wikipedia/duckduckgo/
  github/arxiv via their proper APIs (not scraping), normalized + interleaved; honors `?engines=`.
  wikipedia/arxiv/github are reliable API sources; DuckDuckGo scraping is best-effort (bot-blocks under
  load) and degrades gracefully.
- **Clickable-link citations:** every statement's `[n]` is an inline Markdown link to its exact source
  (`[\[n\]](url)`), confirmed rendered clickable by `marked`. Requirement: web-search facts each carry
  their link.
- **UI command palette:** tappable `/research` `/search` `/propose` chips in the discussion compose
  (`NotePanel.svelte` + tested `ui/src/lib/agentCommands.ts`) — one-tap on phone, still typeable.

## Two real bugs found by end-to-end testing (both fixed + regression-tested)
1. **Watcher disabled propose/research** — the resident `@name` watcher (the frontend path) called
   `handle(id, intent, false, …)`, so `/research` (and `/propose`) silently degraded to a chat answer.
   Fixed to pass `true` (the discussion root is always a note; `handle` still only acts on the explicit
   command). `crates/fm-agent-run/src/watch.rs`.
2. **Relative vault path doubled the git index path** — `create_proposal` built `temp_index =
   vault.join(".git")…` and passed it as `GIT_INDEX_FILE`, but `git()` runs with the vault as cwd, so a
   **relative** vault (`FM_VAULT=vault`, as `pixi run serve` sets) re-resolved it to `vault/vault/.git`
   → **every proposal failed with a 500**. Hidden in prod (absolute vault paths). Fixed by
   canonicalizing to an absolute index path (`crates/fm-core/src/git.rs`), + regression test
   `it_works_when_the_vault_path_is_relative` in `crates/fm-core/tests/proposal_branch.rs`.

- **Verified end-to-end through the real app** (fm-serve + auto-spawned agent + real Qwen3-4B + the
  multi-source proxy): `@qwen3-4b-2507 /research "What is RAG?"` → a grounded proposal with 5 claims,
  each quote-verified against 7 wikipedia+arxiv sources, clickable links, additive to the host note.
## The PR refine cycle — a note's living proposal (owner: "PR discussion IS the note discussion")

Built + verified live (real model + multi-source search through fm-serve). The design the owner
converged on: **the PR discussion is the note's own discussion — not separate.** A note has **one living
proposal**; `/research` or `/propose` again **refines the same PR** (never stacks), and the PR shows
**inside the note's discussion view**.
- **`create_proposal` is now an upsert** (`crates/fm-app/src/commands.rs`): if the note already has an
  OPEN proposal (a proposal note that `targets` it — new `thread::TARGETS` — with a live branch, via new
  `git::branch_open`), it **revises that branch** (delete+rebuild same `proposal/<id>`) and returns the
  same proposal note. Else opens a new one recording `targets`.
- **`proposal_for(note)`** (command + dispatch + `ipc.proposalFor`) → the note's open PR id, so
  `NotePanel` renders `ProposalReview` (diff + Accept/Reject) inside the discussion, refreshed on open,
  on poll (agent works async), and after accept/reject.
- **Reject keeps a declined record** (owner's call — symmetric with accept keeping a merged note): it
  drops the branch but KEEPS the proposal note, marked `thread::DECLINED`; `ProposalDiff` gains a
  `declined` flag so the UI labels it. `proposal_diff` also tolerates a *gone* note (return
  `exists:false`) — fixes the "no such proposal" error a stale list / just-rejected PR hit.
- Verified end-to-end: create → `/research` (PR appears in the note view) → `/research` again (SAME PR
  revised, count unchanged) → Accept (merges, PR closes) / Reject (kept as declined record).
- Tests: `proposal_create.rs` (refine-same-PR, uncommitted-note-merges, reject-keeps-declined),
  `ProposalReview.svelte.test.ts` (accept + reject), 285 UI tests.

## Live-found bugs fixed this session (all regression-tested)
1. Watcher passed `allow_propose=false` → `/research` degraded to chat in the frontend. (`watch.rs`)
2. Relative vault path (`FM_VAULT=vault`) doubled the git index path → every proposal failed; then the
   commit-before-propose fix itself had a relative-vs-absolute path bug. (`git.rs`, `commands.rs`)
3. Truncation on source-heavy queries → bounded pack + retry-tighter. (`lib.rs` research)
4. Reject deleted the proposal note → "no such proposal" on reload → now a kept declined record.

- **Status:** full workspace `pixi run test` green (60 suites); `ci/checks.sh` passes; UI 285 tests
  green; my changes clippy-clean. **Not committed** (owner commits on ask). Deployed + live at :8765.
  **Next:** vault config for user-added sources UI; grounding-quality tuning (fewer "not supported");
  then the shared safety substrate → laptop audio→transcript.

## The direction (approved)
Evolve the shipped single-model `@name` agent into a **modality-aware note assistant**: **narrow
specialist tools fired at the right moment, command-driven first, plus the always-present general LLM**
that also orchestrates them. Multimodality (text/math/audio/graphs) is **core**; scope is narrow —
**not** general vision / affect / intent-from-sound. Four goals: (1) grounded web research structured
into notes · (2) image→LaTeX · (3) audio→transcript · (4) the general LLM.

## What the research settled (cited in the plan)
- **Routing = a `match`-arm in the existing `fm_app::dispatch`**, not an ML router (the command already
  names the task; RouteLLM/NotDiamond/etc. recover intent we already have).
- **Chain, don't cascade** on edge (NVIDIA SLM thesis 2506.02153); specialist-first, escalate rarely.
- **Runtime = one-at-a-time load→run→unload + a fail-closed admission gate**; only **whisper.cpp** matches
  the shipped out-of-band `llama-server` pattern. ONNX/OpenCV are *linked* native blobs; PyTorch OCR has
  no on-device runtime.
- **Grounding = orchestrator machinery, not model cleverness:** quote-first + **deterministic Rust
  substring-verify** + numbered sources; `local-deep-researcher` is ~200 lines of deterministic glue we
  reimplement (better) rather than adopt.

## Load-bearing safety invariants (see [[agent-orchestration-vision]] memory)
1. **Insertion-only** — never propose/perform deleting a source artifact.
2. **An out-of-band specialist never holds write-creds to the blob store** — media passed by value /
   read-only handle (the one residual data-loss path; lives in the process boundary).
3. Human-merge terminal gate; provenance-marked adjunct insertions; idempotency keyed on
   (blob-hash, specialist, model-version).

## Honest ceiling / next increment
- **Ceiling:** research = grounded lookup + light 2-hop (not multi-hop deep research); audio = laptop v1,
  phone spike; **image→LaTeX = laptop spike behind an onnxruntime gate, no defensible phone path in
  mid-2026**.
- **Next increment (v1 lead, if/when the owner starts building):** the grounded-research **text profile**
  on the already-loaded model (SearXNG + numbered pack + quote-first + substring-verify), hermetic, zero
  new runtime. Then the dispatch match-arm + safety substrate, then laptop audio→transcript.
- **Open (owner's call):** start building v1 now, or hold as the agreed design and resume the
  collaboration matrix. Commit only when asked.
