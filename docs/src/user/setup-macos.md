# Set up formicaria on macOS

formicaria runs in your browser, from a folder on your own computer. Nothing is installed, there is
no account, and nothing you write leaves this machine.

## 1. Unpack it

Double-click the file you downloaded — `formicaria-…-macos-arm64.tar.gz` — and drag the folder it
produces somewhere real, such as your **Documents** folder.

## 2. Start it

Open the folder and **right-click** (or Control-click):

**`Start formicaria.command`** → **Open** → **Open** again in the dialog.

**The first time you must right-click and choose Open — a double-click will be refused.** macOS
blocks software that carries no paid signature from Apple, and right-clicking is how you tell it you
meant to. After the first time, double-click works normally.

Your browser opens by itself. That is the app — you are ready to write.

A black Terminal window stays open beside it. That is normal; it is how the app runs.

## 3. Stop it

Close the browser tab. formicaria shuts down a few seconds later. (Closing the Terminal window
also stops it.)

## Where your notes are kept

In the **`vault`** folder, inside the folder you unpacked.

That is what makes this portable: copy the whole folder to a USB stick or another Mac and your notes
travel with it. Nothing is left behind on the machine you were using.

Your notes are the files in `vault/notes/` — one plain text file per note, which you can open in any
editor. This app is not needed to read them.

## What else is in the folder

Only three things are for you:

- **`Start formicaria.command`** — starts it.
- **`Manual.html`** — this manual.
- **`vault`** — your notes.

`program` holds the application itself and `manual` holds this manual's pages. You never need to
open either.

## If something goes wrong

**"cannot be opened because it is from an unidentified developer."** You double-clicked instead of
right-clicking. Right-click the file, choose **Open**, then **Open** in the dialog.

**It says the port is already being used.** Another program is using the address formicaria wants.
If it is another copy of formicaria, it will simply open your browser at the copy already running.

**A blank page.** Close the tab and start it again.

---

Next: [Your first ten minutes](./first-note.md).
