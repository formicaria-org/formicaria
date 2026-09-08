//! **The scope, asserted at the door rather than at the store.**
//!
//! `fm_core::Scoped` is tested directly in `fm-core/tests/scoped.rs`; those tests prove the
//! mechanism. These prove the *wiring* — that every command actually goes through it — which is
//! the half that rots. A read path added later that reaches `Vaults::all` instead of
//! `Vaults::store(scope)` would leave every `Scoped` test passing and still leak.
//!
//! So these go through `dispatch_as`, exactly as `fm-serve` will for a paired device, and ask
//! the questions a tablet would ask.

use fm_app::{dispatch, dispatch_as, vaults::VaultConfig, App, Host, Output, Scope};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn as_scope(app: &App, scope: &Scope, cmd: &str, args: Value) -> Result<Value, String> {
    dispatch_as(cmd, &args, &[], app, &NoHost, scope).map(|o| match o {
        Output::Json(b) if b.is_empty() => Value::Null,
        o => serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null),
    })
}

/// `personal` (default) + `lab`, one note in each. Returns the ids in that order.
fn two_vaults() -> (TempDir, TempDir, TempDir, App, String, String) {
    let home = tempdir().unwrap();
    let a = tempdir().unwrap();
    let b = tempdir().unwrap();
    for d in [&a, &b] {
        std::fs::create_dir_all(d.path().join("notes")).unwrap();
    }
    let owned: Vec<(String, PathBuf)> =
        vec![("personal".into(), a.path().to_path_buf()), ("lab".into(), b.path().to_path_buf())];
    let configs: Vec<VaultConfig> = owned
        .iter()
        .map(|(n, p)| VaultConfig { name: n.clone(), path: p.clone(), restic: None })
        .collect();
    let app = App::new(
        MultiStore::open(&owned).unwrap(),
        configs,
        Some(home.path().join("vaults.json")),
        true,
    );

    let mine = dispatch(
        "capture",
        &json!({ "body": "a private thought\n", "vault": "personal" }),
        &[],
        &app,
        &NoHost,
    )
    .unwrap();
    let mine: Value = serde_json::from_slice(&mine.into_bytes()).unwrap();
    let theirs = dispatch(
        "capture",
        &json!({ "body": "the shared experiment\n", "vault": "lab" }),
        &[],
        &app,
        &NoHost,
    )
    .unwrap();
    let theirs: Value = serde_json::from_slice(&theirs.into_bytes()).unwrap();

    let (mine, theirs) =
        (mine["id"].as_str().unwrap().to_string(), theirs["id"].as_str().unwrap().to_string());
    (home, a, b, app, mine, theirs)
}

fn lab() -> Scope {
    Scope::Only(vec!["lab".into()])
}

/// The views a tablet actually opens. Each is a separate assertion because each reaches storage
/// by its own route, and it only takes one of them bypassing `store(scope)` to leak everything.
#[test]
fn every_view_is_scoped() {
    let (_h, _a, _b, app, _, _) = two_vaults();

    let recent = as_scope(&app, &lab(), "recent", json!({})).unwrap();
    let bodies: Vec<&str> =
        recent.as_array().unwrap().iter().filter_map(|o| o["preview"].as_str()).collect();
    assert_eq!(bodies.len(), 1, "recent leaked another audience: {bodies:?}");

    let hits = as_scope(&app, &lab(), "search", json!({ "query": "thought" })).unwrap();
    assert_eq!(
        hits.as_array().unwrap().len(),
        0,
        "full-text search reached into a vault the caller cannot see"
    );

    let board = as_scope(&app, &lab(), "board", json!({ "groupBy": "status" })).unwrap();
    let carded: usize = board["columns"]
        .as_array()
        .map(|cols| cols.iter().map(|c| c["cards"].as_array().map_or(0, Vec::len)).sum())
        .unwrap_or(0);
    assert_eq!(carded, 1, "the board showed a note from another audience");

    let agenda = as_scope(&app, &lab(), "agenda", json!({})).unwrap();
    assert!(
        !serde_json::to_string(&agenda).unwrap().contains("private thought"),
        "agenda leaked: {agenda}"
    );
}

