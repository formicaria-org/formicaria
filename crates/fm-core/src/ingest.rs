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

/// Store `bytes` in the vault's blob store and extract what is searchable — the
/// browser-upload twin of [`ingest_file`]. MIME is sniffed from the bytes (magic
/// bytes, never the extension); text is extracted from the *stored* blob path so
/// `pdftotext` still runs on a real file.
pub fn ingest_bytes(vault: &Path, filename: &str, bytes: &[u8]) -> Result<Ingested, StoreError> {
    let store = BlobStore::new(vault);
    let Stored { hash, deduped } = store.put_bytes(bytes)?;
    let blob = store.path_for(&hash);

    let sniffed = infer::get(bytes).map(|t| t.mime_type().to_string());
    let text = extract_text(&blob, sniffed.as_deref());
    let mime = sniffed.unwrap_or_else(|| {
        if text.is_some() { "text/plain".into() } else { "application/octet-stream".into() }
    });
    Ok(Ingested { hash, deduped, mime, filename: filename.to_string(), text })
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

/// The derived-thumbnail path for a blob, `vault/derived/<hash>/thumb.webp`.
/// [`thumbnail`] writes it best-effort; the gallery and read view read it back to
/// show a preview (falling back to the AssetMissing placeholder when it is
/// absent). One place owns this layout so writer and reader can never disagree.
pub fn thumb_path(vault: &Path, hash: &str) -> PathBuf {
    vault.join("derived").join(hash).join("thumb.webp")
}

/// Sniff a stored blob's MIME by magic bytes (never the extension) — what the
/// read view uses to pick an inline element (image / PDF / video / audio).
/// `infer` reads only the first ~8 KiB, so this is O(1) on any blob size. `None`
/// for formats with no signature (plain text, SVG, CSV…), rendered as a link.
pub fn sniff_mime(path: &Path) -> Option<String> {
    infer::get_from_path(path).ok().flatten().map(|t| t.mime_type().to_string())
}

/// Best-effort thumbnail via libvips (subprocess). Writes
/// `vault/derived/<hash>/thumb.webp`. Failure is not fatal — the gallery falls
/// back to the AssetMissing placeholder. Returns the thumbnail path on success.
pub fn thumbnail(vault: &Path, hash: &str) -> Result<PathBuf, StoreError> {
    let blob = BlobStore::new(vault).path_for(hash);
    let dir = thumb_path(vault, hash).parent().expect("thumb path has a parent").to_path_buf();
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
