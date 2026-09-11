# Installing a new version

## From 0.5.2 on, formicaria updates itself

**Settings → This machine** says when a newer version is out. formicaria asks GitHub once a day, and
**Check now** asks straight away; nothing about you, this computer or your notes is sent.

1. Press **Get v…**. It downloads the new version and checks that it is exactly what the release
   published, signed by us. Anything that does not match is refused, and nothing has changed yet.
2. Press **Restart into v…**, then **Yes, restart**. What you have typed is saved first. formicaria
   replaces its own program inside the folder it runs from, and this page reconnects by itself in a few
   seconds. **Your notes do not move** — they stay in the `vault` folder beside it.
3. The version you had is kept. If the new one does not suit you, **Settings → Go back to v…** puts it
   back.

**If the new version will not start**, start formicaria again the way you always do. When it fails to
start a few times in a row, the launcher puts the previous version back by itself.

Settings says when a copy cannot update itself, and why — usually a folder the system protects (move
it to your home folder), or a folder set up by a version older than 0.5.2, which needs the way below
once. On a phone, see [formicaria on an Android phone](./android.md#updating).

## By hand, into a new folder

**Your notes are not in the new download. They are still in the folder you have been
using.** Nothing about an update removes them — but you do have to bring them across,
and this section is how.

## Why there is anything to do at all

A download does not install itself. Each version arrives as its own folder —
`formicaria-v0.2.1-linux-x86_64`, then `formicaria-v0.2.2-linux-x86_64` — and your
notes live in the `vault` folder *inside* the one you have been using. That is what
makes the app portable: copy the folder to a USB stick and your notes go with it.

The cost of that is this page. A new download is a new folder, so it starts empty and
opens on the same **Start here** note you saw the first time. It looks as though your
writing is gone. It is not: it is in the old folder, untouched.

> **Do not delete the old folder until you have started the new one and seen your
> notes in it.** Everything below is reversible while the old folder still exists.

## The short way

1. Unpack the new download **beside** the old folder.
2. Open the **new** folder and double-click:

   | Your system | File |
   |---|---|
   | Windows | `Update from an older folder.bat` |
   | macOS | `Update from an older folder.command` |
   | Linux | `Update from an older folder.sh` |

3. It finds the old folder, says how many notes it is about to copy and whether they
   have history, and waits for you to type `yes`.

If you keep several old versions it will not guess between them — it lists what it
found and asks you to drag the one you want onto the same file.

It copies **into** the new folder only. The old folder is never written to and never
deleted, so it remains a complete second copy until you remove it yourself.

## The manual way

Copy the `vault` folder from the old folder into the new one. That is the whole
operation — your notes, their history, your attachments, your saved views and your
theme all live inside it.

Two things not to copy:

- **`vaults.json`.** It records each notebook's *absolute* location, so the old
  folder's copy points back at the old folder. Bring it across and the new version
  quietly keeps writing into the folder you are about to delete. Leave it behind and
  it is written afresh, pointing at the vault beside it.
- **`vault/index.sqlite`**, if you see one. It is this machine's search index, rebuilt
  automatically the next time the app starts. Copying it is harmless but pointless.

## Notebooks you keep somewhere else

If you made notebooks outside the app's folder — anywhere of your own choosing — the
new version will not know about them until you add them again. **The notes themselves
are untouched wherever you put them.** Add each one back with **New vault**, pointing
at the folder that is already there.

The update script lists any it finds in your old settings, so you know what to
re-add rather than discovering the gap later.

## On the phone

Different mechanics, same question.

- **Updating the app keeps everything.** Install the new APK over the old one and your
  notes, history and attachments stay where they are.
- **Uninstalling erases them, permanently.** A phone vault lives in the app's private
  storage, which Android deletes with the app. There is no copy anywhere else unless
  you made one.
- **The phone has no snapshot tier** — `restic` does not exist on Android — so
  attachments on a phone have exactly one copy until they reach a computer.

> **Before you uninstall, or reset the phone: back up.** Open **Back up** and push your
> notes to a remote. That covers the text and its history. Photos and recordings need a
> computer — see [Backup & versioning](./backup.md).

## Which version am I running?

**Settings → This machine → version.** A release shows its tag; a build made from
source says `dev`. It is worth checking when you have two folders and are not sure
which one your browser is showing.
