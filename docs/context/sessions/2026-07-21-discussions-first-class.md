# 2026-07-21 — Discussions as first-class objects

The owner reframed discussion: it should not live *under* a note but be a first-class thing —
created from the `+` menu, living in its own Discussions view, standalone (subjects linked
emergently inside messages, not pre-attached). Refined by a **3-agent review** (maintainability /
longevity / simplicity), which converged on one insight that made the feature nearly free and safe.

## The self-anchor

**A discussion is a note whose `thread_of` points at itself** (`thread_of: note:<own-id>`). That one
move:
- makes the marker a *reference* (parse-guarded) instead of a forgeable/eraseable `discussion:true`
  bool — all three agents flagged the bool as the #1 hazard (a board drag could erase the note, and
  it is permanent in history, Ruling 4);
- makes a discussion *already* an `is_message` note, so it is excluded from board/agenda/timeline/
  recent (`notes_base`), from `activity()`, and from the mock's `isNote` **with zero new plumbing** —
  no third hidden note-class, no new CI-grepped literal;
- unifies the model: `thread_of` already encodes the anchor, so a standalone discussion points at
  itself and a comment-on-a-note points at that note — one mechanism, a future board pin is just
  `note:<id>#<element>` (Ruling 13). Nothing to reshape in history later.

The per-note Discussion panel is **kept** (the agents were unanimous): it is the code the
discussion-open reuses, and it is the anchored-comments seed — both surfaces render through one
thread block, so they cannot drift.

## What shipped

- **Backend:** `thread::is_discussion_root` (the only new predicate); `thread()` excludes the root
  itself (one line); `create_discussion(title, vault)` writes the self-anchor (a command, because
  `set_property` refuses `thread_of`); `discussions()` lists roots + count + `last_activity`
  (most-active-first, store query); `discussion_participants()` reads **who left a message** from git
  per vault (Ruling 14, derived never stored — `activity()` drops messages so it can't be reused),
  merged into `DiscussionSummary` by the dispatcher. Tests in `tests/discussions.rs` (incl. real-git
  participants).
- **UI:** `+ New discussion` in `CREATE_MENU`; a `discussions` built-in pane (via `BUILTIN_PANES`) +
  `Discussions.svelte` (title + `VaultBadge` + participants, most-active-first); a `chat` icon;
  `NotePanel` gains `isDiscussion` (self-anchor) that suppresses the document body and shows the
  **existing** thread block as the pane body, always open, with an inline editable title. Mock
  mirrors `isDiscussion`/`create_discussion`/`discussions`.
- **Docs:** `notes.md` (standalone discussions + the Discussions view), `commands.md`
  (`create_discussion`, `discussions`).

`pixi run ci` green (Rust + 243 UI tests + checks + docs). **Uncommitted**, with the rest of the
session's collaboration work.

## Deferred (no one-way door)

The `/` reference picker in the reply composer (plain `[text](note:id)` works today), extracting
`Thread.svelte` if `NotePanel` bloats, participant avatars, and any decision about removing the
per-note panel. Note the honest limit: a just-created discussion appears in the Discussions view
immediately (roots come from the store), but its *participants* only populate once its files are
committed (they come from git log).
