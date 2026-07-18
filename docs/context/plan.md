# Plan — formicaria, one sequenced program

The single **forward** document: what we intend to build next, in order, and *why not
the obvious alternative*. It supersedes and folds in the old `roadmap.md` (near-term
single-user work) and sits above [`collaboration-design.md`](./collaboration-design.md),
which stays as the line-by-line **code audit** behind Track C — this file carries the
sequence and the rulings; that file carries the receipts.

Like the collaboration doc, most of what's below does **not exist yet**. When a line
ships, delete it here and fold the outcome into `overview.md` / `decisions.md`.

_Last updated: 2026-07-18, after **Track V's V2+V3 shipped, both blocked decisions were
settled, and the compiled work queue was emptied**
(`sessions/2026-07-18-vault-as-a-repo-and-the-queue.md`). A vault no longer has to be a folder
made for it: `vault.json` says where the notes are, the auto-commit stages only what the app
wrote, restic snapshots only the vault's own directories, and the squash stops at commits you
wrote by hand. **`git2` is rejected** and "git is a capability, not a dependency" stands.
Before that, **Track M's host-side band shipped in full** — rulings 1
(`fm_app::dispatch`, the one command surface), 6 (liveness/reindex split), 7 (the streaming blob
route), 8 (the explicit sync loop) and the phone shell + board touch fallback; plus the whiteboard
element merge, the O(n²) poll fix, a lost-update guard on `update_body`, and a `check-ui` CI gate
(`sessions/2026-07-18-dispatch-and-blob-route.md`,
`sessions/2026-07-18-sync-loop-and-scene-merge.md`). Deliberately **not** built: the cold-start
`Incremental` switch (rejected), and rulings 2/3's **git2 swap + Ruling B's image-strip, both
⛔ blocked on owner decisions** — see below. **M0–M8 stay blocked on the Android toolchain**, and
spike (iii) PAT clone is untouched. Before that, **Track M — mobile was planned** (formicaria on the phone;
the owner overrode `MASTERPLAN:57`'s "phone = server + auth" framing — the app runs **on the
phone itself**). The design + receipts are in [`mobile-design.md`](./mobile-design.md); a
compact Track M is below; the load-bearing rulings and two reversals are in `decisions.md`.
Before that, **Track V's V1 shipped** (the first-run gate + the vault list's first writer —
`sessions/2026-07-17-create-vault.md`) and the UI direction was set: **sanitize `render.ts`,
de-modalize the note trail, then earn `.view`** (the layout ask turned out to be
`MASTERPLAN:323`'s own deferred design, not a new feature). Before that, **Track C Phases 0, 1
and 2 shipped** and the rename landed with them: a shared vault works end-to-end and the plural
is literally true. **Next: Track V4 — adoption** (transient ids, so any `.md` reads for free and is stamped only
when you cite or edit it). V1–V3 have shipped, so V4 is the largest unbuilt item in this file;
the UI track is independent; Track M's host-side band is done and M0–M8 are blocked on the
Android toolchain *and* on a git backend the `git2` rejection deliberately leaves open.
Everything not marked SHIPPED still describes things that do not exist._

## The vision — *formicaria*

**Knowledge management, task scheduling, and collaboration, on files you own.** The
evolution from the single-user tool is not three new subsystems bolted on. It is the same
tool, seen by more than one person.

### Three pillars, one atom

Knowledge, scheduling, and collaboration are **three views of one Markdown file**:

- a **task** is a note with a `due`;
- a **message** is a note with a target;
- a **shared note** is a note in a different repo.

That is the whole reason this stays small. **Resist every urge to add a fourth thing.**
The moment scheduling gets its own store, or messages get their own format, the tool has
three products to maintain and the atom stops paying rent.

### Held to (the founding spirit, unchanged)

- **Files-as-truth; the atom is the file.** One note = one Markdown file; no per-block
  ids or timestamps, ever (that change *voids this plan* — `MASTERPLAN.md:456`).
- **The `fm-query` purity seam.** No `std::fs` / db / paths in `fm-query` — compile-time +
  CI-grep enforced. It is what turns a storage swap into a backend change.
- **Literal-free renderers.** No workflow enums (`todo/doing/done`) in `ui/src/renderers/`.
- **Bounded, replaceable dependencies.** Lean on tools that already solve a problem whole
  (git, restic, Excalidraw, KaTeX) and shell out. **No CRDT library, no sync framework,
  no plugin API** — those are the ones that would own us. Every seam exists so the answer
  to "can we swap this?" stays yes.
- **The UI is the product, not the plan's afterthought.** Git does the heavy lifting
  *precisely so* the interface can be the thing we build. A phase that ships only Rust
  shipped nothing. (GitHub already has the correct backend and a developer's diff — which
  is exactly why nobody keeps notes in it.)

### The competitive line

**Nobody offers files you own + git underneath + a UI a non-git person can use.** GitHub
has the substrate and shows a knowledge worker a diff. Obsidian's Git plugin is a fiddly
wrapper with no review, no blob story, no vault-aware views. Notion has the multiplayer
and owns your data. Thin if our UI is thin; a moat if it's good.

## The name — **done** (landed 2026-07-17 with Phase 2)

A *formicarium* is one colony's nest; *formicaria* is the plural — a **set** of vaults,
one per audience, coordinating through git rather than a hub. The rule was to rename only
once the plural was literally true; Phase 2 made it true and the rename went with it, as
its own commit. No code identifiers moved (every crate was already `fm-*`, and `fm`
abbreviates either name).

**Nothing is called formicarium any more.** The two literals originally held back —
`git.rs`'s placeholder identity and Excalidraw's `source` — were kept only for backward
compatibility, and there was none to keep: the author is the only user and no vault was
running on the placeholder. Both moved.

**One rule survives the rename.** `PLACEHOLDER_EMAIL` is a **sentinel matched by value**:
`identity()` compares `user.email` against it to decide "nobody real signs this vault", and
a vault's `.git/config` is per-machine, so we cannot migrate the ones we cannot see. It was
safe to change *once*, while every vault in the world was the author's. Change it again and
every vault still carrying the old value silently acquires a real identity, `set_remote`
stops asking, and the provenance hole Phase 0 closed is open. Don't.

## Two rulings the owner has now made

Both were left "for the owner to rule on deliberately"; both are now decided. Each reduces
to *reuse an existing seam*, which is why they fit the project rather than stretch it. Full
entries are in [`decisions.md`](./decisions.md).

### A. Inline meeting actions — promote-to-note, never per-block

A checkbox typed mid-meeting (`- [ ] Ravi to send the draft`) stays **plain Markdown in
the body**, rendered as an interactive toggle that rewrites the body bytes. It does **not**
auto-appear in the agenda. A deliberate gesture — a `/promote` slash entry or a button on
the line — **extracts that line into its own note file** (a task note with `status`/`due`,
back-linked to the source via `[Title](note:<ulid>)`), which then flows into board/agenda
as a first-class atom. Honours *the atom is the file*; keeps mid-meeting friction at "type
`- [ ]`". **Rejected:** a body-scan "checkboxes → agenda" pass (manufactures items that
don't round-trip and re-pollutes the very views the 2026-07-16 assets decision cleaned up).

### B. Board images — strip to the blob store on save (⛔ **blocked**, and the ordering claim below was wrong)

Excalidraw embeds a pasted image as a **base64 data URL inside the note body**, rewritten
whole on every pointer move — routing bulk binary through git and breaking the
"blobs are already out of git" premise. The intent stands: **on board save, strip inline
`files` out of the scene into the content-addressed blob store** (reuse
`BlobStore::put_bytes` + ingest's MIME sniff), leaving only blob references in the scene
JSON; **rehydrate on load** via `GET /api/blob/<reference>` (which now exists).

**It cannot ship until one question is answered.** `blobs/` is gitignored, so the moment
images live there instead of in the body, a shared board shows *no images* on a
collaborator's clone. Pick (a) git-track whiteboard-embedded blobs as a scoped exception
(simplest, fully offline, hash-deduped), or (b) a blob mirror (the scale path). Undecided.

**And "must land before boards are shared" turned out to be false** — boards are shared
*now*, un-stripped, because the element merge (`fm-core/src/scene.rs`) shipped without
needing this. So the real cost of waiting is churn, not correctness. Two traps found while
auditing: `onDestroy` flushes **synchronously**, so an async upload on save loses the last
stroke before closing a board (upload eagerly on paste instead); and `lastSerialized`
compares the **raw** serialization, so it must switch to the stripped text or the
no-op-save guard breaks. **Rejected:** accept-and-document — a 2 MB screenshot becomes
~2.7 MB of churn per stroke.

## The sequenced program

Three tracks. **Track S** (single-user near-term) is independent — ship any item anytime.
**Track C** (collaboration) is strictly ordered and **Phase 0 gates everything**.
**Track V** (a vault is a folder you already have) is ordered too: V2 gated V3–V4, and
**V1–V3 have all shipped, so V4 is next**. The one
cross-tie: Ruling B lands before Track C Phase 3 shares boards.

> **Interleave rule:** Phase 0 fixes *live single-user bugs*, so it earns its place even if
> collaboration never starts — do it first. Track S items (calendar, whiteboard-in-note,
> PDF) need nothing from Track C and can run ahead of or alongside Phases 1–3. **Track V's
> V2 is the same shape**: four live bugs that earn their place even if you never convert a
> repo, and two of them are silent.

### Track S — single-user near-term (from the old roadmap.md)

1. **Time on `start`/`due`** — **DONE** (`decisions.md`; `sessions/2026-07-15-time-on-dates.md`).
2. **Calendar import (ICS → notes, one-way).** `fm ics pull <url|path>` → one note per
   `VEVENT`; `DTSTART`/`DTEND` → `start`/`due` stamps; `SUMMARY` → title;
   `LOCATION`/`ORGANIZER`/attendees → frontmatter; `tags: [meeting, <source>]`; **body left
   empty for the owner.** Idempotence is the trick: key each note to the event `UID`, store
   `ics_uid`/`ics_seq`, update times when `SEQUENCE` bumps, and **never touch the body**. No
   TLS in `fm-serve` → **shell out to `curl`** (add to pixi; **zero new Rust deps**);
   hand-rolled ICS parser (~200 lines, RFC 5545 line-unfolding, `VALUE=DATE` → timeless
   `Stamp`). `TZID`/UTC → naive local wall-clock at import. Risk: NUS may block calendar
   publishing → fallback is a manually exported `.ics` in a watched folder (same parser, no
   network). Target both NUS M365 and Google, merged into one vault, tagged by source.
3. **Calendar export (notes → ICS, the other way).** `GET /calendar.ics` from `fm-serve`
   emitting every note with a `start`/`due`, so Outlook/Google can **subscribe**. A ~100-line
   formatter; ships with #2. **Two-way write-back stays rejected** (Graph/CalDAV + OAuth +
   token store + conflict resolution for ~10% more value).
4. **Whiteboard-in-a-note + one PDF.** Embed **by reference**: a note embeds a board
   (`![[<board-id>]]`); `render.ts` resolves it to a static SVG via Excalidraw's
   `exportToSvg`, click-to-edit. (Rejected: inlining scene JSON — bloats the `.md`, wrecks
   diffs, can't be reused.) A `/draw` slash entry creates the board and inserts the embed.
   **PDF = `window.print()` + an `@media print` stylesheet** — the browser's "Save as PDF"
   is the engine, zero new deps. **Ruling B (image-strip) lands here or before board
   sharing.**
5. **Source trait + local extraction** (design phase). A `Source` trait beside `Store` —
   the sanctioned extension seam, **not a plugin API** (those rot). Two rules pinned before
   any model: *the model proposes, the file system disposes* (extraction lands as a draft
   the owner confirms — a hallucinated date must not be able to enter the vault); and
   *local + optional* (no cloud inference, degrade cleanly when the model is absent — the
   `media` pixi feature is the precedent for isolating heavy extras). ICS (#2) should be
   written so it *is* the first `Source`. Cheapest real email step, model-free: **drag an
   `.eml` onto a note** (ingest already sniffs MIME → headers to frontmatter, body to note,
   attachments to blobs).

### Track C — collaboration (from collaboration-design.md; do not reorder)

**Phase 0 — stop the app dying on a merge. ✅ SHIPPED 2026-07-17** —
[`sessions/2026-07-17-phase-0.md`](./sessions/2026-07-17-phase-0.md). All four items
landed in `fm-core` with regression tests, `pixi run ci` green, driven end-to-end
against a real `fm-serve`: the tolerant loader (a conflicted note is skipped, named and
warned about, not fatal), the mid-merge auto-commit refusal, the `push_squashed`
ancestry guard (design-doc §19 closed), and a **real committer identity**, asked for at
the one moment it matters — a vault gaining a remote. Reduced to one line because
Phases 1–3 order against it; the detail is in the session entry and `decisions.md`.

**The gate is open.** Phase 1 is the next work.

**Phase 1 — make one shared vault trustworthy. ✅ SHIPPED 2026-07-17** —
[`sessions/2026-07-17-phase-1.md`](./sessions/2026-07-17-phase-1.md). Verified as a real
round trip through the API against two vaults and a bare remote: they push → `ls-remote`
notices → we edit a *different paragraph of the same note* → `pull` merges clean → both
edits are in the note and visible → we push back → their clone pulls ours. The plan's own
bar — *"two clones edit different lines of one note → clean auto-merge, no `updated:`
conflict"* — passes.

What landed: the **`FileStore::put` staleness guard** (`StoreError::Conflict`, the
lost-update bug); **incremental reindex** + the **3 s local poll**, folded into the
heartbeat that already ran at that cadence rather than a second timer; **`ls-remote`
remote poll** (`git::remote_moved`, moves no refs) and **`pull`**; and the **`.md` merge
driver** (`fm-core/src/merge.rs`, `fm merge-md`, installed by `ensure_repo`).

Two rulings made while building, both in `decisions.md`: the driver **shells out to `git
merge-file` for the body** rather than carrying a diff3, and it is **never installed
unless we can name an `fm` binary that exists** — git reads a driver that fails to run as
a conflict and hands back *ours* with no markers, which is silent data loss dressed as a
normal file.

**Mostly closed since.** The *automatic* awareness is wired: a visibility-gated 45 s
`remote_moved` poll feeds a top-bar chip with one-click pull, and that pull goes through
`sync.svelte.ts`'s `pullVault` (commit first, then name any conflicted notes rather than
throwing a string). **Still open, small:** `FileStore::skipped()` is stderr-only, so a note
that could not be read is invisible in-app — the one place the app knows something is wrong
and does not say so where you are looking.

**Phase 2 — multi-vault. ✅ SHIPPED 2026-07-17** —
[`sessions/2026-07-17-phase-2.md`](./sessions/2026-07-17-phase-2.md). Verified against two
real vaults through the API: a capture with no vault named lands in the default; the board
spans both with each note badged by where it actually lives; `vault:` appears in **zero**
files; `backup_status` is a per-vault list; a remote set on `lab` leaves `personal`
untouched; and a typo'd vault name is refused, not defaulted.

`Object.vault` (derived, never serialized — the byte round-trip test caught `to_file`
writing it back on its **first run**), the **`candidates`** seam (`Store::query` is now a
default method; FTS5 federates for free — 10 accent-folded hits across two vaults, an
answer only the index can give), **`MultiStore`**, the **vault-list config file**
(`~/.config/formicaria/vaults.json`; absent = the single vault, so nothing to migrate),
per-vault git, badges, and a vault filter. Filter/group by vault needed **no**
query-engine change, exactly as decision 5 predicted.

**The two decisions this file never made, now made** (full entries in `decisions.md`): git
is per-vault, so the backup panel became a **list, not a form** — N remotes, N identities,
N "someone pushed" — because a `push` that quietly meant "the first vault" is the
overstatement `destination.ts` exists to prevent; and **blob resolution searches every
vault**, because content-addressing makes that correct rather than merely convenient,
where vault-scoping references would re-couple notes to locations and break the
cross-vault links ULIDs give for free.

**✅ And the rename landed with it** (2026-07-17, its own commit — a rename should read as
"only strings moved"). The tool is **formicaria**. Both booby traps survived contact and
are now commented *in the code*, because after the rename they look like a missed one:
`git.rs`'s placeholder identity is a sentinel matched **by value**, and Excalidraw's
`source` is written into every board's JSON on disk. The `~/.config/formicaria/vaults.json`
path moved too, with a one-release fallback to the old location — a config left behind
would otherwise fall through to the single vault and look exactly like the other vaults
vanishing.

**Phase 3 — the differentiators.**

- **~~The `.excalidraw` 3-way merge driver~~ — ✅ SHIPPED 2026-07-18**, and in a different
  shape than written: it is not a driver and not in `fm-cli`. There is no `.excalidraw` file —
  a board's scene is the **body of its own `<ulid>.md`** — so the existing `*.md merge=fm`
  attribute already routes it, and the merge is a branch inside `fm-core/src/scene.rs` that
  `merge_body` tries before the text merge. Everything else held: **not** Excalidraw's
  `reconcileElements`, which is a base-less 2-way union that resurrects every deleted shape;
  the 3-way rule recovers deletion intent (in base & gone one side → honour the delete;
  absent from base → an add, keep; in both → higher `version` wins, tie-break lower
  `versionNonce`); and the fractional `index` decides z-order. Verified through real git in
  `fm-cli/tests/merge.rs`. **It did *not* need Ruling B's image-strip first** — that ordering
  claim was wrong, and boards sync un-stripped today.
- **Backlinks → anchored comments → discussion.** The forward half of references shipped
  (`[Title](note:<ulid>)` + sliding panes, 2026-07-16); the **reverse index** (body scan on
  reindex, or a `links` table) is still open. A comment is a note linking to a target → the
  **comments panel *is* the backlinks panel** (one mechanism, two features). Discussion =
  **one file per message, ULID-named** (Maildir: nobody touches the same file, git merges
  trivially, ULIDs order for free), living in the vault it's about. A message is a
  `Kind::Note`, so it needs a **property-based exclusion in `board`/`agenda`/`recent`** (a
  query-layer change on the assets-decision seam) or a 20-message thread floods the views.
  **Never call it chat** — 20 s latency is fine for durable discussion, broken for chat;
  ephemeral coordination belongs in Signal/Slack, not a knowledge base.

### Track V — a vault is a folder you already have

**Where this goes:** every project repo is a vault, its notes beside the code or manuscript
they describe, so formicaria renders and searches the Markdown of *every* repo you own and a
note in one project cites a note in another. That is **"vaults are audiences" completed**,
not strained: one repo = one audience = one collaborator list, and your paper's notes have
different readers than your code's. The reading half already works —
`MultiStore::get` walks every vault (ULIDs are globally unique, so no URI scheme), FTS5
federates through the `candidates` seam.

**V1 — the first-run gate + the vault list's first writer. ✅ SHIPPED 2026-07-17** —
[`sessions/2026-07-17-create-vault.md`](./sessions/2026-07-17-create-vault.md).

**V2 — co-tenancy correctness. ✅ ALL FOUR SHIPPED 2026-07-18** (both silent ones among
them). It gated V3–V4; **V3 has since shipped and V4 is the next feature.**
Every one is a **live bug today**, verified by reading the code, not hypothesised. They only
bite once a repo holds work that isn't ours — which is exactly what V3 enables, so this lands
first.

1. **✅ FIXED — `push_squashed` ate the user's own commits.** `newest_foreign` walks
   `tracking..HEAD` newest-first and stops the squash at the first commit whose subject is
   not `auto:`/`backup:`; a dedicated vault has none, so it collapses to one exactly as
   before (pinned by a test asserting both). Original finding: `reset --soft <tracking-ref>` collapses
   *every* unpushed commit, so three hand-written manuscript commits become one `backup:`.
   Its own comment justifies the squash by "auto-commit fires every few seconds" — that
   justifies squashing **ours**, never theirs. **Fix:** compute `base` as the newer of
   `tracking(vault)` and the most recent commit we did not write, discriminated by the
   **`auto:`/`backup:` message prefix — never the author**: a PC has one git user and we
   commit *as* them, so author-based detection cannot work. Dedicated vault → every unpushed
   commit is ours → identical to today. No mode, no flag; derived from the history.
2. **✅ FIXED — `.gitattributes` was skipped if the file existed** (`git.rs`), and every real repo has
   one → `*.md merge=fm` never lands → **the merge driver silently never engages** → every
   concurrent edit collides on `updated:` inside the YAML fence and the note stops parsing.
   *This is Track C Phase 1's disaster, reintroduced by conversion.*
3. **✅ FIXED — `.gitignore` was skipped if it existed** → `blobs/`, `index.sqlite`, `derived/` unignored
   → the 5 s auto-commit commits your blobs and your SQLite index. **Fix for 2 and 3: append
   the missing line; never skip the file.**
4. **✅ FIXED — `commit_all` did `git add -A`.** It stages **exactly the paths `put`/`delete`
   recorded** — the plan's own requirement, not the directory approximation it passed through
   first — so a project repo's half-written code is untouched, a curated index survives, and a
   note you are hand-editing in Vim is not caught mid-sentence. The write-list is deliberately
   *not* on the `Store` trait (that seam carries no paths, which is what keeps a storage swap a
   backend change); `Vaults.store` is a concrete `MultiStore` and reaches it there. **And
   restic is fixed too**: it snapshots the notes dir + `blobs/`, never the vault root, so a
   project vault stops putting your `.env` and `data/` into a repo that may be a lab's.
   The trade, stated: a note edited outside the app is now never committed *by* the app.
   Original finding: → commits half-written code every 5 s and destroys a
   staged index. Across ten project repos this is the primary failure, not an edge case.
   **Fix: `git add` only the paths formicaria actually wrote** — `put` knows the file it just
   wrote, so `FileStore` hands git an exact list. Not `-A`, and *not even the whole notes
   dir*: your own hand-edits to `docs/`, mid-sentence in vim, must never be committed by us.
   **And restic** snapshotted the whole vault path → the lab's restic repo would hold your
   `data/` and `.env`. It now takes the notes dir + blobs dir instead.

**V3 — the descriptor. ✅ SHIPPED 2026-07-18** (`fm-core/src/descriptor.rs`). `vault.json`
carries `name`/`description`/`notes`; `FileStore::named` reads it, so a repo whose notes live
in `docs/` is adopted with no import step and new notes land beside the existing ones rather
than in a `notes/` dir nobody asked for. Every field optional, absent file = today's
behaviour. Two things the build added to the design: **name precedence is caller > descriptor
> directory** (`FileStore::open` now passes no opinion, since the directory name is the
*weakest* signal — it is whatever git called the clone — and passing it made the descriptor
unable to ever win); and `list_vaults` fills a blank config name from the store, or adopting a
repo showed a vault called `""`. **A `notes` path that escapes the vault is refused**:
audience is decided by location, so notes outside the repo would make "who can see this"
unanswerable. Original design:

`<vault>/vault.json`, git-tracked, **bounded by one rule: every
field must be a fact git cannot supply.** Git already knows authorship and history
(`git log`), the audience (`git remote` + who can clone), and — neatly — `.gitignore` *is
already* a truth-vs-cache declaration. So: no author, no collaborators, no remote, no
history. Three things are left: `name`, `description`, `notes` (where the notes are). Read in
`FileStore::named`, which already holds `notes: PathBuf` as a field — close to a one-line
change. Optional throughout; absent = today's behaviour. **Rejected:** a descriptor declaring
*two* locations (read-here/write-there silently moves notes on first edit).

**V4 — adoption. "A doc reads for free; it becomes a note when you use it."** Any `.md`
renders and searches with a **transient, index-only id** — nothing written to the user's
repo. The first time you **cite or edit it**, it is stamped with a real ULID. Same
promote-by-gesture pattern as ruling A above, and it is what makes rendering every repo's
docs actually free: ten commits putting ULIDs in READMEs your collaborators read is a bad
trade. It also retires the objection to derived ids — *"they break links when a file moves"*
only bites if you can link to them, and **you cannot until promotion stamps a real one**.
While transient, `git log --diff-filter=A` supplies the real `created`, so a promoted note
keeps its true dates rather than the day you converted — *use git for what only git knows*.
**The one thing config and a parser cannot cover is writing:** `path_for(id) =
notes.join("{id}.md")` — the filename *is* the id, so `docs/installation.md` reads perfectly
and the first edit writes `docs/01KX….md`, orphaning the original. So **the index records
each note's path and `put` writes back where it found it**, falling back to `{id}.md` for
notes we create — which is also what makes a recursive walk safe (`reindex`'s `read_dir` is
flat today, so nested Markdown is invisible). **`FileStore::skipped()` must reach the GUI
first** (it is stderr-only) or a failed adoption is indistinguishable from an empty vault.

**Rulings carried:** formicaria never "owns" a repo — every repo it touches is the user's,
local and writable, and multiple writers is what git is *for*; it always acts as though it
owns it. (A "guest vs owner" mode was proposed and rejected: the distinction that survives is
**whose commits**, and git answers that.)

### Track M — formicaria on the phone (planned 2026-07-18)

**The owner overrode `MASTERPLAN:57`.** Not "phone as a thin client to the laptop's server" —
the **app runs on the phone itself**, so you collaborate with yourself (and others) across
devices over the *same* git-repo vaults, with **feature parity of today's views** (notes, Board,
Agenda/Calendar, Timeline, Search, and the Excalidraw whiteboard), editable and merged on both
platforms. Multiple repos = N remotes = the desktop `MultiStore` model, unchanged. Constraint:
**minimal decade-scale maintenance.** Recommended: **Tauri v2 mobile, Android first, reusing the
entire Rust core** — the phone is a *thin frontend over the same shared core*, nothing forks.

Full design, code audit, and staged sequence (spikes → M0–M8) in
[`mobile-design.md`](./mobile-design.md). This entry carries only the sequence-defining rulings
(the detail, and *why not the alternative*, is over there):

1. **One command surface — extract dispatch into `fm-app`. — SHIPPED 2026-07-18.** The tempting
   story ("`fm-app` is already fronted by `fm-serve` *and* `fm-cli*`, so nothing forks") was
   **false**: `fm-cli` reimplements against `fm-core`; `fm-serve::api()` was a second surface; a
   Tauri bridge would have been a third. `api()`'s `match` + the single-lock
   `Vaults{MultiStore + Vec<VaultConfig>}` discipline now live in `fm_app::dispatch`, with
   `vaults.rs` moved up beside them; `fm-serve` is a transport shell. Three deltas from the plan
   as written: the lock stayed **inside** `App` (a `&mut Vaults` parameter would hold it across
   `backup_status`'s per-vault `git ls-remote`), query params are **either/never both** with the
   JSON body (else an uploaded `.json` asset is read as its own arguments), and `open_external`
   became a one-method `Host` trait. `fm-cli` still has not migrated onto it — that remains the
   open half of "one command library". Detail in
   [`mobile-design.md`](./mobile-design.md#ruling-1--one-command-surface-the-load-bearing-correction).
2. **`git.rs`: subprocess `git` → in-process `git2` (libgit2), HTTPS-only. — ⛔ BLOCKED, do not
   build as written (audited 2026-07-18).** Three premises are wrong or unweighed:
   - **libgit2 cannot invoke external merge drivers.** Only text/union/binary are registered and
     it contains no process-spawn at all. Porting `pull()` would **silently disable the `.md`
     frontmatter merge** — the whole Phase 1 achievement — while a collaborator's terminal `git
     pull` still honours it: two merge semantics in one vault. No workaround exists
     (`git_merge_driver_register` is unbound, and it would not affect anyone else's git anyway).
   - **`deny.toml` forbids linking GPL code** ("GPL tools like pdftotext/libvips are invoked as
     subprocesses and never appear in this graph"). libgit2 is GPL-2.0-with-linking-exception;
     `cargo deny` passes it **only because `libgit2-sys` under-declares** as `MIT OR Apache-2.0`
     while vendoring ~230k lines of GPL C. `ci/third-party.sh` reads the same field, so we would
     ship binaries omitting a notice the exception requires. That is an owner policy call, not a
     silent pass.
   - The Android C cross-compile is real but *not* the blocker the ruling thought it was
     (`libgit2-sys` builds via `cc`, no cmake needed).
   `gix` remains the documented pure-Rust future swap; its push still is not shipped.
3. **Merge body: `git2::merge_file`/`MergeFileOptions`. — ⛔ BLOCKED as written: that function does
   not exist** (audited 2026-07-18). git2 0.20.4 exposes only
   `Repository::merge_file_from_index`, which needs `IndexEntry`s and would pollute the ODB —
   contradicting `merge.rs`'s own design. The buffer API `git_merge_file` *is* bound in
   `libgit2-sys`, but git2 imports that crate **privately**, so this needs a direct `libgit2-sys`
   dependency plus ~40 lines of unsafe FFI — a different decision from the one written here, and
   it inherits ruling 2's licence question. The plan's "verified against the git2-rs docs" was
   not. If it is ever done: the desktop `.md` driver **stays installed**, driver + both app-pulls
   call the same `merge_files`, and a differential test (new engine vs `git merge-file` over
   random triples) gates the swap.
3b. **The whiteboard merge — ✅ SHIPPED 2026-07-18, and it needed none of the above.**
   `crates/fm-core/src/scene.rs` merges scenes element-wise from `merge_body`, before the text
   merge. **There is no `.excalidraw` file and no second driver** — `FileStore` writes
   `<ulid>.md` and the existing `*.md merge=fm` attribute already routes board notes into
   `merge_files`; text elsewhere implying a separate driver would have had someone build one that
   never fires. Verified through **real git** in `crates/fm-cli/tests/merge.rs`.
4. **Auth: PAT-first (user-owned, host-agnostic, un-vendored); OAuth device flow optional.** Token
   Keystore-encrypted — a **scoped reversal** of "the app stores no secret" (a phone has no
   ambient credential-helper). Inject a per-URL `CredentialSource` into the three network fns
   (serves multi-repo + refresh + tests); **`set_identity` on clone** so mobile commits carry real
   provenance (the `PLACEHOLDER_EMAIL` sentinel).
5. **Build the real streaming `GET /api/blob/<hash>`. — SHIPPED 2026-07-18 (desktop half).**
   `fm-serve/src/blob.rs`: streamed in 64 KB chunks, `sniff_mime`, `Accept-Ranges`, `Range`
   (206/416); the UI's inline media points at it and object URLs survive only for the mock
   backend. It also turned out to be a **security** change — a blob is now at a navigable
   same-origin URL and blobs come from collaborators — hence `nosniff` plus
   `Content-Disposition: attachment` outside an inline-safe allowlist. Mobile's `blob://`
   protocol handler is still to build, but now against a working reference.
6. **Efficiency — ✅ SHIPPED 2026-07-18, with one deliberate deviation.** Liveness and reindex are
   two beats now: `POST /api/alive` (transport-level, no lock, no filesystem) at 15 s, and the
   `ping` reindex at 15 s **only while the tab is visible**, plus an unconditional refresh on
   `visibilitychange`. The watchdog idle window went 10 s → 90 s — and *that*, not the split, is
   what fixes the documented background-tab kill, because a throttled `alive` beat is throttled
   exactly as much as a throttled `ping`. **Deviation: desktop keeps a poll.** Fully
   lifecycle-driven refresh is a real desktop regression — a visible-but-never-refocused window
   (formicaria tiled beside Vim, or on a second monitor) fires no lifecycle event and would simply
   stop updating. The battery argument that motivates dropping the poll is a phone argument.
   **Incremental cold-start open was tried and rejected (2026-07-18)**: mtime-only detection is
   blind to `restic restore`/`rsync -a`/`cp -p`, and the full rebuild at open is the only thing
   that heals them; it also needs an index-format version gate nothing enforces and an
   `objects(path)` index (`forget_path` full-scans, so Incremental can be *slower* than Full
   after a big pull). The benefit is mobile-only — desktop starts once. What the audit did find
   was a live bug: the deletion sweep was O(notes²) on every 3 s beat; fixed and pinned by a perf
   budget. **Auto-push is explicit — ✅ SHIPPED 2026-07-18** as `ui/src/lib/sync.svelte.ts`
   (`commit → push`, on reject `pull → merge → push once more`), never the silent-loop the naive
   "auto-push after commit" would wedge: **exactly one retry**, and **never a push after a
   conflicted pull**. Wired to the backup panel's button and to the "get changes" nudge.
   **Nothing fires it on a timer** — whether writing a note should publish it unasked is an
   outward-facing default, and the shipped design says backup is "a conversation, not a
   fire-and-forget", so it wants a decision rather than an assumption.
7. **Mobile shell stays trivially CSS — ✅ SHIPPED 2026-07-18.** A media-query reflow of the
   already-tested shared renderers and no new stateful layout: the pane workspace collapses to
   one column (`--cols` is overridden, not read — a workspace saved on a laptop must not arrive
   on a phone as four 4rem columns), the top bar wraps, the board snap-scrolls one column at a
   time, and `pointer: coarse` bumps the 3px-padding targets to ~44px. Board's touch gap
   (pragmatic-DnD's element adapter doesn't fire on touch) got the **tap→move-to-column
   fallback — which was the only option**: `@atlaskit/pragmatic-drag-and-drop` 2.0.1 ships element/external/text-selection and
   **no pointer adapter**, nor does any of its 12 companion packages (audited 2026-07-18), so
   earlier text offering "or swap in the pointer adapter" was wrong. The fallback is cheap
   because `Board` already has `columns: {value,label}[]` and `onmove(id, value, beforeId)`, so
   the menu reuses the desktop write path with no new command. Watch `ci/checks.sh` — it greps
   `ui/src/renderers` case-insensitively for a whole-word `todo|doing|done`, so a "Done" button
   label would fail the build — the menu's labels come from `col.label`, i.e. runtime data, so it
   stays as generic as the drag it replaces. Built as real `<button>`s rather than touch
   handlers, which is what makes it **testable without a phone**: a jsdom click exercises the
   same path a tap does (`ui/src/renderers/Card.touch.test.ts`, 5 tests). What CI still cannot
   check is narrowed to "is the target big enough for a finger", not "does moving a card work".
8. **Sync is a seam, git one provider** — a thin `SyncProvider` trait with `GitSyncProvider` as
   the *sole* impl; build no second provider now (design against Syncthing's profile *on paper*;
   make `history` a queried optional capability). Backends default free & serverless (rclone → ~70
   backends; a free GitHub/GitLab private repo is the zero-server option), self-hosted first-class.
   Keeps "no CRDT / no sync framework" intact.

**Open fork (per-install config, not a rewrite):** **Path B** (on-device `git2`) is the
destination — the real "the app runs on the phone." **Path A** (transport-only; desktop records,
a free transport replicates) is a low-risk *early demo* — but it retreats toward the
satellite-of-desktop model the owner overrode and fails a phone-only collaborator, so it is not
the end state. **Top risk:** the Android SDK/NDK are not conda-packaged, so the toolchain escapes
pixi — pin the whole matrix in CI (this ≠ `pixi.lock` reproducibility).

## Cross-cutting decisions carried in (don't re-derive)

- **Real-time never exists.** Target fast-async; a bare git remote *is* the coordinator (a
  push is an atomic compare-and-swap on a ref). No CRDT.
- **Vaults are audiences.** One vault = one repo = one remote = one collaborator list; a
  *set* of vaults, not one shared vault.
- **Replicate, don't RPC.** `MultiStore` over local clones; laptops sleep, availability
  beats freshness.
- **Awareness over enforcement.** "Ravi pushed 2 min ago" is a `git log` query and beats
  any lock; **no lock ref**. Ephemeral state is never a file.
- **Branches/PRs are overkill for daily notes** — earn their place only for an *agent
  proposing* changes to shared knowledge (human gate) or a big restructuring.
- **The always-on peer's guest web UI is out of scope** until someone writes its threat
  model (`fm-serve` is localhost-only, no TLS, one write lock). Take only the bare-repo
  half.
- **Heavy assets: a blob mirror is distribution, restic is backup.** Content-addressed +
  immutable = `rclone copy vault/blobs remote:blobs`, idempotent by hash. Decide it once,
  in the mirror context, not twice.

## The code wins — stale lines to distrust in MASTERPLAN

`formicaria/MASTERPLAN.md` is the canonical *spec* but has drifted from the code; per this
folder's own rule, **the code wins**. Known-stale, left uncorrected in the spec on purpose
(flagged here instead of churning a 51 KB doc):

- **Auto-commit self-contradicts:** `:391` lists it *not built*, `:426` lists it *shipped*.
  Reality: it exists, debounced **5 s** with **no blur handler** (`App.svelte`), best-effort
  and silent (`known-issues.md`). The spec's "500 ms→disk, 30 s/blur→commit" is aspiration.
- ~~**`update_body`'s mtime check (`:319`) does not exist**~~ — **the spec is right again**,
  by accident of wording: Phase 1 put the check in `FileStore::put`, under `update_body`
  rather than in it, so it now covers `set_property` and every future writer too.
- **`pulldown-cmark → HTML` never materialized** — the only HTML assembly is one
  `marked.parse()` in `render.ts`.
- **`:57`'s "mobile = a server + auth decision" (phone as thin client) is overridden** — Track M
  runs the app **on the phone itself** (see `mobile-design.md`, `decisions.md`).

## Verification (each slice, when it ships)

- `pixi run ci` green. (This file lives in `docs/context/`, which is **not** in the mdBook
  `SUMMARY.md`, so it never enters the `docs` build — keep it out of `docs/src/SUMMARY.md`.)
- **Phase 0**, testable without a UI: a note with conflict markers must not stop
  `FileStore::open`; `commit_all` refuses while `MERGE_HEAD` exists (✅ regression-test it);
  `push_squashed` refuses when the tracking ref isn't an ancestor of `HEAD` (✅ the test is:
  fetch, push, assert their commit still reachable).
- **Lost-update test is `set_property`, not just `update_body`:** pull a change, drag the
  card, assert the pulled body survived.
- **Byte round-trip first:** `vault:` never appears in any `.md`.
- `MemoryStore`/`FileStore` equivalence still holds across the `candidates` refactor; search
  still uses FTS5 when federated (a PDF's extracted text is still findable).
- **`.md` driver:** two clones edit *different lines* of one note → clean auto-merge, no
  `updated:` conflict. That is the test that says shared vaults work.
- **`.excalidraw` driver:** two clones move *different* shapes → clean merge; the *same*
  shape → deterministic winner, no markers; **one clone deletes a shape the other didn't
  touch → it stays deleted** (the case `reconcileElements` cannot pass).
- Track S: ICS re-pull is idempotent and **never rewrites a body**; `GET /calendar.ics`
  round-trips a `start`/`due`; a board embeds into a note and prints to PDF with math/images
  intact.
