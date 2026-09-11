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

A vault is an ordinary directory of files, and becomes a git repository wherever git is
available. Running several — one per audience, such as personal
notes, a lab's notes, and a paper shared with a collaborator — is the normal case, and is
what the name refers to: a *formicarium* is one colony's nest, and *formicaria* is the
plural.

Notes, tasks and shared documents are not separate systems. A task is a note with a `due`
property; a shared note is a note in a different repository.

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
| **History** — versions of a note beyond the current session | `git` on Linux and macOS; **nothing on Windows**, which carries its own | Notes remain intact as files; no history is recorded |
| **Backup and sharing** — send a vault somewhere else, get a collaborator's changes | a remote, and `git` except on Windows | The vault remains local |
| **PDF text search** — text inside a PDF becomes searchable | `pdftotext` (poppler) | The PDF is stored and displayed but not indexed |
| **Thumbnails** — previews for images and PDFs | `vipsthumbnail` (libvips) | A placeholder is shown instead of a preview |
| **Media backup** — encrypted, deduplicated snapshots of your notes *and* their attachments | `restic`, and a repository per vault (the password is set in the app; `RESTIC_PASSWORD` overrides it) | Notes still back up on their own; media remains local |
| **Open in default application** | `xdg-open` — Linux only; macOS and Windows provide this | That action reports an error |
| **Study assistant** — a local model answering and drafting in your notes | nothing: it fetches its own runtime and model on first enable. Exercised on Linux and Android; **on macOS and Windows this is compiled and type-checked but has never been run** | Settings shows the reason instead of a switch |
| **Reading images** — `/transcribe` on a photographed page | a model with a projector, offered as a choice at first enable | The assistant says it cannot see pictures rather than guessing at one |
| **Transcribing recordings** — `/transcribe` on audio | nothing on Linux or Windows: a further ~170 MB it fetches when you turn the switch on (the Windows path is untried, like the rest of Windows). **No macOS build exists upstream** | The switch says no runtime is published for this platform |

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

