# Configuration and optional tools

What the app needs, where it keeps things, and the settings that live outside the app.

## A vault on disk

A vault is a directory: notes in `notes/*.md` (one Markdown file each, with YAML frontmatter), media
in `blobs/`, addressed by content hash. It becomes a git repository wherever git is available.

## The vault list

Several vaults (one per audience: personal notes, a lab's notes, a paper shared with a collaborator)
are listed in a file. On Linux it is `~/.config/formicaria/vaults.json`, on macOS
`~/Library/Application Support/formicaria/vaults.json`, and on Windows
`%APPDATA%\formicaria\vaults.json`:

```json
{
  "vaults": [
    { "name": "personal", "path": "~/notes" },
    { "name": "lab",      "path": "~/lab-notes", "restic": "/backups/lab" }
  ]
}
```

The first entry receives new notes. Each vault is its own git repository with its own
collaborators. The app writes this file itself when you add a vault in **Settings → Vaults &
backup**.

## Environment variables

None is needed. They are for running the server by hand.

| Variable | Default | Purpose |
|---|---|---|
| `FM_VAULT` | *(none: the app asks on first run)* | Vault directory, when no vault list is configured |
| `FM_VAULTS` | per-OS config path | Location of the vault list |
| `FM_CONFIG_DIR` | per-OS config path | Directory the vault list and the restic password live in |
| `FM_ADDR` | `127.0.0.1:8765` | Address to bind |
| `FM_OPEN` | on | Open the browser on start; `0` keeps it closed |
| `FM_UI_DIST` | unset (the embedded interface) | Serve the interface from a directory instead |
| `FM_AUTO_SHUTDOWN` | on | Closing the last formicaria tab stops the app, unless a reminder is waiting; `0` keeps it running |
| `FM_RESTIC_REPO` | unset | Restic repository for a single-vault install (a vault list uses its own `restic` field) |
| `RESTIC_PASSWORD` | unset | Password for the restic repositories |
| `FM_GIT_TOKEN` | unset | Git token, read only where there is no `git` binary (the phone, Windows without git) |

The server binds `127.0.0.1` and has no authentication: it is not made to be reached from a network.

## Optional tools

When a tool is absent its feature is unavailable and the app says so; nothing else is affected.
formicaria looks for them on `PATH`.

| Feature | Requires | Without it |
|---|---|---|
| **History**: versions of a note | `git` on Linux and macOS; nothing on Windows, which carries its own | Notes stay intact as files; no history |
| **Backup and sharing** | a remote, and `git` except on Windows | The vault stays local |
| **PDF text search** | `pdftotext` (poppler) | The PDF is stored and shown, not indexed |
| **Thumbnails** | `vipsthumbnail` (libvips) | A placeholder instead of a preview |
| **Media backup**: encrypted snapshots of notes and attachments | `restic`, and a repository per vault | Notes still back up through git; media stays local |
| **Open in the default application** | `xdg-open` on Linux; built in on macOS and Windows | That action reports an error |

```sh
# Debian / Ubuntu
sudo apt install git poppler-utils libvips-tools restic
# macOS
brew install git poppler vips restic
# any OS, via pixi
pixi global install git poppler libvips restic
```

The study assistant, transcription, calendars and mail need nothing installed by hand: the app
fetches what it needs when you turn them on, or walks you through it in their chapters.