/// The vault list is what builds the switcher, the copy-to menu and the create-target picker.
/// A scoped client must not even learn the other audiences exist — the *names* disclose.
#[test]
fn the_vault_list_shows_only_what_the_caller_can_reach() {
    let (_h, _a, _b, app, _, _) = two_vaults();

    let all = as_scope(&app, &Scope::All, "list_vaults", json!({})).unwrap();
    assert_eq!(all.as_array().unwrap().len(), 2, "the machine's own user still sees both");

    let mine = as_scope(&app, &lab(), "list_vaults", json!({})).unwrap();
    let names: Vec<&str> =
        mine.as_array().unwrap().iter().filter_map(|v| v["name"].as_str()).collect();
    assert_eq!(names, vec!["lab"], "a scoped client saw a vault it has no access to");
}

/// **The backup panel discloses more than the switcher does, and it was not scoped at all.**
///
/// `list_vaults` was filtered from the start because *a vault's name discloses* — and
/// `backup_status` carries the name **plus** the remote URL, the committer's name and email, the
/// unpushed count and the conflicted paths, for every vault on the machine. It reached
/// `configs()` directly instead of taking the scope, so a paired device granted one audience could
/// read where every other audience is hosted and who signs it.
///
/// Exactly the rot the header of this file predicts: a read path that reaches the vault list
/// rather than `Vaults::store(scope)` leaves every `Scoped` mechanism test passing and still
/// leaks. Found on 2026-09-08 while chasing an unrelated question, by noticing that its own
/// sibling one line below — `backup_latest(app, scope, ..)` — does take the scope.
#[test]
fn the_backup_panel_shows_only_the_vaults_the_caller_can_reach() {
    let (_h, _a, _b, app, _, _) = two_vaults();

    let all = as_scope(&app, &Scope::All, "backup_status", json!({})).unwrap();
    assert_eq!(
        all["vaults"].as_array().unwrap().len(),
        2,
        "the machine's own user still sees both"
    );

    let mine = as_scope(&app, &lab(), "backup_status", json!({})).unwrap();
    let names: Vec<&str> =
        mine["vaults"].as_array().unwrap().iter().filter_map(|v| v["name"].as_str()).collect();
    assert_eq!(names, vec!["lab"], "a scoped client saw a vault it has no access to");

    // The names are the cheapest thing here to check and the least of what leaked, so the
    // serialised answer is checked whole: a remote URL or a committer's email reaching a caller
    // that cannot read the vault is the disclosure, whatever field it arrives in.
    let text = serde_json::to_string(&mine).unwrap();
    assert!(!text.contains("personal"), "the other audience appears somewhere in:\n{text}");
}

