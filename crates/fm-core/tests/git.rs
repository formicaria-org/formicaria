//! Vault versioning via git — the notes' durable, pushable history. Hermetic: a
//! tempdir vault, a repo-local identity written by `ensure_repo`, no network.
//! Skipped (not failed) when git is absent.

use fm_core::git;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn ensure_repo_initializes_once_and_writes_gitignore() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();

    assert!(git::ensure_repo(vault.path()).unwrap(), "repo created on first run");
    assert!(vault.path().join(".git").exists(), ".git directory present");

    let ignore = fs::read_to_string(vault.path().join(".gitignore")).unwrap();
    assert!(ignore.contains("index.sqlite"), "disposable index ignored");
    assert!(ignore.contains("derived/"), "regenerable thumbnails ignored");
    assert!(ignore.contains("blobs/"), "heavy blobs ignored (they sync out-of-band)");

    assert!(!git::ensure_repo(vault.path()).unwrap(), "already a repo on the second run");
}

#[test]
fn commit_all_commits_changes_then_reports_a_clean_tree() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let notes = vault.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("01.md"), "the durable knowledge\n").unwrap();

    assert!(git::commit_all(vault.path(), "first snapshot").unwrap(), "committed the new note");

    let log = Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["log", "--oneline"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&log.stdout).lines().count(),
        1,
        "exactly one commit in the history"
    );

    // A clean tree is not an error — it is the common case for a debounced
    // auto-commit and must report "nothing to commit" as `false`.
    assert!(!git::commit_all(vault.path(), "no-op").unwrap(), "clean tree → nothing to commit");
}

/// Commit a note, so each call adds exactly one commit to the vault's history.
fn write_and_commit(vault: &std::path::Path, name: &str, body: &str) {
    let notes = vault.join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join(name), body).unwrap();
    assert!(git::commit_all(vault, &format!("auto: {name}")).unwrap());
}

fn log_count(repo: &std::path::Path) -> usize {
    let out =
        Command::new("git").arg("-C").arg(repo).args(["log", "--oneline"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().count()
}

/// Answer the question `set_remote` asks. A vault only gains a remote once someone
/// real is attached to it, so every push test below has to be somebody.
fn identify(vault: &std::path::Path) {
    git::set_identity(vault, "Ravi Test", "ravi@example.org").unwrap();
}

/// The state `ensure_repo` leaves a vault in on a machine with no git config at
/// all: committing works — the notes are the point — but on the placeholder rather
/// than a person.
///
/// Written repo-locally and explicitly, because local beats global: on a machine
/// that *does* have a `~/.gitconfig` (every developer's), `ensure_identity` finds it
/// and writes no placeholder, so leaving this implicit would silently test the
/// developer's own identity and prove nothing.
fn no_identity(vault: &std::path::Path) {
    git::ensure_repo(vault).unwrap();
    for (k, v) in [("user.email", "formicaria@localhost"), ("user.name", "formicaria")] {
        Command::new("git").arg("-C").arg(vault).args(["config", k, v]).output().unwrap();
    }
}

#[test]
fn a_repo_we_did_not_create_still_gets_the_ignore_rules() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    // A vault someone `git init`ed by hand. Without the ignore rules the next
    // `add -A` would sweep blobs/ and the index into history, and a push would
    // ship every PDF to the remote — the light tier would silently be heavy.
    let vault = tempdir().unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg(vault.path())
        .output()
        .unwrap()
        .status
        .success());

    assert!(!git::ensure_repo(vault.path()).unwrap(), "already a repo — we did not create it");

    let ignore = fs::read_to_string(vault.path().join(".gitignore")).unwrap();
    assert!(ignore.contains("blobs/"), "heavy blobs ignored even in a hand-made repo");
    assert!(ignore.contains("index.sqlite"), "per-machine index ignored");
}

