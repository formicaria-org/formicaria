//! An email, read — and a conversation, as a note. Pure: no network, no store, no disk.
//!
//! # The shape of the work (`decisions.md` 2026-10-09)
//!
//! Mail reaches the app read-only, through Gmail's API with the `gmail.readonly` scope; the transport
//! in `fm-serve` fetches each message as Gmail's JSON (`format=full`) and hands it here. This module
//! does two things with it, both deterministic:
//!
//! 1. [`from_gmail`] reads one message into a [`Message`]: who, when, the subject, the plain text, and
//!    any **calendar invite** it carries. An invite is structured — `UID`, `SEQUENCE`, `DTSTART` — so
//!    the caller sends it down the same path as your own calendar ([`crate::calendar`]), with no model:
//!    a meeting arriving by email becomes a meeting note at once, and its update or cancellation moves
//!    or marks the same note, because both are keyed on the invite's `UID`.
//! 2. [`reconcile`] keeps **one note per conversation** (Gmail's thread), titled by its subject, and
//!    holding **the whole exchange**: one block per message — sender, To, Cc, date and time,
//!    attachment names, and the text with its quoted history trimmed — appended in the order they
//!    arrive, plus `participants` and a link that opens the thread in Gmail (`decisions.md`, *the email
//!    exchange lives in the conversation note*). The assistant reads that note like any other.
//!
//! # What is never done
//!
//! The title is never rewritten, and **nothing already in the body is changed or moved**: a message is
//! only ever appended, found again by an invisible `<!-- gmail:<id> -->` marker so a re-read adds
//! nothing. Nothing is deleted, and no date is inferred from prose here — that is the model's job,
//! behind a human's acceptance.

use std::collections::{HashMap, HashSet};

use fm_model::{Kind, Object, PropertyValue, Stamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset};

/// The conversation a note stands for — Gmail's `threadId`.
pub const GMAIL_THREAD: &str = "gmail_thread";
/// Who wrote the latest message, as their display name where there is one.
pub const MAIL_FROM: &str = "mail_from";
/// When the latest message arrived, as a stamp in the person's wall clock.
pub const MAIL_LAST: &str = "mail_last";
/// The tag a conversation note carries.
pub const MAIL_TAG: &str = "mail";
/// Everyone who wrote or was addressed in the conversation, by display name.
pub const PARTICIPANTS: &str = "participants";
/// Opens the conversation in Gmail's web interface.
pub const MAIL_LINK: &str = "mail_link";

/// One email, as much of it as the app uses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    /// Display names (or bare addresses) of the `To:` and `Cc:` recipients.
    #[serde(default)]
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    /// File names of the attachments. Listed in the note; the files themselves are not copied.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// Gmail's `internalDate`: when the message arrived, UTC seconds. Reliable where the `Date:`
    /// header is whatever the sender's machine claimed.
    pub at: i64,
    /// The readable text: the `text/plain` part, or the HTML part with its markup removed.
    pub text: String,
    /// Every `text/calendar` part, as iCalendar text.
    pub calendars: Vec<String>,
}

