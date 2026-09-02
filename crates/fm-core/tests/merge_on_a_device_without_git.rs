//! **The phone, reproduced: a body merge on a device with no `git` binary at all.**
//!
//! This is the regression test for a bug that shipped and froze vaults. `merge.rs`'s
//! `text_3way` shelled out to `git merge-file` unconditionally — no `cfg`, no backend
//! routing — while `git_native::pull` called it with `?` for every path libgit2 left
//! conflicted. The phone has no `git` (verified on the owner's device; `known-issues.md`
//! external fact #0), so any pull that had to merge prose died with
//! `could not run git merge-file`, **mid-merge**, leaving a `MERGE_HEAD` that
//! `git_native::commit_all` then refuses to commit past. The vault stops accepting writes.
//!
//! **Why no existing test caught it.** Every native-backend test runs on a developer machine
//! or in CI, where `git` *is* on PATH — so `text_3way` quietly succeeded through the very
//! binary the phone lacks, and the whole suite went green on a path the phone could not take.
//! `force_native(true)` did not help: it routes `vcs::` calls, and the body engine was not
//! routed at all. The only honest reproduction is a process where `git` genuinely cannot be
//! found, which is what this file is.
//!
//! It is its own test binary on purpose. `git::available()` is a `OnceLock`, so the PATH has
//! to be cleared before anything in the process asks — and one test per binary is what makes
//! clearing a process-global environment variable sound.

#![cfg(feature = "native-git")]

use fm_core::merge::{merge_texts, Merged};

/// Three versions of one note: a base, and two people editing different paragraphs of it —
/// the ordinary concurrent edit, and the case `merge.rs` exists to resolve.
fn triple(ours_line: &str, theirs_line: &str) -> (String, String, String) {
    let note = |updated: &str, second: &str| {
        format!(
            "---\nid: 01JQ0000000000000000000000\ntype: note\ntitle: t\n\
             created: 2026-07-17T10:00:00Z\nupdated: {updated}\n---\n\n\
             first paragraph, untouched\n\n{second}\n\nlast paragraph, untouched\n"
        )
    };
    (
        note("2026-07-17T10:00:00Z", "the original middle"),
        note("2026-07-17T11:00:00Z", ours_line),
        note("2026-07-17T12:00:00Z", theirs_line),
    )
}

#[test]
fn a_body_merge_works_on_a_device_with_no_git_binary() {
    // First statement in the process, before anything can populate `git::available()`'s
    // `OnceLock` or spawn a thread that reads the environment.
    std::env::set_var("PATH", "");

    assert!(
        !fm_core::git::available(),
        "this test is meaningless unless `git` is genuinely unreachable — \
         PATH was cleared but a binary was still found"
    );

    // Both sides edited the *same* middle paragraph differently: libgit2's index would leave
    // this path conflicted, and the prose then reaches the text engine. This is the exact
    // shape that used to return Err on the phone.
    let (base, ours, theirs) = triple("my version of the middle", "their version of the middle");
    let (text, verdict) = merge_texts(&base, &ours, &theirs, 7)
        .expect("a body merge must not need a `git` binary — this is the bug");

    assert_eq!(verdict, Merged::Conflicted, "two edits to one paragraph are a real conflict");
    assert!(text.contains("<<<<<<< ours"), "the markers carry our label:\n{text}");
    assert!(text.contains(">>>>>>> theirs"), "the markers carry their label:\n{text}");
    // The property the whole module exists for: markers land in the body, so the note still
    // parses and still opens in the editor rather than dropping out of the index.
    fm_core::frontmatter::from_file(&text)
        .expect("a conflicted note must still parse — the markers belong in the body");

    // And the clean case, which is the one a user actually notices: two people, two different
    // paragraphs, one note, no conflict.
    let base = "alpha\nbravo\ncharlie\ndelta\necho\n";
    let ours = "alpha CHANGED\nbravo\ncharlie\ndelta\necho\n";
    let theirs = "alpha\nbravo\ncharlie\ndelta\necho CHANGED\n";
    let (text, verdict) = merge_texts(base, ours, theirs, 7).expect("clean merge without git");
    assert_eq!(verdict, Merged::Clean);
    assert_eq!(text, "alpha CHANGED\nbravo\ncharlie\ndelta\necho CHANGED\n");
}
