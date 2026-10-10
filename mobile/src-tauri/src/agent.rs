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

use crate::MobileHost;
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

/// The bundled audio→transcript runtime — whisper.cpp's `whisper-server`, cross-compiled to arm64 and
/// staged (STATIC, single self-contained binary) by `ci/android-stage-whisper.sh`. Renamed `lib*.so`
/// for the same exec-from-jniLibs reason as [`SERVER_LIB`]. Absent ⇒ transcription simply stays off.
const WHISPER_SERVER_LIB: &str = "libwhisper-server.so";

/// What Settings needs to know about this phone, **in the same shape fm-serve answers**.
///
/// `(installed, why, transcribe_available)`, mirroring `/api/agent_status`
/// (`crates/fm-serve/src/agent.rs`). The shape is not a detail: the phone renders the *same*
/// `SettingsPanel.svelte` as the desktop, which reads `st.installed` — and `undefined` is falsy.
/// Answering only `{enabled, transcribe}` therefore made the row print "not available" with an
/// empty reason and no switch, on the one platform where the whole stack is bundled in the APK.
///
/// Checked, not assumed. `ci/android-stage-runtime.sh` is what puts the runtime in `jniLibs`, and a
/// build made without it should say so rather than offer a switch onto a missing binary — the same
/// stance the desktop takes when `agents/` is absent.
pub fn availability() -> (bool, String, bool) {
    // Our own cdylib is always mapped, so its directory is where Android extracted every packaged
    // `lib*.so` — the one place an app may exec from.
    let Some(dir) = native_lib_dir("libformicaria_mobile_lib.so") else {
        return (
            false,
            "The assistant cannot locate this app's own library directory, so it cannot start a \
             model. Everything else in formicaria works normally."
                .into(),
            false,
        );
    };
    let installed = dir.join(SERVER_LIB).exists();
    let why = if installed {
        String::new()
    } else {
        "This build of the app does not carry the model runtime, so the assistant cannot be turned \
         on. Everything else in formicaria works normally."
            .to_string()
    };
    // The weights are fetched on first enable, so what is asked here is whether the *runtime* is
    // aboard — the honest capability on a phone, where the download is part of turning it on.
    let transcribe_available = installed && dir.join(WHISPER_SERVER_LIB).exists();
    (installed, why, transcribe_available)
}

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

/// The agent's live control state — is a model running, and the off-switch to stop it. The desktop
/// mirror is fm-serve's `AgentState` + `agent.json`; on the phone the user drives it from Settings
/// (the `agent_status` / `set_agent` transport calls, answered in [`crate::fm`]). "Off" means **no
/// model in memory**: the watch loop is stopped and the `llama-server` child killed.
struct Control {
    running: bool,
    /// Which launch attempt is the current one. **The off-switch cannot be a flag alone**, because
    /// for the first minutes of a launch there is nothing to switch off: `launch` claims the slot,
    /// then spends up to a ~1.4 GB resumable download on a background thread before a stopper
    /// exists. `stop()` in that window used to `take()` a `None` stopper, set `running = false` and
    /// return — so "off" did not turn it off (the download finished and started a model against a
    /// persisted `enabled: false`), and `is_running()` then answered `false` while a model ran,
    /// which let the next on-toggle past the double-launch guard and spawn a *second*
    /// `llama-server` on the same port. A generation counter is what lets the worker discover it
    /// was cancelled at any point, including before it had anything to cancel.
    generation: u64,
    stopper: Option<fm_agent::watchdog::StopFlag>,
    /// The whisper-server off-switch, when audio transcription is on. Tripped alongside `stopper` so
    /// "off" (and app exit) frees the whisper model's memory too.
    whisper_stopper: Option<fm_agent::watchdog::StopFlag>,
}
fn control() -> &'static std::sync::Mutex<Control> {
    static C: std::sync::OnceLock<std::sync::Mutex<Control>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        std::sync::Mutex::new(Control {
            running: false,
            generation: 0,
            stopper: None,
            whisper_stopper: None,
        })
    })
}

