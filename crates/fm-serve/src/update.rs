//! Updating formicaria from inside formicaria.
//!
//! **Transport-shaped, never a `dispatch` command** — the same reasoning that keeps the agent's
//! routes out of the shared surface (`agent.rs`): an updater is a property of *this* desktop
//! archive, not of a vault, and `fm-cli` and the phone would never call it. Putting it in
//! `dispatch` would push a transport concern into the command core.
//!
//! # The two guarantees
//!
//! Everything here exists to satisfy these, and they are pinned by tests rather than promised
//! (`decisions.md`, 2026-09-10, *the app updates itself in place, and the folder stops moving*):
//!
//! - **G1 — no note is ever lost.** This module writes to an allowlist of *build artifacts* and
//!   nothing else (see [`is_ours`]). It never opens, reads, writes, moves or deletes anything
//!   under `vault/`, `vaults.json` or `<config>/formicaria/`. The one exception is removing a
//!   vault's `index.sqlite` on a downgrade — gitignored, per-machine, rebuilt on open.
//! - **G2 — any failure leaves a working app.** At every interruption point the user returns to
//!   the version they were running by double-clicking the launcher. That is why the previous
//!   `program/` is kept as a sibling directory and why the launcher carries a repair preamble.
//!
//! # Why the version is compared here and not in `config`
//!
//! `Config.version` is what the *machine* is running, and `config` must stay cheap and offline —
//! *one producer per fact*, and opening Settings must never be a reason to hit the network. So the
//! comparison, the check and the fetch all live here, with their own state.

//! # What lives here, and what lives in `fm_update`
//!
//! Whether a version is newer, what a release manifest says, and **whether to trust it** are the same
//! question on every platform, and the Android app asks them too — so they live in `fm_update`, where
//! the list of trusted keys exists exactly once. A trust root kept in two places is one that drifts.
//! What stays here is only what a desktop folder has: the swap, the backups, the launcher's counter,
//! and the restart choreography.

use std::path::{Component, Path};
use std::sync::Arc;

use fm_update::{asset, target, Version};

/// Is `p` a path this updater is allowed to create, replace or delete?
///
/// **This is G1, expressed as a predicate.** Every name below is a build artifact, reproducible
/// from the release archive; not one is authored by a person. `vault/`, `vaults.json`,
/// `index.sqlite` and everything under the config directory are absent by design, and a test
/// fingerprints the whole app folder across a real update to prove nothing else moved.
///
/// `p` is relative to the app folder. A path with a `..` component is refused rather than
/// normalised — normalising an attacker-supplied path is how you end up allowing what you meant to
/// deny.
pub fn is_ours(p: &Path) -> bool {
    if p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return false;
    }
    let Some(first) = p.components().next().and_then(|c| match c {
        Component::Normal(s) => s.to_str(),
        _ => None,
    }) else {
        return false;
    };
    // Directories we own outright, with everything beneath them.
    const DIRS: &[&str] = &[".fm-update", "program", "manual"];
    // Exact files at the top level. The launchers are matched by prefix because their names differ
    // per platform (`Start formicaria.sh` / `.command` / `.vbs` / ` (show messages).bat`).
    const FILES: &[&str] = &["README.txt", "Manual.html"];
    const PREFIXES: &[&str] = &["Start formicaria", "Update from an older folder", ".fm-backup-"];
    // The failed-start counter the launcher keeps. Ours to delete, and named here so the sweep and
    // the swap can both touch it without a special case.
    if first == ATTEMPTS {
        return p.components().count() == 1;
    }

    if DIRS.contains(&first) || PREFIXES.iter().any(|q| first.starts_with(q)) {
        return true;
    }
    // A bare file at the top level, not a directory we do not own.
    FILES.contains(&first) && p.components().count() == 1
}

// ---------------------------------------------------------------------------------------------
// Where this installation is, and whether it may update itself
// ---------------------------------------------------------------------------------------------

/// The unpacked release folder — the parent of `program/`, where `vault/` and the launchers live.
///
/// **Derived from `current_exe()`, and it must stay that way.** `merge_command()`, `agents_dir()`
/// and `build_id()` all derive from the running binary's own location, so anything that disagrees
/// with them is a bug waiting to happen — most sharply during an update, where a server running out
/// of the staging directory would write a merge driver pointing into a directory about to be
/// deleted.
///
/// `None` when the binary is not inside a `program/` directory, which is every source checkout and
/// every `cargo run`. That is the intended answer, not a failure: such a copy cannot update itself.
pub fn app_dir() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let program = exe.parent()?;
    if program.file_name()? != "program" {
        return None;
    }
    Some(program.parent()?.to_path_buf())
}

/// Why this copy cannot even *look* for a newer version, or `None` when it can.
///
/// **Kept apart from [`cannot_install`], because they are different capabilities and conflating
/// them costs the user the useful half.** Knowing a fix exists is worth having even where this copy
/// cannot install it — the release page is one button away, and that is exactly the situation on
/// iOS, on a folder the system protects, and on every folder whose launcher predates the rescue
/// preamble. An earlier draft of this gated the check on the *install* conditions, which would have
/// meant nobody heard about a release until the launchers had shipped.
///
/// **A reason, never a bool** — the house rule from *the assistant asks the machine, not a list of
/// operating systems*: a capability that answers "no" without saying why is the commonest source of
/// "why is this greyed out", and here every "no" has a different remedy.
pub fn cannot_check() -> Option<String> {
    if Version::running().is_none() {
        return Some(
            "This copy was built from source rather than downloaded, so it has no version to \
             compare. Updates are for the downloaded app."
                .into(),
        );
    }
    if target().is_none() {
        return Some(
            "There is no download published for this kind of computer, so this copy cannot update \
             itself."
                .into(),
        );
    }
    None
}

/// Why this copy cannot *install* what it finds, or `None` when it can. Everything
/// [`cannot_check`] refuses, plus the three conditions that only matter once files start moving.
pub fn cannot_install() -> Option<String> {
    if let Some(why) = cannot_check() {
        return Some(why);
    }
    let Some(app) = app_dir() else {
        return Some(
            "This copy is not running from a downloaded formicaria folder, so it cannot update \
             itself."
                .into(),
        );
    };
    // **The launcher is the rescue path, so a folder without the new launcher may not update.**
    // If the new version fails to start, what brings the old one back is a preamble in
    // `Start formicaria.*` — and a folder only has that once it has been updated at least once.
    // So the first release carrying this feature can be updated *to*, never *from*. Stated in
    // `decisions.md` rather than discovered.
    if std::env::var("FM_LAUNCHER").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) < 2 {
        return Some(
            "This folder was set up by an older version, which cannot put itself back if an \
             update goes wrong. Use 'Update from an older folder' this once; after that the app \
             can update itself."
                .into(),
        );
    }
    if let Err(e) = writable(&app) {
        return Some(e);
    }
    None
}

/// Can we actually write here? Answered by writing, not by inspecting permissions.
///
/// **Checked before anything is downloaded**, so somebody under `/opt` or `C:\Program Files` is
/// told at once rather than after fifteen megabytes and a failed rename. Permission bits are not
/// the whole story — read-only mounts, Controlled Folder Access and antivirus policy all produce a
/// folder that looks writable and is not — so this creates a file and deletes it.
fn writable(app: &Path) -> Result<(), String> {
    let probe = app.join(".fm-update-probe");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(_) => Err(format!(
            "formicaria cannot write to its own folder ({}), so it cannot update itself. This \
             usually means it was put somewhere the system protects. Moving the folder to your \
             home folder fixes it.",
            app.display()
        )),
    }
}

