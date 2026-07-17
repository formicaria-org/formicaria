//! `fm-serve` — the browser fallback. It exposes the same commands the desktop
//! window does (the pure functions in `fm_app::commands`, over one shared
//! `FileStore`) as a tiny localhost HTTP+JSON API, and serves the built UI. The
//! frontend's `ipc.ts` calls `POST /api/<command>` with the same camelCase args
//! it hands Tauri, so the identical bundle runs in a browser against the REAL
//! vault — no webkit involved.
//!
//! Localhost only, single user. std-only networking, thread-per-connection.

use fm_app::commands;
use fm_core::{backup, git, MultiStore, Reindex, Store};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// One configured vault: an audience, where it lives, and where its media backs up to.
struct VaultConfig {
    name: String,
    path: PathBuf,
    /// This vault's restic repo, or `None` when its media has nowhere to go.
    ///
    /// Per vault because **a restic repo is per repository** — backing up two vaults
    /// means two repos, so "the" restic repo for a set of vaults is not a thing that
    /// exists. A vault without one is not an error: you may well want your lab's notes
    /// shared over git and its media backed up by the lab, not by you.
    restic: Option<String>,
}

struct AppState {
    store: Mutex<MultiStore>,
    /// Every vault, in configured order. **The first is the default**: a note that names
    /// no vault (every fresh capture) lands there, so it should be the personal one. A
    /// single-vault install is just a list of one, which is why nothing below has a
    /// "multi" special case.
    vaults: Vec<VaultConfig>,
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
    let dist = std::env::var("FM_UI_DIST").unwrap_or_else(|_| "ui/dist".to_string());
    let addr = std::env::var("FM_ADDR").unwrap_or_else(|_| "127.0.0.1:8765".to_string());

    // Both spellings of "us": a browser sends whichever hostname the user typed.
    let port = addr.rsplit(':').next().unwrap_or("8765").to_string();
    let origins =
        vec![format!("http://127.0.0.1:{port}"), format!("http://localhost:{port}")];

