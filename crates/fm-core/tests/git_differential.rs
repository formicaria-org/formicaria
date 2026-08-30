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
/// Not on Windows: the memory path does not exist there, because libgit2 speaks WinHTTP and uses
/// the machine's own certificate store. Gated to match the function, not to skip a platform we
/// simply failed to build for — see `Cargo.toml`'s `cfg(not(windows))` dependency note.
#[cfg(all(feature = "native-git", not(windows)))]
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

// ===========================================================================
// The proposal lifecycle.
//
// Ported to libgit2 on 2026-07-24 because it was reachable only through `crate::git`, so on a
// phone — no `git` binary — creating, reviewing, accepting or rejecting a proposal all failed.
// A proposal is the only way a UI-only user merges anything, so the device that needed it most
// had none of it.
// ===========================================================================

/// A realistic note: frontmatter the merge engine treats structurally, and a body it merges by
/// line. The `updated:` field is the one that collides on *every* concurrent edit, which is the
/// entire reason the `merge=fm` driver exists.
/// It has to be a *parseable* note: the engine resolves `updated` structurally only when all
/// three sides parse, and falls back to a line-based 3-way when they do not — which is exactly
/// how a fixture that is merely note-shaped hides the behaviour under test.
const NOTE: &str = "---\nid: 01JQ0000000000000000000000\ntype: note\ntitle: t\n\
                    created: 2026-07-17T10:00:00Z\nupdated: 2026-07-24T10:00:00Z\n---\n\n\
                    alpha\nbravo\ncharlie\ndelta\necho\n";

/// A vault with one committed note, on the placeholder identity.
fn seed(vault: &Path, rel: &str, body: &str) {
    if !vault.join(".git").exists() {
        git::ensure_repo(vault).unwrap();
        no_identity(vault);
    }
    let path = vault.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    git::commit_all(vault, "seed", &[path]).unwrap();
}

/// Create → open? → read → diff → load → revise → accept, run through both backends over the
/// same starting vault, with **real git** as the oracle for the resulting trees.
#[test]
fn the_proposal_lifecycle_agrees_across_backends() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    seed(a.path(), "notes/n1.md", NOTE);
    seed(b.path(), "notes/n1.md", NOTE);

    let br = "proposal/p1";
    let proposed = NOTE.replace("bravo", "bravo revised");
    git::create_proposal_branch(a.path(), br, "notes/n1.md", &proposed, "propose", None).unwrap();
    git_native::create_proposal_branch(b.path(), br, "notes/n1.md", &proposed, "propose", None)
        .unwrap();

    // The tree is the guarantee — not the patch text, which each backend renders its own way.
    assert_eq!(
        g(a.path(), &["rev-parse", "proposal/p1^{tree}"]),
        g(b.path(), &["rev-parse", "proposal/p1^{tree}"]),
        "the proposed tree must be identical"
    );
    // And the working tree is untouched by a proposal, on both.
    assert_eq!(fs::read_to_string(a.path().join("notes/n1.md")).unwrap(), NOTE);
    assert_eq!(fs::read_to_string(b.path().join("notes/n1.md")).unwrap(), NOTE);

    assert!(git::branch_open(a.path(), br) && git_native::branch_open(b.path(), br));
    assert_eq!(
        git::file_on_branch(a.path(), br, "notes/n1.md"),
        git_native::file_on_branch(b.path(), br, "notes/n1.md"),
    );
    assert_eq!(git_native::file_on_branch(b.path(), br, "notes/n1.md").as_deref(), Some(&*proposed));

    let (ea, fa, pa) = git::branch_diff(a.path(), br).unwrap();
    let (eb, fb, pb) = git_native::branch_diff(b.path(), br).unwrap();
    assert_eq!((ea, &fa), (eb, &fb), "existence and file list are exact");
    assert_eq!(fa, vec!["notes/n1.md".to_string()]);
    // The patch text is *declared* asymmetric (libgit2 renders its own). Assert it says the
    // right thing, not that it says it in the same bytes.
    for p in [&pa, &pb] {
        assert!(p.contains("notes/n1.md") && p.contains("+bravo revised"), "patch: {p}");
    }
    assert_eq!(
        git::proposal_load(a.path()).unwrap(),
        git_native::proposal_load(b.path()).unwrap(),
        "open count and byte weight feed the vault guardrails"
    );

    // Revise moves the ref atomically; the branch must still be the one branch.
    let revised = NOTE.replace("bravo", "bravo twice");
    git::revise_proposal_branch(a.path(), br, "notes/n1.md", &revised, "revise", None).unwrap();
    git_native::revise_proposal_branch(b.path(), br, "notes/n1.md", &revised, "revise", None)
        .unwrap();
    assert_eq!(
        g(a.path(), &["rev-parse", "proposal/p1^{tree}"]),
        g(b.path(), &["rev-parse", "proposal/p1^{tree}"]),
    );

    assert_eq!(
        git::merge_proposal_branch(a.path(), br).unwrap(),
        git_native::merge_proposal_branch(b.path(), br).unwrap(),
    );
    assert_eq!(
        g(a.path(), &["rev-parse", "HEAD^{tree}"]),
        g(b.path(), &["rev-parse", "HEAD^{tree}"]),
        "accepting must land the same tree"
    );
    assert_eq!(fs::read_to_string(b.path().join("notes/n1.md")).unwrap(), revised);
    assert!(!git::branch_open(a.path(), br) && !git_native::branch_open(b.path(), br));
}

