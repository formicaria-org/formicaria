//! Integrity manifest — a plain sha256 inventory of the blob store, written as
//! `vault/manifest.json` (git-tracked, small). Because blobs are content-
//! addressed, the manifest key *is* the hash; the manifest adds an authoritative
//! record of *which* blobs should exist and how big they are, so `verify` can
//! tell "a blob rotted" (content no longer matches its name) from "a blob is
//! gone" (recorded but absent) from "a blob appeared" (present but unrecorded).
//!
//! It is deliberately a boring, forever-readable format (à la an OCI image
//! manifest); a future step may minisign it, but the durability value is in the
//! inventory itself.

use crate::blob::BlobStore;
use crate::StoreError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MANIFEST_VERSION: u32 = 1;

/// An empty inventory, so [`Manifest::merge`] can treat "no common ancestor" as one more
/// (empty) side rather than as a special case.
static EMPTY: Manifest = Manifest { schema: MANIFEST_VERSION, blobs: BTreeMap::new() };

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub schema: u32,
    /// sha256 hash -> byte size. The key is the content address itself.
    pub blobs: BTreeMap<String, u64>,
}

impl Manifest {
    /// Inventory the blob store as it is on disk right now.
    pub fn build(vault: &Path) -> Result<Manifest, StoreError> {
        let mut blobs = BTreeMap::new();
        for path in BlobStore::new(vault).blob_paths() {
            let name = file_name(&path);
            let size = fs::metadata(&path).map_err(io)?.len();
            blobs.insert(name, size);
        }
        Ok(Manifest { schema: MANIFEST_VERSION, blobs })
    }

    pub fn path(vault: &Path) -> PathBuf {
        vault.join("manifest.json")
    }

