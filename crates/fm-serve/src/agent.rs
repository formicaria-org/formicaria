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
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .map(|v| v["enabled"].as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Whether audio→transcript is on — the "Audio transcription" toggle, stored beside `enabled` in
/// `agent.json` (`{"transcribe": true}`). Read when the agent starts; exported to the agent stack as
/// `FM_TRANSCRIBE=1` so it launches whisper. Default off — no whisper runtime loads unless asked.
pub fn transcribe_enabled() -> bool {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .map(|v| v["transcribe"].as_bool().unwrap_or(false))
        .unwrap_or(false)
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

        let Some(dir) = agents_dir() else {
            set("failed", 0, None, Some("no place to keep the assistant on this machine".into()));
            return;
        };
        if let Err(e) = std::fs::create_dir_all(dir.join("models")) {
            set("failed", 0, None, Some(format!("cannot create {}: {e}", dir.display())));
            return;
        }
        // The catalogue: whatever is already in the tools directory, else the copy shipped beside
        // the binary — copied in on first use, exactly as the phone writes its embedded one.
        let Some(mpath) = manifest_path() else {
            set("failed", 0, None, Some("the assistant has no model catalogue here".into()));
            return;
        };
        if mpath != dir.join("models.toml") {
            let _ = std::fs::copy(&mpath, dir.join("models.toml"));
        }
        let manifest = match fm_agent_run::manifest::Manifest::read(&mpath) {
            Ok(m) => m,
            Err(e) => {
                set("failed", 0, None, Some(e));
                return;
            }
        };
        let name = model.unwrap_or_else(|| manifest.default.clone());

        // 1. The runtime. Skipped where a platform has no verified archive pinned — which is a
        //    capability to report, never a reason to fetch something unverified.
        let progress = |stage: &'static str| {
            let st = Arc::clone(&st);
            move |done: u64, total: Option<u64>| {
                if st.generation.load(Ordering::SeqCst) == gen {
                    *st.provisioning.lock().unwrap() =
                        Some(Provision { stage: stage.into(), done, total, error: None });
                }
            }
        };
        if !dir.join("runtime").join(server_bin()).exists() {
            let Some(key) = fm_agent_run::manifest::Manifest::platform_key() else {
                set(
                    "failed",
                    0,
                    None,
                    Some(
                        "no model runtime has been published for this kind of computer yet".into(),
                    ),
                );
                return;
            };
            let Some(rt) = manifest.runtime(key) else {
                set(
                    "failed",
                    0,
                    None,
                    Some(format!("no verified model runtime is pinned for {key} yet")),
                );
                return;
            };
            if !set("runtime", 0, None, None) {
                return;
            }
            if let Err(e) = fm_agent_run::fetch::ensure_runtime(
                &dir.join("runtime"),
                rt,
                server_bin(),
                &progress("runtime"),
                &cancelled,
            ) {
                if live() {
                    set("failed", 0, None, Some(e));
                }
                return;
            }
        }

        // 2. The weights.
        if !set("model", 0, None, None) {
            return;
        }
        if let Err(e) = fm_agent_run::fetch::ensure_model(
            &dir.join("models"),
            &manifest,
            &name,
            &progress("model"),
            &cancelled,
        ) {
            if live() {
                set("failed", 0, None, Some(e));
            }
            return;
        }

        // 3. The projector, only when asked for: it is another download and buys exactly one
        //    capability. Its absence disables image reading and nothing else.
        if want_vision {
            if !set("projector", 0, None, None) {
                return;
            }
            if let Err(e) = fm_agent_run::fetch::ensure_mmproj(
                &dir.join("models"),
                &manifest,
                &name,
                &progress("projector"),
                &cancelled,
            ) {
                if live() {
                    set("failed", 0, None, Some(e));
                }
                return;
            }
        }

        if !live() {
            return;
        }
        set("ready", 0, None, None);
        spawn(st.port);
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
    agents_dir().is_some_and(|d| provisioned_in(&d))
}

