//! **A vault is an audience, so a caller is a member of some and not others.**
//!
//! `MultiStore` deliberately sees everything: it is what lets you read across a boundary you
//! cannot accidentally *write* across, and it is right for the person sitting at the machine.
//! It became wrong the moment a second device could ask, because every read path — board,
//! agenda, search, recent, `.view` files, and a bare `get` by id — is federated over the whole
//! set.
//!
//! `Scoped` is the same store with a guest list. These tests are the guest list's specification,
//! and they are deliberately written as *leak* tests: each one names a way an audience boundary
//! could be crossed and asserts it is not. A test that only checked "the allowed vault works"
//! would pass just as happily against a `Scoped` that enforced nothing at all.

use fm_core::{MultiStore, Reindex, Scoped, Store, StoreError};
use fm_model::{Kind, Object};
use fm_query::Query;
use tempfile::{tempdir, TempDir};

/// Two vaults — `personal` (the default, first) and `lab` — each with one note.
/// Returns the store, the note ids, and the tempdirs that must outlive it.
fn two_vaults() -> (MultiStore, String, String, TempDir, TempDir) {
    let a = tempdir().unwrap();
    let b = tempdir().unwrap();
    std::fs::create_dir_all(a.path().join("notes")).unwrap();
    std::fs::create_dir_all(b.path().join("notes")).unwrap();
    let mut store = MultiStore::open(&[
        ("personal".to_string(), a.path().to_path_buf()),
        ("lab".to_string(), b.path().to_path_buf()),
    ])
    .unwrap();

    let mut mine = Object::new(Kind::Note, "a private thought\n");
    mine.vault = "personal".into();
    store.put(&mine).unwrap();

    let mut theirs = Object::new(Kind::Note, "the shared experiment\n");
    theirs.vault = "lab".into();
    store.put(&theirs).unwrap();

    (store, mine.id.to_string(), theirs.id.to_string(), a, b)
}

fn all(store: &dyn Store) -> Vec<String> {
    // A default `Query` carries an empty filter, which matches everything — so this asks the
    // widest question there is, which is the right shape for a leak test.
    store.query(&Query::default()).unwrap().rows.into_iter().map(|o| o.body).collect()
}

/// The baseline both ways: unscoped sees everything, scoped sees its own vault only. If the
/// first half ever fails, `Scoped` has broken the desktop; if the second fails, it leaks.
#[test]
fn a_scope_narrows_what_a_query_can_reach() {
    let (mut store, _, _, _a, _b) = two_vaults();

    let unscoped = Scoped::new(&mut store, None);
    let seen = all(&unscoped);
    assert_eq!(seen.len(), 2, "the unrestricted view is the machine's own, and sees both");

    let lab = ["lab".to_string()];
    let scoped = Scoped::new(&mut store, Some(&lab));
    let seen = all(&scoped);
    assert_eq!(seen, vec!["the shared experiment\n"], "a scoped read sees one audience");
    assert_eq!(scoped.names(), vec!["lab"]);
}

/// **The leak the ULID makes easy.** Ids are globally unique, so an id from another audience
/// resolves perfectly against the full store — and ids travel: a `note:<ulid>` link inside a
/// shared note, a screenshot, a git log. Reaching a note by id must be scoped by *where the
/// note lives*, not by whether the caller happens to know its name.
#[test]
fn an_id_from_another_audience_does_not_resolve() {
    let (mut store, private_id, shared_id, _a, _b) = two_vaults();
    let lab = ["lab".to_string()];
    let scoped = Scoped::new(&mut store, Some(&lab));

    let theirs = scoped.get(shared_id.parse().unwrap()).unwrap();
    assert!(theirs.is_some(), "its own vault still answers");

    let mine = scoped.get(private_id.parse().unwrap()).unwrap();
    assert!(
        mine.is_none(),
        "a note in an unshared vault must not be readable by id — knowing the id is not \
         membership of the audience"
    );
}

/// **Where an unstated audience lands.** Every fresh capture arrives with no vault, and the
/// unscoped path files those in `vaults[0]`. For a scoped caller that would write a note into a
/// vault it cannot even read — and `git` then makes the disclosure permanent.
#[test]
fn a_capture_with_no_vault_lands_in_the_callers_own() {
    let (mut store, _, _, _a, _b) = two_vaults();
    let lab = ["lab".to_string()];

    let fresh = Object::new(Kind::Note, "typed on the tablet\n");
    assert_eq!(fresh.vault, "", "a fresh capture states no audience — that is the premise");

    let id = fresh.id;
    {
        let mut scoped = Scoped::new(&mut store, Some(&lab));
        scoped.put(&fresh).unwrap();
    }

    // It is in `lab`, and it says so — a file whose location and frontmatter disagreed would be
    // its own bug the next time anything routed on it.
    let landed = store.get(id).unwrap().expect("the note was written");
    assert_eq!(landed.vault, "lab", "it must land in the caller's default, not the machine's");
    assert_eq!(
        all(&Scoped::new(&mut store, Some(&["personal".to_string()]))).len(),
        1,
        "and personal still holds only its own original note"
    );
}

