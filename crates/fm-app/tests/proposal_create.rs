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
fn a_proposal_against_an_uncommitted_note_commits_it_and_merges_cleanly() {
    // Regression (found in live use): a proposal against a note whose debounced auto-commit hasn't
    // fired must still be acceptable. Without committing the note first, the branch ADDS the file, and
    // accepting it hits an add/add conflict against main's untracked copy — `main` stays safe, but the
    // proposal can never merge. create_proposal now snapshots the note first, so accept merges cleanly.
    if !have_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    // A committed seed gives the repo a HEAD to branch from…
    let _seed = commands::capture(&mut store, "seed", "").unwrap();
    git::commit_all(dir.path(), "seed", &store.written()).unwrap();
    // …but the TARGET note is left UNCOMMITTED, exactly as a freshly-captured note is.
    let target = commands::capture(&mut store, "original body", "").unwrap().id;
    assert!(
        !show(dir.path(), &format!("HEAD:notes/{target}.md")).status.success(),
        "precondition: the target note is uncommitted"
    );

    let prop =
        commands::create_proposal(&mut store, dir.path(), &target, "revised body", &ProposalLimits::default(), None)
            .unwrap();

    // create_proposal committed the note, so it is now on HEAD…
    assert!(
        show(dir.path(), &format!("HEAD:notes/{target}.md")).status.success(),
        "create_proposal must commit the target note so the branch shares a base with main"
    );
    // …and accepting the proposal MERGES cleanly rather than conflicting.
    let outcome = commands::accept_proposal(&store, dir.path(), &prop.id).unwrap();
    assert!(matches!(outcome, git::Accepted::Merged), "expected a clean merge, got {outcome:?}");
    let merged = show(dir.path(), &format!("HEAD:notes/{target}.md"));
    assert!(
        String::from_utf8_lossy(&merged.stdout).contains("revised body"),
        "the accepted change did not land on main"
    );
}

#[test]
fn a_second_proposal_on_the_same_note_refines_the_first_instead_of_stacking() {
    // The living-PR model: `/research` / `/propose` again on a note REFINES its single proposal (same
    // proposal note + branch, new content) rather than opening a second one. This is what lets the
    // conversation continue on the original note without the proposal fragmenting.
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let first =
        commands::create_proposal(&mut store, p, &target, "first revision", &ProposalLimits::default(), None)
            .unwrap();
    let second =
        commands::create_proposal(&mut store, p, &target, "second revision", &ProposalLimits::default(), None)
            .unwrap();

    // Same proposal note — one living PR, not a new one — and only one is listed.
    assert_eq!(first.id, second.id, "a second proposal must refine the same PR, not open a new one");
    assert_eq!(commands::proposals(&store).unwrap().len(), 1, "exactly one open proposal per note");

    // The branch now carries the LATEST revision, not the stale first one.
    let branch = format!("proposal/{}", first.id);
    let on_branch = String::from_utf8_lossy(&show(p, &format!("{branch}:notes/{target}.md")).stdout).into_owned();
    assert!(on_branch.contains("second revision"), "branch not revised to the latest: {on_branch}");
    assert!(!on_branch.contains("first revision"), "the stale first revision remains: {on_branch}");
}

#[test]
fn rejecting_a_proposal_drops_the_branch_but_keeps_it_as_a_declined_record() {
    // Reject is the inverse of accept and symmetric with it: the branch goes away (the change is
    // dropped, `main` untouched) but the proposal note is KEPT as a declined record — a proposal is
    // immortal, like a merged one. It is no longer the note's *open* PR, so a fresh proposal opens anew.
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop =
        commands::create_proposal(&mut store, p, &target, "revised", &ProposalLimits::default(), None).unwrap();
    let branch = format!("proposal/{}", prop.id);
    assert!(git::branch_open(p, &branch), "precondition: the branch is open");

    commands::reject_proposal(&mut store, p, &prop.id).unwrap();

    // Branch gone; the note is kept as a declined record, but it is no longer the note's OPEN proposal.
    assert!(!git::branch_open(p, &branch), "reject must drop the branch");
    assert!(
        commands::proposals(&store).unwrap().iter().any(|o| o.id == prop.id),
        "the declined proposal is kept as a record, not deleted"
    );
    assert_eq!(
        commands::proposal_for(&store, p, &target).unwrap(),
        None,
        "a declined proposal is not the note's open PR"
    );
    // A fresh proposal opens a NEW PR rather than refining the declined one.
    let fresh =
        commands::create_proposal(&mut store, p, &target, "second try", &ProposalLimits::default(), None).unwrap();
    assert_ne!(fresh.id, prop.id, "after a reject a new proposal opens, not a refine of the declined one");
}

#[test]
fn the_proposed_note_is_readable_and_an_edit_revises_the_same_pr() {
    // The review needs to SHOW the proposed note and let the user EDIT it before accepting. An edit is
    // saved back through create_proposal, which revises the SAME PR (and rebuilds from current main —
    // that is also how a stale conflict gets resolved without a merge tool).
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop =
        commands::create_proposal(&mut store, p, &target, "the proposed body", &ProposalLimits::default(), None)
            .unwrap();

    let content = commands::proposal_content(&store, p, &prop.id).unwrap().expect("proposal has content");
    assert_eq!(content.host, target, "the content names the host note it edits");
    assert!(content.body.contains("the proposed body"), "the proposed body is readable: {}", content.body);

    // Saving an edit revises the same PR to the new body.
    let again =
        commands::create_proposal(&mut store, p, &target, "an edited body", &ProposalLimits::default(), None).unwrap();
    assert_eq!(again.id, prop.id, "editing revises the same PR, not a new one");
    let edited = commands::proposal_content(&store, p, &prop.id).unwrap().unwrap();
    assert!(edited.body.contains("an edited body"), "the edit is reflected on the branch: {}", edited.body);
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
    // A SECOND note, so the second proposal is a genuinely NEW PR. (A second proposal on the SAME note
    // just refines the first — the living-PR model — and rightly does NOT count against the ceiling.)
    let target2 = commands::capture(&mut store, "another note", "").unwrap().id;
    git::commit_all(p, "second note", &store.written()).unwrap();
    // Allow exactly one open proposal.
    let one = ProposalLimits { max_open: 1, ..ProposalLimits::default() };

    // First proposal: fine (0 open + this one = 1).
    commands::create_proposal(&mut store, p, &target, "first take", &one, None).unwrap();
    // A second proposal on a DIFFERENT note: refused (1 already open + this one = 2 > 1).
    let err = commands::create_proposal(&mut store, p, &target2, "second take", &one, None).unwrap_err();
    assert!(format!("{err}").contains("open proposals"), "expected an open-count refusal, got: {err}");

    // And a REVISE of an existing proposal is NOT refused at the ceiling (it replaces, not adds).
    commands::create_proposal(&mut store, p, &target, "first take, refined", &one, None)
        .expect("refining the existing proposal must not trip the open-count ceiling");
}
