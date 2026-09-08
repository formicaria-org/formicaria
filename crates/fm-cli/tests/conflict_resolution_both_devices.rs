//! **The same conflict, resolved on a laptop and on a phone, must end in the same bytes.**
//!
//! `fm-app/tests/conflict_without_markers.rs` pins the behaviour on the subprocess backend. This
//! file pins the *other* device, the one that has no `git` binary at all — because the two
//! resolutions are written by completely different code (`git add`/`git rm` versus editing the
//! libgit2 index and writing the blob by hand), and a shared vault is only usable if they agree.
//!
//! It lives in `fm-cli` for the same reason the mixed-device tests do: `native-git` is a feature
//! here, and `force_native` is what stands in for "this device has no git", which a dev machine can
//! never actually be.
#![cfg(feature = "native-git")]

use fm_core::{git, vcs};
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn g(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap()
}

/// `force_native` is process-global (it stands in for a property of a *device*), so no two
/// scenarios may be mid-flight at once.
///
/// It also puts the built `fm` beside the test binary, which is not housekeeping: `git::ensure_repo`
/// runs inside every `commit_all` and `pull` and resolves the driver as `current_exe().parent()/fm`
/// — the same way the product finds `fm` beside `fm-serve`. Without it `ensure_repo` *clears* the
/// driver (there is no `fm` in `deps/`) and the "desktop" half of every test here silently measures
/// **bare git** rather than anything of ours, which for a file whose whole subject is "both devices
/// agree" is the difference between a grading harness and a decoration. It lives behind the lock so
/// it happens once, before any test body, rather than racing between threads. The same helper, for
/// the same reason, is in `collaboration_pipeline.rs` and `mixed_device_collaboration.rs`.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner());
    ensure_fm_beside_test_binary();
    guard
}

/// Put the built `fm` **beside the test binary**, because `git::ensure_repo` — which runs inside
/// every `commit_all` and `pull` — resolves the merge driver as `current_exe().parent()/fm`, the
/// same way the product finds `fm` beside `fm-serve`. Without it `ensure_repo` *clears* the driver
/// (there is no `fm` in `deps/`) and the desktop half of every test here silently measures **bare
/// git** rather than anything of ours.
///
/// **Copied when missing *or stale*, which is the half that was wrong.** The original guard was
/// `if !dst.exists()`, so whatever a previous run left there answered as the merge driver for ever:
/// on 2026-09-07 that was a six-week-old `fm`, and three suites were grading a build nobody had
/// made since. Renamed into place rather than written in place, so a second test binary doing this
/// concurrently never sees a half-copied driver.
/// Put the built `fm` **beside the test binary**, because `git::ensure_repo` — which runs inside
/// every `commit_all` and `pull` — resolves the merge driver as `current_exe().parent()/fm`, the
/// same way the product finds `fm` beside `fm-serve`. Without it `ensure_repo` *clears* the driver
/// (there is no `fm` in `deps/`) and the desktop half of every test here silently measures **bare
/// git** rather than anything of ours.
///
/// **Two ways to get this wrong, both found on 2026-09-07, both by their symptoms rather than by
/// reading the code.**
///
/// 1. The original guard was `if !dst.exists()`, so whatever a previous run left there answered for
///    ever — a **six-week-old `fm`**, and three suites were grading a build nobody had made since.
/// 2. Replacing that with an mtime comparison is *also* wrong, and worse because it looks right.
///    `pixi run ci` runs `test` (no `native-git`) and `test-native-git` as separate tasks, so cargo
///    alternates two different `fm` builds through the same path — and restoring a cached artifact
///    moves its mtime **backwards**. "Older than the source" is then simply not what "stale" means
///    here. Measured: a 102 MB native build at 18:59 and a 57 MB plain one at 18:57, in that order.
///
/// So the copy is keyed on a **stamp** of the source's (length, mtime) — an equality, not an
/// ordering — and the new binary is verified to execute *before* it is renamed into place, because
/// the two tasks can also be mid-swap on the source while this reads it. A driver git cannot run is
/// silent: git takes the failed exec as "conflict" and hands back `%A` untouched, so the merge
/// yields one side with no markers and no error anywhere. That is what the flake looked like.
fn ensure_fm_beside_test_binary() {
    let src = Path::new(env!("CARGO_BIN_EXE_fm"));
    let dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let (dst, stamp_path) = (dir.join("fm"), dir.join("fm.stamp"));

    let Ok(meta) = std::fs::metadata(src) else { return };
    let stamp = format!("{:?}:{}", meta.modified().ok(), meta.len());
    if std::fs::read_to_string(&stamp_path).is_ok_and(|s| s == stamp) && dst.exists() {
        return;
    }

    let tmp = dir.join(format!("fm.{}.tmp", std::process::id()));
    let copied = std::fs::copy(src, &tmp).is_ok();
    // Does it actually run? Any exit status will do — `fm` with no arguments prints its usage and
    // fails, which still proves the OS could execute the file. What this rejects is a truncated or
    // half-written copy, which is the only failure mode that matters and the only silent one.
    if copied && std::process::Command::new(&tmp).output().is_ok() {
        let _ = std::fs::rename(&tmp, &dst);
        let _ = std::fs::write(&stamp_path, &stamp);
    }
    let _ = std::fs::remove_file(&tmp);
}

const ID: &str = "01KY1TK571KRCB5PKAH3FCGCYE";

fn note(body: &str) -> String {
    format!(
        "---\nschema: 1\nid: {ID}\ntype: note\ntitle: Meeting\ncreated: 2026-07-21T07:52:36Z\nupdated: 2026-07-21T08:07:08Z\n---\n{body}\n"
    )
}

/// A vault mid-merge with one delete/modify conflict — deleted on this side, edited on the other.
/// Built with the git binary, because the *setup* is not what is under test.
fn mid_merge(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault");
    let rel = format!("notes/{ID}.md");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    g(&vault, &["init", "-q", "-b", "main"]);
    g(&vault, &["config", "user.name", "Tester"]);
    g(&vault, &["config", "user.email", "t@example.com"]);
    std::fs::write(vault.join(&rel), note("the original")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "add"]);
    g(&vault, &["checkout", "-q", "-b", "theirs"]);
    std::fs::write(vault.join(&rel), note("edited on the other device")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "edit"]);
    g(&vault, &["checkout", "-q", "main"]);
    g(&vault, &["rm", "-q", &rel]);
    g(&vault, &["commit", "-qm", "delete"]);
    g(&vault, &["merge", "theirs"]);
    vault
}

