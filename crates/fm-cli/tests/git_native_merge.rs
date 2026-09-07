//! Do the two git backends merge a note the same way? — the question the in-process backend
//! exists to answer.
//!
//! **Here rather than in `fm-core` because this needs the `.md` merge driver.** `ensure_repo`
//! points `merge.fm.driver` at the `fm` binary beside the running one, and only this crate
//! builds one. Run without it, the subprocess side falls back to git's plain text merge and
//! conflicts on the `updated:` line the app rewrites on every save — so the comparison would be
//! grading two broken things against each other.
//!
//! What is being checked: libgit2 cannot invoke a merge driver at all, so the native backend
//! reaches the same answer by calling `merge_texts` itself. If these ever disagree, a phone and
//! a laptop disagree about what a merged note is.
#![cfg(feature = "native-git")]

use fm_core::{git, git_native};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::{tempdir, TempDir};

fn have_git() -> bool {
    Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// Point this repo's `merge=fm` at the `fm` we just built — the product does this with the
/// binary beside `fm-serve`, which a test binary in `target/debug/deps` is not.
fn install_driver(repo: &Path) {
    let fm = Path::new(env!("CARGO_BIN_EXE_fm"));
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["config", "merge.fm.driver", &format!("'{}' merge-md %O %A %B %L", fm.display())])
        .output()
        .unwrap();
}

fn no_identity(vault: &Path) {
    for (k, v) in [("user.email", "formicaria@localhost"), ("user.name", "formicaria")] {
        Command::new("git").arg("-C").arg(vault).args(["config", k, v]).output().unwrap();
    }
}

/// A note the app writes, with the `updated:` line that makes every concurrent edit collide.
fn note(updated: &str, status: &str, body: &str) -> String {
    format!(
        "---\nid: 01JQ0000000000000000000000\ntype: task\ntitle: shared\n\
         created: 2026-07-17T10:00:00Z\nupdated: {updated}\nstatus: {status}\n---\n\n{body}"
    )
}

/// Set up a bare remote, plus two clones of it that have both moved on.
///
/// `mine` is driven by the backend under test; `theirs` always by real git, because it stands
/// in for the collaborator whose terminal we do not control.
fn diverged(mine_backend: &str) -> (TempDir, TempDir, TempDir, String) {
    let bare = tempdir().unwrap();
    Command::new("git").args(["init", "--bare"]).arg(bare.path()).output().unwrap();
    let url = bare.path().to_str().unwrap().to_string();

    let origin = tempdir().unwrap();
    fs::create_dir_all(origin.path().join("notes")).unwrap();
    fs::write(
        origin.path().join("notes/01.md"),
        note("2026-07-17T10:00:00Z", "todo", "Line one.\nLine two.\nLine three.\n"),
    )
    .unwrap();
    git::ensure_repo(origin.path()).unwrap();
    install_driver(origin.path());
    no_identity(origin.path());
    git::set_identity(origin.path(), "Origin", "origin@example.org").unwrap();
    git::commit_all(origin.path(), "base", &[origin.path().join("notes/01.md")]).unwrap();
    Command::new("git")
        .arg("-C")
        .arg(origin.path())
        .args(["push", &url, "HEAD:refs/heads/main"])
        .output()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(bare.path())
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .output()
        .unwrap();

    // Ours.
    let mine = tempdir().unwrap();
    fs::remove_dir_all(mine.path()).unwrap();
    if mine_backend == "native" {
        git_native::clone(&url, mine.path()).unwrap();
        git_native::set_identity(mine.path(), "Me", "me@example.org").unwrap();
    } else {
        git::clone(&url, mine.path()).unwrap();
        git::set_identity(mine.path(), "Me", "me@example.org").unwrap();
    }

    // Theirs — always real git, and it pushes first so ours must merge.
    let theirs = tempdir().unwrap();
    fs::remove_dir_all(theirs.path()).unwrap();
    Command::new("git").arg("clone").arg(&url).arg(theirs.path()).output().unwrap();
    for (k, v) in [("user.email", "them@example.org"), ("user.name", "Them")] {
        Command::new("git").arg("-C").arg(theirs.path()).args(["config", k, v]).output().unwrap();
    }
    // Both halves of the driver on every repo that will merge: the attribute travels in the
    // repo, the definition deliberately does not.
    install_driver(mine.path());
    install_driver(theirs.path());
    (bare, mine, theirs, url)
}

