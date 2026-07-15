# formicarium — master plan

**The single, self-contained specification for formicarium. Drafted 2026-07-14; revised 2026-07-15 (browser-first: the native Tauri window was removed and the app now runs in the browser via a local `fm-serve`).** Earlier design docs (BRIEF, PLAN, SPEC, REQUIREMENTS, ADR-001…007) are folded in here and removed — everything needed to implement lives in this one file. The key rationale and prior-art citations that justified each decision are preserved in the **Evidence & prior art** appendix at the end.

---

## Context — why this document exists

The user (a researcher) wants one local place to keep notes, ideas, meetings, tasks, and the media/artifacts that go with them — with nice-looking inline math and media, minimal footprint, extensibility, and a 10-year lifespan. An earlier design set (11 docs) reasoned carefully but left forks open and made two questionable calls (plain-textarea editor as the *whole* story; a Go binary justified on the wrong grounds). This plan resolves every fork against *current (mid-2026) evidence*: a critical read of how competitor tools lived and died, cross-domain solutions to the hard sub-problems, and a tool-by-tool stack justification. It is meant to be handed to implementation.

**Resolved decisions:** Build it (rich reading, simple writing) · **Rust core + a browser UI served by a local `fm-serve`** (the native Tauri v2 window was built and then *removed* — see the reversal table) · **Markdown is the source of truth**; v1 editor = plain-textarea edit + a rendered read view; **CodeMirror 6 live-preview deferred to v2** · files-as-truth, disposable per-machine SQLite index · external edits allowed (poll, never inotify) · agenda-first deadlines with *derived* urgency (no priority field) · **single-user, local-only** (a localhost server, no auth/cloud) · extensibility via 3 layers, **no plugin API**.

### Key decisions that reversed earlier drafts

An adversarial review of the earlier drafts drove the decisions below. Stated plainly so the record is honest about what changed and why:

| Earlier draft | This plan | Reason |
|---|---|---|
| Go + `modernc.org/sqlite` | **Overridden → Rust + Tauri v2** | Go was defended on distribution (weak: one owned machine). The real axis is 10-year dependency-rot resistance *and* a native-shell ecosystem (real global hotkey, OS-webview, asset protocol). Rust/Tauri wins both; `rusqlite` bundled gives static SQLite+FTS5. |
| Plain textarea, no build step | **Adopted, partially** | v1 ships textarea capture + a rendered read view — CM6's decoration layer (the single biggest build risk) moves to v2. The "no npm/no bundler" corollary does **not** apply: on Tauri+Svelte we have a bundler regardless, so the read renderer and IPC are a normal bundled TS app. |
| File watcher may be unneeded | **Adopted (the "yes" branch)** | External writers exist (Vim, `git pull`, Syncthing) → detect via **mtime polling on a timer**, never inotify. Reindex-on-startup + poll. |
| blake3 hashing | **sha256** | Verifiable with `sha256sum` forever; universality beats ~1.7 s. |
| 4-weekend kill criterion | **8 weekends** | The researcher-not-web-dev correction stands. |
| `Store.query()` left undesigned | **Designed** | Typed `Query`/`Filter`/`Predicate` struct; full-text is just another predicate. See Backend architecture. |
| Rust + Tauri v2 native window | **Overridden → browser UI via local `fm-serve`** (2026-07-15) | The WebKitGTK window never painted reliably (blank/gray; Risk #1). Rather than keep fighting it, the window was removed and the app now runs in the user's real browser, served by a tiny std-only HTTP server (`fm-serve`) fronting the same command functions. The webview's advantages (global hotkey, asset protocol) went with it; none were load-bearing. `fm-app` is now a command *library*. |

## What it is

A local, browser-based tool (a localhost server + your browser): capture ideas fast, retrieve them years later, see the closest deadline, and keep every figure/deck/paper/recording attached to the thought that produced it. Binaries: `fm` (CLI) and `fm-serve` (the local server); the UI is served to your browser.

**The name.** A formicarium is the apparatus you build so that an emergent structure becomes observable — you provide the medium and the glass, the colony digs the tunnels. Notes accumulate; views reveal the structure that formed. Don't impose the taxonomy — build the glass. The binary is `fm` (not `formica` — that's a countertop; and never `ant` — Apache Ant has owned that command for twenty-five years).

**Success:** used daily for three months without wanting to leave. **It can fail only two ways: capture is slow, or retrieval fails.** Everything else is decoration — optimize against those two above all.

---

## Critical assessment — every recurring competitor failure, and our answer

The tool's *shape* (local-first, files-as-truth, single-user, no-plugin-API, desktop) structurally avoids the four deadliest recurring killers and consciously accepts the rest.

| Recurring failure mode | Who it killed / hurt | formicarium's answer |
|---|---|---|
| Cloud / proprietary lock-in, weak offline, lossy export | Notion (CSV-not-MD export, CDN images break), Evernote (ENEX loss), Roam, Tana | **Files-as-truth in open Markdown+YAML.** Nothing to export; offline by construction; "leave anytime" is free. |
| Plugin / ecosystem rot + plugin security holes | Obsidian (Kanban/Projects/Dataview abandoned; community plugins run **unsandboxed with full filesystem + network access to your entire corpus** — a structural supply-chain attack surface) | **No third-party plugin API.** Extension = modular Rust core + declarative `.view` files + a personal Lua hatch. No external maintainers to abandon you; no unsandboxed attack surface. |
| Block-granularity → forced DB rewrite | Roam (born in it), Logseq (~3-yr rewrite, split user base) | **Atom = the file, never the block.** No per-bullet ids/timestamps — an invariant, enforced. |
| Multi-user sync-conflict / CRDT complexity; DB-corruption-by-sync | Anytype (custom CRDT, hard self-host), Joplin (silent overwrite), Trilium (generic sync corrupts its live DB) | Single-user; text syncs via git/Syncthing; **the SQLite index is per-machine and NEVER synced**; conflicts surface as Syncthing `.sync-conflict-*` copies, not silent last-write-wins. |
| Metadata with no query layer (inert structure) | Dendron | **Query engine + FTS5 is core.** Every frontmatter property is queryable; the whole point is retrieval. |
| Scale / perf cliffs | Roam (large graphs), TiddlyWiki (single file), Tana | Disposable index, incremental reindex (<10 s @ 10k), viewport-only render decoration, virtualized lists — all as **CI-enforced perf budgets**. |
| WYSIWYG ↔ Markdown round-trip loss | Joplin (RTE silently drops formatting) | **Markdown is literal truth. v1 edits raw text in a textarea (byte round-trip is free); the read view renders it. v2's CM6 live-preview decorations are a pure view layer, never mutating the doc.** Byte-identical round-trip, guaranteed by construction + a CI fixture test. |
| Fragile save mechanism | TiddlyWiki (browser file-write hacks kept breaking) | Atomic temp+rename to real files; git auto-commit for undo. |
| Abandonment = data death | Athens (dead, proprietary-ish graph), Dendron (survived because files were open) | Open format + local files: the *app* can die without taking the notes. OSS so the community can fork (as Trilium→TriliumNext proved). |
| Onboarding wall | Emacs/Org, Tana, Capacities, RemNote | Capture box always focused; type and go. No object model or keybinding grammar to learn first. |
| Rewrite paralysis / breaking format changes | Logseq (DB migration froze shipping), SilverBullet (v2 hard break) | **Freeze + version the on-disk format from commit 1** (schema version in the vault); files must outlive every app rewrite. |