/// The predicate itself, against a named directory, so it is tested without a 2.5 GB download —
/// the same shape as [`transcribe_staged_in`] below and for the same reason.
fn provisioned_in(dir: &std::path::Path) -> bool {
    if !dir.join("runtime").join(server_bin()).exists() {
        return false;
    }
    std::fs::read_dir(dir.join("models")).is_ok_and(|mut d| {
        d.any(|e| {
            e.is_ok_and(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("gguf")))
        })
    })
}

/// How much disk the downloaded model and runtime are using, in bytes.
///
/// Reported so the removal control can say what it will free — "delete 2.5 GB" is a decision
/// someone can make, "delete the model" is a leap of faith.
fn provisioned_bytes_in(dir: &std::path::Path) -> u64 {
    ["models", "runtime"]
        .iter()
        .filter_map(|sub| std::fs::read_dir(dir.join(sub)).ok())
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Delete the downloaded model and runtime, and nothing else.
///
/// **Only inside the tools directory, and only its two subdirectories.** A checkout's `agents/` is
/// the same shape and holds files a developer put there by hand, so this refuses to run against one
/// — `looks_like_a_checkout` is the same tell `agents_dir` uses to prefer it.
///
/// Notes are never touched: they are not in this directory and this function names the only two
/// subdirectories it will remove.
fn remove_provisioned() -> Result<u64, String> {
    let dir = agents_dir().ok_or("no tools directory on this machine")?;
    if looks_like_a_checkout(&dir) {
        return Err(
            "this installation runs from a source checkout, so the model in `agents/` is yours to \
             manage — formicaria will not delete it for you"
                .into(),
        );
    }
    let freed = provisioned_bytes_in(&dir);
    for sub in ["models", "runtime"] {
        let p = dir.join(sub);
        if p.exists() {
            std::fs::remove_dir_all(&p)
                .map_err(|e| format!("could not remove {}: {e}", p.display()))?;
        }
    }
    Ok(freed)
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
        let progress = {
            let st = Arc::clone(&st);
            move |done: u64, total: Option<u64>| {
                if st.generation.load(Ordering::SeqCst) == gen {
                    *st.provisioning.lock().unwrap() =
                        Some(Provision { stage: "transcribe".into(), done, total, error: None });
                }
            }
        };

        let Some(dir) = agents_dir() else {
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
        let Some(rt) = whisper_runtime_key().and_then(|k| manifest.runtime(k)) else {
            set(
                "failed",
                0,
                None,
                Some(
                    "no speech-to-text runtime has been published for this kind of computer yet"
                        .into(),
                ),
            );
            return;
        };

        set("transcribe", 0, None, None);
        if let Err(e) = fm_agent_run::fetch::ensure_runtime(
            &dir.join("runtime"),
            rt,
            whisper_bin(),
            &progress,
            &cancelled,
        ) {
            if live() {
                set("failed", 0, None, Some(e));
            }
            return;
        }
        // The weights are an ordinary catalogued model, fetched exactly as the phone fetches them.
        let name = manifest.whisper_desktop().unwrap_or_else(|| "ggml-base.en".to_string());
        if let Err(e) = fm_agent_run::fetch::ensure_model(
            &dir.join("models"),
            &manifest,
            &name,
            &progress,
            &cancelled,
        ) {
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

/// The model server's filename on this platform.
fn server_bin() -> &'static str {
    if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

/// Spawn the local study agent — the opt-in script in `agents/`, told which fm-serve port to watch.
/// Returns whether it started. The agent (a separate process) then follows this server's liveness and
/// stops itself when we stop answering, so it needs no supervision from here.
pub fn spawn(port: u16) -> bool {
    if let Some(why) = unavailable() {
        eprintln!("study agent: enabled, but it cannot run here — {why}");
        return false;
    }
    let Some(dir) = agents_dir() else { return false };
    let Some(exe) = agent_serve_path() else { return false };

    // **No shell.** This ran `bash agents/start-agent.sh`, which started `search-proxy.py`, trapped
    // it on exit and `tee`d a log — about ten lines of real work between two scripts, wrapping a
    // supervisor that was already Rust. The phone has run this same stack with no shell since it
    // shipped; the desktop now does too, which is what lets it run where `bash` and `python3` are
    // not a given, and what lets it run at all from an unpacked archive.
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("--agents-dir").arg(&dir).arg("--serve-port").arg(port.to_string());
    // In-process HTTPS search, since there is no proxy to start without a shell.
    cmd.arg("--web-direct");

    // The whisper decision `agents/agent-serve.sh` used to make, from the predicate that already
    // existed here for the settings row — so the duplication `ci/checks.sh` polices goes away.
    if transcribe_enabled() && transcribe_staged_in(&dir) {
        cmd.arg("--whisper-port").arg((WHISPER_PORT).to_string());
    }

    // The rolling log `start-agent.sh` kept, minus `tee`: a released app has no terminal to read.
    match log_file(&dir) {
        Some(f) => {
            let Ok(err) = f.try_clone() else { return false };
            cmd.stdout(std::process::Stdio::from(f)).stderr(std::process::Stdio::from(err));
        }
        // No log is not a reason to refuse to start; it is a reason to say so.
        None => eprintln!("study agent: could not open its log — running without one"),
    }

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
    if in_tools.exists() {
        return Some(in_tools);
    }
    let beside = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("models.toml")))
        .filter(|p| p.exists())?;
    Some(beside)
}

#[cfg(unix)]
fn executable(p: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(p: &std::path::Path) -> bool {
    p.is_file()
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
    if let Err(e) = fm_agent_run::fm_agent::watchdog::ResourceMonitor::sample(
        &fm_agent_run::fm_agent::watchdog::SystemMonitor,
    ) {
        return Some(format!(
            "The study assistant cannot run on this computer: {e} Everything else in formicaria \
             works normally — your notes, search, boards and backup are unaffected."
        ));
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
    agents_dir().is_some_and(|d| transcribe_staged_in(&d))
}

/// The port the audio runtime listens on, as `agents/agent-serve.sh` chose it.
const WHISPER_PORT: u16 = 8082;

/// The speech-to-text server's filename on this platform.
fn whisper_bin() -> &'static str {
    if cfg!(windows) {
        "whisper-server.exe"
    } else {
        "whisper-server"
    }
}

/// The catalogue key for this platform's whisper archive, or `None` where upstream publishes none.
///
/// **macOS has no build at whisper.cpp v1.9.1**, which is a different fact from "this machine is
/// missing a file" and has to be reported differently — a user cannot fix an archive that does not
/// exist, and telling them to try would waste their time.
fn whisper_runtime_key() -> Option<&'static str> {
    match fm_agent_run::manifest::Manifest::platform_key()? {
        "linux_x64" => Some("whisper_linux_x64"),
        "windows_x64" => Some("whisper_windows_x64"),
        _ => None,
    }
}

/// Could audio transcription be provisioned here, if the user asked for it?
///
/// Distinct from [`transcribe_available`], which asks whether it is already here. The switch needs
/// both: *here* means turn it on, *fetchable* means offer to fetch it, and neither means say why.
pub fn transcribe_fetchable() -> bool {
    whisper_runtime_key().is_some_and(|k| {
        manifest_path()
            .and_then(|p| fm_agent_run::manifest::Manifest::read(&p).ok())
            .is_some_and(|m| m.runtime(k).is_some())
    })
}

/// `agent-serve` beside the running binary — the same idiom as `fm_core::git::merge_command`, and
/// for the same reason: never a bare name hoping `PATH` will answer.
pub fn agent_serve_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bin = exe.parent()?.join(if cfg!(windows) { "agent-serve.exe" } else { "agent-serve" });
    bin.exists().then_some(bin)
}

/// The agent's rolling log, `<agents-dir>/runtime/agent.log`, rolled at 5 MB exactly as
/// `start-agent.sh` did — an assistant that has been enabled for months should not have written an
/// unbounded file into a user's config directory.
fn log_file(dir: &std::path::Path) -> Option<std::fs::File> {
    let runtime = dir.join("runtime");
    std::fs::create_dir_all(&runtime).ok()?;
    let log = runtime.join("agent.log");
    if std::fs::metadata(&log).is_ok_and(|m| m.len() > 5_000_000) {
        let _ = std::fs::rename(&log, runtime.join("agent.log.1"));
    }
    std::fs::OpenOptions::new().create(true).append(true).open(&log).ok()
}

/// The predicate itself, against a named directory, so it can be tested without a staged runtime.
fn transcribe_staged_in(dir: &std::path::Path) -> bool {
    executable(&dir.join("runtime").join(whisper_bin()))
        && dir.join("models").join("ggml-base.en.bin").is_file()
}

/// `<config>/formicaria/agent.json` — the per-device on/off setting, beside `vaults.json`.
fn config_path() -> Option<PathBuf> {
    fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("agent.json"))
}

