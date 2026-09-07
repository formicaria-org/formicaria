//! The `.md` merge driver, against real git. Two clones, real commits, real merges —
//! because the thing being tested is a contract with git, not a function.

use fm_core::git;
use fm_model::PropertyValue;
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
    g(
        repo,
        &[
            "config",
            "merge.fm-manifest.driver",
            &format!("'{}' merge-manifest %O %A %B", fm.display()),
        ],
    );
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

/// **The card-drag case: two people drag one card to two columns, and neither loses.**
///
/// This test replaced a characterization test that locked the opposite behaviour, and the doc
/// comment that stood here is worth knowing about before changing anything: until 2026-09-07
/// `merge_objects` returned `None` on a divergent field, the whole file — YAML fence included —
/// went through the text merge, and the markers landed *inside* the frontmatter, so `from_file`
/// refused the note and it disappeared from every view. That was ruled deliberate on 2026-07-19
/// ("loud-and-absent beats quiet-and-wrong") on the grounds that the only alternative was letting
/// ours win, which is silent loss.
///
/// It is neither now. The loser is demoted into `conflict-status` beside the winner: both values
/// are in the file, the file **parses**, and the note keeps working. The reversal, the winner rule
/// and the acceptance clause it answers are in `decisions.md`, 2026-09-07 — **read it before
/// "fixing" this test in either direction.**
#[test]
fn a_divergent_status_field_keeps_both_values_and_the_note_still_parses() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let (ours, theirs) = two_clones(&task("2026-07-17T10:00:00Z", "todo", "Ship the thing.\n"));
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
    assert!(
        merge.status.success(),
        "a divergent field no longer stops the merge:\n{}",
        String::from_utf8_lossy(&merge.stderr)
    );

    let merged = fs::read_to_string(ours.path().join(rel)).unwrap();

    // The property that must never regress: nothing was resolved by fiat. Both values are here,
    // and now they are here in a file a person and a parser can both read.
    let obj = fm_core::frontmatter::from_file(&merged)
        .expect("the fence survives a divergent field — this is the change:\n");
    assert_eq!(obj.status.as_deref(), Some("done"), "the later `updated` wins:\n{merged}");
    assert_eq!(
        obj.extra.get("conflict-status"),
        Some(&PropertyValue::List(vec![PropertyValue::Text("doing".into())])),
        "and the loser is kept beside it, not discarded:\n{merged}"
    );
    assert!(!merged.contains("<<<<<<<"), "no markers anywhere near the YAML:\n{merged}");

    // Nothing about the note's own content moved.
    assert_eq!(obj.body.trim(), "Ship the thing.", "the body is untouched:\n{merged}");

    // **The merge is a fixed point.** This is what replaces the old "never Clean" acceptance
    // condition: a merge that settles a field must settle it the same way next time, or the two
    // devices re-derive the disagreement on every pull for ever.
    let (again, outcome) = fm_core::merge::merge_texts(&merged, &merged, &merged, 7).unwrap();
    assert_eq!(outcome, fm_core::merge::Merged::Clean, "re-merging the result is a non-event");
    assert_eq!(again, merged, "and changes nothing:\n{again}");
}

/// **The other device must reach the same file, byte for byte.** The winner rule reads only
/// content — later `updated`, then `Ord` — so which side is "ours" cannot enter into it. A rule
/// that preferred ours would pass the test above unchanged and would have the two devices trading
/// the same card back and forth for ever, which is the failure the 2026-07-19 acceptance clause
/// named.
///
/// Red proof: make the winner `ours.clone()` unconditionally in `Demote::settle` and this fails
/// while every other merge test still passes.
#[test]
fn both_devices_merge_a_divergent_field_to_the_same_bytes() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let base = task("2026-07-17T10:00:00Z", "todo", "Ship the thing.\n");
    let a = task("2026-07-17T11:00:00Z", "doing", "Ship the thing.\n");
    let b = task("2026-07-17T12:00:00Z", "done", "Ship the thing.\n");

    let (from_a, _) = fm_core::merge::merge_texts(&base, &a, &b, 7).unwrap();
    let (from_b, _) = fm_core::merge::merge_texts(&base, &b, &a, 7).unwrap();
    assert_eq!(from_a, from_b, "the two devices disagree about their own merge");
}

