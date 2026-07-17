# Decisions — the *why*

Condensed, load-bearing decisions and reversals. Each entry is: **decision —
why — consequence**. The canonical, fuller spec is
[`formicaria/MASTERPLAN.md`](../../formicaria/MASTERPLAN.md); this is the
quick-recall version. Newest first.

## formicaria: three pillars, one atom; renamed when the plural became true (2026-07-17)
**Why:** the tool grows into *knowledge management + task scheduling + collaboration*
without becoming three products. **Consequence:** those are three **views of one Markdown
file** — a task is a note with a `due`, a message a note with a target, a shared note a
note in a different repo — so *resist adding a fourth thing*. The full sequenced program
lives in [plan.md](./plan.md) (Track S single-user + Track C collaboration), with
[collaboration-design.md](./collaboration-design.md) as its code audit. **The name changed
only once it was true:** `formicarium`→`formicaria` (the plural = a *set* of vaults) waited
for multi-vault to ship, and landed with it on 2026-07-17 as its own commit — a rename
should read as "only strings moved". No code identifiers moved (`fm-*`/`fm` fit either
name). **Two literals still say `formicarium` on purpose**, and are commented in place
because they now look like a missed rename: `git.rs`'s placeholder identity is a *sentinel
matched by value* (renaming it hands every vault still on the placeholder a "real"
identity and silently reopens the hole Phase 0 closed — see the identity entry below), and
Excalidraw's `source` field is written into **every board's JSON on disk**, so renaming it
would rewrite every board file for a string nobody sees.

## Vaults are audiences: git is per-vault, blobs are searched, hiding is only a view (2026-07-17)
**Why:** multi-vault forced three questions the plan had collapsed into "wiring", and each
has a wrong answer that loses data quietly. **Consequences, in order of how badly the
wrong answer bites:**

**Git is per-vault, so the backup panel is a list, not a form.** One vault = one repo =
one remote = one collaborator list, so `commit`/`push`/`pull`/`set_git_remote`/
`set_identity` each take a vault, and `backup_status` returns one entry per vault: N
remotes, N identities, N unpushed counts, N "someone pushed". There is no honest way to
collapse those — a single "unpushed" across a set of vaults is a number about nothing. The
tempting shortcut, letting `push` mean "the first vault", is exactly the overstatement
`destination.ts` was written to prevent: a backup that silently skips the lab vault. The
verdict names the vaults that did **not** make it, because "your notes are backed up"
while one sat still is the one sentence this panel must never say. An **unknown** vault
name is an error, never a fallback: writing a lab note into personal is a disclosure git
history makes permanent, and the reverse loses it. *Identity is per-vault too* — a vault is
an audience, so the name on a lab repo need not be the one on your personal notes.

**Blob resolution searches every vault.** A `sha256:` reference deliberately does not say
which vault holds the bytes, and it must not: that is what keeps a cross-vault `note:`/
`asset:` link free and lets ULIDs stay the only identifier anyone needs. Searching is not
a shortcut, it is *correct* — content-addressing means whichever vault answers, the bytes
hash to the reference, so they are the same bytes. **Rejected:** vault-scoping references,
which would re-couple a note to a location and break the links Phase 2 gets for nothing.
*Ingest is the exception and takes an explicit vault*: a blob should land beside the notes
that will reference it, never in an audience that shouldn't have it.

**restic does not respect vault boundaries, and says so.** One repo, one password, every
vault — because restic is *backup*, not distribution: a snapshot is disaster recovery for
the person who owns all these vaults anyway, so mirroring git's boundaries into it would
buy nothing and cost a repo and a password per vault. The asymmetry with the git tier is
real, so the panel states it rather than implying otherwise with a per-vault checkbox that
doesn't exist.

**Hiding a vault is a view preference, not a permission.** The filter lives in
localStorage next to the column order, and filters client-side. It changes what is on
screen and nothing else; who can see a note is decided by which repo holds the file, and
nothing in a browser can change that. The chips are styled quiet so they never read like
an access control.

