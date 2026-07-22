//! The study agent running **in-process** on the phone.
//!
//! On the desktop the agent is a separate `agent-serve` process that reaches the vault over HTTP to
//! `fm-serve`. A phone has no such server (ruling 7), so the *same* runner instead reaches the vault
//! through [`fm_app::dispatch`] in-process — a second implementation of the `VaultAccess` seam. The
//! orchestration loop itself ([`fm_agent_run::watch::serve_loop`]) is byte-for-byte the desktop's.
//!
//! The model is a local `llama-server` (the prebuilt android-arm64 build) launched under the same
//! watchdog, talking over `127.0.0.1:<port>` — exactly as on the desktop. The **runtime binary + its
//! libs** are bundled in the APK's `jniLibs` and run from the app's native-library dir (the only place
//! Android permits `exec`); the **weights** are fetched into app storage (`agents/models/`) on first
//! enable. If the runtime isn't bundled or the model can't be fetched yet, the agent simply doesn't
//! start and the app runs on as a notebook.

use crate::AndroidHost;
use fm_agent::launch::SupervisedModel;
use fm_agent::preflight::Need;
use fm_agent::watchdog::{Limits, SystemMonitor};
use fm_agent_run::fmserve::VaultAccess;
use fm_agent_run::manifest::Manifest;
use fm_agent_run::nativelib::native_lib_dir;
use fm_agent_run::watch::{serve_loop, wait_ready};
use fm_agent_run::Agent;
use fm_app::{dispatch, App};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// The model catalogue, shipped **with the app** so first run has one without any asset plumbing —
/// the same `agents/models.toml` the desktop uses (its `runtime_url` is desktop-only and ignored here,
/// since the phone's runtime is bundled in `jniLibs`, not fetched). Written to app storage once, then
/// user-editable.
const EMBEDDED_MANIFEST: &str = include_str!("../../../agents/models.toml");

/// The bundled runtime binary's name in `jniLibs` — `llama-server` renamed to a `lib*.so` so Android
/// packages it and lets us `exec` it from `nativeLibraryDir` (see [`native_lib_dir`]).
const SERVER_LIB: &str = "libllama-server.so";

/// The in-process presence board — the mobile mirror of fm-serve's `AgentRegistry`. A phone has no
/// server to hold "who is online", so the shell holds it here: the runner marks its model present once
/// it is serving and absent when it stops, and the `agents` transport call ([`crate::fm`]) reads it to
/// feed the webview's `@`-mention picker. Kept off the vault-command path so the core stays
/// agent-agnostic, exactly as `agents` is an fm-serve endpoint and never a dispatch command.
fn presence() -> &'static std::sync::Mutex<Vec<String>> {
    static P: std::sync::OnceLock<std::sync::Mutex<Vec<String>>> = std::sync::OnceLock::new();
    P.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// The `@name`s of agents serving right now — what the `@`-picker lists. Empty until a model is up.
pub fn online_agents() -> Vec<String> {
    presence().lock().map(|g| g.clone()).unwrap_or_default()
}

fn set_present(name: &str, present: bool) {
    if let Ok(mut g) = presence().lock() {
        g.retain(|n| n != name);
        if present {
            g.push(name.to_string());
        }
    }
}

/// The in-process [`VaultAccess`]: every call is one `fm_app::dispatch`, the exact door the shell's
/// `fm` command already uses. No socket, no fm-serve — the vault cannot "go away," so `alive()` is
/// always true and only the model ending stops the loop.
struct DispatchVault {
    app: Arc<App>,
}

