//! The whole cycle a user actually drives, end to end on a **real git repo**: a **discussion inside a
//! note**, an **agent proposal** off that discussion, a **review** of its diff, and **accepting** it.
//! No model runs — the model's only job is to author the proposed *body*; every step around it is
//! deterministic, and that deterministic machinery is exactly what this exercises with a real
//! `FileStore`, so "does the propose→accept feature work in realistic settings" gets a yes/no answer.
//!
//! It also pins down what "accept" *is* today: a plain **`git merge` of the `proposal/<id>` branch**.
//! formicaria creates and *reviews* proposals in-app (`create_proposal` + `proposal_diff` /
//! `ProposalReview.svelte`), but has **no in-app merge/accept command yet** — so the accept step here
//! is the git a user runs by hand. If an `accept_proposal` command is ever added, this is the
//! behaviour it must reproduce: the branch merges cleanly into `main`, the edit lands, and the
//! discussion that produced it survives.

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, FileStore};
use std::path::Path;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn git_run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git").arg("-C").arg(repo).args(args).output().unwrap()
}

#[test]
fn a_note_discussion_yields_a_proposal_that_is_reviewed_then_accepted() {
    if !have_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let mut store = FileStore::open(p).unwrap();

    // 1. An ordinary note — the host the whole cycle hangs off.
    let host = commands::capture(&mut store, "# Talk with Arne\n\nrough notes", "").unwrap().id;
    git::commit_all(p, "note: talk with arne", &store.written()).unwrap();

    // 2. A discussion *in that note* — a comment thread on an ordinary note (`thread_of` → the host),
    //    NOT a first-class discussion. This is the exact case that was silently unwatched earlier this
    //    session; the cycle has to start from it. A human asks; the agent answers and offers a draft.
    commands::reply(&mut store, &host, "@qwen3-4b-2507 tidy these notes and add a summary?")
        .unwrap();
    commands::reply(&mut store, &host, "On it — I've drafted a cleaned-up version. /propose")
        .unwrap();
    git::commit_all(p, "discuss: talk with arne", &store.written()).unwrap();

    let t = commands::thread(&store, &host).unwrap();
    assert_eq!(t.count, 2, "the note carries a two-message discussion");

    // 3. The agent proposes an edit to the host note. In production the model authors `body`; the
    //    store-aware runner then calls exactly this, attributing the commit to the model's identity.
    let author = Some(("qwen3-4b-2507", "qwen3-4b-2507@fm-agents.local"));
    let body = "# Talk with Arne\n\n## Summary\n\nArne and I agreed to ship the assistant on-device.\n\n## Notes\n\nrough notes";
    let prop = commands::create_proposal(
        &mut store,
        p,
        &host,
        body,
        &ProposalLimits::default(),
        author,
        commands::Record::default(),
    )
    .unwrap();
    // Record the proposal *note* on main so the Collaboration feed lists it — what dispatch does after
    // a successful create_proposal.
    git::commit_all(p, "backup: proposal", &store.written()).unwrap();

    // main is deliberately untouched: the live note still reads the original until a human accepts.
    let live = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    assert!(
        live.contains("rough notes") && !live.contains("## Summary"),
        "a proposal must never change main on its own: {live}"
    );

    // 4. Review — the read half of accepting: the diff shows the proposed change, and the branch is
    //    attributed to the model that made it (legible provenance, not a trust boundary).
    let diff = commands::proposal_diff(&store, p, &prop.id).unwrap();
    assert!(diff.exists, "an open proposal has a reviewable diff");
    assert!(diff.patch.contains("## Summary"), "the diff shows the new content: {}", diff.patch);
    assert!(
        diff.files.iter().any(|f| f.contains(host.as_str())),
        "the diff names the host note as the changed file"
    );
    let who = String::from_utf8_lossy(
        &git_run(p, &["log", "-1", "--format=%an", &format!("proposal/{}", prop.id)]).stdout,
    )
    .trim()
    .to_string();
    assert_eq!(who, "qwen3-4b-2507", "the proposal is attributed to the model that made it");

    // 5. Accept = merge the proposal branch into main. There is no in-app command for this yet, so
    //    this is the git a user runs by hand. It merges cleanly with no driver and no conflict: the
    //    branch touched only the host note; main only added the proposal note (disjoint files).
    let merge = git_run(p, &["merge", "--no-ff", "--no-edit", &format!("proposal/{}", prop.id)]);
    assert!(
        merge.status.success(),
        "accepting the proposal must merge cleanly into main: {}",
        String::from_utf8_lossy(&merge.stderr)
    );

    // 6. The accepted change is now live on main...
    let after = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    assert!(
        after.contains("## Summary") && after.contains("on-device"),
        "the accepted edit is now on main: {after}"
    );
    // ...and the discussion that produced it survived the merge (re-read from disk, as the app does
    // after a merge rewrites files under it).
    let reread = FileStore::open(p).unwrap();
    assert_eq!(
        commands::thread(&reread, &host).unwrap().count,
        2,
        "accepting a proposal leaves the note's discussion intact"
    );

    // 7. A completed proposal: drop the merged branch. The review surface then reports it as gone
    //    ("merged or deleted") rather than erroring — the proposal note outlives its branch, exactly
    //    the `exists: false` path ProposalReview.svelte renders.
    git_run(p, &["branch", "-D", &format!("proposal/{}", prop.id)]);
    let done = commands::proposal_diff(&store, p, &prop.id).unwrap();
    assert!(
        !done.exists && done.patch.is_empty(),
        "a merged proposal's branch is gone; the note lives on"
    );
}

