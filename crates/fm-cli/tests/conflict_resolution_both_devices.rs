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
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
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

/// **Editing the note is not a resolution, and the commit path must say so by refusing.**
///
/// Verified against real git: writing clean text over a `UU` path leaves all three index stages in
/// place, so the path is still unmerged. Both backends must therefore report "nothing committed"
/// rather than committing — the subprocess one already does, and the phone must not diverge, because
/// libgit2 will happily build a tree from a *resolved-looking* index and lose the second parent.
#[test]
fn editing_a_marker_conflict_does_not_commit_on_either_device() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let _lock = serial();
    let dir = tempfile::tempdir().unwrap();
    let vault = mid_merge_markers(dir.path());
    let rel = format!("notes/{ID}.md");
    let abs = vec![vault.join(&rel)];
    // The user edits the markers out in the app's editor and it saves.
    std::fs::write(vault.join(&rel), note("our paragraph\n\ntheir paragraph")).unwrap();
    let before = head(&vault);

    for native in [false, true] {
        vcs::force_native(native);
        let made = vcs::commit_all(&vault, "auto", &abs);
        vcs::force_native(false);
        let made = made.unwrap_or_else(|e| panic!("native={native}: commit_all errored: {e}"));
        assert!(!made, "native={native}: must not commit while the path is still unmerged");
        assert_eq!(head(&vault), before, "native={native}: and must create no commit");
        assert!(vault.join(".git/MERGE_HEAD").exists(), "native={native}: still mid-merge");
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
