# Set up formicaria on macOS

formicaria runs in your browser, from a folder on your own computer. Nothing is installed, there is
no account, and nothing you write leaves this machine.

## 1. Unpack it

Double-click the file you downloaded — `formicaria-…-macos-arm64.tar.gz` — and drag the folder it
produces somewhere real, such as your **Documents** folder.

## 2. Let macOS know you meant it

The first time, macOS refuses to open formicaria and offers you only **Done** and
**Move to Trash**.

> **Do not click "Move to Trash".** Nothing is wrong with the files. macOS is saying it cannot see
> who wrote this software, because Apple charges $99 a year to sign it and we have not paid.

1. Double-click **`Start formicaria.command`**. Click **Done**.
2. Open the **Apple menu** → **System Settings** → **Privacy & Security**.
3. Scroll to the bottom. A line says formicaria was blocked. Click **Open Anyway** beside it.
4. Confirm with Touch ID or your Mac password.
5. Double-click **`Start formicaria.command`** again, then click **Open**.

That is once, ever. From then on a double-click just works.

Step 3 only appears for a little while after step 1, so do them one after the other.

## 3. Start it

Double-click **`Start formicaria.command`**.

Your browser opens by itself. That is the app — you are ready to write.

A black Terminal window stays open beside it. That is normal; it is how the app runs.

## 4. Stop it, and start it again

Close the browser tab. formicaria shuts down a few seconds later. (Closing the Terminal window
also stops it.)

To come back — tomorrow, or in five minutes — double-click **`Start formicaria.command`** again.
Your notes are still in the `vault` folder, exactly as you left them.

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

**"Apple could not verify … it may contain malware."** This is the block described in step 2. Click
**Done** — never *Move to Trash* — then allow it under **System Settings → Privacy & Security**.

**Older instructions say to right-click and choose Open.** That worked on macOS versions before
Sequoia. Apple removed it; System Settings is the way now.

**It says the port is already being used.** Another program is using the address formicaria wants.
If it is another copy of formicaria, it will simply open your browser at the copy already running.

**A blank page.** Close the tab and start it again.

---

Next: [Your first ten minutes](./first-note.md).
