//! Gmail, read-only — the transport half of `decisions.md` 2026-10-09.
//!
//! # Read-only by scope, not by promise
//!
//! The one permission ever asked for is [`SCOPE`], `gmail.readonly`. Google itself refuses a write
//! made with a token that only carries it, so "this app cannot send, delete or change your mail" is
//! not this module's promise, it is the token's limit. Three things keep it that way:
//!
//! - **One scope constant**, and a test that the consent address carries it and nothing else.
//! - **Only reads reach Gmail**: listing messages, getting one, getting one attachment, and the
//!   profile (to show which account is connected). The only `POST`s are to Google's sign-in token
//!   and revoke endpoints, never to Gmail. `ci/checks.sh` fails on any other Google address here.
//! - **The person's own Google Cloud client.** Shipping one shared client for a restricted scope needs
//!   Google's verification and a security assessment; the person creating their own keeps the whole
//!   arrangement between them and Google.
//!
//! # What leaves Gmail, and where it goes
//!
//! Only mail with the label the person chose, since the date they chose — never the whole mailbox.
//! **The label chooses conversations** (2026-10-15): a label sits on each message, and a reply sent or
//! received after the person labelled the exchange usually carries none, so every message since that
//! date in a conversation with a labelled message is read. Otherwise the exchange stops at the last
//! message the person happened to label, and its meeting silently goes stale.
//! Each conversation becomes one note holding the exchange — senders, recipients, times and text
//! (`fm_core::mail`; `decisions.md`, *the email exchange lives in the conversation note*) — which is
//! also what the assistant reads. A calendar invite inside a message becomes a meeting note at once,
//! through the same path as the calendars.
//!
//! # Autonomous
//!
//! A background thread reads every [`INTERVAL`] while the app is open. The person signs in once; a
//! Google Cloud project in "testing" mode asks for that again every 7 days, and the panel says so
//! plainly rather than going quiet.
//!
//! **A read that brings something new wakes the assistant** (2026-10-16): it raises [`NOVELTY`], which
//! the assistant asks for at every poll (`/api/mail_novelty`, seconds apart), and the meeting pass runs
//! at once rather than at its next minute. What the assistant concluded about each conversation comes
//! back through `/api/mail_heard` and is shown in Collaboration — a reply that only confirms the
//! meeting changes nothing, and "nothing to change" has to be visible, or it reads as nothing happened.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The only permission this app asks Google for.
pub const SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const GMAIL: &str = "https://gmail.googleapis.com/gmail/v1/users/me";

/// One minute (2026-10-16; 5 since 2026-10-15, 15 before). A quiet mailbox costs one Gmail request a
/// read — the sign-in is reused while it lasts ([`TOKEN`]), and each conversation is opened only when
/// its `historyId` moved — so a reply reaches the assistant within about a minute of arriving.
const INTERVAL: Duration = Duration::from_secs(60);
/// At most this many new messages per read, so a first read over a busy label cannot occupy the
/// machine for an hour. The rest come at the next read.
const PER_READ: usize = 200;
const LIMIT: usize = 25 * 1024 * 1024;

