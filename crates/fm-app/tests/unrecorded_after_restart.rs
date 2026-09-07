//! **Reproducing "146 notes not in history" on a laptop, with no phone involved.**
//!
//! The phone showed 146 and nobody could say what they were — app-private storage is unreadable on a
//! release build, MIUI eats our logcat tag, and the owner works only through the UI. But the
//! *mechanism* needs no device at all, and this is it:
//!
//! `commit_all` stages exactly the paths `FileStore::put`/`delete` recorded — deliberately, so a vault
//! that is also a project repo never has its owner's staged work swept into an `auto:` commit. That
//! record is **per-process memory**. Android kills backgrounded apps constantly, so on a phone almost
//! every relaunch orphaned whatever the previous run had written: not lagging, *permanently
//! unstageable*, and nothing said so.
//!
//! Dropping and re-opening the `App` is exactly what a restart does to that record, which makes the
//! whole failure reproducible in milliseconds. What is pinned here is the shape of the bug **and** the
//! shape of the fix: `commit_all` alone still records nothing (that precision is not a bug and must not
//! be "fixed"), while `unrecorded` finds them and `record_unrecorded` commits them.

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use std::path::Path;
use std::process::Command;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn have_git() -> bool {
    Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .unwrap()
}

fn call(app: &App, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    dispatch(cmd, &args, &[], app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(serde_json::Value::Null))
}