#[test]
fn the_phone_sees_the_same_conflict_kind_as_the_laptop() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let dir = tempfile::tempdir().unwrap();
    let vault = mid_merge(dir.path());

    vcs::force_native(false);
    let laptop = vcs::conflicted(&vault).unwrap();
    vcs::force_native(true);
    let phone = vcs::conflicted(&vault).unwrap();
    vcs::force_native(false);

    // Same path, same kind, same "there is nothing here to edit" verdict. If these ever disagree,
    // one of the two devices is offering the user a resolution that does not apply.
    assert_eq!(laptop, phone, "the two backends must classify a conflict identically");
    assert_eq!(phone.len(), 1);
    assert_eq!(phone[0].kind, git::ConflictKind::DeletedByUs);
    assert!(!phone[0].kind.has_markers());
}

#[test]
fn the_phone_can_keep_theirs_and_finish_the_merge_in_process() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let dir = tempfile::tempdir().unwrap();
    let vault = mid_merge(dir.path());
    let rel = format!("notes/{ID}.md");

    vcs::force_native(true);
    let out = vcs::resolve_conflict(&vault, &rel, git::Keep::Theirs);
    vcs::force_native(false);
    out.unwrap();

    assert!(vcs::conflicted(&vault).unwrap().is_empty(), "resolved");
    assert!(!vault.join(".git/MERGE_HEAD").exists(), "and the merge is committed, in-process");
    let text = std::fs::read_to_string(vault.join(&rel)).expect("their note is on disk");
    assert!(text.contains("edited on the other device"), "their bytes, exactly");
    // The working tree and the index must agree: a staged blob whose bytes differ from the file is
    // the shape that makes the *next* status lie about what is committed.
    let status = g(&vault, &["status", "--porcelain"]);
    assert!(
        String::from_utf8_lossy(&status.stdout).trim().is_empty(),
        "clean tree: {}",
        String::from_utf8_lossy(&status.stdout)
    );
    // History records a merge, with both parents — not a single-parent commit that silently drops
    // the other side's history.
    let parents = g(&vault, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(
        String::from_utf8_lossy(&parents.stdout).split_whitespace().count(),
        3,
        "one commit + two parents"
    );
}

#[test]
fn the_phone_can_keep_the_deletion() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let dir = tempfile::tempdir().unwrap();
    let vault = mid_merge(dir.path());
    let rel = format!("notes/{ID}.md");

    vcs::force_native(true);
    let out = vcs::resolve_conflict(&vault, &rel, git::Keep::Mine);
    vcs::force_native(false);
    out.unwrap();

    assert!(!vault.join(&rel).exists(), "the deletion stands on disk");
    let ls = g(&vault, &["ls-files", "--", &rel]);
    assert!(ls.stdout.is_empty(), "and in the index");
    assert!(!vault.join(".git/MERGE_HEAD").exists(), "merge finished");
}

#[test]
fn both_devices_list_unrecorded_notes_identically() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    g(&vault, &["init", "-q", "-b", "main"]);
    g(&vault, &["config", "user.name", "T"]);
    g(&vault, &["config", "user.email", "t@e.com"]);
    std::fs::write(vault.join("notes/01AAAAAAAAAAAAAAAAAAAAAAAA.md"), note("one")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "one"]);
    std::fs::write(vault.join("notes/01BBBBBBBBBBBBBBBBBBBBBBBB.md"), note("forgotten")).unwrap();
    std::fs::write(vault.join("notes/01AAAAAAAAAAAAAAAAAAAAAAAA.md"), note("one, edited")).unwrap();

    vcs::force_native(false);
    let mut laptop = vcs::unrecorded(&vault, "notes").unwrap();
    vcs::force_native(true);
    let mut phone = vcs::unrecorded(&vault, "notes").unwrap();
    vcs::force_native(false);
    laptop.sort_by(|a, b| a.path.cmp(&b.path));
    phone.sort_by(|a, b| a.path.cmp(&b.path));

    assert_eq!(laptop, phone, "an untracked note and a modified one, seen the same way by both");
    assert_eq!(phone.len(), 2, "{phone:?}");
}

/// **The two backends must agree on *how* a note is out of history, not just that it is.**
///
/// The count alone was the whole problem: 146 unrecorded notes on the owner's phone could have been
/// 146 notes that exist nowhere else (urgent) or 146 notes something was needlessly rewriting (a
/// different bug), and the device knew but could not say. Now it says — so the two devices have to say
/// the same thing, or the phone's report means something different from the laptop's.
#[test]
fn both_devices_agree_on_how_a_note_is_out_of_history() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    g(&vault, &["init", "-q", "-b", "main"]);
    g(&vault, &["config", "user.name", "T"]);
    g(&vault, &["config", "user.email", "t@e.com"]);
    let tracked = "notes/01AAAAAAAAAAAAAAAAAAAAAAAA.md";
    let doomed = "notes/01DDDDDDDDDDDDDDDDDDDDDDDD.md";
    std::fs::write(vault.join(tracked), note("committed")).unwrap();
    std::fs::write(vault.join(doomed), note("about to go")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "two notes"]);

    // One of each kind, which is what a real vault looks like after a while.
    std::fs::write(vault.join(tracked), note("committed, then edited")).unwrap(); // Modified
    std::fs::remove_file(vault.join(doomed)).unwrap(); // Deleted
    std::fs::write(vault.join("notes/01NNNNNNNNNNNNNNNNNNNNNNNN.md"), note("brand new")).unwrap(); // New

    let read = |native: bool| {
        vcs::force_native(native);
        let mut v = vcs::unrecorded(&vault, "notes").unwrap();
        vcs::force_native(false);
        v.sort_by(|a, b| a.path.cmp(&b.path));
        v
    };
    let laptop = read(false);
    let phone = read(true);

    assert_eq!(laptop, phone, "the kinds must match, not only the paths");
    let kind_of = |p: &str| laptop.iter().find(|u| u.path == p).map(|u| u.kind);
    assert_eq!(kind_of(tracked), Some(git::UnrecordedKind::Modified), "{laptop:?}");
    assert_eq!(kind_of(doomed), Some(git::UnrecordedKind::Deleted), "{laptop:?}");
    assert_eq!(
        kind_of("notes/01NNNNNNNNNNNNNNNNNNNNNNNN.md"),
        Some(git::UnrecordedKind::New),
        "{laptop:?}"
    );
}