#[test]
fn a_rejected_push_restores_the_history_it_squashed() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();

    // Our vault, with one push behind it so a tracking ref exists.
    let vault = tempdir().unwrap();
    write_and_commit(vault.path(), "01.md", "one\n");
    identify(vault.path());
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(vault.path(), "backup: first").unwrap();

    // Another machine pushes, so the real remote is now ahead of our stale
    // tracking ref — the divergence this whole design fails safe on.
    let other = tempdir().unwrap();
    Command::new("git").arg("clone").arg(bare.path()).arg(other.path()).output().unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["config", "user.email", "t@t"]).output().unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["config", "user.name", "t"]).output().unwrap();
    fs::write(other.path().join("theirs.md"), "from elsewhere\n").unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["add", "-A"]).output().unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["commit", "-m", "theirs"]).output().unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["push", "origin", "HEAD"]).output().unwrap();

    // Two commits here, so the push squashes before it tries — and gets rejected.
    write_and_commit(vault.path(), "02.md", "two\n");
    write_and_commit(vault.path(), "03.md", "three\n");
    let before = log_count(vault.path());

    assert!(git::push_squashed(vault.path(), "backup: second").is_err(), "diverged → rejected");

    // The squash is a bet on the push landing. It didn't, so the granular history
    // must be back: charging the user their undo for a backup that never happened
    // is the worst of both outcomes.
    assert_eq!(log_count(vault.path()), before, "history restored, not left collapsed");
    let files = Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .unwrap();
    let files = String::from_utf8_lossy(&files.stdout);
    assert!(files.contains("notes/03.md"), "and the work itself survived: {files}");
}

/// The landmine Phase 1's `pull` would have armed: once anything fetches, the
/// tracking ref holds a collaborator's tip, and squashing onto it would commit our
/// tree with their commit as parent — deleting their work in a push that
/// fast-forwards cleanly, so nothing rejects it. A count of unpushed commits reads
/// as "normal" in exactly that case; ancestry is the real question.
#[test]
fn a_fetched_remote_that_moved_refuses_the_squash_instead_of_eating_it() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();

    let vault = tempdir().unwrap();
    write_and_commit(vault.path(), "01.md", "one\n");
    identify(vault.path());
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(vault.path(), "backup: first").unwrap();

    // A collaborator pushes work we do not have.
    let other = tempdir().unwrap();
    Command::new("git").arg("clone").arg(bare.path()).arg(other.path()).output().unwrap();
    for (k, v) in [("user.email", "t@t"), ("user.name", "t")] {
        Command::new("git").arg("-C").arg(other.path()).args(["config", k, v]).output().unwrap();
    }
    fs::write(other.path().join("precious.md"), "their unpublished work\n").unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["add", "-A"]).output().unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["commit", "-m", "theirs"]).output().unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["push", "origin", "HEAD"]).output().unwrap();

    // *** The fetch. *** Our tracking ref now points at their tip, so it is no
    // longer an ancestor of ours — this is what Phase 1's poll/pull will do.
    Command::new("git").arg("-C").arg(vault.path()).arg("fetch").output().unwrap();

    write_and_commit(vault.path(), "02.md", "two\n");
    write_and_commit(vault.path(), "03.md", "three\n");

    let err = git::push_squashed(vault.path(), "backup: second").unwrap_err().to_string();
    assert!(err.contains("pull first"), "refused with a message that says what to do: {err}");

    // Their commit must still be the remote's tip: unreached, unrewritten.
    let head = Command::new("git")
        .arg("-C")
        .arg(bare.path())
        .args(["log", "-1", "--pretty=%s"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&head.stdout).trim(), "theirs", "their work survived");
}

