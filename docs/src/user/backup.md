# Backup & versioning

Your vault holds two very different kinds of thing, so it is backed up in two
tiers. **Back up** does the light one by default; the heavy one is a box you tick.

| | What it carries | Where |
|---|---|---|
| **Notes** (default) | your notes, views, themes, `manifest.json` — plain text, with history | git → your remote |
| **Snapshot** (*Take a snapshot*) | your notes **and** `blobs/`: images, PDFs, video — no history | restic |

The split is not an accident. Your notes are small, plain, and mergeable, so git
carries them anywhere for free. Your media is heavy and is deliberately kept out
of git (`blobs/` is in the vault's `.gitignore`), which is what keeps the notes
repo small and clonable forever — but it also means **pushing your notes does not
back up your media**. The Vaults section says so every time, and tells you afterwards
exactly what left the machine.

Neither tier is a copy of the other, and neither is a copy of the whole folder.
Git has your history and your views and themes; the snapshot has your attachments.
**Between them they do not cover everything in the vault folder** — see
[what a snapshot contains](#including-attachments-restic) below.

**Attachments can travel in git too, if you want them to.** A vault can set a size
limit — *Send attachments under* in **Settings** — and anything at or under it is
pushed with the notes, so a screenshot in a note arrives with the note on your other
machine. It is off by default, and it is set per vault rather than per computer,
because git history is permanent: a large file committed once is in every clone
forever. Use it for small things and leave the heavy ones to the snapshot tier.

**There is a ceiling: 100MB per file, and the app will not go past it.** formicaria
does not use [git-lfs](https://git-lfs.com), so an attachment that travels in git is
stored whole, in history, for good — every clone downloads it again, and taking it
back means rewriting history other people have already pulled. Above roughly 100MB
most hosts (GitHub among them) refuse the push outright, and they refuse it *after*
your commit is made, which is the worst moment to discover it. So:

| Limit you set | What happens |
|---|---|
| up to 50MB | accepted |
| 50–100MB | accepted, and Settings says what it costs |
| over 100MB | refused, with the reason |

If a vault's `vault.json` asks for more than 100MB — hand-edited, or written by
another machine — the extra is simply not sent, and Settings tells you so rather than
letting a push fail. Anything above the ceiling belongs in the snapshot tier below,
which carries attachments of any size and has none of these problems.

## Setting up (once)

Open **Settings → Vaults & backup** and paste your vault's git remote under that vault, for
example `git@github.com:you/notes.git`. That is the only setup the light tier needs. Everything
about a vault is set there and only there, in one block: its folder, where it is sent, its snapshot
repo, which attachments travel with it, when it was last saved, and removing it from the list.

The **Back up** button is for doing, not setting up: it opens a short menu — *Back up notes*, *Get
their changes*, and *Take a snapshot* once a vault has a snapshot repo — so nothing is sent by
pressing it once.

Your vault is its **own git repository**, independent of the app's source: its
remote is yours to choose and has nothing to do with where formicaria's code
lives. formicaria initializes the repo on first edit and **auto-commits** a few
seconds after each change, so your history is local from the start; the remote is
just where you send it.

**Authentication is your existing git setup where you have one** — an ssh-agent key
or a credential helper, exactly as a `git push` in a terminal would use, and
formicaria keeps nothing of its own. An SSH remote always works this way.

Where there is no such setup — an `https://` remote on a machine with no credential
helper, which is the usual case on Windows and on the phone — **Back up** asks for an
access token and stores it itself, in its own private storage. It says which of the two
is happening at the moment it asks. Use a fine-grained token limited to the one
repository, with an expiry date.

## Backing up

**formicaria backs up by itself.** A little after you stop writing, and when you leave the app, it
gets the changes from your other devices and sends yours: the same thing the **Back up** button
does. It never does it while you are typing, and never for a vault you have not backed up once
yourself. If two devices changed the same sentence, it says which notes to look at, exactly as
after a backup you pressed. **Settings → Vaults & backup** has the switch if you would rather press.
It works while the app is open; a device that is closed sends when you next open it.

Press **Back up**, then **Back up notes** — that is the whole action. It saves anything
outstanding, **gets the changes from your other devices and collaborators first, merges them**,
and then sends yours, rather than making you do that by hand. If the
merge turns up genuine conflicts it stops, names the notes, and does *not* push (publishing
conflict markers as content would be worse than not publishing). It reports what happened
either way — including whether your remote is genuinely off this
machine. (A remote can be a local path or a `file://` URL, which is a fine way to
back up to an external drive but does not survive the drive; the Vaults section labels that
honestly rather than calling it backed up.)

**Your remote gets one commit per backup — of the app's own churn.** Auto-commit
fires every few seconds while you work, so pushing that raw would bury your remote in
thousands of `auto:` commits. They are squashed into a single `backup:` commit
instead. The trade: locally you can step back through individual edits only as far as
your last push — before that, each push is one step.

**Commits you wrote yourself are never squashed.** If your vault is also a repo you
commit to by hand — a manuscript, a project — those commits are yours and keep their
shape; the squash stops at the most recent one that formicaria did not write. Only the
app's own `auto:`/`backup:` commits are collapsed.

Your *first* push is never squashed either, since squashing history that has never been
backed up would destroy the only copy of it.

## Including attachments (restic)

Give a vault a backup repo in **Settings → Vaults & backup**, then choose **Back up → Take a
snapshot** to send its **notes and attachments** to a
[restic](https://restic.net) repository: dedup, encryption, integrity, and
off-site remotes, none of which formicaria reimplements.

**What a snapshot contains, exactly.** Your notes directory and `blobs/` — and
nothing else. Not the other things that live at the top of the vault folder:
**not `views/`, not `themes/`**, not `manifest.json`, not `vault.json`, not the
search index, and **not the git history** — `.git` is never snapshotted. Those
travel in the notes tier instead, which is one reason to use both. If your vault
folder is also a project you work in, that project's own files are yours to back up
your own way; they are deliberately left alone rather than swept into your backups.

A restore therefore gives you back your notes and attachments as files, with no
history behind them — a recovery, not a second copy of your repository.

Set it up in **Backup**, in two steps:

1. **Where the backups go.** Each notebook gets its own **backup repo** — a folder
   on another drive, or a remote like `sftp:you@host:/backup`. Leave it empty and
   that notebook's attachments simply stay on this computer.
2. **A password.** The backups are encrypted, so they need one. You set it once and
   it covers every backup repo on this computer.

> **Keep the password somewhere safe.** If it is lost, the backups it protects
> cannot be opened again — by anyone. Your notes themselves are unaffected: they are
> plain files, and they travel with the notes backup above.

The password is kept in a file on **this** computer, readable only by you, and never
inside the vault or the vault list. Two things follow. A new computer — or a
reinstall — needs it typed in again, so it is worth storing where you keep your other
passwords rather than only in that file. And backing up that file is not a substitute
for knowing the password; the file is a convenience, the password is the key.

### When did this last work?

![Settings, Vaults: the vault's git remote, what the snapshot tier would carry, and the
Back up button](../images/backup.png)

Under each repository, the Vaults section says when that vault's last snapshot was taken. It is
three different answers and they mean different things:

- **Last snapshot: `<date>`** — the repository was read and this is the newest formicaria snapshot
  in it. It also says how many paths that snapshot covered.
- **Never backed up** — the repository opened fine and has no formicaria snapshot in it at all.
  Configured is not the same as done, and this is the state worth noticing.
- **Last snapshot: unknown — `<reason>`** — this computer could not ask: no restic, no password,
  or a repository it could not open. That is different from "never", and the Vaults section will not say
  one when it means the other.

It is read when you open the Vaults section rather than continuously, because asking restic means running
it — and for a repository that is not on this machine, reaching across the network.

If **restic** is not installed on this computer, the fields do not appear and the
tick box says why — notes are unaffected either way. To install it:

```sh
# Debian / Ubuntu
sudo apt install restic
# macOS
brew install restic
# any OS, via pixi
pixi global install restic
```

**On Android there is no snapshot tier at all** — restic is not available there, so a
phone's attachments have only the copy on the phone until they reach a computer.
Notes are unaffected: the phone pushes them with git like everything else.

**A snapshot does not need git.** *Take a snapshot* runs whether or not a vault has a git
remote — and since a snapshot carries the notes directory as well as `blobs/`, that is a real
backup of everything, not a half one. Each vault gets its own line in the result: what went in,
where it went, or why it could not be taken.

A vault can also be brought *back* from a snapshot inside the app — **New vault →
Restore a backup** — and not only from the command line below.

From a terminal you also get `fm backup`, `fm restore`, and `fm check` (the last
re-reads every pack to catch silent bit-rot). `RESTIC_PASSWORD` still works if you
prefer to set it in the environment: it wins over the stored one.

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
