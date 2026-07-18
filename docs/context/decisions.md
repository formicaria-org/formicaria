# Decisions — the *why*

Condensed, load-bearing decisions and reversals. Each entry is: **decision —
why — consequence**. The canonical, fuller spec is
[`formicaria/MASTERPLAN.md`](../../formicaria/MASTERPLAN.md); this is the
quick-recall version. Newest first.

## Mobile is the app on the phone, not a thin client — one core, git-coordinated (2026-07-18)
**Why:** the owner overrode `MASTERPLAN:57`, which deferred mobile as *"a server + auth
decision"* (the phone as a thin client to the laptop's `fm-serve`). The goal is an app that runs
**on the phone itself** — collaborate with yourself/others across devices over the *same* git-repo
vaults, feature-parity of every view including the whiteboard, editable and merged on both
platforms — under one constraint: **minimal decade-scale maintenance.** That constraint plus
*"not two parallel workflows / seen seamlessly"* both point to **one shared Rust core reused on
both platforms**, not a second implementation (a PWA would re-implement `merge`/`query` in JS and
could diverge on the exact concurrent edits sync must reconcile — silent loss on the one operation
that matters). **Recommended:** Tauri v2 mobile, Android first, embedding `fm-core`/`fm-query`/
`fm-model`/`fm-app`; the phone is a *thin frontend*, nothing in the core forks. Full design +
audit in [`mobile-design.md`](./mobile-design.md); sequence in [`plan.md`](./plan.md) Track M.
**Consequence** — the rulings, each a fix an adversarial review + three code audits forced:

- **One command surface (the load-bearing fix).** The premise *"`fm-app` is already fronted by
  `fm-serve` and `fm-cli`, so nothing forks"* is **false**: `fm-cli` reimplements against
  `fm-core` (no `fm-app` dep), `fm-serve::api()` is a second surface, a Tauri bridge would be a
  third. Extract `api()`'s `match` + the single-lock `Vaults{MultiStore + Vec<VaultConfig>}`
  discipline into `fm_app::dispatch`; both transports go thin over it. This is what makes "one
  command library" *true*, and it precedes every milestone.
- **Two reversals, owned in writing — and reversal (1) is now ⛔ BLOCKED (audited 2026-07-18).**
  (1) **`git2` in `fm-core` would reverse "git is a capability, not a dependency"** — linking
  libgit2 compiles a git implementation into the core *always*, even for a git-less desktop user;
  in exchange in-process git beats "hope `git` is on PATH". **It must not be built as written**,
  for two reasons the ruling never weighed: **libgit2 cannot invoke external merge drivers** (no
  process-spawn exists in it; `git_merge_driver_register` is unbound), so porting `pull()` would
  *silently disable* the `.md` frontmatter merge while a collaborator's terminal `git pull` still
  honours it — two merge semantics in one vault; and **`deny.toml` forbids linking GPL code**,
  which libgit2 is (GPL-2.0-with-linking-exception) — `cargo deny` passes it only because
  `libgit2-sys` under-declares as `MIT OR Apache-2.0`, and `ci/third-party.sh` reads the same
  field, so we would ship binaries omitting a required notice. Needs an owner decision, not a
  silent pass. `gix` stays the documented pure-Rust swap once its push ships.
  (2) **A Keystore-held token reverses "the app stores no secret of its own"** — scoped to
  the fact a phone has no ambient credential-helper; encrypted under an Android Keystore key,
  never in prefs or the remote URL. Untouched (no mobile shell exists yet).
- **Auth is PAT-first (user-owned, un-vendored), OAuth device flow optional.** A pasted
  fine-grained token is host-agnostic (Gitea included) and needs no vendored OAuth-App
  registration on the critical path; OAuth is the convenience. A **per-URL injected
  `CredentialSource`** (not a global `OnceLock<Fn>`) serves multi-repo + refresh + tests;
  **`set_identity` on clone** keeps the `PLACEHOLDER_EMAIL` provenance sentinel honest on mobile.
- **Merge stays in the libgit2 family — ⛔ BLOCKED: the named function does not exist**
  (audited 2026-07-18). `git2` 0.20.4 exposes only `Repository::merge_file_from_index`, which
  needs index entries and would pollute the ODB. The buffer-shaped `git_merge_file` is bound in
  `libgit2-sys` but `git2` imports that crate **privately**, so this needs a direct `libgit2-sys`
  dependency plus unsafe FFI — a different decision, inheriting the licence question above. The
  rejection of a niche crate (`diffy`) on the one path that must never corrupt still stands. If
  ever built: the desktop `.md` driver **stays installed**; driver + both app-pulls call the same
  `merge_files`; a differential test (vs `git merge-file`) gates the swap.
