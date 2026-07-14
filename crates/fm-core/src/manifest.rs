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