/// **The sorted set, which is the other half of byte identity.** When *each* side already carries a
/// demoted value the other has not seen, "ours first, then theirs" is a different order on the two
/// devices — same set, different bytes, and a merge that is not a fixed point. The test above cannot
/// see this: there the carried sets are empty and there is only one fresh loser to order.
///
/// Red proof: drop `losers.sort()` from `merge_demoted` and this fails alone.
#[test]
fn two_devices_carrying_different_demoted_values_still_agree_byte_for_byte() {
    let with = |updated: &str, demoted: &str| {
        format!(
            "---\nid: 01JQ0000000000000000000000\ntype: task\ntitle: shared task\n\
             created: 2026-07-17T10:00:00Z\nupdated: {updated}\nstatus: done\n\
             conflict-status:\n  - {demoted}\n---\n\nShip the thing.\n"
        )
    };
    let base = with("2026-07-17T10:00:00Z", "todo");
    let a = with("2026-07-17T11:00:00Z", "doing");
    let b = with("2026-07-17T12:00:00Z", "blocked");

    let (from_a, _) = fm_core::merge::merge_texts(&base, &a, &b, 7).unwrap();
    let (from_b, _) = fm_core::merge::merge_texts(&base, &b, &a, 7).unwrap();
    assert_eq!(from_a, from_b, "the two devices order the same set differently");

    // And the base's own value, which both sides dropped, stays dropped.
    let obj = fm_core::frontmatter::from_file(&from_a).expect("parses:\n");
    assert_eq!(
        obj.extra.get("conflict-status"),
        Some(&PropertyValue::List(vec![
            PropertyValue::Text("blocked".into()),
            PropertyValue::Text("doing".into()),
        ])),
        "both sides' records survive, and `todo` — deleted by both — does not:\n{from_a}"
    );
}

/// A second disagreement **adds** to the record; it does not overwrite the first. Overwriting
/// would be exactly the silent loss the whole mechanism exists to prevent, one level in.
#[test]
fn a_second_divergence_keeps_the_first_loser_too() {
    let base = task("2026-07-17T10:00:00Z", "todo", "Ship the thing.\n");
    let a = task("2026-07-17T11:00:00Z", "doing", "Ship the thing.\n");
    let b = task("2026-07-17T12:00:00Z", "done", "Ship the thing.\n");
    let (once, _) = fm_core::merge::merge_texts(&base, &a, &b, 7).unwrap();

    // From that shared state, the two devices disagree again.
    let c = once.replace("status: done", "status: review").replace("T12:00:00Z", "T13:00:00Z");
    let d = once.replace("status: done", "status: blocked").replace("T12:00:00Z", "T14:00:00Z");
    let (twice, _) = fm_core::merge::merge_texts(&once, &c, &d, 7).unwrap();

    let obj = fm_core::frontmatter::from_file(&twice).expect("still parses:\n");
    assert_eq!(obj.status.as_deref(), Some("blocked"), "later `updated` again:\n{twice}");
    assert_eq!(
        obj.extra.get("conflict-status"),
        Some(&PropertyValue::List(vec![
            PropertyValue::Text("doing".into()),
            PropertyValue::Text("review".into()),
        ])),
        "both earlier values are still there, sorted:\n{twice}"
    );
}

