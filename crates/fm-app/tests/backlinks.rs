//! Backlinks: "what links here", derived by scanning note bodies for `note:` references — no
//! reverse index to keep true (the files stay the truth). A plain `note:` mention and an
//! `![](note:id)` embed both count; a note is never its own backlink.

use fm_app::commands::{backlinks, capture};
use fm_core::MemoryStore;

#[test]
fn a_note_that_links_here_is_a_backlink_but_never_itself() {
    let mut s = MemoryStore::new();
    let target = capture(&mut s, "the target", "").unwrap().id;
    let linker = capture(&mut s, &format!("see [it](note:{target}) for detail"), "").unwrap().id;
    capture(&mut s, "unrelated, no links", "").unwrap();

    let back: Vec<String> = backlinks(&s, &target).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(back, vec![linker], "only the note that references the target");
    assert!(!back.contains(&target), "a note is never its own backlink");
}

#[test]
fn an_embed_counts_as_a_backlink_and_an_unreferenced_note_has_none() {
    let mut s = MemoryStore::new();
    let target = capture(&mut s, "target", "").unwrap().id;
    let embedder = capture(&mut s, &format!("![](note:{target})"), "").unwrap().id;
    let lonely = capture(&mut s, "nobody links to me", "").unwrap().id;

    let back: Vec<String> = backlinks(&s, &target).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(back, vec![embedder], "an embed is a link too");
    assert!(backlinks(&s, &lonely).unwrap().is_empty(), "no references, no backlinks");
}
