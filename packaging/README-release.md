# formicaria

A local-first research notebook. Your notes are **Markdown files you own** — one note per
file, plain text, readable without this app, on disk, forever. Knowledge, task scheduling
and collaboration are three views of the same file.

This archive contains the whole application.

---

## Run it

There is no installer and no setup step. Unpack, and run `fm-serve`. It starts a small
server on your own machine and opens your browser — that is the app.

**Linux**
```sh
tar xzf formicaria-*-linux-x86_64.tar.gz
cd formicaria-*
./fm-serve
```

**macOS**
```sh
tar xzf formicaria-*-macos-arm64.tar.gz
cd formicaria-*
xattr -dr com.apple.quarantine .    # macOS quarantines downloads; this is not signed
./fm-serve
```

**Windows**
```powershell
# Unzip, then from that folder:
.\fm-serve.exe
```

Then open <http://127.0.0.1:8765>. Stop it with Ctrl-C.

> **Keep `fm` and `fm-serve` together.** They must stay in the same folder. `fm` is the
> command-line tool, but it is *also* what git calls to merge notes — if it isn't beside
> `fm-serve`, the note merge driver is silently not installed, and two people editing one
> note will conflict on every save instead of merging cleanly.

---

## Where your notes live

By default, a `vault` folder next to wherever you ran `fm-serve` from. To choose:

```sh
FM_VAULT=/path/to/my/notes ./fm-serve        # Linux/macOS
$env:FM_VAULT="C:\path\to\notes"; .\fm-serve.exe   # Windows PowerShell
```

A vault is just a directory. Your notes are `vault/notes/*.md`. Back it up, sync it, open
it in Vim — it's yours, and this app is not required to read it.

Several vaults (one per audience — personal, lab, a paper with someone) go in a config
file. **Linux** `~/.config/formicaria/vaults.json`, **macOS** `~/Library/Application
Support/formicaria/vaults.json`, **Windows** `%APPDATA%\formicaria\vaults.json`:

```json
{
  "vaults": [
    { "name": "personal", "path": "~/notes" },
    { "name": "lab",      "path": "~/lab-notes", "restic": "/backups/lab" }
  ]
}
```

The first is the default for new notes. Each vault is its own git repo with its own
collaborators — which is the point: **where a note lives decides who can see it**, and
that is not something you can mistype into a file.

---

## Optional features

The notebook — writing, tasks, dates, board, agenda, search — needs **nothing installed**.
It works out of the box on a machine with no tools at all.

Everything below is a *feature*. If its tool isn't on your `PATH`, that feature simply
isn't available and the app says so. It never breaks the notebook. Install them however
you like — your system package manager, [pixi](https://pixi.sh), brew, whatever.

| Install | To get |
|---|---|
| **git** | History (undo that outlives the session), backup, and sharing a vault with other people |
| **pdftotext** (poppler) | The text inside a PDF you drop in becomes searchable |
| **vipsthumbnail** (libvips) | Thumbnails for images and PDFs |
| **restic** | Encrypted, deduplicated backup of your media (needs `RESTIC_PASSWORD` set, and a `restic` path per vault in the config above) |

Linux: `apt install git poppler-utils libvips-tools restic` ·
macOS: `brew install git poppler vips restic` ·
Windows: `winget install Git.Git` (and the rest via [pixi](https://pixi.sh) or your
preferred manager)

**git is the one worth having.** Without it your notes are still safe — they're files —
but there is no history to go back to.

---

## What this is not

No account, no cloud, no telemetry, nothing phones home. The server binds `127.0.0.1` and
is for you alone; do not expose it to a network — it has no authentication, because it was
never meant to need any.

---

Licensed MIT OR Apache-2.0. Source: <https://github.com/singhbal-baljinder/formicaria>
