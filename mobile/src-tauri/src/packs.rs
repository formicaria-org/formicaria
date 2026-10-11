//! Fetching a reference pack on the phone — the twin of `fm-serve/src/packs.rs`, and it answers
//! the same two commands with the same keys so one Settings row reads both.
//!
//! `fm-app` says what is offered and where it goes and links no HTTP client; this is the fetch.
//! It rides on the updater's feature because that is what carries `fm-fetch` on a phone, **not** on
//! the assistant's: a food log has to work with the assistant off (`decisions.md#toolchain`,
//! 2026-10-11).

use fm_fetch::job::Job;
use serde_json::json;

static FOOD: Job = Job::new();

fn progress() -> String {
    let p = FOOD.progress();
    json!({
        "can_download": true,
        "running": p.running,
        "received": p.received,
        "total": p.total,
        "error": p.error,
        "done": p.done,
    })
    .to_string()
}

/// `pack_progress` and `pack_download`. Never unasked: nothing here runs until someone presses the
/// control in Settings, which has already shown the pack's name, size and licence.
pub fn handle(cmd: &str, args: &serde_json::Value) -> Result<String, String> {
    if cmd == "pack_download" {
        if args.get("cancel").and_then(|v| v.as_bool()).unwrap_or(false) {
            FOOD.cancel();
        } else {
            let (url, sha) = fm_app::packs::food_source()
                .ok_or("The food list is not available to download yet.")?;
            let dest = fm_app::packs::food_path()
                .ok_or("this device has nowhere to keep the food list")?;
            // Already running is not an error: the answer is the same progress either way.
            FOOD.start(url, dest, Some(sha));
        }
    }
    Ok(progress())
}