/// A vault mid-merge with a **marker** conflict (`UU`): both sides edited the same note.
fn mid_merge_markers(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault");
    let rel = format!("notes/{ID}.md");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    g(&vault, &["init", "-q", "-b", "main"]);
    g(&vault, &["config", "user.name", "Tester"]);
    g(&vault, &["config", "user.email", "t@example.com"]);
    std::fs::write(vault.join(&rel), note("base")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "base"]);
    g(&vault, &["checkout", "-q", "-b", "theirs"]);
    std::fs::write(vault.join(&rel), note("their paragraph")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "theirs"]);
    g(&vault, &["checkout", "-q", "main"]);
    std::fs::write(vault.join(&rel), note("our paragraph")).unwrap();
    g(&vault, &["add", "-A"]);
    g(&vault, &["commit", "-qm", "ours"]);
    g(&vault, &["merge", "theirs"]);
    vault
}

fn head(vault: &Path) -> String {
    String::from_utf8_lossy(&g(vault, &["rev-parse", "HEAD"]).stdout).trim().to_string()
}

/// **Editing the note is not a resolution — that note stays out of history, and nothing else does.**
///
/// Two contracts in one place, because they are the same decision seen from both sides.
///
/// 1. **The conflicted note is not committed.** Verified against real git: writing clean text over a
///    `UU` path leaves all three index stages in place, so the path is still unmerged. Staging it is
///    how git is told "the human resolved it", and the commit would then enshrine whatever is on
///    disk as the note's content and push it. Both backends must refuse it — libgit2 will otherwise
///    happily build a tree from a resolved-looking index and lose the merge's second parent.
///
/// 2. **Every other note is.** This half was missing until 2026-09-07 and it is what the incident
///    was: `commit_all` returned early on *any* unmerged path, so two stuck notes stopped 202 from
///    reaching history for 39 days — 196 of them brand new and no part of the merge. The owner's
///    verdict was that a user who has never heard of a merge conflict should not lose their whole
///    vault's history to one: *"all the rest should be committable and synchable."*
///
/// This test used to assert `!made` — "no commit at all" — which was the behaviour and is now the
/// bug. Read `decisions.md` (2026-09-07) before changing it back.
#[test]
fn a_marker_conflict_blocks_its_own_note_and_nothing_else_on_either_device() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let other_rel = "notes/01M1WXF2FW4SSXJVKX3J81BMSW.md";

    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let vault = mid_merge_markers(dir.path());
        let rel = format!("notes/{ID}.md");

        // The user edits the markers out in the app's editor and it saves — which settles nothing,
        // and they have no way to know that. Meanwhile they keep working on a different note.
        std::fs::write(vault.join(&rel), note("our paragraph\n\ntheir paragraph")).unwrap();
        std::fs::write(vault.join(other_rel), note("written while the conflict stood")).unwrap();
        let before = head(&vault);

        vcs::force_native(native);
        let made = vcs::commit_all(&vault, "auto", &[vault.join(&rel), vault.join(other_rel)]);
        vcs::force_native(false);
        let made = made.unwrap_or_else(|e| panic!("native={native}: commit_all errored: {e}"));

        assert!(made, "native={native}: the unrelated note must reach history, conflict or not");
        assert_ne!(head(&vault), before, "native={native}: and that means a commit");
        let landed = g(&vault, &["show", "--name-only", "--format=", "HEAD"]);
        let landed = String::from_utf8_lossy(&landed.stdout);
        assert!(landed.contains(other_rel), "native={native}: the other note is in it:\n{landed}");
        assert!(
            !landed.contains(rel.as_str()),
            "native={native}: the conflicted note must NOT be — staging it publishes the markers \
             as content:\n{landed}"
        );

        // The conflict is exactly where it was: still unmerged, still mid-merge, still a person's
        // decision. Committing around it must not look like resolving it.
        assert!(vault.join(".git/MERGE_HEAD").exists(), "native={native}: still mid-merge");
        let porcelain = g(&vault, &["status", "--porcelain"]);
        assert!(
            String::from_utf8_lossy(&porcelain.stdout)
                .lines()
                .any(|l| l.contains(rel.as_str()) && l.starts_with("UU")),
            "native={native}: the path is still unmerged:\n{}",
            String::from_utf8_lossy(&porcelain.stdout)
        );

        // **The hazard the ruling names, and the only reason this is safe.**
        // `finish_merge_if_resolved` builds the merge commit from the *real* index. A note committed
        // around the conflict but left at its old content there would be silently reverted the
        // moment the merge completed — deleted at exactly the moment the user fixed the thing that
        // was blocking them, which is the worst possible time to lose a day's writing.
        g(&vault, &["add", "--", &rel]);
        vcs::force_native(native);
        vcs::commit_all(&vault, "auto: resolved", &[vault.join(&rel)])
            .unwrap_or_else(|e| panic!("native={native}: finishing: {e}"));
        vcs::force_native(false);
        assert!(
            !vault.join(".git/MERGE_HEAD").exists(),
            "native={native}: the merge finished, so the vault is not frozen"
        );
        let survived = g(&vault, &["show", &format!("HEAD:{other_rel}")]);
        assert!(
            String::from_utf8_lossy(&survived.stdout).contains("written while the conflict stood"),
            "native={native}: finishing the merge deleted the note written during it — the \
             lockstep hazard in `decisions.md`, 2026-09-07"
        );
    }
}

