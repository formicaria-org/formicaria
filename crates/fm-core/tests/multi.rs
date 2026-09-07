//! Multi-vault: seeing across a boundary you cannot accidentally cross.

use fm_core::{FileStore, MultiStore, Store, StoreError};
use fm_model::{Kind, Object, PropertyValue};
use fm_query::{Filter, Op, Predicate, Query, SortKey};
use tempfile::tempdir;

/// Two vaults — personal (the default) and lab — with one note each.
fn two_vaults() -> (tempfile::TempDir, MultiStore) {
    let dir = tempdir().unwrap();
    let personal = dir.path().join("personal");
    let lab = dir.path().join("lab");
    let m = MultiStore::open(&[("personal".into(), &personal), ("lab".into(), &lab)]).unwrap();
    (dir, m)
}

/// The point of the plural: one set of views over several audiences, with every note
/// still knowing — from its location, not its text — which audience it belongs to.
#[test]
fn views_span_vaults_and_every_note_knows_its_audience() {
    let (_d, mut m) = two_vaults();

    let mut mine = Object::new(Kind::Note, "my private half-thought");
    mine.status = Some("todo".into());
    m.put(&mine).unwrap(); // no vault named → the default

    let mut ours = Object::new(Kind::Note, "the lab's shared finding");
    ours.status = Some("todo".into());
    ours.vault = "lab".into();
    m.put(&ours).unwrap();

    // One board, both audiences.
    let all = m.query(&Query::default()).unwrap();
    assert_eq!(all.total, 2, "the board spans vaults");
    assert_eq!(m.get(mine.id).unwrap().unwrap().vault, "personal", "defaulted to the first");
    assert_eq!(m.get(ours.id).unwrap().unwrap().vault, "lab", "routed by name");

    // Filtering by vault needed no query-engine change: `Predicate::Prop` is generic and
    // `Object::get("vault")` answers it.
    let only_lab = m
        .query(&Query {
            filter: Filter::new().and(Predicate::Prop {
                key: "vault".into(),
                op: Op::Eq,
                value: PropertyValue::Text("lab".into()),
            }),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(only_lab.total, 1, "filter by audience");
    assert_eq!(only_lab.rows[0].id, ours.id);

    // And grouping by it, for the same reason.
    let grouped = m.query(&Query { group_by: Some("vault".into()), ..Default::default() }).unwrap();
    let mut names: Vec<_> = grouped
        .groups
        .unwrap()
        .iter()
        .map(|g| match &g.key {
            PropertyValue::Text(t) => t.clone(),
            other => panic!("vault groups by its text: {other:?}"),
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["lab", "personal"], "group by audience");
}

/// Writes are routed, never fanned out — and a note can only land in a vault that
/// exists. This is the one place a note crosses a boundary, so it is the one place that
/// has to be uninteresting.
#[test]
fn a_write_goes_to_exactly_one_vault_and_never_invents_one() {
    let (d, mut m) = two_vaults();

    let mut ours = Object::new(Kind::Note, "for the lab only");
    ours.vault = "lab".into();
    m.put(&ours).unwrap();

    // On disk in the lab's repo, and nowhere near the personal one.
    assert!(d.path().join(format!("lab/notes/{}.md", ours.id)).exists(), "in the lab vault");
    assert!(
        !d.path().join(format!("personal/notes/{}.md", ours.id)).exists(),
        "and not copied into the other audience",
    );

    // A vault nobody configured is an error, not a silent default: quietly writing a
    // note meant for "lab" into "personal" is a disclosure, and the reverse loses it.
    let mut nowhere = Object::new(Kind::Note, "for a vault that isn't there");
    nowhere.vault = "typo".into();
    match m.put(&nowhere) {
        Err(StoreError::Io(msg)) => assert!(msg.contains("typo"), "named the vault: {msg}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Federated search must still be FTS5, not a substring scan — a PDF's extracted text is
/// findable in *every* vault or the feature quietly stopped working when it got plural.
/// This is the case the `candidates` seam exists for: narrow per child, rank once.
#[test]
fn search_still_uses_fts_when_federated_and_pagination_stays_honest() {
    let (_d, mut m) = two_vaults();

    for (i, vault) in [("personal", "personal"), ("lab", "lab")].iter().enumerate() {
        for n in 0..5 {
            let mut o = Object::new(Kind::Note, format!("réunion about trust regions {i}{n}"));
            o.vault = vault.1.to_string();
            m.put(&o).unwrap();
        }
    }
    // One note nobody is looking for, to prove the filter does something.
    let mut other = Object::new(Kind::Note, "unrelated");
    other.vault = "lab".into();
    m.put(&other).unwrap();

    // Diacritic folding is FTS5's (`remove_diacritics 2`), NOT the pure engine's
    // substring scan — so a hit here proves the index answered, across both vaults, and
    // that the residual filter did not re-run `Text` and throw the folded matches away.
    let hits = m
        .query(&Query {
            filter: Filter::new().and(Predicate::Text("reunion".into())),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(hits.total, 10, "FTS federates: five from each vault, accent-folded");

    // The union is ranked once, so a page is a page across the whole set — not each
    // child's top 3 glued together.
    let page = m
        .query(&Query {
            filter: Filter::new().and(Predicate::Text("reunion".into())),
            sort: vec![SortKey::asc("created")],
            limit: Some(3),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.rows.len(), 3, "the window is honoured");
    assert_eq!(page.total, 10, "and total still counts everything that matched");
}

/// Zero vaults is the **first-run state**, not an error. `open` used to refuse it, on the
/// reasoning that silence reads like an empty vault — right while zero was unreachable,
/// wrong now that the UI gates on it and names it. So: reads are honestly empty, and a
/// write — which has nowhere to go — says `NoVaults` rather than indexing `vaults[0]` and
/// panicking. The panic is the whole reason this test exists.
#[test]
fn a_store_over_no_vaults_is_a_legal_state_and_says_so() {
    let empty: &[(String, &std::path::Path)] = &[];
    let mut m = MultiStore::open(empty).unwrap();

    assert!(m.is_empty());
    assert!(m.names().is_empty());

    // Reads: empty, not an error. Concatenating nothing is nothing.
    let r = m.query(&Query::default()).unwrap();
    assert_eq!(r.total, 0, "no vaults → no rows");
    assert!(m.get(fm_model::Id::new()).unwrap().is_none());

    // Writes: loud. Never a panic, and never `Io`, so the caller can branch on it.
    let o = Object::new(Kind::Note, "nowhere to put this");
    assert!(
        matches!(m.put(&o), Err(StoreError::NoVaults)),
        "a write with no vaults must say so, not panic on vaults[0]"
    );
}

/// **This test is the feature.** Creating a vault must make it live without restarting
/// the server — so `add` on an already-open store has to route by name, and queries have
/// to span the newcomer, with no reopen anywhere in sight.
///
/// It also pins `add`'s one rule: it **appends**. `vaults[0]` is the default that receives
/// every fresh capture, so an insert would silently move where new notes land.
#[test]
fn add_makes_a_vault_live_without_a_restart() {
    let dir = tempdir().unwrap();
    let (personal, lab) = (dir.path().join("personal"), dir.path().join("lab"));

    let mut m = MultiStore::open(&[("personal".into(), &personal)]).unwrap();
    let mut mine = Object::new(Kind::Note, "before the lab existed");
    m.put(&mine).unwrap();

    m.add(FileStore::named(&lab, "lab").unwrap());
    assert_eq!(m.names(), vec!["personal", "lab"], "appended, so the default is untouched");

    // The newcomer takes writes by name, immediately.
    let mut ours = Object::new(Kind::Note, "the lab's first note");
    ours.vault = "lab".into();
    m.put(&ours).unwrap();

    // And a capture that names no vault still lands in the original default.
    mine = Object::new(Kind::Note, "still mine");
    m.put(&mine).unwrap();

    // Queries span both, with no reopen.
    let r = m.query(&Query::default()).unwrap();
    assert_eq!(r.total, 3, "the query spans the vault added after opening");
    let vaults: Vec<&str> = r.rows.iter().map(|o| o.vault.as_str()).collect();
    assert_eq!(vaults.iter().filter(|v| **v == "lab").count(), 1);
    assert_eq!(vaults.iter().filter(|v| **v == "personal").count(), 2);
}

/// A `MultiStore` over one vault must behave exactly like that vault. If the plural
/// changes the singular's answers, the abstraction is not free and something is wrong.
#[test]
fn one_vault_through_multistore_matches_the_vault_itself() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("solo");
    {
        let mut s = FileStore::named(&path, "solo").unwrap();
        for body in ["alpha note", "beta note", "gamma"] {
            let mut o = Object::new(Kind::Note, body);
            o.vault = "solo".into();
            s.put(&o).unwrap();
        }
    }
    let single = FileStore::named(&path, "solo").unwrap();
    let multi = MultiStore::open(&[("solo".into(), &path)]).unwrap();

    for q in [
        Query::default(),
        Query { filter: Filter::new().and(Predicate::Text("note".into())), ..Default::default() },
        Query { group_by: Some("status".into()), ..Default::default() },
        Query { sort: vec![SortKey::asc("created")], limit: Some(2), ..Default::default() },
    ] {
        let a = single.query(&q).unwrap();
        let b = multi.query(&q).unwrap();
        assert_eq!(a.total, b.total, "same total");
        let ids = |r: &fm_query::QueryResult| r.rows.iter().map(|o| o.id).collect::<Vec<_>>();
        assert_eq!(ids(&a), ids(&b), "same rows, same order");
    }
}
