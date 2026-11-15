//! **Music** (`decisions.md#data` `#ui`, 2026-11-13, *a sixth group, Music*): a song is a note with
//! `log: music`, its audio an asset like any other, its details copied from the file's tags into
//! the note by the *Add a song* sheet.
//!
//! What this module answers is the library: **every song's note, in one pass**, so a screen asks
//! once and filters, sorts and groups in memory while the person types (the `perf.rs` rule).

use crate::dto::ObjectMeta;
use crate::thread::{log_base, notes_base, LOG};
use fm_core::{Store, StoreError};
use fm_model::PropertyValue;
use fm_query::{Query, SortKey};

/// The `log` kind of a song.
pub const MUSIC: &str = "music";

/// Every song, as its note's metadata (title, properties, assets), sorted by title.
///
/// Not bounded, on purpose: a library is the person's whole collection, and the screen that shows
/// it needs all of it to filter as they type. Its cost is one pass over the vault's notes, which
/// `tests/perf.rs` holds to a measure.
pub fn library(store: &dyn Store) -> Result<Vec<ObjectMeta>, StoreError> {
    let q =
        Query { filter: log_base(MUSIC), sort: vec![SortKey::asc("title")], ..Default::default() };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        // Only `Kind` reaches the index, so this is one pass over every note of the vault, with
        // `log` checked after each is read (`known-issues.md`, *every logged entry still costs the
        // board a read*). The check is the engine's; this is the belt-and-braces `health::read`
        // keeps.
        .filter(|o| o.get(LOG) == PropertyValue::Text(MUSIC.into()))
        .map(ObjectMeta::from)
        .collect())
}

/// The tag that makes a note a playlist.
pub const PLAYLIST: &str = "playlist";

/// A playlist: a note tagged `playlist`, whose body links its songs in order.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    /// The songs' note ids, **in the body's order**, repeats kept: a playlist may play a song
    /// twice. A link to a note that is not a song is the UI's to skip, not this list's.
    pub songs: Vec<String>,
}

/// Every playlist, by title. One pass over the vault's notes, like [`library`].
///
/// **The body is the truth** (`music-plan-2026-11-13.md`, *Playlists*): a numbered list of
/// `[Title](note:<id>)` links, so a playlist reads and edits as an ordinary note, and two devices
/// adding to one merge like any note.
pub fn playlists(store: &dyn Store) -> Result<Vec<Playlist>, StoreError> {
    let q = Query { filter: notes_base(), sort: vec![SortKey::asc("title")], ..Default::default() };
    Ok(store
        .query(&q)?
        .rows
        .iter()
        .filter(|o| o.tags.iter().any(|t| t.eq_ignore_ascii_case(PLAYLIST)))
        .map(|o| Playlist {
            id: o.id.to_string(),
            title: o.title.clone().unwrap_or_default(),
            songs: note_links(&o.body),
        })
        .collect())
}

/// The `note:<ulid>` links of a body, in order, repeats kept (`refs::references` sorts and
/// de-duplicates, which a playlist cannot use).
fn note_links(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find("note:") {
        let tail = &rest[at + 5..];
        let id: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        if id.len() == 26 && id.parse::<fm_model::Id>().is_ok() {
            out.push(id.to_ascii_uppercase());
        }
        rest = tail;
    }
    out
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

    #[test]
    fn a_playlist_is_a_tagged_note_and_its_songs_keep_the_body_order() {
        let mut store = MemoryStore::default();
        let made =
            create_notes(&mut store, "personal", &[draft("A", MUSIC), draft("B", MUSIC)]).unwrap();
        let (a, b) = (made[0].id.clone(), made[1].id.clone());
        let list = NoteDraft {
            title: "Long drive".into(),
            tags: vec!["playlist".into()],
            body: format!("1. [B](note:{b})\n2. [A](note:{a})\n3. [B](note:{b})\n"),
            ..Default::default()
        };
        let plain = NoteDraft {
            title: "Not a playlist".into(),
            body: format!("[A](note:{a})"),
            ..Default::default()
        };
        create_notes(&mut store, "personal", &[list, plain]).unwrap();
        let found = playlists(&store).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Long drive");
        assert_eq!(found[0].songs, [b.clone(), a, b]);
    }
}
