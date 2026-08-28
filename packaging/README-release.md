# formicaria

A local-first research notebook. Your notes are **Markdown files you own** — one note per
file, plain text, readable without this app, on disk, forever. Knowledge, task scheduling
and collaboration are three views of the same file.

This archive contains the whole application.

---

## Run it

There is no installer. Unpack the archive, then double-click the launcher inside it. It starts a
small server on your own machine and opens your browser — that *is* the app. Nothing is installed,
nothing is written outside this folder, and nothing is sent anywhere.

**Unpack it first.** Running from inside a zip preview gives you a temporary folder that Windows
or macOS deletes afterwards — and takes your notes with it.

| | Double-click |
|---|---|
| **Windows** | `formicaria.vbs` — starts it with no console window. Use `formicaria.bat` instead if you want to see the messages, or if scripting is disabled on your machine. |
| **macOS** | `Formicaria.command` — **right-click it and choose Open the first time**, then click "Open" in the dialog. A Terminal window stays open beside the app; that is normal, and closing it quits. |
| **Linux** | `formicaria.sh`. Some file managers ask what to do with a script — choose "Run". From a terminal: `./formicaria.sh`. |

Your browser opens on its own. To stop, close the browser tab — the app quits a few seconds later.

> **Windows and macOS will warn you.** These binaries are not signed by Microsoft or Apple, which
> costs money we have not spent, so you get "Windows protected your PC" (click **More info → Run
> anyway**) or a Gatekeeper dialog (right-click → **Open**). The warning is about the absence of a
> paid signature, not about anything found in the file.

> **Keep `fm` and `fm-serve` together.** They must stay in the same folder. `fm` is the
> command-line tool, but it is *also* what git calls to merge notes — if it isn't beside
> `fm-serve`, the note merge driver is silently not installed, and two people editing one
> note will conflict on every save instead of merging cleanly.

---

## Learn to use it

The full manual is in this archive. Open **`manual/index.html`** in your browser. It needs no
network and no account — it reads the same whether or not you are online. Unpack the archive
first: opened from inside a zip preview, the page appears without its styling or its search.

It covers everything this sheet does not: views, panes and boards; writing notes, links and
backlinks, templates and checkboxes; images, PDFs and whiteboards; search; backup; sharing a
vault and working through proposals; and the optional on-device assistant. There is a reference
section too — every command, and every frontmatter field.

Prefer plain text? The same pages are Markdown in **`manual/source/`**, readable in any editor.
The manual is files you own, exactly like your notes.

---

## Where your notes live

**In the `vault` folder inside this one.** The launcher points formicaria at it explicitly, so
everything — your notes and the list of your vaults — stays in this folder and nowhere else.

That is what makes this a portable app: copy the whole folder to a USB stick or another computer
and your notes travel with it. Nothing is left behind in your home directory.

Your notes are `vault/notes/*.md`. Plain Markdown files, one per note. Back them up, sync them,
open them in any editor — they are yours, and this app is not required to read them.

To keep notes somewhere else instead, set `FM_VAULT` before starting — but then you are choosing
the location yourself, and the folder stops being portable:

```sh
FM_VAULT=/path/to/my/notes ./fm-serve        # Linux/macOS
$env:FM_VAULT="C:\path\to\notes"; .\fm-serve.exe   # Windows PowerShell
```

Several vaults — one per audience, say personal and lab — are listed in `vaults.json` beside the
app. The first is the default for new notes. **Where a note lives decides who can see it**, which
is not something you can mistype into a file.

---

## Requirements

Two things, and no more:

- **A web browser** — any current Firefox, Chrome, Safari or Edge. This *is* the interface;
  there is no separate desktop window.
- **A 64-bit OS** — Linux x86-64 with glibc 2.34+ (Ubuntu 22.04+), macOS on Apple silicon,
  or Windows 10/11 x64.

No runtime, no account, no network: the interface, its fonts, the maths renderer and the
diagram renderer are all inside the binary. Whiteboards ship their own fonts too, so
drawing never reaches the internet. (Chinese and Japanese text in a whiteboard falls back
to a system font — the CJK font is 13 MB and is not bundled.)

## Optional features

The notebook — writing, tasks, dates, board, agenda, search — needs nothing beyond the two
above. It works on a machine with no tools installed at all.

Everything below is a *feature*. If its tool isn't on your `PATH`, that feature simply
isn't available and the app says so. It never breaks the notebook. Install them however
you like — your system package manager, [pixi](https://pixi.sh), brew, whatever.

| Install | To get | Without it |
|---|---|---|
| **git** | History — undo that outlives the session — plus backup and sharing a vault | Notes are still safe (they are files); no history to go back to |
| **pdftotext** (poppler) | The text inside a PDF becomes searchable | The PDF is still stored and shown, just not searchable |
| **vipsthumbnail** (libvips) | Thumbnails for images and PDFs | A placeholder where the preview would be |
| **restic** | Encrypted, deduplicated backup of your media (needs `RESTIC_PASSWORD`, and a `restic` path per vault above) | Notes still back up over git; media stays local |
| **xdg-open** (Linux only) | The "open in default app" button | That one button errors |

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

MIT licensed — see `LICENSE`. `THIRD-PARTY.md` lists the libraries built into these
binaries and their notices. The manual is `manual/` — open `manual/index.html`, or read its
Markdown sources in `manual/source/`.
Source: <https://github.com/singhbal-baljinder/formicaria>
