//! Reminders said by the program itself, when nobody has a page open
//! (`decisions.md`, 2026-10-13, *the program on a computer stays up while a reminder is waiting,
//! and the assistant does not*).
//!
//! The page hands over the same list it hands Android: a title, a line of words and a moment.
//! Nothing here reads a vault or does date arithmetic. While a page is open the page says its
//! own reminders and this stays quiet.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// As on the phone: fifty at most.
const MOST: usize = 50;
/// A reminder that came due while the machine slept is still said on waking, within this long.
const LATE_MS: u64 = 12 * 60 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub title: String,
    pub text: String,
    /// Milliseconds since 1970, as the page worked it out from this device's own clock.
    pub at: u64,
}

pub struct RemindState {
    items: Mutex<Vec<Item>>,
    file: Option<PathBuf>,
    /// The last page has gone and a reminder is still waiting: the program stays, counts nobody
    /// as present and refuses the assistant.
    pub resting: AtomicBool,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl RemindState {
    pub fn load() -> Self {
        // A test never reads or writes this machine's own list.
        #[cfg(test)]
        return Self::at(None);
        #[cfg(not(test))]
        Self::at(fm_app::vaults::config_dir().map(|d| d.join("formicaria").join("reminders.json")))
    }

    fn at(file: Option<PathBuf>) -> Self {
        let items = file
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str::<Vec<Item>>(&t).ok())
            .unwrap_or_default();
        Self { items: Mutex::new(items), file, resting: AtomicBool::new(false) }
    }

    fn keep(&self, items: &[Item]) {
        let Some(p) = &self.file else { return };
        if let Ok(text) = serde_json::to_string(items) {
            // Titles of the person's notes: theirs alone to read.
            let _ = fm_app::secrets::write_private(p, &text);
        }
    }

    /// Replace the list with what the page just worked out.
    pub fn set(&self, mut items: Vec<Item>) {
        items.sort_by_key(|i| i.at);
        items.truncate(MOST);
        for i in &mut items {
            i.title = i.title.chars().take(200).collect();
            i.text = i.text.chars().take(200).collect();
        }
        let mut held = self.items.lock().unwrap_or_else(|e| e.into_inner());
        if *held != items {
            self.keep(&items);
            *held = items;
        }
    }

    /// Is there anything still to say?
    pub fn waiting(&self) -> bool {
        self.waiting_at(now_ms())
    }

    fn waiting_at(&self, now: u64) -> bool {
        self.items.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|i| i.at > now)
    }

    /// Take what has come due off the list; what is worth saying still is returned.
    fn take_due(&self, now: u64) -> Vec<Item> {
        let mut held = self.items.lock().unwrap_or_else(|e| e.into_inner());
        if !held.iter().any(|i| i.at <= now) {
            return Vec::new();
        }
        let (due, rest): (Vec<Item>, Vec<Item>) = held.drain(..).partition(|i| i.at <= now);
        self.keep(&rest);
        *held = rest;
        due.into_iter().filter(|i| now - i.at <= LATE_MS).collect()
    }
}

/// Say what comes due while no page is open. A page that is open says its own.
pub fn run_in_background(state: Arc<crate::AppState>) {
    std::thread::spawn(move || {
        // Long enough for a page opened at the start to have arrived, so yesterday's list is
        // not said over the top of it.
        std::thread::sleep(Duration::from_secs(20));
        loop {
            std::thread::sleep(Duration::from_secs(5));
            let due = state.remind.take_due(now_ms());
            let alone = state.remind.resting.load(Ordering::Relaxed)
                || state.tabs.lock().map(|t| t.is_empty()).unwrap_or(false);
            if alone {
                for i in due {
                    notify(&i.title, &i.text);
                }
            }
            // The last one has been said: nothing is left to stay for.
            if state.remind.resting.load(Ordering::Relaxed) && !state.remind.waiting() {
                std::process::exit(0);
            }
        }
    });
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Req {
    items: Vec<Item>,
}

/// `/api/remind_set` and `/api/rest_end`. Both are in `REMOTE_DENIED`: they act on this machine.
pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    state: &Arc<crate::AppState>,
) -> Option<std::io::Result<()>> {
    match path {
        "/api/remind_set" => {
            let req: Req = serde_json::from_slice(body).unwrap_or_default();
            state.remind.set(req.items);
            Some(crate::write_response(stream, "200 OK", "application/json", b"null"))
        }
        // A newer copy asking a resting one to make way. Honoured only while resting: a program
        // somebody is using is never stopped by a request.
        "/api/rest_end" => {
            if !state.remind.resting.load(Ordering::Relaxed) {
                return Some(crate::write_response(
                    stream,
                    "409 Conflict",
                    "text/plain",
                    b"in use",
                ));
            }
            let out = crate::write_response(stream, "200 OK", "text/plain", b"leaving");
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(150));
                std::process::exit(0);
            });
            Some(out)
        }
        _ => None,
    }
}

