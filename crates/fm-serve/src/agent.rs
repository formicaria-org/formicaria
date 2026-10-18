//! All of fm-serve's study-agent machinery, behind the `agent` cargo feature: the presence/activity
//! registry, the `/api/agent_*` transport endpoints, and the auto-start spawn. Build
//! `--no-default-features` and this whole module (and its `AppState` field) is gone — so the claim
//! the rest of the file makes, that the core never learns the agent exists and `rm -rf agents/` is
//! byte-identical, is **compiler-enforced**, the way `fm-query`'s purity is.
//!
//! It is still transport-shaped: never a `dispatch` command, never on disk (bar the on/off setting),
//! so even *with* the feature the shared command core stays agent-agnostic.

use crate::agent_registry::AgentRegistry;
use crate::{write_response, AppState};
use std::path::PathBuf;

// The Myrme contract this app is written for, stated once in `fm_app::MYRME_CONTRACT`.
fm_app::check_myrme_contract!();

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// All the agent-only state, in one field so `AppState` gains exactly one gated member: the live
/// registry, whether the agent process was already spawned this run, and the port it watches.
pub struct AgentState {
    pub registry: AgentRegistry,
    pub running: AtomicBool,
    pub port: u16,
    /// What the first-enable download is doing, for the Settings row to poll. **In memory and
    /// transport-only**, like the registry beside it: a download is a fact about this run, and
    /// writing it to disk would leave a stale "downloading" behind a crash.
    pub provisioning: std::sync::Mutex<Option<Provision>>,
    /// Bumped every time provisioning is started or stopped. A worker compares it before each step
    /// and abandons the work when it no longer matches — **the phone's protocol, ported unchanged**
    /// (`mobile/src-tauri/src/agent.rs`), and the reason a cancel lands even while a 2.5 GB
    /// download is mid-flight, before any stopper exists to trip.
    pub generation: std::sync::atomic::AtomicU64,
}

/// A first-enable download in flight, or how the last one ended.
#[derive(Clone, serde::Serialize)]
pub struct Provision {
    /// What is being fetched right now, in words the row can print: `model`, `projector`,
    /// `runtime`, `ready`, or `failed`.
    pub stage: String,
    pub done: u64,
    /// `None` while a server sends no length — a progress line must then say bytes, not a lie
    /// about a percentage.
    pub total: Option<u64>,
    /// Set when `stage` is `failed`. Printed verbatim, as every other capability reason here is.
    pub error: Option<String>,
}

