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

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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

const INTERVAL: Duration = Duration::from_secs(15 * 60);
/// At most this many new messages per read, so a first read over a busy label cannot occupy the
/// machine for an hour. The rest come at the next read.
const PER_READ: usize = 200;
const LIMIT: usize = 25 * 1024 * 1024;

static READING: AtomicBool = AtomicBool::new(false);
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
    last: Option<Value>,
    last_checked: u64,
    /// Which note format the messages already read were written in. Below [`FORMAT`], `seen` is
    /// cleared once so every message is read again into the current format — safe, because a message
    /// already in its note is recognised by its marker and not written twice.
    format: u32,
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
            s.format = FORMAT;
            Ok(())
        })?;
    }
    let s = load();
    if s.refresh_token.is_empty() {
        return Err("not signed in".into());
    }
    let token = access_token(&s)?;
    let mut ids = Vec::new();
    let mut page = String::new();
    loop {
        let mut url = format!("{GMAIL}/messages?maxResults=100&q={}", enc(&query(&s)));
        if !page.is_empty() {
            url.push_str(&format!("&pageToken={}", enc(&page)));
        }
        let v = get_json(&url, &token)?;
        for m in v["messages"].as_array().into_iter().flatten() {
            if let Some(id) = m["id"].as_str() {
                if !s.seen.iter().any(|x| x == id) {
                    ids.push(id.to_string());
                }
            }
        }
        match v["nextPageToken"].as_str() {
            Some(p) if ids.len() < PER_READ => page = p.to_string(),
            _ => break,
        }
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
        Ok(())
    })?;
    report["at"] = json!(now());
    report["waiting"] = json!(waiting);
    Ok(report)
}

fn read_all(state: &crate::AppState) {
    if READING.swap(true, Ordering::SeqCst) {
        return;
    }
    let last = match read_once(state) {
        Ok(v) => v,
        Err(e) => json!({ "at": now(), "error": e }),
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
        loop {
            let s = load();
            if !s.refresh_token.is_empty()
                && now().saturating_sub(s.last_checked) >= INTERVAL.as_secs()
            {
                read_all(&state);
            }
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
        _ => return None,
    };
    let (status, out) = match result {
        Ok(v) => ("200 OK", v),
        Err(e) => ("400 Bad Request", json!({ "error": e })),
    };
    Some(crate::write_response(stream, status, "application/json", out.to_string().as_bytes()))
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
fn disconnect() -> Result<(), String> {
    let s = load();
    if !s.refresh_token.is_empty() {
        // Best effort: if Google cannot be reached, forgetting it here still stops all reading.
        let _ = post_form(REVOKE_URL, &form(&[("token", &s.refresh_token)]));
    }
    edit(|s| {
        s.refresh_token.clear();
        s.email.clear();
        s.seen.clear();
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