/// Creating a proposal must refuse a name that already exists, and refuse a vault with no
/// commits — identically.
#[test]
fn creating_a_proposal_agrees_on_its_refusals() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    // No commits yet: nothing to propose a change against.
    git::ensure_repo(a.path()).unwrap();
    git_native::ensure_repo(b.path()).unwrap();
    no_identity(a.path());
    no_identity(b.path());
    assert!(git::create_proposal_branch(a.path(), "proposal/p", "n.md", "x", "m", None).is_err());
    assert!(
        git_native::create_proposal_branch(b.path(), "proposal/p", "n.md", "x", "m", None).is_err()
    );

    seed(a.path(), "notes/n1.md", NOTE);
    seed(b.path(), "notes/n1.md", NOTE);
    for _ in 0..1 {
        git::create_proposal_branch(a.path(), "proposal/p", "notes/n1.md", "x", "m", None).unwrap();
        git_native::create_proposal_branch(b.path(), "proposal/p", "notes/n1.md", "x", "m", None)
            .unwrap();
    }
    // A proposal owns a fresh-ULID branch, so a collision is a bug, not a race.
    assert!(git::create_proposal_branch(a.path(), "proposal/p", "notes/n1.md", "y", "m", None)
        .is_err());
    assert!(git_native::create_proposal_branch(
        b.path(),
        "proposal/p",
        "notes/n1.md",
        "y",
        "m",
        None
    )
    .is_err());
}

/// Rejecting is deleting the branch, and it must be idempotent on both.
#[test]
fn rejecting_a_proposal_agrees_and_is_idempotent() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    seed(a.path(), "notes/n1.md", NOTE);
    seed(b.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    let proposed = NOTE.replace("bravo", "nope");
    git::create_proposal_branch(a.path(), br, "notes/n1.md", &proposed, "m", None).unwrap();
    git_native::create_proposal_branch(b.path(), br, "notes/n1.md", &proposed, "m", None).unwrap();

    git::delete_branch(a.path(), br).unwrap();
    git_native::delete_branch(b.path(), br).unwrap();
    assert!(!git::branch_open(a.path(), br) && !git_native::branch_open(b.path(), br));
    // Rejected twice, or only ever existing on another clone: success, not an error.
    git::delete_branch(a.path(), br).unwrap();
    git_native::delete_branch(b.path(), br).unwrap();
    // `main` is untouched — a proposal is only ever an off-main branch.
    assert_eq!(fs::read_to_string(b.path().join("notes/n1.md")).unwrap(), NOTE);
}

