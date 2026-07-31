//! The Android shell: a window, the platform seam, and nothing else.
//!
//! **Everything it answers goes to [`fm_app::dispatch`]** — the same door `fm-serve` uses. That
//! is ruling 1 of the mobile design, and the whole reason a second frontend costs a file rather
//! than a fork: the `match` over command names, the vault list, and the lock discipline all live
//! in `fm-app`, so this crate has no opinion about any of them.
//!
//! There is exactly one `#[tauri::command]`, taking a command name and a JSON blob, because the
//! wire contract already *is* "name plus JSON" — `ui/src/lib/ipc.ts` posts precisely that to
//! `/api/<cmd>`. Enumerating thirty wrapper functions here would be thirty places to forget one.

use std::sync::Arc;

use fm_app::{dispatch, App, Host};

// Behind the `agent` feature (default on). A `--no-default-features` build compiles none of it, so
// a notes-only APK never links the model runner — the mobile half of "the core never knows the
// agent exists." See Cargo.toml.
#[cfg(feature = "agent")]
mod agent;

/// Android's answer to "hand this file to whatever owns it" is an `Intent`, which needs the
/// JVM. Wiring that is a later milestone (`tauri-plugin-opener`); until then this says so
/// rather than pretending, because a silent no-op here looks like a broken PDF to a user.
struct AndroidHost;

/// The last startup failure, if there was one — and the lock that serialises attempts to fix it.
///
/// A startup failure used to be a `?` out of the `setup` hook, i.e. a panic on the thread Tauri
/// runs `run()` on. The Android Activity does not die with it: `WryActivity.onCreate` has long
/// since returned, so the webview stays up, the IPC replies stop, and the page waits forever on a
/// `list_vaults` that will never answer. Holding the reason here instead means every command can
/// hand the UI a sentence to render.
static BOOT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Ensures the after-the-store work happens exactly once however many times boot is attempted.
static FINISH: std::sync::Once = std::sync::Once::new();

/// Open the vaults, publish them, and kick off everything that comes after — or return why not.
///
/// **Callable more than once, and that is the point.** The first draft recorded a failure in a
/// `OnceLock` and stopped there, which froze the message for the life of the process: the UI's
/// "Try again" button re-invoked `list_vaults`, `list_vaults` found no managed state, and answered
/// the same stale sentence — a button that provably could not help, in front of an app that a
/// single transient failure (an `index.sqlite` busy for one launch, a read that failed under memory
/// pressure) had bricked until the user found the app switcher. Since `manage` works on an
/// `AppHandle` at any time, a retry can genuinely retry.
///
/// `catch_unwind` because the shell must survive a panic *inside* the open — rusqlite, a descriptor
/// parse, serde — for the same reason: a panicked thread leaves a webview asking a backend that
/// will never speak, and the phone has no other channel to say so.
fn boot(handle: &tauri::AppHandle) -> Result<Arc<App>, String> {
    use tauri::Manager;
    let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(App::load))
        .map_err(|_| "opening the vaults panicked — see logcat for the backtrace".to_string())
        .and_then(|r| r.map_err(|e| format!("could not open vaults: {e}")));
    let (fm_app, skipped) = match loaded {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            return Err(e);
        }
    };
    if !skipped.is_empty() {
        // Same discipline as the desktop: a note that could not be read is named, never
        // swallowed. The UI surfaces these on the heartbeat.
        log::warn!("unreadable notes: {}", skipped.join("; "));
    }
    let app_state = Arc::new(fm_app);
    // **Published before anything else runs.** From here the `fm` command can answer, so every
    // millisecond spent after this point is a millisecond the UI has data for.
    handle.manage(app_state.clone());
    // One line, and it is load-bearing: it is what `ci/android-smoke.sh` greps to prove the vaults
    // opened *before* the optional work — the ordering whose absence made the first open gray.
    // (On the owner's own phone even this is invisible: MIUI suppresses our tag, which is why the
    // same fact has to reach the screen — see known-issues.md.)
    log::info!("vaults ready: {} vault(s)", app_state.configs().map(|c| c.len()).unwrap_or(0));

    // The rest is off the critical path, in this order, on one thread: the trust store first (the
    // agent's downloads need it), then the model. Both used to sit *above* the open, where they
    // delayed the first thing the user sees to buy nothing — neither is needed to read a note.
    let after = handle.clone();
    let for_agent = app_state.clone();
    FINISH.call_once(move || {
        std::thread::spawn(move || {
            install_ca_bundle(&after);
            // A notes-only build (`--no-default-features`) compiles no agent at all, so the store
            // it would have been handed is deliberately dropped here.
            #[cfg(not(feature = "agent"))]
            let _ = for_agent;
            #[cfg(feature = "agent")]
            {
                use tauri::Manager;
                if let Ok(dir) = after.path().app_data_dir() {
                    if let Err(e) = agent::start(for_agent, dir.join("agents")) {
                        log::info!("study agent not started: {e}");
                    }
                }
            }
        });
    });
    Ok(app_state)
}

