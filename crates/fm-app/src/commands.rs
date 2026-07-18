//! The command surface — one pure function per IPC call, each over the [`Store`]
//! seam so it is testable with `MemoryStore` and identical against `FileStore`.
//! `fm_app::dispatch` is the one surface that wraps these; a transport only frames them. Nothing here knows what HTTP is.

use crate::dto::{value_string, Board, Column, NoteDetail, ObjectMeta};
use crate::refs;
use fm_core::{apply_property, ingest, BlobStore, Manifest, Store, StoreError};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Filter, Op, Predicate, Query, SortKey};
use serde::Serialize;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use ulid::Ulid;

/// Group every note by `group_by` into board columns, newest card first. The
/// property is opaque: pass `"status"` for a status board, or any custom key —
/// no code changes.
///
/// Assets are excluded: an asset is a blob a note *references*, not something you
/// plan, so it has no place in a column. Find one via `search` (its extracted
/// text is indexed) or via the note that references it. Same rule in [`agenda`]
/// and [`recent`]; `search` and [`gallery`] deliberately still see assets.
pub fn board(store: &dyn Store, group_by: &str) -> Result<Board, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Kind(vec![Kind::Note])),
        group_by: Some(group_by.to_string()),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    let res = store.query(&q)?;
    let columns = res
        .groups
        .unwrap_or_default()
        .into_iter()
        .map(|g| Column {
            value: value_string(&g.key),
            label: g.label,
            cards: g.rows.iter().map(ObjectMeta::from).collect(),
        })
        .collect();
    Ok(Board { group_by: group_by.to_string(), columns })
}

/// The gallery: every asset, newest first. This is the S4 checkpoint — a
/// *second* renderer that is nothing but a different query (`type = asset`) over
/// the same engine and the same `ObjectMeta`. No new query machinery, no new
/// storage path; if this were expensive, the `Store`/`fm-query` seam would not
/// be real. Thumbnails and lazy loading arrive with S5's ingest; until a blob
/// exists, the tile shows the shared "asset not found" placeholder.
pub fn gallery(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Kind(vec![Kind::Asset])),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// The agenda: the closest-deadline view. Everything with a `due` date that is
/// not done, soonest first. This is "zero new code" — it is the same engine and
/// the same `ObjectMeta`, just a different filter and sort; urgency is *derived*
/// in the card from `due`, never stored (there is no priority field).
///
/// "done" is the completion convention: a note with no status is not done, so it
/// is included (`status != done` keeps nulls). A meeting carries a date, so it
/// shows up here too. This filter is the one place the string "done" is written;
/// it stays out of the renderers (a `.view` file could override it).
pub fn agenda(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: Filter::new()
            .and(Predicate::Kind(vec![Kind::Note]))
            .and(Predicate::Prop {
                key: "due".into(),
                op: Op::Exists,
                value: PropertyValue::Null,
            })
            .and(Predicate::Prop {
                key: "status".into(),
                op: Op::Ne,
                value: PropertyValue::Text("done".into()),
            }),
        sort: vec![SortKey::asc("due")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Fetch one note with its full body — the read view's payload. The list
/// commands return meta only; the body crosses IPC only when a note is opened.
pub fn get(store: &dyn Store, id: &str) -> Result<Option<NoteDetail>, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    Ok(store
        .get(id)?
        .map(|o| NoteDetail { meta: ObjectMeta::from(&o), body: o.body.clone() }))
}

/// Capture a note; the text becomes the body. `vault` is the audience it joins —
/// empty means the default vault, an unknown name is refused by `Store::put`'s
/// routing (same discipline as `ingest`). Returns the new card's meta so the UI
/// can slot it into the board without a full refetch.
pub fn capture(store: &mut dyn Store, body: &str, vault: &str) -> Result<ObjectMeta, StoreError> {
    let mut obj = Object::new(Kind::Note, body);
    obj.vault = vault.to_string();
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
}

/// Replace a note's body and write it back to disk (bumping `updated`). The
/// body is stored byte-for-byte — the editor is a plain textarea holding literal
/// Markdown, so the round-trip (edit -> store -> read) is lossless by
/// construction, the invariant the whole files-as-truth design rests on.
///
/// `base` is the `updated` stamp the caller last saw, and it is the **lost-update guard for
/// an editor that has been open a while**. Returns the new stamp, which the caller holds for
/// its next write.
///
/// `FileStore::put` already refuses a write whose file moved on disk since we indexed it —
/// but that check cannot see this case. `pull` merges and then *reindexes*, because a merge
/// is invisible until it does; the reindex records the post-merge mtime, so from `put`'s
/// point of view everything is in sync while the open pane still holds pre-merge text. The
/// staleness is in the client, so the client has to be the one to declare what it edited.
/// Without this, a debounced auto-save silently overwrites a collaborator's merged
/// paragraph and leaves a clean history saying you wrote it.
///
/// An **empty `base` opts out** — `fm-cli`, curl, and anything that never read the note
/// keep working, still covered by the mtime guard in `put`.
pub fn update_body(
    store: &mut dyn Store,
    id: &str,
    body: &str,
    base: &str,
) -> Result<String, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    // Compare the serialized form, which is what crossed the wire — reparsing the caller's
    // string would turn a formatting difference into a spurious conflict.
    if !base.is_empty() && crate::dto::stamp(obj.updated) != base {
        return Err(StoreError::Conflict(id));
    }
    obj.body = body.to_string();
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(crate::dto::stamp(obj.updated))
}

