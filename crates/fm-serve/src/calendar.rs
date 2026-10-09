//! Your own calendars, read on their own — the transport half of `decisions.md` 2026-10-09.
//!
//! # Why this is in the server and not a command
//!
//! Two reasons, and both are rulings rather than taste. **No HTTP client may enter `fm-app` or
//! `fm-core`**, so the fetch has to happen in a shell; and **a private calendar address is a
//! secret**, so it lives in a `0600` file in this machine's config directory and every route here is
//! in `REMOTE_DENIED`. Converting the bytes into notes is the shared, model-free `calendar_sync`
//! command, reached through `dispatch` so the `ping` generation moves like any other write.
//!
//! # Read-only by construction
//!
//! A "secret address in iCal format" can only be read: there is no verb on it that writes. This
//! module issues one `GET` per calendar and nothing else.
//!
//! # Autonomous
//!
//! A background thread reads every calendar a few seconds after start and then once an
//! [`INTERVAL`]. The person adds an address once; after that meetings appear and move on their own.
//! A failure is never a dialog — it is recorded on that calendar's row, where the panel shows it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How often the calendars are read. Google refreshes its secret feed every few hours at best, so
/// more often than this buys nothing and costs the publisher a request.
const INTERVAL: Duration = Duration::from_secs(60 * 60);
/// A calendar larger than this is not a personal calendar.
const LIMIT: usize = 16 * 1024 * 1024;

/// One read at a time, from the background thread or a *Get now* press.
static PULLING: AtomicBool = AtomicBool::new(false);
/// Every read-modify-write of the settings file goes through this.
static FILE: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Default, Clone)]
struct Settings {
    /// The wall clock a `Z` time is shown in. Stated, because this process cannot read the
    /// machine's own (see `fm_core::events`).
    #[serde(default = "default_offset")]
    offset: String,
    #[serde(default)]
    feeds: Vec<Feed>,
    /// Unix seconds of the last completed read of all calendars.
    #[serde(default)]
    last_checked: u64,
}

fn default_offset() -> String {
    "+08:00".into()
}

#[derive(Serialize, Deserialize, Clone)]
struct Feed {
    label: String,
    /// **The secret.** Never serialised to a client — [`public`] is the only view that leaves.
    url: String,
    vault: String,
    #[serde(default)]
    last: Option<Value>,
}

fn path() -> Option<PathBuf> {
    Some(fm_app::vaults::config_dir()?.join("formicaria").join("calendars.json"))
}

fn load() -> Settings {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| Settings { offset: default_offset(), ..Default::default() })
}

fn store(s: &Settings) -> Result<(), String> {
    let p = path().ok_or("this machine has nowhere to keep settings")?;
    let text = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    fm_app::secrets::write_private(&p, &text)
}

fn edit<T>(f: impl FnOnce(&mut Settings) -> Result<T, String>) -> Result<T, String> {
    let _g = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = load();
    let out = f(&mut s)?;
    store(&s)?;
    Ok(out)
}

/// The person's stated wall clock, shared with the mail reader: one setting, one answer.
pub fn offset() -> String {
    load().offset
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `https://calendar.google.com/calendar/ical/…/private-…/basic.ics` → `calendar.google.com`.
/// Enough to recognise which calendar it is; nothing an onlooker could use.
fn host_of(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or("").to_string()
}

/// What a client may see: everything except the address.
fn public(s: &Settings) -> Value {
    json!({
        "offset": s.offset,
        "lastChecked": s.last_checked,
        "pulling": PULLING.load(Ordering::SeqCst),
        "feeds": s.feeds.iter().map(|f| json!({
            "label": f.label,
            "host": host_of(&f.url),
            "vault": f.vault,
            "last": f.last,
        })).collect::<Vec<_>>(),
    })
}

/// Read every calendar once. Returns after all of them; each row records its own outcome.
fn pull_all(state: &crate::AppState) {
    if PULLING.swap(true, Ordering::SeqCst) {
        return;
    }
    let s = load();
    let mut results = Vec::new();
    for f in &s.feeds {
        let outcome = fm_fetch::get(&f.url, LIMIT)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                let args = json!({ "vault": f.vault, "source": f.label, "offset": s.offset });
                fm_app::dispatch("calendar_sync", &args, &bytes, &state.app, &crate::Desktop)
                    .map(|o| o.into_bytes())
            })
            .and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string()));
        let last = match outcome {
            Ok(mut v) => {
                v["at"] = json!(now());
                v
            }
            // The fetch error names the URL, and the URL is the secret — so only the host is kept.
            Err(e) => json!({ "at": now(), "error": e.replace(&f.url, &host_of(&f.url)) }),
        };
        results.push((f.label.clone(), last));
    }
    let _ = edit(|cur| {
        for (label, last) in results {
            if let Some(f) = cur.feeds.iter_mut().find(|f| f.label == label) {
                f.last = Some(last);
            }
        }
        cur.last_checked = now();
        Ok(())
    });
    PULLING.store(false, Ordering::SeqCst);
}

