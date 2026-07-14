//! The command surface — one pure function per IPC call, each over the [`Store`]
//! seam so it is testable with `MemoryStore` and identical against `FileStore`.
//! The Tauri binary wraps these; nothing here knows Tauri exists.

use crate::dto::{value_string, Board, Column, ObjectMeta};
use fm_core::{apply_property, Store, StoreError};
use fm_model::{Id, Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
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

/// Capture a note; the text becomes the body. Returns the new card's meta so the
/// UI can slot it into the board without a full refetch.
pub fn capture(store: &mut dyn Store, body: &str) -> Result<ObjectMeta, StoreError> {
    let obj = Object::new(Kind::Note, body);
    store.put(&obj)?;
    Ok(ObjectMeta::from(&obj))
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
