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

**All 81 of them are below**, grouped by what they are for. If you add an arm to
`dispatch_inner`, add its row here — `ci/checks.sh` counts the two and fails when they disagree,
because a reference that is *nearly* complete is one a reader stops trusting.

### Reading the vault

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `board`          | `groupBy`                    | `Board` (columns)      | group by any property |
| `agenda`         | —                            | `ObjectMeta[]`         | dated & not done, soonest first |
| `recent`         | —                            | `ObjectMeta[]`         | all notes, newest-created first |
| `gallery`        | —                            | `ObjectMeta[]`         | assets, newest first. **No renderer draws it** — the Gallery view was removed and a `.view` asking for `gallery` renders as a timeline; the command stays because `fm-cli` and the asset pickers use it |
| `search`         | `query`                      | `ObjectMeta[]`         | FTS5; empty query → `[]` |
| `get`            | `id`                         | `NoteDetail \| null`   | meta + full body |
| `backlinks`      | `id`                         | `ObjectMeta[]`         | every note whose body carries a `note:` link to this one |
| `templates`      | —                            | `ObjectMeta[]`         | the notes tagged as templates, for the new-note picker |
| `stale`          | `since`                      | `ObjectMeta[]`         | notes untouched in git since `since` (a `--since` value, default `90 days ago`), oldest first. **Derived from git, stored nowhere**; a vault with no history is skipped, not reported as wholly stale |
| `activity`       | `since`                      | `EditEvent[]`          | who last edited what, straight from git log |
| `duplicates`     | —                            | `DuplicateFamily[]`    | notes grouped by identical body, and what pruning them would remove. A read: it changes nothing |
| `config`         | —                            | `Config`               | version, the vault list's path and whether it is writable, every vault, each vault's restic repo, and which `FM_*` variables are set. The one call a support question can start from |
| `ping`           | `since`                      | `{changed, git}`       | the 15 s visible-tab reindex poll |

### Writing notes

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `capture`        | `body`, `vault`              | `ObjectMeta`           | creates a note |
| `set_property`   | `id`, `key`, `value`         | —                      | writes one frontmatter field (see the table below) |
| `update_body`    | `id`, `body`, `base`         | the new `version`      | byte-for-byte body write; `base` is the `version` you last saw — a mismatch is refused (see below). `''` opts out |
| `delete`         | `id`                         | —                      | unlinks the file and both index rows |
| `create_paper`   | `input`, `vault`             | `ObjectMeta`           | a note from a pasted citation, DOI or arXiv id — parsed locally, never looked up online |
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
| `create_proposal` | `id`, `body`, `why`, `kind`, `tool`, `query`, `authorName`, `authorEmail` | `ObjectMeta` | write the draft to a review branch and the proposal note beside it. `kind`/`tool`/`query` record *how* it was produced — a model's `/propose`, `/research` or `/transcribe`, and what it searched for |
| `proposal_for`   | `id`                         | `ObjectMeta \| null`   | the open proposal against this note, if there is one |
| `proposal_diff`  | `id`                         | a unified diff         | what accepting would change, read off the branch |
| `proposal_content` | `id`                       | `{body, …}`            | the proposed body itself, for the side-by-side review |
| `proposal_shown` | `id`                         | —                      | records that a human actually looked at it. Idempotent, fire-and-forget: it must never fail the screen it is reporting about, and it is what separates *left alone* from *never displayed* |
| `accept_proposal` | `id`                        | `ObjectMeta`           | apply the branch's body to the note and close the proposal |
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

### Vaults

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `list_vaults`    | —                            | `VaultInfo[]`          | `[]` **is the first-run signal**. Scoped — the one place the scope is visible to a user, because listing a vault the caller cannot read would leak its name |
| `check_path` / `create_vault` | `name`, `path`  | `PathCheck` / `VaultInfo[]` | the surface owns the verdict, not the form |
| `clone_vault`    | `url`, `name`, `path`, `gitName`, `gitEmail` | `VaultInfo[]` | clone a collaborator's vault and register it |
| `forget_vault`   | `name`                       | `VaultInfo[]`          | removes it from the list. **The files are left alone** — forgetting is not deleting |
| `restore_vault`  | `name`, `path`, `repo`       | `VaultInfo[]`          | rebuild a vault from a restic repository |
| `copy_note` / `copy_status` / `uncopy_note` | `id`, `vault` | see `dto.rs` | cross-vault copy, its pre-check, and its undo |
| `set_supervision` | `vault`, `collect`, `publish` | `VaultInfo[]`         | the two supervision-corpus consents, always written together — an absent key and a deliberate *no* must not look the same to whoever answers for it later |

### Import

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `check_import`   | `source`, `path`, `vault`, `name` | `ImportCheck`     | what is there and what would happen. Asked on every keystroke while someone types a folder, so it is a pure read |
| `run_import`     | `source`, `path`, `vault`, `name`, `stubs` | `ImportResult` | convert a Logseq or Obsidian folder into notes |

### Backup

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `backup_status`  | —                            | `BackupStatus`         | `{vaults, git, restic, restic_password_set}`; each vault carries `{name, remote, unpushed, identity, remote_moved, conflicts, restic_repo, restic_ready, git_assets_max}` — the password only ever as a bool, never by value |
| `backup_latest`  | `vault`                      | `LatestBackup`         | **when this vault was last snapshotted.** Deliberately *not* a field on `backup_status`, which polls every 45 s and already spawns a process per vault. Three distinct answers: a snapshot (`id` + `time` + the source `paths` it recorded), `id: null` with no `unavailable` — the repository opened and has **never** been written to — and `unavailable` with a reason, meaning this machine cannot tell you. An unreadable repository is *reported*, not raised |
| `backup`         | `vault`                      | —                      | restic snapshot of the vault's notes directory **and** `blobs/` — never the vault root, so no `.git`, `views/`, `themes/` or `vault.json`; repo per vault, password from `RESTIC_PASSWORD` or the stored one |
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
| `list_views` / `run_view` | `name` (run only)   | `ViewInfo[]` / `ViewResult` | saved `.view` files; no `Query` crosses the wire. A file with a mistake is still **listed, with its parse error** |
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

## Two routes that are not commands

A command answers with a value. These two cannot, so they live in the transport instead —
putting them in `dispatch` would push an HTTP concern into the shared surface.

### `POST /api/alive`

Liveness, and only liveness: the UI beats it every 15 s so the auto-shutdown watchdog knows
a tab is open. It takes no lock, reads no files, and never reaches `dispatch`. It is
separate from `ping` because a hidden tab must keep the app alive **without** making it
reindex a vault nobody is looking at — and because the watchdog belongs to *this* server, so
a frontend without one would never call it.

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
