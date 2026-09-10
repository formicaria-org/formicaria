//! Vault versioning via git — the notes' durable, pushable history. Hermetic: a
//! tempdir vault, a repo-local identity written by `ensure_repo`, no network.
//! Skipped (not failed) when git is absent.

use fm_core::git;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// The note files in a vault — what `FileStore::put` would have recorded had these tests
/// gone through the store rather than writing files directly. `commit_all` now stages an
/// explicit list, so a test has to say what it wrote, the same as the app does.
fn notes_of(vault: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(vault.join("notes"))
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "md"))
                .collect()
        })
        .unwrap_or_default()
}

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

    assert!(
        git::commit_all(vault.path(), "first snapshot", &notes_of(vault.path())).unwrap(),
        "committed the new note"
    );

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
    assert!(
        !git::commit_all(vault.path(), "no-op", &notes_of(vault.path())).unwrap(),
        "clean tree → nothing to commit"
    );
}

/// A note captured *and* deleted before the debounced auto-commit runs is in the batch `paths`
/// (the store recorded both the write and the delete) but is neither on disk nor tracked. Git
/// cannot name it, so `git add -A -- <it>` rejects the whole batch — which used to pause history
/// and backup for every other change too. It must be dropped silently, and the real change commit.
#[test]
fn a_note_created_then_deleted_before_its_first_commit_does_not_brick_the_commit() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let vault = tempdir().unwrap();
    let notes = vault.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join("real.md"), "kept\n").unwrap();

    // The phantom: recorded as touched, never written to disk (created-then-deleted). Not on disk,
    // not tracked — the exact pathspec `git add` cannot match.
    let phantom = notes.join("phantom.md");
    assert!(!phantom.exists());
    let paths = vec![notes.join("real.md"), phantom];

    let committed = git::commit_all(vault.path(), "phantom in the batch", &paths)
        .expect("a create-then-delete must not brick the commit");
    assert!(committed, "the real note was still committed");

    let ls = Command::new("git").arg("-C").arg(vault.path()).args(["ls-files"]).output().unwrap();
    let files = String::from_utf8_lossy(&ls.stdout);
    assert!(files.contains("notes/real.md"), "the real note is tracked");
    assert!(!files.contains("phantom"), "the phantom never entered git");
}

/// Commit a note, so each call adds exactly one commit to the vault's history.
fn write_and_commit(vault: &std::path::Path, name: &str, body: &str) {
    let notes = vault.join("notes");
    fs::create_dir_all(&notes).unwrap();
    fs::write(notes.join(name), body).unwrap();
    assert!(git::commit_all(vault, &format!("auto: {name}"), &notes_of(vault)).unwrap());
}

