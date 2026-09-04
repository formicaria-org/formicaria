//! Encodings a frontend shell needs on the way to [`dispatch`](crate::dispatch::dispatch).
//!
//! **Why these live here and not in the shell that uses them.** Both functions below were written
//! in `mobile/src-tauri/src/lib.rs`, which is **excluded from the workspace**
//! (`Cargo.toml`: it pulls Tauri and, through `fm-core/native-git`, libgit2 + OpenSSL). So
//! `cargo test --workspace` never compiled them, the crate has no `tests/` directory and no
//! `#[cfg(test)]` module, and `pixi run ci` substitutes structural greps for compilation.
//!
//! The practical consequence was that **`b64_decode` — which every photo taken on the phone passes
//! through — had never been executed by a test.** It is hand-rolled, it is the only thing standing
//! between a mangled IPC payload and a corrupt blob, and its own docstring promises a behaviour
//! ("fails loudly instead of ingesting a prefix of the photo") that nothing checked.
//!
//! Moving the pure logic into the workspace is the cheap half of that problem: no second test
//! runner, no Android toolchain in CI, the tests just run. What stays in the mobile crate is what
//! genuinely needs Tauri. This is the same instinct as the `fm-query` seam — the part that can be
//! pure, is, because that is the part that can be tested.

/// Standard base64 → bytes.
///
/// Hand-rolled rather than adding a crate for one function on one platform: the alphabet is fixed,
/// there is no padding subtlety worth a dependency, and this is the only place in the tree that
/// decodes any.
///
/// **Returns `None` on any character outside the alphabet**, so a truncated or mangled payload
/// fails loudly instead of ingesting a prefix of the photo. That distinction is the whole reason
/// this is not a `filter`: a decoder that skips what it does not understand turns a corrupted
/// transfer into a *successfully stored, silently wrong* blob — and because the blob store is
/// content-addressed, the wrong bytes get a valid-looking reference and a note that points at
/// them. `\n` and `\r` are skipped because MIME base64 wraps, and `=` because it is padding.
///
/// Note what this deliberately does **not** reject: a bit-length that is not a multiple of 8. A
/// stray trailing character leaves `bits` non-zero and those leftover bits are dropped, which
/// matches every mainstream decoder.
pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    const fn val(c: u8) -> i8 {
        match c {
            b'A'..=b'Z' => (c - b'A') as i8,
            b'a'..=b'z' => (c - b'a' + 26) as i8,
            b'0'..=b'9' => (c - b'0' + 52) as i8,
            b'+' => 62,
            b'/' => 63,
            _ => -1,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u8;
    for &c in s.as_bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' {
            continue;
        }
        let v = val(c);
        if v < 0 {
            return None;
        }
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Minimal percent-decoding for the one place a reference crosses a URL.
///
/// Hand-rolled rather than adding a crate: a `sha256:` reference is hex plus one colon, so the
/// only escape that ever appears is `%3A`. Anything else passes through unchanged, which is the
/// conservative direction — a reference that fails to resolve renders a placeholder, where a
/// panic on a malformed escape would take down the note.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference encoder, so the round trip is graded against something other than itself.
    /// Deliberately a *different* implementation shape from the decoder (table lookup, explicit
    /// padding) — a round trip against your own inverse mostly proves you are self-consistent.
    fn b64_encode(bytes: &[u8]) -> String {
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
            let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
            out.push(A[(n >> 18) as usize & 63] as char);
            out.push(A[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
            out.push(if chunk.len() > 2 { A[n as usize & 63] as char } else { '=' });
        }
        out
    }

    /// Every padding residue. `len % 3` is the only thing that changes the tail encoding, so
    /// 0/1/2 bytes past a boundary is the whole space — and off-by-one in the tail is the classic
    /// hand-rolled-base64 bug, which would silently truncate every photo by a byte or two.
    #[test]
    fn round_trips_every_padding_residue() {
        for len in 0..=12usize {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let encoded = b64_encode(&bytes);
            assert_eq!(
                b64_decode(&encoded).as_deref(),
                Some(bytes.as_slice()),
                "length {len} did not survive the round trip (encoded as {encoded:?})",
            );
        }
    }

    /// Known-answer, so a symmetric bug in both of the above cannot hide.
    /// `printf 'Man' | base64` → `TWFu`; `printf 'M' | base64` → `TQ==`.
    #[test]
    fn matches_known_answers() {
        assert_eq!(b64_decode("TWFu").unwrap(), b"Man");
        assert_eq!(b64_decode("TWE=").unwrap(), b"Ma");
        assert_eq!(b64_decode("TQ==").unwrap(), b"M");
        assert_eq!(b64_decode("").unwrap(), b"");
    }

    /// The whole photo, at a size a phone actually produces.
    ///
    /// Nothing in the tree had ever decoded a payload above 67 bytes (the 1×1 PNG in
    /// `acquire.rs`), so the only sizes exercised were ones where an accumulator or capacity bug
    /// cannot show. Non-repeating bytes on purpose: a buffer of identical values round-trips even
    /// when the decoder loses track of position.
    #[test]
    fn round_trips_a_phone_sized_photo() {
        let bytes: Vec<u8> = (0..4_000_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
        let decoded = b64_decode(&b64_encode(&bytes)).expect("a well-formed payload must decode");
        assert_eq!(decoded.len(), bytes.len(), "the decoded photo changed length");
        assert!(decoded == bytes, "the decoded photo differs from what was encoded");
    }

    /// MIME base64 wraps at 76 columns, and a decoder that chokes on the newline would reject
    /// every wrapped payload.
    #[test]
    fn tolerates_line_wrapping() {
        let bytes: Vec<u8> = (0..200u16).map(|i| i as u8).collect();
        let wrapped: String = b64_encode(&bytes)
            .as_bytes()
            .chunks(76)
            .map(|c| format!("{}\r\n", std::str::from_utf8(c).unwrap()))
            .collect();
        assert_eq!(b64_decode(&wrapped).unwrap(), bytes);
    }

    /// **The promise in the docstring, made checkable.**
    ///
    /// A payload with a character outside the alphabet must yield `None`, not a prefix. If this
    /// ever regresses to skipping bad characters, ingest stores a truncated photo under a
    /// perfectly valid content hash and reports success — the exact silent-corruption shape the
    /// zero-byte-photo incident took, and one no other test in the tree can see.
    #[test]
    fn refuses_a_mangled_payload_rather_than_ingesting_a_prefix() {
        let bytes: Vec<u8> = (0..90u8).collect();
        let good = b64_encode(&bytes);

        for bad in ['!', '@', ' ', '\t', '-', '_', '.'] {
            let mut mangled = good.clone();
            mangled.insert(40, bad);
            assert_eq!(
                b64_decode(&mangled),
                None,
                "a payload containing {bad:?} decoded instead of failing",
            );
        }
    }

    /// Truncation is the other half: a transfer cut short is *still valid base64*, so it decodes —
    /// and it must decode to something visibly different, never quietly to the whole file. This
    /// pins the property the blob store relies on: different bytes, different hash, different
    /// reference.
    #[test]
    fn a_truncated_payload_decodes_short_not_whole() {
        let bytes: Vec<u8> = (0..300u16).map(|i| (i * 7) as u8).collect();
        let full = b64_encode(&bytes);
        let cut = b64_decode(&full[..full.len() / 2]).expect("valid base64, just less of it");
        assert!(cut.len() < bytes.len(), "a truncated payload decoded to the full length");
        assert_eq!(cut.as_slice(), &bytes[..cut.len()], "the surviving prefix should be intact");
    }

    #[test]
    fn percent_decodes_a_reference() {
        assert_eq!(percent_decode("sha256%3Adeadbeef"), "sha256:deadbeef");
        assert_eq!(percent_decode("sha256:deadbeef"), "sha256:deadbeef");
    }

    /// Malformed escapes pass through rather than panicking — `%` is not special enough to lose a
    /// note over, and this runs during render of any note that has an asset.
    #[test]
    fn percent_decode_survives_malformed_escapes() {
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%ZZ"), "%ZZ");
        assert_eq!(percent_decode("a%"), "a%");
        assert_eq!(percent_decode("%3"), "%3");
        // A valid escape immediately before the end still needs its two digits present.
        assert_eq!(percent_decode("x%3Ay"), "x:y");
    }
}

/// Types safe to render as a top-level document, if someone navigates straight to a blob.
///
/// An allowlist, and deliberately not "everything except a deny-list": the set of things a browser
/// will execute grows, and a new one must default to *download*, not to *render*. SVG is excluded
/// on purpose — it is an image everywhere it matters (an `<img>` ignores the disposition and
/// disables script inside it) and a scriptable document only here.
///
/// **Moved here from `fm-serve/src/blob.rs` on 2026-09-04**, with `parse_range`, because the phone
/// needs both and had neither. The reasoning `blob.rs` gives is not desktop-specific: a blob is
/// reachable as a same-origin URL, and **blobs arrive from collaborators through the merge driver**
/// — so "someone else's file, served from your origin, opened as a document" is a hazard on every
/// platform that serves one. Android had no `nosniff`, no CSP and no disposition at all.
pub fn inline_safe(ctype: &str) -> bool {
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
/// Three-valued on purpose, because HTTP distinguishes three outcomes and conflating them is how a
/// seek silently returns the wrong bytes:
/// - `None` — no range, or a syntax we don't implement (multi-range). Send the whole file; RFC 9110
///   says an unsatisfiable *syntax* must be ignored, not rejected.
/// - `Some(None)` — understood, but outside the file. That is a 416.
/// - `Some(Some((start, end)))` — an inclusive byte range, clamped to the file.
pub fn parse_range(header: &str, total: u64) -> Option<Option<(u64, u64)>> {
    let spec = header.trim().strip_prefix("bytes=")?.trim();
    // One range only. A multi-range request needs a multipart/byteranges body; no media element
    // sends one, and answering it wrongly is worse than ignoring it.
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

/// What a blob response should be, given the file's size and the request's `Range`.
///
/// **The whole decision, with no I/O and no HTTP type in sight** — so both transports can share it
/// and the gate can test it. `fm-serve` writes a raw HTTP response; the phone builds a
/// `tauri::http::Response`. Neither difference is a *policy* difference, and until 2026-09-04 the
/// phone had no policy at all: it read the entire blob into a `Vec`, sent
/// `application/octet-stream`, and honoured no `Range`.
#[derive(Debug, PartialEq, Eq)]
pub struct BlobReply {
    pub status: u16,
    /// The inclusive byte window to read, or `None` for a 416 (nothing to send).
    pub range: Option<(u64, u64)>,
    /// `Content-Range`'s value, when one is owed.
    pub content_range: Option<String>,
    /// False when the type is not on the inline allowlist and must be sent as an attachment.
    pub inline: bool,
}

/// Decide a blob response. `range` is the raw `Range` header, if the client sent one.
pub fn blob_reply(total: u64, ctype: &str, range: Option<&str>) -> BlobReply {
    let inline = inline_safe(ctype);
    match range.and_then(|h| parse_range(h, total)) {
        // Understood and unsatisfiable: 416, and RFC 9110 requires the total be disclosed.
        Some(None) => BlobReply {
            status: 416,
            range: None,
            content_range: Some(format!("bytes */{total}")),
            inline,
        },
        Some(Some((start, end))) => BlobReply {
            status: 206,
            range: Some((start, end)),
            content_range: Some(format!("bytes {start}-{end}/{total}")),
            inline,
        },
        // No range, or a syntax we do not implement — send the whole thing.
        None => BlobReply {
            status: 200,
            range: if total == 0 { None } else { Some((0, total - 1)) },
            content_range: None,
            inline,
        },
    }
}

#[cfg(test)]
mod blob_reply_tests {
    use super::{blob_reply, inline_safe, parse_range};

    /// The three-valued contract, which is the whole reason `parse_range` is not a `bool` or an
    /// `Option<(u64, u64)>`: HTTP distinguishes *no range*, *unsatisfiable range* and *a range*,
    /// and collapsing any two of them is how a seek silently returns the wrong bytes.
    #[test]
    fn a_range_header_has_three_possible_meanings() {
        // A range.
        assert_eq!(parse_range("bytes=0-99", 1000), Some(Some((0, 99))));
        // Open-ended means "to the end"; asking past the end is legal and clamps.
        assert_eq!(parse_range("bytes=500-", 1000), Some(Some((500, 999))));
        assert_eq!(parse_range("bytes=500-99999", 1000), Some(Some((500, 999))));
        // A suffix range — the last N bytes, which is what a media element sends to read a
        // trailing index.
        assert_eq!(parse_range("bytes=-100", 1000), Some(Some((900, 999))));
        // Understood and outside the file: a 416, not a 200.
        assert_eq!(parse_range("bytes=1000-", 1000), Some(None));
        assert_eq!(parse_range("bytes=-0", 1000), Some(None));
        assert_eq!(parse_range("bytes=0-", 0), Some(None));
        // Not understood: ignore it and send everything. RFC 9110 requires exactly this, and a
        // multi-range request is the case that matters — answering one wrongly is worse than
        // ignoring it, because no media element sends one.
        assert_eq!(parse_range("bytes=0-9,20-29", 1000), None);
        assert_eq!(parse_range("kilograms=0-9", 1000), None);
        assert_eq!(parse_range("bytes=abc", 1000), None);
    }

    /// The allowlist defaults to *download*, and SVG is the case it exists for: an image
    /// everywhere it matters, and a scriptable document only when navigated to directly.
    #[test]
    fn only_known_safe_types_render_as_a_document() {
        assert!(inline_safe("image/png"));
        assert!(inline_safe("video/mp4"));
        assert!(inline_safe("audio/ogg"));
        assert!(inline_safe("application/pdf"));
        assert!(inline_safe("image/jpeg; charset=binary"), "parameters must not defeat it");

        assert!(!inline_safe("image/svg+xml"), "scriptable when navigated to");
        assert!(!inline_safe("text/html"));
        assert!(!inline_safe("application/octet-stream"));
        assert!(!inline_safe(""));
    }

    /// **The decision both transports now share.** Until 2026-09-04 the phone had none of this: it
    /// read the whole blob into a `Vec`, sent `application/octet-stream`, and honoured no `Range`
    /// — while its own comment claimed `<video>` could seek "without the file ever being held
    /// whole in memory".
    #[test]
    fn a_blob_reply_answers_the_range_it_was_asked_for() {
        // No range: the whole file, no Content-Range.
        let r = blob_reply(1000, "video/mp4", None);
        assert_eq!(r.status, 200);
        assert_eq!(r.range, Some((0, 999)));
        assert_eq!(r.content_range, None);
        assert!(r.inline);

        // A range: 206, the window, and the total disclosed.
        let r = blob_reply(1000, "video/mp4", Some("bytes=100-199"));
        assert_eq!(r.status, 206);
        assert_eq!(r.range, Some((100, 199)));
        assert_eq!(r.content_range.as_deref(), Some("bytes 100-199/1000"));

        // Unsatisfiable: 416, nothing to read, and the total still disclosed — a client that
        // guessed the length needs to learn the real one from the refusal.
        let r = blob_reply(1000, "video/mp4", Some("bytes=5000-"));
        assert_eq!(r.status, 416);
        assert_eq!(r.range, None);
        assert_eq!(r.content_range.as_deref(), Some("bytes */1000"));

        // The type decides the disposition independently of the range.
        assert!(!blob_reply(10, "image/svg+xml", None).inline);
    }

    /// An empty blob is not an error and has nothing to send. Worth pinning because
    /// `total - 1` underflows on a `u64` and would panic in a release build's debug assertions —
    /// or, worse, wrap to `u64::MAX` and ask for a read of the whole address space.
    #[test]
    fn an_empty_blob_has_no_bytes_to_send_and_does_not_underflow() {
        let r = blob_reply(0, "application/octet-stream", None);
        assert_eq!(r.status, 200);
        assert_eq!(r.range, None, "there is no byte zero to send");
    }
}
