//! FileStore: markdown files are the truth; a SQLite table is the disposable
//! per-machine index, rebuilt from the files on open. It implements the same
//! `Store` contract as `MemoryStore`, so the two are interchangeable behind the
//! seam — a test asserts they return equivalent query results.
//!
//! S0 keeps the query path simple: load every object from the index and run the
//! pure engine (identical to MemoryStore). SQLite FTS5 and SQL-side filtering
//! arrive in S1, where search performance is the point.

use crate::frontmatter;
use crate::{Reindex, ReindexStats, Store, StoreError};
use fm_model::{Id, Object};
use fm_query::{run, Query, QueryResult};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct FileStore {
    notes: PathBuf,
    db: Connection,
}

impl FileStore {
    /// Open (creating if needed) a vault at `root`, then rebuild the index from
    /// the files on disk. The index is disposable: delete `index.sqlite`,
    /// reopen, and it reconstructs exactly.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref();
        let notes = root.join("notes");
        fs::create_dir_all(&notes).map_err(io)?;
        let db = Connection::open(root.join("index.sqlite")).map_err(sql)?;
        let mut store = FileStore { notes, db };
        store.init_schema()?;
        store.reindex(Reindex::Full)?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), StoreError> {
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS objects (
                     id        TEXT PRIMARY KEY,
                     path      TEXT NOT NULL,
                     mtime_ns  INTEGER NOT NULL,
                     content   TEXT NOT NULL
                 );",
            )
            .map_err(sql)
    }

    fn path_for(&self, id: Id) -> PathBuf {
        self.notes.join(format!("{id}.md"))
    }

    /// Write via temp-file + rename so the canonical file is never half-written.
    /// The temp lives in the same directory, so the rename is atomic.
    fn write_atomic(path: &Path, contents: &str) -> Result<(), StoreError> {
        let tmp = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(io)?;
            f.write_all(contents.as_bytes()).map_err(io)?;
            f.sync_all().map_err(io)?;
        }
        fs::rename(&tmp, path).map_err(io)?;
        Ok(())
    }

    fn upsert_row(&self, id: Id, path: &Path, content: &str) -> Result<(), StoreError> {
        let mtime = mtime_ns(path)?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO objects (id, path, mtime_ns, content)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id.to_string(), path.to_string_lossy(), mtime, content],
            )
            .map_err(sql)?;
        Ok(())
    }

    fn load_all(&self) -> Result<Vec<Object>, StoreError> {
        let mut stmt = self.db.prepare("SELECT content FROM objects").map_err(sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(sql)?;
        let mut objs = Vec::new();
        for row in rows {
            let content = row.map_err(sql)?;
            objs.push(frontmatter::from_file(&content).map_err(parse)?);
        }
        Ok(objs)
    }
}

impl Store for FileStore {
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError> {
        let content: Option<String> = self
            .db
            .query_row(
                "SELECT content FROM objects WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql)?;
        match content {
            Some(c) => Ok(Some(frontmatter::from_file(&c).map_err(parse)?)),
            None => Ok(None),
        }
    }

    fn put(&mut self, obj: &Object) -> Result<(), StoreError> {
        let content = frontmatter::to_file(obj).map_err(parse)?;
        let path = self.path_for(obj.id);
        Self::write_atomic(&path, &content)?;
        self.upsert_row(obj.id, &path, &content)?;
        Ok(())
    }

    fn delete(&mut self, id: Id) -> Result<(), StoreError> {
        if self.get(id)?.is_none() {
            return Err(StoreError::NotFound(id));
        }
        let path = self.path_for(id);
        if path.exists() {
            fs::remove_file(&path).map_err(io)?;
        }
        self.db
            .execute("DELETE FROM objects WHERE id = ?1", [id.to_string()])
            .map_err(sql)?;
        Ok(())
    }

    fn query(&self, q: &Query) -> Result<QueryResult, StoreError> {
        let objs = self.load_all()?;
        Ok(run(q, &objs))
    }

    fn reindex(&mut self, _mode: Reindex) -> Result<ReindexStats, StoreError> {
        // S0: always a full rebuild — the index is disposable and cheap at this
        // scale. Incremental (mtime-diff) reindex arrives with external-edit
        // polling in a later slice.
        self.db.execute_batch("DELETE FROM objects;").map_err(sql)?;
        let mut scanned = 0usize;
        let mut updated = 0usize;
        for entry in fs::read_dir(&self.notes).map_err(io)? {
            let path = entry.map_err(io)?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            scanned += 1;
            let content = fs::read_to_string(&path).map_err(io)?;
            let obj = frontmatter::from_file(&content).map_err(parse)?;
            self.upsert_row(obj.id, &path, &content)?;
            updated += 1;
        }
        Ok(ReindexStats { scanned, updated })
    }
}

fn mtime_ns(path: &Path) -> Result<i64, StoreError> {
    let modified = fs::metadata(path).map_err(io)?.modified().map_err(io)?;
    Ok(modified
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0))
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
fn sql(e: rusqlite::Error) -> StoreError {
    StoreError::Io(format!("sqlite: {e}"))
}
fn parse(e: frontmatter::ParseError) -> StoreError {
    StoreError::Parse(e.to_string())
}
