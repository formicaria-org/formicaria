# Backup & versioning

formicarium keeps your notes safe in two complementary ways.

## Git versioning (automatic)

Your vault is its **own git repository** (separate from the app's source). On
first edit, formicarium initializes it (with a `.gitignore` that excludes the
index, thumbnails, and blobs) and then **auto-commits** a few seconds after each
change. Every edit is therefore in history, and you can push the vault repo to
GitHub for an off-site copy of your notes as plain text:

```sh
cd vault
git remote add origin <your-repo-url>
git push -u origin main
```

## Restic backup (on demand)

The **Back up** button snapshots the whole vault (notes + blobs + manifest,
excluding the disposable index and thumbnails) to a
[restic](https://restic.net) repository — dedup, encryption, integrity, and
off-site remotes, none of which formicarium reimplements. Set these before
launching:

```sh
export FM_RESTIC_REPO=/path/or/remote/for/restic
export RESTIC_PASSWORD=…            # keep this safe — it encrypts the repo
```

Then click **Back up**. From the CLI you also get `fm backup`, `fm restore`, and
`fm check` (the last re-reads every pack to catch silent bit-rot).

## Integrity

`fm verify` reports problems like a note with unparseable frontmatter or a
reference to a missing blob. `fm verify --scrub` re-hashes every blob to detect
bit-rot (a blob whose bytes no longer match its own filename).
