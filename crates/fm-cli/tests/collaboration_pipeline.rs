//! **Two local users on one shared git vault, concurrently, through the real app.** This is the
//! "does collaboration actually work end to end" test, and the CLI crate is the only place that can
//! host it: it has both the real `fm` merge driver (via `CARGO_BIN_EXE_fm`) *and* `fm_app::commands`.
//!
//! The model the owner asked for: a **bare remote** plus **two clones** (Ada and Ravi), each opened as
//! a real `FileStore`, edited through the real commands, and synced by real `git` push/pull. Every
//! scenario runs on **both merge backends** — the subprocess `fm` driver (desktop) and, under the
//! `native-git` feature, the in-process native merge (phone) — so "works the same on both devices" is
//! proven rather than assumed.
//!
//! **What the `native` pass here does NOT cover — read this before trusting the label.** The loop
//! swaps only the `pull` function. Every other call in these scenarios goes through `fm_core::vcs`,
//! which prefers a `git` binary whenever one exists — so on any developer machine (and in CI) the
//! committing and the whole proposal lifecycle run on the **subprocess** backend in both passes.
//! That is exactly how a phone-only breakage survived a green `[native]` cross-user proposal test
//! until 2026-07-24. Device-pinned coverage lives in `mixed_device_collaboration.rs`, which uses
//! `vcs::force_native` to stand in for a device that has no git binary at all.
//!
//! The single-user propose→review→accept cycle is covered in `fm-app/tests/proposal_cycle.rs`.

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, FileStore};
use std::path::Path;
use std::process::Command;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn g(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git").arg("-C").arg(repo).args(args).output().unwrap()
}

fn identity(repo: &Path, name: &str, email: &str) {
    g(repo, &["config", "user.name", name]);
    g(repo, &["config", "user.email", email]);
}

/// Point a repo's `merge=fm` at the `fm` binary we just built (the real desktop driver). In the
/// product `ensure_repo` does this with the binary beside `fm-serve`; a test binary in
/// `target/debug/deps` is not it.
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