    let vaults = load_vaults();
    let store = MultiStore::open(
        &vaults.iter().map(|v| (v.name.clone(), v.path.clone())).collect::<Vec<_>>(),
    )
    .expect("open vaults");
    // A vault opens even when a note is unreadable — a conflicted merge is the usual
    // cause, and refusing to start would take away the very app you need to fix it. But
    // those notes are now absent from every view, so say which ones: silently serving an
    // incomplete vault is the one outcome worse than not starting at all.
    let skipped = store.skipped();
    if !skipped.is_empty() {
        eprintln!("warning: {} note(s) could not be read and are missing from every view:", skipped.len());
        for note in &skipped {
            eprintln!("  {note}");
        }
        eprintln!("  (a conflicted merge? resolve the markers and reload.)");
    }
    let state = Arc::new(AppState {
        store: Mutex::new(store),
        vaults,
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
    for v in &state.vaults {
        println!("formicaria is serving {} at {}", v.name, v.path.display());
    }
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
        // Searched across vaults: the reference names bytes, not a place.
        "asset_status" => {
            let r = s("reference");
            let found = state.vaults.iter().find_map(|v| {
                commands::asset_status(&v.path, &r).ok().filter(|st| st.has_blob)
            });
            match found {
                Some(st) => json(st),
                // Absent everywhere. Still not an error — "media absence is a warning,
                // never an error" — so answer with the default vault's honest "no".
                None => json(commands::asset_status(&state.vault("")?.path, &r).map_err(err)?),
            }
        }
        "resolve_asset" => {
            let (r, k) = (s("reference"), s("kind"));
            state
                .vaults
                .iter()
                .find_map(|v| commands::resolve_asset_bytes(&v.path, &r, &k).ok())
                .ok_or_else(|| format!("blob not present in any vault: {r}"))
        }
        "open_external" => {
            open_blob(state, &s("reference"))?;
            Ok(Vec::new())
        }
        "commit" => {
            // Hold the store lock so a commit can't snapshot the vault mid-write
            // (fm-serve is thread-per-connection). Matches the retired desktop bin.
            let _guard = lock(state)?;
            json(git::commit_all(&state.vault(&s("vault"))?.path, &s("message")).map_err(err)?)
        }
        "backup" => {
            run_backup(state, &s("vault"))?;
            Ok(Vec::new())
        }
        // What the two backup tiers would actually do right now — the panel needs
        // this to promise the user only what it can deliver.
        "backup_status" => json(backup_status(state)?),
        // The identity rides along because this is the one moment it is worth
        // asking for: a vault gaining a remote is a vault gaining an audience, and
        // from here on every commit carries a name into somebody else's clone.
        // Empty means "don't touch it" — a user whose git is already configured is
        // never asked, so the panel sends nothing.
        "set_git_remote" => {
            let (name, email) = (s("name"), s("email"));
            if !name.is_empty() || !email.is_empty() {
                git::set_identity(&state.vault(&s("vault"))?.path, &name, &email).map_err(err)?;
            }
            git::set_remote(&state.vault(&s("vault"))?.path, &s("url")).map_err(err)?;
            Ok(Vec::new())
        }
        "push" => {
            // Same reason as `commit`: don't let a push snapshot the vault mid-write.
            let _guard = lock(state)?;
            json(git::push_squashed(&state.vault(&s("vault"))?.path, &s("message")).map_err(err)?)
        }
        // Bring a collaborator's work home. Holds the store lock for the same reason
        // push does — a merge rewrites notes under the app's feet, and the very next
        // incremental reindex is what makes them visible.
        "pull" => {
            let mut store = lock(state)?;
            let outcome = git::pull(&state.vault(&s("vault"))?.path).map_err(err)?;
            // The merge just wrote files behind the index's back. Re-read now rather
            // than leave the user staring at pre-pull content until the next heartbeat.
            store.reindex(Reindex::Incremental).map_err(err)?;
            json(match outcome {
                git::Pulled::UpToDate => PullResult { merged: 0, conflicts: Vec::new() },
                git::Pulled::Merged(n) => PullResult { merged: n, conflicts: Vec::new() },
                git::Pulled::Conflicted(f) => PullResult { merged: 0, conflicts: f },
            })
        }
        // The browser heartbeat, which doubles as **the local poll**. Liveness was
        // already refreshed by `handle` before dispatch, so the answer here is the
        // other half: has the vault moved under us?
        //
        // `get`/`query` serve SQLite, and a full reindex only runs at `open` — so
        // without this a `git pull`, a merge driver, or an edit in Vim is *invisible*
        // to a running app. The tab already beats every 3s for the watchdog, which is
        // exactly the cadence a local poll wants, so it costs one incremental reindex
        // rather than a second timer and a second round-trip. Quiet is the common
        // case and quiet is a stat per file.
        "ping" => {
            let changed = lock(state)?.reindex(Reindex::Incremental).map_err(err)?;
            json(Ping { changed: changed.updated > 0 || changed.removed > 0 })
        }
        // Binary upload: the raw request body IS the file; the name rides in the
        // query string (`/api/ingest?name=<urlencoded>`).
        "ingest" => {
            let name = query_param(query, "name").unwrap_or_else(|| "asset".to_string());
            // Into the vault the caller names — the blob lands beside the notes that
            // will reference it, and never in an audience that shouldn't have it. Both
            // the path and the name, so the bytes and the asset note land in the *same*
            // vault: splitting them puts the file in one audience and its note in another.
            let into = state.vault(&query_param(query, "vault").unwrap_or_default())?;
            let (path, vault_name) = (into.path.clone(), into.name.clone());
            json(
                commands::ingest(&mut *lock(state)?, &path, &vault_name, &name, body)
                    .map_err(err)?,
            )
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

fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, MultiStore>, String> {
    state.store.lock().map_err(|e| e.to_string())
}

impl AppState {
    /// The path of a named vault; an empty name means the default (the first).
    ///
    /// An **unknown** name is an error, never a fallback. Quietly writing a note meant
    /// for "lab" into "personal" is a disclosure that git history makes permanent, and
    /// the reverse silently loses the note — so a typo has to be loud.
    fn vault(&self, name: &str) -> Result<&VaultConfig, String> {
        if name.is_empty() {
            return Ok(&self.vaults[0]);
        }
        self.vaults
            .iter()
            .find(|v| v.name == name)
            .ok_or_else(|| format!("no vault named '{name}'"))
    }

    /// Where a blob really is. **Every vault is searched**, because a `sha256:`
    /// reference deliberately does not say which vault holds the bytes — and it should
    /// not: that is what keeps a cross-vault `note:`/`asset:` link free, and what lets
    /// ULIDs be the only identifier anyone needs. Content-addressing makes searching
    /// *correct* rather than merely convenient: whichever vault answers, the bytes hash
    /// to the reference, so they are the same bytes. Vault-scoping references would
    /// re-couple a note to a location and break the links.
    fn find_blob(&self, reference: &str) -> Result<PathBuf, String> {
        for v in &self.vaults {
            if let Ok(p) = commands::blob_path(&v.path, reference) {
                return Ok(p);
            }
        }
        Err(format!("blob not present in any vault: {reference}"))
    }
}

/// The vault list — **the first app-level config file**, and a deliberate break with
/// `decisions.md`'s "no new config file" (the backup panel keeps the remote in the
/// vault's own `.git/config`). A *set* of vaults cannot live inside any one vault, and
/// `FM_VAULT` is a single path. Breaking that principle on purpose, in one place, beats
/// breaking it by accident later.
///
/// `FM_VAULTS` points at the file; otherwise `$XDG_CONFIG_HOME/formicaria/vaults.json`.
/// Absent means the single-vault setup — `FM_VAULT`, named after its own directory —
/// which is every install that exists today. Nothing to migrate, and a list of one
/// behaves exactly like the old single vault.
///
/// `restic` is optional and **per vault**, because a restic repo *is* per repository:
/// there is no such thing as "the" restic repo for a set of vaults, so backing all of
/// them up means one repo each. A vault without one simply has nowhere to put its media,
/// and the panel says so rather than pretending.
///
/// ```json
/// { "vaults": [ { "name": "personal", "path": "/home/you/vault", "restic": "/backup/personal" },
///               { "name": "lab",      "path": "/home/you/lab-notes" } ] }
/// ```
fn load_vaults() -> Vec<VaultConfig> {
    let single = || {
        let path = PathBuf::from(std::env::var("FM_VAULT").unwrap_or_else(|_| "vault".into()));
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "vault".into());
        // The single-vault install kept its restic repo in FM_RESTIC_REPO. Honour it, so
        // an existing setup keeps backing up without touching anything.
        let restic = std::env::var("FM_RESTIC_REPO").ok().filter(|s| !s.is_empty());
        vec![VaultConfig { name, path, restic }]
    };

    let Some(config) = vault_list_path() else { return single() };
    let Ok(text) = std::fs::read_to_string(&config) else { return single() };
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            // Loud. A malformed vault list that fell back to the single vault would look
            // exactly like "my other vaults vanished", which is a bad hour.
            eprintln!("error: {} is not valid JSON: {e}", config.display());
            eprintln!("  falling back to the single vault. Fix the file and restart.");
            return single();
        }
    };
    let entries: Vec<VaultConfig> = parsed
        .get("vaults")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| {
                    let name = v.get("name")?.as_str()?.to_string();
                    let path = v.get("path")?.as_str()?;
                    Some(VaultConfig {
                        name,
                        path: PathBuf::from(expand_home(path)),
                        restic: v
                            .get("restic")
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .map(expand_home),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    if entries.is_empty() {
        eprintln!("warning: {} lists no vaults; using FM_VAULT", config.display());
        return single();
    }
    entries
}

fn vault_list_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FM_VAULTS") {
        return Some(PathBuf::from(p));
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok()?;
    Some(base.join("formicaria").join("vaults.json"))
}

/// `~` in a config file is what a human writes; nothing else expands it for us.
fn expand_home(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => match std::env::var("HOME") {
            Ok(home) => format!("{home}/{rest}"),
            Err(_) => path.to_string(),
        },
        None => path.to_string(),
    }
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn json<T: serde::Serialize>(v: T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&v).map_err(|e| e.to_string())
}

fn open_blob(state: &AppState, reference: &str) -> Result<(), String> {
    let path = state.find_blob(reference)?;
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open the file: {e}"))
}

/// The heartbeat's answer: did anything change on disk that the tab is not showing?
/// A bool, not a count — the UI's only choice is whether to re-run its query.
#[derive(serde::Serialize)]
struct Ping {
    changed: bool,
}

/// What a pull did. `conflicts` non-empty is a *result*, not an error: those notes have
/// markers in their body (the `.md` driver keeps them out of the frontmatter), so they
/// still open in the editor for a human to settle.
#[derive(serde::Serialize)]
struct PullResult {
    merged: u32,
    conflicts: Vec<String>,
}

/// One vault's git standing. **Per vault, not per app** — one vault is one repo, one
/// remote, one collaborator list, so every field here is singular *about that vault* and
/// there is no honest way to collapse them. A single "unpushed" number across a set of
/// vaults would be a number about nothing.
#[derive(serde::Serialize)]
struct VaultStatus {
    /// The audience. Doubles as the argument every git command takes back.
    name: String,
    /// Where this vault's notes push to (`origin`), or null when unset.
    remote: Option<String>,
    /// Commits made here but not on the remote; null when never pushed.
    unpushed: Option<u32>,
    /// Who this vault's commits are signed by, or null when nobody real is — the panel
    /// asks for a name only when this is null, so anyone whose git is already configured
    /// never sees the question. Per vault on purpose: a vault is an audience, and the
    /// name on a lab repo need not be the one on your personal notes.
    identity: Option<git::Identity>,
    /// Someone else has pushed work we don't have. Null when unknowable (no remote,
    /// never pushed, or offline — a sleeping laptop is not an error). One `ls-remote`,
    /// which moves no refs: knowing must not itself be the thing that puts the vault
    /// in the state the ancestry guard has to survive.
    remote_moved: Option<bool>,
    /// Notes with conflict markers sitting in them, waiting for a human.
    conflicts: Vec<String>,
    /// Where this vault's media backs up to — a path or URL, so the UI can say whether
    /// it would leave this machine. **Never the password.** Null when this vault has no
    /// restic repo, which is not an error: a restic repo is per repository, so a set of
    /// vaults needs one each, and you may well not want one for all of them.
    restic_repo: Option<String>,
    /// This vault's media could actually be backed up right now: it has a repo *and*
    /// `RESTIC_PASSWORD` is set.
    restic_ready: bool,
}

/// What each backup tier can do right now, per vault. fm-core stays free of environment
/// and configuration concerns, so the env-derived half is assembled here.
///
/// **Both tiers are per vault.** Git was always: one vault, one repo, one remote. And
/// restic is too, for the same shape of reason — a restic repo *is* per repository, so
/// backing up a set of vaults means a repo each. There is no app-wide media destination
/// to report, which is why there is no field here for one.
#[derive(serde::Serialize)]
struct BackupStatus {
    /// Every vault, in configured order; the first is the default. A single-vault
    /// install is a list of one, so the panel needs no separate shape for it.
    vaults: Vec<VaultStatus>,
}

fn backup_status(state: &AppState) -> Result<BackupStatus, String> {
    // One password for every repo. A per-vault password would have to live somewhere,
    // and the one place it must never live is the config file next to the paths.
    let has_password = std::env::var("RESTIC_PASSWORD").is_ok();
    let vaults = state
        .vaults
        .iter()
        .map(|v| VaultStatus {
            name: v.name.clone(),
            remote: git::remote(&v.path).unwrap_or(None),
            unpushed: git::unpushed(&v.path).unwrap_or(None),
            identity: git::identity(&v.path),
            remote_moved: git::remote_moved(&v.path).unwrap_or(None),
            conflicts: git::conflicts(&v.path).unwrap_or_default(),
            restic_ready: v.restic.is_some() && has_password,
            restic_repo: v.restic.clone(),
        })
        .collect();
    Ok(BackupStatus { vaults })
}

/// The media tier, for **one** vault — snapshot it into *its own* restic repo.
///
/// Per vault because a restic repo is per repository: there is no single destination
/// that could hold a set of vaults, so each one either has a repo of its own or has
/// nowhere for its media to go. A vault without one is not an error and must not be
/// silently folded into someone else's repo — the caller is told, by name, that this
/// vault's media stayed put. Refusing to say so is the overstatement this whole panel
/// exists to prevent.
fn run_backup(state: &AppState, vault: &str) -> Result<(), String> {
    let v = state.vault(vault)?;
    let repo = v.restic.as_ref().ok_or_else(|| {
        format!("no restic repo configured for '{}' — its media has nowhere to go", v.name)
    })?;
    let password = std::env::var("RESTIC_PASSWORD")
        .map_err(|_| "set RESTIC_PASSWORD for the restic repository".to_string())?;
    backup::backup(&v.path, Path::new(repo), &password)
        .map_err(|e| format!("backing up '{}': {e}", v.name))
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
