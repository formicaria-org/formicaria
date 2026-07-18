//! S5b: the agenda is the closest-deadline view — the same engine, a different
//! filter (`due` set AND not done) and sort (due ascending). Urgency is derived
//! in the card, so there is nothing here but selection and ordering to test.

use fm_app::commands::{agenda, capture, set_property};
use fm_core::{MemoryStore, Store};

fn note(store: &mut dyn Store, body: &str, due: Option<&str>, status: Option<&str>) -> String {
    let id = capture(store, body, "").unwrap().id;
    if let Some(d) = due {
        set_property(store, &id, "due", d).unwrap();
    }
    if let Some(s) = status {
        set_property(store, &id, "status", s).unwrap();
    }
    id
}

#[test]
fn agenda_shows_only_dated_unfinished_items_soonest_first() {
    let mut s = MemoryStore::new();
    let later = note(&mut s, "write discussion", Some("2026-08-01"), Some("todo"));
    let soon = note(&mut s, "submit camera-ready", Some("2026-07-16"), Some("doing"));
    let _no_due = note(&mut s, "someday idea", None, Some("todo")); // excluded: no due
    let _done = note(&mut s, "shipped thing", Some("2026-07-10"), Some("done")); // excluded: done
    let no_status = note(&mut s, "dentist", Some("2026-07-20"), None); // included: null != done

    let items = agenda(&s).unwrap();
    let ids: Vec<&str> = items.iter().map(|c| c.id.as_str()).collect();

    // Sorted by due ascending: soon (07-16) < no_status (07-20) < later (08-01).
    assert_eq!(ids, vec![soon.as_str(), no_status.as_str(), later.as_str()]);
    // The done item and the undated item are absent.
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|c| c.due.is_some()));
    assert!(items.iter().all(|c| c.status.as_deref() != Some("done")));
}

#[test]
fn agenda_is_empty_with_no_deadlines() {
    let mut s = MemoryStore::new();
    note(&mut s, "just a thought", None, None);
    assert!(agenda(&s).unwrap().is_empty());
}
