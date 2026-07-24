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
        &["config", "merge.fm-manifest.driver", &format!("'{}' merge-manifest %O %A %B", fm.display())],
    );
}

fn ensure_fm_beside_test_binary() {
    let dst = std::env::current_exe().unwrap().parent().unwrap().join("fm");
    if !dst.exists() {
        let _ = std::fs::copy(env!("CARGO_BIN_EXE_fm"), &dst);
    }
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
        let id = commands::capture(
            &mut store,
            "# Doc\n\nalpha\nbravo\ncharlie\ndelta\necho",
            "",
        )
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
        commands::update_body(&mut store, &host, "# Doc\n\nalpha edited\nbravo\ncharlie\ndelta\necho", "")
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
        )
        .unwrap();
        commit_push(laptop.path(), &store, "backup: proposal");
        (host, p)
    });

    on_phone(|| {
        vcs::pull(phone.path()).unwrap();
        let mut store = open(phone.path());
        commands::reject_proposal(&mut store, phone.path(), &prop.id).unwrap();
        // The app auto-commits every write; without this the decision never leaves the phone.
        commit_push(phone.path(), &store, "reject: declined");
    });

    // Gone from the shared remote — nobody else will fetch it again.
    let refs = String::from_utf8_lossy(&g(phone.path(), &["ls-remote", "origin"]).stdout).into_owned();
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
        assert!(diff.patch.contains("second attempt"), "the reviewer sees the revision: {}", diff.patch);
        assert!(!diff.patch.contains("first attempt"), "and not the superseded one: {}", diff.patch);
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
        )
        .unwrap();
        commit_push(phone.path(), &store, "backup: proposal");
        p
    });

    // Mid-edit in the bystander note: written to disk by the store, not yet committed.
    on_phone(|| {
        let mut store = open(phone.path());
        commands::update_body(&mut store, &bystander, "# Bystander\n\nhalf a thought I am still typing", "")
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
        )
        .unwrap();
        let p2 = commands::create_proposal(
            &mut store,
            phone.path(),
            &second,
            "# Two\n\noriginal two\n\ntranscript two",
            &ProposalLimits::default(),
            Some(("agent", "agent@example.org")),
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
