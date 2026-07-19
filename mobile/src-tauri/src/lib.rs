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

/// Android's answer to "hand this file to whatever owns it" is an `Intent`, which needs the
/// JVM. Wiring that is a later milestone (`tauri-plugin-opener`); until then this says so
/// rather than pretending, because a silent no-op here looks like a broken PDF to a user.
struct AndroidHost;

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
    let Some(state) = app.try_state::<Arc<App>>() else { return not_found() };

    // **Ingest rides the same scheme, in the other direction.** Tauri's raw IPC body does not
    // exist on Android — its own docs say so: "On Android, InvokeBody::Raw is not supported. The
    // enum will always contain InvokeBody::Json." A photo through JSON means base64, a third
    // larger and copied several times. A protocol handler receives the request body as bytes, so
    // a POST here carries the file exactly as the picker produced it.
    //
    // It also makes this identical in shape to the desktop, which POSTs the file to
    // `/api/ingest`. One transport, two directions, and the same `dispatch` arm underneath.
    if method.eq_ignore_ascii_case("POST") && path.starts_with("ingest") {
        let param = |k: &str| {
            query
                .split('&')
                .filter_map(|kv| kv.split_once('='))
                .find(|(n, _)| *n == k)
                .map(|(_, v)| percent_decode(v))
                .unwrap_or_default()
        };
        let args = serde_json::json!({ "name": param("name"), "vault": param("vault") });
        return match dispatch("ingest", &args, body, &state, &AndroidHost) {
            Ok(out) => tauri::http::Response::builder()
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(out.into_bytes())
                .unwrap_or_else(|_| not_found()),
            Err(e) => {
                log::error!("ingest: {e}");
                tauri::http::Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(e.into_bytes())
                    .unwrap_or_else(|_| not_found())
            }
        };
    }

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

#[tauri::command]
fn fm(
    cmd: String,
    args: serde_json::Value,
    app: tauri::State<'_, Arc<App>>,
) -> Result<String, String> {
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
            install_ca_bundle(app.handle());
            // The git token, if this device has one. **Only ever reached here**: a desktop
            // delegates to git's credential helper and stores nothing, so this call is the
            // mobile half of that split (`fm_app::secrets`). Must run before the first sync,
            // and after `configure_paths` — it reads from the config directory that sets up.
            fm_app::secrets::install_into_env();
            // Opening the vaults is the one slow thing at startup; do it after the paths are
            // set, and fail loudly rather than starting with a store that is not there.
            let (fm_app, skipped) = App::load().map_err(|e| -> Box<dyn std::error::Error> {
                format!("could not open vaults: {e}").into()
            })?;
            if !skipped.is_empty() {
                // Same discipline as the desktop: a note that could not be read is named, never
                // swallowed. The UI surfaces these on the heartbeat.
                log::warn!("unreadable notes: {}", skipped.join("; "));
            }
            tauri::Manager::manage(app, Arc::new(fm_app));
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
        .invoke_handler(tauri::generate_handler![fm])
        .run(tauri::generate_context!())
        .expect("error while running formicaria");
}
