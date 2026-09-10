//! Resumable, checksum-verified downloading — **generic over `{url, dest, sha256}`** and nothing
//! else.
//!
//! The bytes stream to a `<dest>.part` sidecar; a re-run resumes it with an HTTP `Range` request,
//! and `dest` only ever appears once the whole file is present and (if a checksum was given)
//! verified — so a caller never finds a half-written file. It pulls a pure-Rust blocking HTTPS
//! client (`ureq`, default `rustls` TLS with bundled roots) so it needs no system TLS and
//! cross-compiles to Android, plus `sha2` to verify.
//!
//! # Why this is its own crate
//!
//! It began inside `fm-agent-run`, written to fetch a GGUF on a phone with no `curl`. When the app
//! gained the ability to update *itself* (`decisions.md`, 2026-09-10) a second caller needed
//! exactly this and nothing else around it — and `fm-agent-run` re-exports `fm_agent`
//! unconditionally, so reaching the downloader through it would have linked the whole study-agent
//! pipeline into a notes-only build. That would have quietly retired the property `ci/checks.sh`
//! guards: that `fm-serve --no-default-features` is a *provably* agent-free core.
//!
//! So the generic half moved here and `fm-agent-run::fetch` kept the model-shaped wrappers
//! (`ensure_model`, `ensure_runtime`, `ensure_mmproj`) and the runtime-archive extractors, which
//! know about catalogues and keep-lists. Nothing about the engine changed in the move: the
//! `Accept-Encoding: identity` rule, the 416 handling, the early-end backstop and the
//! `Fatal`/`Transient` split are all load-bearing and are documented where they sit.
//!
//! **Deliberately dependency-light**, like the crate it came from: no `tempfile`, no async, no HTTP
//! framework. It cross-compiles to Android, and that is a constraint, not an accident.

use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// **Is retrying this worth anything?** — the one question the retry loop needs answered.
///
/// It used to retry everything twelve times with backoff, which is right for a dropped mobile
/// connection and wrong for a full disk: the same failure, forty seconds apart, twelve times, before
/// a message the user could have had at once. Cancelling is the same shape — a user who pressed stop
/// must not wait out a backoff.
#[derive(Debug)]
pub enum FetchError {
    /// Retrying cannot help: the disk is full, the path is unwritable, the user cancelled.
    Fatal(String),
    /// The network did what networks do. Resume and try again.
    Transient(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Fatal(m) | FetchError::Transient(m) => f.write_str(m),
        }
    }
}

/// Connect and **read** deadlines.
///
/// The read one is the load-bearing half and there was none. A half-open connection — the mobile
/// case, where the peer vanishes without a FIN — leaves `read` blocked forever, so the download hung
/// with no error, no progress, and no timeout: `MAX_STALLS` counts *failed attempts*, and an attempt
/// that never returns is never one. A stall the retry loop cannot see is a stall it cannot survive.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

fn http() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(CONNECT_TIMEOUT).timeout_read(READ_TIMEOUT).build()
}

/// One download: from `url` to `dest`, optionally verified against `sha256` (hex, case-insensitive).
pub struct Download<'a> {
    pub url: &'a str,
    pub dest: &'a Path,
    pub sha256: Option<&'a str>,
}