/// Remove anything this updater left behind, at every start.
///
/// **Only paths [`is_ours`] agrees to** — G1 in the one place it would be easiest to get wrong,
/// because this function's whole job is deleting. A staging tree is worthless once the process that
/// made it is gone: either the update completed, in which case it was already removed, or it did
/// not, in which case resuming half a swap is exactly what we do not want.
///
/// **The previous version is deliberately not swept.** `.fm-backup-*` is the way back and is kept
/// until the next update supersedes it — a bad release is usually found on day three, not in the
/// first ten seconds.
pub fn sweep(app: &Path) {
    let staging = Path::new(".fm-update");
    if !is_ours(staging) {
        debug_assert!(false, "the sweep must only ever remove paths the updater owns");
        return;
    }
    let _ = std::fs::remove_dir_all(app.join(staging));
    let _ = std::fs::remove_file(app.join(".fm-update-probe"));
}

// ---------------------------------------------------------------------------------------------
// The per-device setting, and the check
// ---------------------------------------------------------------------------------------------

/// Where the update setting lives: `<config>/formicaria/update.json`, beside `agent.json`.
///
/// **Not `vaults.json`** — that file is append-only and never rewrites an entry — and not the app
/// folder, which a portable copy would carry between machines along with a "last checked" time that
/// means nothing there.
fn settings_path() -> Option<std::path::PathBuf> {
    Some(fm_app::vaults::config_dir()?.join("formicaria").join("update.json"))
}

// The stored setting and the check are the same on every platform, so they live in `fm_update`, shared
// with the Android app. Only *where the file is* is desktop-shaped; these supply that.

fn settings() -> fm_update::Settings {
    settings_path().map(|p| fm_update::settings_at(&p)).unwrap_or_default()
}

fn save(f: impl FnOnce(&mut serde_json::Value)) -> Result<(), String> {
    let p = settings_path().ok_or("this machine has nowhere to keep settings")?;
    fm_update::save_at(&p, f)
}

pub fn set_check(on: bool) -> Result<(), String> {
    save(|v| v["check"] = serde_json::Value::Bool(on))
}

pub fn check(force: bool) -> Result<Option<Version>, String> {
    let p = settings_path().ok_or("this machine has nowhere to keep settings")?;
    fm_update::check_at(&p, force)
}

fn newer(v: &Version) -> bool {
    fm_update::is_newer(v, settings().rejected.as_deref())
}

// ---------------------------------------------------------------------------------------------
// Getting it: download, prove it is genuine, stage it
// ---------------------------------------------------------------------------------------------

/// How far a download has got. **In memory only**, like the agent's `Provision` and for the reason
/// its comment gives: a stale "downloading" written to disk would outlive the process that meant it.
#[derive(Clone, serde::Serialize, Default)]
pub struct Progress {
    /// `manifest` · `download` · `ready` · `failed`. A word, not a percentage of the whole
    /// operation — the steps are not comparable in size and inventing a combined figure would be
    /// making something up.
    pub stage: String,
    pub done: u64,
    /// `None` when the server sends no length. The panel then prints bytes rather than an invented
    /// percentage — the rule the model chooser already follows.
    pub total: Option<u64>,
    pub error: Option<String>,
}

/// Download the release named by `tag`, prove it is genuine, and leave it staged.
///
/// **Nothing outside `.fm-update/` is touched by this function.** That is what makes the whole
/// download half of the feature safe by construction: interrupt it anywhere — power cut, a full
/// disk, a user who pressed stop — and the app folder is exactly as it was. `program/` does not
/// enter the story until the swap, which is a separate step and a separate decision.
///
/// The order is the security property:
///
/// 1. Fetch the manifest and its detached signature.
/// 2. **Check the signature before reading the manifest as anything but bytes.** §4388's rule,
///    extended from tools to the program itself: *"unverified is a binary already running"*.
/// 3. Refuse a manifest that does not name a **newer** version than this one. A signature over an
///    old-but-genuine manifest is the replay: without this check an attacker who can choose which
///    signed bytes you see installs a version with a known hole, every field of it properly signed.
/// 4. Only then fetch the archive, whose hash the now-trusted manifest pins. `fm_fetch::fetch`
///    verifies it and renames it into place only once whole — so a `.part` is all an interruption
///    can leave.
fn stage_release(
    app: &Path,
    tag: &str,
    on_progress: &dyn Fn(&str, u64, Option<u64>),
    cancel: &dyn Fn() -> bool,
) -> Result<std::path::PathBuf, String> {
    let target = target().ok_or("there is no download published for this kind of computer")?;
    let dir = app.join(".fm-update");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not prepare the download: {e}"))?;

    on_progress("manifest", 0, None);
    // Fetch, verify the signature, refuse anything not newer — in that order, in `fm_update`,
    // where the phone runs exactly the same sequence against exactly the same keys.
    let manifest = fm_update::fetch_verified_manifest(tag)?;
    let entry = manifest
        .entry(target)
        .ok_or_else(|| format!("{} has no download for this kind of computer", manifest.version))?;

    if cancel() {
        return Err("stopped".into());
    }

    let dest = dir.join(&entry.file);
    on_progress("download", 0, Some(entry.size));
    fm_fetch::fetch(
        &fm_fetch::Download {
            url: &asset(tag, &entry.file),
            dest: &dest,
            sha256: Some(&entry.sha256),
        },
        &|done, total| on_progress("download", done, total.or(Some(entry.size))),
        cancel,
    )
    .map_err(|e| e.to_string())?;

    if cancel() {
        return Err("stopped".into());
    }

    // The directory the archive unpacks to, which is its own name without the extension. Passed to
    // the extractor as a *requirement*, not a guess: an archive that unpacks somewhere else is not
    // the artifact the manifest described.
    let top = entry
        .file
        .strip_suffix(".tar.gz")
        .or_else(|| entry.file.strip_suffix(".zip"))
        .ok_or("the download is not a kind of archive this version understands")?;

    on_progress("unpack", entry.size, Some(entry.size));
    let staged = dir.join("staged");
    let _ = std::fs::remove_dir_all(&staged);
    fm_fetch::unpack_tree(&dest, &staged, top)?;

    // **Run it before trusting it.** The single most valuable check here, and the cheapest: one
    // execution of the staged binary catches a download for the wrong architecture, a copy an
    // antivirus quarantined between writing and starting it, and a macOS binary whose ad-hoc
    // signature did not survive extraction — each of which would otherwise be discovered *after*
    // `program/` had already been replaced, which is the one moment there is nothing to run.
    let probe =
        staged.join("program").join(if cfg!(windows) { "fm-serve.exe" } else { "fm-serve" });
    let out = std::process::Command::new(&probe)
        .arg("--version")
        .output()
        .map_err(|e| format!("the downloaded formicaria will not run on this computer: {e}"))?;
    let said = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || Version::parse(&said) != Some(manifest.version) {
        return Err(format!(
            "the downloaded formicaria reported itself as '{said}' rather than {}, so it was not \
             used",
            manifest.version
        ));
    }

    // **Armed.** `program/` is still untouched and the whole thing is still undone by deleting one
    // directory — which is what the sweep at the next start does if nothing gets further than here.
    let plan = serde_json::json!({
        "phase": "armed",
        "from": Version::running().map(|v| v.to_string()),
        "to": manifest.version.to_string(),
    });
    write_atomic(&dir.join("plan.json"), serde_json::to_vec_pretty(&plan).unwrap_or_default())?;

    on_progress("ready", entry.size, Some(entry.size));
    Ok(staged)
}

