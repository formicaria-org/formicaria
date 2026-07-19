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
    unsafe {
        std::env::set_var("FM_CONFIG_DIR", &dir);
        // First run needs *a* vault or the app opens on the first-run screen with nowhere to
        // create one — on a phone there is no shell to `mkdir` with.
        if std::env::var_os("FM_VAULT").is_none() {
            let vault = dir.join("vault");
            let _ = std::fs::create_dir_all(vault.join("notes"));
            std::env::set_var("FM_VAULT", &vault);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            configure_paths(app.handle());
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
