//! `commands::create_proposal` end to end: a real `FileStore` over a real git repo. A proposal must
//! land its change on a `proposal/*` branch and a proposal note, while `main` (the working tree) is
//! left untouched — and the size guardrails must **refuse** an over-limit change, never truncate it.

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, FileStore};
use std::path::Path;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn show(repo: &Path, rev_path: &str) -> std::process::Output {
    Command::new("git").arg("-C").arg(repo).args(["show", rev_path]).output().unwrap()
}

/// A store over a fresh git repo with one committed note; returns (dir, store, target-note-id).
fn vault_with_a_note() -> (tempfile::TempDir, FileStore, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    let target = commands::capture(&mut store, "original body", "").unwrap();
    // Commit the note so `main` has a HEAD to branch from.
    git::commit_all(dir.path(), "first", &store.written()).unwrap();
    (dir, store, target.id)
}

#[test]
fn a_proposal_lands_on_a_branch_and_a_note_leaving_main_untouched() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();

    let prop =
        commands::create_proposal(&mut store, p, &target, "revised body", &ProposalLimits::default(), None)
            .unwrap();
    let branch = format!("proposal/{}", prop.id);

    // The branch carries the revised note.
    let on_branch = show(p, &format!("{branch}:notes/{target}.md"));
    assert!(on_branch.status.success(), "branch should exist with the target file");
    let branch_body = String::from_utf8_lossy(&on_branch.stdout);
    assert!(branch_body.contains("revised body"), "branch note not revised: {branch_body}");

    // `main`'s copy on disk is still the original — the proposal did not touch it.
    let on_disk = std::fs::read_to_string(p.join(format!("notes/{target}.md"))).unwrap();
    assert!(on_disk.contains("original body"), "the live note was changed: {on_disk}");
    assert!(!on_disk.contains("revised body"));

    // The proposal is what the Collaboration view lists.
    let listed = commands::proposals(&store).unwrap();
    assert!(listed.iter().any(|o| o.id == prop.id), "proposal not listed");
}

#[test]
fn an_over_size_change_is_refused_not_truncated() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    // A tiny per-change ceiling: the serialized note is far larger than 10 bytes.
    let tight = ProposalLimits { max_change_bytes: 10, ..ProposalLimits::default() };
    let err = commands::create_proposal(&mut store, dir.path(), &target, "revised body", &tight, None)
        .unwrap_err();
    assert!(format!("{err}").contains("at most"), "expected a guardrail refusal, got: {err}");
    // Nothing was created — no proposal branch, no proposal note.
    assert!(commands::proposals(&store).unwrap().is_empty());
    let branches = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["for-each-ref", "--format=%(refname:short)", "refs/heads/proposal/"])
        .output()
        .unwrap();
    assert!(branches.stdout.is_empty(), "a refused proposal left a branch behind");
}

#[test]
fn a_proposals_diff_shows_its_change_and_tolerates_a_gone_branch() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop =
        commands::create_proposal(&mut store, p, &target, "revised body", &ProposalLimits::default(), None)
            .unwrap();

    let diff = commands::proposal_diff(&store, p, &prop.id).unwrap();
    assert!(diff.exists);
    assert!(diff.patch.contains("revised body"), "diff should show the change: {}", diff.patch);
    assert!(diff.files.iter().any(|f| f.contains(target.as_str())), "the changed file is listed");

    // Delete the branch: the proposal note outlives it, so the diff reports `exists: false` rather
    // than erroring.
    let branch = format!("proposal/{}", prop.id);
    Command::new("git").arg("-C").arg(p).args(["branch", "-D", &branch]).output().unwrap();
    let gone = commands::proposal_diff(&store, p, &prop.id).unwrap();
    assert!(!gone.exists);
    assert!(gone.patch.is_empty());
}

#[test]
fn a_proposal_is_attributed_to_its_model_when_one_is_given() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    // An agent proposes as its model — the commit's author reflects that.
    let author = Some(("lfm2.5-230m", "lfm2.5-230m@fm-agents.local"));
    let prop =
        commands::create_proposal(&mut store, p, &target, "revised body", &ProposalLimits::default(), author)
            .unwrap();
    let branch = format!("proposal/{}", prop.id);
    let who = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["log", "-1", "--format=%an <%ae>", &branch])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();
    assert_eq!(who, "lfm2.5-230m <lfm2.5-230m@fm-agents.local>");
}

#[test]
fn the_per_vault_open_count_is_enforced() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    // Allow exactly one open proposal.
    let one = ProposalLimits { max_open: 1, ..ProposalLimits::default() };

    // First proposal: fine (0 open + this one = 1).
    commands::create_proposal(&mut store, p, &target, "first take", &one, None).unwrap();
    // Second: refused (1 already open + this one = 2 > 1).
    let err = commands::create_proposal(&mut store, p, &target, "second take", &one, None).unwrap_err();
    assert!(format!("{err}").contains("open proposals"), "expected an open-count refusal, got: {err}");
}