static READING: AtomicBool = AtomicBool::new(false);
/// Raised by every read that wrote something; the assistant runs its meeting pass when it moves.
/// In memory: after a restart it starts again at 0, which the assistant also reads as a change.
static NOVELTY: AtomicU64 = AtomicU64::new(0);
/// The access token and when it stops working (seconds since 1970). Google's lasts an hour; reusing it
/// keeps a quiet read to one request. In memory only, like the sign-in in progress.
static TOKEN: Mutex<Option<(String, u64)>> = Mutex::new(None);
/// How many of the assistant's verdicts are kept.
const HEARD: usize = 12;
static FILE: Mutex<()> = Mutex::new(());
/// The sign-in in progress: `(state, PKCE verifier, redirect address)`. In memory only — a sign-in
/// that outlives a restart is one nobody is waiting for.
static PENDING: Mutex<Option<(String, String, String)>> = Mutex::new(None);

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct Settings {
    client_id: String,
    /// Google calls it a secret, and for a desktop client says plainly that it is not one. Kept
    /// owner-only anyway, with the token.
    client_secret: String,
    /// **The secret.** Never serialised to a client.
    refresh_token: String,
    email: String,
    label: String,
    /// `YYYY-MM-DD`: nothing older is read.
    since: String,
    vault: String,
    /// Message ids already read, so each is read once.
    seen: Vec<String>,
    /// Conversation id -> Gmail's `historyId` for it when last read in full. A conversation whose
    /// marker has not moved has nothing new, and is not opened again.
    threads: std::collections::HashMap<String, String>,
    last: Option<Value>,
    last_checked: u64,
    /// Which note format the messages already read were written in. Below [`FORMAT`], `seen` is
    /// cleared once so every message is read again into the current format — safe, because a message
    /// already in its note is recognised by its marker and not written twice.
    format: u32,
    /// What the assistant concluded about each conversation it read, newest first ([`HEARD`] kept).
    heard: Vec<Heard>,
}

/// One verdict of the meeting pass: which conversation, up to which message, and what it made of it.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct Heard {
    note: String,
    title: String,
    latest: String,
    said: String,
    at: u64,
}

/// 2: the whole exchange in the note (2026-10-09). 0/1: subject and last sender only.
const FORMAT: u32 = 2;

fn dir() -> Option<PathBuf> {
    Some(fm_app::vaults::config_dir()?.join("formicaria"))
}

fn path() -> Option<PathBuf> {
    Some(dir()?.join("gmail.json"))
}

