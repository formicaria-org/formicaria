//! Conflicts: notes that came back from a merge with both versions marked in the body. Derived by
//! scanning for the markers — a note is in conflict iff its body has an opening AND a closing marker,
//! so a note that merely *mentions* one is not flagged, and resolving a note drops it off by itself.

use fm_app::commands::{capture, conflicts};
use fm_core::MemoryStore;

#[test]
fn a_note_with_conflict_markers_is_listed_and_a_clean_one_is_not() {
    let mut s = MemoryStore::new();
    let conflicted = capture(
        &mut s,
        "intro\n<<<<<<< ours\nmine\n=======\ntheirs\n>>>>>>> theirs\noutro",
        "",
    )
    .unwrap()
    .id;
    capture(&mut s, "a perfectly clean note", "").unwrap();

    let listed: Vec<String> = conflicts(&s).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(listed, vec![conflicted], "only the note carrying both markers");
}

#[test]
fn a_dangling_opening_marker_alone_is_not_a_conflict() {
    let mut s = MemoryStore::new();
    // A line that opens a marker but never closes it — not a real conflict (e.g. a note about git).
    capture(&mut s, "<<<<<<< just talking about merges\nno closing marker here", "").unwrap();
    assert!(conflicts(&s).unwrap().is_empty(), "needs both an opening and a closing marker");
}