/// Accepting through the app's own command — the backend behind the GUI's "Accept" button — rather
/// than raw `git merge`. A UI-only user never runs git, so this path has to work: resolve the
/// proposal's branch, merge it into main, delete the branch, and be idempotent if pressed twice.
#[test]
fn accept_proposal_the_gui_button_merges_the_branch_into_main() {
    if !have_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let mut store = FileStore::open(p).unwrap();

    let host = commands::capture(&mut store, "# Note\n\noriginal", "").unwrap().id;
    git::commit_all(p, "note", &store.written()).unwrap();
    let prop = commands::create_proposal(
        &mut store,
        p,
        &host,
        "# Note\n\nrevised",
        &ProposalLimits::default(),
        None,
        commands::Record::default(),
    )
    .unwrap();
    git::commit_all(p, "backup: proposal", &store.written()).unwrap();

    // Accept — the merge the GUI button triggers.
    assert_eq!(commands::accept_proposal(&store, p, &prop.id).unwrap(), git::Accepted::Merged);

    // The change is live on main...
    let after = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    assert!(
        after.contains("revised") && !after.contains("original"),
        "the accepted edit is on main: {after}"
    );
    // ...the branch is gone, so the review surface reports it done...
    assert!(
        !commands::proposal_diff(&store, p, &prop.id).unwrap().exists,
        "the branch is deleted after accept"
    );
    // ...and pressing Accept again is a harmless no-op, not an error.
    assert_eq!(commands::accept_proposal(&store, p, &prop.id).unwrap(), git::Accepted::AlreadyGone);
}