fn log_count(repo: &std::path::Path) -> usize {
    let out = Command::new("git").arg("-C").arg(repo).args(["log", "--oneline"]).output().unwrap();
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
    assert!(Command::new("git").arg("init").arg(vault.path()).output().unwrap().status.success());

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
    Command::new("git")
        .arg("-C")
        .arg(other.path())
        .args(["config", "user.email", "t@t"])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(other.path())
        .args(["config", "user.name", "t"])
        .output()
        .unwrap();
    fs::write(other.path().join("theirs.md"), "from elsewhere\n").unwrap();
    Command::new("git").arg("-C").arg(other.path()).args(["add", "-A"]).output().unwrap();
    Command::new("git")
        .arg("-C")
        .arg(other.path())
        .args(["commit", "-m", "theirs"])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(other.path())
        .args(["push", "origin", "HEAD"])
        .output()
        .unwrap();

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
    Command::new("git")
        .arg("-C")
        .arg(other.path())
        .args(["commit", "-m", "theirs"])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(other.path())
        .args(["push", "origin", "HEAD"])
        .output()
        .unwrap();

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
    assert!(git::commit_all(vault.path(), "theirs", &notes_of(vault.path())).unwrap());
    g(&["checkout", "-"]);
    fs::write(vault.path().join("notes/01.md"), "our line\n").unwrap();
    assert!(git::commit_all(vault.path(), "ours", &notes_of(vault.path())).unwrap());

    let merge = g(&["merge", "theirs"]);
    assert!(!merge.status.success(), "the merge really did conflict");
    let before = log_count(vault.path());

    assert!(
        !git::commit_all(vault.path(), "auto: 5s later", &notes_of(vault.path())).unwrap(),
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
    assert_eq!(
        git::push_squashed(vault.path(), "backup: first").unwrap(),
        0,
        "first push: no squash"
    );
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

/// **The Track V bug that was silent.** Every real repo already has a `.gitattributes` and
/// a `.gitignore`, so "skip the file if it exists" meant that the moment a vault was a
/// project you already owned, `merge=fm` never landed and `blobs/` was never ignored.
///
/// The first is Track C Phase 1's disaster reintroduced by conversion: without the
/// attribute git uses its built-in text merge, every concurrent edit collides on the
/// `updated:` line the app rewrites on each save, and the markers land inside the YAML
/// fence where the note stops parsing. The second commits your blobs and your per-machine
/// SQLite index, then pushes them.
#[test]
fn adopting_a_repo_that_already_has_these_files_still_gets_our_rules() {
    let vault = tempdir().unwrap();
    // A repo as it actually arrives: initialised, with both files already populated by
    // whoever set the project up.
    std::process::Command::new("git").arg("-C").arg(vault.path()).arg("init").output().unwrap();
    std::fs::write(vault.path().join(".gitattributes"), "*.png binary\n").unwrap();
    std::fs::write(vault.path().join(".gitignore"), "target/\n*.log\n").unwrap();

    git::ensure_repo(vault.path()).unwrap();

    let attrs = std::fs::read_to_string(vault.path().join(".gitattributes")).unwrap();
    let ignore = std::fs::read_to_string(vault.path().join(".gitignore")).unwrap();

    // Ours landed…
    assert!(attrs.contains("*.md merge=fm"), "the merge driver must engage:\n{attrs}");
    for line in ["index.sqlite", "derived/", "blobs/"] {
        assert!(ignore.lines().any(|l| l == line), "{line} must be ignored:\n{ignore}");
    }
    // …without touching theirs. A writer that rewrites what it did not author is one you
    // cannot point at someone's repo.
    assert!(attrs.contains("*.png binary"), "their rule survived:\n{attrs}");
    assert!(ignore.contains("target/") && ignore.contains("*.log"), "theirs survived:\n{ignore}");
}

/// Running twice must not stack duplicates — `ensure_repo` is called on every commit.
#[test]
fn ensuring_a_repo_repeatedly_does_not_duplicate_lines() {
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    git::ensure_repo(vault.path()).unwrap();
    git::ensure_repo(vault.path()).unwrap();

    let ignore = std::fs::read_to_string(vault.path().join(".gitignore")).unwrap();
    assert_eq!(ignore.lines().filter(|l| *l == "blobs/").count(), 1, "{ignore}");
    let attrs = std::fs::read_to_string(vault.path().join(".gitattributes")).unwrap();
    // Each rule exactly once, matched on the *whole line*. A `contains("merge=fm")` here
    // would also match `merge=fm-manifest` and so count two rules as a duplicated one — the
    // substring is a trap now that there are two drivers.
    for rule in ["*.md merge=fm text eol=lf", "manifest.json merge=fm-manifest text eol=lf"] {
        assert_eq!(attrs.lines().filter(|l| l.trim() == rule).count(), 1, "{rule}\n{attrs}");
    }
}

/// A file without a trailing newline must not get our line glued onto its last one.
#[test]
fn a_file_missing_its_trailing_newline_is_not_corrupted() {
    let vault = tempdir().unwrap();
    std::process::Command::new("git").arg("-C").arg(vault.path()).arg("init").output().unwrap();
    std::fs::write(vault.path().join(".gitignore"), "target/").unwrap(); // no newline

    git::ensure_repo(vault.path()).unwrap();

    let ignore = std::fs::read_to_string(vault.path().join(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "target/"), "their rule is intact:\n{ignore}");
    assert!(ignore.lines().any(|l| l == "blobs/"), "and ours is its own line:\n{ignore}");
}

/// **A vault is increasingly a repo you already have** — notes beside the code they
/// describe. The debounced auto-commit runs every 5 s in that repo, so `git add -A` made
/// it a second author: it staged your half-written function and committed it under `auto:`.
#[test]
fn the_auto_commit_never_touches_files_the_app_did_not_write() {
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();

    // The project's own work, mid-edit, exactly as it sits while you are typing.
    std::fs::create_dir_all(vault.path().join("src")).unwrap();
    std::fs::write(vault.path().join("src/lib.rs"), "fn half_written(  \n").unwrap();
    // And a note, which is ours.
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();
    std::fs::write(vault.path().join("notes/01JQ.md"), "---\nid: x\n---\nbody\n").unwrap();

    assert!(git::commit_all(vault.path(), "auto: test", &notes_of(vault.path())).unwrap());

    let files = std::process::Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["show", "--name-only", "--format=", "HEAD"])
        .output()
        .unwrap();
    let committed = String::from_utf8_lossy(&files.stdout);
    assert!(committed.contains("notes/01JQ.md"), "our note is committed:\n{committed}");
    assert!(!committed.contains("src/lib.rs"), "their half-written code must NOT be:\n{committed}");
}

/// Losing a curated index is not recoverable by re-running anything, so the auto-commit
/// must leave one alone.
#[test]
fn the_auto_commit_leaves_a_users_staged_index_staged() {
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();

    // The user stages something of their own, intending to commit it themselves.
    std::fs::write(vault.path().join("theirs.txt"), "carefully staged\n").unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["add", "theirs.txt"])
        .output()
        .unwrap();

    // Meanwhile the app saves a note and the debounce fires.
    std::fs::write(vault.path().join("notes/01JQ.md"), "---\nid: x\n---\nbody\n").unwrap();
    git::commit_all(vault.path(), "auto: test", &notes_of(vault.path())).unwrap();

    // Their file is still staged and still uncommitted — theirs to commit, when they choose.
    let staged = std::process::Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["diff", "--cached", "--name-only"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&staged.stdout).contains("theirs.txt"),
        "the user's staged file was swept into our commit"
    );
}

