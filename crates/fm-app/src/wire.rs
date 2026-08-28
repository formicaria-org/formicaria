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
