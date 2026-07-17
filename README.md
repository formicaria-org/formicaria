<p align="center">
  <img src="docs/assets/formicaria.svg" alt="Three ant colonies, each nest a labyrinth of galleries, linked by braided trails with ants moving both ways along them" width="820">
</p>

<h1 align="center">formicaria</h1>

<p align="center">
  <em>Knowledge, task scheduling and collaboration — on files you own.</em>
</p>

---

A *formicarium* is one colony's nest. The plural is the architecture: a **set of vaults**,
one per audience — your own notes, the lab's, a paper with someone — each its own repo with
its own collaborators, coordinating through git rather than through anyone's server. Most
of the traffic stays inside a colony. Some of it flows both ways along the routes between.

**Your notes are Markdown files you own.** One note per file, plain text, on your disk,
readable in Vim or a text editor with this app nowhere in sight. In the worst case they
survive as what they always were: files. Heavy media are content-addressed blobs referenced
from notes, so the notes repo stays small and clonable for a decade.

Knowledge, scheduling and collaboration aren't three subsystems — they're three views of
one file. A task is a note with a `due`. A shared note is a note in a different repo.

## Install

Download the archive for your platform from
[**Releases**](https://github.com/singhbal-baljinder/formicaria/releases), unpack, and run
`fm-serve`. There is no installer and no setup step.

```sh
tar xzf formicaria-*-linux-x86_64.tar.gz && cd formicaria-*
./fm-serve                     # → http://127.0.0.1:8765
```

Linux · macOS (Apple silicon) · Windows. Each archive carries its own README with the
per-OS details. **Keep `fm` beside `fm-serve`** — the second binary is what git calls to
merge notes.

<details>
<summary>Or build from source</summary>

Everything is pinned through [pixi](https://pixi.sh); nothing else needs installing.

```sh
pixi run serve            # build + serve, dev loop (a .svelte edit needs no Rust rebuild)
pixi run build            # → target/release/{fm-serve,fm} + ui/dist, then run fm-serve
```

`pixi run app` runs the already-built release binary with no rebuild — what the
[`packaging/`](packaging/README.md) desktop launcher uses.
</details>

## What it needs

**Nothing.** The core — writing, tasks, dates, board, agenda, search — is one
self-contained binary with the UI baked in. It runs on a machine with no tools installed
at all.

Everything else is a *feature*, found on your `PATH` and reported honestly when it's
absent. Install them however you like — apt, brew, [pixi](https://pixi.sh), whatever:

| Install | To get |
|---|---|
| **git** | History — undo that outlives the session — plus backup and sharing a vault |
| **pdftotext** | The text inside a PDF you drop in becomes searchable |
| **vipsthumbnail** | Thumbnails for images and PDFs |
| **restic** | Encrypted, deduplicated backup of your media |

## How sharing works

A vault is a git repo. Give it a remote and it's shared; the people who can clone it are
exactly the people who can read it. **Where a note lives decides who can see it** — not a
label in the file, because a label you can type is a label you can typo, and git history is
forever.

Two people editing *different paragraphs of the same note* merge cleanly. That sounds
unremarkable and isn't: the file format rewrites `updated:` on every save, so any two
concurrent edits collide on that line, inside the YAML, where the parser refuses them. A
frontmatter-aware merge driver resolves the collision the format manufactures — and when a
conflict is real, it lands in the *body*, so the note still opens and you settle it in the
editor.

No account, no cloud, nothing phones home. The server binds `127.0.0.1` and has no
authentication, because it was never meant to need any — don't expose it.

## Documentation

The manual is an mdBook under [`docs/`](docs/) (`pixi run docs`); start at
`docs/src/introduction.md`. The design spec is
[`formicaria/MASTERPLAN.md`](formicaria/MASTERPLAN.md). For maintainers,
[`docs/context/`](docs/context/README.md) is the project's working memory — what exists,
*why*, and what doesn't.

## Development

One gate:

```sh
pixi run ci               # test + test-ui + deny + checks + docs
```

CI runs it on Linux when code changes, and on macOS + Windows too. A `v*` tag builds and
publishes all three platforms.

```text
crates/  fm-model · fm-query · fm-core · fm-app · fm-serve · fm-cli
ui/      Svelte 5 + Vite (baked into the binary at build time)
docs/    the mdBook manual + docs/context (maintainer notes)
vault/   your notes (its own git repo; git-ignored by this one)
```

## Licence

[MIT](LICENSE). Use it, change it, sell it, close it — keep the notice, that's all.
Release archives also carry `THIRD-PARTY.md`, the notices of the libraries built into the
binary.
