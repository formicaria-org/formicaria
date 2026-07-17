# Collaboration — design (nothing here is built)

_Written 2026-07-17; **revised the same day after a line-by-line re-audit** of every
claim against `c9cd1ad` — see [What the code actually says](#what-the-code-actually-says).
Like [roadmap.md](./roadmap.md), this file describes things that do **not** exist. It is
the design layer under the roadmap's collaboration line: the decisions, what was verified
against the code, and the order to build in. When a phase ships, delete it here and fold
the outcome into `overview.md` / `decisions.md`._

## Goal

**formicaria**: knowledge management, task scheduling, and collaboration, on files
you own. Multiple people work on **artifacts** (notes, boards, any file) and on
**schedules**, with personal and shared knowledge kept apart. Simplicity and
efficiency over power.

### Three pillars, one atom

Knowledge, scheduling, and collaboration are not three subsystems — they are three
views of one Markdown file. That is the whole reason this stays small: a task is a
note with a `due`; a message is a note with a target; a shared note is a note in a
different repo. **Resist every urge to add a fourth thing.** The moment scheduling
gets its own store, or messages get their own format, the tool has three products
to maintain and the atom stops paying rent.

### The name: formicarium → formicaria *(done — landed 2026-07-17 with Phase 2; nothing carries the old name)*

A *formicarium* is one ant colony's nest. The plural is the architecture: a set of
vaults, one per audience (3), each served by its own instance, coordinating through
git rather than through a hub (2, 6). The singular name describes the old
single-user tool; the plural describes what it becomes. Rename when Phase 2 lands
and the plural is true — not before, or the docs promise something that doesn't
exist.

**Cost: low, and known.** Every crate is already `fm-*` and the binary is `fm`,
which abbreviates either name — so **no code identifiers move**. The blast radius is
six user-facing strings (`fm-cli/src/main.rs:16`, `fm-serve/src/main.rs:63`,
`App.svelte`'s wordmark, and the Excalidraw `source` literal in
`App.svelte:253` / `mock.ts:171`), plus `docs/` and `packaging/`. Two cautions: the
Excalidraw `source` field is **written into every board's JSON on disk**, so
changing it changes files — leave it or migrate deliberately; and
`git.rs`'s placeholder identity. *(Updated: the audit called it a bug to delete; Phase 0
found deleting it breaks committing on any non-FQDN host, so it was kept as a **sentinel**
`identity()` matches by value. It moved to `formicaria@localhost` with the rename — safe
once, because no vault was on it. Changing it again is not safe: see `decisions.md`.)*

## Principles this plan is held to

- **The UI is the product, not the plan's afterthought.** Git does the heavy lifting
  precisely so the interface can be the thing we actually build. A correct backend
  with a developer's interface is what GitHub already is, and the reason nobody uses
  it for notes. Every phase below must land with the UI that makes it usable — a
  phase that ships only Rust is a phase that shipped nothing.
- **Bounded dependencies.** Lean on tools that already solve a problem completely
  (git, restic, Excalidraw, KaTeX) and shell out rather than reimplement. But each
  new dependency must earn its weight and be *replaceable*: the seams exist so the
  answer to "can we swap this?" is always yes. No CRDT library, no sync framework,
  no plugin API — those are the ones that would own us.
- **Reusable over clever.** Every feature here that is nearly free is nearly free
  because an existing seam pays for it: `Predicate::Prop` gives vault filtering,
  ULIDs give cross-vault links, backlinks give comments, the `Store` trait gives
  federation. If a proposal needs a new seam, that is the signal to re-read it.

## The strategic conclusion (read this first)

**The substrate is done — it is git. The storage layer is not.**

Every *distributed-systems* question (coordination, permissions, provenance,
history, proposals, conflict handling) already has a git answer, for zero lines.
That half of the original conclusion survived the audit intact: decisions 1–6
stand as written.

