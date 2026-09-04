# Glossary

Words this manual uses that mean something specific here. Most of them appear in the user
chapters before anything defines them, which is what this page is for.

### Asset

A file that a note refers to — an image, a PDF, a recording. An asset gets a note of its own so
it can be searched and linked, but never a card on the Board, the Agenda or the Timeline: it is
something a note is *about*, not something you are planning to do. See
[Assets & media](../user/assets.md).

### Audience

Who can read a set of notes. In formicaria an audience is **a vault**, because a vault is a git
repository and the people who can clone it are the people who can read it. There is no per-note
permission, deliberately: a field inside a file can be mistyped, and git history is permanent.

### Blob

The stored bytes of an asset, kept once under the SHA-256 of its own content — see
*content-addressed*. Blobs live in `<vault>/blobs/` and are deliberately **not** carried by an
ordinary git push, which is why the [snapshot tier](../user/backup.md) exists.

### Content-addressed

Stored under a name computed from the bytes themselves rather than from a filename. Two identical
files are therefore one file, automatically, and a file that changes gets a different name rather
than overwriting anything. It also means damage is detectable: re-hash the bytes and compare to
the name they are filed under, which is what `verify --scrub` does.

### Discussion

A thread of messages. Either **on a note** — replies about that note, kept with it — or
**first-class**, a discussion that belongs to no particular note and is the root of its own
thread. See [Creating & organizing notes](../user/notes.md#discussion).

### Frontmatter

The block of YAML at the very top of a note, between two `---` lines. It holds the note's
structured fields — `title`, `status`, `due`, `tags`, and any of your own. **Only frontmatter is
structured-queryable**; the body is full-text only. If you want to filter on something, it goes
here. See the [frontmatter schema](./frontmatter.md).

### FTS

Full-text search. formicaria keeps a SQLite FTS5 index of every note's text — including text
extracted from PDFs — so search is fast at any vault size. The index is **disposable**: it is
rebuilt from the Markdown files, never backed up, and never synced.

### Merge driver

A small program git calls when two people changed the same file. formicaria installs one for
`.md`, because the note format rewrites `updated:` on every save — so without it *any* two
concurrent edits collide on that line, inside the YAML, where the parser rejects the result. The
driver merges frontmatter field by field and puts genuine body conflicts in the note's body, where
you can open and fix them. It is the `fm` binary, which is why `fm` must stay beside `fm-serve`.

### Note

One Markdown file. That is the whole definition, and it is the invariant the rest of the design
rests on: a task is a note with a `due` property, a whiteboard is a note whose body is a drawing,
a shared document is a note in a different repository.

### Proposal

A suggested change to a note, living on its own git branch with a note describing it, which you
read and then accept, edit or turn down. It is how the [study assistant](../user/assistant.md)
writes: it can propose, never write to `main`. See [Collaboration](../user/collaboration.md).

### Remote

The other copy of a vault's git repository — usually on a hosting service, but a folder on a
drive works too. Giving a vault a remote is what makes it shareable *and* what makes it backed up;
the two are the same act. Sharing a vault means giving it a remote and giving people access.

### Snapshot

An encrypted, deduplicated backup of a vault's notes **and** its attachments, taken by
[restic](https://restic.net). The other tier — git — carries the notes and their history but not
the media, which is why both exist. See [Backup & versioning](../user/backup.md).

### ULID

The identifier every note is named by: 26 characters, sortable by the time it was created. Used
instead of a title because a title changes and a filename that changes breaks every link to it.
You will see one in a `note:` link and as a note's filename, and you never have to type one.

### Vault

A directory of notes that is also a git repository. Running several — personal notes, a lab's
notes, a paper shared with one collaborator — is the normal case and is what the name *formicaria*
refers to. Each vault has its own remote, its own collaborators, and its own backup. See
[The vault format](../user/vault.md).

### View

A saved way of looking at your notes — a query plus a renderer — kept as a small file in
`<vault>/views/`. Because it lives in the vault it travels to collaborators like a note does.
See [Views](../user/views.md).