/// Once the index is settled, the commit path must **finish the merge**: two parents, and the merge
/// state cleared. A single-parent commit here silently drops the incoming side from history, and a
/// `MERGE_HEAD` left standing freezes the vault for good (`push_squashed` and the proposal merge both
/// refuse over an unfinished merge) — on a phone, with no shell, that is unrecoverable.
#[test]
fn a_settled_merge_is_committed_with_both_parents_on_either_device() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let vault = mid_merge_markers(dir.path());
        let rel = format!("notes/{ID}.md");
        std::fs::write(vault.join(&rel), note("both paragraphs, resolved")).unwrap();
        // Settle the index the way a resolution does — the point under test is what the *commit*
        // then does with the merge that is still in flight.
        g(&vault, &["add", "--", &rel]);

        vcs::force_native(native);
        let made = vcs::commit_all(&vault, "auto", &[vault.join(&rel)]);
        vcs::force_native(false);
        made.unwrap_or_else(|e| panic!("native={native}: {e}"));

        assert!(
            !vault.join(".git/MERGE_HEAD").exists(),
            "native={native}: MERGE_HEAD must be cleared, or the vault is frozen forever"
        );
        let parents = g(&vault, &["rev-list", "--parents", "-1", "HEAD"]);
        assert_eq!(
            String::from_utf8_lossy(&parents.stdout).split_whitespace().count(),
            3,
            "native={native}: a merge commit has two parents; one parent silently drops their history"
        );
        let status = g(&vault, &["status", "--porcelain"]);
        assert!(
            String::from_utf8_lossy(&status.stdout).trim().is_empty(),
            "native={native}: clean tree"
        );
    }
}

/// **"I reconciled it in the editor" must actually settle the conflict**, on both devices.
///
/// This is the resolution the product lacked entirely. Editing the note leaves git's index unmerged,
/// so before this the ordinary way of resolving a conflict settled nothing: the vault stayed frozen,
/// and — because the conflict list was derived from markers in the body — the note silently left the
/// UI as soon as the markers were gone, taking the only sign of trouble with it.
#[test]
fn marking_an_edited_conflict_resolved_settles_it_on_either_device() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let vault = mid_merge_markers(dir.path());
        let rel = format!("notes/{ID}.md");
        // The user keeps both paragraphs — the answer neither side holds alone, which is exactly why
        // "keep theirs / keep mine" is the wrong tool for a marker conflict.
        std::fs::write(vault.join(&rel), note("our paragraph\n\ntheir paragraph")).unwrap();

        vcs::force_native(native);
        let out = vcs::resolve_conflict(&vault, &rel, git::Keep::Edited);
        vcs::force_native(false);
        out.unwrap_or_else(|e| panic!("native={native}: {e}"));

        assert!(vcs::conflicted(&vault).unwrap().is_empty(), "native={native}: settled");
        assert!(!vault.join(".git/MERGE_HEAD").exists(), "native={native}: merge finished");
        let parents = g(&vault, &["rev-list", "--parents", "-1", "HEAD"]);
        assert_eq!(
            String::from_utf8_lossy(&parents.stdout).split_whitespace().count(),
            3,
            "native={native}: both parents recorded"
        );
        let committed = g(&vault, &["show", "-s", "--format=%H"]);
        assert!(!committed.stdout.is_empty(), "native={native}: a commit exists");
        // The reconciled text is what landed — both paragraphs, no markers.
        let show = g(&vault, &["show", &format!("HEAD:{rel}")]);
        let text = String::from_utf8_lossy(&show.stdout);
        assert!(
            text.contains("our paragraph") && text.contains("their paragraph"),
            "native={native}"
        );
        assert!(!text.contains("<<<<<<<"), "native={native}: no markers in history");
    }
}

/// **The guard.** Marking a conflict resolved while the markers are still there would stage them, and
/// git reads a staged file as "the human resolved it" — so `<<<<<<<` would become the note's content
/// and travel to every collaborator. Both backends must refuse, and say why.
#[test]
fn marking_resolved_is_refused_while_markers_remain_on_either_device() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let vault = mid_merge_markers(dir.path());
        let rel = format!("notes/{ID}.md");
        // Untouched: the file is git's marked-up merge result.
        let before = std::fs::read_to_string(vault.join(&rel)).unwrap();
        assert!(before.contains("<<<<<<<"), "precondition: git left markers");

        vcs::force_native(native);
        let out = vcs::resolve_conflict(&vault, &rel, git::Keep::Edited);
        vcs::force_native(false);
        let err = out.expect_err("must refuse");

        assert!(
            format!("{err}").contains("still has conflict markers"),
            "native={native}: names the reason: {err}"
        );
        // And it must have changed nothing: still conflicted, still mid-merge.
        assert!(!vcs::conflicted(&vault).unwrap().is_empty(), "native={native}: untouched");
        assert!(vault.join(".git/MERGE_HEAD").exists(), "native={native}: still mid-merge");
    }
}

/// A vault with a real remote and a second clone, so `pull` genuinely runs. `mid_merge` above
/// builds its conflict with `git merge` directly, which is right for testing *resolution* — but
/// the auto-settle under test lives in `pull`, so it needs the real thing.
///
/// Returns `(remote, ours, theirs)`. `ours` is the device that will edit; `theirs` deletes.
fn two_clones(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let remote = dir.join("remote.git");
    g(dir, &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]);

    let seed = dir.join("seed");
    let rel = format!("notes/{ID}.md");
    std::fs::create_dir_all(seed.join("notes")).unwrap();
    g(&seed, &["init", "-q", "-b", "main"]);
    g(&seed, &["config", "user.name", "Tester"]);
    g(&seed, &["config", "user.email", "t@example.com"]);
    std::fs::write(seed.join(&rel), note("the original")).unwrap();
    g(&seed, &["add", "-A"]);
    g(&seed, &["commit", "-qm", "add"]);
    g(&seed, &["remote", "add", "origin", remote.to_str().unwrap()]);
    g(&seed, &["push", "-q", "-u", "origin", "main"]);

    let mut out = Vec::new();
    for name in ["ours", "theirs"] {
        let path = dir.join(name);
        g(dir, &["clone", "-q", remote.to_str().unwrap(), path.to_str().unwrap()]);
        g(&path, &["config", "user.name", "Tester"]);
        g(&path, &["config", "user.email", "t@example.com"]);
        out.push(path);
    }
    (remote, out.remove(0), out.remove(0))
}

