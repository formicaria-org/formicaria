# ADR-004: Asset pipeline — extract, delegate, render only when forced

**Status:** Proposed — narrows the rendition scope of ADR-003
**Date:** 2026-07-14
**Depends on:** ADR-003 (asset store)

---

## Context

ADR-003 specified a rendition pipeline (ffmpeg, LibreOffice, pdftoppm) as though every asset needed transcoding. That over-scoped the problem, and it was flagged as the largest single scope addition in the project.

Two facts shrink it substantially:

1. **Browsers already render most of the corpus natively.**
2. **On Ubuntu, `xdg-open` is a perfect viewer for everything else — provided the browser and the server are on the same machine.**

The rendition pipeline is therefore not a baseline requirement. It is **the price of remote access.**

---

## Decision

> **Extract text from everything at ingest. Never render an asset — delegate to whichever OS is holding the browser.**
>
> **The only rendition ever built is a thumbnail.**

### Two tiers of viewing — there is no third

| Tier | Mechanism | Cost | Formats |
|---|---|---|---|
| **1. Browser-native** | `<img>`, `<embed>`, `<video>`, `<audio>` | **Zero** | jpg, png, webp, gif, svg, **pdf**, mp4/h264, webm, mp3, wav, text, code, csv, md |
| **2. OS handoff** | download the original, let the OS open it | **Zero** | Everything else — pptx, docx, xlsx, mkv, psd, ipynb |

**Tier 2 works on every platform, which is the finding that collapses this ADR.**

- **Ubuntu** → `xdg-open` → LibreOffice, VLC, GIMP, Evince.
- **iOS** → Quick Look renders pptx, docx, xlsx, pdf natively.
- **Android** → the same, via its own document viewers.

Every operating system already ships a better file viewer than we could build. **So we build none.**

### The rendition subsystem is deleted, not deferred

The earlier draft of this ADR assumed remote access forced a transcoding pipeline, and priced phone support at "ffmpeg + LibreOffice + a job queue." **That was wrong.** A phone is not a thin client with no viewer — it is a full OS with a viewer of its own. The correct move on mobile is identical to the correct move on desktop: hand it the original bytes and get out of the way.

Therefore:

| Component | Status |
|---|---|
| LibreOffice → PDF conversion | **Deleted.** The phone opens the pptx. |
| ffmpeg video transcoding / proxies | **Deleted.** mp4 plays everywhere; exotic codecs download to VLC. |
| `pdftoppm` page rendering | **Deleted.** Every browser has a PDF viewer. |
| Rendition job queue | **Deleted.** Nothing left to queue but thumbnails. |
| **Thumbnails** | **Kept** — `libvips` for images, `ffmpeg` poster frames for video. |

**Thumbnails are not a phone cost.** They are required for the desktop gallery regardless (a grid of 500 images cannot lazy-load full-size files). They were never remote-access overhead.

### Consequence: phone access is now cheap

It was the most expensive feature in the project. It is now among the cheapest:

| Cost | Real? |
|---|---|
| Auth | Already mandated by ADR-001 §8, remote or not |
| Responsive CSS | Effectively free |
| Tailscale | Configuration, not code |
| Range-request serving | Already required for desktop video |
| Rendition pipeline | **Gone** |

**The one case that would bring ffmpeg back:** streaming a 2 GB recorded talk to a phone over cellular. Downloading the original is genuinely bad there, and a low-res proxy would be the right answer. That is a specific, opt-in, later feature — flag it, do not build it.

---

## The ingest pipeline

Runs once per blob, on write. Steps 1–4 are cheap and synchronous-ish; step 5 is async.

```
bytes
  ↓ 1. hash (blake3)                → blob id, dedup
  ↓ 2. sniff MIME (libmagic)        → NEVER trust the file extension
  ↓ 3. extract metadata (exiftool)  → dimensions, duration, pages, dates → properties
  ↓ 4. extract text                 → FTS index          ★ the important one
  ↓ 5. renditions (async)           → thumbnails; web proxies only if remote
```

### Step 4 is the highest-value step in the whole system

Filtering assets by tag and date is trivial — assets are objects with properties, so the existing query engine (ADR-001 §3) already covers it, and a gallery is just another view. That comes free.

What does **not** come free is *"which of my 300 PDFs mentions trust-region clipping?"*

| Source | Tool | Result |
|---|---|---|
| PDF (text layer) | `pdftotext` | searchable |
| PDF (scanned) | `tesseract` | OCR → searchable |
| docx / pptx | `pandoc` | searchable |
| plain text, code, csv | direct | searchable |

Extracted text goes into **the same FTS5 index as note bodies**. One search box, one query, covering everything.

This turns the tool from *"search my notes"* into **"search everything I have ever read or written."** For a researcher, that is plausibly the single highest-value feature in the system — and it costs one subprocess call at ingest.

---

## The Ubuntu toolchain

All `apt install`. All mature. Nothing to write.

| Job | Tool | Notes |
|---|---|---|
| MIME detection | `libmagic` / `file` | Sniff bytes, never trust extensions |
| PDF → text | `pdftotext` (poppler-utils) | Makes PDFs searchable |
| Office → text | `pandoc` | docx/pptx → text or markdown |
| OCR | `tesseract` | Scanned PDFs, screenshots of slides |
| Metadata | `exiftool` | → typed properties |
| Image thumbnails | `libvips` | Much faster and lighter than ImageMagick |
| Video poster frames | `ffmpeg` | Thumbnails only. **No transcoding.** |
| Open natively | `xdg-open` (Ubuntu) / OS download handler (iOS, Android) | **Build no viewers, on any platform.** |
| Full-text search | SQLite **FTS5** | Notes + extracted asset text, one index |
| Grep fallback | `ripgrep` | Index-free search when the index is rebuilding |

**Every one of these is a subprocess call.** No bindings, no libraries, no version hell. If a tool is missing, the feature degrades (no OCR) rather than the system failing.

**Not in this table, deliberately:** `libreoffice`, `pdftoppm`, and ffmpeg *transcoding*. Every OS ships a viewer. We are not one.

---

## Consequences

**Easier**
- Viewing is free on every platform. Browser renders what it can; the OS opens the rest.
- Phone support costs auth + responsive CSS, not a transcoding pipeline.
- Search reaches inside documents, not just around them.
- Every processing dependency is optional and degrades gracefully.
- The gallery is not a feature — it is a view over the query engine. It costs a weekend, per the ADR-001 thesis.

**Harder**
- Ingest is a multi-step pipeline that can partially fail. Each step must be independently retryable, and a failed OCR must not block the blob from being stored.
- OCR is slow (seconds per page). Strictly async, strictly optional.
- Downloading a 2 GB original to a phone over cellular is genuinely bad UX. Accepted. See below.

**Revisit when**
- Streaming large recordings to a phone over cellular becomes a real need → and *only then* does a low-res `ffmpeg` proxy earn its place.
- The corpus contains enough scanned material that OCR becomes a bottleneck rather than a background nicety.

---

## Action items

1. [ ] Ingest pipeline: hash → sniff → metadata → extract text → (async) thumbnail.
2. [ ] Extracted asset text goes into the **same** FTS5 index as notes. One search, not two.
3. [ ] OS handoff for tier 2: `xdg-open` on Ubuntu, plain download elsewhere. Same code path, different endpoint.
4. [ ] Thumbnails via `libvips` / `ffmpeg` poster frames. **This is the only rendition that exists.**
5. [ ] **Do not install LibreOffice or write a transcoder.** If you find yourself reaching for either, re-read this ADR — the OS on the other end already has a viewer.
