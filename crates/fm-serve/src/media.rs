//! **Save media: from any site, kept as one final file** (`decisions.md` 2026-11-17; plan:
//! `web-media-privacy-plan-2026-11-17.md`, part D). The computer's half: the tools and the jobs.
//!
//! - **The tools** — yt-dlp and ffmpeg (with ffprobe) — are fetched on first use into this
//!   machine's settings folder (`tools/media/`), each checked against the SHA-256 pinned in this
//!   release (`media/tools.json`, `pixi run media-bump`). yt-dlp keeps itself current between
//!   releases with its own updater, weekly, and once more when a site has changed.
//! - **A job** fetches only what was chosen (the video, the sound as served, or the sound as MP3),
//!   works inside the vault's own staging folder (`.fm-ingest/`, never synced, never backed up), and
//!   only the final file enters the vault (content-addressed: a file already there is not stored
//!   twice). Then the note it becomes: a song in Music, a video in Videos, or a picture-like
//!   attachment in a note. The staging folder goes on success, failure or cancel.
//!
//! What to ask for, how to read the tool and which notes to write are `fm_app::media` (pure, shared
//! with the phone). Every route here is in `REMOTE_DENIED`: a paired device does not run programs on
//! this machine.

use fm_app::media::{self as m, Ask, Into as Dest, Line, What};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

const PINS: &str = include_str!("../../../media/tools.json");

fn pins() -> Value {
    serde_json::from_str(PINS).expect("media/tools.json is malformed (pixi run media-bump)")
}

/// This machine, as `tools.json` names platforms.
fn platform(ffmpeg: bool) -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("linux", "aarch64") => Some("linux-aarch64"),
        ("windows", "x86_64") => Some("win64"),
        ("macos", "aarch64") => Some(if ffmpeg { "mac-arm64" } else { "mac" }),
        ("macos", "x86_64") => Some(if ffmpeg { "mac-x86_64" } else { "mac" }),
        _ => None,
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
static TEST_TOOLS: Mutex<Option<PathBuf>> = Mutex::new(None);

/// `tools/media/` in this machine's settings folder.
fn tools_dir() -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Some(d) = TEST_TOOLS.lock().ok().and_then(|d| d.clone()) {
        return Ok(d);
    }
    fm_app::vaults::config_dir()
        .map(|d| d.join("formicaria").join("tools").join("media"))
        .ok_or_else(|| "this machine has no settings folder for the media tools".to_string())
}

#[cfg(unix)]
fn make_runnable(p: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}
#[cfg(not(unix))]
fn make_runnable(_: &Path) -> Result<(), String> {
    Ok(())
}

fn fetch(url: &str, dest: &Path, sha256: &str, cancel: &AtomicBool) -> Result<(), String> {
    fm_fetch::fetch(&fm_fetch::Download { url, dest, sha256: Some(sha256) }, &|_, _| {}, &|| {
        cancel.load(Ordering::Relaxed)
    })
    .map_err(|e| format!("the media tools could not be fetched: {e}"))
}

/// One file out of a zip (by its path inside, or its name anywhere), into `dest`.
fn unzip_one(zip: &Path, inside: &str, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let at = (0..z.len())
        .find(|&i| {
            z.by_index(i).ok().is_some_and(|e| {
                let n = e.name().map(|n| n.to_string()).unwrap_or_default();
                n == inside || n.rsplit('/').next() == Some(inside)
            })
        })
        .ok_or_else(|| format!("{} holds no {inside}", zip.display()))?;
    let mut e = z.by_index(at).map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("new");
    let mut out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut e, &mut out).map_err(|e| e.to_string())?;
    drop(out);
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    make_runnable(dest)
}

/// **yt-dlp, ready.** Fetched when missing or when this release pins another; checked against the
/// pin. Answers its path.
fn ytdlp(cancel: &AtomicBool) -> Result<PathBuf, String> {
    let plat = platform(false).ok_or("Save media does not run on this kind of computer yet")?;
    let pin = &pins()["yt-dlp"]["files"][plat];
    let (url, sha) = (pin["url"].as_str().unwrap_or(""), pin["sha256"].as_str().unwrap_or(""));
    let dir = tools_dir()?.join("yt-dlp");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bin = dir.join(exe("yt-dlp"));
    let stamp = dir.join("pinned");
    if bin.exists() && std::fs::read_to_string(&stamp).ok().as_deref() == Some(sha) {
        return Ok(bin);
    }
    let dl = dir.join("download");
    let _ = std::fs::remove_file(&dl);
    fetch(url, &dl, sha, cancel)?;
    std::fs::rename(&dl, &bin).map_err(|e| e.to_string())?;
    make_runnable(&bin)?;
    std::fs::write(&stamp, sha).map_err(|e| e.to_string())?;
    let _ = std::fs::write(dir.join("checked"), "");
    Ok(bin)
}

