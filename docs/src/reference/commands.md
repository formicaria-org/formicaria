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

| Command          | Args                         | Returns                | Notes |
|------------------|------------------------------|------------------------|-------|
| `board`          | `groupBy`                    | `Board` (columns)      | group by any property |
| `gallery`        | —                            | `ObjectMeta[]`         | assets, newest first |
| `agenda`         | —                            | `ObjectMeta[]`         | dated & not done, soonest first |
| `recent`         | —                            | `ObjectMeta[]`         | all notes, newest-created first |
| `search`         | `query`                      | `ObjectMeta[]`         | FTS5; empty query → `[]` |
| `get`            | `id`                         | `NoteDetail \| null`   | meta + full body |
| `capture`        | `body`                       | `ObjectMeta`           | creates a note |
| `set_property`   | `id`, `key`, `value`         | —                      | writes one frontmatter field |
| `update_body`    | `id`, `body`, `base`         | the new `version`      | byte-for-byte body write; `base` is the `version` you last saw — a mismatch is refused (see below). `''` opts out |
| `ingest`         | *(binary body)* `?name=`     | `ObjectMeta`           | upload → asset note; raw bytes |
| `resolve_asset`  | `reference`, `kind`          | bytes (ArrayBuffer)    | `kind` = `full` \| `thumb`; whole blob in memory — prefer the blob route below |
| `asset_status`   | `reference`                  | `{has_blob,has_thumb,mime}` | sniffed MIME |
| `open_external`  | `reference`                  | —                      | opens the blob in the OS default app |
| `commit`         | `message`                    | `{committed, conflicts}` | git-commit the vault. `committed: false` with `conflicts` non-empty is **not** "nothing to do" — the vault is mid-merge and nothing will be committed until those notes are settled |
| `push`           | `message`                    | `u32`                  | squash the unpushed window → push; returns commits squashed (0 on the first push) |
| `backup_status`  | —                            | `BackupStatus`         | `{vaults, git, restic, restic_password_set}`; each vault carries `{name, remote, unpushed, identity, remote_moved, conflicts, restic_repo, restic_ready}` — the password only ever as a bool, never by value |
| `set_git_remote` | `vault`, `url`, `name`, `email` | —                   | sets the vault's `origin`, and its git identity when `name`/`email` are non-empty (asked only of a vault git has never met); blank URL refused |
| `backup`         | `vault`                      | —                      | restic snapshot, media included; repo per vault, password from `RESTIC_PASSWORD` or the stored one |
| `set_restic_repo` | `vault`, `repo`             | `BackupStatus`         | point one vault's media backup at a repository; empty `repo` clears it |
| `set_restic_password` / `clear_restic_password` | `password` / — | `BackupStatus` | the one password every restic repo here uses; written `0600` beside the vault list, never into it |
| `pull`           | `vault`                      | `{merged, conflicts}`  | fetch + merge through the `.md` driver; `conflicts` is a *result*, not an error |
| `delete`         | `id`                         | —                      | unlinks the file and both index rows |
| `activity`       | `since`                      | `EditEvent[]`          | who last edited what, straight from git log |
| `reply`          | `id`, `body`                 | `ObjectMeta`           | post a [discussion](../user/notes.md#discussion) message on a note (or another message — it re-roots to the same thread). One file, ULID-named, in the target's vault. No `vault` arg: a reply joins the audience of the note it is about |
| `thread`         | `id`                         | `ThreadView`           | a note's discussion — `{root, count, messages[]}`, each message with a server-computed `depth`. `root: null` when the note itself was deleted (the discussion survives it) |
| `proposals`      | —                            | `ObjectMeta[]`         | every open [proposal](../user/collaboration.md#proposals) across vaults (notes carrying `proposes: branch:<name>`), newest first. A store query like `recent`, not a git read — it lists proposal *notes*; branch state is derived elsewhere |
| `create_discussion` | `title`, `vault`          | `ObjectMeta`           | create a first-class [discussion](../user/notes.md#discussion) — a note that is the root of its own thread (`thread_of` points at itself). A dedicated command because `set_property` refuses `thread_of` |
| `discussions`    | —                            | `DiscussionSummary[]`  | every first-class discussion across vaults, most-recently-active first: `{…root, count, last_activity, participants}`. Participants come from git authorship. Comment threads on an ordinary note are *not* here — they stay with their note |
| `stale`          | `since`                      | `ObjectMeta[]`         | notes untouched in git since `since` (a `--since` value, default `90 days ago`), oldest first. **Derived from git, stored nowhere**; a vault with no history is skipped, not reported as wholly stale |
| `ping`           | —                            | `{changed, git}`       | the 15 s visible-tab reindex poll |
| `list_vaults`    | —                            | `VaultInfo[]`          | `[]` **is the first-run signal** |
| `check_path` / `create_vault` | `name`, `path`  | `PathCheck` / `VaultInfo[]` | the surface owns the verdict, not the form |
| `list_views` / `run_view` | `name` (run only)   | `ViewInfo[]` / `ViewResult` | saved `.view` files; no `Query` crosses the wire |
| `copy_note` / `copy_status` / `uncopy_note` | `id`, `vault`, … | see `dto.rs` | cross-vault copy, its pre-check, and its undo |

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
| `tags`  | comma- or space-separated                      | clears      |
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
