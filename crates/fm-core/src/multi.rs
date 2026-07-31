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
    /// Configured vaults that would not open, as `(name, why)` — kept so the app can say which and
    /// why, instead of a vault silently not being there. See [`Self::open`].
    unopened: Vec<(String, String)>,
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
    ///
    /// **One unopenable vault must not take the others with it.** This used to `?` on the first
    /// failure, so a cloned vault whose directory had gone, or one corrupt `index.sqlite`, refused
    /// *every* vault — the whole app, over one folder. And the documented escape hatch ("delete
    /// `index.sqlite` and reopen, it reconstructs exactly") needs a shell, which the phone does not
    /// have and the owner does not use.
    ///
    /// So: open what opens, and **keep the failures by name** ([`Self::unopened`]) rather than
    /// swallowing them. Skipping silently would trade a wall for a lie — the user would see a vault
    /// simply missing, which is indistinguishable from data loss. Same discipline as
    /// `FileStore::skipped`: named, never dropped.
    pub fn open<P: AsRef<Path>>(vaults: &[(String, P)]) -> Result<Self, StoreError> {
        let mut open = Vec::new();
        let mut unopened = Vec::new();
        for (name, path) in vaults {
            match FileStore::named(path, name.clone()) {
                Ok(store) => open.push(store),
                Err(e) => unopened.push((name.clone(), e.to_string())),
            }
        }
        Ok(MultiStore { vaults: open, unopened })
    }

    /// Vaults that are configured but could not be opened, as `(name, why)`. Empty is the normal
    /// case; anything here is a vault the user is configured for and cannot currently see, which is
    /// something the app must say out loud.
    pub fn unopened(&self) -> &[(String, String)] {
        &self.unopened
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

/// **The same set of vaults, minus the ones this caller is not an audience for.**
///
/// `MultiStore` exists to let you *see* across a boundary you cannot accidentally cross —
/// which is exactly right for the person sitting at the machine, and exactly wrong for anyone
/// else. Everything above assumes the caller is entitled to every vault, because until a client
/// could arrive over a network that was true by construction.
///
/// This is the same type with a guest list. It is a **view, not a second store**: no separate
/// index, no copy, and it goes through the identical `candidates`/`route` seams, so a scoped
/// read cannot drift from an unscoped one. `None` means "every vault" and is the default
/// everywhere — `fm-cli`, the phone, and any caller on loopback — so nothing that exists today
/// changes shape.
///
/// **Why in `fm-core` and not in the command layer.** The mechanism needs `MultiStore`'s vault
/// list, and a filter applied *after* a federated read is not a filter, it is a redaction —
/// `candidates` would already have loaded the other audience's notes into memory to drop them
/// again, and every future read path would have to remember to redact. Narrowing the set the
/// read runs over is the only version that cannot be forgotten. The *policy* — which vaults a
/// given caller gets — stays out of here entirely; this type only enforces what it is handed.
pub struct Scoped<'a> {
    inner: &'a mut MultiStore,
    /// `None` = every vault. `Some` = these names, and no others.
    allow: Option<&'a [String]>,
}

impl<'a> Scoped<'a> {
    /// Restrict `inner` to `allow`. `None` is the unrestricted view and costs nothing.
    pub fn new(inner: &'a mut MultiStore, allow: Option<&'a [String]>) -> Self {
        Scoped { inner, allow }
    }

    fn permits(&self, vault: &str) -> bool {
        match self.allow {
            None => true,
            Some(names) => names.iter().any(|n| n == vault),
        }
    }

    /// The vault a note with no stated audience belongs in.
    ///
    /// **Not `vaults[0]`, which is what the unscoped path uses.** A capture from a client
    /// scoped to "lab" carries no vault — every fresh capture does — and routing it to the
    /// configured default would file it in "personal": a note written by someone who cannot
    /// even read that vault, and a disclosure `git` then makes permanent. The default for a
    /// scoped caller is the first vault it can actually see.
    fn default_vault(&self) -> Result<String, StoreError> {
        match self.allow {
            None => Ok(String::new()),
            Some(names) => names
                .iter()
                .find(|n| self.inner.vaults.iter().any(|v| v.name() == n.as_str()))
                .cloned()
                .ok_or(StoreError::NoVaults),
        }
    }

