# ADR-002: Artifacts — reference, don't ingest

**Status:** Proposed
**Date:** 2026-07-14
**Depends on:** ADR-001 (atom = file)

---

## Context

The system must help track not only ideas but the *things* research produces: code, algorithms, presentations, figures, datasets, model outputs. Many formats.

The tempting move is to make the notes system **store** these. That move rebuilds a file manager, an asset manager, and version control — all badly — and destroys the lightweight requirement in a single decision.

## Decision

> **A note *references* an artifact. It never contains one.**
>
> The system is a **catalog over artifacts**, not a container for them.

The reason is not only simplicity. Six months later, the thing you need is not a copy of a file. It is: *"find the note explaining why I ran this, and get me back to the exact code and data."* That is a **pointer with provenance**. A git commit SHA is a stronger, more permanent, more verifiable pointer than any copy could be.

---

## Three tiers of pointer

| Thing | Pointer | Rots? | Why |
|---|---|---|---|
| **Code, experiments** | `repo + commit SHA + path` | **No** — immutable by construction | The repo already has history, diffs, branches. Copying a `.py` into the notes dir creates a second, worse, drifting copy. A SHA resolves to the exact state of the *whole project*, not one orphaned file. |
| **Figures, slides, PDFs, exports** | `sha256` into a content-addressed blob store | **No** — the hash *is* the content | Immutable, auto-deduplicated (same figure pasted twice = stored once), permanently cacheable (`Cache-Control: immutable` works, because the URL never changes for the same bytes). |
| **Bulk data** (checkpoints, datasets, run dirs) | filesystem path + description | **Yes** | A 2GB checkpoint does not belong in the notes directory. Record where it is and what it was. This is the only tier that can break — which is exactly what `verify` is for. |

### Frontmatter shape

```yaml
---
id: 01J8ZQK4XN9P
type: log
title: GAE lambda sweep, inner-loop instability
created: 2026-07-14T09:12:00Z
tags: [meta-rl, exploration]

code:
  - repo: meta-rl
    commit: a3f9c2e
    path: algos/gae.py
assets:
  - sha256:9f2a...   # loss curves
  - sha256:c41b...   # slide from group meeting
data:
  - path: /data/runs/2026-07-14-sweep/
    note: 12 seeds, lambda in [0.9, 0.99]
---
```

Everything queryable is a property. Everything heavy is a pointer.

---

## Consequences

**Easier**
- The notes system never has to understand `.pptx`, `.ipynb`, `.pt`, or any other format. It stores a hash or a path.
- Code stays in the repo where it belongs, with its own history and its own privacy boundary.
- Notes stay tiny. The corpus stays greppable, git-able, and fast to reindex.
- Provenance is exact and verifiable, not approximate.

**Harder**
- Tier-3 pointers rot if you move things. Accepted, mitigated by `verify`.
- Requires a small amount of discipline at capture time (commit before you reference).

**Explicit non-goal: previewers.**
The browser natively renders PDF, images, plain text, and code. It does **not** render `.pptx` / `.docx` / `.xlsx`. Making it do so means shelling out to headless LibreOffice — a heavy dependency that costs more than it returns. For those formats: show the metadata, link out, let the OS open it. Refusing this saves an entire subsystem.

---

## The `verify` command

The counterpart to "don't ingest" is "check the pointers." Same idea as `git fsck`.

Walks the corpus and reports:
- git refs that no longer resolve (commit gone, repo moved)
- blob hashes missing from the asset store
- data paths that no longer exist
- wikilinks pointing at deleted objects
- frontmatter that fails to parse

Run it on a schedule. **Cheap sanity check that catches drift before your backups make the drift permanent.** Never auto-delete, never auto-fix — report only.

---

## Action items

1. [ ] Define the `Store` interface (ADR-001) so that artifact refs are *just properties* — the store must not special-case them.
2. [ ] Content-addressed blob store: `assets/<first2>/<next2>/<full-hash>.<ext>`.
3. [ ] `verify` before S6. It is not a nice-to-have; it is what makes reference-not-ingest safe.
4. [ ] Resist the first request to "just store the file directly." That is the whole ADR.
