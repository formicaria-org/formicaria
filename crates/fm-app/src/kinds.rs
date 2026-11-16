//! **The library's kinds of notes** (`docs/context/workspace-design-2026-11-15.md` §13.2;
//! `decisions.md` 2026-11-16, *one library of notes*). formicaria is a library; everything in it is a
//! note, of special, structured kinds, *"like special kinds of books"*: a study note, a web clip, a
//! paper, a recipe, a log entry, a song, a meeting, a mail conversation, a discussion, a proposal, an
//! asset. Each kind is **recognised once, here, from the markers notes already carry** (no new
//! property is written), and travels with every note to the UI and to Myrme as
//! [`crate::dto::ObjectMeta::kind_key`], so nobody guesses from tags.
//!
//! For each kind the table says which views navigate it (its catalogues), where notes of it come
//! from (its sources), where a new one belongs by default (its place), and how it can go out of the
//! library (its hand-offs). Read by the `kinds` command.

use fm_model::{Kind, Object, PropertyValue};
use serde::Serialize;

/// One kind of note.
#[derive(Debug, Serialize)]
pub struct KindSpec {
    pub key: &'static str,
    pub label: &'static str,
    /// One character, where there is no room for an icon.
    pub mark: &'static str,
    /// The views that navigate it (their group or name).
    pub views: &'static [&'static str],
    /// Where notes of it come from.
    pub sources: &'static [&'static str],
    /// Where a new one belongs by default: a section, and a shelf within it if any.
    pub place: &'static str,
    /// How it can leave the library, handed to another service by the person.
    pub out: &'static [&'static str],
}

/// The kinds, in the order they are recognised: the first that matches wins; a study note takes the
/// rest.
pub const KINDS: &[KindSpec] = &[
    KindSpec {
        key: "asset",
        label: "File",
        mark: "▣",
        views: &["the notes using it"],
        sources: &["phone gallery", "files", "downloads"],
        place: "where it is used",
        out: &["share"],
    },
    KindSpec {
        key: "conversation",
        label: "Conversation",
        mark: "✉",
        views: &["Mail"],
        sources: &["gmail"],
        place: "Mail",
        out: &["mail reply"],
    },
    KindSpec {
        key: "proposal",
        label: "Proposal",
        mark: "⇄",
        views: &["Collaboration"],
        sources: &["myrme", "people"],
        place: "Collaboration",
        out: &[],
    },
    KindSpec {
        key: "discussion",
        label: "Discussion",
        mark: "❝",
        views: &["Collaboration", "Discussions"],
        sources: &["myrme", "people"],
        place: "Collaboration",
        out: &["mail new", "mail reply"],
    },
    KindSpec {
        key: "meeting",
        label: "Meeting",
        mark: "◷",
        views: &["Calendar", "Tasks"],
        sources: &["calendars", "gmail"],
        place: "Calendar",
        out: &["calendar"],
    },
    KindSpec {
        key: "song",
        label: "Song",
        mark: "♪",
        views: &["Music"],
        sources: &["files", "downloads"],
        place: "Music",
        out: &["share"],
    },
    KindSpec {
        key: "log",
        label: "Entry",
        mark: "◆",
        views: &["Food", "Fitness", "Health"],
        sources: &["typed", "a watch"],
        place: "its view",
        out: &[],
    },
    KindSpec {
        key: "recipe",
        label: "Recipe",
        mark: "◍",
        views: &["Food"],
        sources: &["web", "typed"],
        place: "Food",
        out: &["share"],
    },
    KindSpec {
        key: "paper",
        label: "Paper",
        mark: "❡",
        views: &["Notes"],
        sources: &["web", "bibtex"],
        place: "Notes › Papers",
        out: &["share"],
    },
    KindSpec {
        key: "web",
        label: "Web page",
        mark: "◎",
        views: &["Notes"],
        sources: &["web"],
        place: "Notes › Web",
        out: &["share"],
    },
    KindSpec {
        key: "study",
        label: "Study note",
        mark: "✎",
        views: &["Notes"],
        sources: &[],
        place: "Notes",
        out: &["mail new", "calendar", "share"],
    },
];

fn text(o: &Object, key: &str) -> Option<String> {
    match o.extra.get(key)? {
        PropertyValue::Null => None,
        v => Some(v.display()),
    }
}
fn has(o: &Object, key: &str) -> bool {
    text(o, key).is_some_and(|v| !v.trim().is_empty())
}
fn tagged(o: &Object, tag: &str) -> bool {
    o.tags.iter().any(|t| t == tag)
}

/// The kind of a note, from the markers it already carries.
pub fn kind_of(o: &Object) -> &'static str {
    if o.kind == Kind::Asset {
        return "asset";
    }
    if has(o, fm_core::mail::GMAIL_THREAD) {
        return "conversation";
    }
    if has(o, crate::thread::PROPOSES) {
        return "proposal";
    }
    if has(o, crate::thread::THREAD_OF) {
        return "discussion";
    }
    if has(o, fm_core::calendar::ICS_UID) || tagged(o, fm_core::calendar::MEETING_TAG) {
        return "meeting";
    }
    match text(o, crate::thread::LOG).as_deref() {
        Some("music") => return "song",
        Some(l) if !l.trim().is_empty() => return "log",
        _ => {}
    }
    if has(o, crate::log::RECIPE) {
        return "recipe";
    }
    if tagged(o, "paper") || has(o, "doi") {
        return "paper";
    }
    if text(o, crate::source::SOURCE).as_deref() == Some("web") || tagged(o, "web") {
        return "web";
    }
    "study"
}

#[cfg(test)]
mod tests {
    use super::*;
    fn note(tags: &[&str], props: &[(&str, &str)]) -> Object {
        let mut o = Object::new(Kind::Note, "");
        o.tags = tags.iter().map(|t| t.to_string()).collect();
        for (k, v) in props {
            o.extra.insert(k.to_string(), PropertyValue::Text(v.to_string()));
        }
        o
    }

    #[test]
    fn each_kind_is_recognised_by_the_markers_notes_already_carry() {
        assert_eq!(kind_of(&note(&[], &[("gmail_thread", "t")])), "conversation");
        assert_eq!(kind_of(&note(&[], &[("proposes", "branch:x")])), "proposal");
        assert_eq!(
            kind_of(&note(&[], &[("thread_of", "note:01JD0000000000000000000000")])),
            "discussion"
        );
        assert_eq!(kind_of(&note(&["meeting"], &[])), "meeting");
        assert_eq!(kind_of(&note(&[], &[("ics_uid", "u@x")])), "meeting");
        assert_eq!(kind_of(&note(&[], &[("log", "music")])), "song");
        assert_eq!(kind_of(&note(&[], &[("log", "food")])), "log");
        assert_eq!(kind_of(&note(&["recipe"], &[("recipe", "pasta")])), "recipe");
        assert_eq!(kind_of(&note(&["paper"], &[])), "paper");
        assert_eq!(kind_of(&note(&["web"], &[("source", "web")])), "web");
        assert_eq!(kind_of(&note(&["lab"], &[])), "study");
    }

    #[test]
    fn every_kind_names_its_views_and_its_place() {
        let mut keys: Vec<_> = KINDS.iter().map(|k| k.key).collect();
        for k in KINDS {
            assert!(!k.views.is_empty() && !k.place.is_empty(), "{}", k.key);
        }
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), KINDS.len());
        assert_eq!(KINDS.last().unwrap().key, "study");
    }
}