/// Naming another audience explicitly is refused, rather than silently redirected. Silently
/// moving it would be the same disclosure with a friendlier error.
#[test]
fn writing_into_an_unshared_vault_is_refused() {
    let (mut store, _, _, _a, _b) = two_vaults();
    let lab = ["lab".to_string()];

    let mut note = Object::new(Kind::Note, "smuggled\n");
    note.vault = "personal".into();
    let id = note.id;

    let mut scoped = Scoped::new(&mut store, Some(&lab));
    assert!(matches!(scoped.put(&note), Err(StoreError::Io(_))), "refused, not redirected");
    drop(scoped);

    assert!(store.get(id).unwrap().is_none(), "and nothing was written anywhere");
}

/// A note you cannot read, you cannot destroy. `delete` fans across vaults looking for the id,
/// which without a scope is a way to delete out of an audience you were never shown.
#[test]
fn deleting_out_of_an_unshared_vault_is_refused() {
    let (mut store, private_id, shared_id, _a, _b) = two_vaults();
    let lab = ["lab".to_string()];
    let private: fm_model::Id = private_id.parse().unwrap();

    {
        let mut scoped = Scoped::new(&mut store, Some(&lab));
        assert!(
            matches!(scoped.delete(private), Err(StoreError::NotFound(_))),
            "a note in an unshared vault is Not Found, which is also all a caller should learn"
        );
    }
    assert!(store.get(private).unwrap().is_some(), "and it is still there");

    // Its own vault still works, or the scope would be a read-only cage rather than an audience.
    let mut scoped = Scoped::new(&mut store, Some(&lab));
    scoped.delete(shared_id.parse().unwrap()).unwrap();
}

/// Unreadable notes are vault-shaped news too: a scoped client learning that `personal` has a
/// conflicted merge in it has learned a filename out of an audience it was never given.
#[test]
fn skipped_notes_are_scoped_too() {
    let a = tempdir().unwrap();
    let b = tempdir().unwrap();
    std::fs::create_dir_all(a.path().join("notes")).unwrap();
    std::fs::create_dir_all(b.path().join("notes")).unwrap();
    // A note that cannot be parsed, in the vault the caller is NOT in.
    std::fs::write(a.path().join("notes/broken.md"), "---\nnot: [valid\n---\nbody\n").unwrap();
    let mut store = MultiStore::open(&[
        ("personal".to_string(), a.path().to_path_buf()),
        ("lab".to_string(), b.path().to_path_buf()),
    ])
    .unwrap();
    store.reindex(Reindex::Full).unwrap();

    assert!(!store.skipped().is_empty(), "the machine's own view sees the broken note");

    let lab = ["lab".to_string()];
    let scoped = Scoped::new(&mut store, Some(&lab));
    assert!(
        scoped.skipped().is_empty(),
        "a scoped client must not be told about an unreadable note in a vault it cannot see"
    );
}

/// `allows` is what the blob route asks, because `GET /api/blob` is not a command and so cannot
/// reach the seams above. Pinned separately for that reason.
#[test]
fn allows_answers_membership_directly() {
    let (mut store, _, _, _a, _b) = two_vaults();

    let lab = ["lab".to_string()];
    let scoped = Scoped::new(&mut store, Some(&lab));
    assert!(scoped.allows("lab"));
    assert!(!scoped.allows("personal"));
    assert!(!scoped.allows(""), "an unnamed vault is not a wildcard");

    let unscoped = Scoped::new(&mut store, None);
    assert!(unscoped.allows("personal") && unscoped.allows("lab"));
}

/// A scope naming a vault that does not exist gets nothing — it must not fall back to the
/// default, which is how a revoked or renamed vault would silently become "all of them".
#[test]
fn a_scope_over_a_vault_that_does_not_exist_is_empty_not_everything() {
    let (mut store, _, _, _a, _b) = two_vaults();
    let ghost = ["archived".to_string()];
    let scoped = Scoped::new(&mut store, Some(&ghost));

    assert!(all(&scoped).is_empty(), "no vault matches, so nothing is readable");
    assert!(scoped.names().is_empty());

    let note = Object::new(Kind::Note, "nowhere to go\n");
    let mut scoped = Scoped::new(&mut store, Some(&ghost));
    assert!(
        matches!(scoped.put(&note), Err(StoreError::NoVaults)),
        "and a capture has no default to fall back to, which must be loud"
    );
}

/// The empty scope — a paired device whose vaults were all revoked. Reads must be empty and
/// writes must fail; an empty allowlist meaning "unrestricted" is the classic inversion.
#[test]
fn an_empty_scope_grants_nothing_rather_than_everything() {
    let (mut store, private_id, _, _a, _b) = two_vaults();
    let none: [String; 0] = [];
    let scoped = Scoped::new(&mut store, Some(&none));

    assert!(all(&scoped).is_empty(), "an empty allowlist is empty, not a wildcard");
    assert!(scoped.get(private_id.parse().unwrap()).unwrap().is_none());
    assert!(scoped.names().is_empty());
    assert!(!scoped.allows("personal"));
}
