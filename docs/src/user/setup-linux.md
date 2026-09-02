# Set up formicaria on Linux

formicaria runs in your browser, from a folder on your own computer. Nothing is installed, there is
no account, and nothing you write leaves this machine.

## 1. Unpack it

Extract the file you downloaded — `formicaria-…-linux-x86_64.tar.gz` — somewhere real, such as your
home folder or Documents. Your file manager can do this, or:

```sh
tar xzf formicaria-*-linux-x86_64.tar.gz
```

## 2. Start it

Open the folder and double-click:

**`Start formicaria.sh`**

If your file manager asks what to do with it, choose **Run**. (Some ask every time; some open
scripts in a text editor instead — if that happens, open a terminal in the folder and run
`./Start\ formicaria.sh`.)

Your browser opens by itself. That is the app — you are ready to write.

## 3. Stop it, and start it again

Close the browser tab. formicaria shuts down a few seconds later.

To come back — tomorrow, or in five minutes — double-click **`Start formicaria.sh`** again. Your
notes are still in the `vault` folder, exactly as you left them.

## Where your notes are kept

In the **`vault`** folder, inside the folder you unpacked.

That is what makes this portable: copy the whole folder to a USB stick or another computer and your
notes travel with it. Nothing is left behind on the machine you were using.

Your notes are the files in `vault/notes/` — one plain text file per note, which you can open in any
editor. This app is not needed to read them.

## What else is in the folder

Only three things are for you:

- **`Start formicaria.sh`** — starts it.
- **`Manual.html`** — this manual.
- **`vault`** — your notes.

`program` holds the application itself and `manual` holds this manual's pages. You never need to
open either.

## If something goes wrong

**Double-clicking opens it in an editor.** That is a file-manager setting. Open a terminal in the
folder and run `./Start\ formicaria.sh`.

**It says the port is already being used.** Another program is using the address formicaria wants.
If it is another copy of formicaria, it will simply open your browser at the copy already running.

**A blank page.** Close the tab and start it again.

## The study assistant

formicaria has an optional local AI assistant, and on Linux **this download can run it**. Turn it on
in **Settings → Study assistant**. The first time, it asks which model you want, tells you how large
it is and under what licence, and downloads it — between 0.7 and 2.5 GB depending on your choice,
once, kept on this computer and used offline afterwards. Letting it read photographed pages adds a
further 0.84 GB, and is offered as its own choice. You can stop part-way; what has arrived is kept.

Turning recordings into text is a further switch beside it, and a further ~170 MB download, which it also fetches for you.

It is off by default and costs nothing while off. Everything else — your notes, search, boards,
backup and sharing — works normally whether or not you ever turn it on. The
[study assistant](./assistant.md) chapter explains what it can do.

---

Next: [Your first ten minutes](./first-note.md).