impl AgentState {
    pub fn new(port: u16) -> Self {
        Self {
            registry: AgentRegistry::new(),
            running: AtomicBool::new(false),
            port,
            provisioning: std::sync::Mutex::new(None),
            generation: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// Typed bodies for the endpoints — parsed with serde rather than poking a `Value`, so a malformed
/// body degrades to an empty default instead of a chain of `unwrap_or`s.
#[derive(serde::Deserialize, Default)]
struct ActivityReq {
    #[serde(default)]
    discussion: String,
    #[serde(default)]
    stage: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    done: bool,
}

#[derive(serde::Deserialize, Default)]
struct DiscReq {
    #[serde(default)]
    discussion: String,
}

#[derive(serde::Deserialize, Default)]
struct PresentReq {
    #[serde(default)]
    name: String,
    /// The model server that can read a picture, when the assistant has one.
    #[serde(default)]
    reader: Option<ReaderReq>,
}

#[derive(serde::Deserialize)]
struct ReaderReq {
    port: u16,
    model: String,
    /// Whether the catalogue records the model as fit to read a nutrition label.
    #[serde(default)]
    labels: bool,
}

#[derive(serde::Deserialize, Default)]
struct LabelReq {
    /// The picture, base64.
    #[serde(default)]
    data: String,
}

#[derive(serde::Deserialize, Default)]
struct EnabledReq {
    #[serde(default)]
    enabled: bool,
    /// Which catalogued model to provision, when the caller chose one. `None` takes the
    /// catalogue's default — the first-enable screen names it either way.
    #[serde(default)]
    model: Option<String>,
    /// Fetch the projector too, so the assistant can read images. A separate, larger download and
    /// therefore a separate answer, never inferred from the model having one.
    #[serde(default)]
    vision: bool,
}

#[derive(serde::Deserialize, Default)]
struct UpdateModelReq {
    /// "Not now": remember that this default was declined, and download nothing.
    #[serde(default)]
    dismiss: bool,
    /// Stop a download of the newer model that is under way. The assistant itself is left as it
    /// is: stopping this is not turning it off.
    #[serde(default)]
    cancel: bool,
}

#[derive(serde::Deserialize, Default)]
struct TranscribeReq {
    #[serde(default)]
    transcribe: bool,
}

/// Whether the local study agent should auto-start with formicaria. On when `FM_AGENT` is set (a
/// dev/override), otherwise from the per-device setting `<config>/agent.json` (`{"enabled": true}`)
/// that the settings screen writes. **Default off** — opt-in, so a user who never turns it on runs
/// pure, super-light formicaria.
pub fn enabled() -> bool {
    if std::env::var_os("FM_AGENT").is_some() {
        return true;
    }
    settings()["enabled"].as_bool().unwrap_or(false)
}

/// Whether audio→transcript is on — the "Audio transcription" toggle, stored beside `enabled` in
/// `agent.json` (`{"transcribe": true}`). Read when the agent starts; exported to the agent stack as
/// `FM_TRANSCRIBE=1` so it launches whisper. Default off — no whisper runtime loads unless asked.
pub fn transcribe_enabled() -> bool {
    settings()["transcribe"].as_bool().unwrap_or(false)
}

/// Whether this person chose *better listening* (`agent.json` `"listening": "better"`, 2026-10-18):
/// the larger speech model, slower and closer to what was said. Default: everyday listening.
fn listening_better() -> bool {
    settings()["listening"].as_str() == Some("better")
}

/// The speech weights this device should run, from the catalogue and the choice above.
fn whisper_chosen(manifest: &fm_agent_run::manifest::Manifest) -> String {
    manifest.whisper_for(listening_better()).unwrap_or_else(|| "ggml-base".to_string())
}

/// Whether the chosen speech weights still have to be downloaded — true after the move from
/// base.en, or after choosing better listening, while the earlier weights keep transcribing.
fn whisper_wanted_here() -> bool {
    let (Some(tools), Some(m)) = (tools(), read_manifest()) else { return false };
    tools.speech_wanted(&m, &whisper_chosen(&m))
}

/// Myrme's view of the tools folder (`fm_agent_run::host::Tools`, contract 1.1): everything about
/// what is installed there, fetched into it, removed from it, or started from it is Myrme's to
/// know. A checkout's `agents/` is marked as one, so nothing in it is removed or swept.
fn tools() -> Option<fm_agent_run::host::Tools> {
    agents_dir().map(|d| {
        let checkout = is_checkout(&d);
        fm_agent_run::host::Tools::new(d, checkout)
    })
}

fn read_manifest() -> Option<fm_agent_run::manifest::Manifest> {
    manifest_path().and_then(|p| fm_agent_run::manifest::Manifest::read(&p).ok())
}

/// Provision the assistant — fetch the runtime, the model, and the projector if one is wanted —
/// then start it. Runs on its own thread; the caller returns immediately.
///
/// **The generation counter is the whole cancellation story**, taken from the phone. A 2.5 GB
/// download cannot be interrupted by a stop flag that does not exist until the model is running, so
/// the worker re-reads the generation before and during every step and abandons the work the moment
/// it no longer matches. Turning the assistant off bumps it; so does turning it on again.
///
/// Nothing here is fatal to the app: a failure sets `stage: "failed"` with the reason, which the
/// Settings row prints, and leaves the `.part` files where a retry resumes them.
pub fn provision_and_spawn(state: &Arc<AgentState>, model: Option<String>, want_vision: bool) {
    provision(state, model, want_vision, After::Start);
}

/// What a finished download leads to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum After {
    /// A first enable: start the assistant now.
    Start,
    /// A newer model accepted while the old one is running: it takes over at the next launch, the
    /// way every other change to the assistant does (`set_agent`'s own rule).
    NextLaunch,
}

fn provision(state: &Arc<AgentState>, model: Option<String>, want_vision: bool, after: After) {
    let gen = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let st = Arc::clone(state);
    std::thread::spawn(move || {
        let set = |stage: &str, done: u64, total: Option<u64>, error: Option<String>| {
            if st.generation.load(Ordering::SeqCst) != gen {
                return false; // retired: stop touching shared state
            }
            *st.provisioning.lock().unwrap() =
                Some(Provision { stage: stage.into(), done, total, error });
            true
        };
        let live = || st.generation.load(Ordering::SeqCst) == gen;
        let cancelled = || !live();

        let Some(tools) = tools() else {
            set("failed", 0, None, Some("no place to keep the assistant on this machine".into()));
            return;
        };
        // The catalogue: whatever is already in the tools directory, else the copy shipped beside
        // the binary — copied in on first use, exactly as the phone writes its embedded one.
        let Some(mpath) = manifest_path() else {
            set("failed", 0, None, Some("the assistant has no model catalogue here".into()));
            return;
        };
        if let Err(e) = std::fs::create_dir_all(tools.dir()) {
            set("failed", 0, None, Some(format!("cannot create {}: {e}", tools.dir().display())));
            return;
        }
        if mpath != tools.dir().join("models.toml") {
            let _ = std::fs::copy(&mpath, tools.dir().join("models.toml"));
        }
        let manifest = match fm_agent_run::manifest::Manifest::read(&mpath) {
            Ok(m) => m,
            Err(e) => {
                set("failed", 0, None, Some(e));
                return;
            }
        };
        let name = model.unwrap_or_else(|| manifest.default.clone());

        // The runtime, the weights, and the projector when asked for — Myrme's to fetch
        // (`host::Tools::provision`); the progress shown and the cancel are this app's.
        let progress = fm_agent_run::host::Progress {
            step: &|stage| set(stage.word(), 0, None, None),
            bytes: &|stage, done, total| {
                if st.generation.load(Ordering::SeqCst) == gen {
                    *st.provisioning.lock().unwrap() =
                        Some(Provision { stage: stage.word().into(), done, total, error: None });
                }
            },
            cancelled: &cancelled,
        };
        if let Err(e) = tools.provision(&manifest, &name, want_vision, &progress) {
            if live() {
                set("failed", 0, None, Some(e));
            }
            return;
        }

        if !live() {
            return;
        }
        // Remember what was chosen, and the default it was chosen against. Without this the model
        // picked on the first-enable screen was never the one started (the supervisor took the
        // catalogue's default), and nothing could tell a deliberate choice from an old install.
        if let Err(e) = remember_choice(Some(&name), &manifest.default) {
            eprintln!("study agent: could not record the chosen model: {e}");
        }
        match after {
            After::Start => {
                set("ready", 0, None, None);
                spawn(st.port);
            }
            After::NextLaunch => {
                set("restart", 0, None, None);
            }
        }
    });
}

/// Is the assistant's stack already on this machine — runtime **and** weights?
///
/// Distinct from `unavailable()`, which asks whether this copy could ever run it. This asks whether
/// turning it on costs a download, which is the difference between a switch and a question.
///
/// The weights are checked by "any `.gguf` in `models/`" rather than by name: the catalogue's
/// default can change under an installation that already has a perfectly good model, and
/// re-downloading 2.5 GB because a default moved would be the wrong answer.
pub fn provisioned() -> bool {
    let (chosen, seen) = choice();
    tools().is_some_and(|t| t.is_provisioned(chosen.as_deref(), seen.as_deref()))
}

/// Delete the downloaded model and runtime, and nothing else — Myrme's `Tools::remove`, which also
/// refuses in a source checkout.
///
/// Until 2026-10-18 this refused whenever the folder held a `models.toml`, taking that for a
/// checkout. But provisioning copies the catalogue into the per-user tools folder, so on every
/// installed copy the removal control refused with "runs from a source checkout". The checkout
/// test is now the one `agents_dir` makes: is this the per-user tools folder or not.
fn remove_provisioned() -> Result<u64, String> {
    tools().ok_or("no tools directory on this machine")?.remove()
}

/// Is this directory a **checkout's** `agents/` rather than the per-user tools directory?
///
/// The catalogue is the tell: a checkout carries `models.toml` in git, and an empty `agents/`
/// sitting beside a released binary must not shadow the real tools directory — which is exactly
/// what "does this path exist" would have done.
fn looks_like_a_checkout(dir: &std::path::Path) -> bool {
    dir.join("models.toml").exists()
}

/// Fetch the speech-to-text runtime and its weights, then remember that transcription is wanted.
///
/// **The same shape as [`provision_and_spawn`]**, deliberately: one generation counter, one
/// progress field, one cancel story. A second download mechanism beside the first is how the two
/// drift into disagreeing about what "cancelled" means.
///
/// It does **not** restart the assistant. Whisper is a launch flag on a separate process, so it
/// applies the next time the assistant starts — which is what `agents/agent-serve.sh` did too, and
/// what the settings row has always said.
pub fn provision_transcribe(state: &Arc<AgentState>) {
    let gen = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let st = Arc::clone(state);
    std::thread::spawn(move || {
        let live = || st.generation.load(Ordering::SeqCst) == gen;
        let cancelled = || !live();
        let set = |stage: &str, done: u64, total: Option<u64>, error: Option<String>| {
            if live() {
                *st.provisioning.lock().unwrap() =
                    Some(Provision { stage: stage.into(), done, total, error });
            }
        };
        let Some(tools) = tools() else {
            set("failed", 0, None, Some("no place to keep the assistant on this machine".into()));
            return;
        };
        let Some(mpath) = manifest_path() else {
            set("failed", 0, None, Some("the assistant has no model catalogue here".into()));
            return;
        };
        let manifest = match fm_agent_run::manifest::Manifest::read(&mpath) {
            Ok(m) => m,
            Err(e) => {
                set("failed", 0, None, Some(e));
                return;
            }
        };
        let progress = fm_agent_run::host::Progress {
            step: &|stage| {
                set(stage.word(), 0, None, None);
                live()
            },
            bytes: &|_, done, total| {
                if st.generation.load(Ordering::SeqCst) == gen {
                    *st.provisioning.lock().unwrap() =
                        Some(Provision { stage: "transcribe".into(), done, total, error: None });
                }
            },
            cancelled: &cancelled,
        };
        if let Err(e) = tools.provision_speech(&manifest, &whisper_chosen(&manifest), &progress) {
            if live() {
                set("failed", 0, None, Some(e));
            }
            return;
        }
        if !live() {
            return;
        }
        let _ = set_transcribe(true);
        set("ready", 0, None, None);
    });
}

/// Spawn the local study agent — the opt-in script in `agents/`, told which fm-serve port to watch.
/// Returns whether it started. The agent (a separate process) then follows this server's liveness and
/// stops itself when we stop answering, so it needs no supervision from here.
pub fn spawn(port: u16) -> bool {
    if let Some(why) = unavailable() {
        eprintln!("study agent: enabled, but it cannot run here — {why}");
        return false;
    }
    let Some(exe) = agent_serve_path() else { return false };

    // **No shell.** This ran `bash agents/start-agent.sh`, which started `search-proxy.py`, trapped
    // it on exit and `tee`d a log — about ten lines of real work between two scripts, wrapping a
    // supervisor that was already Rust. The phone has run this same stack with no shell since it
    // shipped; the desktop now does too, which is what lets it run where `bash` and `python3` are
    // not a given, and what lets it run at all from an unpacked archive.
    // **Which model** is decided here, not left to the supervisor's default: the one this person
    // chose and still has, else whatever catalogued model is on disk. An update that moves the
    // catalogue's default therefore changes nothing until they accept the offer in Settings.
    let Some((manifest, _, found)) = installed_now() else {
        eprintln!("study agent: enabled, but there is no model catalogue to start from");
        return false;
    };
    let Some(model) = found.running else {
        eprintln!(
            "study agent: enabled, but no model the catalogue lists is on this machine — \
             Settings offers the download"
        );
        return false;
    };
    // The command is Myrme's to build (`host::Tools::launch`): it sweeps an earlier model's files
    // (never in a checkout), passes this app's port and instance, the device's name, in-process web
    // search, and speech when it is on and staged, and logs to `runtime/agent.log`.
    let Some(tools) = tools() else { return false };
    let mut cmd = tools.launch(
        &manifest,
        &fm_agent_run::host::Launch {
            exe: &exe,
            serve_port: port,
            serve_instance: crate::instance_id(),
            model: &model,
            device: &device_name(),
            speech: transcribe_enabled(),
            speech_weights: &whisper_chosen(&manifest),
        },
    );
    match cmd.spawn() {
        Ok(_) => {
            println!("study agent: starting — it comes up in a few seconds");
            true
        }
        Err(e) => {
            eprintln!("study agent: could not start: {e}");
            false
        }
    }
}

/// The agent stack's directory — `runtime/` and `models/` live under it.
///
/// **Two places, and the order is the decision.** A checkout's `agents/` wins when it is there, so
/// the dev loop is untouched by any of this. Otherwise it is `<config>/formicaria/tools`, the
/// per-user directory `vaults::config_dir` already resolves correctly on Linux, macOS and Windows —
/// and already holds `vaults.json` and `agent.json`, so this adds a directory rather than a
/// concept.
///
/// **Not inside the unpacked app folder**, which was the other candidate (`outstanding.md` §2.6b)
/// and would have travelled when the folder is copied. It loses on two counts: the folder is not
/// writable when someone unpacks to `/opt` or `C:\Program Files`, and every update would re-download
/// gigabytes into the new folder. The vault stays portable; a model is a per-machine cache and is
/// treated as one. Accepted and recorded in `decisions.md`.
pub fn agents_dir() -> Option<PathBuf> {
    // The checkout, when this is a dev run: `agents/` beside the binary, or at the repo root.
    let beside = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("agents")));
    if let Some(dir) = beside
        .into_iter()
        .chain(std::iter::once(PathBuf::from("agents")))
        .find(|p| looks_like_a_checkout(p))
    {
        return Some(dir);
    }
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("tools"))
}