- **Whiteboards merge element-wise, in pure Rust — ✅ SHIPPED 2026-07-18.** `fm-core/src/scene.rs`,
  called from `merge_body` before the text merge. **Why not Excalidraw's own
  `reconcileElements`:** it takes two scenes and *no base*, so it cannot tell "you deleted this"
  from "I added this" and keeps the element either way — every deleted shape returns on the next
  sync. A 3-way merge has the base and honours the deletion. Higher `version` wins, lower
  `versionNonce` breaks ties (deterministic on both machines, which is what stops the next sync
  diverging), fractional `index` keeps the z-order, file maps unite. **There is no `.excalidraw`
  file and no second driver** — `FileStore` writes `<ulid>.md` and the existing `*.md merge=fm`
  attribute already routes board notes into `merge_files`.
- **Sync is a seam, git one provider — but build nothing extra.** A thin `SyncProvider` trait with
  `GitSyncProvider` as the *sole* impl; design against Syncthing's profile *on paper*; make
  `history`/authorship a **queried optional capability** (that is the test the seam isn't
  git-shaped). Backends default free & serverless (rclone → ~70 backends; a free private GitHub/
  GitLab repo is the zero-server option), self-hosted first-class. **Keeps "no CRDT / no sync
  framework" intact** — extends "a bare git remote is the coordinator" to "coordinator is a role."