/// Extract a whole archive tree **faithfully** — every file, in its own place, with its mode.
///
/// **Deliberately not the runtime extractor.** `fm-agent-run`'s `unpack_tar_gz`/`unpack_zip`
/// flatten every entry to its file name and apply a keep-list (the wanted binary, `*.so`/`*.dylib`/
/// `*.dll`, `LICENSE*`), which is right for a llama.cpp release and catastrophic here: pointed at a
/// formicaria archive they would drop `manual/`, `README.txt` and every launcher, and collapse
/// `program/` into the destination root. Two extractors, two jobs, and the difference is stated in
/// both places so nobody reaches for the wrong one.
///
/// Everything must sit under a single top-level directory named `expect`, which is **stripped** —
/// so `formicaria-v0.6.0-linux-x86_64/program/fm-serve` lands at `<dest>/program/fm-serve`.
/// Requiring the name is a check as much as a convenience: an archive that unpacks somewhere other
/// than where the manifest says it should is not the artifact we asked for.
///
/// # What is refused, and why each one matters
///
/// - **Any path escaping `dest`** — a `..` component, an absolute path, or a Windows drive/UNC
///   prefix. Refused rather than normalised: normalising an attacker-supplied path is how you end
///   up permitting what you meant to deny.
/// - **Symlinks and hard links, outright.** A formicaria archive contains none, so nothing is lost
///   — and a symlink is precisely how a well-formed-looking entry reaches outside the destination
///   *after* the path check has passed, by being followed on the next write.
/// - **Anything that is not a regular file or a directory** — devices, fifos, sockets.
/// - **More than `MAX_BYTES` or `MAX_ENTRIES`.** An archive that expands without bound is a disk
///   filled on a machine whose owner asked for a 15 MB download.
///
/// **Mode is preserved through `entry.unpack()`, byte for byte.** On macOS an arm64 Mach-O carries
/// an ad-hoc signature that AMFI requires in order to `execve` it at all, so any transformation of
/// the bytes produces a binary that dies with `Killed: 9` and no message a user can see. Copying is
/// the only safe operation on an executable here.
pub fn unpack_tree(archive: &Path, dest: &Path, expect: &str) -> Result<(), String> {
    /// A release is ~15 MB packed and well under 100 MB unpacked; these bound the pathological
    /// case without being near the real one.
    const MAX_BYTES: u64 = 256 * 1024 * 1024;
    const MAX_ENTRIES: usize = 20_000;

    let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    fs::create_dir_all(dest).map_err(|e| format!("could not prepare {}: {e}", dest.display()))?;

    // Where an entry is allowed to land: under `dest`, with `expect/` stripped. `None` means refuse.
    let placed = |raw: &Path| -> Option<PathBuf> {
        let mut it = raw.components();
        match it.next() {
            Some(std::path::Component::Normal(first)) if first == expect => {}
            _ => return None,
        }
        let rest: PathBuf = it.as_path().to_path_buf();
        if rest.as_os_str().is_empty() {
            return None; // the top-level directory itself; nothing to write
        }
        if !rest.components().all(|c| matches!(c, std::path::Component::Normal(_))) {
            return None;
        }
        Some(dest.join(rest))
    };

    let mut seen = 0usize;
    let mut bytes = 0u64;
    let mut budget = |n: u64| -> Result<(), String> {
        seen += 1;
        bytes += n;
        if seen > MAX_ENTRIES || bytes > MAX_BYTES {
            return Err("the download expands to far more than a formicaria release".to_string());
        }
        Ok(())
    };

    if name.ends_with(".zip") {
        let f = File::open(archive).map_err(|e| format!("open {name}: {e}"))?;
        let mut zip = zip::ZipArchive::new(f).map_err(|e| format!("read {name}: {e}"))?;
        for i in 0..zip.len() {
            let mut e = zip.by_index(i).map_err(|e| format!("read {name}: {e}"))?;
            // `enclosed_name` is zip's own refusal of traversal; `placed` then applies ours.
            let raw = e.enclosed_name().ok_or("refusing an entry that escapes the folder")?;
            let Some(out) = placed(&raw) else {
                return Err(format!(
                    "refusing '{}', which is not where it says it is",
                    raw.display()
                ));
            };
            // A unix-mode symlink stored in a zip. `S_IFLNK` is 0o120000.
            if e.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
                return Err("refusing a link inside the download".to_string());
            }
            if e.is_dir() {
                fs::create_dir_all(&out).map_err(|err| format!("{}: {err}", out.display()))?;
                continue;
            }
            budget(e.size())?;
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|err| format!("{}: {err}", p.display()))?;
            }
            let mut w = File::create(&out).map_err(|err| format!("{}: {err}", out.display()))?;
            std::io::copy(&mut e, &mut w).map_err(|err| format!("{}: {err}", out.display()))?;
            #[cfg(unix)]
            if let Some(m) = e.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&out, fs::Permissions::from_mode(m));
            }
        }
        return Ok(());
    }

    let f = File::open(archive).map_err(|e| format!("open {name}: {e}"))?;
    let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(f));
    // Off: we set modes from the header ourselves and never want ownership from an archive.
    ar.set_preserve_permissions(true);
    ar.set_unpack_xattrs(false);
    for entry in ar.entries().map_err(|e| format!("read {name}: {e}"))? {
        let mut e = entry.map_err(|err| format!("read {name}: {err}"))?;
        let kind = e.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            return Err("refusing a link inside the download".to_string());
        }
        if !kind.is_file() && !kind.is_dir() {
            return Err("refusing an entry that is neither a file nor a folder".to_string());
        }
        let raw = e.path().map_err(|err| format!("read {name}: {err}"))?.into_owned();
        let Some(out) = placed(&raw) else {
            return Err(format!("refusing '{}', which is not where it says it is", raw.display()));
        };
        if kind.is_dir() {
            fs::create_dir_all(&out).map_err(|err| format!("{}: {err}", out.display()))?;
            continue;
        }
        budget(e.size())?;
        if let Some(p) = out.parent() {
            fs::create_dir_all(p).map_err(|err| format!("{}: {err}", p.display()))?;
        }
        // `unpack` copies bytes and applies the header's mode — the only safe operation on a
        // signed executable.
        e.unpack(&out).map_err(|err| format!("{}: {err}", out.display()))?;
    }
    Ok(())
}

