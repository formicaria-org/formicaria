//! `git::create_proposal_branch` against real git. The contract being tested is with git itself —
//! that a proposal branch appears with the proposed file on it, while `HEAD`, the working tree, and
//! the index are left **exactly** as they were. "It only ever adds a branch" is the whole safety
//! argument for proposals, so it is verified against the tool, not asserted in prose.

use fm_core::git;
use std::path::Path;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn g(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git").arg("-C").arg(repo).args(args).output().unwrap()
}

fn out(repo: &Path, args: &[&str]) -> String {
    String::from_utf8_lossy(&g(repo, args).stdout).trim().to_string()
}

/// A repo with one committed note, ready to propose against.
fn repo_with_a_note() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    g(p, &["init"]);
    g(p, &["config", "user.email", "t@example.com"]);
    g(p, &["config", "user.name", "Tester"]);
    std::fs::create_dir_all(p.join("notes")).unwrap();
    std::fs::write(p.join("notes/x.md"), "original body").unwrap();
    g(p, &["add", "-A"]);
    g(p, &["commit", "-m", "first"]);
    dir
}

#[test]
fn it_creates_a_branch_with_the_change_and_touches_nothing_else() {
    if !have_git() {
        return;
    }
    let dir = repo_with_a_note();
    let p = dir.path();
    let head_before = out(p, &["rev-parse", "HEAD"]);

    git::create_proposal_branch(p, "proposal/01ABC", "notes/x.md", "PROPOSED body", "propose: edit x")
        .unwrap();

    // The branch exists and carries the proposed content.
    assert!(g(p, &["rev-parse", "--verify", "refs/heads/proposal/01ABC"]).status.success());
    assert_eq!(out(p, &["show", "proposal/01ABC:notes/x.md"]), "PROPOSED body");
    assert_eq!(out(p, &["log", "-1", "--format=%s", "proposal/01ABC"]), "propose: edit x");

    // HEAD did not move; the working tree still holds the ORIGINAL; nothing is staged/dirty.
    assert_eq!(out(p, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(std::fs::read_to_string(p.join("notes/x.md")).unwrap(), "original body");
    assert_eq!(out(p, &["status", "--porcelain"]), "");

    // The temp index was cleaned up.
    let leftover: Vec<_> = std::fs::read_dir(p.join(".git"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("fm-proposal-"))
        .collect();
    assert!(leftover.is_empty(), "temp index left behind: {leftover:?}");
}

#[test]
fn a_brand_new_file_is_added_on_the_branch_only() {
    if !have_git() {
        return;
    }
    let dir = repo_with_a_note();
    let p = dir.path();

    git::create_proposal_branch(p, "proposal/01NEW", "notes/fresh.md", "a new note", "propose: new note")
        .unwrap();

    // On the branch the new file exists; on HEAD it does not.
    assert_eq!(out(p, &["show", "proposal/01NEW:notes/fresh.md"]), "a new note");
    assert!(!g(p, &["cat-file", "-e", "HEAD:notes/fresh.md"]).status.success());
    assert!(!p.join("notes/fresh.md").exists(), "new file leaked into the working tree");
}

#[test]
fn it_refuses_to_clobber_an_existing_branch() {
    if !have_git() {
        return;
    }
    let dir = repo_with_a_note();
    let p = dir.path();
    git::create_proposal_branch(p, "proposal/01DUP", "notes/x.md", "one", "m").unwrap();
    let err = git::create_proposal_branch(p, "proposal/01DUP", "notes/x.md", "two", "m").unwrap_err();
    assert!(format!("{err}").contains("already exists"), "got: {err}");
    // The first proposal's content is intact — the refused second write changed nothing.
    assert_eq!(out(p, &["show", "proposal/01DUP:notes/x.md"]), "one");
}

#[test]
fn it_refuses_when_the_repo_has_no_commits() {
    if !have_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    g(p, &["init"]);
    let err = git::create_proposal_branch(p, "proposal/01EMPTY", "notes/x.md", "body", "m").unwrap_err();
    assert!(format!("{err}").contains("no commits"), "got: {err}");
}