/// A repo that is dirty only with someone else's work has nothing for us to commit — and
/// must not report that it made one.
#[test]
fn a_repo_dirty_only_with_their_work_reports_nothing_to_commit() {
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();
    git::commit_all(vault.path(), "auto: baseline", &notes_of(vault.path())).unwrap();

    std::fs::write(vault.path().join("their-code.rs"), "fn theirs() {}\n").unwrap();

    assert!(!git::commit_all(vault.path(), "auto: test", &notes_of(vault.path())).unwrap());
}

/// **The Track V bug: the squash ate the user's own commits.**
///
/// `reset --soft <tracking>` collapsed *every* unpushed commit. Squashing the app's
/// 5-second `auto:` churn is the whole reason the squash exists; collapsing three
/// hand-written manuscript commits into one `backup:` never was. The work survives, its
/// shape does not — which is the quiet kind of loss.
#[test]
fn the_squash_stops_at_a_commit_the_user_wrote_by_hand() {
    let bare = tempdir().unwrap();
    std::process::Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();

    let note = |n: u32| {
        std::fs::write(
            vault.path().join(format!("notes/{n}.md")),
            format!("---\nid: {n}\n---\nbody {n}\n"),
        )
        .unwrap();
    };
    let hand_commit = |msg: &str, file: &str| {
        std::fs::write(vault.path().join(file), "their work\n").unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(vault.path())
            .args(["add", file])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(vault.path())
            .args(["commit", "-m", msg])
            .output()
            .unwrap();
    };

    // A baseline that is already pushed, so `tracking` exists.
    note(1);
    git::commit_all(vault.path(), "auto: one", &notes_of(vault.path())).unwrap();
    // **Say who we are before setting a remote**, which `set_remote` requires: past that point
    // every commit carries a name into somebody else's clone. Without this the test inherited
    // an identity from the developer's *global* git config — `git config` falls through to it —
    // so it passed on a machine that had ever run `git config --global user.name` and nowhere
    // else. Found 2026-09-10 when the gate ran on a fresh runner for the first time in weeks.
    git::set_identity(vault.path(), "Tester", "tester@example.org").unwrap();
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(vault.path(), "backup: first").unwrap();

    // Now: the app churns, the user writes a real commit, the app churns again.
    note(2);
    git::commit_all(vault.path(), "auto: two", &notes_of(vault.path())).unwrap();
    hand_commit("Rewrite the introduction", "chapter.md");
    note(3);
    git::commit_all(vault.path(), "auto: three", &notes_of(vault.path())).unwrap();

    git::push_squashed(vault.path(), "backup: second").unwrap();

    let log = std::process::Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["log", "--format=%s", "-5"])
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&log.stdout);
    assert!(
        log.contains("Rewrite the introduction"),
        "the user's own commit must survive the squash:\n{log}"
    );
}