## Notes merge through a driver that shells out for the body — and is never installed unless it can run (2026-07-17)
**Why:** `updated:` is rewritten on every save, so *any* two concurrent edits to one note
collide on that line even when the two people touched different paragraphs — and git's
markers land inside the YAML fence, where `from_file` rightly refuses them and the note
drops out of the vault. Every concurrent edit, by construction, for a reason that is
entirely our own doing. **Consequence:** `fm-core/src/merge.rs` + `fm merge-md`, wired up
by `.gitattributes` (`*.md merge=fm`) and `merge.fm.driver`. It resolves structurally what
is mechanically resolvable — `updated` = the later reading (a clock, not an opinion),
`id`/`created` = base, `tags`/`assets`/`code` = union, any field only one side touched
takes that side — and **hands the body to `git merge-file`**: a 3-way text merge is a
solved problem and shelling out is the house rule (no diff3 to get wrong, no new dep).
*The property that pays for the whole thing:* frontmatter is always emitted whole and
valid, so **a conflict lands in the body** — the note still parses, still indexes, and
still opens in the editor with the markers in the textarea. That is what makes conflict
surfacing possible at all, and it is why "resolve markers in the textarea" is a feature
rather than a wish. A genuinely divergent *field* (both sides set `status` differently)
falls back to a whole-file merge rather than picking a winner: resolving by fiat is the
silent loss this phase exists to stop. **Two traps, both load-bearing.** (1) The
`merge.fm.driver` definition lives in `.git/config` and deliberately does **not** travel —
git will not let a repo ship a command that runs on your machine — so `ensure_repo`
installs it on every open, exactly as it writes `.gitignore` and the identity; only
`.gitattributes` travels. (2) **Never install a driver we cannot point at.** Found the
hard way: pointing at a bare `fm` and hoping PATH would answer meant git ran a
nonexistent command, took the non-zero exit as "conflict", and handed back `%A`
*untouched* — i.e. ours, with **no markers** — so the user resolves a normal-looking file
and silently deletes their collaborator's edit. `merge_command()` returns `None` unless an
`fm` binary really sits beside the running one, and no driver at all degrades safely to
git's built-in text merge. **Rejected:** a bare `fm` on PATH (PATH at `git pull` time is
not PATH now, and being wrong is data loss); resolving field conflicts by `updated`
last-writer-wins (fiat, i.e. the CRDT mistake decision 1 rules out).

## A vault gains an identity when it gains an audience, not before (2026-07-17)
**Why:** `ensure_identity` wrote `formicarium <formicarium@localhost>` whenever
`user.email` was unset — the default state of a researcher who never configured git. In a
shared vault that attributes *everyone's* commits to the same fake name, gutting the
provenance that "awareness over enforcement" (no locks; "Ravi pushed 2 min ago" is a `git
log` query) is built on. But simply deleting the fallback is worse, not better: git cannot
invent an identity on a host without a FQDN, so a fresh vault would fail to commit at all
— and the auto-commit swallows its errors, so the notes would silently stop being
versioned. **Consequence:** the placeholder stays, scoped to what it is honest for — a
vault **nobody else can see**. `git::identity()` reports that exact literal as `None`
("nobody real signs this"), and `git::set_remote` refuses while it stands, because a
remote is precisely the moment a name starts travelling into someone else's clone and git
history is forever. So the question is asked **once**, in the backup panel, at the only
moment the answer matters — and anyone whose git is already configured never sees it. Two
consequences worth keeping: the sentinel is **load-bearing**, so renaming
`formicarium@localhost` would turn every vault running on it into a "real" identity and
silently reopen the hole; and detection is by-value, so a vault that has been on the
placeholder for months **heals itself** the moment the user answers. Identity is written
**repo-locally** — a vault is an audience, so the name on a lab repo need not be the one
on your personal notes, and this app has no business editing anyone's global git config.
Enforced at `set_remote` only: `commit_all` cannot refuse (a silent stop is worse than a
fake name on a private commit), so an already-remote'd vault is nudged by the panel, which
shows the question whenever `backup_status.identity` is null.