/// The model catalogue this installation reads.
///
/// Shipped in the archive beside the binary and copied into the tools directory on first use, the
/// way the phone writes its embedded copy on every start — so a release has a catalogue without a
/// checkout, and a hand-edited one is still honoured.
pub fn manifest_path() -> Option<PathBuf> {
    let dir = agents_dir()?;
    let in_tools = dir.join("models.toml");
    let beside = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("models.toml")))
        .filter(|p| p.exists());
    // **The shipped catalogue is the current one.** It used to be copied into the tools directory
    // once and read from there for ever, so an updated app kept the catalogue of the release that
    // first enabled the assistant: no new model, and no changed setting either. Once per run, a
    // shipped copy that differs replaces the tools copy (the old one is kept beside it). A checkout
    // is never touched: its `agents/models.toml` is the developer's file, in git.
    static REFRESH: std::sync::Once = std::sync::Once::new();
    if let (Some(shipped), false) = (&beside, is_checkout(&dir)) {
        REFRESH.call_once(|| refresh_catalogue(shipped, &in_tools));
    }
    if in_tools.exists() {
        return Some(in_tools);
    }
    beside
}

/// Is `dir` a source checkout's `agents/` rather than this user's tools directory?
fn is_checkout(dir: &std::path::Path) -> bool {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("tools")).as_deref() != Some(dir)
}

