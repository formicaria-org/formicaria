//! **DNS messages for the phone-wide block** (`privacy-plan-2026-11-19.md` 2.5, 4.4): the name asked
//! for, a refusal, the names an answer leads through (its CNAME chain), how long it may be kept, and
//! a kept answer given to a new question. Wire format only (RFC 1035), no network: the shell sends.

/// The name asked for (the first question), lower case; `None` for a message that is not a question.
pub fn qname(m: &[u8]) -> Option<String> {
    if m.len() < 12 || u16::from_be_bytes([m[4], m[5]]) == 0 {
        return None;
    }
    read_name(m, 12).map(|(n, _)| n.to_ascii_lowercase())
}

/// The question's type (1 = A, 28 = AAAA, 65 = HTTPS…).
pub fn qtype(m: &[u8]) -> Option<u16> {
    let (_, end) = read_name(m, 12)?;
    Some(u16::from_be_bytes([*m.get(end)?, *m.get(end + 1)?]))
}

/// "No such name": the question back as an answer with no records (NXDOMAIN), its id kept.
pub fn refuse(q: &[u8]) -> Vec<u8> {
    let mut r = q.to_vec();
    if r.len() < 12 {
        return r;
    }
    r[2] = 0x80 | (q[2] & 0x79); // QR; opcode and RD kept
    r[3] = 0x83; // RA, NXDOMAIN
    for b in &mut r[6..12] {
        *b = 0;
    }
    r
}

/// The names the answer section leads through: every CNAME's target (`a.example` → `b.cdn.example`).
pub fn cname_targets(m: &[u8]) -> Vec<String> {
    records(m)
        .into_iter()
        .filter(|r| r.kind == 5)
        .filter_map(|r| read_name(m, r.data).map(|(n, _)| n.to_ascii_lowercase()))
        .collect()
}

/// The shortest time to live in the answer section, in seconds; `None` when it has no records.
pub fn min_ttl(m: &[u8]) -> Option<u32> {
    records(m).iter().map(|r| r.ttl).min()
}

/// A kept answer for a new question: the new question's id.
pub fn with_id(answer: &[u8], question: &[u8]) -> Vec<u8> {
    let mut a = answer.to_vec();
    if a.len() >= 2 && question.len() >= 2 {
        a[0] = question[0];
        a[1] = question[1];
    }
    a
}

/// The answer says "no such name" or failed (anything but NOERROR).
pub fn failed(m: &[u8]) -> bool {
    m.len() < 12 || m[3] & 0x0f != 0
}

struct Record {
    kind: u16,
    ttl: u32,
    data: usize,
}

fn records(m: &[u8]) -> Vec<Record> {
    let mut out = Vec::new();
    if m.len() < 12 {
        return out;
    }
    let qd = u16::from_be_bytes([m[4], m[5]]);
    let an = u16::from_be_bytes([m[6], m[7]]);
    let mut at = 12;
    for _ in 0..qd {
        let Some((_, end)) = read_name(m, at) else { return out };
        at = end + 4;
    }
    for _ in 0..an {
        let Some((_, end)) = read_name(m, at) else { return out };
        if end + 10 > m.len() {
            return out;
        }
        let kind = u16::from_be_bytes([m[end], m[end + 1]]);
        let ttl = u32::from_be_bytes([m[end + 4], m[end + 5], m[end + 6], m[end + 7]]);
        let len = u16::from_be_bytes([m[end + 8], m[end + 9]]) as usize;
        let data = end + 10;
        if data + len > m.len() {
            return out;
        }
        out.push(Record { kind, ttl, data });
        at = data + len;
    }
    out
}

/// A name at `at` (following compression pointers), and where the bytes after it start.
fn read_name(m: &[u8], mut at: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
    let mut end = None;
    for _ in 0..64 {
        let len = *m.get(at)? as usize;
        if len == 0 {
            return Some((labels.join("."), end.unwrap_or(at + 1)));
        }
        if len & 0xc0 == 0xc0 {
            let ptr = ((len & 0x3f) << 8) | *m.get(at + 1)? as usize;
            end.get_or_insert(at + 2);
            if ptr >= at {
                return None;
            }
            at = ptr;
            continue;
        }
        if len > 63 {
            return None;
        }
        labels.push(String::from_utf8_lossy(m.get(at + 1..at + 1 + len)?).into_owned());
        at += 1 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(n: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for l in n.split('.') {
            out.push(l.len() as u8);
            out.extend_from_slice(l.as_bytes());
        }
        out.push(0);
        out
    }

    fn question(n: &str, id: u16) -> Vec<u8> {
        let mut q = id.to_be_bytes().to_vec();
        q.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
        q.extend(name(n));
        q.extend_from_slice(&[0, 1, 0, 1]);
        q
    }

    /// An invented answer: `www.shop.example` → CNAME `shop.tracker.example` → A 192.0.2.7.
    fn cloaked() -> Vec<u8> {
        let mut a = question("www.shop.example", 7);
        a[2] = 0x81;
        a[3] = 0x80;
        a[7] = 2; // two answers
        a.extend_from_slice(&[0xc0, 12, 0, 5, 0, 1, 0, 0, 0x0e, 0x10]); // CNAME, ttl 3600
        let target = name("shop.tracker.example");
        a.extend_from_slice(&(target.len() as u16).to_be_bytes());
        let target_at = a.len();
        a.extend(target);
        a.extend_from_slice(&[0xc0, target_at as u8, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 192, 0, 2, 7]);
        a
    }

    #[test]
    fn the_name_and_type_asked_for_are_read() {
        let q = question("Pixel.Tracker.Example", 1);
        assert_eq!(qname(&q).as_deref(), Some("pixel.tracker.example"));
        assert_eq!(qtype(&q), Some(1));
        assert_eq!(qname(&[0; 5]), None, "too short");
        assert_eq!(qname(&q[..14]), None, "cut short");
    }

    #[test]
    fn a_refusal_is_the_question_answered_with_no_such_name() {
        let q = question("pixel.tracker.example", 0xbeef);
        let r = refuse(&q);
        assert_eq!(&r[0..2], &[0xbe, 0xef], "its id");
        assert_eq!(r[2] & 0x80, 0x80, "an answer");
        assert_eq!(r[3] & 0x0f, 3, "NXDOMAIN");
        assert_eq!(qname(&r).as_deref(), Some("pixel.tracker.example"));
        assert!(failed(&r));
    }

    #[test]
    fn a_cloaked_tracker_is_seen_through_its_cname() {
        let a = cloaked();
        assert_eq!(cname_targets(&a), vec!["shop.tracker.example".to_string()]);
        assert_eq!(min_ttl(&a), Some(60));
        assert!(!failed(&a));
        let again = with_id(&a, &question("www.shop.example", 9));
        assert_eq!(&again[0..2], &[0, 9]);
    }

    #[test]
    fn a_pointer_loop_is_refused_not_followed() {
        let mut a = question("x.example", 1);
        a[7] = 1;
        let at = a.len();
        a.extend_from_slice(&[0xc0, at as u8, 0, 5, 0, 1, 0, 0, 0, 1, 0, 2, 0xc0, at as u8]);
        assert!(cname_targets(&a).is_empty());
    }
}