/// The background reader: a first pass shortly after start, then once an [`INTERVAL`], keyed on the
/// stored `last_checked` so a restart does not re-read what was read a minute ago.
pub fn check_in_background(state: Arc<crate::AppState>) {
    std::thread::spawn(move || {
        // Let the app finish starting first, the same courtesy the updater's check takes.
        std::thread::sleep(Duration::from_secs(8));
        loop {
            let s = load();
            if !s.feeds.is_empty() && now().saturating_sub(s.last_checked) >= INTERVAL.as_secs() {
                pull_all(&state);
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Req {
    label: String,
    url: String,
    vault: String,
    offset: String,
}

pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    state: &Arc<crate::AppState>,
) -> Option<std::io::Result<()>> {
    let req: Req = serde_json::from_slice(body).unwrap_or_default();
    let result: Result<(), String> = match path {
        "/api/calendars" => Ok(()),
        "/api/calendar_add" => add(&req, state),
        "/api/calendar_remove" => edit(|s| {
            s.feeds.retain(|f| f.label != req.label);
            Ok(())
        }),
        "/api/calendar_offset" => fm_app::commands::parse_offset(&req.offset).and_then(|_| {
            edit(|s| {
                s.offset = req.offset.trim().to_string();
                Ok(())
            })
        }),
        // *Get now*. Explicit, so it ignores the interval.
        "/api/calendar_pull" => {
            pull_all(state);
            Ok(())
        }
        _ => return None,
    };
    let (status, out) = match result {
        Ok(()) => ("200 OK", public(&load())),
        Err(e) => ("400 Bad Request", json!({ "error": e })),
    };
    Some(crate::write_response(stream, status, "application/json", out.to_string().as_bytes()))
}

fn add(req: &Req, state: &crate::AppState) -> Result<(), String> {
    let (label, url) = (req.label.trim().to_string(), req.url.trim().to_string());
    if label.is_empty() {
        return Err("give the calendar a short name, such as work".into());
    }
    // `webcal://` is what Outlook and Apple hand out for the same address over HTTPS.
    let url = match url.strip_prefix("webcal://") {
        Some(rest) => format!("https://{rest}"),
        None => url,
    };
    if !url.starts_with("https://") {
        return Err("paste the calendar's address — it starts with https:// or webcal://".into());
    }
    // Resolved now, so a mistyped vault is refused here and not at three in the morning.
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
        if s.feeds.iter().any(|f| f.label == label) {
            return Err(format!("there is already a calendar called '{label}'"));
        }
        s.feeds.push(Feed { label, url, vault, last: None });
        // Read the new one at the next tick rather than waiting out the interval.
        s.last_checked = 0;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The address never reaches a client.** Searched across the whole serialised answer, so a
    /// field somebody adds later without thinking is caught too.
    #[test]
    fn the_public_view_never_carries_the_secret_address() {
        let secret =
            "https://calendar.google.com/calendar/ical/me%40gmail.com/private-0123abcd/basic.ics";
        let s = Settings {
            offset: "+08:00".into(),
            feeds: vec![Feed {
                label: "work".into(),
                url: secret.into(),
                vault: "notes".into(),
                last: Some(json!({ "error": "x" })),
            }],
            last_checked: 0,
        };
        let out = public(&s).to_string();
        assert!(!out.contains("private-0123abcd"), "{out}");
        assert!(out.contains("calendar.google.com"), "the host is what identifies it: {out}");
    }
}
