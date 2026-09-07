//! **One unopenable vault must not take the others with it.**
//!
//! `MultiStore::open` used to `?` on the first failure, so a cloned vault whose directory had gone —
//! or one corrupt `index.sqlite` — refused *every* vault, i.e. the whole app, over one folder. The
//! documented escape hatch ("delete `index.sqlite` and reopen, it reconstructs exactly") needs a
//! shell, which the phone does not have and the owner does not use, so that was terminal.
//!
//! The fix is only an improvement if the missing vault is **named**: silently skipping it would trade
//! a wall for a lie, and a vault that is simply not there is indistinguishable from lost notes.

use fm_core::{MultiStore, Store};
use fm_query::Query;

#[test]
fn a_vault_that_cannot_open_does_not_refuse_the_others_and_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good");
    std::fs::create_dir_all(good.join("notes")).unwrap();
    // A regular *file* where a vault directory should be: unopenable, and the shape a moved or
    // half-deleted vault leaves behind.
    let broken = dir.path().join("broken");
    std::fs::write(&broken, b"not a directory").unwrap();

    let store = MultiStore::open(&[
        ("good".to_string(), good.clone()),
        ("broken".to_string(), broken.clone()),
    ])
    .expect("open must succeed for the vaults that can be opened");

    assert_eq!(store.names(), vec!["good"], "the good vault is live");
    // Named, with a reason — this is what the heartbeat reports and the UI states.
    let unopened = store.unopened();
    assert_eq!(unopened.len(), 1, "{unopened:?}");
    assert_eq!(unopened[0].0, "broken");
    assert!(!unopened[0].1.is_empty(), "and it says why");
    // And the app is usable: a query over the surviving vault answers rather than erroring.
    assert!(store.query(&Query::default()).is_ok(), "the app still works");
}

#[test]
fn all_good_vaults_report_nothing_unopened() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    for p in [&a, &b] {
        std::fs::create_dir_all(p.join("notes")).unwrap();
    }
    let store = MultiStore::open(&[("a".to_string(), a), ("b".to_string(), b)]).unwrap();
    assert_eq!(store.names(), vec!["a", "b"]);
    assert!(store.unopened().is_empty(), "the normal case says nothing");
}