/// **ffmpeg and ffprobe, ready**, in one folder (yt-dlp is told where). Answers the folder.
fn ffmpeg(cancel: &AtomicBool) -> Result<PathBuf, String> {
    let plat = platform(true).ok_or("Save media does not run on this kind of computer yet")?;
    let p = pins();
    let pin = &p["ffmpeg"]["files"][plat];
    let version = p["ffmpeg"]["version"].as_str().unwrap_or("");
    let dir = tools_dir()?.join("ffmpeg");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = dir.join("version");
    let (ff, probe) = (dir.join(exe("ffmpeg")), dir.join(exe("ffprobe")));
    if ff.exists()
        && probe.exists()
        && std::fs::read_to_string(&stamp).ok().as_deref() == Some(version)
    {
        return Ok(dir);
    }
    let s = |k: &str| pin[k].as_str().unwrap_or("").to_string();
    let zip1 = dir.join("download.zip");
    let _ = std::fs::remove_file(&zip1);
    fetch(&s("url"), &zip1, &s("sha256"), cancel)?;
    if let Some(inner) = pin["inner"].as_str() {
        // Windows: one archive holding both.
        unzip_one(&zip1, &format!("{inner}ffmpeg.exe"), &ff)?;
        unzip_one(&zip1, &format!("{inner}ffprobe.exe"), &probe)?;
    } else {
        unzip_one(&zip1, "ffmpeg", &ff)?;
        let zip2 = dir.join("download-probe.zip");
        let _ = std::fs::remove_file(&zip2);
        fetch(&s("probe_url"), &zip2, &s("probe_sha256"), cancel)?;
        unzip_one(&zip2, "ffprobe", &probe)?;
        let _ = std::fs::remove_file(&zip2);
    }
    let _ = std::fs::remove_file(&zip1);
    std::fs::write(&stamp, version).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// yt-dlp's own updater, when it last ran more than a week ago (or `force`, after a site changed).
/// Its failure is not a reason to stop: the copy in hand is tried.
fn freshen(bin: &Path, force: bool) {
    let mark = bin.with_file_name("checked");
    let stale = std::fs::metadata(&mark)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_none_or(|age| age > Duration::from_secs(7 * 24 * 3600));
    if !(stale || force) {
        return;
    }
    let _ = Command::new(bin)
        .args(["-U", "--update-to", "stable"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = std::fs::write(&mark, "");
}

/// A job as the sheet shows it.
#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub what: What,
    /// `tools`, `downloading`, `converting`, `filing`, `done`, `failed`, `cancelled`.
    pub state: &'static str,
    pub progress: Option<f64>,
    pub error: Option<String>,
    /// The note it became.
    pub note: Option<String>,
}

struct Job {
    view: JobView,
    cancel: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

#[derive(Default)]
pub struct Media {
    jobs: Mutex<Vec<Job>>,
    next: AtomicU64,
}

impl Media {
    fn set(&self, id: u64, f: impl FnOnce(&mut JobView)) {
        if let Ok(mut jobs) = self.jobs.lock() {
            if let Some(j) = jobs.iter_mut().find(|j| j.view.id == id) {
                f(&mut j.view);
            }
        }
    }
    fn views(&self) -> Vec<JobView> {
        self.jobs.lock().map(|j| j.iter().map(|j| j.view.clone()).collect()).unwrap_or_default()
    }
}

/// **Probe**: what the address holds, before anything is fetched (title, length, heights, whether
/// the sound comes alone). One run of yt-dlp, about a second.
pub fn probe(url: &str) -> Result<Value, String> {
    let cancel = AtomicBool::new(false);
    let bin = ytdlp(&cancel)?;
    freshen(&bin, false);
    let out = Command::new(&bin)
        .args(["-J", "--no-playlist", "--no-warnings", "--no-cache-dir", "--", url])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("yt-dlp could not start: {e}"))?;
    if !out.status.success() {
        return Err(m::explain(&String::from_utf8_lossy(&out.stderr)));
    }
    let info: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    Ok(m::probe_summary(&info))
}

/// Start a job; answers its id. It runs on its own thread.
pub fn start(state: &Arc<crate::AppState>, ask: Ask) -> Result<u64, String> {
    if !(ask.url.starts_with("https://") || ask.url.starts_with("http://")) {
        return Err("only a web address can be saved".into());
    }
    if ask.into == Dest::Note && ask.note.is_empty() {
        return Err("which note should it go in?".into());
    }
    let media = &state.media;
    let id = media.next.fetch_add(1, Ordering::Relaxed) + 1;
    let job = Job {
        view: JobView {
            id,
            url: ask.url.clone(),
            title: String::new(),
            what: ask.what,
            state: "tools",
            progress: None,
            error: None,
            note: None,
        },
        cancel: Arc::new(AtomicBool::new(false)),
        child: Arc::new(Mutex::new(None)),
    };
    let (cancel, child) = (Arc::clone(&job.cancel), Arc::clone(&job.child));
    if let Ok(mut jobs) = media.jobs.lock() {
        // Finished jobs are kept for the sheet, the last twenty.
        let done =
            jobs.iter().filter(|j| matches!(j.view.state, "done" | "failed" | "cancelled")).count();
        if done > 20 {
            if let Some(at) =
                jobs.iter().position(|j| matches!(j.view.state, "done" | "failed" | "cancelled"))
            {
                jobs.remove(at);
            }
        }
        jobs.push(job);
    }
    let st = Arc::clone(state);
    std::thread::spawn(move || {
        let r = run(&st, id, &ask, &cancel, &child);
        st.media.set(id, |v| match r {
            Ok(note) => {
                v.state = "done";
                v.progress = Some(1.0);
                v.note = Some(note);
            }
            Err(_) if cancel.load(Ordering::Relaxed) => v.state = "cancelled",
            Err(e) => {
                v.state = "failed";
                v.error = Some(e);
            }
        });
    });
    Ok(id)
}

pub fn cancel(state: &crate::AppState, id: u64) {
    if let Ok(jobs) = state.media.jobs.lock() {
        if let Some(j) = jobs.iter().find(|j| j.view.id == id) {
            j.cancel.store(true, Ordering::Relaxed);
            if let Ok(mut c) = j.child.lock() {
                if let Some(c) = c.as_mut() {
                    let _ = c.kill();
                }
            }
        }
    }
}

/// The job itself. Answers the note it became.
fn run(
    state: &Arc<crate::AppState>,
    id: u64,
    ask: &Ask,
    cancel: &AtomicBool,
    child: &Mutex<Option<Child>>,
) -> Result<String, String> {
    let media = &state.media;
    let bin = ytdlp(cancel)?;
    let ff = ffmpeg(cancel)?;
    freshen(&bin, false);
    let (vault, vpath) = m::vault_of(&state.app, &crate::Desktop, &ask.vault)?;
    let session = format!("media-{id}-{}", std::process::id());
    let stage = vpath.join(".fm-ingest").join(&session);
    let work = stage.join("work");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut tried_update = false;
        let (file, info) = loop {
            media.set(id, |v| {
                v.state = "downloading";
                v.progress = None;
            });
            match fetch_media(state, id, &bin, &ff, &work, ask, cancel, child) {
                Ok(done) => break done,
                // A site that changed: yt-dlp updates itself once and tries again.
                Err(e)
                    if !tried_update
                        && e.contains("changed how it serves")
                        && !cancel.load(Ordering::Relaxed) =>
                {
                    tried_update = true;
                    freshen(&bin, true);
                }
                Err(e) => return Err(e),
            }
        };
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        media.set(id, |v| v.state = "filing");
        m::keep(&state.app, &crate::Desktop, &vault, &file, &session, ask, &info)
    })();
    let _ = std::fs::remove_dir_all(&stage);
    result
}