fn their_push(theirs: &Path, body: &str, status: &str) {
    fs::write(theirs.join("notes/01.md"), note("2026-07-17T12:00:00Z", status, body)).unwrap();
    for a in [vec!["add", "-A"], vec!["commit", "-m", "theirs"]] {
        Command::new("git").arg("-C").arg(theirs).args(&a).output().unwrap();
    }
    Command::new("git").arg("-C").arg(theirs).args(["push", "origin", "HEAD"]).output().unwrap();
}

/// **The test the whole in-process backend exists to pass.**
///
/// Two people edit different lines of one note. The desktop gets a clean merge because git runs
/// the `.md` driver; libgit2 cannot run a driver at all, so the native backend has to reach the
/// same answer by calling `merge_texts` itself. If these two ever disagree, a phone and a laptop
/// disagree about what a merged note is — which is the corruption this project is built to
/// prevent.
#[test]
fn a_concurrent_edit_merges_the_same_way_on_both_backends() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let mut results = Vec::new();
    for backend in ["subprocess", "native"] {
        let (_bare, mine, theirs, _url) = diverged(backend);
        their_push(theirs.path(), "Line one CHANGED BY THEM.\nLine two.\nLine three.\n", "todo");

        // We edit a *different* line, and the app rewrites `updated:` on every save — which is
        // the collision the driver exists to absorb.
        fs::write(
            mine.path().join("notes/01.md"),
            note(
                "2026-07-17T11:00:00Z",
                "todo",
                "Line one.\nLine two.\nLine three CHANGED BY ME.\n",
            ),
        )
        .unwrap();
        let commit = if backend == "native" { git_native::commit_all } else { git::commit_all };
        commit(mine.path(), "auto: mine", &[mine.path().join("notes/01.md")]).unwrap();

        let pulled = if backend == "native" {
            git_native::pull(mine.path())
        } else {
            git::pull(mine.path())
        }
        .unwrap();
        let merged = fs::read_to_string(mine.path().join("notes/01.md")).unwrap();
        results.push((backend, pulled, merged));
    }

    let (_, sub_pulled, sub_text) = &results[0];
    let (_, nat_pulled, nat_text) = &results[1];

    assert!(
        matches!(sub_pulled, git::Pulled::Merged(_)),
        "subprocess merged cleanly: {sub_pulled:?}"
    );
    assert!(
        matches!(nat_pulled, git::Pulled::Merged(_)),
        "native must too, or the driver gap is real: {nat_pulled:?}"
    );

    for (name, text) in [("subprocess", sub_text), ("native", nat_text)] {
        assert!(text.contains("CHANGED BY THEM"), "{name} kept their edit:\n{text}");
        assert!(text.contains("CHANGED BY ME"), "{name} kept ours:\n{text}");
        assert!(!text.contains("<<<<<<<"), "{name} merged cleanly, no markers:\n{text}");
        fm_core::frontmatter::from_file(text)
            .unwrap_or_else(|e| panic!("{name} produced a readable note: {e}\n{text}"));
    }
    assert_eq!(sub_text, nat_text, "the two backends produced different merged notes");
}

/// And a real disagreement must fail the same way on both: markers **in the body**, the note
/// still parseable, and the path named rather than swallowed.
#[test]
fn a_genuine_conflict_is_reported_the_same_way_on_both_backends() {
    if !have_git() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    for backend in ["subprocess", "native"] {
        let (_bare, mine, theirs, _url) = diverged(backend);
        their_push(theirs.path(), "The disputed line, their way.\n", "todo");

        fs::write(
            mine.path().join("notes/01.md"),
            note("2026-07-17T11:00:00Z", "todo", "The disputed line, my way.\n"),
        )
        .unwrap();
        let commit = if backend == "native" { git_native::commit_all } else { git::commit_all };
        commit(mine.path(), "auto: mine", &[mine.path().join("notes/01.md")]).unwrap();

        let pulled = if backend == "native" {
            git_native::pull(mine.path())
        } else {
            git::pull(mine.path())
        }
        .unwrap();
        match pulled {
            git::Pulled::Conflicted(files) => {
                assert!(
                    files.iter().any(|f| f.contains("01.md")),
                    "{backend} named the note: {files:?}"
                );
            }
            other => panic!("{backend} should have conflicted, got {other:?}"),
        }

        let text = fs::read_to_string(mine.path().join("notes/01.md")).unwrap();
        assert!(text.contains("<<<<<<<"), "{backend} shows the disagreement:\n{text}");
        assert!(
            text.contains("my way") && text.contains("their way"),
            "{backend} keeps both sides:\n{text}"
        );
        // The property the whole `.md` driver exists for: markers in the BODY, so the note
        // still parses and still opens in the editor.
        fm_core::frontmatter::from_file(&text)
            .unwrap_or_else(|e| panic!("{backend} left a readable note: {e}\n{text}"));
    }
}
