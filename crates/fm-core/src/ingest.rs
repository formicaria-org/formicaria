//! Reference, don't ingest: catalog the artifact, never copy it into a note.
//! One function stores the bytes as a content-addressed blob, sniffs the MIME by
//! magic bytes, and extracts *searchable text* — the highest-value feature
//! (search inside every paper). Any step past hashing may fail and only degrades
//! that one feature; the blob is committed regardless.
//!
//! The extracted text is returned to the caller, which puts it in the asset
//! note's **body** (git-tracked). That is deliberate: the heavy binary is a
//! git-ignored blob synced out-of-band, but the lightweight searchable text
//! travels with the notes — so a fresh clone can search a paper's contents
//! before the blob has even synced, and the disposable index rebuilds from the
//! `.md` files alone with no re-extraction.

use crate::blob::{BlobStore, Stored};
use crate::StoreError;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Ingested {
    pub hash: String,
    pub deduped: bool,
    pub mime: String,
    pub filename: String,
    /// Extracted searchable text (PDF via pdftotext, or the file itself when it
    /// is text). `None` for binary media with no text to extract.
    pub text: Option<String>,
}

/// Store `src` in the vault's blob store and extract what is searchable.
pub fn ingest_file(vault: &Path, src: &Path) -> Result<Ingested, StoreError> {
    let store = BlobStore::new(vault);
    let Stored { hash, deduped } = store.put_file(src)?;
    let blob = store.path_for(&hash);

    // Sniff on the original bytes; magic-byte detection, never the extension.
    let sniffed = infer::get_from_path(src).map_err(io)?.map(|t| t.mime_type().to_string());
    let text = extract_text(&blob, sniffed.as_deref());
    let mime = sniffed.unwrap_or_else(|| {
        // No known signature: call it text if it parsed as UTF-8, else opaque.
        if text.is_some() { "text/plain".into() } else { "application/octet-stream".into() }
    });
    let filename =
        src.file_name().and_then(|n| n.to_str()).unwrap_or("asset").to_string();
    Ok(Ingested { hash, deduped, mime, filename, text })
}

fn extract_text(blob: &Path, mime: Option<&str>) -> Option<String> {
    let text = match mime {
        Some("application/pdf") => pdftotext(blob)?,
        Some(m) if m.starts_with("text/") => std::fs::read_to_string(blob).ok()?,
        // No known magic signature → maybe plain text; read iff valid UTF-8.
        None => std::fs::read_to_string(blob).ok()?,
        // A known binary (image, audio, video, …) has no extractable text.
        _ => return None,
    };
    let text = text.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// poppler's `pdftotext`; `-` writes plain text to stdout. Invoked as a
/// subprocess, never linked — poppler is GPL and must stay out of our binary.
fn pdftotext(blob: &Path) -> Option<String> {
    let out = Command::new("pdftotext").arg("-q").arg(blob).arg("-").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Best-effort thumbnail via libvips (subprocess). Writes
/// `vault/derived/<hash>/thumb.webp`. Failure is not fatal — the gallery falls
/// back to the AssetMissing placeholder. Returns the thumbnail path on success.
pub fn thumbnail(vault: &Path, hash: &str) -> Result<PathBuf, StoreError> {
    let blob = BlobStore::new(vault).path_for(hash);
    let dir = vault.join("derived").join(hash);
    std::fs::create_dir_all(&dir).map_err(io)?;
    // vipsthumbnail resolves a *relative* `-o` against the INPUT file's
    // directory, which mangles the path when the vault is relative (the default).
    // Canonicalize to an absolute output path so it lands in derived/ regardless.
    let thumb = std::fs::canonicalize(&dir).map_err(io)?.join("thumb.webp");
    // `.output()` (not `.status()`) so vipsthumbnail's stderr never spams the CLI
    // — thumbnailing is best-effort and its failure only degrades a gallery tile.
    let out = Command::new("vipsthumbnail")
        .arg(&blob)
        .arg("--size")
        .arg("400x400")
        .arg("-o")
        .arg(format!("{}[Q=80]", thumb.display()))
        .output()
        .map_err(|e| StoreError::Io(format!("vipsthumbnail: {e}")))?;
    if !out.status.success() {
        return Err(StoreError::Io(format!(
            "vipsthumbnail failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(thumb)
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