## Inline meeting actions become their own note, never a per-block atom (2026-07-17)
**Why:** an owner types `- [ ] Ravi to send the draft` mid-meeting and wants it to show up
in the agenda — but making inline checkboxes first-class agenda items needs **per-block
identity**, which *the atom is the file* forbids (`MASTERPLAN.md:456`: it "voids this
plan"). **Consequence:** a checkbox stays **plain Markdown in the body** (an interactive
toggle that rewrites the body bytes, no id), and does **not** auto-appear in any planning
view. A deliberate gesture — a `/promote` slash entry or a per-line button — **extracts the
line into its own note file** (a task note with `status`/`due`, back-linked via
`[Title](note:<ulid>)`), which then flows into board/agenda as a normal atom. Friction stays
at "type `- [ ]`"; scheduling is an explicit promotion. **Rejected:** a body-scan
"checkboxes → agenda" pass — it manufactures second-class items that don't round-trip and
re-pollutes the exact views the assets decision below cleaned up. **Rejected:** per-block
ids/timestamps — voids the plan.

## Board images strip to the content-addressed blob store on save (2026-07-17)
**Why:** Excalidraw's `serializeAsJSON(…, 'local')` embeds a pasted image as a **base64
data URL inside the note body** (`BinaryFileData.dataURL`), and `Whiteboard.svelte`
re-serializes the whole scene on every debounced `onChange` — so a 2 MB screenshot becomes
~2.7 MB of **git churn per pointer move**, routing bulk binary through the note body and
breaking the "blobs are already out of git" premise (the two-tier-backup decision). This is
latent today and unbounded the moment boards are shared. **Consequence:** on board save,
**strip the scene's inline `files` into the blob store** (reuse `BlobStore::put_bytes` +
ingest's MIME sniff, dedup by sha256), leaving only blob references in the `.excalidraw`
JSON; **rehydrate on load** via `resolve_asset` / the planned `GET /api/blob/<hash>`. Reuses
the existing blob seam rather than adding one, and must land **before** the `.excalidraw`
merge driver shares boards (plan.md Track C Phase 3). **Rejected:** accept-and-document —
untenable once boards sync.

## Assets are query-layer-excluded from the planning views (2026-07-16)
**Why:** an asset is a blob a note *references*, not a thing you plan; a PDF
getting its own board card and timeline entry was noise. Filtering in each
renderer was rejected — it must be repeated per view and silently forgotten by
the next one. **Consequence:** `board`/`agenda`/`recent` carry
`Predicate::Kind(vec![Kind::Note])`; `FileStore` delegates structured predicates
to `fm_query::run`, so this cost no storage-layer change. **`search` and `gallery`
still see assets on purpose** — search (which indexes extracted PDF text) is the
only way to *find* an asset, and it is what backs the `/` menu's asset insertion,
so filtering it too would make ingested files unreachable. Two knock-ons: a board
grouped by `type` can now only answer `note` (the old
"falsifies-the-thesis" test was rewritten to pin the new rule; the generic
group-by claim still lives in `board_by_a_custom_property_…`), and the timeline's
type pill became dead — the status chip took its slot.

## Status rotates through the vault's own values; card order is a view preference (2026-07-16)
**Why:** setting a status meant entering edit mode and *typing* it. The obvious
fix — a `<select>` of `todo/doing/done` — would hardcode one workflow's enum into
the UI, which "generic, literal-free renderers" exists to prevent.
**Consequence:** `status.ts`'s `nextStatus(current, known)` cycles the statuses
`App.learnStatuses` already collects from fetched data, plus unset (so rotating
can always clear, with no separate control) — the chip renders any workflow and
the CI literal grep stays green. Typing a *new* value stays the Details editor's
job; it joins the cycle once a note carries it.
Separately, a card's **position within a column** is remembered in
`localStorage['fm-card-order']`, not in the note. **Rejected:** an `order`
property in frontmatter — it would rewrite a note file on every drag, and where a
card sits on your board is a view preference, not knowledge. Same reasoning, and
the same per-browser no-sync caveat, as the existing column order.

## `start`/`due` are a `Stamp` (day + OPTIONAL time), not a Date/DateTime pair (2026-07-15)
**Why:** the owner needs meeting times ("a time option beyond the date"), but an
all-day deadline must stay expressible and must not churn on disk. Two obvious
designs were rejected:
1. **Reuse `PropertyValue::Date` + `DateTime`.** `PropertyValue` derives `Ord`
   from **variant order first**, so every all-day item would sort before every
   timed item regardless of the actual day — silently wrecking agenda order,
   `Op::Lt/Gt`, and group bucketing. This is a trap, not a preference.