/// **Two stuck notes must not freeze two hundred — the incident, reproduced.**
///
/// On a real phone, one delete/modify conflict left the index unmerged and `commit_all` refuses
/// while anything is unmerged — so **202 unrelated notes could not be committed or pushed for 39
/// days**, 196 of them brand new and no part of the merge. The count was visible; the cause was
/// not; and the vault never recovered on its own.
///
/// What this pins, on **both backends**, because the phone runs the one a dev machine never does:
///
/// 1. A pull that hits a delete/modify **finishes** — no `MERGE_HEAD` left standing.
/// 2. The note is **kept**, and `Pulled::Merged` **names it**. A resolution nobody is told about is
///    the silence this replaced (`decisions.md`, 2026-09-07).
/// 3. **The vault commits afterwards.** This is the assertion the incident is about: notes written
///    after the conflict reach history instead of piling up behind it.
///
/// Proven red by removing the auto-settle branch from either backend's `pull`: the pull returns
/// `Conflicted`, `MERGE_HEAD` stands, and the follow-up `commit_all` answers `false` with the new
/// note stranded — which is exactly the phone's state on the day this was written.
#[test]
fn a_note_the_other_device_deleted_is_kept_and_the_vault_keeps_committing() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();

    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (_remote, ours, theirs) = two_clones(dir.path());
        let rel = format!("notes/{ID}.md");

        // The other device deletes the note and pushes.
        g(&theirs, &["rm", "-q", &rel]);
        g(&theirs, &["commit", "-qm", "delete it there"]);
        g(&theirs, &["push", "-q", "origin", "main"]);

        // This device edits the same note and commits, then pulls their deletion.
        std::fs::write(ours.join(&rel), note("edited here, deleted there")).unwrap();
        g(&ours, &["add", "-A"]);
        g(&ours, &["commit", "-qm", "edit it here"]);

        vcs::force_native(native);
        let outcome = vcs::pull(&ours).unwrap_or_else(|e| panic!("native={native}: pull: {e}"));

        match outcome {
            git::Pulled::Merged { kept, .. } => assert_eq!(
                kept,
                vec![rel.clone()],
                "native={native}: the kept note must be named, not silently resurrected"
            ),
            other => panic!("native={native}: a delete/modify must not stall the pull: {other:?}"),
        }

        assert!(
            !ours.join(".git/MERGE_HEAD").exists(),
            "native={native}: the merge is finished, so nothing is left to freeze the vault"
        );
        assert!(
            ours.join(&rel).exists(),
            "native={native}: the note is kept — deleting it here would be the unrecoverable answer"
        );

        // **The assertion the whole incident is about.** A note written after the conflict must
        // reach history rather than pile up behind it.
        // Absolute, as `adoptable` and the write-record both hand them over: `commit_all` tests
        // `p.exists()` on the path as given, so a relative one resolves against the *process*
        // working directory and stages nothing on the native backend.
        let after = ours.join("notes/01M1WXF2FW4SSXJVKX3J81BMSW.md");
        std::fs::write(&after, note("written after the conflict")).unwrap();
        let made = vcs::commit_all(&ours, "auto: after the conflict", std::slice::from_ref(&after))
            .unwrap_or_else(|e| panic!("native={native}: commit_all: {e}"));
        assert!(made, "native={native}: the vault must commit again once the merge is settled");

        vcs::force_native(false);
    }
}