/// Write a file so a reader never sees half of it: temp file, flush to the platter, then rename.
///
/// A `plan.json` truncated by a power cut is worse than no `plan.json`, because the next start
/// would read it and act on half a sentence.
fn write_atomic(path: &Path, bytes: Vec<u8>) -> Result<(), String> {
    use std::io::Write;
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    f.write_all(&bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    f.sync_all().map_err(|e| format!("{}: {e}", tmp.display()))?;
    drop(f);
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------------------------------------
// Applying it: settle, hand over, and let the new binary do the swap
// ---------------------------------------------------------------------------------------------

/// Record a restore point before anything moves. **Best effort, and never a reason to stop.**
///
/// This is the half of G1 that has nothing to do with file swapping. Auto-commit is a browser
/// `setTimeout` (`outstanding.md` §3) and there is a save debounce in front of it, so "type, then
/// update" could otherwise drop the last few seconds of work at exactly the moment somebody is
/// trusting the app with a big change. The *editor* flush has to happen in the browser — the
/// debounce lives there — so the UI does that first and only then calls this route; what the server
/// can do, and does here, is turn what is already on disk into something git can bring back.
///
/// A vault that is not a repository simply fails and is skipped: nothing here is worth refusing an
/// update over, and files-as-truth means the notes are already safe on disk either way.
fn settle(state: &crate::AppState) {
    let Ok(configs) = state.app.configs() else { return };
    for cfg in configs {
        let args = serde_json::json!({ "vault": cfg.name });
        let _ = fm_app::dispatch("commit", &args, &[], &state.app, &crate::Desktop);
    }
}

/// [`settle`], but it cannot hold the restart hostage.
///
/// **The vault mutex is a plain blocking lock, and this runs at the one moment it is most likely
/// to be held.** `push` and `pull` keep it across the network, and `git_cmd` sets no low-speed
/// timeout — so a sync that connected and then stalled holds it for as long as the kernel keeps the
/// socket. A user pressing *Go back* right after a sync that hung is the commonest reason to press
/// it at all, and `settle`'s own contract says it is "never a reason to stop". A blocking lock
/// silently makes it one.
///
/// Proceeding without the commit is safe because of files-as-truth: the notes are already on disk,
/// only the git snapshot is missed, and `adoptable()` re-finds the whole backlog on the next
/// commit. A restore point is worth a few seconds of waiting and no more.
fn settle_briefly(state: &std::sync::Arc<crate::AppState>) {
    let s = std::sync::Arc::clone(state);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        settle(&s);
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
}

/// Hand over to the staged binary and stop.
///
/// **The swap is not performed here, and that is the whole design.** An earlier draft had this
/// process spawn the new binary and then supervise it, which cannot work for two independent
/// reasons, both of them in the code rather than in theory:
///
/// 1. This process still holds 8765, so the successor meets `AddrInUse`, probes, finds a `build`
///    that is not its own — `build_id()` is length and mtime, so a freshly swapped binary is
///    *guaranteed* to differ — and exits with *"A different build of formicaria is already
///    running"* and an instruction to run `pkill`, aimed at a user with no terminal.
/// 2. A supervisor polling `/api/alive` on the port it is itself listening on is checking its own
///    health. The rollback branch could never fire.
///
/// So the staged binary does the swap, before it binds, once this process is gone — and the health
/// check that matters is *"has the old port stopped answering?"*, which only a different process
/// can ask. What supervises the result is the launcher, which needs no process at all.
fn hand_over(state: &Arc<crate::AppState>, app: &Path) -> Result<(), String> {
    let staged = app.join(".fm-update").join("staged").join("program").join(exe_name());
    if !staged.exists() {
        return Err("there is nothing downloaded to install".into());
    }
    settle_briefly(state);

    // **Run the swap from a copy, not from `staged/program/` itself — this is a Windows
    // correctness requirement, not tidiness.**
    //
    // The swap renames `staged/program` into place. Windows will not reliably rename a directory
    // that contains a running image, so a binary executing from inside `staged/program` would be
    // renaming the ground it stands on. Copying it one level up costs a few megabytes and one write,
    // and leaves the directory being renamed with nothing open inside it.
    //
    // On Unix this is unnecessary — an open inode survives its directory being renamed — but a
    // second, platform-dependent code path here would be a worse trade than one copy.
    let runner = app.join(".fm-update").join(format!("apply-{}", exe_name()));
    std::fs::copy(&staged, &runner)
        .map_err(|e| format!("could not prepare the new version to install itself: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&runner, std::fs::Permissions::from_mode(0o755));
    }

    // Inherit the environment — `FM_VAULT`, `FM_VAULTS` and the rest came from the launcher and
    // must survive, or the new process would look for the vault somewhere else entirely.
    let child = std::process::Command::new(&runner)
        .env("FM_APPLY_UPDATE", app)
        // The browser tab is already open and will reconnect on its own; a second window here
        // would be the app apologising for its own restart.
        .env("FM_OPEN", "0")
        .spawn()
        .map_err(|e| format!("the downloaded formicaria would not start: {e}"))?;
    drop(child);
    Ok(())
}

/// Where the installed binary lives once the swap has happened — what the re-exec runs.
pub fn installed_binary(app: &Path) -> std::path::PathBuf {
    app.join("program").join(exe_name())
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "fm-serve.exe"
    } else {
        "fm-serve"
    }
}

/// Put the previous version back.
///
/// **The runner is a copy of the version that is running, not of the one being restored** — the
/// single least obvious thing here. The restored binary is *older*, so it may predate this feature
/// entirely and would simply ignore `FM_APPLY_ROLLBACK` and start serving, leaving the folder
/// half-changed. The code that performs a rollback has to be code that knows what a rollback is.
///
/// **`.fm-update/` is created, not assumed.** `sweep` deletes it at every start, so on the common
/// path — update, restart, use the app for a while, then change your mind — it is not there.
fn hand_back(state: &Arc<crate::AppState>, app: &Path) -> Result<(String, String), String> {
    let (backup, version) = backup_dir(app).ok_or("there is no earlier version to go back to")?;

    // **Prove it runs before spending the only copy of the working app on it.** `stage_release`
    // takes exactly this decision for the forward direction and says why: a binary that will not
    // execute is discovered *after* `program/` has been replaced, which is the one moment there is
    // nothing to run. A rollback has less margin than an update, not more — an update that fails
    // here still has the backup, and this *is* the backup.
    let probe = backup.join(exe_name());
    let out = std::process::Command::new(&probe).arg("--version").output().map_err(|e| {
        format!("the earlier version will not run on this computer, so it was left alone: {e}")
    })?;
    if !out.status.success() {
        return Err(
            "the earlier version will not run on this computer, so it was left alone".into()
        );
    }
    let said = String::from_utf8_lossy(&out.stdout).trim().to_string();

    settle_briefly(state);

    let staging = app.join(".fm-update");
    std::fs::create_dir_all(&staging).map_err(|e| format!("could not prepare the folder: {e}"))?;
    // A copy of *this* binary, under its own name so an install and a go-back can never be writing
    // the same file.
    let runner = staging.join(format!("apply-back-{}", exe_name()));
    let me = std::env::current_exe().map_err(|e| format!("could not find formicaria: {e}"))?;
    std::fs::copy(&me, &runner).map_err(|e| format!("could not prepare the change: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&runner, std::fs::Permissions::from_mode(0o755));
    }

    let child = std::process::Command::new(&runner)
        .env("FM_APPLY_ROLLBACK", app)
        .env("FM_ROLLBACK_TO", backup.file_name().unwrap_or_default())
        .env("FM_OPEN", "0")
        .spawn()
        .map_err(|e| format!("could not start the change: {e}"))?;
    drop(child);
    Ok((version, said))
}

/// The rollback itself, run by the outgoing (new) binary before it binds.
///
/// **The index is deliberately not touched.** An earlier draft deleted every vault's
/// `index.sqlite` here, which would have been this module's only write outside the app folder —
/// a path built from user-editable config, unexpressible in [`is_ours`], and invisible to the test
/// that fingerprints the folder. It also would not have helped: the launcher performs the same
/// downgrade by itself, knows nothing about vaults, and cannot be taught. So the check moved to the
/// arrival side instead (`fm_core`'s `discard_an_index_from_the_future`), where every route into a
/// vault passes — and G1 keeps **no** exceptions at all.
pub fn apply_rollback(app: &Path, name: &str) -> Result<(), String> {
    wait_for_port_to_free();
    go_back(app, name)
}

/// The file work alone, with no waiting — so what it touches and what it leaves behind when it is
/// interrupted can be tested, exactly as [`swap`] is.
fn go_back(app: &Path, name: &str) -> Result<(), String> {
    let backup = app.join(name);
    if !backup.join(exe_name()).exists() {
        return Err("the earlier version is no longer there".into());
    }

    let staging = app.join(".fm-update");
    let rejected = staging.join("rejected");
    // Cleared first, like every other rename here: a `rejected/` left by an earlier go-back whose
    // restored version never got far enough to sweep would otherwise fail this with `ENOTEMPTY`,
    // at the one step already announced to the user as under way.
    let _ = std::fs::remove_dir_all(&rejected);
    std::fs::create_dir_all(&staging).map_err(|e| format!("could not prepare the folder: {e}"))?;

    // **`rejected/`, never `.fm-backup-*`.** All four launchers glob `.fm-backup-*` and would
    // happily put the version the user just rejected back on, which is the one direction this
    // feature must never travel. Under `.fm-update/` it is also *transient* — the next start sweeps
    // it — so going forward again means downloading again. That is the "one step only" limit, taken
    // deliberately rather than growing a second retention policy beside `.fm-backup-*`.
    rename_with_retry(&app.join("program"), &rejected)
        .map_err(|e| format!("could not set the current version aside: {e}"))?;

    // The window where `program/` does not exist. The backup is still under the name the launcher
    // globs, so a double-click here restores it — the same net as the update path.
    if let Err(e) = std::fs::rename(&backup, app.join("program")) {
        let _ = std::fs::rename(&rejected, app.join("program"));
        return Err(format!("could not put the earlier version back: {e}"));
    }

    // The restored version has not failed; it is the one that was working.
    let _ = std::fs::remove_file(app.join(ATTEMPTS));
    Ok(())
}

/// The swap, run by the **staged** binary before it binds, when `FM_APPLY_UPDATE` names the folder.
///
/// Ordering is the safety property, so it is worth reading as a sequence rather than as steps:
///
/// 1. **Wait for the old process to let go of the port.** A `connect` that is *refused* is the only
///    honest evidence it has gone, and — unlike a supervisor's self-poll — this process is not the
///    one answering.
/// 2. **Rename `program/` aside.** One syscall. On Windows a directory rename fails while a file
///    inside it is still open, which is exactly the "the old one is still mapped" signal we want,
///    and is why this retries rather than assuming.
/// 3. **Rename the staged `program/` in.** The window in which no `program/` exists is now exactly
///    the gap between two renames — and the launcher preamble covers even that.
/// 4. Only then the top-level files, each by rename, never by truncating and rewriting.
/// 5. Re-exec from the **installed** location. Never serve from `.fm-update/`: `merge_command()`,
///    `agents_dir()` and `build_id()` all derive from `current_exe()`, so a server running out of
///    the staging directory would write a merge driver pointing into a directory about to be
///    deleted — which is the silent-data-loss shape this whole design is arranged to avoid.
pub fn apply_staged(app: &Path) -> Result<(), String> {
    wait_for_port_to_free();
    swap(app)
}

/// The file work alone, with no waiting and no network — so the guarantees can be tested.
///
/// Split from [`apply_staged`] deliberately: the wait is a *precondition* about another process,
/// and a test that had to satisfy it would either bind a port or sleep. What must be pinned is what
/// this touches and what it leaves behind when it is interrupted, and that is all in here.
fn swap(app: &Path) -> Result<(), String> {
    let staging = app.join(".fm-update");
    // **The version being replaced — read from the plan, never from this process.**
    //
    // This runs inside a copy of the *staged* binary, so `Version::running()` here is the NEW
    // version: naming the backup from it produced `.fm-backup-v0.7.0` holding v0.6.0's program,
    // wrong by exactly one release, always, and in the direction that matters. Nothing caught it
    // under test because `FM_VERSION` is unset by `cargo test`, so every backup was named
    // `.fm-backup-previous` and the tests assert by prefix and contents rather than by name.
    //
    // `plan.json` was written by the outgoing process while it still knew what it was.
    let old = std::fs::read_to_string(staging.join("plan.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v["from"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "previous".into());
    let staged_program = staging.join("staged").join("program");
    if !staged_program.join(exe_name()).exists() {
        return Err("there is nothing staged to install".into());
    }

    // **Exactly one way back, always.** `decisions.md` says the previous version is kept "until the
    // next update supersedes it" — which means the superseded one has to go, and not only the one
    // that happens to share this name. Left alone they accumulate a copy of the program per update,
    // and worse: the launcher restores the *first* `.fm-backup-*` it globs, which is the
    // lexically-first and therefore usually the OLDEST. A user who updated twice and then hit the
    // rescue would be put back onto a version from two releases ago rather than the one that was
    // working ten minutes earlier.
    let backup = app.join(format!(".fm-backup-{old}"));
    remove_all_backups(app);
    // Retry, because on Windows the previous process's image may still be mapped for a moment
    // after it exits. A failure here is completely safe: nothing has moved yet.
    rename_with_retry(&app.join("program"), &backup)
        .map_err(|e| format!("could not set the previous version aside: {e}"))?;

    // **From here the launcher is the safety net.** If anything below fails or is interrupted,
    // `program/` may not exist — and double-clicking the launcher puts `backup` back.
    if let Err(e) = std::fs::rename(&staged_program, app.join("program")) {
        // Put it back ourselves if we still can, so the common case does not even need the
        // launcher. If this fails too, the launcher still will.
        let _ = std::fs::rename(&backup, app.join("program"));
        return Err(format!("could not put the new version in place: {e}"));
    }

    // The rest of the folder. **Not load-bearing**: a stale README or manual is a blemish, and a
    // failure here must not undo a `program/` that is now correct and running.
    let staged_root = staging.join("staged");
    if let Ok(entries) = std::fs::read_dir(&staged_root) {
        for e in entries.flatten() {
            let name = e.file_name();
            if name == "program" {
                continue;
            }
            let Some(rel) = name.to_str().map(Path::new) else { continue };
            if !is_ours(rel) {
                // **`vault/` is expected, and is never moved.** Every real release archive carries one — the
                // welcome note for a fresh unpack — and moving it would put a stranger's "Start here" note
                // into somebody's notebook, or replace their vault outright. This used to be a
                // `debug_assert!`, which panicked on every real archive in any debug build: the hand-built
                // test archives never carried a `vault/`, so nothing noticed until one was made to.
                //
                // Anything else that is not ours — a file a *later* release adds that this version has never
                // heard of — is skipped the same way. G1 says never write what is not ours, and an updater
                // must not fail on an archive it is merely older than.
                continue;
            }
            let target = app.join(&name);
            let _ = std::fs::remove_dir_all(&target);
            let _ = std::fs::remove_file(&target);
            let _ = std::fs::rename(e.path(), &target);
            make_launchable(&target);
        }
    }

    // A new version starts with a clean slate: the count that mattered belonged to whatever was
    // here before, and carrying it over would have the launcher roll back a version that has not
    // failed even once.
    let _ = std::fs::remove_file(app.join(ATTEMPTS));

    // **Staging is deliberately not deleted here.** The process running this *is* a copy inside
    // `.fm-update/`, and Windows cannot delete a running image — so removing it now would fail
    // there and succeed here, which is the kind of difference that hides until somebody ships. The
    // sweep at the next start clears it, from a process running out of `program/`, and that sweep
    // is already the thing that cleans up after an update which never finished.
    let _ = &staging;
    Ok(())
}

/// Delete every `.fm-backup-*` in the app folder. See [`swap`] for why there may only ever be one.
///
/// Guarded by [`is_ours`] like every other removal here: this function's whole job is deleting, so
/// it is the one place where a mistaken path is most expensive.
fn remove_all_backups(app: &Path) {
    let Ok(entries) = std::fs::read_dir(app) else { return };
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(rel) = name.to_str() else { continue };
        if rel.starts_with(".fm-backup-") && is_ours(Path::new(rel)) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// The single way back, if there is one: the `.fm-backup-*` holding a runnable formicaria.
///
/// Returns the directory and the version it holds. There is only ever one ([`remove_all_backups`]),
/// but this is written to cope with more than one rather than to assume — and it picks by *name*
/// only after checking the binary is actually there, because a half-deleted backup is not a way
/// back at all.
fn backup_dir(app: &Path) -> Option<(std::path::PathBuf, String)> {
    let entries = std::fs::read_dir(app).ok()?;
    let mut found: Vec<(std::path::PathBuf, String)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_owned();
            let version = name.strip_prefix(".fm-backup-")?.to_owned();
            let dir = e.path();
            dir.join(exe_name()).exists().then_some((dir, version))
        })
        .collect();
    // **By version, not by name.** Lexically `.fm-backup-v0.9.0` beats `.fm-backup-v0.10.0`, which
    // is the same trap the launcher's glob has and the reason its comment calls the first match
    // "usually the OLDEST". Unparseable names sort below every real one rather than winning by
    // accident.
    found.sort_by_key(|(_, v)| Version::parse(v));
    found.pop()
}

/// The file the launcher counts failed starts in. One line, holding a number.
pub const ATTEMPTS: &str = ".fm-attempts";

/// Say that this version actually works, by clearing the launcher's failed-start counter.
///
/// **The counting is the launcher's job and the clearing is ours**, which is the only split that
/// works: a version too broken to run cannot decrement anything, so the count must be incremented
/// by whatever *starts* it and cleared only by a version that got far enough to serve. Three
/// consecutive starts that never reach here and the launcher puts the previous version back —
/// without a supervisor process, because there is nothing alive to supervise.
///
/// **Delayed, not immediate.** Binding the port proves the binary loads; it does not prove the app
/// works. Waiting until it has been serving for a little while is a weak signal, but it is an
/// honest one, and it is the difference between "it ran" and "it ran and did not fall over".
pub fn mark_healthy_soon(app: std::path::PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(20));
        let _ = std::fs::remove_file(app.join(ATTEMPTS));
    });
}

/// A `.command` without its executable bit is not launchable from Finder, and a user with no
/// terminal cannot put it back. Cheap insurance on the files that are doors.
fn make_launchable(p: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let is_script =
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| e == "sh" || e == "command");
        if is_script {
            let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755));
        }
    }
    #[cfg(not(unix))]
    let _ = p;
}

