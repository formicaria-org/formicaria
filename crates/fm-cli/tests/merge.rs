//! The `.md` merge driver, against real git. Two clones, real commits, real merges —
//! because the thing being tested is a contract with git, not a function.

use fm_core::git;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn g(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git").arg("-C").arg(repo).args(args).output().unwrap()
}

/// A note as the app writes it, with the `updated:` line that makes every concurrent
/// edit collide.
fn note(updated: &str, tags: &str, body: &str) -> String {
    format!(
        "---\nid: 01JQ0000000000000000000000\ntype: note\ntitle: shared note\ncreated: 2026-07-17T10:00:00Z\nupdated: {updated}\ntags:\n{tags}---\n\n{body}"
    )
}

/// Point this repo's `merge=fm` at the `fm` we just built. In the product
/// `ensure_repo` does this with the binary beside `fm-serve`; here the test binary
/// lives in `target/debug/deps`, so `current_exe`'s sibling is not it.
fn install_driver(repo: &Path) {
    let fm = Path::new(env!("CARGO_BIN_EXE_fm"));
    g(repo, &["config", "merge.fm.driver", &format!("'{}' merge-md %O %A %B %L", fm.display())]);
}

/// Set up one repo holding a note, and a clone of it, both ready to merge through us.
fn two_clones(initial: &str) -> (tempfile::TempDir, tempfile::TempDir) {
    let ours = tempdir().unwrap();
    git::ensure_repo(ours.path()).unwrap();
    // This machine's `init.defaultBranch` is not ours to assume, and an unborn HEAD
    // is the only moment this is free.
    g(ours.path(), &["symbolic-ref", "HEAD", "refs/heads/main"]);
    fs::create_dir_all(ours.path().join("notes")).unwrap();
    fs::write(ours.path().join("notes/01JQ0000000000000000000000.md"), initial).unwrap();
    // `.gitattributes` is written by ensure_repo and must be committed to travel.
    g(ours.path(), &["add", "-A"]);
    g(ours.path(), &["commit", "-m", "base"]);

    let theirs = tempdir().unwrap();
    fs::remove_dir_all(theirs.path()).unwrap();
    Command::new("git").arg("clone").arg(ours.path()).arg(theirs.path()).output().unwrap();
    for (k, v) in [("user.email", "them@example.org"), ("user.name", "Them")] {
        g(theirs.path(), &["config", k, v]);
    }
    install_driver(ours.path());
    install_driver(theirs.path());
    (ours, theirs)
}

/// **The test that says shared vaults work.** Two people edit different lines of one
/// note. Because the app rewrites `updated:` on every save, git's own text merge
/// conflicts on that line every single time — inside the YAML, where it also breaks
/// the parser. The driver has to make this a non-event.
#[test]
fn two_people_editing_different_lines_of_one_note_merge_cleanly() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let (ours, theirs) = two_clones(&note(
        "2026-07-17T10:00:00Z",
        "- shared\n",
        "First paragraph.\n\nSecond paragraph.\n",
    ));
    let rel = "notes/01JQ0000000000000000000000.md";

    // They rewrite the second paragraph and save — so `updated` moves, and they add a tag.
    fs::write(
        theirs.path().join(rel),
        note(
            "2026-07-17T12:00:00Z",
            "- shared\n- theirs\n",
            "First paragraph.\n\nSecond paragraph, improved by them.\n",
        ),
    )
    .unwrap();
    g(theirs.path(), &["commit", "-am", "theirs"]);

    // We rewrite the *first* paragraph. Different lines, same note, same `updated:`.
    fs::write(
        ours.path().join(rel),
        note(
            "2026-07-17T11:00:00Z",
            "- shared\n- ours\n",
            "First paragraph, improved by us.\n\nSecond paragraph.\n",
        ),
    )
    .unwrap();
    g(ours.path(), &["commit", "-am", "ours"]);

    // Pull theirs into ours.
    g(ours.path(), &["remote", "add", "them", theirs.path().to_str().unwrap()]);
    g(ours.path(), &["fetch", "them"]);
    let merge = g(ours.path(), &["merge", "them/main", "-m", "merge"]);

    let merged = fs::read_to_string(ours.path().join(rel)).unwrap();
    assert!(
        merge.status.success(),
        "clean merge, no conflict on the line we rewrite on every save:\n{}\n--- file:\n{merged}",
        String::from_utf8_lossy(&merge.stderr),
    );
    assert!(!merged.contains("<<<<<<<"), "no markers anywhere:\n{merged}");

    // Both people's prose is there.
    assert!(merged.contains("First paragraph, improved by us."), "our edit:\n{merged}");
    assert!(merged.contains("Second paragraph, improved by them."), "their edit:\n{merged}");
    // `updated` is the later of the two readings — a clock, not an opinion.
    assert!(merged.contains("updated: 2026-07-17T12:00:00Z"), "the later updated wins:\n{merged}");
    // Tags unite: neither person's tag is silently dropped.
    assert!(merged.contains("ours") && merged.contains("theirs"), "tags united:\n{merged}");
    // And identity never moved.
    assert!(merged.contains("created: 2026-07-17T10:00:00Z"), "created is the base's:\n{merged}");
}