### What it deliberately will NOT do (and why)

- **No mobile app (v1).** Desktop-only. This is the exact weakness that hurt Roam/Trilium/SilverBullet — stated openly rather than half-delivered. Multi-*desktop* is covered by file sync. Phone can be added later without touching the data model (it becomes a server+auth decision).
- **No real-time collaboration.** Structurally excluded by single-user + files (like Trilium by choice). A genuine limitation, not a bug.
- **No WYSIWYG / block editor.** Markdown stays canonical; a tree-first editor cannot guarantee lossless round-trip. Rejected to avoid Joplin's fidelity problem.
- **No live-preview editing surface in v1.** You edit raw Markdown in a textarea; the read view renders it beautifully. Live-preview (CM6 decorations) is a v2 upgrade — deferred because it is the single biggest build risk and adds nothing to *capture* or *retrieval*.
- **No structural query over note *body*.** Only frontmatter is structured-queryable; the body is full-text only (the same constraint Obsidian Bases shipped deliberately). If you want to filter on something, it goes in frontmatter.
- **No third-party plugin API — ever.** You have no strangers; you have git and an editor. Obsidian's unsandboxed plugin model is exactly the attack surface a single-user tool never has to open.
- **No telemetry, ever** — not even opt-in. No accounts, no hosted anything, no AI features in v1 (revisit once the core is boring and stable).
- **No app-level encryption.** Full-disk encryption is the OS's job (LUKS); the app does it worse. And **vendor every asset locally — no CDN calls, ever** (a CDN seeing what you load is the exact lock-in failure that broke Notion's images).

---

## Architecture — three seams, and nothing else matters

```
 Browser UI     board · agenda/calendar · timeline · gallery · search  ← cheap, swappable (a query + a renderer)
      │  transport: fm-serve HTTP /api/<cmd>  (prod)  ·  in-memory mock  (dev/test)
 query engine   filter · sort · generic group-by         ← THE STABLE CORE. Touches no filesystem, ever.
   ── Store seam ─────────────────────────────────────   ← SEAM 1 (compile-time guarded)
      ├─ MemoryStore  (tests, zero I/O)
      └─ FileStore    markdown files (truth) + SQLite index (disposable, per-machine)
   ── Blob seam ──────────────────────────────────────   ← SEAM 2: content-addressed, own volume, git-ignored
   ── OS seam ────────────────────────────────────────   ← SEAM 3: shell out — pdftotext · vipsthumbnail · git · restic · xdg-open
```

**Seam 1 is made structural, not conventional:** the pure crates (`fm-model`, `fm-query`) do not depend on `rusqlite`, `std::fs`, or any path type, so the query engine *cannot* do I/O — it fails to compile if it tries. CI adds belt-and-suspenders greps. This is the insurance policy that turns a future storage swap into a backend change instead of Logseq's three-year rewrite.

---

## Data model

One object = one note = **one Markdown file**; structure in YAML frontmatter. ULID id; links point at ids, never filenames. **Atom = the file, never the block.**

```yaml
---
schema: 1                       # on-disk format version — frozen early, migratable
id: 01J8ZQK4XN9P
type: note                      # note | task | meeting | asset
status: doing                   # enum, EXCLUSIVE — a property, never a tag
due: 2026-07-20                 # working target date, nullable — SOFT by default
hard: true                      # present only for fixed external commitments
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T11:03:00Z
tags: [meta-rl]                 # multivalued, unordered — NOT status
assets: [sha256:9f2a…]          # content hashes
code:   [meta-rl@a3f9c2e]       # immutable git refs
---
The GAE lambda seems to interact badly with inner-loop adaptation. $\lambda = 0.95$.
```

- **No `priority` field, ever.** Urgency is computed: `overdue > soon > this-week > later > none`; `hard` breaks ties and is flagged so a fixed deadline never hides behind slipping soft ones.
- **The soft `due` field is the priority dial** — nudging it forward *is* reprioritizing, so editing `due` is a one-keystroke gesture (`d` to set, `[`/`]` to ±1 day).
- A meeting is a note with `type: meeting` and a date; it appears in the agenda.

**The granularity landmine — read before agreeing.** "Atom = the file" is load-bearing and enforced. The trap is "every message should have a timestamp, like a GitHub comment": if *message* = a note (one file), it's safe — frontmatter carries `created`/`updated` natively, and per-entry timestamps come free by making entries *files* (capture writes a new small file; file mtime is entry mtime). But if *message* = a bullet inside a note, you have just specified block granularity — per-bullet ids/timestamps that standard Markdown cannot carry — which is exactly the atom that forced Logseq's three-year rewrite. If you ever want per-bullet ids, per-bullet timestamps, or block-level backlinks: **stop.** That is not a feature request; it changes the atom and voids this plan.

**Reference, don't ingest.** The tool is a *catalog over* artifacts, never a *container for* them. Three tiers of pointer, by how they rot:

