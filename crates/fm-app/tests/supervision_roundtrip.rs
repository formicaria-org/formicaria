//! **Can a dataset actually be rebuilt from what the app writes?**
//!
//! Every other test here proves one field. This proves the thing a downstream tool depends on: that
//! a whole review event, written through the real command door, comes back out of git intact — and
//! comes back out using **git alone**, because the tool that consumes this is a separate program
//! that will not link `fm-app`.
//!
//! So the writes go through [`fm_app::dispatch`], exactly as `fm-serve` frames them, and every
//! assertion reads with the `git` binary, never through the app. Asking an implementation to grade
//! itself proves nothing; the question is whether an outsider can see what we claim is there.

use fm_app::{dispatch, vaults::VaultConfig, App, Host, Output};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn have_git() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Read the repository with **real git**, never through the app.
fn g(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn call(app: &App, cmd: &str, args: Value) -> Value {
    let out: Output =
        dispatch(cmd, &args, &[], app, &NoHost).unwrap_or_else(|e| panic!("{cmd} failed: {e}"));
    match out {
        Output::Json(b) if b.is_empty() => Value::Null,
        o => serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null),
    }
}

/// One git-backed vault, and an `App` over it.
fn vault() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    let owned: Vec<(String, PathBuf)> = vec![("personal".into(), dir.path().to_path_buf())];
    let configs = vec![VaultConfig {
        name: "personal".into(),
        path: dir.path().to_path_buf(),
        restic: None,
    }];
    let app = App::new(
        MultiStore::open(&owned).unwrap(),
        configs,
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, dir, app)
}

/// A trailer, read back through git's own parser rather than by matching text.
fn trailer(repo: &Path, rev: &str, key: &str) -> String {
    g(
        repo,
        &[
            "log",
            "-1",
            &format!("--format=%(trailers:key={key},valueonly)"),
            rev,
        ],
    )
}

#[test]
fn a_whole_review_event_survives_the_round_trip_through_git() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    let p = dir.path();

    // A note, committed so there is a base to propose against.
    let note = call(
        &app,
        "capture",
        json!({ "body": "# Battery\n\nrough notes\n" }),
    );
    let note_id = note["id"].as_str().unwrap().to_string();
    call(&app, "commit", json!({ "message": "auto: seed" }));

    // 1. THE AGENT PROPOSES — carrying the input half: which tool, what was asked, what it read.
    let prop = call(
        &app,
        "create_proposal",
        json!({
            "id": note_id,
            "body": "# Battery\n\nLithium-ion, per the sources.\n",
            "authorName": "qwen3-vl-4b",
            "authorEmail": "qwen3-vl-4b@fm-agents.local",
            "tool": "research",
            "query": "what battery chemistry do electric cars use",
            "sources": ["https://en.wikipedia.org/wiki/Electric_vehicle"],
        }),
    );
    let prop_id = prop["id"].as_str().unwrap().to_string();
    let branch = format!("proposal/{prop_id}");
    let model_text = g(p, &["show", &format!("{branch}:notes/{note_id}.md")]);
    assert!(model_text.contains("Lithium-ion, per the sources."));

    // 2. IT IS SHOWN to a person. Without this, "left alone" and "never opened" are one absence.
    call(&app, "proposal_shown", json!({ "id": prop_id }));

    // 3. THE HUMAN CORRECTS IT, and says why and what kind of correction it is.
    call(
        &app,
        "create_proposal",
        json!({
            "id": note_id,
            "body": "# Battery\n\nLithium-iron-phosphate, per the sources.\n",
            "why": "the model picked the wrong chemistry @wrong",
            "kind": "factual",
        }),
    );

    // 4. AND ACCEPTS.
    let outcome = call(&app, "accept_proposal", json!({ "id": prop_id }));
    assert_eq!(outcome["outcome"], "merged", "accept should merge cleanly");

    // ---------------------------------------------------------------------------------
    // Now read it back as an OUTSIDE TOOL must: with git, knowing only the conventions.
    // ---------------------------------------------------------------------------------

    // The human's final text is on the branch HEAD points at (whatever git named it).
    let final_text = g(p, &["show", &format!("HEAD:notes/{note_id}.md")]);
    assert!(
        final_text.contains("Lithium-iron-phosphate"),
        "the correction is what landed"
    );

    // The MODEL'S ORIGINAL is still reachable — the pair is what makes this a training example, and
    // without retention a revise would have force-orphaned it.
    let kept: Vec<String> = g(
        p,
        &["for-each-ref", "--format=%(objectname)", "refs/fm/review/"],
    )
    .lines()
    .map(str::to_string)
    .collect();
    assert!(!kept.is_empty(), "the superseded proposal must be retained");
    let recovered = kept
        .iter()
        .map(|oid| g(p, &["show", &format!("{oid}:notes/{note_id}.md")]))
        .find(|t| t.contains("Lithium-ion, per the sources."));
    assert!(
        recovered.is_some(),
        "the model's pre-correction text must be recoverable"
    );

    // The reason, the labels and the provenance all parse as real trailers.
    let head = g(p, &["rev-parse", "HEAD"]);
    let accepted = g(p, &["rev-parse", &format!("{head}^2")]); // the accept merge's second parent
    assert_eq!(trailer(p, &accepted, "SchemaRev"), "1");
    assert_eq!(trailer(p, &accepted, "Kind"), "factual");
    assert_eq!(trailer(p, &accepted, "Consent"), "local");
    let body = g(p, &["log", "-1", "--format=%b", &accepted]);
    assert!(
        body.contains("the model picked the wrong chemistry @wrong"),
        "the reviewer's reason must survive, got:\n{body}"
    );

    // The INPUT half is on the agent's own proposal, which is the retained one.
    let with_input = kept
        .iter()
        .find(|oid| trailer(p, oid, "Tool") == "research")
        .expect("the agent's proposal must carry its input");
    assert_eq!(
        trailer(p, with_input, "Query"),
        "what battery chemistry do electric cars use"
    );
    assert!(trailer(p, with_input, "Sources").contains("Electric_vehicle"));
    assert_eq!(
        trailer(p, with_input, "Assisted-by"),
        "formicaria-agent:qwen3-vl-4b",
        "and who produced it",
    );

    // `shown` lives on the immortal proposal note, because it is a fact that arrives after the
    // commit is written.
    let prop_note = g(p, &["show", &format!("HEAD:notes/{prop_id}.md")]);
    assert!(
        prop_note.contains("shown:"),
        "the sighting must be recorded, got:\n{prop_note}"
    );
}