/// **The resurrection outlives the run that caused it, identically on both backends.**
///
/// The persistent half of `outstanding.md` §2.12: a kept note was reported in one step line and one
/// dismissible banner, both gone by the next screen. `kept_notes` reads it back out of the merge
/// commit that did it, so the answer survives a restart, a fresh clone, and the device that never
/// ran the merge at all.
///
/// **The byte-agreement arm is the point.** `git_differential.rs` has no pull test — it says so at
/// its own delete/modify comment — so until now nothing anywhere compared what the two backends
/// *wrote* on this path. A trailer spelled differently by the two engines is a phone and a desktop
/// disagreeing about what happened, months later, with no test to catch it.
///
/// Proven red four ways: dropping `record_kept_in_merge_message`; writing the trailer *after* the
/// resolve loop (the subprocess side then has no `MERGE_MSG` left and the record never lands);
/// omitting `kept_trailers` from the native merge message; and acknowledging without moving the
/// ref.
#[test]
fn a_kept_note_is_still_reported_after_the_run_that_kept_it() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();

    let mut wrote: Vec<String> = Vec::new();
    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (_remote, ours, theirs) = two_clones(dir.path());
        let rel = format!("notes/{ID}.md");

        g(&theirs, &["rm", "-q", &rel]);
        g(&theirs, &["commit", "-qm", "delete it there"]);
        g(&theirs, &["push", "-q", "origin", "main"]);
        std::fs::write(ours.join(&rel), note("edited here, deleted there")).unwrap();
        g(&ours, &["add", "-A"]);
        g(&ours, &["commit", "-qm", "edit it here"]);

        vcs::force_native(native);
        vcs::pull(&ours).unwrap_or_else(|e| panic!("native={native}: pull: {e}"));

        // 1. It is still reported *after* the run — the whole debt.
        let kept = vcs::kept_notes(&ours).unwrap();
        assert_eq!(
            kept,
            vec![rel.clone()],
            "native={native}: the resurrection must outlive the run"
        );

        // 2. Read back by real git, so the two backends are compared on what they WROTE, not on
        //    what each of them can parse.
        let msg = String::from_utf8(
            std::process::Command::new("git")
                .arg("-C")
                .arg(&ours)
                .args(["log", "-1", "--merges", "--format=%B"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let trailer: Vec<&str> = msg.lines().filter(|l| l.starts_with(git::KEPT_TRAILER)).collect();
        assert_eq!(
            trailer,
            vec![format!("{}{rel}", git::KEPT_TRAILER)],
            "native={native}: the merge commit records exactly the kept path"
        );
        wrote.push(trailer.join("\n"));

        // 3. A second read is the same read: nothing here consumes what it reports.
        assert_eq!(
            vcs::kept_notes(&ours).unwrap(),
            kept,
            "native={native}: reading is not consuming"
        );

        // 4. Acknowledging silences it — and only up to the HEAD that was acknowledged.
        vcs::mark_kept_seen(&ours).unwrap();
        assert!(
            vcs::kept_notes(&ours).unwrap().is_empty(),
            "native={native}: once seen, it stops being reported"
        );

        // 5. A *later* resurrection is above the watermark, so it raises again rather than being
        //    swallowed by an acknowledgement of an older one.
        let second = "notes/01M1WXF2FW4SSXJVKX3J81BMSW.md";
        std::fs::write(ours.join(second), note("a second note")).unwrap();
        g(&ours, &["add", "-A"]);
        g(&ours, &["commit", "-qm", "add a second note"]);
        g(&ours, &["push", "-q", "origin", "main"]);
        g(&theirs, &["pull", "-q", "--no-rebase", "origin", "main"]);
        g(&theirs, &["rm", "-q", second]);
        g(&theirs, &["commit", "-qm", "delete the second one there"]);
        g(&theirs, &["push", "-q", "origin", "main"]);
        std::fs::write(ours.join(second), note("edited again here")).unwrap();
        g(&ours, &["add", "-A"]);
        g(&ours, &["commit", "-qm", "edit the second one here"]);
        vcs::pull(&ours).unwrap_or_else(|e| panic!("native={native}: second pull: {e}"));
        assert_eq!(
            vcs::kept_notes(&ours).unwrap(),
            vec![second.to_string()],
            "native={native}: a resurrection after the watermark must raise again"
        );

        vcs::force_native(false);
    }
    assert_eq!(wrote[0], wrote[1], "the two backends write the same trailer, byte for byte");
}

/// **A field two devices disagree about must not stop the vault either.**
///
/// The sibling of the delete/modify test above, for the other shape that used to freeze things.
/// Until 2026-09-07 a divergent `status` dropped the whole file into a text merge, so the markers
/// landed inside the YAML fence: the note stopped parsing, stopped appearing in every view, and the
/// path stayed unmerged — the same freeze, from a different cause. Now the loser is demoted into
/// `conflict-status` and the merge completes.
///
/// What this pins, on **both backends** (the phone runs the one a dev machine never does):
///
/// 1. The pull **finishes** — no `MERGE_HEAD` left standing, nothing unmerged.
/// 2. Both values are in the file and **the file parses**, which is the whole reversal.
/// 3. **The vault commits afterwards**, which is what the 39-day incident was actually about.
///
/// Proven red by restoring the old escalation — `return None` from `merge_objects` when
/// `Demote::settle` cannot use the three-way rule: the pull comes back `Conflicted`, the note no
/// longer parses, and the follow-up `commit_all` leaves the new note stranded.
#[test]
fn a_divergent_field_settles_itself_and_the_vault_keeps_committing() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();

    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (_remote, ours, theirs) = two_clones(dir.path());
        let rel = format!("notes/{ID}.md");

        // Two people drag one card to two columns. Identical bodies: nothing else is in dispute.
        let body = "the body nobody touched";
        std::fs::write(theirs.join(&rel), note_with("done", "2026-07-22T09:00:00Z", body)).unwrap();
        g(&theirs, &["commit", "-qam", "their status"]);
        g(&theirs, &["push", "-q", "origin", "main"]);

        std::fs::write(ours.join(&rel), note_with("doing", "2026-07-22T08:00:00Z", body)).unwrap();
        g(&ours, &["commit", "-qam", "our status"]);

        // The desktop reaches `merge.rs` **through git's driver**; `ensure_repo` (inside `pull`)
        // installs it, and `serial()` is what makes that possible here. The phone needs neither:
        // `git_native::pull` calls `merge_texts` itself, having no driver to invoke.
        vcs::force_native(native);
        let outcome = vcs::pull(&ours).unwrap_or_else(|e| panic!("native={native}: pull: {e}"));
        assert!(
            matches!(outcome, git::Pulled::Merged { .. }),
            "native={native}: a divergent field must no longer stall the pull: {outcome:?}"
        );
        assert!(
            !ours.join(".git/MERGE_HEAD").exists(),
            "native={native}: the merge is finished, so nothing is left to freeze the vault"
        );

        let disk = std::fs::read_to_string(ours.join(&rel)).unwrap();
        let obj = fm_core::frontmatter::from_file(&disk)
            .unwrap_or_else(|e| panic!("native={native}: the note must still parse: {e}\n{disk}"));
        assert_eq!(
            obj.status.as_deref(),
            Some("done"),
            "native={native}: later `updated`:\n{disk}"
        );
        assert_eq!(
            obj.extra.get("conflict-status"),
            Some(&fm_model::PropertyValue::List(vec![fm_model::PropertyValue::Text(
                "doing".into()
            )])),
            "native={native}: and the loser is beside it, not discarded:\n{disk}"
        );

        // Absolute, for the reason spelled out in the delete/modify test above.
        let after = ours.join("notes/01M1WXF2FW4SSXJVKX3J81BMSW.md");
        std::fs::write(&after, note("written after the disagreement")).unwrap();
        let made = vcs::commit_all(&ours, "auto: after the disagreement", &[after])
            .unwrap_or_else(|e| panic!("native={native}: commit_all: {e}"));
        assert!(made, "native={native}: the vault must keep committing through a disagreement");

        vcs::force_native(false);
    }
}

/// The same note with a chosen `status` and `updated`, for the divergent-scalar case.
fn note_with(status: &str, updated: &str, body: &str) -> String {
    format!(
        "---\nschema: 1\nid: {ID}\ntype: note\ntitle: Meeting\nstatus: {status}\n\
         created: 2026-07-21T07:52:36Z\nupdated: {updated}\n---\n{body}\n"
    )
}

