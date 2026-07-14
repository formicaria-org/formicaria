//! The storage seam. [`Store`] is the single interface between the query engine
//! and any backend. [`MemoryStore`] is the zero-I/O reference used to verify the
//! query contract; `FileStore` (markdown files + SQLite FTS5 index) arrives in a
//! later slice and implements the *same* trait — swapping storage is then a
//! backend change, not a rewrite.

use fm_model::{Id, Object};
use fm_query::{run, Query, QueryResult};
use std::collections::HashMap;
use thiserror::Error;

pub mod frontmatter;
mod file;
pub use file::FileStore;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("object not found: {0}")]
    NotFound(Id),
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// How much of the index to rebuild. `Full` = drop and rebuild from files — the
/// disposable-index escape hatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reindex {
    Incremental,
    Full,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReindexStats {
    pub scanned: usize,
    pub updated: usize,
}

/// The seam. Exactly five methods; nothing filesystem-shaped leaks through — no
/// paths, no mtimes, no directory handles.
pub trait Store {
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError>;
    fn put(&mut self, obj: &Object) -> Result<(), StoreError>;
    fn delete(&mut self, id: Id) -> Result<(), StoreError>;
    fn query(&self, q: &Query) -> Result<QueryResult, StoreError>;
    fn reindex(&mut self, mode: Reindex) -> Result<ReindexStats, StoreError>;
}

/// In-memory store — no filesystem, no database. The store *is* the index, so
/// the query-engine contract is verified against it with zero I/O.
#[derive(Default)]
pub struct MemoryStore {
    objects: HashMap<Id, Object>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.objects.len()
    }
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

impl Store for MemoryStore {
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError> {
        Ok(self.objects.get(&id).cloned())
    }

    fn put(&mut self, obj: &Object) -> Result<(), StoreError> {
        self.objects.insert(obj.id, obj.clone());
        Ok(())
    }

    fn delete(&mut self, id: Id) -> Result<(), StoreError> {
        self.objects.remove(&id).map(|_| ()).ok_or(StoreError::NotFound(id))
    }

    fn query(&self, q: &Query) -> Result<QueryResult, StoreError> {
        let objs: Vec<Object> = self.objects.values().cloned().collect();
        Ok(run(q, &objs))
    }

    fn reindex(&mut self, _mode: Reindex) -> Result<ReindexStats, StoreError> {
        // Nothing to (re)index in memory; the map is already canonical.
        Ok(ReindexStats { scanned: self.objects.len(), updated: 0 })
    }
}
