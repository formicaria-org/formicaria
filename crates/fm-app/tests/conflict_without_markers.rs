//! **The conflict you cannot see, and the notes the app forgot it wrote.**
//!
//! Both of these cost the owner a week of history on 2026-07-31, in one incident:
//!
//! 1. A **delete/modify** conflict — a note deleted on the laptop and edited on the phone — has no
//!    conflict markers and never can (one side has no file, so there is nothing to interleave and the
//!    `.md` merge driver is never called). The product listed conflicts by scanning note bodies for
//!    `<<<<<<<`, so the note appeared in no surface; and the advice every message gave — *"open each
//!    one, both versions are marked in the text"* — was impossible to follow. Meanwhile `commit_all`
//!    refuses while a vault is mid-merge, so that one invisible note froze **every** commit in the
//!    vault.
//! 2. `commit_all` stages only the paths `FileStore::put`/`delete` recorded, and that record is
//!    **per-process memory** — so notes written before the last restart were not lagging, they were
//!    permanently unstageable, and nothing said so. 95 of them had accumulated.
//!
//! These tests drive real git, because the whole class of bug lives in what git does and what our
//! abstraction assumed it did.

use std::path::Path;
use std::process::Command;

use fm_core::{git, vcs};

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn git_run(repo: &Path, args: &[&str]) -> std::process::Output {
    let out = Command::new("git")
        .current_dir(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("git");
    assert!(
        out.status.success() || args[0] == "merge" || args[0] == "pull",
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn note(id: &str, body: &str) -> String {
    format!(
        "---\nschema: 1\nid: {id}\ntype: note\ntitle: Meeting\ncreated: 2026-07-21T07:52:36Z\nupdated: 2026-07-21T08:07:08Z\n---\n{body}\n"
    )
}

const ID: &str = "01KY1TK571KRCB5PKAH3FCGCYE";

/// Build a vault mid-merge with exactly one delete/modify conflict: `main` deleted the note, the
/// branch we merge edited it. This is the owner's situation, reduced.
fn vault_with_delete_modify_conflict(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git_run(&vault, &["init", "-q", "-b", "main"]);
    git_run(&vault, &["config", "user.name", "Tester"]);
    git_run(&vault, &["config", "user.email", "t@example.com"]);
    let rel = format!("notes/{ID}.md");
    std::fs::write(vault.join(&rel), note(ID, "the original template")).unwrap();
    git_run(&vault, &["add", "-A"]);
    git_run(&vault, &["commit", "-qm", "add the template"]);

    // Their side: edit it.
    git_run(&vault, &["checkout", "-q", "-b", "theirs"]);
    std::fs::write(vault.join(&rel), note(ID, "the template, edited on the phone")).unwrap();
    git_run(&vault, &["add", "-A"]);
    git_run(&vault, &["commit", "-qm", "edit the template"]);

    // Our side: delete it.
    git_run(&vault, &["checkout", "-q", "main"]);
    git_run(&vault, &["rm", "-q", &rel]);
    git_run(&vault, &["commit", "-qm", "delete the template"]);

    // The merge that cannot be settled textually.
    git_run(&vault, &["merge", "theirs"]);
    vault
}

#[test]
fn a_delete_modify_conflict_is_reported_with_its_kind_and_carries_no_markers() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_delete_modify_conflict(dir.path());
    let rel = format!("notes/{ID}.md");

    let found = vcs::conflicted(&vault).unwrap();
    assert_eq!(found.len(), 1, "exactly one unmerged path");
    assert_eq!(found[0].path, rel);
    assert_eq!(found[0].kind, git::ConflictKind::DeletedByUs, "we deleted it, they edited it");
    assert_eq!(found[0].kind.code(), "DU");

    // The load-bearing assertion: **there is nothing in the file to edit.** Any surface that tells
    // the user to open the note and keep the text they want is lying in this case.
    assert!(!found[0].kind.has_markers(), "a delete/modify conflict has no markers to resolve");
    let text = std::fs::read_to_string(vault.join(&rel)).unwrap();
    assert!(!text.contains("<<<<<<<"), "git wrote no markers, because it cannot");
    // And it is invisible to the marker scan the UI used to list conflicts from.
    assert!(!text.contains("======="), "no markers of any kind");
}

#[test]
fn keeping_theirs_restores_the_note_and_finishes_the_merge() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_delete_modify_conflict(dir.path());
    let rel = format!("notes/{ID}.md");
    assert!(vault.join(".git/MERGE_HEAD").exists(), "precondition: mid-merge");

    vcs::resolve_conflict(&vault, &rel, git::Keep::Theirs).unwrap();

    assert!(vcs::conflicted(&vault).unwrap().is_empty(), "nothing unmerged any more");
    // **The merge must be finished, not merely un-conflicted.** `commit_all` refuses while
    // MERGE_HEAD exists, and it stages only paths it remembers writing — so if resolving left
    // MERGE_HEAD standing, the vault would sit frozen with nothing left to resolve and no surface
    // able to explain why. That is the trap this asserts against.
    assert!(!vault.join(".git/MERGE_HEAD").exists(), "the merge is committed");
    let text = std::fs::read_to_string(vault.join(&rel)).expect("their note is on disk");
    assert!(text.contains("edited on the phone"), "their version won");
    let status = git_run(&vault, &["status", "--porcelain"]);
    assert!(String::from_utf8_lossy(&status.stdout).trim().is_empty(), "clean tree after resolving");
}

#[test]
fn keeping_mine_honours_the_deletion_and_finishes_the_merge() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_delete_modify_conflict(dir.path());
    let rel = format!("notes/{ID}.md");

    vcs::resolve_conflict(&vault, &rel, git::Keep::Mine).unwrap();

    assert!(vcs::conflicted(&vault).unwrap().is_empty());
    assert!(!vault.join(".git/MERGE_HEAD").exists(), "the merge is committed");
    assert!(!vault.join(&rel).exists(), "the deletion stands");
    let ls = git_run(&vault, &["ls-files", "--", &rel]);
    assert!(ls.stdout.is_empty(), "and git no longer tracks it");
}