impl DispatchVault {
    fn call(&self, cmd: &str, args: Value) -> Result<Value, String> {
        let out = dispatch(cmd, &args, &[], &self.app, &AndroidHost)?;
        let bytes = out.into_bytes();
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}

impl VaultAccess for DispatchVault {
    fn get(&self, id: &str) -> Result<Value, String> {
        self.call("get", json!({ "id": id }))
    }
    fn thread(&self, note: &str) -> Result<Value, String> {
        self.call("thread", json!({ "id": note }))
    }
    fn discussions(&self) -> Result<Value, String> {
        self.call("discussions", json!({}))
    }
    fn alive(&self) -> bool {
        true
    }
    fn search(&self, query: &str) -> Result<Value, String> {
        self.call("search", json!({ "query": query }))
    }
    fn reply(&self, note: &str, body: &str) -> Result<Value, String> {
        self.call("reply", json!({ "id": note, "body": body }))
    }
    fn reply_as(&self, note: &str, body: &str, name: &str, email: &str) -> Result<Value, String> {
        self.call("reply", json!({ "id": note, "body": body, "authorName": name, "authorEmail": email }))
    }
    fn create_proposal(&self, note: &str, body: &str, name: &str, email: &str) -> Result<Value, String> {
        self.call("create_proposal", json!({ "id": note, "body": body, "authorName": name, "authorEmail": email }))
    }
    // Presence/activity drive the desktop's "working…" wheel via fm-serve's side channel. On mobile
    // that channel is the app itself; wiring it to an in-process registry the webview can poll is a
    // follow-up — the agent answers without it (the reply lands and the thread refreshes).
    fn activity(&self, _disc: &str, _stage: &str, _question: &str) {}
    fn activity_done(&self, _disc: &str) {}
    fn present(&self, _name: &str) {}
}

/// Start the in-process agent. The model **runtime** (`llama-server` + its `ggml`/`llama` libs) is
/// bundled in the APK's `jniLibs` and lives in the app's native-library dir — the only place Android
/// lets us `exec` it; the **model file** is fetched into app storage on first enable. `agents_dir`
/// (app storage) holds `models.toml` and `models/`.
///
/// Returns quickly. All the slow, may-fail work — writing the first-run manifest, downloading ~150 MB
/// of weights, cold-loading the model — happens on a background thread; if any of it fails, the app is
/// a working notebook without the agent (the failure is logged, never surfaced as a crash).
pub fn start(app: Arc<App>, agents_dir: PathBuf) -> Result<(), String> {
    // Write the shipped catalogue on every start, not just first run: the phone's models.toml is app
    // *config* (there is no editor for it on device), so an app update must be able to change the
    // model and the per-device defaults. A stale first-run copy is exactly why the phone kept running
    // the old default after an update. Cheap (a few hundred bytes) and idempotent.
    let manifest_path = agents_dir.join("models.toml");
    std::fs::create_dir_all(&agents_dir).map_err(|e| format!("mkdir {}: {e}", agents_dir.display()))?;
    std::fs::write(&manifest_path, EMBEDDED_MANIFEST).map_err(|e| e.to_string())?;
    let manifest = Manifest::read(&manifest_path)?;
    // The phone's own pick (default_mobile) — a smaller model than the laptop default.
    let model_name = manifest.mobile_default().to_string();

    // The runtime is exec'd from the native-library dir; refuse early (before spawning) if it isn't
    // bundled, so the reason is one clear log line rather than a launch failure deep in the thread.
    let runtime_dir = native_lib_dir("libformicaria_mobile_lib.so")
        .ok_or_else(|| "cannot locate the native-library dir from /proc/self/maps".to_string())?;
    let server_bin = runtime_dir.join(SERVER_LIB);
    if !server_bin.exists() {
        return Err(format!("no bundled model runtime at {} — build the APK with the runtime staged into jniLibs", server_bin.display()));
    }

    let models_dir = agents_dir.join("models");
    // A writable place for the runner's timing log (the runtime dir is read-only extracted libs).
    let logs_dir = agents_dir.join("runtime");
    let _ = std::fs::create_dir_all(&logs_dir);
    let (port, ctx, threads) = (manifest.port, manifest.ctx, manifest.mobile_threads());

    std::thread::spawn(move || {
        // Fetch the weights if this is the first enable (resumable + checksum, retrying through the
        // mobile-network drops). Slow, and offline just means "not yet" — logged, not fatal. Log once
        // per MB, not per 64 KB chunk.
        let last_mb = std::cell::Cell::new(u64::MAX);
        let model_gguf = match fm_agent_run::fetch::ensure_model(&models_dir, &manifest, &model_name, &|done, total| {
            let mb = done / 1_000_000;
            if last_mb.replace(mb) != mb {
                match total {
                    Some(t) => log::info!("study agent: fetching {model_name} {mb}/{} MB", t / 1_000_000),
                    None => log::info!("study agent: fetching {model_name} {mb} MB"),
                }
            }
        }) {
            Ok(p) => p,
            Err(e) => {
                log::info!("study agent: no model yet ({e})");
                return;
            }
        };

        // Reclaim space: once the current model is in place, delete any other weights left over from a
        // previous default (e.g. after an app update changed the pick). Keep only the model in use.
        if let Ok(entries) = std::fs::read_dir(&models_dir) {
            for e in entries.flatten() {
                let p = e.path();
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                if p != model_gguf && (ext == "gguf" || ext == "part") {
                    if std::fs::remove_file(&p).is_ok() {
                        log::info!("study agent: removed unused model {}", p.display());
                    }
                }
            }
        }

        let mut cmd = std::process::Command::new(&server_bin);
        cmd.env("LD_LIBRARY_PATH", &runtime_dir)
            .arg("-m")
            .arg(&model_gguf)
            .args([
                "--host", "127.0.0.1",
                "--port", &port.to_string(),
                "-c", &ctx.to_string(),
                "-t", &threads.to_string(),
                "--no-warmup",
            ]);
        let model_bytes = std::fs::metadata(&model_gguf).map(|m| m.len()).unwrap_or(500_000_000);
        let model = match SupervisedModel::launch(cmd, SystemMonitor, &Need::new(model_bytes, 1_000_000_000), Limits::resident()) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("study agent: model failed to launch: {e}");
                return;
            }
        };

        let agent = Agent {
            fm: DispatchVault { app },
            model_port: port,
            model: model_name.clone(),
            searxng_port: None, // mobile web search: through the shell's HTTPS, a later step
            max_reply_chars: 600,
            retrieve: 3,
            history_budget: 4000,
        };
        wait_ready(port);
        log::info!("study agent listening in-process — @{model_name}");
        set_present(&model_name, true); // now the @-picker can suggest it
        let stopper = model.stopper();
        serve_loop(&agent, &model_name, &logs_dir, 1, &|| model.finished(), &|| stopper.stop());
        set_present(&model_name, false);
        let _ = model.wait();
        log::info!("study agent stopped");
    });
    Ok(())
}