/// A real disagreement must still conflict — resolving it by fiat is the silent data
/// loss this whole phase exists to stop. But it must conflict **in the body**, so the
/// note still parses, still indexes, and still opens in the editor with the markers
/// where a human can see them. That property is what the conflict-surfacing UI needs.
#[test]
fn a_real_conflict_lands_in_the_body_leaving_the_note_readable() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let (ours, theirs) =
        two_clones(&note("2026-07-17T10:00:00Z", "- shared\n", "The disputed line.\n"));
    let rel = "notes/01JQ0000000000000000000000.md";

    fs::write(
        theirs.path().join(rel),
        note("2026-07-17T12:00:00Z", "- shared\n", "Their version of the line.\n"),
    )
    .unwrap();
    g(theirs.path(), &["commit", "-am", "theirs"]);

    fs::write(
        ours.path().join(rel),
        note("2026-07-17T11:00:00Z", "- shared\n", "Our version of the line.\n"),
    )
    .unwrap();
    g(ours.path(), &["commit", "-am", "ours"]);

    g(ours.path(), &["remote", "add", "them", theirs.path().to_str().unwrap()]);
    g(ours.path(), &["fetch", "them"]);
    let merge = g(ours.path(), &["merge", "them/main", "-m", "merge"]);
    assert!(!merge.status.success(), "the same line really did conflict");

    let merged = fs::read_to_string(ours.path().join(rel)).unwrap();
    assert!(merged.contains("<<<<<<<"), "the conflict is shown, not resolved by fiat:\n{merged}");

    // The point: the frontmatter is intact, so the note still loads. Before the driver
    // the markers landed in the YAML and the note dropped out of the vault entirely.
    let obj = fm_core::frontmatter::from_file(&merged)
        .expect("a conflicted note still parses — the markers are in the body");
    assert!(obj.body.contains("<<<<<<<"), "and the human sees them in the editor");
    assert!(obj.body.contains("Our version of the line."), "both sides are offered");
    assert!(obj.body.contains("Their version of the line."), "both sides are offered");
}

/// A note with a `status`, which is what the Board, Agenda and Calendar actually query.
fn task(updated: &str, status: &str, body: &str) -> String {
    format!(
        "---\nid: 01JQ0000000000000000000000\ntype: task\ntitle: shared task\ncreated: 2026-07-17T10:00:00Z\nupdated: {updated}\nstatus: {status}\n---\n\n{body}"
    )
}