/// Make `in_tools` a copy of `shipped` when they differ, keeping what was there as
/// `models.toml.previous`. Does nothing when the tools copy does not exist yet (first use copies it)
/// or already matches.
fn refresh_catalogue(shipped: &std::path::Path, in_tools: &std::path::Path) {
    let (Ok(new), Ok(old)) = (std::fs::read(shipped), std::fs::read(in_tools)) else { return };
    if new == old {
        return;
    }
    let _ = std::fs::copy(in_tools, in_tools.with_extension("toml.previous"));
    match std::fs::write(in_tools, new) {
        Ok(()) => println!("study agent: the model catalogue was updated with the app"),
        Err(e) => eprintln!("study agent: could not update the model catalogue: {e}"),
    }
}

/// **Why the assistant cannot run on this machine** — or `None` when it can.
///
/// A *reason*, not a bool, because the settings row has to print it: "not available" with no cause
/// is a dead end, and the three causes below want three different answers from the reader (buy
/// nothing / install the stack / this OS is not there yet).
///
/// The distinction from `enabled()` is the whole point. `enabled()` reads a stored flag and cannot
/// fail, so the settings row used to offer a toggle that returned `{"ok":true}`, promised *"Starts
/// with formicaria on the next launch"*, and did nothing at all. That is precisely the failure
/// `decisions.md` already ruled against: **a capability must mean "this will work", never "this is
/// configured".**
///
/// **Static on purpose — this is not `preflight::admit`.** `admit` reads *instantaneous* free
/// memory to decide whether to start the model right now, and it belongs where it is. Wiring it in
/// here would tell a user with a few browser tabs open that the assistant "is not installed" —
/// false, unactionable, and gone again by the time they looked. The same rule ("say what will
/// actually happen") broken in the other direction.
pub fn unavailable() -> Option<String> {
    // The OS gate is first because it cannot be fixed by installing anything. `fm_agent`'s resource
    // monitor reads `/proc`, and **fails closed everywhere else** — so on Windows and macOS
    // `preflight::admit` refuses before a model is ever spawned. Offering a switch there is
    // offering a switch onto a refusal. (`decisions.md#agent`, and the plan that split this out:
    // those platforms arrive with a monitor of their own, not by relaxing this.)
    //
    // **Replaced by an actual reading (2026-09-02).** It used to be a hardcoded OS list, which is a
    // claim about the code rather than about the machine. Now it *asks*: take one sample, and the
    // platform is supported exactly when the answer is usable. That is the same "a capability with
    // a reason" stance the rest of this file already takes — and it is what makes the Windows and
    // macOS arms safe to ship from a machine that cannot compile them. If either is wrong, the
    // sample fails or returns something implausible, and this refuses with a reason. The bad
    // outcome — a believable-but-wrong number letting a model onto a machine with no room — is
    // caught by `plausible()` inside the monitor, which *is* compiled and tested here.
    if let Some(why) = fm_agent_run::host::unsupported() {
        return Some(why);
    }
    // The supervisor itself, which ships in the archive beside `fm-serve`. Everything else — the
    // model, the runtime — is fetched on first enable, so its absence is not a reason to refuse.
    if agent_serve_path().is_none() {
        return Some(
            "This copy of formicaria did not come with the assistant, so it cannot be turned on. \
             Everything else works normally — your notes, search, boards and backup are unaffected."
                .into(),
        );
    }
    // A catalogue is needed to know *what* to fetch. Beside the binary in a release, in `agents/`
    // in a checkout.
    if manifest_path().is_none() {
        return Some(
            "The assistant cannot find its list of models on this machine, so it does not know \
             what to download. Everything else works normally."
                .into(),
        );
    }
    // **No `bash` or `python3` check any more.** They were here because the launch shelled out to
    // two scripts; it no longer does, and a capability check for a tool nothing runs is a refusal
    // with no cause. `on_path`'s Windows arm could not check `.exe`/`PATHEXT` anyway, so deleting
    // the check is better than fixing it.
    None
}

/// **Is the speech-to-text runtime actually here?** — the "Audio transcription" toggle's capability.
///
/// Without this the toggle stored a preference, answered `{"ok":true}`, and changed nothing: whisper
/// is chosen by `agent-serve.sh`, which turns it on *only* when both these files are present, so a
/// user could tick the box, restart, and find no transcription and no explanation anywhere.
///
/// **The condition is a copy of the one in `agents/agent-serve.sh`** — the same two paths — because
/// a shell script cannot export a predicate. `ci/checks.sh` asserts the two stay in step.
pub fn transcribe_available() -> bool {
    tools().is_some_and(|t| t.speech_staged())
}

/// Could audio transcription be provisioned here, if the user asked for it?
///
/// Distinct from [`transcribe_available`], which asks whether it is already here. The switch needs
/// both: *here* means turn it on, *fetchable* means offer to fetch it, and neither means say why.
pub fn transcribe_fetchable() -> bool {
    read_manifest().is_some_and(|m| fm_agent_run::host::Tools::speech_fetchable(&m))
}

/// `agent-serve` beside the running binary — the same idiom as `fm_core::git::merge_command`, and
/// for the same reason: never a bare name hoping `PATH` will answer.
pub fn agent_serve_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bin = exe.parent()?.join(if cfg!(windows) { "agent-serve.exe" } else { "agent-serve" });
    bin.exists().then_some(bin)
}

/// `<config>/formicaria/agent.json` — the per-device on/off setting, beside `vaults.json`.
fn config_path() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("agent.json"))
}

/// Everything in `agent.json`, or an empty object. One reader, so every setting is read the same
/// way and a key this version does not know survives being written back.
fn settings() -> serde_json::Value {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}))
}

/// Write some keys of `agent.json`, **leaving the others as they are** — so the switch, the audio
/// sub-switch and the remembered model choice never clobber one another.
fn write_keys(pairs: &[(&str, serde_json::Value)]) -> Result<(), String> {
    let path = config_path().ok_or("no config directory on this OS")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut all = settings();
    for (k, v) in pairs {
        all[*k] = v.clone();
    }
    std::fs::write(&path, format!("{all}\n")).map_err(|e| e.to_string())
}