/// A revise force-moves the proposal ref and a reject deletes it — the two paths that used to
/// destroy the model's text. Both backends must keep the outgoing commit, under the same name, and
/// the record must stay invisible to every walk that already exists.
///
/// This is behavioural, so the route-parity grep in `ci/checks.sh` cannot see it: `retain_proposal_tip`
/// is internal to each backend, and two implementations that disagree here collect different corpora
/// on the laptop and the phone.
#[test]
fn a_revised_or_rejected_proposal_keeps_its_outgoing_commit_on_both() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    seed(a.path(), "notes/n1.md", NOTE);
    seed(b.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    let first = NOTE.replace("bravo", "first draft");
    git::create_proposal_branch(a.path(), br, "notes/n1.md", &first, "propose: m", None).unwrap();
    git_native::create_proposal_branch(b.path(), br, "notes/n1.md", &first, "propose: m", None)
        .unwrap();

    let tip = |v: &Path| g(v, &["rev-parse", &format!("refs/heads/{br}")]);
    let (a1, b1) = (tip(a.path()), tip(b.path()));

    // REVISE: the ref moves onto a commit re-parented on HEAD. Without retention the first draft
    // is orphaned here, which is exactly how the owner's vault lost ten commits.
    let second = NOTE.replace("bravo", "second draft");
    git::revise_proposal_branch(a.path(), br, "notes/n1.md", &second, "revise: m", None).unwrap();
    git_native::revise_proposal_branch(b.path(), br, "notes/n1.md", &second, "revise: m", None)
        .unwrap();

    assert_eq!(g(a.path(), &["rev-parse", &format!("refs/fm/review/p1/{a1}")]), a1);
    assert_eq!(g(b.path(), &["rev-parse", &format!("refs/fm/review/p1/{b1}")]), b1);
    // The point is the CONTENT, not the ref: the superseded draft is still readable.
    assert!(g(a.path(), &["show", &format!("{a1}:notes/n1.md")]).contains("first draft"));
    assert!(g(b.path(), &["show", &format!("{b1}:notes/n1.md")]).contains("first draft"));

    // REJECT: the branch goes; its final tip is then the only surviving copy.
    let (a2, b2) = (tip(a.path()), tip(b.path()));
    git::delete_branch(a.path(), br).unwrap();
    git_native::delete_branch(b.path(), br).unwrap();
    assert_eq!(g(a.path(), &["rev-parse", &format!("refs/fm/review/p1/{a2}")]), a2);
    assert_eq!(g(b.path(), &["rev-parse", &format!("refs/fm/review/p1/{b2}")]), b2);
    assert!(g(a.path(), &["show", &format!("{a2}:notes/n1.md")]).contains("second draft"));
    assert!(g(b.path(), &["show", &format!("{b2}:notes/n1.md")]).contains("second draft"));

    // Invisible where it must be. A retained record that changed any of these would be a
    // regression in the app, not a feature: no branch, nothing in HEAD's history, and — the one
    // that would actually brick the product — no guardrail slot consumed.
    for (v, keep) in [(a.path(), &a2), (b.path(), &b2)] {
        assert_eq!(g(v, &["for-each-ref", "--format=%(refname)", "refs/heads/proposal/"]), "");
        assert!(!g(v, &["log", "--format=%H"]).contains(keep.as_str()));
    }
    assert_eq!(git::proposal_load(a.path()).unwrap().0, 0);
    assert_eq!(git_native::proposal_load(b.path()).unwrap().0, 0);
}

/// Retiring a settled proposal frees its guardrail slot without touching the remote, and both
/// backends must agree — `ci/checks.sh` proves the route exists on each, never that they behave the
/// same, and `decisions.md` records the two once having opposite bugs on one path.
#[test]
fn retiring_a_settled_proposal_agrees_and_keeps_the_commit() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let (a, b) = pair();
    seed(a.path(), "notes/n1.md", NOTE);
    seed(b.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    let proposed = NOTE.replace("bravo", "settled");
    git::create_proposal_branch(a.path(), br, "notes/n1.md", &proposed, "propose: m", None).unwrap();
    git_native::create_proposal_branch(b.path(), br, "notes/n1.md", &proposed, "propose: m", None)
        .unwrap();
    let tip = |v: &Path| g(v, &["rev-parse", &format!("refs/heads/{br}")]);
    let (a1, b1) = (tip(a.path()), tip(b.path()));
    assert_eq!(git::proposal_load(a.path()).unwrap().0, 1);
    assert_eq!(git_native::proposal_load(b.path()).unwrap().0, 1);

    git::retire_proposal(a.path(), br).unwrap();
    git_native::retire_proposal(b.path(), br).unwrap();

    // The slot is released on both...
    assert_eq!(git::proposal_load(a.path()).unwrap().0, 0);
    assert_eq!(git_native::proposal_load(b.path()).unwrap().0, 0);
    assert!(!git::branch_open(a.path(), br) && !git_native::branch_open(b.path(), br));

    // ...and the text is kept on both — retiring joins the unlabelled pool, it does not drop it.
    assert!(g(a.path(), &["show", &format!("{a1}:notes/n1.md")]).contains("settled"));
    assert!(g(b.path(), &["show", &format!("{b1}:notes/n1.md")]).contains("settled"));
    assert_eq!(g(a.path(), &["rev-parse", &format!("refs/fm/review/p1/{a1}")]), a1);
    assert_eq!(g(b.path(), &["rev-parse", &format!("refs/fm/review/p1/{b1}")]), b1);

    // Idempotent: retiring twice is success, exactly as rejecting twice is.
    git::retire_proposal(a.path(), br).unwrap();
    git_native::retire_proposal(b.path(), br).unwrap();
}

