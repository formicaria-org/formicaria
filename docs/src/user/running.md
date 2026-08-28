# Starting formicaria

formicaria runs in your browser, served by a small program on your own machine. There is no
installer, no account and no cloud. Nothing you write leaves this computer unless you ask it to.

## Unpack, then double-click

Download the archive for your system and **unpack it somewhere real** — your Documents or Desktop.
Opening it straight from inside the zip gives you a temporary folder that gets deleted afterwards,
along with anything you wrote.

Then double-click the launcher:

| | Double-click |
|---|---|
| **Windows** | `formicaria.vbs` — starts with no console window. `formicaria.bat` does the same thing with the messages visible. |
| **macOS** | `Formicaria.command` — **right-click and choose Open the first time**, then click "Open". A Terminal window stays open beside the app; closing it quits. |
| **Linux** | `formicaria.sh`. If your file manager asks what to do with a script, choose "Run". |

Your browser opens by itself at `http://127.0.0.1:8765`. That address is your own computer — the
`127.0.0.1` part means it goes nowhere else.

**To stop it, close the browser tab.** The app quits a few seconds later.

### The warning you will see

Windows and macOS both warn about software that has not been signed by Microsoft or Apple. These
builds are not signed, so you get "Windows protected your PC" (choose **More info → Run anyway**)
or a Gatekeeper refusal (right-click → **Open**). The warning is about the missing signature, not
about anything found in the files.

## Where your notes go

In the `vault` folder inside the one you unpacked. Nothing is written to your home directory, and
nothing is installed — which means the whole folder is portable: copy it to a USB stick or another
computer and your notes come along.

Your notes are `vault/notes/*.md` — one plain Markdown file per note, readable in any editor, with
or without this app.

## If it does not start

**"Port 8765 is already being used."** Something else on your computer has that address.
formicaria tells you so and stops rather than guessing. If it is another copy of formicaria, it
will simply open your browser at the copy already running.

**Nothing happens when you double-click.** On Linux, some file managers open scripts in a text
editor instead of running them; open a terminal in the folder and run `./formicaria.sh`. On
Windows, try `formicaria.bat`, which shows what went wrong instead of hiding it.

**The page loads but is empty.** Close the tab and start the app again. If it persists, the
`Skipped` panel in the app lists any notes that could not be read.

## Running it a different way

You do not need any of this to use formicaria; it is here for people who want it.

Set `FM_VAULT` to keep your notes somewhere other than the app folder — but then you have chosen
the location, and the folder is no longer portable:

```sh
FM_VAULT=/path/to/my/notes ./fm-serve
```

Other settings, all optional:

| Variable | What it does |
|---|---|
| `FM_VAULT` | Where the notes live, when you would rather choose than use the folder beside the app. |
| `FM_VAULTS` | Where the list of vaults is stored. The launcher points it beside the app. |
| `FM_ADDR` | The address to listen on. Use it if `8765` is taken: `FM_ADDR=127.0.0.1:8788`. |
| `FM_OPEN` | Open the browser at startup. The launcher sets it. |
| `FM_AUTO_SHUTDOWN` | Quit when the browser tab closes. The launcher sets it. |
| `FM_RESTIC_REPO`, `RESTIC_PASSWORD` | Media backup — see [Backup & versioning](./backup.md). |

Building and running from the source code is a different job with different tools; it is covered in
the developer guide under [Testing & CI](../dev/testing.md).