/// GET a **small** document into memory — a manifest, a signature, a release listing.
///
/// Separate from [`fetch`] because it answers a different question. `fetch` is for artifacts: it
/// streams to a `.part`, resumes, and lands a file. This is for the handful of bytes you must read
/// *before* you know whether you want an artifact at all, and reading those through a file would
/// mean writing attacker-supplied bytes to disk to decide whether to trust them.
///
/// **Capped, because an unbounded read into memory is a denial of service the caller cannot see.**
/// A server that answers a manifest request with an endless body would otherwise grow this `Vec`
/// until the process died. `limit` is a hard ceiling on what is read, not on what is declared: a
/// lying `Content-Length` changes nothing.
pub fn get(url: &str, limit: usize) -> Result<Vec<u8>, FetchError> {
    // Its own agent, so the User-Agent is ours. A caller asking "is there a newer release?" is
    // making a request on a person's behalf, and `ureq/2.x` says nothing true about who is asking;
    // a fixed `formicaria` says exactly as much as is needed and nothing about the machine.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout_read(READ_TIMEOUT)
        .user_agent("formicaria")
        .build();
    let resp = agent
        .get(url)
        // Same reason as `fetch`: `ureq` transparently decodes gzip, so a declared length would
        // describe the encoded body while these are the decoded bytes.
        .set("Accept-Encoding", "identity")
        .call()
        .map_err(|e| FetchError::Transient(format!("could not reach {url}: {e}")))?;
    let mut buf = Vec::new();
    // `take(limit + 1)` so an over-long body is *detected* rather than silently truncated into
    // something that might still parse.
    resp.into_reader()
        .take(limit as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| FetchError::Transient(format!("could not read {url}: {e}")))?;
    if buf.len() > limit {
        return Err(FetchError::Fatal(format!("{url} answered more than {limit} bytes")));
    }
    Ok(buf)
}

