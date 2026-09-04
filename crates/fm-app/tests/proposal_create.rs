//! `commands::create_proposal` end to end: a real `FileStore` over a real git repo. A proposal must
//! land its change on a `proposal/*` branch and a proposal note, while `main` (the working tree) is
//! left untouched — and the size guardrails must **refuse** an over-limit change, never truncate it.

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, FileStore};
use std::path::Path;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn show(repo: &Path, rev_path: &str) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", rev_path])
        .output()
        .unwrap()
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

    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised body",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let branch = format!("proposal/{}", prop.id);

    // The branch carries the revised note.
    let on_branch = show(p, &format!("{branch}:notes/{target}.md"));
    assert!(
        on_branch.status.success(),
        "branch should exist with the target file"
    );
    let branch_body = String::from_utf8_lossy(&on_branch.stdout);
    assert!(
        branch_body.contains("revised body"),
        "branch note not revised: {branch_body}"
    );

    // `main`'s copy on disk is still the original — the proposal did not touch it.
    let on_disk = std::fs::read_to_string(p.join(format!("notes/{target}.md"))).unwrap();
    assert!(
        on_disk.contains("original body"),
        "the live note was changed: {on_disk}"
    );
    assert!(!on_disk.contains("revised body"));

    // The proposal is what the Collaboration view lists.
    let listed = commands::proposals(&store).unwrap();
    assert!(
        listed.iter().any(|o| o.id == prop.id),
        "proposal not listed"
    );
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
    let target = commands::capture(&mut store, "original body", "")
        .unwrap()
        .id;
    assert!(
        !show(dir.path(), &format!("HEAD:notes/{target}.md"))
            .status
            .success(),
        "precondition: the target note is uncommitted"
    );

    let prop = commands::create_proposal(
        &mut store,
        dir.path(),
        &target,
        "revised body",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    // create_proposal committed the note, so it is now on HEAD…
    assert!(
        show(dir.path(), &format!("HEAD:notes/{target}.md"))
            .status
            .success(),
        "create_proposal must commit the target note so the branch shares a base with main"
    );
    // …and accepting the proposal MERGES cleanly rather than conflicting.
    let outcome = commands::accept_proposal(&store, dir.path(), &prop.id).unwrap();
    assert!(
        matches!(outcome, git::Accepted::Merged),
        "expected a clean merge, got {outcome:?}"
    );
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
    let first = commands::create_proposal(
        &mut store,
        p,
        &target,
        "first revision",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let second = commands::create_proposal(
        &mut store,
        p,
        &target,
        "second revision",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    // Same proposal note — one living PR, not a new one — and only one is listed.
    assert_eq!(
        first.id, second.id,
        "a second proposal must refine the same PR, not open a new one"
    );
    assert_eq!(
        commands::proposals(&store).unwrap().len(),
        1,
        "exactly one open proposal per note"
    );

    // The branch now carries the LATEST revision, not the stale first one.
    let branch = format!("proposal/{}", first.id);
    let on_branch =
        String::from_utf8_lossy(&show(p, &format!("{branch}:notes/{target}.md")).stdout)
            .into_owned();
    assert!(
        on_branch.contains("second revision"),
        "branch not revised to the latest: {on_branch}"
    );
    assert!(
        !on_branch.contains("first revision"),
        "the stale first revision remains: {on_branch}"
    );
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
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let branch = format!("proposal/{}", prop.id);
    assert!(
        git::branch_open(p, &branch),
        "precondition: the branch is open"
    );

    commands::reject_proposal(&mut store, p, &prop.id, None).unwrap();

    // Branch gone; the note is kept as a declined record, but it is no longer the note's OPEN proposal.
    assert!(!git::branch_open(p, &branch), "reject must drop the branch");
    assert!(
        commands::proposals(&store)
            .unwrap()
            .iter()
            .any(|o| o.id == prop.id),
        "the declined proposal is kept as a record, not deleted"
    );
    assert_eq!(
        commands::proposal_for(&store, p, &target).unwrap(),
        None,
        "a declined proposal is not the note's open PR"
    );
    // A fresh proposal opens a NEW PR rather than refining the declined one.
    let fresh = commands::create_proposal(
        &mut store,
        p,
        &target,
        "second try",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    assert_ne!(
        fresh.id, prop.id,
        "after a reject a new proposal opens, not a refine of the declined one"
    );
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
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "the proposed body",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    let content = commands::proposal_content(&store, p, &prop.id)
        .unwrap()
        .expect("proposal has content");
    assert_eq!(
        content.host, target,
        "the content names the host note it edits"
    );
    assert!(
        content.body.contains("the proposed body"),
        "the proposed body is readable: {}",
        content.body
    );

    // Saving an edit revises the same PR to the new body.
    let again = commands::create_proposal(
        &mut store,
        p,
        &target,
        "an edited body",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    assert_eq!(
        again.id, prop.id,
        "editing revises the same PR, not a new one"
    );
    let edited = commands::proposal_content(&store, p, &prop.id)
        .unwrap()
        .unwrap();
    assert!(
        edited.body.contains("an edited body"),
        "the edit is reflected on the branch: {}",
        edited.body
    );
}

#[test]
fn an_over_size_change_is_refused_not_truncated() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    // A tiny per-change ceiling: the serialized note is far larger than 10 bytes.
    let tight = ProposalLimits {
        max_change_bytes: 10,
        ..ProposalLimits::default()
    };
    let err = commands::create_proposal(
        &mut store,
        dir.path(),
        &target,
        "revised body",
        &tight,
        None,
        commands::Record::default(),
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("at most"),
        "expected a guardrail refusal, got: {err}"
    );
    // Nothing was created — no proposal branch, no proposal note.
    assert!(commands::proposals(&store).unwrap().is_empty());
    let branches = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args([
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/proposal/",
        ])
        .output()
        .unwrap();
    assert!(
        branches.stdout.is_empty(),
        "a refused proposal left a branch behind"
    );
}

#[test]
fn a_proposals_diff_shows_its_change_and_tolerates_a_gone_branch() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised body",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    let diff = commands::proposal_diff(&store, p, &prop.id).unwrap();
    assert!(diff.exists);
    assert!(
        diff.patch.contains("revised body"),
        "diff should show the change: {}",
        diff.patch
    );
    assert!(
        diff.files.iter().any(|f| f.contains(target.as_str())),
        "the changed file is listed"
    );

    // Delete the branch: the proposal note outlives it, so the diff reports `exists: false` rather
    // than erroring.
    let branch = format!("proposal/{}", prop.id);
    Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["branch", "-D", &branch])
        .output()
        .unwrap();
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
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised body",
        &ProposalLimits::default(),
        author,
        commands::Record::default(),
    )
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
    let target2 = commands::capture(&mut store, "another note", "")
        .unwrap()
        .id;
    git::commit_all(p, "second note", &store.written()).unwrap();
    // Allow exactly one open proposal.
    let one = ProposalLimits {
        max_open: 1,
        ..ProposalLimits::default()
    };

    // First proposal: fine (0 open + this one = 1).
    commands::create_proposal(
        &mut store,
        p,
        &target,
        "first take",
        &one,
        None,
        commands::Record::default(),
    )
    .unwrap();
    // A second proposal on a DIFFERENT note: refused (1 already open + this one = 2 > 1).
    let err = commands::create_proposal(
        &mut store,
        p,
        &target2,
        "second take",
        &one,
        None,
        commands::Record::default(),
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("open proposals"),
        "expected an open-count refusal, got: {err}"
    );

    // And a REVISE of an existing proposal is NOT refused at the ceiling (it replaces, not adds).
    commands::create_proposal(
        &mut store,
        p,
        &target,
        "first take, refined",
        &one,
        None,
        commands::Record::default(),
    )
    .expect("refining the existing proposal must not trip the open-count ceiling");
}

