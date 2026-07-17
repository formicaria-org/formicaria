<p align="center">
  <img src="docs/assets/formicaria.svg" alt="Three ant colonies, each nest a labyrinth of galleries and corridors, every one linked to every other by braided trails, with ants moving both ways along all of it" width="820">
</p>

<h1 align="center">formicaria</h1>

<p align="center">
  <em>Knowledge, task scheduling and collaboration — on files you own.</em>
</p>

---

A *formicarium* is one colony's nest. The plural is the architecture: a **set of vaults**,
one per audience — your own notes, the lab's, a paper with a collaborator — each its own
repository with its own people, coordinating through git rather than through anyone's
server.

**Your notes are Markdown files you own.** One note per file, plain text, on your disk,
readable in any editor with this app nowhere in sight. Knowledge, scheduling and
collaboration are not three subsystems; they are three views of one file. A task is a note
with a `due`. A shared note is a note in a different repository.

## Install

Download the archive for your platform from
[**Releases**](https://github.com/singhbal-baljinder/formicaria/releases), unpack it, and
run `fm-serve`. There is no installer and no setup step.

```sh
tar xzf formicaria-*-linux-x86_64.tar.gz && cd formicaria-*
./fm-serve                    # → open http://127.0.0.1:8765
```

Each archive carries its own README with per-OS notes (macOS quarantines unsigned
downloads and needs one `xattr` command). **Keep `fm` beside `fm-serve`** — the second
binary is what git calls to merge notes, and separating them costs you clean merges.

<details>
<summary>Build from source instead</summary>

Everything is pinned through [pixi](https://pixi.sh); nothing else needs installing.

```sh
pixi run serve      # build + serve (a .svelte edit needs no Rust rebuild)
pixi run build      # → target/release/{fm-serve,fm}; then run fm-serve
```
</details>

## Requirements

formicaria runs as a local web app: a small server on your machine, and your browser as
the window. So there are two requirements, and no more.

| | |
|---|---|
| **A web browser** | Any current Firefox, Chrome, Safari or Edge. This *is* the interface — there is no separate desktop window. |
| **A 64-bit OS** | **Linux** x86-64 with glibc 2.34+ (Ubuntu 22.04+, Debian 12+, Fedora 35+) · **macOS** on Apple silicon · **Windows** 10/11 x64 |

That is the whole list. No runtime to install, no account, and no network connection — the
interface, the fonts, the maths and the diagram renderer are all inside the binary. Your
vault is a folder.

## Features

Everything below works with nothing but a browser.

| Feature | What it does |
|---|---|
| **Notes** | Markdown with YAML frontmatter, one note per file. Edit here or in Vim — the bytes round-trip either way. |
| **Tasks & scheduling** | Any note takes a `start` and a `due`, each with an optional time, so an all-day deadline and a 15:00 meeting are the same field. |
| **Board** | Kanban that groups by *any* property, not just status. Drag a card to set that property — and to a position within the column. |
| **Agenda** | Month/week calendar drawing a bar from `start` to `due`, plus a list grouped by urgency. |
| **Timeline** | Your notes as a journal, by day. |
| **Search** | Full-text over every note, instantly — SQLite FTS5, case- and accent-folding. |
| **Note links** | `[Title](note:…)` renders as a live title chip; clicking it opens the target as a pane alongside, so the trail you followed stays on screen. |
| **Whiteboards** | A note whose body is an Excalidraw scene — a first-class note that happens to draw, so it appears on the board and in the agenda like any other. |
| **Maths & diagrams** | KaTeX and Mermaid, bundled: `$…$` and ` ```mermaid ` blocks render offline. |
| **Media** | Drop in an image, PDF or video — stored once by content hash, referenced from the note, shown inline. |

### Features that need something installed

Each of these is optional. If its tool is not on your `PATH`, that feature is unavailable
and the app says so; nothing else is affected. Install them with your system package
manager, [pixi](https://pixi.sh), brew — however you prefer.

| Feature | Needs | Without it |
|---|---|---|
| **History** — undo that outlives the session, and every past version of a note | `git` | Notes are still safe (they are files); there is just no history to go back to |
| **Backup & sharing** — push a vault to a remote, pull a collaborator's work | `git`, and a remote you can push to | The vault stays on this machine |
| **PDF search** — the text *inside* a PDF becomes findable | `pdftotext` (poppler) | The PDF is still stored and displayed, just not searchable |
| **Thumbnails** — previews for images and PDFs | `vipsthumbnail` (libvips) | A placeholder where the preview would be |
| **Media backup** — encrypted, deduplicated snapshots of your blobs | `restic`, a repo per vault, and `RESTIC_PASSWORD` | Notes still back up over git; media stays local |
| **"Open in default app"** | `xdg-open` — **Linux only**; macOS and Windows have this built in | That one button errors |
| **Whiteboard hand-drawn fonts** | An internet connection (Excalidraw fetches them) | Whiteboards work, in system fonts |

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

Windows: `winget install Git.Git`, and the rest through [pixi](https://pixi.sh).
</details>

## Your notes

A vault is a directory: notes in `vault/notes/*.md`, media in `vault/blobs/` addressed by
hash. Point the app at one with `FM_VAULT`:

```sh
FM_VAULT=~/notes ./fm-serve
```

Several vaults — one per audience — go in a config file (`~/.config/formicaria/vaults.json`
on Linux; `~/Library/Application Support/formicaria/` on macOS; `%APPDATA%\formicaria\` on
Windows):

```json
{
  "vaults": [
    { "name": "personal", "path": "~/notes" },
    { "name": "lab",      "path": "~/lab-notes", "restic": "/backups/lab" }
  ]
}
```

The first is where new notes go. Each vault is its own git repository, with its own people.

## How sharing works

Give a vault a remote and it is shared: the people who can clone it are exactly the people
who can read it. **Where a note lives decides who can see it** — never a label inside the
file, because a label you can type is a label you can typo, and git history is permanent.

Two people editing *different paragraphs of the same note* merge cleanly. That sounds
unremarkable and is not: the format rewrites `updated:` on every save, so any two
concurrent edits collide on that line, inside the frontmatter, where the parser refuses
them. A frontmatter-aware merge driver resolves the collision the format itself
manufactures. When a conflict is genuine it lands in the note's *body*, so the note still
opens and you settle it in the editor.

The server binds `127.0.0.1` and has no authentication, because it was never meant to need
any — do not expose it to a network. No account, no cloud, nothing phones home.

## Documentation

The manual is an mdBook under [`docs/`](docs/) — `pixi run docs`, then start at
`docs/src/introduction.md`. The design spec is
[`formicaria/MASTERPLAN.md`](formicaria/MASTERPLAN.md).

## Development

```sh
pixi run ci        # test + test-ui + deny + checks + docs — the single gate
```

CI runs the gate on Linux, macOS and Windows whenever code changes; a `v*` tag builds and
publishes all three. [`docs/context/`](docs/context/README.md) is the maintainers' working
memory: what exists, why, and what does not.

```text
crates/  fm-model · fm-query · fm-core · fm-app · fm-serve · fm-cli
ui/      Svelte 5 + Vite (baked into the binary at build time)
docs/    the mdBook manual
vault/   your notes (its own git repo; ignored by this one)
```

## Licence

[MIT](LICENSE). Use it, change it, sell it, close it — keep the notice, that is all.
Release archives also carry `THIRD-PARTY.md`, the notices of the libraries built into the
binary.