/// Fetch `d`, resuming a partial `<dest>.part` if present. `on_progress(done, total)` fires as bytes
/// arrive (`total` is `None` when the server sends no length). **Idempotent:** an existing `dest`
/// returns `Ok` without touching the network — verifying it first if a checksum was given.
pub fn fetch(
    d: &Download,
    on_progress: &dyn Fn(u64, Option<u64>),
    cancel: &dyn Fn() -> bool,
) -> Result<(), FetchError> {
    use FetchError::{Fatal, Transient};

    // Already in place? Trust it if we can verify it; with no checksum to check against, a present
    // dest is taken as done (the caller chose not to pin one). A wrong-bytes dest is removed + refetched.
    if d.dest.exists() {
        match d.sha256 {
            Some(want) if verify(d.dest, want).map_err(Fatal)? => return Ok(()),
            Some(_) => {
                let _ = fs::remove_file(d.dest);
            }
            None => return Ok(()),
        }
    }
    if cancel() {
        return Err(Fatal("download cancelled".into()));
    }

    let part = part_path(d.dest);
    let have = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let mut req = http().get(d.url);
    // **Never compressed, ranged or not.** `ureq`'s default `Accept-Encoding: gzip` is transparently
    // decoded, so `Content-Length` describes the *encoded* body while the bytes written are the
    // decoded ones — the byte accounting below would be comparing two different numbers. Worse with
    // a `Range`, which the server applies to the encoded stream: resuming at the decoded offset
    // appends the wrong bytes and the file is silently wrong until the checksum catches it (and if
    // no checksum is pinned, never). A GGUF is already compressed; this costs nothing.
    req = req.set("Accept-Encoding", "identity");
    if have > 0 {
        req = req.set("Range", &format!("bytes={have}-"));
    }
    let resp = req.call().map_err(|e| Transient(format!("GET {}: {e}", d.url)))?;
    let status = resp.status();

    // 206 = the server honoured the range → append; 200 = it ignored it → start over; 416 = the range
    // is past the end, i.e. `.part` is already the whole file → straight to verify.
    let (mut file, mut done, read_body) = match status {
        206 => (open_append(&part)?, have, true),
        200 => (open_truncate(&part)?, 0, true),
        416 => (open_append(&part)?, have, false),
        s => return Err(Transient(format!("GET {}: unexpected status {s}", d.url))),
    };

    // How many bytes this file should end up with. **Not read from a 416's `Content-Length`** — that
    // describes the error page, not the file, so the old code reported a total of "what we already
    // have plus the length of an apology" and the completeness check below would have failed a file
    // that was already whole.
    let total = if read_body {
        resp.header("Content-Length")
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|len| done + len)
    } else {
        Some(have)
    };
    on_progress(done, total);

    if read_body {
        let mut reader = resp.into_reader();
        let mut buf = [0u8; 64 * 1024];
        loop {
            if cancel() {
                // The `.part` stays: a cancelled download is a paused one, and the next run resumes
                // from exactly here rather than re-fetching the gigabytes already on disk.
                return Err(Fatal("download cancelled".into()));
            }
            let n = reader.read(&mut buf).map_err(|e| Transient(format!("read {}: {e}", d.url)))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(|e| write_failed(&part, e))?;
            done += n as u64;
            on_progress(done, total);
        }
    }
    file.flush().map_err(|e| write_failed(&part, e))?;
    drop(file);

    // **A body that stopped early is not a download.** Without this, a connection cut at 80 % was
    // renamed over `dest` and `ensure_model` returned that truncated file forever — nothing in the
    // tree ever deleted a bad `.gguf`, so the only self-heal was uninstalling the app, which also
    // destroys the vault (`known-issues.md`). Transient on purpose: the `.part` is genuinely that
    // far along, and the next attempt resumes from it.
    //
    // `ureq` happens to catch the declared-length case first, in its own reader. This is the
    // backstop that does not depend on it: the invariant is ours, the check belongs at our level,
    // and a body delimited by connection close (no `Content-Length`) reaches this line and nothing
    // else. Same reasoning as the transport refusing to delegate a check it can make itself.
    if let Some(want) = total {
        if done < want {
            return Err(Transient(format!(
                "{} ended early: {done} of {want} bytes — resuming",
                d.url
            )));
        }
    }

    if let Some(want) = d.sha256 {
        if !verify(&part, want).map_err(Fatal)? {
            // Corrupt: drop the partial so the next attempt starts clean rather than resuming garbage.
            let _ = fs::remove_file(&part);
            return Err(Transient(format!(
                "checksum mismatch for {} — deleted the partial, retry",
                d.dest.display()
            )));
        }
    }
    fs::rename(&part, d.dest)
        .map_err(|e| Fatal(format!("rename {} -> {}: {e}", part.display(), d.dest.display())))?;
    Ok(())
}

