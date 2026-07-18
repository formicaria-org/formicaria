<p align="center">
  <img src="docs/assets/formicaria.svg" alt="Three ant colonies, each nest a labyrinth of galleries and corridors, every one linked to every other by braided trails, with ants moving both ways along all of it" width="820">
</p>

<h1 align="center">formicaria</h1>

<p align="center">
  <em>A local-first research notebook: knowledge, task scheduling and collaboration over plain Markdown files.</em>
</p>

---

formicaria stores each note as a single Markdown file with YAML frontmatter, in a directory
you control. It runs as a local web application: a small server on your machine, and your
browser as the interface. There is no account, no cloud service and no network dependency.

A vault is an ordinary git repository. Running several — one per audience, such as personal
notes, a lab's notes, and a paper shared with a collaborator — is the normal case, and is
what the name refers to: a *formicarium* is one colony's nest, and *formicaria* is the
plural.

Notes, tasks and shared documents are not separate systems. A task is a note with a `due`
property; a shared note is a note in a different repository.

## Requirements

| | |
|---|---|
| **A web browser** | Any current Firefox, Chrome, Safari or Edge. This is the interface; there is no separate desktop window. |
| **A 64-bit OS** | Linux x86-64 with glibc 2.34 or later (Ubuntu 22.04+, Debian 12+, Fedora 35+) · macOS on Apple silicon · Windows 10/11 x64 |

Nothing else is required. The interface, fonts, maths renderer and diagram renderer are
compiled into the binary, so no runtime, package manager or network connection is needed to
run it.

## Install

