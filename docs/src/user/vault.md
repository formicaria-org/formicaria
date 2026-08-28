# The vault format

A vault is one folder. Everything durable is a plain file you could read without
this app.

```text
vault/
├── notes/            # one Markdown note per file, ULID-named   (git-tracked)
│   └── 01J….md
├── blobs/            # content-addressed media, sha256/ab/cd/…  (git-ignored)
├── derived/          # regenerable thumbnails                    (git-ignored)
├── manifest.json     # sha256 → size inventory of the blobs      (git-tracked)
├── vault.json        # optional: what this vault is called and where
│                     #           its notes live                    (git-tracked)
├── views/            # saved .view queries (YAML)                (git-tracked)
├── .gitattributes    # routes *.md through the fm merge driver    (git-tracked)
├── index.sqlite      # disposable per-machine FTS index          (git-ignored)
└── .gitignore
```

## `vault.json` — using a repo you already have

A vault does not have to be a folder made for it. If you already have a project — code, a
manuscript, a thesis — whose notes live in `docs/`, drop a `vault.json` at its root and
formicaria reads them where they are:

```json
{
  "name": "the paper",
  "description": "notes beside the manuscript",
  "notes": "docs"
}
```

Every field is optional and the file itself is optional; without one you get the layout
above. `notes` is relative to the vault root and **must stay inside it** — which audience a
note belongs to is decided by which repo holds it, so notes living outside would make "who
can see this" unanswerable.

There is no `author`, `collaborators`, `remote` or `created` field here, because git already
records all of those.

`name` gives way to the name in your own vault list if you set one there: a repo you cloned
does not get to rename your audience out from under you.

A note is Markdown with a YAML frontmatter header:

```markdown
---
schema: 1
id: 01J8Z9X0K2A3B4C5D6E7F8G9H0
type: note
status: doing
due: 2026-07-20
hard: true
tags:
  - meta-rl
created: 2026-07-15T10:00:00Z
updated: 2026-07-15T12:30:00Z
project: alpha        # any custom key is preserved
---

The note body is plain Markdown. Math with $\lambda$, ```mermaid``` diagrams,
and asset references like ![figure](asset:sha256-…) all render in the read view.
```

Key properties of the format:

- **The file is the atom.** One note = one file; there are no per-block ids or
  timestamps to corrupt.
- **Custom keys survive.** Any frontmatter key you add is preserved through
  read/edit/write and is immediately usable for grouping.
- **The index is disposable.** `index.sqlite` is rebuilt from the files on every
  open; delete it any time. It must never be synced between machines.

See [Frontmatter schema](../reference/frontmatter.md) for the full field list.
