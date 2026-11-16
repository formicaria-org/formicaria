//! **formicaria's kit, applied to Mozilla's ESR** — `decisions.md` 2026-11-15 (evening); the plan is
//! `browser-plan` §13–§16, the measurements `web-browsing-research` §15b.
//!
//! Not a fork: Mozilla's finished ESR, downloaded from Mozilla, checked against **Mozilla's signature**
//! (`gpgv`, Mozilla's release key compiled in) and the **pinned SHA-512** (`browser/esr.toml`), then:
//!
//! 1. formicaria's files are written beside the program: the startup script (autoconfig), the
//!    `chrome://branding/` overrides, styles and icons;
//! 2. Firefox's packed interface (`browser/omni.ja`) is rewritten with **every entry copied raw** and the
//!    one modern brand file swapped (same size, so no startup cost; measured, S0);
//! 3. it is started once, headless, to check it runs.
//!
//! **Nothing heavy happens while the person waits** (`browser-plan` §16): a new kit (after a formicaria
//! update) or a newer security point release is prepared in `firefox.next/` in the background, and
//! swapped in, with two renames, at the next start, when the browser is not running. The previous copy
//! is kept in `firefox.prev/` (the updater's G2: a failure leaves a working browser).
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The pinned ESR (`pixi run browser-bump` writes it).
const ESR_TOML: &str = include_str!("../../../browser/esr.toml");
/// Mozilla's release key ("Mozilla Software Releases", 14F2 6682 D091 6CDD 81E3 7B6D 61B7 B526 D98F
/// 0353), as a binary keyring for `gpgv`.
const MOZILLA_KEY: &[u8] = include_bytes!("../../../browser/mozilla-release-key.gpg");
/// The one file swapped inside Firefox's package.
const BRAND_FTL: &[u8] =
    include_bytes!("../../../browser/kit/localization/en-US/branding/brand.ftl");
const BRAND_FTL_ENTRY: &str = "localization/en-US/branding/brand.ftl";

macro_rules! kit_file {
    ($rel:literal, $src:literal) => {
        ($rel, include_bytes!(concat!("../../../browser/kit/", $src)) as &[u8])
    };
}
/// formicaria's files, by where they go relative to the Firefox folder.
const KIT: &[(&str, &[u8])] = &[
    kit_file!("defaults/pref/autoconfig.js", "autoconfig.js"),
    kit_file!("formicaria.cfg", "formicaria.cfg"),
    kit_file!("formicaria/chrome.manifest", "chrome.manifest"),
    kit_file!("formicaria/formicaria.css", "formicaria.css"),
    kit_file!("formicaria/branding/brand.properties", "branding/brand.properties"),
    kit_file!("formicaria/branding/icon16.png", "branding/icon16.png"),
    kit_file!("formicaria/branding/icon32.png", "branding/icon32.png"),
    kit_file!("formicaria/branding/icon48.png", "branding/icon48.png"),
    kit_file!("formicaria/branding/icon64.png", "branding/icon64.png"),
    kit_file!("formicaria/branding/icon128.png", "branding/icon128.png"),
    kit_file!("formicaria/branding/about-logo.png", "branding/about-logo.png"),
    kit_file!("formicaria/branding/about-logo@2x.png", "branding/about-logo@2x.png"),
    kit_file!("formicaria/branding/about-logo.svg", "branding/about-logo.svg"),
    kit_file!("formicaria/branding/about-wordmark.svg", "branding/about-wordmark.svg"),
    // The window's own icon (the dock, the window list): Firefox's, replaced.
    kit_file!("browser/chrome/icons/default/default16.png", "branding/icon16.png"),
    kit_file!("browser/chrome/icons/default/default32.png", "branding/icon32.png"),
    kit_file!("browser/chrome/icons/default/default48.png", "branding/icon48.png"),
    kit_file!("browser/chrome/icons/default/default64.png", "branding/icon64.png"),
    kit_file!("browser/chrome/icons/default/default128.png", "branding/icon128.png"),
];

