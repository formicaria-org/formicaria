//! `fm-serve` — the browser fallback. It exposes the same commands the desktop
//! window does (the pure functions in `fm_app::commands`, over one shared
//! `FileStore`) as a tiny localhost HTTP+JSON API, and serves the built UI. The
//! frontend's `ipc.ts` calls `POST /api/<command>` with the same camelCase args
//! it hands Tauri, so the identical bundle runs in a browser against the REAL
//! vault — no webkit involved.
//!
//! Localhost only, single user. std-only networking, thread-per-connection.

use fm_app::commands;
use fm_core::{backup, git, FileStore};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct AppState {
    store: Mutex<FileStore>,
    vault: PathBuf,
    dist: PathBuf,
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

fn main() {
    let vault = std::env::var("FM_VAULT").unwrap_or_else(|_| "vault".to_string());
    let dist = std::env::var("FM_UI_DIST").unwrap_or_else(|_| "ui/dist".to_string());
    let addr = std::env::var("FM_ADDR").unwrap_or_else(|_| "127.0.0.1:8765".to_string());

    // Both spellings of "us": a browser sends whichever hostname the user typed.
    let port = addr.rsplit(':').next().unwrap_or("8765").to_string();
    let origins =
        vec![format!("http://127.0.0.1:{port}"), format!("http://localhost:{port}")];

    let store = FileStore::open(&vault).expect("open vault");
    let state = Arc::new(AppState {
        store: Mutex::new(store),
        vault: PathBuf::from(&vault),
        dist: PathBuf::from(&dist),
        origins,
        last_seen: Mutex::new(Instant::now()),
        connected: AtomicBool::new(false),
    });

    if !state.dist.join("index.html").exists() {
        eprintln!(
            "warning: {} has no index.html — build the UI first (pnpm -C ui build)",
            state.dist.display()
        );
    }

    let listener = TcpListener::bind(&addr).unwrap_or_else(|e| panic!("bind {addr}: {e}"));
    println!("formicarium is serving the vault at {}", state.vault.display());
    println!("open  http://{addr}  in your browser");

    // The launcher sets FM_OPEN so a double-click opens the default browser. A
    // brief delay lets the listener accept before the browser's first request.
    if std::env::var_os("FM_OPEN").is_some() {
        let url = format!("http://{addr}");
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
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
/// The UI sends a heartbeat (`POST /api/ping`) every few seconds; when the last
/// tab closes the heartbeats stop and, after a short idle window, we quit — so
/// closing the tab closes the app. The idle window is longer than a page reload
/// (which briefly pauses the heartbeat), so a refresh doesn't kill the server.
/// Before the very first request a longer grace covers a cold browser start.
fn spawn_watchdog(state: Arc<AppState>) {
    const IDLE: Duration = Duration::from_secs(10);
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

    // Read headers; we care about the body length and who is calling.
    let mut content_length = 0usize;
    let mut origin: Option<String> = None;
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

    let (status, ctype, data) = if method == "POST" && path.starts_with("/api/") {
        api(&path[5..], &body, state)
    } else if method == "GET" || method == "HEAD" {
        static_file(&path, state)
    } else {
        ("405 Method Not Allowed", "text/plain".to_string(), b"method not allowed".to_vec())
    };

    write_response(&mut stream, status, &ctype, &data)
}

/// Dispatch one API command. Returns raw bytes for `resolve_asset`, JSON for
/// everything else, and a plain-text 500 body on error (which the UI shows in
/// its error banner or degrades to the missing-asset placeholder).
fn api(cmd: &str, body: &[u8], state: &AppState) -> (&'static str, String, Vec<u8>) {
    // Split an optional query string off the command (e.g. `ingest?name=foo.png`).
    // Every other arm sees the base command, so JSON dispatch is unaffected.
    let (cmd, query) = cmd.split_once('?').unwrap_or((cmd, ""));
    let args: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let s = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();

    let result: Result<Vec<u8>, String> = (|| match cmd {
        "board" => json(commands::board(&*lock(state)?, &s("groupBy")).map_err(err)?),
        "gallery" => json(commands::gallery(&*lock(state)?).map_err(err)?),
        "agenda" => json(commands::agenda(&*lock(state)?).map_err(err)?),
        "get" => json(commands::get(&*lock(state)?, &s("id")).map_err(err)?),
        "search" => json(commands::search(&*lock(state)?, &s("query")).map_err(err)?),
        "recent" => json(commands::recent(&*lock(state)?).map_err(err)?),
        "capture" => json(commands::capture(&mut *lock(state)?, &s("body")).map_err(err)?),
        "set_property" => {
            commands::set_property(&mut *lock(state)?, &s("id"), &s("key"), &s("value"))
                .map_err(err)?;
            Ok(Vec::new())
        }
        "update_body" => {
            commands::update_body(&mut *lock(state)?, &s("id"), &s("body")).map_err(err)?;
            Ok(Vec::new())
        }
        "delete" => {
            commands::delete(&mut *lock(state)?, &s("id")).map_err(err)?;
            Ok(Vec::new())
        }
        "asset_status" => json(commands::asset_status(&state.vault, &s("reference")).map_err(err)?),
        "resolve_asset" => {
            commands::resolve_asset_bytes(&state.vault, &s("reference"), &s("kind")).map_err(err)
        }
        "open_external" => {
            open_blob(state, &s("reference"))?;
            Ok(Vec::new())
        }
        "commit" => {
            // Hold the store lock so a commit can't snapshot the vault mid-write
            // (fm-serve is thread-per-connection). Matches the retired desktop bin.
            let _guard = lock(state)?;
            json(git::commit_all(&state.vault, &s("message")).map_err(err)?)
        }
        "backup" => {
            run_backup(state)?;
            Ok(Vec::new())
        }
        // What the two backup tiers would actually do right now — the panel needs
        // this to promise the user only what it can deliver.
        "backup_status" => json(backup_status(state)?),
        "set_git_remote" => {
            git::set_remote(&state.vault, &s("url")).map_err(err)?;
            Ok(Vec::new())
        }
        "push" => {
            // Same reason as `commit`: don't let a push snapshot the vault mid-write.
            let _guard = lock(state)?;
            json(git::push_squashed(&state.vault, &s("message")).map_err(err)?)
        }
        // The browser heartbeat — the request itself already refreshed liveness
        // in `handle`, so this only needs to answer 200 so the tab knows we're up.
        "ping" => Ok(Vec::new()),
        // Binary upload: the raw request body IS the file; the name rides in the
        // query string (`/api/ingest?name=<urlencoded>`).
        "ingest" => {
            let name = query_param(query, "name").unwrap_or_else(|| "asset".to_string());
            json(commands::ingest(&mut *lock(state)?, &state.vault, &name, body).map_err(err)?)
        }
        other => Err(format!("unknown command: {other}")),
    })();

    match result {
        Ok(bytes) if cmd == "resolve_asset" => {
            ("200 OK", "application/octet-stream".to_string(), bytes)
        }
        Ok(bytes) => ("200 OK", "application/json".to_string(), bytes),
        Err(msg) => ("500 Internal Server Error", "text/plain; charset=utf-8".to_string(), msg.into_bytes()),
    }
}

fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, FileStore>, String> {
    state.store.lock().map_err(|e| e.to_string())
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn json<T: serde::Serialize>(v: T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&v).map_err(|e| e.to_string())
}

fn open_blob(state: &AppState, reference: &str) -> Result<(), String> {
    let path = commands::blob_path(&state.vault, reference).map_err(err)?;
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open the file: {e}"))
}

/// What each backup tier can do right now. fm-core stays free of environment and
/// configuration concerns, so the env-derived half is assembled here.
#[derive(serde::Serialize)]
struct BackupStatus {
    /// Where the notes push to (`origin`), or null when unset.
    remote: Option<String>,
    /// Commits made here but not on the remote; null when never pushed.
    unpushed: Option<u32>,
    /// The restic repo — a path or URL, so the UI can say whether media would
    /// leave this machine. Never the password.
    restic_repo: Option<String>,
    /// Both restic env vars present, i.e. a full backup could actually run.
    restic_ready: bool,
}

fn backup_status(state: &AppState) -> Result<BackupStatus, String> {
    let restic_repo = std::env::var("FM_RESTIC_REPO").ok().filter(|s| !s.is_empty());
    Ok(BackupStatus {
        remote: git::remote(&state.vault).map_err(err)?,
        unpushed: git::unpushed(&state.vault).map_err(err)?,
        restic_ready: restic_repo.is_some() && std::env::var("RESTIC_PASSWORD").is_ok(),
        restic_repo,
    })
}

fn run_backup(state: &AppState) -> Result<(), String> {
    let repo = std::env::var("FM_RESTIC_REPO")
        .map_err(|_| "set FM_RESTIC_REPO to a restic repository path".to_string())?;
    let password = std::env::var("RESTIC_PASSWORD")
        .map_err(|_| "set RESTIC_PASSWORD for the restic repository".to_string())?;
    backup::backup(&state.vault, Path::new(&repo), &password).map_err(err)
}

/// Serve a file from the built UI. Unknown non-file paths fall back to
/// index.html so the single-page app owns client-side routing. Guards against
/// `..` path traversal.
fn static_file(path: &str, state: &AppState) -> (&'static str, String, Vec<u8>) {
    let clean = path.split('?').next().unwrap_or("/");
    let rel = if clean == "/" { "index.html" } else { clean.trim_start_matches('/') };
    if rel.split('/').any(|seg| seg == "..") {
        return ("400 Bad Request", "text/plain".to_string(), b"bad path".to_vec());
    }
    let file = state.dist.join(rel);
    if let Ok(bytes) = std::fs::read(&file) {
        return ("200 OK", content_type(rel).to_string(), bytes);
    }
    match std::fs::read(state.dist.join("index.html")) {
        Ok(bytes) => ("200 OK", "text/html; charset=utf-8".to_string(), bytes),
        Err(_) => ("404 Not Found", "text/plain".to_string(), b"not found".to_vec()),
    }
}

/// Pull one `key=value` out of a `&`-joined query string, percent-decoded.
fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| percent_decode(v))
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

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
