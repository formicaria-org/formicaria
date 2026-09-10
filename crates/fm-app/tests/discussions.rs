//! Discussions as first-class objects. A discussion is a note whose `thread_of` points at
//! **itself** — the self-anchor. That one move makes it an `is_message` note (so it drops out of
//! every planning view for free), while `is_discussion_root` tells it apart from a reply. Proven:
//! 1. a new discussion is self-rooted and absent from the note/planning views, present in `discussions()`;
//! 2. a reply joins it, and `thread()` shows the reply but **not** the discussion root itself;
//! 3. `discussions()` orders by activity, not creation;
//! 4. "who left a message" is read from git (real commits).

use fm_app::commands::{
    agenda, board, create_discussion, discussion_participants, discussions, recent, reply, thread,
};
use fm_core::MemoryStore;

#[test]
fn a_discussion_is_self_rooted_and_absent_from_the_planning_views() {
    let mut s = MemoryStore::new();
    let d = create_discussion(&mut s, "Should we drop the second arm?", "").unwrap();

    // Self-anchored: `thread_of` points at its own id — a reference, not a forgeable bool.
    assert_eq!(d.props.get("thread_of").unwrap(), &serde_json::json!(format!("note:{}", d.id)));

    // It is a discussion (listed on the Discussions surface)...
    let listed: Vec<String> = discussions(&s).unwrap().into_iter().map(|x| x.root.id).collect();
    assert_eq!(listed, vec![d.id.clone()]);
    // ...and therefore not a note you plan: absent from the timeline/`/`-picker, board and agenda.
    assert!(recent(&s).unwrap().is_empty(), "a discussion never appears in the note list/picker");
    let cards: usize = board(&s, "status").unwrap().columns.iter().map(|c| c.cards.len()).sum();
    assert_eq!(cards, 0, "nor on the board");
    assert!(agenda(&s).unwrap().is_empty(), "nor the agenda");
}

#[test]
fn a_reply_joins_the_discussion_and_the_root_is_not_in_it() {
    let mut s = MemoryStore::new();
    let d = create_discussion(&mut s, "Weekly sync", "").unwrap();

    let m = reply(&mut s, &d.id, "kicking it off").unwrap();
    assert_eq!(
        m.props.get("thread_of").unwrap(),
        &serde_json::json!(format!("note:{}", d.id)),
        "a reply to a self-rooted discussion joins that discussion"
    );

    let t = thread(&s, &d.id).unwrap();
    assert!(t.root.is_some(), "the discussion is its own root");
    assert_eq!(t.count, 1, "one message");
    assert_eq!(t.messages[0].body, "kicking it off");
    assert!(
        t.messages.iter().all(|msg| msg.meta.id != d.id),
        "the root is never a message in its own thread"
    );

    assert_eq!(discussions(&s).unwrap()[0].count, 1, "the summary counts the reply");
}

#[test]
fn discussions_are_listed_most_recently_active_first() {
    let mut s = MemoryStore::new();
    let a = create_discussion(&mut s, "older", "").unwrap();
    let _b = create_discussion(&mut s, "newer", "").unwrap();

    // Activity, not creation, decides order — a late reply lifts the older discussion to the top.
    reply(&mut s, &a.id, "a late reply").unwrap();

    let order: Vec<String> = discussions(&s).unwrap().into_iter().map(|d| d.root.id).collect();
    assert_eq!(order.first(), Some(&a.id), "the just-replied discussion is most recent");
}

#[test]
fn an_empty_title_is_refused() {
    let mut s = MemoryStore::new();
    assert!(create_discussion(&mut s, "   ", "").is_err(), "a discussion needs a title");
}

/// "Who left a message" is git authorship (Ruling 14) — derived, never stored. Real git, real
/// commits: the contract under test is with `git log`.
#[test]
fn who_left_a_message_is_read_from_git() {
    use fm_core::FileStore;
    if std::process::Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut store = FileStore::open(dir.path()).unwrap();
    fm_core::git::ensure_repo(dir.path()).unwrap();
    fm_core::git::set_identity(dir.path(), "Ada Lovelace", "ada@example.org").unwrap();

    let d = create_discussion(&mut store, "the topic", "").unwrap();
    reply(&mut store, &d.id, "first").unwrap();
    let paths = store.written();
    assert!(fm_core::git::commit_all(dir.path(), "seed", &paths).unwrap());

    let who = discussion_participants(&store, dir.path(), "@0").unwrap();
    // **When this goes red, say what git actually had.** `known-issues.md` has carried this test as
    // an intermittent failure with an *unidentified* mechanism since 2026-08: the assertion said
    // only "the discussion has participants", which tells the next reader nothing about whether the
    // commit was empty, the log was empty, or the mapping dropped it. These three lines are the
    // difference between another sighting and a diagnosis.
    let parts = who.get(&d.id).unwrap_or_else(|| {
        let log = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args([
                "log",
                "--no-merges",
                "--since=@0",
                "--pretty=format:%H %an <%ae>",
                "--name-only",
            ])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_else(|e| format!("<git log failed: {e}>"));
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["status", "--porcelain"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        panic!(
            "the discussion has participants\n\
             discussion id: {}\n\
             paths handed to commit_all ({}): {:?}\n\
             participants map keys: {:?}\n\
             --- git log --since=@0 --name-only ---\n{}\n\
             --- git status --porcelain ---\n{}",
            d.id,
            paths.len(),
            paths,
            who.keys().collect::<Vec<_>>(),
            log,
            status,
        )
    });
    assert!(parts.iter().any(|p| p.email == "ada@example.org"), "the poster is listed");
}