/// The merge backends every scenario runs on: the desktop subprocess driver always, and the phone's
/// native in-process merge when the feature is compiled in.
type Pull = fn(&Path) -> Result<git::Pulled, fm_core::StoreError>;
fn backends() -> Vec<(&'static str, Pull)> {
    #[allow(unused_mut)]
    let mut v: Vec<(&'static str, Pull)> = vec![("driver", git::pull as Pull)];
    #[cfg(feature = "native-git")]
    v.push(("native", fm_core::git_native::pull as Pull));
    v
}

/// Put the built `fm` binary **beside the test binary**, because `git::ensure_repo` (called inside
/// `commit_all` and `pull`) resolves the merge driver via `current_exe().parent()/fm` — exactly as the
/// product finds `fm` beside `fm-serve`. Without this, every `ensure_repo` in the real code path would
/// *clear* the driver (no `fm` in `deps/`), and a plain merge would conflict on every `updated:` line —
/// the exact failure the driver exists to prevent. This makes the real driver path work unmodified,
/// rather than side-stepping it with a raw `git merge`.
/// Put the built `fm` **beside the test binary**, because `git::ensure_repo` — which runs inside
/// every `commit_all` and `pull` — resolves the merge driver as `current_exe().parent()/fm`, the
/// same way the product finds `fm` beside `fm-serve`. Without it `ensure_repo` *clears* the driver
/// (there is no `fm` in `deps/`) and the desktop half of every test here silently measures **bare
/// git** rather than anything of ours.
///
/// **Copied when missing *or stale*, which is the half that was wrong.** The original guard was
/// `if !dst.exists()`, so whatever a previous run left there answered as the merge driver for ever:
/// on 2026-09-07 that was a six-week-old `fm`, and three suites were grading a build nobody had
/// made since. Renamed into place rather than written in place, so a second test binary doing this
/// concurrently never sees a half-copied driver.
/// Put the built `fm` **beside the test binary**, because `git::ensure_repo` — which runs inside
/// every `commit_all` and `pull` — resolves the merge driver as `current_exe().parent()/fm`, the
/// same way the product finds `fm` beside `fm-serve`. Without it `ensure_repo` *clears* the driver
/// (there is no `fm` in `deps/`) and the desktop half of every test here silently measures **bare
/// git** rather than anything of ours.
///
/// **Two ways to get this wrong, both found on 2026-09-07, both by their symptoms rather than by
/// reading the code.**
///
/// 1. The original guard was `if !dst.exists()`, so whatever a previous run left there answered for
///    ever — a **six-week-old `fm`**, and three suites were grading a build nobody had made since.
/// 2. Replacing that with an mtime comparison is *also* wrong, and worse because it looks right.
///    `pixi run ci` runs `test` (no `native-git`) and `test-native-git` as separate tasks, so cargo
///    alternates two different `fm` builds through the same path — and restoring a cached artifact
///    moves its mtime **backwards**. "Older than the source" is then simply not what "stale" means
///    here. Measured: a 102 MB native build at 18:59 and a 57 MB plain one at 18:57, in that order.
///
/// So the copy is keyed on a **stamp** of the source's (length, mtime) — an equality, not an
/// ordering — and the new binary is verified to execute *before* it is renamed into place, because
/// the two tasks can also be mid-swap on the source while this reads it. A driver git cannot run is
/// silent: git takes the failed exec as "conflict" and hands back `%A` untouched, so the merge
/// yields one side with no markers and no error anywhere. That is what the flake looked like.
fn ensure_fm_beside_test_binary() {
    // **Once per process, because the tests in this binary run as threads and share `deps/`.**
    // The temp name used to be `fm.{process::id()}.tmp` — constant within a process — so all of
    // this file's tests raced on one path: one thread `exec`s it while another is still writing
    // (`Text file busy`), a third renames it away under a fourth (`No such file or directory`).
    // Every one of those errors was discarded, so the losers simply carried on without a driver and
    // measured bare git.
    //
    // It passed on a developer machine because a *stamped leftover* `deps/fm` from an earlier run
    // made every thread return early, so no copy was ever attempted. On a fresh runner all of them
    // attempt it at once — which is why `ci.yml` failed deterministically on exactly the tests that
    // ran before the winning `rename` landed, and why no developer ever saw it (2026-09-10).
    //
    // `Once` also makes the late threads *wait* rather than race, which is the property the tests
    // actually need: by the time any of them merges, the driver is in place.
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(ensure_fm_beside_test_binary_inner);
}

fn ensure_fm_beside_test_binary_inner() {
    let src = Path::new(env!("CARGO_BIN_EXE_fm"));
    let dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let (dst, stamp_path) = (dir.join("fm"), dir.join("fm.stamp"));

    let meta =
        std::fs::metadata(src).unwrap_or_else(|e| panic!("no fm binary at {}: {e}", src.display()));
    let stamp = format!("{:?}:{}", meta.modified().ok(), meta.len());
    if std::fs::read_to_string(&stamp_path).is_ok_and(|s| s == stamp) && dst.exists() {
        return;
    }

    // **Every step is now checked, and that is the point of this rewrite.** Until 2026-09-10 each
    // of these was `let _ = …` and the whole function could fail without a word: the driver would
    // simply be absent, `install_merge_driver` would *clear* `merge.fm.driver`, and every "desktop"
    // assertion here would quietly measure **bare git** — which conflicts on the `updated:` line of
    // any two-sided edit. That is precisely the failure this helper exists to prevent, reproduced by
    // the helper itself, and it is what `ci.yml` hit on 2026-09-10: two tests failing identically on
    // a runner while passing on every developer machine, with nothing anywhere saying why.
    //
    // A test fixture that cannot be established is not a reason to carry on quietly.
    // Unique per process *and* per call: two test binaries share this `deps/` directory and cargo
    // is free to have both here at once.
    let tmp = dir.join(format!("fm.{}.{:?}.tmp", std::process::id(), std::thread::current().id()));
    if let Err(e) = std::fs::copy(src, &tmp) {
        let _ = std::fs::remove_file(&tmp);
        panic!(
            "could not copy the fm driver into {}: {e}\n  \
             git resolves merge.fm.driver as current_exe().parent()/fm, so without it every\n  \
             desktop merge here silently becomes a plain git merge.",
            dir.display()
        );
    }
    // Does it actually run? Any exit status will do — `fm` with no arguments prints its usage and
    // fails, which still proves the OS could execute the file. What this rejects is a truncated or
    // half-written copy, which is the only failure mode that matters and the only silent one.
    if let Err(e) = std::process::Command::new(&tmp).output() {
        let _ = std::fs::remove_file(&tmp);
        panic!("the copied fm driver at {} will not execute: {e}", tmp.display());
    }
    if let Err(e) = std::fs::rename(&tmp, &dst) {
        let _ = std::fs::remove_file(&tmp);
        panic!("could not put the fm driver at {}: {e}", dst.display());
    }
    // The stamp is only a cache, so a failure to write it costs a copy next time and nothing else.
    let _ = std::fs::write(&stamp_path, &stamp);
}

/// A bare remote plus two clones of it, both carrying the `fm` driver and tracking `main`.
/// Returns (remote, ada_dir, ravi_dir).
fn two_users() -> (tempfile::TempDir, tempfile::TempDir, tempfile::TempDir) {
    ensure_fm_beside_test_binary();
    let remote = tempfile::tempdir().unwrap();
    Command::new("git").args(["init", "--bare", "-b", "main"]).arg(remote.path()).output().unwrap();

    // Seed the remote with a base commit so both clones have `main` and the `.gitattributes` that
    // makes `*.md merge=fm` (written by ensure_repo) — without it, git's plain merge conflicts on
    // every `updated:` line and the whole point is lost.
    let seed = tempfile::tempdir().unwrap();
    git::ensure_repo(seed.path()).unwrap();
    g(seed.path(), &["symbolic-ref", "HEAD", "refs/heads/main"]);
    identity(seed.path(), "seed", "seed@example.org");
    g(seed.path(), &["add", "-A"]);
    g(seed.path(), &["commit", "-m", "base"]);
    let url = remote.path().to_str().unwrap();
    g(seed.path(), &["remote", "add", "origin", url]);
    g(seed.path(), &["push", "-u", "origin", "main"]);

    let ada = tempfile::tempdir().unwrap();
    let ravi = tempfile::tempdir().unwrap();
    for (dir, name, email) in
        [(&ada, "Ada", "ada@example.org"), (&ravi, "Ravi", "ravi@example.org")]
    {
        std::fs::remove_dir_all(dir.path()).unwrap();
        Command::new("git").arg("clone").arg(url).arg(dir.path()).output().unwrap();
        identity(dir.path(), name, email);
        install_driver(dir.path());
    }
    (remote, ada, ravi)
}

/// Open a fresh `FileStore` over a clone — the "re-read after git wrote files behind our back" the app
/// does after any pull/merge, done the simplest way.
fn open(dir: &Path) -> FileStore {
    FileStore::open(dir).unwrap()
}

/// Commit everything a store just wrote, then push `main` to the remote.
fn commit_push(dir: &Path, store: &FileStore, msg: &str) {
    git::commit_all(dir, msg, &store.written()).unwrap();
    g(dir, &["push", "origin", "main"]);
}

#[test]
fn a_note_one_user_creates_reaches_the_other_after_a_sync() {
    if !have_git() {
        return;
    }
    for (backend, pull) in backends() {
        let (_remote, ada, ravi) = two_users();

        // Ada writes a note and pushes it.
        let mut ada_store = open(ada.path());
        let id =
            fm_app::commands::capture(&mut ada_store, "# Plan\n\nship the thing", "").unwrap().id;
        commit_push(ada.path(), &ada_store, "note: plan");

        // Ravi, who had nothing, pulls and now has it — byte-identical, and readable through the store.
        let outcome = pull(ravi.path()).unwrap();
        assert!(
            matches!(outcome, git::Pulled::Merged { .. }),
            "[{backend}] Ravi should merge Ada's note: {outcome:?}"
        );
        let ravi_store = open(ravi.path());
        let note = fm_app::commands::get(&ravi_store, &id).unwrap();
        assert!(note.is_some(), "[{backend}] Ravi can read the note Ada created");
        assert!(note.unwrap().body.contains("ship the thing"), "[{backend}] with Ada's content");
    }
}

/// Create + sync a shared note, returning its id — the starting point for the concurrent-edit
/// scenarios, where both users already hold the same note.
fn shared_note(ada: &Path, ravi: &Path, pull: Pull, body: &str) -> String {
    let mut ada_store = open(ada);
    let id = fm_app::commands::capture(&mut ada_store, body, "").unwrap().id;
    commit_push(ada, &ada_store, "note: shared");
    pull(ravi).unwrap();
    id
}

#[test]
fn concurrent_edits_to_different_lines_of_one_note_merge_cleanly() {
    if !have_git() {
        return;
    }
    for (backend, pull) in backends() {
        let (_remote, ada, ravi) = two_users();
        let base = "# Note\n\npara one\n\npara two";
        let id = shared_note(ada.path(), ravi.path(), pull, base);

        // Ada rewrites the first paragraph and pushes.
        let mut ada_store = open(ada.path());
        fm_app::commands::update_body(
            &mut ada_store,
            &id,
            "# Note\n\npara ONE (ada)\n\npara two",
            "",
        )
        .unwrap();
        commit_push(ada.path(), &ada_store, "ada: para one");

        // Ravi rewrites the *second* paragraph from the same base, commits (now behind), and pulls.
        let mut ravi_store = open(ravi.path());
        fm_app::commands::update_body(
            &mut ravi_store,
            &id,
            "# Note\n\npara one\n\npara TWO (ravi)",
            "",
        )
        .unwrap();
        git::commit_all(ravi.path(), "ravi: para two", &ravi_store.written()).unwrap();
        let outcome = pull(ravi.path()).unwrap();

        // Different lines → the driver merges it to a non-event: both edits present, no markers, and
        // the manufactured `updated:` collision never surfaces.
        assert!(
            matches!(outcome, git::Pulled::Merged { .. }),
            "[{backend}] a different-line edit must merge clean: {outcome:?}"
        );
        let body = fm_app::commands::get(&open(ravi.path()), &id).unwrap().unwrap().body;
        assert!(body.contains("ONE (ada)"), "[{backend}] Ada's edit survived: {body}");
        assert!(body.contains("TWO (ravi)"), "[{backend}] Ravi's edit survived: {body}");
        assert!(!body.contains("<<<<<<<"), "[{backend}] and no conflict markers: {body}");
    }
}

/// Strip git conflict markers keeping the "ours" side — the mechanical part of what a user does by
/// hand in the raw editor when resolving a conflict. Test-only.
fn resolve_keep_ours(text: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        if t.starts_with("<<<<<<<") {
            continue;
        } else if t.starts_with("=======") {
            skipping = true;
            continue;
        } else if t.starts_with(">>>>>>>") {
            skipping = false;
            continue;
        }
        if !skipping {
            out.push_str(line);
        }
    }
    out
}