/// The vault store, or a sentence saying why not — retrying the open if it never succeeded.
///
/// **Deliberately not a `tauri::State<Arc<App>>` parameter.** That extractor answers an unmanaged
/// state with *"state not managed for field `app` on command `fm`. You must call `.manage()`"* — a
/// message about our own wiring, shown to someone holding a phone. Resolving it by hand costs a few
/// lines and lets the shell recover instead of explaining itself.
fn vault_state(handle: &tauri::AppHandle) -> Result<Arc<App>, String> {
    use tauri::Manager;
    if let Some(state) = handle.try_state::<Arc<App>>() {
        return Ok(state.inner().clone());
    }
    // Serialise the attempts: the webview, the heartbeat and the agent all call in, and two
    // concurrent `App::load()`s would open — and reindex — every vault twice.
    let mut last = BOOT.lock().map_err(|_| "the startup lock is poisoned".to_string())?;
    // Re-check under the lock: whoever we queued behind may have succeeded.
    if let Some(state) = handle.try_state::<Arc<App>>() {
        *last = None;
        return Ok(state.inner().clone());
    }
    match boot(handle) {
        Ok(app) => {
            *last = None;
            Ok(app)
        }
        Err(e) => {
            *last = Some(e.clone());
            Err(e)
        }
    }
}

impl Host for AndroidHost {
    fn open_external(&self, path: &std::path::Path) -> Result<(), String> {
        Err(format!(
            "opening {} outside the app needs the platform opener, which this build does not \
             have yet",
            path.display()
        ))
    }
}