/// A dedicated vault — every commit ours — must behave exactly as before: one `backup:`.
#[test]
fn a_vault_of_only_our_commits_still_squashes_to_one() {
    let bare = tempdir().unwrap();
    std::process::Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();
    let vault = tempdir().unwrap();
    git::ensure_repo(vault.path()).unwrap();
    std::fs::create_dir_all(vault.path().join("notes")).unwrap();

    let note = |n: u32| {
        std::fs::write(
            vault.path().join(format!("notes/{n}.md")),
            format!("---\nid: {n}\n---\nbody {n}\n"),
        )
        .unwrap();
    };

    note(1);
    git::commit_all(vault.path(), "auto: one", &notes_of(vault.path())).unwrap();
    // **Say who we are before setting a remote**, which `set_remote` requires: past that point
    // every commit carries a name into somebody else's clone. Without this the test inherited
    // an identity from the developer's *global* git config — `git config` falls through to it —
    // so it passed on a machine that had ever run `git config --global user.name` and nowhere
    // else. Found 2026-09-10 when the gate ran on a fresh runner for the first time in weeks.
    git::set_identity(vault.path(), "Tester", "tester@example.org").unwrap();
    git::set_remote(vault.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(vault.path(), "backup: first").unwrap();

    note(2);
    git::commit_all(vault.path(), "auto: two", &notes_of(vault.path())).unwrap();
    note(3);
    git::commit_all(vault.path(), "auto: three", &notes_of(vault.path())).unwrap();

    let squashed = git::push_squashed(vault.path(), "backup: second").unwrap();

    assert_eq!(squashed, 2, "both auto commits collapse, exactly as before");
}

/// **The clone path — step 4 of the mobile sequence, on the backend that exists today.**
///
/// `git.rs` had no clone at all until now, so this is new code under every possible git
/// backend rather than a port of something. Two things have to hold, and neither is
/// obvious from the outside:
///
/// 1. **A clone is not yet a vault.** The `*.md merge=fm` attribute travels in the repo, but
///    the `merge.fm.driver` *definition* lives in `.git/config` and deliberately does not —
///    git will not let a repo ship a command that runs on your machine. A collaborator who
///    clones and gets only half of that silently falls back to git's plain text merge and
///    conflicts on the `updated:` line of every concurrent edit, which is the entire thing
///    the driver exists to prevent. So `clone` installs it.
/// 2. **The first commit must be attributable.** A cloned vault has an audience by
///    definition — that is what makes the placeholder committer actively wrong there rather
///    than merely unhelpful, and it is the hole ruling 5 was written to close.
#[test]
fn a_cloned_vault_gets_the_driver_and_commits_as_a_real_person() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    // A remote with one note already in it, exactly as a collaborator would leave it.
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();
    let origin = tempdir().unwrap();
    write_and_commit(origin.path(), "01.md", "theirs\n");
    identify(origin.path());
    git::set_remote(origin.path(), bare.path().to_str().unwrap()).unwrap();
    git::push_squashed(origin.path(), "backup: first").unwrap();

    // Clone into a path that does not exist yet — the real shape of "add a shared vault".
    let parent = tempdir().unwrap();
    let dest = parent.path().join("cloned-vault");
    git::clone(bare.path().to_str().unwrap(), &dest).unwrap();

    assert!(dest.join(".git").exists(), "it is a repo");
    assert!(dest.join("notes/01.md").exists(), "their work came with it");

    // `ensure_repo` ran: the merge attribute and the ignore rules are both here, on a repo
    // this process did not create.
    let attrs = fs::read_to_string(dest.join(".gitattributes")).unwrap();
    assert!(attrs.contains("merge=fm"), "the merge attribute is present: {attrs}");
    let ignore = fs::read_to_string(dest.join(".gitignore")).unwrap();
    for line in ["index.sqlite", "derived/", "blobs/"] {
        assert!(
            ignore.contains(line),
            "clone gets the ignore rules too, missing {line}:\n{ignore}"
        );
    }

    // The *other* half of the driver — the `merge.fm.driver` definition in `.git/config` — is
    // asserted in `fm-cli`'s `a_fresh_clone_gets_both_halves_of_the_driver`, not here.
    // `ensure_repo` points the driver at the `fm` binary beside the running one and refuses to
    // install one it cannot find; this test binary lives in `target/debug/deps`, where there is
    // no `fm`. Asserting it here would test the layout of the test harness, not the product.

    // The placeholder is what `ensure_repo` leaves when the machine has no identity. A clone
    // must not commit on it, so set one and check the commit that lands.
    git::set_identity(&dest, "Ravi Test", "ravi@example.org").unwrap();
    write_and_commit(&dest, "02.md", "ours\n");

    let author = Command::new("git")
        .arg("-C")
        .arg(&dest)
        .args(["log", "-1", "--format=%ae"])
        .output()
        .unwrap();
    let author = String::from_utf8_lossy(&author.stdout).trim().to_string();
    assert_eq!(author, "ravi@example.org");
    assert_ne!(author, "formicaria@localhost", "a shared vault must not commit as the placeholder");
}

/// Refuse before writing anything, rather than letting `git clone` create the directory and
/// then fail inside it — a half-made vault is worse than none, and the message should be
/// about what the user was doing.
#[test]
fn cloning_into_a_non_empty_directory_is_refused_and_writes_nothing() {
    if !have_git() {
        eprintln!("skipping git test: git not on PATH");
        return;
    }
    let occupied = tempdir().unwrap();
    fs::write(occupied.path().join("mine.md"), "do not touch\n").unwrap();

    let err = git::clone("https://example.invalid/repo.git", occupied.path()).unwrap_err();
    assert!(format!("{err}").contains("not empty"), "says why: {err}");
    assert!(!occupied.path().join(".git").exists(), "and wrote nothing");
    assert_eq!(fs::read_to_string(occupied.path().join("mine.md")).unwrap(), "do not touch\n");
}

