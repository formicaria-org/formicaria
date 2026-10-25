//! The study agent running **in-process** on the phone.
//!
//! On the desktop the agent is a separate `agent-serve` process that reaches the vault over HTTP to
//! `fm-serve`. A phone has no such server (ruling 7), so the *same* runner instead reaches the vault
//! through [`fm_app::dispatch`] in-process — a second implementation of the `VaultAccess` seam. The
//! orchestration loop itself — run by Myrme's `host::Tools::run_in_process` — is the desktop's.
//!
//! The model is a local `llama-server` (the prebuilt android-arm64 build) launched under the same
//! watchdog, talking over `127.0.0.1:<port>` — exactly as on the desktop. The **runtime binary + its
//! libs** are bundled in the APK's `jniLibs` and run from the app's native-library dir (the only place
//! Android permits `exec`); the **weights** are fetched into app storage (`agents/models/`) on first
//! enable. If the runtime isn't bundled or the model can't be fetched yet, the agent simply doesn't
//! start and the app runs on as a notebook.

use crate::MobileHost;

// The Myrme contract this app is written for, stated once in `fm_app::MYRME_CONTRACT`.
fm_app::check_myrme_contract!();
use fm_agent_run::fmserve::VaultAccess;
use fm_agent_run::manifest::Manifest;
use fm_app::{dispatch, App};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// The model catalogue, shipped **with the app** so first run has one without any asset plumbing —
/// the same `agents/models.toml` the desktop uses (its `runtime_url` is desktop-only and ignored here,
/// since the phone's runtime is bundled in `jniLibs`, not fetched). Written to app storage once, then
/// user-editable.
const EMBEDDED_MANIFEST: &str = fm_agent_run::CATALOGUE;

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
    fm_agent_run::host::Bundled::availability(bundle().as_ref())
}

/// This app's bundled runtime: our own cdylib is always mapped, so its directory is where Android
/// extracted every packaged `lib*.so` — the one place an app may exec from.
fn bundle() -> Option<fm_agent_run::host::Bundled> {
    fm_agent_run::host::Bundled::locate("libformicaria_mobile_lib.so")
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

/// **What the in-process assistant is doing, per discussion** — the phone's half of what
/// `fm-serve`'s `agent_registry` keeps on the desktop: the stage the wheel names, the question, when
/// the turn began, and the reply so far while it is written (Myrme's contract 1.9, 2026-10-25).
/// This vault's `activity*` calls write it and the shell's `agent_activity_poll` reads it, in the
/// desktop's shape, so the webview has no phone branch. Until then the phone answered
/// `{"active": false}` to every poll and showed only its own guess.
struct Activity {
    stage: String,
    question: String,
    draft: String,
    since: std::time::Instant,
}

/// A turn quiet longer than this died without clearing, as on the desktop: the wheel never spins
/// forever.
const ACTIVITY_TTL: std::time::Duration = std::time::Duration::from_secs(180);

fn activity() -> &'static std::sync::Mutex<std::collections::HashMap<String, Activity>> {
    static A: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, Activity>>> =
        std::sync::OnceLock::new();
    A.get_or_init(Default::default)
}

/// `agent_activity_poll`'s answer for one discussion — the same fields `fm-serve` answers.
pub fn activity_poll(disc: &str) -> Value {
    let Ok(mut act) = activity().lock() else { return json!({ "active": false }) };
    if act.get(disc).is_some_and(|a| a.since.elapsed() > ACTIVITY_TTL) {
        act.remove(disc);
    }
    match act.get(disc) {
        Some(a) => json!({
            "active": true, "stage": a.stage, "question": a.question,
            "elapsed_secs": a.since.elapsed().as_secs(), "draft": a.draft,
        }),
        None => json!({ "active": false }),
    }
}