#[test]
fn concurrent_edits_to_the_same_line_conflict_visibly_and_can_be_resolved() {
    if !have_git() {
        return;
    }
    for (backend, pull) in backends() {
        let (_remote, ada, ravi) = two_users();
        let base = "# Note\n\nthe one line";
        let id = shared_note(ada.path(), ravi.path(), pull, base);
        let rel = format!("notes/{id}.md");

        // Both rewrite the SAME line, differently.
        let mut ada_store = open(ada.path());
        fm_app::commands::update_body(
            &mut ada_store,
            &id,
            "# Note\n\nthe one line — Ada's take",
            "",
        )
        .unwrap();
        commit_push(ada.path(), &ada_store, "ada: the line");

        let mut ravi_store = open(ravi.path());
        fm_app::commands::update_body(
            &mut ravi_store,
            &id,
            "# Note\n\nthe one line — Ravi's take",
            "",
        )
        .unwrap();
        git::commit_all(ravi.path(), "ravi: the line", &ravi_store.written()).unwrap();
        let outcome = pull(ravi.path()).unwrap();

        // The intended UX (decisions.md #4 / merge.rs): a real disagreement is KEPT, in the body, not
        // resolved by fiat — and crucially the note still *parses and renders* (frontmatter stays
        // valid), so it is findable and fixable rather than vanishing.
        assert!(
            matches!(outcome, git::Pulled::Conflicted(_)),
            "[{backend}] a same-line edit must conflict: {outcome:?}"
        );
        let ravi_store = open(ravi.path());
        let note = fm_app::commands::get(&ravi_store, &id).unwrap();
        assert!(
            note.is_some(),
            "[{backend}] a conflicted note still parses and renders (not skipped)"
        );
        let body = note.unwrap().body;
        assert!(
            body.contains("<<<<<<<") && body.contains(">>>>>>>"),
            "[{backend}] both sides are kept in the body: {body}"
        );
        assert!(
            body.contains("Ada's take") && body.contains("Ravi's take"),
            "[{backend}] neither side was dropped"
        );
        // And it is surfaced as a conflict to resolve, not silently.
        let listed: Vec<String> =
            fm_app::commands::conflicts(&ravi_store).unwrap().into_iter().map(|m| m.id).collect();
        assert!(listed.contains(&id), "[{backend}] the conflict is listed for the user");

        // Ravi resolves it (keep his side), commits to finish the merge, pushes.
        let resolved = resolve_keep_ours(&std::fs::read_to_string(ravi.path().join(&rel)).unwrap());
        std::fs::write(ravi.path().join(&rel), &resolved).unwrap();
        g(ravi.path(), &["add", "-A"]);
        g(ravi.path(), &["commit", "--no-edit"]);
        g(ravi.path(), &["push", "origin", "main"]);

        // Converged: Ravi's clean note has no markers, and Ada pulling gets exactly it.
        let ravi_final = fm_app::commands::get(&open(ravi.path()), &id).unwrap().unwrap().body;
        assert!(
            !ravi_final.contains("<<<<<<<") && ravi_final.contains("Ravi's take"),
            "[{backend}] resolved clean: {ravi_final}"
        );
        pull(ada.path()).unwrap();
        let ada_final = fm_app::commands::get(&open(ada.path()), &id).unwrap().unwrap().body;
        assert_eq!(ada_final, ravi_final, "[{backend}] both users converge on the resolution");
    }
}

