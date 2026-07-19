//! The two git backends must be indistinguishable — proven, not asserted.
//!
//! `fm_core::git` shells out; `fm_core::git_native` links libgit2 for platforms with no `git`
//! binary (verified on a real device: Android has none). Two implementations of the sync path
//! that disagree is the corruption this project exists to prevent, so every test here runs the
//! *same* sequence through both and compares the observable result.
//!
//! **Real `git` is the oracle for both.** Comparisons read the repositories back with the
//! subprocess, never with libgit2 — asking an implementation to grade itself proves nothing,
//! and the thing that actually matters is that a collaborator's ordinary `git` sees the same
//! repository either way.
//!
//! Same discipline as `merge_differential.rs`, for the same reason.
#![cfg(feature = "native-git")]

use fm_core::{git, git_native};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::{tempdir, TempDir};

fn have_git() -> bool {
    Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// Read a repo back with **real git**, never with either backend under test.
fn g(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Two empty vaults, to run the same sequence through each backend.
fn pair() -> (TempDir, TempDir) {
    (tempdir().unwrap(), tempdir().unwrap())
}

/// Put **both** vaults on the repo-local placeholder before comparing anything about identity.
///
/// Without this the suite grades the machine it runs on rather than the code. **Both** backends
/// resolve an identity the same way — repo-local first, then the user's `~/.gitconfig` — so on
/// any developer's box a fresh vault reports *their* name, and a test expecting "nobody signs
/// this yet" fails for a reason that has nothing to do with either backend. Measured: given a
/// global identity, subprocess and native return byte-identical `Identity` values and neither
/// writes a placeholder. `tests/git.rs` carries the same helper for the same reason.
fn no_identity(vault: &Path) {
    for (k, v) in [("user.email", "formicaria@localhost"), ("user.name", "formicaria")] {
        Command::new("git").arg("-C").arg(vault).args(["config", k, v]).output().unwrap();
    }
}

/// The vault files, the return value, and the placeholder identity all have to match.
///
/// The placeholder is the one that bit: the first native `ensure_repo` did not write it, so a
/// commit on a fresh vault failed for want of a signature — on the one platform that has no
/// global git config to fall back on. This test is why that is fixed.
#[test]
fn ensure_repo_leaves_both_vaults_in_the_same_state() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    assert_eq!(
        git::ensure_repo(a.path()).unwrap(),
        git_native::ensure_repo(b.path()).unwrap(),
        "both report 'created' identically"
    );
    assert!(!git::ensure_repo(a.path()).unwrap(), "and both report false the second time");
    assert!(!git_native::ensure_repo(b.path()).unwrap());

    for (x, y) in [(a.path(), b.path())] {
        for f in [".gitignore", ".gitattributes"] {
            assert_eq!(
                fs::read_to_string(x.join(f)).unwrap(),
                fs::read_to_string(y.join(f)).unwrap(),
                "{f} differs between backends"
            );
        }
        // Both must be *committable* — that is the property that matters, and the one the
        // first native version failed. Not compared by value: `git config --get` falls back to
        // the global config, so on a developer's machine these legitimately differ (pinned in
        // `the_backends_differ_on_global_git_config`).
        assert!(
            !g(x, &["config", "--get", "user.email"]).is_empty(),
            "a vault with no committer cannot commit at all"
        );
        assert!(!g(y, &["config", "--get", "user.email"]).is_empty());
    }
}

/// Identity: accepted values, refused values, and what `identity()` reports back.
#[test]
fn identity_agrees_on_what_is_a_person_and_what_is_refused() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    git::ensure_repo(a.path()).unwrap();
    git_native::ensure_repo(b.path()).unwrap();
    no_identity(a.path());
    no_identity(b.path());

    // The placeholder is a sentinel, not a person — both must report nobody.
    assert_eq!(git::identity(a.path()), None);
    assert_eq!(git_native::identity(b.path()), None);

    // Every refusal must be a refusal on both.
    for (n, e) in [("", "a@b.c"), ("Ada", ""), ("Ada", "not-an-email"), ("Ada", "formicaria@localhost")] {
        assert_eq!(
            git::set_identity(a.path(), n, e).is_err(),
            git_native::set_identity(b.path(), n, e).is_err(),
            "disagreed about refusing ({n:?}, {e:?})"
        );
        assert!(git::set_identity(a.path(), n, e).is_err(), "({n:?}, {e:?}) must be refused");
    }

    // And an accepted one lands the same, read back with real git.
    git::set_identity(a.path(), " Ada Lovelace ", " ada@example.org ").unwrap();
    git_native::set_identity(b.path(), " Ada Lovelace ", " ada@example.org ").unwrap();
    assert_eq!(git::identity(a.path()), git_native::identity(b.path()));
    assert_eq!(g(a.path(), &["config", "--get", "user.name"]), "Ada Lovelace", "trimmed");
    assert_eq!(
        g(a.path(), &["config", "--get", "user.email"]),
        g(b.path(), &["config", "--get", "user.email"])
    );
}