    fn visible(&self) -> impl Iterator<Item = &FileStore> {
        self.inner.vaults.iter().filter(|v| self.permits(v.name()))
    }

    /// The audiences this caller can see, in configured order.
    pub fn names(&self) -> Vec<&str> {
        self.visible().map(|v| v.name()).collect()
    }

    /// Whether this caller may touch `vault`. The blob route needs this: `GET /api/blob` is
    /// not a command, so it cannot reach the seams below and has to ask directly.
    pub fn allows(&self, vault: &str) -> bool {
        self.permits(vault)
    }

    /// Unreadable notes, from the visible vaults only — a scoped client must not learn that a
    /// vault it cannot see has a conflicted merge in it.
    pub fn skipped(&self) -> Vec<crate::SkippedNote> {
        self.visible().flat_map(|v| v.skipped().iter().cloned()).collect()
    }
}

impl Store for Scoped<'_> {
    /// Scoped by **where the note lives**, not by what it is. A ULID is globally unique, so an
    /// id from another audience resolves perfectly well against the full store — which is the
    /// whole hazard: an id learned legitimately (a link in a shared note, a screenshot) would
    /// otherwise read straight out of a vault the caller was never given.
    fn get(&self, id: Id) -> Result<Option<Object>, StoreError> {
        for v in self.visible() {
            if let Some(o) = v.get(id)? {
                return Ok(Some(o));
            }
        }
        Ok(None)
    }

    /// Routed exactly as before, but the route has to be one this caller was given — and an
    /// unstated audience resolves to *their* default, not the machine's.
    fn put(&mut self, obj: &Object) -> Result<(), StoreError> {
        let vault =
            if obj.vault.is_empty() { self.default_vault()? } else { obj.vault.clone() };
        if !self.permits(&vault) {
            return Err(StoreError::Io(format!("no vault named '{vault}'")));
        }
        if vault == obj.vault {
            self.inner.route(&vault)?.put(obj)
        } else {
            // Re-stamp rather than mutate the caller's object: the vault is part of what gets
            // written, so routing it here and leaving the note claiming a different audience
            // would be a file that disagrees with its own location.
            let mut obj = obj.clone();
            obj.vault = vault.clone();
            self.inner.route(&vault)?.put(&obj)
        }
    }

    /// Only from a vault this caller can see. A note it cannot read, it cannot delete —
    /// otherwise `delete` is a way to destroy what you were never shown.
    fn delete(&mut self, id: Id) -> Result<(), StoreError> {
        let allow = self.allow.map(<[String]>::to_vec);
        for v in &mut self.inner.vaults {
            if allow.as_ref().is_some_and(|a| !a.iter().any(|n| n == v.name())) {
                continue;
            }
            match v.delete(id) {
                Err(StoreError::NotFound(_)) => continue,
                other => return other,
            }
        }
        Err(StoreError::NotFound(id))
    }

    /// The narrowing seam, over the visible vaults only. **This is the enforcement point that
    /// matters**: board, agenda, search, recent, activity and every `.view` reach storage
    /// through here, so they are all scoped by this one function rather than each remembering.
    fn candidates(&self, filter: &Filter) -> Result<(Vec<Object>, Filter), StoreError> {
        let mut all = Vec::new();
        let mut residual = filter.clone();
        for v in self.visible() {
            let (objs, r) = v.candidates(filter)?;
            all.extend(objs);
            residual = r;
        }
        Ok((all, residual))
    }

    fn reindex(&mut self, mode: Reindex) -> Result<ReindexStats, StoreError> {
        let allow = self.allow.map(<[String]>::to_vec);
        let mut total = ReindexStats::default();
        for v in &mut self.inner.vaults {
            if allow.as_ref().is_some_and(|a| !a.iter().any(|n| n == v.name())) {
                continue;
            }
            let s = v.reindex(mode)?;
            total.scanned += s.scanned;
            total.updated += s.updated;
            total.removed += s.removed;
            total.skipped.extend(s.skipped);
        }
        Ok(total)
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