/// **Nothing a person wrote becomes unreachable after a merge.**
///
/// The owner's principle is *"do not lose data"*, and the honest reading of it here is durability:
/// whatever each side typed is still retrievable afterwards — from the working tree, or from one of
/// the merge's parents. That is already true, and it is true by accident unless something pins it.
/// This is the test a future "simplification" has to argue with before it can start discarding a
/// side, which is the entire reason for writing it.
///
/// **The surveyed field says this is the property that gets lost.** Of the seven documented
/// families of conflict resolution (2026-09-07), only three lose nothing — markers, sidecar files,
/// and an in-app conflict item — and every one of them works by *refusing to produce a single
/// merged file*. Anything that yields one clean note is trading data for convenience: `merge=union`
/// keeps the bytes and destroys the meaning with no signal at all, and CRDT text merges are proven
/// to interleave concurrent insertions into, in Kleppmann's words, "an unreadable jumble of
/// letters". So the durability we have is worth a test, not an assumption.
///
/// Both content-bearing shapes are covered, on both backends. Delete/modify is pinned separately by
/// `a_note_the_other_device_deleted_is_kept_and_the_vault_keeps_committing`, where the surviving
/// side is a note and the discarded side is a *deletion* — the one place this project knowingly
/// drops an intention, and it says so in `decisions.md`.
///
/// **The first version of this test was vacuous, and the red proof is what caught it.** It asserted
/// "each side is in the working tree *or* in a parent of HEAD" — but a clean merge writes a commit
/// with *both* parents, so `HEAD^2` always holds the other side and the assertion could never fail.
/// It passed against a mutation that discarded half the user's work.
///
/// So the property is stated the way it actually protects somebody: **you are never silently left
/// with one side.** Either both survive in the file, or the merge said it conflicted. A merge that
/// keeps one side and reports success is the failure — git having the other parent is no comfort to
/// a person who was never told to go looking.
///
/// Proven red by making `merge_texts` return ours with `Merged::Clean`: the file then holds one
/// side and the pull reports success, which is precisely what must not happen.
#[test]
fn neither_side_of_a_conflict_becomes_unreachable() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();

    for native in [false, true] {
        // --- Case 1: both devices rewrite the same line of the body. ---
        {
            let dir = tempfile::tempdir().unwrap();
            let (_r, ours, theirs) = two_clones(dir.path());
            let rel = format!("notes/{ID}.md");

            std::fs::write(theirs.join(&rel), note("the version written on the phone")).unwrap();
            g(&theirs, &["commit", "-qam", "their edit"]);
            g(&theirs, &["push", "-q", "origin", "main"]);

            std::fs::write(ours.join(&rel), note("the version written on the laptop")).unwrap();
            g(&ours, &["commit", "-qam", "our edit"]);

            vcs::force_native(native);
            let outcome = vcs::pull(&ours);
            vcs::force_native(false);

            let disk = std::fs::read_to_string(ours.join(&rel)).unwrap_or_default();
            both_survive_or_it_said_so(
                native,
                &disk,
                &outcome,
                ["written on the phone", "written on the laptop"],
            );
        }

        // --- Case 2: both devices set the same scalar field differently. ---
        // The field holds one value, so "keep both" is not expressible in the *field's* type — but
        // it is expressible in the note's, and since 2026-09-07 the loser is demoted into
        // `conflict-status` beside the winner. Either way the assertion here is the durable one:
        // neither answer disappears. `a_divergent_field_settles_itself_and_the_vault_keeps_
        // committing` below is what pins the newer, stronger behaviour.
        {
            let dir = tempfile::tempdir().unwrap();
            let (_r, ours, theirs) = two_clones(dir.path());
            let rel = format!("notes/{ID}.md");

            std::fs::write(
                theirs.join(&rel),
                note_with("done", "2026-07-22T09:00:00Z", "the body nobody touched"),
            )
            .unwrap();
            g(&theirs, &["commit", "-qam", "their status"]);
            g(&theirs, &["push", "-q", "origin", "main"]);

            std::fs::write(
                ours.join(&rel),
                note_with("doing", "2026-07-22T08:00:00Z", "the body nobody touched"),
            )
            .unwrap();
            g(&ours, &["commit", "-qam", "our status"]);

            vcs::force_native(native);
            let outcome = vcs::pull(&ours);
            vcs::force_native(false);

            let disk = std::fs::read_to_string(ours.join(&rel)).unwrap_or_default();
            both_survive_or_it_said_so(native, &disk, &outcome, ["done", "doing"]);
        }
    }
}

/// **Both sides are in the file, or the pull admitted it could not do that.**
///
/// Deliberately *not* "the bytes are somewhere in git". A merge commit keeps both parents by
/// construction, so any assertion that accepts `HEAD^2` is satisfied by a merge that threw half the
/// user's work out of the working tree — which is how the first version of this test passed a
/// mutation that did exactly that. What matters is whether the person is left holding one side
/// **without being told**.
fn both_survive_or_it_said_so(
    native: bool,
    disk: &str,
    outcome: &Result<git::Pulled, fm_core::StoreError>,
    sides: [&str; 2],
) {
    if sides.iter().all(|s| disk.contains(s)) {
        return; // both are in front of the user; nothing to report
    }
    let missing: Vec<&str> = sides.iter().copied().filter(|s| !disk.contains(s)).collect();
    assert!(
        matches!(outcome, Ok(git::Pulled::Conflicted(_))),
        "native={native}: the merge dropped {missing:?} from the note and still reported \
         {outcome:?} — a side may only go missing when the app says so"
    );
}