/// Read one Gmail message (`users.messages.get`, `format=full`). `None` if it is not one.
///
/// A calendar part too large for Gmail to inline arrives with an `attachmentId` and no `data`; the
/// caller fetches it and writes it into `body.data` first — see [`calendar_attachments`].
pub fn from_gmail(v: &Value) -> Option<Message> {
    let payload = v.get("payload")?;
    let header = |name: &str| -> String {
        payload
            .get("headers")
            .and_then(Value::as_array)
            .and_then(|hs| {
                hs.iter().find(|h| {
                    h.get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
                })
            })
            .and_then(|h| h.get("value").and_then(Value::as_str))
            .map(decode_words)
            .unwrap_or_default()
    };
    let at = v
        .get("internalDate")
        .and_then(|d| d.as_str().map(str::to_string).or_else(|| d.as_i64().map(|n| n.to_string())))
        .and_then(|d| d.parse::<i64>().ok())
        .map(|ms| ms / 1000)
        .unwrap_or(0);

    let mut parts = Vec::new();
    walk(payload, &mut parts);
    let plain: Vec<&(String, String)> = parts.iter().filter(|(m, _)| m == "text/plain").collect();
    let text = if !plain.is_empty() {
        plain.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join("\n")
    } else {
        parts
            .iter()
            .filter(|(m, _)| m == "text/html")
            .map(|(_, h)| html_to_text(h))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let calendars = parts
        .iter()
        .filter(|(m, _)| m == "text/calendar" || m == "application/ics")
        .map(|(_, t)| t.clone())
        .collect();

    Some(Message {
        id: v.get("id")?.as_str()?.to_string(),
        thread_id: v.get("threadId")?.as_str()?.to_string(),
        subject: header("Subject"),
        from: display_name(&header("From")),
        to: addresses(&header("To")),
        cc: addresses(&header("Cc")),
        attachments: attachment_names(payload),
        at,
        text: trim_quoted(text.trim()),
        calendars,
    })
}

/// Every calendar part that Gmail did not inline: `(part path, attachmentId)`. The path is the list
/// of `parts` indexes from the payload down, so the caller can put the fetched data back in place.
pub fn calendar_attachments(v: &Value) -> Vec<(Vec<usize>, String)> {
    let mut out = Vec::new();
    fn go(p: &Value, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, String)>) {
        let mime = p.get("mimeType").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
        let body = p.get("body");
        let has_data = body.and_then(|b| b.get("data")).is_some();
        if (mime == "text/calendar" || mime == "application/ics") && !has_data {
            if let Some(id) = body.and_then(|b| b.get("attachmentId")).and_then(Value::as_str) {
                out.push((path.clone(), id.to_string()));
            }
        }
        if let Some(parts) = p.get("parts").and_then(Value::as_array) {
            for (i, c) in parts.iter().enumerate() {
                path.push(i);
                go(c, path, out);
                path.pop();
            }
        }
    }
    if let Some(p) = v.get("payload") {
        go(p, &mut Vec::new(), &mut out);
    }
    out
}

/// Put fetched attachment data (Gmail's base64url `data`) into the part at `path`.
pub fn fill_attachment(v: &mut Value, path: &[usize], data: &str) {
    let mut p = &mut v["payload"];
    for i in path {
        p = &mut p["parts"][*i];
    }
    p["body"]["data"] = Value::String(data.to_string());
}

/// Depth-first over the MIME tree, collecting `(mime type, decoded text)` for every leaf that has
/// data. An attachment that is not a calendar (a PDF, an image) is skipped — the text of a message is
/// what it says, not what it carries.
fn walk(p: &Value, out: &mut Vec<(String, String)>) {
    if let Some(parts) = p.get("parts").and_then(Value::as_array) {
        for c in parts {
            walk(c, out);
        }
        return;
    }
    let mime = p.get("mimeType").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
    let is_text = mime.starts_with("text/") || mime == "application/ics";
    let filename = p.get("filename").and_then(Value::as_str).unwrap_or("");
    let calendar = mime == "text/calendar" || mime == "application/ics";
    if !is_text || (!filename.is_empty() && !calendar) {
        return;
    }
    if let Some(data) = p.get("body").and_then(|b| b.get("data")).and_then(Value::as_str) {
        if let Some(bytes) = base64url(data) {
            out.push((mime, text_of(&bytes)));
        }
    }
}

/// Bytes → text: UTF-8 where it is, Latin-1 otherwise. Latin-1 maps every byte to a character, so a
/// Windows-1252 message from an older Italian mail client reads correctly bar a few punctuation marks,
/// instead of being refused.
fn text_of(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|b| *b as char).collect(),
    }
}

/// Gmail's base64url, with or without padding. `None` on anything that is not.
pub fn base64url(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            b'=' | b'\r' | b'\n' => continue,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// RFC 2047 encoded words — `=?UTF-8?B?…?=` and `=?ISO-8859-1?Q?…?=` — in a header value. Italian
/// subjects arrive this way whenever they carry an accent. Anything malformed is left as written.
fn decode_words(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut last_was_word = false;
    while let Some(i) = rest.find("=?") {
        let (before, tail) = rest.split_at(i);
        let decoded = (|| {
            let inner = &tail[2..];
            let (charset, inner) = inner.split_once('?')?;
            let (enc, inner) = inner.split_once('?')?;
            let end = inner.find("?=")?;
            let raw = &inner[..end];
            let bytes = match enc.to_ascii_uppercase().as_str() {
                "B" => base64url(raw)?,
                "Q" => q_decode(raw),
                _ => return None,
            };
            let text = if charset.eq_ignore_ascii_case("utf-8") {
                String::from_utf8_lossy(&bytes).into_owned()
            } else {
                text_of(&bytes)
            };
            Some((text, 2 + charset.len() + 1 + enc.len() + 1 + end + 2))
        })();
        match decoded {
            Some((text, used)) => {
                // Whitespace between two adjacent encoded words is not part of the text.
                if !(last_was_word && before.trim().is_empty()) {
                    out.push_str(before);
                }
                out.push_str(&text);
                rest = &tail[used..];
                last_was_word = true;
            }
            None => {
                out.push_str(before);
                out.push_str("=?");
                rest = &tail[2..];
                last_was_word = false;
            }
        }
    }
    out.push_str(rest);
    out
}

fn q_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = b.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
        match (b[i], hex.and_then(|h| u8::from_str_radix(h, 16).ok())) {
            (b'_', _) => out.push(b' '),
            (b'=', Some(v)) => {
                out.push(v);
                i += 2;
            }
            (c, _) => out.push(c),
        }
        i += 1;
    }
    out
}