/// Delete a note: remove its Markdown file and drop it from the index. The
/// destructive counterpart of `capture` — `Store::delete` already unlinks the
/// `.md` and both index rows, and returns `NotFound` for an unknown id, so this
/// is a thin, id-parsing wrapper (the UI gates it behind a second confirmation).
pub fn delete(store: &mut dyn Store, id: &str) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    store.delete(id)
}

/// Set one property and write it back to disk (bumping `updated`). This is the
/// board's drag write-back: dropping a card into a column calls this with the
/// column's `value`, so a drop and `fm set` change the file identically. An
/// empty `value` clears the property (the "(none)" column).
pub fn set_property(
    store: &mut dyn Store,
    id: &str,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    apply_property(&mut obj, key, value)?;
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(())
}

/// Full-text search across every note, newest-updated first. This is "zero new
/// code" again — the same engine and `ObjectMeta`, just a `Text` predicate. In
/// `FileStore` that predicate is answered by SQLite FTS5 (prefix terms, ranked);
/// in `MemoryStore` by a substring scan — so the one command works identically
/// against both, exactly as the CLI's `fm search` does. An empty needle returns
/// nothing rather than dumping the whole vault.
pub fn search(store: &dyn Store, query: &str) -> Result<Vec<ObjectMeta>, StoreError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let q = Query {
        filter: Filter::new().and(Predicate::Text(query.to_string())),
        sort: vec![SortKey::desc("updated")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Every note, newest-created first — the timeline/journal feed, and the
/// suggestion list behind a bare `/` in the editor. Assets are excluded (see
/// [`board`]); the renderer groups the rest by creation day into a Logseq-style
/// journal.
pub fn recent(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Kind(vec![Kind::Note])),
        sort: vec![SortKey::desc("created")],
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// Normalize an asset reference to its blob hash. Notes, the gallery, and the
/// mock all spell the same blob differently — stored as `sha256:<hex>`, written
/// in Markdown as `asset:sha256-<hex>`, or passed bare — so every asset path
/// funnels through here to one lowercase hex hash.
fn parse_ref(reference: &str) -> Result<String, StoreError> {
    let r = reference.trim();
    let r = r.strip_prefix("asset:").unwrap_or(r);
    let r = r.strip_prefix("sha256:").or_else(|| r.strip_prefix("sha256-")).unwrap_or(r);
    let hash = r.trim();
    if hash.len() < 4 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(StoreError::Parse(format!("not an asset reference: {reference}")));
    }
    Ok(hash.to_ascii_lowercase())
}

/// Read the bytes of a referenced asset. `kind` selects the derived thumbnail
/// (`"thumb"`) or the full blob (anything else). A missing blob is an ordinary `Err`,
/// which the UI degrades to the "asset not available" placeholder (media absence is a
/// warning, never a crash).
///
/// **Not how inline media reaches the read view any more.** This reads the whole file into
/// memory to hand it back, which is the wrong shape for a 300 MB video and cannot seek;
/// `<img>`/`<video>`/`<iframe>` point at the streaming `GET /api/blob/<reference>` route
/// instead (`fm-serve/src/blob.rs`). What still needs this: thumbnails, and any frontend
/// with no HTTP route to stream from — which is why it stays.
pub fn resolve_asset_bytes(vault: &Path, reference: &str, kind: &str) -> Result<Vec<u8>, StoreError> {
    let hash = parse_ref(reference)?;
    let path = match kind {
        "thumb" => ingest::thumb_path(vault, &hash),
        _ => BlobStore::new(vault).path_for(&hash),
    };
    std::fs::read(&path).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))
}