// ---------------------------------------------------------------------------
// The reviewer's reason. It is the single most valuable field in a supervision record and the one
// most likely to be dropped as redundant — "git already shows what changed". It does not: git shows
// *what*, never *why*, and an edit pair stored without its reason measured BELOW the untouched model
// on hard prompts (arXiv:2503.04378).
// ---------------------------------------------------------------------------

/// The sanitiser, on its own: prose written by a person is untrusted input to a format with rules.
#[test]
fn a_review_note_is_made_safe_for_a_commit_message() {
    // Nothing to say is a first-class answer, not a failure — a required prompt produces
    // satisficing, not reasons, so every one of these is a legitimate skip.
    for empty in ["", "   ", "\n\n", "---", "...", "  --- \n"] {
        assert_eq!(
            commands::review_note(empty),
            None,
            "{empty:?} should be treated as skipped"
        );
    }

    // `---` at column 0 terminates a commit message for git's own patch tooling, and EVERY note in
    // this vault begins with exactly that as YAML frontmatter — so a pasted fragment is a live
    // hazard, not a hypothetical one. Collapsing to one line leaves no delimiter to find.
    let pasted = "the model invented a citation\n---\nschema: 1\nid: 01ABC";
    let got = commands::review_note(pasted).unwrap();
    assert!(!got.contains('\n'), "must be one line, got {got:?}");
    assert!(got.starts_with("the model invented a citation"));

    // A final `Token: value` paragraph would otherwise be parsed as a git trailer and mix a human
    // sentence into the machine fields; one line makes that impossible too.
    assert_eq!(
        commands::review_note("  wrong  \n  name  "),
        Some("wrong name".into())
    );

    // Bounded: a reason is a sentence, not an essay pasted into history forever.
    assert_eq!(
        commands::review_note(&"x".repeat(900))
            .unwrap()
            .chars()
            .count(),
        500
    );
}