// ---------------------------------------------------------------------------
// Native-only: the accept path's safety properties.
//
// These are not differential — the subprocess backend needs the `fm` binary installed as a merge
// driver to behave this way, and a test vault has none. They pin the properties that the *first*
// draft of this port got wrong, every one of which costs the user a note.
// ---------------------------------------------------------------------------

/// **libgit2 never runs the `merge=fm` driver** — it resolves no named drivers and silently falls
/// back to a line-based text merge. So the manufactured `updated:` collision, which happens on
/// *every* concurrent edit, would make essentially every accept report a conflict. The resolution
/// through `merged_text` is what prevents that, and this is the assertion a naive port fails.
#[test]
fn accepting_resolves_the_updated_collision_that_libgit2_alone_would_conflict_on() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";

    let proposed = NOTE.replace("echo", "echo revised").replace("updated: 2026-07-24T10", "updated: 2026-07-24T11");
    git_native::create_proposal_branch(v.path(), br, "notes/n1.md", &proposed, "m", None).unwrap();

    // Meanwhile main moves on, touching a different line *and* the same `updated:` field.
    let moved = NOTE.replace("alpha", "alpha edited").replace("updated: 2026-07-24T10", "updated: 2026-07-24T12");
    fs::write(v.path().join("notes/n1.md"), &moved).unwrap();
    git_native::commit_all(v.path(), "edit", &[v.path().join("notes/n1.md")]).unwrap();

    assert_eq!(
        git_native::merge_proposal_branch(v.path(), br).unwrap(),
        git::Accepted::Merged,
        "the frontmatter collision must not surface as a conflict"
    );
    let after = fs::read_to_string(v.path().join("notes/n1.md")).unwrap();
    assert!(!after.contains("<<<<<<<"), "no markers: {after}");
    assert!(after.contains("alpha edited") && after.contains("echo revised"), "{after}");
}

/// A genuine disagreement fails **closed**: nothing on disk moves, `HEAD` does not move, and the
/// repository is not left mid-merge. The user's escape hatch is to edit the proposed body and
/// save, which revises the proposal onto current `main` — and that only works if accepting a
/// conflicted proposal changed nothing.
#[test]
fn a_real_conflict_leaves_the_vault_exactly_as_it_was() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    git_native::create_proposal_branch(
        v.path(),
        br,
        "notes/n1.md",
        &NOTE.replace("bravo", "their side"),
        "m",
        None,
    )
    .unwrap();

    let mine = NOTE.replace("bravo", "my side");
    fs::write(v.path().join("notes/n1.md"), &mine).unwrap();
    git_native::commit_all(v.path(), "edit", &[v.path().join("notes/n1.md")]).unwrap();
    let head_before = g(v.path(), &["rev-parse", "HEAD"]);

    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Conflicted);
    assert_eq!(g(v.path(), &["rev-parse", "HEAD"]), head_before, "HEAD must not move");
    assert_eq!(fs::read_to_string(v.path().join("notes/n1.md")).unwrap(), mine);
    assert_eq!(g(v.path(), &["status", "--porcelain"]), "", "and no half-merge is left behind");
    assert!(git_native::conflicts(v.path()).unwrap().is_empty());
    assert!(git_native::branch_open(v.path(), br), "the proposal is still there to revise");
}