/// Ask whatever of ours holds `addr` to leave if it is only resting. True when it said it would,
/// and the port has come free.
pub fn ask_to_leave(addr: &str) -> bool {
    use std::io::{Read, Write};
    let Ok(mut sock) = std::net::TcpStream::connect(addr) else { return false };
    let t = Some(Duration::from_millis(800));
    let _ = sock.set_read_timeout(t);
    let _ = sock.set_write_timeout(t);
    let req = format!(
        "POST /api/rest_end HTTP/1.1\r\nHost: {addr}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    if sock.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = Vec::new();
    let _ = sock.take(4096).read_to_end(&mut buf);
    if !buf.starts_with(b"HTTP/1.1 200") {
        return false;
    }
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        if std::net::TcpStream::connect(addr).is_err() {
            return true;
        }
    }
    false
}

/// The system's own notification, through a program each system already has. No window, no
/// shell, and the words travel as arguments or environment, never inside a script's text.
/// Says nothing where there is no such program: a reminder is not worth failing over.
fn notify(title: &str, text: &str) {
    let title = if title.is_empty() { "formicaria" } else { title };
    #[cfg(target_os = "linux")]
    let cmd = {
        let mut c = std::process::Command::new("notify-send");
        c.args(["--app-name=formicaria", "--"]).arg(title).arg(text);
        Some(c)
    };
    #[cfg(target_os = "macos")]
    let cmd = {
        let mut c = std::process::Command::new("osascript");
        c.args([
            "-e",
            "on run argv",
            "-e",
            "display notification (item 2 of argv) with title (item 1 of argv)",
            "-e",
            "end run",
        ])
        .arg(title)
        .arg(text);
        Some(c)
    };
    #[cfg(target_os = "windows")]
    let cmd = {
        use std::os::windows::process::CommandExt;
        // A toast under PowerShell's own name, which every Windows 10 and 11 has registered.
        const SCRIPT: &str = "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
            $x = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
            $t = $x.GetElementsByTagName('text'); \
            $t.Item(0).AppendChild($x.CreateTextNode($env:FM_REMIND_TITLE)) > $null; \
            $t.Item(1).AppendChild($x.CreateTextNode($env:FM_REMIND_TEXT)) > $null; \
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe').Show([Windows.UI.Notifications.ToastNotification]::new($x))";
        let mut c = std::process::Command::new("powershell");
        c.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", SCRIPT])
            .env("FM_REMIND_TITLE", title)
            .env("FM_REMIND_TEXT", text)
            // CREATE_NO_WINDOW: no console flashing up beside the notification.
            .creation_flags(0x0800_0000);
        Some(c)
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let cmd: Option<std::process::Command> = {
        let _ = (title, text);
        None
    };
    if let Some(mut c) = cmd {
        c.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        // Waited for on its own thread, so nothing is left behind as a zombie.
        if let Ok(mut child) = c.spawn() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(title: &str, at: u64) -> Item {
        Item { title: title.into(), text: "Starts now".into(), at }
    }

    #[test]
    fn something_is_waiting_only_until_its_moment() {
        let s = RemindState::at(None);
        assert!(!s.waiting_at(0), "an empty list keeps nothing running");
        s.set(vec![item("Dentist", 2_000)]);
        assert!(s.waiting_at(1_000));
        assert!(!s.waiting_at(2_000), "a reminder whose moment has come is not waited for");
    }

    #[test]
    fn what_comes_due_is_said_once_and_the_rest_is_kept() {
        let s = RemindState::at(None);
        s.set(vec![item("Later", 9_000), item("Dentist", 2_000)]);
        assert_eq!(s.take_due(1_000), vec![]);
        assert_eq!(s.take_due(2_500), vec![item("Dentist", 2_000)]);
        assert_eq!(s.take_due(2_600), vec![], "said twice");
        assert!(s.waiting_at(2_600), "the later one was dropped");
    }

    #[test]
    fn a_reminder_long_past_is_dropped_without_a_word() {
        let s = RemindState::at(None);
        s.set(vec![item("Yesterday", 1_000)]);
        assert_eq!(s.take_due(1_000 + LATE_MS + 1), vec![]);
        assert!(!s.waiting_at(0), "it was kept");
    }

    #[test]
    fn the_list_is_bounded_and_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("fm-remind-{}", std::process::id()));
        let file = dir.join("reminders.json");
        let s = RemindState::at(Some(file.clone()));
        s.set((0..80).map(|n| item(&"x".repeat(500), 1_000 + n)).collect());
        let again = RemindState::at(Some(file));
        let held = again.items.lock().unwrap();
        assert_eq!(held.len(), MOST);
        assert_eq!(held[0].title.chars().count(), 200);
        drop(held);
        let _ = std::fs::remove_dir_all(dir);
    }
}
