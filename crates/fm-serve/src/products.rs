//! One packaged food, looked up by its barcode when a person asks — the transport half of
//! `decisions.md` 2026-10-12, *a barcode may be looked up, once, when the person asks*.
//!
//! In the server and not a command for the reason the calendars are: no HTTP client may enter
//! `fm-app`. Which address, under which name, and how the answer is read are all
//! `fm_app::product`; this makes the one request and writes nothing.

use serde_json::{json, Value};

pub fn route(stream: &mut dyn crate::Conn, path: &str, body: &[u8]) -> Option<std::io::Result<()>> {
    if path != "/api/product_lookup" {
        return None;
    }
    let code = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("code").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default();
    let found = fm_app::product::lookup(&code, &|url, agent| {
        fm_fetch::request("GET", url, &[("User-Agent", agent)], None, fm_app::product::LIMIT)
            .map_err(|e| e.to_string())
    });
    let (status, out) = match found {
        Ok(p) => ("200 OK", json!(p).to_string()),
        Err(e) => ("400 Bad Request", e),
    };
    let ctype = if status == "200 OK" { "application/json" } else { "text/plain" };
    Some(crate::write_response(stream, status, ctype, out.as_bytes()))
}
