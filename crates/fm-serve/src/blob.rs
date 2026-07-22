//! `GET /api/blob/<reference>` — blob bytes, streamed, with `Range`.
//!
//! The one route that is not a command, and it exists because a command is the wrong shape
//! for it. `resolve_asset` answers with a `Vec<u8>`: to show a 300 MB video it reads the
//! whole file into the server's memory, ships all of it over one response, and the browser
//! wraps it in an object URL — so the bytes exist twice, in full, before the first frame
//! plays, and seeking re-does it. A `<video>`/`<iframe>` pointed at a URL instead asks for
//! the range it actually needs.
//!
//! `resolve_asset` stays: the in-memory mock backend (`pnpm dev`, Vitest) has no HTTP, and
//! thumbnails are small enough that a second path for them would earn nothing.
//!
//! **Why this needs no CSRF guard, and what it needs instead.** The POST guard exists
//! because a page the user merely visited can POST a side effect. This is a GET of
//! content-addressed bytes, and the capability *is* the address: you cannot ask for a blob
//! without already knowing its sha256, which is not a thing another origin can guess or
//! enumerate. A cross-origin page that somehow knew one could load it into an `<img>`, but
//! not read the bytes back — we send no `Access-Control-Allow-Origin`, so the browser's own
//! same-origin policy stops it. **Do not add CORS headers here**; that is the line this
//! reasoning rests on. Extending the Origin allowlist to GET would not help either:
//! browsers send no `Origin` on `<img>`/`<video>` subresource loads, so the guard would
//! have to permit absent-Origin — which is the very case it would be trying to refuse.
//!
//! The hazard this route *does* introduce is the opposite direction, and it is handled in
//! [`inline_safe`]: a blob is now something the user's own browser can navigate to, and
//! blobs arrive from collaborators.

use crate::{write_response, AppState};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::TcpStream;

/// How much we move between disk and socket at a time. The point of the whole route is
/// never to hold the file in memory, so this is the only buffer.
const CHUNK: usize = 64 * 1024;