/// The reason reaches git as the commit BODY — never the subject, whose prefix is the squash
/// barrier (`push_squashed` collapses only `auto:`/`backup:`, so a sentence there would be pushed
/// away). Both the first proposal and a revision carry it.
#[test]
fn a_proposal_carries_the_reviewers_reason_as_the_commit_body() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised body",
        &ProposalLimits::default(),
        None,
        commands::Record {
            why: Some("the model misheard the electrolyte name"),
            ..Default::default()
        },
    )
    .unwrap();
    let branch = format!("proposal/{}", prop.id);
    let msg = |rev: &str| {
        String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(p)
                .args(["log", "-1", "--format=%B", rev])
                .output()
                .unwrap()
                .stdout,
        )
        .to_string()
    };
    let m = msg(&branch);
    let mut lines = m.lines();
    assert!(
        lines.next().unwrap().starts_with("propose: change to"),
        "subject must be unchanged"
    );
    assert_eq!(lines.next(), Some(""), "body is separated by a blank line");
    assert_eq!(
        lines.next(),
        Some("the model misheard the electrolyte name")
    );

    // A revision carries its own reason; a skipped one leaves a bare subject rather than failing.
    commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised again",
        &ProposalLimits::default(),
        None,
        commands::Record {
            why: Some("dropped the speculative paragraph"),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(msg(&branch).contains("dropped the speculative paragraph"));
    commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised once more",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    // No reason given: the body is skipped, but the machine trailers still ride along.
    let bare = msg(&branch);
    assert!(!bare.contains("dropped the speculative paragraph"));
    assert!(bare.contains("SchemaRev:"));
}

