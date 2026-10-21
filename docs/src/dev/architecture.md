# Architecture

formicaria is a small Rust workspace behind a browser UI. The design is built
on three seams so that each layer can change without breaking the others.

## The three seams

```text
        Browser (Svelte 5)
  Board · Agenda/Calendar · Timeline · Activity · Discussions · Search · NotePanel
             │  ipc.ts:  POST /api/<cmd>  (prod)  ·  in-memory mock (dev/test)
             ▼
        fm-serve  ── transport shell: frames HTTP, nothing else. Also serves the
             │        UI, GET /api/blob/<ref> (streamed, Range) and POST /api/alive
             ▼
   fm_app::dispatch ── THE one command surface: takes the vault lock, runs the
             │           command, hands back Output::{Json,Bytes}
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
  `git`) are **shelled out to**, not linked — so their (often GPL) licenses
  never enter our binary and they can be upgraded independently. The one exception is
  libgit2, linked where there is no `git` program to call: the phone and Windows
  (`decisions.md#track-m`).

## The crates

| Crate      | Role                                                              |
|------------|------------------------------------------------------------------|
| `fm-model` | `Object`, `Kind`, `PropertyValue` — the data model (no I/O).      |
| `fm-query` | the query engine + `Store` contract tests (no I/O — seam 1).      |
| `fm-core`  | `FileStore`, `BlobStore`, ingest, verify, manifest, backup, git.  |
| `fm-app`   | **library**: `dispatch` (the one command surface + the `Host` trait), the command functions, `.view` execution, the vault registry (`vaults`), DTOs. |
| `fm-serve` | the HTTP **transport** over `fm_app::dispatch`; owns only framing. |
| `fm-cli`   | the `fm` command-line tool over the same core.                   |
| `fm-fetch` | downloads with a pinned hash: models, runtimes, reference packs.  |
| `fm-update`| finding, verifying and installing a newer version of the app.     |
| `agent-serve` | the binary of the assistant's resident process (package `fm-agent-serve`). Its code is Myrme's; this crate only gives it a binary in this workspace. |

**The assistant, Myrme, lives in its own repository**
([formicaria-org/myrme](https://github.com/formicaria-org/myrme)), and `fm-serve` links its
`fm-agent-run` crate by a pinned tag. There it is a **deterministic orchestrator** (`fm-agent`): a
fixed pipeline whose model touch-points and web search sit behind seams, so it is unit-tested with
fakes and needs no model and no network; the model never drives control flow, holds a tool, or
loops. All vault I/O goes through a running `fm-serve`, never a second `FileStore`, so `fm-serve`
stays the single writer.

The assistant is behind an optional `agent` feature, in `fm-serve` and in the phone's shell alike,
and a build without it contains no assistant at all — which is what an Android `--no-default-features` build ships.

## The command surface

The frontend never sends SQL or a query struct — it calls **named commands** with
simple args, and `fm-app` builds the query behind `dispatch`. Each command is a
plain function in `crates/fm-app/src/commands.rs` over the `Store` seam, so it is
unit-testable with `MemoryStore` and behaves identically against `FileStore`.

Adding a *frontend* therefore means writing a shell — parse a request into
`(cmd, args, body)`, call `dispatch`, encode the `Output` — not a second copy of the
dispatch table. `fm-cli` is what that costs when you don't: it re-implements the flows
against `fm-core` instead, which is the fork `dispatch` was extracted to stop repeating.

See [Commands](../reference/commands.md) for the full list, and
[How to add a feature](./adding-features.md) to extend it.
