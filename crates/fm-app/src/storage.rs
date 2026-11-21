//! **Unused attachments, found and put aside** (the owner, 2026-11-21: *"I want to delete a note but
//! all the assets that are not used by any other note are still there … a broad clean up option in
//! settings to allow the user to remove assets that nobody is using"*; `decisions.md` 2026-11-21,
//! *Storage: unused attachments*).
//!
//! An attachment is a file in `blobs/`, and every one has an **asset note** of its own (`asset_note`),
//! made when it came in. So "used by nobody" has two meanings, and they are told apart:
//!
//! - **nothing points at it at all** — no note, not even an asset note: a leftover, safe to remove;
//! - **only its own attachment entry points at it** — the note that used it was deleted, *or* it is
//!   an attachment kept on its own (a PDF dropped into the Library). The person decides.
//!
//! A note in the Deleted list (the last 30 days of history) still counts as using its files, so
//! bringing it back brings them back. Removed files are **held** for [`HOLD_DAYS`] in `.fm-held/`
//! (beside `.fm-ingest/`, never staged by git), can be put back, and go for good after that; a
//! removed file's attachment entry goes to the Deleted list like any deleted note.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use fm_core::blob::{media_kind, BlobStore, MediaKind};
use fm_core::{Store, StoreError};
use fm_model::{Kind, PropertyValue};
use fm_query::Query;

/// Where removed files wait, under the vault.
pub const HELD_DIR: &str = ".fm-held";
/// How long they wait before they go for good.
pub const HOLD_DAYS: u64 = 30;

/// A file no note uses, and what it is.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Unused {
    pub hash: String,
    pub size: u64,
    /// `sound`, `video`, `picture` or `document`.
    pub kind: &'static str,
    /// Its attachment entry's title (the file's name when it came in); empty with none.
    pub name: String,
    /// The attachment entries that point at it (removed with it, to the Deleted list).
    pub entries: Vec<String>,
}

/// What the Storage section shows for one vault.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Report {
    /// Nothing points at these at all.
    pub nothing: Vec<Unused>,
    /// Only their own attachment entry points at these.
    pub attachment_only: Vec<Unused>,
    /// What is held, waiting to go for good or to be put back.
    pub held: Vec<Held>,
}

/// A removed file, waiting.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Held {
    pub hash: String,
    pub size: u64,
    /// When it was removed (seconds since 1970).
    pub at: u64,
}

