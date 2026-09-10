//! **A laptop and a phone on one shared vault, doing what people actually do.**
//!
//! `collaboration_pipeline.rs` runs two users through the real commands, and parameterises them
//! over "both backends" — but only by swapping the `pull` function. Every *other* call in those
//! scenarios goes through [`fm_core::vcs`], which prefers a `git` binary whenever one exists, so
//! on any developer machine the `[native]` pass silently ran the subprocess backend for
//! committing and for the entire proposal lifecycle. That is how a phone-only breakage
//! ("could not run git — is it installed?") survived a green two-user proposal test.
//!
//! So this file pins the **device**, not just the merge function. The phone side runs with
//! `vcs::force_native(true)`, which stands in for "this device has no git binary" — the one
//! thing a dev machine can never be. Everything else is the real thing: real clones, a real bare
//! remote, real `fm_app::commands`, and the real `fm` merge driver on the laptop.
//!
//! **Why the pairing matters.** The two devices resolve a merge by completely different means —
//! the laptop shells out to the `merge=fm` driver, the phone cannot spawn a process at all and
//! resolves in-process via `merge::merge_texts`. A shared vault is only usable if those two agree
//! on the resulting *bytes*, so nearly every assertion here ends with "and both devices hold the
//! same content".
#![cfg(feature = "native-git")]

use fm_app::commands;
use fm_core::proposal::ProposalLimits;
use fm_core::{git, vcs, FileStore};
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn g(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git").arg("-C").arg(repo).args(args).output().unwrap()
}

/// `force_native` is process-global (it stands in for a property of a *device*), so no two
/// scenarios may be mid-flight at once. Every test takes this for its whole body.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

/// Run a step **as the phone**: no `git` binary, so every `vcs::` call lands on libgit2.
fn on_phone<T>(f: impl FnOnce() -> T) -> T {
    vcs::force_native(true);
    let out = f();
    vcs::force_native(false);
    out
}

/// Run a step **as the laptop**: the subprocess backend and the real merge driver.
fn on_laptop<T>(f: impl FnOnce() -> T) -> T {
    vcs::force_native(false);
    f()
}

fn identity(repo: &Path, name: &str, email: &str) {
    g(repo, &["config", "user.name", name]);
    g(repo, &["config", "user.email", email]);
}

/// Point a clone's `merge=fm` at the `fm` binary just built — the real desktop driver. Only the
/// laptop gets it: a phone has no binary to name, which is the whole reason the native backend
/// resolves merges itself.
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

/// A bare remote and two clones of it: `laptop` (driver installed) and `phone` (none).
fn devices() -> (tempfile::TempDir, tempfile::TempDir, tempfile::TempDir) {
    ensure_fm_beside_test_binary();
    let remote = tempfile::tempdir().unwrap();
    Command::new("git").args(["init", "--bare", "-b", "main"]).arg(remote.path()).output().unwrap();

    let seed = tempfile::tempdir().unwrap();
    git::ensure_repo(seed.path()).unwrap();
    g(seed.path(), &["symbolic-ref", "HEAD", "refs/heads/main"]);
    identity(seed.path(), "seed", "seed@example.org");
    g(seed.path(), &["add", "-A"]);
    g(seed.path(), &["commit", "-m", "base"]);
    let url = remote.path().to_str().unwrap();
    g(seed.path(), &["remote", "add", "origin", url]);
    g(seed.path(), &["push", "-u", "origin", "main"]);

    let laptop = tempfile::tempdir().unwrap();
    let phone = tempfile::tempdir().unwrap();
    for (dir, name, email) in
        [(&laptop, "Ada", "ada@example.org"), (&phone, "Ravi", "ravi@example.org")]
    {
        std::fs::remove_dir_all(dir.path()).unwrap();
        Command::new("git").arg("clone").arg(url).arg(dir.path()).output().unwrap();
        identity(dir.path(), name, email);
    }
    install_driver(laptop.path());
    (remote, laptop, phone)
}