/// The single command. `cmd` is the name `ipc.ts` would have POSTed; `args` is the same JSON
/// body. The reply is the raw bytes `dispatch` produced, handed back as a string because every
/// command the shell can reach answers JSON (blob bytes go over a protocol handler, not IPC —
/// ruling 7).
/// Blob bytes, over a URI scheme rather than the command channel — **the "ruling 7" path that
/// was described in this file's own header and never written.**
///
/// Without it media does not work on this platform in either direction: `resolve_asset` answers
/// with raw bytes, and [`fm`] runs every reply through `String::from_utf8`, so a JPEG fails to
/// decode; while `assetUrl` points at `/api/blob/…`, an HTTP route that exists only in
/// `fm-serve` and never on a phone.
///
/// A protocol handler is the right shape rather than base64 over IPC: it **streams**, so a video
/// is not held in memory twice on the way to the screen, and the webview can seek within it.
///
/// `fmblob://localhost/<url-encoded reference>` — the reference is whatever the note wrote
/// (`sha256:…` or `asset:sha256-…`), resolved by the same command the desktop uses, so there is
/// one implementation of what a reference means.
fn blob_response(
    app: &tauri::AppHandle,
    method: &str,
    uri: &str,
    body: &[u8],
) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode};
    use tauri::Manager; // `try_state` lives on the trait, not on `AppHandle` itself
    let not_found = || {
        Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap_or_default()
    };
    // Everything after the host: `fmblob://localhost/<ref>` → `<ref>`.
    let Some(rest) = uri.split_once("://").and_then(|(_, r)| r.split_once('/')).map(|(_, r)| r)
    else {
        return not_found();
    };
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));

    // **The preflight is still answered, though nothing this app writes needs it now.** On
    // Android the page is served from `http://tauri.localhost` while this handler answers on
    // `http://fmblob.localhost` — a *different origin* — so any `fetch` here that is not a simple
    // request gets an `OPTIONS` first and stalls until something replies. Nothing did once, and
    // it surfaced in the page as a bare `TypeError: Failed to fetch` with no status to explain
    // it. Four lines to keep that from being rediscovered.
    if method.eq_ignore_ascii_case("OPTIONS") {
        return tauri::http::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, OPTIONS")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Max-Age", "86400")
            .body(Vec::new())
            .unwrap_or_else(|_| not_found());
    }

    let Some(state) = app.try_state::<Arc<App>>() else { return not_found() };

    // **Ingest does *not* ride this scheme, and cannot.** A POST here reaches this function
    // with its body silently dropped: wry intercepts through
    // `WebViewClient.shouldInterceptRequest(view, request: WebResourceRequest)`, and Android's
    // `WebResourceRequest` has no body accessor — there is nothing for wry to pass on. This
    // handler served `POST /ingest` for a while and stored every photo as zero bytes while
    // reporting success. Ingest lives on the `fm_ingest` IPC command; see its note.
    //
    // This scheme stays the way blobs come *out*, which is what it is good at: a GET streams,
    // and `<video>` can seek without the file ever being held whole in memory.

    let reference = percent_decode(path);
    let args = serde_json::json!({ "reference": reference, "kind": "full" });
    match dispatch("resolve_asset", &args, &[], &state, &AndroidHost) {
        Ok(out) => {
            let bytes = out.into_bytes();
            Response::builder()
                // Sniffed by the webview: the blob store is content-addressed and does not keep
                // the MIME beside the bytes, and guessing wrongly here would be worse than
                // letting the browser look.
                .header("Content-Type", "application/octet-stream")
                .header("Access-Control-Allow-Origin", "*")
                .body(bytes)
                .unwrap_or_else(|_| not_found())
        }
        // A missing blob is the ordinary case for a vault whose media has not synced — the note
        // renders a placeholder, which is the same thing the desktop does.
        Err(e) => {
            log::warn!("blob {reference}: {e}");
            not_found()
        }
    }
}