/// A `To:`/`Cc:` list → display names. Split on commas outside quotes, since a quoted name may hold one
/// (`"Rossi, Maria" <m@x.it>`).
fn addresses(list: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in list.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            ',' if !quoted => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.iter().map(|a| display_name(a)).filter(|a| !a.is_empty()).collect()
}

/// Every attachment's file name, calendar invites excluded (they become meetings, not files).
fn attachment_names(p: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(parts) = p.get("parts").and_then(Value::as_array) {
        for c in parts {
            out.extend(attachment_names(c));
        }
    }
    let mime = p.get("mimeType").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
    if let Some(name) = p.get("filename").and_then(Value::as_str).filter(|n| !n.is_empty()) {
        if mime != "text/calendar" && mime != "application/ics" {
            out.push(decode_words(name));
        }
    }
    out
}

/// Cut a reply's quoted history: everything from the first line that introduces it. The earlier
/// messages are already above it in the note, and every reply repeating the whole thread would make a
/// note grow with the square of its length.
///
/// English and Italian forms of Gmail, Outlook and Apple Mail, plus a run of `>` lines. A message
/// that is *only* quoted text keeps it, rather than becoming empty.
pub fn trim_quoted(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let starts = |l: &str| {
        let t = l.trim();
        let lower = t.to_lowercase();
        t.starts_with('>')
            || (lower.starts_with("on ") && lower.ends_with("wrote:"))
            || (lower.starts_with("il ") && lower.ends_with("ha scritto:"))
            || (lower.starts_with("in data ") && lower.ends_with("ha scritto:"))
            || lower.starts_with("-----original message-----")
            || lower.starts_with("-----messaggio originale-----")
            || lower.starts_with("---------- forwarded message")
            || lower.starts_with("---------- messaggio inoltrato")
            // Outlook's header block: "From: …" / "Da: …" directly followed by a "Sent:"/"Inviato:".
            || ((lower.starts_with("from:") || lower.starts_with("da:")) && t.len() < 200)
    };
    let cut = lines.iter().enumerate().position(|(i, l)| {
        if !starts(l) {
            return false;
        }
        let lower = l.trim().to_lowercase();
        if lower.starts_with("from:") || lower.starts_with("da:") {
            // Only Outlook's block, never a sentence that happens to begin "Da: …".
            return lines.get(i + 1).is_some_and(|n| {
                let n = n.trim().to_lowercase();
                n.starts_with("sent:")
                    || n.starts_with("inviato:")
                    || n.starts_with("date:")
                    || n.starts_with("data:")
            });
        }
        true
    });
    match cut {
        Some(0) | None => text.trim().to_string(),
        Some(i) => lines[..i].join("\n").trim_end().to_string(),
    }
}

/// The block one message adds to its conversation note. The marker is invisible in the read view and
/// is how a re-read recognises a message it has already written.
pub fn render(m: &Message, offset: UtcOffset) -> String {
    let when = OffsetDateTime::from_unix_timestamp(m.at)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .to_offset(offset);
    let stamp = format!(
        "{}-{:02}-{:02} {:02}:{:02}",
        when.year(),
        u8::from(when.month()),
        when.day(),
        when.hour(),
        when.minute()
    );
    let from = if m.from.is_empty() { "(unknown sender)" } else { &m.from };
    let mut out = format!("{}\n### {from} — {stamp}\n", marker(&m.id));
    let mut who = Vec::new();
    if !m.to.is_empty() {
        who.push(format!("To: {}", m.to.join(", ")));
    }
    if !m.cc.is_empty() {
        who.push(format!("Cc: {}", m.cc.join(", ")));
    }
    if !who.is_empty() {
        out.push_str(&who.join(" · "));
        out.push('\n');
    }
    if !m.attachments.is_empty() {
        out.push_str(&format!("Attachments: {}\n", m.attachments.join(", ")));
    }
    out.push('\n');
    out.push_str(m.text.trim());
    out.push('\n');
    out
}

