# Set up formicaria on Windows

formicaria runs in your browser, from a folder on your own computer. Nothing is installed, there is
no account, and nothing you write leaves this machine.

## 1. Unzip it

Right-click the file you downloaded — `formicaria-…-windows-x86_64.zip` — and choose
**Extract All…**. Put it somewhere real, such as your **Documents** folder.

> **Do not skip this step.** Windows will show you what is inside a zip without extracting it, but
> that is a temporary folder it deletes later — and it would take your notes with it.

## 2. Start it

Open the folder you just extracted and double-click:

**`Start formicaria.vbs`**

Your browser opens by itself. That is the app — you are ready to write.

**The first time, Windows will warn you.** You will see *"Windows protected your PC"*. Click
**More info**, then **Run anyway**. The warning means the program carries no paid signature from
Microsoft; it is not a report of anything found in the file.

## 3. Stop it

Close the browser tab. formicaria shuts down a few seconds later.

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
