# Archived — `overview.md`'s "Last verified" narrative (through 2026-07-24)

This is the running session-log narrative that had accumulated in `overview.md`'s preamble
(lines 6–283) before the 2026-07-24 context de-bloat. It was lifted here verbatim rather than
deleted: its durable facts already live in `decisions.md` and the dated `sessions/` entries, and
the always-loaded `overview.md` is now a current-state model, not a log. Kept one `ls` away for
recall; superseded as the source of truth by the files it points at.

---

_Last verified: 2026-07-22 — **the study assistant now runs ON-DEVICE on both platforms.** A local
LLM answers `@name` mentions in a note's discussion, researches (opt-in web), and drafts note edits —
opt-in, notes stay local. **Phone:** in-process, the arm64 `llama-server` bundled in the APK's
`jniLibs` (run from `nativeLibraryDir` via `/proc/self/maps`, no JVM shim), the model fetched on first
enable (`ureq`/`rustls`, resumable), kept alive by a foreground service; on/off toggle with clean
shutdown (`RunEvent::Exit` + `PR_SET_PDEATHSIG`, no orphan); runs `lfm2.5-1.2b` (~16 tok/s, 4 of 8
cores). **Laptop:** `agent-serve` subprocess on the **RTX 3050 via the Vulkan runtime** (no CUDA
toolkit), `qwen3-4b-2507` (~52 tok/s, in VRAM). **One shared runner** (`fm-agent-run`); the only
platform code is the launch/transport seam. Config-driven (`agents/models.toml`: per-device
default/threads, `gpu=auto` with CPU fallback, `max_reply_chars`). Context is the host note + its
**linked notes** (one hop, text only) + the discussion — no vault-wide RAG. The agent watches **note
comment threads** too (`thread_roots`, not just first-class `discussions`). Model picks are
evidence-based (`model-benchmarks-`/`model-selection-research-2026-07-22.md`). Reversed the earlier
FFI-everywhere plan (`forward-plan-review-2026-07-22.md`). Full arc:
`sessions/2026-07-22-agent-shipped-on-device-and-hardened.md`. Prior:
2026-07-20 (evening) — four defects found by reviewing this repo's own code
were fixed, then re-reviewed adversarially and four of the fixes found incomplete and repaired:
`copy_note` was still leaking through `status`/`tags`, the `manifest.json` merge had a deletion
rule the format cannot support, its driver could exit non-zero (which git reads as a conflict on
a clean file), and the commit-freeze fix missed the auto-commit caller. `fm-serve` now sends a
Content-Security-Policy; the Android shell has one too, launched clean on the phone with zero
violations, though images and boards are still unopened there. Prior entry:
**media works on a phone, end to end.** A file attached from a note
reaches `ingest_bytes` and renders back in the note. It travels as base64 through the `fm_ingest`
IPC command, because Android delivers **no request body** to a custom-scheme handler and no raw
IPC body either — a POST arrives empty and silent, which stored every photo as zero bytes until
the repeated empty-string hash gave it away
(`sessions/2026-07-20-capture-on-a-real-phone.md`). Video is still capped and refused, pending
somewhere for a phone's blobs to survive. Prior:
2026-07-19 — **a private git repo now clones onto a phone.** The trust store is
loaded into libgit2 **from memory**, because `openssl-src` builds every Android target with
`no-stdio` and no file-based certificate loading can work there at all — five diagnoses were spent
producing better files before that was found (`sessions/2026-07-19-git-on-the-phone.md`). Prior:
**`fm_core::vcs` is now the only way the app reaches git** — and as of 2026-07-24 that is
**grep-enforced** (`ci/checks.sh`) rather than merely intended: the libgit2 backend existed, was
differentially tested, and was called by nothing, so a phone reported "git not installed" while
carrying a working copy of it. The first pass fixed the *history* call sites and the claim was
written as though finished; the entire **proposal lifecycle** was a second wave still naming
`git::` directly, so a phone could not create, review, accept or reject a proposal at all —
which is every `/transcribe`, `/propose` and `/research`
(`sessions/2026-07-24-proposals-on-the-phone.md`). A vault can also now be
**acquired from elsewhere**, not only created
locally: `clone_vault` (git, with history) and `restore_vault` (restic, notes + media, no
history), both through the one shared `acquire::naturalise` step that strips per-machine state.
The same pass fixed a live silent-data-loss path — a stale absolute `merge.fm.driver` made git
report a conflict on a file that looked clean, so "resolving" it deleted a collaborator's edit
(`sessions/2026-07-19-acquiring-a-vault.md`). Prior: 2026-07-18 — **the compiled work queue was
emptied**
(`sessions/2026-07-18-vault-as-a-repo-and-the-queue.md`). Unreadable notes are **named in the
app** instead of on stderr (they were absent from every view with no explanation — the one
place the app knew a note was missing and told only a terminal); the board's column drag
**does something again** (`boardOrder.ts` was a tested pure core that nothing imported any
more — *a unit test cannot catch a caller that stops calling*); `fm-serve`'s CSRF and `Host`
guards have socket-level tests, which caught the `Host` guard refusing `[::1]:8765` — a
request from ourselves; `mock.ts` binds each arm to its real DTO, so a shape the Rust never
sends now fails `check-ui`; the auto-commit stages **exactly the paths `put`/`delete`
recorded**, so a note you are hand-editing in Vim is not swept in; and the lost-update token
is a **content hash** rather than the `updated` stamp, because a stamp only moves for writers
that bump it and Vim does not (1.6 ms in release on a 2.8 MB whiteboard body, pinned by a perf
budget). `fm-cli` now shares `commands::asset_note` rather than rebuilding it — its copy had
already drifted, never setting `obj.vault`. Before that, **A vault can be a repo you already have**
(`plan.md` Track V). Three of V2's four co-tenancy bugs are fixed, including both silent ones:
`.gitattributes`/`.gitignore` were **skipped when the file already existed**, so in any real
repo `*.md merge=fm` never landed (the merge driver silently never engaged — Phase 1's
disaster reintroduced by conversion) and `blobs/`+`index.sqlite` were committed and pushed;
`commit_all`'s `git add -A` staged your half-written code and your curated index every 5 s;
and `push_squashed` collapsed hand-written commits into one `backup:` (the boundary is now the
`auto:`/`backup:` message prefix — **never the author**, since we commit *as* the user). V3
shipped too: **`vault.json`** (`fm-core/src/descriptor.rs`) carries the three facts git cannot
supply — `name`, `description`, and **where the notes are** — so a project whose notes live in
`docs/` is adopted with no import step. Also decided, under the project's own principles:
**the git2 swap is rejected** and "git is a capability, not a dependency" stands (linking
libgit2 contradicts shell-out-don't-link, fails `deny.toml`'s permissive-only rule on
everything but a metadata technicality, and would silently disable the `.md` merge driver);
and **whiteboard blobs will be git-tracked** when the image-strip lands, which is deferred
because boards work today and a canvas cannot be verified without eyes on it. Before that,
**Sync is an explicit sequence now, whiteboards merge, and three
plan items were stopped before being built**
(`sessions/2026-07-18-sync-loop-and-scene-merge.md`). `ui/src/lib/sync.svelte.ts` is the loop:
`commit → push`, and on a rejection `pull → merge → push once more` — **exactly one retry**, and
**never a push after a conflicted pull** (publishing conflict markers as content is worse than not
publishing). `crates/fm-core/src/scene.rs` merges whiteboards **element-wise** before the text
merge ever sees the JSON — deletions honoured against the base (which is precisely what
Excalidraw's base-less `reconcileElements` cannot do), higher `version` wins, lower `versionNonce`
breaks ties, fractional `index` keeps the z-order — and it is **not** a second merge driver: there
is no `.excalidraw` file, `FileStore` writes `<ulid>.md` and `*.md merge=fm` already routes boards
there. Liveness split from reindex (`POST /api/alive`, a 15 s beat; the reindex beat is 15 s and
**visible-tab-only**; the watchdog idle window 10 s → 90 s, which is what actually fixes the
background-tab kill). Six live bugs fixed — the last three found by audit *this* session and closed rather than
inherited: a **duplicated `id:`** made the poll re-index and refresh forever (first path now
wins, the other is named); **no `Host` validation** left a DNS-rebinding path straight past the
CSRF guard; and a **surfaced error was wiped** by any background refresh. Plus: auto-commit committed **only the default vault**;
the "get changes" nudge pulled **without committing first** (git will not merge over a dirty tree —
the backup panel already knew; the same rule existed in two spellings); and **a stale editor could
overwrite a merge that landed under it** — `update_body` now takes the **version** the caller
last saw (the body's hash) and refuses a superseded write. The mtime guard in `FileStore::put` could not cover that
one, because `pull` reindexes right after merging and so re-arms the very mtime the guard compares:
*noticing the change is what disarmed the protection against it*. On rejection the pane reloads and
puts the unsaved draft back **below** the merged text with markers — the `.md` driver's stance, one
level up. **Stopped on purpose,
each now a blocked task:** the **git2 swap** (`git2::merge_file` does not exist; libgit2 **cannot
invoke external merge drivers**, so porting `pull()` would silently disable the `.md` driver while
a collaborator's terminal git still honours it; and `deny.toml`'s permissive-only rule is violated
in spirit — it passes only because `libgit2-sys` under-declares its licence), the **Excalidraw
image-strip** (`blobs/` is gitignored, so stripping stops shared boards showing images, and the
two options are undecided), and **"swap in pragmatic-DnD's pointer adapter"** (2.0.1 ships no such
adapter). Before that, **Track M's host-side band shipped: there is one command surface
now, and blobs stream** (`sessions/2026-07-18-dispatch-and-blob-route.md`). **`fm_app::dispatch`**
is the single door to every command — `fm-serve`'s `api()` match, the `Vaults` state and its lock
discipline moved into `fm-app` (and `vaults.rs` moved up with them), leaving the server an HTTP
shell that parses a request into `(cmd, args, body)`, calls `dispatch`, and frames the `Output`.
That is Track M ruling 1, and it *precedes every mobile milestone*: until there was one door, each
new frontend was another copy of the dispatch table (`fm-cli` is the standing proof — it
re-implements against `fm-core` instead of calling `commands`). Three corrections the build made
to the plan: the lock could **not** move into the signature (five arms drop it before slow I/O —
`backup_status` shells out per vault), query params are **either/never both** with the JSON body
(else a `.json` asset would be read as its own arguments and file itself into another audience),
and `open_external` became a one-method **`Host` trait** rather than a `#[cfg]` ladder. Alongside
it, **`GET /api/blob/<reference>`** finally exists (ruling 7): streamed from disk, the sniffed
`Content-Type` that `resolve_asset` used to throw away, `Accept-Ranges` and real `Range` — so
`<video>`/`<iframe>` range-request instead of buffering a whole file into RAM twice. Serving blobs
from a *navigable* same-origin URL is a security change too, so anything outside an inline-safe
allowlist is sent `Content-Disposition: attachment` (blobs arrive from collaborators; an SVG
rendered as a document would run script in this origin). And **the 3 s poll stopped being
O(notes²)** — the deletion sweep tested membership against a `Vec`; 453 ms → 50 ms per quiet beat
at 10k notes, now pinned by a perf budget. The plan's cold-start `Incremental` switch was
**rejected on purpose** (mtime-only detection is blind to `restic restore`/`rsync -a`, and the
full rebuild at open is the only thing that heals them — see known-issues). `pixi run ci` gained
**`check-ui`**: `vite build` never typechecked, so a component calling an unimported function
built clean and threw in the browser — which is exactly the bug this session introduced and
shipped into `ui/dist` before catching it. Before that, **Track M — mobile was *planned*** (not built): formicaria on the
phone **itself**, overriding `MASTERPLAN:57`'s "phone = thin client" framing — one shared Rust
core, git-coordinated across devices (`sessions/2026-07-18-mobile-port-plan.md`; design in
[mobile-design.md](../mobile-design.md), rulings + the two reversals in
[decisions.md](../decisions.md), sequence as Track M in [plan.md](../plan.md)). Nothing mobile
exists yet. Before that, **Collaboration is git, exposed** (not reimplemented)
(`sessions/2026-07-18-git-collaboration-visualized.md`). One read-only `git::activity` (a
`git log --name-only` over `notes/*.md`, where a file's stem *is* its ULID) yields each note's last
editor, and that single command powers all of it: **`EditedBy` labels** ("● name · 5m ago") on
every card and the open note (person-coloured by the shared `hashHue`, delivered via the
runes-in-module store `activity.svelte.ts`, no prop-drilling); a first-class **Activity pane** (git's
log as a workspace view); a **contributor filter** (chips like the vault filter — `App.shown` gained
an author check, so hiding a person applies everywhere); and **automatic "someone pushed" awareness**
(a slow visibility-gated `remote_moved` poll → one-click `pull`). Nothing is stored — git stays the
source of truth. Deferred: creator attribution, anchored comments (need backlinks), live presence
(needs the descoped peer). Before that, **Cross-vault: create-in-vault + restrictive copy**
(`sessions/2026-07-18-cross-vault-copy.md`). You can now pick which vault a new note/board is
born in (a top-bar destination picker; `capture` gained a `vault` arg, mirroring `ingest`), and
**copy a note into another vault**. Copy is **restrictive by default** — only the prose travels;
`fm-app/refs.rs::strip_cross_vault` drops every `note:`/`asset:` reference (label and all) so a
copy can never point outside its new vault (ideas flow, artifacts don't). A copy is a **new note**
(fresh ULID); opting into "also copy the files" carries the first-degree blobs *into* the target
(content-addressed dedup, manifest refreshed) so it's self-contained; note links stay stripped
(linked-notes tier deferred). Every copy is behind a plain warning and leaves an `uncopy_note`
**Undo**. `Object.vault` is still never a field — audience is set by *which FileStore receives the
put*. Before that, **The UI is a flexible pane workspace now**
(`sessions/2026-07-18-flexible-workspace.md`). The single global view became a **CSS grid of
panes** the user opens, reorders (drag the header grip), resizes (drag the corner), and closes;
a horizontal **top bar** replaced the tall left rail (the explicit "sidebar wastes vertical
space" complaint). `panes.ts` is the pure core — `Pane`/`Workspace`, `MAX_PANES=8`, a
`Math.random`/`Date`-free `paneId()`, and the load-bearing **feed-key dedup**: N panes over M
distinct feeds cost M fetches, not N. Each pane's `onmove` carries its **own** `groupBy`, so two
boards no longer write each other's property. The whole `Workspace` persists to
`localStorage['fm-workspace']` (a view preference, never a vault). Deliberately a flat pane list
+ spans, **not** hardcoded preset splits and **not** a recursive split tree — the ask was
"rearrange them where I want," and the split tree is the one shape with no stopping point.
A note is a **pane kind** too now (`kind:'note'` → `NotePanel`, board-notes → whiteboard), so
the old side-trail is retired; `openNoteInPane` dedups by id so one note never opens two editors
over one file. Verified structurally (CI green, 164 UI tests) but **not seen** — the browser
extension is not connected, so the owner's eyes remain the layout check. Before that, **`.view`
files: saved queries, any renderer**
(`sessions/2026-07-17-view-files.md`). A `.view` (YAML, in `vault/views/`, git-tracked) is
`query + a renderer` — `MASTERPLAN:329`'s own deferred design, now built. Parsed server-side
in `fm-app/views.rs` → `fm_query::Query`; the UI sends a **name** (`list_views`/`run_view`),
so no `Query` crosses the wire — which keeps serde off the pure crates and the
`PropertyValue` `Ord` trap unreachable (the DSL has no ordered `prop` comparison; dates go
through `date:`). A `.view` **extends** a preset's filter by `Vec::extend`, so `Kind(Note)`
(the assets exclusion) is never forgotten; a broken view is listed with its parse error, not
dropped. This gives the engine's previously-unreachable predicates
(`Not`/`Any`/`TagsAll`/`TagsAny`/`DateRange`) their first callers. Before that, **the read
view sanitizes, and the note trail stopped being modal**
(`sessions/2026-07-17-ui-sanitize-demodalize.md`). `render.ts` now runs DOMPurify on
`marked`'s output before the DOM sees it — the top security item in `known-issues.md`, made
live by collaboration (a shared note body could run script in your origin and push your vault
anywhere); the config widens the URI allow-list by exactly `note:`/`asset:`/`sha256:` so our
own chips and inline media survive. And the note trail is a **peer grid column** now, not a
`z-50` overlay with a backdrop: the board stays live beside an open note (see the "Views"
note below). Before that, **A vault is now created, not invented**
(`sessions/2026-07-17-create-vault.md`). The `FM_VAULT` default (`"vault"`, *relative*) is
gone: unset means **zero vaults**, a real state that gates the whole UI on a first-run
screen, because a typo or a launcher started from another cwd used to silently create an
empty vault named after the mistake while your notes appeared to vanish. `vaults.json` gains
**its first writer** (`fm-app/src/vaults.rs`, then in `fm-serve` — value-tree merge, append-only, refuses a
file it could not parse, writes the whole live list so an `FM_VAULT` vault cannot vanish);
`check_path`/`create_vault`/`list_vaults`; `MultiStore::open(&[])` legal + `NoVaults` +
`add` (live, no restart); one mutex over the store **and** the list, since `api()` already
took them in opposite orders. Creation deliberately does **not** `git init` — `commit_all`
already does, and eager init inside a repo the user owns would `git init` a nested one
shadowing theirs. Before that, **Track C Phases 0, 1 and 2 shipped: a shared vault works, and
the plural is true** (`sessions/2026-07-17-phase-{0,1,2}.md`). Two people can now edit
different paragraphs of the same note and the merge is **clean**, across a *set* of vaults
each with its own repo and audience — verified as a real round trip through the API, not
just in tests. Phase 1 added: the **`FileStore::put` staleness guard**
(`StoreError::Conflict` — *the* lost-update bug, where a board drag rewrote the whole file
from a stale copy); **incremental reindex** + a local poll folded into the existing
`ping` heartbeat (without it a `git pull` is invisible, because the views are served from
SQLite) — *the fold was undone 2026-07-18: liveness is `POST /api/alive` now and the poll
is a separate, visible-tab-only 15 s beat*; **`git::remote_moved`** (one `ls-remote`, moves no refs) and **`pull`**; and the
**`.md` merge driver** (`fm-core/src/merge.rs`, `fm merge-md`, installed by `ensure_repo`)
which resolves `updated:`/`tags` structurally and hands the body to `git merge-file` — so a
conflict lands **in the body**, leaving the note parseable and editable. **Phase 2 shipped
too: the plural is now true** — a *set* of vaults, each its own repo and audience, under
one set of views (`Object.vault` derived from location and never serialized — *location is
the permission*; the **`candidates` seam** so `Store::query` is a default method and FTS5
federates for free; `MultiStore`; the vault list at `~/.config/formicaria/vaults.json`;
**per-vault git** — the backup panel is a list, not a form; **blobs searched across
vaults**; badges + a vault filter). **And the rename landed with it: the tool is
`formicaria` now**, the plural being the architecture, and nothing carries the old name.
One rule outlives it: `git.rs`'s `PLACEHOLDER_EMAIL` is a sentinel matched **by value**, so
changing it again would hand every vault still on the old value a "real" identity and
reopen the provenance hole Phase 0 closed (`decisions.md`).
Before that, **Phase 0 made the app survive a merge**: four fixes, each a single-user bug today and data loss the moment a vault is
shared: `reindex` **skips an unreadable note** instead of failing
`FileStore::open` (one conflicted `.md` used to brick startup — `fm-serve` names it on
stderr, though it is still invisible in-app: see known-issues); `commit_all` **refuses
mid-merge** rather than committing `<<<<<<<` as a note's content; `push_squashed`
**squashes only onto an ancestor**, closing a silent data-loss path that opens the moment
anything fetches; and a vault needs a **real committer identity before it can gain a
remote** — the backup panel asks for a name and email, but only of people git has never met
(`git::identity`, `backup_status.identity`). The `formicaria@localhost` placeholder is a
**sentinel** for audience-less vaults, matched by value (`decisions.md`). Before that, the **forward plan was consolidated** into [plan.md](../plan.md) (the program:
Track S single-user + Track C collaboration; `roadmap.md` folded in,
`collaboration-design.md` kept as its audit). Before that,
**backup is now two tiers** (`BackupPanel.svelte`):
the button opens a panel that sets the vault's **git remote in-app** and pushes
the **notes** by default (no app-held secret — ambient ssh/credential-helper),
squashing the unpushed window into one `backup:` commit; **restic (media
included) is an opt-in checkbox**. The panel states what each tier does and does
**not** carry, and reports whether the data actually **left the machine**
(`destination.ts` — a local path is a legitimate destination but must never be
called off-site). Before that, a **double-click in the read view opens the editor
with the caret on the word you clicked** (`locate.ts` maps by word *ordinal*, not
by source positions — see the session note for why). Before that,
**five UX fixes**: assets are
filtered out of `board`/`agenda`/`recent` **in the query layer** (`search`/`gallery`
still see them — that is how you find a PDF, and how `/` inserts one); a
**StatusChip** rotates status through the vault's own values from a card or the
open note; a dropped board card **keeps its position** in the column
(`fm-card-order` in localStorage, like column order); the `/` menu opens **at the
caret** and seeds with **recent notes**; and a note opens its editor on
**double-click** as well as from the **Edit button** (which stays on every note —
the gesture is a shortcut, not a replacement), with **Ctrl+S** to save. The
double-click now **lands the caret on the word you clicked** (`locate.ts`). Before
that: **notes reference notes**: `[Title](note:<ulid>)`
(deliberately *not* `[[wikilinks]]` — see the session note), inserted by the same
`/` menu as assets, rendered as a live title+status chip, and clicking one opens
the target as a **pane to the right** so the trail you followed stays on screen
(`openIds: string[]`). **The trail is a peer grid column, not a modal overlay**
(de-modalized 2026-07-17): the board stays live beside an open note, no backdrop dismisses
it — reading a note is no longer a *mode*, which is the thesis that a note *is* the task
*is* the card, made literal. `wide` (persisted) now means "the note takes the whole content
area" vs "docks beside the view". The `.app` grid is three custom-property columns
(`--rail`/`--main`/`--trail`) so rail-collapse and the trail compose without a
grid-template explosion.
Backlinks are still not built. Before that: notes+tags (no user "type"), optional settable
`start`+`due` **stamps that now carry an optional time** (`2026-07-20T14:30`), so
a meeting is expressible; calendar bars start→due, note panel full-screen
by default, red brand accent matching the app icon, launcher UX (release icon,
reopen, close-tab-quits), and **board notes** (freeform Excalidraw whiteboard,
lazy-loaded) — now **treated exactly like notes** (a "Details" props editor over
the canvas → agenda/calendar/board-trackable). **Sidebar is search-first**: the
persistent Search field is the primary input, with New note / New board buttons
below (quick-capture box removed). Gallery removed. On top of note-delete, media
copy-notice, column reorder, SVG fix. Next work is planned in
[plan.md](../plan.md) — the sequenced program (Track S:
calendar sync, whiteboard-in-note + PDF; Track C: collaboration)._
