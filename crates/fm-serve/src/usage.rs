//! **The usage diary on a computer** (`fm_app::diary`, `decisions.md` 2026-11-20): this program's
//! start and how the last one ended, every stop and its reason, and every five minutes the memory and
//! processor of each part — this server, formicaria's browser, the assistant and what it started.
//! formicaria's browser's own end (its exit code, a signal if it crashed or was killed) and its error
//! output (`browser.log`, the previous run's beside it) are kept too: on 2026-11-19 a browser that
//! "crashed" had to be worked out afterwards from Firefox's own files.

use fm_app::diary;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Opened, the start said, and the sampler started. Nothing if this machine has no settings folder.
pub fn start(browser_dir: Option<PathBuf>) {
    let Some(d) = diary::init("computer") else { return };
    d.started(
        option_env!("FM_VERSION").unwrap_or("dev"),
        json!({ "os": std::env::consts::OS, "arch": std::env::consts::ARCH }),
    );
    std::thread::spawn(move || {
        let mut prev = HashMap::new();
        let mut day = 0u64;
        loop {
            std::thread::sleep(diary::every());
            d.flush();
            if d.is_on() && cfg!(target_os = "linux") {
                let all = diary::processes();
                let parts = parts(&all, browser_dir.as_deref());
                d.record("sample", json!({ "parts": diary::sample(&parts, &all, &mut prev) }));
            }
            // Once a day, the oldest days go.
            let today = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|t| t.as_secs() / 86_400)
                .unwrap_or(0);
            if today != day {
                day = today;
                d.prune();
            }
        }
    });
}

/// This program leaving, and why (said before each of its own exits).
pub fn stop(reason: &str) {
    if let Some(d) = diary::get() {
        d.stopped(reason);
    }
}

/// The parts, by their processes: this server; formicaria's browser (its program's folder); and the
/// rest of what this server started (the assistant, its model, the media tools).
fn parts<'a>(all: &[diary::Proc], browser_dir: Option<&Path>) -> Vec<(&'a str, Vec<u32>)> {
    let me = std::process::id();
    let browser: Vec<u32> = match browser_dir {
        Some(b) => all
            .iter()
            .filter(|p| p.exe.as_deref().is_some_and(|e| e.starts_with(b)))
            .map(|p| p.pid)
            .collect(),
        None => Vec::new(),
    };
    let rest: Vec<u32> = diary::descendants(all, me)
        .into_iter()
        .filter(|p| *p != me && !browser.contains(p))
        .collect();
    vec![("server", vec![me]), ("browser", browser), ("assistant", rest)]
}

/// Where formicaria's browser writes its error output: the diary's `browser.log`, the previous run's
/// kept as `browser.log.1`. Nowhere while the diary is off.
pub fn browser_output() -> std::process::Stdio {
    let Some(d) = diary::get().filter(|d| d.is_on()) else { return std::process::Stdio::null() };
    let log = d.dir().join("browser.log");
    let _ = std::fs::rename(&log, d.dir().join("browser.log.1"));
    std::fs::File::create(&log)
        .map(std::process::Stdio::from)
        .unwrap_or_else(|_| std::process::Stdio::null())
}

/// The browser just started (`handed_over`: one was already running and took the address): its end,
/// when it comes, said — how long it ran, its exit code, or the signal that ended it.
pub fn watch_browser(mut child: std::process::Child, handed_over: bool) {
    let began = Instant::now();
    std::thread::spawn(move || {
        let Ok(status) = child.wait() else { return };
        if handed_over {
            return; // the running browser took the address; this one only passed it on
        }
        let Some(d) = diary::get() else { return };
        #[cfg(unix)]
        let signal = std::os::unix::process::ExitStatusExt::signal(&status);
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        d.record(
            "browser_exit",
            json!({ "code": status.code(), "signal": signal, "ran_s": began.elapsed().as_secs() }),
        );
        // The log stays small: a browser that ran for days and wrote much is cut to its last part.
        let log = d.dir().join("browser.log");
        if std::fs::metadata(&log).map(|m| m.len() > 5 * 1024 * 1024).unwrap_or(false) {
            if let Ok(b) = std::fs::read(&log) {
                let _ = std::fs::write(&log, &b[b.len() - 1024 * 1024..]);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_are_this_server_the_browser_and_what_this_server_started() {
        let me = std::process::id();
        let p = |pid, ppid, exe: &str| diary::Proc {
            pid,
            ppid,
            exe: Some(PathBuf::from(exe)),
            ticks: 0,
        };
        let all = vec![
            p(me, 1, "/app/program/fm-serve"),
            p(90_001, me, "/app/program/agent-serve"),
            p(90_002, 90_001, "/tools/llama-server"),
            p(90_003, me, "/cfg/browser/firefox/firefox"),
            p(90_004, 1, "/cfg/browser/firefox/firefox"), // a content process, reparented
            p(90_005, 1, "/usr/bin/other"),
        ];
        let got = parts(&all, Some(Path::new("/cfg/browser/firefox")));
        assert_eq!(got[0], ("server", vec![me]));
        assert_eq!(got[1], ("browser", vec![90_003, 90_004]));
        assert_eq!(got[2], ("assistant", vec![90_001, 90_002]));
    }
}