/// Serve one blob, honouring `Range`. `head` sends the headers and no body — which is how a
/// media element discovers the length before it starts seeking.
pub fn serve(
    stream: &mut TcpStream,
    state: &AppState,
    reference: &str,
    range: Option<&str>,
    head: bool,
) -> io::Result<()> {
    let reference = crate::percent_decode(reference);
    let path = match fm_app::dispatch::blob_path(&state.app, &reference) {
        Ok(p) => p,
        // Absent is a 404, not a 500: "media absence is a warning, never an error" — the
        // read view degrades to its placeholder.
        Err(msg) => return write_response(stream, "404 Not Found", "text/plain", msg.as_bytes()),
    };

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("{}: {e}", path.display());
            return write_response(stream, "404 Not Found", "text/plain", msg.as_bytes());
        }
    };
    let total = file.metadata()?.len();

    // The MIME the read view picks its element from. `resolve_asset` threw this away and
    // answered `application/octet-stream` for everything, leaving the browser to
    // content-sniff — which is why the caller had to ask `asset_status` for the MIME
    // separately and build a typed `Blob` by hand.
    let ctype = fm_core::ingest::sniff_mime(&path)
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let (start, end) = match range.and_then(|r| parse_range(r, total)) {
        Some(Some(r)) => r,
        // A `Range` header we understood, asking for bytes that aren't there. 416 must
        // report the real length so the client can ask again correctly.
        Some(None) => {
            // Carries the same guards as every other exit from this module. It is the one
            // branch that builds its own header block, which is exactly how it came to be the
            // only response in the server without them.
            let header = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\n\
                 Content-Security-Policy: default-src 'none'; sandbox\r\n\
                 X-Content-Type-Options: nosniff\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(header.as_bytes())?;
            return stream.flush();
        }
        // No `Range`, or one we could not parse. Serving the whole thing is the correct
        // answer to both: an unparseable Range is defined to be ignored.
        None => (0, total.saturating_sub(1)),
    };

    let partial = range.is_some() && (start, end) != (0, total.saturating_sub(1));
    let length = if total == 0 { 0 } else { end - start + 1 };

    let mut header = String::new();
    header.push_str(if partial { "HTTP/1.1 206 Partial Content\r\n" } else { "HTTP/1.1 200 OK\r\n" });
    header.push_str(&format!("Content-Type: {ctype}\r\n"));
    header.push_str(&format!("Content-Length: {length}\r\n"));
    // Without this a browser will not seek — it assumes the whole file must be downloaded
    // before it can play anything past the start.
    header.push_str("Accept-Ranges: bytes\r\n");
    // We sniffed the type ourselves; never let the browser second-guess it into something
    // executable. Pairs with the disposition below.
    header.push_str("X-Content-Type-Options: nosniff\r\n");
    // Belt to the disposition's braces: if a blob ever *is* rendered as a document, it may
    // load nothing and reach nowhere. Costs one header; means a future widening of
    // `inline_safe` cannot quietly re-open the navigation hazard described below.
    header.push_str("Content-Security-Policy: default-src 'none'; sandbox\r\n");
    if !inline_safe(&ctype) {
        // **The one real hazard this route introduces.** A blob is now at a same-origin URL
        // a browser can *navigate* to, where before it was bytes the page wrapped itself.
        // Note bodies — and therefore the blobs they reference — arrive from collaborators
        // through the `.md` merge driver, so an SVG or an HTML attachment rendered as a
        // top-level document would run its script in the app's own origin, with the whole
        // `/api` surface in reach. `attachment` makes a navigation download the file
        // instead of rendering it.
        //
        // Subresource loads are unaffected — `<img src>`/`<video src>`/`<iframe src>` all
        // ignore Content-Disposition — which is why SVG can be off this list and still
        // render inline as an image (and an SVG in an `<img>` cannot run script anyway).
        header.push_str("Content-Disposition: attachment\r\n");
    }
    if partial {
        header.push_str(&format!("Content-Range: bytes {start}-{end}/{total}\r\n"));
    }
    // Consistent with every other response this server sends. Content-addressed bytes are
    // immutable and could safely be cached forever, but writing vault content into the
    // browser's on-disk cache is a decision about the user's data, not a performance knob.
    header.push_str("Cache-Control: no-store\r\nConnection: close\r\n\r\n");
    stream.write_all(header.as_bytes())?;

    if head || length == 0 {
        return stream.flush();
    }

    file.seek(SeekFrom::Start(start))?;
    let mut remaining = length;
    let mut buf = vec![0u8; CHUNK];
    while remaining > 0 {
        let want = CHUNK.min(remaining as usize);
        let n = file.read(&mut buf[..want])?;
        if n == 0 {
            break; // truncated under us; the client sees a short body and retries
        }
        if let Err(e) = stream.write_all(&buf[..n]) {
            return hung_up(e);
        }
        remaining -= n as u64;
    }
    stream.flush().or_else(hung_up)
}

/// A client that stopped listening mid-stream is **normal here**, not an error worth
/// printing. Dragging a `<video>` scrubber abandons the in-flight range the instant the new
/// one is issued, so treating a broken pipe as a connection failure would print an alarming
/// line for every seek — training the reader to ignore the one that eventually matters.
fn hung_up(e: io::Error) -> io::Result<()> {
    match e.kind() {
        io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset => Ok(()),
        _ => Err(e),
    }
}

/// Types safe to render as a top-level document, if someone navigates straight to a blob.
///
/// An allowlist, and deliberately not "everything except a deny-list": the set of things a
/// browser will execute grows, and a new one must default to *download*, not to *render*.
/// SVG is excluded on purpose — it is an image everywhere it matters (an `<img>` ignores
/// the disposition and disables script inside it) and a scriptable document only here.
fn inline_safe(ctype: &str) -> bool {
    let base = ctype.split(';').next().unwrap_or("").trim();
    match base {
        "image/svg+xml" => false,
        "application/pdf" => true,
        _ => {
            base.starts_with("image/") || base.starts_with("video/") || base.starts_with("audio/")
        }
    }
}

