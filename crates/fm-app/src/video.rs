//! **Videos** (`decisions.md` 2026-11-17, *media from any site*): a video is a note with `log: video`,
//! its file an asset in `video`, as a song keeps its audio (`music.rs`). Collections are notes tagged
//! `collection` whose body links their videos in order, as playlists link songs.
//!
//! Answers the library in one pass, like Music: a screen asks once and filters in memory.

use crate::dto::ObjectMeta;
use crate::music::Playlist;
use crate::thread::{log_base, notes_base, LOG};
use fm_core::{Store, StoreError};
use fm_model::PropertyValue;
use fm_query::{Query, SortKey};

/// The tag that makes a note a collection of videos.
pub const COLLECTION: &str = "collection";

/// Every video, as its note's metadata, sorted by title.
pub fn library(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: log_base(crate::media::VIDEO),
        sort: vec![SortKey::asc("title")],
        ..Default::default()
    };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        .filter(|o| o.get(LOG) == PropertyValue::Text(crate::media::VIDEO.into()))
        .map(ObjectMeta::from)
        .collect())
}

/// Every collection, by title: its videos' note ids in the body's order.
pub fn collections(store: &dyn Store) -> Result<Vec<Playlist>, StoreError> {
    let q = Query { filter: notes_base(), sort: vec![SortKey::asc("title")], ..Default::default() };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        .filter(|o| o.tags.iter().any(|t| t.eq_ignore_ascii_case(COLLECTION)))
        .map(|o| Playlist {
            id: o.id.to_string(),
            title: o.title.clone().unwrap_or_default(),
            songs: crate::music::note_links(&o.body),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::{create_notes, NoteDraft};
    use fm_core::MemoryStore;
    use std::collections::BTreeMap;

    #[test]
    fn the_library_is_every_video_and_collections_link_them() {
        let mut store = MemoryStore::default();
        let mut v = BTreeMap::new();
        v.insert("log".to_string(), serde_json::json!("video"));
        let mut s = BTreeMap::new();
        s.insert("log".to_string(), serde_json::json!("music"));
        let made = create_notes(
            &mut store,
            "personal",
            &[
                NoteDraft { title: "Ants at night".into(), props: v.clone(), ..Default::default() },
                NoteDraft { title: "A song".into(), props: s, ..Default::default() },
                NoteDraft { title: "Bees".into(), props: v, ..Default::default() },
            ],
        )
        .unwrap();
        let titles: Vec<_> =
            library(&store).unwrap().iter().map(|o| o.title.clone().unwrap()).collect();
        assert_eq!(titles, ["Ants at night", "Bees"]);
        let body = crate::media::playlist_with("", &made[0].id, "Ants at night");
        create_notes(
            &mut store,
            "personal",
            &[NoteDraft {
                title: "Insects".into(),
                body,
                tags: vec![COLLECTION.into()],
                ..Default::default()
            }],
        )
        .unwrap();
        let c = collections(&store).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].songs, vec![made[0].id.clone()]);
    }
}
