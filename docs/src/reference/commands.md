# Commands

The frontend calls these over `POST /api/<cmd>` (JSON args, camelCase). Each is a
function in `crates/fm-app/src/commands.rs` (except the git/restic ones —
`commit`, `push`, `pull`, `backup`, `backup_status`, `set_git_remote` — which are the OS
seam, not `Store` operations, so `fm-core` is called directly; the vault-registry and
`.view` arms likewise live in `crates/fm-app/src/vaults.rs` and `crates/fm-app/src/views.rs`). Args map to
Rust snake_case.

Every one of them is reached through **`fm_app::dispatch`**, the single command surface.
`fm-serve` is an HTTP shell over it — it parses a request into `(cmd, args, body)`, calls
`dispatch`, and frames the answer. Adding a frontend means writing a new shell, not a
second copy of the table below.

**All 120 of them are below**, grouped by what they are for. If you add an arm to
`dispatch_inner`, add its row here — `ci/checks.sh` counts the two and fails when they disagree,
because a reference that is *nearly* complete is one a reader stops trusting. (That sentence was
itself untrue until 2026-09-05: the check tested membership only, never counted, and this line
said 81 while there were 85. It now checks the count and both directions, so a documented command
dispatch no longer has fails too.)

### Reading the vault

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `board`          | `groupBy`                    | `Board` (columns)      | group by any property |
| `agenda`         | —                            | `ObjectMeta[]`         | dated & not done, soonest first |
| `reminders`      | `from` (`YYYY-MM-DD`), `days?` (default 14, at most 60) | `Reminder[]` | read-only: the moments at which an unfinished note is due to be said — per note and per date (`start`, `due`) its `at` (`YYYY-MM-DDTHH:MM`, the note's time or 09:00 of its day, less its `remind` minutes), `what`, `when` (the note's own time where the reminder is earlier) and `all_day`. `remind: off` leaves a note out. Nothing is stored: the page asks again and hands the list to whatever shows it |
| `recent`         | —                            | `ObjectMeta[]`         | all notes, newest-created first |
| `gallery`        | —                            | `ObjectMeta[]`         | assets, newest first. **No renderer draws it** — the Gallery view was removed and a `.view` asking for `gallery` is listed as broken and says what to change it to; the command stays because `fm-cli` and the asset pickers use it |
| `search`         | `query`                      | `ObjectMeta[]`         | FTS5; empty query → `[]` |
| `get`            | `id`                         | `NoteDetail \| null`   | meta + full body |
| `backlinks`      | `id`                         | `ObjectMeta[]`         | every note whose body carries a `note:` link to this one |
| `templates`      | —                            | `ObjectMeta[]`         | the notes tagged as templates, for the new-note picker |
| `stale`          | `since`                      | `ObjectMeta[]`         | notes untouched in git since `since` (a `--since` value, default `90 days ago`), oldest first. **Derived from git, stored nowhere**; a vault with no history is skipped, not reported as wholly stale |
| `activity`       | `since`                      | `EditEvent[]`          | who last edited what, straight from git log. `since` defaults to `1 year ago` |
| `duplicates` (also documented with `prune_duplicates` under **Vaults**) | — | `DuplicateFamily[]` | notes grouped by identical body, and what pruning them would remove. A read: it changes nothing |
| `config`         | —                            | `Config`               | version, the vault list's path and whether it is writable, what is installed, and which `FM_*` variables are set — **facts about the machine, not about any vault**. It carried a copy of the vault list and a second list of restic repos until 2026-09-08; both came from `list_vaults`' producer, so this answered nothing it was the source of. Now takes no lock and spawns no git. The one call a support question can start from |
| `ping`           | `since`                      | `{changed, git}`       | the 15 s visible-tab reindex poll |