What did **not** survive is "~100% of the value is in the UI." Every defect the
audit found is in Rust, not Svelte. There is no staleness guard on *any* write; no
`pull`; a `push` that silently deletes a collaborator's work the moment anything
fetches (see 19); and — the one that reframes the whole doc — **the file format
manufactures the conflicts and the loader treats them as fatal**. `updated:` is
rewritten on every save (`frontmatter.rs:67`), so *any* two concurrent edits to one
note collide on that line and git must conflict. The markers land inside the YAML,
`from_file` returns `ParseError::Yaml` (`frontmatter.rs:97`), `reindex` propagates it
(`file.rs:218`), and `FileStore::open` fails (`file.rs:39`) — **the app will not
start**. Then the shipped auto-commit (`git add -A`, `git.rs:96`) commits the markers
as a resolution five seconds later and pushes them.

So the honest form: **the distributed system is designed; the single-user
assumptions baked into the storage layer are not, and they are most of the work.**
The doc's own Phase 1 half-admitted this while the headline denied it. Phase 0 below
is the admission made explicit. It is unglamorous, it is entirely backend, and
nothing ships without it.

The competitive claim stands unchanged: GitHub has the substrate and shows a
knowledge worker a developer's diff. Obsidian's Git plugin is a fiddly wrapper with
no review, no blob story, no vault-aware views. Notion has the multiplayer and owns
your data. **Nobody offers files you own + git underneath + a UI a non-git person
can use.** Thin if the UI is thin; a moat if it is good.

## What the code actually says

Re-audited against `c9cd1ad`. Every row was checked in the source, not recalled.

| Claim | Verdict |
|---|---|
| `fm_query::run` is pure over `&[Object]`, `Predicate::Prop` is generic → vault filter/group needs no query-engine change | **True.** `fm-query/src/lib.rs:115,48,141`; `Prop` evaluates `obj.get(key)`, which falls through to `extra` (`fm-model/src/lib.rs:159`). **Decision 5 holds as written.** |
| The `to_file`/`extra` trap is real | **True.** `Object::get` falls back to `extra` (`fm-model/src/lib.rs:159`) and `to_file` writes every `extra` key back (`frontmatter.rs:78`). |
| `update_body` does **not** do the mtime check `MASTERPLAN.md:319` claims | **True, and understated.** `set_property` (`commands.rs:129`) has the identical read-modify-write and is *not* named — see 19. |
| `Predicate::Text` is FTS5 in `FileStore`, substring in the pure engine | **True.** `file.rs:181-197` vs `fm-query/src/lib.rs:145`. Federated search can't union — but the fix is not the one Phase 2 proposes. |
| Excalidraw exports `reconcileElements`; elements carry `version`/`versionNonce` | **True** (`@excalidraw/excalidraw@0.18.1`, `types/excalidraw/index.d.ts:14`, `element/types.d.ts:50,54`) — **but the conclusion drawn from it is wrong.** See 10. |
| The `Store` trait is the seam | **True**, 5 methods (`fm-core/src/lib.rs:54`). But `load_all` is a **private inherent method on `FileStore`** (`file.rs:107`), not on the trait — Phase 2's "add `load_all` to the trait" is a real change, and the wrong one. See Phase 2. |
| *(new)* `FileStore::get` reads **SQLite, not the file** | `file.rs:140`. `reindex` runs only at `open()`. A `git pull` is therefore **invisible** to a running app — the local poll isn't a Vim nicety, it is the only thing that makes a pull observable at all. |
| *(new)* `reindex` ignores its `_mode` and always rebuilds fully | `file.rs:199`. A 3 s poll would re-parse the entire vault every 3 s. The incremental path is a **dependency** of the poll, not a later nicety. |
| *(new)* `ensure_identity` writes a placeholder identity repo-locally | **Was true; fixed in Phase 0.** A user with no global git config attributed **every** commit in a shared vault to the same fake identity, gutting the provenance decisions 2 and 14 are built on. Now: the placeholder is kept only for a vault with no remote, `git::identity()` reports it as nobody, and `set_remote` refuses while it stands. The audit's implied fix — *delete the fake* — was wrong: git cannot auto-detect an identity on a non-FQDN host, so a fresh vault would fail to commit, silently (the auto-commit swallows errors). |