    /// Write atomically (temp + rename), so a crash never leaves a half manifest.
    pub fn write(&self, vault: &Path) -> Result<(), StoreError> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| StoreError::Io(e.to_string()))?;
        let path = Self::path(vault);
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(io)?;
            f.write_all(json.as_bytes()).map_err(io)?;
            f.write_all(b"\n").map_err(io)?;
            f.sync_all().map_err(io)?;
        }
        fs::rename(&tmp, &path).map_err(io)?;
        Ok(())
    }

    /// Read one manifest from an explicit file. The merge driver is handed three temp files by
    /// git's plumbing, with no vault anywhere in sight, so it cannot go through [`Manifest::read`].
    /// An absent or empty file is `None` — git passes an empty `%O` when the file is new on both
    /// sides, which is "no common ancestor", not a parse error.
    pub fn read_file(path: &Path) -> Result<Option<Manifest>, StoreError> {
        let s = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(e)),
        };
        if s.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_str(&s).map_err(|e| StoreError::Parse(e.to_string()))?))
    }

    /// Write to an explicit file, atomically. The driver's counterpart to
    /// [`Manifest::read_file`]: git expects the answer written back over `%A`, wherever that
    /// happens to be.
    ///
    /// Temp + rename for the same reason [`Manifest::write`] does it — a bare `fs::write` that
    /// failed halfway would leave a truncated JSON document sitting in the worktree *and* fail
    /// the driver, which is the worst of both.
    pub fn write_file(&self, path: &Path) -> Result<(), StoreError> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| StoreError::Io(e.to_string()))?;
        let tmp = path.with_extension("json.merge.tmp");
        fs::write(&tmp, json + "\n").map_err(io)?;
        fs::rename(&tmp, path).map_err(io)
    }

    /// Merge two inventories. **Purely additive: `base ∪ ours ∪ theirs`.** Never conflicts.
    ///
    /// Git merged `manifest.json` as ordinary text, so two people ingesting a file on the same
    /// day produced `<<<<<<<` markers inside a JSON document no user wrote, can read, or can
    /// resolve — and while it sat conflicted `commit_all` refused to commit *anything* in the
    /// vault. A union fixes that: the key *is* the content address, so two sides can never
    /// disagree about what an entry means.
    ///
    /// **Why there is no deletion rule, which is the subtle part.** The obvious 3-way rule —
    /// "in base and gone from one side ⇒ that side deleted it" — is *wrong here*, and it
    /// silently destroys the manifest's reason to exist. `blobs/` is gitignored
    /// (`git::write_gitignore`) and every writer is a full rebuild from local disk
    /// ([`Manifest::build`]), so this is a **git-tracked inventory of a per-machine store**:
    /// each clone legitimately holds a different subset of the blobs. "Absent on their side"
    /// therefore means *"they never received those bytes"*, not *"they deleted them"* — the
    /// format carries no deletion intent to recover. Applying the rule anyway made the manifest
    /// converge on the **intersection** of what each machine happened to hold, erasing records
    /// of blobs that exist; `verify` would then report the survivor as unrecorded and advise a
    /// rebuild, which re-expressed the same false deletion on the next merge.
    ///
    /// The cost of being additive is an entry that outlives its blob everywhere, which `verify`
    /// reports as "recorded blob is missing from the store" and a `fm manifest` rebuild clears.
    /// That is a visible, recoverable inaccuracy; the alternative was silent, permanent loss of
    /// the very record the file exists to keep.
    ///
    /// `base` is still taken (and unioned) rather than ignored: an entry both sides happen to
    /// lack locally is still a blob the vault knows about.
    pub fn merge(base: Option<&Manifest>, ours: &Manifest, theirs: &Manifest) -> Manifest {
        let mut blobs = BTreeMap::new();
        for m in [base.unwrap_or(&EMPTY), theirs, ours] {
            for (hash, size) in &m.blobs {
                // `ours` last, so a size disagreement resolves to ours *deterministically*
                // rather than by iteration order. Sizes should never disagree for one content
                // address, but `build` records whatever `fs::metadata` says — so a truncated or
                // rotted file yields a different size for the same hash. Picking a side by rule
                // keeps two clones merging the same pair of commits to byte-identical output;
                // `verify --scrub` is what actually catches the rot.
                blobs.insert(hash.clone(), *size);
            }
        }
        // **Never claim a schema we do not implement.** Taking `max` let a v1 binary emit a
        // v1-semantics map labelled v2, which a v2 reader would then trust.
        let schema = ours.schema.max(theirs.schema).min(MANIFEST_VERSION);
        Manifest { schema, blobs }
    }

    pub fn read(vault: &Path) -> Result<Option<Manifest>, StoreError> {
        let path = Self::path(vault);
        if !path.exists() {
            return Ok(None);
        }
        let s = fs::read_to_string(&path).map_err(io)?;
        Ok(Some(serde_json::from_str(&s).map_err(|e| StoreError::Parse(e.to_string()))?))
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(entries: &[(&str, u64)]) -> Manifest {
        Manifest {
            schema: MANIFEST_VERSION,
            blobs: entries.iter().map(|(h, s)| (h.to_string(), *s)).collect(),
        }
    }

    /// The case that produced `<<<<<<<` inside a JSON file: two people ingest on the same
    /// day, from the same base. Both files must survive, and there is nothing to resolve.
    #[test]
    fn concurrent_ingests_both_survive_and_cannot_conflict() {
        let base = m(&[("aa", 1)]);
        let ours = m(&[("aa", 1), ("bb", 2)]);
        let theirs = m(&[("aa", 1), ("cc", 3)]);

        let merged = Manifest::merge(Some(&base), &ours, &theirs);

        assert_eq!(merged, m(&[("aa", 1), ("bb", 2), ("cc", 3)]));
    }

    /// **The merge is additive, and this is the test that says why.** "In base, absent on one
    /// side" is NOT a deletion here: `blobs/` is gitignored and the manifest is rebuilt from
    /// local disk, so each clone holds a different subset and absence only means "they never
    /// received those bytes". Honouring it as a deletion made the manifest converge on the
    /// intersection of what each machine happened to hold — erasing the record of a blob that
    /// exists, which is the one thing the file is for.
    #[test]
    fn a_blob_the_other_machine_never_received_is_not_a_deletion() {
        let base = m(&[("aa", 1), ("bb", 2)]);
        let ours = m(&[("aa", 1), ("bb", 2)]); // we hold bb, on disk, right now
        let theirs = m(&[("aa", 1)]); // they rebuilt on a machine without it

        assert_eq!(Manifest::merge(Some(&base), &ours, &theirs), m(&[("aa", 1), ("bb", 2)]));
        // Symmetric in both key set AND values — two clones merging the same pair of commits
        // must produce byte-identical files, or every sync shows a spurious diff.
        assert_eq!(Manifest::merge(Some(&base), &theirs, &ours), m(&[("aa", 1), ("bb", 2)]));
    }

    /// Sizes should never disagree for one content address, but `build` records whatever
    /// `fs::metadata` says — so a truncated file yields a different size for the same hash.
    /// Resolve by rule (ours), never by iteration order, so the output is deterministic.
    #[test]
    fn a_size_disagreement_resolves_deterministically() {
        let ours = m(&[("aa", 100)]);
        let theirs = m(&[("aa", 40)]);

        assert_eq!(Manifest::merge(None, &ours, &theirs), m(&[("aa", 100)]));
        assert_eq!(Manifest::merge(None, &theirs, &ours), m(&[("aa", 40)]));
    }

    /// Never label a map with a schema this binary does not implement.
    #[test]
    fn a_future_schema_is_never_claimed() {
        let ours = m(&[("aa", 1)]);
        let future = Manifest { schema: 99, blobs: Default::default() };

        assert_eq!(Manifest::merge(None, &ours, &future).schema, MANIFEST_VERSION);
    }

    /// No common ancestor (the file is new on both sides): everything is an addition.
    #[test]
    fn with_no_base_every_entry_is_an_addition() {
        let merged = Manifest::merge(None, &m(&[("aa", 1)]), &m(&[("bb", 2)]));
        assert_eq!(merged, m(&[("aa", 1), ("bb", 2)]));
    }
}
