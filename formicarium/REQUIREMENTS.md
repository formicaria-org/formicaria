# Requirements & Architecture — Personal Research Workspace

**Status:** Draft v0.1 — for shredding
**Date:** 2026-07-14
**Owner:** (you)

---

## 1. Purpose

A single local tool where a researcher can:

1. Capture notes and ideas fast, without deciding where they go.
2. Retrieve prior ideas reliably, months or years later.
3. Organize work as tasks with states, and *see* those states at a glance (kanban).
4. See time-bound work on a calendar.
5. Eventually, sketch freely on a spatial canvas.

It is a **lab notebook that can also be a task board**. Both halves live in one corpus, so a task can link to the idea that spawned it and the note that recorded the result.

### Success criterion

You use it daily for three months without wanting to leave. If you abandon it, the failure was almost certainly *capture friction* or *retrieval failure* — optimize against those two above all else.

---

## 2. Non-goals

Explicit, so scope cannot drift silently.

- No multi-user, no collaboration, no sharing, no permissions.
- No cloud service. No hosted anything. No accounts.
- No telemetry, ever. Not even opt-in.
- No plugin API in v1. Theming (CSS) is the only extension point.
- No native mobile app. Mobile = responsive web page.
- No real-time sync engine. External tools (git, Syncthing) handle sync if needed.
- No AI features in v1. Revisit once the core is boring and stable.

---

## 3. The architectural thesis

> **One data model. Many views.**

Everything in the corpus is an **object** with **typed properties**. Every view — kanban, calendar, timeline, list, backlink panel — is a **saved query plus a renderer**.

There is no "kanban feature." There is a query (`status = doing`) and a renderer that draws columns. Adding a view later is a weekend, not a rewrite. This is the generalization Logseq spent years refactoring *toward*; start there instead.

**Corollary:** any feature request that cannot be expressed as "a query plus a renderer" is a red flag. Interrogate it before building it.

---

## 4. Data model

### 4.1 The object

One Markdown file = one object. YAML frontmatter carries typed properties; the body carries content.

```markdown
---
id: 01J8ZQK4XN9P               # stable, survives rename
type: task                     # note | task | idea | log
title: Fix advantage estimator
status: doing                  # enum — EXCLUSIVE
due: 2026-07-20                # date | null
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T11:03:00Z
tags: [meta-rl, exploration]   # multi-valued, NOT status
links: [01J8ZQ...]             # explicit object refs
---

The GAE lambda seems to interact badly with the inner-loop
adaptation. See $\lambda = 0.95$ runs from Tuesday.
```

### 4.2 Properties vs tags — do not conflate

| Concept | Semantics | Example | Used for |
|---|---|---|---|
| **Property** | Single-valued, typed, often enum | `status: doing` | Kanban columns, calendar placement, sorting |
| **Tag** | Multi-valued, unordered, freeform | `tags: [meta-rl]` | Topical filtering, cross-cutting retrieval |

A kanban column is an exclusive state. A tag is not. Modeling status as a tag permits a task to sit in three columns simultaneously and forces defensive hacks forever. **Status is a property.**

### 4.3 Identity

Stable ID in frontmatter, *not* filename. Filenames may change; links must not break. Human-readable filename is a display convenience, the ID is the truth.

**Open:** ULID vs UUIDv7 vs short nanoid. ULID is time-sortable and human-tolerable — leaning that way.

### 4.4 Types

`note`, `task`, `idea`, `log`. Deliberately few. A `task` is just a `note` with `status` and optionally `due`. Resist adding types; add properties instead.

---

## 5. Functional requirements

Prioritized. Ship in this order. Do not start a tier until the previous one is boring.

### Tier 0 — Walking skeleton (must exist before anything else)

- [ ] Capture a note in under 2 seconds from an empty screen.
- [ ] Notes persist to disk as Markdown + frontmatter.
- [ ] Reload the app; notes are still there.
- [ ] Full-text search returns results.
- [ ] Every object shows created + updated timestamps (relative display, absolute on hover).