/// The remote guard is the provenance model: no audience while nobody real signs the vault.
#[test]
fn set_remote_agrees_including_its_refusals() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    git::ensure_repo(a.path()).unwrap();
    git_native::ensure_repo(b.path()).unwrap();
    no_identity(a.path());
    no_identity(b.path());

    // On the placeholder: both refuse, and neither leaves a remote behind.
    assert!(git::set_remote(a.path(), "https://example.org/x.git").is_err());
    assert!(git_native::set_remote(b.path(), "https://example.org/x.git").is_err());
    assert_eq!(git::remote(a.path()).unwrap(), None);
    assert_eq!(git_native::remote(b.path()).unwrap(), None);

    git::set_identity(a.path(), "Ada", "ada@example.org").unwrap();
    git_native::set_identity(b.path(), "Ada", "ada@example.org").unwrap();

    // An empty URL is refused by both — `git remote add origin ""` otherwise succeeds and the
    // vault ends up claiming a destination it does not have.
    assert!(git::set_remote(a.path(), "  ").is_err());
    assert!(git_native::set_remote(b.path(), "  ").is_err());

    // Set, then reset: idempotent, last URL wins, on both.
    for url in ["https://example.org/one.git", "https://example.org/two.git"] {
        git::set_remote(a.path(), url).unwrap();
        git_native::set_remote(b.path(), url).unwrap();
        assert_eq!(git::remote(a.path()).unwrap().as_deref(), Some(url));
        assert_eq!(git_native::remote(b.path()).unwrap(), git::remote(a.path()).unwrap());
        assert_eq!(g(a.path(), &["remote", "get-url", "origin"]), url);
        assert_eq!(g(b.path(), &["remote", "get-url", "origin"]), url);
    }
}

/// Committing: the return value, the resulting tree, and — the load-bearing one — that
/// **only the named paths are staged**.
///
/// A vault may also be a repo holding source or a manuscript. `add -A` in the five-second
/// debounce means committing a file the user is hand-editing in Vim, mid-sentence. The two
/// backends stage by completely different mechanisms, so this is exactly where they could
/// silently diverge.
#[test]
fn commit_all_agrees_on_what_is_committed_and_what_is_left_alone() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    for v in [a.path(), b.path()] {
        fs::create_dir_all(v.join("notes")).unwrap();
        fs::write(v.join("notes/01.md"), "ours\n").unwrap();
        // NOT ours: the user's own file, which neither backend may touch.
        fs::write(v.join("manuscript.tex"), "do not commit me\n").unwrap();
    }
    let ours = |v: &Path| vec![v.join("notes/01.md")];

    assert_eq!(
        git::commit_all(a.path(), "auto: 01", &ours(a.path())).unwrap(),
        git_native::commit_all(b.path(), "auto: 01", &ours(b.path())).unwrap(),
        "both report having committed"
    );

    // Read back with real git: same files, same message, same count.
    for v in [a.path(), b.path()] {
        let files = g(v, &["show", "--name-only", "--format=", "HEAD"]);
        assert!(files.contains("notes/01.md"), "the note is in: {files}");
        assert!(!files.contains("manuscript.tex"), "the user's file is NOT: {files}");
        assert_eq!(g(v, &["log", "-1", "--format=%s"]), "auto: 01");
        assert_eq!(g(v, &["log", "--oneline"]).lines().count(), 1);
    }
    assert_eq!(
        g(a.path(), &["show", "--name-only", "--format=", "HEAD"]),
        g(b.path(), &["show", "--name-only", "--format=", "HEAD"]),
        "the committed file sets are identical"
    );

    // A clean tree is not an error — it is the common case for a debounced auto-commit.
    assert_eq!(
        git::commit_all(a.path(), "auto: again", &ours(a.path())).unwrap(),
        git_native::commit_all(b.path(), "auto: again", &ours(b.path())).unwrap(),
        "both report 'nothing to commit'"
    );
    assert!(!git::commit_all(a.path(), "auto: again", &ours(a.path())).unwrap());
    for v in [a.path(), b.path()] {
        assert_eq!(g(v, &["log", "--oneline"]).lines().count(), 1, "no empty commit accreted");
    }
}

