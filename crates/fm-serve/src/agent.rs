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
}

impl AgentState {
    pub fn new(port: u16) -> Self {
        Self { registry: AgentRegistry::new(), running: AtomicBool::new(false), port }
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

/// Spawn the local study agent — the opt-in script in `agents/`, told which fm-serve port to watch.
/// Returns whether it started. The agent (a separate process) then follows this server's liveness and
/// stops itself when we stop answering, so it needs no supervision from here.
pub fn spawn(port: u16) -> bool {
    if let Some(why) = unavailable() {
        eprintln!("study agent: enabled, but it cannot run here — {why}");
        return false;
    }
    let Some(script) = script_path() else { return false };
    let mut cmd = std::process::Command::new("bash");
    cmd.arg(&script).arg(port.to_string());
    // The agent stack inherits this; agent-serve.sh turns on whisper only when it is set.
    if transcribe_enabled() {
        cmd.env("FM_TRANSCRIBE", "1");
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

/// Where the agent stack is, if it is on this machine at all.
///
/// **Resolved beside the running binary first, not against the working directory.** This was a bare
/// relative `agents/start-agent.sh`, which resolves against whatever cwd the process was handed —
/// `$HOME` for a double-clicked launcher — so a released build could never find the stack even if it
/// had been shipped. Same reasoning, and the same fix, as `fm_core::git`'s merge-driver lookup:
/// *never a bare relative path hoping the cwd will answer.*
///
/// The checkout root stays a fallback, because that is where the dev loop runs from.
pub fn script_path() -> Option<PathBuf> {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("agents").join("start-agent.sh")));
    beside
        .into_iter()
        .chain(std::iter::once(PathBuf::from("agents/start-agent.sh")))
        .find(|p| p.exists())
}

/// The agent stack's directory — `runtime/`, `models/` and the scripts live under it.
fn agents_dir() -> Option<PathBuf> {
    script_path().and_then(|p| p.parent().map(PathBuf::from))
}

/// Is `program` an executable file on `PATH`?
///
/// The mode bit matters: a *readable* `bash` that cannot be executed fails at `spawn` exactly like
/// an absent one, and a capability check that stops one step short of the thing it is predicting is
/// how this row came to say "yes" about a machine that answers "no".
fn on_path(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    std::env::split_paths(&path).any(|dir| executable(&dir.join(program)))
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
    if !cfg!(any(target_os = "linux", target_os = "android")) {
        return Some(
            "The study assistant runs on Linux today. Everything else in formicaria works normally \
             here — your notes, search, boards and backup are unaffected."
                .into(),
        );
    }
    if script_path().is_none() {
        return Some(
            "The assistant is not on this machine yet, so it cannot be turned on. Everything else \
             works normally — your notes, search, boards and backup are unaffected."
                .into(),
        );
    }
    // `start-agent.sh` is bash and runs `search-proxy.py`. Both are shelled out to by name, so a
    // machine without them fails at `spawn` — into a stderr the launcher hides by design.
    let missing: Vec<&str> = ["bash", "python3"].into_iter().filter(|p| !on_path(p)).collect();
    if !missing.is_empty() {
        return Some(format!(
            "The assistant needs {} on this machine, and cannot find {}. Everything else works \
             normally.",
            missing.join(" and "),
            if missing.len() == 1 { "it" } else { "them" },
        ));
    }
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

/// The predicate itself, against a named directory, so it can be tested without a staged runtime.
fn transcribe_staged_in(dir: &std::path::Path) -> bool {
    executable(&dir.join("runtime").join("whisper-server"))
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
pub fn route(stream: &mut dyn crate::Conn, path: &str, body: &[u8], state: &AppState) -> Option<std::io::Result<()>> {
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
            })
            .to_string()
            .into_bytes(),
            )
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
            match set_enabled(want) {
                // Turning it ON starts it immediately (the atomic guard makes sure only one spawns);
                // OFF just persists the setting — the running agent stops when the app closes.
                Ok(()) => {
                    if want && !state.agent.running.swap(true, Ordering::Relaxed) && !spawn(state.agent.port) {
                        state.agent.running.store(false, Ordering::Relaxed);
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
            // **The same refusal `set_agent` makes, for the same reason.** `agent-serve.sh` starts
            // whisper only when its runtime and model are both staged, so without them this stored
            // a preference, answered `{"ok":true}`, and transcription simply never happened.
            if want && !transcribe_available() {
                return Some(crate::write_response(
                    stream,
                    "409 Conflict",
                    "text/plain; charset=utf-8",
                    b"The speech-to-text runtime is not on this machine, so audio transcription \
                      cannot be turned on yet. The assistant works normally without it.",
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

    /// The `PATH` scan predicts a `spawn`, so it has to agree with one. `sh` is on every machine
    /// this crate builds for; the second name is not on any.
    #[test]
    fn a_program_is_on_path_only_if_it_could_actually_be_run() {
        assert!(on_path("sh"));
        assert!(!on_path("fm-no-such-program-anywhere"));
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