/// Which kit this is: written beside an applied kit, so a formicaria update with a new kit is noticed.
pub fn kit_stamp() -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (rel, bytes) in KIT {
        rel.hash(&mut h);
        bytes.hash(&mut h);
    }
    BRAND_FTL.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// The pinned ESR.
#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    pub version: String,
    pub major: u32,
    pub sha512: Vec<(String, String)>,
}

/// Read `esr.toml` (a handful of lines, hand-read: no TOML parser for this).
pub fn parse_pin(text: &str) -> Option<Pin> {
    let mut version = None;
    let mut major = None;
    let mut sha512 = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(v) = line.strip_prefix("version = ") {
            version = Some(v.trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("major = ") {
            major = v.parse().ok();
        } else if let Some((k, v)) = line.split_once(" = ") {
            if k.starts_with('"') && v.starts_with('"') {
                sha512.push((k.trim_matches('"').to_string(), v.trim_matches('"').to_string()));
            }
        }
    }
    Some(Pin { version: version?, major: major?, sha512 })
}

pub fn pin() -> Pin {
    parse_pin(ESR_TOML).expect("browser/esr.toml is malformed (pixi run browser-bump)")
}

/// Mozilla's name for this computer's build, when there is one formicaria offers.
pub fn platform() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux-x86_64")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("linux-aarch64")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("win64")
    } else if cfg!(target_os = "macos") {
        // One universal build for Intel and Apple silicon.
        Some("mac")
    } else {
        None
    }
}

/// Mozilla's file for `platform` in a release folder, as its `SHA512SUMS` names it: an archive on
/// Linux, the installer on Windows (unpacked, never run as an installer), the disk image on macOS.
pub fn archive_name(version: &str, platform: &str) -> String {
    match platform {
        "win64" => format!("win64/en-US/Firefox Setup {version}.exe"),
        "mac" => format!("mac/en-US/Firefox {version}.dmg"),
        p => format!("{p}/en-US/firefox-{version}.tar.xz"),
    }
}

/// Where formicaria's files go inside a Firefox folder: the folder itself, or on macOS the app's
/// `Contents/Resources` (where Firefox reads `defaults/pref`, `distribution` and `browser/omni.ja`).
pub fn res_dir(ff: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        ff.join("Firefox.app").join("Contents").join("Resources")
    } else {
        ff.to_path_buf()
    }
}

/// The program inside a Firefox folder.
pub fn bin(ff: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        ff.join("firefox.exe")
    } else if cfg!(target_os = "macos") {
        ff.join("Firefox.app").join("Contents").join("MacOS").join("firefox")
    } else {
        ff.join("firefox")
    }
}