/// What can read a picture right now: the running model, when its projector is loaded. Said by
/// the runner with each heartbeat (`VaultAccess::present`), as on the desktop, and cleared when
/// the model stops, so a label is never handed to a server that has gone.
fn reader() -> &'static std::sync::Mutex<Option<fm_agent_run::ImageReader>> {
    static R: std::sync::OnceLock<std::sync::Mutex<Option<fm_agent_run::ImageReader>>> =
        std::sync::OnceLock::new();
    R.get_or_init(|| std::sync::Mutex::new(None))
}

/// **The OCR a label is read with** (Myrme 0.8.0, contract 1.7): the assistant's own, opened once in
/// this process by Myrme, with the APK's ONNX Runtime. There whenever its files are on the phone,
/// whether or not the assistant is running — which is what lets a phone read a label at all: its
/// vision model was never fit to.
fn label_ocr(
    agents_dir: &std::path::Path,
) -> Option<std::sync::Arc<dyn fm_agent_run::ocr::ReadPage>> {
    let manifest = catalogue(agents_dir).ok()?;
    let bundle = bundle()?;
    fm_agent_run::ocr::shared(agents_dir, &manifest, Some(bundle.dir())).ok()
}

/// A packet's label read off a photograph (`fm_agent_run::label`): text back, nothing written.
pub fn read_label(agents_dir: &std::path::Path, image: &[u8]) -> Result<Value, String> {
    let reader = reader().lock().ok().and_then(|g| g.clone());
    let got =
        fm_agent_run::label::read_with(label_ocr(agents_dir).as_deref(), reader.as_ref(), image)?;
    Ok(serde_json::json!({ "text": got.text, "model": got.model }))
}

/// Whether a label can be read on this phone, asked before anybody takes a picture.
pub fn label_reader(agents_dir: &std::path::Path) -> Value {
    let reader = reader().lock().ok().and_then(|g| g.clone());
    fm_agent_run::label::ready_json_with(label_ocr(agents_dir).as_deref(), reader.as_ref())
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
    /// Where that whisper-server listens, while it does — what dictation sends a recording to.
    whisper_port: Option<u16>,
}
fn control() -> &'static std::sync::Mutex<Control> {
    static C: std::sync::OnceLock<std::sync::Mutex<Control>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        std::sync::Mutex::new(Control {
            running: false,
            generation: 0,
            stopper: None,
            whisper_stopper: None,
            whisper_port: None,
        })
    })
}

/// Is `generation` still the launch the app wants? False once anything has stopped or restarted it.
fn current(generation: u64) -> bool {
    control().lock().map(|c| c.running && c.generation == generation).unwrap_or(false)
}