#[test]
fn a_rejected_proposal_keeps_both_its_text_and_the_reason_it_was_refused() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    let p = dir.path();
    let note = call(&app, "capture", json!({ "body": "# Cells\n\nnotes\n" }));
    let note_id = note["id"].as_str().unwrap().to_string();
    call(&app, "commit", json!({ "message": "auto: seed" }));

    let prop = call(
        &app,
        "create_proposal",
        json!({
            "id": note_id,
            "body": "# Cells\n\nInvented Reference (2019) says otherwise.\n",
            "authorName": "qwen3-vl-4b",
            "authorEmail": "qwen3-vl-4b@fm-agents.local",
            "tool": "research",
        }),
    );
    let prop_id = prop["id"].as_str().unwrap().to_string();
    call(&app, "proposal_shown", json!({ "id": prop_id }));
    call(
        &app,
        "reject_proposal",
        json!({ "id": prop_id, "why": "invented a source that does not exist @hallucinated" }),
    );

    // `main` is untouched — that is what rejecting means.
    let head_text = g(p, &["show", &format!("HEAD:notes/{note_id}.md")]);
    assert!(
        !head_text.contains("Invented Reference"),
        "a reject must not touch the note"
    );

    // But the rejected TEXT is still recoverable. This is the half that used to be deleted outright,
    // and it is the only negative signal the corpus can ever have.
    let kept: Vec<String> = g(
        p,
        &["for-each-ref", "--format=%(objectname)", "refs/fm/review/"],
    )
    .lines()
    .map(str::to_string)
    .collect();
    let recovered = kept
        .iter()
        .map(|oid| g(p, &["show", &format!("{oid}:notes/{note_id}.md")]))
        .find(|t| t.contains("Invented Reference"));
    assert!(
        recovered.is_some(),
        "the rejected text must survive; refs were {kept:?}"
    );

    // ...and so is *why*. A reject writes no commit and its text never reaches the branch, so the
    // proposal note is the only place that reason can live.
    let prop_note = g(p, &["show", &format!("HEAD:notes/{prop_id}.md")]);
    assert!(prop_note.contains("declined: true"), "got:\n{prop_note}");
    assert!(
        prop_note.contains("invented a source that does not exist @hallucinated"),
        "the refusal's reason must survive, got:\n{prop_note}"
    );
    assert!(
        prop_note.contains("shown:"),
        "and that a person actually looked"
    );
}