/// An app over one vault — built fresh each time, which is the point: a new `App` has no memory of
/// what an earlier one wrote, exactly like a relaunched phone.
fn open_app(home: &TempDir, vault: &Path) -> App {
    // **Run the phone's backend when the feature is on** (2026-09-07). Every assertion in this file
    // is about a failure first seen on a phone, and until now not one of them had ever executed
    // against libgit2 — the file was absent from `test-native-git`, so it only ever ran the
    // subprocess backend the phone does not have. That is the same blind spot that let the
    // `git merge-file` shell-out ship: green everywhere, on a path the device could not take.
    //
    // The `git()` helper below still uses the real binary for *setup*, which is right: it stands in
    // for the other device, and what is under test is what the app does, not how the fixture is
    // built.
    #[cfg(feature = "native-git")]
    fm_core::vcs::force_native(true);
    let store = MultiStore::open(&[("v".to_string(), vault.to_path_buf())]).unwrap();
    App::new(
        store,
        vec![VaultConfig { name: "v".into(), path: vault.to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    )
}

#[test]
fn notes_written_before_a_restart_are_found_and_recordable() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // ── Session zero: one note, recorded. ──
    //
    // Needed so the vault has history *and* so `ensure_repo`'s own files (`.gitattributes`,
    // `.gitignore`) are already committed. `commit_all` stages those unconditionally — they must
    // travel or a collaborator's clone loses the merge driver — so on a brand-new repo it has
    // something of its own to commit even when no note is its own. Skipping this step is what made
    // the first draft of this test assert something false about the code.
    {
        let app = open_app(&home, &vault);
        call(&app, "capture", serde_json::json!({ "body": "already recorded", "vault": "v" }))
            .unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    }

    // ── Session one: write three notes through the real command, then let the process end. ──
    {
        let app = open_app(&home, &vault);
        // Through the command door, the same one the phone uses — `App::lock` is private, which is the
        // seam doing its job.
        for body in ["first", "second", "third"] {
            call(&app, "capture", serde_json::json!({ "body": body, "vault": "v" })).unwrap();
        }
    }

    // ── Session two: a fresh App, with no memory of those writes. ──
    let app = open_app(&home, &vault);

    // **This assertion was reversed on 2026-08-20, deliberately — see `decisions.md#data`.**
    //
    // It used to require that `commit` record *nothing* here, on the reasoning that staging only
    // what this process wrote is what protects a project vault from an `auto:` commit sweeping the
    // user's index, and that "a future change that makes this line commit is a regression".
    //
    // The protection is real; attributing it to the *per-process memory* was the error. What
    // actually provides it is the **naming scheme**: `adoptable` stages only
    // `<notes dir>/<ULID>.md`, which is what `FileStore` writes and nothing else produces. The
    // memory contributed nothing to safety and one large cost — whether your work is recorded
    // depended on when the process happened to start. The owner met that as **180 notes not in
    // history in a vault with a remote, which pressing Backup would not clear**, and named the
    // product rule it breaks: *"When I press backup it should commit and push. I do not commit as a
    // user, that is a background concept."*
    //
    // `App::load` had already abandoned the old property at open (it adopts, so the next commit
    // after a relaunch records everything); this test only still asserted it because `open_app`
    // uses `App::new` and skips that path. So the invariant was already gone in production and
    // alive only here.
    //
    // What is asserted instead is the property that was actually being protected, and it is
    // asserted *directly* rather than as a side effect of forgetfulness — see the two files below.
    std::fs::write(vault.join("notes").join("hand-written.md"), "not ours: no ULID name\n")
        .unwrap();
    std::fs::write(vault.join("README.md"), "the project's own file\n").unwrap();

    let committed = call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }))
        .expect("commit must not error");
    assert_eq!(committed["committed"], true, "the three notes must now be recorded: {committed}");

    let tracked = String::from_utf8_lossy(&git(&vault, &["ls-files"]).stdout).to_string();
    assert_eq!(
        tracked.lines().filter(|l| l.ends_with(".md")).count(),
        4,
        "session zero's note plus the three from session one, and nothing else: {tracked}"
    );
    assert!(
        !tracked.contains("hand-written.md"),
        "a file in the notes dir that this app did not name must never be staged: {tracked}"
    );
    assert!(
        !tracked.contains("README.md"),
        "a file outside the notes dir must never be staged: {tracked}"
    );
    // Clean up so the `unrecorded` counts below describe only the notes under test.
    std::fs::remove_file(vault.join("notes").join("hand-written.md")).unwrap();
    std::fs::remove_file(vault.join("README.md")).unwrap();

    // ── The reconciliation surface, on a backlog `commit` has not already cleared. ──
    //
    // Written straight to disk rather than through `capture`, because `commit` now records what it
    // can find: to describe an outstanding backlog the notes have to arrive *after* it.
    for body in ["fourth", "fifth", "sixth"] {
        let o = fm_model::Object::new(fm_model::Kind::Note, body.to_string());
        std::fs::write(
            vault.join("notes").join(format!("{}.md", o.id)),
            fm_core::frontmatter::to_file(&o).unwrap(),
        )
        .unwrap();
    }
    let un = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    assert_eq!(un[0]["vault"], "v");
    assert_eq!(un[0]["count"], 3, "{un}");
    // **All three are `new`.** This is the distinction the phone could not report: three notes that
    // exist nowhere else, not three notes something was rewriting.
    assert_eq!(un[0]["new"], 3, "{un}");
    assert_eq!(un[0]["modified"], 0);
    assert_eq!(un[0]["deleted"], 0);
    // And enough detail to recognise them without a shell: a title or first line, a size, a time.
    let first = &un[0]["notes"][0];
    assert!(first["id"].as_str().is_some_and(|s| !s.is_empty()), "{first}");
    assert_eq!(first["kind"], "new");
    assert!(first["bytes"].as_u64().is_some_and(|b| b > 0), "{first}");
    assert!(first["modified"].as_str().is_some_and(|s| s.contains('T')), "{first}");

    // Recording them is the one-click fix, and it works from a process that never wrote them.
    let rec = call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    assert_eq!(rec["committed"], true, "{rec}");
    assert_eq!(rec["notes"], 3, "{rec}");
    let after = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    assert_eq!(after, serde_json::json!([]), "nothing left unrecorded: {after}");
}

