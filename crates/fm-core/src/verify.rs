//! `verify` — report-only integrity check, in the spirit of `git fsck`. It never
//! changes anything: it reads the vault, lists what looks wrong, and lets the
//! human decide. The severities encode the plan's stance on media absence: a
//! blob referenced but missing is a **warning** (it may just not have synced
//! yet), while unparseable frontmatter or a bit-rotted blob is an **error**.
//!
//! `--scrub` is the genuinely hard part of "lasts ten years": it re-hashes every
//! blob and flags any whose bytes no longer match their content address — silent
//! bit-rot that nothing else would catch. With a manifest present it also spots
//! blobs that vanished (recorded but absent) or appeared (present but unrecorded).

use crate::blob::{sha256_file, BlobStore};
use crate::manifest::Manifest;
use crate::{frontmatter, StoreError};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Severity,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub notes: usize,
    pub blobs: usize,
    pub scrubbed: bool,
    pub issues: Vec<Issue>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Error).count()
    }
    pub fn warnings(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).count()
    }
    /// Clean = no errors. Warnings (e.g. an unsynced blob) don't fail a verify.
    pub fn ok(&self) -> bool {
        self.errors() == 0
    }
    fn error(&mut self, target: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            severity: Severity::Error,
            target: target.into(),
            message: message.into(),
        });
    }
    fn warn(&mut self, target: impl Into<String>, message: impl Into<String>) {
        self.issues.push(Issue {
            severity: Severity::Warning,
            target: target.into(),
            message: message.into(),
        });
    }
}

pub fn verify(vault: &Path, scrub: bool) -> Result<Report, StoreError> {
    let mut report = Report { scrubbed: scrub, ..Report::default() };
    let blobs = BlobStore::new(vault);

    // 1. Parse every note; an unparseable one is an error. Collect blob refs.
    let notes_dir = vault.join("notes");
    let mut referenced: Vec<(String, String)> = Vec::new();
    if notes_dir.exists() {
        for entry in fs::read_dir(&notes_dir).map_err(io)? {
            let path = entry.map_err(io)?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            report.notes += 1;
            let note = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let content = fs::read_to_string(&path).map_err(io)?;
            match frontmatter::from_file(&content) {
                Ok(obj) => {
                    for a in &obj.assets {
                        if let Some(hash) = a.strip_prefix("sha256:") {
                            referenced.push((note.clone(), hash.to_string()));
                        }
                    }
                }
                Err(e) => report.error(note, format!("unparseable frontmatter: {e}")),
            }
        }
    }

    // 2. A referenced blob that isn't present is a warning (may be unsynced).
    for (note, hash) in &referenced {
        if !blobs.exists(hash) {
            report.warn(
                format!("{note} -> sha256:{}", short(hash)),
                "referenced blob is missing (not synced yet?)",
            );
        }
    }

    // 3. Inventory the blobs; with --scrub, re-hash each to catch bit-rot.
    let paths = blobs.blob_paths();
    report.blobs = paths.len();
    if scrub {
        for path in &paths {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let actual = sha256_file(path)?;
            if actual != name {
                report.error(
                    format!("blobs/.../{}", short(&name)),
                    format!("BIT ROT: content hashes to {} but is stored as {}", short(&actual), short(&name)),
                );
            }
        }
        // Cross-check against the manifest, if one has been written.
        if let Some(manifest) = Manifest::read(vault)? {
            let on_disk: std::collections::BTreeSet<String> =
                paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
            for hash in manifest.blobs.keys() {
                if !on_disk.contains(hash) {
                    report.error(
                        format!("manifest: {}", short(hash)),
                        "recorded blob is missing from the store",
                    );
                }
            }
            for hash in &on_disk {
                if !manifest.blobs.contains_key(hash) {
                    report.warn(
                        format!("blobs/.../{}", short(hash)),
                        "blob not recorded in manifest (run `fm manifest`)",
                    );
                }
            }
        }
    }

    Ok(report)
}

fn short(hash: &str) -> String {
    hash.chars().take(12).collect()
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