## Converged decisions

1. **Real-time does not exist.** CRDTs don't remove conflicts, they resolve them by
   fiat (hence interleaving anomalies). It is always coordinated async access.
   Target **fast-async**; `MASTERPLAN.md:448` ("no real-time collaboration, ever")
   stands unamended.
2. **Don't build a coordinator — the bare repo is it.** A push is an **atomic
   compare-and-swap on a ref**: be current or be rejected, then pull/merge/retry.
   That is literally "I move, you do not". Free with it: provenance, history as
   transcript, policy via server-side hooks, proposals via branches. *(Verified:
   `git.rs:251` is a plain `git push`, which is non-fast-forward-rejecting. The CAS
   is real. `push_squashed` around it is **not** — see 19. And "policy via hooks"
   exists only on a self-hosted peer; GitHub gives you branch protection instead.)*
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
   `Store` trait over local clones. Re-audited and **confirmed**: `fm_query::run` is
   pure over `&[Object]` and `Predicate::Prop` is already generic → filter/group by
   vault needs **no fm-query change**. Cross-vault links are free (ULIDs are globally
   unique — the problem Dendron needed a URI scheme for). `put` routes by
   `obj.vault`; that is the only place a note crosses vaults.
6. **Replicate, don't RPC.** Federating slow nodes = same work + serialization.
   Laptops sleep; availability beats freshness. The winners replicate (Matrix,
   CouchDB, git, Syncthing); remote-query federation struggles (SPARQL `SERVICE`,
   Solid). RPC buys only "query what you may not hold" — a *governance* feature,
   later, needing auth.
7. **The always-on peer is two products; only one is free.** As a **git remote with
   uptime** it is genuinely zero-architecture — it is a bare repo, and GitHub already
   is one. As a **web UI for guests** it is a second product, and "needs auth — a
   deployment choice, not an architecture change" is **false**. `fm-serve` is
   `//! Localhost only, single user` (`main.rs:8`): its CSRF guard allows any request
   with **no `Origin` header** on the stated grounds that "there are no ambient
   credentials to abuse" (`main.rs:169-180`) — adding a session cookie inverts that
   premise and turns the rule into an open door. Its allowed origins are hardcoded to
   `127.0.0.1`/`localhost` at the configured port (`main.rs:41-43`), so any other host
   is refused. It has no TLS. `open_external` runs `xdg-open` **on the server**
   (`main.rs:287`), and one `Mutex<FileStore>` (`main.rs:21`) is every guest's write
   lock on one vault. **Take the bare-repo half; the guest UI is unbudgeted and out of
   scope until someone writes its threat model.**
8. **Latency: poll, and stop there.** GitHub + `ls-remote` every 15–30 s is well
   inside rate limits (use HTTPS, not SSH). ~~A peer pushing a "ref moved" SSE
   ping~~ — **cut.** Decision 12 declares 20 s "unremarkable"; an SSE fast-path to
   reach 1–2 s optimizes a latency this design has already ruled a non-problem, and it
   only exists on the peer that decision 7 just descoped. Poll, degrade, move on.
9. **Heavy assets: restic is backup, not distribution.** One password = all-or-
   nothing access to every snapshot. Distribution wants **blob mirrors**: content-
   addressed + immutable = a static file server keyed by hash (`GET /sha256/ab/cd/
   <hash>`). ~~Several tried in order~~ — one mirror; a fallback chain is ceremony.
   The existing rule *media absence is a warning, never an error* already makes
   partial blob availability work everywhere. **This is the same idea roadmap.md
   already reasons through** under "An append-only blob mirror" (`rclone copy`,
   idempotent by hash) — decide it there, once, not twice.