/// **Step 5: the push is verified against the remote, not against the pusher.**
///
/// A squashing push collapses the user's granular history on the bet that the push lands.
/// With subprocess git the exit code is trustworthy and this never fires — it is here for
/// whatever replaces it, where "success" becomes our own parser's opinion and a false
/// success would charge the user their undo for a backup that never happened.
///
/// What is asserted is the invariant itself: after a squashing push returns Ok, the remote
/// really does point at what we have locally.
#[test]
fn a_squashing_push_leaves_the_remote_holding_exactly_our_head() {
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

    // Several auto-commits, so the next push has something to collapse — the case where a
    // false success would actually cost history.
    write_and_commit(vault.path(), "02.md", "two\n");
    write_and_commit(vault.path(), "03.md", "three\n");
    let squashed = git::push_squashed(vault.path(), "backup: second").unwrap();
    assert_eq!(squashed, 2, "both auto commits collapsed");

    let local = Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let local = String::from_utf8_lossy(&local.stdout).trim().to_string();

    // Ask the remote directly, the same way the guard does.
    let remote = Command::new("git")
        .arg("-C")
        .arg(vault.path())
        .args(["ls-remote", "origin", "refs/heads/main", "refs/heads/master"])
        .output()
        .unwrap();
    let remote = String::from_utf8_lossy(&remote.stdout);
    let remote_sha = remote.split_whitespace().next().unwrap_or_default();

    assert_eq!(remote_sha, local, "the remote holds exactly what we pushed\n{remote}");

    // And the squash really did happen — the guard must not have quietly rolled it back.
    assert_eq!(log_count(vault.path()), 2, "first push, then one squashed commit");
}

/// **The invariant that stops silent data loss: `merge.fm.driver` never names a binary that
/// is not there.**
///
/// The definition lives in `.git/config` and holds an *absolute* path to `fm`. Git declines
/// to carry `.git/config` in a clone, which is why `ensure_repo` reinstalls it — but a
/// directory *copy* carries it verbatim, and so does every reinstall to a different prefix, a
/// dev build where a release one ran, a package shipping `fm-serve` without `fm`, and mobile,
/// which has no `fm` beside it at all.
///
/// A driver naming a path that no longer exists is far worse than no driver. Git takes any
/// non-zero exit as "conflict" and hands back `%A` untouched — *our* version, with no markers
/// in it. The user sees a conflict, opens a file that looks completely normal, resolves it,
/// and has silently deleted their collaborator's edit. An absent driver instead degrades to
/// git's built-in text merge: uglier, and visible.
///
/// Written to hold whether or not an `fm` happens to sit beside this test binary, because
/// that is the real invariant and it must not depend on how the suite was built.
#[test]
fn a_stale_merge_driver_is_removed_rather_than_left_pointing_at_nothing() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempdir().unwrap();
    let vault = dir.path();
    git::ensure_repo(vault).unwrap();

    // Exactly what arrives with a copied `.git/config`: a driver naming a machine we are not.
    let stale = "/nonexistent/prefix/fm merge-md %O %A %B %L";
    Command::new("git")
        .current_dir(vault)
        .args(["config", "merge.fm.driver", stale])
        .output()
        .unwrap();

    git::ensure_repo(vault).unwrap();

    let out = Command::new("git")
        .current_dir(vault)
        .args(["config", "--get", "merge.fm.driver"])
        .output()
        .unwrap();
    let configured = String::from_utf8_lossy(&out.stdout).trim().to_string();

    assert_ne!(configured, stale, "the stale driver must not survive `ensure_repo`");
    if configured.is_empty() {
        return; // No `fm` on this machine: unset is the correct answer.
    }
    // Otherwise it was re-pointed, and what it points at must actually exist — the whole
    // point. The value is `'<path>' merge-md …`, quoted because the path may contain spaces.
    let binary = configured
        .strip_prefix('\'')
        .and_then(|r| r.split_once('\''))
        .map(|(p, _)| p.to_string())
        .unwrap_or_else(|| panic!("unexpected driver format: {configured}"));
    assert!(
        std::path::Path::new(&binary).exists(),
        "the driver must name a binary that exists, or none at all — got {binary}"
    );
}