/// Whether a referenced asset can be shown: is the blob present locally, and has
/// a thumbnail been generated? The UI uses this to choose between a real preview,
/// an "open externally" affordance, and the missing-asset placeholder.
#[derive(Clone, Debug, Serialize)]
pub struct AssetStatus {
    pub has_blob: bool,
    pub has_thumb: bool,
    /// Sniffed MIME of the blob (magic bytes) — the read view picks its inline
    /// element from this. `None` when the blob is absent or has no signature.
    pub mime: Option<String>,
}

pub fn asset_status(vault: &Path, reference: &str) -> Result<AssetStatus, StoreError> {
    let hash = parse_ref(reference)?;
    let store = BlobStore::new(vault);
    let has_blob = store.exists(&hash);
    Ok(AssetStatus {
        has_blob,
        has_thumb: ingest::thumb_path(vault, &hash).exists(),
        mime: has_blob.then(|| ingest::sniff_mime(&store.path_for(&hash))).flatten(),
    })
}

/// What is actually at a path, for the "create a vault here?" form to answer with.
///
/// Facts only — every one of these is *reported*, and what is refused versus merely warned
/// about is policy, decided by the caller that also knows the vault list. Splitting it here
/// is the same seam [`asset_status`] uses: this crate knows the filesystem, not the config.
#[derive(Clone, Debug, Serialize)]
pub struct PathFacts {
    /// Absolute; the caller expands `~` before handing it over.
    pub path: String,
    pub exists: bool,
    /// No entries at all. Not an error — see `notes`.
    pub empty: bool,
    /// `.md` files directly under `<path>/notes`. **Adoption is free** — `FileStore::named`
    /// reindexes whatever is already there, so this needs no code. But it must be *said*:
    /// creating a vault over someone's notes silently is the surprise this field prevents.
    pub notes: usize,
    pub not_a_directory: bool,
    /// The parent doesn't exist either, so we'd be creating a chain of directories.
    pub parent_missing: bool,
    /// **Probed, not inferred from mode bits.** We create and remove a temp entry, because
    /// a capability must mean "this will work", never "this is configured" — which is
    /// exactly how `restic_ready` came to enable a checkbox that then failed. Root, and a
    /// read-only mount that lies about its permissions, are why the bits are not the answer.
    pub writable: bool,
    /// Already a git repo. Not an error: we leave its history alone, and its identity is
    /// one fewer thing to ask the user for.
    pub git_repo: bool,
}