/// Persist both switches together. `agent.json` holds `{"enabled": .., "transcribe": ..}` plus the
/// model choice below; the settings screen writes it via `/api/set_agent` and `/api/set_transcribe`.
fn write_settings(enabled: bool, transcribe: bool) -> Result<(), String> {
    write_keys(&[("enabled", enabled.into()), ("transcribe", transcribe.into())])
}

/// The model this person picked or accepted, and the catalogue default at that moment (or when
/// they last said "not now"). See Myrme's `host::Tools::installed` for what each is for.
fn choice() -> (Option<String>, Option<String>) {
    let all = settings();
    let get = |k: &str| all[k].as_str().map(str::to_string);
    (get("model"), get("default_seen"))
}

/// This device's name for its Myrme (`agent.json` `device`), or `laptop` (2026-10-18). Read when the
/// assistant starts; the app stamps the messages written here with the same word.
fn device_name() -> String {
    settings()["device"]
        .as_str()
        .and_then(fm_agent_run::fm_agent::convo::device_word)
        .unwrap_or_else(|| "laptop".into())
}

fn remember_choice(model: Option<&str>, default_seen: &str) -> Result<(), String> {
    let mut pairs = vec![("default_seen", serde_json::Value::from(default_seen))];
    if let Some(m) = model {
        pairs.push(("model", m.into()));
    }
    write_keys(&pairs)
}

/// The catalogue, where the models are kept, and what Myrme's `Tools::installed` makes of them. `None` when this machine has no catalogue to read.
fn installed_now(
) -> Option<(fm_agent_run::manifest::Manifest, PathBuf, fm_agent_run::installed::Installed)> {
    let tools = tools()?;
    let manifest = fm_agent_run::manifest::Manifest::read(&manifest_path()?).ok()?;
    let (chosen, seen) = choice();
    let found = tools.installed(&manifest, chosen.as_deref(), seen.as_deref());
    Some((manifest, tools.dir().to_path_buf(), found))
}

fn set_enabled(enabled: bool) -> Result<(), String> {
    write_settings(enabled, transcribe_enabled())
}

fn set_transcribe(transcribe: bool) -> Result<(), String> {
    write_settings(enabled(), transcribe)
}

/// How long after launch to wait before warming the agent's model. See `spawn_at_launch` for why.
const AGENT_WARMUP_DELAY_SECS: u64 = 5;

/// At launch: start the agent if the setting is on and the script is present.
///
/// **Deferred on purpose.** The agent stack reads a multi-GB model off disk and loads it onto the
/// GPU the instant it starts. Doing that the moment the browser is told to open starves the very
/// disk and GPU the browser needs to paint the app's first frame — so on a laptop the window crawls
/// in *even though fm-serve was listening within ~30 ms* (measured). We claim the run slot **now**
/// (so a concurrent settings toggle can't also spawn) but wait a few seconds before the actual
/// load, giving the page an uncontended window to render; the assistant then warms in the
/// background, a beat after the app is already usable. If the deferred spawn fails, we release the
/// slot so the settings toggle can retry.
pub fn spawn_at_launch(state: Arc<AppState>) {
    if !enabled() || state.agent.running.swap(true, Ordering::Relaxed) {
        return;
    }
    let port = state.agent.port;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(AGENT_WARMUP_DELAY_SECS));
        refresh_runtime_if_outdated(&state.agent);
        if !spawn(port) {
            state.agent.running.store(false, Ordering::Relaxed);
        } else if transcribe_enabled() && whisper_wanted_here() && transcribe_fetchable() {
            // The earlier speech weights run meanwhile; the chosen ones apply at the next start.
            provision_transcribe(&state.agent);
        }
    });
}

/// **An app update brings the runtime its catalogue pins** (`decisions.md#agent`, 2026-10-18). A
/// runtime on disk was never fetched again, so a faster or fixed one never reached anyone who
/// already had the assistant. Myrme knows whether the runtime here is older than the pinned one,
/// and swaps it only once the new archive is fetched and checked; here it runs before the assistant
/// starts (the files are not in use), and shows in the same progress field as a first download.
/// A failure is reported and the assistant starts on the runtime it had — never on none.
fn refresh_runtime_if_outdated(agent: &Arc<AgentState>) {
    let (Some(tools), Some(manifest)) = (tools(), read_manifest()) else { return };
    if !tools.runtime_outdated(&manifest) {
        return;
    }
    let gen = agent.generation.load(Ordering::SeqCst);
    let live = || agent.generation.load(Ordering::SeqCst) == gen;
    let show = |stage: &str, done: u64, total: Option<u64>, error: Option<String>| {
        if live() {
            *agent.provisioning.lock().unwrap() =
                Some(Provision { stage: stage.into(), done, total, error });
        }
    };
    let cancelled = || !live();
    let progress = fm_agent_run::host::Progress {
        step: &|stage| {
            show(stage.word(), 0, None, None);
            live()
        },
        bytes: &|stage, done, total| show(stage.word(), done, total, None),
        cancelled: &cancelled,
    };
    println!("study agent: updating the model runtime to the one this release pins");
    match tools.refresh_runtime(&manifest, &progress) {
        Ok(()) => show("ready", 0, None, None),
        Err(e) => {
            eprintln!(
                "study agent: the runtime could not be updated ({e}); starting on the one here"
            );
            show("ready", 0, None, None);
        }
    }
}