/// **Deleting the line is the user's undo, and it has to stick.** A plain union would re-add the
/// value from whichever device has not looked at it yet — and then from the one that has, for ever,
/// with no way to dismiss it. So the set honours a removal by a side that had it.
///
/// The companion half: once the user *promotes* the loser, it stops being a loser everywhere,
/// because a demoted value equal to the winner is dropped.
#[test]
fn a_demoted_value_can_be_dismissed_and_promoted() {
    let base = task("2026-07-17T10:00:00Z", "todo", "Ship the thing.\n");
    let a = task("2026-07-17T11:00:00Z", "doing", "Ship the thing.\n");
    let b = task("2026-07-17T12:00:00Z", "done", "Ship the thing.\n");
    let (settled, _) = fm_core::merge::merge_texts(&base, &a, &b, 7).unwrap();
    assert!(settled.contains("conflict-status"), "precondition:\n{settled}");

    // One device dismisses it; the other has not looked. The dismissal survives the pull.
    let dismissed: String = settled
        .lines()
        .filter(|l| !l.contains("conflict-status") && *l != "- doing")
        .fold(String::new(), |mut acc, l| {
            acc.push_str(l);
            acc.push('\n');
            acc
        });
    assert!(!dismissed.contains("doing"), "the line really is gone:\n{dismissed}");
    let (after, _) = fm_core::merge::merge_texts(&settled, &dismissed, &settled, 7).unwrap();
    assert!(!after.contains("conflict-status"), "a dismissal is not undone by a pull:\n{after}");

    // And promoting it clears the record on every device, not only the one that promoted.
    let promoted = settled.replace("status: done", "status: doing");
    let (after, _) = fm_core::merge::merge_texts(&settled, &promoted, &settled, 7).unwrap();
    let obj = fm_core::frontmatter::from_file(&after).expect("parses:\n");
    assert_eq!(obj.status.as_deref(), Some("doing"), "{after}");
    assert_eq!(obj.extra.get("conflict-status"), None, "no longer a disagreement:\n{after}");
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
    fs::write(
        theirs.path().join("notes/theirs.md"),
        note("2026-07-17T12:00:00Z", "- shared\n", "A note only they have.\n"),
    )
    .unwrap();
    g(theirs.path(), &["add", "-A"]);
    g(theirs.path(), &["commit", "-m", "theirs"]);
    g(theirs.path(), &["push", "-q", "bare", "HEAD:main"]);

    // The cheap question, asked before spending a fetch: has anything moved?
    assert_eq!(git::remote_moved(ours.path()).unwrap(), Some(true), "ls-remote sees their push");

    match git::pull(ours.path()).unwrap() {
        git::Pulled::Merged { incoming, .. } => {
            assert_eq!(incoming, 1, "one commit of theirs arrived")
        }
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

/// The same contract for `manifest.json`, and the reason it needed one.
///
/// The manifest is staged on **every** commit, so two people who each attach a file — an
/// ordinary Tuesday in a shared vault — both rewrite it from the same base. Git's text merge
/// then puts `<<<<<<<` markers inside a JSON document that no user wrote, can read, or can
/// resolve; and while it sits conflicted `commit_all` refuses to commit anything else in the
/// vault, so the whole thing silently stops recording. A union cannot conflict: the key *is*
/// the content.
#[test]
fn two_people_attaching_files_do_not_conflict_in_the_manifest() {
    if !have_git() {
        eprintln!("skipping merge test: git not on PATH");
        return;
    }
    let base = r#"{"schema":1,"blobs":{"aa":1}}"#;
    let (ours, theirs) = two_clones_with_manifest(base);

    fs::write(ours.path().join("manifest.json"), r#"{"schema":1,"blobs":{"aa":1,"bb":2}}"#)
        .unwrap();
    g(ours.path(), &["commit", "-am", "we attach a file"]);
    fs::write(theirs.path().join("manifest.json"), r#"{"schema":1,"blobs":{"aa":1,"cc":3}}"#)
        .unwrap();
    g(theirs.path(), &["commit", "-am", "they attach a different file"]);

    let out = g(theirs.path(), &["pull", "--no-rebase", "-q", "origin", "main"]);
    let merged = fs::read_to_string(theirs.path().join("manifest.json")).unwrap();

    assert!(
        out.status.success(),
        "the pull must not stop on the manifest:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!merged.contains("<<<<<<<"), "no markers in a file a user cannot resolve:\n{merged}");
    for hash in ["aa", "bb", "cc"] {
        assert!(merged.contains(&format!("\"{hash}\"")), "{hash} survived the merge:\n{merged}");
    }
    // And it is still parseable JSON, which is the whole point of not text-merging it.
    let parsed: serde_json::Value = serde_json::from_str(&merged).unwrap();
    assert_eq!(parsed["blobs"].as_object().unwrap().len(), 3);
}

/// [`two_clones`] for the manifest: same shape, but the tracked file under test is
/// `manifest.json` rather than a note.
fn two_clones_with_manifest(initial: &str) -> (tempfile::TempDir, tempfile::TempDir) {
    let ours = tempdir().unwrap();
    git::ensure_repo(ours.path()).unwrap();
    g(ours.path(), &["symbolic-ref", "HEAD", "refs/heads/main"]);
    fs::write(ours.path().join("manifest.json"), initial).unwrap();
    g(ours.path(), &["add", "-A"]);
    g(ours.path(), &["commit", "-m", "base"]);

    let theirs = tempdir().unwrap();
    fs::remove_dir_all(theirs.path()).unwrap();
    Command::new("git").arg("clone").arg(ours.path()).arg(theirs.path()).output().unwrap();
    for (k, v) in [("user.email", "them@example.org"), ("user.name", "Them")] {
        g(theirs.path(), &["config", k, v]);
    }
    // The clone's `origin` is our working tree, so it can pull from us directly.
    install_driver(ours.path());
    install_driver(theirs.path());
    (ours, theirs)
}
