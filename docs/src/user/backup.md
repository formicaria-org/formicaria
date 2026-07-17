# Backup & versioning

Your vault holds two very different kinds of thing, so it is backed up in two
tiers. **Back up** does the light one by default; the heavy one is a box you tick.

| | What it carries | Where |
|---|---|---|
| **Notes** (default) | your notes, views, themes, `manifest.json` — plain text | git → your remote |
| **Media** (tick to include) | all of the above **plus** `blobs/`: images, PDFs, video | restic |

The split is not an accident. Your notes are small, plain, and mergeable, so git
carries them anywhere for free. Your media is heavy and is deliberately kept out
of git (`blobs/` is in the vault's `.gitignore`), which is what keeps the notes
repo small and clonable forever — but it also means **pushing your notes does not
back up your media**. The panel says so every time, and tells you afterwards
exactly what left the machine.

## Setting up (once)

Click **Back up** and paste your vault's git remote — for example
`git@github.com:you/notes.git`. That is the only setup the light tier needs.

Your vault is its **own git repository**, independent of the app's source: its
remote is yours to choose and has nothing to do with where formicaria's code
lives. formicaria initializes the repo on first edit and **auto-commits** a few
seconds after each change, so your history is local from the start; the remote is
just where you send it.

**Authentication is your existing git setup** — an ssh-agent key or a credential
helper, exactly as a `git push` in a terminal would use. formicaria stores no
password or token of its own. If you have never pushed from this machine before,
set up an SSH key with your host first; the app cannot prompt you for one (there
is nowhere to type it), so it will report an auth failure instead of hanging.

## Backing up

Click **Back up**, then **Back up notes**. It commits anything outstanding, pushes,
and reports what happened — including whether your remote is genuinely off this
machine. (A remote can be a local path or a `file://` URL, which is a fine way to
back up to an external drive but does not survive the drive; the panel labels that
honestly rather than calling it backed up.)

**Your remote gets one commit per backup.** Auto-commit fires every few seconds
while you work, so pushing them raw would bury your remote in thousands of `auto:`
commits. Everything since your last push is squashed into a single `backup:` commit
instead. The trade: locally you can step back through individual edits only as far
as your last push — before that, each push is one step. Your *first* push is never
squashed, since squashing history that has never been backed up would destroy the
only copy of it.

## Including media (restic)

Tick **Include media** to also snapshot the whole vault — blobs and all — to a
[restic](https://restic.net) repository: dedup, encryption, integrity, and
off-site remotes, none of which formicaria reimplements. It needs two environment
variables set before you launch:

```sh
export FM_RESTIC_REPO=/path/or/remote/for/restic
export RESTIC_PASSWORD=…            # keep this safe — it encrypts the repo
```

Without them the checkbox is disabled and says so. Note that the desktop launcher
does not set them — this tier is for a terminal launch, or for a launcher you have
edited yourself. From the CLI you also get `fm backup`, `fm restore`, and `fm check`
(the last re-reads every pack to catch silent bit-rot).

If you only ever push notes, **restore what you can and check what you lost**: a
git-only restore brings back every note, and `fm verify` then reads the notes' own
`asset:` references and names exactly which media is missing — so you know what to
re-fetch rather than having to guess. (`manifest.json` adds bit-rot detection on
top of that, but only the `fm manifest` command writes it; the app never does, so
don't count on it being current.)

## Integrity

`fm verify` reports problems like a note with unparseable frontmatter or a
reference to a missing blob. `fm verify --scrub` re-hashes every blob to detect
bit-rot (a blob whose bytes no longer match its own filename).
