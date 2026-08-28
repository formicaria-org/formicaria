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
    let Some(script) = script_path() else {
        eprintln!("study agent: enabled, but the agent stack is not on this machine — skipping");
        return false;
    };
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

/// **Can this machine actually run the assistant?** — not "is the switch on".
///
/// The distinction is the whole point. `enabled()` reads a stored flag and cannot fail, so the
/// settings row used to offer a toggle that returned `{"ok":true}`, promised *"Starts with
/// formicaria on the next launch"*, and did nothing at all — because the release archive ships no
/// `agents/` directory and the only diagnostic went to a stderr the Windows launcher hides by
/// design. That is precisely the failure `decisions.md` already ruled against: **a capability must
/// mean "this will work", never "this is configured".**
pub fn installed() -> bool {
    script_path().is_some()
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
        "/api/agent_status" => (
            "200 OK",
            "application/json",
            format!(
                "{{\"enabled\": {}, \"transcribe\": {}, \"installed\": {}}}",
                enabled(),
                transcribe_enabled(),
                installed()
            )
            .into_bytes(),
        ),
        "/api/set_agent" => {
            let want = serde_json::from_slice::<EnabledReq>(body).unwrap_or_default().enabled;
            // **Refuse rather than report success.** Turning this on when the stack is not here
            // stored the preference, answered `{"ok":true}`, and silently did nothing — the one
            // place this app told a user something worked when it had not.
            if want && !installed() {
                return Some(crate::write_response(
                    stream,
                    "409 Conflict",
                    "text/plain; charset=utf-8",
                    b"The study assistant is not installed on this machine, so it cannot be turned \
                      on yet. Everything else works as normal.",
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