/// **Two devices merging the same pair of commits must produce the same bytes.**
///
/// There is an asymmetry worth pinning. On a desktop, `git merge` runs the `fm merge-md` driver for
/// every `.md`, so our frontmatter-aware merge sees all of them. On a phone, `repo.merge()` does
/// libgit2's own text merge first and `git_native::pull` only re-merges the paths libgit2 left
/// *conflicted* — so a note libgit2 merges cleanly never passes through our rules at all.
///
/// In practice that window is narrow, because `updated:` is rewritten on every save and therefore
/// collides on any two concurrent edits, forcing the path to conflict and our merge to run. This
/// pins the narrow case anyway: **a note whose `updated` happens to agree, edited in two different
/// places.** If the two backends' internal text merges ever diverge — different diff algorithms
/// being the obvious way — the devices commit different bytes from identical inputs and then
/// conflict with each other forever afterwards.
///
/// `merge_differential.rs` grades our `text_3way` across 400 generated triples. It does **not**
/// cover this path, because this one never reaches `text_3way`.
///
/// **What this compares, and the correction it needed.** It used to say the `fm merge-md` driver was
/// not installed here, so both sides fell back to their built-in text merges and a mutation inside
/// `merge_texts` "correctly" left it green. That was true and it was the bug: `serial()` now puts a
/// **fresh** `fm` beside the test binary, so the desktop half runs the real driver, and this is a
/// test of driver-versus-libgit2 after all. A `merge_texts` mutation now fails it.
///
/// Proven red by perturbing one backend's output directly (an extra line appended in
/// `git_native::pull` after the merge): the byte comparison fails, naming the divergence.
#[test]
fn a_clean_merge_is_byte_identical_on_both_devices() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let rel = format!("notes/{ID}.md");
    let mut results = Vec::new();

    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (_r, ours, theirs) = two_clones(dir.path());

        // Same `updated` on both sides, so the line the app rewrites on every save does not
        // collide — this is the case that can merge cleanly without our rules being consulted.
        let base = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n";
        std::fs::write(ours.join(&rel), note_with("todo", "2026-07-21T08:07:08Z", base)).unwrap();
        g(&ours, &["commit", "-qam", "a longer body"]);
        g(&ours, &["push", "-q", "origin", "main"]);
        g(&theirs, &["pull", "-q", "--no-rebase", "origin", "main"]);

        // They change the first line; we change the last. Far apart, so a 3-way merge settles it.
        let their_body = base.replace("one\n", "ONE (theirs)\n");
        std::fs::write(theirs.join(&rel), note_with("todo", "2026-07-21T08:07:08Z", &their_body))
            .unwrap();
        g(&theirs, &["commit", "-qam", "their end"]);
        g(&theirs, &["push", "-q", "origin", "main"]);

        let our_body = base.replace("eight\n", "EIGHT (ours)\n");
        std::fs::write(ours.join(&rel), note_with("todo", "2026-07-21T08:07:08Z", &our_body))
            .unwrap();
        g(&ours, &["commit", "-qam", "our end"]);

        vcs::force_native(native);
        let outcome = vcs::pull(&ours);
        vcs::force_native(false);

        assert!(
            matches!(outcome, Ok(git::Pulled::Merged { .. })),
            "native={native}: edits at opposite ends of a note must merge, not conflict: {outcome:?}"
        );
        let merged = std::fs::read_to_string(ours.join(&rel)).unwrap();
        assert!(merged.contains("ONE (theirs)"), "native={native}: their edit survived");
        assert!(merged.contains("EIGHT (ours)"), "native={native}: our edit survived");
        results.push(merged);
    }

    assert_eq!(
        results[0], results[1],
        "the two backends produced different bytes from identical inputs — the devices would then \
         disagree with each other forever, from a merge that both of them called clean"
    );
}

/// **The same guarantee for a note whose frontmatter is not already canonical — which is where it
/// actually broke.**
///
/// A real divergence, found on 2026-09-07 the moment the harness above started installing the
/// driver. The desktop's `git merge` invokes `fm merge-md` for **every** path both sides changed,
/// so `merge_texts` reparses the note and re-emits its frontmatter whole. `git_native::pull` only
/// revisited what libgit2 left *conflicted* — so a note libgit2 merged cleanly never passed
/// through our rules, and the phone committed the file's original frontmatter while the desktop
/// committed the canonical form. **Different bytes, same pair of commits**, and then the two
/// devices conflict with each other for ever over a note nobody edited.
///
/// The note here is deliberately not what `to_file` writes: keys out of order, a two-space list
/// indent, no `schema:` — every one of which an external editor, an import or an older version of
/// this app leaves behind. The pair of edits is far apart with a matching `updated:`, which is what
/// lets libgit2 settle it without asking us.
///
/// Fixed by `settle_the_paths_libgit2_merged_itself`. Red proof: delete that call from
/// `git_native::pull` and this fails alone, naming both spellings.
#[test]
fn the_two_devices_agree_even_when_the_note_was_not_written_by_this_app() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let rel = format!("notes/{ID}.md");
    let hand_written = |body: &str| {
        format!(
            "---\ntitle: Meeting\ntype: note\nid: {ID}\ntags:\n  - a\nstatus: todo\n\
             updated: 2026-07-21T08:07:08Z\ncreated: 2026-07-21T07:52:36Z\n---\n{body}\n"
        )
    };
    let base = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n";
    let mut results = Vec::new();

    for native in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (_r, ours, theirs) = two_clones(dir.path());

        std::fs::write(ours.join(&rel), hand_written(base)).unwrap();
        g(&ours, &["commit", "-qam", "a note some other editor wrote"]);
        g(&ours, &["push", "-q", "origin", "main"]);
        g(&theirs, &["pull", "-q", "--no-rebase", "origin", "main"]);

        std::fs::write(theirs.join(&rel), hand_written(&base.replace("one\n", "ONE (theirs)\n")))
            .unwrap();
        g(&theirs, &["commit", "-qam", "their end"]);
        g(&theirs, &["push", "-q", "origin", "main"]);

        std::fs::write(ours.join(&rel), hand_written(&base.replace("eight\n", "EIGHT (ours)\n")))
            .unwrap();
        g(&ours, &["commit", "-qam", "our end"]);

        vcs::force_native(native);
        let outcome = vcs::pull(&ours);
        vcs::force_native(false);
        assert!(
            matches!(outcome, Ok(git::Pulled::Merged { .. })),
            "native={native}: this must still be a clean merge: {outcome:?}"
        );
        let merged = std::fs::read_to_string(ours.join(&rel)).unwrap();
        assert!(merged.contains("ONE (theirs)") && merged.contains("EIGHT (ours)"), "{merged}");
        results.push(merged);
    }

    assert_eq!(
        results[0], results[1],
        "the phone kept the note's original frontmatter and the desktop rewrote it — the devices \\
         would now conflict for ever over a note neither of them edited"
    );
}