/// The machine half of the record. These are real git trailers, not text that merely looks like
/// them — asserted through git's own parser, because "it renders right" is a different claim.
#[test]
fn a_proposal_carries_machine_trailers_that_git_itself_parses() {
    if !have_git() {
        return;
    }
    let read = |repo: &std::path::Path, rev: &str, key: &str| {
        String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(repo)
                .args([
                    "log",
                    "-1",
                    &format!("--format=%(trailers:key={key},valueonly)"),
                    rev,
                ])
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string()
    };

    // A person proposing by hand is not "assisted by" anything, and supplies no input half.
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let mine = commands::create_proposal(
        &mut store,
        p,
        &target,
        "my own edit",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    let mine_branch = format!("proposal/{}", mine.id);
    assert_eq!(read(p, &mine_branch, "SchemaRev"), "1");
    assert_eq!(
        read(p, &mine_branch, "Assisted-by"),
        "",
        "a human proposal is not agent-assisted"
    );
    assert_eq!(read(p, &mine_branch, "Tool"), "");

    // An agent's proposal names the model — `Assisted-by`, never `Co-authored-by`: the ecosystem
    // rejected the latter because a model cannot hold accountability. And it records the INPUT,
    // without which the proposal is an answer with no question.
    let (dir2, mut store2, target2) = vault_with_a_note();
    let p2 = dir2.path();
    let sources = vec![
        "https://en.wikipedia.org/wiki/Electric_vehicle".to_string(),
        "https://en.wikipedia.org/wiki/Lithium-ion_battery".to_string(),
    ];
    let theirs = commands::create_proposal(
        &mut store2,
        p2,
        &target2,
        "the model's edit",
        &ProposalLimits::default(),
        Some(("qwen3-vl-4b", "qwen3-vl-4b@fm-agents.local")),
        commands::Record {
            // Deliberately shaped like a trailer: it must stay PROSE. Trailers are only the last
            // paragraph, so an earlier one cannot be mistaken for a field.
            why: Some("Assisted-by: not-a-real-model"),
            tool: Some("research"),
            query: Some("what battery chemistry is best for electric cars"),
            sources: &sources,
            kind: Some("factual"),
        },
    )
    .unwrap();
    let b = format!("proposal/{}", theirs.id);
    assert_eq!(
        read(p2, &b, "Assisted-by"),
        "formicaria-agent:qwen3-vl-4b",
        "the human sentence must not become a field",
    );
    assert_eq!(read(p2, &b, "Tool"), "research");
    assert_eq!(
        read(p2, &b, "Query"),
        "what battery chemistry is best for electric cars"
    );
    let cited = read(p2, &b, "Sources");
    assert!(cited.contains("Electric_vehicle") && cited.contains("Lithium-ion_battery"));
    assert!(
        !cited.contains('\n'),
        "a trailer is single-line by definition"
    );

    // The one axis the corpus cannot be split on later: style saturates after ~1k examples while
    // factual and reasoning keep climbing, so unlabelled the two are indistinguishable.
    assert_eq!(read(p2, &b, "Kind"), "factual");
}

/// `Kind` is a closed vocabulary. A free-text axis is one nobody can group by, and grouping by it is
/// the entire point — so anything outside the set is dropped, never stored.
#[test]
fn an_unknown_correction_kind_is_dropped_rather_than_stored() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised",
        &ProposalLimits::default(),
        None,
        commands::Record {
            kind: Some("vibes"),
            ..Default::default()
        },
    )
    .unwrap();
    let out = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(p)
            .args([
                "log",
                "-1",
                "--format=%(trailers:key=Kind,valueonly)",
                &format!("proposal/{}", prop.id),
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();
    assert_eq!(out, "", "an unknown kind must not reach the record");
    assert!(commands::REVIEW_KINDS.contains(&"style"));
}

/// Reject writes no commit of its own (`main` is untouched by design) and the rejected text never
/// reaches `main` at all — so the proposal note is the ONLY place a rejection's reason can survive.
#[test]
fn a_rejection_keeps_its_reason_on_the_proposal_note() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "revised body",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    commands::reject_proposal(
        &mut store,
        p,
        &prop.id,
        Some("invented a source that does not exist"),
    )
    .unwrap();

    let obj = fm_core::Store::get(&store, prop.id.parse().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        obj.get(fm_app::thread::DECLINED_WHY),
        fm_model::PropertyValue::Text("invented a source that does not exist".into()),
    );
    // Still a declined record, as before — the reason rides along, it does not replace anything.
    assert_eq!(
        obj.get(fm_app::thread::DECLINED),
        fm_model::PropertyValue::Bool(true)
    );
}