/// Cloning, offline, over `file://` — and both must make the clone a *vault*, not just a repo.
#[test]
fn clone_agrees_and_both_make_the_result_a_vault() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // A remote with one note, left as a collaborator would.
    let origin = tempdir().unwrap();
    fs::create_dir_all(origin.path().join("notes")).unwrap();
    fs::write(origin.path().join("notes/01.md"), "theirs\n").unwrap();
    git::ensure_repo(origin.path()).unwrap();
    git::commit_all(origin.path(), "theirs", &[origin.path().join("notes/01.md")]).unwrap();
    let url = origin.path().to_str().unwrap();

    let parent = tempdir().unwrap();
    let (da, db) = (parent.path().join("a"), parent.path().join("b"));
    git::clone(url, &da).unwrap();
    git_native::clone(url, &db).unwrap();

    for d in [&da, &db] {
        assert!(d.join("notes/01.md").exists(), "their work came with it");
        assert!(
            fs::read_to_string(d.join(".gitattributes")).unwrap().contains("merge=fm"),
            "a clone is not a vault until it has the merge attribute"
        );
    }
    assert_eq!(
        g(&da, &["log", "-1", "--format=%s"]),
        g(&db, &["log", "-1", "--format=%s"]),
        "same history arrived"
    );

    // And both refuse a non-empty destination, before writing anything.
    let occupied = tempdir().unwrap();
    fs::write(occupied.path().join("mine"), "keep\n").unwrap();
    assert!(git::clone(url, occupied.path()).is_err());
    assert!(git_native::clone(url, occupied.path()).is_err());
    assert!(!occupied.path().join(".git").exists(), "and neither wrote anything");
}

// The *pull* comparison lives in `crates/fm-cli/tests/git_native_merge.rs`, not here.
// It needs the `.md` merge driver installed, and `ensure_repo` points the driver at the `fm`
// binary beside the running one — which in `fm-core`'s test harness does not exist. Run here,
// the subprocess side silently falls back to git's plain text merge and conflicts on the
// `updated:` line, so the comparison would grade two broken things against each other.