/// Classify a write failure. **A full disk is the one that must not be retried**: twelve attempts
/// with backoff is up to ten minutes spent arriving at the same sentence, and the disk does not
/// empty itself in the meantime. `StorageFull` is `std`'s portable name for `ENOSPC` and Windows'
/// `ERROR_DISK_FULL`, so this needs no platform code and no new dependency.
fn write_failed(part: &Path, e: std::io::Error) -> FetchError {
    let msg = format!("write {}: {e}", part.display());
    match e.kind() {
        std::io::ErrorKind::StorageFull => {
            FetchError::Fatal(format!("{msg} — there is not enough free space for this download"))
        }
        std::io::ErrorKind::PermissionDenied => FetchError::Fatal(msg),
        _ => FetchError::Transient(msg),
    }
}

/// `<dest>.part` — the resume sidecar beside the final file.
pub fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

fn open_append(p: &Path) -> Result<File, FetchError> {
    ensure_parent(p)?;
    OpenOptions::new().create(true).append(true).open(p).map_err(|e| write_failed(p, e))
}

fn open_truncate(p: &Path) -> Result<File, FetchError> {
    ensure_parent(p)?;
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(p)
        .map_err(|e| write_failed(p, e))
}

pub fn ensure_parent(p: &Path) -> Result<(), FetchError> {
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| write_failed(dir, e))?;
    }
    Ok(())
}