/// Every `sha256:` or `sha256-` reference in a text, as lowercase hex — however a note names a
/// file (its text's `asset:sha256-…`, a property's `sha256:…`, its `assets` list).
pub fn hashes_in(text: &str, out: &mut HashSet<String>) {
    let mut rest = text;
    while let Some(at) = rest.find("sha256") {
        let tail = &rest[at + 6..];
        if let Some(sep) = tail.chars().next().filter(|c| *c == ':' || *c == '-') {
            let hex: String =
                tail[sep.len_utf8()..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            if hex.len() == 64 {
                out.insert(hex.to_ascii_lowercase());
            }
        }
        rest = tail;
    }
}

fn text_of(v: &PropertyValue) -> String {
    match v {
        PropertyValue::Text(t) => t.clone(),
        PropertyValue::List(l) => l.iter().map(text_of).collect::<Vec<_>>().join(" "),
        other => format!("{other:?}"),
    }
}

/// The vault's unused files. `deleted` is the text of each note in the Deleted list, whose files
/// still count as used.
pub fn report(
    store: &dyn Store,
    vault_name: &str,
    vault: &Path,
    deleted: &[String],
) -> Result<Report, StoreError> {
    let rows = store.query(&Query::default())?.rows;
    let mut used = HashSet::new();
    let mut entries: HashMap<String, (Vec<String>, String, String)> = HashMap::new();
    for o in rows.iter().filter(|o| o.vault == vault_name) {
        if o.kind == Kind::Asset {
            let mime = match o.extra.get("mime") {
                Some(PropertyValue::Text(m)) => m.clone(),
                _ => String::new(),
            };
            let mut own = HashSet::new();
            for a in &o.assets {
                hashes_in(a, &mut own);
            }
            for h in own {
                let e = entries.entry(h).or_insert_with(|| {
                    (Vec::new(), o.title.clone().unwrap_or_default(), mime.clone())
                });
                e.0.push(o.id.to_string());
            }
            continue;
        }
        hashes_in(&o.body, &mut used);
        for a in &o.assets {
            hashes_in(a, &mut used);
        }
        for v in o.extra.values() {
            hashes_in(&text_of(v), &mut used);
        }
    }
    for t in deleted {
        hashes_in(t, &mut used);
    }
    let (mut nothing, mut attachment_only) = (Vec::new(), Vec::new());
    for p in BlobStore::new(vault).blob_paths() {
        let Some(hash) = p.file_name().and_then(|n| n.to_str()).map(str::to_ascii_lowercase) else {
            continue;
        };
        if hash.len() != 64 || used.contains(&hash) {
            continue;
        }
        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        let (ids, name, mime) = entries.remove(&hash).unwrap_or_default();
        let kind = match media_kind(&p) {
            MediaKind::Sound => "sound",
            MediaKind::Video => "video",
            MediaKind::Other if mime.starts_with("image/") => "picture",
            MediaKind::Other => "document",
        };
        let u = Unused { hash, size, kind, name, entries: ids };
        if u.entries.is_empty() {
            nothing.push(u);
        } else {
            attachment_only.push(u);
        }
    }
    let by_size = |a: &Unused, b: &Unused| b.size.cmp(&a.size).then(a.hash.cmp(&b.hash));
    nothing.sort_by(by_size);
    attachment_only.sort_by(by_size);
    Ok(Report { nothing, attachment_only, held: held(vault) })
}

fn held_dir(vault: &Path) -> PathBuf {
    vault.join(HELD_DIR)
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// **Put these unused files aside** — only those still unused now (asked again, so a note written
/// since the list was shown keeps its file). Their attachment entries go to the Deleted list.
/// Answers how many files and bytes.
pub fn remove(
    store: &mut dyn Store,
    vault_name: &str,
    vault: &Path,
    deleted: &[String],
    hashes: &[String],
) -> Result<(u32, u64), StoreError> {
    let r = report(store, vault_name, vault, deleted)?;
    let want: HashSet<String> = hashes.iter().map(|h| h.to_ascii_lowercase()).collect();
    let dir = held_dir(vault);
    std::fs::create_dir_all(&dir).map_err(|e| StoreError::Io(format!("{}: {e}", dir.display())))?;
    let blobs = BlobStore::new(vault);
    let (mut files, mut bytes) = (0u32, 0u64);
    for u in r.nothing.iter().chain(r.attachment_only.iter()).filter(|u| want.contains(&u.hash)) {
        let from = blobs.path_for(&u.hash);
        let to = dir.join(&u.hash);
        if std::fs::rename(&from, &to).is_err() {
            continue;
        }
        let _ = std::fs::write(dir.join(format!("{}.at", u.hash)), now().to_string());
        for id in &u.entries {
            let _ = crate::commands::delete(store, id);
        }
        files += 1;
        bytes += u.size;
    }
    Ok((files, bytes))
}

/// What is held in this vault, newest first.
pub fn held(vault: &Path) -> Vec<Held> {
    let dir = held_dir(vault);
    let mut out: Vec<Held> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.len() != 64 || !name.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            let at = std::fs::read_to_string(dir.join(format!("{name}.at")))
                .ok()
                .and_then(|t| t.trim().parse().ok())
                .unwrap_or(0);
            Some(Held { hash: name, size, at })
        })
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at).then(a.hash.cmp(&b.hash)));
    out
}

/// **Put held files back** where they were. Their attachment entries come back from the Deleted
/// list, as any deleted note does. Answers how many.
pub fn restore(vault: &Path, hashes: &[String]) -> u32 {
    let dir = held_dir(vault);
    let blobs = BlobStore::new(vault);
    let mut n = 0;
    for h in hashes.iter().map(|h| h.to_ascii_lowercase()) {
        if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let to = blobs.path_for(&h);
        if let Some(parent) = to.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::rename(dir.join(&h), &to).is_ok() {
            let _ = std::fs::remove_file(dir.join(format!("{h}.at")));
            n += 1;
        }
    }
    n
}

/// **Let held files go for good** once they have waited [`HOLD_DAYS`]. At open, best-effort.
pub fn purge(vault: &Path) -> u32 {
    purge_older_than(vault, now().saturating_sub(HOLD_DAYS * 24 * 60 * 60))
}

/// [`purge`] with the cut-off given, for tests.
pub fn purge_older_than(vault: &Path, before: u64) -> u32 {
    let dir = held_dir(vault);
    let mut n = 0;
    for h in held(vault) {
        if h.at != 0 && h.at < before {
            let _ = std::fs::remove_file(dir.join(&h.hash));
            let _ = std::fs::remove_file(dir.join(format!("{}.at", h.hash)));
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every way a note names a file is a reference — and so is the text of a note in the Deleted
    /// list, which `report` reads the same way.
    #[test]
    fn a_reference_is_found_however_a_note_names_the_file() {
        let a = "a".repeat(64);
        let b = "B".repeat(64);
        let c = "c".repeat(64);
        let mut out = HashSet::new();
        hashes_in(&format!("![x](asset:sha256-{a})"), &mut out);
        hashes_in(&format!("audio: sha256:{b}\nassets:\n  - sha256:{c}"), &mut out);
        hashes_in("sha256 alone, sha256:short, nothing else", &mut out);
        assert_eq!(out, [a, b.to_ascii_lowercase(), c].into_iter().collect());
    }
}