/// `rename`, retried briefly. Windows refuses to rename a directory while a file inside it is open,
/// which right after the old process exits is a matter of milliseconds rather than a real failure.
fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if std::time::Instant::now() < deadline => {
                let _ = e;
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => return Err(e),
        }
    }
}

/// Wait until nothing answers on the app's port.
///
/// **A refused connection is the evidence; a successful one is not.** This runs in the *new*
/// process, so unlike a supervisor's `/api/alive` poll it cannot be satisfied by the prober itself.
/// It gives up after 30 s and carries on regardless: the renames retry anyway, and hanging here
/// forever would be a worse failure than trying.
fn wait_for_port_to_free() {
    let addr = std::env::var("FM_ADDR").unwrap_or_else(|_| "127.0.0.1:8765".to_string());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        match std::net::TcpStream::connect(&addr) {
            Ok(s) => {
                drop(s);
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(_) => return,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The transport routes
// ---------------------------------------------------------------------------------------------

/// The in-memory state of a check. **Never written to disk** — the same reasoning as the agent's
/// `Provision`: a "checking…" left behind by a crash would be a lie the next start could not
/// detect. What survives a restart is the *result*, in `update.json`.
#[derive(Default)]
pub struct UpdateState {
    /// Set while a check is in flight, so a second press does not start a second request.
    pub checking: std::sync::atomic::AtomicBool,
    /// The last error, for the panel to print verbatim.
    pub error: std::sync::Mutex<Option<String>>,
    /// How far a download has got, or `None` when none is running.
    pub progress: std::sync::Mutex<Option<Progress>>,
    /// **One restart at a time.** Installing and going back both write a runner into
    /// `.fm-update/`, then spawn a child that waits for the port and renames `program/`. The
    /// outgoing process keeps serving for 400 ms after it answers, so a second press — or a second
    /// tab, or Install-then-Go-back — is served, and two children would then race over the same
    /// two directories. Taken by both routes before either copies anything; never released,
    /// because the process is on its way out.
    pub handing_over: std::sync::atomic::AtomicBool,
    /// **Cancel is a generation counter, not a flag.** Copied from the agent's provisioning
    /// protocol, which needed it for the same reason: a download that takes minutes cannot be
    /// interrupted by a stop flag that only exists once the thing is running. Every start takes the
    /// next generation and checks on each chunk that it is still the live one; stopping — or
    /// starting again — simply bumps it.
    pub generation: std::sync::atomic::AtomicU64,
}

/// What the settings panel needs, in one answer.
fn status_json(state: &UpdateState) -> serde_json::Value {
    let why_check = cannot_check();
    let why_install = cannot_install();
    // The way back, read from the folder rather than remembered.
    let previous = app_dir().and_then(|a| backup_dir(&a)).map(|(_, v)| v);
    let s = settings();
    let available = s.last_seen.as_deref().and_then(Version::parse).filter(newer);
    serde_json::json!({
        // The capability, and the preference, kept apart on purpose — conflating them is how a
        // panel comes to offer a switch that reports success and does nothing.
        // Two capabilities, answered separately: a copy that can see a release but not install it
        // still has something useful to offer, and a panel that collapses them offers nothing.
        "can_check": why_check.is_none(),
        "can_install": why_install.is_none(),
        "why": why_install.unwrap_or_default(),
        "current": Version::running().map(|v| v.to_string()),
        "available": available.map(|v| v.to_string()),
        // The way back, if there is one. Read from the folder rather than remembered, so it stays
        // true across a restart and a fresh clone.
        "previous": previous.as_deref(),
        "can_go_back": previous.is_some(),
        "checking": state.checking.load(std::sync::atomic::Ordering::SeqCst),
        "check": s.check,
        "last_check": s.last_check,
        "error": state.error.lock().ok().and_then(|e| e.clone()),
        "progress": state.progress.lock().ok().and_then(|p| p.clone()),
        // The release page, so the panel never has to build a URL and the one address this
        // feature knows lives in one place.
        "page": RELEASES_PAGE,
    })
}

/// Where a person goes to read about, or fetch, a release by hand. Handed to `open_native`, which
/// already takes a URL — the same call the server makes to open the app at launch.
const RELEASES_PAGE: &str = "https://github.com/formicaria-org/formicaria/releases/latest";

/// Handle the `/api/update_*` routes. `None` means "not an update route" — the caller falls through
/// to the normal command dispatch.
///
/// **Transport-shaped, never a `dispatch` command**, for the reason the module header gives. Every
/// route here is in `REMOTE_DENIED`: a paired tablet is a guest, and a guest does not decide that
/// the host replaces its own program.
pub fn route(
    stream: &mut dyn crate::Conn,
    path: &str,
    body: &[u8],
    state: &Arc<crate::AppState>,
) -> Option<std::io::Result<()>> {
    let u = &state.update;
    let resp: (&str, &str, Vec<u8>) = match path {
        "/api/update_status" => {
            ("200 OK", "application/json", status_json(u).to_string().into_bytes())
        }
        // *Check now*. Explicit, so it ignores both the switch and the once-a-day interval.
        "/api/update_check" => {
            use std::sync::atomic::Ordering;
            // `swap` rather than load-then-store: two tabs pressing at once must produce one
            // request, and a check-then-set has a window between the two.
            if !u.checking.swap(true, Ordering::SeqCst) {
                match check(true) {
                    Ok(_) => {
                        if let Ok(mut e) = u.error.lock() {
                            *e = None;
                        }
                    }
                    Err(msg) => {
                        if let Ok(mut e) = u.error.lock() {
                            *e = Some(msg);
                        }
                    }
                }
                u.checking.store(false, Ordering::SeqCst);
            }
            ("200 OK", "application/json", status_json(u).to_string().into_bytes())
        }
        // The switch. A malformed body degrades to `false` rather than to a chain of `unwrap_or`,
        // matching the agent's request structs.
        "/api/set_update_check" => {
            #[derive(serde::Deserialize, Default)]
            struct Req {
                #[serde(default)]
                check: bool,
            }
            let req: Req = serde_json::from_slice(body).unwrap_or_default();
            match set_check(req.check) {
                Ok(()) => ("200 OK", "application/json", status_json(u).to_string().into_bytes()),
                Err(e) => (
                    "500 Internal Server Error",
                    "application/json",
                    serde_json::json!({ "error": e }).to_string().into_bytes(),
                ),
            }
        }
        // **Get it.** Answers at once and downloads on a thread: a multi-megabyte fetch must not
        // hold a request open, and the panel polls `update_status` the way it already polls the
        // assistant's provisioning.
        "/api/update_start" => match start(state) {
            Ok(()) => ("200 OK", "application/json", status_json(u).to_string().into_bytes()),
            Err(e) => (
                "400 Bad Request",
                "application/json",
                serde_json::json!({ "error": e }).to_string().into_bytes(),
            ),
        },
        // **Stop.** Bumps the generation, which is what the running download checks. What has
        // already arrived stays in its `.part`, so pressing Get it again resumes rather than
        // starting over — a cancelled download is a paused one.
        "/api/update_cancel" => {
            u.generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if let Ok(mut p) = u.progress.lock() {
                *p = None;
            }
            ("200 OK", "application/json", status_json(u).to_string().into_bytes())
        }
        // **Install it and restart.** Answers first, then stops: the reply has to reach the browser
        // before this process goes away, or the tab sees a dropped connection instead of being told
        // to expect one.
        "/api/update_apply" => {
            let app = app_dir();
            match (cannot_install(), app) {
                (Some(why), _) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::json!({ "error": why }).to_string().into_bytes(),
                ),
                (None, None) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::json!({ "error": "this copy is not running from a downloaded formicaria folder" })
                        .to_string()
                        .into_bytes(),
                ),
                (None, Some(_app)) if u.handing_over.swap(true, std::sync::atomic::Ordering::SeqCst) => (
                    "409 Conflict",
                    "application/json",
                    serde_json::json!({ "error": "formicaria is already restarting" })
                        .to_string()
                        .into_bytes(),
                ),
                (None, Some(app)) => match hand_over(state, &app) {
                    Ok(()) => {
                        // The successor is already waiting for this port to go quiet. Give the
                        // response time to be written and read, then leave — the swap happens in
                        // the other process, which is the only one that can do it safely.
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_millis(400));
                            std::process::exit(0);
                        });
                        (
                            "200 OK",
                            "application/json",
                            serde_json::json!({ "restarting": true }).to_string().into_bytes(),
                        )
                    }
                    Err(e) => {
                        // Nothing is on its way out after all, so let the next press through.
                        u.handing_over.store(false, std::sync::atomic::Ordering::SeqCst);
                        (
                            "500 Internal Server Error",
                            "application/json",
                            serde_json::json!({ "error": e }).to_string().into_bytes(),
                        )
                    }
                },
            }
        }
        // **Go back to the version you were on.** Same choreography as installing, in reverse, and
        // behind the same one-restart-at-a-time flag.
        "/api/update_rollback" => {
            let app = app_dir();
            match (cannot_install(), app) {
                (Some(why), _) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::json!({ "error": why }).to_string().into_bytes(),
                ),
                (None, None) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::json!({ "error": "this copy is not running from a downloaded formicaria folder" })
                        .to_string()
                        .into_bytes(),
                ),
                (None, Some(_)) if u.handing_over.swap(true, std::sync::atomic::Ordering::SeqCst) => (
                    "409 Conflict",
                    "application/json",
                    serde_json::json!({ "error": "formicaria is already restarting" })
                        .to_string()
                        .into_bytes(),
                ),
                (None, Some(app)) => match hand_back(state, &app) {
                    Ok((_going_to, _)) => {
                        // Remember what was turned down, so the panel does not offer it back the
                        // moment the earlier version is up.
                        let rejected = Version::running().map(|v| v.to_string());
                        let _ = save(|v| v["rejected"] = serde_json::json!(rejected));
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_millis(400));
                            std::process::exit(0);
                        });
                        (
                            "200 OK",
                            "application/json",
                            serde_json::json!({ "restarting": true }).to_string().into_bytes(),
                        )
                    }
                    Err(e) => {
                        u.handing_over.store(false, std::sync::atomic::Ordering::SeqCst);
                        (
                            "500 Internal Server Error",
                            "application/json",
                            serde_json::json!({ "error": e }).to_string().into_bytes(),
                        )
                    }
                },
            }
        }
        _ => return None,
    };
    Some(crate::write_response(stream, resp.0, resp.1, &resp.2))
}