/// Parse one `bytes=` range against a known length.
///
/// Three-valued on purpose, because HTTP distinguishes three outcomes and conflating them
/// is how a seek silently returns the wrong bytes:
/// - `None` — no range, or a syntax we don't implement (multi-range). Send the whole file;
///   RFC 9110 says an unsatisfiable *syntax* must be ignored, not rejected.
/// - `Some(None)` — understood, but outside the file. That is a 416.
/// - `Some(Some((start, end)))` — an inclusive byte range, clamped to the file.
fn parse_range(header: &str, total: u64) -> Option<Option<(u64, u64)>> {
    let spec = header.trim().strip_prefix("bytes=")?.trim();
    // One range only. A multi-range request needs a multipart/byteranges body; no media
    // element sends one, and answering it wrongly is worse than ignoring it.
    if spec.contains(',') {
        return None;
    }
    let (from, to) = spec.split_once('-')?;
    let (from, to) = (from.trim(), to.trim());

    if from.is_empty() {
        // `bytes=-500` — the last 500 bytes. Zero is unsatisfiable, not "the whole file".
        let n: u64 = to.parse().ok()?;
        if n == 0 || total == 0 {
            return Some(None);
        }
        return Some(Some((total.saturating_sub(n), total - 1)));
    }

    let start: u64 = from.parse().ok()?;
    if start >= total {
        return Some(None); // includes an empty file, where every range is unsatisfiable
    }
    let end = match to.is_empty() {
        true => total - 1,
        // Clamped: asking past the end is legal and means "to the end".
        false => to.parse::<u64>().ok()?.min(total - 1),
    };
    if end < start {
        return Some(None);
    }
    Some(Some((start, end)))
}

#[cfg(test)]
mod tests {
    use super::{inline_safe, parse_range};
    use crate::AppState;
    use std::io::{BufRead, BufReader, Read};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;
    use std::time::Instant;

