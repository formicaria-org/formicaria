# ADR-003: The asset store is a media subsystem, not an attachments folder

**Status:** Proposed — supersedes the asset section of ADR-002
**Date:** 2026-07-14
**Depends on:** ADR-001 (atom = file), ADR-002 (reference, don't ingest)

---

## Context

ADR-002 treated assets as *attachments*: hash them, drop them in a directory, reference by hash. That is correct for a few hundred figures and **wrong for a decade of images, videos, and presentations**.

The corpus described is a **media library**: figures, papers, slide decks, recorded talks, screen captures. Realistic ten-year size: **100 GB – 1 TB**. Individual objects range from a 200 KB plot to a 4 GB recording.

This is a subsystem with its own lifecycle, not a folder.

**It is also not a distributed systems problem.** 1 TB is one disk. No object-store cluster, no S3, no CDN, no MinIO. Correct sizing is as important as correct architecture — overbuilding here is the fastest route to violating the lightweight requirement.

---

## Decision

> **A dedicated, content-addressed blob store on its own volume, with an asynchronous derived-rendition cache, served over HTTP with range support, garbage-collected, and backed up by a CDC-based tool.**

The notes corpus stays **pure text**. Assets never enter the notes git repo. This is now a load-bearing constraint, not a preference — it is what keeps the notes repo tiny, fast, cloneable, and cheap to version forever.

---

## The five parts

### 1. Blob store — immutable, content-addressed, dedicated volume

```
$ASSET_ROOT/blobs/<first2>/<next2>/<full-hash>
```

- `blake3` or `sha256`. The hash is the **only** identifier. No filenames, no paths, no IDs.
- Immutable. A blob is never modified, only added or garbage-collected.
- Free file-level dedup: the same figure referenced from 8 notes is one blob.
- **`ASSET_ROOT` is configurable and lives on its own volume.** Notes on the SSD; assets on the big disk or the NAS. This is the "dedicated filesystem" requirement, and it is a config line, not an architecture.
- Original bytes are never transcoded. What went in comes out.

### 2. Rendition cache — derived, async, disposable

A 2 GB video cannot be shown inline. A `.pptx` cannot be shown at all. Renditions are the subsystem I originally omitted.

```
$ASSET_ROOT/derived/<source-hash>/<rendition-spec>
```

| Source | Renditions | Tool |
|---|---|---|
| image | thumbnail, web-size | vips / imagemagick |
| video | poster frame, low-res scrub proxy | ffmpeg |
| pdf | first-page thumbnail | pdftoppm |
| pptx / docx / xlsx | → pdf → thumbnail | headless LibreOffice |

**This reverses the "no previewers" rule in ADR-002.** That rule was right when `.pptx` was an attachment; it is wrong now that it is a stored first-class object. The dependency is safe *because renditions are a disposable cache* — exactly the same status as the search index. Delete the whole `derived/` tree and it regenerates.

**Renditions are generated asynchronously.** Ingest returns the instant the blob is written and hashed. The thumbnail appears later; the UI shows a placeholder until it does. **Capture must never block on ffmpeg.** (This is Wikipedia's job queue, at one-user scale — a table and a worker, not Kafka.)

### 3. Serving — HTTP with range requests

- **`Range:` / `206 Partial Content` is mandatory.** Without it, a browser cannot seek in a video, and video is unusable. Small omission, total failure.
- Content-addressed URLs never change for the same bytes → `Cache-Control: public, max-age=31536000, immutable`. The browser caches permanently and correctly.
- Serve blobs and renditions through the same auth boundary as notes. An unauthenticated `/blobs/<hash>` endpoint is an unauthenticated dump of your entire private corpus.

### 4. Garbage collection — the thing LFS gets wrong

Git LFS keeps every version of every tracked file forever, and reclaiming server-side storage is painful. A content-addressed store plus GC avoids this entirely:

- Mark: walk all notes, collect every referenced hash.
- Sweep: any blob not reachable from a note is a candidate.
- **Never delete automatically.** Report, quarantine, require an explicit `--confirm`. A GC bug that eats your talks is unrecoverable.

Without GC you accumulate orphans forever. With unsafe GC you lose data. Build it carefully, run it rarely.

### 5. Durability and dedup — CDC belongs *here*, not in the serving layer

**This is the correction to "you don't need content-defined chunking."**

CDC (Xet, restic, borg) deduplicates at ~64 KB chunk boundaries set by a rolling hash, so editing part of a large file only rewrites the affected chunks. Real and valuable. But chunked stores are **bad at fast random access**, and the serving layer's entire job is fast random access.

So split the concerns:

| Layer | Needs | Solution |
|---|---|---|
| **Serving** | instant `open()`, byte-range seeks | whole files on disk, content-addressed |
| **Durability** | dedup, compression, encryption, integrity, remote, pruning | **restic** or **borg** over `$ASSET_ROOT` |

`restic` *is* a content-defined-chunking, content-addressed, encrypted, deduplicating, integrity-checked store with pluggable remotes and `forget --prune`. It is a mature Go binary. **You get every property Xet advertises, locally, for free, by running one backup command.** Write none of it.

- `restic backup $ASSET_ROOT` — dedup + encrypt + snapshot
- `restic check` — integrity verification
- `restic forget --prune` — actual reclamation

---

## What was considered and rejected

| Option | Why not |
|---|---|
| **Git LFS** | Needs a server; keeps every version forever; server-side pruning is hard; pain is CI/team-shaped, which does not apply. The thing everyone is migrating away from. |
| **files.link / managed CDN** | Cloud service with an API key. Violates requirement #1 (local, private). The blog recommending it is the vendor's own. |
| **Hugging Face Xet storage** | The *protocol* is open and the CDC algorithm is worth learning from. The *storage* is HF's hosted S3 service. Take the algorithm, not the vendor. |
| **git-annex** | Genuinely capable and the closest fit — location-aware, many backends, offline. But it is a second content model to learn and reason about, and `restic` covers durability with less conceptual load. Reconsider if multi-machine asset placement becomes a real need. |
| **DVC** | Right tool, wrong repo. Belongs in the Meta-RL repo for datasets/checkpoints/pipelines. The notes tool only records `repo + commit + dvc path`. |
| **MinIO / S3 / object cluster** | Solving a 100 TB problem. You have a 1 TB problem. |

---

## Consequences

**Easier**
- Assets scale to a career's worth of media without touching the notes repo.
- Videos and decks are genuinely usable (thumbnails, previews, seeking) rather than opaque links.
- Dedup, encryption, integrity, and remote backup arrive free via restic.
- The store is relocatable and verifiable: the hash is the truth, so the volume can move disks.

**Harder**
- The rendition pipeline is a real subsystem: a job queue, worker, and three external binaries (ffmpeg, LibreOffice, pdftoppm). This is the single largest addition to scope in the project so far. **Acknowledge it as such.**
- GC must exist and must be conservative.
- Range-request serving must be correct, or video silently breaks.

**Revisit when**
- Assets exceed ~2 TB, or must live on more than one machine → look hard at git-annex.
- On-disk revisions of large files become common → serving-layer CDC starts to pay.

---

## Action items

1. [ ] `ASSET_ROOT` config, separate volume, **outside the notes git repo**. Enforce in code.
2. [ ] Blob store: `put(bytes) -> hash`, `get(hash)`, `stat(hash)`, `gc(reachable)`. Same seam discipline as ADR-001's `Store`.
3. [ ] Async rendition worker. Ingest must return before rendering starts.
4. [ ] HTTP range support, verified with a 2 GB video, before declaring media "done".
5. [ ] `restic` backup of `$ASSET_ROOT` on a schedule. This is the durability story — not a nice-to-have.
6. [ ] Extend `verify` (ADR-002) to check every referenced hash exists as a blob.
