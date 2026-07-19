//! A set of vaults behind one [`Store`] — *formicaria*, the plural.
//!
//! **Vaults are audiences.** One vault = one repo = one remote = one collaborator list.
//! Not one shared vault with labels on the notes: a frontmatter `access:` field has zero
//! enforcement power, and git history is forever, so one typo would be permanent
//! disclosure to everyone who ever cloned. Location is the permission, and this type is
//! what lets you *see* across a boundary you cannot accidentally cross.
//!
//! It is deliberately thin, because the seams already paid for it:
//!
//! - **Reading across vaults** is [`Store::candidates`] concatenated. Each child narrows
//!   its own FTS; the engine ranks the union once. Grouping, sorting and pagination stay
//!   correct because they were never per-child to begin with.
//! - **Filtering and grouping by vault** needed no query-engine change at all:
//!   `Predicate::Prop` is generic and `Object::get("vault")` answers it.
//! - **Cross-vault links** are free: ULIDs are globally unique, so a `note:<ulid>` in one
//!   vault resolves to a note in another with no URI scheme to invent.
//!
//! **Writing never fans out.** `put` routes by `obj.vault` and goes to exactly one child.
//! That is the only place in the system where a note crosses a boundary, and it is one
//! line, on purpose.

use crate::{FileStore, Reindex, ReindexStats, Store, StoreError};
use fm_model::{Id, Object};
use fm_query::Filter;
use std::path::{Path, PathBuf};

pub struct MultiStore {
    vaults: Vec<FileStore>,
}

impl MultiStore {
    /// Open every vault in the list. The **first is the default**: a note that names no
    /// vault (anything from `Object::new`, i.e. every fresh capture) lands there, so the
    /// first entry should be the personal one.
    ///
    /// **Empty is legal.** It used to be refused, on the reasoning that a store over no
    /// vaults answers every query with silence and reads exactly like an empty vault —
    /// true, and the right call while zero was unreachable. It is now the **first-run
    /// state**, named by `list_vaults` returning `[]` and gated on by the UI, so the
    /// silence is never shown to anyone. Reads over zero vaults are honestly empty;
    /// [`Self::route`] makes writes loud.
    pub fn open<P: AsRef<Path>>(vaults: &[(String, P)]) -> Result<Self, StoreError> {
        let mut open = Vec::new();
        for (name, path) in vaults {
            open.push(FileStore::named(path, name.clone())?);
        }
        Ok(MultiStore { vaults: open })
    }

    /// Bring a vault into the live set. The point of the whole thing: creating a vault
    /// must not need a restart.
    ///
    /// **Appends, never inserts.** `vaults[0]` is the default and receives every fresh
    /// capture, so inserting would silently move where new notes land. At zero the new
    /// vault becomes the default, which is right — it is the only one.
    pub fn add(&mut self, store: FileStore) {
        self.vaults.push(store);
    }

    /// No vaults at all — the first-run state, not an error. See [`Self::open`].
    pub fn is_empty(&self) -> bool {
        self.vaults.is_empty()
    }

    /// The audiences, in configured order. The first is the default for new notes.
    pub fn names(&self) -> Vec<&str> {
        self.vaults.iter().map(|v| v.name()).collect()
    }

    /// The note files this app wrote or deleted in **one** vault since its last successful
    /// commit — what the auto-commit stages. Reached through the concrete type on purpose:
    /// the `Store` seam carries no paths, and that is what keeps a storage swap a backend
    /// change rather than a rewrite.
    pub fn written(&self, vault: &str) -> Vec<PathBuf> {
        self.vaults.iter().find(|v| v.name() == vault).map(|v| v.written()).unwrap_or_default()
    }

    /// Forget one vault's write list — only after its commit actually landed.
    pub fn clear_written(&mut self, vault: &str) {
        if let Some(v) = self.vaults.iter_mut().find(|v| v.name() == vault) {
            v.clear_written();
        }
    }

    /// Notes no vault could read, prefixed with the vault they were in — otherwise
    /// "conflicted.md" names a file the user has three of.
    /// Every vault's unreadable notes, concatenated. No longer prefixes the vault name
    /// into a string: each child already stamps `vault` onto the entry, which is what
    /// lets a caller group the list by audience instead of parsing one back out.
    pub fn skipped(&self) -> Vec<crate::SkippedNote> {
        self.vaults.iter().flat_map(|v| v.skipped().iter().cloned()).collect()
    }

    /// The child a note belongs to. An unnamed vault means "not from a store yet" — a
    /// fresh capture — and gets the default rather than an error: refusing to save a new
    /// note because nobody told it its audience would be absurd.
    ///
    /// With no vaults there is no default to fall back to, so this is where a write over
    /// the first-run state becomes loud rather than indexing `[0]` and panicking.
    fn route(&mut self, vault: &str) -> Result<&mut FileStore, StoreError> {
        if self.vaults.is_empty() {
            return Err(StoreError::NoVaults);
        }
        if vault.is_empty() {
            return Ok(&mut self.vaults[0]);
        }
        self.vaults
            .iter_mut()
            .find(|v| v.name() == vault)
            .ok_or_else(|| StoreError::Io(format!("no vault named '{vault}'")))
    }
}

impl Store for MultiStore {
    /// First hit wins, and there can only be one: ULIDs are globally unique, so an id
    /// identifies a note across every vault without a scheme to disambiguate it.
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError> {
        for v in &self.vaults {
            if let Some(o) = v.get(id)? {
                return Ok(Some(o));
            }
        }
        Ok(None)
    }

    /// Routed, never fanned out — the one place a note crosses an audience boundary.
    fn put(&mut self, obj: &Object) -> Result<(), StoreError> {
        self.route(&obj.vault.clone())?.put(obj)
    }

    /// Delete from whichever vault holds it. `NotFound` only when nobody does.
    fn delete(&mut self, id: Id) -> Result<(), StoreError> {
        for v in &mut self.vaults {
            match v.delete(id) {
                Err(StoreError::NotFound(_)) => continue,
                other => return other,
            }
        }
        Err(StoreError::NotFound(id))
    }

    /// Concatenate. Every child narrows its own FTS index and reports the same residual
    /// — `candidates` derives it from the filter alone, so children cannot disagree, and
    /// they must not: one residual is applied to the whole union, so a child that had
    /// already applied `Text` mixed with one that hadn't would give both false positives
    /// and dropped matches.
    fn candidates(&self, filter: &Filter) -> Result<(Vec<Object>, Filter), StoreError> {
        let mut all = Vec::new();
        let mut residual = filter.clone();
        for v in &self.vaults {
            let (objs, r) = v.candidates(filter)?;
            all.extend(objs);
            residual = r;
        }
        Ok((all, residual))
    }

    fn reindex(&mut self, mode: Reindex) -> Result<ReindexStats, StoreError> {
        let mut total = ReindexStats::default();
        for v in &mut self.vaults {
            let s = v.reindex(mode)?;
            total.scanned += s.scanned;
            total.updated += s.updated;
            total.removed += s.removed;
            total.skipped.extend(s.skipped);
        }
        Ok(total)
    }
}