/// **Every arm that hands back a vault list, not just the one that was noticed.** `backup_status`
/// was found by accident; the sweep that followed found eight more places returning the same shape
/// unfiltered — `config` (which also lists each vault's backup repo, a path and often a host), the
/// two settings writes that return the refreshed list, and the four vault-lifecycle commands.
///
/// They are asserted together because they failed together and for one reason: the filter was two
/// lines duplicated per site rather than a function, so each was written correctly in isolation and
/// nothing checked the sum. `scoped_infos` is that function now, and this is the test that notices
/// when the ninth arm forgets to call it.
#[test]
fn no_arm_hands_back_a_vault_list_the_caller_may_not_see() {
    let (_h, a, _b, app, _, _) = two_vaults();

    // **`unrecorded` needs a git repo to have anything to leak.** Without one it answers `[]`
    // whatever the filter does, and the assertion below would pass for the wrong reason — the trap
    // this sweep exists to avoid. `personal` becomes a repo whose note is untracked, so the
    // unscoped caller genuinely has something to disclose.
    let git = std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if git {
        fm_core::git::ensure_repo(a.path()).expect("the fixture vault must become a repo");
        let all = as_scope(&app, &Scope::All, "unrecorded", json!({})).unwrap();
        assert!(
            serde_json::to_string(&all).unwrap().contains("personal"),
            "precondition: unscoped, there is something to disclose:\n{all}"
        );
    }

    for (cmd, args) in [
        ("list_vaults", json!({})),
        ("backup_status", json!({})),
        ("config", json!({})),
        ("set_git_assets_max", json!({ "vault": "lab", "max": "" })),
        ("set_supervision", json!({ "vault": "lab", "collect": true, "publish": false })),
        ("conflicts", json!({})),
        ("unrecorded", json!({})),
        ("check_path", json!({ "name": "lab", "path": "/tmp/fm-scope-probe" })),
        ("forget_vault", json!({ "name": "lab" })),
    ] {
        let out = as_scope(&app, &lab(), cmd, args).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        let text = serde_json::to_string(&out).unwrap();
        assert!(
            !text.contains("personal"),
            "`{cmd}` disclosed a vault outside the caller's scope:\n{text}"
        );
    }
}

/// **A vault a caller cannot see is a vault it cannot remove.** `forget_vault` looks its target up
/// in the *unfiltered* list, so a device paired to one audience could unregister another one — a
/// **write** across the boundary, where every other leak found on 2026-09-08 was a read. The fix
/// that day scoped only the list the command hands *back*.
///
/// Refused the way `Scope` refuses everything else: an unknown vault and an out-of-scope vault give
/// the identical error, so a caller cannot probe for names it was not given.
///
/// Proven red by looking the target up in `g.configs()` without `scope.allows`.
#[test]
fn a_vault_the_caller_cannot_see_cannot_be_removed() {
    let (_h, _a, _b, app, _, _) = two_vaults();

    let err = as_scope(&app, &lab(), "forget_vault", json!({ "name": "personal" }))
        .expect_err("a scoped caller must not be able to forget another audience's vault");
    assert!(
        !err.contains("personal") || err.contains("no vault named"),
        "and the refusal must not confirm the name exists: {err}"
    );

    // Still there, and still the machine's own user's to remove.
    let all = as_scope(&app, &Scope::All, "list_vaults", json!({})).unwrap();
    assert_eq!(all.as_array().unwrap().len(), 2, "nothing was removed");
    assert!(as_scope(&app, &Scope::All, "forget_vault", json!({ "name": "personal" })).is_ok());
}

/// **`check_path` answers from the whole list**, so `name_taken` / `path_taken` / `overlaps` tell a
/// scoped caller that a vault it cannot see exists — and `overlaps` names it outright. The
/// new-vault form is exactly where a caller would go looking.
///
/// Proven red by reading `v.list` rather than the caller's slice.
#[test]
fn the_new_vault_form_does_not_confirm_vaults_the_caller_cannot_see() {
    let (_h, a, _b, app, _, _) = two_vaults();

    let out = as_scope(
        &app,
        &lab(),
        "check_path",
        json!({ "name": "personal", "path": a.path().to_string_lossy() }),
    )
    .unwrap();
    assert_eq!(out["name_taken"], false, "the name of an unseen vault is not confirmed:\n{out}");
    assert_eq!(out["path_taken"], false, "nor its folder:\n{out}");
    assert!(out["overlaps"].is_null(), "and it is certainly not named:\n{out}");
}

/// Reaching a note by id, which is the shape every deep link and every `note:<ulid>` reference
/// has. The id is not a secret and travels freely; membership of the audience is the check.
#[test]
fn a_note_in_another_vault_is_not_reachable_by_id() {
    let (_h, _a, _b, app, private_id, shared_id) = two_vaults();

    let ours = as_scope(&app, &lab(), "get", json!({ "id": &shared_id })).unwrap();
    assert!(ours["id"].is_string(), "its own note still resolves");

    let theirs = as_scope(&app, &lab(), "get", json!({ "id": &private_id }));
    match theirs {
        Err(_) => {}
        Ok(v) => assert!(v.is_null(), "a note from another audience was served by id: {v}"),
    }
}