/// Run yt-dlp for the choice; answers the finished file and the media's details.
#[allow(clippy::too_many_arguments)]
fn fetch_media(
    state: &crate::AppState,
    id: u64,
    bin: &Path,
    ff: &Path,
    work: &Path,
    ask: &Ask,
    cancel: &AtomicBool,
    child: &Mutex<Option<Child>>,
) -> Result<(PathBuf, Value), String> {
    let args = m::ytdlp_args(
        ask.what,
        ask.quality,
        &ask.mp3,
        &work.to_string_lossy(),
        Some(&ff.to_string_lossy()),
        &ask.url,
        &ask.meta,
    );
    let mut c = Command::new(bin)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("yt-dlp could not start: {e}"))?;
    let out = c.stdout.take();
    let mut err = c.stderr.take();
    *child.lock().map_err(|e| e.to_string())? = Some(c);
    let said = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let (mut file, mut info) = (None, Value::Null);
    if let Some(o) = out {
        for line in BufReader::new(o).lines().map_while(Result::ok) {
            match m::read_line(&line) {
                Line::Progress(p) => state.media.set(id, |v| v.progress = p),
                Line::Converting => state.media.set(id, |v| {
                    v.state = "converting";
                    v.progress = None;
                }),
                Line::File(f) => file = Some(PathBuf::from(f)),
                Line::Info(i) => {
                    let t = i["title"].as_str().unwrap_or("").to_string();
                    state.media.set(id, |v| v.title = t);
                    info = i;
                }
                Line::Other => {}
            }
        }
    }
    let status = child.lock().map_err(|e| e.to_string())?.take().map(|mut c| c.wait());
    let stderr = said.join().unwrap_or_default();
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    match status {
        Some(Ok(s)) if s.success() => {}
        _ => return Err(m::explain(&stderr)),
    }
    let file = file.filter(|f| f.exists()).ok_or("yt-dlp finished without a file")?;
    Ok((file, info))
}