/// Minimal percent-decoding for the one place a reference crosses a URL.
///
/// Hand-rolled rather than adding a crate: a `sha256:` reference is hex plus one colon, so the
/// only escape that ever appears is `%3A`. Anything else passes through unchanged, which is the
/// conservative direction — a reference that fails to resolve renders a placeholder.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// **Ingest, the only way Android allows.**
///
/// The bytes arrive base64 in a JSON argument, which is wasteful and is nonetheless the only
/// transport that exists here. Two doors were tried and both are closed by the platform:
///
/// - **Tauri's raw IPC body.** Tauri's own docs: *"On Android, `InvokeBody::Raw` is not
///   supported. The enum will always contain `InvokeBody::Json`."*
/// - **A POST to the custom scheme.** wry intercepts through
///   `WebViewClient.shouldInterceptRequest(view, request: WebResourceRequest)`, and Android's
///   `WebResourceRequest` exposes the URL, the method and the headers — **and no body**. There is
///   no accessor to add; wry reads none because none exists. A POST reaches the handler with its
///   body silently dropped, which is exactly how every photo taken on a phone came to be stored
///   as zero bytes: `ingest` hashed nothing, and every capture produced the same reference,
///   `e3b0c442…b855`, the SHA-256 of the empty string.
///
/// So base64 it is: a third larger and copied a few times, against media that does not arrive at
/// all. The size ceiling that costs is real and is stated to the user rather than discovered as a
/// crash — see `MAX_INGEST` below.
#[tauri::command]
fn fm_ingest(
    name: String,
    vault: String,
    data: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let app = vault_state(&app_handle)?;
    let bytes = b64_decode(&data).ok_or_else(|| format!("{name}: could not decode the file"))?;
    let args = serde_json::json!({ "name": name, "vault": vault });
    let out = dispatch("ingest", &args, &bytes, &app, &AndroidHost).inspect_err(|e| {
        log::error!("ingest: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// Standard base64 → bytes. Hand-rolled rather than adding a crate for one function on one
/// platform: the alphabet is fixed, there is no padding subtlety worth a dependency, and this is
/// the only place in the tree that decodes any.
///
/// Returns `None` on any character outside the alphabet, so a truncated or mangled payload fails
/// loudly instead of ingesting a prefix of the photo.
fn b64_decode(s: &str) -> Option<Vec<u8>> {
    const fn val(c: u8) -> i8 {
        match c {
            b'A'..=b'Z' => (c - b'A') as i8,
            b'a'..=b'z' => (c - b'a' + 26) as i8,
            b'0'..=b'9' => (c - b'0' + 52) as i8,
            b'+' => 62,
            b'/' => 63,
            _ => -1,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u8;
    for &c in s.as_bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' {
            continue;
        }
        let v = val(c);
        if v < 0 {
            return None;
        }
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[tauri::command]
fn fm(
    cmd: String,
    args: serde_json::Value,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // `agents`/`agent_status`/`set_agent` are **transport** concerns — the assistant's presence and its
    // on/off setting, the data behind the UI's `@`-picker and the Settings toggle. On the desktop
    // fm-serve answers these; the core `dispatch` never has them (that keeps the core agent-agnostic).
    // So the phone answers them the same way, here in the shell — never dispatched into the vault.
    if cmd == "agents" {
        #[cfg(feature = "agent")]
        return Ok(serde_json::json!({ "agents": agent::online_agents() }).to_string());
        #[cfg(not(feature = "agent"))]
        return Ok("{\"agents\":[]}".to_string());
    }
    #[cfg(feature = "agent")]
    if cmd == "agent_status" || cmd == "set_agent" || cmd == "set_transcribe" {
        use tauri::Manager;
        let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?.join("agents");
        if cmd == "set_agent" {
            let on = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            agent::set_running(vault_state(&app_handle)?, dir, on)?;
            return Ok("{\"ok\":true}".to_string());
        }
        if cmd == "set_transcribe" {
            // Persist only — whisper is chosen at the next agent start (a launch flag on a separate
            // process), so it applies when the assistant next starts, exactly like the desktop.
            let on = args.get("transcribe").and_then(|v| v.as_bool()).unwrap_or(false);
            agent::set_transcribe(&dir, on);
            return Ok("{\"ok\":true}".to_string());
        }
        return Ok(serde_json::json!({
            "enabled": agent::is_enabled(&dir), "transcribe": agent::is_transcribe_enabled(&dir),
        })
        .to_string());
    }
    #[cfg(not(feature = "agent"))]
    if cmd == "agent_status" || cmd == "set_agent" || cmd == "set_transcribe" {
        return Ok("{\"enabled\":false,\"transcribe\":false,\"ok\":true}".to_string());
    }
    // Resolved here rather than in the signature, so the arms above answer while the vaults are
    // still opening (the `@`-picker and the Settings toggles need no store) and this one reports
    // *why* it cannot.
    let app = vault_state(&app_handle)?;
    // **Logged before it is returned.** The UI shows the message, but a phone screen is not
    // somewhere a stack of failures can be compared — and the whole point of the tag is that
    // a failing clone can be read off `adb logcat` instead of retyped by hand.
    let out = dispatch(&cmd, &args, &[], &app, &AndroidHost).inspect_err(|e| {
        log::error!("{cmd}: {e}");
    })?;
    String::from_utf8(out.into_bytes()).map_err(|e| e.to_string())
}

/// Where this device keeps our data.
///
/// **This is the hole Settings found.** `fm_app::vaults::config_dir()` falls to a catch-all arm
/// on Android that reads `FM_CONFIG_DIR`, and nothing sets it — so without this a phone cannot
/// persist a vault list at all. Tauri knows the platform's app-data directory, so the shell is
/// the right place to supply it: it is exactly the kind of fact only the platform has, which is
/// why it is set here rather than guessed inside `fm-app`.
fn configure_paths(handle: &tauri::AppHandle) {
    use tauri::Manager;
    let Ok(dir) = handle.path().app_data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    // Safety: single-threaded, before any vault is opened. `set_var` is the only way to reach
    // `config_dir()`, which reads the environment by design so that the same code works under
    // `fm-serve`, the CLI and here.
    // Every vault this device makes lives in here, and nothing else on the device writes to it.
    //
    // **A phone has no place a user could name.** There is no `$HOME`, no shell to `mkdir`
    // with, and no file manager that can reach an app's storage — so the desktop's "type a
    // folder" question has no answer here, and asking it produced a literal `~/notes` resolved
    // against the process working directory. The platform gives an app exactly one directory it
    // may write to; this is it, and `fm-app` puts vaults inside it by name.
    //
    // **The cost, stated:** Android deletes this when the app is uninstalled. That is the price
    // of a sandbox nothing else can touch, and it is the reason a phone vault wants a git remote
    // or a restic repo pointed at it rather than being the only copy.
    let root = dir.join("vaults");
    let _ = std::fs::create_dir_all(&root);

    // Safety: single-threaded, before any vault is opened. `set_var` is the only way to reach
    // `config_dir()`, which reads the environment by design so that the same code works under
    // `fm-serve`, the CLI and here.
    unsafe {
        std::env::set_var("FM_CONFIG_DIR", &dir);
        std::env::set_var("FM_VAULT_ROOT", &root);
        // First run needs *a* vault or the app opens on the first-run screen with nowhere to
        // create one — on a phone there is no shell to `mkdir` with.
        if std::env::var_os("FM_VAULT").is_none() {
            // **The pre-root default is honoured when it exists.** Earlier builds put the first
            // vault at `<app_data>/vault`, outside the root introduced here. Repointing at the
            // new location would leave those notes on disk and invisible — indistinguishable
            // from data loss to the person holding the phone. So an existing one keeps its
            // path, and only a fresh install starts inside the root.
            let legacy = dir.join("vault");
            let vault = if legacy.exists() { legacy } else { root.join("notes") };
            let _ = std::fs::create_dir_all(vault.join("notes"));
            std::env::set_var("FM_VAULT", &vault);
        }
    }
}

/// Give the vendored OpenSSL a CA trust store, without which **every** HTTPS remote fails.
///
/// `git2` links OpenSSL statically, and a vendored build bakes in a default certificate
/// directory that does not exist on Android. With no trust store libgit2 reports "the SSL
/// certificate is invalid" for a perfectly good github.com — an error that names the server
/// rather than the missing bundle, which is exactly how it wasted an evening.
///
/// **Not `SSL_CERT_DIR` pointed at Android's store**, which looks correct and silently does
/// nothing: that lookup is by hashed filename, and Android names its certificates with
/// OpenSSL's *pre-1.0.0* subject hash while a modern OpenSSL computes a different one. Measured
/// on the device — `01419da9.0` on disk, `8d89cda1` from `openssl -subject_hash`. See
/// [`fm_app::ca_bundle`] for the full reasoning; it concatenates instead, which uses no hashed
/// lookup at all.
///
/// Both directories are offered because which exists varies by version: the historical
/// `/system/etc/security/cacerts` and the Conscrypt APEX that newer releases serve from. On the
/// measured device both are present and overlap, which the bundle deduplicates.
///
/// Best-effort and non-fatal: a phone that cannot build a bundle is still a working notebook —
/// it just cannot reach an HTTPS remote, which is the same position it was in before.
fn install_ca_bundle(handle: &tauri::AppHandle) {
    use tauri::Manager;
    let Ok(dir) = handle.path().app_data_dir() else { return };
    let dirs = [
        std::path::Path::new("/apex/com.android.conscrypt/cacerts"),
        std::path::Path::new("/system/etc/security/cacerts"),
    ];
    match fm_app::ca_bundle::install(&dirs, &dir.join("ca-bundle.pem")) {
        Ok(n) => log::info!("ca-bundle: {n} certificates loaded into libgit2"),
        // Said out loud rather than swallowed: this is the difference between "sync is broken"
        // and "sync cannot verify anyone", and the two look identical from the UI.
        Err(e) => log::error!("ca-bundle: {e}"),
    }
}

/// Send this shell's diagnostics somewhere they can actually be read.
///
/// **Android routes neither Rust's stdout nor its stderr anywhere.** Every `eprintln!` in this
/// file has been writing into a void — which is precisely how "the SSL certificate is invalid"
/// stayed unexplained across several builds while the app was already reporting the cause. A
/// startup diagnostic nobody can read is not a diagnostic.
///
/// Everything lands under the `formicaria` tag: `adb logcat -s formicaria`.
fn install_logger() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("formicaria"),
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // First, so that everything below is visible — including its own failures.
            install_logger();
            configure_paths(app.handle());
            // The git token, if this device has one. **Only ever reached here**: a desktop
            // delegates to git's credential helper and stores nothing, so this call is the
            // mobile half of that split (`fm_app::secrets`). Must run before the first sync,
            // and after `configure_paths` — it reads from the config directory that sets up.
            // Cheap (one small file), so it stays on the critical path.
            fm_app::secrets::install_into_env();
            // Opening the vaults is the one slow thing left before the app can answer anything.
            //
            // **Nothing slow may be added above this line, and a failure here must not panic.**
            // The webview is built by Tauri *before* this hook runs (`tauri::app::setup` creates
            // the config windows first), so the page is already loading and already invoking
            // while we are here — and the event loop that delivers a reply does not start until
            // this returns. Whatever goes wrong, the phone must end up with a backend that can
            // say so: a `?` here fails the hook, which panics this thread and leaves the Activity
            // holding a webview with nothing behind it. That is not a crash the user can see or
            // report — it is a screen that never paints. Diagnosed on the owner's phone
            // 2026-07-31 ("gray screen on the first open, fine on the second").
            //
            // So the whole of it lives in `boot`, which cannot fail upward, and which the first
            // arriving command will attempt again if this pass did not manage it.
            if let Ok(mut last) = BOOT.lock() {
                *last = boot(app.handle()).err();
            }
            Ok(())
        })
        .register_uri_scheme_protocol("fmblob", |ctx, req| {
            blob_response(
                ctx.app_handle(),
                req.method().as_str(),
                &req.uri().to_string(),
                req.body(),
            )
        })
        .invoke_handler(tauri::generate_handler![fm, fm_ingest])
        .build(tauri::generate_context!())
        .expect("error while running formicaria")
        .run(|_app, event| {
            // When the app exits, stop the study agent's model — trip its off-switch so the
            // llama-server child is killed rather than left to be reaped (PDEATHSIG is the backstop).
            #[cfg(feature = "agent")]
            if let tauri::RunEvent::Exit = event {
                agent::stop();
            }
            let _ = &event;
        });
}
