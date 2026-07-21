# Collaboration

formicaria is built for working with other people **asynchronously** — not a chat, but durable,
anchored discussion and proposed changes that travel through git and reconcile when everyone
syncs. The reasoning about your notes lives *in* your notes, so a collaborator who clones the
vault gets the conversation and the proposals too, offline and searchable in five years — not
locked in a service's database.

Two things make this work, and both reuse the ordinary note:

- **[Discussion](./notes.md#discussion)** — one message is one note, so two people replying at
  once are writing different files and never conflict.
- **Proposals** — a way to suggest a change without pushing it straight to everyone.

## Proposals

A **proposal** is a git branch plus an ordinary note that describes it and carries its discussion.
The note lives on `main` and carries `proposes: branch:<name>`; the branch holds the actual
change. Because it is just a note, its discussion is the same [thread panel](./notes.md#discussion)
every note has, and it travels to collaborators through git like everything else.

Proposals exist because **a vault is a git remote, and you may not have permission to push to its
`main`.** Rather than failing when you try to back up, you propose the change on a branch and
discuss it; someone who *does* have access reviews and merges it. It is also how an **AI agent**
contributes safely: an agent can open a proposal, but only a human can accept (merge) one.

It is a **voluntary protocol, never enforced.** If you can push to `main`, you may propose a change
*or* simply make it — often a change is agreed in conversation and pushed directly. formicaria
offers the proposal path; it never forces it.

### The Collaboration view

The **Collaboration** view (open it from the view picker, or “Open Collaboration” in the command
palette) lists the open proposals across your vaults, newest first. Open one to read it and its
discussion, and to take part.

Proposals are kept out of the Board, Agenda and Timeline — like a discussion message, a proposal
is not something you *plan* — but they remain fully searchable.

> **What is here today.** You can **view and discuss** proposals in this view. Creating a proposal
> from a note or whiteboard, seeing its diff, and accepting (merging) it are being built next; a
> proposal is created today by putting a note carrying `proposes: branch:<name>` in the vault (by
> hand, or by an agent). Conflicts awaiting resolution are surfaced by the
> [backup panel](./backup.md), not here.

### How a proposal ages

A branch can be merged, renamed, or deleted, but the proposal note — and its discussion — lives on
in git history. So a proposal that names a branch which no longer exists is shown with a *warning*,
never hidden: the reasoning about a change outlives the change. Whether a proposal is still open,
merged, or abandoned is read from git, not stored in the note. See the
[frontmatter reference](../reference/frontmatter.md#proposal-semantics) for the exact rules.
