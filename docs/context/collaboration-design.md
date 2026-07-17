# Collaboration — design (nothing here is built)

_Written 2026-07-17. Like [roadmap.md](./roadmap.md), this file describes things
that do **not** exist. It is the design layer under the roadmap's collaboration
line: the decisions, what was verified against the code, and the order to build
in. When a phase ships, delete it here and fold the outcome into `overview.md` /
`decisions.md`._

## Goal

Multiple people collaborate on **artifacts** (notes, boards, any file) and on
**task scheduling**, with personal and shared knowledge kept apart, and full
control over the files. Simplicity and efficiency over power.

## The strategic conclusion (read this first)

**The architecture is done — it is git.** Every hard question (coordination,
permissions, provenance, history, proposals, conflict handling) already has a git
answer, for zero lines. Which means **~100% of the value is in the UI**, and that
is where the work is. Stop designing the distributed system; design the
collaboration UI.

That is also the honest answer to "are we rebuilding GitHub?" — no: GitHub has the
substrate and shows a knowledge worker a developer's diff. Obsidian's Git plugin
is a fiddly wrapper with no review, no blob story, no vault-aware views. Notion has
the multiplayer and owns your data. **Nobody offers files you own + git underneath
+ a UI a non-git person can use.** Thin if the UI is thin; a moat if it is good.

## Converged decisions

1. **Real-time does not exist.** CRDTs don't remove conflicts, they resolve them by
   fiat (hence interleaving anomalies). It is always coordinated async access.
   Target **fast-async**; `MASTERPLAN.md:448` ("no real-time collaboration, ever")
   stands unamended.
2. **Don't build a coordinator — the bare repo is it.** A push is an **atomic
   compare-and-swap on a ref**: be current or be rejected, then pull/merge/retry.
   That is literally "I move, you do not". Free with it: provenance, history as
   transcript, policy via server-side hooks, proposals via branches.
3. **Vaults are audiences.** One vault = one repo = one remote = one collaborator
   list. A **set** of vaults (personal / lab / paper-with-X), **not** one dedicated
   shared vault. Contexts are plural.
4. **Location is the permission — never an access field.** Git replicates
   *repositories, not files*; a frontmatter `access:` label has zero enforcement
   power, and **git history is forever**, so one typo (or one agent mistake) is
   permanent disclosure to everyone who cloned. `Object.vault` **is** the access
   quantifier — derived from location, never serialized. You get the
   one-vault-with-labels UX with a boundary that cannot be typo'd.
5. **Federated views, per-vault writes.** `MultiStore` implements the existing
   `Store` trait over local clones. Audited: `fm_query::run` is pure over
   `&[Object]` and `Predicate::Prop` is already generic → filter/group by vault
   needs **no fm-query change**. Cross-vault links are free (ULIDs are globally
   unique — the problem Dendron needed a URI scheme for). `put` routes by
   `obj.vault`; that is the only place a note crosses vaults.
6. **Replicate, don't RPC.** Federating slow nodes = same work + serialization.
   Laptops sleep; availability beats freshness. The winners replicate (Matrix,
   CouchDB, git, Syncthing); remote-query federation struggles (SPARQL `SERVICE`,
   Solid). RPC buys only "query what you may not hold" — a *governance* feature,
   later, needing auth.
7. **The always-on peer is a peer with uptime, never an authority.** It gives a
   reliable replica, a **web UI for guests** (nothing to install, nothing to
   clone), and the latency fix below. Needs auth (not built) — a deployment
   choice, not an architecture change.
8. **Latency is the remote's owner, not git.** GitHub + `ls-remote` every 5s ≈
   5–10s (well inside rate limits; use HTTPS, not SSH). Own peer ≈ 1–2s. Best:
   the peer pushes a **"ref moved" ping (SSE)** carrying no data; clients pull via
   git. Data path stays git; only the wakeup is pushed; degrades to polling.
9. **Heavy assets: restic is backup, not distribution.** One password = all-or-
   nothing access to every snapshot. Distribution wants **blob mirrors**: content-
   addressed + immutable = a static file server keyed by hash (`GET /sha256/ab/cd/
   <hash>`), several tried in order. The existing rule *media absence is a warning,
   never an error* already makes partial blob availability work everywhere.
10. **Blackboards are the differentiator.** Verified: Excalidraw ships
    `reconcileElements(local, remote, appState)` and elements carry
    `version`/`versionNonce`. So a **git merge driver for `.excalidraw`** (union by
    element id, higher version wins, tie-break nonce, honour `isDeleted`) makes
    whiteboards **merge like text** — ~100 lines, reusing Excalidraw's own
    semantics, no base revision needed. **Nobody has this**: Miro can't (no file),
    GitHub can't (doesn't know a scene), Obsidian's plugin just conflicts. This is
    "visual collaboration + full control" in one feature.
