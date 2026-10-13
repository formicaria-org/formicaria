//! The study agent's transient side channel: **presence** (which agents are alive) and **per-discussion
//! activity** (the "working…" wheel's stage + elapsed). In-memory only — never touches the vault,
//! never a `dispatch` command, so the core stays agent-agnostic. Extracted from the HTTP handlers so
//! its TTL rules — *the wheel never spins forever*, and *a missed heartbeat doesn't flip a live agent
//! offline* — are unit-tested in isolation instead of only exercised over a socket.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A turn quiet longer than this means the agent died mid-turn without clearing — expire it so the
/// wheel can never spin forever (the failure mode every LLM tool we surveyed shares).
const ACTIVITY_TTL: Duration = Duration::from_secs(180);
/// Comfortably longer than the agent's ~5 s heartbeat, so a missed beat or two doesn't read as gone.
const PRESENCE_TTL: Duration = Duration::from_secs(20);

struct Activity {
    stage: String,
    question: String,
    /// When this turn began — kept across stage updates so elapsed reflects the whole turn.
    since: Instant,
}

/// One discussion's live status as a poller sees it.
pub struct ActivityView {
    pub stage: String,
    pub question: String,
    pub elapsed_secs: u64,
}

/// The two TTL boards. Cheap to share behind an `Arc`; each board guards itself.
pub struct AgentRegistry {
    activity: Mutex<HashMap<String, Activity>>,
    present: Mutex<HashMap<String, Instant>>,
    /// The model server that can read a picture, as the assistant last said, and when it said so.
    reader: Mutex<Option<(fm_agent_run::ImageReader, Instant)>>,
    activity_ttl: Duration,
    presence_ttl: Duration,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self {
            activity: Mutex::new(HashMap::new()),
            present: Mutex::new(HashMap::new()),
            reader: Mutex::new(None),
            activity_ttl: ACTIVITY_TTL,
            presence_ttl: PRESENCE_TTL,
        }
    }
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// The agent reports the stage it is on for a discussion (`reading your notes`, `thinking`, …).
    pub fn set_activity(&self, disc: &str, stage: &str, question: &str) {
        let mut act = self.activity.lock().unwrap();
        let since = act.get(disc).map(|a| a.since).unwrap_or_else(Instant::now);
        act.insert(
            disc.to_string(),
            Activity { stage: stage.to_string(), question: question.to_string(), since },
        );
    }

    /// The reply landed, or the turn errored/timed out — hide the wheel.
    pub fn clear_activity(&self, disc: &str) {
        self.activity.lock().unwrap().remove(disc);
    }

    /// The current status for a discussion, or `None` when idle (or stale — a dead agent).
    pub fn activity(&self, disc: &str) -> Option<ActivityView> {
        let mut act = self.activity.lock().unwrap();
        if act.get(disc).is_some_and(|a| a.since.elapsed() > self.activity_ttl) {
            act.remove(disc);
        }
        act.get(disc).map(|a| ActivityView {
            stage: a.stage.clone(),
            question: a.question.clone(),
            elapsed_secs: a.since.elapsed().as_secs(),
        })
    }

    /// An agent says it is alive, by `@name`.
    pub fn heartbeat(&self, name: &str) {
        self.present.lock().unwrap().insert(name.to_string(), Instant::now());
    }

    /// The assistant says what can read a picture here, with each heartbeat; `None` when nothing
    /// can. Kept only as long as a heartbeat is: a stopped assistant has no reader.
    pub fn set_reader(&self, reader: Option<fm_agent_run::ImageReader>) {
        *self.reader.lock().unwrap() = reader.map(|r| (r, Instant::now()));
    }

    /// What can read a picture right now, if the assistant has said so recently.
    pub fn reader(&self) -> Option<fm_agent_run::ImageReader> {
        let held = self.reader.lock().unwrap();
        held.as_ref().filter(|(_, seen)| seen.elapsed() < self.presence_ttl).map(|(r, _)| r.clone())
    }

    /// The agents seen within the presence TTL, sorted — the @-picker list.
    pub fn online(&self) -> Vec<String> {
        let mut present = self.present.lock().unwrap();
        present.retain(|_, seen| seen.elapsed() < self.presence_ttl);
        let mut names: Vec<String> = present.keys().cloned().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short() -> AgentRegistry {
        AgentRegistry {
            activity_ttl: Duration::from_millis(15),
            presence_ttl: Duration::from_millis(15),
            ..Default::default()
        }
    }

    #[test]
    fn activity_round_trips_and_clears() {
        let r = AgentRegistry::new();
        assert!(r.activity("d").is_none());
        r.set_activity("d", "thinking", "q?");
        let v = r.activity("d").unwrap();
        assert_eq!(v.stage, "thinking");
        assert_eq!(v.question, "q?");
        r.clear_activity("d");
        assert!(r.activity("d").is_none());
    }

    #[test]
    fn online_lists_sorted() {
        let r = AgentRegistry::new();
        r.heartbeat("b");
        r.heartbeat("a");
        assert_eq!(r.online(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn a_stale_activity_expires_so_the_wheel_never_spins_forever() {
        let r = short();
        r.set_activity("d", "thinking", "q");
        assert!(r.activity("d").is_some());
        std::thread::sleep(Duration::from_millis(35));
        assert!(r.activity("d").is_none());
    }

    #[test]
    fn a_reader_is_believed_only_as_long_as_a_heartbeat() {
        let r = short();
        assert!(r.reader().is_none(), "nobody has said anything can read");
        let it =
            fm_agent_run::ImageReader { port: 8081, model: "m".into(), prompt: None, labels: true };
        r.set_reader(Some(it.clone()));
        assert_eq!(r.reader(), Some(it.clone()));
        std::thread::sleep(Duration::from_millis(35));
        assert!(r.reader().is_none(), "a stopped assistant's reader is not asked");
        r.set_reader(Some(it));
        r.set_reader(None);
        assert!(r.reader().is_none(), "and an assistant that cannot see says so");
    }

    #[test]
    fn a_stale_heartbeat_does_not_keep_an_agent_online() {
        let r = short();
        r.heartbeat("x");
        assert_eq!(r.online(), vec!["x".to_string()]);
        std::thread::sleep(Duration::from_millis(35));
        assert!(r.online().is_empty());
    }
}