### Writing notes

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `capture`        | `body`, `vault`              | `ObjectMeta`           | creates a note |
| `set_property`   | `id`, `key`, `value`         | `{ previous }`         | writes one frontmatter field (see the table below), and answers the value it replaced (`null` if none), in the form it reads back — what undo sets again |
| `set_properties` | `id`, `props`                | `{ previous }`         | several fields of one note in one write; an empty value removes a field. `previous` maps each key to the value it had (`null` if none). Nothing is written unless every key is accepted |
| `create_notes`   | `notes`, `vault`             | `ObjectMeta[]`         | several notes in one call, each made whole: `{ title, body, tags, start, props }`. Every note is checked before any is written. A number in `props` must be whole. The only way to set `log` |
| `update_body`    | `id`, `body`, `base`         | the new `version`      | byte-for-byte body write; `base` is the `version` you last saw — a mismatch is refused (see below). `''` opts out |
| `delete`         | `id`                         | `DeletedNote`          | unlinks the file and both index rows, and answers the note's file and notebook so the UI's undo can put it back |
| `restore_note`   | `vault`, `file`              | `ObjectMeta`           | put a deleted note back from its file; refused if a note with that id exists again |
| `deleted_notes`  | `days` (default 30)          | `DeletedRow[]`         | read-only: notes deleted in the last `days`, read from each notebook's own history (git, or libgit2 on the phone), skipping ones that exist again |
| `restore_deleted` | `vault`, `id`, `days`       | `ObjectMeta`           | bring back a note listed by `deleted_notes`, exactly as it was before it was deleted |
| `create_paper`   | `input`, `vault`             | `ObjectMeta`           | a note from a pasted BibTeX entry, DOI, arXiv id, URL or bare title — parsed locally, never looked up online |
| `paper_bibtex`   | `id`                         | `String`               | that note's frontmatter rendered back as a BibTeX entry |

