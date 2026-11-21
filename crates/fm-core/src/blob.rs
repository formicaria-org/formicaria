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
use std::sync::atomic::{AtomicU64, Ordering};

// A per-process counter so concurrent uploads get distinct temp files (fm-serve
// is thread-per-connection). `put_file` uses the bare pid; `put_bytes` adds this.
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

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

    /// Every blob file under `blobs/sha256`, recursively — the inventory `verify`
    /// and the manifest walk.
    pub fn blob_paths(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        walk(&self.root.join("sha256"), &mut out);
        out
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

    /// Store a byte slice as a content-addressed blob — the browser-upload path
    /// (a `File`/`Blob` gives bytes, not a path). Same identity and dedup as
    /// [`put_file`](Self::put_file): hash first, and if that blob already exists,
    /// write nothing. We hold the whole slice, so hashing is one shot.
    pub fn put_bytes(&self, bytes: &[u8]) -> Result<Stored, StoreError> {
        fs::create_dir_all(&self.root).map_err(io_err)?;
        let hash = hex(&Sha256::digest(bytes));
        let dest = self.path_for(&hash);
        if dest.exists() {
            return Ok(Stored { hash, deduped: true }); // already have these exact bytes
        }
        let tmp = self.root.join(format!(
            ".incoming-{}-{}",
            std::process::id(),
            TMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        {
            let mut writer = File::create(&tmp).map_err(io_err)?;
            writer.write_all(bytes).map_err(io_err)?;
            writer.sync_all().map_err(io_err)?;
        }
        fs::create_dir_all(dest.parent().expect("blob path has a parent")).map_err(io_err)?;
        fs::rename(&tmp, &dest).map_err(io_err)?;
        Ok(Stored { hash, deduped: false })
    }
}

/// The sha256 hex of a byte slice. Used to fingerprint a note's source id so a re-copy
/// into a vault can recognise and replace its prior copy — one-way, so the fingerprint
/// leaks neither the source id nor which vault it came from.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Re-hash a file's bytes to sha256 hex — the scrub's core: a blob whose content
/// no longer hashes to its own filename has bit-rotted.
pub fn sha256_file(path: &Path) -> Result<String, StoreError> {
    let mut f = File::open(path).map_err(io_err)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(io_err)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
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

/// **How much this vault is willing to send with its notes** — the count and total bytes of the
/// attachments a commit would carry, by the same rule `git`'s staging walk applies.
///
/// **Here, not in `git`, because nothing about it is git.** It is a directory walk and a `stat`
/// per file against a limit from `vault.json`; `ci/checks.sh` refuses a `fm_core::git::` call from
/// `dispatch` precisely because that module shells out and a phone has no `git` binary — and this
/// answer has to be available on exactly that phone.
///
/// Exists because a device could not be asked. When the libgit2 backend learned to stage
/// attachments (2026-09-09) the selection rule came with it unchanged: every blob at or under
/// `git_assets_max`, **not** a diff against what the remote already holds. So a device that has
/// been accumulating photos and has never sent one stages its whole backlog the first time it can.
/// A user's phone had been taking photos since July; the first backup after that change died
/// with a broken pipe, and again on a retry, and nothing in the app could say whether five
/// megabytes were pending or five hundred. Its logs are unreadable, so the number had to come from
/// the app.
///
/// Empty and free in the default configuration: no `git_assets_max`, no walk.
///
/// **Eligible, not outstanding.** It does not subtract what the remote already has — that needs
/// the index, which is backend-specific — and the number that explains a stuck push is how much
/// the rule selects.
pub fn eligible_assets(vault: &std::path::Path) -> Result<(u32, u64), crate::StoreError> {
    let desc = crate::descriptor::Descriptor::read(vault)?;
    let Some(max) = desc.git_assets_max else {
        return Ok((0, 0));
    };
    let max = crate::descriptor::effective_git_assets_max(max);
    let mut count = 0u32;
    let mut bytes = 0u64;
    for p in BlobStore::new(vault).blob_paths() {
        if let Ok(m) = std::fs::metadata(&p) {
            if travels(&desc, &p, m.len(), max) {
                count += 1;
                bytes += m.len();
            }
        }
    }
    Ok((count, bytes))
}

/// One step of a staged backup: send every attachment at or under `cap`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssetBatch {
    /// The size limit this step stages up to, in bytes. Cumulative by construction — a step
    /// includes everything the steps before it did, which is exactly what makes each push carry
    /// only the difference.
    pub cap: u64,
    /// Attachments this step adds that the one before it did not.
    pub count: u32,
    /// Bytes this step adds.
    pub bytes: u64,
}

/// **Split a backlog of attachments into pushes that can survive a phone connection.**
///
/// The failure this exists for: a device that has never sent attachments stages *all* of them the
/// first time it can, because the rule is a filter over the blob store rather than a diff against
/// the remote. A user's phone had 65 attachments totalling 127.2 MB waiting, and the push died
/// with a broken pipe — twice, identically, which is what ruled out a transient drop.
///
/// **Batches are a rising size limit, not an arbitrary partition, and that is the whole trick.**
/// `blobs_within` selects everything at or under a limit, so raising the limit step by step yields
/// a strictly growing set: each commit re-stages what is already committed (a no-op) and adds only
/// what the raise newly admits, so each *push* carries only the difference. No index bookkeeping,
/// no per-file state, and it is exactly the manual workaround — raise the setting a step at a
/// time — done for the user instead of explained to them.
///
/// Smallest first, so the earliest steps are the cheapest and a connection that cannot survive the
/// whole backlog still makes progress. Anything over the vault's own `git_assets_max` is not in
/// here at all: it is not going to travel, and pretending otherwise would plan a step that cannot
/// finish.
///
/// **A batch may exceed `budget`**, in one case: a single attachment larger than it. Splitting
/// below one file is not possible, and refusing to plan it would leave that file permanently
/// unsendable. Files of identical size share a step for the same reason — a cap cannot separate
/// them.
pub fn asset_batches(
    vault: &std::path::Path,
    budget: u64,
) -> Result<Vec<AssetBatch>, crate::StoreError> {
    let desc = crate::descriptor::Descriptor::read(vault)?;
    let Some(max) = desc.git_assets_max else {
        return Ok(Vec::new());
    };
    let max = crate::descriptor::effective_git_assets_max(max);
    let mut sizes: Vec<u64> = BlobStore::new(vault)
        .blob_paths()
        .into_iter()
        .filter_map(|p| std::fs::metadata(&p).ok().map(|m| (m.len(), p)))
        .filter(|(n, p)| travels(&desc, p, *n, max))
        .map(|(n, _)| n)
        .collect();
    sizes.sort_unstable();

    let mut out: Vec<AssetBatch> = Vec::new();
    let (mut count, mut bytes) = (0u32, 0u64);
    for (i, &n) in sizes.iter().enumerate() {
        // Equal sizes cannot be told apart by a cap, so they close together.
        let last_of_its_size = sizes.get(i + 1) != Some(&n);
        count += 1;
        bytes += n;
        if last_of_its_size && (bytes >= budget || i + 1 == sizes.len()) {
            out.push(AssetBatch { cap: n, count, bytes });
            count = 0;
            bytes = 0;
        }
    }
    Ok(out)
}

/// What a stored file is, read off its first bytes: the one thing a blob's name (its hash) cannot
/// say. Used to keep sound and video out of git unless the vault lets them in
/// ([`crate::descriptor::Descriptor::git_sound`]), and to find the audio a note holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Sound,
    Video,
    Other,
}

/// [`MediaKind`] of a file's first bytes. MP3 (ID3, or an MPEG/ADTS frame), Ogg, FLAC, WAV, AIFF,
/// AMR, CAF and an M4A/M4B box are sound; any other MP4/QuickTime box and Matroska/WebM are video
/// (a WebM holding only sound counts as video: both are heavy, and both stay out by default).
pub fn media_kind_of(head: &[u8]) -> MediaKind {
    let at = |i: usize, s: &[u8]| head.len() >= i + s.len() && &head[i..i + s.len()] == s;
    if at(0, b"ID3") || at(0, b"OggS") || at(0, b"fLaC") || at(0, b"#!AMR") || at(0, b"caff") {
        return MediaKind::Sound;
    }
    if (at(0, b"RIFF") && at(8, b"WAVE")) || (at(0, b"FORM") && (at(8, b"AIFF") || at(8, b"AIFC")))
    {
        return MediaKind::Sound;
    }
    if head.len() >= 2 && head[0] == 0xFF && head[1] & 0xE0 == 0xE0 {
        return MediaKind::Sound; // an MPEG audio or ADTS (AAC) frame
    }
    if at(4, b"ftyp") {
        return if at(8, b"M4A ") || at(8, b"M4B ") || at(8, b"M4P ") || at(8, b"F4A ") {
            MediaKind::Sound
        } else {
            MediaKind::Video
        };
    }
    if at(0, &[0x1A, 0x45, 0xDF, 0xA3]) || (at(0, b"RIFF") && at(8, b"AVI ")) {
        return MediaKind::Video;
    }
    MediaKind::Other
}

/// [`media_kind_of`] a file on disk (its first 16 bytes); `Other` when it cannot be read.
pub fn media_kind(path: &Path) -> MediaKind {
    use std::io::Read;
    let mut head = [0u8; 16];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    media_kind_of(&head[..n])
}

/// **The one rule for which attachment travels with a vault's notes**: not empty, at or under the
/// size limit, and — for sound and video — only where the vault lets them. Every caller that picks
/// files for git (the commit, the count of what is waiting, the staged batches) asks this, so they
/// cannot come to disagree.
pub fn travels(desc: &crate::descriptor::Descriptor, path: &Path, len: u64, max: u64) -> bool {
    if len == 0 || len > max {
        return false;
    }
    match media_kind(path) {
        MediaKind::Sound => desc.git_sound,
        MediaKind::Video => desc.git_video,
        MediaKind::Other => true,
    }
}

/// HMAC-SHA-256 (RFC 2104) of `msg` under `key`, as hex. A keyed fingerprint: unlike
/// [`sha256_hex`], it cannot be reversed by trying every value of a small field without the key —
/// which is what a watch's 32-bit serial number would otherwise allow (`fm_app::watch`, the coach
/// plan's P1.2). Written here over `sha2` rather than adding a crate for twenty lines.
pub fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(k.map(|b| b ^ 0x36));
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(k.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    hex(&outer.finalize())
}

#[cfg(test)]
mod hmac_tests {
    use super::*;

    /// RFC 4231, test cases 1 and 2.
    #[test]
    fn hmac_matches_the_rfc_vectors() {
        assert_eq!(
            hmac_sha256_hex(&[0x0b; 20], b"Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        assert_eq!(
            hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    /// **Sound and video stay out of git unless the vault lets them in** (2026-11-21), whatever their
    /// size, and every picker of files for git agrees: the commit's selection, the count of what is
    /// waiting, and the staged batches.
    #[test]
    fn sound_and_video_travel_only_where_the_vault_says() {
        let v = tempfile::tempdir().unwrap();
        std::fs::write(v.path().join("vault.json"), r#"{"git_assets_max":"50MB"}"#).unwrap();
        let store = BlobStore::new(v.path());
        store.put_bytes(b"ID3\x04\x00\x00 a song").unwrap();
        store.put_bytes(b"\x1a\x45\xdf\xa3 a clip, as WebM").unwrap();
        store.put_bytes(b"\x89PNG\r\n\x1a\n a picture").unwrap();
        let picked = |v: &std::path::Path| crate::git::blobs_within_capped(v, None).unwrap().len();
        assert_eq!(picked(v.path()), 1, "the picture alone");
        assert_eq!(eligible_assets(v.path()).unwrap().0, 1);
        assert_eq!(
            asset_batches(v.path(), u64::MAX).unwrap().iter().map(|b| b.count).sum::<u32>(),
            1
        );

        crate::descriptor::Descriptor::set_git_media(v.path(), true, false).unwrap();
        assert_eq!(picked(v.path()), 2, "and the song, once sound may travel");
        assert_eq!(eligible_assets(v.path()).unwrap().0, 2);
        let d = crate::descriptor::Descriptor::read(v.path()).unwrap();
        assert!(d.git_sound && !d.git_video);
        assert_eq!(d.git_assets_max, Some(50_000_000), "the size limit is kept");

        crate::descriptor::Descriptor::set_git_media(v.path(), false, false).unwrap();
        let text = std::fs::read_to_string(v.path().join("vault.json")).unwrap();
        assert!(!text.contains("git_sound") && text.contains("git_assets_max"), "{text}");
    }

    #[test]
    fn a_file_is_known_by_its_first_bytes() {
        use super::MediaKind::*;
        assert_eq!(media_kind_of(b"ID3\x04"), Sound);
        assert_eq!(media_kind_of(&[0xFF, 0xFB, 0x90]), Sound); // an MPEG frame
        assert_eq!(media_kind_of(&[0xFF, 0xF1, 0x50]), Sound); // ADTS AAC
        assert_eq!(media_kind_of(b"OggS\x00"), Sound);
        assert_eq!(media_kind_of(b"fLaC"), Sound);
        assert_eq!(media_kind_of(b"RIFF\x00\x00\x00\x00WAVE"), Sound);
        assert_eq!(media_kind_of(b"\x00\x00\x00\x20ftypM4A "), Sound);
        assert_eq!(media_kind_of(b"\x00\x00\x00\x20ftypisom"), Video);
        assert_eq!(media_kind_of(&[0x1A, 0x45, 0xDF, 0xA3, 0x01]), Video);
        assert_eq!(media_kind_of(b"\x89PNG\r\n\x1a\n"), Other);
        assert_eq!(media_kind_of(b"%PDF-1.7"), Other);
        assert_eq!(media_kind_of(b""), Other);
    }
}
