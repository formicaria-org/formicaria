//! A paper enters the vault as an ordinary note, from whatever the user had to hand.

use fm_app::commands;
use fm_core::{MemoryStore, Store};
use fm_model::PropertyValue;

fn store() -> MemoryStore {
    MemoryStore::default()
}

#[test]
fn a_bibtex_paste_becomes_a_paper_note_in_one_write() {
    let mut s = store();
    let meta = commands::create_paper(
        &mut s,
        r#"@inproceedings{vaswani2017attention,
             title={Attention Is All You Need},
             author={Vaswani, Ashish and Shazeer, Noam},
             booktitle={NeurIPS}, year={2017},
             doi={10.48550/arXiv.1706.03762}}"#,
        "vault",
    )
    .unwrap();

    // A **note**, not an asset: `views.rs` hardcodes `Kind(Note)` for every planning view, so an
    // asset could appear in none of them.
    assert_eq!(meta.kind, "note");
    assert_eq!(meta.title.as_deref(), Some("Attention Is All You Need"));
    assert_eq!(meta.tags, vec!["paper".to_string()]);

    let obj = s.get(meta.id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(obj.get("authors"), PropertyValue::Text("Vaswani, Ashish; Shazeer, Noam".into()));
    assert_eq!(obj.get("venue"), PropertyValue::Text("NeurIPS".into()));
    assert_eq!(obj.get("doi"), PropertyValue::Text("10.48550/arXiv.1706.03762".into()));
    // **Typed the way a hand-edited file would type it** — an `Int`, not `Text`. `PropertyValue`
    // compares the variant before the value, so a library holding both sorts into two blocks.
    assert_eq!(obj.get("year"), PropertyValue::Int(2017));
}

#[test]
fn an_identifier_alone_is_enough_to_start() {
    let mut s = store();
    let meta = commands::create_paper(&mut s, "https://arxiv.org/abs/1706.03762", "vault").unwrap();
    let obj = s.get(meta.id.parse().unwrap()).unwrap().unwrap();
    // Normalised off the URL wrapper, so the same paper pasted two ways is one value.
    assert_eq!(obj.get("arxiv"), PropertyValue::Text("1706.03762".into()));
    assert_eq!(obj.tags, vec!["paper".to_string()]);

    let doi = commands::create_paper(&mut s, "doi:10.1000/xyz123", "vault").unwrap();
    let obj = s.get(doi.id.parse().unwrap()).unwrap().unwrap();
    assert_eq!(obj.get("doi"), PropertyValue::Text("10.1000/xyz123".into()));
}

#[test]
fn a_bare_title_is_a_perfectly_good_way_to_start() {
    let mut s = store();
    let meta = commands::create_paper(&mut s, "Attention Is All You Need", "vault").unwrap();
    assert_eq!(meta.title.as_deref(), Some("Attention Is All You Need"));
    assert_eq!(meta.tags, vec!["paper".to_string()]);

    // Even nothing at all: an empty paper you fill in through the details panel.
    let empty = commands::create_paper(&mut s, "   ", "vault").unwrap();
    assert_eq!(empty.title, None);
    assert_eq!(empty.tags, vec!["paper".to_string()]);
    assert_eq!(empty.kind, "note");
}

#[test]
fn a_paper_note_lands_in_the_planning_views_an_asset_never_could() {
    let mut s = store();
    commands::create_paper(&mut s, "Attention Is All You Need", "vault").unwrap();
    // The board's own base filter — `Kind(Note)` — sees it.
    let board = commands::board(&s, "status").unwrap();
    let seen: usize = board.columns.iter().map(|c| c.cards.len()).sum();
    assert_eq!(seen, 1, "a paper is plannable, which is the whole reason it is a note");
    assert_eq!(fm_model::Kind::Note, fm_model::Kind::Note);
}