/// The port of the speech server this phone runs, while it runs — `None` when transcription is off,
/// did not fit, or the assistant has not started yet.
pub fn whisper_port() -> Option<u16> {
    control().lock().ok().and_then(|c| c.whisper_port)
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
        c.whisper_port = None;
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
/// This phone's name for its Myrme (`agent.json` `device`), or `phone` (2026-10-18). Read when the
/// assistant starts; the app stamps the messages written here with the same word.
pub fn device_name(agents_dir: &std::path::Path) -> String {
    std::fs::read_to_string(agents_dir.join("agent.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["device"].as_str().and_then(fm_agent::convo::device_word))
        .unwrap_or_else(|| "phone".into())
}

/// Name this phone's Myrme; takes effect when the assistant next starts. Answers the word kept.
pub fn set_device_name(agents_dir: &std::path::Path, typed: &str) -> String {
    let word = fm_agent::convo::device_word(typed).unwrap_or_else(|| "phone".into());
    write_keys(agents_dir, &[("device", Value::from(word.clone()))]);
    word
}

/// Whether this person chose *better listening* (`"listening": "better"`, 2026-10-18) — the larger
/// speech model, the same choice and key as the desktop's. Applies when the assistant next starts.
pub fn listening_better(agents_dir: &std::path::Path) -> bool {
    std::fs::read_to_string(agents_dir.join("agent.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .is_some_and(|v| v["listening"].as_str() == Some("better"))
}

pub fn set_listening(agents_dir: &std::path::Path, better: bool) {
    write_keys(
        agents_dir,
        &[("listening", Value::from(if better { "better" } else { "everyday" }))],
    );
}

/// What choosing better listening downloads, when the catalogue offers a choice at all.
pub fn listening_better_bytes() -> Option<u64> {
    let m = Manifest::parse(EMBEDDED_MANIFEST);
    let name = m.whisper_for(true)?;
    (name != m.whisper_for(false)?).then(|| m.model(&name)?.bytes).flatten()
}

fn write_settings(agents_dir: &std::path::Path, enabled: bool, transcribe: bool) {
    write_keys(agents_dir, &[("enabled", enabled.into()), ("transcribe", transcribe.into())]);
}

/// The model this person accepted, and the catalogue default at that moment (or when they last said
/// "not now") — the two things Myrme's `host::Tools::installed_for` needs to tell a kept model from
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
    std::fs::create_dir_all(agents_dir)
        .map_err(|e| format!("mkdir {}: {e}", agents_dir.display()))?;
    std::fs::write(&path, EMBEDDED_MANIFEST).map_err(|e| e.to_string())?;
    Manifest::read(&path)
}

/// Which model runs on this phone, and whether a newer one is on offer — the desktop's rule, from
/// the same function, against the phone's own default.
pub fn installed(agents_dir: &std::path::Path) -> Option<fm_agent_run::installed::Installed> {
    let manifest = catalogue(agents_dir).ok()?;
    let (chosen, seen) = choice(agents_dir);
    Some(fm_agent_run::host::Tools::new(agents_dir, false).installed_for(
        &manifest,
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
    fetch_and_switch(app, agents_dir, manifest, default, offer.bytes)
}

/// The models this phone may run, for Settings' chooser: Myrme's phone choices
/// (`Manifest::phone_choices`, contract 1.5), each with the few words the chooser shows ("Faster",
/// "More accurate, slower"), what it downloads, and whether it is the one running here.
pub fn models(agents_dir: &std::path::Path) -> Value {
    let Ok(manifest) = catalogue(agents_dir) else {
        return json!([]);
    };
    let running = installed(agents_dir).and_then(|i| i.running);
    let default = manifest.mobile_default();
    let list: Vec<Value> = manifest
        .phone_choices()
        .iter()
        .map(|m| {
            json!({
                "name": m.name, "label": m.phone_label, "bytes": m.bytes, "license": m.license,
                "vision": m.mmproj.is_some(), "mmproj_bytes": m.mmproj_bytes,
                "default": m.name == default, "running": running.as_deref() == Some(m.name.as_str()),
            })
        })
        .collect();
    Value::Array(list)
}

/// Record the model picked in the chooser before a **first** enable, so [`launch`] fetches it
/// rather than the default. Ignored when a model is already here (switching is [`choose_model`]) or
/// when the name is not one of this phone's choices.
pub fn prefer_first(agents_dir: &std::path::Path, name: &str) {
    let Ok(manifest) = catalogue(agents_dir) else { return };
    let here = installed(agents_dir).and_then(|i| i.running).is_some();
    if !here && manifest.phone_choices().iter().any(|m| m.name == name) {
        remember_choice(agents_dir, Some(name), manifest.mobile_default());
    }
}

/// Switch this phone to another of its models (Settings → *Model*): download it **beside** the one
/// running, record the choice, and restart on it — the same path as accepting a newer model. Only
/// a phone choice is accepted; choosing the one already running does nothing.
pub fn choose_model(app: Arc<App>, agents_dir: PathBuf, name: &str) -> Result<(), String> {
    let manifest = catalogue(&agents_dir)?;
    let Some(total) = manifest
        .phone_choices()
        .iter()
        .find(|m| m.name == name)
        .map(|m| m.bytes.map(|b| b + m.mmproj_bytes.unwrap_or(0)))
    else {
        return Err(format!("{name} is not a model this phone can use."));
    };
    if installed(&agents_dir).and_then(|i| i.running).as_deref() == Some(name) {
        return Ok(());
    }
    fetch_and_switch(app, agents_dir, manifest, name.to_string(), total)
}

/// Download `target` (weights and projector) on a background thread, then record it as this phone's
/// choice and, if the assistant is on, restart on it. Progress is [`update_progress`]'s.
fn fetch_and_switch(
    app: Arc<App>,
    agents_dir: PathBuf,
    manifest: Manifest,
    target: String,
    total: Option<u64>,
) -> Result<(), String> {
    let default = manifest.mobile_default().to_string();
    let set = |v: Value| {
        if let Ok(mut g) = updating().lock() {
            *g = Some(v);
        }
    };
    set(json!({ "stage": "model", "done": 0, "total": total, "error": null }));
    let gen = UPDATE_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let live = move || UPDATE_GEN.load(std::sync::atomic::Ordering::SeqCst) == gen;
        let progress = |stage: &'static str| {
            move |done: u64, total: Option<u64>| {
                if !live() {
                    return;
                }
                if let Ok(mut g) = updating().lock() {
                    *g = Some(
                        json!({ "stage": stage, "done": done, "total": total, "error": null }),
                    );
                }
            }
        };
        let never = move || !live();
        // The weights and the projector — Myrme's to fetch; the phone's runtime is bundled.
        let model_progress = progress("model");
        let projector_progress = progress("projector");
        let fetched = fm_agent_run::host::Tools::new(&agents_dir, false).provision_weights(
            &manifest,
            &target,
            true,
            &fm_agent_run::host::Progress {
                step: &|_| live(),
                bytes: &|stage, done, total| match stage {
                    fm_agent_run::host::Stage::Projector => projector_progress(done, total),
                    _ => model_progress(done, total),
                },
                cancelled: &never,
            },
        );
        if !live() {
            return; // stopped by the person: say nothing, change nothing
        }
        match fetched {
            Ok(_) => {
                remember_choice(&agents_dir, Some(&target), &default);
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
                        log::warn!(
                            "study agent: the new model is downloaded but did not start: {e}"
                        );
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
        c.whisper_port = None;
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
    /// The app's clock (2026-10-24, *one app clock*): what the store reads and writes on, so the
    /// assistant never falls back to a zone of its own here.
    fn zone(&self) -> Option<String> {
        self.call("clock", json!({})).ok()?["zone"].as_str().map(str::to_string)
    }
    /// The whole `clock` answer (Myrme contract 1.8): with it the assistant also takes the app's
    /// `today`, and proposes a start as a moment (`instants`).
    fn clock(&self) -> Option<Value> {
        self.call("clock", json!({})).ok()
    }
    fn reply(&self, note: &str, body: &str) -> Result<Value, String> {
        self.call("reply", json!({ "id": note, "body": body }))
    }
    fn reply_as(&self, note: &str, body: &str, name: &str, email: &str) -> Result<Value, String> {
        self.call(
            "reply",
            json!({ "id": note, "body": body, "authorName": name, "authorEmail": email }),
        )
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

    /// A new note proposed by the assistant — a recipe made from a recording (`/recipe`,
    /// 2026-10-18). The phone could not, until now: the trait's default refuses, which is right for
    /// a vault that cannot, and wrong for this one, which reaches the same `dispatch` the desktop does.
    fn propose_note(
        &self,
        about: &str,
        note: &Value,
        name: &str,
        email: &str,
        origin: &fm_agent_run::fmserve::Origin,
    ) -> Result<Value, String> {
        let mut args = note.clone();
        args["about"] = json!(about);
        args["authorName"] = json!(name);
        args["authorEmail"] = json!(email);
        args["tool"] = json!(origin.tool);
        args["query"] = json!(origin.query);
        args["sources"] = json!(origin.sources);
        self.call("propose_note", args)
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
        let mime =
            fm_core::ingest::sniff_mime(&path).unwrap_or_else(|| "application/octet-stream".into());
        Ok((bytes, mime))
    }
    // The wheel's stage and the reply so far, kept in process for `agent_activity_poll` (see
    // `activity`). A later stage keeps the draft, as on the desktop, so it does not flicker away
    // just before the reply lands.
    fn activity(&self, disc: &str, stage: &str, question: &str) {
        if let Ok(mut act) = activity().lock() {
            let a = act.entry(disc.to_string()).or_insert_with(|| Activity {
                stage: String::new(),
                question: String::new(),
                draft: String::new(),
                since: std::time::Instant::now(),
            });
            a.stage = stage.to_string();
            a.question = question.to_string();
        }
    }
    fn activity_done(&self, disc: &str) {
        if let Ok(mut act) = activity().lock() {
            act.remove(disc);
        }
    }
    fn activity_draft(&self, disc: &str, text: &str) {
        if let Ok(mut act) = activity().lock() {
            let a = act.entry(disc.to_string()).or_insert_with(|| Activity {
                stage: "writing the reply".into(),
                question: String::new(),
                draft: String::new(),
                since: std::time::Instant::now(),
            });
            a.draft = text.to_string();
        }
    }
    fn present(&self, _name: &str, sees: Option<&fm_agent_run::ImageReader>) {
        if let Ok(mut g) = reader().lock() {
            *g = sees.cloned();
        }
    }
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
    // A first enable fetches the model picked in the chooser (`prefer_first`), else the default.
    let picked = choice(&agents_dir).0.filter(|n| manifest.model(n).is_some());
    let model_name = found.running.unwrap_or_else(|| picked.unwrap_or_else(|| default.clone()));
    if first_enable {
        remember_choice(&agents_dir, Some(&model_name), &default);
    }

    // The runtime is exec'd from the native-library dir; refuse early (before spawning) if it isn't
    // bundled, so the reason is one clear log line rather than a launch failure deep in the thread.
    let bundle = bundle()
        .ok_or_else(|| "cannot locate the native-library dir from /proc/self/maps".to_string())?;
    if !bundle.dir().join(fm_agent_run::host::BUNDLED_SERVER).exists() {
        return Err(format!(
            "no bundled model runtime in {} — build the APK with the runtime staged into jniLibs",
            bundle.dir().display()
        ));
    }
    // Audio transcription is opt-in (Settings); the same speech weights as every device, or the
    // better ones when chosen (2026-10-18).
    let speech = is_transcribe_enabled(&agents_dir).then(|| {
        manifest.whisper_for(listening_better(&agents_dir)).unwrap_or_else(|| "ggml-base".into())
    });
    let device = device_name(&agents_dir);

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
        // Everything about the assistant's files and processes is Myrme's
        // (`fm_agent_run::host::Tools::run_in_process`); this app keeps its own state — the
        // generation that says a launch is still wanted, the stop switches Settings-off and app
        // exit use, and the `@`-picker's presence board.
        let tools = fm_agent_run::host::Tools::new(&agents_dir, false);
        let job = fm_agent_run::host::InProcess {
            vault: DispatchVault { app },
            model: model_name.clone(),
            offered: default,
            speech,
            device,
        };
        let hooks = fm_agent_run::host::Hooks {
            // **Stop means stop, mid-download**: an off-toggle retires this generation.
            cancelled: &|| !current(generation),
            // Register the off-switch — unless an off-toggle landed meanwhile, in which case the
            // model that just started is nobody's and is stopped rather than registered.
            model_up: &|stopper| match control().lock() {
                Ok(mut c) if c.running && c.generation == generation => {
                    c.stopper = Some(stopper);
                    true
                }
                _ => false,
            },
            speech_up: &|stopper, port| match control().lock() {
                Ok(mut c) if c.running && c.generation == generation => {
                    c.whisper_stopper = Some(stopper);
                    c.whisper_port = Some(port);
                    true
                }
                _ => false,
            },
            // As **Myrme**, the name a person types on every device (2026-10-18), not the model's.
            present: &|on| set_present(fm_agent::convo::MYRME, on),
            log: &|line| log::info!("{line}"),
        };
        if let Err(e) = tools.run_in_process(&bundle, &manifest, job, &hooks) {
            log::info!("study agent: {e}");
        }
        if let Ok(mut g) = reader().lock() {
            *g = None;
        }
        clear_running(generation);
    });
    Ok(())
}