#[test]
fn garbage_collection_does_not_take_the_record() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    let p = dir.path();
    let note = call(&app, "capture", json!({ "body": "# GC\n\nnotes\n" }));
    let note_id = note["id"].as_str().unwrap().to_string();
    call(&app, "commit", json!({ "message": "auto: seed" }));

    let prop = call(
        &app,
        "create_proposal",
        json!({ "id": note_id, "body": "# GC\n\nfirst draft\n", "tool": "propose" }),
    );
    let prop_id = prop["id"].as_str().unwrap().to_string();
    // A revision, so the first draft is superseded and only the retention ref holds it.
    call(
        &app,
        "create_proposal",
        json!({ "id": note_id, "body": "# GC\n\nsecond draft\n" }),
    );
    call(
        &app,
        "reject_proposal",
        json!({ "id": prop_id, "why": "changed my mind" }),
    );

    let kept: Vec<String> = g(
        p,
        &["for-each-ref", "--format=%(objectname)", "refs/fm/review/"],
    )
    .lines()
    .map(str::to_string)
    .collect();
    assert!(
        kept.len() >= 2,
        "both the draft and the revision should be retained, got {kept:?}"
    );

    // The whole durability claim in one command: unreachable objects are pruned NOW, and a retained
    // record is reachable, so it stays. This is what Step 0 had to rescue by hand precisely because
    // nothing referenced those commits.
    let gc = Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["gc", "--prune=now"])
        .output()
        .unwrap();
    assert!(
        gc.status.success(),
        "gc failed: {}",
        String::from_utf8_lossy(&gc.stderr)
    );

    for oid in &kept {
        let t = g(p, &["cat-file", "-t", oid]);
        assert_eq!(t, "commit", "{oid} did not survive gc --prune=now");
    }
    assert!(
        g(p, &["fsck", "--unreachable", "--no-progress"]).is_empty(),
        "nothing left dangling"
    );
}

#[test]
fn pushing_does_not_collapse_the_record_away() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    let p = dir.path();

    // A real remote, because `push_squashed` is the one operation that REWRITES local history:
    // `reset --soft` back to the squash floor, then one commit. Anything above that floor is gone
    // from history — so this asks whether a routine backup silently deletes the corpus.
    let remote = tempdir().unwrap();
    assert!(Command::new("git")
        .args(["init", "--bare", "-q"])
        .arg(remote.path())
        .status()
        .unwrap()
        .success());
    call(
        &app,
        "set_git_remote",
        json!({ "vault": "personal", "url": remote.path().to_string_lossy() }),
    );

    let note = call(&app, "capture", json!({ "body": "# Push\n\nnotes\n" }));
    let note_id = note["id"].as_str().unwrap().to_string();
    call(&app, "commit", json!({ "message": "auto: seed" }));

    let prop = call(
        &app,
        "create_proposal",
        json!({
            "id": note_id,
            "body": "# Push\n\nthe model's draft\n",
            "authorName": "qwen3-vl-4b",
            "authorEmail": "qwen3-vl-4b@fm-agents.local",
            "tool": "propose",
        }),
    );
    let prop_id = prop["id"].as_str().unwrap().to_string();
    call(&app, "proposal_shown", json!({ "id": prop_id }));
    call(
        &app,
        "create_proposal",
        json!({ "id": note_id, "body": "# Push\n\nmy correction\n", "why": "wrong tense", "kind": "style" }),
    );
    call(&app, "accept_proposal", json!({ "id": prop_id }));

    let retained: Vec<String> = g(
        p,
        &["for-each-ref", "--format=%(objectname)", "refs/fm/review/"],
    )
    .lines()
    .map(str::to_string)
    .collect();
    assert!(!retained.is_empty());

    // Back up, twice — the second push is the one with a tracking ref, and therefore the one that
    // actually squashes.
    call(
        &app,
        "push",
        json!({ "vault": "personal", "message": "backup: one" }),
    );
    call(&app, "capture", json!({ "body": "# Later\n\nmore\n" }));
    call(&app, "commit", json!({ "message": "auto: later" }));
    call(
        &app,
        "push",
        json!({ "vault": "personal", "message": "backup: two" }),
    );

    // The accept merge is not an `auto:`/`backup:` commit, so it is a squash barrier and survives —
    // and with it the model's proposal, as the merge's second parent.
    let accepts = g(p, &["log", "--oneline", "--grep=^accept: merge"]);
    assert!(
        !accepts.is_empty(),
        "the acceptance must survive a push, history:\n{}",
        g(p, &["log", "--oneline"])
    );

    // The retained commits are held by refs of our own, which `push_squashed` never touches.
    for oid in &retained {
        assert_eq!(
            g(p, &["cat-file", "-t", oid]),
            "commit",
            "{oid} was lost to a push"
        );
    }
    let recovered = retained
        .iter()
        .map(|oid| g(p, &["show", &format!("{oid}:notes/{note_id}.md")]))
        .find(|t| t.contains("the model's draft"));
    assert!(
        recovered.is_some(),
        "the model's original must survive a backup"
    );

    // And the correction is still what the note says.
    assert!(g(p, &["show", &format!("HEAD:notes/{note_id}.md")]).contains("my correction"));
}