/// The realistic cycle: the first draft is **not** accepted. The user reads it, is unhappy with
/// *specific parts*, says so, the agent revises, and this repeats **several rounds** before the user
/// finally accepts. Each round is a fresh `proposal/<id>` branch; a version the user rejects is
/// **declined** (its branch deleted, freeing the open-proposal slot) — the design's "a branch a human
/// declines" — and only the version that satisfies them is merged. The whole exchange stays in the
/// note's discussion as an audit trail, and every attempt is still listed as a proposal note even
/// though only one branch was ever merged.
///
/// (`create_proposal` with `author = None` attributes to the vault's own identity, so the exact same
/// path serves a **human collaborator** proposing by hand — here every round is the agent, but nothing
/// about the machinery is agent-specific.)
#[test]
fn a_user_iterates_with_the_agent_over_several_rounds_before_accepting() {
    if !have_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let mut store = FileStore::open(p).unwrap();

    // Two different agents on two different devices: the laptop's default (qwen3-4b) and the phone's
    // (lfm2.5-1.2b). The cycle must not care which one — a user can start a refinement on the laptop
    // and continue it on the phone, or have two agents each take a round. The commit author records
    // which model did which round; nothing else in the machinery is model- or device-specific.
    let laptop = Some(("qwen3-4b-2507", "qwen3-4b-2507@fm-agents.local"));
    let phone = Some(("lfm2.5-1.2b", "lfm2.5-1.2b@fm-agents.local"));

    // The note whose discussion the whole back-and-forth happens in.
    let host =
        commands::capture(&mut store, "# Talk with Arne\n\nrough notes about the roadmap", "")
            .unwrap()
            .id;
    git::commit_all(p, "note: talk with arne", &store.written()).unwrap();

    // A small helper for one round: the user's message, the answering agent (which model, on which
    // device), its reply, and its proposed body. Returns the proposal's id so the caller can review,
    // then either decline or accept it.
    let round = |store: &mut FileStore,
                 user: &str,
                 by: Option<(&str, &str)>,
                 agent_says: &str,
                 body: &str|
     -> String {
        commands::reply(store, &host, user).unwrap();
        commands::reply(store, &host, agent_says).unwrap();
        let prop = commands::create_proposal(
            store,
            p,
            &host,
            body,
            &ProposalLimits::default(),
            by,
            commands::Record::default(),
        )
        .unwrap();
        git::commit_all(p, "discuss + backup: proposal", &store.written()).unwrap();
        prop.id
    };
    let author_of = |id: &str| -> String {
        String::from_utf8_lossy(
            &git_run(p, &["log", "-1", "--format=%an", &format!("proposal/{id}")]).stdout,
        )
        .trim()
        .to_string()
    };
    // Declining a version = deleting its branch: the review surface then reports it gone, and the
    // per-vault open-proposal slot is freed for the next attempt.
    let decline = |id: &str| {
        git_run(p, &["branch", "-D", &format!("proposal/{id}")]);
    };
    let open_count = || git::proposal_load(p).unwrap().0;

    // --- Round 1 (laptop, qwen): too terse. The user wanted the action items kept, not just a summary. ---
    let v1 = round(
        &mut store,
        "@qwen3-4b-2507 can you tidy this and add a short summary?",
        laptop,
        "Sure — here's a cleaned-up draft. /propose",
        "# Talk with Arne\n\n## Summary\n\nWe agreed to ship the assistant on-device.",
    );
    assert_eq!(open_count(), 1, "round 1 leaves exactly one open proposal");
    assert_eq!(author_of(&v1), "qwen3-4b-2507", "round 1 was the laptop model");
    let d1 = commands::proposal_diff(&store, p, &v1).unwrap();
    assert!(
        d1.patch.contains("## Summary") && !d1.patch.contains("Action items"),
        "v1 is summary-only: {}",
        d1.patch
    );
    decline(&v1);
    assert!(!commands::proposal_diff(&store, p, &v1).unwrap().exists, "the declined v1 is gone");
    assert_eq!(open_count(), 0, "declining frees the open-proposal slot");

    // --- Round 2 (phone, lfm2.5): the user picked the conversation back up on the phone. Better, but
    //     wrong order and a typo — summary should lead, and "Arnne" is misspelled. ---
    let v2 = round(
        &mut store,
        "Better, but keep the action items too — and lead with the summary.",
        phone,
        "Good call — added the action items. /propose",
        "# Talk with Arnne\n\n## Action items\n\n- Bundle the runtime\n\n## Summary\n\nShip the assistant on-device.",
    );
    assert_eq!(open_count(), 1, "round 2 opens a fresh proposal after the first was declined");
    assert_eq!(
        author_of(&v2),
        "lfm2.5-1.2b",
        "round 2 was the phone model — the cycle spans devices"
    );
    let d2 = commands::proposal_diff(&store, p, &v2).unwrap();
    assert!(d2.patch.contains("Action items"), "v2 now carries the action items: {}", d2.patch);
    decline(&v2); // summary still isn't first, and the name is wrong.

    // --- Round 3 (back on the laptop, qwen): summary first, name fixed. This one is right. ---
    let v3 = round(
        &mut store,
        "Almost — put the summary first, and it's \"Arne\", one n.",
        laptop,
        "Fixed: summary first, name corrected. /propose",
        "# Talk with Arne\n\n## Summary\n\nShip the assistant on-device.\n\n## Action items\n\n- Bundle the runtime",
    );
    let d3 = commands::proposal_diff(&store, p, &v3).unwrap();
    assert!(d3.exists, "the final proposal is open and reviewable");

    // The user is satisfied → accept = merge the third branch into main.
    commands::reply(&mut store, &host, "Perfect — accepting this one.").unwrap();
    let merge = git_run(p, &["merge", "--no-ff", "--no-edit", &format!("proposal/{v3}")]);
    assert!(
        merge.status.success(),
        "the accepted round must merge cleanly: {}",
        String::from_utf8_lossy(&merge.stderr)
    );
    git_run(p, &["branch", "-D", &format!("proposal/{v3}")]); // completed → close it out

    // The accepted version — and *only* it — is now live on main: summary first, name fixed.
    let live = std::fs::read_to_string(p.join(format!("notes/{host}.md"))).unwrap();
    let summary_at = live.find("## Summary").expect("summary present");
    let actions_at = live.find("## Action items").expect("action items present");
    assert!(summary_at < actions_at, "the accepted version leads with the summary: {live}");
    assert!(
        live.contains("Arne\n") && !live.contains("Arnne"),
        "the accepted version fixed the name: {live}"
    );

    // No branch was left open, but every attempt is still recorded — three proposal notes as an audit
    // trail of the back-and-forth, even though only one was ever merged.
    assert_eq!(open_count(), 0, "no open proposals remain after the cycle");
    let reread = FileStore::open(p).unwrap();
    assert_eq!(
        commands::proposals(&reread).unwrap().len(),
        3,
        "all three attempts stay recorded as proposals"
    );

    // The whole conversation lived in the note's discussion and survived the merge: three user asks,
    // three agent answers, and the final accept — seven messages.
    assert_eq!(
        commands::thread(&reread, &host).unwrap().count,
        7,
        "the full back-and-forth is preserved in the note"
    );
}