fn open(dir: &Path) -> FileStore {
    FileStore::open(dir).unwrap()
}

/// Commit what a store just wrote and push `main` — through `vcs`, so the caller's device
/// decides which backend does the committing.
fn commit_push(dir: &Path, store: &FileStore, msg: &str) {
    vcs::commit_all(dir, msg, &store.written()).unwrap();
    g(dir, &["push", "origin", "main"]);
}

fn body_of(dir: &Path, id: &str) -> String {
    commands::get(&open(dir), id).unwrap().unwrap().body
}

// ===========================================================================
// The journeys.
// ===========================================================================

/// **The one the port existed for.** The phone proposes a change and pushes the branch; the
/// laptop reviews and accepts it. Before 2026-07-24 the first step could not run on a phone at
/// all — `create_proposal` died with "could not run git (is it installed?)".
#[test]
fn a_proposal_made_on_the_phone_is_accepted_on_the_laptop() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    // The laptop writes the note both devices will share.
    let host = on_laptop(|| {
        let mut store = open(laptop.path());
        let id = commands::capture(&mut store, "# Doc\n\noriginal line", "").unwrap().id;
        commit_push(laptop.path(), &store, "note: doc");
        id
    });

    // The phone syncs, then proposes — every git call here is libgit2.
    let prop = on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        let p = commands::create_proposal(
            &mut store,
            phone.path(),
            &host,
            "# Doc\n\noriginal line\n\nadded from the phone",
            &ProposalLimits::default(),
            Some(("Ravi", "ravi@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: proposal");
        p
    });

    // The laptop pulls, reviews the phone's proposal, and accepts it.
    on_laptop(|| {
        vcs::pull(laptop.path()).unwrap();
        let store = open(laptop.path());
        let diff = commands::proposal_diff(&store, laptop.path(), &prop.id).unwrap();
        assert!(diff.exists, "the laptop can review a proposal the phone pushed");
        assert!(diff.patch.contains("added from the phone"), "patch: {}", diff.patch);
        assert_eq!(
            commands::accept_proposal(&store, laptop.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
        );
        g(laptop.path(), &["push", "origin", "main"]);
    });

    // And the phone converges on it.
    on_phone(|| {
        vcs::pull(phone.path()).unwrap();
    });
    assert!(body_of(laptop.path(), &host).contains("added from the phone"));
    assert_eq!(
        body_of(phone.path(), &host),
        body_of(laptop.path(), &host),
        "both devices must hold the same bytes"
    );
}

