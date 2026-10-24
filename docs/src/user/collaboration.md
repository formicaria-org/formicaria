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

The **Collaboration** view (open it from the sidebar) gathers the things that need a **person**: at the top, a **Needs resolution** panel of any
notes still in conflict (both versions marked in their bodies after a merge), and below it the
**open proposals** across your vaults, newest first. Open a conflict to merge the two versions by
hand; open a proposal to read it and its discussion, and to take part.

Below them, folded, are the **closed** proposals, newest first, each marked **Merged**,
**Declined** or **Closed** with the day it ended — like a forge's closed pull requests. Open one to
see what it did: for a merged one, the change it brought into the note; for a declined one, the
reason given and, when this device kept it, the text that was turned down. A closed proposal is a
record, so it can only be read.

The conflict list is *derived* — a note is in it exactly while its body carries the merge markers, so
resolving a note makes it drop off by itself (nothing to dismiss). It matters that it lives here and
not only in a toast: a note in conflict is not saved to history, and the notebook cannot exchange changes with your
other devices until it is settled, so it must stay **findable**.

Proposals are kept out of the Board, Agenda and Timeline — like a discussion message, a proposal
is not something you *plan* — but they remain fully searchable.

> **What is here today.** The whole review round trip: a proposal is **created** — by the study
> assistant, or by hand with a note carrying `proposes: branch:<name>` — and you **see its diff**,
> **discuss** it, then **accept** it, **edit it first**, or **turn it down**. Accepting merges the
> branch into `main`. Conflicts are found and opened here too.
>
> What is not here yet: creating a proposal from a note or whiteboard with a button, rather than
> asking the assistant or writing the frontmatter yourself.

### How a proposal ages

A branch can be merged, renamed, or deleted, but the proposal note — and its discussion — lives on
in git history. So a proposal that names a branch which no longer exists is shown with a *warning*,
never hidden: the reasoning about a change outlives the change. How a proposal **ended** — merged,
declined, or closed — is written on its note when it ends, and is the same on every device; while
nothing has ended it, it is open. See the
[frontmatter reference](../reference/frontmatter.md#proposal-semantics) for the exact rules.