/// A settled proposal must not hold a guardrail slot hostage — and retiring it must keep its text.
///
/// The path that bites is a **peer's** rejection: only their copy of the branch is deleted, and
/// `pull` fetches without `--prune`, so ours survives while the note arrives marked declined.
/// `proposal_load` then counts it against `max_open` forever, with no way for a UI-only user to
/// clear it — proposals simply start being refused.
#[test]
fn a_settled_proposal_stops_holding_a_slot_but_keeps_its_text() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    let target2 = commands::capture(&mut store, "another note", "")
        .unwrap()
        .id;
    git::commit_all(p, "second note", &store.written()).unwrap();
    let one = ProposalLimits {
        max_open: 1,
        ..ProposalLimits::default()
    };

    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "the model's text",
        &one,
        None,
        commands::Record::default(),
    )
    .unwrap();
    let branch = format!("proposal/{}", prop.id);
    let tip = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["rev-parse", &format!("refs/heads/{branch}")])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();
    assert_eq!(git::proposal_load(p).unwrap().0, 1);

    // Exactly what a peer's reject leaves behind: the note says declined, our branch does not know.
    let mut obj = fm_core::Store::get(&store, prop.id.parse().unwrap())
        .unwrap()
        .unwrap();
    obj.extra.insert(
        fm_app::thread::DECLINED.into(),
        fm_model::PropertyValue::Bool(true),
    );
    fm_core::Store::put(&mut store, &obj).unwrap();

    // A new proposal on a different note now succeeds: the settled one was retired, not counted.
    commands::create_proposal(
        &mut store,
        p,
        &target2,
        "a later proposal",
        &one,
        None,
        commands::Record::default(),
    )
    .expect("a settled proposal must not block a new one");

    // The slot is freed...
    let refs = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(p)
            .args([
                "for-each-ref",
                "--format=%(refname)",
                &format!("refs/heads/{branch}"),
            ])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();
    assert_eq!(refs, "", "the settled branch should be retired");

    // ...and the data is NOT. Retiring joins the unlabelled pool; it does not drop it.
    let kept = show(p, &format!("{tip}:notes/{target}.md"));
    assert!(
        kept.status.success(),
        "the retired proposal's commit must still be readable"
    );
    assert!(String::from_utf8_lossy(&kept.stdout).contains("the model's text"));
}

/// Consent travels **with the record**, not merely with the vault.
///
/// Two answers, kept separate because the first does not imply the second — the distinction that
/// froze the only comparable open corpus of AI corrections. And stamped per record, because a flag
/// flipped next year must not silently relicense everything captured before it.
#[test]
fn every_record_carries_the_consent_it_was_made_under() {
    if !have_git() {
        return;
    }
    let read = |repo: &std::path::Path, rev: &str, key: &str| {
        String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(repo)
                .args([
                    "log",
                    "-1",
                    &format!("--format=%(trailers:key={key},valueonly)"),
                    rev,
                ])
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string()
    };
    let propose = |dir: &tempfile::TempDir, store: &mut FileStore, target: &str, body: &str| {
        commands::create_proposal(
            store,
            dir.path(),
            target,
            body,
            &ProposalLimits::default(),
            None,
            commands::Record::default(),
        )
        .unwrap()
    };

    // Default: collect locally, publish nothing. Publication is never inferred — it cannot be recalled.
    let (dir, mut store, target) = vault_with_a_note();
    let a = propose(&dir, &mut store, &target, "default");
    assert_eq!(
        read(dir.path(), &format!("proposal/{}", a.id), "Consent"),
        "local"
    );

    // Granted explicitly in the vault's own file — not per device, because publishing relicenses
    // shared content and the loosest machine must not decide for everyone.
    let (dir2, mut store2, target2) = vault_with_a_note();
    std::fs::write(
        dir2.path().join("vault.json"),
        r#"{"supervision":{"collect":true,"publish":true}}"#,
    )
    .unwrap();
    let b = propose(&dir2, &mut store2, &target2, "granted");
    assert_eq!(
        read(dir2.path(), &format!("proposal/{}", b.id), "Consent"),
        "local,publish"
    );

    // Opted out: no record at all, not merely a flag saying so. The subject the app has always
    // written, and nothing else.
    let (dir3, mut store3, target3) = vault_with_a_note();
    std::fs::write(
        dir3.path().join("vault.json"),
        r#"{"supervision":{"collect":false}}"#,
    )
    .unwrap();
    let c = propose(&dir3, &mut store3, &target3, "opted out");
    let br = format!("proposal/{}", c.id);
    assert_eq!(read(dir3.path(), &br, "Consent"), "");
    assert_eq!(read(dir3.path(), &br, "SchemaRev"), "");
    let msg = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(dir3.path())
            .args(["log", "-1", "--format=%B", &br])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();
    assert_eq!(
        msg.lines().count(),
        1,
        "opting out must leave no record, got: {msg:?}"
    );
}