| Thing | Pointer | Rots? |
|---|---|---|
| Code / experiments | `repo + commit SHA + path` (`code:`) | **No** — a SHA resolves the exact whole-project state; immutable by construction |
| Figures / slides / PDFs | `sha256` into the content-addressed blob store (`assets:`) | **No** — the hash *is* the content; auto-dedup |
| Bulk data (checkpoints, run dirs) | filesystem path + description (`data:`) | **Yes** — the one tier that can break, which is exactly what `verify` is for |

Everything queryable is a property; everything heavy is a pointer. Six months later you don't want a stale copy of a file — you want a *pointer with provenance* back to the exact code and data.

---

## Vault layout — one folder, knowledge in a subfolder

The application folder holds the app; the knowledge is a subfolder (its own independent git repo; the app's `.gitignore` excludes it). Vault path is configurable (default `./vault`).

```
formicarium/                    # the application (Rust workspace + frontend) — app's own git repo
└── vault/                      # THE KNOWLEDGE — a separate git repo
    ├── notes/                  # *.md — git-TRACKED text (the truth)
    ├── views/*.view            # declarative views (TOML) — git-tracked
    ├── themes/*.css            # CSS themes — git-tracked
    ├── scripts/*.lua           # optional Lua hatch — git-tracked
    ├── manifest.json           # integrity manifest (blob sha256 inventory) — git-tracked, small
    ├── blobs/sha256/ab/cd/<hash>   # media — git-IGNORED, own volume, Syncthing-synced
    ├── derived/<hash>/thumb.webp   # thumbnails — git-ignored, disposable
    ├── index.sqlite            # disposable, git-ignored, PER-MACHINE, NEVER synced
    └── .gitignore              # blobs/, derived/, index.sqlite
```

- **Heavy/media files are never git-tracked** (a `.githooks/pre-commit` 5 MB guard enforces it) and live on their own volume.
- **Media absence must never block the tool.** A missing blob is a warning, not an error: every surface (read-view widget, card chip, gallery tile) renders a shared "asset not found" placeholder and keeps working. `verify` reports missing blobs; nothing auto-deletes.

---

## The stack — tool-by-tool, with the reason for each

Efficiency, ease-of-use, open-source, cutting-edge-but-well-used. Licenses vetted (`cargo-deny` gate in CI).

> **Versions are indicative mid-2026 targets, not verified installs.** Verify each is current and re-pin at first commit (`cargo-deny` / `npm` resolve at pin time). Do not treat the exact patch numbers below as gospel.

### Shell, backend core (Rust)

| Tool | Ver | Why this one | License |
|---|---|---|---|
| **Tauri v2** | 2.11.5 | Native window + real global capture hotkey today; OS-webview (3–15 MB, no bundled Chromium); best 10-yr momentum. | MIT/Apache-2.0 |
| tauri-plugin-global-shortcut | 2.3.2 | `⌘/Ctrl+Space` always-focused capture from anywhere. | MIT/Apache-2.0 |
| tauri-plugin-opener | 2.5.4 | OS handoff (`xdg-open`) for non-web-native formats. | MIT/Apache-2.0 |
| **rusqlite** (`bundled`) | 0.40.1 | Compiles SQLite into the binary (no cgo, no runtime dep); **FTS5 on by default**. | MIT |
| serde + **serde_yaml_ng** | 1.0.228 / 0.10.0 | Frontmatter. (`serde_yaml` is **deprecated/archived** — do not use.) | MIT/Apache |
| ulid | 2.0.1 | Time-sortable stable ids. | MIT |
| sha2 | 0.11.0 | sha256 content addressing — verifiable with `sha256sum` forever (chosen over blake3 for universality). | MIT |
| infer | 0.19.0 | Pure-Rust MIME sniff (no libmagic C dep) — never trust extensions. | MIT |
| kamadak-exif | 0.6.1 | Image metadata → properties (pure Rust). | BSD-2 |
| notify (PollWatcher) | 8.2.0 | External-edit detection via **polling, not inotify**. | CC0 |
| walkdir + rayon | 2.5.0 / 1.12.0 | Parallel reindex traversal. | MIT |
| pulldown-cmark | 0.13.4 | Markdown parse where the backend needs it (and available to the read renderer). | MIT |
| time | 0.3.53 | Dates/timestamps. | MIT/Apache |
| git2 | 0.21.0 | Auto-commit + code-ref resolution (libgit2: GPL-2 **with linking exception** → safe). | MIT/Apache |
| **mlua** (`lua54`,`vendored`) | 0.12.0 | Optional scripting hatch (off by default). | MIT |
| clap · thiserror · tempfile · tracing | 4.6.1 · 2.0.18 · 3.27.0 | CLI, errors, atomic temp files, logging. | MIT/Apache |

### Frontend (TypeScript)

| Tool | Ver | Why | v1? | License |
|---|---|---|---|---|
| **Svelte 5** (runes) | 5.56.4 | Compiles to a tiny runtime, no vdom — "nice-looking but light." | v1 | MIT |
| Vite | 8.1.4 | Build + lazy `manualChunks`. | v1 | MIT |
| **KaTeX** | 0.16.x | Synchronous inline LaTeX render in the read view — fast, no reflow. | v1 | MIT |
| Mermaid | 11.16.0 | Diagrams-as-code, rendered inline in the read view (lazy). | v1 | MIT |
| **@excalidraw/excalidraw** | 0.18.1 | Freeform canvas as standalone `.excalidraw` JSON assets — **replaces tldraw** (source-available/watermarked; a 10-yr landmine). | v1 | MIT |
| @atlaskit/pragmatic-drag-and-drop | 2.0.1 | Board/gallery DnD; framework-agnostic. | v1 | Apache-2.0 |
| @tanstack/svelte-virtual | 3.13.32 | Virtualized gallery grid + long lists. | v1 | MIT |
| **CodeMirror 6** (`@codemirror/view` etc.) | view 6.43.x | Live-preview *editing surface* — the only editor that keeps the Markdown file itself as literal truth (byte round-trip). | **v2** | MIT |
| @codemirror/lang-markdown + @lezer/markdown | 6.5.0 / 1.7.1 | GFM + custom math/wikilink Lezer extensions (for the v2 live-preview editor). | **v2** | MIT |
| MathLive | 0.110.0 | Optional interactive equation-input widget. | **v2** | MIT |

**Escape hatches, pre-planned:** search → **Tantivy 0.26.1** (Lucene-class, if FTS5 ever isn't enough); it isn't at 10k docs. Storage → `SqliteStore` behind the same `Store` seam if files-as-truth ever must go. Editor → CM6 live-preview swaps into the same Markdown-as-truth invariant when the textarea genuinely hurts.

### OS subprocess tools (invoked, never linked — license-isolated)

| Tool | Why | License / note |
|---|---|---|
| **pdftotext** (poppler) | Extract PDF text → FTS5 — *the highest-value feature* (search inside every paper). | GPL — **subprocess only**. (Link `pdfium-render` BSD-3 only if ever needed.) |
| **vipsthumbnail** (libvips) | Image/video thumbnails for the gallery. | LGPL — subprocess sidesteps relink duties; avoids libvips thread-safety issues. |
| **restic** | Backup: dedup + encryption + integrity + remote. `restic check --read-data` = the bit-rot scrub. | BSD-2 |
| **git** | History/undo (auto-commit) + text merge on sync. | GPL — via `git2` (linking-exception) or subprocess. |
| **Syncthing 2.x** | Whole-vault + media sync across your machines; conflicts kept as `.sync-conflict-*` copies. | MPL-2.0 |
| tesseract *(v2)* · ffmpeg *(v2)* | OCR of scanned PDFs; video poster frames. | Apache / LGPL — subprocess only. |

---

## Dependency & environment management — three lockfiles, one entry point

The stack spans three dependency domains, and only two of them have an obvious home. Rust crates → `Cargo.lock`. JS packages → `pnpm-lock.yaml` (pnpm over npm: content-addressed store, strict hoisting, fast). The **third domain — the toolchain and the native/subprocess tools** (`rustc`/`cargo`, `node`/`pnpm`, poppler, libvips, restic, ffmpeg, tesseract, and the webview build libs) — is the one every plan hand-waves as "assume `apt` has it," and it is exactly the layer that rots over ten years: apt versions drift, a `pdftotext` flag changes, a libvips soname bumps, and a build that worked in 2026 fails to reproduce in 2031.

**Decision: adopt [pixi](https://pixi.sh) as the umbrella environment + task runner**, layered *over* Cargo and pnpm (it replaces neither). pixi resolves conda-forge (+ PyPI) into a **cross-platform `pixi.lock` that pins exact versions and hashes** of the toolchain and every system tool, and doubles as a task runner so the whole polyglot build has one entry point.

```
 pixi.toml + pixi.lock          ← THE UMBRELLA: toolchain + system tools + task runner (conda-forge)
   ├─ rust, nodejs, pnpm        ← toolchains (pinned, reproducible; no rustup/nvm drift)
   ├─ poppler, libvips, restic, ffmpeg, tesseract   ← the subprocess tools, version-locked
   └─ tasks:  build · dev · test · lint · reindex-bench · package
        │  delegates to ↓                     │  delegates to ↓
   Cargo.lock  (Rust crates)            pnpm-lock.yaml  (JS packages)
```

**Why pixi over the alternatives** (a real choice, not a rubber stamp):

| Option | Verdict for this project |
|---|---|
| `apt` + a `setup.sh` | Simplest, but **nothing is version-locked** — the drift problem stays unsolved. Rejected. |
| **pixi** | conda-forge has every subprocess tool *and* `rust`/`nodejs`; gentle (a researcher likely already knows conda); one `pixi.lock`; built-in task runner. **Chosen.** |
| Nix / devenv | The gold standard for 10-yr reproducibility (pins webkit too), but the friction is real and this is a solo researcher, not a Nix shop. Kept as the escape hatch if pixi's native coverage ever proves insufficient. |
| mise / asdf | Manages *toolchain versions* well but **not** system C-libs like poppler/libvips. Insufficient. |

**The GUI webview — SUPERSEDED (2026-07-15).** The native Tauri window (and its `gui` pixi environment with `webkit2gtk4.1`/`libsoup`/`gtk3`) was removed after the WebKitGTK surface never painted reliably. The app now runs in the user's own browser, served by `fm-serve` (std-only, no webview to provision), so there is **no GUI native stack to pin** — the default env builds and runs everything. The glibc 2.34 platform floor is kept only because it's harmless and met everywhere.

**Sketch (`pixi.toml`, pin at first commit):**
```toml
[project]
name = "formicarium"
channels = ["conda-forge"]
platforms = ["linux-64"]          # desktop-only v1; add osx-arm64 when a Mac build is wanted

[dependencies]                    # toolchain + version-locked native tools (NOT app-level deps)
rust = "~=1.83"
nodejs = "~=22.0"
pnpm = "*"
poppler = "*"                     # provides pdftotext
libvips = "*"                     # provides vipsthumbnail
restic = "*"
cargo-deny = "*"

[feature.media.dependencies]      # v2 extractors, isolated in their own environment
ffmpeg = "*"
tesseract = "*"

[environments]
default = { features = [] }
media   = { features = ["media"] }

[tasks]
serve    = "pnpm -C ui build && cargo run -p fm-serve"   # build + serve the browser app (FM_OPEN opens it)
docs     = "mdbook build docs"                            # render the manual (also in `ci`)
test     = { cmd = "cargo test --workspace && pnpm -C ui test" }
lint     = "cargo deny check && cargo clippy -- -D warnings"
seam     = "cargo test -p fm-query"          # the zero-I/O seam suite
```

**Runtime, not just build.** Because pixi pins the subprocess tools, `pixi run dev` (or `pixi shell`) puts the *exact* `pdftotext`/`vipsthumbnail`/`restic` on `PATH` regardless of what apt has — so "search inside every paper" and the bit-rot scrub behave identically on every machine and in CI. For a distributable `.deb`, declare these as Debian runtime deps instead; for the single owned machine that is v1, the pixi environment *is* the runtime.

**Consequences woven into the rest of the plan:**
- **First three commits** gain a step: commit `pixi.toml` + `pixi.lock` alongside the workspace scaffolding, and make CI run `pixi run test` / `pixi run seam` / `pixi run lint` so the perf-budget, seam, and `cargo-deny` gates all execute inside the locked environment.
- **`cargo-deny`** (license gate) and the two CI greps run as pixi tasks — one `pixi install` reproduces the entire dev/CI toolchain.
- **Longevity:** three lockfiles committed to the app repo mean a 2031 checkout resolves to the same toolchain and the same `pdftotext`. With the webview gone there is no GUI native surface to pin — the lean default env builds and runs everything.

---

## Backend architecture (Rust workspace)

Three crates so seam 1 is a compile-time guarantee:

- **`fm-model`** — pure: `Object`, `Kind`, `PropertyValue`, `schema.rs` (SCHEMA_VERSION + `migrate()`). No fs, no db.
- **`fm-query`** — pure query engine: `Query`/`Filter`/`Predicate`/`SortKey`, `engine::run`, generic `group()`. No fs, no db.
- **`fm-core`** — everything with I/O: `Store` trait, `MemoryStore`, `FileStore` (frontmatter + atomic write + SQLite/FTS5 index + reindex), `ingest`, `blob`, `verify` + manifest, `history` (git), `backup` (restic), `view`, `script` (mlua, `#[cfg(feature="lua")]`).
- Plus **`fm-cli`** (`fm add|reindex|verify|manifest|backup`), **`fm-app`** (the command *library* — `commands` + DTOs), and **`fm-serve`** (the std-only HTTP server that fronts those commands to the browser).

**The seam:**
```rust
pub trait Store {
    fn get(&self, id: Id) -> Result<Option<Object>>;
    fn put(&mut self, obj: &Object) -> Result<()>;      // file write + index in ONE txn
    fn delete(&mut self, id: Id) -> Result<()>;
    fn query(&self, q: &Query) -> Result<QueryResult>;
    fn reindex(&mut self, mode: Reindex) -> Result<ReindexStats>;  // Incremental | Full(DROP+rebuild)
}
```
`Query.filter` is a vec of `Predicate` (`Kind`, `Prop{key,op,value}`, `TagsAll/Any`, `DateRange`, **`Text(String)`**, `Not`, `Any`). **Full-text is just another predicate:** `FileStore` implements `Text` with FTS5 (`MATCH`), `MemoryStore` with a substring scan — same contract, so search is not special. **Generic group-by** reads any property by name via `Object::get(key)`; the board renderer receives opaque `Vec<Group>` and never sees the string `"status"`.

**Ingest** = one function, three transports (Tauri paste/drag over IPC; `fm add` with `cp --reflink=auto`; watched inbox in v2). Steps: stream-hash sha256 → dedup (existing hash → discard+return) → atomic commit to `blobs/` → sniff MIME → extract metadata → extract text → FTS → **return** → async thumbnail (fire-and-forget). Any step past hashing can fail and only degrades that feature.

**Index lifecycle:** reindex on startup; 3 s mtime poll for external edits; in-app writes update the index in the same transaction; `fm reindex --full` = DROP+rebuild (<10 s @ 10k). A reindex-idempotence test proves the index is genuinely disposable.

**`verify` + integrity manifest** (report-only, à la `git fsck`): unparseable frontmatter, unresolved code refs, missing blobs (→ warn), dangling links (v2). `manifest.json` is a plain (optionally minisign-signed) sha256 inventory; `fm verify --scrub` re-hashes blobs against it to catch bit-rot — the genuinely hard part of "lasts 10 years," treated as first-class.

---

## Frontend architecture

**Editor & read view (v1) — split, not live-preview.** The single biggest build risk (a CM6 live-preview decoration layer) is deferred to v2. v1 ships two low-risk pieces behind one Markdown-as-truth invariant:

```
 v1 (ships)                                   v2 (deferred upgrade)
 ┌────────────────────────────┐               ┌────────────────────────────┐
 │ EDIT: plain <textarea>      │   same        │ EDIT: CM6 live-preview      │
 │   literal Markdown bytes    │   Markdown    │   decoration ViewPlugin     │
 │   500ms debounce → atomic   │   files as    │   (ref: retronav/ixora,     │
 │   write; mtime check        │   truth,      │   codemirror-rich-markdoc)  │
 │ READ: Svelte render view    │   byte        │   swaps in behind the same  │
 │   pulldown-cmark → HTML     │   round-trip  │   round-trip fixture test   │
 │   + KaTeX + media/mermaid/  │   invariant   └────────────────────────────┘
 │   excalidraw + asset:       │
 │   resolver + Missing tile   │
 └────────────────────────────┘
```

- **EDIT (textarea):** raw Markdown, always-focused capture. Typing stops → 500 ms → atomic write (temp+rename) via the existing `update_body` IPC, which does the `mtime` staleness check. Byte-identical round-trip is *trivial* here because a textarea holds literal bytes — no decoration layer can corrupt a note you also edit in Vim.
- **READ (render component):** a Svelte view that parses the Markdown (pulldown-cmark or a small TS Markdown lib) and renders it nicely: inline **KaTeX** for `$…$`/`$$…$$`, **Mermaid** (lazy) for diagram blocks, and `WidgetType`-free HTML widgets for image/video/audio/pdf/excalidraw via the `asset:` resolver, each falling back to the shared **`AssetMissing`** placeholder. This is where "looks nice" lives in v1, and it is far lower risk than editing-surface decorations.
- **Round-trip invariant + CI test:** `read → store → read === bytes`. It holds trivially for the textarea and re-applies unchanged when CM6 arrives in v2 (`read → mount in CM6 → toString() === bytes`). Build each read-view widget one element at a time, each gated by a round-trip fixture.

**Five generic renderers = query + a renderer** (`.view` config files remain planned):
| Renderer | Query | Behavior |
|---|---|---|
| **board** | any | columns from distinct values of `groupBy` (**any** property); Pragmatic DnD drop → `set_property(id, key, value)`. **Renderer must not contain `todo`/`doing`/`done` — CI greps `ui/src/renderers/**` and fails the build if found.** |
| **agenda** | `status!=done AND due!=null`, sort due asc | the "closest deadline" view; a **Month/Week calendar** or a list. Urgency computed in the card, `hard` flagged — **zero new query code** |
| **timeline** | all, created desc | a Logseq-style journal grouped by creation day |
| **gallery** | `type=asset` | grid of thumbnails |
| **search** | `Text` predicate (FTS5) | full-text results across notes + extracted PDF text |

**Commands** (query engine stays in Rust; the frontend calls **named commands** with simple args — the `Query` struct is built server-side, never sent). The real surface is **14**: `board`, `gallery`, `agenda`, `recent`, `search`, `get`, `capture`, `set_property`, `update_body`, `ingest` (binary upload → asset note), `resolve_asset`, `asset_status`, `open_external`, `commit`, `backup`. (`reindex`/`verify`/`manifest` are CLI-only; reindex also happens implicitly on `FileStore::open`.) `ObjectMeta.props` is an open map, so **custom frontmatter properties flow through with no code change** — required for board-by-any-property.

**Asset resolution + graceful absence:** `asset:sha256-…` → bytes fetched over `/api/resolve_asset`, wrapped in a typed object URL. The read view renders by sniffed MIME with **native browser elements** — `<img>`, a scrollable `<iframe>` for PDF, `<video>`/`<audio>` — no JS media libraries. `asset_status` reports `has_blob:false` (not synced yet) → shared `AssetMissing` placeholder; try/catch in the resolver. Nothing crashes. Authoring: drag a file into the editor or type `/` to search-and-insert an asset.

**Extensibility, layer 1 = CSS themes:** design tokens as CSS custom properties; pill/urgency colors via `data-value` attribute selectors, so themes color arbitrary enum values while renderer code stays literal-free. A theme is one CSS file. Layer 2 = `.view` files (shared with backend). Layer 3 = the optional Lua hatch.

**Perf budgets (CI assertions from commit 3):** capture interactive < 200 ms · search render < 100 ms @ 10k · keystroke→paint < 16 ms · reindex < 10 s @ 10k · core JS < 300 KB gz (KaTeX/Mermaid/Excalidraw lazy-loaded; CM6/grammars only enter the bundle in v2).

---

## Extensibility — three layers, no plugin API

1. **Modular Rust core.** A new renderer is a match arm; a new store is `impl Store`; a new extractor is an arm in `ingest::text`. Afternoon-sized, by you, with no public API to freeze.
2. **Declarative `.view` files** (TOML: filter/group/sort/renderer). Zero code execution, git-diffable. Handles ~80% of "reshape my notes." Ships with four hand-written views.
3. **Optional embedded Lua (`mlua`)**, off by default. Scripts live in `vault/scripts/`, hot-reloaded; single trusted user ⇒ no sandbox. A tiny surface (`fm.query`, `fm.get`, `fm.put`, `fm.ingest`) for computed views / bulk transforms / capture templates. Explicitly a *personal* hatch — no stable ABI, no distribution. (Modeled on SilverBullet Space Lua; Obsidian's unsandboxed plugin model is the reason we refuse a real plugin API.)

## Sync, backup, durability

- **git** for note text (undo + non-overlapping merge). Auto-commit (500 ms→disk, 30 s/blur→commit) buys undo, **not** readable history — accepted.
- **Syncthing 2.x** for the whole vault incl. media; genuine conflicts become `.sync-conflict-*` copies (surfaced, not silently lost). **No CRDT** (single user; not worth the cost). **The index is never synced** (Trilium's DB-corruption lesson) — each machine rebuilds it.
- **restic** to an external drive + an rclone/S3 remote (3-2-1). `restic check --read-data` on a schedule = bit-rot scrub. **Test the restore in month one** — an untested backup is not a backup.

## Cross-domain ideas adopted

OCI-style signed integrity manifest (bit-rot inventory) · git loose→packed lifecycle idea to bound inode count on a 1 TB blob store · restic CDC for sub-file dedup in backup · optional per-note `(device, logical_clock)` to *detect* concurrent edits without a CRDT · Tantivy immutable-segment model as the search escape hatch.

---

## Build order — S4 is the checkpoint

| | Slice | Proves |
|---|---|---|
| S0 | type → atomic write → reindex on reload → still there | the pipeline exists |
| S1 | search box → FTS5 → results w/ timestamps | retrieval works |
| S2 | properties editable; `status` settable | the typed model holds |
| S3 | **Board:** group by `status`, drag writes back | the thesis is alive |
| **S4** | **Gallery:** a *second* renderer over the same query layer | **the thesis is proven — or dead.** If it costs more than a weekend, `fm-query` isn't a real seam; stop and fix it. |
| S5 | assets (hash/dedup/pdftotext/thumbnail) + agenda `.view` + **rendered read view (KaTeX/media/mermaid inline)** | media library + deadlines + "looks nice" |
| S6 | `verify` + manifest + restic + a **tested restore** | durability |

**First three commits:** (0) `pixi.toml` + `pixi.lock` + `Cargo`/`pnpm` workspace scaffolding, so every later commit builds and tests inside the locked environment (`pixi run test|seam|lint`). (1) `fm-model` + `fm-query` + `MemoryStore` + the query suite that passes with **zero filesystem access** — this commit *is* the seam. (2) `.githooks/pre-commit` 5 MB guard + `core.hooksPath`. (3) Perf budgets + `cargo-deny` + the two greps (`fm-query` fs-imports; renderer `todo|doing|done`) as CI assertions, all run as pixi tasks while they're trivially green.

**Kill criteria:** board not working after 8 weekends → install SilverBullet, you've learned enough · S4 painful → architecture wrong, stop · 2 weeks without opening it during the build → it isn't solving your problem, find out why.

## Build status — what works, what doesn't (as of 2026-07-14)

The whole build order **S0–S6 is implemented and committed on `main`**. What is *verified* vs. *unverified* differs by layer — recorded here honestly so nothing is mistaken for "done":

**Verified working** (exercised end-to-end with the real, pixi-pinned subprocess tools):

- **Backend / CLI (`fm`), S0–S6.** Capture → atomic write → reindex-on-reload; an external Vim edit is picked up on reindex. FTS5 search returns timestamped hits — **including words that appear only inside an ingested PDF** (`pdftotext` → FTS, the killer feature). Properties editable with custom-property round-trip (no silent data loss). Assets: content-addressed blobs with dedup, text extraction, `vipsthumbnail` thumbnails. Durability: `manifest` → `verify --scrub` catches bit-rot (exits non-zero) → `restic backup` → `check --read-data` → `restore` diffs **byte-identical**. Reindex is idempotent (the index is disposable).
- **Query seam.** `cargo test -p fm-query` passes with **zero filesystem access**; the board renderer is generic (groups by any property; the `todo|doing|done` CI grep stays green).
- **Frontend SPA in a browser.** `svelte-check` + `vite build` clean; the board/gallery/agenda renderers, the note read/edit view, and inline KaTeX/Mermaid **render correctly in a normal browser** (kanban board confirmed by hand). This proves the UI logic and the IPC *shape* are sound.
- **Browser app via `fm-serve` — this is the product.** The command library (DTOs, board/agenda/timeline/gallery/search, property write-back, asset resolution over the `Store` seam) is unit-tested and webkit-free; a std-only HTTP server fronts it to the browser and serves `ui/dist`. The real command surface is **14** (not the earlier 7). The native Tauri window was removed (see the reversal table).
- **CI.** `pixi run ci` is green: workspace tests + `cargo-deny` license/advisory gate + the architectural greps.

**Not verified / not yet working:**

- **On-screen rendering — no longer a question.** The blank-window problem was retired by dropping the WebKitGTK webview: the UI now renders in the user's real browser (Chromium/Firefox/…), which paints reliably and gives native inline media (scrollable PDF `<iframe>`, `<video>`/`<audio>`) for free. Former Risk #1 is closed.
- **Deferred v1 GUI-interactive bits** (need a real display; not built): global capture hotkey, live asset/thumbnail display via the Tauri asset protocol, `.view` config files, git auto-commit on idle/blur.

## Resource weight & framework choice (reassessed 2026-07-14)

**Verdict: light at this scale; the architecture is sound.** Boot loads a **72 KB** entry chunk; a plain note adds ~135 KB; KaTeX and Mermaid are *doubly* lazy — behind the note-panel dynamic import **and** feature-gated (no `$` → no KaTeX; no ` ```mermaid ` → no Mermaid). There are **no background threads, timers, watchers, or polling** — zero idle CPU from our code. The Rust working set is tens of MB.

**Superseded by the move to a browser app (2026-07-15).** The old worry here was WebKitGTK's ~150–300 MB idle RAM from the embedded webview. That cost is gone: formicarium no longer ships a webview. The server (`fm-serve`) is std-only — no HTTP framework, no bundled browser — so its resident set is tens of MB; the UI runs in a browser the user already has open. The single-digit-MB Rust binaries and the lazy KaTeX/Mermaid budgets still hold. **Decision: the browser is the product; keep the server tiny.**

**Tuning applied (this pass):**
- **Mermaid gated to a lightweight set** — the cytoscape-backed `architecture`/`mindmap` diagrams are aliased out of the bundle (`ui/vite.config.ts` → `ui/src/lib/cytoscape-stub.ts`), dropping ~0.6 MB of graph libraries (`ui/dist` 5.1 MB → 4.5 MB). Everyday diagrams (flowchart, sequence, gantt, class, state, ER, pie, …) use dagre and are unaffected; a dropped type fails gracefully to its code fence (`render.ts` per-block `try/catch`).
- **Release profile added** (`[profile.release]` in root `Cargo.toml`: `strip` + thin-LTO + one codegen unit) so the shipped binary is ~10–25 MB, not the 227 MB *debug* build. (The multi-GB `target/` is dev cache — `cargo clean` reclaims it.)
- **Reindex runs in one transaction** (`fm-core::file::reindex`) — was ~3 un-batched SQL statements per note; a single commit is the cheap scaling win.

**Deferred until the vault reaches ~1–2k notes** (latent O(n) costs, negligible at current scale): incremental mtime-diff reindex — the `mtime_ns` column and the `Reindex::Incremental` variant already exist, unused — so every open is O(changed) not O(all); and avoiding a full YAML re-parse of the whole corpus on every board/gallery/agenda render (an in-memory object cache, or promoting frontmatter fields to SQL columns). Gate both behind a reindex perf-budget test.

## Risks (ranked)

1. **~~Linux WebKitGTK rendering~~ — RESOLVED by removal (2026-07-15).** This was the top risk: the WebKitGTK window rendered blank and on-screen rendering stayed unverified. It is retired — the native window was removed and the UI now runs in the user's real browser, which renders reliably and is what the user actually uses. No webview, no risk. (The remaining risks below stand: libvips subprocess robustness, KaTeX cost, git auto-commit noise, cross-store tokenizer drift, and CM6 in v2.)
2. **libvips thread-safety** → subprocess `vipsthumbnail`, never in-process.
3. **KaTeX cost in math-dense read views** → render once per note view, LRU cache, lazy Mermaid; measure worst case.
4. **git auto-commit noise / `git add -A` cost past 10k files** → commit on idle/blur only; measure; don't build "commit management."
5. **Cross-store query drift** (FTS5 tokenizer vs substring scan) → pin `unicode61 remove_diacritics 2`; equivalence test catches regressions.
6. **CM6 live-preview decoration layer (v2 risk).** No turnkey lib; ~1,200–1,800 LOC; highest build risk in the whole project — which is exactly why it is deferred out of v1. When built: one element at a time, each round-trip-tested; stays in 16 ms via viewport-only iteration, `WidgetType.eq()`, incremental reparse, LRU KaTeX cache.

## Verification (how to prove each slice works end-to-end)

- **S0/S2:** capture a note in the app → confirm a real `.md` file with correct frontmatter appears under `vault/notes/` → edit the same file in Vim → trigger poll/reindex → change is reflected in the app (proves external-edit path). Byte-round-trip CI test: `read → store → read === original bytes` (textarea path).
- **S1:** seed 10k fixture notes + a few PDFs (`fm add`) → search a term that appears only inside a PDF → it returns → assert < 100 ms (CI perf assertion).
- **S3/S4:** drag a card between board columns → confirm the frontmatter value changed on disk → point the *same* board at `groupBy: type` → get a board of note/task/asset (the board-renderer falsifiability test) → build the gallery and time it (the S4 gamble).
- **Seam:** `cargo test -p fm-query` passes with zero filesystem access; reindex-idempotence test (index → snapshot queries → `--full` rebuild → identical results).
- **S5:** paste an image twice → one blob (dedup); reference a blob, delete it → "asset not found" placeholder renders and the app keeps working; `fm verify` reports it; a math+image+mermaid note renders correctly in the read view.
- **S6:** `restic backup` → `restic restore` to a scratch dir → diff against the vault → `fm verify --scrub` clean. Run `/verify` (project verify skill) on each slice.

## Deferred (v2+)

**CM6 live-preview editing surface** (the decoration layer — the biggest single build risk) · Wikilink *backlinks panel* · `.view` config files (layer-2 extensibility; the renderers are hardcoded for now) · ⌘K command palette polish · watched inbox · OCR (tesseract) · pptx/docx extraction (pandoc) · GC (report+quarantine only) · phone/remote access (auth) · MathLive equation editor · semantic search. *(Shipped since this list was written: the month/week calendar, the timeline/journal, search, the properties editor, asset drag/slash authoring, inline PDF/video/audio, one-click backup + git auto-commit.)*

---

**First move:** the three seam commits above (the zero-I/O query suite, the pre-commit 5 MB guard, the CI perf/grep budgets), then drive S0.

---

## Trade-offs & accepted limits

Files-as-truth with a file-level atom is the right call, but it has costs. Accept these now, in writing, so they are not discovered as bugs later.

| Limit | Consequence | Mitigation |
|---|---|---|
| No atomic multi-object transactions | Moving 5 cards = 5 file writes; a crash mid-op leaves inconsistent state | Write-temp-then-rename per file; no operation is designed to move many objects atomically; `verify` catches drift |
| No referential integrity | Delete a linked note → dangling links; the filesystem won't stop you | `verify` reports dangling refs (v2 for link-graph); never auto-delete |
| No native relations | Links are ids in a list property — no foreign keys, no joins | Resolve through the index; same constraint Obsidian shipped |
| Rename cost | Renaming a page would rewrite every referrer (Logseq's reason #1) | **Stable ULID in frontmatter; links by id, never filename** — kills the rename problem outright |
| Body not queryable | Structure in the body is invisible to board/agenda | If you want to filter on it, it's a property (frontmatter). Body is full-text only |
| YAML parse cost on reindex | 10k files × parse | Bounded by the <10 s full-reindex budget; cache by mtime+size if ever violated |
| Filesystem quirks | Case-insensitivity, path length, unicode normalization | Filenames are **display only**; the id is truth; sanitize aggressively |
| Auto-commit ≠ readable history | Thousands of `auto:` commits; `git log` is noise | Accepted — you bought *undo/crash-safety*, not a narrative. Don't build "commit management" |
| No real-time collaboration, ever | Single-user by construction | Already a non-goal; do not revisit |

---

## Evidence & prior art — why these decisions, not others

The architecture is the intersection of what shipped and survived, avoiding two documented failure modes. The load-bearing proof, preserved so the claims above aren't bare assertions:

**Logseq — block granularity forced a file→DB rewrite.** The founder's stated reasons: (1) file-system sync/rename cost — renaming a page rewrites every referrer; (2) real-time collaboration on Markdown files is "almost impossible"; (3) **not every block has a persistent id or timestamp**. Reason 3 is load-bearing: Logseq's atom is the *block*, and standard Markdown cannot carry per-block metadata. Outcome: the project split into two products (files version in maintenance; DB canonical, Markdown an export), at a cost of roughly three years and a halved user base. The payoff of that rewrite — Kanban/Calendar/Gallery views, properties, classes — *is our target feature set*, which we reach without the block atom.
  · https://discuss.logseq.com/t/database-version-too-drastic-choice/20346 · https://discuss.logseq.com/t/logseq-og-markdown-vs-logseq-db-sqlite/34608

**Obsidian Bases — existence proof of "one model, many views" on plain files.** A saved-query config layered over Markdown + YAML frontmatter (table/cards/list); the data stays in the notes. Its deliberate, stated constraints are the price of admission we also pay: **Bases does not query the note body** (all queryable data lives in frontmatter); inline `field:: value` is invisible; **no native relation type** (relations are wikilinks in a list property, no referential integrity). Proof the configuration works without abandoning the file format.
  · https://deepwiki.com/obsidianmd/obsidian-help/5.1-introduction-to-bases

**Dendron — the negative control.** Files + frontmatter + hierarchy, but **no query layer, so the metadata was inert** ("under-utilized because we don't have a built-in way of easily querying by it"; "virtually everything stops working past 10k notes"). Structured properties without a query engine are decoration — which is why our query engine + FTS5 is core, not optional. Bonus: because Dendron was open + local, its shutdown didn't destroy anyone's data — **file-based storage is bus-factor insurance** for a solo project whose largest risk is author burnout.
  · https://wiki.dendron.so/ · https://github.com/dendronhq/dendron/discussions/3890

**SilverBullet — files + a Lua query engine, actively maintained.** Plain Markdown, self-hosted single binary, scripting for queries/custom views. Independent confirmation that files + query layer is viable — and the model for our optional Lua hatch. (Run it for two weeks before building: if it's enough, you saved six months.)
  · https://silverbullet.md/ · https://lwn.net/Articles/1030941/

**Kanban's convergent evolution — "views belong in core, as generic renderers."** Both baselines walked the same path from opposite directions. Obsidian: kanban-as-plugin stored boards in its *own* format (a silo, un-queryable) → the Projects plugin people relied on **was abandoned** (plugin rot) → the community rebuilt it correctly as a generic renderer that **generates columns from any property** and writes the value back on drop → Obsidian is now pulling Kanban/Calendar into core. Logseq's DB rewrite made the same views first-class from the other side. Two projects, different architectures, identical destination: a view is *a query + a renderer*, and a board that hardcodes `todo/doing/done` is a feature you'll rebuild the day you group by priority. Hence the CI grep that fails the build on those strings.
  · https://community.obsidian.md/plugins/kanban-bases-view · https://forum.obsidian.md/t/bases-kanban-view/101593

**The plugin-security corollary.** A plugin API lets strangers extend your software without touching your source — and Obsidian's community plugins run unsandboxed with full filesystem + network access to the entire corpus. For a single-user tool that is nearly pure cost (permanent API surface freezes refactors; one bad plugin taxes the whole app; a decade of your private notes is the blast radius). You have no strangers; you have git and an editor. Extensibility comes from CSS themes and declarative `.view` files — no code execution — plus a personal, off-by-default Lua hatch.