#[test]
fn resolving_a_path_that_is_not_in_conflict_is_an_error_not_a_silent_success() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_delete_modify_conflict(dir.path());
    let err = vcs::resolve_conflict(&vault, "notes/nope.md", git::Keep::Theirs).unwrap_err();
    // Silence here would be the worst answer: a UI button that reports success and changes nothing
    // is how a user concludes the app is lying to them.
    assert!(format!("{err}").contains("not in conflict"), "says what was wrong: {err}");
}

#[test]
fn notes_the_app_never_staged_are_listed_and_can_be_recorded() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git_run(&vault, &["init", "-q", "-b", "main"]);
    git_run(&vault, &["config", "user.name", "Tester"]);
    git_run(&vault, &["config", "user.email", "t@example.com"]);
    std::fs::write(vault.join("notes/01AAAAAAAAAAAAAAAAAAAAAAAA.md"), note("01AAAAAAAAAAAAAAAAAAAAAAAA", "first")).unwrap();
    git_run(&vault, &["add", "-A"]);
    git_run(&vault, &["commit", "-qm", "one note"]);

    // Two notes written by a process that has since forgotten them — the state after any restart.
    for id in ["01BBBBBBBBBBBBBBBBBBBBBBBB", "01CCCCCCCCCCCCCCCCCCCCCCCC"] {
        std::fs::write(vault.join(format!("notes/{id}.md")), note(id, "written and forgotten")).unwrap();
    }
    // And one *edit* to a tracked note, which is equally forgotten.
    std::fs::write(vault.join("notes/01AAAAAAAAAAAAAAAAAAAAAAAA.md"), note("01AAAAAAAAAAAAAAAAAAAAAAAA", "first, edited")).unwrap();

    let missing = vcs::unrecorded(&vault, "notes").unwrap();
    assert_eq!(missing.len(), 3, "two new notes and one modified one: {missing:?}");

    // Recording them is exactly what `commit_all` does when it is *told* the paths — the point is
    // that nothing was telling it.
    let abs: Vec<std::path::PathBuf> = missing.iter().map(|r| vault.join(r)).collect();
    assert!(vcs::commit_all(&vault, "record them", &abs).unwrap(), "committed");
    assert!(vcs::unrecorded(&vault, "notes").unwrap().is_empty(), "nothing left unrecorded");
}

#[test]
fn a_conflicted_path_is_not_reported_as_an_unrecorded_note() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    // The two surfaces must not both claim the same note: a conflict has its own resolution, and
    // "record it" would stage a half-merged path — for a marker conflict, publishing the markers.
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_delete_modify_conflict(dir.path());
    let missing = vcs::unrecorded(&vault, "notes").unwrap();
    assert!(missing.is_empty(), "the conflicted note belongs to the conflict surface: {missing:?}");
}