11. **Discussion = notes. One file per message, ULID-named.** Thread-as-one-file
    conflicts on every concurrent append; one-file-per-message means nobody ever
    touches the same file (git merges trivially) and ULIDs give chronological order
    free. That is Maildir, proven for 30 years, and it is "the atom is the file"
    paying off. Lives **in the vault it is about** (audience matches; ULIDs
    resolve). **This beats GitHub**: GitHub's issues/PR comments live in its
    database, not in git — clone the repo and the reasoning is gone. Yours clones,
    works offline, and is searchable in five years.
12. **Never call it chat.** The name is the latency contract: 20s is unremarkable
    for a comment thread and broken for "chat". The tool owns durable, anchored,
    contextual discussion; **Signal/Slack owns ephemeral coordination** — and
    coordination chatter *should not* be in a knowledge base.
13. **Anchored comments, not chat, are the real gap.** GitHub has line comments,
    Figma/Docs have pinned comments, note tools have none. A comment is a note
    linking to a target → **the comments panel is the backlinks panel** (one
    mechanism, two features). Excalidraw elements have ids, so comments can pin to
    a board element — Figma behaviour, nearly free.
14. **Awareness over enforcement.** Humans coordinate socially; the tool's job is
    to make collisions visible and cheap, not to prevent them. "Ravi pushed 2 min
    ago" is a `git log` query and beats any lock. Honest limit: git knows only
    *pushed* state — true presence needs the peer.
15. **Ephemeral state is never a file.** *If it would embarrass you in five years,
    don't commit it. Status is not knowledge.* Presence/status → **derived** from
    git log or pushed by the peer, stored nowhere. Locks → a **custom ref**
    (`refs/fm/locks`), force-updatable, never merged to main, no trace in history
    (how git-bug/git-appraise work). Only durable discussion is a file.
16. **Forgetting is a clone flag, not a feature.** `--filter=blob:limit=200k`
    (partial clone: notes always, big-file history lazily) and `--depth=N`. Zero
    code. Later, if boards bite: scenes as content-addressed blobs + a retention
    policy, letting the missing-blob placeholder do the degrading — but it fights
    the merge driver and files-as-truth, so not now.
17. **Size: notes are cheap, boards are the risk.** 10k markdown notes ≈ tens of MB
    packed; text deltas well; blobs are already out of git. Excalidraw scenes are
    100KB–1MB and rewritten *wholesale* per edit — that is where size accumulates.
18. **Branches/PRs are overkill for daily notes.** Branching is a developer skill.
    It earns its place in exactly two cases: an **agent proposing** changes to
    shared knowledge (human gate), and a big restructuring. Otherwise push to main
    and talk to each other.

## Sequencing (do not reorder)

**Phase 1 — make one shared vault trustworthy.** Nothing else matters until this
is done; multi-vault on top of it is prettier navigation over data that silently
loses work.
- **The staleness guard on write.** `MASTERPLAN.md:319` claims `update_body` does
  an mtime check. **It does not** — `mtime_ns` is stored (`file.rs:53,89`) and read
  by nothing. Today a pulled-in change is silently clobbered by the note you have
  open. This is *the* lost-update bug.
- The **3s local mtime poll** (unmet S0/S2 criterion; needed for Vim anyway) and a
  **15–30s remote `ls-remote` poll** (fetch only when the ref moved).
- **`pull`.** Caution: `push_squashed` relies on a *stale* tracking ref to reject a
  diverged push — adding fetch means it must re-check divergence, not assume it.
- **Conflict surfacing** — list conflicted notes; resolve markers in the textarea.
  **Do not build a merge UI.**
- **Semantic line breaks** — a convention, zero code, the biggest single
  improvement to merge quality.

**Phase 2 — multi-vault.** `Object.vault` (real field, explicit `get()` arm,
**nothing in `to_file`**), `load_all` on the `Store` trait, `MultiStore`,
`ObjectMeta.vault`, badges + a vault filter in the UI.

**The trap:** `Object::get` falls back to `self.extra` (`fm-model/src/lib.rs:159`)
and `to_file` writes **every** `extra` key back (`fm-core/src/frontmatter.rs:78`).
Putting `vault` in `extra` would serialize location into frontmatter — making the
permission a forgeable content field. Test the byte round-trip first.

**Not free:** `Predicate::Text` is a substring scan in the pure engine; `FileStore`
overrides it with FTS5 (which indexes extracted PDF text). `MultiStore` must
**delegate search per-child and merge**, unlike every other view.

**Phase 3 — the differentiators.** The `.excalidraw` merge driver; discussion +
anchored comments (= backlinks); the peer's "ref moved" ping.

## Verification

- `pixi run ci` green.
- **Byte round-trip first**: `vault:` never appears in any `.md`.
- `MemoryStore`/`FileStore` equivalence still holds with `load_all` on the trait.
- Two vaults: board/agenda/timeline show both with badges; filtering hides one; a
  `note:` link across vaults resolves. Search still uses FTS5 (a PDF's extracted
  text is still findable), not a substring scan.
- Merge driver: two clones move *different* shapes → clean auto-merge; the same
  shape → deterministic winner, no conflict markers.