/// "Shown" is stamped once, server-side, and never overwritten.
///
/// It is the only thing that separates *the reviewer read this and left it alone* from *nobody ever
/// opened it* — two absences that look identical in the data and mean opposite things about the
/// model. Re-stamping would destroy the other half of its value: the gap between being shown and
/// being decided is how long someone actually spent.
#[test]
fn a_proposal_records_the_first_time_it_was_actually_shown() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let prop = commands::create_proposal(
        &mut store,
        dir.path(),
        &target,
        "revised",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    let shown = |st: &FileStore| {
        fm_core::Store::get(st, prop.id.parse().unwrap())
            .unwrap()
            .unwrap()
            .get(fm_app::thread::SHOWN)
    };
    assert_eq!(
        shown(&store),
        fm_model::PropertyValue::Null,
        "unseen until someone looks"
    );

    assert!(
        commands::mark_proposal_shown(&mut store, &prop.id).unwrap(),
        "first sighting writes"
    );
    let first = shown(&store);
    let fm_model::PropertyValue::Text(stamp) = first.clone() else {
        panic!("expected a timestamp, got {first:?}")
    };
    assert!(
        stamp.contains('T') && stamp.ends_with('Z'),
        "expected RFC3339 UTC, got {stamp:?}"
    );

    // A second viewing is not a second FIRST viewing.
    assert!(
        !commands::mark_proposal_shown(&mut store, &prop.id).unwrap(),
        "no write the second time"
    );
    assert_eq!(
        shown(&store),
        first,
        "the first sighting must not be overwritten"
    );

    // Only proposals have the concept at all.
    let err = commands::mark_proposal_shown(&mut store, &target).unwrap_err();
    assert!(format!("{err}").contains("not a proposal"), "got: {err}");
}

