//! Fetching a reference pack — the one part of it a transport does.
//!
//! `fm-app` knows what is offered, where it goes and how to read it, and links no HTTP client
//! (`decisions.md#toolchain`). This module is the fetch: two routes over one background job. It
//! sits behind its own feature for the usual reason — it carries `fm-fetch`, and
//! `--no-default-features` must keep building the std-only server.
//!
//! **Never unasked.** Nothing here runs until someone presses the control in Settings, which has
//! already shown the pack's name, size and licence (`decisions.md#agent`, 2026-10-10).

use fm_fetch::job::Job;
use serde_json::json;

static FOOD: Job = Job::new();

fn progress() -> serde_json::Value {
    let p = FOOD.progress();
    json!({
        "can_download": true,
        "running": p.running,
        "received": p.received,
        "total": p.total,
        "error": p.error,
        "done": p.done,
    })
}

fn start() -> Result<(), String> {
    let (url, sha) =
        fm_app::packs::food_source().ok_or("The food list is not available to download yet.")?;
    let dest = fm_app::packs::food_path().ok_or("this device has nowhere to keep the food list")?;
    // Already running is not an error: the answer is the same progress either way.
    FOOD.start(url, dest, Some(sha));
    Ok(())
}

pub fn route(stream: &mut dyn crate::Conn, path: &str, body: &[u8]) -> Option<std::io::Result<()>> {
    let result = match path {
        "/api/pack_progress" => Ok(()),
        "/api/pack_download" => {
            let cancel = serde_json::from_slice::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get("cancel").and_then(|c| c.as_bool()))
                .unwrap_or(false);
            if cancel {
                FOOD.cancel();
                Ok(())
            } else {
                start()
            }
        }
        _ => return None,
    };
    let (status, out) = match result {
        Ok(()) => ("200 OK", progress()),
        Err(e) => ("400 Bad Request", json!({ "error": e })),
    };
    Some(crate::write_response(stream, status, "application/json", out.to_string().as_bytes()))
}