- **Auto-push is explicit, never silent** (`reject → pull → merge → re-push`), or the naive
  "auto-push after commit" wedges into the documented best-effort-and-silent failure loop against
  `push_squashed`'s deliberate reject-on-moved-remote. **Build the real streaming `GET
  /api/blob/<hash>`** (it never existed) for both platforms. **Mobile shell stays trivially CSS**
  (the pane-grid "unverifiable by CI" precedent applied, so it earns no e2e).

**Rejected:** a mobile PWA (a second, divergent merge/query implementation — the exact silent-loss
risk); CRDT/per-block ids (voids "the atom is the file", doesn't merge media, for a real-time we
don't need); a global-closure credential source (can't serve N repos, can't rotate on 401,
untestable); killing the desktop merge driver (reintroduces frontmatter corruption on terminal
`git pull`); **Path A (transport-only) as the *end state*** — it retreats toward the
satellite-of-desktop model this decision overrides and fails a phone-only collaborator, so it is a
low-risk *early demo*, not where Track M lands. **Top risk carried:** the Android SDK/NDK are not
conda-packaged, so the toolchain escapes the pixi-only house rule — pin the whole matrix in CI
(≠ `pixi.lock` reproducibility).

## `git2` is rejected; git stays a subprocess capability (2026-07-18)

**Decision made under the project's own principles**, after the audit found `mobile-design.md`'s
rulings 2/3/6 rest on wrong premises. The earlier "reversal, owned in writing" is itself
reversed: **"git is a capability, not a dependency" stands.**

Every principle in this repo points the same way, which is why this is a decision rather than
a preference:

- **"Bounded, replaceable dependencies — lean on tools that already solve a problem whole and
  shell out."** Linking libgit2 is the exact opposite move. Shelling out to `git` is the
  stance that makes `git` swappable at all.
- **The licence gate.** `deny.toml` says any GPL crate that is *linked* must fail the build,
  and names pdftotext/libvips as the pattern: GPL tools are **invoked**, never linked. libgit2
  is GPL-2.0-with-linking-exception. `cargo deny` would pass it only because `libgit2-sys`
  declares `MIT OR Apache-2.0` while vendoring ~230k lines of GPL C — clearing a gate on a
  metadata technicality is not satisfying it, and `ci/third-party.sh` would then ship binaries
  omitting a notice the exception requires.
- **Do not ruin what works.** libgit2 cannot invoke external merge drivers. Porting `pull()`
  silently disables the `.md` frontmatter merge while a collaborator's terminal `git pull`
  still honours it — Phase 1 undone, quietly, in the one place that must never corrupt.
- **Minimal decade-scale maintenance.** Ruling 3's actual shape is a direct `libgit2-sys`
  dependency plus unsafe FFI around a function `git2` deliberately keeps private
  (`git2::merge_file` does not exist). That is a maintenance liability, not a simplification.

**What this costs:** mobile has no `git` binary, so the phone port cannot use `git.rs` as-is.
That is a real constraint and it is *not* solved here, deliberately — mobile is blocked on the
Android toolchain anyway, and pre-committing to the wrong backend to unblock something that
does not exist is how the wrong backend gets built. When it is live, the options are: ship a
git binary with the app, revisit `gix` once its push ships (the documented pure-Rust escape),
or the transport-only Path A. **Rejected:** doing it now "so mobile is ready".

## Whiteboard images: git-track them, and do not strip yet (2026-07-18)

Two questions, answered separately.

**Which storage — decided: git-track whiteboard-embedded blobs, a scoped exception.**
`mobile-design.md` offered (a) that, or (b) a blob mirror over rclone/S3. (a) wins on the
stated principles: it is one `.gitignore` exception against a whole new subsystem; it stays
fully offline; content-addressing already dedups it; and (b) is a sync framework by another
name, which *"no CRDT library, no sync framework"* rules out. Recorded so nobody has to
re-litigate it.

**When to strip — decided: not yet, and this is the honest reason.** Boards sync *today*,
un-stripped, because the element merge (`fm-core/src/scene.rs`) shipped without needing the
strip — so the plan's "must land before boards are shared" was already false. What remains is
churn, not correctness: a 2 MB screenshot is ~2.7 MB rewritten per stroke. That cost is felt
on a phone (flash wear, battery) and barely on a desktop, so its beneficiary is the platform
that does not exist yet. Against that: the change is in the whiteboard save path, it has three
known traps (`onDestroy` flushes synchronously, so an async upload on save loses the last
stroke — upload eagerly on paste instead; `lastSerialized` compares the *raw* serialization
and must switch to the stripped text; and storing bytes needs either a thin `put_blob` command
or an asset note per screenshot), and **a canvas cannot be verified without eyes on it**.
Shipping a blind change to the one view whose failure mode is "your drawing is gone" fails
*do not ruin what works*. It lands when someone can watch it happen.

## Collaboration is git, *exposed* — not reimplemented (2026-07-18)
**Why:** the machinery (per-vault git, `.md` merge driver, push/pull, signed identity) already
shipped; git knows who changed what and when, but nothing surfaced it. The user's framing: *use
git and expose it*, don't build features on top. **Consequence:** one read-only
`git::activity` = `git log --name-only` over `notes/*.md` (a note file's stem *is* its ULID, so no
mapping) yields, walking newest-first, each note's **last editor** (`--no-merges`, since a merge's
author is the merger). That single command powers **all** collaboration visualisation:
- **`EditedBy` labels** on every card and the open note ("● name · 5m ago"), person-coloured by
  the same `hashHue` as vault badges, delivered to renderers via a runes-in-module store
  (`activity.svelte.ts`) — no prop-drilling.
- An **Activity pane** (git's log as a first-class workspace view), and a **contributor filter**
  (chips like the vault filter; `App.shown` gained an author check, so one click hides a person
  everywhere).
- **Automatic "someone pushed" awareness** — a slow, visibility-gated `remote_moved` network poll
  (not the 3 s heartbeat) feeding a one-click-pull chip, wiring the deferred item with existing
  commands.
**Rejected / deferred:** storing any authorship (git already knows — the app writes nothing);
"created by" and per-commit logs (last-editor map is the MVP); anchored comments (need the
deferred backlinks index); live presence (needs the descoped peer — git knows only *pushed*
state). The placeholder committer (`formicaria@localhost`) displays as "you".

## Every entity shows its vault, as a name-coloured badge, in every view (2026-07-18)
**Why:** with a set of vaults, "who can see this?" is a property you must be able to read off any
note, board or asset at a glance — but the badge existed only on board cards, and was a neutral
outlined chip whose colour the *theme* was meant to assign per vault (keyed off a `data-vault`
attribute). A theme can't colour a vault it has never heard of, so arbitrary vault names got no
colour. **Consequence:** one shared `ui/src/lib/VaultBadge.svelte` (+ pure `vaultColor.ts`:
`vaultHue(name)` — a deterministic hash → hue) is used by **Card (board), Agenda, Timeline,
Search, Calendar, and the open NotePanel**, so the same vault reads the same colour everywhere.
The badge fixes saturation/lightness so white-on-hue stays legible on every hue in both themes;
Calendar bars are too small for a name, so they use the compact `dot` form with the name in the
tooltip. Still shown only when `vault` is set (a single-vault install has no boundary), and the
colour still tells the truth about audience because `vault` is derived from location, never from
the file's content. No vault name lives in a renderer — the colour is derived generically.

## Cross-vault copy is restrictive by default; create picks a vault (2026-07-18)
**Why:** with multiple vaults you couldn't choose where a new note was born, and porting a
note to another audience had no path. Files-as-truth makes the copy "just a copy" — but a
naive one leaks: a copied note keeping its `note:`/`asset:` references would, once its new
vault is pushed, expose ids/hashes/filenames from *another* vault and leave its audience with
dangling pointers. **Consequence:** create-in-vault threads one `vault` string through
`capture` (mirroring `ingest`; routing/refusal already lived in `MultiStore::route`) and a
top-bar destination picker. Copy is governed by one principle — **ideas flow, artifacts do
not**:

- **Restrictive default.** `copy_note` copies *only the prose*: a fresh ULID (a copy is a new
  note — same id in two vaults makes one unreachable via `get`), body rewritten by
  `fm-app/refs.rs::strip_cross_vault` to drop every `note:` link and (unless opted in) every
  `asset:`/`sha256:`/local-image reference — whole Markdown span, label included — replaced by
  a fixed marker, and `assets`/`code` cleared. So a copy can never point outside its new vault.
- **Opt-in carries into the target, never points out.** `with_assets` copies the first-degree
  blobs *into* the target (`BlobStore`, content-addressed dedup; manifest refreshed) so it is
  self-contained; note links stay stripped (the linked-notes tier is deferred, and by the same
  principle each copied child would itself be prose-only).
- **Sensitive → warn + undo.** The "Copy to…" popover states the write is permanent in the
  target's git history; every copy leaves an `uncopy_note` Undo that also reclaims the blobs it
  newly wrote (only those nothing else there still references).
- **Rejected:** a same-ULID "move-as-copy" (ambiguous reads), and vault-scoping references to
  make a copy self-describing (re-couples a note to a location — the same rejection `.view`
  and the blob store already made). `Object.vault` stays derived-from-location, never a field.

## `.view` files are parsed server-side; the wire carries a name, never a query (2026-07-17)
**Why:** the user asked for a customizable multi-pane workspace; three designs + an
adversarial critic found the ask was already `MASTERPLAN.md:323` — *"five generic renderers =
query + a renderer, `.view` config files remain planned"* — and that the tempting route
(put `Query`/`Filter` on the wire so the UI builds filters) has two traps. **Consequence:** a
`.view` is a YAML file in `vault/views/`, parsed **server-side** (`fm-app/views.rs`) into an
`fm_query::Query`; the UI sends only a **name** (`list_views`/`run_view`). Four properties are
load-bearing:

**No `Query` on the wire.** Putting it there would force serde onto `fm-query`/`fm-model`,
where `Object.vault` is *never serialized on purpose* (else the permission is forgeable by a
typo), and would hand a client `PropertyValue`'s variant-order `Ord` trap. A name crossing the
wire has neither risk. (Note the CI grep would **not** have caught serde on those crates — it
greps `rusqlite|sqlx|std::fs`, so this is enforced by *not writing the derive*, deliberately.)

**The filter DSL has no ordered `prop` comparison.** `prop:` supports `eq`/`ne`/`exists` only;
date windows go through `date:` (a real `DateRange` over parsed `Date`s). So the `Ord` trap —
comparing a `Text` against a `Stamp` and getting a confident wrong answer — is *structurally
unreachable* from a `.view`, not merely discouraged.

**A `.view` extends a preset; it never replaces one.** `Filter { all }` is a top-level AND, so
user conjuncts compose onto the renderer's base by `Vec::extend`. The base for
board/agenda/timeline is `Kind(Note)`, written once in Rust — so the assets-exclusion decision
survives: a `.view` cannot widen a board to include assets, only narrow within notes.

**A broken `.view` is named, never dropped.** `list_views` includes an unparseable file with
its error (and the filename stem as name); `run_view` returns the parse error as the response.
The parse-error discipline is the whole reason a saved query is safe to hand a non-programmer.

**Consequence for the UI:** the sidebar lists views under the built-in nav; a selected view
owns the stage, rendering through the *same* renderers as the built-ins (a custom board
supports cross-column status drag but not within-column ordering — kept small). YAML not TOML
(a deliberate deviation from `MASTERPLAN:132`): `serde_yaml_ng` already parses frontmatter, so
zero new deps, and a `.view` reads like the top of a note. **Rejected:** the pane grid (see the
de-modalize entry); `Query` on the wire (both traps above); a query-builder UI / DSL grammar
(the file *is* the language, and it is already YAML); moving presets to the client (re-opens
the "filtering forgotten per view" bug `decisions.md` closed).

## The read view sanitizes untrusted note bodies (2026-07-17)
**Why:** `render.ts` assigns `marked.parse()` straight to `innerHTML`, and a note body is no
longer only the author's own text — collaboration made bodies arrive from other people through
the `.md` merge driver. `fm-serve`'s CSRF guard allows no-Origin requests, so a hostile
`<img onerror>` running in our origin can call any `/api/*`: read every note, delete them, or
set a remote and push a private vault off the machine. This was the top item in
`known-issues.md`, and its "single-user, low-risk" excuse expired the day two people could
share a vault. **Consequence:** DOMPurify runs on `marked`'s output before the DOM sees it
(`sanitize()` in `render.ts`), so scripts and event handlers are stripped. **The load-bearing
subtlety:** the sanitizer must not eat our *own* pipeline. Three URI schemes are ours —
`note:` (a reference chip), `asset:`/`sha256:` (inline blobs) — and they are **not** in
DOMPurify's default allow-list, so a naive call strips them and every asset image and note
chip silently vanishes. The config widens the URI regexp by exactly those three (they are
inert in a browser and fully replaced before display, so they add no sink) and nothing else.
Math (`span[data-math]`) and Mermaid (`code.language-mermaid`) placeholders survive because
the resolve passes run *after* sanitize; tests pin both the stripping and the survival.
Mermaid's SVG sink keeps relying on its own `securityLevel: 'strict'` — layering DOMPurify on
it risks dropping the `foreignObject` it uses for text, a regression headless CI cannot see.
**Rejected:** hand-rolling a sanitizer (the one thing worse than none); sanitizing Mermaid's
output (its strict mode is the designed control).

## The note trail is a peer column, not a modal overlay (2026-07-17)
**Why:** the trail (`openIds` + `NotePanel`) was `position:fixed; z-index:50` with a backdrop
that closed it on an outside click — i.e. reading a note was a **mode**. That contradicts the
founding thesis that a note *is* the task *is* the board card (`plan.md`): if a note is just
another view of the same file, seeing it should not dim and disable the board. The user asked
for a customizable multi-pane workspace; exploring it (three designs + an adversarial critic)
found the real want underneath — *"my agenda visible while I write the note it's about"* — and
that the layout ask was already `MASTERPLAN.md:323`'s own deferred `.view` design, not a new
feature. **Consequence:** de-modalize instead of building a layout engine. `.trail` becomes
the **third column** of the `.app` grid, a peer of `.main`; the `.overlay`/`.backdrop` are
deleted; `NotePanel` loses its `100vh`/`100vw` viewport-locking for `100%`. The grid is three
custom-property columns (`--rail`/`--main`/`--trail`) so rail-collapse and the trail compose
without a combinatorial explosion of `grid-template-columns` rules. `wide` (already persisted)
stops meaning "modal vs less modal" and starts meaning "the note takes the whole content area
vs docks beside the view" — which is what a user always thought it meant. **Rejected, and this
is the load-bearing part — REVERSED 2026-07-18; the pane grid shipped:** a **pane grid**
(pick N splits, any view in any cell). The objections, and what became of each:

- *"Configurability standing in for design"* — the owner asked for it directly, which is the
  one thing that settles this class of argument.
- *"A second, incompatible pane concept beside the trail"* — resolved by **deleting the
  trail**: a note became a pane kind (`kind:'note'`), so there is one pane concept, not two.
  That is what made the reversal safe rather than additive.
- *"Unverifiable by `pixi run ci`"* — **this one stood.** The layout is still not verified by
  CI: `panes.ts` is a pure, tested core (pane list, spans, feed-key dedup) and the geometry is
  not. The precedent still holds — the phone reflow is media-query-only for the same reason.
- *"A saved arrangement is what `.view` files are for"* — wrong, and usefully so. A `.view` is
  *a query plus a renderer*; an arrangement is which panes are on screen. Different things,
  and `.view` shipped separately.
- Also rejected at the time: threading `groupBy` through `onMove`/`onReorder`. That bug is
  reachable exactly when two boards are on screen — i.e. under the pane grid — so it became
  real the moment this reversed, and was fixed with it.

**The stopping line, as it now stands:** a flat pane list plus spans, **not** a recursive
split tree. A pane still cannot contain a pane — the part of the original ruling that
survived, because the split tree is the one shape with no natural stopping point.

## A vault is created, not invented; the vault list gains its first writer (2026-07-17)
**Why:** `load_vaults()` read `vaults.json` and **nothing wrote it** — hand-edited JSON, so
there was no path from "I want a vault" to a configured, opened, listed one. And it could
never return empty: `FM_VAULT` defaulted to the *relative* `"vault"`, so a typo, or the
launcher started from a different cwd, silently `create_dir_all`ed a working empty vault
named after the mistake while your notes appeared to have vanished. Configuration
masquerading as capability — the `restic_ready` shape. **Consequence:** the default is gone
(`FM_VAULT` set explicitly still works; unset means **zero vaults**, a real state that gates
the whole UI on a first-run screen); `MultiStore::open(&[])` is legal and `route` returns
`StoreError::NoVaults`, so reads over zero are honestly empty and **writes are loud**;
`check_path`/`create_vault`/`list_vaults` (`[]` is *the* first-run signal — not
`backup_status`, which shells out per vault including a network `ls-remote`, and making the
screen shown when nothing exists depend on the slowest git command is backwards). Six rules
are load-bearing:

**`vaults::save` merges into the parsed `Value` tree and appends only.** Never a typed serde
round-trip: `#[serde(flatten)] extra` would reformat a human's whole file and turn "I don't
understand this" into "I silently dropped it". It refuses a file it could not parse —
overwriting a hand-edited list is the loss the malformed-JSON warning exists to shout about
— and **writes the whole live list**, because `FM_VAULT` set with no `vaults.json` plus a
second vault created means `load` starts preferring the file, ignores `FM_VAULT`, and
**vault #1 vanishes on the next start**.

**JSON before memory: the config write is the commit point.** Reverse it and a failed write
leaves an in-memory vault that vanishes on restart *while the user captures notes into it*.
A failed open never deletes the directory (it may have pre-existed; this codebase does not
delete user data on a failure path), and partial success is reported as partial.

**Creation does not `git init`.** `commit_all` already calls `ensure_repo` on the first
auto-commit, gated by `ping.git`. Eager init buys an empty `.git` five seconds early,
imposes structure at the moment we promise not to, and — if the path sits inside a repo the
user owns — `ensure_repo` probes only `<path>/.git`, finds none, and `git init`s a **nested
repo shadowing theirs**, writing the placeholder identity into it. `blobs/` and `derived/`
*are* created eagerly: no git needed, and git cannot track an empty directory anyway.

**One mutex over the store and the list, never two.** `api()` already took them in opposite
orders (`commit`/`push` lock-then-resolve; `ingest` the reverse), so two would be AB/BA. The
trap that survives: with one, `lock(state)` + a separate `state.vault()` is a
**self-deadlock** — `std::sync::Mutex` is not reentrant — so those arms resolve through the
guard they already hold. `config()` returns **owned**, which is also what lets `ingest`
borrow `&mut store` at the same time. And the guard is dropped before I/O everywhere except
`commit`/`push`/`pull`: `backup_status` shells out per vault including a network
`ls-remote`, and holding it across that would stall every 3 s `ping`.

**`writable` is probed, never inferred from mode bits.** Create and remove a temp entry.
Configuration is not capability — the `restic_ready` rule applied verbatim. Likewise the
verdict (`ok`) is computed server-side: duplicating the policy in Svelte is how a button
enables and then fails.

**`allVaults` derives from `list_vaults`, not from loaded notes.** It used to come from
fetched cards, so a vault with nothing in it did not exist as far as the sidebar was
concerned — "empty vault" and "no vault" were indistinguishable, which is the exact
confusion the first-run screen exists to end. The vault you just made is the one most likely
to be empty.

**Rejected:** a native folder dialog (a browser cannot pick a server's directory, and
shelling out to zenity means the core spawns a process to do its own first run — so: a typed
path, validated server-side per keystroke); moving the config into `fm-core` (it is env by
definition, and `main.rs:652` already rules that fm-core stays free of environment and
configuration concerns); an `open_lossy` for the startup panic (real, but pre-existing and
uncoupled — see `known-issues.md`).

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
name). Nothing carries the old name now: the two literals initially held back (the
placeholder identity, Excalidraw's `source`) were held back *for backward compatibility*,
and there was none to keep — the author is the only user and no vault ran on the
placeholder. **The rule that outlives the rename:** `PLACEHOLDER_EMAIL` is a sentinel
matched **by value**, and a vault's `.git/config` is per-machine, so changing it again
hands every vault still on the old value a "real" identity and reopens the hole Phase 0
closed. Safe once, while every vault was the author's. Not twice.

## The core ships as one file; pixi is the only package manager; deps come from wherever (2026-07-17)
**Why:** the owner's ruling. *pixi is the only package manager, including for generating
the executables. CI emits a binary per OS as an artifact, and those binaries work with the
optional deps installed however the user prefers — pixi being one way.* **Consequence:**
`fm-serve/build.rs` bakes `ui/dist` into the binary (hand-rolled `include_bytes!` table —
~40 lines against a dependency, in a crate that is deliberately std-only networking), so
the binary alone *is* the app. Verified: alone in an empty directory, empty `PATH`, no
`ui/dist`, no pixi — it serves the UI, its JS, the SPA fallback and the API. `FM_UI_DIST`
has **no default**: set explicitly it reads from disk (the dev loop keeps its speed — a
`.svelte` edit needs no Rust rebuild), unset it serves itself. The Linux-only assumptions
went with it (`open_native()` knows macOS's `open` and Windows' `cmd /C start ""`;
`config_dir()`/`home()` know `Application Support`, `%APPDATA%`, `USERPROFILE`) — hand-rolled
over the `dirs` crate, since it is three env lookups. `pixi.toml` covers linux-64 /
osx-64 / osx-arm64 / win-64, and **all of it resolves**, including restic, poppler, libvips
and vs2022 for rusqlite's bundled SQLite — so the default env stays flat and the `media`
feature split was not needed. CI gates on all three OSes (not fail-fast: the point is to
learn *which* is broken) and uploads `fm-serve` + `fm` per platform; a tag attaches a zip
each. **`fm` is never optional beside `fm-serve`** — `ensure_repo` installs the merge driver
by pointing git at the binary beside the running one. **The binary does not care how the
optional tools got onto `PATH`** — pixi, apt, brew — which is what makes one artifact serve
every user. **Rejected:** installing tools system-wide as a prerequisite (that is the
user's choice, not ours); baking the pixi env's path into the launcher (re-couples the very
thing this removed); musl-static (the binary already runs with no pixi env; glibc 2.34 is
met by any 2021+ distro).

## Every external tool is an optional feature that declares itself (2026-07-17)
**Why:** the owner's ruling, and it settles a class of question rather than one case: *the
core should let you take notes and schedule tasks on a local PC with nothing installed. If
you want PDFs rendered nicely, you install that dependency. A missing dep means that
feature doesn't work — the core still does.* **Consequence:** the core is `FileStore` over
Markdown files and **spawns nothing**. Verified against a machine with an empty `PATH` — no
git, no restic, no pdftotext, no vipsthumbnail: capture, `due` scheduling, agenda, board,
search and edit all work. Everything external is a *feature* with a *declared capability*:

| tool | feature it buys | without it |
|---|---|---|
| **git** | history (local undo past this session), backup, collaboration | `git::available()` → the tab stops the 5s auto-commit and says so once; the panel says git isn't installed |
| **restic** | media (blob) backup | `backup::available()` → `restic_ready` is false, the checkbox is off and says why |
| **pdftotext** | a PDF's text is searchable | blob still stored and rendered; extraction returns `None` — no text, no error |
| **vipsthumbnail** | gallery thumbnails | tile falls back to the missing-asset placeholder |
| **xdg-open** | "open in the OS app" | that one action errors |

**The rule that generalises:** a capability must mean *"this will work"*, never *"this is
configured"*. `restic_ready` broke it — it meant "repo set + password set", so on a machine
with no restic the checkbox enabled, you ticked it, and it failed. Configuration is not
capability. **And absence must be stated, not swallowed:** the notebook working while a
feature silently doesn't is how you discover on the day you need it. **Rejected:** treating
any of these as hard requirements (the core demonstrably needs none); bundling them into
the binary (they are subprocesses — see the packaging note in `plan.md`).

## Git is a capability, not a dependency (2026-07-17)
**Why:** the owner pushed back that formicaria runs on vaults on a single PC and should not
be tightly coupled to git — git is for backup and collaboration. Tested: **true, and the
code already agreed.** With no git on the machine at all (empty `PATH`), the server starts
and capture / board / agenda / search / edit all work. Files-as-truth means the notebook is
a directory of Markdown files; `FileStore` never spawns git. **But it was optional in fact
and not in design**: the 5 s auto-commit spawned git, failed, and the UI swallowed it —
forever — while `backup_status` reported `remote: null, identity: null`, which is
indistinguishable from "you haven't set it up yet". A vault quietly unversioned, discovered
on the day you need the history. **Consequence:** `git::available()` (cached — git does not
appear mid-run), reported on the heartbeat (`ping.git`) and in `backup_status.git`. The tab
**stops scheduling** the auto-commit when there is no git and says so **once**; the backup
panel says *"git isn't installed — your notes are safe, they're files on disk, but nothing
on this panel can run"* instead of inviting you to type a remote into a tier that cannot
run. **One nuance the framing missed:** git is not *only* backup and collaboration — it is
also **local history**, the undo that outlives the session, on a single PC with no remote.
So: optional, and worth having. Which is exactly why its absence must be stated rather than
swallowed. **Rejected:** making git a hard requirement (the notebook demonstrably doesn't
need it); reimplementing versioning (that is what shelling out to git buys us).

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

**restic is per vault too — a restic repo *is* per repository.** *(Corrected 2026-07-17:
the first cut made it one repo for the whole set, on the theory that a snapshot is
disaster recovery rather than sharing. That was wrong on its own terms — restic has no
notion of "part of a repo", so backing up a set of vaults means a repo each, and one repo
holding several vaults is not a design choice, it is a merge of things that were kept
apart on purpose.)* So each vault carries an optional `restic` in the vault list, and a
vault without one simply has nowhere to put its media — which is **not** an error: you may
well want the lab's notes shared over git and its media backed up by the lab, not by you.
"Include media" backs up the vaults that have a repo and **names the ones that don't**,
per vault, in the report and in the verdict. Silently skipping them would be the exact
overstatement `destination.ts` exists to prevent. `RESTIC_PASSWORD` is still one password
for every repo: a per-vault password has to live somewhere, and the one place it must
never live is the config file sitting next to the paths. The single-vault install's
`FM_RESTIC_REPO` still works and becomes that vault's repo.

**An asset joins the audience of the note it was dropped on.** `ingest` takes the vault's
**path and its name**, because they answer different things — the blob store takes a path,
the `Store` routes by name — and they must agree. The first cut passed only the path:
bytes went to the lab vault while the asset note went to the default, so the people who
could see the file could not see the note describing it, and the note pointed at bytes its
own vault never had. Caught by running it, not by a test.

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
git's built-in text merge. **The corollary bit later:** `pixi run build` shipped only
`fm-serve`, so in a release install there *was* no `fm` beside it and the driver silently
never installed — the centrepiece of Phase 1, absent, with every test green (the tests
build `fm-cli` themselves). Found by a clean release build, not by CI. The build task now
builds both and says why. **Rejected:** a bare `fm` on PATH (PATH at `git pull` time is
not PATH now, and being wrong is data loss); resolving field conflicts by `updated`
last-writer-wins (fiat, i.e. the CRDT mistake decision 1 rules out).

## A vault gains an identity when it gains an audience, not before (2026-07-17)
**Why:** `ensure_identity` wrote a placeholder committer (`formicaria@localhost`) whenever
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
`PLACEHOLDER_EMAIL` would turn every vault running on it into a "real" identity and
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
the existing blob seam rather than adding one. **⛔ BLOCKED (2026-07-18), and the ordering
claim was wrong:** the board merge already shipped without this (`fm-core/src/scene.rs`, under
the existing `*.md merge=fm` driver — there is no separate `.excalidraw` driver), so boards sync
today *un-stripped*. The strip cannot ship until one question is answered: **`blobs/` is
gitignored**, so once images live there instead of in the scene body, a shared board shows no
images on a collaborator's clone. Pick (a) git-track whiteboard-embedded blobs as a scoped
exception, or (b) a blob mirror — `mobile-design.md` names both and decides neither. Two further
traps found while auditing: `onDestroy` flushes *synchronously*, so an async upload on save loses
the last stroke (upload eagerly on paste instead), and `lastSerialized` compares the **raw**
serialization, so it must switch to the stripped text or the no-op-save guard breaks.

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
one `marked.parse()` in `render.ts`. *(That output is now sanitized with DOMPurify before the DOM sees it — see the sanitize entry.)* The then-unsanitized-innerHTML gap in
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
`backup:` commit (`reset --soft <base>` + commit). Three constraints are
load-bearing:
- **Only *our* commits are the window** (added 2026-07-18, Track V). The
  justification above — auto-commit fires every few seconds — justifies collapsing
  what *we* wrote and nothing else. `newest_foreign` walks `tracking..HEAD` and
  stops the squash at the first commit whose subject is not `auto:`/`backup:`, so
  three hand-written manuscript commits in a vault that is also a project repo stay
  three commits. Discriminated by **message prefix, never author**: we commit as the
  user's own identity, so an author test classifies everything as ours.
- **Never squash the first push.** With no tracking ref, "unpushed" means the
  *entire* history — destroying history that has never left the machine is
  exactly backwards. First push sends it whole; every later push is one commit.
- **Never fetch in this flow.** The tracking ref is stale on purpose: a remote
  another machine moved then stays ahead of it, so our push is *rejected* and the
  user is told (verified). Fetch first and the `reset --soft` would rebase onto
  their tip and silently overwrite their content with our tree.

**Accepted cost:** granular undo only reaches back to the last push; before that
each push is one step. This narrows (does not remove) the undo auto-commit buys.