Download the archive for your platform from
[Releases](https://github.com/singhbal-baljinder/formicaria/releases), unpack it, and run
`fm-serve`:

```sh
tar xzf formicaria-*-linux-x86_64.tar.gz && cd formicaria-*
./fm-serve                    # serves http://127.0.0.1:8765
```

Each archive contains per-OS notes in its own README; macOS quarantines unsigned downloads
and requires one `xattr` command. `fm` must stay in the same directory as `fm-serve`: it is
the binary git invokes to merge notes, and the merge driver is not installed if it cannot
be found.

<details>
<summary>Building from source</summary>

The toolchain is pinned with [pixi](https://pixi.sh); no other prerequisites.

```sh
pixi run serve      # build and serve; editing a .svelte file needs no Rust rebuild
pixi run build      # → target/release/{fm-serve,fm}
```
</details>

## Features

The following require only a browser.

| Feature | Description |
|---|---|
| **Notes** | Markdown with YAML frontmatter, one note per file. Editing in the application or in an external editor round-trips byte for byte. |
| **Tasks and scheduling** | Any note accepts `start` and `due` properties, each with an optional time, so all-day deadlines and timed meetings use the same field. |
| **Board** | Kanban grouped by any property, not only status. Dragging a card sets that property and its position in the column. |
| **Agenda** | Month and week calendar drawing a bar from `start` to `due`, plus a list grouped by urgency. |
| **Timeline** | Notes ordered as a journal by day. |
| **Search** | Full-text search over all notes via SQLite FTS5, with case and accent folding. |
| **Note references** | `[Title](note:…)` renders as a live title chip; following one opens the target in a pane alongside the source. |
| **Whiteboards** | A note whose body is an Excalidraw scene. It remains a note, so it appears on the board and in the agenda like any other. |
| **Maths and diagrams** | KaTeX and Mermaid are bundled; `$…$` and ` ```mermaid ` blocks render without network access. |
| **Media** | Images, PDFs and video are stored once by content hash, referenced from notes and displayed inline. |

### Features requiring additional software

These are optional. When a tool is absent the corresponding feature is unavailable and the
application reports it; nothing else is affected. Install them with a system package
manager, [pixi](https://pixi.sh), Homebrew or any other method — formicaria only looks on
`PATH`.

| Feature | Requires | Behaviour without it |
|---|---|---|
| **History** — versions of a note beyond the current session | `git` | Notes remain intact as files; no history is recorded |
| **Backup and sharing** — push a vault to a remote, pull a collaborator's changes | `git`, and a remote | The vault remains local |
| **PDF text search** — text inside a PDF becomes searchable | `pdftotext` (poppler) | The PDF is stored and displayed but not indexed |
| **Thumbnails** — previews for images and PDFs | `vipsthumbnail` (libvips) | A placeholder is shown instead of a preview |
| **Media backup** — encrypted, deduplicated snapshots of blobs | `restic`, a repository per vault, and `RESTIC_PASSWORD` | Notes still back up via git; media remains local |
| **Open in default application** | `xdg-open` — Linux only; macOS and Windows provide this | That action reports an error |

<details>
<summary>Installing the optional tools</summary>

```sh
# Debian / Ubuntu
sudo apt install git poppler-utils libvips-tools restic
# macOS
brew install git poppler vips restic
# any OS, via pixi
pixi global install git poppler libvips restic
```

Windows: `winget install Git.Git`; the remainder through [pixi](https://pixi.sh).
</details>

## Vaults

A vault is a directory: notes in `vault/notes/*.md`, media in `vault/blobs/` addressed by
content hash. Select one with `FM_VAULT`:

```sh
FM_VAULT=~/notes ./fm-serve
```

Multiple vaults are configured in a file — `~/.config/formicaria/vaults.json` on Linux,
`~/Library/Application Support/formicaria/vaults.json` on macOS,
`%APPDATA%\formicaria\vaults.json` on Windows:

```json
{
  "vaults": [
    { "name": "personal", "path": "~/notes" },
    { "name": "lab",      "path": "~/lab-notes", "restic": "/backups/lab" }
  ]
}
```

The first entry receives new notes. Each vault is an independent git repository with its
own collaborators.

### Configuration

| Variable | Default | Purpose |
|---|---|---|
| `FM_VAULT` | *(none — the app asks on first run)* | Vault directory, when no vault list is configured |
| `FM_VAULTS` | per-OS config path | Location of the vault list |
| `FM_ADDR` | `127.0.0.1:8765` | Address to bind |
| `FM_OPEN` | unset | Open the browser on start |
| `FM_UI_DIST` | unset (uses the embedded interface) | Serve the interface from a directory instead |
| `RESTIC_PASSWORD` | unset | Password for the restic repositories |

## Sharing

Sharing a vault means giving its git repository a remote: the people who can clone it are
the people who can read it. Access is determined by which repository a note is in, not by
any field inside the file — a value in a file can be mistyped, and git history is permanent.

Concurrent edits to different parts of one note merge without conflict. This requires a
frontmatter-aware merge driver, because the file format rewrites `updated:` on every save:
without it, any two concurrent edits collide on that line, inside the YAML, where the
parser rejects the result. Genuine conflicts are placed in the note's body, so the note
still opens and can be resolved in the editor.

The server binds `127.0.0.1` and has no authentication. It is not designed to be exposed to
a network.

## Documentation

The manual is an mdBook under [`docs/`](docs/) — build with `pixi run docs`; start at
`docs/src/introduction.md`. The design specification is
[`formicaria/MASTERPLAN.md`](formicaria/MASTERPLAN.md).
[`docs/context/`](docs/context/README.md) holds maintainer notes: current state, the
reasoning behind design decisions, and known gaps.

## Development

```sh
pixi run ci        # test + test-ui + check-ui + deny + checks + docs — the single gate
```

**`pixi run ci` locally *is* the gate.** Every GitHub workflow here — `ci`, `cross`, `docs`,
`release` — is `workflow_dispatch:` only: nothing runs on a push, and a `v*` tag publishes
nothing until someone presses Run workflow. That is deliberate, not an oversight.

```text
crates/  fm-model · fm-query · fm-core · fm-app · fm-serve · fm-cli
ui/      Svelte 5 + Vite, compiled into the binary at build time
docs/    mdBook manual
vault/   notes (a separate git repository; ignored by this one)
```

## Licence

[MIT](LICENSE). Release archives also contain `THIRD-PARTY.md`, listing the licences of the
libraries compiled into the binaries.