/// Persist both settings together, so writing one never clobbers the other. `agent.json` holds
/// `{"enabled": .., "transcribe": ..}`; the settings screen writes it via `/api/set_agent` and
/// `/api/set_transcribe`.
fn write_settings(enabled: bool, transcribe: bool) -> Result<(), String> {
    let path = config_path().ok_or("no config directory on this OS")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, format!("{{\"enabled\": {enabled}, \"transcribe\": {transcribe}}}\n"))
        .map_err(|e| e.to_string())
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
        if !spawn(port) {
            state.agent.running.store(false, Ordering::Relaxed);
        }
    });
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
                "transcribe": transcribe_enabled(),
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
                // What removing it would free, so the control can say so rather than ask for faith.
                "provisioned_bytes": agents_dir().map(|d| provisioned_bytes_in(&d)).unwrap_or(0),
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
        // The "Audio transcription" toggle. Persist only — whisper is chosen when the agent *starts*
        // (it is a launch flag on a separate process), so a change applies the next time the assistant
        // starts, exactly like turning the assistant off applies on the next app close. The UI says so.
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
            }
            ("200 OK", "application/json", b"{\"ok\":true}".to_vec())
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

    /// **The difference between "turn it on" and "download a few gigabytes".** Half a stack is not
    /// a stack: a runtime with no weights, or weights with no runtime, must both read as not
    /// provisioned, or the first enable would skip the question and start a model that is not there.
    #[test]
    fn a_half_provisioned_directory_is_not_provisioned() {
        let dir = scratch("provisioned");
        std::fs::create_dir_all(dir.join("runtime")).unwrap();
        std::fs::create_dir_all(dir.join("models")).unwrap();
        assert!(!provisioned_in(&dir), "an empty pair of directories is nothing at all");

        std::fs::write(dir.join("runtime").join(server_bin()), b"#!/bin/true").unwrap();
        assert!(!provisioned_in(&dir), "a runtime with no weights cannot answer anything");

        std::fs::write(dir.join("models").join("some-model.gguf"), b"weights").unwrap();
        assert!(provisioned_in(&dir), "runtime + weights is the whole condition");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The weights are matched by **extension, not by name**, on purpose: the catalogue's default
    /// can move under an installation that already holds a perfectly good model, and re-downloading
    /// gigabytes because a default changed would be the wrong answer.
    #[test]
    fn any_gguf_counts_as_weights_whatever_the_catalogue_now_prefers() {
        let dir = scratch("provisioned-any");
        std::fs::create_dir_all(dir.join("runtime")).unwrap();
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("runtime").join(server_bin()), b"x").unwrap();
        std::fs::write(dir.join("models").join("a-model-nobody-ships-today.GGUF"), b"w").unwrap();
        assert!(provisioned_in(&dir), "case-insensitive, and not tied to a filename we chose");

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

    /// The toggle used to store a preference, answer `{"ok":true}`, and change nothing: whisper is
    /// started by `agents/agent-serve.sh` only when **both** of these are staged. Neither file on
    /// its own is a capability — half a runtime transcribes nothing.
    #[test]
    fn audio_transcription_needs_the_runtime_and_the_model_together() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("runtime")).unwrap();
        std::fs::create_dir_all(root.join("models")).unwrap();
        assert!(!transcribe_staged_in(root), "nothing staged");

        let server = root.join("runtime").join("whisper-server");
        std::fs::write(&server, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(!transcribe_staged_in(root), "runtime but no model");

        std::fs::write(root.join("models").join("ggml-base.en.bin"), b"x").unwrap();
        assert!(transcribe_staged_in(root), "both staged");
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
