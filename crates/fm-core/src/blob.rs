//! Content-addressed blob store. Heavy media never lives in a note and never in
//! git: it is written once, keyed by the sha256 of its bytes, under
//! `vault/blobs/sha256/ab/cd/<hash>`. The hash *is* the identity, so identical
//! bytes are stored once (dedup for free), and `sha256sum` verifies any blob
//! forever — no bespoke format to decode in ten years.
//!
//! The two-level `ab/cd` fan-out keeps any single directory from filling with
//! millions of entries (the git loose-object trick), which matters on a blob
//! volume that may reach a terabyte.

use crate::StoreError;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub struct BlobStore {
    root: PathBuf, // vault/blobs
}

/// Outcome of storing a blob: its hash, and whether the bytes were already
/// present (so the caller can report dedup — "paste an image twice, one blob").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub hash: String,
    pub deduped: bool,
}

impl BlobStore {
    pub fn new(vault: &Path) -> Self {
        BlobStore { root: vault.join("blobs") }
    }

    /// The content-addressed path for a hash, `sha256/ab/cd/<hash>`.
    pub fn path_for(&self, hash: &str) -> PathBuf {
        self.root.join("sha256").join(&hash[0..2]).join(&hash[2..4]).join(hash)
    }

    pub fn exists(&self, hash: &str) -> bool {
        hash.len() >= 4 && self.path_for(hash).exists()
    }

    /// Stream `src` once: hash it while copying to a temp file, then atomically
    /// rename into place. If a blob with that hash already exists, the temp is
    /// discarded and `deduped` is true — the bytes are never written twice.
    pub fn put_file(&self, src: &Path) -> Result<Stored, StoreError> {
        fs::create_dir_all(&self.root).map_err(io_err)?;
        let mut reader = File::open(src).map_err(io_err)?;

        // Hash and buffer to a temp in the blob root (same filesystem → atomic
        // rename). We stream in chunks so a 1 GB checkpoint never loads into RAM.
        let tmp = self.root.join(format!(".incoming-{}", std::process::id()));
        let mut hasher = Sha256::new();
        {
            let mut writer = File::create(&tmp).map_err(io_err)?;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = reader.read(&mut buf).map_err(io_err)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                writer.write_all(&buf[..n]).map_err(io_err)?;
            }
            writer.sync_all().map_err(io_err)?;
        }

        let hash = hex(&hasher.finalize());
        let dest = self.path_for(&hash);
        if dest.exists() {
            let _ = fs::remove_file(&tmp); // already have these exact bytes
            return Ok(Stored { hash, deduped: true });
        }
        fs::create_dir_all(dest.parent().expect("blob path has a parent")).map_err(io_err)?;
        fs::rename(&tmp, &dest).map_err(io_err)?;
        Ok(Stored { hash, deduped: false })
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn io_err(e: io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