fn marker(id: &str) -> String {
    format!("<!-- gmail:{id} -->")
}

/// `"Maria Rossi" <maria@unipd.it>` → `Maria Rossi`; a bare address stays an address.
fn display_name(from: &str) -> String {
    let f = from.trim();
    match f.split_once('<') {
        Some((name, addr)) => {
            let name = name.trim().trim_matches('"').trim();
            if name.is_empty() {
                addr.trim_end_matches('>').trim().to_string()
            } else {
                name.to_string()
            }
        }
        None => f.to_string(),
    }
}

/// HTML → text with its line structure kept: a model reading an email needs to see where one line
/// ends, which [`crate::events::strip_tags`] alone (a feed-title helper) flattens away.
fn html_to_text(html: &str) -> String {
    let mut s = html.to_string();
    for (pat, rep) in [
        ("<br", "\n<br"),
        ("</p>", "</p>\n"),
        ("</div>", "</div>\n"),
        ("</tr>", "</tr>\n"),
        ("</li>", "</li>\n"),
    ] {
        s = s.replace(pat, rep).replace(&pat.to_ascii_uppercase(), rep);
    }
    // Drop `<style>`/`<script>` bodies, which are text to a tag-stripper and noise to a reader.
    for tag in ["style", "script"] {
        while let Some(a) = s.to_ascii_lowercase().find(&format!("<{tag}")) {
            let end = s.to_ascii_lowercase()[a..]
                .find(&format!("</{tag}>"))
                .map(|e| a + e + tag.len() + 3);
            match end {
                Some(e) => s.replace_range(a..e, ""),
                None => break,
            }
        }
    }
    s.lines()
        .map(|l| crate::events::decode_entities(&crate::events::strip_tags(l)))
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `Re: R: Fwd: Riunione` → `Riunione`. English and Italian reply/forward prefixes, any number of them.
pub fn base_subject(s: &str) -> String {
    let mut t = s.trim();
    loop {
        let lower = t.to_ascii_lowercase();
        // Chinese clients (回复 reply, 答复 answer, 转发 forward) are common in a university inbox, and
        // write the colon full-width as often as not.
        let cut = [
            "re:",
            "r:",
            "fwd:",
            "fw:",
            "i:",
            "inoltra:",
            "rif:",
            "回复:",
            "答复:",
            "转发:",
            "回复：",
            "答复：",
            "转发：",
        ]
        .iter()
        .find(|p| lower.starts_with(*p))
        .map(|p| p.len());
        match cut {
            Some(n) => t = t[n..].trim_start(),
            None => break,
        }
    }
    if t.is_empty() {
        "(no subject)".to_string()
    } else {
        t.to_string()
    }
}

/// What a mail read did to the conversation notes.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub messages: usize,
    #[serde(rename = "newConversations")]
    pub new_conversations: usize,
    #[serde(rename = "updatedConversations")]
    pub updated_conversations: usize,
}

/// Index notes by [`GMAIL_THREAD`].
pub fn index<'a>(notes: impl IntoIterator<Item = &'a Object>) -> HashMap<String, Object> {
    notes
        .into_iter()
        .filter_map(|o| match o.get(GMAIL_THREAD) {
            PropertyValue::Text(k) if !k.is_empty() => Some((k, o.clone())),
            _ => None,
        })
        .collect()
}