If Tier 0 is unpleasant to use, nothing built on top of it will save it.

### Tier 1 — The core loop

- [ ] Typed properties editable in the UI (not just raw YAML).
- [ ] Query layer: filter by property, tag, type, date range; sort; group.
- [ ] **Kanban view** — columns from `status`, drag to change status, writes back to frontmatter.
- [ ] **Calendar view** — objects placed by `due` (and/or `created`).
- [ ] Tag filtering across all views.
- [ ] Rendering: code blocks with syntax highlight, LaTeX math, images, links.

### Tier 2 — Research affordances

- [ ] `[[wiki links]]` between objects, with backlinks panel ("linked mentions").
- [ ] Saved queries — user-defined views, not just the three built-ins.
- [ ] Asset handling (see §7).
- [ ] Themeable via CSS.

### Tier 3 — Optional / earn their place

- [ ] Phone access (see §8).
- [ ] Whiteboard / canvas (see §6 — read the warning first).
- [ ] Semantic search over embeddings.
- [ ] Graph view.

---

## 6. The whiteboard warning

A whiteboard is **not a view**. Kanban, calendar and timeline are *derived* from objects via query. A canvas is a **spatial document** with its own coordinates, z-order, freeform shapes, and no query semantics. It cannot be derived, and it does not fit the "one model, many views" thesis.

**Logseq built whiteboards. They became a maintenance sink and never integrated cleanly.** This is the single most likely way this project dies.

**Decision:** whiteboard is a **separate document type** that can *embed* objects by ID. It is quarantined behind a hard interface boundary. It is built **last**, on `tldraw` (mature, MIT-ish, do the license check). If it starts leaking concepts into the core data model, stop and cut it.

---

## 7. Assets

Content-addressed storage — the Git/Docker trick.

- Hash file contents (SHA-256), store at `assets/ab/cd/abcd….ext`.
- Reference by hash. Automatic dedup: same image pasted twice = stored once.
- URLs never change for the same content → `Cache-Control: immutable` works perfectly.
- Served directly by the local server.

Assets, not notes, are what will actually grow large. Ten thousand notes is ~20MB of text. A few hundred figures and PDFs will dwarf that.

---

## 8. Access and privacy

### 8.1 Threat model — fill this in before writing auth code

Complete the sentence: *"I do not want ______ to be able to read my notes."*

| Threat | Mitigation | Who owns it |
|---|---|---|
| Cloud vendor reads my data | Tool is local. Solved by construction. | Architecture |
| Someone steals my laptop | LUKS full-disk encryption | **OS, not the app** |
| Someone on my LAN | Auth + TLS | App + Tailscale |
| A CDN sees what I load | Vendor all assets locally — no CDN calls, ever | Discipline |

**Do not build encryption into the app.** The OS does it better.

### 8.2 The phone-access tension — state it plainly

Local-first and remote-phone-access are in **direct tension**. Enabling phone access means running an HTTP server that exposes your complete private corpus on a network. That single decision is a larger privacy risk than everything else in this document combined.

Therefore:

- **Build auth from day one**, even on localhost. Password + session cookie, rate-limited. Retrofitting auth is painful and the "it's only localhost" assumption breaks the instant you want your phone.
- **LAN access** (bind `0.0.0.0`, phone hits `laptop.local:PORT`) covers ~90% of real need. Zero infrastructure.
- **Remote access** → Tailscale. It handles NAT traversal, TLS and identity. Do not hand-roll port forwarding, DDNS, and certs.
- Either do this properly or **don't ship phone access**. Half-doing it is the worst outcome.

---

## 9. Non-functional requirements — numeric, therefore falsifiable

"Lightweight" is a vibe until it has numbers. These are tests you can fail.