#[test]
fn the_kinds_tell_the_two_stories_apart() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // A committed starting point, written and recorded by one session.
    let id = {
        let app = open_app(&home, &vault);
        let meta = call(&app, "capture", serde_json::json!({ "body": "committed", "vault": "v" }))
            .unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
        meta["id"].as_str().unwrap().to_string()
    };

    // Then, from a fresh process: one note edited, one new note, one committed note deleted.
    std::fs::write(
        vault.join(format!("notes/{id}.md")),
        std::fs::read_to_string(vault.join(format!("notes/{id}.md"))).unwrap() + "\nan edit\n",
    )
    .unwrap();
    std::fs::write(
        vault.join("notes/01NEWNEWNEWNEWNEWNEWNEWNEW.md"),
        "---\nschema: 1\nid: 01NEWNEWNEWNEWNEWNEWNEWNEW\ntype: note\ntitle: Fresh\ncreated: 2026-07-31T00:00:00Z\nupdated: 2026-07-31T00:00:00Z\n---\nbrand new\n",
    )
    .unwrap();

    let app = open_app(&home, &vault);
    let un = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    // **The whole point of this pass**: one number split into two meanings. A pile of `modified` says
    // something is rewriting notes; a pile of `new` says notes exist in one place only.
    assert_eq!(un[0]["new"], 1, "{un}");
    assert_eq!(un[0]["modified"], 1, "{un}");
    // **An untitled note must never be labelled with its frontmatter.** The first version of
    // `title_of` scanned the raw file, so every untitled note came back titled `schema: 1` — seen on
    // the owner's phone within the hour, where it read as "142 malformed notes" and nearly sent the
    // diagnosis down a wrong path.
    let all: Vec<String> = un[0]["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["title"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(
        !all.iter().any(|t| t.starts_with("schema:") || t.starts_with("id:")),
        "a title must come from the body, never from the frontmatter: {all:?}"
    );

    // The titled note is nameable in the list, not just a ULID.
    let titles: Vec<&str> =
        un[0]["notes"].as_array().unwrap().iter().filter_map(|n| n["title"].as_str()).collect();
    assert!(titles.contains(&"Fresh"), "a note is named by its title: {titles:?}");
}

/// **The root-cause fix: a relaunch adopts what the previous session wrote.**
///
/// The chip and the panel make the orphaned notes *visible*; they do not stop them happening. `App`
/// now rebuilds the write-record from the filesystem at open, so the next ordinary commit records
/// them — no click, and no new staging semantics. This drives `App::load`, which is the only path
/// that reconciles, through `FM_VAULTS`/`FM_VAULT` (the same override `fm-serve` documents).
#[test]
fn a_relaunch_adopts_the_previous_session_s_notes_and_the_next_commit_records_them() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // Session zero: one note, recorded, so `.gitattributes`/`.gitignore` are already in history.
    {
        let app = open_app(&home, &vault);
        call(&app, "capture", serde_json::json!({ "body": "recorded", "vault": "v" })).unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    }
    // Session one writes two notes and dies without committing.
    {
        let app = open_app(&home, &vault);
        for body in ["orphan one", "orphan two"] {
            call(&app, "capture", serde_json::json!({ "body": body, "vault": "v" })).unwrap();
        }
    }

    // Session two comes up through `App::load`, which is what a relaunch really does.
    let app = {
        // Safety: single-threaded test setup, before the app reads either variable.
        unsafe {
            std::env::set_var("FM_VAULTS", home.path().join("vaults.json"));
            std::env::set_var("FM_VAULT", &vault);
        }
        let (app, _skipped) = App::load().expect("load");
        app
    };

    // **The ordinary commit records them.** Before this change it answered "nothing of mine changed"
    // and those two notes were unstageable for the life of the vault.
    let out = call(&app, "commit", serde_json::json!({ "message": "auto", "vault": "v" }))
        .expect("commit");
    assert_eq!(out["committed"], true, "the adopted notes were committed: {out}");
    let after = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    assert_eq!(after, serde_json::json!([]), "nothing left outstanding: {after}");
    // And they are really in history, by name.
    let log = String::from_utf8_lossy(&git(&vault, &["log", "--stat", "-1"]).stdout).to_string();
    assert_eq!(log.matches(".md").count(), 2, "both orphans in the commit: {log}");
}

