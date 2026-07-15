//! FileStore proves S0's pipeline (write -> disk -> reindex -> read) and that it
//! honours the SAME `Store` contract as `MemoryStore`.

use fm_core::{FileStore, MemoryStore, Store};
use fm_model::{Kind, Object};
use fm_query::{Filter, Predicate, Query, SortKey};
use tempfile::tempdir;

fn seed(store: &mut dyn Store) {
    let mut a = Object::new(Kind::Note, "trust region clipping");
    a.status = Some("doing".into());
    a.tags = vec!["meta-rl".into()];
    let mut b = Object::new(Kind::Note, "advantage estimator");
    b.status = Some("done".into());
    let c = Object::new(Kind::Note, "idea about GAE lambda");
    store.put(&a).unwrap();
    store.put(&b).unwrap();
    store.put(&c).unwrap();
}

/// The S0 acceptance test: type -> atomic write -> reindex on reload -> still there.
#[test]
fn write_reindex_read() {
    let dir = tempdir().unwrap();
    let vault = dir.path();

    let id = {
        let mut s = FileStore::open(vault).unwrap();
        let o = Object::new(Kind::Note, "still here after reload");
        let id = o.id;
        s.put(&o).unwrap();
        id
    };

    // A real Markdown file landed on disk.
    assert!(vault.join(format!("notes/{id}.md")).exists());

    // "Reload": a brand-new FileStore reindexes from files and still has it.
    let s2 = FileStore::open(vault).unwrap();
    let got = s2.get(id).unwrap().expect("note survives reload");
    assert_eq!(got.body, "still here after reload");
}

/// The index is genuinely disposable: delete it, reopen, it rebuilds from files.
#[test]
fn index_is_disposable() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    {
        let mut s = FileStore::open(vault).unwrap();
        seed(&mut s);
    }
    std::fs::remove_file(vault.join("index.sqlite")).unwrap();

    let s = FileStore::open(vault).unwrap();
    let r = s.query(&Query::default()).unwrap();
    assert_eq!(r.total, 3);
}

/// FileStore and MemoryStore honour one contract: the same queries return the
/// same objects (compared as sets, since neither promises order without a sort).
#[test]
fn filestore_matches_memorystore() {
    let dir = tempdir().unwrap();
    let mut fs_store = FileStore::open(dir.path()).unwrap();
    seed(&mut fs_store);

    let mut mem = MemoryStore::new();
    for o in fs_store.query(&Query::default()).unwrap().rows {
        mem.put(&o).unwrap();
    }

    let queries = vec![
        Query { filter: Filter::new().and(Predicate::Text("trust".into())), ..Default::default() },
        Query { group_by: Some("status".into()), ..Default::default() },
        Query { sort: vec![SortKey::asc("created")], ..Default::default() },
    ];
    for q in &queries {
        let a = fs_store.query(q).unwrap();
        let b = mem.query(q).unwrap();
        assert_eq!(a.total, b.total);
        let mut ida: Vec<_> = a.rows.iter().map(|o| o.id).collect();
        let mut idb: Vec<_> = b.rows.iter().map(|o| o.id).collect();
        ida.sort();
        idb.sort();
        assert_eq!(ida, idb, "FileStore and MemoryStore disagree");
    }
}