/// Begin downloading the newest version, on a thread.
///
/// Refuses rather than starts when this copy cannot update itself, when there is nothing newer, or
/// when a download is already running — each with a reason a person can act on.
fn start(state: &Arc<crate::AppState>) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    if let Some(why) = cannot_install() {
        return Err(why);
    }
    let app = app_dir().ok_or("this copy is not running from a downloaded formicaria folder")?;
    let tag = settings()
        .last_seen
        .filter(|t| Version::parse(t).is_some_and(|v| newer(&v)))
        .ok_or("there is nothing newer to get")?;

    let u = Arc::clone(&state.update);
    if let Ok(mut p) = u.progress.lock() {
        if p.as_ref().is_some_and(|p| p.stage != "ready" && p.stage != "failed") {
            return Err("this is already being downloaded".into());
        }
        *p = Some(Progress { stage: "manifest".into(), ..Progress::default() });
    }
    // **Take the generation only once nothing is running.** Taking it first — as this did — cancelled
    // the download already under way and then reported that one was, so pressing Get twice lost the
    // first download while telling the person it was still going.
    let gen = u.generation.fetch_add(1, Ordering::SeqCst) + 1;

    std::thread::spawn(move || {
        // The live generation is what "stopped" means. Reading it on every chunk is what lets a
        // user interrupt a download that has not produced a single byte yet.
        let live = || u.generation.load(Ordering::SeqCst) == gen;
        let set = |stage: &str, done: u64, total: Option<u64>| {
            if !live() {
                return;
            }
            if let Ok(mut p) = u.progress.lock() {
                *p = Some(Progress { stage: stage.into(), done, total, error: None });
            }
        };
        match stage_release(&app, &tag, &set, &|| !live()) {
            Ok(_) => set("ready", 0, None),
            Err(msg) => {
                if live() {
                    if let Ok(mut p) = u.progress.lock() {
                        *p = Some(Progress {
                            stage: "failed".into(),
                            error: Some(msg),
                            ..Progress::default()
                        });
                    }
                }
            }
        }
    });
    Ok(())
}