/// One note per conversation: create it on the first message, **append** each message the note does
/// not yet hold (in arrival order), widen `participants`, and move the "latest message" fields forward.
/// The title is never rewritten, and nothing already in the body — the person's own writing above,
/// between or below the messages — is changed or moved.
///
/// **Reading the same messages twice changes nothing** — the transport may re-send a batch after a
/// failure, and a note that drifted on every retry would be a note nobody trusts. Each message's
/// marker is what makes that true.
pub fn reconcile(
    messages: &[Message],
    existing: &HashMap<String, Object>,
    offset: UtcOffset,
) -> (Vec<Object>, Vec<Object>, Report) {
    let mut report = Report { messages: messages.len(), ..Default::default() };
    // thread -> (note, is new)
    let mut out: HashMap<String, (Object, bool)> = HashMap::new();
    let mut changed: HashSet<String> = HashSet::new();
    let mut sorted: Vec<&Message> = messages.iter().collect();
    sorted.sort_by_key(|m| m.at);
    for m in sorted {
        let (mut note, is_new) = match out.remove(&m.thread_id) {
            Some(pair) => pair,
            None => match existing.get(&m.thread_id) {
                Some(n) => (n.clone(), false),
                None => {
                    let mut n = Object::new(Kind::Note, "");
                    n.title = Some(base_subject(&m.subject));
                    n.tags = vec![MAIL_TAG.to_string()];
                    n.extra.insert(GMAIL_THREAD.into(), PropertyValue::Text(m.thread_id.clone()));
                    n.extra.insert(
                        MAIL_LINK.into(),
                        PropertyValue::Text(format!(
                            "https://mail.google.com/mail/u/0/#all/{}",
                            m.thread_id
                        )),
                    );
                    (n, true)
                }
            },
        };
        let when = OffsetDateTime::from_unix_timestamp(m.at)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
            .to_offset(offset);
        let stamp = Stamp::at(when.date(), when.time());
        // Compared through its text form: a stamp written to frontmatter is read back by the generic
        // YAML loader, which need not hand it back as a `Stamp`. Matching only the variant made every
        // re-read look newer — caught by the round-trip test through a real vault.
        let newer = match note.get(MAIL_LAST).display().parse::<Stamp>() {
            Ok(s) => stamp > s,
            Err(_) => true,
        };
        if !note.body.contains(&marker(&m.id)) {
            if !note.body.is_empty() && !note.body.ends_with("\n\n") {
                note.body.push_str(if note.body.ends_with('\n') { "\n" } else { "\n\n" });
            }
            note.body.push_str(&render(m, offset));
            changed.insert(m.thread_id.clone());
        }
        let mut people = participants(&note);
        let before = people.len();
        for name in std::iter::once(&m.from).chain(&m.to).chain(&m.cc) {
            if !name.is_empty() && !people.contains(name) {
                people.push(name.clone());
            }
        }
        if people.len() != before {
            note.extra.insert(
                PARTICIPANTS.into(),
                PropertyValue::List(people.into_iter().map(PropertyValue::Text).collect()),
            );
            changed.insert(m.thread_id.clone());
        }
        if newer {
            note.extra.insert(MAIL_LAST.into(), PropertyValue::Stamp(stamp));
            if !m.from.is_empty() {
                note.extra.insert(MAIL_FROM.into(), PropertyValue::Text(m.from.clone()));
            }
            changed.insert(m.thread_id.clone());
        }
        out.insert(m.thread_id.clone(), (note, is_new));
    }
    let (mut create, mut update) = (Vec::new(), Vec::new());
    for (thread, (note, is_new)) in out {
        if is_new {
            report.new_conversations += 1;
            create.push(note);
        } else if changed.contains(&thread) {
            report.updated_conversations += 1;
            update.push(note);
        }
    }
    (create, update, report)
}