/// Inspect a path for the create-vault form. Never mutates anything that outlives the
/// call: the write probe cleans up after itself.
pub fn inspect_path(path: &Path) -> PathFacts {
    let md = std::fs::metadata(path);
    let exists = md.is_ok();
    let not_a_directory = md.as_ref().map(|m| !m.is_dir()).unwrap_or(false);
    let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);

    let empty = is_dir
        && std::fs::read_dir(path).map(|mut d| d.next().is_none()).unwrap_or(false);

    // Only `notes/*.md`, non-recursively — `FileStore::reindex` reads exactly that, so
    // counting anything else here would promise notes that never appear.
    let notes = std::fs::read_dir(path.join("notes"))
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0);

    // The nearest existing ancestor is what we can actually probe: the path itself may
    // not exist yet, and "can I create it?" is a question about its parent.
    let probe_at = if is_dir { Some(path.to_path_buf()) } else { nearest_existing(path) };
    let parent_missing = !exists && probe_at.as_deref() != path.parent();

    PathFacts {
        path: path.to_string_lossy().into_owned(),
        exists,
        empty,
        notes,
        not_a_directory,
        parent_missing,
        writable: probe_at.map(|p| can_write(&p)).unwrap_or(false),
        git_repo: path.join(".git").exists(),
    }
}

/// The closest ancestor that exists, so a path we are about to create can still be
/// probed. `None` when even the root is unreachable.
fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut p = path.parent()?;
    loop {
        if p.is_dir() {
            return Some(p.to_path_buf());
        }
        p = p.parent()?;
    }
}