/// **The squash, compared.** This is the most destructive operation either backend performs —
/// it rewrites local history on the bet that a push lands — so the two must agree not only on
/// the happy path but on every guard, and `git log` on the far side is the oracle for both.
///
/// Runs over a bare `file://` remote, offline: a real network is not needed to prove the
/// history is shaped identically, and a test that needed one would not run.
#[test]
fn push_squashed_agrees_on_what_it_collapses_and_what_it_refuses() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }

    // One bare remote per backend, so neither sees the other's pushes.
    let setup = |name: &str| {
        let bare = tempdir().unwrap();
        Command::new("git")
            .args(["init", "--bare", bare.path().to_str().unwrap()])
            .output()
            .unwrap();
        let work = tempdir().unwrap();
        fs::create_dir_all(work.path().join("notes")).unwrap();
        git::ensure_repo(work.path()).unwrap();
        git::set_identity(work.path(), name, "who@example.org").unwrap();
        git::set_remote(work.path(), bare.path().to_str().unwrap()).unwrap();
        (bare, work)
    };

    // Three `auto:` commits, exactly what a few seconds of typing produces.
    // `round` is load-bearing: rewriting identical bytes stages nothing, `commit_all` correctly
    // returns false, and the second window would have had no commits to squash at all — the
    // test would then pass for the wrong reason on a backend that squashed nothing.
    let churn = |vault: &std::path::Path,
                 commit: fn(&std::path::Path, &str, &[std::path::PathBuf]) -> Result<bool, fm_core::StoreError>,
                 round: u32| {
        for i in 0..3 {
            let f = vault.join(format!("notes/r{round}n{i}.md"));
            fs::write(&f, format!("note {round}.{i}\n")).unwrap();
            assert!(commit(vault, &format!("auto: {round}.{i}"), &[f]).unwrap(), "a real commit");
        }
    };

    let (bare_a, a) = setup("Sub Process");
    let (bare_b, b) = setup("Lib Git2");
    churn(a.path(), git::commit_all, 1);
    churn(b.path(), git_native::commit_all, 1);

    // First push: NEVER squashed. "Unpushed" here means the entire history, and destroying
    // history that has never left the machine is exactly backwards.
    assert_eq!(git::push_squashed(a.path(), "backup: one").unwrap(), 0, "subprocess first push");
    assert_eq!(git_native::push_squashed(b.path(), "backup: one").unwrap(), 0, "libgit2 first push");
    let subjects = |bare: &tempfile::TempDir| {
        String::from_utf8(
            Command::new("git")
                .args(["-C", bare.path().to_str().unwrap(), "log", "--format=%s"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
    };
    assert_eq!(subjects(&bare_a), subjects(&bare_b), "the first push sends history whole, both");
    assert_eq!(subjects(&bare_a).lines().count(), 3, "all three commits, not one");

    // Second window: three more `auto:` commits, which SHOULD collapse to one.
    churn(a.path(), git::commit_all, 2);
    churn(b.path(), git_native::commit_all, 2);
    assert_eq!(git::push_squashed(a.path(), "backup: two").unwrap(), 3, "subprocess squashed 3");
    assert_eq!(git_native::push_squashed(b.path(), "backup: two").unwrap(), 3, "libgit2 squashed 3");
    assert_eq!(subjects(&bare_a), subjects(&bare_b), "identical remote history after a squash");
    assert_eq!(
        subjects(&bare_a).lines().next().unwrap(),
        "backup: two",
        "the window became one commit"
    );
    assert_eq!(subjects(&bare_a).lines().count(), 4, "3 original + 1 squashed");
}

/// **A hand-written commit is a floor the squash must not go below.** The justification for
/// collapsing at all is that the app auto-commits every few seconds — that justifies collapsing
/// *ours*, never three manuscript commits the user wrote in a vault that is also a project repo.
///
/// Discriminated by message prefix, never by author: we commit *as* the user, so an author test
/// would classify everything as ours. Both backends must apply the identical rule.
#[test]
fn a_hand_written_commit_stops_the_squash_on_both_backends() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let setup = || {
        let bare = tempdir().unwrap();
        Command::new("git")
            .args(["init", "--bare", bare.path().to_str().unwrap()])
            .output()
            .unwrap();
        let work = tempdir().unwrap();
        fs::create_dir_all(work.path().join("notes")).unwrap();
        git::ensure_repo(work.path()).unwrap();
        git::set_identity(work.path(), "Ada", "ada@example.org").unwrap();
        git::set_remote(work.path(), bare.path().to_str().unwrap()).unwrap();
        fs::write(work.path().join("notes/seed.md"), "seed\n").unwrap();
        git::commit_all(work.path(), "auto: seed", &[work.path().join("notes/seed.md")]).unwrap();
        (bare, work)
    };

    for backend in ["subprocess", "native"] {
        let (bare, w) = setup();
        let commit = if backend == "native" { git_native::commit_all } else { git::commit_all };
        let push = if backend == "native" { git_native::push_squashed } else { git::push_squashed };
        push(w.path(), "backup: first").unwrap();

        // auto, auto, THEIR OWN commit, auto, auto.
        for (i, msg) in [(0, "auto: a"), (1, "auto: b"), (2, "Chapter 3: the ants"), (3, "auto: c"), (4, "auto: d")] {
            let f = w.path().join(format!("notes/1{i}.md"));
            fs::write(&f, format!("{i}\n")).unwrap();
            commit(w.path(), msg, &[f]).unwrap();
        }

        let squashed = push(w.path(), "backup: second").unwrap();
        let log = String::from_utf8(
            Command::new("git")
                .args(["-C", bare.path().to_str().unwrap(), "log", "--format=%s"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();

        assert_eq!(squashed, 2, "{backend}: only the two commits ABOVE the manuscript one");
        assert!(
            log.contains("Chapter 3: the ants"),
            "{backend}: the user's own commit must survive verbatim:\n{log}"
        );
        assert!(log.contains("auto: a") && log.contains("auto: b"),
            "{backend}: commits below the floor are untouched:\n{log}");
    }
}

/// **The trust-store load must not crash, and must actually load.**
///
/// `git_openssl__add_x509_cert` reads `SSL_CTX_get_cert_store(git__ssl_ctx)` after an
/// `openssl_ensure_initialized()` that creates nothing in a non-`GIT_OPENSSL_DYNAMIC` build. Call
/// it before `git_libgit2_init()` has run and it dereferences NULL — a launch crash rather than an
/// error, which is exactly what shipping it unguarded produced on a device.
///
/// This runs the real path against real certificates, so a regression is a failed test rather
/// than a phone that dies on open.
#[cfg(feature = "native-git")]
#[test]
fn adding_certificates_from_memory_initialises_libgit2_first() {
    // A tiny bundle is enough: the crash was in reaching the store at all, not in the count.
    let origin = tempdir().unwrap();
    git::ensure_repo(origin.path()).unwrap();

    // Two real roots, in the shape the Android store hands us — PEM block plus trailing text.
    let pem = std::process::Command::new("openssl")
        .args(["s_client", "-showcerts", "-connect", "example.com:443"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());

    // Offline or no openssl: fall back to asserting the guard rather than skipping entirely —
    // an empty bundle must be refused, and refusing must not crash.
    let Some(pem) = pem.filter(|p| p.contains("BEGIN CERTIFICATE")) else {
        assert!(git_native::add_certs_from_pem(b"").is_err(), "empty is refused, not fatal");
        assert!(
            git_native::add_certs_from_pem(b"not a certificate\n").is_err(),
            "garbage is refused, not fatal"
        );
        return;
    };

    let added = git_native::add_certs_from_pem(pem.as_bytes())
        .expect("certificates from memory must load");
    assert!(added > 0, "at least one certificate reached libgit2's store");
}

/// **The credentials callback must be attached to clone**, which is the one network call that
/// shipped without it.
///
/// `Repository::clone` builds its own default fetch options carrying no callbacks, so a private
/// remote fails with libgit2's "remote authentication required but no callback set" — a message
/// that reads like a missing token even when one is configured. The `file://` clone test above
/// cannot catch this, because a local path never authenticates.
///
/// This asks a real private URL **without** a token. The point is not that it succeeds — it must
/// not — but *which* failure comes back: an authentication refusal means the callback ran and had
/// nothing to offer, whereas "no callback set" means the plumbing is missing again.
#[cfg(feature = "native-git")]
#[test]
fn clone_offers_credentials_rather_than_failing_for_want_of_a_callback() {
    // No token, deliberately: this asserts the shape of the refusal, not access.
    std::env::remove_var("FM_GIT_TOKEN");
    // Bound to a local: `tempdir().unwrap().path()` drops the TempDir at the end of the
    // statement and deletes the directory, so the clone would start with no parent.
    let parent = tempdir().unwrap();
    let dest = parent.path().join("clone-attempt");

    let Err(e) = git_native::clone("https://github.com/singhbal-baljinder/personal-notes.git", &dest)
    else {
        panic!("a private repo must not clone without credentials");
    };
    let msg = format!("{e}").to_lowercase();

    // Offline is not a failure of this test — it is a failure to run it.
    if msg.contains("resolve") || msg.contains("could not connect") || msg.contains("timed out") {
        eprintln!("skipping: no network ({msg})");
        return;
    }
    assert!(
        !msg.contains("no callback set"),
        "clone must attach the credentials callback; got: {msg}"
    );
    // The callback ran and had nothing to offer, and says so in those terms. libgit2's own
    // wording for this case is "no callback set", which is what the assertion above rejects:
    // it describes missing plumbing and sends you to debug the wrong layer entirely.
    assert!(
        msg.contains("needs an access token"),
        "expected the honest 'no token configured' message, got: {msg}"
    );
}