/// The note's `participants`, however the YAML loader handed them back.
fn participants(note: &Object) -> Vec<String> {
    match note.get(PARTICIPANTS) {
        PropertyValue::List(items) => items.iter().map(|v| v.display()).collect(),
        PropertyValue::Text(t) if !t.is_empty() => vec![t],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use time::macros::offset;

    fn b64(s: &str) -> String {
        // Test-only encoder, so fixtures can be written as readable text.
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let b = s.as_bytes();
        let mut out = String::new();
        for chunk in b.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |a, (i, x)| a | (u32::from(*x) << (16 - 8 * i)));
            for i in 0..=chunk.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            }
        }
        out
    }

    fn message(id: &str, thread: &str, at_ms: i64, subject: &str, parts: Value) -> Value {
        json!({
            "id": id, "threadId": thread, "internalDate": at_ms.to_string(),
            "payload": {
                "mimeType": "multipart/mixed",
                "headers": [
                    { "name": "Subject", "value": subject },
                    { "name": "From", "value": "\"Maria Rossi\" <maria@unipd.it>" }
                ],
                "parts": parts
            }
        })
    }

    #[test]
    fn a_message_reads_its_plain_text_sender_and_subject() {
        let v = message(
            "m1",
            "t1",
            1_760_000_000_000,
            "Riunione di gruppo",
            json!([{ "mimeType": "text/plain", "body": { "data": b64("Ci vediamo giovedì alle 15.") } },
                   { "mimeType": "text/html", "body": { "data": b64("<p>ignored</p>") } }]),
        );
        let m = from_gmail(&v).unwrap();
        assert_eq!(m.subject, "Riunione di gruppo");
        assert_eq!(m.from, "Maria Rossi");
        assert_eq!(m.text, "Ci vediamo giovedì alle 15.");
        assert_eq!(m.at, 1_760_000_000);
    }

    #[test]
    fn html_only_mail_keeps_its_lines_and_drops_its_styles() {
        let v = message(
            "m1",
            "t1",
            0,
            "x",
            json!([{ "mimeType": "text/html", "body": { "data":
            b64("<style>p{color:red}</style><p>Prima riga</p><p>Seconda &amp; ultima</p>") } }]),
        );
        assert_eq!(from_gmail(&v).unwrap().text, "Prima riga\nSeconda & ultima");
    }

    #[test]
    fn an_invite_part_is_returned_as_calendar_text() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:u1\r\nSUMMARY:Call\r\nDTSTART:20261020T090000\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let v = message(
            "m1",
            "t1",
            0,
            "Invito",
            json!([
            { "mimeType": "text/plain", "body": { "data": b64("Invito") } },
            { "mimeType": "text/calendar", "body": { "data": b64(ics) } }]),
        );
        let m = from_gmail(&v).unwrap();
        assert_eq!(m.calendars, vec![ics.to_string()]);
    }

    #[test]
    fn a_calendar_attachment_gmail_did_not_inline_can_be_found_and_filled() {
        let mut v = message(
            "m1",
            "t1",
            0,
            "Invito",
            json!([
            { "mimeType": "multipart/alternative", "parts": [
                { "mimeType": "text/plain", "body": { "data": b64("x") } },
                { "mimeType": "text/calendar", "body": { "attachmentId": "A1", "size": 90000 } }]}]),
        );
        let found = calendar_attachments(&v);
        assert_eq!(found, vec![(vec![0, 1], "A1".to_string())]);
        fill_attachment(&mut v, &found[0].0, &b64("BEGIN:VCALENDAR"));
        assert_eq!(from_gmail(&v).unwrap().calendars, vec!["BEGIN:VCALENDAR".to_string()]);
    }

    #[test]
    fn a_pdf_attachment_is_not_read_as_the_message_text() {
        let v = message(
            "m1",
            "t1",
            0,
            "x",
            json!([
            { "mimeType": "text/plain", "body": { "data": b64("corpo") } },
            { "mimeType": "text/plain", "filename": "allegato.txt", "body": { "data": b64("allegato") } }]),
        );
        assert_eq!(from_gmail(&v).unwrap().text, "corpo");
    }

    #[test]
    fn encoded_italian_subjects_are_decoded() {
        assert_eq!(decode_words("=?UTF-8?B?Uml1bmlvbmUgZGkgbHVuZWTDrA==?="), "Riunione di lunedì");
        assert_eq!(decode_words("=?ISO-8859-1?Q?Perch=E9_no?="), "Perché no");
        assert_eq!(decode_words("=?UTF-8?Q?a?= =?UTF-8?Q?b?="), "ab");
        assert_eq!(decode_words("plain"), "plain");
    }

    #[test]
    fn reply_and_forward_prefixes_come_off_the_conversation_title() {
        assert_eq!(base_subject("R: Re: I: Riunione"), "Riunione");
        assert_eq!(base_subject("Fwd: Lab meeting"), "Lab meeting");
        assert_eq!(base_subject("Re:"), "(no subject)");
        assert_eq!(base_subject("回复: Fw: Asking for AB1234X"), "Asking for AB1234X");
        assert_eq!(base_subject("答复：CD5678Y Course"), "CD5678Y Course");
    }

    fn msg(id: &str, thread: &str, at: i64, from: &str) -> Message {
        Message {
            id: id.into(),
            thread_id: thread.into(),
            subject: "Re: Progetto".into(),
            from: from.into(),
            at,
            ..Default::default()
        }
    }

    #[test]
    fn one_note_holds_the_whole_exchange_in_order_with_who_and_when() {
        let mut a = msg("1", "t", 1_760_000_000, "Maria");
        a.text = "Ci vediamo giovedì.".into();
        a.to = vec!["Bal".into()];
        a.cc = vec!["Luca".into()];
        let mut b = msg("2", "t", 1_760_003_600, "Luca");
        b.text = "Porto le slide.".into();
        b.attachments = vec!["slide.pdf".into()];
        let (create, update, report) = reconcile(&[b, a], &HashMap::new(), offset!(+8));
        assert_eq!((create.len(), update.len()), (1, 0));
        let n = &create[0];
        assert_eq!(n.title.as_deref(), Some("Progetto"));
        assert_eq!(n.get(MAIL_FROM), PropertyValue::Text("Luca".into()), "the latest sender");
        let maria = n.body.find("### Maria").expect("Maria's message is in the note");
        let luca = n.body.find("### Luca").expect("Luca's message is in the note");
        assert!(maria < luca, "in the order they were sent:\n{}", n.body);
        assert!(n.body.contains("To: Bal · Cc: Luca"), "{}", n.body);
        assert!(n.body.contains("Attachments: slide.pdf"), "{}", n.body);
        assert!(n.body.contains("Ci vediamo giovedì."));
        // 1_760_000_000 is 2025-10-09 08:53 UTC, 16:53 at +08:00.
        assert!(n.body.contains("### Maria — 2025-10-09 16:53"), "{}", n.body);
        assert_eq!(
            participants(n),
            vec!["Maria".to_string(), "Bal".into(), "Luca".into()],
            "everyone, once each"
        );
        assert!(matches!(n.get(MAIL_LINK), PropertyValue::Text(l) if l.ends_with("#all/t")));
        assert_eq!(report.new_conversations, 1);
    }

    #[test]
    fn a_reply_is_appended_below_the_persons_own_writing_and_a_re_read_adds_nothing() {
        let (create, _, _) =
            reconcile(&[msg("1", "t", 1_760_000_000, "Maria")], &HashMap::new(), offset!(+8));
        let mut note = create[0].clone();
        note.title = Some("Progetto — mie note".into());
        note.body = format!("{}\nDa fare: rispondere entro lunedì.\n", note.body);
        let mine = note.body.clone();
        let existing = index([&note]);
        let (create, update, _) =
            reconcile(&[msg("2", "t", 1_760_090_000, "Luca")], &existing, offset!(+8));
        assert!(create.is_empty());
        let n = &update[0];
        assert_eq!(n.id, note.id);
        assert_eq!(n.title.as_deref(), Some("Progetto — mie note"));
        assert!(n.body.starts_with(&mine), "the person's text is untouched and first:\n{}", n.body);
        assert!(n.body.contains("### Luca"));

        let existing = index([n]);
        let (c, u, _) = reconcile(
            &[msg("1", "t", 1_760_000_000, "Maria"), msg("2", "t", 1_760_090_000, "Luca")],
            &existing,
            offset!(+8),
        );
        assert!(c.is_empty() && u.is_empty(), "a re-read must not touch the note");
    }

    #[test]
    fn quoted_history_is_trimmed_in_english_and_italian() {
        let en = "Sounds good.\n\nOn Tue, 7 Oct 2026 at 09:12, Maria <m@x.it> wrote:\n> earlier";
        assert_eq!(trim_quoted(en), "Sounds good.");
        let it =
            "Va bene.\nIl giorno mar 7 ott 2026 alle ore 09:12 Maria <m@x.it> ha scritto:\n> prima";
        assert_eq!(trim_quoted(it), "Va bene.");
        let outlook = "Ok.\n\nDa: Maria Rossi\nInviato: martedì 7 ottobre\nA: Bal";
        assert_eq!(trim_quoted(outlook), "Ok.");
        // A sentence that merely starts "Da:" is not Outlook's header block.
        assert_eq!(trim_quoted("Da: domani lavoriamo da casa."), "Da: domani lavoriamo da casa.");
        // All-quoted keeps its text rather than vanishing.
        assert_eq!(trim_quoted("> only quoted"), "> only quoted");
    }

    #[test]
    fn recipients_split_on_commas_outside_quotes() {
        assert_eq!(
            addresses("\"Rossi, Maria\" <m@x.it>, luca@y.it"),
            vec!["Rossi, Maria".to_string(), "luca@y.it".into()]
        );
    }
}