/// The mirror image: the laptop proposes, the **phone** reviews and accepts. The accept runs
/// entirely on libgit2 and against `origin/proposal/<id>` — the phone never had a local branch.
#[test]
fn a_proposal_made_on_the_laptop_is_accepted_on_the_phone() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    let (host, prop) = on_laptop(|| {
        let mut store = open(laptop.path());
        let host = commands::capture(&mut store, "# Doc\n\noriginal line", "").unwrap().id;
        commit_push(laptop.path(), &store, "note: doc");
        let p = commands::create_proposal(
            &mut store,
            laptop.path(),
            &host,
            "# Doc\n\noriginal line\n\nadded from the laptop",
            &ProposalLimits::default(),
            Some(("Ada", "ada@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(laptop.path(), &store, "backup: proposal");
        (host, p)
    });

    on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let store = open(phone.path());
        let diff = commands::proposal_diff(&store, phone.path(), &prop.id).unwrap();
        assert!(diff.exists, "the phone can review a proposal it only holds as origin/proposal/*");
        assert!(diff.patch.contains("added from the laptop"), "patch: {}", diff.patch);
        assert_eq!(
            commands::accept_proposal(&store, phone.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
            "the phone accepts a branch it has no local ref for"
        );
        g(phone.path(), &["push", "origin", "main"]);
    });

    on_laptop(|| {
        vcs::pull(laptop.path()).unwrap();
    });
    assert!(body_of(phone.path(), &host).contains("added from the laptop"));
    assert_eq!(body_of(laptop.path(), &host), body_of(phone.path(), &host));
}

/// **The `merge=fm` equivalence, across devices.** The laptop edits a note and pushes; the phone
/// meanwhile holds a proposal touching the *same* note, and accepts it. Both sides therefore
/// rewrote `updated:`, which is the collision the desktop driver exists to absorb — and the phone
/// has no driver to call. If the in-process resolution were missing this reports a conflict, and
/// a phone user could never accept anything once the vault had any traffic.
#[test]
fn the_phone_accepts_cleanly_even_though_main_moved_under_it() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    let host = on_laptop(|| {
        let mut store = open(laptop.path());
        let id = commands::capture(&mut store, "# Doc\n\nalpha\nbravo\ncharlie\ndelta\necho", "")
            .unwrap()
            .id;
        commit_push(laptop.path(), &store, "note: doc");
        id
    });

    // The phone proposes a change to the last line.
    let prop = on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        let p = commands::create_proposal(
            &mut store,
            phone.path(),
            &host,
            "# Doc\n\nalpha\nbravo\ncharlie\ndelta\necho revised",
            &ProposalLimits::default(),
            Some(("Ravi", "ravi@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: proposal");
        p
    });

    // Meanwhile the laptop edits the FIRST line of the same note and pushes.
    on_laptop(|| {
        let mut store = open(laptop.path());
        vcs::pull(laptop.path()).unwrap();
        let mut store2 = open(laptop.path());
        std::mem::swap(&mut store, &mut store2);
        commands::update_body(
            &mut store,
            &host,
            "# Doc\n\nalpha edited\nbravo\ncharlie\ndelta\necho",
            "",
        )
        .unwrap();
        commit_push(laptop.path(), &store, "note: edit");
    });

    // The phone pulls that edit, then accepts its own proposal on top of it.
    on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let store = open(phone.path());
        assert_eq!(
            commands::accept_proposal(&store, phone.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
            "the manufactured `updated:` collision must not surface as a conflict on the phone"
        );
        g(phone.path(), &["push", "origin", "main"]);
    });

    let merged = body_of(phone.path(), &host);
    assert!(!merged.contains("<<<<<<<"), "no conflict markers: {merged}");
    assert!(merged.contains("alpha edited"), "the laptop's edit survives: {merged}");
    assert!(merged.contains("echo revised"), "and so does the proposal: {merged}");

    on_laptop(|| {
        vcs::pull(laptop.path()).unwrap();
    });
    assert_eq!(body_of(laptop.path(), &host), merged, "both devices converge on the same bytes");
}

/// Rejecting on the phone reaches the proposer's laptop — **through the note, not the branch.**
///
/// Worth stating because the obvious mental model is wrong: reject deletes the branch, so you
/// expect the proposer to find out by the branch vanishing. They do not, and they should not — a
/// fetch does not prune their local `proposal/<id>`. What actually carries the decision is
/// `declined: true` on the immortal proposal *note*, which rides `main` like any other edit, and
/// `proposal_diff` checks it **before** it consults git at all. So the leftover branch is inert.
///
/// The commit-and-push after the reject is therefore load-bearing, not ceremony: without it the
/// decision never leaves the device. (An earlier version of this test omitted it and "found" a
/// cross-device reject bug that does not exist.)
#[test]
fn rejecting_on_the_phone_reaches_the_proposer_as_a_declined_record() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    let (host, prop) = on_laptop(|| {
        let mut store = open(laptop.path());
        let host = commands::capture(&mut store, "# Doc\n\noriginal", "").unwrap().id;
        commit_push(laptop.path(), &store, "note: doc");
        let p = commands::create_proposal(
            &mut store,
            laptop.path(),
            &host,
            "# Doc\n\nnot wanted",
            &ProposalLimits::default(),
            Some(("Ada", "ada@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(laptop.path(), &store, "backup: proposal");
        (host, p)
    });

    on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        commands::reject_proposal(&mut store, phone.path(), &prop.id, None).unwrap();
        // The app auto-commits every write; without this the decision never leaves the phone.
        commit_push(phone.path(), &store, "reject: declined");
    });

    // Gone from the shared remote — nobody else will fetch it again.
    let refs =
        String::from_utf8_lossy(&g(phone.path(), &["ls-remote", "origin"]).stdout).into_owned();
    assert!(!refs.contains("proposal/"), "the shared branch is deleted: {refs}");

    on_laptop(|| {
        vcs::pull(laptop.path()).unwrap();
        let store = open(laptop.path());
        let diff = commands::proposal_diff(&store, laptop.path(), &prop.id).unwrap();
        assert!(diff.declined, "the proposer is told it was declined, via the note");
    });
    assert_eq!(body_of(laptop.path(), &host), "# Doc\n\noriginal", "and the note is untouched");
}