#[test]
fn a_proposal_by_one_user_is_reviewed_and_accepted_by_the_other() {
    if !have_git() {
        return;
    }
    for (backend, pull) in backends() {
        let (_remote, ada, ravi) = two_users();

        // Ada writes a note, then proposes a change to it. `create_proposal` pushes the
        // `proposal/<id>` branch to the remote; the proposal note rides `main`.
        let mut ada_store = open(ada.path());
        let host = commands::capture(&mut ada_store, "# Doc\n\noriginal", "").unwrap().id;
        commit_push(ada.path(), &ada_store, "note: doc");
        let prop = commands::create_proposal(
            &mut ada_store,
            ada.path(),
            &host,
            "# Doc\n\nrevised by Ada",
            &ProposalLimits::default(),
            Some(("Ada", "ada@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(ada.path(), &ada_store, "backup: proposal");

        // Ravi pulls — gets the proposal note on `main` and fetches `origin/proposal/<id>` — and can
        // REVIEW it: the branch resolves through the remote-tracking ref, not a local branch he lacks.
        pull(ravi.path()).unwrap();
        let ravi_store = open(ravi.path());
        let diff = commands::proposal_diff(&ravi_store, ravi.path(), &prop.id).unwrap();
        assert!(diff.exists, "[{backend}] Ravi can review Ada's proposal across the sync");
        assert!(
            diff.patch.contains("revised by Ada"),
            "[{backend}] the diff shows Ada's change: {}",
            diff.patch
        );

        // Ravi ACCEPTS it — merges `origin/proposal/<id>` into main and deletes the shared branch.
        assert_eq!(
            commands::accept_proposal(&ravi_store, ravi.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
            "[{backend}] Ravi accepts the cross-user proposal cleanly"
        );
        g(ravi.path(), &["push", "origin", "main"]);

        // Live for Ravi, and Ada converges on it after a pull.
        let ravi_body = commands::get(&open(ravi.path()), &host).unwrap().unwrap().body;
        assert!(
            ravi_body.contains("revised by Ada"),
            "[{backend}] the accepted change is live for Ravi: {ravi_body}"
        );
        pull(ada.path()).unwrap();
        let ada_body = commands::get(&open(ada.path()), &host).unwrap().unwrap().body;
        assert_eq!(ada_body, ravi_body, "[{backend}] both users converge on the accepted proposal");
    }
}