/// Can we really write here? Make something and remove it. Mode bits are configuration;
/// this is capability.
fn can_write(dir: &Path) -> bool {
    let probe = dir.join(format!(".fm-write-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Ingest an uploaded file: store its bytes as a content-addressed blob, extract
/// searchable text, and create an asset note pointing at it — the GUI twin of
/// `fm add`. Returns the new asset's meta so the editor can insert a reference
/// (`![title](asset:sha256-<hash>)`) without a refetch.
/// `vault` is where the bytes go; `vault_name` is the audience the asset **note** joins.
/// Both, because they are answered by different things: the blob store takes a path, and
/// the `Store` routes by name. They must agree — an asset note in one vault describing a
/// blob in another means the people who can see the file cannot see the note, and the
/// person who can see the note is pointing at bytes they never shared.
pub fn ingest(
    store: &mut dyn Store,
    vault: &Path,
    vault_name: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<ObjectMeta, StoreError> {
    let ing = ingest::ingest_bytes(vault, filename, bytes)?;
    let mut obj = Object::new(Kind::Asset, ing.text.clone().unwrap_or_default());
    obj.title = Some(ing.filename.clone());
    obj.assets = vec![format!("sha256:{}", ing.hash)];
    obj.extra.insert("mime".into(), PropertyValue::Text(ing.mime.clone()));
    obj.vault = vault_name.to_string();
    store.put(&obj)?;
    // Best-effort thumbnail, like `fm add`: a missing vipsthumbnail (or failure)
    // only degrades a gallery tile, never the ingest.
    let _ = ingest::thumbnail(vault, &ing.hash);
    Ok(ObjectMeta::from(&obj))
}

/// A recent note edit, as the collaboration views show it: git says who last touched a note and
/// when (`fm_core::git::Touch`); the store says what that note currently is. One `git log` per
/// vault powers the authorship labels, the activity stream, and the contributor filter.
#[derive(Serialize)]
pub struct EditEvent {
    pub id: String,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub vault: String,
    pub author: String,
    pub email: String,
    pub time: String,
}

/// Recent edits in one vault, read from git and resolved against the store. A touch whose note is
/// gone from the index (deleted, or not yet reindexed) is dropped — the view shows notes that
/// exist. `since` is a git `--since` value (e.g. `"1 year ago"`). Read-only.
pub fn activity(
    store: &dyn Store,
    vault_path: &Path,
    since: &str,
) -> Result<Vec<EditEvent>, StoreError> {
    let mut events = Vec::new();
    for t in fm_core::git::activity(vault_path, since)? {
        let Ok(id) = t.id.parse::<Id>() else { continue };
        let Some(obj) = store.get(id)? else { continue };
        events.push(EditEvent {
            id: t.id,
            title: obj.title.clone(),
            kind: obj.kind.as_str().to_string(),
            vault: obj.vault.clone(),
            author: t.author,
            email: t.email,
            time: t.time,
        });
    }
    Ok(events)
}

/// The outcome of a copy: the new note's meta, the blob hashes this copy actually wrote
/// into the target (deduped ones are omitted, so an Undo knows exactly what to take back),
/// and how many prior copies of the same source it replaced.
#[derive(Serialize)]
pub struct CopyResult {
    pub meta: ObjectMeta,
    pub new_blobs: Vec<String>,
    pub replaced: usize,
}

/// A stable, one-way fingerprint of a source note's id, stamped on its copies as `copy_of`.
/// Re-copying the same source into a vault finds its prior copy by this and replaces it, so a
/// vault never accumulates duplicate copies — and the fingerprint reveals neither the id nor
/// the origin vault (unlike embedding the raw `note:<id>`, which is exactly what we strip).
fn provenance(source_id: Id) -> String {
    fm_core::blob::sha256_hex(source_id.to_string().as_bytes())
}

/// Does `target_vault` already hold a copy of the note `id`? The pre-check behind the
/// "this will replace the existing copy" warning.
pub fn copy_status(store: &dyn Store, id: &str, target_vault: &str) -> Result<bool, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let token = provenance(id);
    let want = PropertyValue::Text(token);
    Ok(store
        .candidates(&Filter::new())?
        .0
        .iter()
        .any(|o| o.vault == target_vault && o.get("copy_of") == want))
}

/// Copy a note into another vault. **Restrictive by default:** only the prose travels —
/// every `note:`/`asset:` reference is stripped (see [`refs::strip_cross_vault`]) so the
/// copy can never point at a note or blob outside its new audience, and `assets`/`code`
/// are cleared. `with_assets` opts in to carrying the note's first-degree blobs *into* the
/// target so it is self-contained (note links are still stripped — the linked-notes tier is
/// deferred). A copy is a **new** note (fresh ULID); the source is untouched. `vault_paths`
/// is every vault's `(name, root)`, used to locate a blob wherever it lives and to write the
/// target. An unknown `target_vault`, or the note's own vault, is refused.
pub fn copy_note(
    store: &mut dyn Store,
    id: &str,
    target_vault: &str,
    vault_paths: &[(String, PathBuf)],
    with_assets: bool,
) -> Result<CopyResult, StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let src = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    if src.vault == target_vault {
        return Err(StoreError::Io(format!("note is already in vault '{target_vault}'")));
    }
    let target_path = vault_paths
        .iter()
        .find(|(name, _)| name == target_vault)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| StoreError::Io(format!("no vault named '{target_vault}'")))?;

    // Override, don't duplicate: a re-copy of the same source replaces its prior copies in this
    // vault (found by the `copy_of` fingerprint), so the vault never grows two copies of one note.
    let token = provenance(id);
    let want = PropertyValue::Text(token.clone());
    let stale: Vec<Id> = store
        .candidates(&Filter::new())?
        .0
        .iter()
        .filter(|o| o.vault == target_vault && o.get("copy_of") == want)
        .map(|o| o.id)
        .collect();
    let replaced = stale.len();
    for old in stale {
        store.delete(old)?;
    }

    // The copy: a fresh identity in the target vault, prose rewritten to drop outward refs,
    // stamped with the source fingerprint so a future re-copy finds and replaces it.
    let now = OffsetDateTime::now_utc();
    let mut obj = src.clone();
    obj.id = Ulid::new();
    obj.created = now;
    obj.updated = now;
    obj.vault = target_vault.to_string();
    obj.body = refs::strip_cross_vault(&src.body, with_assets);
    obj.code.clear(); // code blobs are not carried in v1 — never leave an outward pointer
    obj.extra.insert("copy_of".to_string(), PropertyValue::Text(token));

    let mut new_blobs = Vec::new();
    if with_assets {
        // First-degree asset hashes: the body's refs plus any frontmatter `assets`.
        let (mut hashes, _notes) = refs::references(&src.body);
        for a in &src.assets {
            if let Ok(h) = parse_ref(a) {
                hashes.push(h);
            }
        }
        hashes.sort();
        hashes.dedup();
        let target_blobs = BlobStore::new(&target_path);
        for h in &hashes {
            // Find the blob wherever it physically lives, and copy it into the target
            // (content-addressed, so put_file dedups; we record only what was new).
            if let Some((_, src_root)) = vault_paths.iter().find(|(_, p)| BlobStore::new(p).exists(h)) {
                let src_blob = BlobStore::new(src_root).path_for(h);
                let stored = target_blobs.put_file(&src_blob)?;
                if !stored.deduped {
                    new_blobs.push(stored.hash);
                }
            }
        }
        if !new_blobs.is_empty() {
            Manifest::build(&target_path)?.write(&target_path)?;
        }
    } else {
        // Prose-only: no attachment reference of any kind leaves the source vault.
        obj.assets.clear();
    }

    store.put(&obj)?;
    Ok(CopyResult { meta: ObjectMeta::from(&obj), new_blobs, replaced })
}

/// Recede a copy: delete the copied note from its vault, then remove the blobs this copy
/// newly wrote — but only a blob that no note remaining in the target still references
/// (the bytes are content-addressed, so a survivor may now be legitimately shared). The
/// inverse of the `with_assets` branch of [`copy_note`], and safe because the copy has a
/// fresh id with no inbound links yet.
pub fn uncopy_note(
    store: &mut dyn Store,
    id: &str,
    target_vault: &str,
    blobs: &[String],
    vault_paths: &[(String, PathBuf)],
) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    store.delete(id)?;
    if blobs.is_empty() {
        return Ok(());
    }
    let target_path = vault_paths
        .iter()
        .find(|(name, _)| name == target_vault)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| StoreError::Io(format!("no vault named '{target_vault}'")))?;

    // After the delete, which of these hashes does a note *remaining* in the target vault
    // still reference (in frontmatter `assets` or body)?
    let remaining = store.candidates(&Filter::new())?.0;
    let still_used = |hash: &str| {
        remaining.iter().filter(|o| o.vault == target_vault).any(|o| {
            o.assets.iter().any(|a| parse_ref(a).map(|h| h == hash).unwrap_or(false))
                || refs::references(&o.body).0.iter().any(|h| h == hash)
        })
    };

    let target_blobs = BlobStore::new(&target_path);
    let mut changed = false;
    for h in blobs {
        if !still_used(h) {
            let p = target_blobs.path_for(h);
            if p.exists() {
                std::fs::remove_file(&p).map_err(|e| StoreError::Io(e.to_string()))?;
                changed = true;
            }
        }
    }
    if changed {
        Manifest::build(&target_path)?.write(&target_path)?;
    }
    Ok(())
}

/// The on-disk path of a referenced blob, for handing to the OS default app.
/// Errors if the reference is malformed or the blob is not present locally (it
/// may live only in a backup/remote) — the caller surfaces that as a warning.
pub fn blob_path(vault: &Path, reference: &str) -> Result<PathBuf, StoreError> {
    let hash = parse_ref(reference)?;
    let store = BlobStore::new(vault);
    if !store.exists(&hash) {
        return Err(StoreError::Io(format!("blob not present locally: {hash}")));
    }
    Ok(store.path_for(&hash))
}
