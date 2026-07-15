# Architecture

formicarium is a small Rust workspace behind a browser UI. The design is built
on three seams so that each layer can change without breaking the others.

## The three seams

```text
        Browser (Svelte 5)
   Board · Agenda/Calendar · Timeline · Gallery · Search · NotePanel
             │  ipc.ts:  POST /api/<cmd>  (prod)  ·  in-memory mock (dev/test)
             ▼
        fm-serve  ── std-only HTTP server; locks the store, calls a command fn
             │
   ┌─────────┼──────────────── seam 1 ────────────────────────────┐
   │   fm-query  the query engine — CANNOT touch the filesystem    │
   └─────────┼──────────────────────────────────────────────────┘
             ▼  seam 2 (Store)                    seam 3 (OS)
        fm-core:  FileStore = Markdown + SQLite FTS5     → pdftotext / vipsthumbnail
                  BlobStore = content-addressed media    → restic / git (subprocess)
```

- **Seam 1 — the query engine.** `fm-query` compiles queries against objects and
  never performs I/O. This is a **compile-time guarantee**: the crate has no
  dependency on `rusqlite`, `std::fs`, or path types, and CI greps to keep it
  that way. It is the whole insurance policy — the durable core cannot rot into a
  filesystem tangle.
- **Seam 2 — the store.** `fm-core` implements the `Store` trait twice:
  `MemoryStore` (zero-I/O, used to verify the query contract) and `FileStore`
  (Markdown files + a disposable SQLite FTS5 index). Swapping storage is a
  backend change, not a rewrite.
- **Seam 3 — the OS.** Heavy tools (`pdftotext`, `vipsthumbnail`, `restic`,
  `git`) are **shelled out to**, never linked — so their (often GPL) licenses
  never enter our binary and they can be upgraded independently.

## The crates

| Crate      | Role                                                              |
|------------|------------------------------------------------------------------|
| `fm-model` | `Object`, `Kind`, `PropertyValue` — the data model (no I/O).      |
| `fm-query` | the query engine + `Store` contract tests (no I/O — seam 1).      |
| `fm-core`  | `FileStore`, `BlobStore`, ingest, verify, manifest, backup, git.  |
| `fm-app`   | **library**: the command functions (`commands`) + DTOs (`dto`).  |
| `fm-serve` | the HTTP server that fronts `fm-app`'s commands to the browser.   |
| `fm-cli`   | the `fm` command-line tool over the same core.                   |

## The command surface

The frontend never sends SQL or a query struct — it calls **named commands** with
simple args, and `fm-serve` builds the query server-side. Each command is a
plain function in `crates/fm-app/src/commands.rs` over the `Store` seam, so it is
unit-testable with `MemoryStore` and behaves identically against `FileStore`.

See [Commands](../reference/commands.md) for the full list, and
[How to add a feature](./adding-features.md) to extend it.
