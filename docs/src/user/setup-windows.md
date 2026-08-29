# Set up formicaria on Windows

formicaria runs in your browser, from a folder on your own computer. Nothing is installed, there is
no account, and nothing you write leaves this machine.

## 1. Two warnings, and what to click

Your browser and Windows each stop you once. Both mean the same thing — formicaria carries no paid
signature from Microsoft — and neither is a report of anything found in the file.

**In the browser**, at the bottom of the window: *"… was blocked"* or *"… is not commonly
downloaded"*. Click the **three dots** beside that message, then **Keep**, then **Keep anyway**.

**When you start it**, a blue box: *"Windows protected your PC"*. There is no visible button — click
the small grey **More info** text, then **Run anyway**.

## 2. Unzip it

Right-click the file you downloaded — `formicaria-…-windows-x86_64.zip` — and choose
**Extract All…**. Put it somewhere real, such as your **Documents** folder.

> **Do not skip this step.** Windows will show you what is inside a zip without extracting it, but
> that is a temporary folder it deletes later — and it would take your notes with it.

## 3. Start it

Open the folder you just extracted and double-click:

**`Start formicaria.vbs`**

Your browser opens by itself. That is the app — you are ready to write. (This is where the second
warning above appears, the first time only.)

## 4. Stop it, and start it again

Close the browser tab. formicaria shuts down a few seconds later.

To come back — tomorrow, or in five minutes — double-click **`Start formicaria.vbs`** again. Your
notes are still in the `vault` folder, exactly as you left them.

## Where your notes are kept

In the **`vault`** folder, inside the folder you extracted.

That is what makes this portable: copy the whole folder to a USB stick or another computer and your
notes travel with it. Nothing is left behind on the machine you were using.

Your notes are the files in `vault\notes\` — one plain text file per note, which you can open in
any editor. This app is not needed to read them.

## What else is in the folder

Only three things are for you:

- **`Start formicaria.vbs`** — starts it.
- **`Manual.html`** — this manual.
- **`vault`** — your notes.

`program` holds the application itself and `manual` holds this manual's pages. You never need to
open either.

## If something goes wrong

**Nothing happens when I double-click.** Some Windows installations block scripts. Double-click
**`Start formicaria (show messages).bat`** instead — it does the same thing in a window that shows
what went wrong.

**It says the port is already being used.** Another program on your computer is using the address
formicaria wants. If it is another copy of formicaria, it will simply open your browser at the copy
already running.

**A blank page.** Close the tab and start it again.

---

Next: [Your first ten minutes](./first-note.md).