/// Is `generation` still the launch the app wants? False once anything has stopped or restarted it.
fn current(generation: u64) -> bool {
    control().lock().map(|c| c.running && c.generation == generation).unwrap_or(false)
}

/// Is a model running right now? (What Settings shows, and the guard against a double launch.)
pub fn is_running() -> bool {
    control().lock().map(|c| c.running).unwrap_or(false)
}

/// Clear the running state — called on every exit path of the runner thread, so the slot frees up for
/// a future on-toggle whether the model stopped cleanly or failed to start.
///
/// **Only if the caller still owns the slot.** A worker that was cancelled and then finishes its own
/// unwinding must not clear state that belongs to the launch which replaced it: off → on in quick
/// succession would otherwise leave `running = false` with a live model, which is exactly the lie
/// that lets a third launch spawn a duplicate `llama-server` on the same port.
fn clear_running(generation: u64) {
    if let Ok(mut c) = control().lock() {
        if c.generation != generation {
            return;
        }
        c.running = false;
        c.stopper = None;
        c.whisper_stopper = None;
    }
}

/// The persisted on/off setting (`<agents_dir>/agent.json`, `{"enabled": bool}`). **Default on** on the
/// phone (where the assistant has always run), so an app update does not silently turn off a working
/// agent; the user turns it off in Settings when they want it gone from memory.
pub fn is_enabled(agents_dir: &std::path::Path) -> bool {
    std::fs::read_to_string(agents_dir.join("agent.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["enabled"].as_bool())
        .unwrap_or(true)
}

/// Whether audio transcription is on (`<agents_dir>/agent.json`, `{"transcribe": bool}`). **Default
/// off** — the phone's whisper runtime + model load only when the user turns it on, so a phone that
/// never wants it pays nothing (no ~75 MB download, no second model in RAM). Read at agent start.
pub fn is_transcribe_enabled(agents_dir: &std::path::Path) -> bool {
    std::fs::read_to_string(agents_dir.join("agent.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["transcribe"].as_bool())
        .unwrap_or(false)
}

/// Write some keys of `agent.json`, **leaving the others as they are**, so the two switches and
/// the remembered model choice never clobber one another.
fn write_keys(agents_dir: &std::path::Path, pairs: &[(&str, Value)]) {
    let _ = std::fs::create_dir_all(agents_dir);
    let path = agents_dir.join("agent.json");
    let mut all = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    for (k, v) in pairs {
        all[*k] = v.clone();
    }
    let _ = std::fs::write(path, format!("{all}\n"));
}

/// Persist both switches together.
fn write_settings(agents_dir: &std::path::Path, enabled: bool, transcribe: bool) {
    write_keys(agents_dir, &[("enabled", enabled.into()), ("transcribe", transcribe.into())]);
}

/// The model this person accepted, and the catalogue default at that moment (or when they last said
/// "not now") — the two things `fm_agent_run::installed::resolve` needs to tell a kept model from
/// one that has merely not been offered a replacement yet. Same keys as the desktop's `agent.json`.
fn choice(agents_dir: &std::path::Path) -> (Option<String>, Option<String>) {
    let all = std::fs::read_to_string(agents_dir.join("agent.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or(Value::Null);
    let get = |k: &str| all[k].as_str().map(str::to_string);
    (get("model"), get("default_seen"))
}

fn remember_choice(agents_dir: &std::path::Path, model: Option<&str>, default_seen: &str) {
    let mut pairs = vec![("default_seen", Value::from(default_seen))];
    if let Some(m) = model {
        pairs.push(("model", m.into()));
    }
    write_keys(agents_dir, &pairs);
}

/// The catalogue built into this app, written to app storage and read back. Written on every call,
/// not just first run: the phone's `models.toml` is app *config* (there is no editor for it on
/// device), so an app update must be able to change the catalogue. Cheap and idempotent.
fn catalogue(agents_dir: &std::path::Path) -> Result<Manifest, String> {
    let path = agents_dir.join("models.toml");
    std::fs::create_dir_all(agents_dir).map_err(|e| format!("mkdir {}: {e}", agents_dir.display()))?;
    std::fs::write(&path, EMBEDDED_MANIFEST).map_err(|e| e.to_string())?;
    Manifest::read(&path)
}

/// Which model runs on this phone, and whether a newer one is on offer — the desktop's rule, from
/// the same function, against the phone's own default.
pub fn installed(agents_dir: &std::path::Path) -> Option<fm_agent_run::installed::Installed> {
    let manifest = catalogue(agents_dir).ok()?;
    let (chosen, seen) = choice(agents_dir);
    Some(fm_agent_run::installed::resolve(
        &manifest,
        &agents_dir.join("models"),
        manifest.mobile_default(),
        chosen.as_deref(),
        seen.as_deref(),
    ))
}

/// A newer-model download in flight, or how the last one ended — the shape the desktop's
/// `provisioning` has, so the same Settings row reads both.
fn updating() -> &'static std::sync::Mutex<Option<Value>> {
    static U: std::sync::OnceLock<std::sync::Mutex<Option<Value>>> = std::sync::OnceLock::new();
    U.get_or_init(|| std::sync::Mutex::new(None))
}

/// Bumped to retire a newer-model download in flight (its partial file stays, so accepting again
/// resumes). Separate from the launch generation: stopping this download is not turning the
/// assistant off.
static UPDATE_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn cancel_update() {
    UPDATE_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut g) = updating().lock() {
        *g = None;
    }
}

pub fn update_progress() -> Option<Value> {
    updating().lock().ok().and_then(|g| g.clone())
}

/// Accept, or decline, the newer model an app update offered.
///
/// Declining records which default was declined. Accepting downloads it **beside** the running
/// model on a background thread, records the choice, and then restarts the assistant on it — the
/// phone runs the model in-process, so unlike the desktop the switch can happen at once. Nothing is
/// fetched until this is called: an update used to start the download by itself, on whatever network
/// the phone was on.
pub fn update_model(app: Arc<App>, agents_dir: PathBuf, dismiss: bool) -> Result<(), String> {
    let manifest = catalogue(&agents_dir)?;
    let default = manifest.mobile_default().to_string();
    let Some(offer) = installed(&agents_dir).and_then(|i| i.offer) else {
        return Err("There is no newer assistant model to download.".to_string());
    };
    if dismiss {
        remember_choice(&agents_dir, None, &default);
        return Ok(());
    }
    let set = |v: Value| {
        if let Ok(mut g) = updating().lock() {
            *g = Some(v);
        }
    };
    set(json!({ "stage": "model", "done": 0, "total": offer.bytes, "error": null }));
    let gen = UPDATE_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let live = move || UPDATE_GEN.load(std::sync::atomic::Ordering::SeqCst) == gen;
        let models_dir = agents_dir.join("models");
        let progress = |stage: &'static str| {
            move |done: u64, total: Option<u64>| {
                if !live() {
                    return;
                }
                if let Ok(mut g) = updating().lock() {
                    *g = Some(json!({ "stage": stage, "done": done, "total": total, "error": null }));
                }
            }
        };
        let never = move || !live();
        let fetched = fm_agent_run::fetch::ensure_model(&models_dir, &manifest, &default, &progress("model"), &never)
            .and_then(|_| {
                fm_agent_run::fetch::ensure_mmproj(&models_dir, &manifest, &default, &progress("projector"), &never)
            });
        if !live() {
            return; // stopped by the person: say nothing, change nothing
        }
        match fetched {
            Ok(_) => {
                remember_choice(&agents_dir, Some(&default), &default);
                set(json!({ "stage": "ready", "done": 0, "total": null, "error": null }));
                // Switch now, if the assistant is on: stop the old model, start the new one.
                if is_enabled(&agents_dir) {
                    stop();
                    // `stop` retires the old launch asynchronously; give its thread a moment to
                    // release the running slot before the new launch claims it.
                    for _ in 0..50 {
                        if !is_running() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    if let Err(e) = launch(app, agents_dir) {
                        log::warn!("study agent: the new model is downloaded but did not start: {e}");
                    }
                }
            }
            Err(e) => set(json!({ "stage": "failed", "done": 0, "total": null, "error": e })),
        }
    });
    Ok(())
}

fn write_enabled(agents_dir: &std::path::Path, on: bool) {
    write_settings(agents_dir, on, is_transcribe_enabled(agents_dir));
}

/// Persist the "Audio transcription" toggle (from Settings). Whisper is chosen when the agent *starts*
/// (a launch flag on a separate process), so like the desktop this applies at the next assistant start.
pub fn set_transcribe(agents_dir: &std::path::Path, on: bool) {
    write_settings(agents_dir, is_enabled(agents_dir), on);
}

/// Stop the running model now (trip its off-switch → the watchdog kills the `llama-server` child →
/// the watch loop ends and the thread clears the control state). Idempotent; used by the off toggle
/// and by app exit.
pub fn stop() {
    if let Ok(mut c) = control().lock() {
        if let Some(s) = c.whisper_stopper.take() {
            s.stop();
        }
        if let Some(s) = c.stopper.take() {
            s.stop();
        }
        c.running = false;
        // Retires the in-flight launch too, whatever stage it reached. A worker still downloading
        // weights sees this and returns without starting a model; one that has just started a model
        // kills it instead of registering it.
        c.generation += 1;
    }
}

/// Turn the agent on or off at runtime (from Settings). Persists the choice and starts/stops the model
/// immediately, so "off" frees its memory and "on" brings it back — no relaunch needed.
pub fn set_running(app: Arc<App>, agents_dir: PathBuf, on: bool) -> Result<(), String> {
    write_enabled(&agents_dir, on);
    if on {
        launch(app, agents_dir)
    } else {
        stop();
        Ok(())
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
        let out = dispatch(cmd, &args, &[], &self.app, &MobileHost)?;
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
        // `thread_roots`, not `discussions`: watch note comment threads too, so an @-mention in a
        // note's discussion (e.g. "First talk with Arne") is seen — not only first-class discussions.
        self.call("thread_roots", json!({}))
    }
    fn alive(&self) -> bool {
        true
    }
    fn reply(&self, note: &str, body: &str) -> Result<Value, String> {
        self.call("reply", json!({ "id": note, "body": body }))
    }
    fn reply_as(&self, note: &str, body: &str, name: &str, email: &str) -> Result<Value, String> {
        self.call("reply", json!({ "id": note, "body": body, "authorName": name, "authorEmail": email }))
    }
    /// **The phone records the same provenance the desktop does.** `Origin` carries what produced
    /// this proposal — the tool, the verbatim question, and the sources it was given — and
    /// `create_proposal` turns them into git trailers on the model's commit. Dropping them here
    /// would make every proposal made on a phone unattributable in the supervision record, which is
    /// exactly the class of gap that cannot be repaired after the fact.
    fn create_proposal(
        &self,
        note: &str,
        body: &str,
        name: &str,
        email: &str,
        origin: &fm_agent_run::fmserve::Origin,
    ) -> Result<Value, String> {
        self.call(
            "create_proposal",
            json!({
                "id": note,
                "body": body,
                "authorName": name,
                "authorEmail": email,
                "tool": origin.tool,
                "query": origin.query,
                "sources": origin.sources,
            }),
        )
    }
    fn blob_bytes(&self, reference: &str) -> Result<(Vec<u8>, String), String> {
        // In-process there is no HTTP blob route: resolve the path through the same `blob_path` fm-serve
        // uses, then read the bytes read-only. (Transcription itself is a later phone spike — the model
        // runtime isn't wired here yet — but the accessor exists so the seam is uniform.)
        // `Scope::All`: the on-device agent is this phone's own user, the same standing every
        // loopback caller has (`fm_app::scope`). A narrower scope here would be a second, weaker
        // access model — and there is no paired device on the *inside* of the app to narrow to.
        let path = fm_app::dispatch::blob_path(&self.app, &fm_app::Scope::All, reference)?;
        let bytes = std::fs::read(&path).map_err(|e| format!("read blob {reference}: {e}"))?;
        let mime = fm_core::ingest::sniff_mime(&path).unwrap_or_else(|| "application/octet-stream".into());
        Ok((bytes, mime))
    }
    // Presence/activity drive the desktop's "working…" wheel via fm-serve's side channel. On mobile
    // that channel is the app itself; wiring it to an in-process registry the webview can poll is a
    // follow-up — the agent answers without it (the reply lands and the thread refreshes).
    fn activity(&self, _disc: &str, _stage: &str, _question: &str) {}
    fn activity_done(&self, _disc: &str) {}
    fn present(&self, _name: &str) {}
}

/// Start the agent **only if it is enabled** (the persisted on/off setting) and not already running.
/// Called at app launch and by the on-toggle — a no-op when off, so "off" truly runs nothing.
pub fn start(app: Arc<App>, agents_dir: PathBuf) -> Result<(), String> {
    if !is_enabled(&agents_dir) {
        log::info!("study agent: off — enable it in Settings");
        return Ok(());
    }
    launch(app, agents_dir)
}

/// Launch the in-process agent (unconditional). The model **runtime** (`llama-server` + its
/// `ggml`/`llama` libs) is bundled in the APK's `jniLibs` and lives in the app's native-library dir —
/// the only place Android lets us `exec` it; the **model file** is fetched into app storage on first
/// enable. `agents_dir` (app storage) holds `models.toml` and `models/`.
///
/// Returns quickly. All the slow, may-fail work — writing the first-run manifest, downloading the
/// weights, cold-loading the model — happens on a background thread; if any of it fails, the app is a
/// working notebook without the agent (the failure is logged, never surfaced as a crash).
fn launch(app: Arc<App>, agents_dir: PathBuf) -> Result<(), String> {
    if is_running() {
        return Ok(()); // already up — the on-toggle is idempotent
    }
    let manifest = catalogue(&agents_dir)?;
    // **Which model:** the one already on this phone, for as long as the catalogue lists it — an app
    // update that moves `default_mobile` offers the new one in Settings instead of downloading it
    // unasked. Only a phone with no model at all (a first enable) fetches the default here.
    let default = manifest.mobile_default().to_string();
    let found = installed(&agents_dir).unwrap_or_default();
    if found.unsupported {
        return Err("the model on this phone is no longer supported by this version — Settings offers the new one".to_string());
    }
    let first_enable = found.running.is_none();
    let model_name = found.running.unwrap_or_else(|| default.clone());
    if first_enable {
        remember_choice(&agents_dir, Some(&model_name), &default);
    }

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
    let max_reply_chars = manifest.max_reply_chars;
    // Audio transcription is opt-in (Settings). Read it here (agents_dir isn't moved into the thread).
    let transcribe_on = is_transcribe_enabled(&agents_dir);
    let whisper_name = manifest.mobile_whisper().unwrap_or("ggml-tiny.en").to_string();

    // Claim the running slot now, before the (possibly long) download, so a second start()/on-toggle
    // no-ops instead of racing a duplicate model. Cleared on every exit path of the thread below.
    // `generation` is this attempt's identity: everything below checks it is still the wanted one,
    // so an off-toggle during the download is honoured instead of silently losing the race.
    let generation = match control().lock() {
        Ok(mut c) => {
            c.running = true;
            c.generation += 1;
            c.generation
        }
        Err(_) => return Err("the agent's control state is poisoned".to_string()),
    };

    std::thread::spawn(move || {
        // Fetch the weights if this is the first enable (resumable + checksum, retrying through the
        // mobile-network drops). Slow, and offline just means "not yet" — logged, not fatal. Log once
        // per MB, not per 64 KB chunk.
        let last_mb = std::cell::Cell::new(u64::MAX);
        let progress = |done: u64, total: Option<u64>| {
            let mb = done / 1_000_000;
            if last_mb.replace(mb) != mb {
                match total {
                    Some(t) => log::info!("study agent: fetching {model_name} {mb}/{} MB", t / 1_000_000),
                    None => log::info!("study agent: fetching {model_name} {mb} MB"),
                }
            }
        };
        // **Stop means stop, mid-download.** The generation test below already discarded a launch the
        // user had turned off — but only *after* the fetch finished, so switching the assistant off
        // during a 1.4 GB first-run download left it downloading anyway, on mobile data, for as long
        // as it took. The `.part` survives, so turning it back on resumes rather than restarts.
        let cancelled = || !current(generation);
        let model_gguf =
            match fm_agent_run::fetch::ensure_model(&models_dir, &manifest, &model_name, &progress, &cancelled) {
                Ok(p) => p,
                Err(e) => {
                    log::info!("study agent: no model yet ({e})");
                    clear_running(generation);
                    return;
                }
            };

        // The image projector, when the chosen model has one — fetched the same way and by the same
        // function as on the desktop. Best-effort: without it the model starts text-only and says so
        // when asked to read a picture, which is the honest state, not a failed launch.
        if let Err(e) = fm_agent_run::fetch::ensure_mmproj(&models_dir, &manifest, &model_name, &progress, &cancelled) {
            log::info!("study agent: no image projector yet ({e}) — starting text-only");
        }

        // Reclaim space: delete weights that belong to neither the model in use nor the one on offer
        // (whose download may be half done). What is left behind is an earlier model's, after an
        // accepted update.
        for gone in fm_agent_run::installed::sweep(&manifest, &models_dir, &[&model_name, &default]) {
            log::info!("study agent: removed unused model {}", gone.display());
        }

        // **Cancelled while downloading?** Then stop here, before a model is in memory. This is the
        // window an off-toggle used to fall into silently: the user turned the assistant off, the
        // download carried on, and a model started anyway against a persisted `enabled: false`.
        if !current(generation) {
            log::info!("study agent: cancelled before the model was started");
            return;
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
        // The "model dies with its supervisor" backstop (PR_SET_PDEATHSIG) now lives inside
        // SupervisedModel::launch — one shared path for desktop and phone, exercised by fm-agent's CI
        // tests — so it no longer needs repeating here.
        // Whether this model can see is decided here and nowhere else: `add_projector` is the one
        // function, shared with the desktop, that puts a projector on the command line.
        let sees = manifest.add_projector(&mut cmd, &model_name, &models_dir);
        let read_prompt = manifest.read_prompt(&model_name);
        // The projector is loaded into memory beside the weights, so admission must count it.
        let model_bytes = std::fs::metadata(&model_gguf).map(|m| m.len()).unwrap_or(500_000_000)
            + sees.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0);
        let model = match SupervisedModel::launch(cmd, SystemMonitor, &Need::new(model_bytes, 1_000_000_000), Limits::resident()) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("study agent: model failed to launch: {e}");
                clear_running(generation);
                return;
            }
        };
        // Register the off-switch so Settings-off (and app exit) can stop this model now — unless
        // an off-toggle landed in the microseconds since the check above, in which case the model
        // that just started is nobody's and is killed rather than registered. Registering it would
        // publish a stopper for a launch the app has already retired, and the *next* on-toggle would
        // then overwrite it — leaking a model nothing can stop.
        let mine = match control().lock() {
            Ok(mut c) if c.running && c.generation == generation => {
                c.stopper = Some(model.stopper());
                true
            }
            _ => false,
        };
        if !mine {
            log::info!("study agent: cancelled as the model came up — stopping it");
            model.stopper().stop();
            let _ = model.wait();
            return;
        }

        // Audio→transcript: when enabled and the runtime is bundled, fetch the small whisper model and
        // launch whisper-server BESIDE the text model. Its SupervisedModel preflight is the admission
        // gate — on a phone too tight to hold both, it fails and transcription stays off (chat + the
        // notebook keep working). whisper-server is a static, self-contained binary (no shared libs to
        // collide with llama's). The handle is bound for the thread's life so it isn't dropped (killed)
        // early; its off-switch is registered for Settings-off / app exit.
        let mut whisper_port_val: Option<u16> = None;
        let _whisper = if transcribe_on {
            let whisper_bin = runtime_dir.join(WHISPER_SERVER_LIB);
            if !whisper_bin.exists() {
                log::info!("study agent: transcription runtime not bundled — skipping");
                None
            } else {
                match fm_agent_run::fetch::ensure_model(&models_dir, &manifest, &whisper_name, &|_, _| {}, &cancelled)
                {
                    Ok(wmodel) => {
                        let wport = port + 1;
                        let mut wcmd = std::process::Command::new(&whisper_bin);
                        wcmd.arg("-m").arg(&wmodel).args([
                            "--host", "127.0.0.1",
                            "--port", &wport.to_string(),
                            "-t", &threads.to_string(),
                        ]);
                        let wbytes = std::fs::metadata(&wmodel).map(|m| m.len()).unwrap_or(80_000_000);
                        match SupervisedModel::launch(wcmd, SystemMonitor, &Need::new(wbytes, 300_000_000), Limits::resident()) {
                            Ok(wm) => {
                                // Same generation test as the text model: whisper's own download
                                // is another window an off-toggle can land in, and a stopper
                                // registered for a retired launch is a model nothing can stop.
                                match control().lock() {
                                    Ok(mut c) if c.running && c.generation == generation => {
                                        c.whisper_stopper = Some(wm.stopper());
                                    }
                                    _ => {
                                        log::info!("study agent: cancelled — stopping whisper");
                                        wm.stopper().stop();
                                    }
                                }
                                whisper_port_val = Some(wport);
                                log::info!("study agent: audio transcription on (whisper :{wport}, {whisper_name})");
                                Some(wm)
                            }
                            Err(e) => {
                                log::info!("study agent: transcription off — {e}");
                                None
                            }
                        }
                    }
                    Err(e) => {
                        log::info!("study agent: transcription off — no whisper model yet ({e})");
                        None
                    }
                }
            }
        } else {
            None
        };

        let agent = Agent {
            fm: DispatchVault { app },
            model_port: port,
            model: model_name.clone(),
            searxng_port: None, // no local proxy on a phone…
            web_direct: true, // …so /research uses the in-process HTTPS multi-source search (websearch.rs)
            whisper_port: whisper_port_val, // Some when whisper-server came up (transcription on + fits)
            whisper_model: whisper_name.clone(),
            vision: sees.is_some(),
            read_prompt,
            image_specialist: None,
            max_reply_chars,
            history_budget: 4000,
        };
        wait_ready(port);
        log::info!("study agent listening in-process — @{model_name}");
        set_present(&model_name, true); // now the @-picker can suggest it
        let stopper = model.stopper();
        serve_loop(&agent, &model_name, &logs_dir, 1, &|| model.finished(), &|| stopper.stop());
        set_present(&model_name, false);
        let _ = model.wait();
        clear_running(generation);
        log::info!("study agent stopped");
    });
    Ok(())
}
