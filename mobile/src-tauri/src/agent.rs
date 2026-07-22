//! The study agent running **in-process** on the phone.
//!
//! On the desktop the agent is a separate `agent-serve` process that reaches the vault over HTTP to
//! `fm-serve`. A phone has no such server (ruling 7), so the *same* runner instead reaches the vault
//! through [`fm_app::dispatch`] in-process — a second implementation of the `VaultAccess` seam. The
//! orchestration loop itself ([`fm_agent_run::watch::serve_loop`]) is byte-for-byte the desktop's.
//!
//! The model is a local `llama-server` (the prebuilt android-arm64 build) launched under the same
//! watchdog, talking over `127.0.0.1:<port>` — exactly as on the desktop. It lives, with the model
//! file, under an `agents/` directory in app storage (`models.toml` + `models/` + `runtime/`); how it
//! gets there — bundled in the APK's native libs, or fetched on first enable — is a packaging concern,
//! not this module's. If it is absent, [`start`] returns an error the caller logs and the app runs on
//! without the agent.

use crate::AndroidHost;
use fm_agent::launch::SupervisedModel;
use fm_agent::preflight::Need;
use fm_agent::watchdog::{Limits, SystemMonitor};
use fm_agent_run::fmserve::VaultAccess;
use fm_agent_run::manifest::Manifest;
use fm_agent_run::watch::{serve_loop, wait_ready};
use fm_agent_run::Agent;
use fm_app::{dispatch, App};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

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

/// Start the in-process agent: launch the local model under the watchdog, then run the shared watch
/// loop on a background thread against a [`DispatchVault`]. `agents_dir` holds `models.toml`,
/// `models/`, and `runtime/llama-server` (+ its libs). Returns an error (which the caller logs) if the
/// manifest or model isn't present — the app is a working notebook without the agent.
pub fn start(app: Arc<App>, agents_dir: PathBuf) -> Result<(), String> {
    let manifest = Manifest::read(&agents_dir.join("models.toml"))?;
    let model_name = manifest.default.clone();
    let file = manifest
        .file(&model_name)
        .ok_or_else(|| format!("model '{model_name}' is not in {}/models.toml", agents_dir.display()))?;
    let model_gguf = agents_dir.join("models").join(file);
    let runtime = agents_dir.join("runtime");
    let port = manifest.port;

    let mut cmd = std::process::Command::new(runtime.join("llama-server"));
    cmd.env("LD_LIBRARY_PATH", &runtime)
        .arg("-m")
        .arg(&model_gguf)
        .args([
            "--host", "127.0.0.1",
            "--port", &port.to_string(),
            "-c", &manifest.ctx.to_string(),
            "-t", &manifest.threads.to_string(),
            "--no-warmup",
        ]);
    let model_bytes = std::fs::metadata(&model_gguf).map(|m| m.len()).unwrap_or(500_000_000);
    let model = SupervisedModel::launch(cmd, SystemMonitor, &Need::new(model_bytes, 1_000_000_000), Limits::resident())?;

    let agent = Agent {
        fm: DispatchVault { app },
        model_port: port,
        model: model_name.clone(),
        searxng_port: None, // mobile web search: through the shell's HTTPS, a later step
        max_reply_chars: 600,
        retrieve: 3,
        history_budget: 4000,
    };

    // Off the UI thread: wait for the model to warm, then watch discussions until it ends.
    std::thread::spawn(move || {
        wait_ready(port);
        log::info!("study agent listening in-process — @{model_name}");
        let stopper = model.stopper();
        serve_loop(&agent, &model_name, &runtime, 1, &|| model.finished(), &|| stopper.stop());
        let _ = model.wait();
        log::info!("study agent stopped");
    });
    Ok(())
}
