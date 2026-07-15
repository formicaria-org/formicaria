# Commands

The frontend calls these over `POST /api/<cmd>` (JSON args, camelCase). Each is a
function in `crates/fm-app/src/commands.rs` (except `commit`/`backup`, which the
server calls on `fm-core` directly). Args map to Rust snake_case.

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
| `backup`         | —                            | —                      | restic snapshot (env repo/password) |

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