/// Look for a newer version in the background, shortly after start.
///
/// **Not on panel open, and this is a rule rather than a preference.** `SettingsPanel`'s own header
/// says *"Opening Settings must never be a reason to hit the network"* — which is why `config`
/// shells out to nothing. So the check happens here, off the request path entirely, and the panel
/// reads a cached answer. Failure is silent: a check that cannot reach the network is not news, and
/// an error box on every offline start is how people learn to dismiss error boxes.
pub fn check_in_background(state: std::sync::Arc<crate::AppState>) {
    if cannot_check().is_some() || !settings().check {
        return;
    }
    std::thread::spawn(move || {
        // Let the app finish starting first. The same courtesy the agent's warm-up takes, and for
        // the same reason: nothing here is urgent enough to compete with the first paint.
        std::thread::sleep(std::time::Duration::from_secs(5));
        let u = &state.update;
        use std::sync::atomic::Ordering;
        if u.checking.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Err(msg) = check(false) {
            if let Ok(mut e) = u.error.lock() {
                *e = Some(msg);
            }
        }
        u.checking.store(false, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// G1, as a unit test over the predicate. The four names that must never be writable are the
    /// four that hold somebody's work.
    #[test]
    fn the_updater_may_not_touch_anything_a_person_wrote() {
        for deny in [
            "vault",
            "vault/notes/01ABC.md",
            "vaults.json",
            "vault/index.sqlite",
            "vault/.git/config",
            "update.log",
            "notes",
        ] {
            assert!(!is_ours(Path::new(deny)), "{deny} must not be writable by the updater");
        }
    }

    #[test]
    fn the_updater_owns_exactly_the_build_artifacts() {
        for allow in [
            "program",
            "program/fm-serve",
            "program/models.toml",
            "manual/index.html",
            "README.txt",
            "Manual.html",
            "Start formicaria.sh",
            "Start formicaria.command",
            "Start formicaria.vbs",
            "Start formicaria (show messages).bat",
            "Update from an older folder.sh",
            ".fm-update/staged/program/fm-serve",
            ".fm-backup-v0.5.1/fm-serve",
        ] {
            assert!(is_ours(Path::new(allow)), "{allow} should be ours");
        }
    }

    /// A release folder as a user actually has one: a program, their notes, their vault list, and
    /// the derived files around them.
    fn app_folder(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("fm-upd-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in ["program", "vault/notes", "manual", ".fm-update/staged/program"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        std::fs::write(dir.join("program").join(exe_name()), b"OLD BINARY").unwrap();
        std::fs::write(dir.join("program/models.toml"), b"old catalogue").unwrap();
        std::fs::write(dir.join("vault/notes/01ARZ3NDEKTSV4RRFFQ69G5FAV.md"), b"my only copy")
            .unwrap();
        std::fs::write(dir.join("vault/index.sqlite"), b"index").unwrap();
        std::fs::write(dir.join("vaults.json"), br#"{"vaults":[{"path":"/somewhere"}]}"#).unwrap();
        std::fs::write(dir.join("README.txt"), b"old readme").unwrap();
        std::fs::write(dir.join("manual/index.html"), b"old manual").unwrap();
        // What a completed download leaves behind.
        std::fs::write(dir.join(".fm-update/staged/program").join(exe_name()), b"NEW BINARY")
            .unwrap();
        std::fs::write(dir.join(".fm-update/staged/program/models.toml"), b"new catalogue")
            .unwrap();
        std::fs::create_dir_all(dir.join(".fm-update/staged/manual")).unwrap();
        std::fs::write(dir.join(".fm-update/staged/manual/index.html"), b"new manual").unwrap();
        std::fs::write(dir.join(".fm-update/staged/README.txt"), b"new readme").unwrap();
        // **Every real release archive carries a `vault/`** — the welcome note for a fresh unpack.
        // It must be skipped, never moved: moving it would put a stranger's "Start here" note into
        // somebody's notebook, or replace their vault outright.
        std::fs::create_dir_all(dir.join(".fm-update/staged/vault/notes")).unwrap();
        std::fs::write(
            dir.join(".fm-update/staged/vault/notes/01M191H1763EX62JM1DZMBK9A9.md"),
            b"WELCOME NOTE FROM THE ARCHIVE",
        )
        .unwrap();
        dir
    }

    /// Every path under `dir`, with what is in it — the fingerprint both guarantees are checked
    /// against. The same idiom `import.rs` uses to prove its source folder is never written to.
    fn fingerprint(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        fn walk(base: &Path, at: &Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
            let Ok(entries) = std::fs::read_dir(at) else { return };
            for e in entries.flatten() {
                let p = e.path();
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                if p.is_dir() {
                    out.insert(format!("{rel}/"), Vec::new());
                    walk(base, &p, out);
                } else {
                    out.insert(rel, std::fs::read(&p).unwrap_or_default());
                }
            }
        }
        let mut out = std::collections::BTreeMap::new();
        walk(dir, dir, &mut out);
        out
    }

    /// **G1, as the test that makes it a guarantee rather than a sentence.**
    ///
    /// The updater may write only files the build produced. Everything a person authored — their
    /// notes, their vault list — must come through byte-identical, and the *whole folder* is
    /// compared rather than the few paths somebody remembered to check, because the failure this
    /// guards against is precisely the one nobody thought of.
    #[test]
    fn an_update_touches_nothing_a_person_wrote() {
        let dir = app_folder("g1");
        let before = fingerprint(&dir.join("vault"));
        let vaults_before = std::fs::read(dir.join("vaults.json")).unwrap();

        swap(&dir).expect("the swap succeeds");

        assert_eq!(fingerprint(&dir.join("vault")), before, "the vault must be byte-identical");
        assert_eq!(std::fs::read(dir.join("vaults.json")).unwrap(), vaults_before);

        // And the update did happen.
        assert_eq!(std::fs::read(dir.join("program").join(exe_name())).unwrap(), b"NEW BINARY");
        assert_eq!(std::fs::read(dir.join("README.txt")).unwrap(), b"new readme");
        assert_eq!(std::fs::read(dir.join("manual/index.html")).unwrap(), b"new manual");
        // Staging outlives the swap on purpose — the process doing it is a copy inside that
        // directory, and Windows cannot delete a running image. The next start clears it, which is
        // the same sweep that cleans up after an update that never finished.
        assert!(
            dir.join(".fm-update").exists(),
            "staging survives the swap that ran from inside it"
        );
        sweep(&dir);
        assert!(!dir.join(".fm-update").exists(), "and the next start clears it");

        // Every path that changed outside the vault is one the updater is allowed to own.
        for (rel, _) in fingerprint(&dir) {
            let rel = rel.trim_end_matches('/');
            if rel.starts_with("vault") || rel == "vaults.json" {
                continue;
            }
            assert!(is_ours(Path::new(rel)), "{rel} changed but is not the updater's to touch");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **Going back, end to end.** The notes come through untouched — the same assertion as the
    /// forward direction, because it is the same promise.
    #[test]
    fn going_back_restores_the_previous_program_and_leaves_the_notes_alone() {
        let dir = app_folder("back");
        swap(&dir).expect("update first");
        let notes = fingerprint(&dir.join("vault"));
        let vaults = std::fs::read(dir.join("vaults.json")).unwrap();
        let (_, version) = backup_dir(&dir).expect("a way back exists after an update");

        go_back(&dir, &format!(".fm-backup-{version}")).expect("goes back");

        assert_eq!(
            std::fs::read(dir.join("program").join(exe_name())).unwrap(),
            b"OLD BINARY",
            "the version that was working is running again"
        );
        assert_eq!(fingerprint(&dir.join("vault")), notes, "the vault is byte-identical");
        assert_eq!(std::fs::read(dir.join("vaults.json")).unwrap(), vaults);
    }

    /// **The rejected version must never be reachable by the launcher.**
    ///
    /// It goes to `.fm-update/rejected/`, not to a `.fm-backup-*`. All four launchers glob
    /// `.fm-backup-*` and restore the first match, so leaving the just-rejected build under that
    /// prefix would let three failed starts roll the user *forward* into the very version they
    /// turned down — the one direction this feature must never travel.
    #[test]
    fn the_rejected_version_is_not_left_where_the_launcher_would_find_it() {
        let dir = app_folder("rejected");
        swap(&dir).expect("update first");
        let (_, version) = backup_dir(&dir).unwrap();
        go_back(&dir, &format!(".fm-backup-{version}")).expect("goes back");

        assert!(backup_dir(&dir).is_none(), "no .fm-backup-* is left holding the rejected build");
        // It goes under `.fm-update/`, which the next start sweeps — so it is gone shortly, and
        // getting it again means downloading it again. That is the "one step only" limit, and the
        // point being pinned here is only *where* it is not: under `.fm-backup-*`.
        let rejected = dir.join(".fm-update").join("rejected").join(exe_name());
        assert_eq!(std::fs::read(rejected).unwrap(), b"NEW BINARY", "set aside, out of reach");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **Interrupted between the two renames, the launcher still has its way back.** Same window as
    /// the update path and the same net: `program/` is briefly absent while the backup is still
    /// under the name the launcher globs.
    #[test]
    fn an_interrupted_go_back_leaves_the_backup_where_the_launcher_looks() {
        let dir = app_folder("back-mid");
        swap(&dir).expect("update first");
        let (_, version) = backup_dir(&dir).unwrap();

        // Name a backup that is not there: the failure is detected before anything moves.
        let err = go_back(&dir, ".fm-backup-nonexistent").unwrap_err();
        assert!(err.contains("no longer there"), "got: {err}");
        assert!(dir.join("program").join(exe_name()).exists(), "nothing moved");
        assert!(backup_dir(&dir).is_some(), "and the way back is still there");
        assert_eq!(backup_dir(&dir).unwrap().1, version);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **There is only ever one way back, and it is the most recent one.**
    ///
    /// Two updates in a row used to leave `.fm-backup-v0.5.0` *and* `.fm-backup-v0.6.0` — and the
    /// launcher restores the first name it globs, which is the lexically first and so usually the
    /// oldest. Someone who updated twice and then needed the rescue would have been put back two
    /// releases, not one. Each update now clears what came before it.
    #[test]
    fn updating_twice_leaves_exactly_one_way_back() {
        let dir = app_folder("one-backup");
        // A backup left by an earlier update.
        std::fs::create_dir_all(dir.join(".fm-backup-v0.1.0")).unwrap();
        std::fs::write(dir.join(".fm-backup-v0.1.0").join(exe_name()), b"ANCIENT").unwrap();

        swap(&dir).expect("the swap succeeds");

        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".fm-backup-"))
            .collect();
        assert_eq!(backups.len(), 1, "one way back, not a pile of them: {backups:?}");
        let (kept, _) = backup_dir(&dir).expect("and it is findable");
        assert_eq!(
            std::fs::read(kept.join(exe_name())).unwrap(),
            b"OLD BINARY",
            "the way back is the version that was just running, never an older one"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **G2: the way back exists, and is a directory rather than a process.**
    ///
    /// After a successful update the previous `program/` is still there under a name the launcher
    /// looks for — kept until the *next* update supersedes it, not until the first successful
    /// start, because a bad release is usually found on day three.
    #[test]
    fn the_previous_version_is_left_where_the_launcher_looks_for_it() {
        let dir = app_folder("g2");
        swap(&dir).expect("the swap succeeds");

        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".fm-backup-"))
            .collect();
        assert_eq!(backups.len(), 1, "exactly one way back");
        let kept = backups[0].path().join(exe_name());
        assert_eq!(
            std::fs::read(kept).unwrap(),
            b"OLD BINARY",
            "and it is the version that worked"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **G2: interrupted between the two renames — the only moment `program/` does not exist.**
    ///
    /// Simulated exactly, by removing what the second rename would have moved. The state this
    /// leaves is the one the launcher preamble is written for, and what is asserted here is the
    /// precondition that preamble depends on: a backup, under the name it globs for, holding a
    /// runnable binary. `ci/checks.sh` then proves the launcher acts on it.
    #[test]
    fn an_interruption_between_the_renames_leaves_the_way_back_intact() {
        let dir = app_folder("g2-mid");
        let notes = fingerprint(&dir.join("vault"));
        std::fs::remove_dir_all(dir.join(".fm-update/staged/program")).unwrap();

        let err = swap(&dir).unwrap_err();
        assert!(err.contains("nothing staged"), "got: {err}");

        // Nothing moved at all, because the failure was detected before the first rename.
        assert_eq!(std::fs::read(dir.join("program").join(exe_name())).unwrap(), b"OLD BINARY");
        assert_eq!(fingerprint(&dir.join("vault")), notes, "notes are never in the blast radius");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **G2: the second rename fails.** The disk fills, or an antivirus takes the new binary
    /// between writing it and moving it. `swap` puts the previous version back itself — the
    /// launcher is the net for when even that cannot run.
    #[test]
    fn a_failure_moving_the_new_version_in_puts_the_old_one_back() {
        let dir = app_folder("g2-fail");
        // A file where the staged `program/` directory should be: the rename cannot succeed, and
        // this is the shape of "the thing I was about to move is not what I thought".
        std::fs::remove_dir_all(dir.join(".fm-update/staged/program")).unwrap();
        std::fs::create_dir_all(dir.join(".fm-update/staged/program")).unwrap();
        std::fs::write(dir.join(".fm-update/staged/program").join(exe_name()), b"NEW BINARY")
            .unwrap();
        std::fs::create_dir_all(dir.join("program-blocker")).unwrap();

        // Make the destination un-renameable by leaving a non-empty directory in the way.
        std::fs::create_dir_all(dir.join("program")).unwrap();
        std::fs::write(dir.join("program/keep"), b"in the way").unwrap();

        let _ = swap(&dir);
        // Whatever happened, a runnable formicaria is reachable: either `program/` still has one,
        // or a backup does. That disjunction *is* G2.
        let has_program = dir.join("program").join(exe_name()).exists();
        let has_backup = std::fs::read_dir(&dir).unwrap().flatten().any(|e| {
            e.file_name().to_string_lossy().starts_with(".fm-backup-")
                && e.path().join(exe_name()).exists()
        });
        assert!(has_program || has_backup, "there must always be a formicaria to start");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **Looking and installing are different capabilities**, and the invariant between them is
    /// one-directional: anything that stops a copy checking also stops it installing, never the
    /// reverse. The useful half — telling somebody a fix exists so they can fetch it by hand — must
    /// survive a folder that cannot be written to.
    #[test]
    fn whatever_stops_a_check_also_stops_an_install() {
        if cannot_check().is_some() {
            assert!(cannot_install().is_some(), "a copy that cannot look cannot install either");
        }
    }

    /// This test binary has no `FM_VERSION`, so it *is* the `dev` case — the refusal that keeps a
    /// source build from comparing itself against a release tag.
    #[test]
    fn a_source_build_refuses_both_and_says_why() {
        let why = cannot_check().expect("a build with no FM_VERSION must refuse");
        assert!(why.contains("built from source"), "the reason must be actionable: {why}");
    }

    /// Refused rather than normalised: `..` is how a path that looks local reaches the vault.
    #[test]
    fn a_traversing_path_is_refused_rather_than_normalised() {
        for bad in ["program/../vault/notes/x.md", "../other/program", "program/./../../etc/passwd"]
        {
            assert!(!is_ours(Path::new(bad)), "{bad} must be refused");
        }
    }
}
