//! **Music** (`decisions.md#data` `#ui`, 2026-11-13, *a sixth group, Music*): a song is a note with
//! `log: music`, its audio an asset like any other, its details copied from the file's tags into
//! the note by the *Add a song* sheet.
//!
//! What this module answers is the library: **every song's note, in one pass**, so a screen asks
//! once and filters, sorts and groups in memory while the person types (the `perf.rs` rule).

use crate::dto::ObjectMeta;
use crate::thread::{log_base, LOG};
use fm_core::{Store, StoreError};
use fm_model::PropertyValue;
use fm_query::{Query, SortKey};

/// The `log` kind of a song.
pub const MUSIC: &str = "music";

/// Every song, as its note's metadata (title, properties, assets), sorted by title.
///
/// Not bounded, on purpose: a library is the person's whole collection, and the screen that shows
/// it needs all of it to filter as they type. Its cost is one read of each song note, which
/// `tests/perf.rs` holds to a measure.
pub fn library(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q =
        Query { filter: log_base(MUSIC), sort: vec![SortKey::asc("title")], ..Default::default() };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        // `log_base` reaches SQL as a pushdown; the residual check is the engine's, and this is
        // the same belt-and-braces `health::read` keeps.
        .filter(|o| o.get(LOG) == PropertyValue::Text(MUSIC.into()))
        .map(ObjectMeta::from)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::{create_notes, NoteDraft};
    use fm_core::MemoryStore;
    use std::collections::BTreeMap;

    fn draft(title: &str, log: &str) -> NoteDraft {
        let mut props = BTreeMap::new();
        props.insert("log".to_string(), serde_json::json!(log));
        props.insert("artist".to_string(), serde_json::json!("The Example Band"));
        props.insert("duration_s".to_string(), serde_json::json!(214));
        NoteDraft { title: title.into(), props, ..Default::default() }
    }

    #[test]
    fn the_library_is_every_song_and_nothing_else_sorted_by_title() {
        let mut store = MemoryStore::default();
        let drafts =
            [draft("Night Train", MUSIC), draft("Coffee", "food"), draft("Arrival", MUSIC)];
        create_notes(&mut store, "personal", &drafts).unwrap();
        let songs = library(&store).unwrap();
        let titles: Vec<_> = songs.iter().map(|s| s.title.clone().unwrap()).collect();
        assert_eq!(titles, ["Arrival", "Night Train"]);
        assert_eq!(songs[0].props["artist"], "The Example Band");
        assert_eq!(songs[0].props["duration_s"], 214);
    }
}