/// **Characterization test — this locks behaviour we chose to keep, not behaviour we like.**
///
/// Two people drag the same card to different columns. `status` genuinely diverges, so
/// `merge_objects` returns `None` (`merge.rs`) and the whole file — YAML fence included —
/// goes through the text merge. The markers therefore land *inside* the frontmatter, and
/// `from_file` rightly refuses it: the note drops out of every view until a human fixes it.
///
/// That is ugly, and it is **deliberate**. The alternative — letting ours win the field —
/// is forbidden in writing (`merge.rs`: *"silently dropping one side's status change is the
/// same data loss this whole phase exists to stop"*), and it would be strictly worse here:
/// the bodies are identical in the card-drag case, so the merge would come back **Clean**,
/// auto-commit would fire, and the sync loop would push one person's column over the
/// other's with nothing shown to anyone. Loud-and-absent beats quiet-and-wrong.
///
/// So the ruling is: keep this, and build the surface that lists skipped notes
/// (`decisions.md`, 2026-07-19). **If this test fails, someone has reversed that ruling** —
/// go read the decision before "fixing" the test.
#[test]
fn a_divergent_status_field_breaks_the_fence_and_keeps_both_values() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let (ours, theirs) =
        two_clones(&task("2026-07-17T10:00:00Z", "todo", "Ship the thing.\n"));
    let rel = "notes/01JQ0000000000000000000000.md";

    // Identical bodies on purpose: the card-drag case moves a card and touches nothing else.
    fs::write(theirs.path().join(rel), task("2026-07-17T12:00:00Z", "done", "Ship the thing.\n"))
        .unwrap();
    g(theirs.path(), &["commit", "-am", "theirs: dragged to done"]);

    fs::write(ours.path().join(rel), task("2026-07-17T11:00:00Z", "doing", "Ship the thing.\n"))
        .unwrap();
    g(ours.path(), &["commit", "-am", "ours: dragged to doing"]);

    g(ours.path(), &["remote", "add", "them", theirs.path().to_str().unwrap()]);
    g(ours.path(), &["fetch", "them"]);
    let merge = g(ours.path(), &["merge", "them/main", "-m", "merge"]);
    assert!(!merge.status.success(), "a divergent status really does conflict");

    let merged = fs::read_to_string(ours.path().join(rel)).unwrap();

    // The property that must never regress: nothing was resolved by fiat.
    assert!(merged.contains("doing"), "our value survives:\n{merged}");
    assert!(merged.contains("done"), "their value survives too:\n{merged}");
    assert!(merged.contains("<<<<<<<"), "and the disagreement is shown:\n{merged}");

    // The documented cost of that guarantee: unlike a body conflict, this one is *not*
    // readable, because the markers are inside the YAML. The note is absent from every
    // view until a human resolves it — and is surfaced by name as skipped, not lost.
    assert!(
        fm_core::frontmatter::from_file(&merged).is_err(),
        "a fence-broken note does not parse — this is the known cost, see the doc comment:\n{merged}"
    );
}

/// `pull` is the whole point of the guards that came before it: it is the thing that
/// makes the tracking ref move, which is exactly what `push_squashed`'s ancestry guard
/// exists to survive. Here it has to actually bring a collaborator's note home.
#[test]
fn pull_brings_their_work_home_and_says_so() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare", "-b", "main"]).arg(bare.path()).output().unwrap();

    let (ours, theirs) = two_clones(&note("2026-07-17T10:00:00Z", "- shared\n", "Ours.\n"));
    let url = bare.path().to_str().unwrap();
    git::set_identity(ours.path(), "Us", "us@example.org").unwrap();
    git::set_remote(ours.path(), url).unwrap();
    git::push_squashed(ours.path(), "first").unwrap();

    // Nothing of theirs yet.
    assert_eq!(git::pull(ours.path()).unwrap(), git::Pulled::UpToDate, "nothing to pull");

    // A collaborator pushes a note we have never seen.
    g(theirs.path(), &["remote", "add", "bare", url]);
    g(theirs.path(), &["fetch", "-q", "bare"]);
    g(theirs.path(), &["reset", "--hard", "bare/main"]);
    fs::write(theirs.path().join("notes/theirs.md"), note("2026-07-17T12:00:00Z", "- shared\n", "A note only they have.\n")).unwrap();
    g(theirs.path(), &["add", "-A"]);
    g(theirs.path(), &["commit", "-m", "theirs"]);
    g(theirs.path(), &["push", "-q", "bare", "HEAD:main"]);

    // The cheap question, asked before spending a fetch: has anything moved?
    assert_eq!(git::remote_moved(ours.path()).unwrap(), Some(true), "ls-remote sees their push");

    match git::pull(ours.path()).unwrap() {
        git::Pulled::Merged(n) => assert_eq!(n, 1, "one commit of theirs arrived"),
        other => panic!("expected a clean merge, got {other:?}"),
    }
    assert!(ours.path().join("notes/theirs.md").exists(), "their note is on our disk now");
    assert_eq!(git::remote_moved(ours.path()).unwrap(), Some(false), "and we are level again");
    assert!(git::conflicts(ours.path()).unwrap().is_empty(), "nothing left for a human");
}