/// **A duplicate count is what turns a number into a cause.**
///
/// The real incident (2026-07-31): 147 notes outstanding on the phone, of which one prompt to the
/// agent appeared ~138 times, every copy timestamped the same minute. "147 unrecorded" reads as a
/// backlog; "one body written 138 times at machine pace" names a loop. The count must key on the
/// **body**, because copies differ in `id` and `created` — hashing the file would report every
/// duplicate as unique and hide the very thing worth seeing.
#[test]
fn duplicates_are_counted_by_body_and_the_role_names_the_code_path() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // Three copies of one message: same body, different ids and timestamps, exactly as a loop leaves
    // them. Plus one ordinary note, so the counts stay honest.
    // **Valid Crockford base32 — no I, L, O or U.** `01ROOT…` looks like a ULID and is not one, so
    // `parse_note_ref` rejects it and `is_message` silently answers false; the first version of this
    // test used it and blamed the code. `mock.ts` carries the same warning about `M0CK` vs `MOCK`.
    let root = "01RTRTRTRTRTRTRTRTRTRTRTRT";
    for (i, id) in
        ["01CCCCCCCCCCCCCCCCCCCCCC01", "01CCCCCCCCCCCCCCCCCCCCCC02", "01CCCCCCCCCCCCCCCCCCCCCC03"]
            .iter()
            .enumerate()
    {
        std::fs::write(
            vault.join(format!("notes/{id}.md")),
            format!(
                "---\nschema: 1\nid: {id}\ntype: note\ncreated: 2026-07-31T07:44:0{i}Z\nupdated: 2026-07-31T07:44:0{i}Z\nthread_of: note:{root}\n---\n@lfm2.5-230m does mRNA change DNA? /search\n"
            ),
        )
        .unwrap();
    }
    std::fs::write(
        vault.join("notes/01PZPZPZPZPZPZPZPZPZPZPZPZ.md"),
        "---\nschema: 1\nid: 01PZPZPZPZPZPZPZPZPZPZPZPZ\ntype: note\ntitle: An ordinary note\ncreated: 2026-07-31T09:00:00Z\nupdated: 2026-07-31T09:00:00Z\n---\nsomething else entirely\n",
    )
    .unwrap();

    let app = open_app(&home, &vault);
    let un = call(&app, "unrecorded", serde_json::json!({})).unwrap();
    let rows = un[0]["notes"].as_array().unwrap();
    assert_eq!(un[0]["count"], 4, "{un}");

    // The three copies each report the whole family, so any one row tells the story.
    let dupes: Vec<u64> = rows
        .iter()
        .filter(|n| n["role"] == "message")
        .map(|n| n["copies"].as_u64().unwrap())
        .collect();
    assert_eq!(dupes, vec![3, 3, 3], "each copy names the size of its family: {rows:?}");
    // **The role names the code path** — a message came from the reply path, not from capture. That
    // distinction is what a title could never carry.
    assert_eq!(rows.iter().filter(|n| n["role"] == "message").count(), 3);

    let plain = rows.iter().find(|n| n["role"] == "note").expect("the ordinary note");
    assert_eq!(plain["copies"], 1, "a unique note is not a duplicate: {plain}");
    assert_eq!(plain["title"], "An ordinary note");
}