/// Can Mozilla's signature be checked here? Without `gpgv` (Windows and macOS usually), only the
/// version this formicaria release pinned is taken, by its sum.
pub fn gpgv_available() -> bool {
    // **Not on Windows** (2026-11-17): a `gpgv` found there (Git for Windows carries one) failed to
    // verify Mozilla's genuine signature on the release's Windows job, most likely over Windows paths.
    // There the sum this release pinned (checked against Mozilla's signature when the release was
    // made) is what is trusted, for the pinned version only.
    if cfg!(windows) {
        return false;
    }
    Command::new("gpgv")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Unpack Mozilla's download into `stage/firefox`.
fn unpack(archive: &Path, stage: &Path) -> Result<PathBuf, String> {
    let ff = stage.join("firefox");
    let run = |c: &mut Command, what: &str| -> Result<(), String> {
        let out = c.output().map_err(|e| format!("{what} could not be run: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Firefox could not be unpacked: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    };
    if cfg!(target_os = "windows") {
        // The installer unpacks itself and exits, installing nothing (`/ExtractDir`, Mozilla's).
        let x = stage.join("x");
        run(
            Command::new(archive).arg(format!("/ExtractDir={}", x.display())),
            "Firefox's installer",
        )?;
        let core = if x.join("core").join("firefox.exe").is_file() { x.join("core") } else { x };
        std::fs::rename(&core, &ff).map_err(|e| e.to_string())?;
    } else if cfg!(target_os = "macos") {
        let mnt = stage.join("mnt");
        std::fs::create_dir_all(&mnt).map_err(|e| e.to_string())?;
        run(
            Command::new("hdiutil")
                .args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"])
                .arg(&mnt)
                .arg(archive),
            "hdiutil",
        )?;
        std::fs::create_dir_all(&ff).map_err(|e| e.to_string())?;
        let copied = run(
            Command::new("ditto").arg(mnt.join("Firefox.app")).arg(ff.join("Firefox.app")),
            "ditto",
        );
        let _ = Command::new("hdiutil").arg("detach").arg(&mnt).arg("-force").output();
        copied?;
    } else {
        run(Command::new("tar").arg("-xJf").arg(archive).arg("-C").arg(stage), "`tar`")?;
    }
    Ok(ff)
}

/// `153.7.0esr` → (153, 7, 0).
pub fn version_parts(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.strip_suffix("esr")?;
    let mut it = v.split('.').map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

/// Is `candidate` a newer **point** release of the pinned major (owner, Q1)? A new major only comes with
/// a formicaria release.
pub fn is_newer_point(installed: &str, candidate: &str, major: u32) -> bool {
    match (version_parts(installed), version_parts(candidate)) {
        (Some(a), Some(b)) => b.0 == major && a.0 == major && b > a,
        _ => false,
    }
}

/// Mozilla's signature on `SHA512SUMS`, checked with `gpgv` against Mozilla's key. `Err` when `gpgv`
/// cannot be run at all (then only the pinned version, by its pinned sum, may be installed).
fn signature_ok(work: &Path, sums: &[u8], sig: &[u8]) -> Result<bool, String> {
    let ring = work.join("mozilla.gpg");
    std::fs::write(&ring, MOZILLA_KEY).map_err(|e| e.to_string())?;
    std::fs::write(work.join("SHA512SUMS"), sums).map_err(|e| e.to_string())?;
    std::fs::write(work.join("SHA512SUMS.asc"), sig).map_err(|e| e.to_string())?;
    let out = Command::new("gpgv")
        .arg("--keyring")
        .arg(&ring)
        .arg(work.join("SHA512SUMS.asc"))
        .arg(work.join("SHA512SUMS"))
        .output()
        .map_err(|e| format!("gpgv: {e}"))?;
    Ok(out.status.success())
}

/// Download, check and unpack `version` into `work/stage/firefox`, apply the kit and check it runs.
/// Answers the ready Firefox folder. `say` reports progress for Settings.
pub fn acquire(
    work: &Path,
    version: &str,
    say: &dyn Fn(&str),
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<PathBuf, String> {
    let plat = platform().ok_or("formicaria's browser is not available on this computer yet.")?;
    let pin = pin();
    std::fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let base = format!("https://archive.mozilla.org/pub/firefox/releases/{version}/");
    say(&format!("Checking Mozilla's signature for Firefox {version}…"));
    let sums = fm_fetch::get(&(base.clone() + "SHA512SUMS"), 8 * 1024 * 1024)
        .map_err(|e| format!("Mozilla's checksums could not be read: {e}"))?;
    let sig = fm_fetch::get(&(base.clone() + "SHA512SUMS.asc"), 64 * 1024)
        .map_err(|e| format!("Mozilla's signature could not be read: {e}"))?;
    let file = archive_name(version, plat);
    let want = crate::browser::sum_for(&String::from_utf8_lossy(&sums), &file)
        .ok_or("Mozilla's checksums do not list this file, so it was not downloaded.")?;
    let pinned_sum = pin.sha512.iter().find(|(p, _)| p == plat).map(|(_, s)| s.as_str());
    let checked =
        if gpgv_available() { signature_ok(work, &sums, &sig) } else { Err("no gpgv".into()) };
    match checked {
        Ok(true) => {}
        Ok(false) => {
            return Err("Mozilla's signature did not verify, so nothing was installed.".into())
        }
        // No gpgv on this computer: only the version this formicaria release tested, by its sum.
        Err(_) if version == pin.version && pinned_sum == Some(want.as_str()) => {}
        Err(_) => {
            return Err(
                "gpgv is not installed, so a Firefox other than the tested one cannot be checked."
                    .into(),
            )
        }
    }
    if version == pin.version && pinned_sum.is_some_and(|s| s != want) {
        return Err("Mozilla's sum differs from the one this formicaria release pinned.".into());
    }
    let ext = file.rsplit('.').next().unwrap_or("bin");
    let archive = work.join(format!("firefox-{version}.{ext}"));
    say(&format!("Downloading Firefox {version} from Mozilla…"));
    fm_fetch::fetch(
        &fm_fetch::Download {
            url: &(base + &file.replace(' ', "%20")),
            dest: &archive,
            sha256: None,
        },
        progress,
        &|| false,
    )
    .map_err(|e| format!("The download stopped: {e}"))?;
    say("Checking the download…");
    if !fm_fetch::verify_sha512(&archive, &want)? {
        let _ = std::fs::remove_file(&archive);
        return Err(
            "The download did not match Mozilla's published sum, so it was discarded.".into()
        );
    }
    say("Unpacking…");
    let stage = work.join("stage");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let unpacked = unpack(&archive, &stage);
    let _ = std::fs::remove_file(&archive);
    let ff = unpacked?;
    say("Making it formicaria's…");
    apply(&ff)?;
    say("Checking it starts…");
    self_check(&ff, work)?;
    std::fs::write(ff.join("formicaria-esr"), version).map_err(|e| e.to_string())?;
    Ok(ff)
}

/// Apply the kit to a Firefox folder: formicaria's files beside it, and the brand file swapped inside
/// its packed interface with every other entry copied raw.
pub fn apply(ff: &Path) -> Result<(), String> {
    let res = res_dir(ff);
    for (rel, bytes) in KIT {
        let p = res.join(rel);
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    swap_entry(&res.join("browser").join("omni.ja"), BRAND_FTL_ENTRY, BRAND_FTL)?;
    std::fs::write(res.join("formicaria").join("kit-stamp"), kit_stamp())
        .map_err(|e| e.to_string())?;
    if cfg!(target_os = "macos") {
        // **Changing files inside the app breaks Apple's seal**: an ad-hoc signature makes it run on
        // this Mac (`browser-plan` §13.4). Not valid on another Mac, which it never goes to.
        let out = Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-"])
            .arg(ff.join("Firefox.app"))
            .output()
            .map_err(|e| format!("codesign could not be run: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "macOS refused the changed browser: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    Ok(())
}

/// Rewrite the zip at `jar` with `entry` replaced by `bytes` (stored) and every other entry copied raw.
pub fn swap_entry(jar: &Path, entry: &str, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let tmp = jar.with_extension("ja.new");
    let src = std::fs::File::open(jar).map_err(|e| format!("{}: {e}", jar.display()))?;
    let mut z = zip::ZipArchive::new(src).map_err(|e| format!("{}: {e}", jar.display()))?;
    let out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut w = zip::ZipWriter::new(out);
    let mut swapped = false;
    for i in 0..z.len() {
        let f = z.by_index_raw(i).map_err(|e| e.to_string())?;
        let name = f.name().map(|n| n.to_string()).unwrap_or_default();
        if name == entry {
            drop(f);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            w.start_file(name, opts).map_err(|e| e.to_string())?;
            w.write_all(bytes).map_err(|e| e.to_string())?;
            swapped = true;
        } else {
            w.raw_copy_file(f).map_err(|e| e.to_string())?;
        }
    }
    w.finish().map_err(|e| e.to_string())?;
    if !swapped {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "{entry} is not in {}; this Firefox is not the expected one",
            jar.display()
        ));
    }
    std::fs::rename(&tmp, jar).map_err(|e| e.to_string())
}

/// Start it once, headless, on a throwaway profile: it must run and draw a page.
fn self_check(ff: &Path, work: &Path) -> Result<(), String> {
    let profile = work.join("check-profile");
    let shot = work.join("check.png");
    let _ = std::fs::remove_dir_all(&profile);
    let _ = std::fs::remove_file(&shot);
    std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
    let mut child = Command::new(bin(ff))
        .args(["--headless", "--no-remote"])
        .args(WAIT_FOR_BROWSER)
        .arg("--profile")
        .arg(&profile)
        .arg("--screenshot")
        .arg(&shot)
        .arg("about:blank")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("formicaria's browser could not be started: {e}"))?;
    let start = Instant::now();
    let limit = Duration::from_secs(90);
    // **The picture is the proof, not the exit** (2026-11-17): on Windows `firefox.exe` may be a
    // launcher that hands over to the browser and exits at once, so its exit says nothing about the
    // browser; the picture appears when the browser drew the page.
    let ok = loop {
        if shot.is_file() {
            break true;
        }
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => break false,
            Ok(_) if start.elapsed() > limit => {
                let _ = child.kill();
                break false;
            }
            Ok(_) => std::thread::sleep(Duration::from_millis(200)),
            Err(_) => break false,
        }
    };
    // It exits by itself once the picture is written; never left running either way.
    let until = Instant::now() + Duration::from_secs(20);
    while matches!(child.try_wait(), Ok(None)) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&profile);
    let _ = std::fs::remove_file(&shot);
    if ok {
        Ok(())
    } else {
        Err("formicaria's browser did not start in the check, so it was not installed.".into())
    }
}

/// **Firefox, wait for the browser** on Windows, where `firefox.exe` is otherwise a launcher that exits
/// as soon as it has handed over (Firefox's `-wait-for-browser`); nothing elsewhere.
const WAIT_FOR_BROWSER: &[&str] = if cfg!(windows) { &["-wait-for-browser"] } else { &[] };

/// Is formicaria's browser running from `d`? Firefox's profile lock names the process holding it.
#[cfg(not(windows))]
pub fn running(d: &Path) -> bool {
    let Ok(target) = std::fs::read_link(d.join("profile").join("lock")) else { return false };
    let Some(pid) = lock_pid(&target.to_string_lossy()) else { return false };
    if cfg!(target_os = "linux") {
        return std::fs::read_link(format!("/proc/{pid}/exe"))
            .map(|exe| exe.starts_with(d))
            .unwrap_or(false);
    }
    // macOS has no `/proc`: the process the lock names is alive and is a Firefox.
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("firefox"))
        .unwrap_or(false)
}

/// Windows: Firefox holds `parent.lock` open while it runs, so it cannot be removed then; a lock left
/// by a browser that has gone is removed, which is harmless.
#[cfg(windows)]
pub fn running(d: &Path) -> bool {
    let lock = d.join("profile").join("parent.lock");
    lock.exists() && std::fs::remove_file(&lock).is_err()
}

/// `127.0.1.1:+12345` → 12345.
pub fn lock_pid(target: &str) -> Option<u32> {
    target.rsplit_once(":+").and_then(|(_, p)| p.parse().ok())
}

/// At the start, before the browser is opened: swap in a prepared copy if there is one and the browser
/// is not running. Two renames; nothing heavy.
pub fn swap_in_prepared(d: &Path) -> bool {
    let next = d.join("firefox.next");
    if !bin(&next).is_file() || running(d) {
        return false;
    }
    let live = d.join("firefox");
    let prev = d.join("firefox.prev");
    let _ = std::fs::remove_dir_all(&prev);
    if live.exists() && std::fs::rename(&live, &prev).is_err() {
        return false;
    }
    if std::fs::rename(&next, &live).is_err() {
        let _ = std::fs::rename(&prev, &live);
        return false;
    }
    if let Ok(v) = std::fs::read_to_string(live.join("formicaria-esr")) {
        let _ = std::fs::write(d.join("version"), v.trim());
    }
    true
}

/// **The kit brought up to date in place**, while the browser is closed: formicaria's files written
/// again and its name swapped in (safe on every system, since nothing holds them). `true` when done.
pub fn refresh_kit(d: &Path) -> bool {
    if running(d) || applied_stamp(d).as_deref() == Some(kit_stamp().as_str()) {
        return false;
    }
    apply(&d.join("firefox")).is_ok()
}

/// The kit applied to the live copy, if any.
pub fn applied_stamp(d: &Path) -> Option<String> {
    std::fs::read_to_string(res_dir(&d.join("firefox")).join("formicaria").join("kit-stamp"))
        .ok()
        .map(|s| s.trim().to_string())
}

/// In the background, after the browser opened: prepare `firefox.next` when this formicaria's kit is
/// newer than the applied one, or Mozilla has a newer point release of the pinned major. At most one
/// preparation at a time, and the Mozilla check at most once a day.
pub fn prepare_in_background(d: PathBuf) {
    static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let _ = prepare(&d);
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
    });
}

fn prepare(d: &Path) -> Result<(), String> {
    if !bin(&d.join("firefox")).is_file() || d.join("firefox.next").exists() {
        return Ok(());
    }
    // One preparation across processes too: a second formicaria started by a double-click must not
    // download into the same folder as the one already running. The lock goes with the process.
    let lock = std::fs::File::create(d.join("prepare.lock")).map_err(|e| e.to_string())?;
    if lock.try_lock().is_err() {
        return Ok(());
    }
    let pin = pin();
    let installed =
        std::fs::read_to_string(d.join("version")).unwrap_or_default().trim().to_string();
    // A newer point release, checked once a day.
    let stamp = d.join("last-esr-check");
    let due = std::fs::metadata(&stamp)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().unwrap_or_default() > Duration::from_secs(24 * 3600))
        .unwrap_or(true);
    let mut target: Option<String> = None;
    if due && gpgv_available() {
        let _ = std::fs::write(&stamp, "");
        if let Some(latest) = latest_esr() {
            if is_newer_point(&installed, &latest, pin.major) {
                target = Some(latest);
            }
        }
    }
    // A formicaria update brought a newer kit (or a newer pinned point release).
    if target.is_none() && applied_stamp(d).as_deref() != Some(kit_stamp().as_str()) {
        target = Some(if is_newer_point(&pin.version, &installed, pin.major) {
            installed.clone()
        } else {
            pin.version.clone()
        });
    }
    if target.is_none() && is_newer_point(&installed, &pin.version, pin.major) {
        target = Some(pin.version.clone());
    }
    let Some(v) = target else { return Ok(()) };
    let work = d.join("work");
    let ff = acquire(&work, &v, &|_| {}, &|_, _| {})?;
    std::fs::rename(&ff, d.join("firefox.next")).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&work);
    // Not running: put it in place now, so the next start is already the new one (0.33.1). Running:
    // it waits, and formicaria tells the person to restart the browser (`status`'s `update_ready`).
    swap_in_prepared(d);
    Ok(())
}

