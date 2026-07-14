# ADR-005: Interaction model, media ingest, and CI as enforcement

**Status:** Proposed
**Date:** 2026-07-14
**Depends on:** ADR-001 (one model, many views), ADR-003 (asset store), ADR-004 (asset pipeline)

---

## Context

Four ADRs designed a system with no idea what it *feels* like. Every abandoned PKM tool died of **capture friction** or **retrieval failure** — not of missing features. The interaction model is therefore not decoration on top of the architecture; it is the thing the architecture exists to serve.

---

## Part 1 — The interaction model

### Capture

> **There is no "new note" button. The capture box is always present and always focused.**

You land on the page and type. That is the entire interaction. Requirement §5 Tier 0 says *capture in under 2 seconds from an empty screen* — a button is already too slow, and a modal is much too slow.

- Global shortcut (`⌘/Ctrl + Space`) focuses capture from anywhere.
- `Esc` or blur commits. `⌘Enter` commits and clears.
- Each capture is **a new small file** (ADR-001). This is what makes file-level timestamps behave like entry-level timestamps without block IDs.

### Save

> **There is no save button, ever.**

| Trigger | Action |
|---|---|
| Typing stops, 500 ms | Atomic write to disk (temp file + rename) |
| Idle 30 s, or window blur | `git commit -m "auto: <title>"` |

Two debounces, not one. Disk writes must be fast enough to be invisible; commits are slower and can lag. This is Wikipedia's append-only revision model (ADR: research notes), running quietly, using a tool already installed.

### Views are not features

Stream, board, calendar, gallery are **four renderers over one query engine**:

| View | Query |
|---|---|
| Stream | all objects, sorted by `created` desc — the default home |
| Board | group by `status`, drag writes `status` back to frontmatter |
| Calendar | filter `due` within range |
| Gallery | filter `type = asset`, render thumbnails |

Adding a fifth view is a weekend. If it ever isn't, ADR-001 is wrong and everything stops. This is the S4 checkpoint made permanent.

### The note card

Timestamp (relative, absolute on hover), status pill, tags, `git` SHA for referenced code, asset chips. Every element on the card is **a property**, and every property is **queryable**. Nothing is displayed that cannot be filtered on.

---

## Part 2 — Media ingest: three doors, because the browser is the wrong one for big files

A pasted screenshot and a 2 GB recording are not the same problem.

### Door 1 — paste / drag (small: images, PDFs, decks)

```
browser POST (multipart, streamed)
  → server streams to temp file, hashing as it goes (blake3)
  → hash already exists?  → discard temp, return existing hash   ★ instant dedup
  → else                  → move temp → blobs/ab/cd/<hash>
  → sniff MIME (libmagic) · exiftool · extract text → FTS
  → queue thumbnail (async)
  → return {hash, mime, size, title}
  → editor inserts ![deck](asset:blake3-9f2a…) at cursor
```

The same figure pasted into five notes costs one blob. Dedup is not a feature here; it is a consequence of hashing.

### Door 2 — CLI (big: recordings, datasets)

```
fm add ~/talks/neurips.mp4
```

Pushing 2 GB through a browser upload to a server **on the same machine** is absurd. The CLI ingests server-side at disk speed, no HTTP involved.

**Use `cp --reflink=auto`.** On btrfs/XFS this is a copy-on-write clone — instant, and it consumes **zero additional bytes**. It falls back to a normal copy on other filesystems.

**Do not hardlink.** An in-place edit of the original file would mutate a blob that is supposed to be immutable. Reflink gives the speed of a hardlink with the independence of a copy.

### Door 3 — watched inbox folder (bulk)

Drop files into `$INBOX`; they are ingested and the folder empties. This is how 200 slide decks get imported without 200 clicks.

### Opening

`open` on an asset → `xdg-open` locally, plain download remotely. **No viewer was written** (ADR-004).

---

## Part 3 — Git hygiene

### `.gitignore` is not enforcement

Assets live **outside the notes repo** entirely (ADR-003), so in the normal case there is nothing to ignore. The hook below is the safety net for the day a video gets dragged into the wrong window.

### The pre-commit hook is enforcement

```sh
#!/bin/sh
# .githooks/pre-commit — the notes repo stays text-only
limit=$((5*1024*1024))
for f in $(git diff --cached --name-only --diff-filter=AM); do
  [ -f "$f" ] || continue
  if [ "$(wc -c < "$f")" -gt "$limit" ]; then
    echo "reject: $f exceeds 5MB — assets belong in the blob store, not the repo"
    exit 1
  fi
done
```

`git config core.hooksPath .githooks` so it is versioned with the project rather than living in `.git/`.

This is what turns ADR-003's "the notes repo stays pure text" from an intention into a **constraint**. It is the single cheapest guarantee in the project.

### Remote

**Local only, until explicitly decided otherwise.** The repo is initialised locally with no remote configured. Nothing is published anywhere. A CI config file living in the tree is inert until it is pushed to somewhere of your choosing — writing it now costs nothing and commits to nothing.

---

## Part 4 — CI is not for testing. It is for defending the architecture.

### The structural test

> **The query engine passes its entire test suite against `MemoryStore`, with zero filesystem access.**

This test does not check correctness. It checks that **the seam is real** (ADR-001).

The day you — tired, in eight months, in a hurry — reach into the query engine and import a path helper because it is faster, this test fails. That is the architecture defending itself against its own author. It is the difference between a storage swap and Logseq's three-year rewrite.

**If this test ever needs a filesystem to pass, the seam is already gone.** Do not fix the test. Fix the code.

### The ordinary tests

| Test | Guards |
|---|---|
| `MemoryStore` query suite, no fs | **The seam** (structural — see above) |
| `FileStore` round-trip | Storage correctness |
| Reindex idempotence: index → wipe → reindex → identical results | The index is genuinely disposable |
| `verify` on a fixture corpus with deliberate dangling refs | The verifier actually catches things |
| Perf budgets from REQUIREMENTS §9 as **assertions** | "Lightweight" stays falsifiable |
| Pre-commit hook, against an oversized fixture | The hook itself works |

Perf budgets belong in CI **now**, while the corpus is small and they are trivially easy to pass. A budget added later, once it is already being violated, gets deleted rather than fixed.

---

## Consequences

**Easier**
- Capture is fast enough to actually use, which is the only thing that determines whether the project succeeds.
- Big media ingests at disk speed and often at zero storage cost (reflink).
- The architecture is enforced mechanically rather than by memory and discipline.

**Harder**
- Three ingest doors means three code paths to keep consistent. They must converge on the *same* ingest function; only the transport differs.
- Auto-commit produces many small commits. Acceptable — git handles this fine, and squashing is always available later. Do not build a "commit management" feature.
- The watched folder is a file watcher, with all the inotify caveats from ADR-001. Keep it dumb: poll if inotify misbehaves.

---

## Action items

1. [ ] Capture box: always present, always focused, `⌘Space` from anywhere. Build this **first** — it is Tier 0.
2. [ ] Two-tier debounce: 500 ms → disk, 30 s / blur → commit. No save button anywhere in the UI.
3. [ ] One ingest function. Three transports (HTTP, CLI, inbox) call into it.
4. [ ] `cp --reflink=auto` for CLI ingest. Never hardlink.
5. [ ] `.githooks/pre-commit` size guard + `core.hooksPath`. Test the hook in CI.
6. [ ] **`MemoryStore` test with zero filesystem access — before `FileStore` exists.**
7. [ ] Perf assertions in CI from day one.