/// The routes. `None`: not one of these.
pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    state: &Arc<crate::AppState>,
) -> Option<std::io::Result<()>> {
    let args: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let answer = match path {
        "/api/media_probe" => probe(args["url"].as_str().unwrap_or("")),
        "/api/media_save" => serde_json::from_value::<Ask>(args)
            .map_err(|e| format!("not a media request: {e}"))
            .and_then(|a| start(state, a))
            .map(|id| json!({ "id": id })),
        "/api/media_jobs" => Ok(json!(state.media.views())),
        "/api/media_cancel" => {
            cancel(state, args["id"].as_u64().unwrap_or(0));
            Ok(Value::Null)
        }
        "/api/media_tools" => Ok(tools_status()),
        _ => return None,
    };
    Some(match answer {
        Ok(v) => {
            crate::write_response(stream, "200 OK", "application/json", v.to_string().as_bytes())
        }
        Err(e) => crate::write_response(stream, "400 Bad Request", "text/plain", e.as_bytes()),
    })
}

/// Whether the tools are here yet, and which (the sheet says what a first save will fetch).
fn tools_status() -> Value {
    let dir = tools_dir().ok();
    let has = |p: &[&str]| dir.as_ref().is_some_and(|d| p.iter().all(|f| d.join(f).exists()));
    let p = pins();
    json!({
        "ytdlp": has(&["yt-dlp/pinned"]),
        "ffmpeg": has(&["ffmpeg/version"]),
        "ytdlp_version": p["yt-dlp"]["version"],
        "ffmpeg_version": p["ffmpeg"]["version"],
        "supported": platform(false).is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_its_pinned_tools() {
        let p = pins();
        for plat in ["linux-x86_64", "linux-aarch64", "win64", "mac"] {
            let f = &p["yt-dlp"]["files"][plat];
            assert!(f["url"].as_str().unwrap().starts_with("https://github.com/yt-dlp/"), "{plat}");
            assert_eq!(f["sha256"].as_str().unwrap().len(), 64, "{plat}");
        }
        for plat in ["linux-x86_64", "linux-aarch64", "mac-arm64", "mac-x86_64", "win64"] {
            let f = &p["ffmpeg"]["files"][plat];
            assert!(f["url"].as_str().unwrap().starts_with("https://"), "{plat}");
            assert_eq!(f["sha256"].as_str().unwrap().len(), 64, "{plat}");
        }
        assert!(p["ffmpeg"]["files"]["win64"]["inner"].as_str().unwrap().ends_with("/bin/"));
    }

    /// **A whole job, with a stand-in yt-dlp** (no site, no network): only the final file enters the
    /// vault, it becomes a song in a new playlist, and the staging folder is gone.
    #[cfg(unix)]
    #[test]
    fn a_job_keeps_only_the_final_file_and_makes_it_a_song() {
        let d = tempfile::tempdir().unwrap();
        let tools = d.path().join("tools");
        let plat = platform(false).unwrap();
        let p = pins();
        std::fs::create_dir_all(tools.join("yt-dlp")).unwrap();
        std::fs::create_dir_all(tools.join("ffmpeg")).unwrap();
        std::fs::write(
            tools.join("yt-dlp/pinned"),
            p["yt-dlp"]["files"][plat]["sha256"].as_str().unwrap(),
        )
        .unwrap();
        std::fs::write(tools.join("yt-dlp/checked"), "").unwrap();
        std::fs::write(tools.join("ffmpeg/version"), p["ffmpeg"]["version"].as_str().unwrap())
            .unwrap();
        for f in ["ffmpeg/ffmpeg", "ffmpeg/ffprobe"] {
            std::fs::write(tools.join(f), "#!/bin/sh\n").unwrap();
        }
        // Writes `media.m4a` where it is told, says its progress, details and file, as yt-dlp does.
        let script = r#"#!/bin/sh
out=""; while [ $# -gt 0 ]; do [ "$1" = "-o" ] && out="$2"; shift; done
dir=$(dirname "$out"); printf 'AUDIO-BYTES' > "$dir/media.m4a"
echo "FMPROG 5 10 NA"
echo 'FMINFO {"title":"Leafcutters at night","uploader":"Ant Channel","webpage_url":"https://v.example/w?v=1","duration":61}'
echo "FMFILE $dir/media.m4a"
"#;
        std::fs::write(tools.join("yt-dlp/yt-dlp"), script).unwrap();
        make_runnable(&tools.join("yt-dlp/yt-dlp")).unwrap();
        *TEST_TOOLS.lock().unwrap() = Some(tools);

        let vault = d.path().join("v");
        std::fs::create_dir_all(vault.join("notes")).unwrap();
        let store = fm_core::MultiStore::open(&[("v".to_string(), vault.clone())]).unwrap();
        let cfg =
            fm_app::vaults::VaultConfig { name: "v".into(), path: vault.clone(), restic: None };
        let state = Arc::new(crate::AppState::new(
            fm_app::App::new(store, vec![cfg], None, false),
            None,
            Vec::new(),
            0,
        ));
        let ask: Ask = serde_json::from_value(json!({
            "url": "https://v.example/embed/1", "page": "https://news.example/a",
            "what": "audio", "into": "music", "list": "new:Road"
        }))
        .unwrap();
        let id = start(&state, ask).unwrap();
        let job = (0..200)
            .find_map(|_| {
                std::thread::sleep(Duration::from_millis(50));
                state.media.views().into_iter().find(|j| {
                    j.id == id
                        && j.state != "tools"
                        && j.state != "downloading"
                        && j.state != "filing"
                })
            })
            .expect("the job ends");
        assert_eq!(job.state, "done", "{:?}", job.error);
        let note = call(&state, "get", json!({ "id": job.note.unwrap() })).unwrap();
        assert_eq!(note["props"]["log"], "music");
        assert_eq!(note["props"]["artist"], "Ant Channel");
        assert_eq!(note["props"]["page"], "https://news.example/a");
        let playlists = call(&state, "music_playlists", json!({})).unwrap();
        assert_eq!(playlists[0]["title"], "Road");
        assert_eq!(playlists[0]["songs"].as_array().unwrap().len(), 1);
        // Only the final file is in the vault; nothing is left staged.
        let blobs: Vec<_> = walk(&vault.join("blobs"));
        assert_eq!(blobs.iter().filter(|b| std::fs::read(b).unwrap() == b"AUDIO-BYTES").count(), 1);
        let staged = vault.join(".fm-ingest");
        assert!(!staged.exists() || std::fs::read_dir(&staged).unwrap().next().is_none());
        *TEST_TOOLS.lock().unwrap() = None;
    }

    fn call(state: &crate::AppState, cmd: &str, args: Value) -> Result<Value, String> {
        let b = fm_app::dispatch(cmd, &args, &[], &state.app, &crate::Desktop)?.into_bytes();
        serde_json::from_slice(&b).map_err(|e| e.to_string())
    }

    fn walk(d: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p))
            } else {
                out.push(p)
            }
        }
        out
    }

    #[test]
    fn a_file_comes_out_of_a_zip_runnable() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("a.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&z).unwrap());
        w.start_file("pkg/bin/ffmpeg", zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut w, b"#!/bin/sh\n").unwrap();
        w.finish().unwrap();
        let out = d.path().join("ffmpeg");
        unzip_one(&z, "ffmpeg", &out).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"#!/bin/sh\n");
        assert!(unzip_one(&z, "ffprobe", &d.path().join("x")).is_err());
    }
}