Windows: through [pixi](https://pixi.sh). (Not git — the Windows build carries its own.)
</details>

## Requirements

| | |
|---|---|
| **A web browser** | Any current Firefox, Chrome, Safari or Edge. This is the interface; there is no separate desktop window. |
| **A 64-bit OS** | Linux x86-64 with glibc 2.34 or later (Ubuntu 22.04+, Debian 12+, Fedora 35+) · macOS on Apple silicon · Windows 10/11 x64 |

Nothing else is required. The interface, fonts, maths renderer and diagram renderer are
compiled into the binary, so no runtime, package manager or network connection is needed to
run it.

**How far each platform is proven, plainly.** Linux is what this is written and used on daily.
**macOS and Windows have been run by other people, who report download, setup and note-taking
working** — but on earlier releases, not this one, and not checked on this end: there is no Mac and
no Windows machine here. The reports come with real problems attached. See
[Project status](#project-status).

## Install

Download the archive for your platform from
[Releases](https://github.com/formicaria-org/formicaria/releases) and unpack it. Then
double-click the launcher for your system — `Start formicaria.vbs` on Windows,
`Start formicaria.command` on macOS, `Start formicaria.sh` on Linux. Your browser opens by
itself; closing the tab stops the app.

**On a phone**, the same page carries an Android `.apk` and an iPhone `.ipa`, built by the same
release. Both need a little more than a double-click, and each has a chapter:
[Android](docs/src/user/android.md) · [iPhone](docs/src/user/iphone.md) — the iPhone one is not
optional reading, because Apple's free signing expires after seven days.

The archive carries a `README.txt` and the whole manual (`Manual.html`), including what to do
about the security warning your OS shows for unsigned software — **on macOS that is now System
Settings → Privacy & Security → Open Anyway**, not the old right-click.

From a terminal, the binaries are under `program/`:

```sh
tar xzf formicaria-*-linux-x86_64.tar.gz && cd formicaria-*
./program/fm-serve            # serves http://127.0.0.1:8765
```

`fm` must stay in the same directory as `fm-serve`: it is the binary git invokes to merge
notes, and the merge driver is not installed if it cannot be found.

<details>
<summary>Building from source</summary>

The toolchain is pinned with [pixi](https://pixi.sh); no other prerequisites.

```sh
pixi run serve      # build the UI, then serve it against ./vault
pixi run build      # → target/release/{fm-serve,fm}
```
</details>

## Study assistant (optional, local AI)

formicaria can run a small **local AI assistant** that answers in a note's discussion, researches the
web, and drafts note edits you review. It is **off by default**, runs **entirely on your device** (no
account, no API key, no cloud — your notes never leave the machine), and is fully removable.

**Users do not need any of this.** Since 2026-09-02 the app installs the assistant itself: turn it
on in Settings, it asks which model and states the size and licence, and downloads the runtime and
weights with progress and a cancel. Before starting a model it checks it can read how much memory
the machine has free and refuses if it cannot — it will not run a model it cannot watch.

**Observed on Linux and Android; inferred on macOS and Windows.** The per-OS memory readings that
gate it were written on a Linux machine that cannot compile them, so they are type-checked in CI
(`pixi run -e cross check-cross`) and *run* only by the `cross` workflow. Until that has run,
"the assistant works on macOS" is an inference — and this README would rather say so than let you
find out.

The commands below are the **developer** route: they put the same pieces in a checkout's `agents/`,
which the app prefers when it is there, so the dev loop needs no download.

```sh
pixi run fetch-model lfm2.5-1.2b   # download a local model into agents/ (gitignored)
pixi run fetch-whisper             # the audio runtime, staged by hand; the app fetches it itself
pixi run search-proxy              # optional: a local proxy; the app itself searches in-process
# then turn it on in Settings → Study assistant, and in any discussion:
#   @lfm2.5-1.2b summarize this in 3 bullets /search
```

It can only **reply** or **propose a change on a review branch** — never write to `main`. Full guide:
[the study assistant](docs/src/user/assistant.md).

## Android

formicaria also runs on Android, with the assistant on-device. Build a signed APK and install it:

```sh
pixi run android-init      # fetch the Android toolchain into ./.android (one time)
pixi run android-release   # → a signed, 16 KB-aligned APK at mobile/formicaria-<abi>.apk
```

Install it with `adb install -r mobile/formicaria-*.apk` (or copy it to the phone and open it). The
assistant downloads its model on first enable and then runs offline. Built `--no-default-features`,
the app contains none of the assistant — a lighter notes-only build.

## Vaults

A vault is a directory: notes in `vault/notes/*.md`, media in `vault/blobs/` addressed by
content hash. Select one with `FM_VAULT`:

```sh
FM_VAULT=~/notes ./program/fm-serve
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
| `FM_CONFIG_DIR` | per-OS config path | Directory the vault list and the restic password live in |
| `FM_ADDR` | `127.0.0.1:8765` | Address to bind |
| `FM_OPEN` | **on** | Open the browser on start. Set to `0` to keep it closed |
| `FM_UI_DIST` | unset (uses the embedded interface) | Serve the interface from a directory instead |
| `FM_AUTO_SHUTDOWN` | on | Closing the browser tab stops the app. Set to `0` to keep it running |
| `FM_RESTIC_REPO` | unset | Restic repository for a single-vault install (a vault list uses its own `restic` field) |
| `RESTIC_PASSWORD` | unset | Password for the restic repositories |
| `FM_GIT_TOKEN` | unset | Git token, read only where there is no `git` binary and so no credential helper — the phone, and Windows without git installed |

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
`docs/src/introduction.md`. New to the code? [`docs/src/dev/getting-started.md`](docs/src/dev/getting-started.md).

[`formicaria/MASTERPLAN.md`](formicaria/MASTERPLAN.md) is the **design journal**: where the
shape of the project was argued out, including options that were rejected and rulings that
have since been reversed. It is kept because the reasoning is worth having, not because it
describes the current build — for that, read `docs/context/features.md`.

[`docs/context/`](docs/context/README.md) is the maintainer's working memory — decisions and
their reasons, known gaps, and the queue. These are **unedited working notes, not
documentation**: they are written to be useful to whoever is next in the code, they contradict
each other across dates on purpose (`decisions.md` is append-only, so a reversal sits beside
what it reversed), and they are not a description of how to use the app. The manual is.

## Project status

**v0.5.5, and young.** The first commit is dated 2026-07-14; this is one person's project,
built alongside their research rather than as a product. It is used daily on **Linux and
Android**, the two platforms the author works on and the only two anything here is checked
against.

**The other platforms have been run by other people, and reported working.** macOS on **v0.2.1**:
download and setup "works smoothly", notes and search both used
([#2](https://github.com/formicaria-org/formicaria/issues/2), 2026-08-30) — reported alongside real
faults, including a view that could not be recovered by reloading the page, only by restarting the
app. Windows on **v0.2.0 through v0.3.1**, and iOS, both reported working as of 2026-09-09.

**None of it is verified here, and none of it is this version.** There is no Mac, no Windows
machine and no iPhone on this end, so a report is one person's session rather than a suite. And
every report above is against a **v0.2.x or v0.3.x** archive: **nobody has downloaded anything newer** — v0.4.0 and v0.5.0 show zero, and every download of
v0.5.1 and v0.5.2 is this project's own, fetched on 2026-09-11 to test the updater against real
releases rather than hand-built ones — so the current release is exactly as unproven off Linux and Android as
the ones before it were. The builds are compiled and tested by CI and the archives
are real; that is a different claim from someone having opened one.

Treat the feature tables above as *what is implemented*, not as *what is proven on your
machine*. The honest gap list is [`docs/context/known-issues.md`](docs/context/known-issues.md),
kept as working notes rather than marketing; the current state of each feature is
[`docs/context/features.md`](docs/context/features.md).

## Development

```sh
pixi run ci        # test + test-agent-download + test-ui + check-ui + deny + checks
                   # + third-party-check + docs — the single gate
```

**`pixi run ci` locally *is* the gate.** Four of the five GitHub workflows — `ci`, `cross`,
`docs` and `ios` — are `workflow_dispatch:` only: nothing runs on a push or a pull request.
That was originally about money — the repo was private, where Actions minutes are billed, at 10x
on macOS. **That reason is gone**: this repo is public, and GitHub does not charge for standard
runners in public repositories. What keeps the triggers manual now is that the gate runs here, on
one machine, before anything is pushed — a second opinion that fires on every push tells you what
`pixi run ci` already told you.

**The exception is `release.yml`, which fires unattended on a `v*` tag.** Pushing a tag
publishes a release. It is the one workflow you can start by accident.

```text
crates/  fm-model · fm-query · fm-core · fm-app · fm-serve · fm-cli
         fm-agent · fm-agent-run   (the study assistant; optional features)
ui/      Svelte 5 + Vite, compiled into the binary at build time
docs/    mdBook manual
vault/   notes (a separate git repository; ignored by this one)
```

## Contributing

[`CONTRIBUTING.md`](CONTRIBUTING.md) — the pixi-only toolchain, `pixi run ci` as the single gate,
the invariants a change must not break, the **four questions** to ask before adding anything, and
an honest list of what is deliberately *not* wanted. [`SECURITY.md`](SECURITY.md) has the private
route for reporting a vulnerability and a plainly stated threat model.

## Licence

[MIT](LICENSE). [`THIRD-PARTY.md`](THIRD-PARTY.md) lists the licences of everything the
binaries carry — the Rust crates they link, the npm packages baked into the bundled user
interface, and the font families bundled with the whiteboard. It is generated by
`ci/third-party.sh`, checked for staleness by `pixi run ci`, and copied into every release
archive beside the binaries it describes.