/// A committer that arrived with a copied vault is forgotten, so this machine is asked who it
/// is rather than signing a shared history as whoever sent it.
///
/// Only ever runs on freshly-acquired directories (`acquire::naturalise`); on a vault someone
/// works in this would detach their name from their own commits.
#[test]
fn forget_identity_clears_a_committer_that_came_with_the_copy() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempdir().unwrap();
    let vault = dir.path();
    git::ensure_repo(vault).unwrap();
    git::set_identity(vault, "Ada Lovelace", "ada@example.org").unwrap();

    let local = |key: &str| {
        let out = Command::new("git")
            .current_dir(vault)
            .args(["config", "--local", "--get", key])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    assert_eq!(local("user.email"), "ada@example.org", "the sender's identity is here first");

    assert!(git::forget_identity(vault), "there was one to forget");
    assert!(local("user.email").is_empty(), "and the repo no longer carries it");
    assert!(local("user.name").is_empty());

    // Asserted on the *repo's own* config, never on `identity()`: that one resolves the way
    // git would for a commit, so on a machine with a global `user.email` it correctly keeps
    // answering — with **this** machine's person instead of the sender's, which is the entire
    // point of forgetting. "No identity anywhere" was never the goal.

    // Idempotent, and honest about having found nothing the second time.
    assert!(!git::forget_identity(vault), "nothing left to forget");
}

/// **The three failures a clone cannot tell apart.** A typo'd URL, a repo that needs
/// credentials, and being offline all come out of `git clone` as exit 128 with a message about
/// usernames — and they need completely different next steps. `probe` is what separates them
/// *before* a clone has committed to a directory.
///
/// Offline by construction: a `file://` remote for the reachable case, a nonexistent path for
/// the unreachable one. The auth case is classified from git's own wording rather than by
/// contacting a host that would need credentials, because a test that needed the network would
/// not run here.
#[test]
fn probe_separates_reachable_from_needs_auth_from_typo() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    // A real repo on disk, which is reachable with no credentials at all.
    let origin = tempdir().unwrap();
    fs::create_dir_all(origin.path().join("notes")).unwrap();
    fs::write(origin.path().join("notes/01.md"), "theirs\n").unwrap();
    git::ensure_repo(origin.path()).unwrap();
    git::commit_all(origin.path(), "theirs", &[origin.path().join("notes/01.md")]).unwrap();

    assert_eq!(git::probe(origin.path().to_str().unwrap()), git::Probe::Reachable);

    // A path that is not a repo. Must NOT be reported as an auth problem — sending someone to
    // configure credentials for a URL they mistyped is the failure this test exists to prevent.
    let missing = origin.path().join("no-such-repo");
    match git::probe(missing.to_str().unwrap()) {
        git::Probe::Unreachable(_) => {}
        other => panic!("a bad path must not look like an auth problem, got {other:?}"),
    }

    // Empty input is not a remote.
    assert!(matches!(git::probe("   "), git::Probe::Unreachable(_)));
}

/// The classifier, over the wordings git and the forges actually emit. Kept separate from the
/// probe so every phrasing can be covered without a network or a credential.
#[test]
fn auth_failures_are_recognised_and_nothing_else_is() {
    for stderr in [
        "fatal: Authentication failed for 'https://github.com/you/notes.git/'",
        "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
        "git@github.com: Permission denied (publickey).",
        "remote: Invalid username or password.",
        "fatal: unable to access '...': The requested URL returned error: 403 Forbidden",
    ] {
        assert_eq!(
            git::Probe::from_stderr(stderr),
            git::Probe::NeedsAuth,
            "should read as an auth problem: {stderr}"
        );
    }

    // Everything else keeps git's own words and stays "unreachable". A wrong guess here is
    // worse than no guess: it sends the user to fix credentials that were never the problem.
    for stderr in [
        "fatal: repository 'https://github.com/you/typo.git/' not found",
        "fatal: unable to access '...': Could not resolve host: githubb.com",
        "fatal: '/tmp/nope' does not appear to be a git repository",
        // The exact text git emits for a MISSING repo, verified by running it. It contains
        // "correct access rights", which is why that phrase cannot be an auth signal: this
        // string must classify as unreachable or a typo becomes a credentials lecture.
        "fatal: '/nonexistent/no-such-repo' does not appear to be a git repository\nfatal: \
         Could not read from remote repository.\n\nPlease make sure you have the correct \
         access rights\nand the repository exists.",
    ] {
        match git::Probe::from_stderr(stderr) {
            git::Probe::Unreachable(text) => {
                assert!(!text.is_empty(), "git's own words are kept: {stderr}")
            }
            other => panic!("must not claim auth for {stderr}, got {other:?}"),
        }
    }
}