/// The auto-commit fires 5s after any write, so a conflicted pull reaches it within
/// seconds. `add -A` would stage the markers — which git reads as "resolved" — and
/// commit `<<<<<<<` as the note's content, then push it.
#[test]
fn auto_commit_refuses_to_enshrine_conflict_markers() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    write_and_commit(vault.path(), "01.md", "shared line\n");

    let g = |args: &[&str]| {
        Command::new("git").arg("-C").arg(vault.path()).args(args).output().unwrap()
    };
    // Two branches touching the same line — the shape every concurrent note edit
    // takes, since `updated:` is rewritten on every save.
    g(&["checkout", "-b", "theirs"]);
    fs::write(vault.path().join("notes/01.md"), "their line\n").unwrap();
    assert!(git::commit_all(vault.path(), "theirs").unwrap());
    g(&["checkout", "-"]);
    fs::write(vault.path().join("notes/01.md"), "our line\n").unwrap();
    assert!(git::commit_all(vault.path(), "ours").unwrap());

    let merge = g(&["merge", "theirs"]);
    assert!(!merge.status.success(), "the merge really did conflict");
    let before = log_count(vault.path());

    assert!(
        !git::commit_all(vault.path(), "auto: 5s later").unwrap(),
        "mid-merge → nothing committed, and not an error"
    );
    assert_eq!(log_count(vault.path()), before, "no commit was made");
    let on_disk = fs::read_to_string(vault.path().join("notes/01.md")).unwrap();
    assert!(on_disk.contains("<<<<<<<"), "markers still there for the human to resolve");
}

#[test]
fn set_remote_is_idempotent_and_the_last_url_wins() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    assert_eq!(git::remote(vault.path()).unwrap(), None, "a fresh vault has no remote");
    identify(vault.path());

    git::set_remote(vault.path(), "/tmp/first.git").unwrap();
    assert_eq!(git::remote(vault.path()).unwrap().as_deref(), Some("/tmp/first.git"));

    // Saving again must update, not fail on "remote already exists" — the panel
    // just saves whatever the user typed.
    git::set_remote(vault.path(), "/tmp/second.git").unwrap();
    assert_eq!(git::remote(vault.path()).unwrap().as_deref(), Some("/tmp/second.git"));
}

/// The provenance hole: a researcher who never configured git gets the placeholder
/// committer, and without this guard every commit they ever push to a shared vault
/// is attributed to `formicaria` — so "who touched this note?", the question the
/// whole awareness-over-enforcement design rests on, has one answer for everybody.
/// A remote is the moment to ask, because it is the moment a name starts mattering.
#[test]
fn a_vault_cannot_gain_a_remote_until_someone_real_owns_it() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    no_identity(vault.path());
    write_and_commit(vault.path(), "01.md", "a private note\n");
    assert_eq!(git::identity(vault.path()), None, "the placeholder is nobody");

    let err = git::set_remote(vault.path(), "/tmp/shared.git").unwrap_err().to_string();
    assert!(err.contains("who you are"), "asks, and says why: {err}");
    assert_eq!(git::remote(vault.path()).unwrap(), None, "and the vault stayed private");

    // Answer once, and the vault can be shared.
    git::set_identity(vault.path(), "Ravi Patel", "ravi@example.org").unwrap();
    assert_eq!(
        git::identity(vault.path()),
        Some(git::Identity { name: "Ravi Patel".into(), email: "ravi@example.org".into() }),
    );
    git::set_remote(vault.path(), "/tmp/shared.git").unwrap();
    assert_eq!(git::remote(vault.path()).unwrap().as_deref(), Some("/tmp/shared.git"));

    // And it is the identity git actually commits with, not just something we stored.
    write_and_commit(vault.path(), "02.md", "a shared note\n");
    let who = Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["log", "-1", "--pretty=%an <%ae>"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&who.stdout).trim(), "Ravi Patel <ravi@example.org>");
}

/// A user whose git is already configured is never asked — the guard exists for the
/// people who have no identity, and must be invisible to everyone else.
#[test]
fn an_existing_git_identity_is_accepted_as_is() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    // As if it came from the user's global config, before we ever looked.
    Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["config", "user.email", "already@configured.dev"])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["config", "user.name", "Already Configured"])
        .output()
        .unwrap();

    git::set_remote(vault.path(), "/tmp/shared.git").expect("no question asked");
}