/// **The whole record, composed.** Every field above is tested on its own; this asserts they add up
/// to a well-formed commit message rather than merely coexisting.
///
/// The failure this exists to catch is compositional: two features that are each correct can still
/// produce a message where the trailers are not the final paragraph, or a blank line lands in the
/// wrong place — and at that point git stops parsing the machine fields at all, silently, while
/// every unit test still passes.
#[test]
fn the_whole_record_composes_into_one_well_formed_message() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();
    std::fs::write(
        p.join("vault.json"),
        r#"{"supervision":{"collect":true,"publish":true}}"#,
    )
    .unwrap();
    let sources = vec!["https://example.org/a".to_string()];
    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "the corrected text",
        &ProposalLimits::default(),
        Some(("qwen3-vl-4b", "qwen3-vl-4b@fm-agents.local")),
        commands::Record {
            why: Some("misheard the electrolyte name @wrong"),
            tool: Some("transcribe"),
            query: Some("what did the recording say"),
            sources: &sources,
            kind: Some("factual"),
        },
    )
    .unwrap();

    let msg = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["log", "-1", "--format=%B", &format!("proposal/{}", prop.id)])
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();

    // Three paragraphs, in this order and no other: what happened, why, and the machine fields.
    let paras: Vec<&str> = msg.split("\n\n").collect();
    assert_eq!(
        paras.len(),
        3,
        "expected subject / reason / trailers, got:\n{msg}"
    );
    assert!(paras[0].starts_with("propose: change to"));
    assert_eq!(paras[1], "misheard the electrolyte name @wrong");

    // The trailer block is last — the only position git parses — and every key is single-line.
    for line in paras[2].lines() {
        assert!(line.contains(": "), "not a trailer: {line:?}");
    }
    let keys: Vec<&str> = paras[2]
        .lines()
        .filter_map(|l| l.split(':').next())
        .collect();
    assert_eq!(
        keys,
        vec![
            "SchemaRev",
            "Assisted-by",
            "Tool",
            "Query",
            "Sources",
            "Kind",
            "Consent"
        ],
        "the record's shape is versioned by SchemaRev — changing it means bumping that",
    );

    // And git agrees, which is the claim that actually matters.
    let read = |key: &str| {
        String::from_utf8_lossy(
            &Command::new("git")
                .arg("-C")
                .arg(p)
                .args([
                    "log",
                    "-1",
                    &format!("--format=%(trailers:key={key},valueonly)"),
                    &format!("proposal/{}", prop.id),
                ])
                .output()
                .unwrap()
                .stdout,
        )
        .trim()
        .to_string()
    };
    assert_eq!(read("Kind"), "factual");
    assert_eq!(read("Consent"), "local,publish");
    assert_eq!(read("Tool"), "transcribe");
}

/// **A proposal its author turned down must never be merged into `main`.**
///
/// This is the state a peer's reject leaves on *our* machine, and it is not exotic: `reject_proposal`
/// deletes the branch only where it runs. The rejecter's note arrives on our next pull marked
/// `declined`, while `refs/heads/proposal/<id>` — our own local branch — survives, because no fetch
/// flag touches a local head. `retire_settled_proposals` does sweep it, but only when the guardrail
/// ceiling is checked, i.e. when a *new* proposal is created. Between the pull and that moment,
/// `accept_proposal` used to merge withdrawn text straight into `main`.
///
/// **The second assertion is the one that matters.** A test that only checked the error type would
/// still pass against a guard placed *after* the merge — so this reads `main` and requires the
/// proposed text to be absent from it.
#[test]
fn accepting_a_proposal_that_was_turned_down_is_refused_and_main_is_untouched() {
    if !have_git() {
        return;
    }
    let (dir, mut store, target) = vault_with_a_note();
    let p = dir.path();

    let prop = commands::create_proposal(
        &mut store,
        p,
        &target,
        "the withdrawn text",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();

    // Exactly what a peer's reject leaves behind: the note says declined, our branch does not know.
    let mut obj = fm_core::Store::get(&store, prop.id.parse().unwrap())
        .unwrap()
        .unwrap();
    obj.extra.insert(
        fm_app::thread::DECLINED.into(),
        fm_model::PropertyValue::Bool(true),
    );
    fm_core::Store::put(&mut store, &obj).unwrap();

    let err = commands::accept_proposal(&store, p, &prop.id)
        .expect_err("a declined proposal must not be accepted");
    let msg = format!("{err}");
    assert!(msg.contains("turned down"), "the refusal must say why: {msg}");
    assert!(msg.contains("not touched"), "and that main is safe: {msg}");

    // **The assertion the guard exists for.** Read `main` itself, not the return value.
    let head = String::from_utf8_lossy(
        &Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["show", &format!("HEAD:notes/{target}.md")])
            .output()
            .unwrap()
            .stdout,
    )
    .to_string();
    assert!(
        !head.contains("the withdrawn text"),
        "the rejected text reached main:\n{head}"
    );
}