| Metric | Budget | Rationale |
|---|---|---|
| Cold start to usable | < 500 ms | The thing Logseq actually gets wrong |
| Search results rendered | < 100 ms @ 10k objects | Perceived as instant |
| Keystroke → paint | < 16 ms | No typing lag, ever |
| Idle memory (server) | < 100 MB | |
| Distributable binary | < 30 MB | |
| Frontend JS shipped | < 300 KB gzipped | Excluding lazy-loaded KaTeX/highlighter |
| Reindex from scratch | < 10 s @ 10k objects | Index must be cheap to throw away |

Add these to CI as assertions. A budget that isn't enforced is a wish.

### On scale — stop worrying about it

Ten years of heavy note-taking is ~10k objects, ~20MB of text. `ripgrep` handles that in ~30ms. **You do not have a scale problem and you will not get one.** The database exists for *queryability* (kanban/calendar are relational queries with filters and joins), not for volume. Do not let "what if it gets big" justify complexity.

---

## 10. Open decisions — the ADR queue

Each of these gets its own ADR before implementation. Listed in dependency order.

| # | Decision | Options | Notes |
|---|---|---|---|
| 1 | **Source of truth** | (a) Markdown files + rebuildable SQLite index (b) SQLite as truth + export-to-MD | **The biggest one.** (a) = portable, greppable, git-able, but you *will* fight the file watcher and index drift. (b) = atomic, no drift, no watcher, simpler code — but not greppable. Honest question: will you actually edit these in Vim and diff them in git? If no, (b) is stronger than orthodoxy admits. |
| 2 | **Server language** | Go / Rust / Node / Python | Affects *distribution* far more than performance. Go = single static binary, trivial `apt`-free install on Ubuntu. Leaning Go. |
| 3 | **Frontend approach** | Server-rendered + HTMX / Svelte / SolidJS | Kanban drag-and-drop and canvas push toward a real client framework. HTMX may not survive Tier 1. Reassess after the kanban spike. |
| 4 | **Editor** | Textarea / CodeMirror 6 / TipTap | CodeMirror 6 if notes stay Markdown-native. TipTap if you want WYSIWYG. |
| 5 | **Query language** | SQL directly / a small DSL over SQL | A DSL is nicer for saved queries but is a language you now maintain. |
| 6 | **ID scheme** | ULID / UUIDv7 / nanoid | Low stakes. Pick ULID and move on. |

---

## 11. Build plan — vertical slices, not horizontal layers

Never build "the storage layer, then the API, then the UI." Build **thin end-to-end slices** so integration problems surface immediately.

| Slice | Deliverable | Proves |
|---|---|---|
| **S0** | Empty page → type a note → saved to disk → reload → still there | The whole pipeline exists |
| **S1** | Search box → ripgrep or FTS → results with timestamps | Retrieval works |
| **S2** | Frontmatter properties editable; `status` settable | The typed-property model holds |
| **S3** | Kanban view: query `status`, drag between columns, writes back | **The thesis is validated or dead** |
| **S4** | Calendar view from `due` | A *second* view over the same query layer costs a weekend, not a rewrite. If it doesn't, §3 is wrong — stop and fix the model. |
| **S5** | Tags, filters, saved queries | |
| **S6** | Assets, math, code rendering, theming | |
| **S7** | Auth + LAN access | |
| **S8** | Whiteboard (quarantined) | |

**S4 is the checkpoint that matters.** If adding the calendar is easy, the architecture is right. If it's painful, the query layer isn't real and you're building features, not a system. Stop and fix it there — it only gets more expensive.

---

## 12. Before writing any code

1. **Run Memos and SilverBullet for two weeks each.** Cost: one evening of setup. Payoff: this document gets rewritten from *real friction* instead of speculation, which is worth more than any amount of a-priori design. SilverBullet in particular is close to this spec already — knowing precisely why it isn't enough is the strongest possible justification for building.
2. **Answer §8.1** (threat model — one sentence).
3. **Write ADR-1** (source of truth). Everything else depends on it.
4. **Cut three things from §5.** Simplicity is a budget. Right now nothing has been cut, so it has not been paid for.