/// A reviewer asks for changes, the proposer **revises**, and the reviewer sees the new version
/// rather than the old one. The revise force-moves the branch on both the phone and the remote.
#[test]
fn a_revised_proposal_replaces_what_the_reviewer_sees() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    let host = on_laptop(|| {
        let mut store = open(laptop.path());
        let id = commands::capture(&mut store, "# Doc\n\noriginal", "").unwrap().id;
        commit_push(laptop.path(), &store, "note: doc");
        id
    });

    let prop = on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        let p = commands::create_proposal(
            &mut store,
            phone.path(),
            &host,
            "# Doc\n\nfirst attempt",
            &ProposalLimits::default(),
            Some(("Ravi", "ravi@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: proposal");
        // The reviewer said "not like that" — revise the same proposal in place.
        commands::create_proposal(
            &mut store,
            phone.path(),
            &host,
            "# Doc\n\nsecond attempt",
            &ProposalLimits::default(),
            Some(("Ravi", "ravi@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: revision");
        p
    });

    on_laptop(|| {
        vcs::pull(laptop.path()).unwrap();
        let store = open(laptop.path());
        let diff = commands::proposal_diff(&store, laptop.path(), &prop.id).unwrap();
        assert!(diff.exists);
        assert!(
            diff.patch.contains("second attempt"),
            "the reviewer sees the revision: {}",
            diff.patch
        );
        assert!(
            !diff.patch.contains("first attempt"),
            "and not the superseded one: {}",
            diff.patch
        );
        assert_eq!(
            commands::accept_proposal(&store, laptop.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
        );
    });
    assert!(body_of(laptop.path(), &host).contains("second attempt"));
}

/// **A phone with no network still works.** A proposal made offline stays local and can be
/// accepted locally; the push is best-effort and its failure must not cost the user the proposal.
#[test]
fn the_phone_can_propose_and_accept_with_no_remote_at_all() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let solo = tempfile::tempdir().unwrap();

    on_phone(|| {
        vcs::ensure_repo(solo.path()).unwrap();
        identity(solo.path(), "Ravi", "ravi@example.org");
        let mut store = open(solo.path());
        let host = commands::capture(&mut store, "# Field notes\n\nseen a heron", "").unwrap().id;
        vcs::commit_all(solo.path(), "note", &store.written()).unwrap();

        let prop = commands::create_proposal(
            &mut store,
            solo.path(),
            &host,
            "# Field notes\n\nseen a heron\n\nand a kingfisher",
            &ProposalLimits::default(),
            Some(("agent", "agent@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        vcs::commit_all(solo.path(), "backup: proposal", &store.written()).unwrap();

        let store = open(solo.path());
        let diff = commands::proposal_diff(&store, solo.path(), &prop.id).unwrap();
        assert!(diff.exists, "an offline proposal is still reviewable");
        assert_eq!(
            commands::accept_proposal(&store, solo.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
        );
        assert!(body_of(solo.path(), &host).contains("kingfisher"));
    });
}

/// **The data-loss case, in a collaboration setting.** The user is mid-edit in one note — the
/// debounce has not fired — and accepts a proposal about a *different* note. The in-progress work
/// must survive. A whole-tree forced checkout, the obvious way to apply a merge, reverts it.
#[test]
fn accepting_on_the_phone_does_not_eat_an_unsaved_edit_to_another_note() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    let (target, bystander) = on_laptop(|| {
        let mut store = open(laptop.path());
        let a = commands::capture(&mut store, "# Target\n\noriginal", "").unwrap().id;
        let b = commands::capture(&mut store, "# Bystander\n\nuntouched", "").unwrap().id;
        commit_push(laptop.path(), &store, "notes");
        (a, b)
    });

    let prop = on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        let p = commands::create_proposal(
            &mut store,
            phone.path(),
            &target,
            "# Target\n\nproposed",
            &ProposalLimits::default(),
            Some(("Ravi", "ravi@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: proposal");
        p
    });

    // Mid-edit in the bystander note: written to disk by the store, not yet committed.
    on_phone(|| {
        let mut store = open(phone.path());
        commands::update_body(
            &mut store,
            &bystander,
            "# Bystander\n\nhalf a thought I am still typing",
            "",
        )
        .unwrap();

        let store = open(phone.path());
        assert_eq!(
            commands::accept_proposal(&store, phone.path(), &prop.id).unwrap(),
            git::Accepted::Merged,
        );
    });

    assert!(body_of(phone.path(), &target).contains("proposed"), "the accept landed");
    assert!(
        body_of(phone.path(), &bystander).contains("half a thought I am still typing"),
        "and the unsaved edit to an unrelated note survived it"
    );
}

/// Two proposals open at once, accepted one after the other from the phone. The second must
/// still apply after the first moved `main` under it — the everyday case once an agent has run
/// more than once.
#[test]
fn two_open_proposals_accept_one_after_the_other_on_the_phone() {
    if !have_git() {
        return;
    }
    let _s = serial();
    let (_remote, laptop, phone) = devices();

    let (first, second) = on_laptop(|| {
        let mut store = open(laptop.path());
        let a = commands::capture(&mut store, "# One\n\noriginal one", "").unwrap().id;
        let b = commands::capture(&mut store, "# Two\n\noriginal two", "").unwrap().id;
        commit_push(laptop.path(), &store, "notes");
        (a, b)
    });

    on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        let p1 = commands::create_proposal(
            &mut store,
            phone.path(),
            &first,
            "# One\n\noriginal one\n\ntranscript one",
            &ProposalLimits::default(),
            Some(("agent", "agent@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        let p2 = commands::create_proposal(
            &mut store,
            phone.path(),
            &second,
            "# Two\n\noriginal two\n\ntranscript two",
            &ProposalLimits::default(),
            Some(("agent", "agent@example.org")),
            commands::Record::default(),
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: proposals");

        assert_eq!(vcs::proposal_load(phone.path()).unwrap().0, 2, "two proposals are open");

        let store = open(phone.path());
        assert_eq!(
            commands::accept_proposal(&store, phone.path(), &p1.id).unwrap(),
            git::Accepted::Merged,
        );
        let store = open(phone.path());
        assert_eq!(
            commands::accept_proposal(&store, phone.path(), &p2.id).unwrap(),
            git::Accepted::Merged,
            "the second still applies after the first moved main"
        );
        assert_eq!(vcs::proposal_load(phone.path()).unwrap().0, 0, "and both slots are freed");
    });

    assert!(body_of(phone.path(), &first).contains("transcript one"));
    assert!(body_of(phone.path(), &second).contains("transcript two"));
}
