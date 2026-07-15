//! The command surface — one pure function per IPC call, each over the [`Store`]
//! seam so it is testable with `MemoryStore` and identical against `FileStore`.
//! The Tauri binary wraps these; nothing here knows Tauri exists.

use crate::dto::{value_string, Board, Column, NoteDetail, ObjectMeta};
use fm_core::{apply_property, ingest, BlobStore, Store, StoreError};
use fm_model::{Id, Kind, Object, PropertyValue};
use fm_query::{Filter, Op, Predicate, Query, SortKey};
use serde::Serialize;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Group every note by `group_by` into board columns, newest card first. The
/// property is opaque: pass `"status"` for a status board, `"type"` to falsify
/// the thesis (a board of note/task/asset), or any custom key — no code changes.
pub fn board(store: &dyn Store, group_by: &str) -> Result<Board, StoreError> {
    let q = Query {
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

/// Capture a note; the text becomes the body. Returns the new card's meta so the
/// UI can slot it into the board without a full refetch.
pub fn capture(store: &mut dyn Store, body: &str) -> Result<ObjectMeta, StoreError> {
    let obj = Object::new(Kind::Note, body);
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
}

/// Replace a note's body and write it back to disk (bumping `updated`). The
/// body is stored byte-for-byte — the editor is a plain textarea holding literal
/// Markdown, so the round-trip (edit -> store -> read) is lossless by
/// construction, the invariant the whole files-as-truth design rests on.
pub fn update_body(store: &mut dyn Store, id: &str, body: &str) -> Result<(), StoreError> {
    let id: Id = id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let mut obj = store.get(id)?.ok_or(StoreError::NotFound(id))?;
    obj.body = body.to_string();
    obj.updated = OffsetDateTime::now_utc();
    store.put(&obj)?;
    Ok(())
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

/// Every note, newest-created first — the timeline/journal feed. Zero new
/// machinery again: no filter, just a sort, the same shape as the CLI's `fm
/// list`. The renderer groups these by creation day into a Logseq-style journal.
pub fn recent(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query { sort: vec![SortKey::desc("created")], ..Default::default() };
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

/// Read the bytes of a referenced asset for display in the webview. `kind`
/// selects the derived thumbnail (`"thumb"`, what the gallery and inline preview
/// show) or the full blob (anything else). Returning bytes over IPC needs no
/// asset-protocol scope or capability entry — the caller wraps them in an object
/// URL. A missing blob is an ordinary `Err`, which the UI degrades to the
/// "asset not available" placeholder (media absence is a warning, never a crash).
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

/// Ingest an uploaded file: store its bytes as a content-addressed blob, extract
/// searchable text, and create an asset note pointing at it — the GUI twin of
/// `fm add`. Returns the new asset's meta so the editor can insert a reference
/// (`![title](asset:sha256-<hash>)`) without a refetch.
pub fn ingest(
    store: &mut dyn Store,
    vault: &Path,
    filename: &str,
    bytes: &[u8],
) -> Result<ObjectMeta, StoreError> {
    let ing = ingest::ingest_bytes(vault, filename, bytes)?;
    let mut obj = Object::new(Kind::Asset, ing.text.clone().unwrap_or_default());
    obj.title = Some(ing.filename.clone());
    obj.assets = vec![format!("sha256:{}", ing.hash)];
    obj.extra.insert("mime".into(), PropertyValue::Text(ing.mime.clone()));
    store.put(&obj)?;
    // Best-effort thumbnail, like `fm add`: a missing vipsthumbnail (or failure)
    // only degrades a gallery tile, never the ingest.
    let _ = ingest::thumbnail(vault, &ing.hash);
    Ok(ObjectMeta::from(&obj))
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
