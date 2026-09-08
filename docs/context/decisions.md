# Decisions — the *why*

Condensed, load-bearing decisions and reversals. Each entry is: **decision —
why — consequence**. The canonical, fuller spec is
[`formicaria/MASTERPLAN.md`](../../formicaria/MASTERPLAN.md); this is the
quick-recall version.

**Not sorted, and do not try to read it in order.** The file uses two heading conventions that grew
up at different times — `## Title (date, #tags)` and `## date — title #tags` — and they interleave,
so the newest entry is at the **bottom**, not the top. It said "newest first" until 2026-09-05, and
a reader who believed it stopped in early September and missed everything after. **Retrieval is by
subject, never by position: grep a `#tag`.** Every heading carries at least one, and `ci/checks.sh`
fails if one does not — because a third of this file had no tag at all, which made the documented
retrieval path silently miss it.

**This is an append-only log — a decision is superseded, never edited away.** A reversal is added
as a new dated entry and the old one gets a `> SUPERSEDED …` banner pointing to it, so the *chain*
survives (the value of "we tried X, then Y" is the whole chain). Prune only exact duplication.

## Subject index — grep a subject, jump to the decision(s)

The [overview.md](./overview.md) router sends you here by **subject**; find it below, then grep the
heading. Retrieval is per-decision, never "load the whole 1,300-line log."

- **`#seams`** (the compile-time invariants): ***A poisoned lock is recovered, not propagated*** ·
  ***The vault lock is not held across a subprocess or a revwalk*** (read before adding a `dispatch`
  arm that shells out or walks git) · ***`Kind` is the second predicate that reaches SQL***
  (read before adding a pushdown — and before assuming `perf.rs` can see your change) ·
  *`fm-query` may never touch fs/db* · *Generic,
  literal-free renderers* · *Files-as-truth; the atom is the file* · *`fm-cli` shares the command
  library; does not route through `dispatch`* · *Vaults are audiences* (the `candidates` seam).
- **`#git` / `#sync`** (git, merge, collaboration): ***The body merge stops shelling out where there is no shell*** (read before touching `merge::text_3way`, `vcs`'s hand-written arms, or either backend's `commit_all*` — it is a shipped vault-freezing bug and its two guards) · ***A proposal's outgoing commit is kept, so the accepted label can be true*** (read before touching `write_proposal_branch`, `delete_branch` or `refs/fm/*`) · ***libgit2 ships on Windows too*** (the
  exception is now "any device with no git binary" — read before touching `deny.toml`'s scope or
  `fm-serve`'s target-gated dependency) · ***A backend that cannot finish a merge must
  refuse to commit*** (read before touching either `commit_all` — the two backends had opposite bugs
  here) · ***A prose conflict is not turned into two notes*** (read before proposing conflict
  copies — the field survey, the withdrawal, and the two things it would have cost) ·
  ***A conflict blocks its own notes and nothing else*** (read before touching either
  `commit_all`, and before writing any UI message about a mid-merge vault — four were conditioned on
  a proxy that stopped being true) ·
  ***The phone re-merges what libgit2 already merged*** (read before touching
  `git_native::pull` — and before trusting any two-device test, because the harness bug that hid
  this one is recorded there) ·
  ***A divergent field keeps both, by demoting the loser into a field beside it*** (read
  before touching `merge_objects`, the `conflict-` prefix, or the characterization test that locks
  the old behaviour — it reverses a July ruling that named its own reversal conditions, and it is
  conditional on a surface that is not built yet) ·
  ***A merge never stalls on a question whose safe answer is a note*** (read before touching
  `keep_notes_the_other_side_deleted` or either backend's conflict loop) ·
  ***A resurrection is a fact about the merge, so that is where it is recorded*** (read before
  touching either keep branch, `KEPT_TRAILER` or `refs/fm/kept-seen` — it carries why the obvious
  frontmatter marker is a live bug, and why the ordering inside the keep branch is load-bearing) ·
  ***The app says how long a vault has been quiet*** (read before adding another alert to the
  toolbar, or before folding `last_commit` into `backup_status` — it is the one signal that does not
  depend on a detector working, and why committer time is the right clock) ·
  ***The merge unit for prose is a sentence, and `zdiff3` is declined on measurement*** (read
  before touching `merge_body`'s rescue, `split_sentences`/`join_sentences`, or before proposing a
  conflict-style change — it carries the vault measurement, why adjacent sentences still conflict
  on purpose, and why `merge.conflictStyle` is inert for notes) ·
  ***A conflict marks the sentence, and only where narrowing is provably free*** (read before
  touching `join_conflicted`, `marks`, `narrowing_is_safe` or `structured` — it carries the five
  shapes an adversarial audit found, why the guard is asked per *unit* rather than per source line,
  and why declining is a first-class answer there) ·
  *The in-process sync path: the app merges* ·
  *Git is a capability, not a dependency* · *`git2` is rejected* **⟶ + The libgit2 exception**
  (read the pair — it is a reversal chain) · *Notes merge through a driver that shells out* ·
  *Collaboration is git, exposed* · *Squash-on-push — a deliberate reversal* · *A commit that
  committed nothing must say why* · *Acquiring a vault: `naturalise` is the seam* · *Backup is two
  tiers* · ***A backup surface names what its tier carries*** (filed under `#vault`; the git half —
  what `git_assets_max` makes a push carry — is here). **On-device proposal lifecycle:** `sessions/2026-07-24-proposals-on-the-phone.md`.
- **`#track-m`** (mobile/phone): ***The phone's blob route answers a `Range`, and stops lying about
  it*** (read before touching `blob_response` — and note what is *not* verified there) ·
  ***A shell that cannot configure its paths refuses*** · ***iOS ships agent-free, and the subprocess consequence is reversed for Android*** (read before any iOS work, and before assuming the 2026-07-19 no-subprocess clause still binds Android) · ***An iOS build would contradict the project-local-toolchain ruling*** (read before adding any iOS CI job — and never to `release.yml`) · ***The iOS diagnostic channel is stderr, not `os_log`*** **⟶ amended: *the iOS log is a file in the app container*** (read the pair before touching `install_logger` or the Simulator smoke test — stderr was measured to reach nobody) · ***The JS toolchain is pinned across every pixi environment*** (read before changing `nodejs`/`pnpm` or running a mobile CLI from a non-default environment) · ***iOS gets zlib and iconv from the Xcode project*** (read before touching `ci/ios-inject-linker-libs.sh` or wondering why a `staticlib` cannot carry them) · ***iOS is meant to reach users' phones now, and the route is undecided*** **⟶ + *iOS ships as an unsigned IPA that each user signs with their own Apple ID*** (read the pair before any iOS distribution work — the first carries the App Store / TestFlight / sideloading facts, the second picks free-account sideloading and says what it forbids) · ***The phone answers the same status shape as the desktop*** (read
  before adding a key to any status the shared panel renders) · *The owner's five Track M rulings* · *The Track M record drifted* ·
  *Mobile is the app on the phone, not a thin client* · *Android TLS: trust store from memory* ·
  *`fm-serve` sends a CSP* (+ ***the read view may frame its own blob*** — the phone's
  `tauri.conf.json` carries the same clause — **but see *A PDF renders on the desktop; the phone
  opens it externally*: the phone fix was inert and is withdrawn**) · ***Startup is a contract*** (read before touching the shell's `setup`
  hook or the render gate) · ***Every Android IPC command is `(async)`*** (read before adding a
  command — a blocking one freezes the screen, and CI greps for it) · *Android trusts its persisted
  index on open* (the `ColdStart` seam) · ***A file is sliced, so its size stops being a memory limit*** (read before touching `fm_core::chunked`, `MAX_INGEST`, or the boot sweep) · *The Android attachment ceiling is 16 MB* (partly superseded by it) · *An emulator
  may be installed to; the owner's phone may only be looked at*.
- **`#ui`** (workspace/views/render): ***A snapshot says what it held*** (filed under `#vault`;
  the panel half — why the step line stopped printing a fixed phrase — is there too) ·
  ***An overlay is bounded by the visible viewport, and it
  has exactly one scroll surface*** (read before writing any dialog, or before capping any
  covering surface in `vh` — it is also where the NUL-byte-hides-a-file-from-grep trap is
  recorded) · ***One tag is an arrangement, not a query builder***
  (read with the 2026-08-28 saved-view ruling — they are a pair) · ***The read view may frame its own blob*** (read before
  touching `frame-src` or assuming a jsdom test covers a policy) · ***A saved view is an arrangement
  you keep, not a query you write*** (read before adding a filter editor) · *A contributor is an email, everywhere* · ***One view at a time is
  the default, `auto` is gone, and the shell reads its own insets*** (read before touching `Layout`,
  a `data-layout` rule, or a `--safe-*` inset — it supersedes the layout half of *One shell, two
  arrangements*) · ***The chrome is one row at the top, and the pane header belongs to `tiled`
  alone*** (read before touching `Pane.svelte`'s header, `ViewControls`, `ViewBar`'s placement, or
  the board rail's visibility) · ***View customization is withdrawn, the board rail goes, and a
  stale binary wasted a review round*** (read before adding a view-authoring control, before
  touching the board rail, and — always — before telling anyone to reload to see a UI change) ·
  ***The feed gains an excerpt and a thread, and three tempting halves of it are refused*** (read
  before putting an editor, a rendered body, or a double-tap in any list row) ·
  *One shell, two
  arrangements* (partly superseded) · *`.view` files parsed
  server-side* · ***A view says what it leaves out, and a board admits it scrolls*** (read before
  changing what a pane shows about its own filtering) · *The read view sanitizes* · *The note trail is a peer column* · *Browser is the
  product* · *Whiteboard = embedded Excalidraw* · *Board images strip to the blob store* · *Assets
  query-layer-excluded from planning views* · *Status rotates; card order is a view preference* ·
  *`start`/`due` are a `Stamp`* · *Tauri was the light choice; native-GUI rewrite rejected* · *v1
  editor = textarea + read view* · *Markdown→HTML is `marked`*.
- **`#vault`** (audience/cross-vault): ***A snapshot says what it held, and "restic did not say"
  is its own answer*** (read before touching `backup::backup`'s return, the `--json` parse, or any
  surface that describes a snapshot) · ***A backup surface that cannot say* when *is not a backup
  surface*** (read before adding a field to `backup_status`, or before folding a per-vault restic
  spawn into anything that polls) · ***A gate must inspect the same string the router acts on***
  (read before adding to `REMOTE_DENIED`, or before comparing a request path anywhere) ·
  *A vault is labelled by its remote, identified by its local
  name* · *A vault can be forgotten, and forgetting never deletes* ·
  *A caller is a member of some audiences, not all* (`Scope`
  — read before adding a read path or touching `find_blob`/`Vaults::config`) · *Vaults are audiences* · *Every entity shows its vault badge* ·
  *Cross-vault copy is restrictive* · *A vault is created, not invented* · *A vault gains identity
  when it gains an audience* · *formicaria: three pillars, one atom (the rename)* · *Which
  attachments travel: per-vault size limit* · *Content-addressed blobs* ·
  ***A backup surface names what its tier carries*** (read before wording anything about backup, or
  before proposing that restic snapshot the vault root — it also holds the git-lfs argument) ·
  ***Attachments in git have a ceiling, because there is no LFS*** (read before changing
  `git_assets_max`'s bounds, or before adding an "unlimited" option).
- **`#data`**: ***An import converts; adoption renders*** (read before touching `import.rs`, before
  adding a `source_key` convention, and before assuming Track V4 covers this) ·
  ***A tag may contain a space, and both doors must agree what that means*** ·
  ***An anchored asset reference points at a place, and stays an ordinary link***
  (read before touching `parse_ref`, `assetUrl` or the resolve passes) · ***A paper is a note, made from the citation you already have*** (read before
  adding a metadata field or an acquisition route) · ***A list property is writable, and a scalar where a list belongs is read, not
  dropped*** (read before adding a field to `Object` — a typed field with no `apply_property` arm is
  silently lossy) · *A count is a symptom; the kind is the diagnosis* · *The auto-commit stages what we
  wrote* (**+ the explicit catch-up**: it could
  *permanently skip* a note, not merely lag) · *A conflict surface is derived from git, not from
  markers* · *The lost-update token is a content hash* ·
  *The poll answers a comparison, not a report* (the generation counter — read this before
  touching `ping` or assuming one client).
- **`#toolchain`**: ***The gate refuses to run without the tools its tests need*** (read before
  adding a test that skips on a missing binary) ·
  ***A test that names somebody's private repo, and three that only passed
  here*** (read before writing a test that touches a remote, and before trusting a suite that
  has never been run) ·
  ***The repo goes public, and the economics every CI ruling rested on invert***
  (read before touching any workflow, before restoring a trigger, and before believing prose
  in `.github/` that talks about a bill) ·
  ***The licence notice covers what the binary carries, not what Cargo resolves***
  (read before touching `ci/third-party.sh`, `THIRD-PARTY.md`, or either licence guard in
  `ci/checks.sh` — a gate keyed on one manifest is blind to the rest of the artifact) ·
  ***A platform arm nobody can compile gets a tested core and a cross-check*** (read before adding per-OS code, or before trusting a grep-shaped guard) · *The core ships as one file; pixi is the only package manager* · *The TLS
  exception: a self-signed leaf, share-only* (read before touching `rustls`/`rcgen` — the `ring`
  pin is a licence gate) · *Every external
  tool is an optional feature* · *No plugin API* · *A hand-fired release names itself after the ref
  it was fired on* (read before changing a workflow trigger — disabling one re-meanings the rest) ·
  *The manual travels in the archive* (built-and-discarded docs do not exist) ·
  ***The archive carries its own update path, and it copies one way*** (read before touching the
  update script, `vaults.json` copying, or before adding a "you may have made a mistake" warning) ·
  ***What ships is an app, not a binary*** (read before touching `packaging/launcher/` or the
  `stage` step — the portable-vault recipe is exact and the CSP one is not obvious) ·
  ***An addition is checked against the record before it is written*** (read before adding a
  dependency, an archive file, a relaxed guard or a changed default) · *The manual's CSP is a named
  exception* (`#ui`).
- **`#agent`**: ***Advice that cannot succeed is worse than none*** (read before writing a capability message, or before adding anything that spends a user's disk) · ***The assistant asks the machine, not a list of operating systems*** (read before touching `unavailable()`, `SystemMonitor::sample` or `die_with_supervisor`) · ***The assistant provisions itself, so a downloaded copy can run it*** (read before touching the launch path, `models.toml`'s runtime keys, or the first-enable flow) · ***`/transcribe` reads writing too — one verb, two specialists*** (read before adding a specialist or a model file) · *Inline meeting actions become their own note* · *The study agent's model warm-up is
  deferred a few seconds after launch*. (Model/agent decisions that are not yet folded up live in
  `archive/ai-agents-plan-superseded-2026-09-02.md`, which is history rather than instruction.)

## A gate must inspect the same string the router acts on (2026-09-02, `#vault`)

**Decision.** `authorize` now matches `REMOTE_DENIED` against the **route** — `path` with any query
string cut off — instead of against the raw request target.

**Why.** They were different strings, and one `?` was the whole gap. `path` is the request line
verbatim, so it carries `?a=b`; the denial was an exact `REMOTE_DENIED.contains(&path)`; and `api()`
*deliberately* reads a command's arguments from that same query string (`/api/ingest?name=…`). So
`POST /api/delete?id=<ulid>` matched no entry, sailed through, and was then split and dispatched
with the query as its arguments. **Every** host-bound command was reachable from a paired tablet
that way — `check_path`, which reports the existence and writability of any path on the machine;
`set_git_credential`; `create_vault`; `delete`. Found while adding two commands to that list, which
is the only reason it was found at all: the list *looked* like it worked.

**Consequence.** The route is computed once at the top of `authorize` and used for all three
comparisons (`/api/pair`, the `/api/` prefix, the denial), not fixed at the one call site that was
broken — a second string derived a second time is how this comes back. `a_query_string_does_not_
smuggle_a_denied_command_past_the_gate` pins it with the query shapes an attacker would actually
send, and it fails against the old code.

**The general form, worth more than the fix:** a gate that inspects a *different* string from the
one the router acts on has a hole in it by construction. `agent::route` and `share::route` match the
same `path`; they are exact-match and therefore fail closed, but they are the same shape of risk.

## An import converts; adoption renders (2026-09-02, `#data` `#vault` `#seams`)

**Decision.** formicaria imports a **Logseq graph or an Obsidian vault**: `check_import` previews a
folder, `run_import` converts it into notes, either into a new vault or one that already exists.
`fm-core/src/import.rs` owns the walk; `import/logseq.rs` and `import/obsidian.rs` are **pure**
(one file's text → a `Page`), and the resolution pass that turns `[[Name]]` into
`[Name](note:<ULID>)` is pure too — attachments are hashed before it runs and handed to it as a map.

**Why it is not Track V4 adoption, which it superficially resembles.** V4 is *"a doc reads for
free"*: any `.md` renders with a transient, index-only id and **nothing is written into the user's
repo**, because "ten commits putting ULIDs into READMEs your collaborators read is a bad trade".
That is a **rendering** of files you keep owning elsewhere. This is a **conversion** of files you
are migrating away from — `[[Page]]` must become a real reference, `key::` must become queryable
frontmatter, `assets/` must enter the blob store — and none of that is possible in place. V4 remains
unbuilt and unchanged. **The invariant that keeps the two apart: the source folder is opened
read-only and is never written to**, asserted by a test that fingerprints it before and after.

**What a Logseq block becomes, and why nothing is lifted.** A page is **one note whose body is the
outline, verbatim, as nested Markdown lists**. `id::` is dropped and `((ref))` is replaced by the
referenced block's *text* — which is what Logseq itself renders, so nothing visible is lost. This is
forced, not chosen: *Files-as-truth; the atom is the file* and *Inline meeting actions become their
own note* both refuse per-block identity ("Rejected: per-block ids/timestamps — voids the plan").
For the same reason a `TODO` becomes a plain `- [ ]` checkbox that appears in **no** planning view,
and a block's `SCHEDULED:`/`DEADLINE:` stays text. Only a **page-level** property becomes a note
field. Lifting one bullet's date to the note would manufacture exactly the second-class item that
ruling rejects.

**Add-only, and that is a safety decision rather than a limitation.** Each note carries
`source_key` (its path in the source) and `source_library`, so a re-import recognises what it
already did — and **skips it**, never rewrites it. `refuse_if_stale` compares the *indexed* mtime,
which `put` refreshes on every in-app save, so an "update in place" would silently overwrite an edit
the user made here and raise no conflict at all. The lookup is **one** pass into a `HashMap` because
`FileStore::candidates` pushes only `Text` and `Kind` down to SQL: a `Prop` lookup per page is
quadratic on a vault of any size.

**Four traps this had to dodge, each of which was a silent failure.** (1) `to_file` writes `extra`
into the same YAML mapping as the well-known keys and `Mapping::insert` overwrites — so an imported
`type::`/`updated::` would replace ours and the note would stop parsing and vanish from every view;
`base::` is quieter still, because `thread::notes_base` *hides* a note carrying one. Reserved names
are therefore prefixed and counted. (2) `#[[two words]]` shares Logseq's link syntax but is a tag —
counted as a link it reported every multi-word tag as dangling, and would have rewritten one into
`#[label](note:…)` the moment a page shared its name. (3) Obsidian resolves `[[Roadmap]]` **and**
`[[Projects/Roadmap]]`, so a page registers under its path as well as its name or every
folder-qualified link dangles. (4) One `git log` for the whole source, never one per file — a spawn
per page is precisely the `papers-plan.md` B5 mistake.

**Dangling links stay as text.** A page that exists only as a reference is normal in Logseq. A stub
note is a `Kind::Note`, so hundreds of empty ones would appear in every board, agenda, timeline and
feed — in an app with no virtualisation anywhere. Stubs are an explicit opt-in that states how many
notes it will add.

**Where the lock is.** The conversion — a directory walk, a hash per attachment, `pdftotext` per PDF
— runs with the guard **released**; only the `put` loop takes it. The destination is *re-resolved*
after that window, because a vault can be forgotten or a name taken while a large graph converts.
The whole import lands as **one** commit, which is what makes it undoable as one; a failure to
record says so rather than pretending, since the notes are on disk either way.

**Host-bound.** Both commands take a server-side filesystem path, so both join `REMOTE_DENIED` for
the reason already written there for `check_path` — `check_import` is a filesystem oracle over the
whole machine. There is no folder picker, for the reason *A vault is created, not invented* already
records; the source is a typed path validated server-side per keystroke, and **one** `ok` answers
for the source and the destination together so the browser never holds a second opinion.

**Rejected:** a plain "folder of Markdown" mode (with no graph there is nothing to resolve links
against, and a silent half-import is worse than a refusal that names what it looked for); org-mode
graphs; Obsidian plugin formats (Canvas, Dataview), which are counted and named as left behind; and
any two-way sync back — *"`sync` requires git, `copy` works with anything"*, and this is a copy.

**Note on layout:** `import.rs` + `import/` is the first nested module in `crates/`; every other
crate is flat. Three cohesive files earned a directory, and `mod.rs` was avoided so the flat-file
reading order still holds.

## `/transcribe` reads writing too — one verb, two specialists (2026-08-30, `#agent` `#toolchain`)

**Decision.** `/transcribe` now covers **both** modalities: recordings become a transcript, and
handwriting, boards and photographed pages become a digital one. One verb, two specialists, chosen
by what the blob is — transcribing a recording and transcribing a page are the same request, and
making a person pick the command is making them do the dispatch. The vision capability comes from a
**multimodal projector** (`mmproj`) fetched beside the weights we already serve — not a second
model and not a second server.

**Why it costs almost nothing.** The laptop pick has been a vision-language model since 2026-07-24:
`qwen3-vl-4b`, chosen on grounding and abstention, served text-only because the projector was never
fetched. Enabling images is one file from the same pinned Hugging Face commit, under the same
Apache-2.0 licence, loaded with `--mmproj`. No new model, no new runtime, no new port.

**Four questions, answered.** This *does* widen what ships — a fetched artifact — so it earns this
entry. It **extends** the modality-specialist exception rather than contradicting anything: the
transcribe module's own docs already named "image→LaTeX, …" as the next rider, and the four
properties it established are inherited unchanged (bytes by value never a path; insertion-only;
a provenance-marked adjunct that *links* the source; idempotency on
`(blob-hash, specialist, model)`). It is **not** the parked image→LaTeX work
(`outstanding.md` §2.4): that one targets the phone and must therefore *bundle* a model into an MIT
app, which is the constraint that ruled out AGPL Texo. This fetches, does not bundle, and is desktop
only.

**The substrate is now shared, not copied** (`fm_agent::adjunct`). The insertion-only splice is a
safety invariant, and this project keeps re-learning that a rule implemented once per caller is a
rule the next caller forgets — the same argument that produced `fm_app::thread`'s single definition
of the hidden note-classes. Each specialist owns only a fence tag, so a transcript and a reading
coexist in one note and neither supersedes the other.

**A blind model must never be asked to read a picture.** That is the one failure mode that would be
silent and damaging: a text-only model handed an image does not refuse, it invents a fluent reading,
and this pipeline would file it as a provenance-marked block. So vision is a *capability with a
reason* (`decisions.md`, 2026-08-29): `serve.rs` sets it from whether the projector actually made it
onto the command line, and `/transcribe` says *which* capability is missing rather than producing something. Each is checked
separately, because they fail separately — a device with only one still does the half it can.
Truncation at the token cap is likewise a hard error, not a partial reading presented as a whole one.

**The instruction is a *transcription* instruction, and that is the substance of the feature.** Math
comes back as LaTeX, code and pseudocode as fenced blocks, tables as Markdown, and a figure's labels
are transcribed with one line naming what it is — because a photographed equation rendered as prose
is no more editable than the photograph was. And an unreadable symbol becomes `[?]` rather than a
guess: in an equation a plausible wrong character is far worse than a visible gap, since the gap you
notice and fix and the wrong subscript you carry for a year.

**Accepted costs.** The projector is a further download on top of a 2.8 GB model, desktop only —
the phone's `lfm2.5-1.2b` is not vision-capable and reports so. Images are capped at 8 MB per
reading and **refused with a reason**, never silently downscaled. And a vision model is a *witness,
not an oracle*: it misreads handwriting and invents plausible axis labels, which is why the reading
is an adjunct beside the image rather than a replacement for it, and why the block names the model.

## A proposal's outgoing commit is kept, so the *accepted* label can be true (2026-08-30, `#git` `#agent` `#data`)

**Decision.** Before the ref holding a proposal moves (revise) or disappears (reject), point
`refs/fm/review/<proposal-id>/<sha>` at the outgoing commit. Both backends, best-effort, inside
`write_proposal_branch` and `delete_branch`.

**Why — and it is not about tidying up rejects.** A human edit becomes a *new* proposal commit,
which then becomes the accept-merge's second parent, so a clean merge is **always** verbatim
relative to the final proposal. Every accepted proposal therefore looks like the model got it right
first time. Measured on the owner's vault with a prototype exporter: 13 accepts, **13 "verbatim", 0
"accepted-then-changed"** — while **5 of those 13 notes also had an orphaned earlier proposal**. The
corpus git already holds does not merely *lack* the human's correction, **it asserts something
false**, in the direction that flatters the model. Retention is what makes the label truthful.

Also measured, the same day: ten model-authored `propose:`/`revise:` commits were **unreachable and
37 days old** against a 14-day `gc.pruneExpire` default. Reject had been deleting the
highest-value signal for weeks, silently.

**Why a ref namespace, and not a branch, a merge edge, or a note.**
- Outside `refs/heads/*`, so it is invisible to every existing reader: **every history walk in this
  crate pushes HEAD alone**, so `activity`, `newest_foreign`'s squash window and `proposal_load`'s
  guardrail count are all unchanged. Verified empirically *before* the code was written, then
  pinned by the test.
- A branch under `refs/heads/proposal/*` would consume one of `DEFAULT_MAX_OPEN = 25` slots per
  retained proposal and **brick proposals after 25 reviews**, with no UI to clear them.
- **A merge edge was designed and rejected on reproduced evidence** (`merge -s ours` for reject): it
  exits 2 on a *staged* index — the project-vault case this repo explicitly supports — and
  `activity`'s `--no-merges` would then make a **rejected** proposal the note's newest touch,
  attributed to the model, which `stale()` treats as fresh forever.
- Gerrit's NoteDb keeps abandoned patchsets exactly this way (`refs/changes/…`), in production at
  Android and Chromium scale.

**In `delete_branch`, not at the reject call site**, deliberately: that function serves accept *and*
reject, and a record that depends on the caller knowing which is which is precisely the shape this
project keeps re-learning is silently forgotten (the `notes_base` exclusions, twice). On accept it
is redundant and costs one ref.

**Accepted costs.** Retained objects are never pruned, and **libgit2 exposes no gc/repack**, so a
phone accretes loose objects. Bounding this is deprecation-and-stop-exporting, never deletion —
deleting to save space is how Step 0 happened. Retention refs are **not pushed and not fetched** by
default: the corpus is per-device and unioned at export, which is also why rejected text never
reaches a collaborator.

**Pinned by** `a_revised_or_rejected_proposal_keeps_its_outgoing_commit_on_both`
(`crates/fm-core/tests/git_differential.rs`), **checked to fail** with retention neutered. The
route-parity grep cannot see this — `retain_proposal_tip` is internal to each backend — so
`pixi run test-native-git` is the only gate that catches divergence here.

## A PDF renders on the desktop; the phone opens it externally (2026-08-29, `#track-m`)

> Corrects the phone half of *The read view may frame its own blob* (same day). Written because an
> adversarial audit found the claim false — twice, by two independent routes.

**The claim was "PDFs render again on every platform". They never rendered on Android and still do
not, and the fix shipped for the phone was inert.** Three separate reasons, any one sufficient:

1. `mobile-design.md` has said all along that `open_external` is *"the only way to view a PDF on
   Android, since the System WebView can't render PDFs inline."* The always-read layer was made to
   contradict an on-demand doc — the precise failure the two-layer split exists to prevent.
2. `frame-src 'self'` **cannot match `fmblob:`**. The phone's blob URLs come from `blobBase()` as
   `fmblob://localhost/` (or `http://fmblob.localhost/` under wry's workaround), and the evidence
   was two clauses away in the string being edited: `img-src` and `media-src` both enumerate
   `asset: http://asset.localhost fmblob: http://fmblob.localhost` for exactly this reason.
3. The phone serves every blob as `application/octet-stream` (`mobile/src-tauri/src/lib.rs`),
   because the blob store is content-addressed and keeps no MIME beside the bytes. Nothing renders
   a PDF inline from that, whatever the policy says.

**So `tauri.conf.json` goes back to `frame-src 'none'`** — the phone frames nothing, and an inert
change that reads as a fix is worse than no change. `features.md` now says *desktop*.

**The process failure is the point, and it is not subtle.** The verification was a headless browser
on the desktop; the conclusion was written about "every platform". A capability claim was made for a
device that was never tested, in a repo whose standing ruling is *A feature the app cannot deliver
must not offer itself* and whose `known-issues.md` already says layout and CSP claims on the phone
are not provable from CI. **Verify on the device you are about to make a claim about, or scope the
claim to the device you tested.**

## A tag may contain a space, and both doors must agree what that means (2026-08-29, `#data`)

> Supersedes the description of `tags` in *A list property is writable…* (same day), which said
> `tags` splits on `[',', ' ']`. It no longer does.

**`tags` is comma-separated.** It split on `[',', ' ']`, so `Machine Learning` silently became two
unrelated tags — which blocks every mapping of an external name onto a tag (a Zotero collection
called `To Read`, a folder, an imported keyword) and blocks the board's own drag write-back, which
sends a column *name*. A "comma if present, else whitespace" heuristic was tried and rejected: it
left a *single* multi-word tag needing a trailing comma, a rule nobody would guess. One separator,
the same one `assets`/`code` use.

**The read side had to move with it, and that is the half that was missed.** `as_string_seq` had
just been made lenient about a bare scalar, so a hand-written `tags: alpha, beta` read as the single
tag `"alpha, beta"` — and the first save split it into two. **Opening a note and saving it changed
what it was tagged with.** `tags` now parses a scalar by splitting on commas, exactly as
`apply_property` writes it, so the same text means the same tags whichever door it came through.
`assets`/`code` deliberately keep scalar-as-one-element: a path may contain a comma.

**The cost, stated:** `todo urgent` is now one tag, not two. Visible immediately as a single chip,
and the field's placeholder and the manual both say comma.

**And this entry exists because it nearly did not.** The separator change shipped with a long code
comment, a long commit message, and **no entry** — while the entry above, written ninety minutes
earlier, still asserted the old behaviour as current. That is the four-questions step failing in the
exact way it was written to catch: the check gets applied where an entry was going to be written
anyway, and skipped where the change feels like a bug fix. **A changed default is a decision even
when it arrives inside a fix.**

## An anchored asset reference points at a place, and stays an ordinary link (2026-08-29, `#data`)

`[p. 4](asset:sha256-<hex>#page=4)` is how a note points at a **place in a PDF** rather than at the
file. The fragment is deliberately `#page=N` — the **standard PDF open parameter**, not an
invention — so the reference means something outside this app: measured in a real browser against
our own blob route, it opens the document at that page.

**Three things had to be true first, and none of them was.**

1. **`parse_ref` required every character after the scheme to be hex**, so an anchored reference was
   refused outright with *"not an asset reference"* — and `asset_status`, `resolve_asset` and
   `GET /api/blob` all route through that one function. A fragment says *where to look*, never
   *which bytes*, so it is cut there.
2. **Nothing resolved `<a href="asset:…">`.** `resolveAssets` walked `img` only, while
   `URI_ALLOWED` deliberately lets the scheme through the sanitiser — so an anchored link survived
   as a live link to a scheme nothing handles, which on Android navigates the WebView out of the
   app. That is the trap `noteChip` documents for `note:` and solves with a `<button>`. **Here a
   button would be wrong**: unlike `note:`, this resolves to a real same-origin URL, so an `<a
   href>` is the honest element and the fragment does its job. A new tab, because the note is what
   is being read.
3. **`assetUrl` percent-encoded the whole reference into the path**, burying the `#` where no
   browser could act on it. The blob half is encoded; the fragment is put back where a fragment
   belongs.

**Redaction consumes the fragment.** `bare_token` stopped at the hash, leaving
`⟨removed on copy⟩#page=4` on a cross-vault copy — only a page number, but `REDACTION`'s contract
is to carry *none* of the original, and visible debris reads as a bug.

**What this format deliberately does not carry: geometry.** A highlight is `rects` — plural,
`number[][]` — and one crossing a line break has several, which in a two-column paper is the common
case. Ink is `paths`, an EPUB position is a CFI. None of that belongs in a URL fragment, where it
would be a bespoke serialisation with no schema and no validation, embedded in prose across
thousands of note bodies. The page is the *human* anchor and the degradation path; precise geometry
belongs in a structured block in an annotation note, which is the next step and needs its own entry.

**A verification trap, recorded because it cost a wrong conclusion.** Chrome's built-in PDF viewer
applies `#page=N` only after its plugin has loaded — several seconds. A headless screenshot taken
3 s after navigation shows page 1 and looks like proof the fragment is ignored. It is not; at 8 s it
is page 3. Any future check of this needs a generous settle.

## A paper is a note, made from the citation you already have (2026-08-29, `#data` `#ui`)

**A paper is a `Kind::Note` tagged `paper`, never a `Kind::Asset`.** Forced — `views.rs`'s base
query hardcodes `Kind(Note)` for board/agenda/timeline, so an asset appears in no planning view —
and right anyway: the paper is what you tag, schedule and think about; the PDF is a blob it
references. Everything the notebook already does then works on a library for free: status columns,
`due` dates, tags, FTS, git sync, the phone, backup, the assistant.

**Metadata is flat scalars, and nothing else is possible.** `PropertyValue` has no map variant, so
`authors`/`year`/`venue`/`doi`/`arxiv`/`entry_type`/`cite_key` are strings and an `Int` where the
value is one. Values go through `apply_property`, so a pasted `year` is typed exactly as a
hand-*edited* file types it — the invariant *A list property is writable…* protects.

**Acquisition is offline, and that is the ruling, not a shortcut.** `fm-agent-run`'s `Cargo.toml`
records that `ureq` and its TLS stack are *"agent-only deps — the notes core links neither"*. So a
DOI is **recognised, never resolved**. Three offline routes, in the order they cost the user
anything:

1. **The identifier the PDF prints on itself.** `pdftotext` has already run at ingest, so the
   commonest case needs no typing at all. Only the *front* of the text is scanned — a DOI in the
   bibliography belongs to somebody else's paper, and citing it would be worse than finding nothing.
2. **A pasted BibTeX entry**, which every publisher page and every reference manager exports and
   which carries a whole record with no lookup.
3. **An identifier, a URL, or a bare title.**

One box takes all of them; the backend decides what it got. Parsing is hand-rolled in
`fm-app/src/paper.rs`, no regex and no dependency, matching `refs.rs`.

**`create_paper` writes once.** Every field is applied in memory and `put` once, so a failure cannot
leave a paper with a title and nothing else — and a future library import is not tens of thousands
of round trips, each a full file rewrite plus an index update.

**Deliberately not guessed:** a bare `2401.12345` is not an arXiv id. In running text it is as
likely a figure number or a price, and a false citation on somebody's note is worse than an empty
field.

**BibTeX out is server-side** (`commands::paper_bibtex`) so the format has one implementation whose
round trip with the parser is tested; a copy in TypeScript would drift. It emits the venue under the
field the entry type actually takes — `booktitle` for a proceedings, `journal` for an article — which
a round trip through our own parser could never catch, because both map back to `venue`.

**`hayagriva` (CSL, 2,600 styles) is the obvious upgrade and is deliberately not taken**: it is a
dependency owing its own entry, and one BibTeX entry is a `format!`.

## One tag is an arrangement, not a query builder (2026-08-29, `#ui`)

> Extends *A saved view is an arrangement you keep, not a query you write* (2026-08-28) — the
> reasoning there is unchanged and still governs; this widens the surface by exactly one question.

**The app could not produce a filtered view at all.** `save_view` wrote `name`/`view`/`group_by` and
nothing else, and **refused outright** to overwrite a view carrying a filter, so the only documented
way to have one was to author YAML plus a nine-row predicate grammar in a text editor — for a
headline feature, in an app whose owner works only through the UI (`outstanding.md` §2.6b). The
2026-08-28 entry was right that a UI over that grammar is a query builder nobody non-technical would
use. It left a gap it did not intend: *no* filter, ever, from the app.

**One `tag:` is the whole widening.** "Show me the ones tagged `paper`" is a sentence a person says,
and it is the narrowing they already perform by eye. It is not a predicate language: no operators, no
composition, no nesting — one optional field on the save dialog, empty by default.

**A richer filter is still never flattened.** A hand-written view may carry anything the grammar
allows; this surface can express exactly one tag, so `save_view` refuses anything else and says why,
leaving the file untouched. Re-saving a view whose filter *is* a single tag is allowed — that is the
one shape it can faithfully rewrite (`PredDto::is_only_tag`).

**Two smaller things fell out, both of them the same §2.6b failure.** The naming step was a
`window.prompt()`, which can ask exactly one question — replaced by a dialog that asks both at once.
And `newView` is a command whose default key is `''`, in an app whose command palette was removed:
**"New view" was a labelled capability with no way to invoke it** unless the user bound a key in
Settings. There is now a *Save view* button in the pane header, on the three kinds that *are* an
arrangement (board, agenda, timeline) — a note, a search or an existing view is not one.

**Consequence.** `crates/fm-app/tests/save_view.rs` pins the written YAML, the empty-tag case, the
re-save, and the refusal (including that the original file survives it); `ui/src/App.saveView.test.ts`
pins the dialog, and stubs `window.prompt` to **throw**, so a regression to the prompt fails rather
than quietly passing.

## `Kind` is the second predicate that reaches SQL, and the index may denormalise to make that cheap (2026-08-29, `#seams` `#data`)

**`FileStore::candidates` pushed exactly one predicate down — `Text`, via FTS5 — and answered every
other query with `load_all()`.** That was right while a note was prose: a few hundred bytes, and the
count is what grows. It stops being right the moment a vault holds papers, because `commands::asset_note`
puts a PDF's **extracted text in the body of its asset note**, measured at 1.7–2.2 KB per page. A few
thousand papers is >100 MB of body — and `board`, `agenda`, `recent`, `timeline` and `run_view` all
filter `Kind(Note)`, so every one of them YAML-parsed and copied all of it, on every view switch, to
throw it away.

**`objects` now carries a `kind` column**, and `candidates` narrows the non-FTS branch with it.
Denormalising is legitimate here precisely because **the index is disposable**: it is rebuilt from
the files on open, so it may hold whatever makes a query cheap, and `INDEX_SCHEMA` (bumped 1 → 2)
turns an older index into exactly one free rebuild.

**The predicate is pushed down but deliberately *not* stripped from the residual filter** — unlike
`Text`, which must be stripped because FTS5 is pinned to `remove_diacritics 2` and a second
substring pass would drop folded matches. `Kind` re-applied is simply idempotent, so the pure engine
still decides the answer and `MemoryStore` cannot diverge. The narrowing is an optimisation that
cannot change a result, which is the only kind of pushdown worth having at this seam.

**Only a *top-level* `Kind` is pushed.** `Filter.all` is a conjunction, so one sitting directly in it
must hold for every row; one nested inside `Not`/`Any` does not narrow anything and pushing it would
silently drop rows the engine would have kept.

**The budget that proves it is a ratio, not a millisecond count.** Every existing `perf.rs` case
seeds ~55-byte bodies, so all of them measure note *count* and none can see bytes per note — this
change was invisible to the entire suite by construction.
`a_planning_view_does_not_hydrate_the_assets_it_filters_out` times the same `Kind(Note)` query over
the same 300 notes with and without 600 asset notes carrying 36 MB of text, and asserts the two are
comparable: **measured 1.5x with the pushdown and 4.9x without**, so it fails against the old code.
That follows the lesson `known-issues.md` recorded after the O(n²) rebuild — *assert the shape, not a
duration* — because a wall-clock budget only catches what someone thought to measure at the right
size.

## A list property is writable, and a scalar where a list belongs is read, not dropped (2026-08-29, `#data`)

**`assets` and `code` had no write path, and losing one was silent.** `apply_property` matched
neither key, so both fell through to the `extra` catch-all, which writes a `PropertyValue::Text`;
`to_file` then emitted a **scalar** (`assets: sha256:…`) while `from_file` reads them with
`as_string_seq`, which answered `Vec::new()` for anything that was not a sequence. The value was
gone on the very next load, with no error at any layer — on the one field that ties a note to its
blob. `set_property(id, "assets", …)` therefore appeared to succeed and detached the attachment.

**Two halves, and the second is a repair.** `apply_property` now has `assets`/`code` arms writing
the typed fields; and `as_string_seq` now accepts a **bare scalar as a one-element list**, which
recovers every note already written that way and matches the leniency `Kind::from_str` extends to
legacy `type:` values. A note is a file a person may write by hand, and `assets: sha256:abc` is the
obvious thing to type. Anything that is neither a sequence nor a string is still nothing.

**Comma, never space — deliberately not what `tags` does.** `tags` splits on `[',', ' ']`, which is
why a multi-word tag is unrepresentable through the only write path this app has.
*(**SUPERSEDED the same day**: `tags` is comma-separated now too — see *A tag may contain a space*
below. The reasoning here is why; the description of `tags` is no longer current.)* A blob reference
or a path may not contain a space, so splitting on one buys nothing and costs the same expressivity.
The imitation was the trap: the obvious way to add these arms was to copy the `tags` arm.

**Consequence.** Pinned by `setting_assets_or_code_survives_a_reload`,
`assets_takes_several_references_separated_by_commas` and
`a_hand_written_scalar_asset_is_read_as_a_one_element_list` in `crates/fm-core/tests/edit.rs`; the
first two fail against the old code. The durable lesson: **a typed field with no arm in
`apply_property` is not read-only, it is silently lossy** — the catch-all accepts the write and the
serialiser changes its shape. Any future `Vec<String>` or non-`Text` field needs an arm on the day
it is added.

## The read view may frame its own blob — a policy clause had been forbidding a shipped feature (2026-08-29, `#ui` `#track-m`)

> **CORRECTED same day** by *A PDF renders on the desktop; the phone opens it externally* below —
> the desktop half of this entry stands; **its phone half was wrong and is withdrawn.**

**No PDF has ever rendered in this app.** `ui/src/lib/render.ts` shows every PDF in an `<iframe>`
pointing at `/api/blob/…` — the design `MASTERPLAN.md:341` calls for, native elements, no JS media
library — while the app policy carried **`frame-src 'none'`**, which blocks every frame source
including `'self'`. Both policies had it: the desktop `CSP` and `mobile/src-tauri/tauri.conf.json`.

**Why it survived.** A blocked frame is a broken-document placeholder, not an error — nothing throws,
nothing logs where a user would look. And the one test over that path, `render.test.ts`, asserts the
`<iframe>` **in jsdom, which applies no CSP at all**, so it passed throughout. Meanwhile
`SettingsPanel.svelte` and `packaging/README-release.txt` both told the user PDFs were *"stored,
opened and shown"*. This is the same failure as *"A feature the app cannot deliver must not offer
itself"* below, arriving from the opposite direction: there the capability was missing and the string
lied; here the *feature was built correctly* and a security header quietly forbade it.

**Measured, not reasoned about.** A headless browser served the exact shipped headers reported
`CSP BLOCKED: frame-src ← /api/blob/…`; the same page with `frame-src 'self'` rendered the PDF.

**What the widening does not buy anyone else.** DOMPurify's default tag allowlist contains no
`iframe`/`object`/`embed`/`frame` — checked against the pinned 3.4.12 using `render.ts`'s own
`ALLOWED_URI_REGEXP`, all four sanitise to `""`. A note body arrives from collaborators through the
merge driver and still **cannot introduce a frame**; the only frame on the page is the one the
renderer builds itself, *after* sanitising, from a blob `inline_safe()` had already agreed to serve
inline. `'self'` and no more: never a remote document.

**Two things deliberately left alone.** The blob response keeps its own
`Content-Security-Policy: default-src 'none'; sandbox` — it was the other suspected blocker and was
measured *not* to be one; the PDF renders with it in place, so the belt stays beside the braces. And
`MANUAL_CSP` keeps `frame-src 'none'`: the book is static HTML and frames nothing.

**Consequence.** Pinned by `the_policy_lets_the_read_view_frame_its_own_pdf`, which asserts against
the real response header and **fails against the old value**. The durable lesson is the test's, not
the clause's: *a CSP clause is a claim about what the app is allowed to do, and a DOM test in jsdom
can never check it.*

## A feature the app cannot deliver must not offer itself (2026-08-28, `#agent` `#ui`)

The owner's frame for the release: **a user picks the features they want and never thinks about what
dependency allows it — and asking for a feature must deliver it working, like any app.** Auditing
against that found three places the app claimed a capability it did not have. All three are breaches
of a rule this repo already wrote down: *"a capability must mean 'this will work', never 'this is
configured'"*.

**The assistant was the serious one — the only place the app told a user something worked when it
had not.** `agent_status` answered from a stored flag that cannot fail, so the switch rendered,
`set_agent` returned `{"ok":true}`, and the UI promised *"Starts with formicaria on the next
launch."* — while the release archive ships no `agents/` directory at all and the sole diagnostic
was an `eprintln!` to a stderr the Windows launcher hides by design. It now reports **capability**,
and turning it on where the stack is absent is a **409 with a plain sentence** rather than a cheerful
ok. The stack is also resolved beside the running binary instead of against the working directory —
a bare relative path resolves against whatever a file manager handed the process, so a release could
never have found it even had it been shipped.

**PDF search failed in total silence.** `pdftotext` had no capability check and no user-facing string
anywhere: a PDF ingested with an empty body and search never found it. Now declared like `git` and
`restic`, and named for what it does — *"search inside PDFs"* — not for poppler.

**Previews were advertised and unbuilt.** `render.ts` records that nothing ever requests a thumbnail;
installing libvips delivered nothing. The claim is gone from the release sheet and the manual; the
generation half stays, and building a consumer is in `outstanding.md`.

**And the release sheet's "Optional features" was itself the framing error** — a *tool* table asking
the reader to reason from dependency to capability. Rewritten as capabilities, with the tool name
demoted to a parenthetical.

## A saved view is an arrangement you keep, not a query you write (2026-08-28, `#ui`)

> **SUPERSEDED 2026-08-31** (*view customization is withdrawn…*). The reasoning below stands; what
> is withdrawn is the **surface** — there is no longer a Save view button or palette entry. The
> command, its `ipc.ts` wrapper and every Rust test remain, and a `.view` file still lists and opens.
> The owner's ruling: *"views are basically fixed for now and view customization will need its own
> design plan."* Read that entry before adding any view-authoring control back.

`.view` files could be **neither written nor deleted from the app** — the documented way to have one
was to author YAML plus a nine-row filter grammar in a text editor, for a headline feature, in an app
whose owner works only through the UI. A keyboard command *labelled* "New view" opened the Settings
list: a label promising a capability that did not exist.

**Decision: save the arrangement, not the query.** `save_view` writes what the user is looking at —
the renderer, and a board's grouping — under a name they choose. **Rejected: a filter editor.** The
grammar is nine kinds of predicate and a UI for it is a query builder, which is exactly the thing a
non-technical user was never going to use and the reason the manual documented a text editor instead.
What a person actually does is arrange a board and want to keep it.

Two consequences worth stating. **Saving over a view that carries a filter is refused**, not
flattened — a hand-written view's filter must not vanish because someone pressed save. And **the
filename is derived from the name and never trusted**: a view name reaches this from the UI, so
`../../escape` becomes `------escape.view` inside `views/` (verified) rather than choosing where in
the filesystem we write. The label the user typed is preserved verbatim inside the file, which stays
the same YAML a person would write by hand — it is theirs, in their vault, tracked by git and read by
collaborators.

Editing an existing view's **filter** stays out of the app; the manual should say so plainly rather
than documenting YAML as the way in.

## libgit2 ships on Windows too — the exception is "no git binary", not "not a desktop" (2026-08-28, `#git` `#toolchain`)

**Reversal chain (`#git`):** widens *"The libgit2 exception"* (2026-07-19), which scoped it to
mobile. That entry's reasoning is unchanged and its conditions still hold; only the boundary moves.
Read the pair — and read *"`git2` is rejected"* (2026-07-18) beneath both, which this does **not**
revive.

**Why now.** A Windows user was told *"Git is not installed"* when backing up, and lost history,
backup and collaboration entirely. Windows ships no git and most people will never install one. The
capability to serve them already existed, fully tested, unreachable: `fm_core::vcs` prefers a git
binary and **falls back to libgit2**, built for the phone for exactly this reason, and `fm-serve`
never compiled it in.

**The boundary was drawn in the wrong place.** The 2026-07-19 exception says *mobile*, because at the
time the phone was the only device with no git binary. But the property that earned the exception
was never "is a phone" — it was **"has no git binary"**. Windows satisfies it identically. Stating
the rule by its reason rather than by the example that prompted it is the whole change.

**What this does not touch, and the checking matters more than the conclusion.** It is *not* a
reversal of *"the desktop keeps shelling out"*. `vcs.rs` selects at **runtime**, so a machine with
git is byte-for-byte unaffected — it still uses the binary, and still gets the `.md` merge driver a
collaborator's terminal `git pull` uses. And it does **not** engage Track M ruling 1's sequencing
gates (build the body-merge engine and differential harness *before* swapping): those bind
*replacing* the desktop's backend, not *adding a fallback for machines that have none*. The
`git merge-file` oracle is untouched because the path that uses it is untouched.

**Consequence.** `crates/fm-serve/Cargo.toml` gains a `[target.'cfg(windows)'.dependencies]` entry
pulling `fm-app` with `native-git`. Deliberately a **target-gated dependency rather than a feature
flag**: nobody has to remember to pass it, the release workflow needs no change, and — measured —
`cargo tree` resolves 0 `git2` for the Linux target and 1 for the Windows target, so **`pixi run ci`
on a Linux machine still never builds libgit2 + OpenSSL**. That was a stated position in three files
and it survives intact rather than being overturned.

`deny.toml`'s wording moves with it: the exception is one crate for *any device with no git binary*.
`cargo deny` still cannot see libgit2's real licence and never will, so `ci/checks.sh` carries the
enforcement — now asserting that **Linux and macOS** desktops pull no `git2`, checked against those
targets explicitly rather than against whichever host happens to run it.

**Accepted costs, named rather than discovered later:** a larger Windows binary and a slower Windows
build, since `git2` is pinned to `vendored-openssl` for Android's sake and that is not yet split per
target — Windows compiles OpenSSL it does not need (`outstanding.md`). The two grounds of the
2026-07-18 rejection never retired — weight, and decade-scale maintenance — are accepted here for
Windows specifically, on the basis that a notebook which cannot keep history on the world's most
common desktop OS is not a notebook anyone should be asked to trust.

**Rejected:** telling Windows users to install Git for Windows — the download this was meant to
remove. **Rejected:** shipping a `git.exe` beside the binary — a GPL tool linked in spirit, and the
one-file ruling refuses it. **Rejected:** enabling it on every desktop — Linux and macOS overwhelm-
ingly have git, and the narrower rule is the one whose reason is true.

## An addition is checked against the record before it is written (2026-08-28, `#toolchain`)

Twelve commits landed in one session, each driven by a real user report and each defensible alone.
That is the condition under which a project accretes: every step is justified locally and nothing
checks the sum. The owner asked for a step that catches it.

**The gap was specific.** This repo has an append-only decision log with a subject index, a router,
22 architectural gates and a licence gate — and **nothing that answers "does this contradict
something already ruled on?"** The evidence, gathered rather than assumed:

- **`SUPERSEDED` appears twice in 2,196 lines** — the rule itself, and one banner. Yet an entry
  titled *"Squash-on-push — a deliberate reversal of 'don't build commit management'"* reverses a
  `MASTERPLAN.md` passage that still states the original, unbannered. The convention is not applied
  and nothing notices.
- **The cold-read audit is the only recall check** — manual, quarterly, self-graded. Six of seven
  answered wrong on 2026-07-19; no count recorded since.
- **An entry written that same morning promised a `ci/checks.sh` grep that was never built.** Now
  corrected in place.

**The step** (in `CLAUDE.md`, with a router row): name the subject · read the entries **and read
past them** · classify *permitted / extends an exception / contradicts* · ask whether it widens what
ships. If it contradicts, write the dated reversal **first**.

**"Read past them" is the load-bearing half**, and it came from being wrong. The index files the
desktop-libgit2 question under `#git` and names a two-entry chain. The decisive text is a **third**
entry under `#track-m` — the owner's binding Track M ruling 1 — which names the desktop as the
intended destination and attaches sequencing gates. Reading only what the index pointed at produced
a confidently wrong answer. **An index is a starting point, not a contents page.**

**One guard, chosen because it is the near-miss that prompted this.** `deny.toml` says *"libgit2,
for mobile only"* three times; `cargo deny` reports `licenses ok` for libgit2 **and always will**
(`libgit2-sys` declares MIT/Apache and says nothing about the GPL C it vendors); `checks.sh` passed.
Enabling `fm-serve/native-git` is one line, changes no behaviour on a machine that has git, breaks
no test — and makes that central claim false with nothing red. So `ci/checks.sh` now asserts the
claim mechanically: no shipped desktop crate may pull `git2` in a default build, with a canary that
fails if the check can no longer see git2 even when the feature is on. Widening the exception stays
allowed; doing it **silently** does not.

**Rejected:** guards on every widening surface (dependencies, CSP constants, staged files, defaults)
— one guard that fires beats three that produce noise people learn to route around. **Rejected:**
documentation alone — this repo already documents the discipline it then did not apply.

## The manual's CSP is a named exception, not an oversight (2026-08-28, `#ui` `#track-m`)

`main.rs` calls `script-src 'self'` with no `'unsafe-inline'` **"the clause worth protecting"**, and
records the price already paid for it: Excalidraw's asset-path line was moved out of `index.html`
into its own file rather than relax the policy. The precedent is *restructure the content*.

Then the embedded manual arrived and `MANUAL_CSP` allowed `'unsafe-inline'`, because mdBook writes
six inline `<script>` blocks per page (`path_to_root`, the pre-paint theme, the sidebar) and without
them the manual renders with no theme, no chapter list and no search. That relaxation was reasoned
about and written down — but never checked against the standing clause, which is the whole reason
the conformance step above now exists. Found by applying it retroactively.

**Decision: keep the exception, scoped and named, and state its price.** It applies to `/manual/*`
only; the app's own `CSP` is untouched. What pays for it: `connect-src 'none'` and `form-action
'none'`, both **tighter** than the app's policy, so a manual page can style itself and provably
cannot reach `/api/`. The content is our own build output — no note body, no user text, reaches it.

**The conformant fix, and why it is not being done now.** Per-page CSP hashes: each page's inline
scripts differ, but the union of hashes across the book is bounded and could live in one header
computed at build time. That needs SHA-256 inside `fm-serve/build.rs`, which is std-only by
design — so the fix for a philosophy violation would itself add a dependency to a crate kept
deliberately bare. That trade deserves its own decision rather than being smuggled in here.

**Rejected:** leaving it unrecorded. A relaxation of the one clause the codebase singles out as
worth protecting is exactly the thing that must not sit in a source comment only.

## Opening the browser and quitting with the tab are defaults, not opt-ins (2026-08-28, `#toolchain`)

A user double-clicked `fm-serve.exe` on Windows, copied the address out of the console into their
browser, and then **closed the console** — which stopped the server. Reported alongside the
observation that this cannot happen on Ubuntu, where no terminal appears and there is nothing to
think about.

**That is not a mistake a person makes; it is a trap the program set.** The console printed an
address, which invites copying it and then tidying the black window away, and the one thing keeping
the notebook alive was that window. Every part of it was working as designed.

`FM_OPEN` and `FM_AUTO_SHUTDOWN` both existed and both fixed it — and both were **opt-in, set only
by a launcher**. So the behaviour that makes the app usable was reserved for people who had already
found the right file to click, and denied to the person most likely to click the wrong one. The
defaults are now on. The asymmetry decides it: **someone who double-clicks cannot set an environment
variable, and someone who can set one is a developer who can equally turn it off.** `pixi run serve`
opts out of both explicitly — there a browser tab per restart is noise, and a closed tab means "about
to reload", not "finished".

On Windows only, startup now also says *"Keep this window open while you work. Closing it stops
formicaria."* — because there the console belongs to the program and closing it is fatal, whereas on
macOS and Linux a bare launch happens in a terminal the user already owned.

This does not remove the console, which is what the report actually asked for. That needs
`windows_subsystem = "windows"`, whose real cost is that **every** subprocess then flashes its own
window — and `fm_core::git` shells out from a debounce that runs every few seconds. Measured while
writing this: **23 `Command::new` sites in the whole workspace, 3 of them git.** Contained enough to
route through one spawn helper with `CREATE_NO_WINDOW`, which is the prerequisite; a `ci/checks.sh`
grep would then be needed to keep it that way, since a new `Command::new` would otherwise regress it
silently and only on the platform nobody here can debug.

**Neither the helper nor the grep exists yet — this paragraph describes work, not a mitigation in
place.** Corrected the same day it was written, because the original wording read as though the
guard were part of the change. This repo's own rule is that *a mitigation naming a mechanism must
name an executor that exists* (`README.md`), and an entry that quietly claims a guard it never built
is exactly the drift the conformance step above was added to catch.

## The manual is built per OS, and the short path is the only path a beginner is shown (2026-08-28, `#toolchain`)

The owner's reading of the tester's experience, and it is the sharpest framing yet:

> *"People do not read readmes, they want clear manuals, not too long, that have the instructions
> for their systems as well. Easy to setup and understand."*

Three claims, and only one is a writing problem.

**"They do not read readmes"** retires an assumption this repo had been leaning on. Effort spent on
`README.txt` is effort spent on a file most people never open; the *manual* is the artifact, and
until now it was the one thing the app never pointed at and the archive made hard to find.

**"Instructions for their systems"** was available for free and nobody had taken it. **We already
ship one archive per OS** — the Windows zip knows it is Windows — so a setup chapter listing three
platforms makes every reader skip two-thirds and work out which third is theirs. The one thing a
first-time reader must not have to do is *choose*. So `user/setup.md` is now **assembled, not
written**: `ci/docs.sh` picks `setup-<os>.md`, and the OS is **detected rather than passed**,
because each release job already runs on the target it builds for — the Windows runner produces the
Windows manual with nobody having to remember. `FM_DOCS_OS` overrides for preview; an unknown value
**fails the build** rather than shipping the wrong instructions, which is the failure that would
otherwise be silent and land on a user. The assembled file is gitignored and carries a
"GENERATED — do not edit" banner, because a generated file inside a hand-edited tree eats somebody's
work exactly once.

**"Not too long"** is answered by *what a beginner is shown*, not by deleting anything. The user
guide was eight chapters and ~7,200 words before anyone reached "how do I write a note". Now
**Start here** is two short chapters — set up, then *Your first ten minutes*, which ends by telling
the reader they are done and can go and write — and everything else moved behind **Going further**
and **Reference**, with the developer guide last instead of sharing a beginner's sidebar. Nothing was
removed; the reading *order* was the defect.

**The rule worth keeping: per-artifact truth beats per-reader instruction.** We knew the platform at
build time and were still asking the reader to work it out. Whenever a document has to say "if you
are on X, do this", check first whether the thing shipping it already knows which X.

## The archive's top level is a door, not an inventory — and the readme opens in Notepad (2026-08-28, `#toolchain`)

The same tester came back after the packaging work with three sentences, and every one of them is a
defect we could not have found ourselves:

> *"There is no clear manual to install formicaria upon clicking the manual folder. The README file
> is in an MD file format, I would prefer it to be in a format where I can open it in Notepad as I'm
> unable to open in MD file format. [The] manual folder [shows] many miscellaneous files which may
> cause confusion."*

**`.md` is a developer's file extension.** Double-clicking one on Windows offers an app picker;
`.txt` opens Notepad. We had shipped the one document a stranded user most needs in a format they
could not open, and called it a readme. It is now `README.txt`, written as plain text — no tables,
no backticks — and converted to CRLF for the Windows archive, because Notepad on older Windows 10
renders an LF-only file as one unbroken line.

**A website's entry point is only obvious to someone who knows it is `index.html`.** mdBook emits
~50 files — chapters, stylesheets, fonts, a search index — and we shipped that directory under a
name promising "the manual". Opening it is the reasonable thing to do and it yields noise. So the
folder keeps the machinery and a single `Manual.html` sits one level up and redirects into it. The
door is the artifact; the book is what is behind it.

**And the top level itself was an inventory.** Nine entries, three of which looked launchable
(`formicaria.vbs`, `formicaria.bat`, `fm-serve.exe`) — we had answered "what do I click?" three
times with three different files. Binaries and licence notices moved to `program/`, launchers were
renamed to **`Start formicaria.*`**, and the unzipped folder is now six entries of which exactly one
says *start*.

**The reusable rule: legibility is a property of the artifact, not of the documentation.** Every one
of these was already explained correctly in prose that the user never got far enough to read. A
readme cannot rescue a folder that does not say what to do, and `ci/checks.sh` now holds the whole
chain — `README.txt` → `Manual.html` → `manual/index.html`, plus the binaries staying under
`program/` — because each link is invisible to us and load-bearing for them.

The deeper reason we shipped it: **we had never once unzipped our own release and looked at it.**
Every check we had asserted that files were *present*, which is not the same question as whether a
stranger can tell what to do with them.

## What ships is an app, not a binary — the launcher owns *where*, the server owns *why* (2026-08-28, `#toolchain`)

A non-technical tester opened v0.2.0 cold and reported: the manual assumes a terminal, the manual
was hard to find, and they expected a portable double-click app. Every complaint was structurally
right, and the release was worse than they could see.

**Every feature they were missing already existed and was unreachable.** `FM_OPEN` opens the
browser, `FM_AUTO_SHUTDOWN` stops the server when the tab closes — and the only two files that set
them, `packaging/formicaria.sh` and `install.sh`, are checkout-only and **were never staged into
the archive**. So a download opened no browser and never stopped, which meant the *next* launch met
a held port, and `main.rs` answered that with `panic!`: on a Windows double-click, a console that
flashes and vanishes. Three defects, one cause — nobody had ever run the artifact as a user.

So: **three archive-relative launchers in `packaging/launcher/`**, one per OS, staged by target.
They are deliberately not `packaging/formicaria.sh`, which assumes `target/release/`, `.pixi/` and
falls back to `pixi run build` — from a USB stick that would try to compile the workspace.

**Portable means both vault variables, absolute, resolved from the script's own directory.** Not
`$PWD`: a file manager launches from the user's home, which is the exact hazard behind the
2026-07-17 removal of the relative `FM_VAULT="vault"` default. That ruling kept `FM_VAULT` set
*explicitly* valid, and this is that path, not a revival of what it banned. `FM_VAULTS` must be set
**with** it, because `FM_VAULT` alone stops working the moment a second vault exists — `save`
materialises the list, `load` starts preferring the file, and vault #1 disappears. Verified by
running an unpacked archive from `/`, from a path containing a space: the note landed beside the
app and `~/.config/formicaria/vaults.json` was byte-identical afterwards.

**The division of labour is the reusable part.** A launcher decides *where*; everything that can
fail once the server is up is the server's job to explain, because three launchers cannot each
learn to speak HTTP and the answer must be identical on all three. Hence the busy-port logic moved
*into* `fm-serve`, where it is testable on every platform: it probes `/api/alive`, and answers "the
app you asked for is already running" (exit 0, open the browser) differently from "something else
holds this port" (exit 1, name `FM_ADDR`). Telling those apart matters — guessing wrong either
strands the user or points their browser at a stranger's server.

**`FM_OPEN=0` used to mean yes.** `var_os().is_some()` reads presence, not value. Arguable while
nobody edited the launcher; not arguable now that one ships inside the archive, where changing a 1
to a 0 is the obvious way to turn something off.

**The embedded manual needed its own CSP, and this is the trap worth remembering.** mdBook writes
six inline `<script>` blocks per page — `path_to_root`, the pre-paint theme, the sidebar — and the
app's `CSP` has no `'unsafe-inline'`. Served under it the manual returns 200 with correct bytes and
renders with no theme, no chapter list and no search. **A `curl` check calls that a pass**; only a
browser sees it. `MANUAL_CSP` is looser in exactly one clause and tighter in two: inline script is
allowed, `connect-src`/`form-action` are `'none'`, so a manual page can style itself and provably
cannot reach `/api/`. It also must not inherit the SPA fallback — a mistyped chapter answered with
`index.html` renders the *notebook* and looks like a page that exists.

**`pixi run build` now depends on `docs`.** The binary bakes `docs/book` at compile time, and the
release workflow's order was build-then-docs — which would have shipped a binary whose Help is
empty beside a `manual/` folder that is perfectly correct. Nothing fails; the button just does
nothing. That is the whole class of bug this release exists to stop.

## The manual travels in the archive — documentation that is built and discarded does not exist (2026-08-28, `#toolchain`)

`docs/src` has been a complete 16-file manual for months: ~11,600 words on views, notes, media,
search, backup, collaboration and the assistant, plus a full command and frontmatter reference. CI
rendered it on every single run — `pixi run docs` is in `[tasks.ci]`, and `docs.yml` exists for
nothing else. **Nobody ever received it.** `docs.yml` uploads no artifact and deploys no site, so
the HTML died with the runner; `release.yml` copied five files and none came from `docs/`; and
`packaging/README-release.txt`, which ships as the archive's `README.txt`, is an install sheet that
did not even link to it. Someone who downloaded a release got the whole application and no
instructions for using it.

**The trap is that this looked healthy from the inside.** A green `docs` task and a workflow named
`docs` both report that documentation is *being built*, which is not the same claim as
documentation *reaching a reader* — and nothing can fail for a document nobody was handed, so the
gap is invisible to CI by construction. It is the same shape as the surfaces that would not say
what they knew (2026-08-24), one layer out: the pipeline was honest about every step it ran and
silent about the step that was missing.

So the `stage` step now puts the manual in every archive: rendered HTML at `manual/` (open
`manual/index.html` — fully offline, mdBook's search is client-side and every asset reference is
relative) and the Markdown at `manual/source/`. Both forms deliberately: the HTML is what a reader
wants, the Markdown is what survives, and a manual you own as plain files is the same promise the
notes make.

**Delivery is asserted, never assumed** — the rule `ci/android-release.sh` already applies to its
own artifact. `mdbook` can exit 0 having written nothing, and `docs/src` is a *tree*, so the
obvious `cp docs/src/*.md` ships `SUMMARY.md` and `introduction.md` and drops the fourteen chapters
that matter without a word. The step checks `docs/book/index.html` is non-empty and compares the
staged page count against the source count, so a short copy fails the build instead of shipping a
`manual/` that looks like documentation until you open it. The assertion is on the **staged**
`manual/index.html` rather than mdbook's `docs/book/index.html` — the path the archive's README
sends the reader to is the path the build owes them, and checking the tool's output instead stops
one step short of the artifact. Verified locally against a real archive: 16 of 16 pages, and all
three failure modes caught — the glob mistake at 2 of 16, an empty book, and a tampered page.

**The whole book ships, developer guide included.** Splitting it means maintaining a second mdBook
or a filtered copy, and a manual missing four chapters is a manual whose table of contents lies; a
reader with no interest in `dev/architecture` simply does not click it.

`ci/checks.sh` now holds the sheet and the workflow to the same paths. That is the `docs/context`
router check one layer out, and the difference is who pays: a stale router costs a maintainer one
grep, a stale release sheet costs a *user* the manual — silently, offline, inside an artifact they
have already downloaded and cannot diagnose.

Not fixed here: there is still no in-app route to the manual — see `outstanding.md`.

## A hand-fired release names itself after the ref it was fired on (2026-08-28, `#toolchain`)

`release.yml` stamped the archive version `dev-<sha>` whenever the event was `workflow_dispatch`.
That was correct while `push: tags: ['v*']` existed beside it: a dispatch then genuinely meant *"a
binary from this commit, without cutting a tag"*, and a sha was the only honest name for such a
build.

The no-remote-CI standing order (2026-07-18) removed the tag trigger from every workflow. That
silently changed what a dispatch **means** — firing by hand on a tag became the only way a release
gets built at all — but the condition still asked the event name. So v0.2.0 would have shipped as
`formicaria-dev-3493b4a-*`, attached by the `attach` job to a `v*` release, where the one thing a
downloader needs to read is which version they are holding.

The test is now `GITHUB_REF_TYPE`, which is exactly `tag` or `branch`: a dispatch on a branch keeps
the dev- stamp, a dispatch on a tag is named after the tag.

**The general shape is the part worth keeping.** Disabling a trigger does not merely remove a path;
it changes the meaning of the paths that remain. Any condition that was really asking *"which ref am
I on?"* through the proxy of *"how was I started?"* becomes wrong the moment that proxy stops
holding — and it fails quietly, in the artifact's name, long after the change that broke it.

## A view says what it leaves out, and a board admits it scrolls (2026-08-24, `#ui`)

**Decision.** Two things a surface must disclose about what it is not showing:

1. **`run_view` returns `filters`** — one plain-English phrase per `filter:` entry, built by
   `views::describe_pred` from the file's own DSL (`status is not done`, `tagged lab`, `due between
   X and Y`), and the pane header renders it as a chip that opens the **unfiltered** built-in
   renderer with the same grouping. It rides on `ViewResult`, **not** `ViewInfo`: the words belong
   to the payload they describe, and `list_views` is a separate fetch that runs once per vault
   change and has failed outright on the phone before — an explanation that can arrive late, or
   never, is the bug over again.
2. **The board carries a rail** naming every column with its card count, marking the one at the
   edge and jumping to any of them. It is always in the DOM and shown by CSS — because *whether it
   is needed* is a layout question and layout is the one thing jsdom cannot answer.

   > **AMENDED, then REVERSED, 2026-08-31.** First the unconditional half was dropped (a phone
   > showed the rail even when every column fit). Then the rail was **removed entirely**, at the
   > owner's instruction and against advice — see *view customization is withdrawn, the board rail
   > goes*. Clause 1 (a filtered view says what it leaves out) **stands and is now load-bearing on
   > its own**; clause 2 is gone, and with it the answer to "my done column is not showing up" on a
   > narrow screen.

**Why.** The owner: *"My done column in the board is not showing up, even though I have done
notes."* The notes were there and the built-in board had the column. Two mechanisms can take a
column off the screen and neither said a word: a saved view (`view: board` draws through the **same
renderer** as the Board pane — same pixels, one column fewer) and horizontal scroll (a narrow pane
snaps one column at a time, so column four is three swipes away). The vault's only `.view` was the
first case.

This is the discipline the manual already stated for a *broken* view — *"a broken view tells you
why … so 'no matches' never masquerades as 'your file is wrong'"* — extended to a **working** one:
a filter that deletes a column must not let the absence masquerade as missing notes. It matters
more here than it would elsewhere because a `.view` file can be neither authored nor deleted from
the UI, so the filter was unreachable as well as unseen (`known-issues.md`).

**Consequence.** `describe_pred` carries negation *into* each phrase (`not:` recurses with `neg`
flipped) so a filter reads as English rather than as an expression, and it is total by
construction — it runs over the parsed file, before `lower_pred` validates it, so a conjunct the
engine would reject still gets words instead of a panic. The renderers stay literal-free: the rail
labels are `col.label`, and the chip's words come from the server. Pinned by `views.rs`' own tests,
`ui/src/App.filteredView.test.ts` and `ui/src/renderers/Board.rail.test.ts`.

## Backup is sufficient on its own — `commit` records the backlog, not just this process's writes (2026-08-20, `#data`)

**Decision.** The `commit` arm stages the paths this process wrote **plus** everything `adoptable`
finds outstanding in the vault. Recording is no longer a thing a user can be required to do: Backup
(`commit → push`) clears the "N not in history" chip by itself. `record_unrecorded` stays as a
targeted repair, not as a step anyone must know about.

**Why.** The owner: *"When I press backup it should commit and push. I do not commit as a user,
that is a background concept."* They were looking at **180 notes not in history in a vault that has
a remote**, which Backup would not clear. Committing is git's model, and Backup is the only
durability affordance the product offers — so whatever Backup does has to be enough.

**Consequence — this reverses a guarded invariant, and the reversal is the point.**
`unrecorded_after_restart.rs` asserted that a commit from a fresh process must record *nothing*,
with the comment *"a future change that makes this line commit is a regression"*. The protection it
named is real; attributing it to the **per-process write list** was the error. What actually
protects a project vault from an `auto:` commit sweeping someone's index is the **naming scheme** —
`adoptable` stages only `<notes dir>/<ULID>.md`, which `FileStore` writes and nothing else
produces. The memory added no safety and one large cost: whether your work was recorded depended on
when the process happened to start. `App::load` had already abandoned the property at open (it
adopts, so the next commit after a relaunch records everything), so the invariant was already gone
in production and alive only in a test that used `App::new` and skipped that path. The test now
asserts the real property *directly*: a non-ULID file in the notes dir and a file outside it are
both left alone.

**Also fixed the blind spot that hid it.** `fm-app` had no `native-git` feature, so no test of a
*command* could select the phone's backend — the exact gap `vcs::force_native`'s own doc warns
about. It has one now, and `pixi run test-native-git` runs these tests through libgit2.

**Still open:** `sync.svelte.ts`'s `commitStep` treats `committed: false` with no conflicts as
success and pushes on, so a vault that records nothing can still report `synced`. Harmless once a
commit records everything it can find; wrong in principle, and the reason a silent failure here
looked like a working backup for weeks.

## Every Android IPC command is `(async)` — a blocking one freezes the screen (2026-08-20, `#track-m`)

**Decision.** Both commands in `mobile/src-tauri/src/lib.rs` are `#[tauri::command(async)]`, on the
*sync* functions. `ci/checks.sh` fails the build on a bare `#[tauri::command]` in that file.

**Why.** Three facts nobody had composed. Android never gets Tauri's async custom-protocol IPC
(`canUseCustomProtocol = osName !== 'android'`), so it falls back to `window.ipc.postMessage`;
that is an `@JavascriptInterface` method and wry runs the handler **inline**, and a JS→Java bridge
call is synchronous, so the page's JS thread is parked until Rust returns; and a plain
`#[tauri::command]` is `ExecutionContext::Blocking`. Net: **the UI could not paint for the duration
of any command.** That is why the phone presented as *freezing* rather than as slow, and why every
other cost on the platform — a full-corpus scan, an `ls-remote`, a full FTS rebuild — showed up as
the app locking up. The desktop never saw it: `fm-serve` is thread-per-connection.

**Consequence.** Deliberately `#[tauri::command(async)]` and **not** `async fn`: there is no
`.await` in either body, so no `MutexGuard` can be held across one, and the hazard cannot arise;
`async fn` would create a future in which a refactor could introduce it. Two follow-ons landed in
the same change, both of which the blocking bridge had been hiding: replies can now interleave, so
`NotePanel`'s note-loading effect gained the `cancelled` guard `ProposalReview` already had; and
the `pagehide` flush weakens from effectively-synchronous to fire-and-forget (recorded in
`known-issues.md`). Verified structurally, not by compiling — the mobile crate is
workspace-excluded, which is the same reason the setup-hook guard beside it is a grep.

## Android trusts its persisted index on open; the desktop rebuilds (2026-08-20, `#track-m` `#data`)

**Decision.** `FileStore::open_incremental` / `MultiStore::open_with(_, ColdStart)` /
`App::load_with(ColdStart)`. The Android shell passes `TrustIndex`; `fm-serve` and every test keep
`Rebuild`. Gated on `PRAGMA user_version == INDEX_SCHEMA`, written only **after** the reindex has
committed, falling back to a full rebuild whenever it does not match.

**Why.** `open` rebuilt the entire FTS index from every file, every time. Android kills
backgrounded apps constantly, so that was the cost of *every* relaunch — the app's slowest moment,
all day, next to an `index.sqlite` that was buying nothing. `mobile-design.md` has asked for this
since M0; what was missing was any way to know it is safe, which `known-issues.md` named as
*"cold-start tests that do not exist."*

**Consequence.** It is a **parameter, not a `cfg!`**, because it is a claim about the machine:
incremental reconciles on mtime, so a writer that rewrites a file while preserving its mtime
(`cp -p`, `rsync -a`, a restic restore) is invisible to it. A phone has none of those — no shell,
no restic, and libgit2 writes files fresh. The frontend knows that; the store does not, and a
`#[cfg(target_os)]` in the library would compile a wrong answer for every platform not yet listed
(the same reasoning that made `open_external` a trait). `crates/fm-core/tests/cold_start.rs`
asserts the mtime divergence **as a failing case on purpose**, so the gate's justification is
executable rather than remembered. The marker is a *completion* marker, not a shutdown one:
Android exits by `SIGKILL`, so there is nothing to hook on the way out. The ordering is the whole
guarantee — the marker is written after `reindex` has committed, so **marker present implies rows
durable**, and an interrupted rebuild rolls back without ever reaching it. Writing it inside the
transaction was considered and is strictly worse: it reintroduces the one failure it exists to
prevent (a marker for an index that is not complete).

## The Android attachment ceiling is 16 MB, and the number counts copies (2026-08-20, `#track-m` `#vault`)

> **PARTLY SUPERSEDED (2026-09-04)** — *a file is sliced, so its size stops being a memory limit*.
> The reasoning below is unchanged and still governs the **single-shot** path, which still refuses
> nothing and still carries every ordinary photo. What is superseded is the *consequence*: a file
> over the ceiling is no longer refused, it is chunked. The number now decides which path a file
> takes, not whether it is allowed.


**Decision.** `MAX_INGEST` drops from 48 MB to 16 MB.

**Why.** 48 MB was chosen as a size a WebView could serialise, which measures the wrong thing. A
file on the way in is copied roughly **ten** times — the `File`, the `readAsDataURL` result,
Tauri's `JSON.stringify`, the JS→Java marshal (UTF-16, so ~2.7×), wry's `get_string` and
`to_string_lossy`, `serde_json` into a `Value` and then into the argument, and the decoded
`Vec<u8>`. A 12 MP photo peaks near 150 MB; the old ceiling permitted a peak near 600 MB, which is
not a slow attach but `onRenderProcessGone` and the app vanishing with no message. 16 MB keeps
every photo a phone takes (a 48 MP JPEG is ~12 MB) at roughly a third of the peak.

**Consequence.** Chunking the *encode* was considered and rejected as a fix — it removes one copy
of the ten, all the others being on the transport. The real fix is chunked **ingest**
(`fm_ingest_chunk`/`fm_ingest_finish` over `BlobStore::put_file`, which already streams and hashes
in 64 KB chunks), which would bound the transient regardless of file size and lift the video
refusal. That is a transport change and is deliberately **not** smuggled in beside a bug fix; this
number buys the headroom to do it properly. Recorded as outstanding in `known-issues.md`.

## A conflict surface must be derived from git, not from markers in the text (2026-07-31, `#git` `#data`)

**Decision.** `conflicts` is answered from **`vcs::conflicted`** — git's unmerged paths, each with its
kind — unioned with the old body-marker scan, and every entry says whether there are markers to edit.
`resolve_conflict(vault, path, keep)` keeps a side for the kinds that have none, **and finishes the
merge when it was the last one**. Both git backends implement it, and a test asserts they agree
byte-for-byte (`fm-cli/tests/conflict_resolution_both_devices.rs`).

**Why.** A surface derived from a *symptom* misses every case without the symptom. Markers are the
symptom of exactly one of git's seven conflict codes; a delete/modify (`DU`/`UD`) has none, because
one side has no blob and the `.md` driver is never called. So the app warned about a conflict it could
not display, and told the user in every message to *"open each one, both versions are marked in the
text"* — impossible advice, with no other action offered, while `commit_all`'s refusal to commit
mid-merge froze the whole vault. Measured cost on the owner's laptop: **7 days, 95 notes never
committed**, from one invisible note (`sessions/2026-07-31-the-gray-screen-on-first-open.md` covers
the same day's Android work; this one is in `known-issues.md`).

**Consequence.** Finishing the merge is part of `resolve_conflict`, not the caller's job: `commit_all`
stages only paths the app remembers writing, so it can answer "nothing of ours changed" and return
*without* committing — leaving `MERGE_HEAD` standing, which is itself what makes it refuse. Resolving
the last conflict and staying frozen would have been the same bug wearing a different hat.
**`keep theirs`/`keep mine` is deliberately not offered for a marker conflict**: there, both sides'
text exists and picking one discards the other, so the editor is the honest tool.

## `commit_all` may lag, but it must never *silently skip* — an explicit catch-up exists (2026-07-31, `#data`)

**Decision.** `vcs::unrecorded(vault, notes_rel)` lists notes on disk that git does not have, a
toolbar chip counts them, and `record_unrecorded` stages exactly those and commits. The debounced
auto-commit is **unchanged** — still only the paths this process recorded writing.

**Why.** The precision was right and its scope was wrong. Staging only recorded paths is what keeps a
vault that is also a project repo from having its owner's carefully staged work swept into an `auto:`
commit every five seconds — but that record is **per-process memory**, so a note written before the
last restart could never be staged by it. Not "history lags", which is what `known-issues.md` claimed:
history was *permanently missing* those notes, and nothing surfaced it. 95 had accumulated.

**Consequence.** The catch-up is **explicit and human-initiated**, never on a timer — the same
reasoning as *"auto-push is explicit, never silent"*: it stages files the app does not remember
writing, which is a judgement a person should make. And the count is a **persistent chip**, not a
banner, for the reason the "unreadable notes" chip is: the condition lasts until someone acts, and its
entire failure mode was silence.

## A debounce needs a ceiling, and a duplicate is removed only after git has it (2026-07-31, `#data`)

**Two decisions from one incident**, in which ~142 copies of one message to the study assistant were
written on the phone inside a single minute and none of them reached git.

**1. The auto-commit debounce is capped.** `scheduleCommit` did `clearTimeout` then `setTimeout(5s)`,
so **every write pushed the deadline out**: under a burst it does not fire late, it never fires at all.
Then the process ended, the pending `setTimeout` died with the tab, and the in-memory write-record died
with the process — leaving those notes unstageable for good. The quiet period stays (a commit per
keystroke is what it exists to prevent), but a commit is never deferred past `COMMIT_MAX_WAIT_MS` after
the *first* pending write. Pinned by `ui/src/App.commitBurst.test.ts`, which against the uncapped
version reports **zero** commits across a 40-second burst.

Worth recording for the next reader: the agent's *own* replies were tracked throughout, because the
`reply` arm commits immediately when an author identity is supplied (Ruling 14). That asymmetry is why
every survivor was a user-side message — the diagnosis followed from noticing it.

**2. Pruning duplicates is refused until they are in history.** `duplicates` groups notes by **body**
(copies differ in `id` and `created`, so hashing the file would call every duplicate unique);
`prune_duplicates` keeps the **oldest** of each family and removes the rest. It **refuses outright**
while any copy is still outside git: deleting an untracked note is unrecoverable, while deleting a
tracked one is one `git checkout` away. So the order — record, then prune — is enforced by the code
rather than left to whoever presses the button, and the UI does not offer the action it would refuse.

**Consequence.** The content of a duplicated note is never lost, only its repetition: one copy always
stays, and the removals are themselves a commit. "Files-as-truth" survives a cleanup button, which is
the only basis on which this app should ever have one.

## The write-record is rebuilt from the filesystem at open — a refinement of "the auto-commit stages what we wrote" (2026-07-31, `#data`)

**Decision.** `App::load` seeds each vault's write-record from `vcs::unrecorded`, filtered to paths that
are ours **by construction**: a `<ULID>.md` inside that vault's own notes directory. The next ordinary
commit then records them. Once at open, never on a timer.

**Why.** The owner reasonably expected that writing a note commits it — *"I thought new note creation
and saving was committing"* — and the app agreed: eight call sites schedule the debounced commit and
every write path reaches one. What failed is subtler. `commit_all` stages `MultiStore::written(vault)`,
which is **per-process memory**, so a note written in a session Android later killed was not *lagging*,
it was permanently unstageable. Nothing surfaced it until the chip shipped, and by then **147 notes**
had accumulated on the phone, in a vault whose only copy of them was the device.

**Why not `add -A`.** That was rejected for a good reason and the reason still holds: a vault may be a
repo the user commits to themselves, and staging everything every five seconds would make this app a
second author of their index. The naming scheme is what makes seeding safe where `add -A` is not —
`FileStore` writes `<ULID>.md` and nothing hand-written looks like that, so adopting exactly those
cannot touch someone's source file. Conflicted paths are already excluded by `unrecorded` itself.

**Why only at open.** The other deliberate property — a note being hand-edited is not swept
mid-sentence — is a *session* property. Open is precisely where the memory was lost, so rebuilding
there restores completeness without touching what happens during the session. And it is announced
(stderr, plus the count in the UI): a startup that quietly adopts files is one nobody can account for
later.

**Consequence.** "Files are never at risk; commits can lag" becomes true as stated — before this, they
could be skipped forever. The chip and panel stay, because they cover what seeding cannot: a vault with
no git, notes not in our naming scheme, and the *deletions* whose recording is also outstanding.
Pinned by `fm-app/tests/unrecorded_after_restart.rs`, which drives the real `App::load`.

## A count is a symptom; the kind is the diagnosis — and the device must be able to state it (2026-07-31, `#data`)

**Decision.** `vcs::unrecorded` returns `UnrecordedNote { path, kind }` with
`kind ∈ {New, Modified, Deleted}`, both backends, held byte-identical by
`fm-cli/tests/conflict_resolution_both_devices.rs`. `dto::Unrecorded` carries per-kind counts plus a
**bounded** 50-row sample (id, title-or-first-line, bytes, mtime), and the toolbar chip opens a panel
that leads with the split instead of committing on one click.

**Why.** The chip said *"146 not in history"* on the owner's phone and **no one could act on it**. That
number could mean 146 notes existing nowhere else — app-private storage is erased by an uninstall and
`blobs/` never travels with a push — or 146 notes something was needlessly rewriting. Opposite
urgencies; opposite responses. The device knew which (it had just run `git status`) and the surface
threw the status code away, mapping porcelain lines to bare paths.

**And it must be the device that says it**, because there is no other channel: on that phone Rust's
stdout is not routed to logcat, the WebView forwards no `console.*`, and MIUI suppresses our own tag.
`run-as` cannot read a release build's private storage. Anything a user must be able to report has to be
rendered on screen — the same rule that put `ca_bundle` in Settings and the skipped notes in a panel.

**Consequence.** The one-click "record everything" became a *second* step behind the panel: recording is
still right, but doing it before knowing which story you are in destroys the evidence for the other one
(a `modified` pile committed is a rewriting bug you can no longer see). Per-kind counts are exact; the
detail list is capped at `dto::UNRECORDED_DETAIL` because a vault can hold thousands and the counts
already answer "how bad" — the sample answers "what happened". A **deleted** note is now labelled as
such rather than silently counted as "not in history", which the old wording quietly mis-stated.

**Reproducible without the device.** `fm-app/tests/unrecorded_after_restart.rs` recreates the whole
failure by dropping and re-opening the `App` — which is exactly what a process kill does to
`commit_all`'s in-memory write-record — and asserts that `commit` alone still records nothing (that
precision protects a project vault and is *not* the bug), while `unrecorded` finds them with the right
kinds and `record_unrecorded` commits them. A class of bug that needed a phone to observe now needs
70 ms of `cargo test`.

## A vault is labelled by its remote, and identified by its local name (2026-07-31, `#vault` `#ui`)

**Decision.** `list_vaults` carries a `label`: the repository behind the vault's remote
(`…/formicarium-vault.git` → `formicarium-vault`), `null` when there is no remote. The UI shows the
label — badge, vault filter, tooltips, and the colour hue — while **every key stays the local `name`**.
`ui/src/lib/vaultLabels.svelte.ts` resolves one from the other, so the twenty-odd components that
already pass a vault name did not each have to learn about labels.

**Why.** The same repository cloned on two devices can carry two different local names — the owner's
laptop said `vault` where the phone said `notes` — so one *audience* looked like two different vaults
depending on which screen you were on. The remote is the thing both devices agree about.

**Why not simply rename the vault.** A vault's name is not a label: it is the write routing key
(`MultiStore::route`), the argument seventeen dispatch arms take, and the key behind the persisted view
preferences (`hiddenVaults`, `fm-board-order`, `fm-card-order`, keyed per vault *and* per group-by).
Renaming to match the remote would silently reset all of those and break any command in flight. So the
split is identity versus display — the same shape as `authorKey`/label for contributors, decided the
same day, for the same reason.

**Applied to three surfaces it had missed, 2026-09-08 — reported from the phone, as the exact symptom
this entry was written to prevent.** The Settings vault list, the Backup summary sentence and the
quiet-vault chip all still interpolated the raw `name`, so Settings listed `vault` and `notes` while
the Backup panel — one tap away, and the only surface that had called `labelFor` — headed the very
same two vaults `vault` and `formicarium-vault`. The app looked like it had four vaults, or two and a
bug. **Neither screen was wrong on its own, which is what made it hard to see**, and the owner had to
find it by reading both and disbelieving them.

Worth stating plainly, because it is the second time this week: *a ruling that names its own symptom
and is then only partly applied reads, from the outside, exactly like no ruling at all.* The mock had
even modelled it correctly the whole time — `lab` carries the label `lab-notes` and `personal` carries
none, so both paths were on screen under `pnpm dev` — and the chip's tests had been passing **because**
the chip ignored labels. `quietLabel`/`quietTitle` now take the resolver as an argument rather than
importing it, so that module stays a pure function of its inputs and its tests still need no store.

**Consequence.** The hue follows the *label*, so one repository is one colour on every device; keying
it on the folder name gave one audience two colours. **Two vaults cloned from the same remote fall
back to their local names — both of them**, not just the second: two identical labels make the vault
filter ambiguous, and hiding notes from the wrong audience is worse than showing a folder name.
`remote_label` reads local `git config` only — no `ls-remote`, no network — so the vault list does not
inherit the slowness that keeps `backup_status` off the heartbeat.

## A vault can be forgotten, and forgetting never deletes (2026-07-31, `#vault`)

**Decision.** `forget_vault` unregisters a vault: dropped from the live `MultiStore` and from
`vaults.json`. **It never touches a file.** The answer reports how many notes were left behind and
where, and the UI (Backup panel, two-step) repeats it — *"Removed 'x'. Its 209 notes are still on disk
at …"* / *"it was empty. Nothing was deleted."* Removing the last vault is allowed and lands on the
first-run screen, which is already a state the UI knows (`list_vaults` → `[]`).

**Why.** Three commands brought a vault into being — `create_vault`, `clone_vault`, `restore_vault` —
and **none took one away.** On a desktop that is a papercut you can fix by editing `vaults.json`; for
this owner it is permanent, because the product is the only way in. And the phone *manufactures* the
problem: `configure_paths` auto-creates an empty default vault on first launch, so the owner had one
they never asked for, could not use, and could not remove (2026-07-31: *"creates only confusion"*).

**Consequence.** "Forget" and "destroy" stay different verbs, and only the reversible one is built: a
vault dropped from the list is re-added by pointing at the same directory, so the worst case of a
mistaken click is retyping a path. That is what makes it safe behind one button — and why the message
must state the count, since "removed from the list" would otherwise read as "erased". The list is
saved to disk **before** the vault leaves memory: if the save fails nothing has changed, which is the
recoverable order. There is deliberately no "and delete the files" option; when someone wants that,
it is a separate decision with a separate confirmation, not a checkbox next to this one.

## A contributor is an email, everywhere — the name is only a label (2026-07-31, `#git` `#ui`)

**Decision.** `activity.svelte.ts` exports `authorKey(e)` = the lowercased email, falling back to the
name — and **both** the contributor chips and the note filter use it. `contributors()` returns
`{key, label}` so the key travels with the label and the two cannot drift.

**Why.** They had drifted. The chips already deduplicated by email, with a comment naming the exact
case (*"a vault signed with a username on one machine and the same person's full name on another is
still one human"*), while `shown()` hid notes by comparing the author **name**. So one chip represented one
person and hid only the spelling it happened to be labelled with. Measured in the owner's own vault:
**534 commits under a username, 77 under the same person's full name, 3 under a lowercased variant
in another vault — one email.** Clicking the chip left 77 notes on screen while reporting "hidden".

**Which name counts, since three things are called one:** the **email** is the identity — it is what
git carries as the stable half, what GitHub attributes commits by, and now what this app groups by. A
git `user.name` is a display string. A **GitHub username** counts for almost nothing here: it is the
username field beside a PAT, and with a token that field is nearly free-form. And a **vault name** is a
local label for an audience, unrelated to any of them. Recording this because the owner reasonably
assumed the GitHub username was the load-bearing one; it is the least.

**Consequence.** `hiddenAuthors` in `localStorage` now holds keys rather than names. A stale entry from
before this change simply matches nothing — the filter fails *open* (the note is shown), which is the
right direction for a preference: a filter that silently hides notes after an upgrade would look like
data loss. The agent identities (`<model>@fm-agents.local`) are separate emails and stay separate
contributors, which is the intent — a model's commits are labelled as the model's.

## A backend that cannot finish a merge must refuse to commit — and "I edited it" is a resolution (2026-07-31, `#git`)

**Decision.** Three rulings, all forced by one differential test:

1. **Neither backend may commit while anything is unmerged.** The subprocess one always refused; the
   libgit2 one had **no such guard**, and `index.add_path` clears a path's conflict stages — so a
   debounced auto-commit five seconds after a conflicting pull committed the note **with its
   `<<<<<<<` markers as content**, dropped the merge's second parent, and pushed it.
2. **A merge in flight is committed as a merge**: `MERGE_HEAD` as the second parent, then
   `cleanup_state()`. A single-parent commit silently drops the incoming history; a `MERGE_HEAD` left
   standing freezes the vault for good (`push_squashed`, `merge_proposal_branch` and the next `pull`
   all refuse over an unfinished merge) — unrecoverable on a phone, which has no shell.
   The subprocess side had the mirror-image bug: it committed with `--only <paths>`, which git refuses
   outright mid-merge (*"cannot do a partial commit during a merge"*), so the ordinary resolution
   failed on every attempt and the vault stayed frozen behind a scary banner.
3. **`Keep::Edited` exists**, because **editing a note resolves nothing as far as git is concerned** —
   verified against real git: clean text over a `UU` path leaves all three index stages, so the path
   stays unmerged. The app's own instruction ("open each one, both versions are marked in the text")
   therefore settled nothing, and since the conflict *list* was derived from markers in the body, the
   note left the UI the moment the markers were tidied — taking the only sign of trouble with it while
   the vault silently stopped recording history. Git's verb for "I reconciled this" is `add`, and
   nothing called it. It is **refused while markers remain** (`merge::has_conflict_markers`, now the
   single definition shared with the conflict list): staging a marked-up file is what git reads as
   "resolved", and it would publish `<<<<<<<` to every collaborator.

**Why it was invisible.** `vcs.rs`'s own comment says the `#[cfg]` inside each routed function means
"the two backends cannot drift in shape without the compiler saying so" — true, and only for a build
that *has* the feature. `pixi run ci` does not build `native-git`, so shape drift reaches only the
phone. Behaviour drift is not checked by any compiler at all: both backends had `commit_all`, and both
were wrong, in opposite directions, on the same path.

**Consequence.** `ci/checks.sh` now greps that every `route!`d name exists in **both** backends (shape,
cheap, in the default gate), and the parity assertions live in `pixi run test-native-git` — where they
were written to fail first and did, catching both bugs on the first run. The check's failure message
says explicitly that a green grep is not evidence the backends agree.

## Startup is a contract: the store is reachable first, and no state renders a blank screen (2026-07-31, `#track-m` `#ui`)

**Decision.** Three rules, all now enforced rather than intended.

1. **The Android `setup` hook may not fail upward, and nothing slow may precede the store.** Tauri
   builds the webview *before* the hook runs (`tauri/src/app.rs:2521`), and the event loop that
   delivers an IPC reply does not start until the hook returns — so the hook's duration is a window
   in which the page is asking questions nobody can answer. Startup therefore lives in one
   `boot()`: `catch_unwind` around `App::load`, `manage` the instant the store exists, then the CA
   bundle and the model on a spawned thread. A failure is recorded in `BOOT` and **every command
   answers with it**, because a `?` there panics the shell's thread and leaves a live webview with
   no backend — which is not a crash anyone can report, it is a screen that never paints.
2. **`BOOT` is a `Mutex`, never a `OnceLock`**, and `boot()` is re-attempted by the first command
   that finds no store. The first version froze the first failure for the life of the process, so
   the UI's "Try again" re-asked, got the same stale sentence, and could not possibly help. A retry
   that cannot retry is worse than no button.
3. **No state of the render gate may paint an empty document.** `vaults === null` renders
   `Starting` — silent for 700 ms so a fast launch never flashes it, then a status, then at 8 s an
   escalation with the backend's verbatim reason and a retry. And a *refusal* is not an empty vault
   list: `.catch(() => (vaults = []))` offered to create a first vault to someone who has ten.

**Why.** The owner's phone opened to a gray screen on the first launch and worked on the second
(`sessions/2026-07-31-the-gray-screen-on-first-open.md`). Every ingredient was ordinary; the
combination was unreportable, because a phone has no console, no stdout in logcat, and (on MIUI) not
even our own tag. **The identical first-paint contention bug had been fixed on the desktop a week
earlier** — "the study agent's model warm-up is deferred", below — and nobody asked whether the phone
had the same shape. It did, and worse: there the *vault open* was in front of the first frame too.

**Consequence.** The boot poll is **time-driven, not rejection-driven** — the actual symptom was an
invoke that never settled, which no `.catch` can observe. `ci/checks.sh` greps the shell structurally
(no `?`/`unwrap`/`expect`/`panic` in the hook or in `boot`; the store managed before
`install_ca_bundle`/`agent::start`), because the mobile crate cannot compile in `pixi run ci` at all
and a rule nothing checks is a rule that lasts one refactor. `mock.ts` grew a fault surface
(`reject`/`hang`/`delay`) so the UI's refusal, stall and never-answers paths are testable — the
general form of a lesson `known-issues.md` had already recorded about one hand-written guard.

## An emulator may be installed to and force-stopped; the owner's phone may only be looked at (2026-07-31, `#track-m`)

**Decision.** Automated device tests target an emulator, and `ci/android-smoke.sh` **refuses to run
against any serial that is not `emulator-*`** — before it checks anything else, including whether its
own tools are present. The real-phone pass stays observation only (screencap via `exec-out`, which
writes nothing on the device; `dumpsys`; `ps`), with the owner doing the installing and the tapping.

**Why.** The test has to install, force-stop, `am kill` and uninstall to be worth anything, and the
device this project is developed against is the owner's personal phone. A guard that is a sentence in
a header is not a guard; this one is the first executable line and was verified to refuse with the
phone attached.

**Consequence.** `pixi run android-smoke` is opt-in, in the `android` feature, and deliberately **not**
part of `pixi run ci` — which must stay green for a contributor with no NDK. What it asserts is shaped
by the bug it exists to catch: **two consecutive cold launches must both paint**, because "fine the
second time" was the whole signature and a one-launch test would have gone green throughout. "Painted"
is `vips deviate` over a screenshot (a flat surface is ~0, a real screen is tens), with every measured
number written to `stats.txt` so the threshold stays grounded rather than guessed.

## The TLS exception: a self-signed **leaf**, share-only, and only for the microphone (2026-07-26, `#vault` `#sync`)

**Decision.** `fm-serve` links `rustls` + `rcgen` behind a `tls` feature (default on;
`--no-default-features` still builds the std-only server, now CI-enforced). The shared listener
gets its own port and a **self-signed leaf certificate**, not a local CA.

**Why, in the shape of the libgit2 exception.** Sharing over plain HTTP was built first and
carries the whole feature — touch editing, photo and video capture (`<input capture>` is a
picker, not `getUserMedia`), ingest, blobs, whiteboard. The exception buys exactly one thing:
`getUserMedia`, which browsers disable outside a secure context and which no amount of
server-side care can grant over http. **You cannot hand-roll TLS**, which is the one place
`decisions.md`'s "40 lines against a dependency" stance does not reach. Terminating TLS in a
subprocess would have been more consistent with "invoke, don't link" and is the route to revisit
if a terminator ever resolves on all four `pixi.toml` platforms.

**Narrowly limited to:** the `tls` feature, `fm-serve` alone (`fm-core`/`fm-app`/`fm-query` link
nothing), and **`ring` backends pinned on both crates** — the default is `aws-lc-rs`, whose
licence includes `OpenSSL`, which `deny.toml` does not allow and which also wants cmake.
`ci/checks.sh` now fails if `aws-lc-rs` appears in `Cargo.lock`, because a transitive
default-feature flip is silent.

**A leaf, not a CA — the security call.** A private CA is the *convenient* design: re-mint the
leaf when DHCP moves and no paired device notices. Rejected. A root CA in a tablet's trust store
signs **any name for that device's entire browsing life**; its key would sit in a config
directory, be swept into whatever backs that directory up (including this app's own restic
backup), and outlive uninstalling formicaria. Leaked leaf key → one notebook server. The DHCP
problem it solved is answered instead by putting `<hostname>.local` in the SANs and advertising
*that*: mDNS is resolved by both iOS and Android, already advertised by avahi/Bonjour, and a name
survives a lease change where a bookmarked IP does not. The certificate is re-minted whenever the
machine's name set changes.

**No click-through path is documented, anywhere.** The 2026-07-19 Android TLS entry rejects
`certificate_check → CertificateOk` emphatically, for skipping hostname verification; telling a
user to dismiss an interstitial is the same act with the user as the actor, and under this design
a warning only ever appears on a certificate that is *not* the expected one. Instead the
certificate is a **file to carry across**, and its SHA-256 fingerprint is printed on the desktop
and shown in Settings so it can be compared against what the device displays before installing.
That comparison *is* the verification, performed by a human because a home network offers no
other root of trust. A device that will not trust it simply has no in-app microphone — a
supported state, with the message already written in `record.ts`. **Also rejected: HSTS** (it
would pin the address to https permanently; when DHCP hands that IP to a printer the user has an
error they cannot clear) and **fetching the certificate over the connection it authenticates**
(trust-on-first-use with no verification step at all).

**Consequence for the cookie.** `Secure` + a week's `Max-Age` on TLS; a **session** cookie on the
plain-HTTP fallback, because a long-lived bearer token in clear on a LAN is precisely the
`userpass_plaintext`-over-an-unauthenticated-connection shape the Android entry refuses.

## A caller is a member of some audiences, not all of them: `Scope` (2026-07-26, `#vault` `#seams`)

**Decision.** `dispatch_as(.., &Scope)` narrows a caller to named vaults; `dispatch` keeps its old
signature and means `Scope::All`, so `fm-cli`, the phone, the study agent and the desktop's own
browser are untouched. Enforcement is `fm_core::Scoped`, a **view** of `MultiStore` over a subset
of its vaults, reached through `Vaults::store(scope)`.

**Why.** *Vaults are audiences* has always been the model, but every caller was the person at the
keyboard, so "sees everything" was correct by construction. It stops being correct the instant a
device can pair. Three specific ways the boundary leaked, each fixed at its own choke point:

- **Reads** federate through `Store::candidates`, so board/agenda/search/recent/activity/`.view`
  are all scoped by narrowing the set the read *runs over*. Filtering results afterwards would not
  be a filter but a redaction — the other audience's notes would already have been read.
- **`find_blob` searched every vault**, and its doc comment *justified* that: "whichever vault
  answers, the bytes hash to the reference." True, and exactly the hazard — content-addressing
  deduplicates, so a hash learned legitimately from a shared vault resolves against a private one.
  `GET /api/blob` is not a command, so it takes the scope explicitly.
- **`Vaults::config("")` resolved the default to `list[0]` *before* the store saw it**, so a
  capture from a scoped caller would have been filed into an audience it cannot read. The default
  is now the caller's first *reachable* vault. Found by a test, not by reading.

**Consequence.** An unknown vault and an out-of-scope vault give the identical error, so a caller
cannot probe for names it was not given; `list_vaults` is filtered, because the switcher and the
copy-to menu are built from it and a vault's *name* discloses. An empty scope grants nothing —
that inversion is how a device whose vaults were revoked would silently gain all of them.
`Scope::All` is a variant, not "a list of every vault", so a vault created later is included
rather than silently denied to the machine's own user.

**Where the halves live:** mechanism in `fm-core` (it needs the vault list, and a check that can
be forgotten is not a check); policy in `fm-app::scope`. Deliberately not a permission system —
no roles, no verbs. Location is the permission, exactly as on disk. Tests:
`fm-core/tests/scoped.rs` (mechanism), `fm-app/tests/scoped_dispatch.rs` (wiring — the half that
rots, since a new read path reaching `Vaults::all` would leave every mechanism test passing).

**Nine arms were still handing back the whole vault list, closed 2026-09-08 — and the file above
predicted exactly this.** `list_vaults` was filtered from the start, because *a vault's name
discloses*. Every **other** arm that returned the same shape was not: `backup_status` (name, remote
URL, committer name and email, unpushed count, conflicted paths), `config` (the list plus each
vault's restic repo — a path, often a host), the two settings writes that hand back the refreshed
list, and the four vault-lifecycle commands. A device paired to one audience could read where every
other audience is hosted and who signs it.

**The cause is worth more than the fix.** The filter was *two lines duplicated per site* rather than
a function, so each site was written correctly in isolation and nothing checked the sum — which is
`CLAUDE.md`'s four questions, failing in the one direction they cannot catch: nobody was *adding*
anything. There is now a single `scoped_infos(&g, scope)` and no arm builds the list itself.

**Found by accident**, chasing an unrelated phone question, by noticing that `backup_status(app)`
takes no scope while its sibling one line below, `backup_latest(app, scope, ..)`, does. The sweep
that followed found the other eight. `scoped_dispatch.rs` now asserts them together, because they
failed together and for one reason.

## The poll answers a comparison, not a report: `ping` carries a generation (2026-07-26, `#seams` `#sync`)

**Decision.** `ping` takes the `since` the client last saw and answers `changed = generation >
since`, where `generation` is a monotonic per-process counter on `App`. It is bumped by any
command not on `dispatch::READ_ONLY`, and by a reindex that finds drift.

**Why — two separate defects, one cause: the answer was a *report*, and a report can only be
made once.**

- **Drift is consumed by whoever asks first.** `changed` was "did *this* incremental reindex find
  moved mtimes?" — but that same reindex writes the fresh mtimes back. The first client to ask
  got the news; every other client was told "nothing changed" indefinitely. `App.svelte` had
  carried a comment describing this since it was written, mitigated by an unconditional refresh
  on `visibilitychange` — a mitigation that **does not exist on a tablet**, which is held in the
  hand and never backgrounded.
- **A write through `dispatch` produced no drift at all** — the larger half, and it was not
  known. `FileStore::put` indexes the file it just wrote, mtime included (`file.rs::index_object`),
  so afterwards the index and the disk agree and an incremental reindex finds *nothing*. The poll
  therefore only ever saw **out-of-band** edits — a `git pull`, the merge driver, Vim. That was
  sufficient for exactly as long as there was one client, and it is why a second client would
  never have seen the first one's notes at all, not merely seen them late.

**Consequence.** N clients each hold their own cursor, so the server needs no identity, no
per-client eviction, and no definition of "gone" — and a client that misses a beat catches up on
the next one. `READ_ONLY` is an **allowlist**, so the failure mode of forgetting to classify a new
command is "one redundant re-query" and never "silently invisible to every other screen"; `ping`
is on it and bumps from inside its own arm instead, or the poll would report a change on every
beat forever. A `since` of 0 (a client with no cursor) is told `changed` deliberately: its initial
queries are not atomic with its first beat, so assuming it is behind is the recoverable error.
Refused writes do **not** bump — a stale `base` on a debounced editor would otherwise fan a
re-query out to every client on every keystroke.

**Consequence for the reader:** `ping` is no longer "is this vault dirty?" — it is "have I fallen
behind?", and those differ the moment there are two clients. Tests: `tests/poll_generation.rs`.

## The in-process sync path: the app merges, because libgit2 cannot (2026-07-19, `#sync` `#git`)

**Decision.** `git_native` now covers the whole collaboration loop — `clone`, `commit_all`,
`pull`, `push`, `unpushed`, `conflicts` — so a phone can share a vault with a desktop.

**Extended 2026-07-24 to the whole proposal lifecycle** (create / review / revise / accept /
reject), which had been left calling `crate::git` directly and therefore did not work on a phone
at all. The same ruling applies for the same reason — `merged_text()` is now the shared decision
both `pull` and `merge_proposal_branch` route through, so one vault cannot hold two merge
semantics. Two accept-path specifics are recorded because they are counter-intuitive and both
cost notes if reversed:

- **Move `HEAD` last.** Decide in memory → write working tree and index → commit. Committing
  first and checking out second leaves an *undetectable* half-state on a kill, which the next
  auto-commit silently converts into a revert of the accepted proposal. A detectable half-state
  (git's own `MERGE_HEAD`) is strictly better than a silent one.
- **Never `checkout_head(force)`; scope the checkout to the merged paths.** Unscoped, it reverts
  every uncommitted edit in the vault — including notes the proposal never mentions.

See `sessions/2026-07-24-proposals-on-the-phone.md`.

**The load-bearing part is the merge.** libgit2 contains no process spawn, so it can never
invoke the `.md` driver that makes two people editing one note a non-event rather than a
conflict on the `updated:` line the app rewrites on every save. Porting `pull` naively would
have silently disabled that: a collaborator running `git pull` in a terminal would still get the
structural merge while the app quietly did a worse one — **two merge semantics in one vault, and
ours the wrong one.** That was the single strongest argument in the original `git2` rejection.

So the app resolves conflicted paths **itself**, by calling `merge::merge_texts` — the *same*
engine `fm merge-md` calls. One engine, two call sites, which is what makes them unable to
diverge. This is why `merge_texts` had to be extracted from its path-shaped wrapper first: none
of this was safe until the engine could be called with three strings.

**Proven, not asserted.** `crates/fm-cli/tests/git_native_merge.rs` runs the same divergence
through both backends and compares: a concurrent edit to different lines of one note comes back
**byte-identical**, cleanly merged, on both. A genuine disagreement conflicts on both, with
markers **in the body** so the note still parses, and the path named rather than swallowed.

**That test lives in `fm-cli`, not `fm-core`, and the reason is a trap worth remembering:**
`ensure_repo` points the driver at the `fm` binary beside the running one, and only `fm-cli`
builds one. Run from `fm-core`'s harness the subprocess side silently falls back to git's plain
text merge and conflicts — so the comparison would grade two broken things against each other.

**Consequences and the things deliberately left out:**
- `repo.merge`, not `merge_trees`. The latter returns a standalone in-memory index that cannot
  be written or `add_path`'d into, so a conflicted pull would be invisible to every later call.
  `repo.merge` leaves a repo-backed index and files on disk, which is what real `git merge` does.
- **The squash is not ported.** `push_squashed` collapses history, guarded by an ancestry check
  and a rollback that verifies the remote ref actually moved. Reimplementing that on a backend
  that has never run against a real remote is exactly the half-shipping this project forbids on
  the path that must never corrupt. **A phone pushes what it has.**
- **Credentials are a PAT from the environment** (`FM_GIT_TOKEN`), and the callback **fails after
  one attempt** — libgit2 retries while it keeps receiving credentials, so a bad token is a hang
  rather than an error. The Keystore-backed source is the shell's job, the same way it supplies
  `FM_CONFIG_DIR`.
- Still unported: `activity` (git log) and `remote_moved`. Neither is on the corruption path;
  both degrade to "unknown" rather than to a wrong answer.

**Reversal condition:** the differential test cannot stay green → stop, because that is the
signal that a phone and a desktop have started disagreeing about what a merged note is.

## One shell, two arrangements — layout adapts by space, never by platform (2026-07-19, `#ui`)

> **SUPERSEDED in part, 2026-08-31** (*one view at a time is the default, `auto` is gone*). This
> entry's **"hard stop at two" survives and is what finally shipped**; everything below about
> *space* deciding the arrangement, and about `auto` being the default, is reversed. `auto` turned
> out to cost a duplicated CSS rule for every narrow fact — and to sync nothing, the preference
> being per-browser. Read that entry before touching `Layout`.

**Decision.** The UI has **exactly two named layouts**, `tiled` and `single`, chosen by a
`layout: 'auto' | 'tiled' | 'single'` preference on the workspace. `auto` is the default and
follows available space. **Both are reachable on every platform** — `single` on a desktop,
`tiled` on a tablet.

**Why not a mobile frontend.** Putting the app on a phone made the tiled grid untenable: today's
answer stacks panes vertically, so you scroll past whole views to reach the next. The tempting
fix is a phone-specific UI. Two decades of cross-platform work says that is the expensive
mistake — `m.example.com`, a separate mobile app team, `Platform.select()` through a codebase:
all converge on content drift and two things to change per feature. What survived is **one
content layer inside an adaptive shell**, and **branching on space and input capability, never
on platform**, because platforms multiply forever while space is a continuum with two or three
thresholds testable at any window width.

This codebase already held that principle and under-implemented it: `App.svelte` argues pointer
beats width because *"a tablet is wide and still has no mouse"*, and explicitly rejected "a
phone-only component tree". This finishes the thought rather than reversing it.

**Consequences:**
- **No viewport-tracking TypeScript.** The whole mechanism is `data-layout` on the app root plus
  CSS; the only new state is a preference string, exactly like `fm-theme`. `single` hides
  non-active panes with `display: none` — every pane stays **mounted and fetched**, so switching
  is instant and the feed layer is untouched.
- **Desktop opt-in is the test, not a courtesy.** A narrow layout only a phone could run is a
  second frontend wearing a setting, and nothing would exercise it during ordinary desktop work.
  If `single` ever stops working in a desktop browser, the design has failed.
- **Hard stop at two.** A general "customisable frontend" is unbounded and lands on the plugin
  API `plan.md` already rejects. Two arrangements are bounded and both are tested.
- **The one setting Settings is allowed to own.** Everything else there is a read-only mirror
  (`vaults::save` is append-only). Layout is a *view* preference like the theme: this browser,
  no vault, nothing on disk.
- **Renderers moved to container queries.** They were querying the *window* while living in a
  pane sized `viewport ÷ cols` — a `colSpan:1` pane on a wide monitor got desktop-width board
  columns it could not fit. `Pane` is now `container-type: inline-size`; `Board`/`Card`'s width
  rules are `@container`, while `pointer: coarse` stays a *media* query because it is a
  capability, not a size. **Container queries are why this advice differs from 2010's** — a
  component can now ask its own box, which is what makes one renderer correct at any width.
- **Safe areas are paid for.** `viewport-fit=cover` plus `env(safe-area-inset-*)` on the shell,
  and `100dvh` rather than `100vh`. On a real phone the toolbar was painting under the status
  bar with the clock on top of the search field — invisible on the emulator.

**Deliberately not done:** swipe between panes. `Board` already uses `scroll-snap-type: x
mandatory` for its columns on narrow screens, so a horizontal pane swipe would compete with a
horizontal column swipe and make both feel broken. If it is ever wanted it must be an edge
gesture, judged on a device. And **no lazy per-pane fetching** — `MAX_PANES` is 8 and feeds
dedupe by `feedKey`, so it would trade instant switching for a loading flash.

**Reversal condition:** a third arrangement is genuinely needed (a tablet rail, say) → that is
the moment to check whether this has become the customisation system it refuses to be, not to
add a fourth.

## The libgit2 exception, and the discovery that `cargo deny` cannot enforce it (2026-07-19, `#git` `#toolchain`)

> **WIDENED (`#git`)** by *"libgit2 ships on Windows too"* (2026-08-28, above): the exception is no
> longer "mobile only" but **"any device with no git binary"**, which is the property this entry's
> reasoning actually rests on. Everything below still holds — the licence analysis, the vendoring,
> the conditions — only the boundary moved. Read this entry for *why the exception is safe*; read the
> 2026-08-28 one for *why Windows was always inside it*.
>
> **Reversal chain (`#git`):** narrows *"`git2` is rejected"* (2026-07-18, below) — which still
> holds for any machine that **has** a git binary; `vcs.rs` chooses at runtime and the binary wins.
> Later realized in code: the `native-git` backend + the `vcs` router now carry the full history
> *and* proposal lifecycle on the phone (`sessions/2026-07-24-proposals-on-the-phone.md`).

**Decision — one named exception, vendored, mobile only.** `fm-core` gains an optional
`native-git` feature (**off by default**) pulling `git2` with `vendored-libgit2`. The desktop
keeps shelling out to `git`; *"git is a capability, not a dependency"* still holds there, and
the `.md` merge driver keeps working for a collaborator's terminal `git pull`. **Why an
exception at all:** a phone has no `git` binary — Android ships none, iOS forbids executing
one. **Why it is safe:** libgit2 is GPL-2.0-only *with its own linking exception*, read
verbatim from the vendored `COPYING` at the pinned version; linking is precisely what that
exception permits. What survives is the **notice** obligation and the duty to publish any
modification to libgit2 — we make none.

**`git2 >= 0.21` is a floor, not a preference.** 0.20.4 — the version GitSync ships — carries
**RUSTSEC-2026-0183** and **-0184**, both `unsound`. The first is squarely on our path:
`Remote::list()` passes a null pointer to `slice::from_raw_parts` when a remote advertises no
refs, which is exactly the empty-remote case a first push meets. The advisories gate caught it
on the first resolve.

**The discovery, and it is the part that matters: `cargo deny` cannot enforce this, and never
will.** `libgit2-sys` declares `license = "MIT OR Apache-2.0"` — true of its Rust wrapper,
silent about the ~230k lines of GPL C it vendors — and cargo-deny treats a valid `license`
field as authoritative. `[[licenses.clarify]]` is **silently inert** in that case regardless of
the path or hash it is given (tested against cargo-deny 0.20.2; a paired
`[[licenses.exceptions]]` reports `license-exception-not-encountered`). Both were written,
tested, found ineffective, and **removed rather than left in place looking effective** — dead
config that appears to enforce something is worse than none.

This *vindicates* the 2026-07-18 `git2` rejection's sharpest line — that the gate "clears on a
metadata technicality" — and goes further: it is not fixable within `deny.toml`.

**Consequence — the enforcement moved to where it can actually run.** `ci/third-party.sh`
carries a licence **override** so the shipped notice states libgit2's real licence, and
`ci/checks.sh` **fails if `libgit2-sys` enters `Cargo.lock` without that override**, so the
dependency cannot arrive without the notice that legally must travel with it. Keyed on the
lockfile, so it fires for an optional dependency too. Both were verified by breaking them.
`deny.toml` now carries the finding in full, pointing the next reader at the real gate rather
than implying it does a job it cannot do.

**Reversal condition:** cargo-deny gains the ability to override a declared `license` field →
move the assertion back and delete the `checks.sh` entry. A maintained permissive pure-Rust
git with working push appears → drop libgit2 entirely.

## The owner's five Track M rulings (2026-07-19, `#track-m`)

Taken by the owner after the drift review, and binding. They close the questions the review left
open; the receipts for each are in `sessions/2026-07-19-mobile-drift-review.md`.

1. **libgit2 is the git backend — and the *intended* end state is one git dependency on both
   platforms, desktop included.** *Why:* it solves the problem whole, and two shipping apps
   already vindicate it (PuppyGit via JNI; GitSync, which migrated *off* JGit onto Rust `git2`
   and ships Android **and** iOS from one Rust core). *Consequence:* `deny.toml` gains **one named
   exception** for libgit2, citing its GPL-2.0 **linking exception**; the linked-vs-invoked ratio
   itself is **not** rewritten (it still correctly protects a future `.deb` shipping `pdftotext`),
   and `ci/third-party.sh` must emit the notice that exception requires. This **reverses** the
   2026-07-18 `git2` rejection on its licence ground.
   > **Sequencing caveat, recorded so it is not lost.** The desktop half is the *destination*, not
   > the next commit, and two findings gate it. (a) **`git merge-file` is the permanent oracle** —
   > it is the only grader the pure-Rust body engine will ever have, so it must survive **in the
   > test harness** even after production stops shelling out; "one git dep" is satisfied by one
   > dep *in the shipped binary*, not by deleting the oracle. (b) **libgit2 cannot invoke external
   > merge drivers**, so an app-side pull must call `merge_files` itself at each conflicted path —
   > the driver stays installed for collaborators running terminal `git pull`, and both routes
   > call the same engine, which is what makes them unable to diverge. **Build the engine and the
   > differential harness first (steps 1–2); swap the backend after.** Swapping first would
   > silently disable the frontmatter merge on the one path that must never corrupt.
2. **iOS eventually; Android now.** *Why:* it costs nothing today and permanently forecloses the
   ship-a-binary temptation, since iOS forbids `fork`/`exec` outright. *Consequence:* no design
   may assume an executable subprocess on device, on either platform.
3. **Non-pixi dependencies are accepted — but they are *project-local*, never a system
   requirement.** *Why:* the Android NDK/SDK have no conda packaging and never will. But
   "outside pixi" must not become "outside the project": a checkout that asks a contributor to
   `sudo apt install` something has moved the dependency into a place the repo cannot pin,
   cannot version, and cannot uninstall — which is the *reproducibility* the pixi rule exists to
   protect, lost by another route. **Refined by the owner 2026-07-19, after a `sudo apt install
   adb` instruction was correctly rejected as a system-wide requirement.**
   *Consequence:* the first written exception to *"pixi is the only package manager"*, and it is
   paid for with:
   - **Everything under a gitignored `.android/` in the repo.** Every piece Google ships —
     `platform-tools` (adb), the NDK, `cmdline-tools` — is a **standalone zip, not an
     installer**, so all of it unzips into the tree and runs from there. Nothing is installed,
     so nothing needs uninstalling, and two checkouts can hold different versions.
   - **One-command bootstrap** (`pixi run android-init`), reading a committed
     `android/toolchain.lock` that pins each artifact by **our own SHA-256** — the durable,
     project-owned assertion. `sdkmanager` and anything it resolves stay quarantined in an
     opt-in environment **declared non-hermetic**.
   - **`pixi run ci` gains no NDK dependency, ever.** A contributor with no Android toolchain
     must still get a green `pixi run ci`.
   - **No system-level device setup either.** USB `adb` on Linux wants udev rules or `plugdev`
     membership, which is exactly the system requirement this rules out — so the documented
     path is **Android 11+ wireless debugging** (`adb pair` with a code over Wi-Fi). No USB, no
     udev rule, no sudo. `adb reverse` is transport-agnostic, so the loopback tunnel that makes
     `fm-serve`'s Host guard pass by construction works identically over Wi-Fi.
   - `openjdk`, `gradle` and the four `rust-std-*-linux-android` targets are conda-forge-native
     and stay in `pixi.toml` — the non-pixi surface is **NDK + SDK + platform-tools only**.
4. **A divergent frontmatter field keeps its current "loud-and-absent" behaviour; the fix is the
   missing UI surface, not a semantic change.** *Why:* it never loses data, and the obvious
   alternative is forbidden in writing at `merge.rs:32-35`. *Consequence:* a characterization test
   locks the behaviour, and a "needs attention" place lists skipped notes with a raw editor. Any
   future semantic change is **its own ruling with its own adversarial pass**.

   > **SUPERSEDED 2026-09-07** — *a divergent field keeps both, by demoting the loser into a field
   > beside it*. The reasoning below stands for the two options it had: with nowhere to put the
   > loser, letting ours win was silent loss. A third option existed unnoticed (`extra` round-trips
   > unknown keys), and the "own ruling with its own adversarial pass" this clause asks for is the
   > 2026-09-07 entry at the end of this file.
5. **The MVP cut line is steps 0–5** of the corrected sequence — everything that needs no git
   backend and no NDK. *Why:* it is demoable in days, fixes real bugs, and stakes nothing on a
   decision whose consequences are not yet observed. *Consequence:* re-cut the line **after step
   0** puts the real UI on real glass.

## The Track M record drifted from the Track M rulings — and the body-merge engine is its own decision (2026-07-19, `#track-m` `#git`)

**Decision — five rulings, from an adversarial drift review** (receipts:
`sessions/2026-07-19-mobile-drift-review.md`; four rulings were each attacked by three skeptics
and **all four were revised under attack**).

1. **The judgement never drifted; the record did.** Every ruling taken since 2026-07-18 traces to
   a stated principle and stands. But the sequence was never re-derived after its foundation was
   rejected: **9 of 12 sequenced items — 7 of 8 inside the MVP cut line — were still written in
   terms of the rejected `git2` backend.** A cold-read test of seven questions a fresh session
   would ask answered **six wrong**. *Consequence:* refuted **arguments** stay verbatim and marked
   (they teach); refuted **instructions** are deleted (they recruit).
2. **The body-merge engine is a decision distinct from the git backend**, and nobody had named it.
   `merge_files` shells `git merge-file` (`merge.rs:204`, `:233`) and takes driver-shaped path
   inputs, so **no engine exists that a phone can call** — under *any* backend. *Consequence:* a
   differential harness against `git merge-file` is a **standing precondition** on any change to
   `merge_files`, and the desktop keeps shelling out **permanently, as the oracle**. Swapping the
   desktop engine would destroy the only grader we will ever have.
3. **Ship-a-git-binary-in-the-APK is struck permanently.** Not on the licence ratio in `deny.toml`
   — which is correct and stays as written — but on dated facts: a bundled binary produces **no
   row** in `ci/third-party.sh`'s `cargo tree` walk, so we become the licence *and* CVE
   distributor of a TLS stack invisible to both gates. That is structurally worse than the defect
   `git2` was rejected for. It is also Android-only forever: iOS forbids `fork`/`exec` outright.
4. **We will not build a backend for a platform that does not exist — but we have evaluated what
   exists.** The option set *as of 2026-07-19* (never "evaluated and closed"): `gix` (permissive,
   but **push is unimplemented**, so it means hand-writing `send-pack` — which the skeptics
   correctly called more sync machinery than the blob mirror already rejected as *"a sync
   framework by another name"*); `git2`/libgit2 (mature push, and **two shipping apps vindicate
   it** — PuppyGit via JNI, and GitSync, which migrated *off* JGit onto Rust `git2` and ships
   Android **and** iOS from one Rust core — but it enters under a wrong licence declaration and
   needs a conscious `deny.toml` exception naming libgit2's linking exception); Path A
   (transport-only, **demo-only, never the end state**, in both places it is mentioned).
   *Consequence:* the merge-driver objection **no longer discriminates** — neither engine can
   invoke an external driver and neither needs to, because the app calls its own `merge_files`
   during the pull. That is GitSync's shipped pattern and is better than a driver: one engine,
   both platforms.
5. **A phone with no git backend is not a failure state.** `git::available()` already returns
   false gracefully (`git.rs:53-58`); such a device is a compliant degraded notebook that reports
   `git: false` and hides the collaboration surfaces. What is constitutive on mobile is **sync**,
   not git — record that rather than suspending the capability model.

**Also found, and it is not a mobile problem:** the invariant at *"Frontmatter merges
structurally"* below is **asserted, not held** — see the ⛔ note there. It reproduces today, on
desktop, in shipped collaboration code.

**Why this entry exists at all:** the 2026-07-18 review's own finding was that the draft *"applied
the project's rules to others and exempted its own proposals"* — and then did the same thing
itself. So: **before any ruling ships, grep this file for the thing it is about, and quote what
you find.** Cite rulings **by subject, never by number** — `mobile-design.md` and `plan.md` number
them differently, and the hybrid propagated a wrong number into this file.

## Mobile is the app on the phone, not a thin client — one core, git-coordinated (2026-07-18, `#track-m`)
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
  /api/blob/<hash>`** — ✅ SHIPPED 2026-07-18 as `GET /api/blob/<reference>` (`fm-serve/src/blob.rs`) — for both platforms. **Mobile shell stays trivially CSS**
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

## The auto-commit stages what we wrote, not where we wrote it (2026-07-18, `#git`)

**Why:** `commit_all` ran `git add -A` every five seconds. In a vault that is also a project
repo — the direction Track V is heading — that is a second author: it staged half-written code
and destroyed a curated index. Scoping it to the vault's *directories* fixed the worst of that
and was still wrong, because in a project vault the notes directory may well be `docs/`, so a
note being hand-edited in Vim was committed mid-sentence.

**Consequence:** `FileStore::put`/`delete` record every path they touch; `commit_all` stages
exactly that list and nothing else. Cleared only once a commit lands, so a failed commit does
not forget what it owed.

**The write-list is deliberately not on the `Store` trait.** That seam carries no paths, no
mtimes and no directory handles, and that is precisely what makes a storage swap a backend
change rather than a rewrite. `Vaults.store` is a concrete `MultiStore`, so it reaches the list
without widening the seam — the same reasoning that keeps `find_blob` off the trait.

**The trade, owned:** a note edited outside the app is now never committed *by* the app. That
is the intent — it is your edit, in your repo, and yours to commit — and it makes true what
`known-issues.md` already claimed about `fm-cli`/Vim writes. **Rejected:** committing anything
we did not write, on the reasoning that "it rides along anyway"; riding along is exactly how a
half-finished sentence becomes a commit.

## The lost-update token is a content hash, not a timestamp (2026-07-18, `#data` `#seams`)

**Why:** `update_body`'s `base` was the `updated` stamp, which only moves for writers that bump
it. The app does. The `.md` merge driver does. **Vim does not** — and `FileStore::put`'s mtime
guard is disarmed a few seconds later by the poll's own reindex, which writes the new mtime
into the index. So the single writer the guard could not see was the one it most needed to.

**Consequence:** the token is the sha256 of the body, carried on `NoteDetail.version` and
returned by `update_body`. Content cannot lie about whether the body moved.

**Measured before committing to it**, because it sits on the whiteboard save path: **1.6 ms in
release**, ~41 ms in debug, for a 2.8 MB body — against a 600 ms save debounce that then writes
and fsyncs that same body, so it is comparable to the write it precedes rather than a new cost.
Pinned by a perf budget, set in debug terms because that is what `pixi run ci` runs, and sized
to catch an algorithmic regression rather than to police the constant factor.

**Rejected:** having the client hash the body itself via Web Crypto — it is available on
localhost, but it makes every save path async for no gain when the server is already holding
the bytes.

## `fm-cli` shares the command library; it does not route through `dispatch` (2026-07-18, `#seams`)

**Why:** the standing debt was recorded as *"migrate `fm-cli` onto `fm_app::dispatch`"*, and
that turns out to be the wrong shape. `dispatch` is a **wire** surface — JSON in, JSON out —
built so a transport can frame it. A CLI wants typed values to print, so `fm show` would have
to serialize and re-parse its own answer. Only 7 of the CLI's 13 commands even have an arm;
`verify`/`manifest`/`restore`/`check`/`reindex` are CLI-only by design, and `merge-md` runs
*before* a store is opened because git invokes it as the merge driver.

**Consequence:** `fm-cli` depends on `fm-app` and calls the **typed command functions**. The
real debt was duplicated *logic*: `Cmd::Add` rebuilt the asset note — title, blob hash, MIME,
put, thumbnail — in five lines that already existed in `commands::ingest`, and the copies had
drifted exactly as duplicated logic does: the CLI's never set `obj.vault`, so a file added from
the command line was stamped with no audience. Both call `commands::asset_note` now. The CLI
keeps streaming from a path (a large file is never held whole) where `ingest` takes bytes from
an upload — that difference is real and kept; the note is not.

**So "one command library" means one implementation of each command, not one entry point.**
`dispatch` is the one *door* for frontends that speak a wire; `commands` is the one *library*
for everything.

## `git2` is rejected; git stays a subprocess capability (2026-07-18, `#git`)

> **PARTIALLY SUPERSEDED (`#git`)** by *The libgit2 exception* (2026-07-19, above): the rejection
> **stands for the desktop** (still shells out to `git`), but the phone — which has no `git` binary —
> now links vendored libgit2 behind the off-by-default `native-git` feature. Read this entry for
> *why linking was rejected as the default*; read the exception for *why the phone is the one place
> it is allowed*. The reversal is deliberate and the chain is the point — do not delete either.

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

## Whiteboard images: git-track them, and do not strip yet (2026-07-18, `#data` `#git`)

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

## Collaboration is git, *exposed* — not reimplemented (2026-07-18, `#git`)
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
  (not the 15 s heartbeat) feeding a one-click-pull chip, wiring the deferred item with existing
  commands.
**Rejected / deferred:** storing any authorship (git already knows — the app writes nothing);
"created by" and per-commit logs (last-editor map is the MVP); anchored comments (need the
deferred backlinks index); live presence (needs the descoped peer — git knows only *pushed*
state). The placeholder committer (`formicaria@localhost`) displays as "you".

## Every entity shows its vault, as a name-coloured badge, in every view (2026-07-18, `#ui` `#vault`)
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

## Cross-vault copy is restrictive by default; create picks a vault (2026-07-18, `#vault`)
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
  a fixed marker, and `assets`/`code` cleared. **Every field a human can type into is stripped
  the same way** — `title`, `status`, `tags`, and both the keys and values of `extra`
  (`refs::strip_value`, recursing into lists). A 2026-07-20 review found the strip was body-only;
  the fix for *that* then missed `status` and `tags`, because they are **typed `Object` fields
  rather than `extra` entries**, so a fix reasoning about the property map never saw them — and
  the test written alongside built its haystack from `title` + `extra` and stayed green. Both are
  free-form user text (`board` groups by any `status` string; a tag is any string). Enumerate the
  fields, do not iterate a map. So a copy can never point outside its new vault.
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

## `.view` files are parsed server-side; the wire carries a name, never a query (2026-07-17, `#seams` `#ui`)
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

## The read view sanitizes untrusted note bodies (2026-07-17, `#ui`)
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

## The note trail is a peer column, not a modal overlay (2026-07-17, `#ui`)
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

## A vault is created, not invented; the vault list gains its first writer (2026-07-17, `#vault`)
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
`ls-remote`, and holding it across that would stall every 15 s `ping`.

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
definition, and `fm-app/src/vaults.rs:3` already rules that fm-core stays free of environment and
configuration concerns); an `open_lossy` for the startup panic (real, but pre-existing and
uncoupled — see `known-issues.md`).

## formicaria: three pillars, one atom; renamed when the plural became true (2026-07-17, `#data`)
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

## The core ships as one file; pixi is the only package manager; deps come from wherever (2026-07-17, `#toolchain`)
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

## Every external tool is an optional feature that declares itself (2026-07-17, `#toolchain`)
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

## Git is a capability, not a dependency (2026-07-17, `#git` `#toolchain`)
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

## Vaults are audiences: git is per-vault, blobs are searched, hiding is only a view (2026-07-17, `#vault`)
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

## Notes merge through a driver that shells out for the body — and is never installed unless it can run (2026-07-17, `#git` `#sync`)
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
silent loss this phase exists to stop.

> **⛔ The italicised invariant above is asserted, not held (found 2026-07-19).** The two
> sentences contradict each other, and the second one is what the code does. `merge_objects`
> returns `None` on a divergent `status` (`merge.rs:104-122`), `merge_files` then calls
> `whole_file`, and that line-merges the **entire file including the YAML fence**
> (`merge.rs:232`). Markers land *inside* the frontmatter, `frontmatter::from_file` rightly
> rejects it, and the note **disappears from every view** — `file.rs:417-420` already names this
> exact cause. Trigger: two people drag one card to different columns. `crates/fm-cli/tests/
> merge.rs` has five tests and **none covers it**.
>
> It fails *loud-and-absent* — the note is collected with a reason (`file.rs:426`, `:471`) and
> surfaced by name (`App.svelte`, "N note(s) could not be read"). That is the right failure, so
> this is a **missing test and a missing UI surface, not a redesign**. Do **not** "fix" it by
> letting ours win the field: `merge.rs:32-35` forbids exactly that in writing, and it would be
> worse — bodies are identical in the card-drag case, so the merge returns **Clean**, auto-commit
> fires at 5s and the sync loop pushes it. Loud-and-absent would become quiet-and-wrong.
>
> Any change to this behaviour is **its own ruling with its own adversarial pass**. Acceptance:
> both values survive in the file, the file parses, and the result is `Conflicted` — never Clean,
> or each machine keeps its own value by fiat and re-derives the conflict forever.
>
> **SUPERSEDED IN PART, 2026-09-07** — *a divergent field keeps both, by demoting the loser into a
> field beside it* (end of file). The diagnosis above is exact and still worth reading; the two
> acceptance conditions about the *file* are now met for the first time. The third — `Conflicted`,
> never Clean — is deliberately not met, and that entry argues why: it was a proxy for the "re-derives
> the conflict forever" failure named in the same sentence, and a winner rule that reads only content
> makes the merge a fixed point, which is what actually prevents that failure.

**Two traps, both load-bearing.** (1) The
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

## A vault gains an identity when it gains an audience, not before (2026-07-17, `#vault` `#git`)
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

## Inline meeting actions become their own note, never a per-block atom (2026-07-17, `#data`)
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

## Board images strip to the content-addressed blob store on save (2026-07-17, `#data`)
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

## Assets are query-layer-excluded from the planning views (2026-07-16, `#seams` `#data`)
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

## Status rotates through the vault's own values; card order is a view preference (2026-07-16, `#ui` `#data`)
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

## `start`/`due` are a `Stamp` (day + OPTIONAL time), not a Date/DateTime pair (2026-07-15, `#data`)
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

## Whiteboard = embedded Excalidraw, lazy-loaded (2026-07-15, `#ui`)
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

## Browser is the product; the native window is removed (2026-07-15, `#ui` `#toolchain`)
**Why:** the Tauri/WebKitGTK window never painted reliably on the developer's
box (blank/gray; mutter/X11 with no compositor). The same SPA rendered correctly
in a real browser, so the UI logic was sound — the webview was the problem.
**Consequence:** `fm-serve` (std-only HTTP) serves the built UI to the default
browser; `fm-app` became a **library only** (no `[[bin]]`, no tauri deps); the
pixi `gui` env, the `e2e/` WebDriver tree, and the deny.toml Tauri-RUSTSEC
waivers are gone. Modern CSS is now fine (real browser, not WebKitGTK). The old
"WebKitGTK blank-window" risk is retired.

## Files-as-truth; the atom is the file (foundational) `#seams` `#data`
**Why:** durability and ownership — a note must survive as plain text without
this app. **Consequence:** one Markdown note = one file; frontmatter is a
generic YAML mapping so unknown/custom props survive read→edit→write with no
silent loss; key order is fixed so `to_file` is byte-idempotent. No per-block
ids/timestamps — block-level structure is explicitly out of scope.

## `fm-query` may never touch fs/db (the insurance policy) `#seams`
**Why:** a pure query engine is what keeps the whole system testable, portable,
and honest about the seam between "data" and "storage." **Consequence:**
enforced by a compile-time boundary *and* a CI grep. Do not add `rusqlite`/
`std::fs`/path handling to `fm-query`, ever. Search works by: `Text` predicate →
FTS5 prefix `MATCH` loads only the hit ids → the pure engine applies the rest.

## Generic, literal-free renderers `#seams` `#ui`
**Why:** a theme/renderer must not encode a specific workflow's enum values, so
arbitrary property values (any status, any type) render + tint without code
changes. **Consequence:** no `todo/doing/done` in `ui/src/renderers/`; column
tints keyed by `[data-value]`, urgency by `[data-urgency]`, labels sourced from
helpers (`urgency.ts`). CI greps enforce it.

## v1 editor = plain `<textarea>` + rendered read view `#ui`
**Why:** a live-preview CodeMirror 6 editor was assessed as the single biggest
build risk. **Consequence:** editing is a textarea over the literal bytes
(`update_body`, byte round-trip tested) with a separate rendered read view
(`render.ts`, `marked`→HTML, lazy KaTeX/Mermaid). CM6 live-preview is **deferred
to v2**.

## Content-addressed blobs, extracted text in the note body `#data`
**Why:** dedup + integrity (bit-rot = a blob no longer hashing to its filename),
and searchability before the git-ignored blob syncs. **Consequence:** blobs are
sha256 with `ab/cd` fan-out; ingest sniffs MIME (`infer`), runs `pdftotext` →
the **asset note's body** (git-tracked, FTS-indexed), and makes a thumbnail.
`put_bytes` dedups before writing.

## Markdown→HTML is JS `marked`, not Rust pulldown-cmark `#ui` `#toolchain`
**Why:** it shipped that way; the MASTERPLAN's "pulldown-cmark→HTML" line never
materialized (the crate is absent). **Consequence:** the only HTML assembly is
one `marked.parse()` in `render.ts`. *(That output is now sanitized with DOMPurify before the DOM sees it — see the sanitize entry.)* The then-unsanitized-innerHTML gap in
[known-issues.md](./known-issues.md).

## No plugin API `#seams`
**Why:** plugin APIs rot and become a compatibility burden. **Consequence:**
extend via modular Rust (add a renderer / store / extractor), declarative
declarative `.view` files, a one-file theme (design tokens), and an optional mlua
hatch — never a stable plugin surface.

## Tauri was the light choice; a native-GUI rewrite is rejected `#toolchain` `#ui`
**Why:** even before removal, Tauri used the system webview (no bundled
Chromium; ~9.6 MB release binary) — lighter than Electron (Logseq/Obsidian run
1–2 GB). **Consequence:** the only genuinely-lighter path (egui/Slint/iced) would
mean rewriting the entire Svelte + KaTeX + Mermaid read view — not worth it.
Now moot (browser), but do not propose a framework rewrite.

## Which attachments travel is a per-vault size limit, in `vault.json` `#vault` `#git`
**Why:** the two-tier split below is right by default and too absolute in practice — a screenshot
in a note is part of the note, and a 200 KB PNG has nothing in common with a 60 MB video except
living in `blobs/`. So `git_assets_max` opts a vault in: blobs at or under it are committed and
pushed with the notes; everything else stays local and restic's.

**In the vault's own file, never in per-device Settings.** This decides what enters *shared,
permanent history*. A per-device value would let the loosest machine decide for every
collaborator, and git cannot take it back: a large file committed once is in every clone forever,
and removing it means rewriting history others have pulled. In `vault.json` the rule travels with
the vault, so everyone pushing to a repo obeys one limit. **Default is absent = notes only**, so
pointing formicaria at someone's existing repo never starts writing binaries into it.

**Consequence:** `git add -f`, per file — `blobs/` stays in `.gitignore` (git cannot filter by
size, and un-ignoring the directory would let everything through). The walk is skipped entirely
when the vault has no opinion, because `commit_all` runs on a 5-second debounce. Lowering the
limit does not untrack what already travelled; the bytes are already in history, so it governs
what travels next. `Descriptor::set_git_assets_max` is the one narrow exception to `write_new`'s
never-overwrite rule, and preserves unknown keys.

## Backup is two tiers: git push (default) + restic (opt-in) `#vault` `#git`
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
restore still knows its blob inventory and `fm verify` names what is missing —
and because it is tracked *and* staged on every commit, it needs its own merge
driver (`manifest.json merge=fm-manifest` → `fm merge-manifest`). Text-
merged it produced `<<<<<<<` inside a JSON file no user wrote or can resolve,
which then froze commits vault-wide.

**The merge is purely additive — `base ∪ ours ∪ theirs` — and the absence of a
deletion rule is the whole decision.** The obvious 3-way rule ("in base, gone
from one side ⇒ deleted") is wrong here: `blobs/` is gitignored and every writer
is a rebuild from local disk, so this is a git-tracked inventory of a
**per-machine** store and each clone legitimately holds a different subset.
"Absent on their side" means *they never received those bytes*. Applying the rule
made the manifest converge on the **intersection** of what each machine happened
to hold, erasing the record of blobs that exist — destroying the one thing the
file is for. The cost of being additive is an entry outliving its blob, which
`verify` reports and `fm manifest` clears: visible and recoverable, versus silent
and permanent.

**The driver must never exit non-zero**, which is a correctness rule, not tidiness:
git reads any non-zero exit as "I left you a conflict in %A" while %A is untouched
and marker-free — the trap `install_merge_driver` documents — and here a conflicted
`manifest.json` freezes commits for the whole vault. So every side is read
leniently (an unparseable manifest is *no information*, not a failure) and a failed
write keeps ours. Android has no driver at all (libgit2 cannot spawn one), so
`git_native::pull` routes the path by name to the same `Manifest::merge`.
`destination.ts` classifies both a git URL and a restic repo as local/remote —
a local path is a legitimate destination but must never be reported as "off this
machine".

## Squash-on-push — a deliberate reversal of "don't build commit management" `#git`
> **Reversal (`#git`), current.** Reverses `MASTERPLAN.md`'s *don't build commit management* — for
> the **remote** only; the local repo still keeps every `auto:` commit for undo.

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

## Acquiring a vault: `naturalise` is the seam, not a transport trait `#vault` `#seams`
*(2026-07-19, `sessions/2026-07-19-acquiring-a-vault.md`)*

**Why:** the owner wants vaults acquirable from elsewhere by several methods —
git and restic now, p2p later — and explicitly *"these vaults can be git init or
not, that's not a core necessary requirement of formicaria."* The tempting seam
("a vault is a directory, so any directory copy moves one") is **false**: vault
state has three tiers, and one of them must *not* travel (`index.sqlite`, the
`.git/config` merge driver, the committer identity) while a third
(`vaults.json`) lives outside the vault entirely.

**Consequence:** `fm_core::acquire::naturalise` owns tier 2 in one place, and
every acquisition path routes through it. A transport moves bytes and nothing
else, so **a new transport cannot corrupt a vault** — it touches neither the
merge path nor the safety. There is deliberately **no `Transport` trait and no
registry**: the extension point is the filesystem, which is also why a folder
synced by other means is already adoptable with no code.

**The rule that bounds it: `sync` requires git, `copy` works with anything.**
There is exactly one merge engine (`merge::merge_texts`) and it is git-shaped, so
only git can be a two-way relationship; every other transport hands you a copy.
A non-git vault can therefore be *shared* but not *collaborated on*, and the UI
says that in those words rather than implying otherwise. This is what keeps p2p
from needing a CRDT layer (`plan.md:67`): p2p moves a copy for any vault, and can
carry packs for git ones.

**Accepted cost:** restic acquisition returns notes and media with **no history**,
because `backup` snapshots the vault's own directories and not its root. It is a
*recovery*, not a *join*, and stating that before the button was preferred to
widening what restic snapshots.


## Android TLS: the trust store is loaded from memory, never from a file `#track-m`
*(2026-07-19, `sessions/2026-07-19-git-on-the-phone.md`)*

**Why:** `openssl-src` passes `no-stdio` to OpenSSL's configure on **every** Android target, so
the vendored build has no `BIO_s_file`. `X509_load_cert_file` therefore fails with
`X509_R_BIO_LIB` on a file that is present, correct and readable by the same process — measured
on a device. `SSL_CERT_FILE`, `SSL_CERT_DIR` and `GIT_OPT_SET_SSL_CERT_LOCATIONS` all end in a
file BIO, so **none of them can ever work there**. Five separate diagnoses were spent producing
better files before this was found.

**Consequence:** `fm_core::git_native::add_certs_from_pem` parses the bundle through a *memory*
BIO and hands each `X509 *` to libgit2's `GIT_OPT_ADD_SSL_X509_CERT` (option 45, which
`libgit2-sys` does not bind, hence the two extra `-sys` dependencies — both resolving to the
libraries `git2` already links). `fm_app::ca_bundle` supplies the bytes from the device's own
Conscrypt store. Two ordering rules are load-bearing and non-obvious:

- It must run **before the first `git2` call in the process**, because `libgit2-sys` never
  defines `GIT_OPENSSL_DYNAMIC` and OpenSSL is initialised **eagerly** inside
  `git_libgit2_init()`.
- It must **initialise libgit2 itself** first. Calling `git_libgit2_opts` raw skips the `init()`
  that every `git2::opts::*` wrapper performs, and `git_openssl__add_x509_cert` then dereferences
  a NULL `git__ssl_ctx` — a launch crash, not an error code.

**Rejected:** shipping Mozilla's `cacert.pem` as an asset (what PuppyGit does). It works, but it
is a trust store that goes stale the day it ships and can only be refreshed by a release. Reading
the platform's means the app follows the device's own trust decisions and OS updates.

**Also rejected, emphatically:** `certificate_check` returning `CertificateOk`. That is what
GitSync does, and it skips hostname verification while sending `userpass_plaintext` over an
unauthenticated connection. The leaf certificate libgit2 hands the callback has no chain, so real
verification is not possible there either.

## `fm-serve` sends a Content-Security-Policy, and it is the "nothing phones home" guard `#ui` `#seams`
**Why:** a note is Markdown, and Markdown renders remote images. A single
`![](https://attacker/p.png?leak=…)` in a note that arrived by merge, by copy, or in a shared
vault fetches the attacker's URL **on render, from the user's machine**. That is not an XSS
chain: it needs no script, it survives DOMPurify (whose allow-list permits `https:`), and it
survives a human reading the diff. Until 2026-07-20 there was no CSP anywhere in the server —
the app self-hosted Excalidraw's fonts precisely to avoid this class, and then left the door
open for note content. **Consequence:** `write_response` sends one policy on every response
(`main.rs::CSP`), plus `nosniff` and `Referrer-Policy: no-referrer`; blobs get
`default-src 'none'; sandbox` behind their existing `Content-Disposition: attachment`.
`frame-ancestors 'none'` is separate from `default-src` and load-bearing: the server sits on a
fixed localhost port with no auth, so any page the user visits could otherwise frame it.
**Accepted losses, both real:** `'unsafe-eval'` is refused, so Excalidraw's harfbuzz font
*subsetter* throws and an exported SVG embeds a font URL rather than the bytes (in-app boards are
fine); and no CSP directive stops a top-level navigation the user clicks, which
`Referrer-Policy: no-referrer` only blunts. **The Android shell now sets its own CSP in
`tauri.conf.json` — it was `null`, i.e. the phone was the only wholly unguarded surface — but
that policy was verified only as far as *launching clean* on a real phone — installed, started,
zero CSP violations in logcat, and the embedded frontend confirmed free of inline `<script>`. It
admits `http://fmblob.localhost` (wry rewrites the custom scheme on Android) and Tauri's `ipc:`
origin, but neither has been exercised: opening a note with an image and a whiteboard is what
would prove them, and getting either wrong shows up as broken images rather than a crash.**

The two clauses that shape the app's own code: `img-src 'self' data: blob:` is the beacon fix
(`data:`/`blob:` are local bytes, so they carry no request off the machine), and **`script-src
'self'` with no `'unsafe-inline'`** — which is why Excalidraw's `EXCALIDRAW_ASSET_PATH` line
moved out of `index.html` into `ui/public/excalidraw-asset-path.js`. Keeping inline scripts out
is what stops a sanitiser bypass from being code execution. `style-src 'unsafe-inline'` is
unavoidable and low-risk: Mermaid injects `<style>` and KaTeX/Excalidraw set `style=` on
everything they draw. The Android shell serves through its own custom scheme and is unaffected.

## A commit that committed nothing must say *why* `#git`
**Why:** `commit_all` returns `Ok(false)` both for "clean tree, nothing to do" and for "this
vault is mid-merge, so I refuse" — and those are opposites. The refusal is right (staging
conflict markers would publish them as content), but read as success it means **every write
after the conflict is saved to disk and never committed**, for as long as the conflict sits
there, while the sync loop reports `synced`. **Consequence:** the `commit` dispatch arm answers
`CommitResult { committed, conflicts }`, filling `conflicts` from `vcs::conflicts` whenever it
committed nothing, and **all three callers** stop at the `conflicts` phase, which the UI already knows how to
show. The third one is the point: `App.svelte`'s 5 s debounced auto-commit is what
`commit_all` itself calls "the default path, not an edge case", and the first version of this
fix touched only `sync.svelte.ts` — so the dominant caller kept swallowing it. The extra
`vcs::conflicts` call is gated on `.git/MERGE_HEAD` existing, so the common case (clean tree)
costs a `stat`, not a second `git status` spawn. The `git.rs`/`git_native.rs` signatures were
deliberately **not** changed: the distinction is only needed where a human is told about it,
and widening the seam would have touched both backends and forty call sites for nothing.

## 2026-08-29 — Windows never needed OpenSSL; asking for it is what broke the build `#git` `#track-m`

The v0.2.1 release job failed on `x86_64-pc-windows-msvc` with *"Could not find directory of OpenSSL
installation."* `outstanding.md` had named this the first suspect and predicted the cause —
compiling vendored OpenSSL needs Perl and NASM on the runner image. **That prediction was wrong, and
the right diagnosis inverts the fix.**

`openssl-sys` was in the Windows graph with **no features at all**. `git2`'s `vendored-openssl`
propagates as `libgit2-sys/vendored-openssl` → `openssl-sys/vendored`, and on Windows the middle
edge does not exist: libgit2 speaks **WinHTTP** there and `libgit2-sys` declares no `openssl-sys`
dependency for that target. But `fm-core` declared `openssl-sys` and `libgit2-sys`
**unconditionally** — they exist for exactly one thing, Android's `GIT_OPT_ADD_SSL_X509_CERT`
in-memory trust store, needed because its vendored OpenSSL is built `no-stdio`. So on Windows they
arrived as *direct* dependencies that `vendored` could not reach, and the build script went looking
for a system library.

Measured on both targets before the change: `openssl-sys v0.9.117` bare on win-64 against
`openssl-sys v0.9.117 openssl-src,vendored` on aarch64-linux-android.

**Decision: do not vendor OpenSSL on Windows — stop asking for it.** Both raw `-sys` crates move to
`[target.'cfg(not(windows))'.dependencies]`; `git_native::add_certs_from_pem` splits, with a Windows
arm returning `Ok(0)` because WinHTTP uses the machine's own certificate store and there is no
in-process store to add to; the differential test is gated to match the function rather than to skip
a platform. Verified: **`openssl-sys` is absent from the Windows dependency graph entirely**, and
Android's is unchanged.

**Why this is better than the vendoring it replaces**, and worth saying because vendoring was the
obvious move and was already written down as the plan: the Windows binary loses a TLS stack it never
called; the runner needs neither Perl nor NASM; and `deny.toml`'s named libgit2/OpenSSL exception
narrows rather than widens — we now distribute vendored OpenSSL on Android only, which is the one
platform that has no alternative. The rule the log already states — *"a bundled binary makes us the
licence and CVE distributor of a TLS stack"* — argues for exactly this direction.

**The general lesson, since this is the second time this week a `[target.…]` detail decided a
build:** a feature enabled on a dependency does not necessarily reach a *sibling* declaration of the
same crate. `cargo tree -f "{p} {f}" --target <triple>` answers it in one command, and would have
answered it before the tag was cut.

## 2026-08-29 — formicaria ships no third-party tool, and here is what it cost to find out `#toolchain`

Asked whether installation is simplified on all four platforms, this session proposed a "features,
not tools" installer: a resolver, a fetched tools pack from `pixi.lock`, and a licence row to answer
the objection. Three adversarial reviews took it apart. Recording the *rejections* with their
numbers, because every one of them is the kind of thing that looks obviously right on the way in.

**Bundling poppler and libvips — rejected on mechanics, not on licence.** The licence argument is
weaker than it first appears: shipping a GPL binary beside an MIT one, invoked as a subprocess with
its own COPYING, is aggregation, and leading with it would not survive contact with a lawyer either
way. The blockers that actually decide it are measured: `poppler` carries **5 prefix placeholders,
2 of them binary-mode**; `libvips` 3 and 1. Those must be substituted for the package to work — and
substitution rewrites the Mach-O, invalidating the ad-hoc signature Apple Silicon requires. Skip it
and `libpoppler` cannot find `share/poppler`, so PDF text extraction **degrades silently for
non-Latin documents while reporting success** — the exact failure `#agent`'s capability ruling
forbids. There is no naive-unpack path that is correct. (`libvips` also declares `imagemagick`,
whose only conda-forge build is `agpl_…`, which declares `ghostscript`; that is a real complication
but it is not what settles it.)

**Bundling restic — rejected on arithmetic and on the absence of an updater.** It looked ideal:
BSD-2-Clause, zero declared dependencies, **zero prefix placeholders**, genuinely relocatable. Two
things killed it. First the size: 9.4 MB is the **zstd-compressed `.conda`**, and the archives are
gzip — measured, restic gzips to **11.8 MB against an 8.8 MB payload, a 134 % increase on the
download**, per platform, forever. (This correction was itself the second in a chain: an earlier
review corrected an uncompressed-vs-compressed error, and the fix repeated the same mistake one
level down. Measure the artifact, not the metadata.) Second, `pixi.toml` says `restic = "*"` — the
lock is a *resolution*, not a pin, so any `pixi update` would silently change what ships. And
**there is no updater anywhere in the tree**: a folder copied to a USB stick in 2026 would still run
0.19.1 against its backups in 2028, with the launcher's `PATH` prefix shadowing any newer restic the
user installed to escape. Track M's ruling against a bundled binary — *"we become the licence and
CVE distributor of a TLS stack invisible to both gates"* — applies unchanged to a static Go binary
with its own `crypto/tls` and cloud backends; "no conda dependencies" is a statement about shared
libraries, not about vendored source.

**A `fm_core::tools` resolver — rejected as a misreading of our own rule.** `merge_command()`'s
*"never a bare relative path"* is about a path **persisted into `.git/config` and re-executed months
later** — *"PATH at `git pull` time is not PATH now."* `Command::new("restic")` resolves in this
process, against this process's PATH; the premise does not transfer. And preferring our copy over
the user's contradicts the 2026-07-17 sentence *"the binary does not care how the optional tools got
onto `PATH`."* A launcher `PATH` line would reach 3 of at least 12 entry points — omitting, among
others, the `.bat` diagnostic launcher, so the diagnostic would report a different capability set
from the app.

**Declaring `vipsthumbnail` — rejected because it would advertise nothing.** It does run, on every
asset ingest (`fm-app/src/commands.rs:1300`), which made a Settings row look like the cheapest win in
the plan. But `ui/src/lib/render.ts:370` says in as many words that `has_thumb` has no consumer: no
thumbnail is requested on any platform. Declaring it would reverse the 2026-08-28 ruling that
*removed* the previews claim for exactly this reason. The consumer comes first, or nothing does.

**Consequence:** the optional tools stay on `PATH`, acquired however the user prefers, exactly as
the 2026-07-17 ruling designed. What ships is what shipped before. The work that survived is
documentation — a broken download link, a macOS instruction Apple invalidated, a false promise on
page one of the manual — plus the guards that keep those from drifting again.

## 2026-08-29 — the welcome screen asks the one question git asks at the worst moment `#ui` `#git`

Git refuses to commit without a committer. The app covers for that — `ensure_repo` writes a
placeholder identity, so a notebook works perfectly with nobody's name on it — but a placeholder is
permanent in a way a setting is not: it is stamped into every commit, and history is not corrected
later. The moment it starts to matter is the moment someone else clones the vault, which is months
of commits too late. So it is asked once, at the start, where the answer is cheap.

**The gate reads `VaultInfo.identity`, not `backup_status`.** That is the load-bearing choice. The
gate sits on the path that renders the app, and `backup_status` runs a network `git ls-remote` per
vault — the slowest command in the product. A first-run screen that waits on it waits on the
network. `list_vaults` already spawns git locally for each vault's `label`, so this is one more
local `git config` read beside a call that was happening anyway; the 2026-07-17 ruling refused
exactly this trade once already. Gated on the **default** vault, because per-vault would make a
welcome screen reappear whenever someone clones a second notebook.

**Skipping is a real answer.** A notebook that demands a name before it will hold a note has
misunderstood what it is: the placeholder keeps full history, and `BackupPanel` asks again at the
moment a remote makes a name matter. The dismissal is remembered in `localStorage` — a convenience,
not state, and losing it re-asks a question that is cheap and skippable again. Every access is
wrapped, and a storage failure means **dismissed**: never trap someone on a screen we cannot
dismiss.

**Hidden entirely where git is absent, or unknown.** Every field would be a control that cannot do
anything, and `gitAvailable` is `null` until the first heartbeat answers — unknown is not "yes", so
the gate fires on `=== true` and not on a maybe.

**Identity is saved first, and on its own.** `set_git_remote` can carry an identity but only
alongside a URL, and the two fail for unrelated reasons — so a typo'd remote must not cost the name
that was typed correctly. Two calls in a fixed order, and a failure of the second reports **both**
halves ("your name was saved; the backup repository was not"), because printing only one of them is
how someone retypes a stored name or walks away believing a remote is set.

## 2026-08-29 — ask git for a repository before asking git what it is missing `#git` `#data`

`outstanding.md` §2.9 had been open and un-root-caused since it was reported: *"a backup with no git
reports success and records nothing."* Neither half of the guess was the bug.

`git::unrecorded` returns an empty list for a directory that is **not a repository** — it has to,
git cannot answer otherwise — and `commit_all` creates the repository as its own first act. So the
`commit` arm swept for orphaned notes *before* there was a repo for them to be missing from, found
none, and `commit_all` then init'd, staged the `.gitignore`/`.gitattributes` it had just written,
and returned `true`. **The first backup of a new vault reported success with not one note in
history** — and `commitStep` carries a no-conflict result straight on to the push and says "synced".
Worse, the notes only *became* visibly unrecorded afterwards, because now there was a repo.

**Decision:** `vcs::ensure_repo` is called in the `commit` arm before `adoptable`, and in
`record_unrecorded` before its `unrecorded` scan. Ordering, not a new responsibility: `commit_all`
still calls it, it returns early on a `.git` that exists, and a machine with no git fails at the
same place with the same error. Pinned by
`a_first_backup_on_a_device_with_no_git_binary_creates_the_repository` on **both** backends.

**What did not reproduce, measured rather than argued:** on Linux with no git binary, `ping.git` and
`backup_status.git` are both `false` and `commit` returns `io error: could not run git (is it
installed?)`. Those surfaces were honest already, and the entry's second sentence was wrong about
them. Recorded because "diagnosed but not root-caused" survived months partly on that sentence.

## 2026-08-29 — formicaria keeps one restic password, and it is not in `vaults.json` `#vault`

Encrypted media backup needed three things and the app could only ever see one of them: restic
installed (a capability it reported), a repo location (a key you hand-edited into `vaults.json` —
Settings said *"there is no UI for it"* in as many words), and `RESTIC_PASSWORD` (documented answer:
*"a launcher you have edited yourself"*). A tier of backup reachable only by someone willing to be
their own system administrator is not a feature this product has.

**The repo:** a **narrow writer**, `vaults::set_restic`, not a relaxed `vaults::save`. `save`
deliberately leaves a known entry's every byte alone — right for a function whose job is appending
vaults, and exactly why it could not be the one that changes a setting. `set_restic` rewrites that
one key of that one named entry and carries every other key, every other entry, and any shape this
app does not understand straight through. The constraint was met, not worked around.

**The password:** kept by the app, `0600`, beside the git token in its own config directory —
**never** in `vaults.json`, which is a file of paths a user may reasonably open, copy, or send
someone while debugging, and a password beside the paths it unlocks is the one place it must never
be. One password for every repo, because a per-vault one only multiplies the places a secret lives.
`RESTIC_PASSWORD` still wins over it, so an already-edited launcher keeps working and the stored
file is a fallback rather than a competing source of truth.

**Unlike the git token, this has no "the platform already has somewhere for it" escape.**
`secrets.rs` exists only where there is no credential helper, because everywhere else git owns the
secret and we must not. Restic has no helper on any platform — `RESTIC_PASSWORD` or
`--password-file`, and nothing else — so this is the only answer, on every platform, and the module
doc says so rather than letting the reader assume the token's reasoning applies.

**Consequence:** losing that file loses the backups, and restic has no recovery. The panel says so
in those words at the moment the password is set, which is the only moment it is actionable. The
same panel gained the **git token field** it never had: an HTTPS remote with no stored credential
asks for one, with the clone form's scope advice; an SSH remote and a machine whose helper already
holds it are asked nothing.

## 2026-08-29 — the assistant is not a user, and must not keep the app alive `#agent` `#ui`

`fm-serve` exits when nobody has been in touch for 90 s, so **closing the tab closes the app**. It
learned "somebody is here" from any authenticated request, refreshed above the route table. The study
agent polls the vault every 1–5 s for as long as it runs — `alive`, `thread_roots`, `agent_present` —
so while the assistant was on, that window **could never close**: the app held itself open by asking
whether it was open, and `fm-serve` plus a 2.4 GB `llama-server` stayed resident until the machine was
rebooted.

Reachable only since `FM_AUTO_SHUTDOWN` was flipped **on by default** (2026-08-28, so that a person
double-clicking a launcher is not left with a console window quietly keeping their notebook running).
Before that only the launcher had a watchdog at all; after it, every user did, and the assistant
silently defeated it — on a laptop, behind a launcher with no console, that is heat and a flat battery.

**Decision:** the runner declares itself with `X-Formicaria-Agent: 1` on **every** request, and a
request that says so is served exactly as before but does not count as somebody being here. Not just
the liveness probe: fixing only `/api/alive` would have left the bug where it was, since every other
poll refreshed the same timer.

**Why a header and not a rule about routes.** The distinction is *who is asking*, not *what they
asked for*; the same `POST /api/alive` is a browser tab saying "I am still here" and the agent saying
"are you still there". It needs no secrecy — the only thing sending it can do is give up your own
claim on keeping the app open, which any client can already do by staying quiet.

**Consequence:** the constant is spelled in two crates that cannot import from each other (nothing
depends on `fm-agent-run`, which is how "the core never learns the agent exists" stays true), so
`ci/checks.sh` asserts the two spellings match. Pinned by
`the_assistants_own_polling_does_not_keep_the_app_alive` and its positive twin.

## 2026-08-29 — a capability is a *reason*, and it is not `preflight::admit` `#agent`

`agent::installed()` was "does `agents/start-agent.sh` exist". On Windows that could answer **yes**
while `spawn` then failed on a missing `bash`, into a stderr the launcher hides by design — the same
class of failure the 2026-08-28 ruling was meant to end. And the audio-transcription toggle had no
capability at all: it stored a preference, answered `{"ok":true}`, and transcribed nothing, because
`agent-serve.sh` starts whisper only when its runtime **and** its model are staged.

**Decision:** `unavailable() -> Option<String>` replaces the bool, and the settings row prints what it
returns. Three causes, three different answers a reader can act on: **this OS is not there yet**
(`fm_agent`'s monitor reads `/proc` and fails closed elsewhere, so `admit` refuses before anything
spawns — Windows and macOS therefore offer nothing), **the stack was never shipped here**, or **`bash`
/ `python3` is missing**. `transcribe_available()` is the same question for whisper, and
`/api/set_transcribe` now refuses like `/api/set_agent` instead of reporting success.

**Static on purpose — deliberately *not* wired to `preflight::admit`.** `admit` reads instantaneous
free memory to decide whether to start a model *right now*, and belongs where it is. Reusing it here
would tell a user with a few tabs open that the assistant "is not installed": false, unactionable,
gone again by the time they looked — the same rule ("say what will actually happen") broken in the
other direction. The bound the capability check does **not** yet cover is the weights; see
`known-issues.md`.

## 2026-08-29 — a pinned checksum needs a pinned revision, or it is a bomb on a timer `#agent`

`fetch.rs` had verification branches that were dead code: `models.toml` published no `sha256`, so a
body that closed early was renamed as complete and `ensure_model` returned that truncated file
forever — the only self-heal was uninstalling the app, which also destroys the vault.

**Decision:** every `[[models]]` entry now pins **`revision` + `sha256` together**. Alone, a checksum
against `resolve/main` is worse than none: `main` is a moving pointer, so the day a repo re-quantises,
every download starts failing verification — permanently, on every device already shipped, with
nothing a user can do and no way to reach a released archive. Pinned to a commit, the URL names the
same bytes forever and a mismatch can only mean a corrupt transfer, which is what a checksum is for.
Provenance: the Hugging Face API's `siblings[].lfs.sha256`, cross-checked by re-hashing the three
files in this checkout — so these pin the bytes that were actually benchmarked. `agents/fetch.sh`
reads and enforces the same pair, so the desktop path is not the unverified one.

**Also, before arming it:** a **read timeout** (there was none — a half-open connection blocks
forever, and `MAX_STALLS` counts *failed attempts*, so an attempt that never returns is never one);
`Accept-Encoding: identity` (a transparently-decoded gzip makes `Content-Length` describe different
bytes than the ones written, and with a `Range` the server applies the offset to the encoded stream,
so a resume appends garbage); a completeness check; and a **fatal / transient split** so a cancel or a
full disk stops at once instead of spending twelve attempts and ~90 s of backoff to reach the same
sentence.

**A free-space *precheck* was considered and rejected.** `std` has no free-space API, so it would mean
`libc` in the one crate whose whole point is being dependency-free enough to cross-compile to Android.
`ErrorKind::StorageFull` is `std`'s portable name for `ENOSPC` and `ERROR_DISK_FULL`, costs nothing,
and is *more* accurate — a precheck can pass and the disk still fill. Same outcome, no new dependency.

**Consequence:** `ensure_model`/`fetch` take a `cancel` predicate. Android already had the predicate
(`current(generation)`) and used it only *after* the fetch, so turning the assistant off during a
1.4 GB first-run download left it downloading on mobile data; now it stops, keeping the `.part` so
turning it back on resumes.

## The study agent's model warm-up is deferred a few seconds after launch (2026-07-24, `#agent`)
**Why:** with the agent enabled, `fm-serve` auto-spawns the whole stack at launch — and the model
server reads a **multi-GB GGUF off disk and loads it onto the GPU the instant it starts**. Since
adopting Qwen3-VL-4B (2.4 GB) that load began *the moment the browser is told to open*, and on the
owner's laptop (4 GB GPU shared Intel iGPU + NVIDIA via Vulkan, Firefox compositing on the same GPU)
it starves exactly the disk and GPU the browser needs to paint the app's first frame. Measured:
`fm-serve` binds in ~57 ms and fires `xdg-open` at ~33 ms, but the *window* crawled in because the
2.4 GB read + Vulkan load hit at t≈0. That is what "double-click takes forever now" was — not the
launcher or the server, which were never slow. **Consequence:** `agent::spawn_at_launch` now claims
the run slot immediately (so a concurrent settings toggle still can't double-spawn) but sleeps
`AGENT_WARMUP_DELAY_SECS` (5 s) on a background thread before the actual spawn, releasing the slot if
that spawn fails. The page gets an uncontended window to render; the assistant warms a beat after the
app is already usable. This is *only* about launch contention — the `/api/set_agent` "turn it on now"
path stays immediate, because there the user is asking for the model, not for the app to open.
Verified after the change: browser-open at +0.06 s, model spawn at +5.09 s.

## 2026-07-31 — an unrecorded note shows **both** its `created` and its file mtime `#data`

The "Not in history" rows showed one timestamp: the file's mtime. All 147 of the phone's rows read
`2026-07-31 07:44`, and I read that as a one-minute burst of writes at machine pace. That inference is
not available from mtime alone — a copy, a restore or a bulk rewrite stamps every file at once, weeks
after the notes were made — and it was steering the diagnosis.

So the row carries `created` (the note's own frontmatter, which survives copying) beside `modified`,
each labelled *made* / *written*, and shown only when they differ. **Made ≠ written is the diagnosis**:
same minute means the notes really were created then; an old `created` with a fresh `modified` means
something rewrote notes it did not need to, which is a different bug with a different fix.

Cost is one extra parse per displayed row, capped at `UNRECORDED_DETAIL` (50) — the same read that
already yields `title` and `role`.

## 2026-07-31 — a refused recording says **why**; `committed: false` was two answers wearing one face `#data`

`commit_all` answers `bool`. It returns `false` when nothing of ours moved — the normal, quiet outcome
of a debounced auto-commit — and also when **git refuses**, because a path is unmerged or the tree is
unchanged. `record_unrecorded` passed that bool through, and the UI printed *"Nothing left to record"*
for both. So the owner tapped Record with 147 notes outstanding and was told there was nothing to do.

A silent refusal on the one action that rescues unrecorded notes is worse than an error, because it
looks like success and stops the user looking. `record_unrecorded` now returns `reason` when it declines
— recomputed at that call site rather than threaded through `commit_all`'s signature, since it is the
only caller that owes an explanation — naming the unmerged notes, the missing identity, or the
unchanged tree, and pointing at the surface that clears it. The UI shows it as an **error**, not a
notice, because the action did not happen, and distinguishes it from an empty vault by `notes > 0`.

Pinned by `fm-app/tests/unrecorded_after_restart.rs` (notes outstanding *and* a merge in flight) and
`ui/src/App.recordRefused.test.ts`.


## 2026-08-30 — the archive ships one note, and a test reads it back `#ui`

The archive already opened into a ready vault rather than asking a stranger to invent a location
(2026-08-28). But a ready vault is an **empty** one, and an empty notebook answers none of *what is
this, and where do I click*. `Welcome.svelte` does not answer it either — by design it asks the one
question git forces (a committer's name) and gets out of the way. Dismissing it landed the reader on
a blank Timeline.

So `release.yml` stages `packaging/welcome/` into `stage/<dir>/vault/notes/`: one ordinary note,
~360 words, titled *Start here*, which is deleted like any other and says so in its last line. **Not
generated by the app on first run** — the vault has to be a plain folder of files either way, and a
note written by the installer is a note the user can copy to a USB stick with the rest.

**It is dated with the build**, by `sed` at staging time rather than a placeholder in the file. A
placeholder does not parse, so the repo copy would not be a note; a real date means that if the
substitution ever stops running the note still works and is merely old. Fail-safe over fail-silent,
which is the whole theme of this entry.

**Why it is tested at all.** The note loader is deliberately tolerant, so a malformed note does not
error — it **disappears**, and the one person guaranteed to hit that is the one least able to
diagnose it. `crates/fm-app/tests/welcome_note.rs` therefore parses the shipped file, checks the
filename still equals the id and that it is not secretly a message or proposal (either would hide it
from every view), asserts it round-trips byte-for-byte so a first edit is not a hundred-line diff,
and **opens a vault staged exactly as the release does — date rewrite included — through the real
`dispatch`**, asserting the Timeline is not empty and Search finds it.

The second test is the one that earned its keep. It pins the controls the note names, and writing it
caught **three false claims in my own draft**: `+` opens a menu (*New note*) rather than typing
straight into a note, `Ctrl+K` is the command palette and not a create shortcut, and — the one that
would have been worst — *"Search finds the text inside PDFs"*, which needs `poppler`, a dependency
`release.yml:298` states plainly that the archive does not ship. A welcome note that promises a
missing feature is worse than no welcome note: the reader concludes they installed it wrong.

## 2026-08-30 — a saved view was never in the commit, and a view we cannot draw now says so `#ui`

Three things about `.view` files, found while planning the customization work and fixed together
because the first one is load-bearing for anything else that writes a file into a vault.

**A view the app saved was never recorded.** `views::save_view` does a bare `std::fs::write`, and
`commit` stages *"exactly the files this app wrote or deleted — not a directory, and certainly not
`-A`"* — the store's write list, plus files matching `<notes dir>/<ULID>.md`. A `.view` matches
neither. So it was written to disk and committed nowhere, while the save dialog said, verbatim,
*"Saved as a file in your vault's `views` folder, so it travels with your notes."* It did not: it
survived on the machine that made it and existed nowhere else. Invisible in the only way that
matters — the view *works*, right up until you pull on the other machine and it is absent, which is
indistinguishable from the app having lost it.

`save_view`/`delete_view` now return the path they touched and `dispatch` enrols it via
`Multi::seed_written`. **This widens the write list**, whose usual safety argument is the ULID
naming scheme. The justification is narrower rather than broader: these are paths *this process just
wrote, this second*, which is the list's original meaning — it can never sweep up a hand-written
file or someone's staged work. Pinned by `saved_view_travels.rs`, which asserts against **real git**
(`ls-tree` on HEAD), not against the write list: the question is not whether we remembered to enrol
the path, it is whether the file is in the commit. It fails against the previous code.

**`renderer: gallery` drew a timeline in silence.** The gallery renderer was deliberately removed —
assets open from the notes that reference them — but the enum arm stayed, so a view asking for it
parsed, loaded clean, and rendered as something else with nothing anywhere saying why. This was the
one path the *"a broken view names itself, never vanishes"* discipline had not been applied to,
because from the loader's side nothing was broken. `list_views` now reports it as an error naming
what to change it to, and `save_view` refuses to author one — the app must not manufacture the
breakage it reports. `search` went the other way and now **draws**: it is the flat, un-bucketed list,
and `Search.svelte` takes no `query` when a view supplies the rows, because a view is a filter
someone wrote, not a search someone typed — a prompt to type would be an instruction the reader
cannot follow. This closes an entry that had been sitting in `outstanding.md` §3 as *"accepted — do
not fix without deciding"*.

**`ViewInfo` gained `vault`.** `list_views` spans every vault; `delete_view` resolves a name against
one. Deleting by name alone therefore resolved against the *default* vault, where a view belonging
to another is simply not found — and `delete_view` treats missing as success. A delete that reports
success having deleted nothing is the worst answer available. `list_views` leaves the field empty
(it is handed a path and has no name to report) and the dispatch arm stamps it while iterating the
configs, which is the only place both are in hand.

**And "Delete view" is now a button.** `delete_view` had existed end to end — command, `ipc.ts`,
mock — since views became saveable, with **no `.svelte` caller anywhere**: a view could be made from
the app and then removed only with a file manager, in an app whose owner works only through the UI.
Two clicks, not a modal: it removes a file from someone's vault so it must not ride on one stray
click, but a modal is a whole new surface for something done twice a year — the same reasoning that
deleted the command palette. The armed state changes the label, not only the colour.

## 2026-08-30 — a theme is a file in the vault, and three questions on top of it `#ui`

`MASTERPLAN.md:133` has listed `themes/*.css   # CSS themes — git-tracked` in the vault layout since
the beginning, and `:343,478` names CSS themes as one of three sanctioned extensibility layers —
*"extensibility comes from CSS themes and declarative `.view` files — no code execution."* Nothing
implemented it: `grep -rn "themes/" crates/ ui/src` returned nothing at all. **So the file layer is
the spec executed, not amended, and owes no reversal.** `app.css` was already built for it — *"three
layers, so a theme is one file... re-skinning needs zero renderer edits"* — and both CSPs already
carried `style-src 'self' 'unsafe-inline'`, now pinned by a test in `fm-serve` so a later tightening
pass cannot remove it in silence. (That test reads the header, not a browser; the neighbouring
`frame-src` test exists because jsdom applies no CSP and a green test there hid a broken feature for
months.)

**A theme is not a note.** Tempting — `NotePanel` already edits arbitrary text — but a stylesheet in
`notes/` becomes indexed, searchable, and permanent noise on every board and timeline; it
contradicts the vault layout collaborators reason about; and note bodies are the app's *designated
untrusted input*, arriving from other people through the `.md` merge driver. Moving a trust boundary
to save writing a textarea is a bad trade.

**The file travels; wearing it does not.** The theme is committed like any vault file, so it arrives
on the other machine. The selection is `localStorage`, like `fm-theme` — a collaborator who pulls
your vault gets your theme and is not forced into it, and a phone and a monitor can differ. This
falls out of the existing rules rather than needing a new one.

**The escape hatch, and what each layer is actually worth.** A theme is arbitrary CSS and there is
**no CSS-level guarantee** against one that hides every control — `!important` at equal specificity
beats anything we write, and saying otherwise would be the kind of claim this file exists to stop.
So: (1) the editor previews live and writes nothing until *Keep*, which catches almost everything;
(2) an **armed-boot guard** that does not depend on CSS at all — a flag is set before a stored theme
is applied and cleared on the first click, key, wheel or touch, so if the app starts with it still
set from last time, nobody could reach anything and the theme is not put back. Recovery is *close it
and open it again*, which works on Android with no address bar and on a phone with no keyboard;
(3) a fixed escape control, on screen **only while the theme is unproven** — the first interaction
takes it away, because that is the moment we learn the app is reachable, and a permanent floating
button would tax every session to insure against a rare one. Only layer 2 is claimed as reliable,
and it is the only one fully tested. Cascade layers were considered and rejected: unlayered styles
outrank every layer, so `@layer` would make the user theme lose to `app.css` and defeat the feature.

**The Appearance form — extends an exception, not a new one.** Three fixed questions (accent, text
size, font) that write token declarations into that same file. **This is not the query builder
arriving through the other door**, and the test is specific: the `.view` filter UI was refused
because it is a *grammar* — nine predicate kinds, with composition and nesting. Three questions with
fixed answer types have no operators, no composition and no nesting; it is the move already approved
as *"one tag is an arrangement, not a query builder."*

**The hard stop, and where the creep is:** *a token may be re-valued; a new token cannot be invented
from Settings* — the same line `keys.ts:3-7` draws when it lets you rebind a command but not invent
one. "Let me set `--surface` too" is still a list and is fine. "Let me write a selector or a media
query" **is** the grammar, and is refused — the text box underneath already exists for exactly that.
Enforced mechanically: `ci/checks.sh` fails if `Appearance.svelte` names a design token in its own
script, so the closed list has one home (`appearance.ts`'s `SUPPORTED`, 59 tokens). Verified by
breaking it on purpose.

When the file says more than the form can, **the form goes read-only and refuses to rewrite it** —
the rule `views::save_view` applies to a filter richer than one tag, for the same reason: silently
flattening someone's work is worse than declining to.

**Density is refused for v1.** `--space-1…8` are consumed literally in ~40 rules; a density knob
means a multiplier token and touching all of them, for one slider. Its own change, with its own
tests, if ever.

## 2026-08-30 — named workspaces are refused, and this is the record of why `#ui`

Obsidian ships saved layouts and it is the obvious next ask, so the reasoning is written down rather
than re-litigated. Literally it adds no third arrangement — `tiled`/`single` remain the only two —
but *"one shell, two arrangements"* recorded a **reversal condition**: *"a third arrangement is
genuinely needed → that is the moment to check whether this has become the customisation system it
refuses to be, not to add a fourth."* This is that moment, and the check fails.

It needs a manager — save, load, rename, delete, set-default, repair a corrupt one — and a manager
is precisely the second-Settings drift that got the command palette deleted. It turns
`localStorage['fm-workspace']` into a keyed collection with lifecycle and migration, in a loader
that already carries back-compat branches for two prior shapes. The research is against it: most
people never curate configurations, and the one workspace here **already persists and restores by
itself**. And in Obsidian this is a *plugin* — which is the company the feature keeps.

**What the want actually is, and the cheap thing that serves it: "reopen the view I just closed."**
One entry in the action list, one member on the closed `Command` union, one closed pane held in
memory. No manager, no new persistence, no ruling touched. Not built here; recorded as the thing to
build if the ask returns.

**Reversal condition:** if a named-configuration manager is ever wanted, the dated reversal must
answer, in its own words, *why is this not the customisation system the app refuses to be?* — and it
supersedes the "Hard stop at two" bullet, not this entry.

## 2026-08-30 — rename is its own command, and Help exists on the phone `#ui`

> **SUPERSEDED in part, 2026-08-31** (*view customization is withdrawn…*). *Rename is its own
> command* is still true of the backend and is why the command survives; what went is the **button**,
> along with Delete view. The Help half of this entry is untouched.

**Rename must never be save-under-the-new-name-then-delete-the-old.** The composition looks
equivalent and is not. `save_view` guards against flattening a filter it cannot express by noticing
that the target file *already exists* — and a brand-new name hits no existing file, so the guard
never fires, the fresh file is written without the filter, and deleting the original destroys the
only copy. Two safe operations, one unsafe result. Worse, it is what a user does *by hand* when the
app offers no rename, so the absence of this command was itself the trap.

`views::rename_view` moves the bytes and rewrites **only the `name:` line**, so a hand-written
filter, the key order, the comments and the spacing all survive as their author left them; the label
goes through the same YAML serialiser `save_view` uses, so a name containing `:` or `#` cannot make
the file parse as something else. It refuses a name already taken rather than overwriting, and both
paths reach the write list — a rename git only half-sees is a file that returns on the next pull.
`rename_theme` is the same rule with less to do: the filename *is* the name, so the bytes are moved
and never rewritten. Pinned by a test that renames a view carrying a `not:` predicate the app could
not have written and checks every byte of it survives.

The UI reuses the dialog that already names views rather than growing a second one — naming a new
view and renaming an old one are the same interaction. **The tag question is not asked when
renaming**, because that field belongs to the file being moved and answering it there would rewrite
a filter the screen cannot describe. Themes rename inline in their own list, which is already on
screen.

**Help now exists on every device.** It was hidden behind `{#if !isPhone()}` because the phone's UI
comes from the Tauri shell, where `/manual/` resolves to nothing and a Help button that 404s is
worse than none. Correct in the small and wrong in the large: the result was a phone with no help at
all, in an app whose first non-technical tester's verdict on Help was that it was *"difficult to
find and click on"*. `HelpPanel.svelte` is a short page in the bundle, on every device, and it links
to the full book only where the book is actually served. Embedding mdBook in the mobile shell is
still worth doing — it needs its own protocol scheme and its own CSP, exactly as `MANUAL_CSP` exists
for on the desktop — and this is what makes the gap survivable meanwhile.

**And the keyboard bindings that already existed are now visible.** Per-command rebinding has
shipped for a while and could not be seen without scrolling to the bottom of Settings, which is the
opposite of what Nielsen's seventh heuristic asks. The action list now shows each action's shortcut
beside it, the standard menu pattern — and shows **"not set"** where there is none, which is how
`newView` stopped being a capability learnable only from a source comment. It also gained a list
entry at all ("Keep this arrangement as a view"), having previously been reachable only from a
pane-header button that appears on three pane kinds.

## 2026-08-30 — the chrome moves to a collapsible side panel, and the top bar goes `#ui`

> **This reverses the rail removal** asserted in `ui/src/App.svelte` — *"The nav is a horizontal top
> bar now (it was a tall left rail that wasted vertical space). Everything the rail held lives here
> in one row."* That claim was never a dated entry, only a comment on the shipping code; it is
> superseded here, and the comment is rewritten to point at this entry.

**Why the original removal was right, and why it no longer applies.** The rail that was removed was a
tall strip holding a handful of icons **beside a top bar that still existed**. It spent a full column
of height to show five things and bought nothing back, so folding those five things into the bar that
was already there was strictly better. The objection was never "a vertical panel is wrong"; it was
"a vertical strip that displaces nothing is wrong".

**What changes it: the panel replaces the top bar rather than joining it.** Everything the top bar
holds — make something, search, which vault, the vault and contributor filters, the sync and
unreadable chips, Back up, Help, Settings — moves into the panel, together with the list of open
views. The top bar is then deleted, not shrunk. So the workspace **gains** the bar's height at every
width, which is the exact resource the original removal was protecting. A strip that displaces a row
is a different proposition from one that sits beside it.

**And it collapses.** The old rail could not be got out of the way; this one narrows to icons and
then to nothing, remembered per browser like the theme. When width is scarce the reader takes it
back — which is the answer to the one real cost, that a panel spends horizontal space on a screen
that may not have it to spare.

**Why a panel and not the view strip (D1).** The strip was the cheaper option and it is the one this
plan recommended. The owner's reading is better: on a wide monitor **horizontal space is the abundant
resource and vertical is the scarce one**, so spending width to buy back height is the trade that
suits the screen. It is also the arrangement Material recommends for an expanded window, arrived at
from the opposite direction.

**This is not a third arrangement.** `decisions.md`'s "hard stop at two" governs **pane placement** —
`tiled` and `single`, both untouched here. This moves *chrome*, which is a different axis; the panel
holds the same controls in a different place and the grid below it is unchanged. Stating that plainly
because the two are easy to conflate, and conflating them is how the hard stop would get worn away by
something that never actually tested it.

**The phone does not get a panel.** On a coarse pointer at narrow width the controls move to a single
**bottom** bar and the top bar goes for a different reason: a top bar on a phone holds actions the
thumb cannot reach, and nearly every app ships one anyway.

> **SUPERSEDED in part, 2026-08-31** (*the chrome is one row at the top*). The **actions** bar stays
> at the bottom exactly as argued here; what moved to the top is the row of **view tabs**, replacing
> the pane header rather than joining anything. The thumb-reach argument is not refuted, it is
> outweighed — see that entry. Branching stays on space and input
capability, never on platform — a wide touch tablet gets the panel, a narrow desktop window gets the
bottom bar.

**Amended 2026-08-30, after using it.** The first build moved the top bar's contents into a column
and changed nothing else — a bar standing on its end, which is not what a rail is. Three faults:

- **It had no views in it**, which was the whole point of the arrangement that was chosen.
  `ViewBar` is still pinned to the bottom of `.body` and hidden above 60rem, so the panel offered no
  way to reach a view at all. The panel now lists **the views you can open** — every built-in plus
  every saved view, a fixed list in a fixed order so it can be learned. That is a different question
  from `ViewBar`'s, which is *which of my open windows to look at*, so the two do not duplicate and
  `ViewBar` is untouched. Both are built from one `viewTargets` array, shared with the action list,
  because two half-menus disagreeing is exactly how the palette drifted into a second Settings.
- **Collapsed, it cropped text instead of hiding it.** `width: 3.5rem` with `overflow: hidden`
  leaves every label laid out and merely cut, so the rail showed a sliver of "Back up" and a sliver
  of a vault name — text severed mid-word reads as broken, not as compact. Labels now live in a
  `.lbl` span the stylesheet can hide, and the controls that are *only* their words (the vault
  picker, the filter chips) are hidden outright rather than reduced to empty boxes. Every one keeps
  the `title` it already had, so a name is a hover away rather than gone.
- **The search showed as a lone centred lens**, because `searchOpen` starts false. The collapsing
  was justified by *"a search field is the widest thing in the bar"* — a statement about a bar. In a
  panel of fixed width a field costs one row of height, so the field is open there and the button is
  kept for the bar. **Both forms render and CSS chooses**: deciding in script would mean measuring
  the viewport, which this same ruling forbids.

**And the labelling has one rule per state**, which is what the owner was reacting to: expanded is
icon **+** label, collapsed is icon **only**. "Back up" loses its word to match Help and Settings
beside it — but it comes back *while saving*, because "is anything happening?" is the one moment an
icon cannot answer.

**Amended again the same day: the pane header stopped being a second switcher.** With the panel
listing views, every window's header still began with a *rotator* — the view's name as a button that
cycled the view on click, and on a wheel or swipe anywhere across the header. One window per view
made those names read as a horizontal bar of views duplicating the vertical one, which is what the
owner saw. The name is now plain text; the header keeps what belongs to *that window* — its
grouping, its density, saving and renaming it, closing it, dragging to move.

This is the honest end of a story already in this file. The rotator's own comment read *"the rotator
button worked and nobody found it"*, and the wheel and swipe were added to compensate for a control
nobody saw. Deleting it is the fix those gestures were standing in for, and the eight tests that
pinned their behaviour went with it — keeping them green would have meant keeping the feature.

**And it did not render.** For two commits the rail described above was `display: none` at every
width: `display: flex` inside a min-width query, `display: none` in a base rule later in the file,
equal specificity, source order deciding. So the entry above described a panel nobody could use, and
removing the rotator on top of it left no way to choose a view at all. Visibility is now decided in
one place — shown by default, hidden only in the narrow query — and `ci/checks.sh` fails if the bare
rule returns. Where the chrome is a bar and there is no rail, a **Views** button opens the same
`viewTargets` list.

**What the rail offers is now separate from what exists.** `BUILTIN_PANES` stays the registry —
every kind still renders, still has an icon, and a workspace saved earlier still opens. Two lists sit
on top of it. **`search` is offered nowhere**: an empty search pane does nothing, because the search
box makes one when you type, so listing it invited a click that produced a blank pane. **`activity`
is off the rail and stays in the palette** — the rail is a column you look at, where every entry is
paid for in attention, while a filterable list can afford a rare one. It is *not* the same view as
the timeline (notes by the day you wrote them, versus every edit including other people's), so
removing it outright would have cost a capability on a premise that does not hold; keeping it one
search away is progressive disclosure rather than deletion.

**The consequence, stated rather than discovered: a window can no longer be re-pointed.** You open
the view you want from the panel and close the one you do not. That is the model the arrangement
implies, and it is worth living with before deciding anything is missing. The label also lost its
border and surface: it had them because it used to be pressable, and dressing a label as a control
is the affordance lie this codebase already has a comment about elsewhere.

**Reversal condition.** If the panel ends up habitually collapsed, it is not earning its width and
the strip (D1) was the right answer after all. That is a question about use, not about pictures, so
it is settled by living with it rather than by argument.

## 2026-08-30 — a status is coloured by hashing the word, not by a list someone guessed `#ui`

`StatusChip.svelte` said of itself that *"the tint comes from `[data-value]` in the theme"*. **No such
rule existed.** The only `[data-value]` colouring in the app was three hardcoded literals on board
column *headers*, so every status pill on every card in every view rendered the same grey whatever
it said — an aspirational comment describing a feature that had never been built.

The obvious fix — extend the hardcoded list — caps colour at a vocabulary we guessed. `blocked`,
`drafting`, `in review` and every other word a real vault uses would still be grey, which is the
same failure with more lines. So the hue is **derived from the word** with `hashHue()`, the function
that already colours vaults and contributors; `vaultColor.ts` records that hashing *replaced* a
theme keyed off a data attribute there, for this exact reason. Renderers stay literal-free because
no renderer names a status — `Board` hashes `col.value`, whatever it happens to be.

**Two custom properties, and the split is load-bearing.** The component emits the raw hash as
`--hash-hue`, inline; the theme derives `--hue` from it and the styling reads `--hue`. Setting
`--hue` inline instead would be shorter and would silently destroy the exceptions below — an inline
style beats every selector, so a named colour could never win. The indirection is what leaves the
last word with the theme. Pinned by a test asserting the component emits `--hash-hue` and never
`--hue`, because this is invisible and would regress without a sound.

**Then the named exceptions, expressed as hue overrides rather than colours.** For a handful of
words the colour is semantic and a hash cannot know it: a finished status must read as finished, not
as whatever 4-in-360 it lands on. `[data-value='…'] { --hue: … }` at the same specificity, declared
after, so it wins — and because the exception moves the *hue*, the pill stays internally consistent:
background, border and text travel together through one mechanism.

**No `color-mix()`.** The first draft used it for the exception backgrounds. Nothing else in this app
uses it and its support in the Android WebView we ship to is unverified, so relying on it would have
been a silent visual failure on the one platform that cannot be debugged (`console.*` reaches no
logcat). Overriding the hue needs only `hsl()`, which is everywhere.

**Colour never carries the meaning alone** — the chip has always shown its word and still does, which
is what keeps this out of the trap the calendar's urgency bars are still in (a 3px border hue with no
second carrier; unfixed, recorded in `known-issues.md`).

## 2026-08-30 — the timeline has two densities, and thumbnails finally have a caller `#ui`

The owner asked for the timeline to show notes *"expanded as if a post in an Instagram app"*, and
then named the reason that matters most: **a shared vault**. One stream, every collaborator's notes
as they arrive, each post saying which audience it came from and who last touched it. `recent`
already queries across every vault in scope and `ObjectMeta` already carries `vault`, so "all in a
single feed" was the existing data shape — what was missing was a shape on screen that made it
legible.

**A mode, not a renderer.** `pane.timelineMode: 'feed' | 'compact'`, defaulting to feed, switched
from a segmented control in the pane header that mirrors the agenda's month/week/list exactly. This
adds no view kind, no `.view` renderer and no third arrangement. **It is emphatically not the
removed gallery**: a gallery browsed *assets*, and was removed because assets open from the notes
that reference them. This browses *notes* — nothing appears in it that the timeline did not already
show, and a note's picture is an attribute of the note, not an entry in its own right.

Both densities stay because they answer different questions, and the research says so: cards are for
heterogeneous browsing, lists scan better for finding a known item. A feed answers "what has been
happening"; the list answers "where is that note". Losing either would be a downgrade.

**The feed was blocked on the thumbnail path, which was half-built on both platforms and finished on
neither.** Thumbnails have been generated on every image ingest, served by `resolve_asset_bytes`,
and reported by `AssetStatus.has_thumb` since they were built — but nothing could ask for one *as a
URL*, so every inline image decoded the original. `render.ts` measures it: ~50 MB of decoded pixels
for one 12 MP photo. A feed makes that per screen. The slot existed on both sides and was empty:
`/api/blob/<hash>` split its query string off and discarded it, and the Android protocol handler
parsed a `query` variable it never read — the `unused variable: query` warning our own release build
printed. `?kind=thumb` fills both. **That warning disappearing is the tell the gap is closed.**

**A missing thumbnail falls back to the full blob** rather than 404ing. A vault ingested before
thumbnails existed, or one where `vipsthumbnail` was never installed, still shows its picture —
slowly, which is the right degradation for "we could not make a small copy". Failing would make the
feed look broken on exactly the vaults that predate it.

**The blob is still resolved first, always.** `find_blob` decides which vault a reference belongs to
by asking whether the blob is *there*, and that is the check keeping a paired device out of another
audience's media. Resolving a `derived/` file directly would route around it, so the full blob
proves the right and only then is the smaller file swapped in. Pinned by a test that a thumbnail
alone, in a vault without the blob, is refused.

**Bounded, because nothing else is.** `recent` returns every note with no limit and there is **no
virtualisation anywhere in this app** — `MASTERPLAN.md` lists `@tanstack/svelte-virtual`; it is not
installed and not used. A row per note was already the ceiling; a post carries an image as well. So
the feed mounts thirty and offers a button for the rest. Thirty is a number to be changed after
watching it on a real vault; what matters is that the cap is **visible and has a way past it**,
rather than a silent truncation the reader cannot tell from a short vault. The compact list is
deliberately left unwindowed — it behaves exactly as it always did, so this change cannot alter the
view that already existed.

**`AssetMissing.svelte` finally has a caller.** Its own comment named "gallery tile, card chip" as
its homes and it had zero imports. A collaborator's note whose media has not synced yet is the
ordinary case in a shared vault, and it has to read as "not here yet", never as a broken app.

**Not done, and not implied:** the read view still points at full blobs. Converting it is the larger
Android win and deserves its own measurement; this work only makes it possible.

**Open question, deliberately not answered here.** The feed is sorted by `created`, so a
collaborator's *new* notes surface but their *edits* do not resurface. Sorting by `updated` would
fix that and introduce a worse problem — the feed would reshuffle under you as you type, since
`updated` moves on every autosave. The post shows its last editor and when, so an edit is visible
where it happened; if resurfacing is wanted it needs a considered activity ordering, not a swapped
sort key.

## 2026-08-30 — a launcher must not hand you a different build, and a closing tab can say so `#ui`

Two changes to how the server starts and stops, both from one report: *"the current one seems still
the old one."*

**The launcher now knows which build is already running.** On `AddrInUse`, if the port was held by
our own formicaria, a new launch printed "already running", opened the browser at it, and exited.
That courtesy is right for a second double-click and wrong for a rebuild: `serving_formicaria` only
checked that `/api/alive` returned 200, so it could not tell an *older binary* from itself. Combined
with a release binary serving the UI compiled into it, the result was that rebuilding and
relaunching showed the old app — indistinguishable from a build that had done nothing.

`/api/alive` now reports a **build id**: the executable's own length and mtime, via `current_exe()`.
Not a version string — the crate version is `0.0.0` and never moves, so it would call every build
identical, which is the bug. Same id, hand over exactly as before; different id, refuse with what to
do about it. The failure goes from silent and wrong to legible.

**A closing tab says so, instead of being waited out.** The 90-second idle window exists to survive
a *throttled* beat — a backgrounded tab's timers drop to about once a minute, and killing the app
under someone still using it is the worse failure. That reasoning is untouched. But silence is only
weak evidence of absence, and a tab that is closing has strong evidence: `pagehide` sends a beacon
to `/api/bye`, and when the last registered tab leaves, the window drops to 8 seconds.

**Why tab *ids* and not a flag.** A reload fires `pagehide` too, and with two tabs open one closing
must not take the server with it. The server keeps a set: a tab joins on its first command and
leaves on its goodbye, the short window applies only when the set empties, and any arrival cancels a
pending goodbye — so a reload's new page (about a second later) is already registered. **A client
that never identifies itself can never trigger the short window**, which is what keeps an old cached
page or an unfamiliar client from being killed after 8 seconds of quiet.

**The id is read off the raw header line, not the lowercased copy** used for matching — the same
thing the cookie beside it does, and for the same reason. The first version read `lower`, so `tab-A`
registered as `tab-a`, the goodbye never matched, and the shutdown silently did nothing. Found by
running it, not by a test; there is a test now.

`sendBeacon` rather than `fetch` because it survives the page going away, and the id travels in the
query because a beacon cannot set headers.

## 2026-08-31 — one view at a time is the default, `auto` is gone, and the shell reads its own insets `#ui` `#track-m`

> **This supersedes the layout half of *One shell, two arrangements* (2026-07-19).** That entry's
> **"hard stop at two" is upheld** — it is what this one finally delivers. What is reversed is the
> existence of a *third* value, `auto`, and its status as the default.

**Three faults, one shape.** Using the 2026-08-30 chrome on a phone and a desktop, the owner
reported six things. Five were the same complaint: the shell shows everything it knows — every open
window as a tab, a count of them, a plus carrying every name, the whole contributor chrome — instead
of the one thing you are looking at. On a phone that was about a third of the screen.

**`auto` is removed, and `single` is the default.** `Layout` is now `'single' | 'tiled'`, exactly
the two the 2026-07-19 entry said it should be. `auto` was the default and meant "follow the
available space", which sounds free and was not:

- **It made every narrow rule a pair.** `known-issues.md` carried an entry saying each one had to be
  written twice — once for `[data-layout='single']`, once inside `@media (max-width: 60rem)` for
  `[data-layout='auto']` — because `auto` was the default and a phone therefore never matched a
  `single` rule. Writing one half was silent, and it shipped that way twice. Removing `auto` deletes
  seven such pairs and the known-issues entry with them.
- **It synced nothing.** The layout preference is `localStorage`, per browser. `auto`'s pitch — tile
  on the laptop, one view on the phone — is what two per-device settings already do.
- **It hid a live bug.** `[data-layout='single'] .topbar .icon-btn { display: none }` outranked the
  rule that shows Help, Settings and the panel toggle in the wide panel. Only a user who
  deliberately chose `single` was bitten, so nobody was. Flipping the default would have deleted
  those three controls from every desktop rail on first launch, and no test could see it: jsdom
  applies no CSS.

**Existing `auto` records are migrated, not grandfathered.** `layout` is written by the first
`persistWorkspace()`, so "they chose `auto`" is false for essentially everyone. And 2026-07-19 set
the test itself — *"if `single` ever stops working in a desktop browser, the design has failed"* —
which a default nobody runs cannot meet. A stored `tiled` is a real choice and is left alone; the
record is stamped with a schema version so the migration runs once.

**Opening a view re-points the window instead of adding one.** This reverses the 2026-08-30 chrome
entry's *"The consequence, stated rather than discovered: a window can no longer be re-pointed. You
open the view you want from the panel and close the one you do not."* That consequence was accepted
on the strength of living with it. Lived with, it is wrong: with one view at a time, tapping a name
means *show me that*, and appending a window answers a question nobody asked — on a phone it also
spends a feed and a scroll position. `focusOrOpen` reuses a pane that already shows the target and
falls back to opening one, which is the generalisation of two retargets the code had already grown
by hand (`openNoteInPane` never opened a note twice; `onSearchInput` re-pointed the one search pane).
**"New window" stays unconditional** — the one place the user is explicitly asking for another, and
the way into `tiled`.

That last retarget was also **silently broken** under `single`: it re-pointed the search pane but
never made it the visible one, so a second query did nothing. One helper fixes the class.

**The shell now reads its own window insets, and the guessed floors stay as a fallback.**
`known-issues.md` had named this fix and recorded it unbuilt: `env(safe-area-inset-*)` is 0 in the
Android WebView because wry never forwards `WindowInsetsCompat` into the page, so `app.css` floored
the insets by hand — `1.75rem` at the top, which is *less than this phone's camera cutout*, and
`0.5rem` at the bottom, which clears a gesture pill but not a three-button navigation bar. Both
guesses were visible on the owner's device and neither was visible to any test.

`MainActivity` now sets `LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES` and pushes the real
`systemBars() | displayCutout()` insets into the page as the `--safe-*` custom properties the
stylesheet already consumes. It lives in `ci/android-inject-service.sh`, which owns that file
wholesale, because `gen/` is gitignored and regenerated — the same reason the foreground service
lives there.

**The floor is kept, against `app.css`'s own instruction to delete it when this landed.** The bridge
fires on an event; a reload before it fires would paint under the camera again, which is the exact
bug being closed. A stale floor costs nothing once a real value arrives, because an inline property
on `documentElement` outranks the `:root` rule. Stating this rather than quietly leaving the comment
wrong.

**Three surfaces bypassed the token layer entirely** and no bridge would have reached them:
`.board-exit` used raw `env()`, `.theme-escape` had a literal `bottom: 12px`, and `UnrecordedPanel`
carried its own `3.25rem` guess. The first two now read the tokens; the third drops its guess for the
real number.

**Arrangement is now a set of inherited custom properties**, not a selector each rule must remember
to match twice. `.app` defines the single-view values and `[data-layout='tiled']` overrides them;
components read `var(--cell)`, `var(--viewbar)`, `var(--view-name)`. This deletes every
`[data-layout=…] :global(…)` rule — including one that **tied on specificity** with `ViewBar`'s own
scoped `.viewbar` and was decided by bundle order, which is the same defect `ci/checks.sh` already
polices for `.panel-views`, sitting unnoticed in a second component.

**Reversal condition.** If `tiled` goes unused now that it must be chosen, it is not earning the
grid, the drag-resize and `MAX_PANES`. That is a question about use, settled by living with it.

## 2026-08-31 — the chrome is one row at the top, and the pane header belongs to `tiled` alone `#ui` `#track-m`

> **This reverses one paragraph of *the chrome moves to a collapsible side panel* (2026-08-30)** —
> *"a top bar on a phone holds actions the thumb cannot reach"*. It also **amends** *A view says
> what it leaves out, and a board admits it scrolls* (2026-08-24); both are bannered below.

**The complaint, and it is one complaint.** Using round 1's shell, the owner asked for eight
subtractions in one breath — the view name row, Save view, the contributor chips, the board's
status labels, the vault chip row — and closed with *"these modifications should go towards a more
lean UI"*. The single shape underneath: **the chrome should be one row, naming the view you are in
and carrying that view's own controls, and nothing else.** Two rows of chrome on a phone is one row
too many, and every control that is not about the view in front of you is noise.

**The view tabs move to the top, and the actions bar stays at the bottom.** The superseded
paragraph is right that a phone's top edge is out of thumb reach, and switching views *is* a
frequent action — that argument is not wrong, it is outweighed. What carries it: only the **tabs**
move. The bar holding *make something*, search, backup and settings stays at the bottom where the
thumb is, which makes this the ordinary tabs-above/actions-below split rather than the top-bar
this project removed. It is also what makes the row affordable: it replaces the pane header rather
than joining it, the same trade that justified the side panel in the first place.

**The pane header becomes an arrangement value, not a deletion.** Asked to remove it outright, the
honest answer was that it is the *only* route to board group-by, agenda month/week/list, timeline
feed/list, the "filtered:" chip, Rename, Delete view, and drag-to-reorder — none of which has a
keyboard command. So the controls are extracted into `ViewControls.svelte` and rendered **twice
from one definition**: in the top bar in `single`, in the pane header in `tiled`. `--pane-head`
joins `--pane-grip`/`--pane-resize` in the block round 1 built.

Keeping the header in `tiled` is not a hedge. With four panes on screen you need names to tell them
apart, a handle to reorder them, and controls that say which pane they belong to; `single` needs
none of that because there is one pane and the bar above it is already talking about that pane.
**Round 1's un-hiding of `.pane-close` is superseded** — it was un-hidden because `ViewBar` had left
the phone, and `ViewBar` is coming back.

**Save view moves to the palette; Rename and Delete do not.** Save already had a palette entry, so
dropping the button costs nothing and the 2026-08-28 ruling (*a saved view is an arrangement you
keep*) stands. Rename and Delete have **no** palette entry, and `Pane.svelte` carries the note that
`delete_view` once shipped end to end *"with no button anywhere calling it, so a view could be made
from the app and then only removed with a file manager"*. They travel with the controls.

**The contributor filter is removed, not relocated.** Both filters funnelled through one `shown()`;
the author half is deleted along with `contributors()`. The *identity* ruling is untouched — every
"edited by" label, its colour hash and the Activity stream stay, and the `activity` fetch keeps two
of its three consumers. What goes is a row of names in the chrome that the owner never filtered by.

**The vault filter becomes a dropdown, and gains something the chip row had.** A chip per vault
does not survive a long list. The menu is the existing `anchorTo`/backdrop skeleton with one
deliberate divergence: every menu in this app is single-shot, and a filter is many-of-many, so its
rows are `menuitemcheckbox` and a click does not close it — which also means **Escape-to-close is
new code here**, since none of the three existing menus handle Escape. The trigger names the state
("All vaults" / "2 of 5"), so a filter that is hiding something says so on its face, which the chip
row only did by colour.

**The board's rail is measured at every width instead of unconditional below 40rem.** The 2026-08-24
principle is upheld and is why the rail is not simply deleted: the owner asked to remove it because
*"we can see them on the title of the columns already"*, which is true only of the columns on
screen — and a board that snaps one column at a time is exactly where it is false. That entry's own
bug report was *"my done column is not showing up"*. What was actually wrong is that the phone
showed the rail **always**, including when every column fit, so it read as clutter rather than as a
map. It now appears when the strip genuinely overflows, which is the only moment it was ever saying
anything.

**And the destination moved into the ＋ menu.** *"The place where a note goes maybe can be selected
when pressing Plus, without having a dedicated scroll down menu that takes all that place."* It was
a permanent `in <select>` in the chrome — a control on screen at all times for a choice you make
only while creating something, and on a narrow bar a whole row of it. It is now a `Create in`
section of the ＋ menu: `menuitemradio` rows, one-of-many, and like the vault filter **the click
does not close the menu**, because you pick where and then pick what, and closing would mean
opening ＋ twice for one note. `fm-create-vault` and `setCreateVault` are untouched; only the
surface moved. It had **no test at all** before — which is part of how a control comes to sit in
the chrome unexamined — and has one now.

**Reversal condition.** If the top row plus the bottom bar still reads as two rows of chrome on the
phone, the next cut is the bottom bar's contents, not the tabs — the tabs are the thing the owner
asked to see.

## 2026-08-31 — view customization is withdrawn, the board rail goes, and a stale binary wasted a review round `#ui` `#toolchain`

> **Reverses** the board-rail half of *A view says what it leaves out, and a board admits it scrolls*
> (2026-08-24) — amended earlier the same day, now removed. **Supersedes** *A saved view is an
> arrangement you keep, not a query you write* (2026-08-28) and the rename/delete half of *rename is
> its own command, and Help exists on the phone* (2026-08-30). All three are bannered.

**The trap first, because it cost a whole review round and was nobody's UI problem.** The owner
reviewed the desktop and reported four faults. Three of them had already been fixed hours earlier —
they were looking at a build from before the work. `fm-serve` serves **the copy of the UI baked into
the binary** unless `FM_UI_DIST` is set (`main.rs:1109-1111`), and the desktop icon
(`packaging/formicaria.sh`) sets only `FM_OPEN`/`FM_AUTO_SHUTDOWN`. So `ui/dist` can be perfectly
fresh, the page can be reloaded any number of times, and the screen still shows the last
`pixi run build`. The assistant checked `ui/dist`'s mtime, declared it current, and told the owner to
reload. **A UI change is not on the owner's screen until `cargo build` re-embeds it**; `ui/dist`
freshness is evidence about the dev server only. Recorded in `known-issues.md` too, because the
failure is silent in both directions — nothing warns, and the UI looks merely wrong rather than old.

**View customization is withdrawn, not deleted.** The owner, looking at a pane headed *Board ·
status* with a group-by box and a Save view button: *"how is the user expected to know what to do
with it? I would remove that entirely: views are basically fixed for now and view customization will
need its own design plan."* That is the right reading of what shipped. Saving a view was reachable
before anyone understood what a view *was*; the group-by box accepted free text against properties
nobody had been shown; and none of it had a story for how a person discovers any of it.

So the **surface** goes — group-by, Save view, Rename, Delete view — and the **capability stays**:
`save_view`/`rename_view`/`delete_view` remain in `fm-app`, their wrappers remain in `ipc.ts`, and
every Rust test remains green. `list_views` and `run_view` are untouched, so a `.view` file written
by hand, or arriving from a collaborator over git, still lists in the rail and still opens. That is
the difference between withdrawing a surface and removing a feature, and it is what makes the
promised design plan cheap to land.

**Which is exactly why the "filtered: …" chip stays.** 2026-08-24 exists because the owner reported
*"my done column is not showing up"* and the cause was a `.view` whose filter removed a column
silently. That entry's own reasoning was about a file that could be *"neither authored nor deleted
from the UI"* — which is precisely the state this ruling returns to. The chip is now the **only**
thing standing between an inherited `.view` and a repeat of that report, so it is more load-bearing
after this change, not less.

**`groupBy` becomes the constant it already defaulted to.** `newPane` sets `status`, and `views.rs`
defaults an unspecified `group_by` to `status`, so nothing structurally changes: one board feed key,
`fm-board-order` always keyed under `status`, and dragging a card between columns doing what
`onSetStatus` already hardcoded. The real loss — grouping by `project` or `tags` from the UI — is
the point, not a side effect.

**The board's column rail is removed, and this one is against advice.** The owner was shown that it
had already been made conditional that morning (gone on a wide desktop, present on a phone only
because a phone genuinely snaps one column at a time), was told in as many words that removing it
re-opens their own 2026-08-24 report, and chose removal anyway. Recording the failure mode rather
than softening it: **on a phone, columns two and beyond are now invisible with nothing on screen
saying they exist.** The reversal condition is a repeat of *"my column is not showing up"* — if that
comes back, this is the entry that predicted it, and the fix is the rail, not something new.

**The theme escape stopped re-arming.** *"I continuously see the Turn off my-appearance in the
bottom right, what for?"* — a fair question about a control that was never meant to persist. It is
the way out of a theme that hides every other control, and it is supposed to vanish on the first
click. The `$effect` that applies a theme also reads `vaultTick` so an edit on another machine lands
without a timer; `vaultTick` bumps whenever `ping` sees the vault move, **including the user's own
writes**. So every edit re-ran the effect, re-armed the guard, and put the button back — and each
re-arm added five more capture-phase listeners without removing the last set. Arming is now gated on
the applied selection actually changing. Re-applying the CSS on a tick stays; that was the feature.
The JS boot guard in `appearance.ts` is untouched — it is the layer that actually rescues the app,
and a theme cannot style it away.

## 2026-08-31 — the feed gains an excerpt and a thread, and three tempting halves of it are refused `#ui` `#data` `#track-m`

**The ask.** *"Improve the feed… visualize the notes more than the first row but fading towards half
of the note. Also, when tapping a note, continue discussions in it, like in instagram comments,
without opening a new window… give an instagram or mastodon like access and visualization to notes,
modify them and access them and discuss them all in the view."* Reviewed by five agents (the standing
cap): two research, three adversarial — scale/phone, principles, interaction.

**The headline: the comment half was already built.** `thread.rs` holds the note-classes, `reply`
writes one file, `thread` reads one discussion, `thread_roots` counts every discussion in one pass,
and the resident agent already watches note comment threads. **A reply is a note** — the *safe* side
of the fork `MASTERPLAN.md:110` names, where "a message is a bullet inside a note" would specify
block granularity and *"voids this plan"*. So this ruling adds a **surface**, and touches no atom.

**Measured, on the owner's own vault: 225 notes, of which 176 are messages and 31 are feed-eligible.**
`recent()` hydrates 207 to return 31, because a message *is* a `Kind::Note` and `notes_base()`'s
exclusions cannot push down. **Making replies frictionless makes the feed's own query worse, forever,
and the ratio only grows.** That is the fact this design is built around, and the reason the perf
budget comes before the composer rather than after it.

### Three refusals, each with the reason, so they are not re-proposed

**1. No editing a note from a feed row.** The single-editor rule — *"two editors open on one note race
each other through `update_body`"* — is implemented as `matchesTarget` over **panes**. A row is not a
pane, so a row editor bypasses it *silently*; and under `single` every pane stays **mounted and
merely `display:none`**, so the second editor is not just open, it is invisible. Worse, a row holds
`preview` (140 chars of line one), not `body` — so it has no honest `base`, and `update_body`'s own
contract says **an empty `base` opts out** of the lost-update guard. An inline editor built from what
a row already has would ship with that guard disarmed by construction.

**The rule instead, which costs no new invariant: _a note has at most one editor, and the editor is a
pane._** A row's *Edit* calls `openNoteInPane`. **Posting a message stays inline, because an insert
races nothing** — the same structural argument the agent seam already makes by exposing `reply` and
deliberately not `update_body`.

**2. No Markdown-rendered bodies in the feed.** `upgradeAsset` sets `img.src` from `assetUrl(ref)`,
whose `kind` **defaults to `'full'`** — the full-blob path the 2026-08-30 feed decision stepped off
by adding `?kind=thumb`. Rendering bodies per row hands it straight back, at N images per screen,
against an unhandled and un-injectable `onRenderProcessGone`; `loading="lazy"` does not save it,
because everything scrolled *past* is fetched and decoded at camera resolution. `renderInto` also
fires a lock-taking `asset_status` per reference and recurses into embeds three deep.

**3. No double-tap.** It collides with `StatusChip`'s tap-to-rotate (the smallest target in the card,
and a mis-hit writes a file and commits) and with its long-press picker; it contradicts the app's one
written gesture rule — **double-click means edit**, in the manual, in Help, and pinned by a test; and
it taxes the now-primary tap by the ~300 ms discrimination window on the most-scrolled surface of the
battery device. The argument that ends it: **the keyboard has no double-anything.** Two destinations
need two focusable targets regardless, and once those exist the gesture is a hidden second way to
reach what a button already does.

**So the post is restructured instead:** an `<article>`, with **image and title as one button that
opens the note** and the body toggling the thread. This also fixes a live defect — today's
`role="button"` makes a post a *leaf* in the accessibility tree, so the `StatusChip` inside it is
already unreachable to assistive technology.

### What is built

**An `excerpt`, char-capped, beside `preview` — never a fraction.** "Half the note" as a *fraction*
removes the only bound on an unbounded field inside the app's one unbounded payload: a whiteboard
body is Excalidraw JSON up to ~2.8 MB, so half of one is a 1.4 MB array element. A char cap keeps the
safety property, and behind a gradient a reader cannot tell 600 characters from "half" anyway. It is
filled **only in `recent()`**, where the body is already in memory — no extra I/O, no new query — and
`preview` is left alone, because ~10 other consumers clamp it to one or two lines.

**Counts from `thread_roots`, threads on tap, one at a time.** `thread()` has no `Kind` conjunct —
deliberately, and correctly — so it cannot push down and reads every row, including a PDF's extracted
text. Thirty of those is ~5 s frozen behind one mutex. `thread_roots` already returns every count in
one pass, so a post says "3 replies" for **one query per refresh**, and the expensive call happens
once, when you tap.

**The feed sorts by `created`, and edits are shown rather than reordered** — closing the open question
the 2026-08-30 timeline entry left. `updated` moves on every 500 ms autosave, so a feed you can type
into would reshuffle the row under your cursor. The byline already carries `EditedBy` from `activity`,
a git read-model immune to a collaborator's clock, so resurfacing is a *label*, not a sort key.

**Vocabulary: `comment` is the English, `message` is the type.** There is currently zero `comment`
noun in the code; adding one would be a fourth name for one object (`Kind::Note` the file, *message*
the class, *thread* the relation). A button may say "Comment".

**Stated rather than discovered:** the feed as primary surface discloses **none** of `notes_base()`'s
three exclusions — assets, messages, proposals — one day after *"a view says what it leaves out"* was
re-affirmed as more load-bearing than before. And cheap comments feed **search**, which deliberately
sees messages: the volume disease this project diagnoses in its competitors. Neither is fixed here;
both are now on the record rather than in nobody's head.

**Reversal condition.** If the excerpt ships and the feed's payload measurably degrades the phone
(the gate: peak-PSS delta under 150 MB for one refresh at 10k notes), the cap comes down before
anything else is added.

## 2026-09-01 — an overlay is bounded by the *visible* viewport, and it has exactly one scroll surface `#ui` `#track-m`

**The report was one panel; the defect was a class.** *"When I press backup options I cannot scroll
and see all the options, in both android and linux (probably valid for all os)."* The owner's own
reading — *probably valid for all os* — is the right one, and it is not a phone quirk. Every dialog
in this app is `position: fixed` inside `.app`, which is `height: 100dvh; overflow: hidden`. **The
document never scrolls.** So an overlay that outgrows the screen has no fallback whatsoever: no page
scroll, no ancestor scrollport, nothing. `BackupPanel`'s `.panel` had **no `max-height` and no
`overflow` at all**, and what fell off the bottom was its Close button and the primary *Back up*
button — the whole point of the panel.

**Four more copies of the same shape, and each was wrong in its own way.** The overlay had been
written five times by hand, and the copies had drifted to `12vh / 8vh / 12vh / 90vh / 80vh`:

- `SettingsPanel` and `SkippedPanel` capped in **`vh`**, which is *the tallest the viewport ever
  gets*. With the URL bar out or the keyboard up over a token field, an `82vh` panel is taller than
  what you can see. `App.svelte`'s `.app` has carried a comment saying exactly this since the
  arrangement work; the panels never read it.
- The shared `.sheet` (Add a paper, New vault) capped at `90vh` and accounted for no inset.
- **`HelpPanel` had no overlay CSS whatever.** Its markup says `class="sheet"` /
  `class="sheet-backdrop"`, copied from `App.svelte` when Help was extracted into its own component
  — but Svelte *scopes* those rules to App's elements, and `.sheet > :global(*)` globalises the
  *child* selector, not `.sheet`. Confirmed in the built bundle: `.sheet` is emitted only as
  `.sheet.svelte-1n46o8q` (App's hash) while `HelpPanel-*.css` carries `.svelte-k1g25y` and no
  `.sheet` at all. So Help had no backdrop, no `position: fixed`, and rendered as an in-flow block
  inside an `overflow: hidden` grid. **Nothing reported this** — not `svelte-check`, which warns
  about an unused selector and has no opinion about a class with no rule; not any test.

**The geometry: the overlay owns the arithmetic, the panel says `100%`.** The overlay is a
`border-box`, `height: 100dvh` layer that spends the insets as padding; the panel is
`max-height: 100%; overflow-y: auto`. The rejected alternative — a `--overlay-max-h` calc
subtracting the insets — was written first and was already wrong at one call site: `.sheet` padded
by `--safe-top`/`--safe-bottom` *and* capped with a token that subtracted them again, costing the
paper dialog ~99 px of its own room on the phone. **A calc that restates a padding is a calc that
can fall out of step with it.** Let layout do the sum.

**One scroll surface, on the panel — not a pinned head over a scrolling body.** `UnrecordedPanel`
already settled this from a device observation on 2026-07-31: an inner `max-height: 40vh;
overflow: auto` meant a finger landing on the inner region scrolled the inner region while the panel
stayed put, and *"the owner scrolled twice and saw the same header both times"*. Two scroll surfaces
on a touch screen is how content becomes unreachable in a second way. If a long panel's head
scrolling away is ever the complaint, the answer is `position: sticky` on `.panel-head` — still one
surface — and never a second scrollport.

**`--bar-floor: 3.25rem`, because the number was being copied by hand.** `known-issues.md` says a
phone surface must not invent its own inset number, and the first draft of *this* fix invented it
four more times. It is now one token in `app.css`; `UnrecordedPanel` reads it instead of its local
copy, and the device's real inset still wins through `max()`. The floor stays for the reason it
always has: `env(safe-area-inset-*)` is 0 in the WebView until `MainActivity`'s bridge fires, and a
floor is only ever wrong by being generous.

**Two CI greps, because no test can see any of this.** jsdom computes no layout, and component tests
never load `app.css` — nothing in `ui/src` can assert a height, an overflow, or that a pixel is on
screen. So the invariant is a `ci/checks.sh` guard in the house style: *no covering surface sizes
itself in `vh`* (with `.read .asset-pdf` named as the one exception — an iframe inside an
already-scrolling pane), and *every overlay panel declares a scrollport*. Both were proved to fail
against the pre-fix CSS before being trusted. The second is a file-level grep and says so in its own
comment: it catches the defect that shipped (none at all), not a misplaced one.

**The guards were nearly born blind, and the cause is worth more than the fix.**
`SkippedPanel.svelte` held a **literal NUL byte** in a template literal (`` `${s.vault}\x00${s.name}` ``).
That made the file `data` rather than text — and **grep skips a binary file in silence.** The new
guard passed over it clean while it carried a `76vh`, and so, for as long as it has existed, has
every other `ci/checks.sh` sweep over `ui/src`. It is now `\0`, the escape, with the same value; and
every sweep here passes `-a`. A guard that is disarmed by a byte nobody can see is worse than no
guard, for the same reason the `fm-query` comment-filter bug was.

**Deliberately not done: consolidating the five copies into one global `.overlay` class.** It is the
right end state — the duplication *is* the root cause, and a token fixes today's five values without
stopping a sixth panel from copying the block wrong. But it means renaming classes and rewriting
markup in six components to fix a CSS bug the owner reported as "I cannot scroll", and that trade is
the owner's to make, not one to take while they are waiting on the fix. Recorded here so the next
session does not have to rediscover the argument.

**Reversal condition.** If a sixth overlay ships with its own hand-written copy of this block — the
guards will let it, since they check units and scrollports, not duplication — the class consolidation
has earned itself and should be done then.

## 2026-09-02 — a backup surface names what its tier carries, and the restic tier carries the notes too `#vault` `#git` `#ui`

**The mechanism was right; every description of it had drifted.** Nothing here changes what backup
*does* — the review that produced this entry found the two tiers well shaped and, unusually for
this repo, genuinely covered: six tests drive a **real** `restic` binary
(`fm-core/tests/backup.rs`, skipped outside pixi, which is why they only truly run under
`pixi run ci`), and `push_squashed` is pinned on both backends by a differential test asserting
byte-identical remote logs. What was wrong was the sentences. Three of them were false.

**1. The restic tier is not "media".** `backup()` snapshots the notes directory **and** `blobs/`
(`fm-core/src/backup.rs:74-104`) — so a snapshot contains the notes, and a restore returns a
working vault rather than a pile of images. The panel called it "media" in every string, and the
manual said it took *"the whole vault — blobs and all"*. Both wrong, in opposite directions: one
understated the coverage, the other overstated it. **The fix is to say so, not to widen the
snapshot.** The narrow scope is deliberate and tested — `a_projects_own_files_are_not_snapshotted`
exists because a vault may also be a project directory, and sweeping in `.env` and `src/` would
make the app a backup tool for files nobody offered it. Recorded here because the tempting fix is
the wrong one and will be proposed again.

**What that scope excludes is larger than it looks, and the manual now says it.** `views/`,
`themes/` and `manifest.json` all live at the **vault root** (`fm-app/src/views.rs:579`,
`themes.rs:28`, `fm-core/src/manifest.rs:46`), so **none of them is in a snapshot**. The two tiers
are not each other's copy and do not between them cover the vault folder. That is defensible — git
carries exactly the things git should carry — but it was nowhere stated, and a user restoring from
restic alone would silently lose every saved view and their theme.

**2. The git tier's scope is per vault, and the panel could not read it.** `git_assets_max`
(2026-07-20, above) lets blobs at or under a limit ride with the notes. The backup panel said
*"Media in `blobs/` is not included"* flatly — false for any vault with a limit, and false in the
dangerous direction: it told someone their attachments stayed home while they were entering
permanent shared history. `VaultStatus` gains `git_assets_max` so the promise line can state what
the tier carries. One descriptor read per vault, beside a network `ls-remote` that already
dominates `backup_status`.

**3. The panel named a mechanism it had itself replaced.** Two strings still said
`RESTIC_PASSWORD isn't set`, and the disabled-checkbox line still sent people to *"your vault
list"* and told them to *restart* — for a repo the field directly above it sets, and a password
that takes effect the moment it is saved (both since 2026-08-29). A surface that tells you to go
and edit a file the surface exists to replace is worse than one that says nothing.

**The deny list gains the restic writers.** `set_restic_repo`, `set_restic_password` and
`clear_restic_password` were reachable from a paired device while `/api/backup` and `/api/config`
were already denied. `set_restic_repo` decides *where a vault's notes and attachments are sent*
and takes an arbitrary `s3:`/`sftp:` string; `clear_restic_password` deletes the only key to every
repository on the machine, which restic cannot recover. Configuring a tier you are not allowed to
run was never a coherent capability to leave a guest.

**git-lfs: asked, and not taken — the arithmetic, so it is not re-derived.** The owner asked
whether full backups through git might want LFS. There is none in the tree: no `filter=lfs` in
`.gitattributes`, nothing in `pixi.toml`, and the only mention anywhere is the two-tier decision
above citing git-annex/git-LFS as prior art it *declined*. Three things stand against adopting it
now, and none is about effort. LFS needs **server-side support**, so it would make "your notes
repo is an ordinary git repo you own" conditional on the host — the property the two-tier split
exists to protect. It is **another binary on PATH**, which the toolchain rule
(`Command::new` with no indirection, `#toolchain`) already names as the unsolved half of the
installer. And it does not remove the permanence problem it appears to solve: an LFS pointer is
still a commit, and the bytes still live somewhere forever. `git_assets_max` already delivers what
the question was really after — attachments travelling with notes — bounded by a size the vault
chooses. **Reversal condition:** if someone needs full-fidelity media *history* (not a current
copy), restic cannot give it and this argument should be reopened.

**Deliberately not done here.** A "last backed up at" line — `backup::latest()` exists in core and
no dispatch command exposes it, so the single most useful fact about a backup is unavailable to
the UI. Letting a restic-only vault run without a git remote; the panel now at least explains why
its own button is disabled instead of leaving it silently dead. Both are in `outstanding.md`.

## 2026-09-02 — attachments in git have a ceiling, because there is no LFS `#vault` `#git`

**Why:** the owner asked whether attachments can be tracked by git and how git manages LFS here.
They can — `git_assets_max` has done it since 2026-07-20 — and **LFS is not used at all**, which
is what makes the second half of the question load-bearing. Without LFS every attachment under a
vault's limit is a whole blob in permanent history: paid for again by every clone, unremovable
without rewriting history other people have pulled, and above roughly 100 MB **rejected by the
host outright — after the commit is already made.** The owner's conclusion, and this decision:
*"if we do not have lfs, we should put a reasonable upper limit."*

**This narrows the 2026-07-20 ruling rather than reversing it** (*Which attachments travel is a
per-vault size limit, in `vault.json`*, above). That entry stands and is not edited: the mechanism
is untouched — still a size, still per vault, still in the vault's own file, still off by default.
Only the range is now bounded. The first design considered here was the opposite — an explicit
"All attachments" option, since today "send everything" means typing `10GB` and hoping — and it
was dropped in favour of a bound. An unbounded option with no LFS behind it is an invitation to a
push that cannot succeed.

**Three bands, and the middle one is the point.** At or under 50 MB, accepted silently. Between 50
and 100 MB it *works*, and Settings names what it costs — this band must not be refused, because
those files do push, and an app that declines what would have worked is deciding for the user
rather than informing them. Above 100 MB, refused with the reason.

**100 MB decimal, deliberately just inside GitHub's 100 MiB** (104,857,600). A file this app
accepts should never be one the host rejects over a rounding difference, so the ceiling sits
inside the wall rather than on it.

**Enforced twice, and both are needed.** `Descriptor::set_git_assets_max` refuses to *write* a
limit above the ceiling — in the core setter, so `fm-cli` and every future caller inherit it
rather than each re-deriving the bound. And `blobs_within` clamps through
`effective_git_assets_max` at the point of staging, because a `vault.json` can ask for more than
this app would ever write: hand-edited, carried from another machine, or written by a version that
had no ceiling. The setter alone would leave exactly those vaults staging a blob no remote will
take. A clamp that is not surfaced is the dishonest half, so Settings states plainly when a vault
asks for more than will be sent.

**`Descriptor::read` is deliberately unchanged.** An over-ceiling value still parses. It is a
*valid* size, and a value that was legal when written must not make a vault unopenable — which is
why this is a clamp and not the hard parse error an unparseable value still gets.

**No escape hatch, and that is a real cost.** A self-hosted Gitea, GitLab or plain SSH remote may
accept far more, and there is no way to tell the app so. One documented constant is clearer than a
second key in the file format serving one audience, and the restic tier already carries
attachments of any size with none of git's permanence. **Reversal condition:** if someone is
actually running a self-hosted remote and wants 500 MB attachments versioned, this is the entry to
reopen — and the answer is more likely a per-vault opt-out than raising the number for everyone.

**The constant is written twice and guarded.** Rust owns it; `ui/src/lib/size.ts` holds a copy so
the form can warn before the backend refuses, and `ci/checks.sh` fails when the two disagree —
proved against a deliberate mismatch before being trusted. Two constants that must agree and no
guard is precisely how they stop agreeing.

## 2026-09-02 — the archive carries its own update path, and it copies one way `#toolchain` `#vault`

**Why:** the owner asked whether a user can update and keep their vaults, *"because we are
shipping a vault with fm — if the user does not check it, it might delete it in the update
process."* The audit found no updater, no version check, no migration, and **the word "update" in
no user-facing document at all** — not `README-release.txt`, not the manual, not the release
notes. The safe procedure existed and was written down nowhere.

**The risk is not what it first looks like.** Each release unpacks into its own versioned folder
(`formicaria-${VERSION}-${target}`), so an extraction cannot overwrite the previous one — the
archive deletes nothing. The danger is the *person*: the vault lives inside the folder, so a new
download opens on a pristine "Start here" note and someone's work appears to be gone. From there
they either believe the update destroyed it or tidy away the "old" folder, and **that** is the
deletion. `README-release.txt` teaches folder-as-app (*"copy this whole folder to a USB stick"*),
which makes "new folder replaces old folder" the natural instinct. The portability that makes the
app what it is, is also what sets the trap.

**Consequence: one script per platform in the archive, and the direction is the decision.** It runs
in the **new** folder and copies the old vault **in** — never the new program out into the old
folder. That way the old folder is never written to and never deleted, so it stays a complete
backup until the user chooses otherwise, and the worst outcome of a mistake is a folder to throw
away rather than a notebook that cannot be recovered. It never deletes anything.

**`vaults.json` is deliberately not copied, and this is the sharp edge.** `vaults::save` writes
each vault's **absolute** path — its own doc comment says so: *"It does freeze that path into
config."* Bring that file across and the new formicaria quietly keeps writing into the folder the
user is about to delete. Left behind, the launcher's `FM_VAULT` points at the vault beside it and
the file is rewritten on first start. The cost is that vaults kept *outside* the folder lose their
registration, so the script reads the old file and **names them** rather than letting the absence
be discovered later. `index.sqlite` is skipped for the reason `acquire::naturalise` already
removes it: per-machine, rebuilt on open, and a stale index that looks current is worse than none.

**Refusing on "already used", not on a note count.** Three signs — `vaults.json`, `vault/.git`,
`vault/index.sqlite`, plus the `update.log` the script writes — distinguish a fresh unpack from a
live notebook. A count cannot: someone who deleted the welcome note has zero notes and a real
vault. Copying a second notebook onto a live one would mix two sets of files with no way to
separate them afterwards.

**No heuristic warning inside the app, and that was a choice.** The tempting feature is "if this
vault holds only the welcome note and a sibling folder has a real one, say so". It was declined:
the same shape is produced by a deliberate second copy, a portable stick, and a fresh install
beside an archived one — and a warning that cries wolf on legitimate uses trains people to dismiss
the one that matters. **What shipped instead is the missing fact**: the app now reports its own
version (Settings → This machine), because until now two unpacked folders were indistinguishable
from the inside. `option_env!("FM_VERSION")`, baked in by the release workflow and `dev` otherwise —
the crates are all `0.0.0`, so there was no version to report even if something had asked. No
comparison, no update check, nothing fetched.

**Guarded, because the sheet promising the script and the workflow staging it are separate files.**
`ci/checks.sh`'s release-sheet check gains the update script's base name. A stale README costs a
user the app; this one would cost them their notes at exactly the moment they believe those notes
are gone.

**Accepted cost:** the Windows `.bat` has been executed by nobody, exactly as `outstanding.md`
already records for `formicaria.vbs`. The `.sh` and `.command` were rehearsed end to end against a
real git-backed vault, including paths containing spaces, an unrelated working directory, two
candidate folders, a bad argument, and a second run.

## 2026-09-02 — the phone answers the same status shape as the desktop, and AI availability is stated per OS `#agent` `#track-m`

**Why:** asked whether using the AI tools is straightforward and well documented on every supported
OS. It is not, and the audit turned up a defect the docs had hidden from themselves.

**The bug, and the seam it came through.** `agent_status` on Android answered `{enabled,
transcribe}` while fm-serve answers `{enabled, transcribe, installed, why, transcribe_available}`.
`SettingsPanel.svelte` reads `st.installed` — and the phone renders **the same component**, because
the mobile shell points its webview at the same `ui/dist`. `undefined` is falsy, so the assistant
row printed *"not available"* with an empty reason and no switch **on the one platform where the
entire stack ships inside the APK**. The rule this earns: *a shell that renders the shared panel
answers the shared shape*. Not a comment — a `ci/checks.sh` grep, because this class of defect
compiles, ships, and is invisible to every test we have (jsdom renders neither shell), and it was
found by reading rather than by anything failing.

**Verified on the device, which is the only place it could be.** Installed as v0.3.1 (versionCode
3001) on the owner's phone: the Settings row renders the assistant toggle *and* the audio
transcription sub-row, which only appears when `installed` **and** `enabled` are both true — the
exact pair the short shape made unreadable. jsdom renders neither shell, so no test in this repo
could have shown this.

**Answered by checking, not by asserting.** `installed` asks whether `libllama-server.so` is
actually in the extracted native-library directory, and `transcribe_available` whether
`libwhisper-server.so` is. `ci/android-stage-runtime.sh` is what puts them there, so a build made
without it now says so, exactly as the desktop does when `agents/` is absent. The weights are
deliberately *not* part of the question: the phone fetches them on first enable, so runtime
presence is the honest capability there.

**Availability is now stated where the reader already is.** `assistant.md` was candid and alone —
the three per-OS setup chapters said nothing about AI, `README-release.txt`'s capability list
omitted it while claiming *"There is nothing else to install. No runtime"*, and `README.md`'s
recipe carried no platform caveat at all. Each per-OS setup chapter now states that OS's answer,
which is what `ci/docs.sh`'s per-OS assembly exists for; `assistant.md` opens with the whole matrix
rather than two paragraphs of disclaimer.

**Stated as a fact with its cause, and no date.** macOS and Windows refuse because the preflight
that decides whether a machine has room for a model reads `/proc` and fails closed elsewhere — so
the app declines to run a model it cannot watch. The docs say that, and say nothing about when it
might change: the honest fix is a monitor for those systems (`GlobalMemoryStatusEx` /
`host_statistics64`), never a relaxed gate, and promising a date for work nobody has scheduled is
how a manual starts lying slowly.

**A correction to our own record.** `outstanding.md` §2.6b said the assistant is *"desktop-excluded
by one Cargo feature"* and that "the Cargo feature is still off there". The `agent` feature is **on**
in the shipped desktop binary — the API routes exist and answer. What is off is
`fm-agent-run/download`, and `pixi run build` never builds that crate at all, so the desktop has no
in-app model fetch and the archive carries no agent stack. Same conclusion, different cause, and the
difference matters to whoever picks the work up.

**Not done, deliberately:** delivering the assistant to the desktop archive, and macOS/Windows
support. Both are real and both stay in `outstanding.md` §2.6b rather than being half-started here.

## 2026-09-02 — the assistant provisions itself, so a downloaded copy can run it `#agent` `#toolchain`

**Why:** the owner asked that *all* users be able to use the AI features. Nobody who downloaded
formicaria could, on any OS — the archive carried no agent stack, `pixi run build` never built
`fm-agent-run`, and the launch path shelled out to `bash agents/start-agent.sh` plus a Python search
proxy. Two independent blockers; this entry is the first, **delivery**. The second, macOS and
Windows, is untouched here and stays gated.

**The archive gains a supervisor, not a runtime.** `agent-serve` and `models.toml` ship beside
`fm-serve`; the model and the model runtime are **fetched on first enable**. Shipping them was never
an option — they are per-platform and gigabytes, and one archive cannot carry four platforms' worth
of binary. This extends the fetched-artifact exception the 2026-08-30 `/transcribe` ruling already
established rather than opening a new one.

**No shell anywhere in the launch.** `fm-serve` spawns `agent-serve` from beside its own binary —
`merge_command`'s idiom, never a bare name hoping `PATH` will answer — and makes the whisper
decision itself from the predicate it already had. `search-proxy.py` is replaced by `--web-direct`,
the in-process HTTPS search the phone has always used: **DuckDuckGo goes** (it needs HTML scraping),
wikipedia, GitHub and arXiv remain, and that loss is stated in the manual rather than discovered.
The `bash`/`python3` capability checks are deleted, not fixed: they described a launch that no
longer happens, and `on_path`'s Windows arm could not check `.exe`/`PATHEXT` anyway.

**Every runtime archive carries a checksum, and one without is dropped.** A model is pinned by a
Hugging Face commit *and* a hash; a runtime is a URL to an executable, so the hash is the only thing
between a redirect and running a stranger's binary. `Manifest::parse` therefore pairs
`runtime_url_<platform>` with `runtime_sha256_<platform>` and **discards a URL whose checksum is
missing** — absent is a capability the caller reports, unverified is a binary already running. The
three Linux archives were downloaded and hashed to pin them; **macOS and Windows have no entry yet**,
deliberately, because the gate still refuses those platforms and an unverified entry parked here
would be a promise the file cannot keep.

> **SUPERSEDED IN PART, THE SAME DAY**, by *The assistant asks the machine, not a list of operating
> systems* below: macOS and Windows archives are pinned and hashed, `zip` became a dependency when a
> Windows archive existed to need it, and the whisper runtime followed on 2026-09-02. What stands,
> and is the reason this paragraph is kept rather than rewritten, is the **rule**: a URL whose
> checksum is missing is dropped, never fetched. What changed is only which platforms had passed
> it yet.

**Flat suffixed keys, not a `[runtime.…]` table.** The hand parser has no concept of a named table:
a `[…]` line that is not `[[models]]` is skipped and its keys are then read as *top-level* ones,
silently. A shape the parser cannot see is worse than an ugly one it can.

**Verified before it is unpacked, and the notice comes with it.** Unpacking an unverified archive
has already written attacker-chosen paths by the time you notice, so the checksum is checked first.
Extraction keeps the binary, the libraries beside it, and anything named `LICENSE` — llama.cpp and
whisper.cpp are MIT and the notice must travel; today it does so only by the accident of
`agents/fetch.sh` copying a whole tarball. `ci/third-party.sh` gains a section for them too, because
`cargo tree` cannot see software we put on a user's disk but do not link.

**Tools live in `<config>/formicaria/tools`, not in the app folder.** `vaults::config_dir` already
resolves it correctly on all three OSs and already holds `vaults.json` and `agent.json`. The
portable alternative (`outstanding.md` §2.6b's `program/tools/`) loses on two counts: the folder is
not writable when someone unpacks to `/opt` or `C:\Program Files`, and every update would
re-download gigabytes into the new folder. **Accepted cost:** a model does not travel on a USB stick
with the app folder. The vault still does, which is the promise that was actually made.

**The first enable is a question, not a switch.** It offers the catalogue with each model's size and
licence — both now machine-readable fields, where they were prose in a `note` no parser could read —
then downloads with progress in **bytes** and a cancel. A percentage would have to be invented when
the server sends no length. This is deliberately *unlike* the phone, whose toggle starts a 1.4 GB
download in silence; that is the failure being avoided, not the model being copied. What **is**
copied from the phone is the generation counter: a worker re-reads it before every step and abandons
the work when it no longer matches, which is how a cancel lands mid-download before any stop flag
exists.

**Deferred, and specified rather than half-started:** macOS and Windows need
`GlobalMemoryStatusEx` / `host_statistics64` behind the existing `ResourceMonitor` trait, and
`die_with_supervisor` is a `#[cfg(not(unix))]` **no-op** — the "model dies with its supervisor"
guarantee is silently absent off Unix, which is an orphaned `llama-server` holding gigabytes.
Note also that `sample()` returning `Err` inside the live watchdog loop *kills a running model*, so
a monitor that works intermittently is worse than none. `zip` is not a dependency yet for the same
reason: no Windows archive is pinned, and an extractor for a format nothing fetches would be an
unused dependency in a shipped binary.

## 2026-09-02 — the assistant asks the machine, not a list of operating systems `#agent` `#toolchain`

**Decision.** macOS and Windows can run the study assistant. The OS allow-list in `unavailable()`
is replaced by **a real reading**: take one `SystemMonitor::sample()`, and the platform is supported
exactly when the answer is usable.

**Why a probe and not a longer list.** A hardcoded list is a claim about our code; a sample is a
fact about the machine in front of the user. It is also the only honest way to ship platform code
from a machine that cannot compile it — this repo has no Windows or macOS `rust-std`, so the two new
arms were written, reviewed and shipped **without ever being type-checked**. A probe degrades
correctly if either is wrong: the sample fails, and the row prints a reason.

**The dangerous failure is not a crash, it is a believable wrong number.** A misread struct field
does not usually error — it returns `0` or something astronomical. Zero refuses every launch (safe);
a huge value **admits a model onto a machine with no room**, which is the exact harm preflight
exists to prevent. So the arithmetic and the bounds live in `plausible()`, which is `cfg`-free,
compiled everywhere and unit-tested here: anything at or above 1 PiB, or exactly zero, is refused
with the number named. The unverifiable syscall arms do one call each and hand their result to
tested code.

**What each platform reads.** Windows: `GlobalMemoryStatusEx`, hand-declared rather than pulling in
`windows-sys` for one call — the same stance that keeps a hand-rolled HTTP client in `fm_agent`.
macOS: the mach kernel's VM statistics **through `libc`**, not hand-written bindings, because the
struct is large and a wrong offset is precisely the silent-wrong-number failure above.
`free + inactive + purgeable`, because macOS keeps almost nothing strictly "free" and counting only
`free_count` would refuse launches that are perfectly fine; speculative pages are excluded, since
counting them tips the estimate optimistic and optimism is the direction that hurts. Neither
reports a load average — Windows has none — which costs nothing: `Limits::resident()` already sets
the load ceiling to infinity, and Android has been memory-only since it shipped.

**A build break found on the way, and worth more than the feature.** `die_with_supervisor` was
`#[cfg(unix)]` and calls `libc::prctl` — which is **Linux-only**; `libc` does not define it on
macOS. Nothing on macOS had ever compiled `fm-agent`, because `fm-serve` did not depend on it until
the assistant shipped. **The change that lets a downloaded copy run the assistant is the same change
that would have broken the macOS release build.** Now `linux`/`android`.

**Windows gains a stronger guarantee than Linux has; macOS gains none.** A Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` kills the model whenever the last handle closes, however this
process dies — better than `PDEATHSIG`, which only fires for a direct parent. It is best-effort and
silent by construction: every failure path leaves exactly the behaviour that shipped before, so a
wrong guess about those APIs cannot stop the assistant starting. **macOS has no equivalent** and
gets none here: there is no `PDEATHSIG`, and a `kqueue`/`EVFILT_PROC` watcher is more machinery than
this has earned on a platform that is one day old to us. Every ordinary path still kills the child;
what is lost is the case where the supervisor is itself killed. Recorded in `known-issues.md` rather
than hidden behind a no-op that reads like it does something.

**Every runtime is pinned and hashed.** macOS ships a `.tar.gz` (the existing extractor), Windows a
`.zip` — which is what finally justified the `zip` dependency, declined a few hours earlier when no
Windows archive was pinned. All five archives were downloaded and hashed here; the zip extractor is
a deliberate mirror of the tar one, same keep-list and same escape refusal, because two formats with
one policy is the only version of this that stays safe.

**Accepted, and the thing to fix next: none of this has run on a real macOS or Windows machine.**
It is not compiled here either. The design makes a mistake *safe* — a bad reading refuses — but it
does not make it *unlikely*, and "refuses on every launch" is a plausible outcome of a wrong
`libc::vm_statistics64` field on an OS nobody has tried. This is the same standing gap
`outstanding.md` records for the Windows launcher, and it is why the setup chapters say the platform
is new rather than implying it is proven.

## 2026-09-02 — a platform arm nobody can compile gets a tested core and a cross-check `#toolchain` `#agent`

**Why:** the macOS and Windows monitors shipped hours earlier having never been through a compiler,
because this checkout has no `rust-std` for either target. That is not a state to leave standing.

**A `cross` pixi environment, isolated exactly as `android` is.** `rust-std-x86_64-pc-windows-msvc`
and `rust-std-aarch64-apple-darwin` at 1.97.x, and `pixi run -e cross check-cross`. Out of the
default environment for the reason the android block already states — `pixi run ci` must stay green
for someone who has downloaded none of it — and here for a sharper one as well: `pixi.lock` shows
the android environment resolving a *different* rustc (1.97.1) from the default (1.97.0), so
cross-target packages in the default environment could quietly move the compiler that builds
releases.

**It checks `fm-agent` only, and that is the whole of the per-OS code.** The wider crates cannot be
type-checked here at all: `fm-agent-run --features download` pulls `ureq` → `rustls` → `ring`, whose
build script compiles C and wants a real MSVC toolchain (`failed to find tool "lib.exe"`). That is a
cross-*build* environment, which `cross.yml` already provides on real runners. This task buys the
fast half — the errors a compiler finds by reading — and says so rather than implying more.

**It paid for itself on the first run**, which is the argument for it: a `std::mem::forget` on a raw
pointer that did nothing (a pointer is `Copy` and has no destructor — it read like a safeguard and
was noise), and a deprecated `libc::mach_host_self`. Kept the deprecated call deliberately — taking
the `mach2` crate for one function is the trade this project declines elsewhere, and if `libc` ever
removes it the cross-check fails on the spot, which is what makes keeping it defensible. Then
proved the check catches the real thing: reintroducing `#[cfg(unix)]` on the `prctl` arm fails the
macOS check with `cannot find function prctl in crate libc` — the exact break that would have taken
down the macOS release build.

**The standing rule this generalises.** A platform arm that cannot be compiled here is written as a
thin syscall wrapper over a `cfg`-free core that *is* compiled and tested — `plausible()` under the
monitors, `parse_proc` before it, `lib_dir_from_maps` before that. The unverifiable part stays small
enough to read; the part that decides anything is tested.

**Two guards that had to be proved, and one that failed the proof.** The archive-staging check —
"what `fm-serve` looks for beside its binary, the release must stage" — passed while the staging was
deleted, because the *comment above the staging line* still said `agent-serve`. A guard satisfied by
prose about the thing reports on its own documentation. It now strips comments before searching, and
was re-proved in both directions. `pixi run check-pins` (ignored by default, ~110 MB) re-fetches
every pinned runtime and confirms it still hashes to what the catalogue records — an upstream
re-upload would otherwise reach a user as an unexplained checksum failure on first enable.

## 2026-09-02 — advice that cannot succeed is worse than none, and a feature that takes 3 GB can give it back `#agent` `#ui`

**Why:** an audit of the manual against what shipped found the docs describing a different product —
including two paragraphs that were **false in the built HTML users have**, one of them written by
this session hours earlier. The audit's own finding is the durable lesson: *the same day's work
falsified the same day's documentation twice*, because the docs were corrected before the feature
landed rather than with it.

**The dead end, which is the part worth keeping.** `preflight::admit` refuses a model the machine
has no room for — correctly — but it refuses **after** the switch is on, in a separate process,
into stderr. So Settings read "On" while the discussion said *"isn't running… Turn the assistant on
in Settings"*: advice the user has already taken, and cannot take again. The fix is not to surface
the refusal (the capability row deliberately does not consult `admit`, `decisions.md`'s 2026-08-29
ruling, because a few browser tabs would then report the assistant "not installed"); it is to
**stop giving the wrong advice**. When a mention goes unanswered the panel now asks whether the
assistant is enabled, once, and says either "turn it on" or "it is on but did not start — most often
memory, close some applications or choose the smaller model". Same two states, two different
sentences, and neither is a loop.

**Removal, because a feature that spends 3.3 GB of someone's disk must be able to hand it back.**
There was no way to reclaim it in the app and no document naming where the files were. The control
states the figure before asking — *"delete 2.5GB"* is a decision a person can make, *"delete the
model"* is a leap of faith — and is two-step, like forgetting a vault, for the same reason: large
and irreversible.

Two refusals are load-bearing. It **will not run against a source checkout**: `agents/` there holds
files a developer put in by hand, and the same `looks_like_a_checkout` tell that makes the dev loop
win also makes this decline. And it is in `REMOTE_DENIED`, beside `set_agent` — a paired tablet is a
guest, and a guest does not free the host's storage.

**Documentation is part of the feature, not after it.** The chapter now leads with the three steps a
downloader takes, prices the download and the image reader separately, names the per-OS path the
files land in, and carries a failure section covering the eight strings the code can actually emit —
written from the code, not from imagination. The assistant is now reachable from `introduction.md`,
`first-note.md` and the in-app Help panel, which on **Android is the only help there is**: the phone
bundles the whole stack and had no text about it outside one Settings row.

**And the docs named three models that do not exist.** `lfm2.5-350m` in the manual, `@lfm2.5-230m`
in the Settings panel's own explanation — the first thing anyone reads about the assistant — and a
third in `agents/README.md`. A reader compared the table to the picker and found no overlap. The
catalogue is now the single source: every example names something `agent_models` will actually
offer.

## 2026-09-02 — the body merge stops shelling out where there is no shell, and the engine is libgit2's `#git` `#track-m`

> **SUPERSEDES** the body-engine half of *`git2` is rejected; git stays a subprocess capability*
> (2026-07-18) and `mobile-design.md`'s ruling 3, which is marked **⛔ refuted** at its own heading:
> it named `git2::merge_file`, which does not exist. The rejection of `diffy` still stands, and so
> does *the desktop keeps shelling out*.

**Decision:** `merge::text_3way` becomes two engines behind one selection rule — the subprocess
where a `git` binary exists, and **libgit2's `git_merge_file` through `libgit2-sys`** where one does
not. The rule is `vcs::native()`, made `pub(crate)` so there is exactly one definition of "this
device has no git". `vcs::commit_all_as` gains the libgit2 arm it never had.

**Why now, and why this was not a feature request.** It was a **shipped bug that froze vaults.**
`text_3way` shelled out unconditionally — no `cfg`, no routing — while `git_native::pull` called it
with `?` for every path libgit2 left conflicted. The phone has no `git` (verified on device,
`known-issues.md` external fact #0). So any pull that had to merge prose died with `could not run
git merge-file`, **mid-merge**, leaving a `MERGE_HEAD` that `commit_all` then refuses to commit past
— and this file already says such a standing `MERGE_HEAD` *"freezes the vault permanently"*. The
2026-07-19 entry recorded the *gap*; what changed is that the native pull shipped on 2026-07-24, so
it stopped being a missing feature and became an erroring pull. **Every concurrent edit to one
note's prose, from two devices, is the trigger** — the ordinary case the feature exists for.

**Why nothing caught it.** Every native-backend test runs where `git` *is* on PATH, so `text_3way`
quietly succeeded through the very binary the phone lacks. `force_native(true)` did not help: it
routes `vcs::` calls, and the body engine was not routed at all. The reproduction had to be a
process where `git` genuinely cannot be found — `crates/fm-core/tests/merge_on_a_device_without_git.rs`,
its own test binary, PATH cleared before `git::available()`'s `OnceLock` can fill.

**Why `libgit2-sys` and not a Rust crate.** It is libgit2's own port of the *same* algorithm, and
`libgit2-sys` is **already an optional `fm-core` dependency** (for the CA-store option), so this
adds no dependency — only a second use of one. That is what makes it cheap now and expensive in
July, when the proposal assumed a wrapper that does not exist. `git2` wraps only
`merge_file_from_index`, which wants index entries and would write into the ODB.

**Consequence** — the swap is gated on bytes, not on reading:
- `tests/merge_differential.rs` gained `the_native_engine_is_byte_identical_to_git_merge_file`:
  400 generated triples, marker sizes 7/12/32, both line endings, graded against **real
  `git merge-file`** rather than against our own wrapper. Proven by deliberate breakage — changing
  one label failed the native gate alone and left the subprocess gate green.
- Both merge suites are wired into `pixi run test-native-git`, because `pixi run ci` builds without
  the feature and would cover the phone's engine with **nothing at all**. That is how the shell-out
  survived: green everywhere, on a path the phone could not take.
- The desktop is unchanged by construction: a `git` binary still wins, so the `fm merge-md` driver
  and the app's own merge stay the same engine, and a collaborator's terminal `git pull` cannot
  disagree with ours.
- `ci/checks.sh` gained *"every hand-written vcs arm reaches both backends"*. The existing routing
  guard walks `route!(…)` entries only, and both `commit_all` and `commit_all_as` are hand-written
  because the macro cannot express a `&[PathBuf]` — so `commit_all_as` was routed nowhere and
  grepped by nothing. Proven by reverting it: the guard names it.
- **The agent's authorship was the second casualty.** `commit_all_as` reached
  `Command::new("git")` on the phone and ENOENTed, swallowed by a `let _ =` in `dispatch.rs`. The
  reply still landed — committed later by the debounced batch, **under the vault's default identity
  instead of the model's**. The premise in `vcs.rs` (*"`native-git` is a differential-test
  feature"*) had simply gone stale: it is the phone's shipped backend.

## 2026-09-02 — iOS: the subprocess consequence is reversed for Android and held for iOS, which ships agent-free `#track-m` `#agent`

> **SUPERSEDES** the consequence clause of *iOS eventually; Android now* (2026-07-19) — *"no design
> may assume an executable subprocess on device, on either platform."* Shipped Android code has
> contradicted that since the on-device agent landed, and **no reversal was ever written**. This is
> that reversal, written late and said plainly rather than left as a silent contradiction.

**Decision:** Android **does** assume an executable subprocess, deliberately — `llama-server` and
`whisper-server` ride in `jniLibs` and are `exec`ed from `nativeLibraryDir`, which is the only
place Android's W^X permits. The 2026-07-19 clause was written to forbid exactly this and was
overtaken without a note. **For iOS the clause stands and is absolute**, so if iOS is ever built it
ships **agent-free (Route C)**: `--no-default-features` already compiles the whole stack out.

**Why not the two alternatives.** *Route A* — link `llama.cpp` in-process via `llama-cpp-2` — keeps
one shared Rust core, which is the property the one-command-surface ruling exists to protect; but
`llama-cpp-sys-2`'s build script has no iOS branch, and the work cannot be done or verified from
this machine. *Route B* — Apple's Foundation Models framework — ships nothing and costs nothing to
download, but is a **second inference backend behind `OpenAiStep`**, iPhone 15 Pro or later, with no
control over the model: two paths to keep answering alike, which is the fork the one-command-surface
ruling forbids. **Neither can be *evaluated* in a Simulator** — no jetsam pressure, no on-device
Metal, no honest tokens/sec — and a Simulator is the only device available (no Mac, no iPhone). A
choice between two engines you cannot measure is a guess.

**Consequence:** iOS is a notebook — notes, git sync, whiteboard — or it is nothing. The assistant
stays desktop + Android, said out loud rather than discovered. Audio would not port whisper.cpp
either: `whisper-rs` is archived with its iOS build issue unresolved, and iOS 26's `SpeechAnalyzer`
is a fourth seam question, not a free win. Full survey: `ios-plan-2026-09-02.md`.

## 2026-09-02 — an iOS build would contradict the project-local-toolchain ruling, and that is the price `#toolchain` `#track-m`

> **Its premise changed 2026-09-03.** This entry reasons from *"shipping is not on the table"* and
> *"a Simulator-only proof"*. The owner has since asked for formicaria on **users' iPhones**, outside
> the App Store — see *iOS is meant to reach users' phones now, and the route is deliberately
> undecided* below. Everything here about the **toolchain** still holds; only the ceiling moved.

> **Scopes** *Non-pixi dependencies are accepted — but they are project-local, never a system
> requirement* (2026-07-19). Not superseded: Android still honours it exactly. This records what
> iOS would cost against it, **before** anyone spends a CI minute.

**Decision:** iOS work, if it happens, is a **stated exception** to the project-local rule, not a
quiet extension of it. Android works because every piece Google ships is a standalone zip that
`ci/android-init.sh` fetches and verifies against a SHA-256 in `android/toolchain.lock` — *"the
checksum, not the URL, is the assertion"*. Apple ships the iOS SDK only inside Xcode, and the Xcode
and Apple SDKs Agreement forbids running it on a non-Apple computer. **There is nothing legal to
pin into a `.ios/`**, and there never will be. The un-pinnable surface moves from a hashed NDK to a
GitHub runner image nobody pins or controls — strictly worse than the position the ruling protects.

> **PARTLY SUPERSEDED (2026-09-04)** — *the repo goes public, and the economics every CI ruling
> rested on invert*. The premise quoted below still holds for "no Mac, no iPhone"; **"repo stays
> private" and "a bounded macOS CI allocation" do not.** On a public repo, standard-runner minutes
> are free, macOS included, so the bound this entry reasons against is gone. What survives it
> unchanged: iOS is an exception to the project-local-toolchain rule rather than a quiet extension
> of it, and no iOS leg may reach `release.yml`.

**Why this is not a veto.** The owner has ruled: no Mac, no iPhone, repo stays private, a bounded
macOS CI allocation spent strategically. Under that, iOS is a **Simulator-only proof** — no signing,
no provisioning profile, no $99/yr, no App Store review, no annual certificate renewal, because all
of those attach to *shipping* and shipping is not on the table. What is bought is an answer to *"is
the port real and what would it cost"*, with a kill criterion written before the money is spent.

**Consequence:** any iOS CI job is `workflow_dispatch`-only and **must not** be added to
`release.yml`, which is the single named exception to the no-remote-CI standing order (2026-07-18)
and fires unattended on every `v*` tag. `gen/apple` is regenerated per run and never committed —
`.gitignore` already forbids the Android equivalent in terms, *"it embeds absolute paths, so
committing it would be committing this machine"* — with an idempotent `ci/ios-inject-*.sh`
re-applying our edits, exactly as `ci/android-inject-service.sh` does.

## 2026-09-02 — `libgit2-sys` is an unconditional dependency, because the merge engine names it `#git` `#toolchain`

> **Amends** *the body merge stops shelling out where there is no shell* (same day, above): the
> engine shipped correct and its dependency declaration did not.

**Decision:** `fm-core` declares `libgit2-sys` in `[dependencies]`, never under a `[target...]`
header, and `ci/checks.sh` fails if that is undone. `openssl-sys` stays target-gated (not Windows,
not iOS) because `add_certs_from_pem` is its only consumer and is gated to match.

**Why:** a crate can only be *named* in Rust if it is a **direct** dependency — reaching it
transitively through `git2` does not make `libgit2_sys::git_merge_file` resolve. `libgit2-sys` had
been target-gated when its only consumer was Android's CA workaround; the new body engine gave it a
consumer on every platform that compiles `native-git`, and the declaration was not moved.

**How it was found, and what that says about the gates.** `ios.yml` rung 1 — the first billed macOS
job — failed with eight `cannot find module or crate libgit2_sys` errors before reaching any C. The
same defect was live on **Windows**, where `fm-serve` enables `native-git`, and it had already been
pushed to `main`. Neither `pixi run ci` nor `cargo check` on Linux can see it: `ci` never builds the
feature, and the excluded targets cannot be type-checked here because their build scripts need MSVC
or Xcode. **Two shipping platforms were broken by a manifest line, and the only thing that could
have caught it was a job that costs money.** Hence the grep, which is free and runs on every push.

**Consequence:** iOS ships libgit2 like every other platform — which is right on its own terms, not
merely convenient: iOS forbids `exec` outright, so the in-process engine is the *only* body merge it
can ever have. The vendored OpenSSL it also drags in stays unused there (SecureTransport), and
shedding it needs an upstream change rather than a manifest trick — see the entry above.

## 2026-09-03 — the iOS diagnostic channel is stderr, not `os_log`, because only one of them can be tested from here `#track-m` `#toolchain`

> **AMENDED the same day** by *the iOS log is a file in the app container* (below). The trigger this
> entry wrote down for itself — "if rung 2 shows an empty pty" — fired on the first run that got far
> enough to test it. stderr reaches nobody on the Simulator; it is kept as a second sink, and the
> channel that is actually read is now a file. The reasoning against `os_log` still stands.

> **Reverses a step of the iOS plan**, not a standing decision: `ios-plan-2026-09-02.md` named
> "the `os_log` backend for `install_logger()`" as rung 2's prerequisite. The prerequisite stands;
> the mechanism does not.

**Decision:** `install_logger`'s iOS arm is a ~15-line `log::Log` implementation writing one
unbuffered line per record to **stderr**, prefixed `formicaria` so it greps the same as Android's
logcat tag. No new crate, no C shim, no FFI. `xcrun simctl launch --console-pty` attaches a pty to
the app's stdout/stderr and prints it, which is what rung 2 will assert against.

**Why not `os_log`, which is the platform-correct answer.** Emitting to it is not a function call:
`os_log_with_type` is a macro that lowers to `_os_log_impl` plus a format descriptor that must land
in the `__TEXT,__os_log` section, which is why every Rust binding for it (`oslog`, `tracing-oslog`)
ships a C shim built by `cc`. That is a dependency **this machine cannot compile**. The mobile shell
does not cross-compile for iOS on Linux — a `cargo check` of it dies in `libdbus-sys` before it
reaches a `--target` flag — and there is no Mac. Worse, getting the section wrong does not fail to
build: it logs `<private>`, a diagnostic channel that lies. Trading a testable channel for an
untestable dependency, in order to reach a reader (`log stream` on a machine the owner does not
have) that nobody will ever be at, is the wrong way round.

**Consequence:** every line of the iOS shell's logger first *executes* inside a billed CI job — so
its entire surface is `log`'s trait and `std::eprintln!`, and the exact code shape was compiled on
Linux in a throwaway crate before being committed. If rung 2 shows an empty pty, the fallback is
`simctl spawn <device> log stream`, not a rewrite. This is Simulator-only reasoning and it is
allowed to be: the owner has no Mac and no iPhone, so a Simulator under CI is the only place this
shell can run. Should a real device ever enter the picture, stderr goes nowhere on one and this
entry must be reversed — that is the trigger, written down now.

## 2026-09-03 — the `rustup` shim is a build tool, so it lives in the tree `#toolchain` `#track-m`

> **Repairs** *non-pixi dependencies are project-local, never a system requirement* (2026-07-19).
> The ruling was right; its implementation had a hole that made it true on exactly one computer.

**Decision:** the shim moves from `.android/bin/rustup` to a tracked **`ci/bin/rustup`**, and
`pixi.toml`'s `android-apk`, `ci/android-release.sh` and any future iOS build step put `ci/bin` on
`PATH`. Its "target is NOT installed" message now names the environment that actually carries the
triple — `[feature.android]` for `*-linux-android*`, `[feature.cross]` for everything else.

**Why:** `.android/` is gitignored — it is where `ci/android-init.sh` unpacks the SDK and NDK
against the checksums in `android/toolchain.lock`. The shim is not downloaded toolchain, it is
*our source*, and nothing ever wrote it: `git ls-files .android` was empty and no script created it.
It existed because someone made it by hand on 2026-07-19 and that machine never lost it. **A fresh
clone could not build the APK** — `pixi run android-apk` puts that directory first on `PATH`, finds
no `rustup`, and Tauri's `rustup target add` then fails or finds a real rustup and installs an
unpinned toolchain, which is the exact outcome the shim exists to prevent.

**How it was found:** checking what a rung-2 iOS job would need. `tauri ios build` shells out to
`rustup target add` the same way `android build` does, so the shim is on the iOS path too — and it
would have been absent on the runner. Rung 1 survived only because `cargo-mobile2` never reached for
rustup during `init`.

**Consequence:** the Android release path changed for a reason that was not Android's. Verified both
ways before landing: `PATH=$PWD/ci/bin:$PATH rustup target add aarch64-apple-ios-sim` reports the
conda-pinned target, and an unknown triple fails loudly naming `[feature.cross.dependencies]`.

## 2026-09-03 — the shell's agent gate is the feature **and** Android, derived in `build.rs` `#track-m` `#agent`

> **Implements** *iOS ships agent-free* (2026-09-02, Route C) — which until now was a sentence in a
> document and a `--no-default-features` somebody would have had to remember on a Mac we do not own.

**Decision:** `mobile/src-tauri/build.rs` emits `--cfg agent_shell` when the `agent` feature is on
**and** `CARGO_CFG_TARGET_OS == "android"`. Every arm in `lib.rs` reads `agent_shell` /
`not(agent_shell)`; none reads the raw feature. `ci/checks.sh` checks both halves.

**Why a build script rather than a feature.** A Cargo feature cannot be conditioned on a target, and
`default = ["agent"]` turns it on everywhere. `src/agent.rs` calls
`fm_agent_run::nativelib::native_lib_dir`, gated to Android since 2026-09-02 because an APK's
`nativeLibraryDir` has no iOS counterpart — so **an iOS build of the shell with default features is
a compile error**, not a degraded app. Naming the condition once also keeps the two-sided logic
honest: the file has `not(...)` arms, and `not(all(feature = "agent", target_os = "android"))`
repeated at eight sites is a bug waiting for a hurried edit.

**The guard was widened, and it caught its own weakness.** The old check grepped `lib.rs` for the
literal `cfg(feature = "agent")` above `mod agent;`. It now requires `cfg(agent_shell)` there *and*
that `build.rs` still tests both `CARGO_FEATURE_AGENT` and `CARGO_CFG_TARGET_OS`. The first version
of that second half **passed while the Android condition was deleted**, because the module doc
quotes `target_os = "android"` in prose — so the guard strips comment lines before grepping. It was
found the only way it could be: by breaking the code and requiring the check to go red.

**Consequence, and the verification that is newly possible.** The mobile shell cannot be compiled
for iOS from here, but it *can* be cross-compiled for Android — the NDK is in `[feature.android]` —
and `--no-default-features` there exercises exactly the `not(agent_shell)` arms an iOS build will
take. Both were checked for `aarch64-linux-android` before landing. That is a proxy, not the thing;
it is also considerably more than the "unverifiable churn against a target that does not build yet"
this work was previously deferred as.

## 2026-09-03 — the JS toolchain is pinned across every pixi environment, because it picks a binary's architecture `#toolchain` `#track-m`

> **Extends** *non-pixi dependencies are project-local, never a system requirement* (2026-07-19) to
> a case it did not anticipate: two pixi environments that are each internally reproducible, and
> disagree with each other.

**Decision:** `nodejs` and `pnpm` are version-pinned in `[dependencies]` (shared by every
environment) rather than left at `"*"`. Environments may still differ in **rust** — `cross` carries
`rust-std` packages built against a newer compiler and that is deliberate — but not in the tool that
selects a native binary.

**Why, and it cost a billed macOS job.** iOS rung 2 failed at `tauri ios init` with
*"Cannot install under Rosetta 2 in ARM default prefix (/opt/homebrew)"*, from `brew`, while
installing `xcodegen`. Nothing about iOS was wrong. `@tauri-apps/cli` is a JS wrapper over a
per-platform native binary chosen by pnpm as an optional dependency, and on `osx-arm64` the two
environments had resolved different package managers:

| | `default` | `cross` |
|---|---|---|
| nodejs | 26.5.0 | 26.8.0 |
| pnpm | **11.13.1** | **12.2.1** |
| rust | 1.97.1 | 1.98.0 |

Rung 1 ran every pnpm step in `default` and passed. Rung 2 ran them in `cross` — the only
environment with the iOS `rust-std` packages — whose pnpm re-resolved the CLI to **darwin-x64**.
The Tauri CLI then ran under Rosetta 2, and the first thing it does is shell out to Homebrew.

**What this says about the failure mode, which is the durable part.** Two environments were each
internally consistent and locked; nothing was unpinned in the sense the 2026-07-19 ruling meant.
The defect was *relative* — a version skew between environments that only manifests as a
**wrong-architecture native binary**, three layers away, in an error message naming Rosetta and
Homebrew. Pinning per-environment is not enough when environments share a `node_modules`.

**Consequence:** `ci/checks.sh` compares the environments and fails if the JS toolchain diverges,
and `ios.yml`'s rung 2 runs *every* pnpm step in the same environment as the build rather than
relying on the pins alone. `ci/ios-smoke.sh` also checks the resolved CLI's architecture against
`uname -m` before it spends 45 minutes, so the next occurrence is one line instead of a Homebrew
message. `nodejs` is pinned to a minor, not a patch: `==26.5.0` has no `osx-64` candidate and the
solve fails outright. The guard could **not** be mutation-proven — with the pin in place a
conflicting feature pin does not solve at all, and un-pinning does not reproduce the old state
because the lock is sticky — so its comparison was verified instead against the `rust` divergence it
deliberately ignores. That limit is written into the guard rather than left implied.

## 2026-09-03 — iOS gets zlib and iconv from the Xcode project, because a staticlib cannot carry them `#track-m` `#toolchain` `#git`

> **Follows** *libgit2-sys is an unconditional dependency* (2026-09-02) — same crate, one layer
> further out: that entry made libgit2 *compile* for iOS, this one makes it *link*.

**Decision:** `ci/ios-inject-linker-libs.sh` adds `- sdk: libz.tbd` and `- sdk: libiconv.tbd` to the
generated `gen/apple/project.yml` and re-runs `xcodegen generate`. `ci/ios-smoke.sh` calls it between
`tauri ios init` and `tauri ios build`, every time.

**Why it is needed at all.** The mobile crate is `crate-type = ["staticlib"]`, and **rustc cannot
put a system dylib inside a static archive**. libgit2's own C objects *are* in `libapp.a` — `cc`
emits `rustc-link-lib=static=git2` and `+bundle` is the default — but `libgit2-sys` also emits a
plain `rustc-link-lib=z`, and `rustc-link-lib=iconv` for every Apple target. Those are instructions
for whoever performs the final link, and here that is **Xcode, which never sees them**. Rung 2's
second firing died on exactly twelve symbols: `_inflate*`/`_deflate*`/`_crc32` from `zstream.o`,
`indexer.o`, `filebuf.o`, and `_iconv*` from `fs_path.o`. **Android never hits this** because it
builds a `cdylib` that rustc links itself — a consequence of the output shape, not of iOS.

**Three tidier-looking mechanisms were checked against the pinned tauri-cli source and rejected:**

- **`tauri.conf.json` → `bundle > iOS > frameworks`** cannot express it. An entry with no extension
  renders as `- sdk: {{this}}.framework`; any other extension is treated as a *vendored* framework
  path relative to `src-tauri`. The template does carry an `ios-vendor-sdks` bucket that renders
  `- sdk: …`, but tauri-cli never populates it — only `frameworks` and `vendor_frameworks`.
- **`cargo:rustc-link-arg` / `.cargo/config.toml`** does not apply: Cargo documents it for binaries,
  examples, tests, benches and **cdylibs**, and staticlib is not in that list.
- **Vendoring** solves half at best. `libz-sys`'s `static` feature would bundle zlib, but
  `libgit2-sys` emits `rustc-link-lib=iconv` unconditionally on Apple with no feature to avoid it,
  and Apple ships `libiconv` only as a system library. iconv is Xcode-side regardless — so doing
  both there is one mechanism instead of two, the second of which would be a platform-gated Cargo
  feature edge, the exact shape that has bitten `fm-core/Cargo.toml` twice.

**Why a script rather than a committed file.** `gen/apple` is gitignored (*"it embeds absolute
paths"*), so a fresh checkout never has the edit — and **`tauri ios build` never runs XcodeGen**;
only `init` does. So the patch must be re-applied *and* regenerated by us, which is precisely the
`ci/android-inject-service.sh` pattern.

**Consequence:** the anchor is `- sdk: WebKit.framework`, unconditional in tauri's template and the
last dependency before `preBuildScripts:`. If the template changes, the script fails loudly naming
the template rather than silently patching nothing. `ci/checks.sh` runs it against a fixture built
from that template — no Mac needed — asserting the anchor is found, both libraries land exactly
once across two runs, and they land *inside* `dependencies:`. Proven by two mutations: a missing
anchor, and an injection placed after `preBuildScripts:`.

## 2026-09-03 — the iOS log is a file in the app container, because stderr reaches nobody `#track-m`

> **Amends** *the iOS diagnostic channel is stderr, not `os_log`* (same day, above). That entry named
> its own reversal trigger — "if rung 2 shows an empty pty" — and rung 2 showed one.

**Decision:** `install_logger`'s iOS arm writes every record to
`std::env::temp_dir()/formicaria.log` — inside the app's data container, which
`xcrun simctl get_app_container` hands the smoke test — **and** to stderr. `ci/ios-smoke.sh` polls
three channels (pty, unified log, app file) and reports which one spoke.

**Why: measured, not reasoned.** Run 91364602829 got the app built, installed, launched and
**painted** (deviation 35.4 against a 71.7 home screen, at t+1s). The unified log carried thirty
seconds of that process's WebKit traffic — resources loading through Tauri's scheme handler — so the
app was unambiguously alive. In the same run the pty capture was **byte-empty** and the unified log
contained **not one** of our records. So the previous entry's central claim — *"`simctl launch
--console-pty` attaches a pty and prints it"* — is false in practice here, and an iOS build had no
voice at all. A startup failure would have been invisible in exactly the way `install_logger` exists
to prevent, and it hid the very question rung 2 was bought to answer: *did the store open?*

**Why a file, and not `os_log` after all.** The argument against `os_log` is unchanged and was never
about preference: emitting to it needs `_os_log_impl` with a format descriptor in `__TEXT,__os_log`,
i.e. a C shim and a crate **this machine cannot compile**, and getting the section wrong logs
`<private>` rather than failing to build. A file needs no bridging assumption, no FFI, no framework
and no dependency — and it is the one sink whose behaviour can be *run* on Linux before it ships,
which is how the next defect was caught (see below). It is Simulator-only reasoning and allowed to
be: a Simulator is the only place this shell runs. On a real device the file still works but nothing
reads it, which is the same trigger, written down again.

**A second defect, caught for free by the same discipline.** The first draft used
`log::set_boxed_logger`, which lives behind `log`'s `std` feature — not enabled here, so it does not
compile. Building this file's logger in a throwaway crate on Linux found it before a runner did;
the fix is a `OnceLock` static, not a features change to a dependency the Android build shares.

**Consequence:** stderr is kept because it costs one line and would start working for free if a
future Xcode fixed the pty — and because the harness now *reports which channel spoke*, so the day
it does work we will know. Two sinks and no dependencies is cheaper than deciding which to trust.

## 2026-09-03 — iOS is meant to reach users' phones now, and the route is deliberately undecided `#track-m`

> **Reverses the ceiling** of *an iOS build would contradict the project-local-toolchain ruling*
> (2026-09-02) and of `ios-plan-2026-09-02.md`'s **"There is no rung 5 — a device install would cost
> $99/yr to produce an artifact nobody can run."** Both were sound under the *"no iPhone"* ruling
> they were written under. That ruling is withdrawn.

**Decision:** iOS work is no longer a survey. The goal is formicaria on **its users' iPhones**,
distributed like the Android APK — **not through the App Store**. **The route is not chosen yet**,
and that deferral is itself the decision: rungs 4 and 3 come first, because if git over HTTPS fails
on iOS the port cannot sync, and a notes app whose notes cannot leave the device is not worth
distributing at any price.

**The facts that will decide it**, researched against Apple's current pages 2026-09-03 and recorded
here so the choice needs no second research pass:

- **iOS has no "install from unknown sources".** The Android model — publish an APK, user taps it,
  it works forever — has *no direct analogue at any price*. Every app carries an Apple-issued
  signature and the device holds a profile authorising it. Platform policy, not an engineering gap.
- **External TestFlight is not the App Store**: 10,000 users, no listing, no search result, no store
  page. One Beta App Review of the first build per group; users install TestFlight and tap. **This
  was not on the menu when the owner declined "the store"** and is the only route resembling the
  Android experience. $99/yr, and a rebuild inside every 90 days or installed builds stop opening.
- **The only $0 route is free-account sideloading**: each user re-signs **every 7 days** with their
  own Apple ID, capped at **3 sideloaded apps per device**, through community tooling driving
  Apple's private APIs. Genuinely free, and genuinely much worse.
- **Neither route needs a Mac.** The `macos-latest` runner already in use *is* the Mac — that was
  never the blocker for shipping, only for debugging. A CSR can be made with `openssl` on Linux.
- **A catch specific to this owner:** Apple's enrolment identity check runs through the Apple
  Developer app and wants an iPhone/iPad or Apple-silicon Mac. The owner has none, so the paid route
  may need a support conversation. Confirm before paying.

**Consequence:** `release.yml` still gains nothing — it is the single named exception to the
no-remote-CI standing order and fires unattended on every `v*` tag. *(2026-09-04: the standing order
itself is superseded — see* the repo goes public *— but this consequence is not. `release.yml` stays
the one unattended workflow whether or not minutes are billed, because the reason it is narrow was
never the money.)* Whatever artifact iOS eventually
produces follows the APK's shape: built by a manual dispatch, attached deliberately. And **G5
(container-relative vault paths) stops being deferrable the moment a distribution channel exists** —
a distributed app *updates*, which is precisely the failure G5 exists to prevent.

## 2026-09-03 — iOS ships as an unsigned IPA that each user signs with their own Apple ID `#track-m`

> **Completes** *iOS is meant to reach users' phones now, and the route is deliberately undecided*
> (2026-09-03, immediately above). That entry chose a destination and named three routes without
> picking one. This picks one. It supersedes nothing — read the pair.

**Decision:** the route is **free-account sideloading**. A manually dispatched CI job produces an
**unsigned `.ipa`**; each user signs it with **their own free Apple ID** and installs it over USB,
using SideStore or AltStore on Windows/macOS, or AltServer-Linux, SideServer-for-Linux or `xtool
install` on Linux. The project holds no Apple account, no certificate, no CI secret, and registers
no device UDIDs — **it cannot observe who runs it**, which is the property that made this route
right rather than merely cheap.

**Why not the better-feeling one.** External TestFlight is the closest analogue to the APK and it
was rejected on two counts, in this order. The owner's intent is that users install it themselves,
off-store, over a cable — TestFlight is off-*store* but not off-Apple, and gates the first build of
every group behind Beta App Review. And the catch recorded above is decisive independent of taste:
Apple's enrolment identity check wants an iPhone, iPad or Apple-silicon Mac, and **the owner has
none**, so the $99 route may not be purchasable here at all. A route that cannot be entered is not a
route.

**What it costs the user, stated plainly because they pay all of it:** the app stops launching after
**7 days** unless refreshed (SideStore does this wirelessly, with no computer after first setup);
a device holds at most **3** sideloaded apps; and they must run a sideloader on their desktop.

**What it costs us: nothing recurring.** `tauri-cli` v2.11.4 already emits exactly this artifact —
`crates/tauri-cli/src/mobile/ios/build.rs:432-509`: `--no-sign` skips `build()` entirely, archives
with `skip_codesign()`, then lifts the `.app` out of the `.xcarchive` and calls `create_ipa()`. **No
`-exportArchive`, no `ExportOptions.plist`, no provisioning profile, no development team** — every
one of those lives in the `else` branch reached only when signing is on. `ci/ios-smoke.sh` has
passed `--no-sign` since the day it was written, for the unrelated reason that a headless runner has
no keychain. The delta between four green Simulator runs and a sideloadable artifact was a target
triple.

**Consequences.**

- **Rung 5 exists again**, asking a different question than the one it was struck for: not *"can we
  ship?"* but *"does a device-target unsigned IPA build?"* `workflow_dispatch`-only, like every
  other iOS rung.
- **`release.yml` still names nothing iOS**, and this is now *enforced* rather than asserted —
  `ci/checks.sh` greps it. The rule was prose in two headers and nothing checked it.
- **The app must never acquire an entitlement a free personal team cannot hold**: App Groups,
  keychain sharing, push, iCloud, associated domains. Any one of them makes the artifact
  unsignable by the people it is built for. `ci/ios-package.sh` asserts their absence, because
  nothing else in this repo would notice one being added.
- **G5 (container-relative vault paths) is now blocking, not deferred.** The entry above already
  said it *"stops being deferrable the moment a distribution channel exists"*; sideloading is worse
  than the update case it was written for, because a re-sign changes the container path on a
  **7-day cycle** while `vaults.json` persists absolute ones.
- **The artifact has never been installed on hardware, and every surface that mentions it says so.**
  There is no iPhone here. CI can prove the `.ipa` is well-formed, arm64, device-platform and
  unsigned; it cannot prove it re-signs, installs, launches or syncs. No user-manual page ships
  until someone with a phone confirms it does.

## 2026-09-03 — rung 3 is not run, and the ladder ends at 1/2/4/5 `#track-m`

> **Amends the sequencing clause** of *iOS is meant to reach users' phones now* (2026-09-03), which
> reads *"rungs 4 and 3 come first"*. The rest of that entry stands.

**Decision:** rung 3 is **deliberately unrun**. Rungs 1, 2, 4 and 5 are green and that is the ladder.

**Why the sequencing clause does not survive contact with what rung 3 actually is.** Its stated
reason was *"if git over HTTPS fails on iOS the port cannot sync, and a notes app whose notes cannot
leave the device is not worth distributing"* — which is **rung 4's** question, and rung 4 is green.
Rung 3 was carried along in the same sentence without its own cost ever being weighed.

**What rung 3 would return, measured rather than assumed.** Per size the sweep boots a simulator,
installs, launches, screenshots and shuts down. It **drives nothing and asserts nothing** — the
deviation check is a "has it painted yet, stop waiting" loop condition, not a pass/fail. So the
output is two PNGs of the **first-run screen** for a human to look at. Against the four questions
`ios-plan` says the rung exists to answer:

| Question | Answered by the sweep as built |
|---|---|
| Safe-area insets without the Kotlin bridge | **Partly** — already answered on iPhone 17 Pro by rung 2, *"content correctly clear of the Dynamic Island"*. The sweep adds two more geometries |
| `100dvh` | **No.** It bites when the keyboard appears; nothing focuses a field |
| The Excalidraw chunk | **No.** The whiteboard is never loaded — the welcome screen is an identity form |
| The `fmblob:` scheme | **No.** Needs a note with an attachment; no note is created |

**And it cannot be made cheap.** Measured floor from two complete runs: build ~450s + simulator boot
86s + install 87s + two launches 38s ≈ **11 minutes**. Rung 5's cache does not rescue it — rung 5
built `aarch64-apple-ios`, rung 3 needs `aarch64-apple-ios-sim`, and the vendored C is per-target.
The owner cancelled three dispatches (at 13, 13 and 5 minutes), and **each cancellation writes no
cache**, so every attempt started as cold as the last. Eleven billed minutes for two screenshots of
one screen is the wrong trade, and saying so is cheaper than discovering it a fourth time.

**Consequence — the real work this exposes.** The three unanswered questions all need the app
**driven, not photographed**: create a note, focus the editor, open the whiteboard, attach an image.
That is `outstanding.md` work in `ios-smoke.sh` (XCUITest, or a debug hook into the WebView), not a
dispatch. **It is a precondition for handing the `.ipa` to anyone**, because those three seams are
exactly what a user touches first and none of them has ever executed on iOS.

## 2026-09-03 — a managed vault persists as `@root/<name>`, resolved at read time `#track-m` `#vault`

**This is G5**, promoted from *"deliberately deferred"* (`ios-plan-2026-09-02.md`) to blocking by the
sideloading route, and now done.

**The failure it removes.** `vaults.json` persisted `absolute(&v.path)`. On a sideloaded iOS build
the app is re-signed every **7 days** and the container UUID changes, so a path written last week
names nothing this week. The notes are still on disk; they are **unreferenced**, and the app opens
to a first-run screen. That is worse than corruption because it looks like deletion, and it would
have happened to every user, weekly.

**Decision:** a vault inside this installation's managed root is written `@root/<rel>` and resolved
against [`vault_root()`] at read time. Absolute everywhere else.

**Why a marker and not a bare relative path.** This file's own doctrine forbids one — *"a relative
path in a config file resolves against the cwd, which is not a thing a config file should do"*, the
bug that made `FM_VAULT=vault` mean a different vault per launch directory. `@root/` states what it
is relative **to**. It follows the `~` precedent already in `expand_home`: a portable prefix
expanded at read time, and left literal when there is nothing to expand to, so it fails loudly as a
missing path rather than resolving somewhere unexpected.

**Desktop is untouched by construction, not by care.** `vault_root()` is `Some` only where the shell
sets `FM_VAULT_ROOT` — `mobile/src-tauri/src/lib.rs:478-499`, `app_data_dir()/vaults`, and nothing
else in the tree sets it. A desktop vault is a folder the user chose; it keeps being written
absolute, and the test that pins that (`saved_paths_are_absolute`) still passes.

**Reading heals, writing does not.** `save` is append-only and *"leaves every byte"* of an entry it
did not create, so an install made from a pre-marker build would keep its stale absolute path
forever. `resolve_path` therefore adopts `<root>/<leaf>` when the stored path is **missing**, the
root exists, and the candidate is a **directory** — narrow on purpose, unreachable on desktop, and
never fired when the stored path resolves (a decoy of the same name inside the root does not win).

**A test-side prerequisite came with it, deliberately before the flake rather than after.**
`FM_VAULT_ROOT` is process-global and now decides how a path is *persisted*, so every test that
reads or writes one races any test that sets it. `a_relative_root_is_not_a_root` was already setting
it unguarded. The lock lives **beside the code, not inside either test module**, because this file
has **two** (`tests` and `contained`) and a lock in one would not protect the other. Same idiom as
`fm-app/tests/backup_records_everything.rs`, `fm-app/src/secrets.rs` and — one day earlier, after it
had already cost ~1 run in 8 — `fm-core/tests/git_transport.rs`.

**What this does *not* fix, found while doing it and left alone on purpose:** `forget_vault` does not
remove anything from an existing `vaults.json`. It calls `save(&remaining, …)`, and `save` appends
only — it never filters the on-disk array against the list it is given — so a forgotten vault
returns on the next start. `dispatch.rs:1628` states the same fact from the other side. The existing
test passes only because it writes to a file that does not yet exist. **Separate bug, separate fix** — and
fixed the same day, in its own change: see *removal is a second narrow writer, not a relaxed
`save`* below.

## 2026-09-03 — the WebView gets a voice: `console.error` reaches the native log `#track-m`

**The gap this closes is documented and was paid for once already.** `known-issues.md`: *"the WebView
routes **no `console.*` output** there either: a signed, installed, MD5-verified build full of
`console.warn` produced zero lines while the native `ca-bundle:` log from the same run came through
fine. Anything you need to read off that device must be rendered on screen. **This cost a full
build/sign/install/ask-the-owner round trip.**"* That was Android, 2026-07-20. iOS inherits it.

**Decision:** the mobile shell injects an initialization script that forwards `console.error`,
`console.warn`, `window.onerror` and `unhandledrejection` to a third IPC command, `fm_log`, which
writes them through the same `log` sink `install_logger` already owns — stderr **and** the app-container
file on iOS, logcat on Android — tagged `web:` so the origin is never ambiguous.

**Why it is worth a third command on a surface that has had two.** `fm` and `fm_ingest` are the
whole IPC surface, deliberately. This adds one, and it is the same *category* as `fm_ingest`: a
**transport** concern, not a notes command, so it does not touch the one-command-surface ruling —
`fm_app::dispatch` remains the only door for anything that is about notes. What it buys:

- **A user's phone becomes debuggable.** Today the only channel is the screen, so a frontend failure
  on someone else's device is unreportable. This route ships to users who cannot be observed.
- **Every WebView assertion in `ci/ios-smoke.sh` becomes possible.** A test that *drives* the app —
  the next item in `outstanding.md` §2.11 — can assert nothing today, because a failed chunk load or
  a broken `fmblob:` fetch is invisible. This is the prerequisite, not the feature.
- **Android gets it too**, and Android is where the gap was found.

**`console.log` is deliberately not forwarded.** Only failures. A phone log that carries every debug
line is a phone log nobody reads, and the volume would ride the IPC surface it is being sent over.

**The script may never break the app.** It runs on every page load on both platforms, before the
frontend. So: wholly inside `try`/`catch`, a re-entrancy flag so a failure in the forwarding path
cannot recurse through the very `console.error` it overrides, a length cap per message, and a bounded
buffer while `__TAURI_INTERNALS__.invoke` does not yet exist — dropped rather than grown if the
bridge never arrives. The original `console` methods are always called first, so behaviour with the
bridge dead is exactly today's behaviour.

**Verified by cross-compiling the shell for `aarch64-linux-android` on Linux**, which the G2 work
established is possible here — this is not another change written blind.

## 2026-09-03 — removal is a second narrow writer, not a relaxed `save` `#vault`

**`forget_vault` did not forget.** It called `vaults::save(&remaining, &config)`, and `save` appends
only: it iterates the list it is given and skips every name already on disk — *"theirs. Leave every
byte of it alone."* **It never filters the on-disk array.** So on every real installation the entry
stayed in `vaults.json` and the vault came back on the next `App::load`. Live on desktop and Android
for as long as `forget_vault` has existed. `dispatch.rs:1628` already stated the same fact from the
other side and nobody joined it up.

**Decision:** a new `vaults::forget(name, to)` removes exactly one entry. `save` keeps its
append-only contract untouched.

**Why not simply relax `save`.** That contract is load-bearing: it is what lets someone hand-edit
`vaults.json` without this app reformatting, reordering or silently dropping what it does not
understand. Relaxing it to "write the list I was given" would turn *"I don't understand this"* into
*"I deleted it"* for every unknown key in the file. `set_restic` already established the alternative
— a narrow writer that changes one thing and carries everything else through — and this is the same
shape, so the file now has exactly two such writers and one appender.

**`forget` differs from `set_restic` in two ways, both deliberate.** A missing entry is **success**,
not an error: `set_restic` refuses one because inventing a vault from a typo would create a phantom,
whereas removal has no such hazard and every reason to be idempotent under retry. A missing **file**
is success too — an `FM_VAULT`-only install has none. A file that exists but does not parse is still
refused, because we do not overwrite a shape we did not understand.

**`forget_vault` now calls both, in this order, and neither alone is enough.** `save(&remaining)`
first, to materialise a live `FM_VAULT` vault that is not in the file yet — `load` prefers the file
over `FM_VAULT` the moment the file has entries, which is the hazard `save`'s own doc describes and
`set_restic` already works around. Then `forget`. The second call gets its own error message rather
than inheriting *"nothing was changed"*, which stops being true once `save` has materialised
anything.

**The test could not have caught it.** `forget_vault.rs`'s `app_over` pointed `App` at a
`vaults.json` that **did not exist**, so `save` started from `{}` and appended, and *"the entry is
not in the file afterwards"* was true because it had never been in the file. It now writes the
fixture through `save` first — and the strengthened test was confirmed **red against the unfixed
code** before the fix landed, because a regression test that has never been red proves nothing.

## 2026-09-03 — rung 3 drives instead of photographing, and is worth running again `#track-m`

> **Amends** *rung 3 is not run, and the ladder ends at 1/2/4/5* (2026-09-03, earlier today). That
> entry stands on its facts and its verdict was right **about the rung as it then existed**. What
> changed is the rung, not the arithmetic.

**What it was.** Per size: boot a simulator, install, launch, screenshot, shut down. Two extra boots
and installs at ~170s each, for two PNGs of the **first-run screen**, driving nothing and asserting
nothing. Three of its four stated questions — `100dvh`, the Excalidraw chunk, `fmblob:` — need the
app *past* first run, which needs a tap. `simctl` cannot tap. That is why it was struck.

**What unlocked it.** `simctl` cannot tap, but it can **write into the data container**. Launch 1
already creates the vault and launch 2 already happens; seeding between them costs a few file
writes and **no extra device**. So `FM_IOS_SEED` puts a git identity, a note and an image into the
vault, and launch 2 renders real content instead of the welcome screen.

- The **identity** is the gate: `App.svelte`'s `needsWelcome` is `!vaults[0].identity`, fed by
  `vcs::identity(&path)` reading the vault's git config.
- The **image** is the point: an `<img src="fmblob:…">` that fails fires a resource error, which
  *the WebView gets a voice* forwards as `web: resource failed to load: …` at ERROR level, which
  `launch`'s existing `formicaria ERROR` grep turns into a failed job. **The assertion is automatic
  and nobody looks at a PNG.** This is why the bridge had to come first.

**Every seeded format was read off `fm-cli`'s own output**, not invented — frontmatter keys, `type:
note` vs `type: asset`, the `assets: [sha256:…]` list, the `blobs/sha256/<aa>/<bb>/<full>` fan-out.
And the seeder is **read back by the app's own reader on Linux, free, on every commit**: `--self-test`
seeds a scratch vault and asks `fm-cli list` what it sees.

**That read-back immediately earned itself.** The first draft's ULIDs were `01SMOKE…` — 25 characters
and containing `O`, which Crockford base32 excludes. The store rejected both notes and `fm-cli list`
answered `0 note(s)`. That is exactly the *"a seeded vault the app rejects fails this rung for the
wrong reason"* case, and it was caught on this machine instead of inside a billed macOS job.

**A second, quieter correction from the same loop:** the read-back check first looked for `dot.png`
in `fm list` output. `fm list` prints id, date and body excerpt and **no title** — a real asset note
made by `fm-cli add` is blank there too. The check could never have passed, against a seeder that
was already right. It asserts the note *count* and the asset's *id* now. A test that cannot pass is
as bad as one that cannot fail.

**The size sweep is kept and still works** — `FM_IOS_SIZES` by hand — but it is off by default. It
buys layout screenshots for two device boots, and nothing here has a layout question worth that.

## 2026-09-03 — the sideload notice is permanent, and gated on the OS `#track-m` `#ui`

**The fact it states recurs weekly**: an iOS build is signed with the user's own free Apple ID and
stops opening about seven days later (*iOS ships as an unsigned IPA that each user signs with their
own Apple ID*). Someone not told that experiences it as the app breaking, and assumes their notes
went with it.

**Decision:** it lives in Settings, beside the other capability facts, **not** in a first-run
dialog. A notice dismissed once would be gone before the first time it mattered — the seventh day,
and every seventh day after. The half that says **"your notes are not affected"** is asserted in a
test rather than left to the wording of the moment, because that is the half that stops a lapsed
signature reading as data loss.

**Gated on `platform === 'ios'`, not on a `sideloaded` flag.** The config DTO gains `platform`,
reported from `std::env::consts::OS`. A `sideloaded` boolean would bake the current distribution
route into the wire contract and be wrong the day the route changes; the OS name will still be true.
**The backend reports a fact; the UI decides what it means.**

**The media warning is gated differently, on `vault_root !== null`, so both phones get it.**
App-private storage goes when the app does and `blobs/` is gitignored, so a push carries notes and
not their media — that was never iOS-specific, and gating it on iOS would have hidden it from the
platform that actually has users.

**A mock that permitted an impossible state is fixed with it.** `mock.ts` let a test set
`platform: 'android'` while `vault_root` stayed `null` — a combination the backend cannot produce,
since `vault_root()` is `Some` exactly where the shell sets `FM_VAULT_ROOT`. The Android test failed
and looked like a broken feature. `vault_root` is now **derived** from `platform` in the mock, so no
test can assert against a state that cannot exist. **A mock is a claim about the backend, and a
false one costs more than no mock at all.**

## 2026-09-03 — the iOS Info.plist is injected, because a missing usage description is a crash `#track-m`

**Not a denied permission — a termination.** On iOS, touching a permission-gated API with no
`NS*UsageDescription` in `Info.plist` makes the system kill the app. `＋ Media`
(`ui/src/lib/NotePanel.svelte`) is rendered on every note being edited with **no platform gate**, and
four of its items reach that hardware: Record audio (`getUserMedia`), Take a photo and Record a video
(the camera), and the file pickers (the photo library). **None of the four keys was declared
anywhere.** So the first tester to edit a note and tap ＋ Media would have crashed the app — and been
right to call it broken. Found by audit, before anyone was asked to install anything.

**Decision:** `ci/ios-inject-plist.sh` patches the generated `project.yml`'s `info: properties:` map,
anchored on `CFBundleVersion` — the last unconditional property the tauri template emits.

**The tidier routes were checked and are closed**, the same way the linker-libs script checked its
own. The template *does* carry a hook, `{{#each apple.plist-pairs}}`, immediately after that anchor —
but tauri-cli never populates it: the only `plist` in `crates/tauri-cli/src/mobile/ios/mod.rs` is
`export_options_plist`, which is the signing export, not `Info.plist`. Same dead end as
`ios-vendor-sdks`, for the same reason. And `gen/apple` is generated and gitignored, so editing it by
hand reaches exactly one machine.

**A second script rather than a bigger one.** `ios-inject-linker-libs.sh` is named for its job and
`decisions.md` — append-only — refers to it by that name; renaming it to cover a second job would
leave dated entries pointing at a file that no longer exists. So two scripts, one honest name each,
and **`xcodegen` still runs exactly once**: `ios_project_ready` calls the plist one with
`FM_SKIP_XCODEGEN=1` and lets the linker-libs one regenerate after both patches are in. Reversing
that order would silently drop the keys.

**`NSBonjourServices` is deliberately absent, and an earlier `known-issues.md` entry saying it was
needed is corrected.** `fm-serve/src/share.rs:357-370` advertises `<hostname>.local` and the phone
*resolves* that name; the key is required for **browsing** services, which nothing here does.
Inventing a service type to satisfy a key we do not need is the exact class of guess this project
keeps paying for.

**Guarded at both ends.** `ci/checks.sh` runs the injection against a template-shaped fixture on
Linux — idempotent, and landing inside `info: properties:` rather than after `entitlements:`, where
XcodeGen would ignore it and the failure would arrive as a crash on a tester's phone instead of at
build time. `ci/ios-package.sh` then asserts all four keys are present in the **built `.ipa`**, which
is the only check that covers XcodeGen and the archive as well as the spec.

**The wording is user-visible.** Each string is what the system prompt shows, so it says what is
accessed and when. A vague reason is a worse prompt and a worse answer to "why does this want my
microphone".

## 2026-09-03 — the licence notice covers what the binary *carries*, not what Cargo *resolves* `#toolchain`

**A gate keyed on one manifest is blind to everything the artifact carries that is not in it.**
`crates/fm-serve/build.rs:26` bakes `ui/dist` into the binary. That bundle carries `marked`,
`katex`, `mermaid`, `dompurify`, `@excalidraw/excalidraw`, `react`/`react-dom` (MIT) and
`@atlaskit/pragmatic-drag-and-drop` (Apache-2.0), plus eight self-hosted font families. Both halves
of the licence gate — `deny.toml` and `ci/third-party.sh` (built from `cargo tree`) — read
`Cargo.lock`, so **neither could see any of it**. MIT and Apache-2.0 require their notice to travel
with a *binary*; OFL requires it to travel with the *font files*. The reasoning was sound and was
simply never extended past Rust; `outstanding.md:174` had queued the npm half and `papers-plan.md:112`
already named the exact packages.

**Decision:** `ci/third-party.sh` now emits four sections in the order the artifact assembles them —
**crates → npm → fonts → downloaded-at-runtime** — and `THIRD-PARTY.md` is **committed at the repo
root**. It existed only inside release archives (`release.yml:141`), so a visitor could not learn
what the binary links without cloning and running pixi. That stops being acceptable the day the repo
is public.

**Three guards, each proven red before being trusted** (the repo's standing rule):
- `ci/third-party-check.sh` regenerates to a temp dir and `diff`s. Red test: deleting the `marked`
  row → *"THIRD-PARTY.md is stale"*. Wired into `pixi run ci`, which is why this drifted before —
  `pixi.toml` defined the `third-party` task and **nothing depended on it**.
- The generator **refuses to emit an `Unknown` row.** pnpm reads only the `license` *field*, so a
  package shipping its licence as a *file* reports `Unknown`; emitting that silently understates
  what ships. Red test: disabling the `khroma` override → *"no licence could be determined for:
  khroma"*.
- `ci/checks.sh` walks the **real** `@excalidraw/excalidraw` fonts directory and fails on a family
  with no notice row. It reads `SKIP` out of `copy-excalidraw-fonts.mjs` rather than retyping it, so
  the two cannot disagree. Red test: removing the Cascadia Code row → refused.

**`svelte` and `@tauri-apps/api` moved from `devDependencies` to `dependencies`.** Not a workaround
to make them appear in the notice — a correctness fix. Svelte 5 compiles components against its own
client runtime, so `svelte` executes in the shipped bundle; `@tauri-apps/api/core` is dynamically
imported by shipped code. Declaring them dev was simply false. Prod closure 227 → **246 packages**.

**Two licences were determined by reading upstream, not by assuming.** `dompurify` is
`(MPL-2.0 OR Apache-2.0)` — **Apache-2.0 is elected**, and the notice records that an election was
made, so the table stays permissive-only and matches `deny.toml`'s posture. And **ComicShanns is
MIT** while every other bundled family is OFL-1.1 — which is why the font guard's failure message
says *do not assume OFL* in those words. Xiaolai is in the notice's prose as **deliberately not
bundled** (the copier skips it) and correctly has no table row.

**The crate table resolves with `--target all`, not for the host.** Two reasons, and the second
only appeared because the file became committed. `cargo tree` with no `--target` resolves for
whatever machine runs it, so a Linux-generated notice omits every Windows-only arm — including
**`libgit2-sys`**, the one crate here vendoring GPL-2.0-only code, which is precisely what its
override was written to state. It was absent from the first generated file. And a host-dependent
table cannot be diffed: `third-party-check` would go red on every OS but the one that wrote it, so
the gate could never move off Linux. 142 → **188 crates**. A superset over-attributes, which is the
safe direction; under-attributing is the direction that is a violation.

**The release now copies the notice instead of regenerating it** (`release.yml`). It used to run
the generator on each runner, which is what made it host-dependent — and it meant **a tag was the
first place the notice was ever computed**. Both are wrong now that the file is reproducible and
gate-checked. So the archive ships the copy a human reviewed, and generation stays where a red
result is cheap, rather than inside the one workflow that fires unattended on a `v*` tag, on
Windows and macOS runners where the generator's new `pnpm licenses list` step has never once run.
The step asserts all four section headings landed, because a truncated legal notice is a violation
that looks like a success.

**The count line counts each table separately.** A single `grep -c '^| '` over a three-table file
reported *402 crates*. Per-table `awk` now, subtracting one header line each — the `|---|` separator
does not match. Cross-checked against pnpm's own count: 142 crates, 246 npm packages, 8 font
families.

**Four questions verdict: permitted, and it widens what the notice covers — so it earns this entry
whatever the verdict.** No GPL/AGPL anywhere in the JS tree, so this was an attribution gap, not a
policy one. Nothing about `deny.toml`'s Rust posture changes.


## 2026-09-04 — the repo goes public, and the economics every CI ruling rested on invert `#toolchain`

**The owner's decision, and it reverses a written one.** `decisions.md:4583` records the ruling
*"no Mac, no iPhone, **repo stays private**, a bounded macOS CI allocation spent strategically"*,
and every workflow in `.github/` is shaped by the sentence after it: minutes are billed, at **1x
Linux / 2x Windows / 10x macOS**, so nothing runs on a push and `ci`, `cross`, `docs` and `ios` are
`workflow_dispatch:` only. That is the 2026-07-18 no-remote-CI standing order, and it has governed
how this project works for seven weeks — including the habit of running four whole test suites
nowhere at all.

**What changes: on a public repository, GitHub Actions minutes are free on standard runners.** Not
cheaper — free, and including `macos-latest`. Every workflow here uses standard runners, so **all
five become free**, and the constraint that shaped the estate simply stops existing. The iOS ladder,
`cross.yml`'s macOS and Windows legs, and the four never-run suites all become affordable at once.

**The flip itself is the owner's, performed by hand.** Nothing in this pass touches visibility, cuts
a tag, dispatches a workflow, or restores a `push:`/`pull_request:` trigger — the repo is still
private while this is written, so re-enabling a trigger would bill immediately, at 10x on macOS.
The trigger blocks are *prepared* and left commented where they already sit, so restoring them
after the flip is one small commit.

**Three things a public repo changes that are not about money.**

1. **Every workflow now declares `permissions: contents: read`.** None did; every job inherited the
   repository default, including four `macos-latest` iOS jobs that need nothing but read. The
   `attach` job in `release.yml` keeps `contents: write`, because publishing a release is what it
   is for — one job, one elevated scope, stated.
2. **The three non-GitHub actions are pinned to a commit SHA**, with the version in a comment. A
   floating tag is a tag someone else can move, and `softprops/action-gh-release` is the one holding
   `contents: write`. `ci/checks.sh` gained a guard so it cannot regress, and `dependabot.yml` keeps
   the pins current — a pinned action that nobody updates is its own problem.
3. **An artifact on a public repo is downloadable by anyone.** `ios.yml`'s rung 5 uploaded the
   unsigned `.ipa`, of which `features.md` says *"nobody should be handed it yet"*. It stops being
   uploaded. The rung still builds it and still produces its stats and logs, which is what the rung
   exists for; the binary just stops being a public download.

**And one silent breakage found before it could happen.** `cross.yml` guards its expensive legs
with `if: inputs.scope == 'full'`. `scope` is a `workflow_dispatch` input, so **on a `push` event
`inputs.scope` is empty** and the condition is false: a pushed run would take the cheap `agent`
path, skip the pnpm cache and skip the entire gate, **and report green**. Fixed to
`github.event_name != 'workflow_dispatch' || inputs.scope == 'full'` *before* anyone restores the
trigger. This is the 2026-08-28 lesson arriving from the other direction — *"disabling a trigger
does not merely remove a path; it changes the meaning of the paths that remain"* — and re-enabling
one does the same thing in reverse.

**The cost prose is rewritten wherever it stops being true.** `ci.yml`, `cross.yml`, `docs.yml`,
`ios.yml`, `release.yml` and `ci/ios-logs.sh` all explain themselves in terms of a private repo and
a bill. Left in place after the flip, that prose is an instruction to the next reader — very
possibly a later session of this project — to re-disable everything for a reason that no longer
holds. `ios.yml`'s kill criteria were denominated in dollars that will not exist.

**What does not change.** `ios.yml` stays manual even when it is free: a 4–11 minute job with a
stop-and-decide point between rungs is an experiment, not a gate, and `ci/checks.sh` still enforces
that no iOS leg reaches `release.yml`. `release.yml` remains the single named exception, still
firing unattended on `v*`. And **macOS remains the one platform nobody here can observe** — a
`cross.yml` run type-checks the assistant's per-OS code and reads memory; it does not use the app.
Free minutes buy compilation, not a user.


## 2026-09-04 — a file is sliced, so its size stops being a memory limit `#track-m` `#data`

**`MAX_INGEST` was never a judgement about attachment size.** It is the point at which a file
encoded base64 into one JSON argument — the only binary door Android leaves open — stops fitting
in memory after being copied roughly ten times between the page and Rust. `outstanding.md` §1.3
put it exactly: *a memory limit wearing a size limit's clothes*. Its visible cost was that **video
was refused on a phone**, with a message telling the user to go and find a desktop.

**Decision:** `fm_core::chunked` — `ingest_chunk` appends a bounded slice to
`<vault>/.fm-ingest/<session>/part`, `ingest_finish` ingests that file through
`BlobStore::put_file`, which already streams and hashes in 64 KB reads. **The transient peak is
one slice, whatever the file weighs.** 2 MB slices from the UI, a 4 MB refusal in the core so a
frontend cannot call the whole file "chunk 0". A 24 MB file is now proven to arrive whole and
byte-identical (`fm-app/tests/chunked_ingest.rs`).

**Content addressing is untouched, and that is asserted rather than assumed.** The hash is taken
once, over the assembled file. A test sends the same bytes single-shot and in eleven pieces and
requires **one blob and one address** — because the failure that would matter here is not a
crash, it is dedup silently switching off for everything that arrived from a phone.

**`seq` is checked, and this is the load-bearing part.** Without it a dropped or repeated chunk
assembles a file that still hashes, still stores, still gets an asset note, and is wrong — the
corruption surfacing days later as an image that will not open, with nothing connecting it to the
upload. A mismatch is refused where it happens, naming the chunk it expected.

**Ordering inside `append` is chosen for the recoverable failure.** The bytes are `sync_all`'d
before the counter advances. Die in between and the next chunk is refused as out of order and the
upload restarts — survivable. The other order accepts a chunk whose bytes never landed, which is a
file with a hole in it that nothing downstream can detect.

**Sessions live at the vault root, dot-prefixed, and not in `blobs/`.** That directory is
content-addressed and `verify` inventories it; a half-uploaded file there would be reported as a
blob whose name disagrees with its contents — the signature of bit-rot. **An integrity checker
must not be taught to expect corruption.** At the root, `backup` (notes + `blobs/`, never the
root) and `verify` both exclude it for free, and `.gitignore` gained a line so a user's own
`git add -A` cannot commit bytes that are not a file yet.

**The orphan session is the one new hazard, and it is swept at boot by age.** Android kills
backgrounded apps constantly — the same fact that forces the write-record rebuild in `App::load`
— so the sweep sits beside it. **Age, never "everything"**: two uploads can be live across a
restart on a device with a paired tablet, and collecting one of those turns a survivable
interruption into a failed upload. The age comes from a stamp the session writes for itself, not
from mtime: mtime is not ours, and a sweep that deletes on a sync tool's timestamp eventually eats
a live upload.

**The gate this was waiting on had already been met.** `outstanding.md` §1.2 ruled *"do not do 2
before 1"* — no lifting the ceiling until the app told people where phone media actually survives,
since a higher ceiling invites people to trust it with more. That shipped on 2026-09-03: Settings
states *"photos and files live only on this phone until they reach a backup — a git push carries
your notes but not their media"*, gated on `vault_root` so both phones get it. Checked before
building, not after.

**A test was reversed, deliberately and in place.** `ingest.phone.test.ts` asserted *"refuses a
file too large for the bridge, and says why"*, which was correct while the single-shot message was
the only door. It now asserts the shape that survives — nothing oversized goes through the
single-shot path — with the old reasoning kept above it, because a test that quietly changes its
mind is a test nobody can date.


## 2026-09-04 — a backup surface that cannot say *when* is not a backup surface `#vault`

**The one fact a person wants from a backup panel could not be shown at any price.**
`fm_core::backup::latest()` has returned the newest `fm`-tagged snapshot since the tier was built,
and **no dispatch command exposed it** — so *when did this last work* was unanswerable, on a screen
whose entire job is to answer it.

**Decision:** `backup_latest`, its own command rather than a field on `VaultStatus`. `backup_status`
polls every 45 s and already shells out per vault; adding a `restic snapshots` there is another
spawn on every beat and, for a repository that is not on this machine, a network round trip. **A
fact worth a process is a fact worth asking for when someone is looking at it.**

**Three answers, not two, and keeping them apart is the design.** A snapshot (id, time, and the
*source paths* it recorded — a snapshot taken on another device names that device's paths, and a
restore that silently used them is a failure worth seeing coming). **Never backed up** — the
repository opened and holds nothing; that is not an error and it is the state that should worry
somebody, so it is not folded into the third. And **cannot tell you**, with the reason: no restic,
no repo, no password, or a repository that would not open. An unreachable destination is *reported,
not raised*: a backup repo on another machine is unreachable as an ordinary Tuesday, and a panel
that throws on it tells the user less than one naming the repo it could not open.

**A restic-only machine can now press the button.** `canRun` required a git remote, so a machine
with restic, a repository and a password had a tick box that ticked and a Back up button that never
enabled. The panel had been made to *say* so, which was honesty rather than a fix. The two tiers
were already independent inside `run()`; only the gate assumed one.

**And the verdict stopped reporting a tier that never ran.** With no remote anywhere, every vault
landed in `stuck` and the panel said *"your notes are still on this machine"* — at the exact moment
the snapshot tier had carried them off it, because a snapshot covers the notes directory as well as
`blobs/`. Not merely unhelpful: false.

**`dispatch.rs` has tests now — its first, ever.** It is the one command surface every frontend
goes through, `fm-core` beneath it is well covered and the UI above it is covered by mocks, and the
seam between them had **zero** `#[test]`. Six, each proven red first, including one that searches
the *whole* serialized `backup_status` for the password rather than checking the fields it knows
about — because a leak arrives as a new field somebody added without thinking, and a test that only
inspects known fields cannot see that. **A UI test presses Back up**, which none ever had.

**MASTERPLAN's own S6 acceptance is kept at last.** It asked for restore → **diff the whole vault**
→ `verify --scrub`, and said *"test the restore in month one"*. What existed compared one note's
bytes. The new test was proven red by making `backup` silently stop carrying `blobs/`: **the
single-note test stayed green while the whole-tree diff failed.** That is the entire argument for
the shape — every failure worth a backup test is a file that is *missing*, and only a set
comparison finds one.


## 2026-09-04 — a test that names somebody's private repo, and three that only passed here `#toolchain`

**Found by running the suites that had never been run**, which was the point of running them.

**`git_differential.rs` asserted that a *private* repo fails to clone**, and named the owner's real
`personal-notes` in a file that was about to become public. Three things wrong with that, and only
the first is obvious: it published the name; it made the suite depend on one account staying as it
is; and it would have **inverted** the day that repo's visibility changed.

**The fix is better than a fixture repo, because of a fact worth recording:** GitHub answers an
unauthenticated `git-upload-pack` with `401 WWW-Authenticate: Basic realm="GitHub"` for a private
repository **and for one that was never created — identically**, deliberately, so a 404 cannot be
used to enumerate private repos. Measured against both on 2026-09-04, and confirmed through
libgit2: the credentials callback runs either way and our wrapper says *"this remote needs an
access token"*. So the URL now names a repository that can never exist. It depends on nobody.

**`acquire.rs` cloned one of the maintainer's unrelated personal repos.** Pointing it at *this*
project's repository was tried first and is **wrong**, which is worth recording because it looks
like the obvious answer: the test asserts an **anonymous** clone, and on the maintainer's own
machine a clone of this repo succeeds through the git credential helper — green while proving
nothing, for exactly the one person least able to notice. It uses `octocat/Hello-World` now,
overridable with `FM_TEST_PUBLIC_REPO`.

**And that run found a test that was passing because of this machine.** `acquire.rs` asserted *"a
typo is not an auth problem"* — and with no credential helper it is one, necessarily, because the
401 above makes a typo and a private repo indistinguishable from outside. `needs_auth` is the
honest answer there. The assertion was a claim about the *machine*, not the code; it is now
conditioned on a helper existing and skips with a reason. **The general form is in
`known-issues.md`: run the suite once with `GIT_CONFIG_GLOBAL=/dev/null` before believing anything
it says about a remote.**

**`ci/android-smoke.sh` had never executed and did not work.** Two bugs, stacked, the second hiding
the first: it looked for the vault under `files/` (there is no `files/` — Tauri's `app_data_dir()`
on Android *is* `/data/data/<pkg>`), and its directory test was
`adb shell run-as $PKG sh -c "[ -d $dir ]"`, which **exits 2 for every path** because `adb shell`
joins its arguments and the device's shell re-parses them. So the wrong path was undiagnosable from
the failure message. Fixed, and **it now passes**: four launches, all painted, deviation ~25.8
against a threshold of 10.

**The durable lesson is about framing.** `test-native-git`, `check-cross`, `check-pins` and
`android-smoke` were filed as blocked on CI minutes. **They are pixi tasks that run on this machine
in seconds.** Three were green on their first run; the fourth was broken and nobody could have
known. It was never a CI gap — it was a habit gap, and it cost a broken script sitting in the tree
for five weeks.


## 2026-09-04 — a poisoned lock is recovered, not propagated `#seams`

**A panic anywhere bricked the whole process.** `App::lock` was
`self.vaults.lock().map_err(|e| e.to_string())`, so one panic while holding the vault guard left the
mutex poisoned and **every command afterwards** — from every client, for the life of the process —
answered with the `Display` of a `PoisonError`. That is not a sentence anyone can act on, and there
is no way back except restarting the app. `paper.rs` records it happening for real: one multi-byte
character in one PDF.

**It is worse than a single-client bug.** The agent thread and the webview both go through
`dispatch`, so a panic in either took out both.

**Decision:** recover with `into_inner()`, and **log it**.

**Is the poisoning load-bearing? No, and the argument is the entry.** Poisoning guards against
reading torn state, and there is exactly one torn state here: `Vaults::add` pushes to `all` and
`list` in turn, and its own doc says that is *"so `store` and `list` cannot disagree"*. But **both
halves are reconstructions of on-disk truth** — `list` mirrors `vaults.json`, which
`import_into_new_vault` deliberately writes *before* memory (*"JSON before memory, so a failed write
never leaves a vault that vanishes on restart"*), and `all` is an index `reindex` rebuilds. So the
worst case recovery admits is a transient in-memory disagreement that a restart fixes — weighed
against a process that answers nothing until it is restarted anyway. The trade is not close.

**Recovery must be loud, or a panic becomes invisible** — which would be a worse bug than the one
this fixes. Once per recovery, not once per call, and worded for a user: their notes are files and
are not at risk.

**The test is deterministic, which is why it is worth having.** `poison_for_test` panics inside a
`catch_unwind` while holding the guard; the test then asserts a read *and a write* still work. It
was proven red against the old one-liner. **This landed first of all the code changes on purpose**:
the two lock-contention tests below it panic on worker threads, and without recovery the first such
panic would poison the mutex and bury the real failure under a wall of unrelated errors.

## 2026-09-04 — the vault lock is not held across a subprocess or a revwalk `#seams`

Three arms took the global vault guard and kept it through work that shells out. `papers-plan.md`
B5 named the ingest one and it sat in a survey document; the others were in `known-issues.md`.

- **`ingest` and `ingest_finish`** held it through `pdftotext` *and* `vipsthumbnail` — two spawns
  per file. A bulk import froze every tab, every pane and the phone's entire UI: 25–40 minutes for
  5,000 PDFs. **`ingest_finish` is four days old and inherited it**, which is the worse half, since
  chunked ingest exists precisely for large, slow files.
- **`activity`** held it across one `git log` per vault, and it is among the first things the UI's
  first `refresh()` fires — so on a cold start every other command queued behind a year of history.

**Decision:** resolve the vault, **drop the guard**, do the slow work, re-take, and **re-resolve**.
The pattern is not new — `run_backup`, `run_import` and `open_skipped` already follow it, and
`run_import` states the rule the re-resolve exists for: *"if the vault was forgotten or moved in
that time, writing to the path we remember would put notes somewhere nothing is watching."*

**`commands::activity` split into `resolve_touches` + `activity`** so the git half and the store
half can run at different times. The public signature is unchanged, so its integration-test callers
did not move, and **the note-class exclusions stay in `commands.rs`** — `ci/checks.sh`'s
notes-base-filter guard carries a *named-file exemption* for `activity`, keyed on that file.

**`ingest::thumbnail` returns early when the thumbnail already exists.** Independently correct — a
thumbnail is a pure function of a content-addressed blob — and it is what keeps `asset_note`'s
later call a `stat` rather than a second subprocess, so no caller (`fm add`, bulk import) loses a
thumbnail to this change.

**The tests are a rendezvous, not a stopwatch**, and that distinction is what lets them sit in
`pixi run ci`. A stub `vipsthumbnail` (and a stub `git` that defers to the real one) blocks until
the test releases it, so the failing case waits out a 5 s timeout and the passing case returns in
0.06 s. Measured both ways.

**Two things the tests taught, both worth keeping.** The activity probe must not itself touch git —
`list_vaults` was the obvious choice and is wrong, because `infos` shells out per vault for the
remote label and identity, so it blocked on the stub rather than on the lock and measured nothing.
And the activity test must be `#[cfg(not(feature = "native-git"))]`: under `test-native-git` the
routing bypasses the subprocess backend, the stub is never invoked, and the test would **hang**
rather than fail.

## 2026-09-04 — a proposal its author turned down is never merged `#git` `#agent`

**`accept_proposal` merged withdrawn text into `main`.** It read the `proposes:` branch and called
`merge_proposal_branch` without ever checking `declined`.

**The state is ordinary, not exotic.** `reject_proposal` deletes the branch *and* stamps `declined`
— but only where it runs. When a **peer** rejects, their copy of the branch goes and ours does not:
we get their `declined` note on the next pull and keep `refs/heads/proposal/<id>`, our own local
head. `retire_settled_proposals` sweeps exactly that, but only where the guardrail ceiling is
checked — when a *new* proposal is created. **The window between pulling a rejection and the next
`create_proposal` is the bug**, and it is as long as the user goes without proposing anything.

**Corrected on the way in:** `outstanding.md` blamed `pull` for lacking `--prune`, and **`--prune`
would not have fixed this.** It removes only *remote-tracking* refs; the ref that survives here is a
local head, which no fetch flag touches. Adding it would have changed both git backends, dragged in
a differential test, and deleted every stale remote-tracking ref in a directory the user may also
run git in by hand — for no effect on the defect. Dropped.

**Refused at the seam, not only in the UI.** `ProposalReview.svelte` already orders its declined
branch ahead of Accept, so the desktop was covered — but `fm-serve`'s HTTP surface, `fm-cli`, the
agent and a stale open pane all reach the function directly. Same stance as `edit.rs`'s `thread_of`
refusal: guard the one gesture that can reach the damage.

**`mock.ts` had the identical gap and was fixed in the same commit.** Its `proposal_content` and
`proposal_for` already checked `declined`; `accept_proposal` did not. A mock that accepts what the
backend refuses lets a UI test go green for the wrong reason — the exact drift `mock.ts`'s own
header warns about.

**The test reads `main`, not the return value.** A test asserting only the error type would still
pass against a guard placed *after* the merge; this one requires the proposed text to be absent
from `HEAD`. Proven red — with the guard disabled, the withdrawn text is on `main`.


## 2026-09-04 — the phone's blob route answers a `Range`, and stops lying about it `#track-m`

**The comment claimed something the API cannot do.** `blob_response` said *"a GET streams, and
`<video>` can seek without the file ever being held whole in memory"*. It did none of that: it took
the bytes through `dispatch("resolve_asset")` and returned `out.into_bytes()` — the whole blob, in
memory, with no `Accept-Ranges` and no `Range` parsing. **And "streams" is not achievable here at
all**: `register_uri_scheme_protocol` hands back a `Response<Vec<u8>>` and Tauri exposes no
streaming body at this seam. So the fix is not "make it stream"; it is **bound the peak to one
requested window**, and say that instead.

**This stopped being latent on 2026-09-04.** Chunked ingest removed the phone's upload ceiling that
same day, so the files this path must serve are now unbounded — a 200 MB video was a 200 MB `Vec`
plus wry's copy, on the device with the least memory.

**Decision:** parse `Range`, answer `206`/`416` with `Content-Range`, advertise
`Accept-Ranges: bytes`, and `seek` + `read_exact` exactly the window.

**`parse_range` and `inline_safe` moved to `fm_app::wire`.** They were private in
`fm-serve/src/blob.rs`; the phone needed both and had neither. `wire.rs`'s own header already made
this argument — *"Moving the pure logic into the workspace is the cheap half of that problem: no
second test runner, no Android toolchain in CI, the tests just run"* — and it is the same instinct
as the `fm-query` seam. A new `blob_reply` joins them: the whole decision, with no I/O and no HTTP
type in it, so both transports share one policy and **`pixi run ci` tests it**. `fm-serve`'s 67
existing tests pass against the moved copy unchanged.

**The phone also shipped none of the desktop's security headers, and that is not a desktop
concern.** `blob.rs` sets `nosniff`, a CSP and an `inline_safe` allowlist forcing
`Content-Disposition: attachment`, and its header argues why: a blob is reachable as a same-origin
URL, and **blobs arrive from collaborators through the merge driver**. That is true on every
platform that serves one. Android had a hardcoded `application/octet-stream` and nothing else. Now
it sniffs the real type and sets all three.

**It resolves through `fm_app::dispatch::blob_path_of_kind` rather than `dispatch`.** That looks
like a breach of the every-command-through-`dispatch` rule and is not: the function is `pub` for
exactly this, its doc says so — *"for a transport that wants to stream the bytes itself"* — and
`fm-serve` already uses it. It is what makes both transports share one resolver, one thumbnail
fallback and one entitlement check.

**What is verified, and what is not — stated because the gap is the interesting part.** The
decision logic has four tests in the gate, proven. The handler compiles for `x86_64-linux-android`
and the app launches, paints four times and logs no error on the emulator. **An actual ranged fetch
on a device is *not* verified.** I seeded a blob and a note referencing it, launched, and got
silence — then seeded a *deliberately missing* blob as a negative control and got silence again,
which proves the handler is not reached from any screen `android-smoke` drives. So the wiring rests
on compilation plus the shared tests, and closing that properly belongs to `outstanding.md` §1.1's
device work. Do not read the green smoke run as covering this.

## 2026-09-04 — a shell that cannot configure its paths refuses, rather than opening the first-run form `#track-m`

**A silent early return produced a *wrong* screen, not a blank one.** `configure_paths` was
`let Ok(dir) = handle.path().app_data_dir() else { return };`. With nothing set, `config_dir()`
answers `None`, `App::load` **succeeds with zero vaults**, and the user meets the ordinary first-run
form — whose `create_vault` then cannot persist anything, because there is nowhere to write the
vault list. Indistinguishable from a fresh install, and every attempt to fix it by hand fails
identically.

**The infrastructure to see it already existed and was not used.** `install_logger()` runs on the
line immediately above, and its doc says that ordering exists *"precisely so that everything after
it — including its own failures — is visible"*.

**Decision:** `configure_paths` returns a verdict; the failure is a sentence saying what it means
for the user, logged; and the verdict is recorded in a `OnceLock` that `boot` consults first.

**A log line alone would not have been enough**, which is why the `OnceLock` is there: without it
`boot` still succeeds with zero vaults, `vault_state` clears its `last` error, and the message
evaporates behind the form. **Only the paths verdict is once-only** — opening the vaults stays
retryable, which is the whole reason `boot` is callable more than once.

**Scoped, so the escape hatch stays honest.** If `FM_CONFIG_DIR` is already set — a desktop debug
run of this library — `app_data_dir()` failing is a warning, not a refusal.

**This is not a "make it work" fix.** The app cannot make a platform produce a data directory.
The defect was that it could not tell you.

**Proven red on the real runtime.** `configure_paths` now logs `vault root:`, and
`ci/android-smoke.sh` asserts that line exists **and precedes `vaults ready`** — the ordering is
what proves the paths were configured rather than skipped. Removing the log line turns the smoke
test red on the emulator; verified both ways, on-device.

## 2026-09-04 — the read view draws the small copy, and the fallback is the load-bearing half `#ui` `#track-m`

**Every inline image decoded at full camera resolution.** A 12 MP JPEG is ~50 MB of pixels; five in
one note is a renderer kill on a phone, and the kill is *silent* — `onRenderProcessGone` is
unhandled, so the framework default takes the process and the app simply vanishes.

> **SUPERSEDES the comment at `render.ts`'s image branch**, which said *"There is no thumbnail path
> on any platform … `has_thumb` has no consumer; `assetUrl` has no `kind`"* and named M8 as the fix.
> Two of its three claims had already stopped being true — `assetUrl` gained a `kind` when the feed
> shipped, and `Timeline.svelte` consumes it. A stale comment asserting the absence of the thing you
> are adding is worse than no comment.

**Decision:** the `<img>` branch uses `asset.thumb ?? asset.url`. Images only — a 400x400 webp is
not a video poster and not a PDF, so every other branch keeps the full blob.

**The fallback is the whole reason this is safe, and it had to land first.**
`resolve_asset_bytes` used to hard-error on a missing thumbnail, and `vipsthumbnail` **does not
exist on Android** — so every image ingested on a phone has a blob and no derivative. Asking for
`?kind=thumb` would have turned each one into a 404 placeholder: a performance fix that breaks the
picture. `resolve_asset_bytes` now routes through `commands::blob_path_of_kind`, which had the
correct behaviour all along and documents both halves of it — the fallback, *and* that the blob is
checked before the thumb so a `derived/` file cannot answer for a vault whose blob the caller was
never entitled to. The two functions had simply drifted apart.

**Which half of M8 this is.** Serving a derivative: done. *Generating* one on Android: still open,
still M8, and still why the fallback is load-bearing rather than defensive.

## 2026-09-04 — the gate refuses to run without the tools its tests need `#toolchain`

**97 tests skip rather than fail when a tool is missing** — 72 on `git`, 7 on `restic`, 3 on being
offline — and a run that skipped them reports `ok` in exactly the same words as one that ran them.
`fm-core/tests/backup.rs` opens by quoting its module's rule, *"an untested backup is not a
backup"*, and skips seven of its own without restic.

**The skipping is right and stays.** `fetch.rs` gives the reason: *"a gate that fails on a train is
a gate people learn to ignore."* A contributor without restic should still get a useful local run.

**What was missing is anything that noticed the difference.** So the skips stay soft and the
**gate** gets loud: `ci/checks.sh` now fails when `git`, `restic`, `pdftotext` or `vipsthumbnail`
is absent. `pixi run ci` is the single gate, and a single gate that can quietly cover a third of the
suite is not one.

`restic`, `poppler` and `libvips` come from the default pixi environment and are present by
construction. **`git` does not** — it is the system binary, deliberately, because git is a
capability and not a dependency. That is precisely the one that goes missing in a bare container.

Same principle as the comment-anchoring guard directly above it in the same file: *a guard that is
disarmed by adding a comment is worse than no guard.* This one would have been disarmed by an
absent binary.

## 2026-09-05 — a snapshot says what it held, and "restic did not say" is its own answer `#vault` `#ui`

`backup` returned unit. So the one thing the media tier could report was *that it happened* — and
the panel, having nothing to say, said the same fixed phrase over every vault: **"notes and
attachments"**. That sentence is wrong for the ordinary desktop vault, which has no `blobs/` yet.
A backup surface whose entire job is to avoid overstating what it did was overstating on every
run, quietly, because the layer below it answered nothing to contradict.

**`fm_core::backup::backup` now answers `Backed`**, and the command answers `BackupRun`:

- **The directories that actually went in.** `notes_dir` by its own name — `docs` for a vault
  whose `vault.json` moved it — and `blobs` true only when there were any. These cost nothing:
  `backup` already decided them, and the old signature simply threw them away.
- **`contents`: restic's own summary**, from `restic backup --json` — short id, files new /
  changed / unmodified, bytes read, bytes the repository grew by. Not one number computed here.
  A count we worked out ourselves sitting beside counts restic gave us is how a surface starts
  being confidently wrong.

**Nullable as a whole, not field by field.** `contents: null` means *restic wrote the snapshot and
did not describe it*. That is not *the snapshot held nothing*, and six separately-nullable numbers
would have put a reader back to deciding which zero was a zero — the exact failure `unpushed: 0`
vs `null` was fixed for on 2026-09-04, and the one `backup_latest` is shaped around (`id: null` is
*never backed up*; `unavailable` is *this machine cannot tell you*). One null, said once.

**A summary that will not parse must never fail the backup.** By the time the line is read the
snapshot exists in the repository. Refusing the whole call because a progress line changed shape
in some future restic would turn a snapshot that is safely written into a reported failure — the
worst answer available, and the one that teaches someone to stop believing the panel. So
`summary()` returns `None` and the caller says so. For the same reason it is all of restic's
numbers or none: a missing field defaulted to zero is indistinguishable from an empty vault.

**The cost of `--json`, paid.** It also moves restic's errors off stderr as sentences and into
JSON objects — `{"message_type":"exit_error","code":12,"message":"Fatal: wrong password…"}` — so
passing stderr through raw would have handed the user a serialized struct where a sentence used
to be. `failed_json` pulls the messages out and falls back to the raw text. Worth naming because
it is the kind of regression a flag like this smuggles in: the feature works, and the failure
path quietly gets worse.

**Not taken: a second `restic snapshots` call after the backup.** It would have given the id and
nothing else, cost another round trip against a network repo, and still not said what changed.

Closes `outstanding.md` §2.10's last residue. Proven red four ways: `summary()` forced to `None`,
`blobs` hardcoded true, the dispatch arm restored to `nothing()`, and the panel's fixed phrase put
back — that last one failing with the old sentence printed verbatim.

## 2026-09-05 — a closed section in the queue leaves a tombstone, because the code cites it by number `#toolchain`

`outstanding.md` opens with a rule it means: *"This file is a queue, not a log. When an entry is
fixed, **delete it** — the first version kept its fixed entries struck through, and within a day it
had become a changelog with nothing to do in it."* Correct, and it has been followed: §2.6 and §2.9
were deleted when they closed.

**And 17 comments in the source cite this file by section number.** `dispatch.rs`, four UI tests,
`backup_cli.rs`, `agent.rs`, `papers-plan.md` — `§2.6b` six times, `§2.10` seven, `§2.6` and `§2.9`
twice each. Four of those already point at nothing: the sections they name were deleted, correctly,
by the rule. Nobody noticed, because deleting a heading breaks a reference silently and in another
crate.

So the two rules were in direct conflict, and each was being followed by someone who could not see
the other. `decisions.md` already ruled on exactly this failure, one file over — *"cite rulings by
subject, never by number — `mobile-design.md` and `plan.md` number them differently, and the hybrid
propagated a wrong number into this file."* The same mistake, made against a different document.

**The ruling: a closed section is replaced by a one-line tombstone, not removed.** It names what
the section was, that it is closed, and where the record went — `decisions.md` for the why,
`features.md` for the status. Three lines, not seventy-six.

**Why a tombstone and not the obvious fix.** Rewriting seventeen comments to cite by subject is the
*better* end state and it is not free: a comment that says *"this is `outstanding.md` §2.10's last
residue"* is doing real work for the next reader, and the subject-shaped version of it is longer and
vaguer. A tombstone keeps every existing citation resolvable at the cost of one line per closed
section, and it makes the closure visible at the place someone looks it up — which deletion does
not. New citations should still prefer the subject; this is about the ones already written.

**What a tombstone is not.** It is not the struck-through entry the rule forbids. The distinction is
length and intent: a tombstone says *this is closed, look here* and stops. A changelog entry
re-litigates the fix. §2.10 had grown to 76 lines of solved problem sitting in a work queue, which
is precisely the failure mode the file's opening paragraph describes.

## 2026-09-07 — the tree is mechanically formatted, and both formatters' settings were measured `#toolchain`

Until now this project had **no formatter at all**: no `rustfmt.toml`, no prettier, no eslint, no
`cargo fmt` in the gate, and no `-D warnings` or `RUSTFLAGS` anywhere. On 235,000 lines, about to be
public, a stranger's first pull request would have met no mechanical baseline — and every review of
it would have spent some of its attention on spacing.

**The rule this follows: record the house style, do not overwrite it.** A formatter adopted on
defaults reformats a codebase into somebody else's habits and calls the result a standard. So every
setting was measured against the code as written, and the measurements are in `rustfmt.toml` and
`ui/.prettierrc` beside the values they justify.

- **`max_width` / `printWidth` = 100.** Excluding comments, the 99th percentile of Rust line length
  is 101 characters and the UI's is 102. Both are written to a hundred columns; nobody had said so.
- **`use_small_heuristics = "Max"` is the setting that mattered for Rust.** rustfmt's default
  `chain_width` is 60% of `max_width`, so an 82-character method chain explodes across five lines —
  and this codebase writes them on one. It takes the adoption diff from 1,480 hunks to 685, and
  from +21,716 lines to a **net −470**: most of what rustfmt does here is *rejoin* lines that had
  been split unnecessarily, which is the opposite of what a bulk reformat usually reads like.
- **Widening prettier makes it worse, which is not the direction I expected.** At `printWidth` 110
  it disagrees with 70 files and at 120 with 74, against 68 at 100 — a wider budget lets it rejoin
  lines the authors had split on purpose. The measurement argued for the narrower setting.

**Comments are never reformatted.** The doc comments here are prose — arguments, incident reports,
the reasoning behind a ruling — hand-wrapped for reading. rustfmt's `wrap_comments` and
`format_code_in_doc_comments` are nightly-only and stay off, and `rustfmt.toml` names them so
whoever finds them and thinks they look tidy is told not to.

**What formatting is not allowed to cost.** Three clippy lints were **refused** rather than obeyed,
each with the reason in the code: `drop(scoped)` is the mechanism that ends a `&mut` borrow, not a
mistake; `merge.rs`'s explicit `None => {}` is where a comment lives that an `if let` has nowhere to
put; and `write_vault_files` was not dead but native-only. A style rule that deletes an explanation
is a bad trade, and the `#[allow]` carries the argument so the next person does not re-litigate it.
One prettier regression is recorded rather than hidden: `size.ts`'s `if (v >= 1) return …` was 101
characters on one line and is now a braceless `if` across two. That is the price of any mechanical
formatter, paid knowingly.

**Both reformats landed as their own commits and are in `.git-blame-ignore-revs`**, which GitHub
honours. That file's own rule: a commit with one real change in it does not belong there, because
blame would step over that change too.

**Cost, stated.** Two devDependencies (`prettier`, `prettier-plugin-svelte`). Neither reaches the
bundle, and `ci/third-party.sh` reads `pnpm licenses --prod`, so the licence notice is unaffected.
The gate grows by four tasks (`fmt`, `fmt-ui`, `clippy`, and `test-native-git` alongside them) and
about 30 seconds.

## 2026-09-07 — a merge never stalls on a question whose safe answer is a note `#git` `#sync` `#data`

**Two notes froze two hundred, for thirty-nine days, on a real phone.** A `DU` delete/modify
conflict — a note deleted on one device and edited on the other — left the index unmerged.
`commit_all` refuses while anything is unmerged, so **202 unrelated notes could not be committed or
pushed**, 196 of them brand new and no part of the merge at all. The owner's verdict: *"the user does
not know about merge conflicts and so on, so all the rest should be committable and synchable."*

**The ruling.** When a pull leaves a conflict whose kinds are `DeletedByUs` or `DeletedByThem`, the
app resolves it automatically **in favour of keeping the note**, finishes the merge, and says so
persistently. Marker kinds (`BothModified`, `BothAdded`) and `BothDeleted` are untouched and still
wait for a human.

**What this costs, stated plainly rather than argued away: it discards a deletion.** Somebody deleted
that note on one device and we are bringing it back. That is a real user action being overridden, and
calling it "safe" without saying so would be the overstatement this project keeps correcting.

**Why it is still right.** The two outcomes are not symmetric. A resurrected note is *visible* and one
tap from being deleted again — `resolve_conflict(…, Keep::Mine)` already exists on both backends. A
note deleted by fiat is gone from the working tree and recoverable only from history, and **on a phone
there is no shell**, so in practice it is not recoverable at all. This is the same trade
`decisions.md`'s blob-manifest rule already made one level down — *"visible and recoverable, versus
silent and permanent"* — and the same asymmetry behind *"deleting an untracked note is unrecoverable,
while deleting a tracked one is one `git checkout` away."*

**What this is NOT, because a reader will reach for the wrong neighbour.** It does **not** touch
`merge.rs`'s *"we never resolve that by fiat"*. That rule governs the **merge engine** choosing
between two pieces of surviving content — a divergent frontmatter field, where picking a winner
silently destroys the loser. A delete/modify has one piece of content and one *absence*; keeping the
content discards no text that git does not still hold on the merge's other parent. The engine is
unchanged, and an auto-resolve attempt on a *frontmatter field* would still be the mistake that was
built and reverted on 2026-07-22.

**Addressing the acceptance clause by name.** *"Any change to this behaviour is its own ruling with
its own adversarial pass. Acceptance: both values survive in the file, the file parses, and the result
is `Conflicted` — never Clean."* That clause is scoped to the frontmatter/body merge, and its test —
*both values survive in the file* — is meaningless where one side has no file. It is not satisfiable
here and it is not meant to be. This is the separate ruling it asks for.

**It must not be silent**, which is where *"explicit, never silent"* still binds. `Pulled::Merged`
carries the kept paths, `PullResult` and `VaultSync` carry them to the UI, and **both doors onto a
pull report them** — the Backup panel's step list and the "someone pushed" chip — each naming the
notes and saying the deletion can be re-applied in a tap. The undo is
`resolve_conflict(…, Keep::Mine)`, which already existed.

**What shipped is weaker than the sentence this entry first carried, and the gap is recorded rather
than glossed.** That sentence promised a *persistent* surface. What is built is a step line (per run)
and a dismissible banner — better than silence, and not the same thing. A persistent surface would
mean a chip like *unrecorded* and *unreadable*, which needs the kept set to live in app state beyond
one sync run. **That is owed**, and it is in `outstanding.md`. Writing the stronger claim and
shipping the weaker one is precisely the drift this file exists to catch, so it is corrected here
rather than left standing.

**Also ruled, and separately:** a conflict that *does* block must not block everything else. Git will
not write a tree from an index holding any conflict entry — that is absolute — but staging an
unrelated path into a conflicted index is legal, `Index::write()` with conflicts present is legal, and
a tree assembled by hand has no mid-merge check at all. So `commit_all` commits `owned` minus the
conflicted paths rather than refusing wholesale. The hazard that governs the implementation: **the
real index must move to stage 0 in lockstep with HEAD**, or `finish_merge_if_resolved` later writes a
merge commit that deletes every note committed this way.

## 2026-09-07 — a divergent field keeps both, by demoting the loser into a field beside it `#git` `#sync` `#data`

**This reverses ruling 4 of 2026-07-19** (*"a divergent frontmatter field keeps its current
loud-and-absent behaviour; the fix is the missing UI surface, not a semantic change"*) **and is the
"own ruling with its own adversarial pass" that both that ruling and the ⛔ block above demand.**
Read the acceptance clause they attach to any reversal before reading this — *"both values survive in
the file, the file parses, and the result is `Conflicted` — never Clean, or each machine keeps its
own value by fiat and re-derives the conflict forever"* — because this entry answers it clause by
clause, including the one clause it does not meet.

**Decision.** When `merge_objects` finds a field that both sides moved to different values, it no
longer returns `None` and drop the whole file into a text merge. It picks a winner by a stated rule
and writes the loser into `extra` under a reserved `conflict-<field>` key, as a set. A card dragged
to two columns merges to `status: done` with `conflict-status:\n  - doing` beside it. The note
parses, indexes, opens, renders and commits — and both values are in it, in plain sight, in the file
the user already knows how to edit.

**The winner rule, and why it has to be exactly this one.** Later `updated` wins; on a tie the
greater `PropertyValue` wins (`fm_model::PropertyValue` derives `Ord`, variant before value, and both
sides of one field share a variant). Both halves read **only content** — never "ours" — so two
devices merging the same pair of commits compute the same winner, the same loser and the same bytes.
The demoted set is **sorted** for that same reason: ours-then-theirs order is the one thing about
this that would otherwise differ between the two devices. This matters more than it looks. *"Each
machine keeps its own value by fiat and re-derives the conflict forever"* is the failure the July
acceptance clause was written to prevent, and **determinism is what prevents it, not `Conflicted`.**
A symmetric merge is a fixed point: merge the same pair again and nothing moves.

**This is not last-write-wins.** LWW is defined by what happens to the loser: it is gone. Here the
loser is a line in the file, one the user can read, grep, query, group a board by, and promote back
in a tap. The whole distinction is that nothing is discarded, and that is exactly why it does not
touch `merge.rs`'s *"we never resolve that by fiat"*. Fiat is **choosing between two pieces of
content by destroying one**; this chooses which one the field displays while keeping both. If the
companion key were ever dropped instead of shown — by a merge that overwrites it rather than uniting
it, by a serializer that skips reserved keys, by a UI that hides them — this would become fiat that
day, and the tests named below are what would fail.

**What the July ruling got right, and what changed underneath it.** It was right that the *then*
available alternative was worse: with nowhere to put the loser, "let ours win" meant a Clean merge,
an auto-commit at 5s, a push, and one person's column silently overwriting another's. It ruled
against that, correctly. What it did not have was a third option. `extra` already round-trips unknown
keys losslessly (`frontmatter.rs`), so a place to keep the loser has existed the whole time and was
simply not used. The ruling also priced "loud-and-absent" as loud. It is not: a fence-broken note
disappears from every view, and the 39-day phone freeze (above) is what that costs in practice.

**Clause by clause.** *Both values survive in the file* — yes, and demonstrably: the winner in the
field, the loser in `conflict-<field>`. *The file parses* — yes, and this is the change: today it
**does not**, because the markers land inside the YAML fence. *`Conflicted`, never Clean* — **no.
This is the clause deliberately not met.** It was a proxy for the reason stated in the same sentence,
and the reason is met by determinism. Keeping it would keep the note unmerged, which under the
principle this project now works to (*a disagreement never stops anything else*) is the failure, not
the safeguard: it is the shape that froze 202 notes for 39 days.

**The costs, stated rather than argued away.** (1) **A demoted value is quieter than a frozen note.**
A note that vanishes is eventually noticed; a `conflict-status` line can sit in a file for months. (2)
**The losing device's user sees their card move.** That is real — though it moved before too, by
vanishing from the board entirely. (3) **The keys accumulate** if nobody resolves them, which is the
documented failure of every conflict-copy system (Syncthing capped its own at 10). Nothing here caps
them; the mitigation is a surface, not a promise that people tidy up.

**So this ruling is conditional on the surface, and the condition is not satisfied yet.** What ships
with it: the note is readable, so the demoted key is visible wherever properties are — the note's own
frontmatter, the editor, a board grouped by it. What is owed: the persistent chip listing notes with
a demoted value, alongside *unrecorded* and *unreadable*, and the one-tap promote. That is the same
debt `outstanding.md` §2.12 already carries for auto-kept notes, and it is now the same debt. **If
that surface is not built, this ruling should be revisited rather than left standing** — a demotion
nobody can see is the fiat this entry spends its length denying.

**Tests that hold it** (`crates/fm-cli/tests/merge.rs`, `conflict_resolution_both_devices.rs`, both
backends): the card-drag case keeps both values and **parses**; the merge is a fixed point (merging
the result again changes nothing); the two devices produce byte-identical files from the same pair of
commits; a second, later divergence **adds** to the set rather than replacing it; and a demoted value
that equals the winner is dropped, because that is agreement, not disagreement.

**One structural rule falls out and is load-bearing: `conflict-*` keys merge as a *set*, never as a
scalar.** The key is its content, so agreement is structural — the same rule that governs `tags`,
`assets`, `code` and `manifest.json`. Running the scalar rule over them would let a divergence *in
the record of a divergence* demote itself into `conflict-conflict-status`, which is both absurd and
unbounded.

**And it is a three-way set merge, not a plain union — this is the one place that deliberately
differs from `tags`.** Deleting the line is how a user says "I have looked at this", so it is the
undo, and under a union the undo does not exist: the first pull from a device that has not looked yet
re-adds the value, that device pulls back and re-adds it again, for ever. A union is right for tags
because the cost of resurrection is a tag you re-delete in a second; here it is a warning you cannot
dismiss. So an element survives only if a side still has it **and neither side that had it in the
base deleted it** — symmetric in ours/theirs, so both devices still compute the same bytes. The
companion rule does the rest: a demoted value equal to the winner is dropped, which is what makes
*promoting* the loser stick everywhere rather than only on the device that did it.

## 2026-09-07 — the phone re-merges what libgit2 already merged, and a stale test binary hid why `#git` `#sync` `#track-m`

**Decision.** After `repo.merge`, `git_native::pull` runs `merged_text` over every path **both sides
changed** — not only the ones libgit2 left conflicted. `settle_the_paths_libgit2_merged_itself`.

**Why: the two devices were producing different bytes from the same pair of commits.** On a desktop
`git merge` invokes `fm merge-md` for every path both sides changed, so `merge_texts` reparses the
note and re-emits its frontmatter whole. The phone only revisited *conflicted* paths, so a note
libgit2 could line-merge never passed through our rules. That is harmless while the two answers
agree — and they stop agreeing the moment a note's frontmatter is not already what `to_file` writes:
a different key order, a two-space list indent, a missing `schema:`. Anything an external editor, an
import, or an older version of this app left behind. The desktop then commits the canonical form,
the phone commits the original, and **the two devices conflict with each other for ever over a note
neither of them edited.** `merged_text`'s own doc comment already promised the opposite: *"a phone
and a desktop cannot disagree about what a merged note is."* This makes that true.

**The trigger is narrow and the consequence is not.** Both sides must have changed the file *and*
their `updated:` lines must agree — otherwise the path conflicts and the existing loop already
handles it. Since the app rewrites `updated` on every save, that means two saves in the same second,
or an edit made outside the app. Rare. Permanent when it happens, and invisible: both devices
report a clean merge.

**The reason it was invisible for months is worth more than the fix.** `crates/fm-cli/tests/*`
copy the built `fm` beside the test binary so `git::ensure_repo` can find it — otherwise
`install_merge_driver` *clears* the driver and the "desktop" half of every two-device test silently
measures bare git. The copy was guarded by `if !dst.exists()`, so whatever a previous run left there
answered for ever: on 2026-09-07 it was a **six-week-old `fm`**, and three suites had been grading a
build nobody had made since. `conflict_resolution_both_devices.rs` did not install it at all, so its
desktop half was never our code. Both are fixed — copy when missing **or stale**, atomically — and
`a_clean_merge_is_byte_identical_on_both_devices` carried a doc comment confidently explaining that
a `merge_texts` mutation leaving it green was *correct rather than a gap*. It was the gap.

**The general lesson, for the traps list: a test fixture that is cached by existence is a test
fixture that expires silently.** The assertion still passes, the name still reads true, and what it
grades is whatever was there first.

## 2026-09-07 — a conflict blocks its own notes and nothing else, and this is what it took `#git` `#sync` `#track-m`

**This implements a ruling that was written the same day and shipped as prose only.** The entry *a
merge never stalls on a question whose safe answer is a note* ends with: *"So `commit_all` commits
`owned` minus the conflicted paths rather than refusing wholesale."* It did not. **Both** backends
still returned early on any unmerged path, and nothing anywhere recorded the gap — so the ruling read
as shipped for as long as nobody checked. That is the drift this file exists to catch, and finding it
inside a day was luck rather than process.

**What the owner actually asked for**, after resolving the phone by hand: *"this is not a nice failure
mode: the user does not know about merge conflicts and so on, so all the rest should be committable
and synchable."* Two stuck notes had kept 202 out of history for 39 days, 196 of them brand new.
Auto-settling delete/modify removed *that* trigger; it did nothing about the next one. A single
`BothModified` still froze everything.

**The decision, restated so it is testable:** a conflicted path is never staged — that is absolute,
since staging one is how git is told a human resolved it and would publish `<<<<<<<` as a note's
content — and every *other* path commits normally. The conflict still waits for a person, which is
the owner's ruling that marker conflicts keep blocking; it waits about those notes, not about the
vault.

**Git will not write a tree from an index holding a conflict entry.** That is not a policy, it is
`GIT_EUNMERGED`, and `git commit --only <paths>` is refused during *any* merge besides. So the tree is
assembled outside the real index, and the two backends do it differently enough to be worth naming:

- **Subprocess:** a temporary index seeded from `HEAD` via `GIT_INDEX_FILE` — the same trick
  `write_proposal_branch` already used — filled from the *real index's* blob ids with
  `update-index --cacheinfo`, then `write-tree`, `commit-tree -p HEAD`, `update-ref`.
- **libgit2:** an in-memory `Index::new` + `read_tree(HEAD)`, entries added by blob id, then
  `write_tree_to`. `add_path` is unavailable on an index with no repository behind it, which is the
  same constraint `merged_text` already documents.

Both write a **single-parent** commit. Writing `MERGE_HEAD` as a second parent would end the merge
with conflicts still in the index, and `finish_merge_if_resolved` — the thing that actually completes
it — would then never run.

**The hazard, and the asymmetry in how it is defended.** `finish_merge_if_resolved` builds the merge
commit from the **real** index, so a path committed around the conflict but left at its old content
there is silently reverted the moment the merge completes: every note written during the conflict,
deleted at the exact moment the user fixes the thing that was blocking them. On the subprocess
backend this cannot happen — the committed set is *derived from* `git diff --cached`, and the tree is
built from those same index entries, so the two are the same data. On libgit2 they are two separate
steps and it is a real hazard; the test mutation that skips the index write fails there and only
there. **Stated rather than smoothed over**, because "both backends are safe" would hide that only one
of them is safe by construction.

**Four user-facing claims were false the moment this landed, and all four were live.** They were
written when a mid-merge commit committed nothing, so `!committed` was a sound proxy for "mid-merge".
It is now the *rare* case, and each of these was conditioned on it:

1. `App.svelte`'s auto-commit banner — *"Nothing in this vault is being committed until they are
   settled"* — would have gone silent in the case that now happens, and was untrue when it did fire.
2. `record_unrecorded`'s reason — the same, on the one button whose job is to rescue unrecorded
   notes, and the count would have dropped to zero as if all were well.
3. `sync.svelte.ts`'s `commitStep` gate would have **pushed past an unfinished merge**, surfacing
   git's refusal instead of a named phase.
4. `BackupPanel`'s `bringDown` gate would have pulled into the same refusal.

The pattern is worth more than the four fixes: **a message conditioned on a failure disappears when
the failure stops happening**, and it disappears quietly. Every one of these is now keyed on the
conflict list itself, which is what actually says a merge is unfinished.

**What is still blocked, said plainly, because the win is narrower than it sounds.** Commits go
through; **sync does not.** `pull` and `push_squashed` both refuse over an unfinished merge, and that
is unchanged and correct. So the honest sentence for a user is *"everything else here is being saved
to history as usual, but this vault cannot sync with the other device until these are settled"* —
which is what the panel now says. The gain is real and specific: notes stop existing only as
untracked files on one device, and the *unrecorded* count stops climbing.

## 2026-09-07 — a prose conflict is not turned into two notes; the plan item is withdrawn `#git` `#sync` `#ui`

> SUPERSEDED IN PART by *the merge unit for prose is a sentence, and `zdiff3` is declined on
> measurement* (2026-09-08). The withdrawal of Pass 2 stands. What changed is the last section: the
> paragraph-oriented half of the queued "cheap thing" shipped, and the `zdiff3` half was measured
> against a real residual conflict and **declined** — on a one-line paragraph it hoists nothing and
> only adds a third copy.

**Decision.** A body two people wrote differently keeps both versions **in the body**, with markers,
and waits for a person. It is not split into a second note with its own id. The approved conflict
plan's Pass 2 — *"a divergent body becomes two notes"*, the `.sync-conflict-*`-style copy that
`MASTERPLAN.md:68` and `:404` have described for months — is **withdrawn**, by the owner, on being
shown what it contradicted.

**Why it was proposed.** It is the one mechanism in the surveyed field (2026-09-07) that loses
nothing *and* never blocks: family #5, the in-app conflict item — Joplin's `Conflicts` notebook,
Evernote, Bear, Standard Notes. It follows from the same idempotence rule everything else here
follows: prose is not idempotent, two paragraphs concatenated are not the note, so "keep both" for a
body cannot mean one file and has to mean two things a person can read.

**Why it is not being built.** Three reasons, in the order that decided it.

1. **It reverses a ruling the owner made the same day**: auto-settle the kinds with a safe answer,
   and *keep blocking marker conflicts*. The plan did not flag that, which is exactly the accidental
   contradiction `CLAUDE.md`'s four questions exist to catch — and it was caught by asking, not by
   the process.
2. **Most of its motivation was the freeze, and the freeze is fixed.** Pass 2's headline was *"the
   vault never freezes"*. That is now true without it: a conflict blocks its own notes and nothing
   else (above). What remains blocked is *sync*, and a conflict copy would not unblock that either —
   `pull` refuses over an unfinished merge whatever the working tree looks like.
3. **It is insurance against a case that has never happened here.** The owner's vault has **zero**
   notes with conflict markers and no note has ever been touched by a merge commit. The case that
   actually bit was delete/modify, settled; the second was a frozen vault, settled.

**What it would have cost, recorded because a future proposal will hit the same wall.** The new
note needs a **content-derived** id — a ULID whose timestamp is the losing side's `updated` and whose
random half is a hash of the losing body — or two devices merging the same pair of commits mint two
different copies and the merge stops being a fixed point, which is the property Pass 1 was built to
establish. And the copy cannot be made in the merge driver (git stages nothing it did not ask for),
so it belongs in both backends' `pull`, beside `keep_notes_the_other_side_deleted`. Neither is hard;
both are more machinery than a case with no occurrences deserves.

**The cheap thing that is *not* withdrawn**, because it reduces how often the question arises at
all: `merge.conflictStyle = zdiff3` and a paragraph-oriented diff cut spurious prose conflicts. That
is a change to how a conflict is *presented*, not to what one means, and it stays on the queue.

**What would reopen this:** prose conflicts actually happening — a vault where notes acquire markers
and sit with them. Then the argument changes from insurance to evidence, and this entry is the
starting point rather than the answer.

## 2026-09-07 — the demotion's condition is met: a chip, a panel, and two writes `#ui` `#sync` `#data`

> SUPERSEDED IN PART by *one chip for everything the merge settled* (below): the chip is no longer
> labelled **both answers** and `DemotedPanel` is now `KeptPanel`, shared with kept notes. Everything
> here about why the surface must exist, and about its three deliberate abstentions, still stands.

**Not a new decision — the discharge of one.** *A divergent field keeps both, by demoting the loser
into a field beside it* (above) made itself conditional in writing: *"a demotion nobody can see is
the fiat this entry spends its length denying"*, and *"if that surface is not built, this ruling
should be revisited rather than left standing."* It is built, so the ruling stands, and this entry is
the receipt — because a condition attached to a ruling and then quietly not met is the same drift as
a ruling written and not implemented, which happened twice in this file today.

**What it is.** A `demoted` read arm scanning for `conflict-*` keys, a **both answers** chip beside
*unreadable* and *not in history*, and `DemotedPanel` — *"`status` is **done** here; the other device
said **doing**"* — with **Keep "doing"** and **Dismiss**.

**Three things it deliberately does not have.**

- **No state of its own.** The disagreement is a key in the note's own frontmatter, so the list is a
  scan: right after a restart, right on the other device, and readable in any text editor. This is
  the half the kept-note chip cannot copy, and why that one is still owed.
- **No new way to write a property.** Both buttons call `set_property` — promoting sets the field
  and clears the record, dismissing clears the record alone. Handing the panel typed values would
  mean a second set of rules about what a value means.
- **No new query predicate.** A full scan, the same shape as `duplicates`. Nothing about `extra`
  reaches SQL — `objects` denormalises only `kind` — so even a `Prop`/`Exists` predicate would be
  evaluated in Rust over hydrated candidates. A prefix filter costs exactly what a predicate would
  and says what it means, and `fm-query`'s DSL does not grow for one chip.

**Dismiss sticks, and that is the merge's doing rather than the panel's.** `merge_demoted` honours a
removal by a side that had the value, specifically so this button is not undone by the next pull —
under a plain union the record would come back from whichever device had not looked yet, for ever.
The two halves were designed together and only work together.

**The chip counts notes, not rows.** Two diverged fields on one note is one thing to look at; a chip
reading "2" for a single note would misstate the size of the problem, which is the whole job of a
chip.

**Deliberately the quietest of the three chips** (no `moved` class): *unreadable* means a note is
missing from every view and *not in history* means a note exists in one place only. This one means
everything is fine and there is a choice waiting. Ranking them the same would flatten exactly the
distinction the surfaces exist to draw.

## 2026-09-07 — the app says how long a vault has been quiet, and that is the alert that depends on nothing `#ui` `#sync` `#git`

**The last item of the conflicts plan, and the one that is not about conflicts.** Two notes froze a
vault for thirty-nine days. The cause is fixed (`47750fb`), the conflict *kinds* are surfaced, a
blocked merge no longer stops the other two hundred notes committing — and none of that answers the
question the incident actually raises: **why did it take thirty-nine days to notice?**

Because every surface in the app reports something a detector *found*. "Unreadable" needs the marker
scan to find a note. "Not in history" needs `unrecorded` to run. "Both answers" needs a merge to have
demoted something. Each is silent when its own detector is missing or blocked — and during those
thirty-nine days the detectors were exactly that: the conflict kinds were not listed anywhere yet,
and `commit_all` refused before `unrecorded` was consulted. A vault that cannot save looked identical
to a vault nobody had written in.

**So: one signal that is incurious about the cause.** `last_commit` — the committer time of `HEAD` —
and the app says *"lab: 39 days since a save"*. It cannot be blocked by what it would report, because
it asks git one question and git always answers it.

**Committer time, not author time.** A commit pulled from the other device was authored whenever they
wrote it and committed here today; "nothing has been saved *here* in N days" means the latter. The
subprocess backend reads `--format=%ct` and libgit2 reads `Commit::time()`, which is the same clock —
`git_differential` pins them to the same integer against a commit whose author and committer dates
are seven weeks apart, so the two cannot be confused by accident.

**Fourteen days, and the size is the argument.** A week off is a holiday. A chip that fires on one is
noise, and noise is read past — which is how thirty-nine days would have passed anyway. Fourteen also
makes *"in weeks"* true of everything this ever reports, so the plural wording stays honest without a
number in it.

**Never-committed is a third state, not a large number.** `null` read as `0` is 1970, which renders
as twenty thousand days of silence aimed at the one person who has done nothing wrong yet. A vault
that has never saved has not *stopped* doing anything; the welcome screen and *not in history* own
that case. A commit dated in the future — two devices, two clocks — is likewise not quiet.

**Its own command rather than a field on `backup_status`.** Same reasoning that put `identity` on
`VaultInfo` and `backup_latest` beside it: `backup_status` spawns a network `ls-remote` per vault and
is polled on a timer, in production only. This is one local `git log -1`, so the answer is there
offline, at first paint, on the phone — which is the device whose vault this actually happened to.
The threshold is deliberately **not** in the backend: when silence is worth mentioning is a display
policy, and it belongs beside the sentence it produces, where a test can read that sentence.

**Loud, unlike the demotion chip.** `both answers` deliberately has no `moved` class because it means
everything is fine and a choice is waiting. This one means something may be wrong and nobody has been
told, so it is filled like *"someone pushed"* — and it opens the Backup panel rather than committing
on the click, because weeks of silence has more than one cause (a merge waiting on a person, a remote
never set, or simply nobody writing) and the panel is where those are told apart. The panel states
the same fact for **every** vault, remote or not, in its own list item outside the remote branch: a
vault with nowhere to push is precisely the one whose silence nobody would otherwise notice.

**It is re-read after a backup**, which is the 2026-08-24 lesson applied before it can be reported
again: the *not in history* count was loaded once per vault-list change, so backing up recorded the
notes and left the chip showing the old number — indistinguishable, from the only screen the owner
uses, from a backup that did not work. An alert that survives the act that answers it stops being
read.

**What would reopen this:** a vault the owner genuinely uses twice a year, nagging forever. The
honest fix then is not a higher threshold but a per-vault "this one is an archive" — and that is a
setting, so it waits until there is a real vault asking for it rather than a hypothetical one.

## 2026-09-07 — a resurrection is a fact about the merge, so that is where it is recorded `#git` `#sync` `#ui`

**The debt:** `outstanding.md` §2.12. When a pull keeps a note the other device deleted, the app
said so in a step line and a dismissible banner — both gone by the next screen, for a decision the
app made on the user's behalf. §2.12 asked for a persistent surface and assumed it would need state:
*"`VaultSync.kept` is cleared at the start of every run by design, so a chip needs its own state."*

**It does not, and that assumption is the interesting part of this entry.** Every persistent alert
in this app is a derivation over on-disk truth — *not in history* reads `git status`, *both answers*
reads frontmatter, *N days since a save* reads `HEAD`. This one reads the **merge commit that did
it**: both backends append a `Kept: <path>` trailer to the merge message, and `kept_notes` walks
merge commits and parses them back. Nothing is stored, so the answer is right after a restart, on a
fresh clone, and — the decisive case — on the device that never ran the merge.

**That decisive case is what killed the two tidier designs.** The ordinary sequence is: the phone
deletes note N and pushes; the laptop had edited N, pulls, keeps it, pushes. **The phone then
fast-forwards** (`git_native.rs`'s `Merged { incoming: 0, kept: [] }`) and N reappears on the device
belonging to the person who deleted it, with no keep branch ever running there. So a `refs/fm/kept/…`
ref per kept note — the tidiest mechanism on the table — reports nothing on the one device whose
user is most owed the notification. A merge-commit trailer survives the fast-forward because the
merge commit *is* what got fetched.

**What was proposed first, and why it was wrong.** The obvious move was to generalise the demotion:
a divergent *field* keeps both with the loser in `conflict-<field>`, so a divergent *existence*
keeps both with `conflict-deleted: true`. Three defects, found by audit before any of it was
written, and the first is a live bug:

- **`merge_demoted` would silently erase it.** It computes `field = key.strip_prefix("conflict-")`
  → `"deleted"`, then `winner = as_written(&m.get("deleted"))` and `losers.retain(|v| *v != winner)`.
  `Object::get` falls through to `extra`, so a note carrying an ordinary user property `deleted: true`
  — a common soft-delete convention — empties the loser list and `m.extra.remove("conflict-deleted")`
  fires. The chip goes quiet on every device, with no user action, at the next merge touching that
  note. That is precisely the failure *a divergent field keeps both* forbids by name.
- **It requires the note to parse**, and the note that gets resurrected is exactly the one that might
  not — an unparseable note is what the *unreadable* chip is for and what froze the phone.
- **The keep branch has no path filter.** It settles every delete/modify path in the repo, so
  `manifest.json`, a `.view` file and (where `git_assets_max` is set) a tracked binary would each
  have had YAML written into them.

The trailer has none of these: it works at the path level — the level the merge itself works at —
and changes **zero note bytes**, so the load-bearing invariant that two devices produce byte-identical
files from the same pair of commits is untouched rather than re-argued. It also reports the
resurrections **already in the vault** from the 2026-09-07 keep, which a marker written from now on
never could.

**Ordering is load-bearing, and it is the one thing that had to change in the keep branch.**
`resolve_conflict` finishes the merge itself once the last conflicted path is settled, so the paths
are now collected *before* the resolve loop and the trailer written into `.git/MERGE_MSG` while the
merge is still open. Written afterwards it lands in the worktree of an already-committed merge and
is never recorded at all — proved red, not reasoned about.

**Merge commits survive `push_squashed`, which is what makes the trailer durable.** `newest_foreign`
classifies by message prefix, and a merge subject is neither `auto:` nor `backup:` — so a merge
commit becomes the squash floor rather than being collapsed into one. Checked in both backends
before this was built; had it gone the other way the whole design would have been unusable.

**Acknowledgement is per device, and that is a decision rather than a limitation.**
`refs/fm/kept-seen` is a watermark at the HEAD the user looked at, on the `retain_proposal_tip`
precedent — outside `refs/heads/*`, so every history walk is unchanged, and unlike a retained
proposal tip it points at a commit already reachable from HEAD, so it retains no objects and that
ruling's accretion cost does not apply. It is per device because the two devices are not asking the
same question: on the one that kept the note it is *"you edited this, they deleted it"*; on the one
that deleted it, it is *"you deleted this and it is back"*. Both people are owed an answer.
All-or-nothing rather than per row, because a watermark answers *"have you looked"* — and looking is
not something you do to one row. The per-row action is deleting it again, which needs no record: the
note is gone at HEAD everywhere, so the row cannot return.

**What would reopen this:** the owner ruling that acknowledgement must be vault-wide. Then the
repaired form of the rejected proposal wins — a reserved key **outside** the `conflict-` prefix,
where the ordinary scalar `three_way` gives removal-sticks semantics for free — and it should be
adopted with its costs (the note must parse; the marker is a visible pseudo-property) stated.

**A defect caught on the way to the commit, and the asymmetry that caused it.** `kept_notes`
degrades to "nothing was resurrected" for a vault with no repo and for one with no commits, on both
backends, deliberately. `mark_kept_seen` did not — the subprocess arm ran `rev-parse HEAD` and the
native arm peeled `HEAD`, and both propagated the failure. `dispatch`'s `kept_seen` walks **every
vault in scope with `?`**, so a single vault that had never been backed up would have made the
acknowledge button fail for all of them, including the vault the user was looking at when they
pressed it. The two halves of a pair have to agree about what "nothing" is; the differential now
pins it, in the shape `last_commit_agrees_on_the_moment_and_on_never` established.

## 2026-09-07 — one chip for everything the merge settled, superseding the "both answers" chip in part `#ui`

> Supersedes in part *the demotion's condition is met: a chip, a panel, and two writes* (above): the
> chip is no longer labelled **both answers** and `DemotedPanel` is now `KeptPanel`. Everything that
> entry says about *why* the surface must exist, and about its three abstentions, still stands.

**Kept notes were owed a chip of their own. It would have been the sixth.** `App.svelte`'s own
media query records what that costs, measured on a device: *"the toolbar wrapped to FOUR rows and
the board began below the halfway mark of a 2400px screen"* — with `.topbar { flex-wrap: wrap }` and
a 2.75rem minimum on every coarse-pointer chip, two chips per row is the honest ceiling on a 390px
phone. The whole section exists to undo that, and adding a sixth chip would have re-earned it on the
one device where delete/modify conflicts actually happen.

**They are one category anyway**: *the two devices disagreed, the merge chose, nothing is blocked.*
So one chip — **N decided for you**, counted by note across both halves — opening one panel with two
sections, each keeping its own sentence and its own action. Still the quietest chip (no `moved`
class): *unreadable* means a note is missing from every view, *not in history* means a note exists
in one place only, and this one means everything is fine and a choice is waiting.

**The sections do not share an action, and that is why they are sections rather than rows.** A
demoted field goes through `set_property`, the path a person typing the value takes. A kept note's
button is `delete` — a different risk class, irreversible from a phone with no shell, which is the
asymmetry the keep ruling itself rests on. So it **arms before it fires**, matching `NotePanel`'s
confirm strip and `BackupPanel`'s Remove/Cancel; a one-tap delete in a list somebody opened to read
is the outlier a stray thumb finds. A kept path that is not a note gets no button at all.

**A staleness bug was fixed on the way, and it was already shipped.** `loadDemoted` ran at boot and
from its own panel and nowhere else — so the chip for a disagreement the current pull produced did
not appear until the app restarted, making the *persistent* surface less prompt than the banner it
was meant to outlast. Both loaders now run after `getTheirChanges` and after a backup, which is the
2026-08-24 lesson (*"the count is a fact about git, and this is the moment git changed"*) applied to
a second surface before it could be reported a second time.

## 2026-09-08 — the merge unit for prose is a sentence, and `zdiff3` is declined on measurement `#git` `#sync` `#data`

> SUPERSEDED IN PART, same day, by *a conflict marks the sentence, and only where narrowing is
> provably free* (below). The
> measurement, the reasoning and the `zdiff3` refusal all stand. What changed is one clause: this
> entry said the finer pass's answer is taken **only if it comes back clean**, so *"a conflict
> either becomes clean or stays byte-for-byte what it is now"*. The conflicted answer is now taken
> too — it is the one that knows which sentence is in dispute — so a conflicted body's bytes **do**
> change, and its line structure with them.

**Decision.** When a body merge conflicts, the text merge is **run a second time with a sentence as
the unit instead of a line**, and its answer is taken only if it comes back clean. `merge.conflictStyle
= zdiff3` is **not** adopted. Together these close `outstanding.md` §2.13, the half of the withdrawn
Pass 2 that was kept as *"the cheap thing that is not withdrawn"*.

**The problem, and it is one we create.** A line merge asks *"did you both change this line?"*, and a
Markdown paragraph is one line — the editor is a textarea and nothing wraps. So two devices editing
two different sentences of a paragraph collide over prose neither of them touched, and the note
blocks on a disagreement that does not exist.

**Measured before it was built, on this repo's own vault (2026-09-08, 771 body lines).** 26% of body
lines carry more than one sentence. Of the collisions two independent edits can have on a line, 54%
are between *different* sentences, and **30% are between sentences far enough apart that the merge
settles them**. On a bullet-shaped corpus (a 336-note Logseq vault) the same figures are 67% and 48%.
So: roughly one prose conflict in three stops happening, and not one of the ones that stops is a
disagreement. The gap between 54% and 30% is the part worth knowing — see the boundary below.

**Why this is not a third merge engine, which `merge.rs` forbids by name.** It is the same
`text_3way`, handed the same prose cut at a smaller seam. Nothing new decides anything: a sentence
that both sides changed still conflicts, and the winner of nothing is chosen by anybody here.

**Why it cannot make anything worse, and this is control flow rather than a test.** The ordinary line
merge runs **first** and its answer stands unless it conflicted; the finer pass is consulted only
then, and only its *clean* results are taken. A merge that is clean today therefore cannot change,
and a conflict either becomes clean or stays byte-for-byte what it is now. There is no third outcome
to test for. (Recorded because it surprised me: moving the finer pass *in front* changes no test —
a split only ever pushes two changes further apart, so a line-clean merge is sentence-clean and
reconstructs the same bytes. The ordering is not what makes the answers right; it is what makes the
blast radius nil without anyone having to trust that argument.)

**The one property it rests on:** `join_sentences(split_sentences(x)) == x`, exactly, for every `x`.
A clean merge is a concatenation of whole units taken from the three inputs, so decoding its result
is the same operation as decoding an input. The cut is `[.!?]` plus at least one space, and only
where two word characters precede the punctuation — which is what keeps `1. ` and `e.g. ` from
becoming units of their own, tiny repeated units being exactly what makes a diff align two unrelated
places. It is tested by round trip over hand-picked shapes *and* 2000 generated strings, because the
hand-picked ones are the ones I thought of.

**The boundary, and it is deliberate: two edits to *adjacent* sentences still conflict.** A 3-way
merge will not merge two changed lines with nothing unchanged between them. Separating the units
with blank scaffolding lines would make git merge them — and that would be this module overriding
git's judgement about whether two touching edits interact, which is the thing it exists not to do. A
conflict git would report is not ours to talk it out of. This is where the 54% becomes 30%.

**`zdiff3` is declined, and the measurement is the reason.** Its benefit is proportional to how many
lines a conflict region spans: it shows the base and *zealously* hoists common lines out of the
region. A Markdown paragraph is one line, so there are no common lines to hoist, and on a real
residual conflict it does exactly one thing — adds a third full copy of the paragraph:

```
<<<<<<< ours
The vault syncs over git. Every note is one Markdown file. The phone runs libgit2.
||||||| base
The vault syncs over git. Every note is one file. The phone runs libgit2.
=======
The vault syncs over git. Every note is a single file. The phone runs libgit2.
>>>>>>> theirs
```

The very property that made the sentence rescue necessary is what makes `zdiff3` useless here, and
after the rescue the residual conflicts are *precisely* the one-line-paragraph cases where it helps
least. Against that, the user of this app reads markers in a phone textarea and is told to delete
them (`SkippedPanel`); a third block is one more version to recognise as not-an-answer. **Both
engines do support it** — `--zdiff3` in git ≥ 2.35, `GIT_MERGE_FILE_STYLE_ZDIFF3` in libgit2 1.9.4,
verified present in the `libgit2-sys` we link — so this is a one-flag change in two places if the
evidence ever turns. It is declined on what it does to *this* file shape, not on principle.

**A correction to §2.13's own premise, worth keeping.** `merge.conflictStyle` is **inert** for notes:
`.gitattributes` says `*.md merge=fm`, so git never runs its own text merge on a note and never reads
that setting. The equivalent knob is a flag on our two engines, which is where the paragraph above
looked for it.

**Graded on both backends**, because a desktop and a phone that rescued a paragraph differently would
each commit their own answer and re-derive the conflict on every pull afterwards — 200 generated
two-device rewordings, byte-for-byte and verdict-for-verdict, in `merge_differential.rs`. The
transform is shared code either side of a routed call, so divergence was not expected; it is graded
because "not expected to" is what the second engine exists to stop anyone saying.

**What would reopen this:** a corpus where paragraphs span several lines — someone importing
hard-wrapped Markdown — which is where `zdiff3` earns its keep and where the sentence rescue matters
less. Both halves are shape-dependent, and the shape was measured once, here, on 2026-09-08.

## 2026-09-08 — a conflict marks the sentence, and only where narrowing is provably free `#git` `#sync` `#data` `#ui`

**Decision.** When the sentence-granular merge conflicts too, **its** output is what the user gets,
so the markers wrap the sentences actually in dispute rather than the paragraph containing them —
**unless handing it over would restructure the note**, in which case the line merge's answer stands
byte for byte. `outstanding.md` §2.14 is closed.

**The problem.** After the sentence rescue shipped this morning, the conflicts that remain are the
ones where two devices edited the *same* or *adjacent* sentences. Their markers still came from the
**line** merge, so a one-line paragraph was printed twice, nearly identically, and finding the
sentence that differed was the reader's job — on a phone, in a textarea. The finer merge had already
worked out which sentence it was, and we were throwing that away.

**The shape that makes this safe, and it is the decision.** A marker has to occupy a whole line, so
narrowing tears the paragraph — and a tear can change *block structure*, not just where the line
breaks fall. So the joined output is **validated**, and `join_conflicted` returns `None` when it
would not be free. Then `merge_body` falls back to the line merge's output, unchanged. **The §2.13
guarantee therefore survives intact: the finer pass can improve a conflict or leave it exactly
alone, and there is no third outcome** — which is what I wrongly claimed by *restricting* the finer
pass to clean results, and can now claim by checking instead.

**What the audit found, because none of it was in my proposal.** Three agents, one adversarial:

1. **An invented blank line — blocking, and self-compounding.** A source line ending
   `<stop><space>` makes the splitter emit an *empty* unit before the newline. The closing marker
   has already ended the line, so keeping that unit's newline too produces a **blank** line: one
   paragraph becomes two, in the renderer, permanently. It compounds, because every line this
   transform tears ends in `". "` — so a note that has had one conflict is primed to trigger it on
   the next. Fixed at the cause: an empty line immediately after a marker contributes nothing.
2. **Two-space sentence spacing became hard breaks.** Every tear lands just after a full stop, and
   two spaces at a line end is a Markdown `<br>`. A torn line now gives up its trailing spaces —
   the newline is what that spacing was for. The fixed-point test now asserts the stronger thing:
   replacing each newline with a space returns the original paragraph, character for character.
3. **`is_marker` read ordinary content as a separator.** `A rule follows. =======` is a sentence;
   the same seven `=` are a separator only *between* an opening marker and a closing one. Markers
   are now found by **scanning** the merge's output, not by asking each line about itself — and the
   `join_conflicted` doc had named this exact hazard while committing it one function later.
4. **A fenced code line was torn into invalid code, and stayed torn.** `structured` guards indented
   code and table rows, but it may only read one line and a ``` fence is not visible that way. The
   whole-output scan catches it and declines; the line merge hands the user two intact candidate
   lines, which for code is the better answer. Under §2.13 this cost was nil, because a torn line
   only ever existed inside a merge that came back *clean* and was rejoined exactly.
5. **A torn sentence beginning `# `, `- ` or `> `** is prose inside a paragraph and a heading, list
   item or quote at the start of a line. Declined.

**Why a validator rather than five more rules in the joiner.** A whole-output scan is not part of
the split/join inverse, so it can look at anything it likes without endangering the one property
this rests on. And declining is cheap: the fallback is the behaviour that shipped this morning.

**The guard is asked at each *unit*, never at each source line — and getting that wrong is a real
bug.** `Intro. |ab. cd.` cuts once, and the `|ab. cd.` left behind is a table row as far as the
joiner can tell. If the splitter asks about the source line (which begins `I`) it cuts again, the
joiner declines to undo that cut, and the transform stops being an inverse. Found by hand before the
code was written; **worth recording that 2000 generated strings did not catch it and the one
hand-picked case did** — and that a later 400,000-string fuzz did not catch it either. Fuzzing
covers the shapes you did not think of; it does not cover the shape you had to reason about to think
of at all.

**Two joiners, not one with a flag.** `join_sentences` is the exact inverse and stays that way,
because a body may legitimately contain a line of `=` characters and the clean path must not read it
as a marker. `join_conflicted` may, because that text is already being restructured. The marker test
is pinned to the `marker_size` **this call passed to the engine**, not to seven — otherwise both
backends would agree with each other while both were wrong, the class of bug a differential cannot
see, which is why it has a behavioural test of its own.

**The cost that is accepted, stated precisely.** A conflicted paragraph comes back split across
lines and stays split after the user resolves it. Each newline stands exactly where a sentence space
was, so the rendered paragraph is character-for-character the one that went in — that is now
asserted, not asserted-about. What does change: the note's **card preview** is the first non-empty
*line*, so a note that has had a prose conflict previews as its first sentence. Left alone
deliberately; changing `preview` is a behaviour change to every card in the app, and this is not the
change that makes it.

**And it withdraws a sliver of §2.13.** Two edits to different sentences of one table row or
indented code line merged cleanly this morning and now conflict, because `structured` will not cut
those lines at all. That trade is right: the rescue was worth having because a paragraph tolerates
being cut, and those lines do not.

**Unchanged on purpose:** `has_conflict_markers` (still the one definition), the guard that refuses
to commit marked-up text, and `merge_texts`' whole-file fallback — a note neither side can parse
never reaches `merge_body` and keeps line-level markers, which is right, because there is no
structure there to be finer about.

**Not resolution by fiat.** Nothing is discarded and nothing is auto-settled that was not settled
before. Only the **boundary** of the marked region moved, and only inwards, towards the
disagreement.