### Discussions

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `reply`          | `id`, `body`, `authorName`, `authorEmail` | `ObjectMeta` | post a [discussion](../user/notes.md#discussion) message on a note (or another message — it re-roots to the same thread). One file, ULID-named, in the target's vault. No `vault` arg: a reply joins the audience of the note it is about |
| `thread`         | `id`                         | `ThreadView`           | a note's discussion — `{root, count, messages[]}`, each message with a server-computed `depth`. `root: null` when the note itself was deleted (the discussion survives it) |
| `thread_roots`   | —                            | `ThreadRoot[]`         | every note that has a discussion, with its message count — what puts the badge on a card |
| `create_discussion` | `title`, `vault`          | `ObjectMeta`           | create a first-class [discussion](../user/notes.md#discussion) — a note that is the root of its own thread (`thread_of` points at itself). A dedicated command because `set_property` refuses `thread_of` |
| `discussions`    | —                            | `DiscussionSummary[]`  | every first-class discussion across vaults, most-recently-active first: `{…root, count, last_activity, participants}`. Participants come from git authorship. Comment threads on an ordinary note are *not* here — they stay with their note |

### Proposals

A [proposal](../user/collaboration.md#proposals) is a note carrying `proposes: branch:<name>`; the
change itself lives on that git branch. Nothing here writes to `main`.

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `proposals`      | —                            | `ObjectMeta[]`         | every open proposal across vaults, newest first. A store query like `recent`, not a git read — it lists proposal *notes*; branch state is derived elsewhere |
| `create_proposal` | `id`, `body`, `why`, `kind`, `tool`, `query`, `authorName`, `authorEmail`, `props` | `ObjectMeta` | write the draft to a review branch and the proposal note beside it. `props` (`start`/`due`/`location`/`addTags`) sets the note's details too; on a revision it goes over the details the proposal already carries, key by key. `kind`/`tool`/`query` record *how* it was produced — a model's `/propose`, `/research` or `/transcribe`, and what it searched for |
| `proposal_for`   | `id`                         | `ObjectMeta \| null`   | the open proposal against this note, if there is one |
| `proposal_diff`  | `id`                         | a unified diff         | what accepting would change, read off the branch |
| `proposal_content` | `id`                       | `{body, …}` or `null`  | the proposed body itself, for the side-by-side review, with the `start`/`due`/`location` it sets and the note's current ones (`wasStart`/`wasDue`/`wasLocation`). `null` when the id does not resolve |
| `proposal_shown` | `id`                         | —                      | records that a human actually looked at it. Idempotent, fire-and-forget: it must never fail the screen it is reporting about, and it is what separates *left alone* from *never displayed* |
| `accept_proposal` | `id`                        | `{outcome}`            | **merge** the proposal's branch and close it — not a body copy. `outcome` is `merged`, `conflicted` or `already_gone`; a proposal its author declined is refused |
| `reject_proposal` | `id`, `why`                 | —                      | close it with a reason. The outgoing commit is kept, so the *accepted*/*rejected* label stays true later |

### Assets and media

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `ingest`         | *(binary body)* `?name=`, `vault` | `ObjectMeta`      | upload → asset note; raw bytes, held whole in memory |
| `ingest_chunk`   | *(binary body)* `session`, `seq`, `vault` | `{session, received}` | one slice of a chunked upload, appended to `<vault>/.fm-ingest/<session>/part`. **`seq` is checked**: a lost or repeated chunk is refused where it happens, because the alternative assembles a file that hashes fine and is quietly wrong |
| `ingest_finish`  | `session`, `name`, `vault`   | `ObjectMeta`           | ingest the assembled file through `BlobStore::put_file` (which streams in 64 KB reads) and write the same asset note `ingest` writes. **The content address is identical either way** — chunks are transport, and the hash is taken once over the whole file |
| `ingest_cancel`  | `session`, `vault`           | —                      | abandon an upload and reclaim its bytes now. Not needed for correctness — abandoned sessions are swept at boot, by age — but a cancel that leaves a gigabyte on a phone until tomorrow is not a cancel |
| `resolve_asset`  | `reference`, `kind`          | bytes (ArrayBuffer)    | `kind` = `full` \| `thumb`; whole blob in memory — prefer `GET /api/blob/<ref>`, which streams and honours `Range` |
| `asset_status`   | `reference`                  | `{has_blob,has_thumb,mime}` | sniffed MIME |
| `open_external`  | `reference`                  | —                      | opens the blob in the OS default app |

### Recording and syncing (git)

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `commit`         | `message`, `vault`           | `{committed, conflicts}` | git-commit the vault. `committed: false` with `conflicts` non-empty is **not** "nothing to do" — the vault is mid-merge and nothing will be committed until those notes are settled |
| `push`           | `message`, `vault`           | `u32`                  | squash the unpushed window → push; returns commits squashed (0 on the first push) |
| `pull`           | `vault`                      | `{merged, conflicts}`  | fetch + merge through the `.md` driver; `conflicts` is a *result*, not an error |
| `set_git_remote` | `vault`, `url`, `name`, `email` | —                   | sets the vault's `origin`, and its git identity when `name`/`email` are non-empty (asked only of a vault git has never met); blank URL refused |
| `set_identity`   | `vault`, `name`, `email`     | —                      | the git identity alone, for a vault that already has a remote |
| `set_git_assets_max` | `vault`, `max`           | `VaultInfo[]`          | the size ceiling on attachments a push may carry. Above it, media stays local and only the notes travel |
| `unrecorded`     | —                            | `Unrecorded[]`         | per vault, the notes on disk that git does not have — **split by kind**, with a bounded sample. The count alone was not a diagnosis: 146 could mean 146 notes existing nowhere else, or 146 something was needlessly rewriting |
| `record_unrecorded` | `vault`                   | `{recorded}`           | commit exactly those |
| `duplicates` / `prune_duplicates` | `vault` (prune only) | `DuplicateFamily[]` / `{pruned}` | prune **refuses** while any copy is still unrecorded, and always keeps the oldest of each family. Deleting an untracked note is unrecoverable; a tracked one is a `git checkout` away, so the order is forced rather than warned about |

### Conflicts and unreadable notes

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `conflicts`      | —                            | `ConflictInfo[]`       | conflicts **with their kind**, because "open it and keep the text you want" is true of exactly one kind. Git is authoritative; notes whose text still carries markers are unioned in |
| `resolve_conflict` | `vault`, `path`, `keep`    | `{resolved}`           | `keep` = `theirs` \| `mine` \| `edited`. `edited` means "I reconciled both in the editor" and is refused while markers remain |
| `read_skipped` / `resolve_skipped` | `vault`, `name`, `text` (resolve only) | the raw text / `{parses}` | the in-app raw editor for a note that will not parse. Works **on any device**, unlike `open_skipped` |
| `open_skipped`   | `vault`, `name`              | —                      | hands the same file to the OS editor. Desktop only, by nature |
| `kept_notes` / `kept_seen` | —                  | `KeptNote[]` / `{seen}` | **notes a merge brought back** after the other device deleted them while this one was editing — read out of the merge commits that recorded it, so the answer outlives the sync run, a restart, and a fresh clone. `id`/`title` are `null` for a kept path that is not a note (a saved view, `manifest.json`): reported honestly, and given no button, because "delete it again" is `delete` and that is a note command. Two terminators, neither needing state: deleting the note again drops the row everywhere, and `kept_seen` moves a **per-device** watermark — the device that kept a note and the device that deleted it are not asking the same question, so acknowledging on one must not answer for the other |
| `demoted`        | —                            | `DemotedField[]`       | fields two devices set differently, where the merge kept both: what the note shows now and what the other device said. A read, and stateless — the loser is a `conflict-<field>` key in the note's own frontmatter, so this is correct after a restart and readable in any text editor |

### Vaults

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `list_vaults`    | —                            | `VaultInfo[]`          | `[]` **is the first-run signal**. **The one producer of what a vault is**: name, path, default, label, identity, attachment limit, supervision and restic repo — every cheap local fact, so no other arm has to re-derive one. Scoped — the one place the scope is visible to a user, because listing a vault the caller cannot read would leak its name |
| `check_path` / `create_vault` | `name`, `path`  | `PathCheck` / `VaultInfo[]` | the surface owns the verdict, not the form |
| `clone_vault`    | `url`, `name`, `path`, `gitName`, `gitEmail` | `VaultInfo[]` | clone a collaborator's vault and register it |
| `forget_vault`   | `name`                       | `VaultInfo[]`          | removes it from the list. **The files are left alone** — forgetting is not deleting |
| `check_move_vault` | `vault`, `path`            | `MoveCheck`            | whether the vault's folder can move there, and how much would move. Asked per keystroke; changes nothing. Host-only |
| `move_vault`     | `vault`, `path`              | `Moved`                | moves the vault's folder, keeping its name, its place in the list and its settings: one rename on the same disk, a copy verified byte for byte across disks. A failure leaves it where it was. Host-only |
| `recoverable_vaults` | —                        | `Recoverable[]`        | vault folders on this device that are not in the list, with a note count — the way back from a `forget_vault` nobody meant. Empty without a managed vault root, and empty for a scoped caller |
| `restore_vault`  | `name`, `path`, `repo`       | `VaultInfo[]`          | rebuild a vault from a restic repository |
| `copy_note` / `copy_status` / `uncopy_note` | `id`, `vault`; `with_assets` (copy) ; `blobs` (uncopy) | `CopyResult` / `CopyStatus` / `CopyResult` | cross-vault copy, its pre-check, and its undo. `with_assets` opts in to carrying the note's first-degree blobs; `blobs` names the ones an undo may take back |
| `set_supervision` | `vault`, `collect`, `publish` | `VaultInfo[]`         | the two supervision-corpus consents, always written together — an absent key and a deliberate *no* must not look the same to whoever answers for it later |

### Import

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `check_import`   | `source`, `path`, `vault`, `name` | `ImportCheck`     | what is there and what would happen — attachments by kind, whether they will be linked or copied, and which vaults already hold this folder. Asked on every keystroke while someone types a folder, so it is a pure read |
| `run_import`     | `source`, `path`, `vault`, `name`, `stubs`, `replace`, `maxMb`, `skipKinds` | `ImportResult` | convert a Logseq or Obsidian folder into notes. `replace` rewrites an earlier import's notes except those edited or deleted since; `maxMb`/`skipKinds` leave attachments out (each still named in its note) |
| `calendar_sync`  | `vault`, `source`, `offset`; **body** = iCalendar text | `CalendarReport` | your own calendar into a vault: new meetings become notes, a newer `SEQUENCE` moves one, a cancellation tags it. No model, no network — `fm-serve` reads the private address and hands the bytes in |
| `mail_sync`      | `vault`, `offset`; **body** = JSON array of Gmail messages (`format=full`) | `MailReport` | Gmail into a vault: one note per conversation (subject, last sender, when — never the text), and every invite read exactly as `calendar_sync` reads a calendar. No model, no network — `fm-serve` reads Gmail with a `gmail.readonly` token and hands the messages in |
| `mail_conversations` | — | `MailConversations` | read-only: every email conversation (id, title, when its latest message arrived, its current `start`), the starts already on the agenda, and the meeting notes linked to each conversation — what the assistant's meeting pass reads |
| `propose_note` | `about`, `title`, `body`, `start`, `due`, `location`, `tags`, author + supervision fields | `ObjectMeta` | propose a **new** note linked to `about` (`meeting_of`): it exists only on its `proposal/<id>` branch until accepted. The assistant uses it for a further meeting fixed in an email exchange |

### Backup

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `backup_status`  | —                            | `BackupStatus`         | `{vaults, git, restic, restic_password_set}`; each vault carries `{name, remote, unpushed, remote_moved, conflicts, restic_ready}` — **only what this command alone can answer**, all of it paid for with a per-vault network `ls-remote`. `identity`, `git_assets_max` and `restic_repo` are local reads and live on `VaultInfo`; they were reported here as well until 2026-09-08, so two screens could disagree about one field. The password only ever as a bool, never by value |
| `asset_batches`  | `vault`, `budget`            | `AssetBatch[]`         | **how a backlog of attachments would be sent, in steps.** A device that has never sent attachments stages every one it holds the first time it can — the rule is a filter over the blob store, not a diff against the remote — so the first backup can be one pack a phone connection cannot finish. Each step is a *rising* size limit, so a step re-stages what is already sent (a no-op) and adds only what its raise newly admits. Local and read-only: a walk and a `stat` per file, no network and no repository access |
| `last_commits`   | —                            | `LastCommit[]`         | **when each vault last saved anything**, as epoch seconds — `null` for a vault that has never been committed, which is a state and not a zero. Its own answer rather than a field on `backup_status` for the same reason `backup_latest` is: one `git log -1` per vault, local and offline, so a phone can be told at first paint that nothing has been saved in five weeks. Scoped like `list_vaults` — how stale a vault is is itself a disclosure. No threshold: when silence is worth mentioning is a display policy |
| `backup_latest`  | `vault`                      | `LatestBackup`         | **when this vault was last snapshotted.** Deliberately *not* a field on `backup_status`, which polls every 45 s and already spawns a process per vault. Three distinct answers: a snapshot (`id` + `time` + the source `paths` it recorded), `id: null` with no `unavailable` — the repository opened and has **never** been written to — and `unavailable` with a reason, meaning this machine cannot tell you. An unreadable repository is *reported*, not raised |
| `backup`         | `vault`                      | `BackupRun`            | restic snapshot of the vault's notes directory **and** `blobs/` — never the vault root, so no `.git`, `views/`, `themes/` or `vault.json`; repo per vault, password from `RESTIC_PASSWORD` or the stored one. **Answers what the snapshot held**: `{vault, notes_dir, blobs, contents}`, where `notes_dir` is the directory it actually took (`docs` for a vault that moved it) and `blobs` says whether there were any — a vault with no attachments yet must not be described as carrying them. `contents` is restic's own summary (`{id, files_new, files_changed, files_unmodified, bytes_processed, bytes_added}`) and is `null` when restic wrote the snapshot without describing it, which is **not** the same as a snapshot that held nothing |
| `set_restic_repo` | `vault`, `repo`             | `BackupStatus`         | point one vault's snapshot (notes + attachments) at a repository; empty `repo` clears it |
| `set_restic_password` / `clear_restic_password` | `password` / — | `BackupStatus` | the one password every restic repo here uses; written `0600` beside the vault list, never into it |

### Credentials

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `probe_remote`   | `url`                        | `RemoteProbe`          | can this URL be reached, and does it need credentials |
| `git_auth`       | `url`                        | `{stored, …}`          | whether a credential for that host is already held |
| `set_git_credential` / `clear_git_credential` | `url`, `username`, `token` / `url` | `{…}` | delegated to the OS credential helper wherever git exists; stored by the app only on a device that has none — which is the phone |

### Views and themes

Both are files in the vault (`views/*.view`, `themes/*.css`), so both travel to collaborators over
git. **The view-authoring buttons were withdrawn from the UI on 2026-08-31**; these commands stayed,
and so did their tests. See [Views](../user/views.md#saved-views).

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `list_views` / `run_view` | `name` (run only)   | `ViewInfo[]` / `ViewResult` | saved `.view` files; no `Query` crosses the wire. A file with a mistake is still **listed, with its parse error**. The views that come with the app are listed after a vault's own, marked `shipped` |
| `log_days`       | `view`, `from`, `to`, `rows` | `{ days }`             | a span of days of one log, asked for by the **name** of a `totals` view. Each day: `sums` (`{ sum, n }` per metric, `n` being how many entries carried it), `of` (entries that day), `targets`, `times` (occasions, first, last — for entries that carry a time), a `low` and `high` beside a sum when an amount was a range, and its entries when `rows` |
| `log_recent`     | `view`, `slot`, `q`, `limit` | `RecentItem[]`         | what a log holds already, one line per distinct thing, most used in `slot` first, each with the values it had the last time |
| `log_recipes`    | `view`                       | `Recipe[]`             | the recipes saved in a log, by name. Each: `servings`, its ingredient entries, `of` (how many), and `sums` — every whole-number value the ingredients carry, added up, with how many of them carried it. Nothing is adjusted for cooking |
| `log_nutrients`  | `view`, `day`                | `Nutrients`            | every vitamin and mineral of one day: per value its `sum`, how many entries carried it, the `reference` for an adult of the person's sex and age where there is one figure (EFSA 2017), an `upper` level where it applies to food, and the day's entries. `needs_sex` when figures were left out for want of it |
| `exercise_library` | —                          | `{ seen, sources, exercises, videos, videos_seen }` | how each exercise is done, as its source wrote it: per card its `steps`, and `about`, `start`, `dose`, `notes` where the source has them, its page and reference, and where the app files it (`equipment`, `area`). `sources` carry each body's credit line and licence. `videos` are links, each a known body's (`tier: body`) or a trainer's (`tier: trainer`) |
| `recipe_library` | `book?`                      | `{ source, home, seen, recipes }` | the recipes carried in the app, as published: each with its `ingredients` and `steps` word for word, its page, and its publisher's own values `per_serving`. `book: wikibooks` gives Italian Wikibooks' Italian recipes (CC BY-SA 4.0): `ingredients`, `steps`, `text`, `notes`, `yield`/`servings`, `region`, `times`, `image` as a link, `authors_url` — and no values |
| `log_kitchen`    | —                            | `{ id, title }[]`      | what the person says they have at home: notes carrying `kitchen`, by name. No amounts |
| `log_products`   | —                            | `Product[]`            | the packaged foods a person wrote down from their labels: notes carrying `product`, each with its `per100` values (the keys every food uses), and `barcode` and `serving_g` when given. In no log and no day |
| `log_routines`   | `view`                       | `Routine[]`            | the sessions kept under a name in a log: each with the `weekdays` it is meant for (`mon` … `sun`) and its `sets` — entries that name the routine instead of a day, so they are in no day until planned onto one |
| `plan_library`   | `group`                      | `{ plans, activities, weeks, hidden }` | the diets (`food`) or regimes (`fitness`) that can be followed, as carried in the app: each with what it rests on, its sources, its checks, what cannot be measured, and links to how it is done. `activities` (fitness only) are activities a named body counts, with what each counts as; `weeks` (fitness only) are example weeks a public body lays out day by day; each card also says its `kind`, the `question` it answers, whether to `start_here`, how it relates to the others, and its notes per flag. Cards whose `not_for` matches the profile's flags are left out and named in `hidden` |
| `plan_profile`   | —                            | `Profile`              | a person's own measurements — `sex`, `born`, `height_cm`, `weight_g`, `pal_x10`, and the opt-in `flags` (`pregnant`, `breastfeeding`, `postmenopausal`, `kidney`, `diabetes_meds`, `eating_disorder`) — from the note carrying a `profile` property, or all empty |
| `plan_preview`   | `id`, `day`                  | `Preview`              | one option with its numbers worked out for this person: the bounds per check, the energy estimate when it uses one, and which measurements are still missing. Writes nothing |
| `plan_progress`  | `group`, `day`               | `Progress`             | the plans in force on `day` — one per question, in `followed`; the first also as `plan`, `checks` and the other top-level fields — and the week holding it (Monday to Sunday) measured against each: per check its bounds, the value so far, `met` / `under` / `over` / `none`, each day's value, and a four-week `trend`. A check the profile now hides is not measured. Empty when nothing is followed |
| `plan_watch`     | `group`, `day`               | `Watch`                | the "worth watching" set of the group, measured on the week holding `day` whichever plan is followed or none: per line its bounds for this person, the value, the state, the days and a four-week `trend`. A line the profile hides is left out. Writes nothing |
| `food_search`    | `q`, `limit`                 | `FoodHit[]`            | foods in the food list whose name has a word beginning with each word typed, the plain food first. The list is carried in the app and unpacked on first use |
| `food_get`       | `id`                         | `FoodDetail` or `null` | one food: every value per 100 g, its household portions, and the list it came from. `id` may be an entry's `source` |
| `pack_status`    | —                            | `PackStatus`           | the food list this app carries: what it says about itself and what it takes on this device, or why it could not be put in place |
| `save_view`      | `vault`, `name`, `view`, `group_by`, `tag` | `ViewInfo[]` | writes the file. A view filtered by hand **keeps its filter** — saving over it is refused rather than quietly dropping what a screen cannot describe |
| `rename_view`    | `vault`, `from`, `to`        | `ViewInfo[]`           | moves the file and leaves its contents alone — which is why it is a rename and not "save under the new name, delete the old" |
| `delete_view`    | `vault`, `name`              | `ViewInfo[]`           | removes only the view; the notes it was showing are untouched |
| `list_themes` / `read_theme` | `vault`, `name` (read only) | `ThemeInfo[]` / the CSS | reading a theme changes nothing |
| `save_theme` / `rename_theme` / `delete_theme` | `vault`, `name`/`from`,`to`, `css` | `ThemeInfo[]` | deliberately **not** read-only: the generation bump is what makes the other pane, and the other machine, notice a theme edit without a timer |

## Property values (`set_property`)

`set_property`'s `value` is parsed per key:

| key     | value format                                   | empty value |
|---------|------------------------------------------------|-------------|
| `type`  | `asset` → asset; **anything else → `note`** (`Kind::from_str` is lenient, so legacy `task`/`meeting` frontmatter migrates silently) | → `note` |
| `status`| any string                                     | clears      |
| `title` | any string                                     | clears      |
| `start` | `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`              | clears      |
| `due`   | `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`              | clears      |
| `log`   | **refused** — what kind of logged entry a note is, is set when `create_notes` makes it | refused |
| `hard`  | `true`/`yes`/`1` → true, else false            | → false     |
| `tags`  | comma-separated; a tag may contain spaces      | clears      |
| *(other)* | free text → a custom frontmatter property    | removes it  |

## The lost-update guard (`update_body`'s `base`)

An editor that has been open a while may be holding a version of the note that no longer
exists — most obviously when a `pull` merged someone else's edit into it. Saving then would
overwrite their text and leave a history saying you wrote it.

`FileStore::put` already refuses a write whose file moved on disk since we indexed it, but
that check cannot see this case: `pull` merges and then **reindexes** (a merge is invisible
until it does), which records the post-merge mtime and stands the guard down exactly when it
mattered. The staleness is in the *client*, so the client declares what it edited: send the
`version` you last saw (it comes back on `get`) as `base`, and hold the one `update_body`
returns for your next write.

A mismatch returns `this note changed on disk since you opened it — reload before saving`.
The UI reloads and puts your unsaved draft back **below** the merged text with conflict
markers — the same stance as the `.md` merge driver: both versions where a human can see
them, never a silent choice.

The token is the **sha256 of the body**, not the `updated` stamp. A stamp only moves for
writers that bump it — the app does and the merge driver does, but hand-editing in Vim does
not — so a stamp left exactly the case `put`'s mtime guard could not cover. Content cannot
lie about whether the body moved. It costs ~1.6 ms on the largest body there is (a whiteboard
scene), against a save debounce that then writes and fsyncs that same body.

## Asset references

An asset is referenced as `asset:sha256-<hex>` (canonical, in Markdown) or
`sha256:<hex>`. `resolve_asset`/`asset_status` accept either, plus a bare hash.

## Routes that are not commands

A command answers with a value. These cannot, so they live in the transport instead —
putting them in `dispatch` would push an HTTP concern into the shared surface.

### `POST /api/alive`

Liveness, and only liveness: the UI beats it every 15 s so the auto-shutdown watchdog knows
a tab is open. It takes no lock, reads no files, and never reaches `dispatch`. It is
separate from `ping` because a hidden tab must keep the app alive **without** making it
reindex a vault nobody is looking at — and because the watchdog belongs to *this* server, so
a frontend without one would never call it.

### `POST /api/remind_set` and `POST /api/rest_end`

The desktop program's side of reminders. `remind_set` takes `{ items: [{ title, text, at }] }`
(`at` in milliseconds), the list the page worked out from `reminders`; the program keeps it in
`<config>/formicaria/reminders.json` and says each through the system's notification when its
moment comes and no page is open. While one is still waiting, the program **rests** instead of
stopping when its last page closes: it answers `503` to the assistant, which then stops its
model. `rest_end` is how a newer copy asks a resting one to make way; a program somebody is
using answers `409`. Both are host-only, and neither is a command: they are about this process.

### `GET /api/blob/<reference>`

Blob bytes, **streamed** from disk with the sniffed `Content-Type`, `Accept-Ranges: bytes`
and honest `Range` support (`206` with `Content-Range`, `416` when unsatisfiable). This is
what `<img>`/`<video>`/`<iframe>` point at, so opening a note with a large attachment costs
no memory and seeking a video costs one range request instead of a whole-file download.

It is a route rather than a command because a command answers with a `Vec<u8>` — the shape
that forces the whole file into memory in the first place.

Two response headers are load-bearing, not decoration. `X-Content-Type-Options: nosniff`
stops the browser second-guessing the sniffed type. And anything outside an inline-safe
allowlist (`image/*` except SVG, `video/*`, `audio/*`, `application/pdf`) is sent
`Content-Disposition: attachment`: blobs arrive from collaborators, and a blob is now at a
URL the browser can *navigate* to, so an SVG or HTML attachment rendered as a top-level
document would run its script in the app's own origin. Subresource loads ignore the
disposition, so inline SVG images still render.
