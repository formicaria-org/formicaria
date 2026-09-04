//! The storage seam. [`Store`] is the single interface between the query engine
//! and any backend. [`MemoryStore`] is the zero-I/O reference used to verify the
//! query contract; [`FileStore`] (markdown files + SQLite FTS5 index) implements the
//! *same* trait, and [`MultiStore`] fans a whole set of vaults out behind one `Store` —
//! so swapping storage is a backend change, not a rewrite.

use fm_model::{Id, Object};
use fm_query::{Filter, Query, QueryResult};
use std::collections::HashMap;
use thiserror::Error;

pub mod descriptor;   // <vault>/vault.json — the facts git cannot supply
pub mod proposal;     // hard guardrails on a proposal's size (vault policy, pure check)
pub mod frontmatter;
mod file;
pub use file::FileStore;
mod multi;
pub use multi::{ColdStart, MultiStore, Scoped};
pub mod edit;
pub use edit::apply_property;
pub mod blob;
pub mod chunked;
pub mod ingest;
pub use blob::BlobStore;
pub use ingest::{ingest_file, ingest_file_named, Ingested};
pub mod manifest;
pub mod verify;
pub use manifest::Manifest;
pub use verify::{verify, Report, Severity};
pub mod acquire;   // the one step every way of getting a vault from elsewhere shares
pub mod import;    // Logseq/Obsidian -> notes; converts, where `acquire` only moves bytes
pub mod backup;
pub mod git;
// In-process git, for platforms with no `git` binary. Non-default: the desktop shells out.
#[cfg(feature = "native-git")]
pub mod git_native;
// Which git backend the app talks to. **Always call through this, never `git`/`git_native`
// directly** — naming a backend at a call site is what left the phone reporting "git not
// installed" while carrying a working libgit2.
pub mod vcs;
pub mod merge;
pub mod scene;   // element-level 3-way merge for whiteboard bodies (see merge::merge_body)

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("object not found: {0}")]
    NotFound(Id),
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    /// The note changed on disk since we last read it, so the copy the caller is
    /// writing back is stale and would silently discard whoever made that change.
    /// Not retryable: the caller must re-read and decide, which is why this is its
    /// own variant rather than an `Io` string.
    #[error("this note changed on disk since you opened it — reload before saving")]
    Conflict(Id),
    /// There are no vaults at all — the first-run state. Reads over zero vaults are
    /// legitimately empty, but a *write* has nowhere to go, so it says so instead of
    /// picking a vault that does not exist. Its own variant rather than an `Io` string
    /// because the caller (the first-run screen) branches on it.
    #[error("no vaults configured — create one first")]
    NoVaults,
}

/// How much of the index to rebuild. `Full` = drop and rebuild from files — the
/// disposable-index escape hatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reindex {
    Incremental,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReindexStats {
    pub scanned: usize,
    pub updated: usize,
    /// Notes whose file vanished since we last indexed it — deleted by a pull, or
    /// an `rm`. Counted only for [`Reindex::Incremental`]: a `Full` rebuild drops
    /// every row by construction, so "removed" would mean nothing there.
    ///
    /// Kept separate from a *failed* parse on purpose. A note that fails to parse is
    /// re-read on every poll and never succeeds, so counting it as a change would
    /// tell the UI the vault moved on every beat, forever.
    pub removed: usize,
    /// Notes that could not be read. Never an error: a vault that refuses to open
    /// because one file is malformed is a vault you cannot use to fix that file —
    /// and once several people share it, a conflicted merge makes this the
    /// *expected* state, not a rarity. Report and carry on.
    pub skipped: Vec<SkippedNote>,
}

/// One note the indexer could not read, and enough to *act* on it.
///
/// This used to be a formatted string (`"vault: file: why"`), which said what was wrong
/// and then made it impossible to do anything about: the UI could not recover a path from
/// it, so the only advice it could offer was "go find this file yourself". Carrying the
/// path is what turns the notification into a place you can fix things from.
///
/// The set doubles as an **allowlist**. Handing a path to the OS is a capability, so the
/// only paths openable this way are ones the indexer itself just reported as broken —
/// there is no name the caller can supply that widens it, and therefore no traversal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkippedNote {
    /// Which vault it belongs to — the audience, and the reason two vaults may hold
    /// the same filename without the list being ambiguous.
    pub vault: String,
    /// The file's own name, e.g. `01JQ….md`. What a human recognises in a list.
    pub name: String,
    /// Where it actually is. The whole point of the struct.
    pub path: std::path::PathBuf,
    /// Why it could not be read, in the parser's own words.
    pub reason: String,
}

/// The seam. Exactly five methods; nothing filesystem-shaped leaks through — no
/// paths, no mtimes, no directory handles.
pub trait Store {
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError>;
    fn put(&mut self, obj: &Object) -> Result<(), StoreError>;
    fn delete(&mut self, id: Id) -> Result<(), StoreError>;
    fn reindex(&mut self, mode: Reindex) -> Result<ReindexStats, StoreError>;

    /// Everything that *could* match `filter`, plus **what of the filter is left to
    /// apply**. This is the seam that federates.
    ///
    /// A store is allowed to narrow — `FileStore` answers a `Text` predicate with an
    /// FTS5 `MATCH` and loads only the hits — and returns the residual so the pure
    /// engine knows what it already did. **The residual is load-bearing, not
    /// bookkeeping:** FTS5 here is pinned to `remove_diacritics 2` and the engine's
    /// substring scan is not, so re-running `Text` over FTS hits would quietly drop
    /// every diacritic-folded match. A store that cannot narrow simply hands back
    /// everything and the filter untouched.
    ///
    /// **Why not `load_all`.** A federating store cannot implement `query` by asking
    /// each child to `query` and merging: you cannot union results that are already
    /// sorted, grouped and paginated and recover `sort`/`group_by`/`limit`/`total`
    /// from them — the winner of a `limit: 10` across two vaults is not the union of
    /// each vault's top 10. Narrowing is the only part that is per-store; ranking is
    /// global. So children return candidates, a `MultiStore` concatenates them, and
    /// [`Store::query`] runs the engine **once** over the union. FTS federates for
    /// free and `fm-query` still never learns that storage exists.
    fn candidates(&self, filter: &Filter) -> Result<(Vec<Object>, Filter), StoreError>;

    /// Narrow, then rank — once, over everything. Not worth overriding: the whole
    /// point of `candidates` is that this half is identical for every store.
    fn query(&self, q: &Query) -> Result<QueryResult, StoreError> {
        let (objects, residual) = self.candidates(&q.filter)?;
        Ok(fm_query::run(&Query { filter: residual, ..q.clone() }, &objects))
    }
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

    /// Nothing to narrow with: hand back everything and the filter untouched, and let
    /// the engine do all of it. This is the reference behaviour `FileStore` must stay
    /// equivalent to — its FTS path is an optimisation, never a different answer.
    fn candidates(&self, filter: &Filter) -> Result<(Vec<Object>, Filter), StoreError> {
        Ok((self.objects.values().cloned().collect(), filter.clone()))
    }

    fn reindex(&mut self, _mode: Reindex) -> Result<ReindexStats, StoreError> {
        // Nothing to (re)index in memory; the map is already canonical.
        // Nothing to skip or remove: a MemoryStore holds parsed objects, so there is
        // no file that could fail to parse or vanish. The fields exist for
        // FileStore's sake.
        Ok(ReindexStats {
            scanned: self.objects.len(),
            updated: 0,
            removed: 0,
            skipped: Vec::new(),
        })
    }
}