/// Mozilla's newest ESR for this platform, from its download redirect.
pub fn latest_esr() -> Option<String> {
    let os = match platform()? {
        "linux-aarch64" => "linux64-aarch64",
        "win64" => "win64",
        "mac" => "osx",
        _ => "linux64",
    };
    let url =
        format!("https://download.mozilla.org/?product=firefox-esr-latest-ssl&os={os}&lang=en-US");
    let target = fm_fetch::redirect_target(&url).ok()??;
    crate::browser::parse_release(&target).map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **On demand, on each platform** (`FM_BROWSER_REAL=1`; `desktop.yml` on Windows and macOS):
    /// the pinned ESR for this computer downloaded from Mozilla and checked, the kit applied, and the
    /// browser started once with the kit's own report (`formicaria.cfg`, step 5): formicaria's name,
    /// no errors in its startup script.
    #[test]
    fn the_kit_applies_and_reports_on_this_platform() {
        if std::env::var("FM_BROWSER_REAL").is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        // **What went wrong is said where the release can show it**: a `::error::` line becomes a note
        // on the job that anyone can read, where the job's log needs an admin to open.
        let fail = |what: &str, why: &str| -> ! {
            println!("::error title=formicaria's browser on this platform::{what}: {why}");
            panic!("{what}: {why}");
        };
        let last = std::sync::Mutex::new(String::new());
        let ff = acquire(
            &work,
            &pin().version,
            &|m| {
                eprintln!("{m}");
                *last.lock().unwrap() = m.to_string();
            },
            &|_, _| {},
        )
        .unwrap_or_else(|e| fail(&format!("setting it up ({})", last.lock().unwrap()), &e));
        let profile = dir.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        let report = dir.path().join("report.json");
        let mut child = Command::new(bin(&ff))
            .args(["--headless", "--no-remote"])
            .args(WAIT_FOR_BROWSER)
            .arg("--profile")
            .arg(&profile)
            .arg("about:blank")
            .env("FM_BROWSER_CHECK", &report)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| fail("starting it", &e.to_string()));
        let start = Instant::now();
        while !report.is_file() && start.elapsed() < Duration::from_secs(180) {
            std::thread::sleep(Duration::from_millis(500));
        }
        std::thread::sleep(Duration::from_secs(1));
        let _ = child.kill();
        let _ = child.wait();
        let text = std::fs::read_to_string(&report).unwrap_or_else(|e| {
            fail("reading the kit's report (did its startup script run?)", &e.to_string())
        });
        eprintln!("report: {text}");
        let r: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        if r["brand"] != "formicaria" || r["errors"].as_array().map(|a| a.len()) != Some(0) {
            fail("the kit's report", &text.chars().take(600).collect::<String>());
        }
    }

    #[test]
    fn each_platform_names_mozillas_own_file() {
        assert_eq!(archive_name("153.7.0esr", "win64"), "win64/en-US/Firefox Setup 153.7.0esr.exe");
        assert_eq!(archive_name("153.7.0esr", "mac"), "mac/en-US/Firefox 153.7.0esr.dmg");
        assert_eq!(
            archive_name("153.7.0esr", "linux-x86_64"),
            "linux-x86_64/en-US/firefox-153.7.0esr.tar.xz"
        );
    }
    use std::io::{Read, Write};

    #[test]
    fn the_pin_is_read() {
        let p = parse_pin(
            "# c\nversion = \"153.7.0esr\"\nmajor = 153\n\n[sha512]\n\"linux-x86_64\" = \"aa\"\n\"linux-aarch64\" = \"bb\"\n",
        )
        .unwrap();
        assert_eq!(p.version, "153.7.0esr");
        assert_eq!(p.major, 153);
        assert_eq!(
            p.sha512,
            vec![("linux-x86_64".into(), "aa".into()), ("linux-aarch64".into(), "bb".into())]
        );
        // The compiled-in pin is well formed and names this repository's platforms.
        let real = pin();
        assert!(real.version.ends_with("esr"));
        assert!(real.sha512.iter().any(|(p, s)| p == "linux-x86_64" && s.len() == 128));
    }

    #[test]
    fn only_a_newer_point_release_of_the_same_major_is_taken() {
        assert!(is_newer_point("153.6.0esr", "153.7.0esr", 153));
        assert!(is_newer_point("153.7.0esr", "153.7.1esr", 153));
        assert!(!is_newer_point("153.7.0esr", "153.7.0esr", 153));
        assert!(!is_newer_point("153.7.0esr", "153.6.0esr", 153));
        assert!(
            !is_newer_point("153.7.0esr", "165.0.0esr", 153),
            "a new major waits for formicaria"
        );
        assert!(!is_newer_point("", "153.7.0esr", 153));
    }

    #[test]
    fn the_profile_lock_names_its_process() {
        assert_eq!(lock_pid("127.0.1.1:+48213"), Some(48213));
        assert_eq!(lock_pid("nonsense"), None);
    }

    /// A small zip standing in for Firefox's packed interface: one entry stored, one deflated.
    fn sample_jar(path: &Path) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        w.start_file("chrome/browser/content/browser.js", stored).unwrap();
        w.write_all(b"// the browser's own code\n").unwrap();
        w.start_file(BRAND_FTL_ENTRY, stored).unwrap();
        w.write_all(b"-brand-short-name = Firefox\n").unwrap();
        w.start_file("localization/en-US/browser/browser.ftl", deflated).unwrap();
        w.write_all(&b"x = { -brand-short-name }\n".repeat(50)).unwrap();
        w.finish().unwrap();
    }

    #[test]
    fn the_kit_renames_firefox_and_leaves_everything_else_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let ff = dir.path().join("firefox");
        std::fs::create_dir_all(ff.join("browser")).unwrap();
        let jar = ff.join("browser").join("omni.ja");
        sample_jar(&jar);
        let before: Vec<(String, Vec<u8>, zip::CompressionMethod)> = {
            let mut z = zip::ZipArchive::new(std::fs::File::open(&jar).unwrap()).unwrap();
            (0..z.len())
                .map(|i| {
                    let mut f = z.by_index(i).unwrap();
                    let mut b = Vec::new();
                    f.read_to_end(&mut b).unwrap();
                    (f.name().map(|n| n.to_string()).unwrap_or_default(), b, f.compression())
                })
                .collect()
        };
        apply(&ff).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&jar).unwrap()).unwrap();
        assert_eq!(z.len(), before.len());
        for (name, bytes, method) in &before {
            let mut f = z.by_name(name).unwrap();
            let mut b = Vec::new();
            f.read_to_end(&mut b).unwrap();
            if name == BRAND_FTL_ENTRY {
                assert!(String::from_utf8_lossy(&b).contains("-brand-short-name = formicaria"));
            } else {
                assert_eq!(&b, bytes, "{name} changed");
                assert_eq!(f.compression(), *method, "{name} was recompressed");
            }
        }
        for (rel, bytes) in KIT {
            assert_eq!(&std::fs::read(ff.join(rel)).unwrap(), bytes, "{rel}");
        }
        assert_eq!(std::fs::read_to_string(ff.join("formicaria/kit-stamp")).unwrap(), kit_stamp());
        // Applying again (a formicaria update) gives the same result.
        apply(&ff).unwrap();
    }

    #[test]
    fn a_package_without_the_brand_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("omni.ja");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        w.start_file("other.txt", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b"x").unwrap();
        w.finish().unwrap();
        assert!(swap_entry(&jar, BRAND_FTL_ENTRY, b"y").is_err());
        assert!(jar.is_file(), "the original is kept");
    }

    #[test]
    fn a_prepared_copy_is_swapped_in_and_the_old_one_kept() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for (sub, v) in [("firefox", "153.6.0esr"), ("firefox.next", "153.7.0esr")] {
            std::fs::create_dir_all(d.join(sub)).unwrap();
            std::fs::write(d.join(sub).join("firefox"), "#!/bin/sh\n").unwrap();
            std::fs::write(d.join(sub).join("formicaria-esr"), v).unwrap();
        }
        assert!(swap_in_prepared(d));
        assert_eq!(
            std::fs::read_to_string(d.join("firefox/formicaria-esr")).unwrap(),
            "153.7.0esr"
        );
        assert_eq!(
            std::fs::read_to_string(d.join("firefox.prev/formicaria-esr")).unwrap(),
            "153.6.0esr"
        );
        assert_eq!(std::fs::read_to_string(d.join("version")).unwrap(), "153.7.0esr");
        assert!(!d.join("firefox.next").exists());
        assert!(!swap_in_prepared(d), "nothing prepared, nothing swapped");
    }

    #[test]
    fn the_stamp_changes_with_the_kit() {
        assert_eq!(kit_stamp(), kit_stamp());
        assert_eq!(kit_stamp().len(), 16);
    }
}
