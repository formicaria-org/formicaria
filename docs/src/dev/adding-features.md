# How to add a feature

formicarium is designed to be extended along its seams, without a plugin API.
Here are the common recipes.

## Add a command (backend function the UI can call)

A command is a pure function over the `Store` seam, fronted over HTTP. It touches
**four** places — keep them in sync:

1. **The pure function** — `crates/fm-app/src/commands.rs`:
   ```rust
   /// All notes tagged `tag`, newest first.
   pub fn tagged(store: &dyn Store, tag: &str) -> Result<Vec<ObjectMeta>, StoreError> {
       let q = Query {
           filter: Filter::new().and(Predicate::TagsAll(vec![tag.to_string()])),
           sort: vec![SortKey::desc("created")],
           ..Default::default()
       };
       Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
   }
   ```
   Add a unit test alongside the others in `crates/fm-app/tests/` (drive it with
   `MemoryStore` or a temp-dir `FileStore`).

2. **The HTTP arm** — `crates/fm-serve/src/main.rs`, in `api()`:
   ```rust
   "tagged" => json(commands::tagged(&*lock(state)?, &s("tag")).map_err(err)?),
   ```

3. **The mock** — `ui/src/lib/mock.ts`, a `case` in `handle()` so dev/tests work
   without a backend.

4. **The typed wrapper** — `ui/src/lib/ipc.ts`:
   ```ts
   export const tagged = (tag: string) => invoke<ObjectMeta[]>('tagged', { tag });
   ```

Args are camelCase in `ipc.ts` and map to the Rust snake_case params. That's it —
no Tauri, no codegen.

## Add a view (renderer)

A view is "a query + a renderer".

1. Add (or reuse) a command that returns the objects you want (above).
2. Create `ui/src/renderers/MyView.svelte` taking `{ cards, onopen }` and
   painting them. **It must stay generic**: never hard-code a status literal
   (`todo`/`doing`/`done`) — color and group only via `card.type`,
   `col.value`/`col.label`, and `urgency(card.due)` surfaced through
   `data-value` / `data-urgency` / `data-type`; the actual tints live in
   `ui/src/app.css`. CI (`ci/checks.sh`) greps `ui/src/renderers/**` and fails
   the build on a leaked status word — this keeps renderers reusable.
3. Wire it into `ui/src/App.svelte`: add the view to the `View` type, a nav
   button, and a branch in the stage that fetches and renders it.

## Add a store

Implement the `Store` trait (`crates/fm-core/src/lib.rs`) for your backend — five
methods (`get`, `put`, `delete`, `query`, `reindex`). Because the query engine is
behind seam 1, your store only has to return objects; the engine does the rest.
Verify it against the shared contract the way `FileStore` does.

## Add an extractor (make a new file type searchable)

Text extraction lives in `crates/fm-core/src/ingest.rs` (`extract_text`). Add a
match arm for the MIME type and shell out to the right tool (as the PDF arm
shells out to `pdftotext`). The extracted text lands in the asset note's body and
becomes full-text searchable — no other change needed.

## Add a theme

A theme is one CSS file. Override the semantic design tokens under a
`:root[data-theme="…"]` block in `ui/src/app.css` (see
[Design system](./design-system.md)). Because every renderer reads tokens (via
the legacy aliases), a new theme re-skins the whole app with no component edits.

## Add a subprocess tool

Pin it in `pixi.toml` `[dependencies]` (conda-forge) so it's reproducible, then
shell out to it from `fm-core` following the `backup.rs` / `ingest.rs` pattern
(`std::process::Command`, map failure to `StoreError`). Never link a GPL tool —
invoke it.