/// Stream `path` through SHA-256 and compare to `want` (case-insensitive hex).
pub fn verify(path: &Path, want: &str) -> Result<bool, String> {
    let mut f = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(got.eq_ignore_ascii_case(want.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    /// The caller never cancels — the ordinary case, spelled once.
    fn never() -> bool {
        false
    }

    /// A throwaway directory. `std::env::temp_dir` rather than a `tempfile` dev-dependency, matching
    /// the tests already here: this crate is deliberately dependency-free so it cross-compiles to
    /// Android, and a test is a poor reason to be the first exception.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fm-fetch-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sha_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// A one-shot localhost HTTP/1.1 server that serves `body`, honouring a single `Range: bytes=N-`
    /// with a 206. Hermetic — no external network. Returns the bound `http://127.0.0.1:PORT/model`.
    fn serve_once(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model", listener.local_addr().unwrap());
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = Vec::new();
            let mut tmp = [0u8; 1024];
            loop {
                let n = stream.read(&mut tmp).unwrap();
                req.extend_from_slice(&tmp[..n]);
                if n == 0 || req.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&req);
            let start = text
                .lines()
                .find_map(|l| l.strip_prefix("Range: bytes="))
                .and_then(|r| r.split('-').next())
                .and_then(|n| n.parse::<usize>().ok());
            match start {
                Some(s) => {
                    let part = &body[s..];
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                        part.len(), s, body.len() - 1, body.len()
                    );
                    stream.write_all(head.as_bytes()).unwrap();
                    stream.write_all(part).unwrap();
                }
                None => {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream.write_all(head.as_bytes()).unwrap();
                    stream.write_all(&body).unwrap();
                }
            }
        });
        url
    }

    /// A release-shaped `.tar.gz`: everything nested under one top-level directory, the way the
    /// `stage` step in `release.yml` builds it.
    fn release_tar(dest: &Path, top: &str, files: &[(&str, &[u8])]) {
        let f = fs::File::create(dest).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        let mut ar = tar::Builder::new(enc);
        for (name, body) in files {
            let mut h = tar::Header::new_gnu();
            h.set_size(body.len() as u64);
            h.set_mode(if name.contains("fm-serve") { 0o755 } else { 0o644 });
            h.set_cksum();
            ar.append_data(&mut h, format!("{top}/{name}"), *body).unwrap();
        }
        ar.into_inner().unwrap().finish().unwrap();
    }

    /// A tar header written by hand, because `tar::Builder` refuses to serialise the paths that
    /// matter here — which is exactly why a hostile archive has to be built this way to test the
    /// refusal at all.
    fn raw_entry(name: &str, typeflag: u8, link: &str, body: &[u8]) -> Vec<u8> {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[100..107].copy_from_slice(b"0000644");
        h[108..115].copy_from_slice(b"0000000");
        h[116..123].copy_from_slice(b"0000000");
        let size = format!("{:011o}", if typeflag == b'0' { body.len() } else { 0 });
        h[124..135].copy_from_slice(size.as_bytes());
        h[136..147].copy_from_slice(b"00000000000");
        h[148..156].copy_from_slice(b"        ");
        h[156] = typeflag;
        h[157..157 + link.len()].copy_from_slice(link.as_bytes());
        let sum: u32 = h.iter().map(|&b| b as u32).sum();
        h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        let mut out = h.to_vec();
        if typeflag == b'0' {
            out.extend_from_slice(body);
            out.resize(out.len().div_ceil(512) * 512, 0);
        }
        out
    }

    fn gz(blocks: Vec<u8>) -> Vec<u8> {
        use std::io::Write;
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(&blocks).unwrap();
        e.write_all(&[0u8; 1024]).unwrap(); // two zero blocks end a tar
        e.finish().unwrap()
    }

    /// **The whole tree, in its own shape.** This is what separates this extractor from the
    /// runtime one beside it: nothing is flattened, nothing is dropped, and the executable bit
    /// survives — a `.command` without it is not launchable from Finder, and a user with no
    /// terminal cannot put it back.
    #[test]
    fn a_release_archive_lands_as_a_faithful_tree() {
        let dir = scratch("tree");
        let top = "formicaria-v0.6.0-linux-x86_64";
        let arc = dir.join("r.tar.gz");
        release_tar(
            &arc,
            top,
            &[
                ("program/fm-serve", b"binary"),
                ("program/models.toml", b"catalogue"),
                ("manual/index.html", b"<html>"),
                ("README.txt", b"read me"),
                ("Start formicaria.sh", b"#!/bin/sh"),
            ],
        );
        let out = dir.join("staged");
        unpack_tree(&arc, &out, top).expect("unpacks");

        assert_eq!(fs::read(out.join("program/fm-serve")).unwrap(), b"binary");
        assert_eq!(fs::read(out.join("manual/index.html")).unwrap(), b"<html>");
        assert_eq!(fs::read(out.join("README.txt")).unwrap(), b"read me");
        assert!(out.join("Start formicaria.sh").exists(), "launchers travel too");
        assert!(!out.join(top).exists(), "the top-level directory is stripped, not nested");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = fs::metadata(out.join("program/fm-serve")).unwrap().permissions().mode();
            assert_eq!(m & 0o111, 0o111, "the executable bit survives extraction");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// `..` is how a path that looks local reaches the vault. Refused, never normalised.
    #[test]
    fn an_entry_that_climbs_out_is_refused() {
        let dir = scratch("climb");
        let top = "formicaria-v0.6.0-linux-x86_64";
        let arc = dir.join("evil.tar.gz");
        fs::write(&arc, gz(raw_entry(&format!("{top}/../../pwned"), b'0', "", b"x"))).unwrap();
        let out = dir.join("staged");
        let err = unpack_tree(&arc, &out, top).unwrap_err();
        assert!(err.contains("refusing"), "got: {err}");
        assert!(!dir.join("pwned").exists() && !out.join("../../pwned").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    /// **A symlink passes a path check and then escapes on the next write.** The archive has none,
    /// so refusing outright costs nothing and closes the hole the path check cannot see.
    #[test]
    fn a_link_inside_the_download_is_refused() {
        let top = "formicaria-v0.6.0-linux-x86_64";
        for (flag, what) in [(b'2', "symlink"), (b'1', "hard link")] {
            let dir = scratch(&format!("link-{what}"));
            let arc = dir.join("evil.tar.gz");
            fs::write(
                &arc,
                gz(raw_entry(&format!("{top}/program/fm-serve"), flag, "/etc/passwd", b"")),
            )
            .unwrap();
            let err = unpack_tree(&arc, &dir.join("staged"), top).unwrap_err();
            assert!(err.contains("link"), "{what} should be refused, got: {err}");
            let _ = fs::remove_dir_all(&dir);
        }
    }

    /// An archive that unpacks somewhere other than where the manifest says is not the artifact we
    /// asked for — the name is a check, not just a convenience.
    #[test]
    fn an_archive_with_the_wrong_top_level_is_refused() {
        let dir = scratch("prefix");
        let arc = dir.join("r.tar.gz");
        release_tar(&arc, "something-else", &[("program/fm-serve", b"binary")]);
        let err =
            unpack_tree(&arc, &dir.join("staged"), "formicaria-v0.6.0-linux-x86_64").unwrap_err();
        assert!(err.contains("not where it says it is"), "got: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn downloads_whole_file_and_verifies_checksum() {
        let body: Vec<u8> = (0..50_000u32).map(|i| i as u8).collect();
        let want = sha_hex(&body);
        let url = serve_once(body.clone());
        let dir = std::env::temp_dir().join(format!("fm-fetch-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let dest = dir.join("m.gguf");
        let _ = fs::remove_file(&dest);

        let last = std::cell::Cell::new(0u64);
        fetch(
            &Download { url: &url, dest: &dest, sha256: Some(&want) },
            &|d, _| last.set(d),
            &never,
        )
        .unwrap();

        assert_eq!(fs::read(&dest).unwrap(), body, "downloaded bytes match");
        assert_eq!(last.get(), body.len() as u64, "progress reached the full size");
        assert!(!part_path(&dest).exists(), ".part is renamed away on success");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resumes_from_a_partial_and_lands_the_whole_file() {
        let body: Vec<u8> = (0..40_000u32).map(|i| (i * 7) as u8).collect();
        let want = sha_hex(&body);
        let url = serve_once(body.clone());
        let dir = std::env::temp_dir().join(format!("fm-fetch-resume-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let dest = dir.join("m.gguf");
        let _ = fs::remove_file(&dest);
        // Pre-seed a partial: the first 25 000 bytes already on disk. The server must Range-serve the rest.
        fs::write(part_path(&dest), &body[..25_000]).unwrap();

        fetch(&Download { url: &url, dest: &dest, sha256: Some(&want) }, &|_, _| {}, &never)
            .unwrap();

        assert_eq!(fs::read(&dest).unwrap(), body, "resumed file equals the whole body");
        let _ = fs::remove_dir_all(&dir);
    }

    fn serve_truncated(
        declared: usize,
        send: Vec<u8>,
    ) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = Vec::new();
            let mut tmp = [0u8; 1024];
            loop {
                let n = stream.read(&mut tmp).unwrap();
                req.extend_from_slice(&tmp[..n]);
                if n == 0 || req.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&req).into_owned());
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&send);
        });
        (url, rx)
    }

    /// **The bug the checksums were pinned to close.** A body that stops early used to be renamed
    /// over `dest` and returned forever — nothing in the tree ever deleted a bad `.gguf`, so the only
    /// self-heal was uninstalling the app, which also destroys the vault. Transient, so the retry
    /// resumes; never a finished file.
    #[test]
    fn a_body_that_ends_early_is_not_a_finished_download() {
        let (url, _rx) = serve_truncated(50_000, vec![7u8; 20_000]);
        let dir = scratch("early");
        let dest = dir.join("m.gguf");

        let err = fetch(&Download { url: &url, dest: &dest, sha256: None }, &|_, _| {}, &never)
            .unwrap_err();

        assert!(matches!(err, FetchError::Transient(_)), "an early close is worth retrying: {err}");
        assert!(!dest.exists(), "a truncated body must never be renamed over dest");
        assert_eq!(
            fs::metadata(part_path(&dest)).unwrap().len(),
            20_000,
            "the bytes that did arrive stay, so the retry resumes rather than restarts"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Compression and byte accounting cannot both be right: `Content-Length` would describe the
    /// encoded body while the bytes written are the decoded ones, and with a `Range` the server
    /// applies the offset to the encoded stream — so a resume appends the wrong bytes.
    #[test]
    fn the_request_never_asks_for_a_compressed_body() {
        let (url, rx) = serve_truncated(10, vec![0u8; 10]);
        let dir = scratch("encoding");
        let dest = dir.join("m.gguf");
        let _ = fetch(&Download { url: &url, dest: &dest, sha256: None }, &|_, _| {}, &never);

        let req = rx.recv().unwrap().to_ascii_lowercase();
        assert!(req.contains("accept-encoding: identity"), "asked for: {req}");
        assert!(!req.contains("gzip"), "gzip was still offered: {req}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Stop must mean stop *now*: a user who turns the assistant off during a 1.4 GB first-run
    /// download must not keep downloading, and must not wait out a retry backoff either. What is
    /// already on disk stays, so turning it back on resumes.
    #[test]
    fn cancelling_stops_mid_download_and_keeps_what_arrived() {
        let body: Vec<u8> = (0..400_000u32).map(|i| i as u8).collect();
        let url = serve_once(body);
        let dir = scratch("cancel");
        let dest = dir.join("m.gguf");

        let seen = std::cell::Cell::new(0u64);
        let err = fetch(
            &Download { url: &url, dest: &dest, sha256: None },
            &|done, _| seen.set(done),
            &|| seen.get() > 100_000,
        )
        .unwrap_err();

        assert!(matches!(err, FetchError::Fatal(_)), "cancelling must not be retried: {err}");
        assert!(!dest.exists(), "a cancelled download is not a finished one");
        let kept = fs::metadata(part_path(&dest)).unwrap().len();
        assert!(kept > 0 && kept < 400_000, "kept a resumable partial, got {kept} bytes");
        let _ = fs::remove_dir_all(&dir);
    }

    /// A full disk does not empty itself while we wait. Retried like a dropped connection it costs
    /// twelve attempts and ~90 s of backoff to arrive at the same sentence the first attempt knew.
    #[test]
    fn a_full_disk_is_fatal_and_a_dropped_write_is_not() {
        let full = std::io::Error::from(std::io::ErrorKind::StorageFull);
        assert!(matches!(write_failed(Path::new("m.part"), full), FetchError::Fatal(_)));

        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert!(matches!(write_failed(Path::new("m.part"), denied), FetchError::Fatal(_)));

        let flaky = std::io::Error::from(std::io::ErrorKind::Interrupted);
        assert!(matches!(write_failed(Path::new("m.part"), flaky), FetchError::Transient(_)));
    }

    #[test]
    fn rejects_a_bad_checksum() {
        let body: Vec<u8> = (0..10_000u32).map(|i| i as u8).collect();
        let url = serve_once(body);
        let dir = std::env::temp_dir().join(format!("fm-fetch-bad-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let dest = dir.join("m.gguf");
        let _ = fs::remove_file(&dest);

        let err = fetch(
            &Download { url: &url, dest: &dest, sha256: Some(&"00".repeat(32)) },
            &|_, _| {},
            &never,
        )
        .unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"), "got: {err}");
        assert!(!dest.exists(), "no dest is left on a bad checksum");
        assert!(!part_path(&dest).exists(), "the corrupt partial is deleted");
        let _ = fs::remove_dir_all(&dir);
    }
}