/// The deployment trap: `.gitattributes` is tracked and travels, but the driver
/// *definition* lives in `.git/config` and deliberately does not. A collaborator who
/// clones must get it installed by `ensure_repo`, or they silently fall back to git's
/// text merge and hit the `updated:` conflict with no sign anything was meant to stop it.
#[test]
fn a_fresh_clone_gets_both_halves_of_the_driver() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let origin = tempdir().unwrap();
    git::ensure_repo(origin.path()).unwrap();
    fs::create_dir_all(origin.path().join("notes")).unwrap();
    fs::write(origin.path().join("notes/a.md"), "---\nid: 01JQ0000000000000000000000\ntype: note\ncreated: 2026-07-17T10:00:00Z\nupdated: 2026-07-17T10:00:00Z\n---\n\nhi\n").unwrap();
    assert!(
        origin.path().join(".gitattributes").exists(),
        "the tracked half is written so it can travel",
    );
    g(origin.path(), &["add", "-A"]);
    g(origin.path(), &["commit", "-m", "base"]);

    let clone = tempdir().unwrap();
    fs::remove_dir_all(clone.path()).unwrap();
    Command::new("git").arg("clone").arg(origin.path()).arg(clone.path()).output().unwrap();

    // What a collaborator has the moment they clone: the attribute, but no driver.
    let attrs = fs::read_to_string(clone.path().join(".gitattributes")).unwrap();
    assert!(attrs.contains("merge=fm"), "the attribute travelled: {attrs}");
    let before = g(clone.path(), &["config", "merge.fm.driver"]);
    assert!(!before.status.success(), "but git refused to let the *command* travel");

    // Opening the vault is what closes the gap — but only ever with a command that is
    // really there. A driver git cannot execute is worse than none: git reads the
    // failure as a conflict and hands back `%A` untouched, which is *our* version with
    // no markers in it, so the user resolves a normal-looking file and silently drops
    // their collaborator's edit. No driver at all degrades to git's built-in text
    // merge, which conflicts honestly.
    git::ensure_repo(clone.path()).unwrap();
    let after = g(clone.path(), &["config", "merge.fm.driver"]);
    let driver = String::from_utf8_lossy(&after.stdout).into_owned();
    if after.status.success() {
        assert!(driver.contains("merge-md %O %A %B %L"), "with git's placeholders: {driver}");
        let named = driver.split('\'').nth(1).unwrap_or_default();
        assert!(Path::new(named).exists(), "we only ever name a binary that exists: {named}");
    } else {
        // Under `cargo test` the running binary is the harness in `deps/`, with no `fm`
        // beside it — so nothing was installed, which is the safe half of the rule.
        assert!(attrs.contains("merge=fm"), "and the attribute still travels: {attrs}");
    }
}

