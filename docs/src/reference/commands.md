# Commands

The frontend calls these over `POST /api/<cmd>` (JSON args, camelCase). Each is a
function in `crates/fm-app/src/commands.rs` (except the git/restic ones —
`commit`, `push`, `backup`, `backup_status`, `set_git_remote` — which are the OS
seam, not `Store` operations, so the server calls `fm-core` directly). Args map to
Rust snake_case.

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
| `update_body`    | `id`, `body`                 | —                      | byte-for-byte body write |
| `ingest`         | *(binary body)* `?name=`     | `ObjectMeta`           | upload → asset note; raw bytes |
| `resolve_asset`  | `reference`, `kind`          | bytes (ArrayBuffer)    | `kind` = `full` \| `thumb` |
| `asset_status`   | `reference`                  | `{has_blob,has_thumb,mime}` | sniffed MIME |
| `open_external`  | `reference`                  | —                      | opens the blob in the OS default app |
| `commit`         | `message`                    | `bool`                 | git-commit the vault; `false` if clean |
| `push`           | `message`                    | `u32`                  | squash the unpushed window → push; returns commits squashed (0 on the first push) |
| `backup_status`  | —                            | `BackupStatus`         | `{remote, unpushed, restic_repo, restic_ready}` — never the restic password |
| `set_git_remote` | `url`                        | —                      | sets the vault's `origin`; blank URL refused |
| `backup`         | —                            | —                      | restic snapshot, media included (env repo/password) |

## Property values (`set_property`)

`set_property`'s `value` is parsed per key:

| key     | value format                                   | empty value |
|---------|------------------------------------------------|-------------|
| `type`  | `note`/`task`/`meeting`/`asset` (lowercase)    | rejected    |
| `status`| any string                                     | clears      |
| `title` | any string                                     | clears      |
| `due`   | `YYYY-MM-DD`                                    | clears      |
| `hard`  | `true`/`yes`/`1` → true, else false            | → false     |
| `tags`  | comma- or space-separated                      | clears      |
| *(other)* | free text → a custom frontmatter property    | removes it  |

## Asset references

An asset is referenced as `asset:sha256-<hex>` (canonical, in Markdown) or
`sha256:<hex>`. `resolve_asset`/`asset_status` accept either, plus a bare hash.