10. **Blackboards are the differentiator — but `reconcileElements` is the wrong
    tool, and the original plan resurrects every deleted shape.** Verified in the
    vendored `@excalidraw/excalidraw@0.18.1`:
    - `reconcileElements(local, remote, appState)` is exported and elements do carry
      `version`/`versionNonce` (`element/types.d.ts:50,54`). Both original claims are
      true.
    - **It is a 2-way merge with no base**, and its tail loop unions unconditionally
      (`dist/dev/index.js:32880-32885`): anything present on either side survives.
    - **The file format has no tombstones to honour.** `_clearElements` is
      `elements.filter(e => !e.isDeleted)` and backs **both** `clearElementsForExport`
      *and* `clearElementsForDatabase` (`chunk-4FTI6OG3.js:24610-24614`); this repo's
      `Whiteboard.svelte` writes `serializeAsJSON(…, 'local')`. So `isDeleted` never
      reaches disk. **Union + no tombstones = every delete is undone by the next
      merge.** "Honour `isDeleted`" was un-implementable as written.
    - `shouldDiscardRemoteElement` reads `localAppState.editingTextElement/
      resizingElement/newElement` (`index.js:32828`) — live UI state a merge driver
      does not have. And the bundle is browser ESM (`window?.…` at `index.js:32841`,
      `react`/`lodash.throttle` imports) invoked by a git driver that runs outside
      `pixi run`. "Reusing Excalidraw's own semantics" is really reusing ~15 lines of
      version comparison.

    **The driver is still viable — and better than proposed, because git hands it the
    base (`%O`).** That is exactly what `reconcileElements` lacks. A 3-way rule
    recovers deletion intent structurally: in base & gone from one side → **honour the
    delete**; absent from base → an add, keep; in both → higher `version` wins,
    tie-break lower `versionNonce`; deleted one side & version-bumped the other →
    the one real conflict, pick a rule and state it. ~80 lines of **Rust in `fm-cli`**
    — no node, no bundle, no AppState, and it works in a bare terminal. Verified safe:
    `restore` preserves `version`/`versionNonce` (`chunk-4FTI6OG3.js:20454-20455`), so
    merely opening a board does not make its elements win. Watch the fractional `index`
    (`element/types.d.ts:59`) — concurrent inserts need re-indexing, and z-order is the
    merge hazard nobody listed. **Nobody has this**: Miro can't (no file), GitHub can't
    (doesn't know a scene), Obsidian's plugin just conflicts. Still the demo — see 20
    for the driver that actually earns its place first.
11. **Discussion = notes. One file per message, ULID-named.** Thread-as-one-file
    conflicts on every concurrent append; one-file-per-message means nobody ever
    touches the same file (git merges trivially) and ULIDs give chronological order
    free. That is Maildir, proven for 30 years, and it is "the atom is the file"
    paying off. Lives **in the vault it is about** (audience matches; ULIDs
    resolve). **This beats GitHub**: GitHub's issues/PR comments live in its
    database, not in git — clone the repo and the reasoning is gone. Yours clones,
    works offline, and is searchable in five years. **Unbudgeted cost:** a message is
    a `Kind::Note`, so a 20-message thread is 20 cards in the board's `(none)` column
    and 20 entries in the Timeline. This is precisely the pollution the 2026-07-16
    assets decision fixed with `Predicate::Kind(vec![Kind::Note])` — and there is no
    kind left to exclude by, because the model says *resist adding kinds, add
    properties* (`fm-model/src/lib.rs:20`). So messages need a **property-based
    exclusion in `board`/`agenda`/`recent`**, a query-layer change on the same seam.
    Cheap, but it is not free and it must land with the feature, not after.
12. **Never call it chat.** The name is the latency contract: 20s is unremarkable
    for a comment thread and broken for "chat". The tool owns durable, anchored,
    contextual discussion; **Signal/Slack owns ephemeral coordination** — and
    coordination chatter *should not* be in a knowledge base.
13. **Anchored comments, not chat, are the real gap — and they are not nearly free.**
    GitHub has line comments, Figma/Docs have pinned comments, note tools have none. A
    comment is a note linking to a target → **the comments panel is the backlinks
    panel** (one mechanism, two features). That elegance is real, but it inverts the
    dependency: **backlinks do not exist.** The forward half shipped 2026-07-16; the
    reverse index is still an open roadmap item needing a body scan on reindex or a
    `links` table. So this feature *is* backlinks, plus a panel. Budget it as such —
    and note it lands the anchor for free on boards, since Excalidraw elements have ids
    (Figma behaviour).
14. **Awareness over enforcement.** Humans coordinate socially; the tool's job is
    to make collisions visible and cheap, not to prevent them. "Ravi pushed 2 min
    ago" is a `git log` query and beats any lock. Honest limit: git knows only
    *pushed* state — true presence needs the peer. **Second honest limit, new:** it
    also needs everyone to *have a name*. `ensure_identity` (`git.rs:62-72`) writes
    the placeholder identity repo-locally whenever `git config user.email`
    is unset — which is the default state of a researcher who never configured git. The
    app must **ask for an identity when a vault gains a remote**, or every "who touched
    this" answer in the product is the same fake name. Phase 0.
15. **Ephemeral state is never a file.** *If it would embarrass you in five years,
    don't commit it. Status is not knowledge.* Presence/status → **derived** from
    git log or pushed by the peer, stored nowhere. Only durable discussion is a file.
    ~~Locks → a custom ref (`refs/fm/locks`)~~ — **cut.** Decision 14 spends a
    paragraph arguing awareness *beats any lock*, then this one designs the lock.
    Pick one; 14 is right. For a researcher and two collaborators a lock ref is pure
    ceremony, and the git-bug/git-appraise precedent is for tools with no other channel.
16. **Forgetting is a clone flag, not a feature.** `--depth=N` costs nothing to
    reach for later. **Caveat on `--filter=blob:limit=200k`:** in a notes repo the
    only git objects over 200k *are the boards*, so that filter makes opening a
    whiteboard a network round-trip — the opposite of the intent. Zero code either way;
    this is a note, not a decision. Scenes as content-addressed blobs + retention would
    fight both the merge driver and files-as-truth. Not now.
17. **Size: notes are cheap, boards are the risk — and the risk is images, not
    strokes.** 10k markdown notes ≈ tens of MB packed; text deltas well. Scenes are
    100KB–1MB and rewritten *wholesale* per edit (`Whiteboard.svelte` re-serializes on
    every `onChange`, debounced 600 ms) — that is where size accumulates. **Worse, and
    unlisted:** `serializeAsJSON` embeds `files` inline (`chunk-4FTI6OG3.js:17935`) and
    `BinaryFileData.dataURL` is a **base64 data URL** (`types.d.ts:65-68`). So an image
    pasted onto a board is base64 **inside the note body**, in git, re-written whole on
    every pointer move — routing around the content-addressed blob store entirely and
    breaking decision 9's premise that "blobs are already out of git". A 2 MB
    screenshot becomes ~2.7 MB of churn per stroke. Decide this before boards are
    shared: strip `files` to the blob store on save, or accept it and say so.
18. **Branches/PRs are overkill for daily notes.** Branching is a developer skill.
    It earns its place in exactly two cases: an **agent proposing** changes to
    shared knowledge (human gate), and a big restructuring. Otherwise push to main
    and talk to each other.
19. **`push_squashed` + any fetch = silent data loss, and the obvious guard doesn't
    catch it.** `push_squashed` does `reset --soft <tracking-ref>` (`git.rs:227`), which
    moves HEAD but leaves the index holding **our** tree. Today that is safe only
    because nothing ever fetches, so the ref is stale by construction
    (`decisions.md`). Once anything fetches — the Phase 1 remote poll, a `pull`, a
    terminal `git fetch` — `origin/main` advances to *their* tip, `reset --soft` lands
    on it, and the commit that follows has their tip as parent and our tree as content:
    **their work is deleted, and the push fast-forwards cleanly.** The trap: the
    existing `count_ahead` check *passes* in exactly this case (there are commits in
    HEAD not in the ref — that is what divergence means), so "re-check divergence"
    reads as satisfied. The correct guard is an **ancestry** test, not a count:
    `git merge-base --is-ancestor <tracking> HEAD` — squash only when the base is an
    ancestor of HEAD; otherwise refuse and tell the user to pull.
20. **Merge drivers are the differentiator — and `.md` earns it before
    `.excalidraw`.** Same infrastructure (`.gitattributes` + an `fm merge-*`
    subcommand), but the `.md` driver fires on *every* concurrent note edit while the
    board driver fires when two people draw on one canvas. A frontmatter-aware `.md`
    driver resolves the manufactured conflict structurally — `updated` = max,
    `id`/`created` = base, `tags` = union, body = ordinary 3-way — and is the single
    highest-value item in this document. **Deployment trap:** `.gitattributes` is
    tracked and travels; the `merge.fm.driver` *definition* lives in `.git/config` and
    does **not** (git refuses, deliberately). A collaborator who clones and never runs
    fm silently gets git's built-in text merge and conflict markers anyway. So
    `ensure_repo` must install the driver config, exactly as it already writes
    `.gitignore` and the committer identity (`git.rs:36-57`).
21. **Multi-vault forces the first app-level config file.** `decisions.md` is proud
    that the backup panel stores the remote in the vault's own `.git/config` — "no new
    config file". A *list of vaults* cannot live inside any one vault, and `FM_VAULT` is
    a single path (`main.rs:36`). Small, but it is a stated principle breaking, and it
    should break deliberately rather than by accident in a PR.

## Sequencing (do not reorder)

**Phase 0 — stop the app from dying on a merge. ✅ SHIPPED 2026-07-17**, all four items
— see [`sessions/2026-07-17-phase-0.md`](./sessions/2026-07-17-phase-0.md) and the
identity entry in [`decisions.md`](./decisions.md). The detail is deleted per this file's
own rule; what remains worth carrying: the tolerant loader's *report* half is only stderr
(the in-app list is Phase 1's conflict surfacing), and the placeholder identity was **not**
deleted as 14 assumed but kept as a sentinel `identity()` matches by value — git cannot
invent an identity on a non-FQDN host, so deleting the fallback would have stopped a fresh
vault committing at all. **The gate is open.**

**Phase 1 — make one shared vault trustworthy.** Nothing else matters until this
is done; multi-vault on top of it is prettier navigation over data that silently
loses work.
- **The staleness guard belongs in `FileStore::put`, not `update_body`.**
  `MASTERPLAN.md:319` claims `update_body` does an mtime check. **It does not** —
  `mtime_ns` is stored (`file.rs:53,89`) and read by nothing. But `update_body`
  (`commands.rs:107`) is not the only offender: `set_property` (`commands.rs:129`) is
  the same read-modify-write, so a board drag after a pull rewrites the **whole file**
  — body included — from stale state, which is worse, because the user never thought
  they were writing prose. Both read `Store::get`, which reads **SQLite, not the file**
  (`file.rs:140`). One guard in `FileStore::put` — compare `mtime_ns(path)` against the
  indexed `mtime_ns`, return `StoreError::Conflict` — covers both, plus every future
  writer, and keeps mtime out of the trait (`fm-core/src/lib.rs:52`: "nothing
  filesystem-shaped leaks through"). This is *the* lost-update bug.
- **Incremental reindex, then the 3 s local poll.** Ordered: `reindex` ignores its
  `_mode` and always rebuilds fully (`file.rs:199`), so a 3 s poll re-parses the whole
  vault every 3 s. The poll is also more load-bearing than "needed for Vim anyway"
  suggests — `get`/`query` serve SQLite, so **without it a pull is invisible to a
  running app**.
- **A 15–30 s remote `ls-remote` poll** (fetch only when the ref moved). `ls-remote`
  does not move the tracking ref; the *fetch* it triggers does — hence Phase 0's
  ancestry guard first.
- **`pull`.**
- **The `.md` merge driver** (decision 20). Promoted out of Phase 3: it is what makes
  a pull survivable rather than a thing you recover from.
- **Conflict surfacing** — list conflicted notes; resolve markers in the textarea.
  **Do not build a merge UI.** Depends on Phase 0's tolerant loader.
- **Semantic line breaks** — a convention, zero code. Real, but **demoted**: it
  improves *body* merges, and the body is not where the conflicts come from.

**Phase 2 — multi-vault.** `Object.vault` (real field, explicit `get()` arm,
**nothing in `to_file`**), `MultiStore`, `ObjectMeta.vault`, badges + a vault filter
in the UI, and the vault-list config file (21).

**The trap:** `Object::get` falls back to `self.extra` (`fm-model/src/lib.rs:159`)
and `to_file` writes **every** `extra` key back (`fm-core/src/frontmatter.rs:78`).
Putting `vault` in `extra` would serialize location into frontmatter — making the
permission a forgeable content field. Test the byte round-trip first. *(Note the
inverse, since `from_file` has no `vault` arm: a hand-typed `vault:` key lands in
`extra` and round-trips forever while `get("vault")` correctly ignores it. Harmless
for the boundary, confusing on disk — strip it in `from_file`.)*

**Not free — and `load_all` is the wrong seam.** `Predicate::Text` is a substring scan
in the pure engine; `FileStore` overrides it with FTS5, which indexes extracted PDF
text (`file.rs:181-197`). Phase 2's original "add `load_all` to the `Store` trait" then
forces `MultiStore` to delegate `query` per-child and merge `QueryResult`s — which
**cannot be right in general**: you cannot union two already-sorted, already-grouped,
already-paginated results and recover `sort`/`group_by`/`limit`/`offset`/`total`. It
happens to work for `search` only because `search` has no limit and no group-by. The
seam that actually federates is the one `FileStore::query` already uses internally:
**`fn candidates(&self, f: &Filter) -> Result<(Vec<Object>, Filter), StoreError>`** —
return the FTS-narrowed candidate set *and the residual filter*, then `Store::query`
becomes a default method running `fm_query::run` **once** over the union of the
children's candidates. `MultiStore::candidates` is a concat. FTS federates for free,
grouping and pagination stay correct, and `fm-query` still never learns storage exists.
(The residual filter is load-bearing: re-evaluating `Text` over FTS hits would drop
diacritic-folded matches, because FTS5 is pinned to `remove_diacritics 2` and the
substring scan is not.)

**Phase 3 — the differentiators.** The `.excalidraw` merge driver as re-specified in
10 (3-way over `%O`, in Rust, not `reconcileElements`) — plus a ruling on 17's
base64-image bomb before boards are shared at all. Then backlinks → anchored comments
→ discussion (13's real order; the panel is the last 10%). The peer's SSE ping is
**cut** (8).

## Verification

- `pixi run ci` green.
- **Phase 0 first, and testable without a UI**: a note containing conflict markers
  must not stop `FileStore::open`; `commit_all` must refuse while `MERGE_HEAD` exists;
  `push_squashed` must refuse when the tracking ref is not an ancestor of HEAD (the
  regression test for 19 is a fetch, then a push, then assert their commit still
  reachable).
- **The lost-update test is `set_property`, not just `update_body`**: pull a change,
  then drag the card, then assert the pulled body survived.
- **Byte round-trip first**: `vault:` never appears in any `.md`.
- `MemoryStore`/`FileStore` equivalence still holds across the `candidates` refactor.
- Two vaults: board/agenda/timeline show both with badges; filtering hides one; a
  `note:` link across vaults resolves. Search still uses FTS5 (a PDF's extracted
  text is still findable), not a substring scan — and still does when federated.
- `.md` driver: two clones edit *different* lines of one note → clean auto-merge with
  no `updated:` conflict. That is the test that says shared vaults work.
- `.excalidraw` driver: two clones move *different* shapes → clean auto-merge; the
  same shape → deterministic winner, no conflict markers; **and one clone deletes a
  shape the other did not touch → it stays deleted.** That last one is the case
  `reconcileElements` cannot pass.
