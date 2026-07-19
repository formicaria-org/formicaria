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
#[tauri::command]
fn fm(
    cmd: String,
    args: serde_json::Value,
    app: tauri::State<'_, Arc<App>>,
) -> Result<String, String> {
    let out = dispatch(&cmd, &args, &[], &app, &AndroidHost)?;
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
        Ok(n) => eprintln!("ca-bundle: {n} certificates"),
        // Said out loud rather than swallowed: this is the difference between "sync is broken"
        // and "sync cannot verify anyone", and the two look identical from the UI.
        Err(e) => eprintln!("ca-bundle: {e}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
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
                eprintln!("unreadable notes: {}", skipped.join("; "));
            }
            tauri::Manager::manage(app, Arc::new(fm_app));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![fm])
        .run(tauri::generate_context!())
        .expect("error while running formicaria");
}
