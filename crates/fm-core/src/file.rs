//! FileStore: markdown files are the truth; SQLite is the disposable per-machine
//! index, rebuilt from the files on open. It implements the same `Store` contract
//! as `MemoryStore`, so the two are interchangeable behind the seam — a test
//! asserts they return equivalent query results.
//!
//! Two tables: `objects` holds the canonical file content; `fts` is a SQLite
//! FTS5 full-text index over each object's `searchable_text()`. A `Text`
//! predicate is answered by an FTS5 `MATCH` that loads only the hits; the pure
//! engine then applies the rest of the query over that candidate set. Every
//! other query (no `Text`) loads all objects and runs the engine — identical to
//! MemoryStore. This is the storage half of "full-text search is just a
//! predicate": search is fast, but not special.

use crate::frontmatter;
use crate::{Reindex, ReindexStats, Store, StoreError};
use fm_model::{Id, Object};
use fm_query::{run, Filter, Predicate, Query, QueryResult};
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
        // `objects` = canonical content; `fts` = full-text over searchable_text().
        // The tokenizer is pinned (`unicode61 remove_diacritics 2`) so search
        // folds case and accents the same way on every machine — the guard
        // against FileStore/MemoryStore query drift (MASTERPLAN risk #5).
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS objects (
                     id        TEXT PRIMARY KEY,
                     path      TEXT NOT NULL,
                     mtime_ns  INTEGER NOT NULL,
                     content   TEXT NOT NULL
                 );
                 CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(
                     id UNINDEXED,
                     text,
                     tokenize = 'unicode61 remove_diacritics 2'
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

    /// Index one object into both tables: the `objects` row (canonical content +
    /// mtime) and the `fts` full-text row. The FTS table is plain (not
    /// external-content), so a refresh is delete-then-insert. `searchable_text()`
    /// decides what's searchable — title + body today, + extracted asset text
    /// (pdftotext) once ingest lands in S5.
    fn index_object(&self, obj: &Object, path: &Path, content: &str) -> Result<(), StoreError> {
        let id = obj.id.to_string();
        let mtime = mtime_ns(path)?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO objects (id, path, mtime_ns, content)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, path.to_string_lossy(), mtime, content],
            )
            .map_err(sql)?;
        self.db.execute("DELETE FROM fts WHERE id = ?1", [&id]).map_err(sql)?;
        self.db
            .execute(
                "INSERT INTO fts (id, text) VALUES (?1, ?2)",
                rusqlite::params![id, obj.searchable_text()],
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

    /// Load only the objects whose full text matches an FTS5 expression, joining
    /// back to `objects` for canonical content. Deserializing just the hits (not
    /// all 10k) is what keeps search under the 100 ms budget.
    fn fts_load(&self, match_expr: &str) -> Result<Vec<Object>, StoreError> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT objects.content
                 FROM fts JOIN objects ON objects.id = fts.id
                 WHERE fts MATCH ?1",
            )
            .map_err(sql)?;
        let rows = stmt.query_map([match_expr], |r| r.get::<_, String>(0)).map_err(sql)?;
        let mut objs = Vec::new();
        for row in rows {
            objs.push(frontmatter::from_file(&row.map_err(sql)?).map_err(parse)?);
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
        self.index_object(obj, &path, &content)?;
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
        self.db
            .execute("DELETE FROM fts WHERE id = ?1", [id.to_string()])
            .map_err(sql)?;
        Ok(())
    }

    fn query(&self, q: &Query) -> Result<QueryResult, StoreError> {
        // Full-text is the one predicate FileStore accelerates. When the filter
        // carries `Text`, resolve it via FTS5 MATCH (loading only the hits) and
        // let the pure engine apply the *remaining* predicates + sort/group/page
        // over that candidate set. Without `Text`, load everything and run the
        // engine — the exact MemoryStore path, so the two stores agree.
        match fts_match_expr(&q.filter) {
            Some(expr) => {
                let candidates = self.fts_load(&expr)?;
                Ok(run(&strip_text(q), &candidates))
            }
            None => {
                let objs = self.load_all()?;
                Ok(run(q, &objs))
            }
        }
    }

    fn reindex(&mut self, _mode: Reindex) -> Result<ReindexStats, StoreError> {
        // S0/S1: always a full rebuild — the index is disposable and cheap at
        // this scale. Incremental (mtime-diff) reindex arrives with external-edit
        // polling in a later slice.
        self.db.execute_batch("DELETE FROM objects; DELETE FROM fts;").map_err(sql)?;
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
            self.index_object(&obj, &path, &content)?;
            updated += 1;
        }
        Ok(ReindexStats { scanned, updated })
    }
}

/// Build the FTS5 MATCH expression for a query, or `None` when the pure path
/// should handle it: no `Text` predicate at all, or a `Text` needle that yields
/// no FTS tokens (e.g. punctuation only) — which FTS can't express but a
/// substring scan can. Every token becomes a prefix term (`"tok"*`) so the
/// search box matches as you type; terms AND together, matching FTS5's default.
///
/// `None` is only returned when it is safe: either there is nothing to strip, or
/// *some* `Text` needle is inexpressible, in which case the whole query (Text
/// included) falls back to the substring engine rather than dropping a filter.
fn fts_match_expr(filter: &Filter) -> Option<String> {
    let needles: Vec<&String> = filter
        .all
        .iter()
        .filter_map(|p| match p {
            Predicate::Text(s) => Some(s),
            _ => None,
        })
        .collect();
    if needles.is_empty() {
        return None;
    }
    let mut terms = Vec::new();
    for needle in needles {
        let tokens = fts_tokens(needle);
        if tokens.is_empty() {
            return None; // inexpressible in FTS — let the substring engine handle it
        }
        for t in tokens {
            terms.push(format!("\"{t}\"*"));
        }
    }
    Some(terms.join(" "))
}

/// Split a needle into FTS-safe tokens: alphanumeric runs, lowercased. Because
/// tokens are alphanumeric-only, double-quoting them in the MATCH expression
/// cannot inject FTS operators.
fn fts_tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Drop `Text` predicates from a query — the FTS MATCH already applied them, so
/// the pure engine only evaluates the remaining (structured) predicates.
fn strip_text(q: &Query) -> Query {
    let mut q = q.clone();
    q.filter.all.retain(|p| !matches!(p, Predicate::Text(_)));
    q
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