/// **The contract for the downstream tool, executable.**
///
/// The exporter is a separate program that will not link this crate; it knows only git and the
/// conventions. This assembles one complete dataset row the way that tool must, and asserts every
/// field it needs is actually derivable — so "we can build the tool" is a checked claim rather than
/// a hope. If this test has to change, the tool has to change with it, and `SchemaRev` is what says
/// so out loud.
#[test]
fn a_complete_dataset_row_is_derivable_from_git_alone() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (_home, dir, app) = vault();
    let p = dir.path();
    std::fs::write(
        p.join("vault.json"),
        r#"{"supervision":{"collect":true,"publish":true}}"#,
    )
    .unwrap();

    let note = call(&app, "capture", json!({ "body": "# Row\n\nbefore\n" }));
    let note_id = note["id"].as_str().unwrap().to_string();
    call(&app, "commit", json!({ "message": "auto: seed" }));

    let prop = call(
        &app,
        "create_proposal",
        json!({
            "id": note_id, "body": "# Row\n\nthe model said this\n",
            "authorName": "qwen3-vl-4b", "authorEmail": "qwen3-vl-4b@fm-agents.local",
            "tool": "research", "query": "what goes here",
            "sources": ["https://example.org/one"],
        }),
    );
    let prop_id = prop["id"].as_str().unwrap().to_string();
    call(&app, "proposal_shown", json!({ "id": prop_id }));
    call(
        &app,
        "create_proposal",
        json!({ "id": note_id, "body": "# Row\n\nwhat I actually meant\n",
                "why": "too vague", "kind": "style" }),
    );
    call(&app, "accept_proposal", json!({ "id": prop_id }));

    // ---- extraction, using only git ------------------------------------------------------
    let head = g(p, &["rev-parse", "HEAD"]);
    let final_commit = g(p, &["rev-parse", &format!("{head}^2")]); // accept merge's 2nd parent
    let base = g(p, &["merge-base", &format!("{head}^1"), &final_commit]);
    let path = format!("notes/{note_id}.md");

    // The three texts a training example is made of.
    let context_before = g(p, &["show", &format!("{base}:{path}")]);
    let final_text = g(p, &["show", &format!("{final_commit}:{path}")]);
    let original = g(
        p,
        &["for-each-ref", "--format=%(objectname)", "refs/fm/review/"],
    )
    .lines()
    .map(|oid| g(p, &["show", &format!("{oid}:{path}")]))
    .find(|t| t.contains("the model said this"))
    .expect("the model's original must be recoverable");

    // The labels and provenance.
    let t = |k: &str| trailer(p, &final_commit, k);
    let why = g(p, &["log", "-1", "--format=%b", &final_commit]);
    let prop_note = g(p, &["show", &format!("HEAD:notes/{prop_id}.md")]);

    // ---- every field a row needs ----------------------------------------------------------
    assert!(
        context_before.contains("before"),
        "the input the model was working from"
    );
    assert!(
        original.contains("the model said this"),
        "what the model produced"
    );
    assert!(
        final_text.contains("what I actually meant"),
        "what the human kept"
    );
    assert!(
        why.contains("too vague"),
        "why they changed it — the field measured most decisive"
    );
    assert_eq!(t("Kind"), "style", "which axis the corpus can be split on");
    assert_eq!(
        t("SchemaRev"),
        "1",
        "which schema this row was written under"
    );
    assert_eq!(t("Consent"), "local,publish", "what may be done with it");
    assert!(
        prop_note.contains("shown:"),
        "that a person actually looked at it"
    );
    // The input half rides on the agent's own proposal, not on the human's revision of it.
    let agent_side = g(
        p,
        &["for-each-ref", "--format=%(objectname)", "refs/fm/review/"],
    )
    .lines()
    .find(|oid| trailer(p, oid, "Tool") == "research")
    .map(str::to_string)
    .expect("the agent's proposal carries the input");
    assert_eq!(trailer(p, &agent_side, "Query"), "what goes here");
    assert!(trailer(p, &agent_side, "Sources").contains("example.org/one"));
    assert_eq!(
        trailer(p, &agent_side, "Assisted-by"),
        "formicaria-agent:qwen3-vl-4b"
    );

    // The outcome is legible without any app: an `accept: merge` commit naming this proposal.
    assert!(
        g(
            p,
            &[
                "log",
                "--oneline",
                "--grep",
                &format!("accept: merge proposal/{prop_id}")
            ]
        )
        .contains("accept: merge"),
        "the disposition must be readable from history alone"
    );
}