2. **Make `due` an `OffsetDateTime`.** RFC 3339 demands an offset; a wall-clock
   intention doesn't have one. It would also force a time on every deadline.

**Consequence:** `fm_model::Stamp { date, time: Option<Time> }` — naive (no
offset: 14:30 means 14:30 where you are, and the file is the truth), minute
granular, one `PropertyValue::Stamp` variant. `Display`/`FromStr` are **inverses**,
which is load-bearing twice over: it keeps `to_file` byte-idempotent (a bare date
in, a bare date out — no vault-wide churn), and it keeps the board's drag
write-back lossless (`value_string` → `display()` → `apply_property`; the old
`DateTime` display dropped the time, so a drag would have erased it). `Stamp` owns
the format on both sides, so the date literal is no longer duplicated across
crates. **Urgency stays day-granular** — a time is presentation, not priority.
`created`/`updated` are unchanged: they are *instants*, so they stay
`OffsetDateTime`/RFC 3339, and the UI's `parseStamp` is anchored so it can never
match one and hand back a UTC day.

## Whiteboard = embedded Excalidraw, lazy-loaded (2026-07-15)
**Why:** the owner wanted a real "drawio but simpler" freeform canvas, not a
diagrams-as-code stand-in, and chose full-featured-fast over build-it-minimal.
**Consequence:** a deliberate reversal of "no new UI runtime dependency" —
`@excalidraw/excalidraw` + `react`/`react-dom` are now deps. Mitigations keep the
base app lean: the editor is **lazily imported** in `Whiteboard.svelte` (React
root mounted via `createElement`, no JSX → no build plugin), so it's a separate
~744 KB-gz chunk that downloads **only when a board opens** — base `index.js`
stays ~34 KB gz (same discipline as KaTeX/Mermaid). **A board is just a note**
with a `view: board` property whose **body is the Excalidraw scene JSON** — no new
`Kind`, no backend change, files-as-truth intact (the `.excalidraw` JSON is plain
text on disk). **Caveats:** Excalidraw fetches fonts from a CDN unless
`EXCALIDRAW_ASSET_PATH` is set — offline it degrades to fallback fonts (local-font
bundling deferred); and the canvas only renders in a real browser, so it's
**unverified in headless CI** (build + code-split + round-trip are verified).

## Browser is the product; the native window is removed (2026-07-15)
**Why:** the Tauri/WebKitGTK window never painted reliably on the developer's
box (blank/gray; mutter/X11 with no compositor). The same SPA rendered correctly
in a real browser, so the UI logic was sound — the webview was the problem.
**Consequence:** `fm-serve` (std-only HTTP) serves the built UI to the default
browser; `fm-app` became a **library only** (no `[[bin]]`, no tauri deps); the
pixi `gui` env, the `e2e/` WebDriver tree, and the deny.toml Tauri-RUSTSEC
waivers are gone. Modern CSS is now fine (real browser, not WebKitGTK). The old
"WebKitGTK blank-window" risk is retired.

## Files-as-truth; the atom is the file (foundational)
**Why:** durability and ownership — a note must survive as plain text without
this app. **Consequence:** one Markdown note = one file; frontmatter is a
generic YAML mapping so unknown/custom props survive read→edit→write with no
silent loss; key order is fixed so `to_file` is byte-idempotent. No per-block
ids/timestamps — block-level structure is explicitly out of scope.

## `fm-query` may never touch fs/db (the insurance policy)
**Why:** a pure query engine is what keeps the whole system testable, portable,
and honest about the seam between "data" and "storage." **Consequence:**
enforced by a compile-time boundary *and* a CI grep. Do not add `rusqlite`/
`std::fs`/path handling to `fm-query`, ever. Search works by: `Text` predicate →
FTS5 prefix `MATCH` loads only the hit ids → the pure engine applies the rest.

## Generic, literal-free renderers
**Why:** a theme/renderer must not encode a specific workflow's enum values, so
arbitrary property values (any status, any type) render + tint without code
changes. **Consequence:** no `todo/doing/done` in `ui/src/renderers/`; column
tints keyed by `[data-value]`, urgency by `[data-urgency]`, labels sourced from
helpers (`urgency.ts`). CI greps enforce it.