/// **The one that fires on an unrelated proposal.** A whole-tree `checkout_head(force)` — the
/// obvious way to write this — reverts every tracked file in the vault that differs from `HEAD`,
/// so a user mid-edit in a note they are not proposing anything about would silently lose it by
/// tapping Accept. The checkout is scoped to the merged paths precisely so this cannot happen.
#[test]
fn accepting_does_not_touch_a_dirty_file_the_proposal_never_mentions() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    seed(v.path(), "notes/n2.md", NOTE.replace("01JQ0000", "01JQ1111").as_str());

    let br = "proposal/p1";
    git_native::create_proposal_branch(
        v.path(),
        br,
        "notes/n1.md",
        &NOTE.replace("bravo", "bravo revised"),
        "m",
        None,
    )
    .unwrap();

    // The user is mid-edit in n2; the debounced auto-commit has not fired yet.
    let in_progress = NOTE.replace("01JQ0000", "01JQ1111").replace("bravo", "half a thought");
    fs::write(v.path().join("notes/n2.md"), &in_progress).unwrap();

    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Merged);
    assert_eq!(
        fs::read_to_string(v.path().join("notes/n2.md")).unwrap(),
        in_progress,
        "the unrelated in-progress note must survive being accepted around"
    );
}

/// Uncommitted work **in a path the merge writes** is refused rather than overwritten — what
/// `git merge` does with "local changes would be overwritten by merge".
#[test]
fn accepting_refuses_rather_than_overwrite_uncommitted_work_in_a_merged_path() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    git_native::create_proposal_branch(
        v.path(),
        br,
        "notes/n1.md",
        &NOTE.replace("bravo", "bravo revised"),
        "m",
        None,
    )
    .unwrap();

    let unsaved = NOTE.replace("bravo", "still typing");
    fs::write(v.path().join("notes/n1.md"), &unsaved).unwrap();

    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Conflicted);
    assert_eq!(fs::read_to_string(v.path().join("notes/n1.md")).unwrap(), unsaved);
}

/// An **untracked** file where the proposal adds one is refused too — git's "untracked working
/// tree files would be overwritten". This is the case the pre-proposal snapshot at
/// `commands.rs:666` normally prevents, and that call was itself failing on the phone.
#[test]
fn accepting_refuses_to_clobber_an_untracked_file_the_proposal_adds() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    git_native::create_proposal_branch(v.path(), br, "notes/n2.md", "proposed n2\n", "m", None)
        .unwrap();

    let theirs = "a captured note nobody committed yet\n";
    fs::write(v.path().join("notes/n2.md"), theirs).unwrap();

    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Conflicted);
    assert_eq!(fs::read_to_string(v.path().join("notes/n2.md")).unwrap(), theirs);
}

/// Accepting **on top of an unfinished pull** must refuse. A native pull deliberately leaves
/// `MERGE_HEAD` and a conflicted index for a human; merging over it would drop the remote's
/// commits from the graph, erase the markers being resolved, and leave a conflicted index that
/// blocks every later `commit_all` in the vault.
#[test]
fn accepting_refuses_while_a_merge_is_still_unfinished() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    git_native::create_proposal_branch(
        v.path(),
        br,
        "notes/n1.md",
        &NOTE.replace("bravo", "bravo revised"),
        "m",
        None,
    )
    .unwrap();

    let head_before = g(v.path(), &["rev-parse", "HEAD"]);
    fs::write(v.path().join(".git/MERGE_HEAD"), format!("{head_before}\n")).unwrap();

    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Conflicted);
    assert_eq!(g(v.path(), &["rev-parse", "HEAD"]), head_before);
    assert!(git_native::branch_open(v.path(), br));
}

/// Accepting something already in `main` creates **no** commit — exec prints "Already up to
/// date." A retried accept (the branch delete never reached the remote, say) must not accrete an
/// empty merge commit each time, and must not re-run the checkout.
#[test]
fn accepting_an_already_merged_proposal_adds_no_commit() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let v = tempdir().unwrap();
    seed(v.path(), "notes/n1.md", NOTE);
    let br = "proposal/p1";
    git_native::create_proposal_branch(
        v.path(),
        br,
        "notes/n1.md",
        &NOTE.replace("bravo", "bravo revised"),
        "m",
        None,
    )
    .unwrap();
    let tip = g(v.path(), &["rev-parse", br]);
    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Merged);

    // The branch survives (an offline delete, or another clone still holding it).
    Command::new("git").arg("-C").arg(v.path()).args(["branch", br, &tip]).output().unwrap();
    let count = g(v.path(), &["rev-list", "--count", "HEAD"]);

    assert_eq!(git_native::merge_proposal_branch(v.path(), br).unwrap(), git::Accepted::Merged);
    assert_eq!(g(v.path(), &["rev-list", "--count", "HEAD"]), count, "no empty merge commit");
    assert!(!git_native::branch_open(v.path(), br), "and it is closed out");
}