fn load() -> Settings {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn edit<T>(f: impl FnOnce(&mut Settings) -> Result<T, String>) -> Result<T, String> {
    let _g = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = load();
    let out = f(&mut s)?;
    let p = path().ok_or("this machine has nowhere to keep settings")?;
    let text = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
    fm_app::secrets::write_private(&p, &text)?;
    Ok(out)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// What a client may see: never the token, never the client secret.
fn public(s: &Settings) -> Value {
    json!({
        "configured": !s.client_id.is_empty(),
        "connected": !s.refresh_token.is_empty(),
        "email": s.email,
        "label": s.label,
        "since": s.since,
        "vault": s.vault,
        "read": s.seen.len(),
        "last": s.last,
        "lastChecked": s.last_checked,
        "reading": READING.load(Ordering::SeqCst),
        "heard": s.heard,
    })
}

// ── small encoders, written out rather than a dependency each ───────────────────────────────────

fn b64url(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = c.iter().enumerate().fold(0u32, |a, (i, x)| a | (u32::from(*x) << (16 - 8 * i)));
        for i in 0..=c.len() {
            out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

/// Percent-encoding for a query or form value.
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn form(pairs: &[(&str, &str)]) -> String {
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

fn random(n: usize) -> Result<String, String> {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).map_err(|e| format!("no system randomness: {e}"))?;
    Ok(b64url(&buf))
}

/// PKCE S256: `base64url(sha256(verifier))`.
fn challenge(verifier: &str) -> String {
    let hex = fm_core::blob::sha256_hex(verifier.as_bytes());
    let raw: Vec<u8> = (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect();
    b64url(&raw)
}

/// The consent address. Separate so a test can assert what it asks for.
fn consent_url(client_id: &str, redirect: &str, state: &str, verifier: &str) -> String {
    format!(
        "{AUTH_URL}?{}",
        form(&[
            ("client_id", client_id),
            ("redirect_uri", redirect),
            ("response_type", "code"),
            ("scope", SCOPE),
            // `offline` + `consent` is what makes Google hand back a refresh token, which is what lets
            // the reading happen without the person present.
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", state),
            ("code_challenge", &challenge(verifier)),
            ("code_challenge_method", "S256"),
        ])
    )
}

// ── talking to Google ─────────────────────────────────────────────────────────────────────────────

fn post_form(url: &str, body: &str) -> Result<(u16, Value), String> {
    let (status, bytes) = fm_fetch::request(
        "POST",
        url,
        &[("Content-Type", "application/x-www-form-urlencoded")],
        Some(body.as_bytes()),
        LIMIT,
    )
    .map_err(|e| e.to_string())?;
    Ok((status, serde_json::from_slice(&bytes).unwrap_or(Value::Null)))
}

fn get_json(url: &str, token: &str) -> Result<Value, String> {
    let auth = format!("Bearer {token}");
    let (status, bytes) = fm_fetch::request("GET", url, &[("Authorization", &auth)], None, LIMIT)
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if status == 200 {
        return Ok(v);
    }
    let why = v["error"]["message"].as_str().unwrap_or("no reason given");
    Err(format!("Gmail answered {status}: {why}"))
}

/// A fresh access token from the stored refresh token. `invalid_grant` means the person has to sign
/// in again — a "testing" project's tokens last 7 days — so the stored one is forgotten and the
/// panel says exactly that.
fn access_token(s: &Settings) -> Result<String, String> {
    let held = TOKEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some((t, _)) = held.filter(|(_, until)| now() + 60 < *until) {
        return Ok(t);
    }
    let (status, v) = post_form(
        TOKEN_URL,
        &form(&[
            ("client_id", &s.client_id),
            ("client_secret", &s.client_secret),
            ("refresh_token", &s.refresh_token),
            ("grant_type", "refresh_token"),
        ]),
    )?;
    if let Some(t) = v["access_token"].as_str().filter(|_| status == 200) {
        let lasts = v["expires_in"].as_u64().unwrap_or(0);
        *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = Some((t.to_string(), now() + lasts));
        return Ok(t.to_string());
    }
    if v["error"].as_str() == Some("invalid_grant") {
        let _ = edit(|s| {
            s.refresh_token.clear();
            Ok(())
        });
        return Err(
            "Google ended the sign-in (it does this every 7 days for a project in testing). \
                    Press Sign in with Google again."
                .into(),
        );
    }
    Err(format!(
        "Google refused the sign-in: {}",
        v["error_description"].as_str().unwrap_or("no reason given")
    ))
}

fn finish_sign_in(code: &str, state: &str) -> Result<(), String> {
    let (want, verifier, redirect) =
        PENDING.lock().unwrap_or_else(|e| e.into_inner()).take().ok_or(
            "this sign-in was not started from formicaria, or it was started before a restart",
        )?;
    // The `state` check is what stops another page from sending this browser here with *its*
    // account's code, which would connect a stranger's mailbox to these notes.
    if want != state {
        return Err("this sign-in does not match the one formicaria started".into());
    }
    let s = load();
    let (status, v) = post_form(
        TOKEN_URL,
        &form(&[
            ("client_id", &s.client_id),
            ("client_secret", &s.client_secret),
            ("code", code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect),
            ("grant_type", "authorization_code"),
        ]),
    )?;
    let (Some(refresh), Some(access)) = (v["refresh_token"].as_str(), v["access_token"].as_str())
    else {
        return Err(format!(
            "Google did not complete the sign-in ({status}): {}",
            v["error_description"].as_str().or(v["error"].as_str()).unwrap_or("no reason given")
        ));
    };
    if !v["scope"].as_str().unwrap_or(SCOPE).split(' ').all(|sc| sc == SCOPE) {
        // Never expected — we asked for one scope — but a token wider than read-only is refused
        // rather than kept, because keeping it would make "read-only" a promise again.
        return Err(
            "Google granted more than read-only access, so formicaria did not keep it".into()
        );
    }
    let email = get_json(&format!("{GMAIL}/profile"), access)
        .ok()
        .and_then(|p| p["emailAddress"].as_str().map(str::to_string))
        .unwrap_or_default();
    edit(|s| {
        s.refresh_token = refresh.to_string();
        s.email = email;
        s.last_checked = 0;
        Ok(())
    })
}

// ── reading ───────────────────────────────────────────────────────────────────────────────────────

/// The Gmail search the person's choices amount to: their label, since their date.
fn query(s: &Settings) -> String {
    let mut q = Vec::new();
    if !s.label.trim().is_empty() {
        q.push(format!("label:\"{}\"", s.label.trim().replace('"', "")));
    }
    if let Some(d) = s.since.get(..10).filter(|d| d.len() == 10) {
        q.push(format!("after:{}", d.replace('-', "/")));
    }
    q.join(" ")
}

fn read_once(state: &crate::AppState) -> Result<Value, String> {
    if load().format < FORMAT {
        edit(|s| {
            s.seen.clear();
            s.threads.clear();
            s.format = FORMAT;
            Ok(())
        })?;
    }
    let s = load();
    if s.refresh_token.is_empty() {
        return Err("not signed in".into());
    }
    let token = access_token(&s)?;
    // The conversations the label chooses, each with Gmail's change marker.
    let mut listed: Vec<(String, String)> = Vec::new();
    let mut page = String::new();
    loop {
        let mut url = format!("{GMAIL}/threads?maxResults=100&q={}", enc(&query(&s)));
        if !page.is_empty() {
            url.push_str(&format!("&pageToken={}", enc(&page)));
        }
        let v = get_json(&url, &token)?;
        for t in v["threads"].as_array().into_iter().flatten() {
            if let (Some(id), Some(h)) = (t["id"].as_str(), t["historyId"].as_str()) {
                listed.push((id.to_string(), h.to_string()));
            }
        }
        match v["nextPageToken"].as_str() {
            Some(p) => page = p.to_string(),
            None => break,
        }
    }
    // Open only the conversations that moved, and take every message in them not yet read.
    let since = since_ms(&s.since);
    let mut ids = Vec::new();
    let mut done: Vec<(String, String)> = Vec::new();
    for (t, h) in &listed {
        if s.threads.get(t) == Some(h) {
            continue;
        }
        if ids.len() >= PER_READ {
            break;
        }
        let v = get_json(&format!("{GMAIL}/threads/{}?format=minimal", enc(t)), &token)?;
        let new = unread_in_thread(&v, &s.seen, since);
        if ids.len() + new.len() <= PER_READ {
            done.push((t.clone(), h.clone()));
        }
        ids.extend(new);
    }
    ids.truncate(PER_READ);
    let waiting = ids.len();

    let mut batch = Vec::new();
    for id in &ids {
        let mut v = get_json(&format!("{GMAIL}/messages/{}?format=full", enc(id)), &token)?;
        for (path, att) in fm_core::mail::calendar_attachments(&v) {
            let a = get_json(
                &format!("{GMAIL}/messages/{}/attachments/{}", enc(id), enc(&att)),
                &token,
            )?;
            if let Some(data) = a["data"].as_str() {
                fm_core::mail::fill_attachment(&mut v, &path, data);
            }
        }
        batch.push(v);
    }
    let offset = crate::calendar::offset();
    let args = json!({ "vault": s.vault, "offset": offset });
    let body = serde_json::to_vec(&batch).map_err(|e| e.to_string())?;
    let out =
        fm_app::dispatch("mail_sync", &args, &body, &state.app, &crate::Desktop)?.into_bytes();
    let mut report: Value = serde_json::from_slice(&out).map_err(|e| e.to_string())?;
    // Marked read only after the notes are written, so a failure part-way re-reads rather than loses.
    edit(|cur| {
        cur.seen.extend(ids.iter().cloned());
        cur.threads.extend(done.iter().cloned());
        Ok(())
    })?;
    report["at"] = json!(now());
    report["waiting"] = json!(waiting);
    if !ids.is_empty() {
        NOVELTY.fetch_add(1, Ordering::SeqCst);
    }
    Ok(report)
}

/// The messages of one conversation (`threads.get`, `format=minimal`) still to read: not read before,
/// not older than `since`, and not a draft, spam or in the bin. Oldest first, as Gmail lists them.
fn unread_in_thread(thread: &Value, seen: &[String], since: u64) -> Vec<String> {
    thread["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| {
            let labels: Vec<&str> =
                m["labelIds"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            let at = m["internalDate"].as_str().and_then(|d| d.parse::<u64>().ok()).unwrap_or(0);
            at >= since && !labels.iter().any(|l| matches!(*l, "DRAFT" | "SPAM" | "TRASH"))
        })
        .filter_map(|m| m["id"].as_str())
        .filter(|id| !seen.iter().any(|x| x == id))
        .map(str::to_string)
        .collect()
}

/// `YYYY-MM-DD` → milliseconds since 1970 at that day's start (UTC); 0 when unset or unreadable, which
/// reads everything — the same as Gmail's search with no `after:`.
fn since_ms(day: &str) -> u64 {
    let mut it = day.get(..10).unwrap_or("").split('-').map(|p| p.parse::<i64>().ok());
    let (Some(Some(y)), Some(Some(m)), Some(Some(d))) = (it.next(), it.next(), it.next()) else {
        return 0;
    };
    // Days from civil (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400_000).unwrap_or(0)
}

fn read_all(state: &crate::AppState) {
    if READING.swap(true, Ordering::SeqCst) {
        return;
    }
    let last = match read_once(state) {
        Ok(v) => v,
        Err(e) => {
            // A refused token is asked for afresh next time, rather than refused for its hour.
            forget_token();
            json!({ "at": now(), "error": e })
        }
    };
    let _ = edit(|s| {
        s.last = Some(last);
        s.last_checked = now();
        Ok(())
    });
    READING.store(false, Ordering::SeqCst);
}

pub fn check_in_background(state: Arc<crate::AppState>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(12));
        // **Opening the app reads at once**, whenever the last read was: the person opened it to see
        // where things stand, and a read due "in fourteen minutes" made them press Read now (the
        // owner, 2026-10-15: "I do not like that every time you need to trigger this manually").
        let mut first = true;
        loop {
            let s = load();
            if !s.refresh_token.is_empty()
                && (first || now().saturating_sub(s.last_checked) >= INTERVAL.as_secs())
            {
                read_all(&state);
            }
            first = false;
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

// ── routes ────────────────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Req {
    client_id: String,
    client_secret: String,
    label: String,
    since: String,
    vault: String,
    /// `location.origin` of the page asking, so the address Google sends the browser back to is the
    /// one this app is actually being used at.
    origin: String,
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let q = path.split_once('?')?.1;
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' => {
                match b
                    .get(i + 1..i + 3)
                    .and_then(|h| std::str::from_utf8(h).ok())
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
                {
                    Some(v) => {
                        out.push(v);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn page(title: &str, text: &str) -> Vec<u8> {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'>\
         <title>formicaria</title><body style='font-family:system-ui;max-width:32rem;margin:3rem auto;padding:0 1rem'>\
         <h1 style='font-size:1.3rem'>{}</h1><p>{}</p></body>",
        esc(title),
        esc(text)
    )
    .into_bytes()
}

pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    state: &Arc<crate::AppState>,
) -> Option<std::io::Result<()>> {
    // Google sends the browser back here after the person approves. A GET, from a top-level
    // navigation — and only ever acted on if its `state` matches the sign-in we started.
    //
    // **Under `/api/` on purpose.** Everything outside `/api/` is served to a paired device without a
    // token (it is how the pairing page itself loads), so a callback there would be reachable from the
    // network. Here it is behind the same gate as every command, and listed in `REMOTE_DENIED`.
    if path == "/api/oauth_google" || path.starts_with("/api/oauth_google?") {
        let result = match (
            query_param(path, "code"),
            query_param(path, "state"),
            query_param(path, "error"),
        ) {
            (_, _, Some(e)) => Err(format!("Google said: {e}")),
            (Some(code), Some(st), None) => finish_sign_in(&code, &st),
            _ => Err("this address is only for finishing a sign-in".into()),
        };
        let (status, html) = match result {
            Ok(()) => ("200 OK", page("Gmail is connected", "You can close this tab and go back to formicaria. Your labelled mail will be read in a moment.")),
            Err(e) => ("400 Bad Request", page("The sign-in did not finish", &e)),
        };
        return Some(crate::write_response(stream, status, "text/html; charset=utf-8", &html));
    }

    let req: Req = serde_json::from_slice(body).unwrap_or_default();
    let result: Result<Value, String> = match path {
        "/api/mail_status" => Ok(public(&load())),
        "/api/mail_setup" => setup(&req, state).map(|_| public(&load())),
        "/api/mail_connect" => connect(&req).map(|url| json!({ "url": url })),
        "/api/mail_read_now" => {
            read_all(state);
            Ok(public(&load()))
        }
        "/api/mail_disconnect" => disconnect().map(|_| public(&load())),
        "/api/mail_novelty" => Ok(json!({ "novelty": NOVELTY.load(Ordering::SeqCst) })),
        "/api/mail_heard" => heard(body).map(|_| json!({ "ok": true })),
        _ => return None,
    };
    let (status, out) = match result {
        Ok(v) => ("200 OK", v),
        Err(e) => ("400 Bad Request", json!({ "error": e })),
    };
    Some(crate::write_response(stream, status, "application/json", out.to_string().as_bytes()))
}

/// Keep the assistant's verdict on one conversation, replacing its earlier one.
fn heard(body: &[u8]) -> Result<(), String> {
    let mut h: Heard = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    if h.note.is_empty() {
        return Err("which conversation?".into());
    }
    h.said = h.said.chars().take(300).collect();
    h.at = now();
    edit(|s| {
        s.heard.retain(|x| x.note != h.note);
        s.heard.insert(0, h);
        s.heard.truncate(HEARD);
        Ok(())
    })
}

fn setup(req: &Req, state: &crate::AppState) -> Result<(), String> {
    let since = req.since.trim();
    if !since.is_empty() && (since.len() != 10 || since.parse::<fm_model::Stamp>().is_err()) {
        return Err("the start date should look like 2026-09-25".into());
    }
    let vault = {
        let list = fm_app::dispatch("list_vaults", &json!({}), &[], &state.app, &crate::Desktop)?
            .into_bytes();
        let names: Vec<String> = serde_json::from_slice::<Vec<Value>>(&list)
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v["name"].as_str().map(str::to_string))
            .collect();
        let want = req.vault.trim();
        match (want.is_empty(), names.first()) {
            (true, Some(first)) => first.clone(),
            (false, _) if names.iter().any(|n| n == want) => want.to_string(),
            _ => return Err(format!("there is no notebook called '{want}'")),
        }
    };
    edit(|s| {
        if !req.client_id.trim().is_empty() {
            if !req.client_id.trim().ends_with(".apps.googleusercontent.com") {
                return Err("the client ID ends in .apps.googleusercontent.com — check you copied the whole of it".into());
            }
            s.client_id = req.client_id.trim().to_string();
        }
        if !req.client_secret.trim().is_empty() {
            s.client_secret = req.client_secret.trim().to_string();
        }
        // A changed label or start date is a different set of mail: read it again from the beginning.
        if s.label != req.label.trim() || s.since != since {
            s.seen.clear();
            s.threads.clear();
            s.last_checked = 0;
        }
        s.label = req.label.trim().to_string();
        s.since = since.to_string();
        s.vault = vault;
        Ok(())
    })
}

fn connect(req: &Req) -> Result<String, String> {
    let s = load();
    if s.client_id.is_empty() || s.client_secret.is_empty() {
        return Err("save the client ID and secret first".into());
    }
    // Only this machine's own addresses. Google allows any port on 127.0.0.1 for a desktop client.
    let port = req
        .origin
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| req.origin.strip_prefix("http://localhost:"))
        .and_then(|p| p.parse::<u16>().ok())
        .ok_or("open formicaria at http://127.0.0.1 to sign in")?;
    let redirect = format!("http://127.0.0.1:{port}/api/oauth_google");
    let (st, verifier) = (random(24)?, random(48)?);
    let url = consent_url(&s.client_id, &redirect, &st, &verifier);
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some((st, verifier, redirect));
    Ok(url)
}

/// Forget the sign-in here and ask Google to cancel it too. The notes already made are kept, like
/// everything else in this app.
fn forget_token() {
    *TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn disconnect() -> Result<(), String> {
    forget_token();
    let s = load();
    if !s.refresh_token.is_empty() {
        // Best effort: if Google cannot be reached, forgetting it here still stops all reading.
        let _ = post_form(REVOKE_URL, &form(&[("token", &s.refresh_token)]));
    }
    edit(|s| {
        s.refresh_token.clear();
        s.email.clear();
        s.seen.clear();
        s.heard.clear();
        s.last = None;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_consent_address_asks_for_read_only_and_nothing_else() {
        let url = consent_url(
            "id.apps.googleusercontent.com",
            "http://127.0.0.1:8765/api/oauth_google",
            "s",
            "v",
        );
        let scope = query_param(&url, "scope").unwrap();
        assert_eq!(scope, "https://www.googleapis.com/auth/gmail.readonly");
        assert!(!scope.contains(' '), "exactly one scope");
        assert_eq!(query_param(&url, "code_challenge_method").as_deref(), Some("S256"));
        assert_eq!(query_param(&url, "access_type").as_deref(), Some("offline"));
    }

    #[test]
    fn pkce_challenge_is_base64url_of_the_sha256() {
        // Expected value computed independently (Python: `urlsafe_b64encode(sha256(v)).rstrip("=")`),
        // so this checks the hash, the hex round trip and the unpadded base64url together.
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFrnWM9I"),
            "Z4Upx6wF0J5DH9June48DfC9Spcw8d7iSwHwDkValCA"
        );
    }

    /// The owner's case (2026-10-15): the exchange was labelled on 2 Oct, and both of today's
    /// messages — the student's and the reply — carry no label. They are read all the same.
    #[test]
    fn every_message_of_a_labelled_conversation_is_read_not_only_the_labelled_ones() {
        let thread = json!({ "messages": [
            { "id": "a", "internalDate": "1790000000000", "labelIds": ["formicaria", "INBOX"] },
            { "id": "b", "internalDate": "1790000100000", "labelIds": ["formicaria", "SENT"] },
            { "id": "c", "internalDate": "1792050000000", "labelIds": ["INBOX"] },
            { "id": "d", "internalDate": "1792054000000", "labelIds": ["SENT"] },
            { "id": "e", "internalDate": "1792055000000", "labelIds": ["DRAFT"] },
            { "id": "f", "internalDate": "1700000000000", "labelIds": ["formicaria"] },
        ]});
        let seen = vec!["a".to_string(), "b".to_string()];
        let since = since_ms("2026-09-25");
        assert_eq!(
            unread_in_thread(&thread, &seen, since),
            ["c", "d"],
            "not the draft, nor before since"
        );
    }

    #[test]
    fn the_start_date_is_read_as_that_days_start() {
        assert_eq!(since_ms("1970-01-02"), 86_400_000);
        assert_eq!(since_ms("2026-09-25"), 1_790_294_400_000);
        assert_eq!(since_ms(""), 0, "unset reads everything");
        assert_eq!(since_ms("soon"), 0);
    }

    #[test]
    fn the_query_is_the_label_since_the_date() {
        let s = Settings {
            label: "formicaria".into(),
            since: "2026-09-25".into(),
            ..Default::default()
        };
        assert_eq!(query(&s), "label:\"formicaria\" after:2026/09/25");
    }

    #[test]
    fn the_public_view_never_carries_a_secret() {
        let s = Settings {
            client_id: "id.apps.googleusercontent.com".into(),
            client_secret: "GOCSPX-secret".into(),
            refresh_token: "1//refresh-token".into(),
            ..Default::default()
        };
        let out = public(&s).to_string();
        assert!(!out.contains("GOCSPX-secret") && !out.contains("refresh-token"), "{out}");
        assert!(out.contains("\"connected\":true"));
    }
}