## v1 editor = plain `<textarea>` + rendered read view
**Why:** a live-preview CodeMirror 6 editor was assessed as the single biggest
build risk. **Consequence:** editing is a textarea over the literal bytes
(`update_body`, byte round-trip tested) with a separate rendered read view
(`render.ts`, `marked`→HTML, lazy KaTeX/Mermaid). CM6 live-preview is **deferred
to v2**.

## Content-addressed blobs, extracted text in the note body
**Why:** dedup + integrity (bit-rot = a blob no longer hashing to its filename),
and searchability before the git-ignored blob syncs. **Consequence:** blobs are
sha256 with `ab/cd` fan-out; ingest sniffs MIME (`infer`), runs `pdftotext` →
the **asset note's body** (git-tracked, FTS-indexed), and makes a thumbnail.
`put_bytes` dedups before writing.

## Markdown→HTML is JS `marked`, not Rust pulldown-cmark
**Why:** it shipped that way; the MASTERPLAN's "pulldown-cmark→HTML" line never
materialized (the crate is absent). **Consequence:** the only HTML assembly is
one `marked.parse()` in `render.ts`. Note the unsanitized-innerHTML gap in
[known-issues.md](./known-issues.md).

## No plugin API
**Why:** plugin APIs rot and become a compatibility burden. **Consequence:**
extend via modular Rust (add a renderer / store / extractor), declarative
`.view` files (planned), a one-file theme (design tokens), and an optional mlua
hatch — never a stable plugin surface.

## Tauri was the light choice; a native-GUI rewrite is rejected
**Why:** even before removal, Tauri used the system webview (no bundled
Chromium; ~9.6 MB release binary) — lighter than Electron (Logseq/Obsidian run
1–2 GB). **Consequence:** the only genuinely-lighter path (egui/Slint/iced) would
mean rewriting the entire Svelte + KaTeX + Mermaid read view — not worth it.
Now moot (browser), but do not propose a framework rewrite.

## Backup is two tiers: git push (default) + restic (opt-in)
**Why:** the vault holds two data classes with nothing in common. Notes are
small, plain, mergeable → git carries them anywhere, authenticated by the user's
own ssh-agent/credential-helper, so **the app stores no secret**. Blobs are heavy
and git-ignored → only restic sees them. The button used to run restic
unconditionally, which was *broken-by-default*: it needs `FM_RESTIC_REPO` +
`RESTIC_PASSWORD` and `packaging/formicaria.sh` never sets them, so every
desktop-icon launch errored. This is the settled split elsewhere (Zotero syncs
metadata and file attachments as separate tiers; git-annex/git-LFS put a pointer
in git and content in special remotes; photo managers back up a small catalog and
hand bulk originals to a file tool). **Consequence:** `BackupPanel.svelte` owns
the vault's remote (stored in the vault's own `.git/config` — no new config
file, and decoupled from the app's source remote by construction). **A push
carries notes only**: media is off-sited only when the box is ticked, stated in
the panel rather than tracked. `manifest.json` is git-tracked, so a git-only
restore still knows its blob inventory and `fm verify` names what is missing.
`destination.ts` classifies both a git URL and a restic repo as local/remote —
a local path is a legitimate destination but must never be reported as "off this
machine".

## Squash-on-push — a deliberate reversal of "don't build commit management"
**Why:** `MASTERPLAN.md:411,447` accept thousands of `auto:` commits as the price
of undo and say *don't build commit management*. That holds for the **local**
repo, but the remote is a different audience: auto-commit fires every few seconds
of editing, so pushing raw would make the GitHub history unreadable.
**Consequence:** `git::push_squashed` collapses the unpushed window into one
`backup:` commit (`reset --soft <tracking-ref>` + commit). Two constraints are
load-bearing:
- **Never squash the first push.** With no tracking ref, "unpushed" means the
  *entire* history — destroying history that has never left the machine is
  exactly backwards. First push sends it whole; every later push is one commit.
- **Never fetch in this flow.** The tracking ref is stale on purpose: a remote
  another machine moved then stays ahead of it, so our push is *rejected* and the
  user is told (verified). Fetch first and the `reset --soft` would rebase onto
  their tip and silently overwrite their content with our tree.

**Accepted cost:** granular undo only reaches back to the last push; before that
each push is one step. This narrows (does not remove) the undo auto-commit buys.