/// Write a blob into the vault's content-addressed store, as `ingest` would, and return its
/// path relative to the vault.
fn plant_blob(vault: &std::path::Path, hash: &str, bytes: usize) -> String {
    let rel = format!("blobs/sha256/{}/{}/{hash}", &hash[0..2], &hash[2..4]);
    let at = vault.join(&rel);
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    fs::write(&at, vec![b'x'; bytes]).unwrap();
    rel
}

fn tracked(vault: &std::path::Path) -> Vec<String> {
    let out = Command::new("git").current_dir(vault).args(["ls-files"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect()
}

/// **Media stays out of git unless the vault asks for it**, which is the default and the whole
/// basis of the two-tier backup split.
#[test]
fn blobs_are_not_committed_when_the_vault_says_nothing() {
    if !have_git() {
        eprintln!("skipping: no git");
        return;
    }
    let d = tempdir().unwrap();
    let vault = d.path();
    fs::create_dir_all(vault.join("notes")).unwrap();
    git::ensure_repo(vault).unwrap();
    fs::write(vault.join("notes/a.md"), "hello").unwrap();
    let small = plant_blob(vault, &"aa".repeat(32), 10);

    git::commit_all(vault, "auto: test", &notes_of(vault)).unwrap();
    assert!(!tracked(vault).contains(&small), "a blob must not travel by default");
}

/// **With a threshold set, small attachments travel and large ones do not.**
///
/// The selection happens here rather than in git, which cannot filter by size — and each chosen
/// file is named explicitly with `-f`, because `blobs/` is gitignored. That combination is the
/// only reason this can be both opt-in and precise.
#[test]
fn a_vault_with_a_threshold_commits_small_blobs_and_leaves_big_ones() {
    if !have_git() {
        eprintln!("skipping: no git");
        return;
    }
    let d = tempdir().unwrap();
    let vault = d.path();
    fs::create_dir_all(vault.join("notes")).unwrap();
    git::ensure_repo(vault).unwrap();
    fs::write(vault.join("vault.json"), "{\n  \"git_assets_max\": \"1kB\"\n}\n").unwrap();
    fs::write(vault.join("notes/a.md"), "hello").unwrap();

    let small = plant_blob(vault, &"aa".repeat(32), 500);
    let big = plant_blob(vault, &"bb".repeat(32), 5000);
    let empty = plant_blob(vault, &"cc".repeat(32), 0);

    git::commit_all(vault, "auto: test", &notes_of(vault)).unwrap();
    let files = tracked(vault);
    assert!(files.contains(&small), "an attachment under the limit should travel: {files:?}");
    assert!(!files.contains(&big), "one over it must not: {files:?}");
    assert!(!files.contains(&empty), "a zero-byte blob is not media: {files:?}");
    // The user's own ignore rule is untouched — the blob was force-added, not un-ignored.
    let ignore = fs::read_to_string(vault.join(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l.trim() == "blobs/"), "blobs/ must stay ignored: {ignore}");
}

/// Setting the threshold **keeps every other key**, including ones this version never heard of.
/// The descriptor is the user's file; a setting may not eat a description they wrote.
#[test]
fn setting_the_asset_limit_preserves_the_rest_of_vault_json() {
    let d = tempdir().unwrap();
    fs::write(
        d.path().join("vault.json"),
        "{\n \"name\": \"lab\",\n \"description\": \"mine\",\n \"future_key\": [1, 2]\n}\n",
    )
    .unwrap();

    fm_core::descriptor::Descriptor::set_git_assets_max(d.path(), Some(2_000_000)).unwrap();
    let text = fs::read_to_string(d.path().join("vault.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["name"], "lab");
    assert_eq!(v["description"], "mine");
    assert_eq!(v["future_key"], serde_json::json!([1, 2]), "unknown keys must survive");
    assert_eq!(v["git_assets_max"], "2MB", "and it round-trips as a size a human would write");

    let back = fm_core::descriptor::Descriptor::read(d.path()).unwrap();
    assert_eq!(back.git_assets_max, Some(2_000_000));

    // Turning it off removes the key rather than leaving `null` behind.
    fm_core::descriptor::Descriptor::set_git_assets_max(d.path(), None).unwrap();
    let text = fs::read_to_string(d.path().join("vault.json")).unwrap();
    assert!(!text.contains("git_assets_max"), "off should look like never-set: {text}");
    assert!(text.contains("\"name\""), "and must not have eaten the rest");
}

// **The ceiling, and the two places it has to hold.**
//
// There is no git-lfs here, so an attachment in git is permanent history that every clone pays
// for again — and above ~100 MB most hosts refuse the push outright, *after* the commit is made.
// So the app refuses to write a limit it would not honour, and refuses to honour one it did not
// write. Both halves are needed: the setter alone would leave a hand-edited `vault.json` staging
// a blob no remote will take.
#[test]
fn an_attachment_limit_above_the_ceiling_is_refused_rather_than_written() {
    let d = tempdir().unwrap();
    fs::write(d.path().join("vault.json"), "{\n  \"name\": \"lab\"\n}\n").unwrap();

    let err = fm_core::descriptor::Descriptor::set_git_assets_max(d.path(), Some(500_000_000))
        .expect_err("500MB is past the ceiling and must not be written");
    let msg = err.to_string();
    assert!(msg.contains("500MB"), "the refusal names what was asked for: {msg}");
    assert!(msg.contains("100MB"), "and the bound it broke: {msg}");

    let text = fs::read_to_string(d.path().join("vault.json")).unwrap();
    assert!(!text.contains("git_assets_max"), "a refused write leaves the file alone: {text}");
    assert!(text.contains("\"name\""), "and certainly does not eat it");

    // The ceiling itself is allowed — the bound is inclusive, so it is a limit and not a gap.
    fm_core::descriptor::Descriptor::set_git_assets_max(
        d.path(),
        Some(fm_core::descriptor::GIT_ASSETS_CEILING),
    )
    .expect("exactly the ceiling is a legal limit");
}

#[test]
fn a_vault_asking_for_more_than_the_ceiling_only_sends_what_the_ceiling_allows() {
    use fm_core::descriptor::{effective_git_assets_max, GIT_ASSETS_CEILING};

    // Driven through the pure clamp rather than by writing a 100 MB blob to disk: the rule is one
    // expression, and `blobs_within` filters on exactly this value.
    assert_eq!(effective_git_assets_max(2_000_000), 2_000_000, "an ordinary limit is untouched");
    assert_eq!(effective_git_assets_max(GIT_ASSETS_CEILING), GIT_ASSETS_CEILING);
    assert_eq!(
        effective_git_assets_max(500_000_000),
        GIT_ASSETS_CEILING,
        "a descriptor asking for more is clamped, not obeyed and not an error"
    );
    assert_eq!(effective_git_assets_max(u64::MAX), GIT_ASSETS_CEILING);

    // And such a descriptor must still *open*. It may have been written by a hand, another
    // machine, or a version with no ceiling, and a value that was legal when written must never
    // make a vault unopenable — that is why the clamp is not a parse error.
    let d = tempdir().unwrap();
    fs::write(d.path().join("vault.json"), "{\n  \"git_assets_max\": \"500MB\"\n}\n").unwrap();
    let desc = fm_core::descriptor::Descriptor::read(d.path())
        .expect("an over-ceiling vault.json still opens");
    assert_eq!(desc.git_assets_max, Some(500_000_000), "read reports the file, unchanged");
    assert_eq!(
        effective_git_assets_max(desc.git_assets_max.unwrap()),
        GIT_ASSETS_CEILING,
        "and the staging walk is what declines to honour it"
    );
}

/// **"I could not ask" is not "you are up to date."**
///
/// `remote_moved` answers `Option<bool>` for one reason: a laptop that is asleep, on a train, or
/// pointed at a remote that has moved must produce `None`, and the two happy answers are covered
/// elsewhere (`fm-cli/tests/merge.rs` pins `Some(true)` and `Some(false)`). Nothing covered the
/// third, which is the one that runs most often — this is polled on a 45-second timer, so *most*
/// calls on a disconnected machine take this path.
///
/// **What `Some(false)` would mean to a user:** the backup panel says the remote has nothing new,
/// so there is nothing to pull. Act on that while a collaborator has actually pushed, and the next
/// local commit diverges history — the exact situation the panel exists to warn about, inverted.
/// A `None` renders as silence instead, which is the honest answer to a question nobody could ask.
///
/// The remote is deleted rather than made unreachable over a network: no sockets, no timeouts, no
/// flakes, and `git ls-remote` fails the same way it does when a host is unreachable.
///
/// Proven red by changing the `!out.status.success()` arm to `Ok(Some(false))` — the test then
/// reports that a vault pointed at a remote that no longer exists is up to date with it.
#[test]
fn a_remote_it_cannot_reach_is_unknown_not_unchanged() {
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

    // While it is reachable, the question has an answer.
    assert_eq!(
        git::remote_moved(vault.path()).unwrap(),
        Some(false),
        "a reachable remote nobody has pushed to has not moved"
    );

    // Now it is gone — a drive unmounted, a host renamed, a laptop off the network.
    std::fs::remove_dir_all(bare.path()).unwrap();

    assert_eq!(
        git::remote_moved(vault.path()).unwrap(),
        None,
        "an unreachable remote is unknown; reporting Some(false) tells the user they are level \
         with a remote nobody could reach"
    );
}