#[test]
fn an_identity_needs_a_name_and_something_that_is_actually_an_email() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    no_identity(vault.path());

    assert!(git::set_identity(vault.path(), "", "ravi@example.org").is_err(), "no name");
    assert!(git::set_identity(vault.path(), "Ravi", "  ").is_err(), "no email");
    // The mistake worth catching: it is invisible once committed, and forever.
    assert!(git::set_identity(vault.path(), "Ravi", "Ravi").is_err(), "a name in the email box");
    // Our own stand-in must never be settable as if it were a person.
    assert!(git::set_identity(vault.path(), "x", "formicaria@localhost").is_err(), "the fake");

    assert_eq!(git::identity(vault.path()), None, "nothing broken got stored");
}

#[test]
fn an_empty_remote_url_is_refused_rather_than_silently_broken() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();

    // `git remote add origin ""` succeeds and leaves a remote that reports its
    // own *name* as its URL — the vault would then claim a destination it hasn't
    // got. Better to refuse than to lie about where a backup went.
    assert!(git::set_remote(vault.path(), "   ").is_err(), "blank URL refused");
    assert_eq!(git::remote(vault.path()).unwrap(), None, "and no remote was created");
}

#[test]
fn push_refuses_without_a_remote() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    write_and_commit(vault.path(), "01.md", "a note\n");

    let err = git::push_squashed(vault.path(), "backup").unwrap_err().to_string();
    assert!(err.contains("no remote configured"), "our own message, not git's stderr: {err}");
}

/// The whole push contract, against a local bare repo — real git, no network and
/// no credentials.
#[test]
fn push_sends_history_whole_the_first_time_then_squashes_each_later_push() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let bare = tempdir().unwrap();
    assert!(Command::new("git")
        .args(["init", "--bare"])
        .arg(bare.path())
        .output()
        .unwrap()
        .status
        .success());

    write_and_commit(vault.path(), "01.md", "one\n");
    write_and_commit(vault.path(), "02.md", "two\n");
    write_and_commit(vault.path(), "03.md", "three\n");
    identify(vault.path());
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();

    // Nothing has ever been pushed, so there is no tracking ref to measure
    // against — and squashing here would destroy the only copy of that history.
    assert_eq!(git::unpushed(vault.path()).unwrap(), None, "no tracking ref before the first push");
    assert_eq!(git::push_squashed(vault.path(), "backup: first").unwrap(), 0, "first push: no squash");
    assert_eq!(log_count(bare.path()), 3, "the first push sends the history as it stands");
    assert_eq!(git::unpushed(vault.path()).unwrap(), Some(0), "nothing left to push");

    // From here on, a window of auto-commits collapses into one remote commit.
    write_and_commit(vault.path(), "04.md", "four\n");
    write_and_commit(vault.path(), "05.md", "five\n");
    assert_eq!(git::unpushed(vault.path()).unwrap(), Some(2));

    assert_eq!(git::push_squashed(vault.path(), "backup: second").unwrap(), 2, "squashed the pair");
    assert_eq!(log_count(bare.path()), 4, "3 + exactly one squashed commit");
    assert_eq!(git::unpushed(vault.path()).unwrap(), Some(0));

    // The squash must not lose content: both notes are in the pushed tree.
    let files = Command::new("git")
        .arg("-C")
        .arg(bare.path())
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .unwrap();
    let files = String::from_utf8_lossy(&files.stdout);
    assert!(files.contains("notes/04.md") && files.contains("notes/05.md"), "squashed: {files}");
}

#[test]
fn a_single_unpushed_commit_is_pushed_as_is() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();

    write_and_commit(vault.path(), "01.md", "one\n");
    identify(vault.path());
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(vault.path(), "backup: first").unwrap();

    // One commit is already one commit — squashing it would be pointless churn.
    write_and_commit(vault.path(), "02.md", "two\n");
    assert_eq!(git::push_squashed(vault.path(), "backup: second").unwrap(), 0, "nothing to squash");
    assert_eq!(log_count(bare.path()), 2);
}