/// Handle the `/api/agent_*` transport routes. `None` means "not an agent route" — the caller falls
/// through to the normal command dispatch. This is the ONE place the agent touches the request path;
/// without the feature it does not exist, and the routes simply aren't there.
pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    state: &AppState,
) -> Option<std::io::Result<()>> {
    let r = &state.agent.registry;
    let resp: (&str, &str, Vec<u8>) = match path {
        // The on/off setting the settings screen reads/writes. A *launcher* concern, deliberately not
        // a dispatch command, so the shared core never learns the agent exists.
        // `installed` is the capability; `enabled` is only the stored preference. The UI needs both
        // so it can offer a switch when there is something to switch on, and say what is missing
        // when there is not.
        "/api/agent_status" => {
            let why = unavailable();
            (
            "200 OK",
            "application/json",
            serde_json::json!({
                "enabled": enabled(),
                // Which assistant this build links (Myrme's release and contract), so a person
                // or a bug report can say which one answered.
                "myrme": fm_agent_run::VERSION,
                "myrme_contract": format!("{}.{}", fm_agent_run::CONTRACT_MAJOR, fm_agent_run::CONTRACT_MINOR),
                "transcribe": transcribe_enabled(),
                // Better listening (2026-10-18): chosen or not, and what choosing it downloads.
                "listening_better": listening_better(),
                "listening_better_bytes": read_manifest().and_then(|m| {
                    let name = m.whisper_for(true)?;
                    (name != m.whisper_for(false)?).then(|| m.model(&name)?.bytes).flatten()
                }),
                // `installed` is the capability; `enabled` is only the stored preference; `why`
                // is what the row prints when the capability is absent, because "not available"
                // with no cause is a dead end.
                "installed": why.is_none(),
                "why": why.unwrap_or_default(),
                // The sub-toggle has a capability of its own — the runtime is a separate download.
                "transcribe_available": transcribe_available(),
                // Not here yet, but downloadable — the row offers instead of hiding. Where this is
                // false *and* `transcribe_available` is false, upstream has no build and the user
                // has nothing to do about it.
                "transcribe_fetchable": transcribe_fetchable(),
                // Whether the model and runtime are already here. `installed` says the app *can*
                // provision; this says whether it still has to — the difference between "turn it
                // on" and "download 2.5 GB and then turn it on", which a person deserves to know
                // before clicking rather than after.
                "provisioned": provisioned(),
                // Which model runs here, and — when an update moved the catalogue's default — the
                // newer one on offer with what it costs to download. `unsupported` means the
                // weights on disk are no longer listed, so the offer is the only way forward.
                "model": installed_now().and_then(|(_, _, f)| f.running),
                // This device's name for its Myrme — what its messages are stamped with.
                "device": device_name(),
                "update": installed_now().and_then(|(_, _, f)| f.offer).map(|o| serde_json::json!({
                    "name": o.name, "bytes": o.bytes, "license": o.license, "replaces": o.replaces,
                })),
                "unsupported": installed_now().is_some_and(|(_, _, f)| f.unsupported),
                // What removing it would free, so the control can say so rather than ask for faith.
                "provisioned_bytes": tools().map(|t| t.bytes()).unwrap_or(0),
                // A download in flight, or how the last one ended. Null when nothing is happening.
                "provisioning": *state.agent.provisioning.lock().unwrap(),
            })
            .to_string()
            .into_bytes(),
            )
        }
        // What the first-enable screen offers: every catalogued model, what it costs, and under
        // what licence — so the question can be asked *before* the download rather than explained
        // after it. Read-only and cheap: one small file, no network.
        "/api/agent_models" => {
            let models = manifest_path()
                .and_then(|p| fm_agent_run::manifest::Manifest::read(&p).ok())
                .map(|m| {
                    let default = m.default.clone();
                    let list: Vec<_> = m
                        .names()
                        .into_iter()
                        .filter_map(|n| {
                            let model = m.model(&n)?;
                            // Whisper weights are catalogued here too, and are not something to
                            // offer as "the assistant" — they arrive with audio transcription.
                            if model.repo.contains("whisper") {
                                return None;
                            }
                            Some(serde_json::json!({
                                "name": n,
                                "bytes": model.bytes,
                                "license": model.license,
                                "vision": model.mmproj.is_some(),
                                "mmproj_bytes": model.mmproj_bytes,
                                "default": n == default,
                            }))
                        })
                        .collect();
                    list
                })
                .unwrap_or_default();
            ("200 OK", "application/json", serde_json::json!(models).to_string().into_bytes())
        }
        // Free the disk the model and runtime are using. Off first: deleting the files under a
        // running model would leave it serving from unlinked inodes and fail confusingly later.
        "/api/remove_agent_model" => {
            let _ = set_enabled(false);
            state.agent.generation.fetch_add(1, Ordering::SeqCst);
            *state.agent.provisioning.lock().unwrap() = None;
            match remove_provisioned() {
                Ok(freed) => (
                    "200 OK",
                    "application/json",
                    serde_json::json!({ "freed": freed }).to_string().into_bytes(),
                ),
                Err(e) => ("409 Conflict", "text/plain; charset=utf-8", e.into_bytes()),
            }
        }
        // **Name this device's Myrme** (2026-10-18): `@Myrme-<name>` then asks this one, and the
        // messages written here are stamped with it. Takes effect when the assistant next starts.
        "/api/set_assistant_name" => {
            let typed = serde_json::from_slice::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v["name"].as_str().map(str::to_owned))
                .unwrap_or_default();
            let word = fm_agent_run::fm_agent::convo::device_word(&typed)
                .unwrap_or_else(|| "laptop".into());
            return Some(match write_keys(&[("device", word.clone().into())]) {
                Ok(()) => crate::write_response(
                    stream,
                    "200 OK",
                    "application/json",
                    serde_json::json!({ "device": word }).to_string().as_bytes(),
                ),
                Err(e) => crate::write_response(
                    stream,
                    "500 Internal Server Error",
                    "text/plain",
                    e.as_bytes(),
                ),
            });
        }
        "/api/set_agent" => {
            let want = serde_json::from_slice::<EnabledReq>(body).unwrap_or_default().enabled;
            // **Refuse rather than report success.** Turning this on when the stack is not here
            // stored the preference, answered `{"ok":true}`, and silently did nothing — the one
            // place this app told a user something worked when it had not.
            if let (true, Some(why)) = (want, unavailable()) {
                return Some(crate::write_response(
                    stream,
                    "409 Conflict",
                    "text/plain; charset=utf-8",
                    why.as_bytes(),
                ));
            }
            let req = serde_json::from_slice::<EnabledReq>(body).unwrap_or_default();
            match set_enabled(want) {
                // Turning it ON provisions first when anything is missing — the model and the
                // runtime are downloads, and this is the only place that knows they are wanted.
                // Already provisioned? Start immediately, as before. OFF bumps the generation, so a
                // download in flight is abandoned rather than left running invisibly.
                Ok(()) => {
                    if want {
                        if provisioned() {
                            if !state.agent.running.swap(true, Ordering::Relaxed)
                                && !spawn(state.agent.port)
                            {
                                state.agent.running.store(false, Ordering::Relaxed);
                            }
                        } else {
                            provision_and_spawn(&state.agent, req.model.clone(), req.vision);
                        }
                    } else {
                        state.agent.generation.fetch_add(1, Ordering::SeqCst);
                        *state.agent.provisioning.lock().unwrap() = None;
                    }
                    ("200 OK", "application/json", b"{\"ok\":true}".to_vec())
                }
                Err(e) => ("500 Internal Server Error", "text/plain", e.into_bytes()),
            }
        }
        // **Accept, or decline, the newer model an update offered.** Accepting downloads it beside
        // the one that is running and records the choice; it takes over at the next launch, like
        // every other change to the assistant. Declining records which default was declined, so the
        // offer is not repeated until the default moves again. Nothing is ever fetched unasked.
        "/api/update_agent_model" => {
            let req = serde_json::from_slice::<UpdateModelReq>(body).unwrap_or_default();
            if req.cancel {
                // Retire the download (it leaves its partial file, so a later accept resumes).
                state.agent.generation.fetch_add(1, Ordering::SeqCst);
                *state.agent.provisioning.lock().unwrap() = None;
                return Some(crate::write_response(
                    stream,
                    "200 OK",
                    "application/json",
                    b"{\"ok\":true}",
                ));
            }
            match installed_now().and_then(|(m, _, f)| f.offer.map(|o| (m, o))) {
                None => (
                    "409 Conflict",
                    "text/plain; charset=utf-8",
                    b"There is no newer assistant model to download.".to_vec(),
                ),
                Some((manifest, _)) if req.dismiss => {
                    match remember_choice(None, &manifest.default) {
                        Ok(()) => ("200 OK", "application/json", b"{\"ok\":true}".to_vec()),
                        Err(e) => ("500 Internal Server Error", "text/plain", e.into_bytes()),
                    }
                }
                Some((manifest, offer)) => {
                    // The projector comes with it: it is part of the size the offer stated, and a
                    // model picked as the default for what it can see is not worth half of.
                    let sees = manifest.model(&offer.name).is_some_and(|m| m.mmproj.is_some());
                    provision(&state.agent, Some(offer.name), sees, After::NextLaunch);
                    ("200 OK", "application/json", b"{\"ok\":true}".to_vec())
                }
            }
        }
        // The "Audio transcription" toggle. Persist only — whisper is chosen when the agent *starts*
        // (it is a launch flag on a separate process), so a change applies the next time the assistant
        // starts, exactly like turning the assistant off applies on the next app close. The UI says so.
        // **Dictation** (2026-10-18): a recording in, the words out, for the note's editor to put
        // at the caret — the person's own words by their own action, so no proposal. The audio is
        // passed by value to the speech server the assistant already runs here and kept nowhere.
        "/api/dictate" => {
            use fm_agent_run::fm_agent::transcribe::{
                dictation_text, Transcribe, WhisperServer, DICTATE_NOT_YET, DICTATE_OFF,
            };
            if !transcribe_enabled() {
                return Some(crate::write_response(
                    stream,
                    "409 Conflict",
                    "text/plain; charset=utf-8",
                    DICTATE_OFF.as_bytes(),
                ));
            }
            if body.is_empty() {
                return Some(crate::write_response(
                    stream,
                    "400 Bad Request",
                    "text/plain; charset=utf-8",
                    b"Nothing was recorded.",
                ));
            }
            let heard = WhisperServer::local(fm_agent_run::host::WHISPER_PORT, "dictation")
                .with_timeout(std::time::Duration::from_secs(600))
                .transcribe(body, "audio/wav");
            match heard {
                Ok(raw) => (
                    "200 OK",
                    "application/json",
                    serde_json::json!({ "text": dictation_text(&raw) }).to_string().into_bytes(),
                ),
                Err(e) => {
                    eprintln!("dictation: {e}");
                    (
                        "503 Service Unavailable",
                        "text/plain; charset=utf-8",
                        DICTATE_NOT_YET.as_bytes().to_vec(),
                    )
                }
            }
        }
        // Better or everyday listening. Recorded, and when transcription is on the chosen weights
        // are fetched now; they apply when the assistant next starts, like the switch above.
        "/api/set_listening" => {
            let better = serde_json::from_slice::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v["better"].as_bool())
                .unwrap_or(false);
            let word = if better { "better" } else { "everyday" };
            match write_keys(&[("listening", serde_json::Value::from(word))]) {
                Ok(()) => {
                    if transcribe_enabled() && whisper_wanted_here() && transcribe_fetchable() {
                        provision_transcribe(&state.agent);
                    }
                    ("200 OK", "application/json", b"{\"ok\":true}".to_vec())
                }
                Err(e) => ("500 Internal Server Error", "text/plain", e.into_bytes()),
            }
        }
        "/api/set_transcribe" => {
            let want = serde_json::from_slice::<TranscribeReq>(body).unwrap_or_default().transcribe;
            // **Turning it on fetches it, if it can be fetched here.** This used to refuse
            // whenever the runtime was absent, which was honest but left the user with a switch
            // they could never use unless they had a checkout. Now: already here → just record the
            // preference; fetchable → download it the way the model is downloaded, with the same
            // progress and cancel; and only where upstream publishes no build at all does it still
            // refuse — because that is the one case the user cannot resolve.
            if want && !transcribe_available() {
                if transcribe_fetchable() {
                    provision_transcribe(&state.agent);
                    return Some(crate::write_response(
                        stream,
                        "200 OK",
                        "application/json",
                        b"{\"ok\":true}",
                    ));
                }
                return Some(crate::write_response(
                    stream,
                    "409 Conflict",
                    "text/plain; charset=utf-8",
                    b"There is no speech-to-text runtime published for this kind of computer yet, \
                      so audio transcription cannot be turned on here. The assistant works \
                      normally without it.",
                ));
            }
            match set_transcribe(want) {
                Ok(()) => ("200 OK", "application/json", b"{\"ok\":true}".to_vec()),
                Err(e) => ("500 Internal Server Error", "text/plain", e.into_bytes()),
            }
        }
        // The live pipeline status the discussion view's turning wheel reads; the agent writes it.
        "/api/agent_activity" => {
            let req: ActivityReq = serde_json::from_slice(body).unwrap_or_default();
            if !req.discussion.is_empty() {
                if req.done {
                    r.clear_activity(&req.discussion);
                } else {
                    let stage = if req.stage.is_empty() { "working" } else { &req.stage };
                    r.set_activity(&req.discussion, stage, &req.question);
                }
            }
            ("200 OK", "application/json", b"{\"ok\":true}".to_vec())
        }
        "/api/agent_activity_poll" => {
            let req: DiscReq = serde_json::from_slice(body).unwrap_or_default();
            let payload = match r.activity(&req.discussion) {
                Some(a) => serde_json::json!({
                    "active": true, "stage": a.stage, "question": a.question, "elapsed_secs": a.elapsed_secs,
                }),
                None => serde_json::json!({ "active": false }),
            };
            ("200 OK", "application/json", payload.to_string().into_bytes())
        }
        // Presence: the agent heartbeats its name; the app asks who is online (the @-picker + warning).
        "/api/agent_present" => {
            let req: PresentReq = serde_json::from_slice(body).unwrap_or_default();
            if !req.name.is_empty() {
                r.heartbeat(&req.name);
                r.set_reader(req.reader.map(|x| fm_agent_run::ImageReader {
                    port: x.port,
                    model: x.model,
                    prompt: None,
                    labels: x.labels,
                }));
            }
            ("200 OK", "application/json", b"{\"ok\":true}".to_vec())
        }
        // Whether a label can be read here, asked before anybody takes a picture.
        "/api/label_reader" => (
            "200 OK",
            "application/json",
            fm_agent_run::label::ready_json(r.reader().as_ref()).to_string().into_bytes(),
        ),
        // A packet's label, read off a photograph (`fm_agent_run::label`). The picture comes by
        // value, goes to the reader the assistant named, and is kept nowhere; what comes back is
        // text for the page to read figures out of, and nothing is written.
        "/api/read_label" => {
            let req: LabelReq = serde_json::from_slice(body).unwrap_or_default();
            let read = match fm_app::wire::b64_decode(&req.data) {
                None => Err("The picture did not arrive whole. Take it again.".to_string()),
                Some(image) => fm_agent_run::label::read(r.reader().as_ref(), &image),
            };
            match read {
                Ok(got) => (
                    "200 OK",
                    "application/json",
                    serde_json::json!({ "text": got.text, "model": got.model })
                        .to_string()
                        .into_bytes(),
                ),
                Err(e) => ("400 Bad Request", "text/plain", e.into_bytes()),
            }
        }
        "/api/agents" => {
            let payload = serde_json::json!({ "agents": r.online() });
            ("200 OK", "application/json", payload.to_string().into_bytes())
        }
        _ => return None,
    };
    Some(write_response(stream, resp.0, resp.1, &resp.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway directory, matching the style of the other file-touching tests here.
    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fm-agent-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A shipped catalogue replaces the copy in the tools directory when it differs, and keeps the
    /// old one. This is what lets an app update change the assistant's model or its settings: the
    /// copy used to be made once and read for ever.
    #[test]
    fn an_updated_app_brings_its_catalogue_and_keeps_the_old_one() {
        let dir = scratch("catalogue-refresh");
        std::fs::create_dir_all(&dir).unwrap();
        let (shipped, in_tools) = (dir.join("shipped.toml"), dir.join("models.toml"));
        std::fs::write(&shipped, b"default = \"new\"").unwrap();

        refresh_catalogue(&shipped, &in_tools);
        assert!(!in_tools.exists(), "nothing to refresh before first use copies it in");

        std::fs::write(&in_tools, b"default = \"old\"").unwrap();
        refresh_catalogue(&shipped, &in_tools);
        assert_eq!(std::fs::read(&in_tools).unwrap(), b"default = \"new\"");
        assert_eq!(
            std::fs::read(dir.join("models.toml.previous")).unwrap(),
            b"default = \"old\"",
            "the catalogue it replaced is kept beside it"
        );

        // Same bytes: nothing is rewritten, and the kept copy is not overwritten with itself.
        std::fs::write(dir.join("models.toml.previous"), b"marker").unwrap();
        refresh_catalogue(&shipped, &in_tools);
        assert_eq!(std::fs::read(dir.join("models.toml.previous")).unwrap(), b"marker");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A directory is a checkout's `agents/` because it carries the catalogue — never merely
    /// because something of that name exists. An empty `agents/` beside a released binary would
    /// otherwise shadow the real tools directory and the assistant would look uninstalled.
    #[test]
    fn an_empty_agents_directory_does_not_masquerade_as_a_checkout() {
        let dir = scratch("checkout");
        assert!(!looks_like_a_checkout(&dir), "an empty directory is not a checkout");
        std::fs::write(dir.join("models.toml"), b"default = \"x\"").unwrap();
        assert!(looks_like_a_checkout(&dir), "the catalogue is the tell");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The gate is a **reading of this machine**, not a list of operating systems.
    ///
    /// **This is the cross-platform test.** On Linux it guards a regression. Run on macOS or
    /// Windows — which is what `cross.yml` does on real runners — it is the first and only thing
    /// that actually *executes* `GlobalMemoryStatusEx` and the mach VM statistics: the arms that
    /// were written on a machine that cannot compile them. A wrong struct layout shows up here as
    /// a refusal or an absurd number, on the platform itself, in CI output.
    ///
    /// So it asserts the **value**, not merely that the call returned. `plausible()`'s own bounds
    /// are deliberately wide (anything under 1 PiB), because its job is to catch nonsense without
    /// second-guessing a machine's size. A test knows more: any computer running this suite has
    /// more than 128 MB free and less than 1 TB, and a reading outside that is a misread field
    /// however plausible it looked to the guard.
    #[test]
    fn the_memory_reading_this_platform_gives_is_usable() {
        let sample = fm_agent_run::fm_agent::watchdog::ResourceMonitor::sample(
            &fm_agent_run::fm_agent::watchdog::SystemMonitor,
        )
        .expect("this machine's memory must be readable — that is the whole platform gate");

        let mb = sample.mem_available_bytes / 1_000_000;
        // Printed so a CI run on macOS or Windows *shows the number*, not just a green tick: the
        // first evidence anyone will have that those arms read the right field.
        println!("available memory as this platform reports it: {mb} MB");
        assert!(
            (128..1_000_000).contains(&mb),
            "{mb} MB is not a believable amount of free memory — the field is probably misread"
        );
    }

    /// The supervisor is looked for **beside the running binary**, never on `PATH` — the
    /// `merge_command` idiom. Under `cargo test` the running binary is a test harness in
    /// `target/debug/deps`, so there is no `agent-serve` beside it and this must answer `None`
    /// rather than finding some other copy.
    #[test]
    fn the_supervisor_is_resolved_beside_the_binary_or_not_at_all() {
        if let Some(p) = agent_serve_path() {
            let exe = std::env::current_exe().unwrap();
            assert_eq!(p.parent(), exe.parent(), "only ever beside the running binary");
        }
    }

    /// A capability must mean "this will work". On a machine that can run it, `unavailable` must
    /// not be inventing a reason; on one that cannot, the reason must be printable — an empty
    /// string in the settings row is the dead end this replaced.
    #[test]
    fn the_reason_is_something_a_reader_can_act_on() {
        if let Some(why) = unavailable() {
            assert!(why.len() > 20, "not a reason a user could act on: {why:?}");
        }
    }
}