/// **A refusal must not be reported as "nothing to do" — and a partial success must not be either.**
///
/// `commit_all` answers `bool`, and it returns `false` for reasons that are worlds apart: nothing of
/// ours moved (the quiet, correct case for a debounced auto-commit) and *git refuses* because a path is
/// unmerged. `record_unrecorded` passed that bool straight up, and the UI printed
/// **"Nothing left to record"** for both — so with notes outstanding and one note mid-merge, the one
/// button that rescues unrecorded notes reported success and did nothing, indefinitely. The owner hit
/// exactly this on the phone with 147 outstanding (2026-07-31).
///
/// What is pinned, **restated 2026-09-07 when `commit_all` stopped refusing wholesale**: with notes
/// found and a merge in flight, the recordable notes are *recorded* (`committed` is now true, and the
/// note is verifiably in the commit), `notes` is not zero, the conflicted note stays out, and `reason`
/// still names the merge. That last one matters more than it did: the reason used to appear only on a
/// refusal, and a refusal is now the rare case — so a message conditioned on failure would be silent
/// exactly when the user needs it, with the outstanding count dropping to zero as if all were well.
#[test]
fn recording_gets_past_a_merge_and_still_names_what_it_left_behind() {
    if !have_git() {
        eprintln!("skipped: no git");
        return;
    }
    let home = tempdir().unwrap();
    let vault = home.path().join("v");
    std::fs::create_dir_all(vault.join("notes")).unwrap();
    git(&vault, &["init", "-q", "-b", "main"]);
    git(&vault, &["config", "user.name", "T"]);
    git(&vault, &["config", "user.email", "t@e.com"]);

    // Session zero, as above: history plus `ensure_repo`'s own files committed.
    {
        let app = open_app(&home, &vault);
        call(&app, "capture", serde_json::json!({ "body": "the shared note", "vault": "v" }))
            .unwrap();
        call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    }
    let rel = {
        let mut found = std::fs::read_dir(vault.join("notes"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        found.sort();
        format!("notes/{}", found.pop().expect("the recorded note"))
    };

    // A delete/modify merge in flight — the kind with no markers, which is how the owner's froze.
    git(&vault, &["checkout", "-q", "-b", "theirs"]);
    std::fs::write(vault.join(&rel), "---\nschema: 1\ntype: note\n---\ntheir edit\n").unwrap();
    git(&vault, &["commit", "-qam", "their edit"]);
    git(&vault, &["checkout", "-q", "main"]);
    git(&vault, &["rm", "-q", &rel]);
    git(&vault, &["commit", "-qm", "our delete"]);
    git(&vault, &["merge", "theirs"]);
    assert!(vault.join(".git/MERGE_HEAD").exists(), "precondition: mid-merge");

    // And an unrecorded note, written by a process that is now gone.
    std::fs::write(
        vault.join("notes/01BBBBBBBBBBBBBBBBBBBBBBBB.md"),
        "---\nschema: 1\nid: 01BBBBBBBBBBBBBBBBBBBBBBBB\ntype: note\ncreated: 2026-07-31T07:44:00Z\nupdated: 2026-07-31T07:44:00Z\n---\nforgotten\n",
    )
    .unwrap();

    let app = open_app(&home, &vault);
    let rec = call(&app, "record_unrecorded", serde_json::json!({ "vault": "v" })).unwrap();
    // **The forgotten note is recorded, conflict or no conflict.** Until 2026-09-07 this asserted
    // the opposite — `committed: false`, "git refuses over an unmerged path" — which was the
    // behaviour and was the 39-day freeze: one stuck note held every other note out of history.
    assert_eq!(rec["committed"], true, "the notes that can be recorded are: {rec}");
    // The load-bearing pair: it *found* notes. `notes: 0` is what the UI reads as "nothing to record",
    // and reporting zero here is the lie that hid the problem.
    assert!(rec["notes"].as_u64().is_some_and(|n| n > 0), "it found work to do: {rec}");
    assert!(
        String::from_utf8_lossy(&git(&vault, &["show", "--name-only", "--format=", "HEAD"]).stdout)
            .contains("01BBBBBBBBBBBBBBBBBBBBBBBB"),
        "and the forgotten note is actually in the commit, not merely claimed"
    );

    // **And the conflict is still named, precisely because the commit succeeded.** A message that
    // only appears when nothing was committed says nothing in the case that now happens, and the
    // count dropping to zero would read as "all clear" while two notes sit stuck for ever.
    let reason = rec["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("mid-merge"), "the reason names the merge: {reason:?}");
    // And it points at the surface that can actually clear it, rather than merely declining.
    assert!(reason.contains("Conflicts"), "and at the way out: {reason:?}");
    // The conflicted note itself is *not* in history — committing it would publish the merge.
    assert!(
        !String::from_utf8_lossy(
            &git(&vault, &["show", "--name-only", "--format=", "HEAD"]).stdout
        )
        .contains(rel.trim_start_matches("notes/")),
        "the conflicted note must stay out until a human settles it"
    );
}