/// Writes, both directions: a capture with no stated vault must land in the caller's own, and
/// naming someone else's must be refused rather than quietly redirected.
#[test]
fn writes_cannot_cross_the_boundary() {
    let (_h, _a, _b, app, _, _) = two_vaults();

    let made = as_scope(&app, &lab(), "capture", json!({ "body": "from the tablet\n" })).unwrap();
    assert_eq!(
        made["vault"].as_str(),
        Some("lab"),
        "an unstated audience must land in the caller's own, not the machine's default"
    );

    let smuggled = as_scope(&app, &lab(), "capture", json!({ "body": "x\n", "vault": "personal" }));
    assert!(smuggled.is_err(), "naming another audience must be refused, not redirected");

    // And the private vault is untouched: still exactly its original note.
    let all = as_scope(&app, &Scope::All, "recent", json!({})).unwrap();
    let personal =
        all.as_array().unwrap().iter().filter(|o| o["vault"].as_str() == Some("personal")).count();
    assert_eq!(personal, 1, "something was written into an audience the caller was not in");
}

/// Deleting is the irreversible one, and `delete` searches every vault for the id.
#[test]
fn deleting_across_the_boundary_is_refused() {
    let (_h, _a, _b, app, private_id, _) = two_vaults();

    let gone = as_scope(&app, &lab(), "delete", json!({ "id": &private_id }));
    assert!(gone.is_err(), "a scoped client deleted a note from another audience");

    let still = as_scope(&app, &Scope::All, "get", json!({ "id": &private_id })).unwrap();
    assert!(still["id"].is_string(), "and the note is still there");
}

/// `Scope::All` must be *exactly* what every caller got before scopes existed — `fm-cli`, the
/// phone, the study agent and the desktop's own browser all still come through `dispatch`.
/// If this ever fails, the feature has cost the machine's own user something.
#[test]
fn the_unscoped_caller_is_unchanged() {
    let (_h, _a, _b, app, private_id, shared_id) = two_vaults();

    for id in [&private_id, &shared_id] {
        let got = as_scope(&app, &Scope::All, "get", json!({ "id": id })).unwrap();
        assert!(got["id"].is_string(), "the machine's own user must reach every vault");
    }
    let recent = as_scope(&app, &Scope::All, "recent", json!({})).unwrap();
    assert_eq!(recent.as_array().unwrap().len(), 2);

    // And the plain `dispatch` entry point is the same thing, since that is what every existing
    // shell calls and none of them were changed.
    let via_plain = dispatch("recent", &json!({}), &[], &app, &NoHost).unwrap();
    let via_plain: Value = serde_json::from_slice(&via_plain.into_bytes()).unwrap();
    assert_eq!(via_plain.as_array().unwrap().len(), 2);
}

/// A scope naming a vault that no longer exists — revoked, renamed, deleted — must collapse to
/// nothing rather than falling back to the default. "No match" becoming "all of them" is the
/// classic shape of this bug.
#[test]
fn a_scope_over_a_missing_vault_grants_nothing() {
    let (_h, _a, _b, app, _, _) = two_vaults();
    let ghost = Scope::Only(vec!["archived".into()]);

    let recent = as_scope(&app, &ghost, "recent", json!({})).unwrap();
    assert!(recent.as_array().unwrap().is_empty(), "nothing is readable");

    let vaults = as_scope(&app, &ghost, "list_vaults", json!({})).unwrap();
    assert!(vaults.as_array().unwrap().is_empty());

    assert!(
        as_scope(&app, &ghost, "capture", json!({ "body": "nowhere\n" })).is_err(),
        "a capture with no reachable default must be loud, not filed in vaults[0]"
    );
}