/// A **whiteboard** through the same driver, against real git.
///
/// A board note's body is an Excalidraw scene: one pretty-printed JSON array rewritten
/// whole on every change. Git's text merge is close to the worst tool for it — two people
/// drawing in opposite corners touch no common shape but do share the punctuation between
/// them — so before `scene.rs` this produced either a spurious conflict or spliced JSON
/// that Excalidraw cannot open. The whole board, lost, because two people drew at once.
///
/// This is the test that says the element merge actually fires *through git*, not just in
/// its own unit tests: the driver is what git invokes, and a driver that never runs is a
/// merge strategy nobody has.
#[test]
fn two_people_drawing_on_one_whiteboard_merge_element_wise() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    // A board is a note whose body is a scene — no new file type, no second driver.
    let board = |updated: &str, elements: &str| {
        format!(
            "---\nid: 01JQ0000000000000000000000\ntype: note\ntitle: shared board\nview: board\n\
             created: 2026-07-17T10:00:00Z\nupdated: {updated}\ntags:\n---\n\n\
             {{\"type\":\"excalidraw\",\"version\":2,\"source\":\"fm\",\"elements\":[{elements}],\
             \"appState\":{{}},\"files\":{{}}}}\n"
        )
    };
    let shape = |id: &str, version: i64, nonce: i64, x: i64| {
        format!(
            "{{\"id\":\"{id}\",\"version\":{version},\"versionNonce\":{nonce},\
             \"type\":\"rectangle\",\"x\":{x}}}"
        )
    };

    let base_shapes = format!("{},{}", shape("keep", 1, 1, 0), shape("doomed", 1, 2, 0));
    let (ours, theirs) = two_clones(&board("2026-07-17T10:00:00Z", &base_shapes));
    let rel = "notes/01JQ0000000000000000000000.md";

    // They draw a new shape, and *move* the one we are about to delete (so their copy of it
    // is strictly newer — the case a base-less reconcile resurrects).
    fs::write(
        theirs.path().join(rel),
        board(
            "2026-07-17T12:00:00Z",
            &format!(
                "{},{},{}",
                shape("keep", 1, 1, 0),
                shape("doomed", 99, 2, 500),
                shape("theirs", 1, 7, 20)
            ),
        ),
    )
    .unwrap();
    g(theirs.path(), &["commit", "-am", "theirs"]);

    // We delete `doomed` and draw our own shape.
    fs::write(
        ours.path().join(rel),
        board(
            "2026-07-17T11:00:00Z",
            &format!("{},{}", shape("keep", 1, 1, 0), shape("ours", 1, 5, 10)),
        ),
    )
    .unwrap();
    g(ours.path(), &["commit", "-am", "ours"]);

    g(ours.path(), &["remote", "add", "them", theirs.path().to_str().unwrap()]);
    g(ours.path(), &["fetch", "them"]);
    let merge = g(ours.path(), &["merge", "them/main", "-m", "merge"]);

    let merged = fs::read_to_string(ours.path().join(rel)).unwrap();
    assert!(
        merge.status.success(),
        "two people drawing at once must not conflict:\n{}\n--- file:\n{merged}",
        String::from_utf8_lossy(&merge.stderr),
    );
    assert!(!merged.contains("<<<<<<<"), "no markers in a scene:\n{merged}");

    // The body must still be a scene the app can open — the failure this test exists for
    // is JSON spliced into something unparseable.
    let body = merged.split("---\n").nth(2).expect("a body after the frontmatter");
    let scene: serde_json::Value =
        serde_json::from_str(body.trim()).expect("the merged body is still valid scene JSON");
    let ids: Vec<&str> = scene["elements"]
        .as_array()
        .expect("elements survived as an array")
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();

    // Both people's new work is on the canvas.
    assert!(ids.contains(&"ours"), "our shape survived: {ids:?}");
    assert!(ids.contains(&"theirs"), "their shape survived: {ids:?}");
    assert!(ids.contains(&"keep"), "the untouched shape survived: {ids:?}");
    // And the one we deleted stays deleted, even though their copy was newer. This is the
    // whole difference from Excalidraw's base-less `reconcileElements`.
    assert!(!ids.contains(&"doomed"), "a deleted shape must not come back: {ids:?}");
}
