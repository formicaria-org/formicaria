//! One download running in the background, with progress a caller can ask about.
//!
//! [`fetch`](crate::fetch) blocks until the file is there. A screen wants to start one, show how
//! far it has got, and let the person stop it — so this is the smallest thing around `fetch` that
//! allows that: one job at a time, no queue, generic over `{url, dest, sha256}` like the rest of
//! the crate. A `Job` is meant to live in a `static`.

use crate::{fetch, Download, FetchError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// Where a job has got to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub running: bool,
    pub received: u64,
    /// `None` until the server has said how large the file is.
    pub total: Option<u64>,
    /// Why the last run stopped, when it did not finish.
    pub error: Option<String>,
    /// The last run finished, and the file is in place and verified.
    pub done: bool,
}

pub struct Job {
    state: Mutex<Progress>,
    stop: AtomicBool,
}

impl Default for Job {
    fn default() -> Self {
        Self::new()
    }
}

/// How many times a dropped connection is resumed before the job gives up and says why.
const RETRIES: usize = 5;

impl Job {
    pub const fn new() -> Self {
        Job {
            state: Mutex::new(Progress {
                running: false,
                received: 0,
                total: None,
                error: None,
                done: false,
            }),
            stop: AtomicBool::new(false),
        }
    }

    fn set(&self, f: impl FnOnce(&mut Progress)) {
        // A poisoned lock only means a previous run panicked; the numbers in it are still numbers.
        let mut g = self.state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut g);
    }

    pub fn progress(&self) -> Progress {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Ask the running job to stop. The partial file stays, so starting again resumes it.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Start fetching `url` into `dest`. Answers `false`, and starts nothing, when a job is
    /// already running.
    pub fn start(&'static self, url: String, dest: PathBuf, sha256: Option<String>) -> bool {
        {
            let mut g = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if g.running {
                return false;
            }
            *g = Progress { running: true, ..Progress::default() };
        }
        self.stop.store(false, Ordering::SeqCst);
        std::thread::spawn(move || {
            if let Some(dir) = dest.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let d = Download { url: &url, dest: &dest, sha256: sha256.as_deref() };
            let mut tries = 0;
            let outcome = loop {
                let got = fetch(
                    &d,
                    &|received, total| {
                        self.set(|p| {
                            p.received = received;
                            p.total = total;
                        })
                    },
                    &|| self.stop.load(Ordering::SeqCst),
                );
                match got {
                    Err(FetchError::Transient(_))
                        if tries < RETRIES && !self.stop.load(Ordering::SeqCst) =>
                    {
                        tries += 1;
                        std::thread::sleep(Duration::from_secs(2));
                    }
                    other => break other,
                }
            };
            self.set(|p| {
                p.running = false;
                match outcome {
                    Ok(()) => p.done = true,
                    Err(e) => p.error = Some(e.to_string()),
                }
            });
        });
        true
    }
}
