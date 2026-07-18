# Plan — formicaria, one sequenced program

The single **forward** document: what we intend to build next, in order, and *why not
the obvious alternative*. It supersedes and folds in the old `roadmap.md` (near-term
single-user work) and sits above [`collaboration-design.md`](./collaboration-design.md),
which stays as the line-by-line **code audit** behind Track C — this file carries the
sequence and the rulings; that file carries the receipts.

Like the collaboration doc, most of what's below does **not exist yet**. When a line
ships, delete it here and fold the outcome into `overview.md` / `decisions.md`.

_Last updated: 2026-07-17, after **Track V's V1 shipped** (the first-run gate + the vault
list's first writer — `sessions/2026-07-17-create-vault.md`) and the UI direction was set:
**sanitize `render.ts`, de-modalize the note trail, then earn `.view`** (the layout ask
turned out to be `MASTERPLAN:323`'s own deferred design, not a new feature). Before that,
**Track C Phases 0, 1 and 2 shipped** and the rename landed with them: a shared vault works
end-to-end and the plural is literally true. **Next: Track V's V2 (four live co-tenancy bugs,
two silent) gates V3–V4; the UI track is independent.** Everything not marked SHIPPED still
describes things that do not exist._

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

### B. Board images — strip to the blob store on save

Excalidraw embeds a pasted image as a **base64 data URL inside the note body**, rewritten
whole on every pointer move — routing bulk binary through git and breaking the
"blobs are already out of git" premise. **On board save, strip inline `files` out of the
scene into the content-addressed blob store** (reuse `BlobStore::put_bytes` + ingest's MIME
sniff), leaving only blob references in the `.excalidraw` JSON; **rehydrate on load** via
the existing `resolve_asset` / planned `GET /api/blob/<hash>` route. Must land **before
boards are shared** (Phase 3, Track C). **Rejected:** accept-and-document — a 2 MB
screenshot becomes ~2.7 MB of churn per stroke.

## The sequenced program

Three tracks. **Track S** (single-user near-term) is independent — ship any item anytime.
**Track C** (collaboration) is strictly ordered and **Phase 0 gates everything**.
**Track V** (a vault is a folder you already have) is ordered too: **V2 gates V3–V4**. The one
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

**Still open, small:** `remote_moved` is computed in `backup_status`, so it is only
consulted when the backup panel opens — the plan's *automatic* 15–30 s poll (be told
without asking) is not wired. Conflicted notes are listed by name in that panel, and the
driver puts markers in the **body** so they open in the editor — but there is no
list outside the panel. `FileStore::skipped()` is still stderr-only.

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

- **The `.excalidraw` 3-way merge driver, in Rust** (~80 lines in `fm-cli`) over git's
  `%O` base — **not** Excalidraw's `reconcileElements`, which is a base-less 2-way union
  that resurrects every deleted shape. The 3-way rule recovers deletion intent: in base &
  gone one side → honour the delete; absent from base → an add, keep; in both → higher
  `version` wins, tie-break lower `versionNonce`. Watch the fractional `index` (z-order is
  the unlisted merge hazard). **Lands after Ruling B's image-strip.**
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

**V2 — co-tenancy correctness. NOT BUILT, and it gates V3–V4. Two of the four are silent.**
Every one is a **live bug today**, verified by reading the code, not hypothesised. They only
bite once a repo holds work that isn't ours — which is exactly what V3 enables, so this lands
first.

1. **`push_squashed` eats the user's own commits.** `reset --soft <tracking-ref>` collapses
   *every* unpushed commit, so three hand-written manuscript commits become one `backup:`.
   Its own comment justifies the squash by "auto-commit fires every few seconds" — that
   justifies squashing **ours**, never theirs. **Fix:** compute `base` as the newer of
   `tracking(vault)` and the most recent commit we did not write, discriminated by the
   **`auto:`/`backup:` message prefix — never the author**: a PC has one git user and we
   commit *as* them, so author-based detection cannot work. Dedicated vault → every unpushed
   commit is ours → identical to today. No mode, no flag; derived from the history.
2. **`.gitattributes` is skipped if the file exists** (`git.rs`), and every real repo has
   one → `*.md merge=fm` never lands → **the merge driver silently never engages** → every
   concurrent edit collides on `updated:` inside the YAML fence and the note stops parsing.
   *This is Track C Phase 1's disaster, reintroduced by conversion.*
3. **`.gitignore` is skipped if it exists** → `blobs/`, `index.sqlite`, `derived/` unignored
   → the 5 s auto-commit commits your blobs and your SQLite index. **Fix for 2 and 3: append
   the missing line; never skip the file.**
4. **`commit_all` does `git add -A`** → commits half-written code every 5 s and destroys a
   staged index. Across ten project repos this is the primary failure, not an edge case.
   **Fix: `git add` only the paths formicaria actually wrote** — `put` knows the file it just
   wrote, so `FileStore` hands git an exact list. Not `-A`, and *not even the whole notes
   dir*: your own hand-edits to `docs/`, mid-sentence in vim, must never be committed by us.
   **And restic** (`backup.rs:57`) snapshots the whole vault path → the lab's restic repo
   holds your `data/` and `.env`. Snapshot the notes dir + blobs dir instead.

**V3 — the descriptor.** `<vault>/vault.json`, git-tracked, **bounded by one rule: every
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