    /// Drive the real route over a real socket. `fm-serve` had no HTTP-layer test at all
    /// before this one, and a `Range` implementation is precisely the code that is right
    /// in the common case and wrong at the edges.
    ///
    /// Returns `(status line, headers, body)`.
    fn get(bytes: &[u8], path_suffix: &str, range: Option<&str>) -> (String, String, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("v");
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let stored = fm_core::BlobStore::new(&vault).put_bytes(bytes).unwrap();

        let store = fm_core::MultiStore::open(&[("v".to_string(), vault.clone())]).unwrap();
        let cfg = fm_app::vaults::VaultConfig { name: "v".into(), path: vault, restic: None };
        let state = AppState::new(fm_app::App::new(store, vec![cfg], None, false), None, Vec::new(), 0);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let reference = format!("{}{path_suffix}", stored.hash);
        let range = range.map(String::from);

        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            super::serve(&mut sock, &state, &reference, range.as_deref(), false).unwrap();
        });

        // Nothing is written from this end: `serve` is handed an already-parsed request, so
        // sending one would only leave unread bytes in the socket — which makes the close
        // an RST rather than a FIN, and the client sees ConnectionReset instead of a body.
        let mut client = TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(&mut client);
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.trim().is_empty() {
                break;
            }
            headers.push_str(&line);
        }
        let mut body = Vec::new();
        reader.read_to_end(&mut body).unwrap();
        server.join().unwrap();
        (status.trim().to_string(), headers, body)
    }

    #[test]
    fn a_whole_blob_comes_back_with_its_sniffed_type_and_range_support() {
        // A real PNG signature, so `sniff_mime` has something to find.
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(std::iter::repeat(b'x').take(500));

        let (status, headers, body) = get(&png, "", None);

        assert_eq!(status, "HTTP/1.1 200 OK");
        assert_eq!(body, png, "the whole blob, byte for byte");
        assert!(headers.contains("Content-Type: image/png"), "{headers}");
        assert!(headers.contains("Accept-Ranges: bytes"), "a browser will not seek without it");
        assert!(headers.contains("X-Content-Type-Options: nosniff"), "{headers}");
        assert!(!headers.contains("Content-Disposition"), "a PNG renders inline");
    }

    /// The reason the route exists: a media element asks for a slice and gets exactly that
    /// slice, not the file.
    #[test]
    fn a_range_request_returns_only_those_bytes() {
        let data: Vec<u8> = (0..=255u8).cycle().take(4096).collect();

        let (status, headers, body) = get(&data, "", Some("bytes=100-199"));

        assert_eq!(status, "HTTP/1.1 206 Partial Content");
        assert_eq!(body, data[100..=199], "exactly the requested window");
        assert!(headers.contains("Content-Range: bytes 100-199/4096"), "{headers}");
        assert!(headers.contains("Content-Length: 100"), "{headers}");
    }

    #[test]
    fn a_range_past_the_end_is_refused_with_the_real_length() {
        let (status, headers, body) = get(b"hello", "", Some("bytes=99-"));

        assert_eq!(status, "HTTP/1.1 416 Range Not Satisfiable");
        assert!(headers.contains("Content-Range: bytes */5"), "{headers}");
        assert!(body.is_empty());
    }

    /// The security property, end to end: a collaborator's SVG must not be something the
    /// browser will render as a document in this origin.
    #[test]
    fn an_svg_blob_is_served_as_a_download_not_a_document() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#;

        let (status, headers, _) = get(svg, "", None);

        assert_eq!(status, "HTTP/1.1 200 OK");
        assert!(headers.contains("Content-Type: image/svg+xml"), "{headers}");
        assert!(headers.contains("Content-Disposition: attachment"), "{headers}");
    }

    /// Media absence is a warning, never a crash — the read view degrades to a placeholder.
    #[test]
    fn an_absent_blob_is_a_404_not_a_500() {
        let (status, _, _) = get(b"present", "0000deadbeef", None);
        assert_eq!(status, "HTTP/1.1 404 Not Found");
    }

    /// The whole point of the allowlist. An SVG is a scriptable document, and note bodies
    /// — so the blobs they reference — come from collaborators.
    #[test]
    fn svg_and_anything_document_shaped_is_never_rendered_inline() {
        assert!(!inline_safe("image/svg+xml"));
        assert!(!inline_safe("text/html"));
        assert!(!inline_safe("application/xhtml+xml"));
        assert!(!inline_safe("application/octet-stream"));
        // The unknown case: a format nobody has thought about yet must download.
        assert!(!inline_safe("application/x-something-new"));
    }

    #[test]
    fn ordinary_media_still_renders_inline() {
        for ok in ["image/png", "image/jpeg", "image/webp", "video/mp4", "audio/mpeg", "application/pdf"] {
            assert!(inline_safe(ok), "{ok} should render inline");
        }
    }

    /// A sniffed type may carry parameters; the allowlist must match on the type itself.
    #[test]
    fn a_content_type_parameter_does_not_defeat_the_allowlist() {
        assert!(inline_safe("image/png; charset=binary"));
        assert!(!inline_safe("image/svg+xml; charset=utf-8"));
    }

    #[test]
    fn an_open_ended_range_runs_to_the_last_byte() {
        assert_eq!(parse_range("bytes=100-", 1000), Some(Some((100, 999))));
    }

    #[test]
    fn a_closed_range_is_inclusive() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some(Some((0, 99))));
    }

    /// What a `<video>` sends when the user drags the scrubber to the end.
    #[test]
    fn a_suffix_range_counts_back_from_the_end() {
        assert_eq!(parse_range("bytes=-500", 1000), Some(Some((500, 999))));
        // Longer than the file: the whole file, not an error.
        assert_eq!(parse_range("bytes=-5000", 1000), Some(Some((0, 999))));
    }

    /// Asking past the end is legal and means "to the end" — returning an error here
    /// would break every client that rounds its request up to a block boundary.
    #[test]
    fn an_end_past_the_file_is_clamped_not_refused() {
        assert_eq!(parse_range("bytes=900-99999", 1000), Some(Some((900, 999))));
    }

    #[test]
    fn a_start_past_the_end_is_unsatisfiable() {
        assert_eq!(parse_range("bytes=1000-", 1000), Some(None));
        assert_eq!(parse_range("bytes=5-1", 1000), Some(None));
        assert_eq!(parse_range("bytes=-0", 1000), Some(None));
    }

    /// Every range over an empty file is unsatisfiable — and must not underflow.
    #[test]
    fn an_empty_file_satisfies_nothing() {
        assert_eq!(parse_range("bytes=0-", 0), Some(None));
        assert_eq!(parse_range("bytes=-10", 0), Some(None));
    }

    /// Ignored, not refused: the whole file is the correct answer to a range we don't
    /// implement or cannot read.
    #[test]
    fn an_unparseable_or_multi_range_is_ignored() {
        assert_eq!(parse_range("bytes=0-10,20-30", 1000), None);
        assert_eq!(parse_range("items=0-10", 1000), None);
        assert_eq!(parse_range("bytes=abc-def", 1000), None);
        assert_eq!(parse_range("", 1000), None);
    }
}
