//! **Going backwards is now something a user can do, so a vault must survive meeting its own
//! future.**
//!
//! Since 2026-09-10 the app updates itself in place and keeps the previous version to fall back to
//! (`decisions.md#toolchain`, *the app updates itself in place, and the folder stops moving*). Two
//! of the three ways back involve no Rust code at all: the launcher restores the previous
//! `program/` by itself when a new version will not start, and a folder can simply be carried
//! backwards on a USB stick. Neither can be taught where the vaults are — the launcher is a shell
//! script — so the check has to live where every route into a vault passes.
//!
//! What it guards against is not obvious. `init_schema` is `CREATE TABLE IF NOT EXISTS`, which
//! leaves a newer table exactly as it found it. Today that is survivable by luck: both columns
//! added so far are satisfiable by an older insert. The day a future version adds a `NOT NULL`
//! column without a default, every insert from an older build fails, `MultiStore` files every vault
//! under `unopened`, `list_vaults` answers `[]` — and `[]` is the first-run signal, so the whole
//! library disappears behind a "create your first vault" screen. The documented escape is "delete
//! `index.sqlite` and reopen", which needs a terminal, and this app's users have none.

use fm_core::{FileStore, Store};
use fm_model::{Kind, Object};
use fm_query::Query;
use rusqlite::Connection;
use tempfile::tempdir;

/// A vault with one note in it, written the way every other test here writes one.
fn vault_with_a_note(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let mut store = FileStore::named(&root, "main").unwrap();
    store.put(&Object::new(Kind::Note, body)).unwrap();
    (dir, root)
}

/// **The failure this exists to prevent, simulated exactly.**
///
/// Bumping `user_version` alone proves nothing on the desktop: `FileStore::named` reindexes
/// unconditionally and re-stamps the marker, so a stale number heals itself by accident. The real
/// hazard is the *table*. `init_schema` is `CREATE TABLE IF NOT EXISTS`, which leaves a newer
/// `objects` exactly as it found it — so a column a future version added `NOT NULL` with no
/// default makes every insert from this build fail. That is what turns into `MultiStore` filing
/// every vault under `unopened`, `list_vaults` answering `[]`, and the library vanishing behind a
/// "create your first vault" screen.
///
/// So this test writes that table, not just that number. It fails without the discard.
#[test]
fn an_index_from_a_newer_version_is_rebuilt_rather_than_reused() {
    let (_d, root) = vault_with_a_note("work I cannot lose");
    let index = root.join("index.sqlite");
    assert!(index.exists(), "the first open wrote an index");

    // A formicaria from the future: `objects` has gained a column this build knows nothing about,
    // and it is `NOT NULL` with no default — so any insert this build writes is rejected.
    let db = Connection::open(&index).unwrap();
    db.execute_batch(
        "ALTER TABLE objects ADD COLUMN a_future_column TEXT NOT NULL DEFAULT 'x';
         CREATE TABLE objects_new (
             id TEXT PRIMARY KEY, path TEXT NOT NULL, mtime_ns INTEGER NOT NULL,
             content TEXT NOT NULL, kind TEXT NOT NULL, fts_rowid INTEGER,
             a_future_column TEXT NOT NULL);
         DROP TABLE objects;
         ALTER TABLE objects_new RENAME TO objects;
         PRAGMA user_version = 99;",
    )
    .unwrap();
    drop(db);
    // And a hot journal beside it, the shape a process killed mid-transaction leaves.
    std::fs::write(root.join("index.sqlite-journal"), b"half a transaction").unwrap();

    let store = FileStore::named(&root, "main").expect("the vault still opens");
    let rows = store.query(&Query::default()).unwrap().rows;
    assert_eq!(rows.len(), 1, "the note survived a downgrade");
    assert!(rows[0].body.contains("work I cannot lose"));
    drop(store);

    let db = Connection::open(&index).unwrap();
    let v: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, 2, "the index is ours again, stamped with our own schema");
    let leftover: i64 = db
        .query_row(
            "SELECT count(*) FROM pragma_table_info('objects') WHERE name = 'a_future_column'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leftover, 0, "the future table went with the file, rather than being adopted");
}

/// **An index of our own schema is left alone.** The counterpart that stops this discarding a
/// perfectly good index because someone read the comparison the wrong way round.
#[test]
fn an_index_this_version_wrote_is_kept() {
    let (_d, root) = vault_with_a_note("still here");
    let index = root.join("index.sqlite");

    // A marker that only survives if the file itself is not replaced.
    let db = Connection::open(&index).unwrap();
    db.execute_batch("CREATE TABLE a_marker (x INTEGER)").unwrap();
    drop(db);

    let store = FileStore::named(&root, "main").unwrap();
    assert_eq!(store.query(&Query::default()).unwrap().rows.len(), 1);
    drop(store);

    let db = Connection::open(&index).unwrap();
    let kept: i64 = db
        .query_row("SELECT count(*) FROM sqlite_master WHERE name = 'a_marker'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(kept, 1, "an index at our own schema is reused, not thrown away");
}
