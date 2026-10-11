//! **The food list is carried in the app** — `decisions.md#toolchain`, 2026-10-11.
//!
//! It is compiled into `fm-app` and unpacked the first time something reads it. These pin that
//! the file in the repository is the one the constants describe, that reading needs no step from
//! anyone, and that a wrong file in its place is replaced.
//!
//! One test, on purpose: `FM_PACKS_DIR` is process-wide, and two tests pointing it at two folders
//! would race.

use fm_app::packs::{self, FOOD_BYTES, FOOD_SHA256, FOOD_VERSION};
use fm_core::pack::FOOD_FILE;

#[test]
fn the_list_the_app_carries_is_unpacked_on_first_use_and_put_right_when_it_is_wrong() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("FM_PACKS_DIR", dir.path());
    let file = dir.path().join(FOOD_FILE);
    assert!(!file.exists());

    // Nobody installs anything: a search is enough.
    let hits = packs::food_search("banana", 5).unwrap();
    assert_eq!(hits[0].name, "Banana, raw", "the plain food first");
    assert!(hits[0].kcal > 50);

    // What landed is byte for byte the list the constants name — so a list rebuilt without
    // updating them fails here, not on someone's phone.
    assert_eq!(std::fs::metadata(&file).unwrap().len(), FOOD_BYTES);
    assert_eq!(fm_core::blob::sha256_file(&file).unwrap(), FOOD_SHA256);
    let s = packs::food_status();
    let info = s.installed.expect("it is there");
    assert_eq!(info.version, FOOD_VERSION);
    assert!(info.foods > 13_000);
    assert!(s.error.is_none());

    // A food's values and portions, and an entry's `source` finds it again.
    let food = packs::food_get(&hits[0].id).unwrap().unwrap();
    assert!(food.per100.contains_key("protein_mg") && food.per100.contains_key("iron_ug"));
    assert!(!food.portions.is_empty());
    let again = packs::food_get(&format!("{}/{}", food.pack, food.id)).unwrap().unwrap();
    assert_eq!(again.name, food.name);

    // Something else in its place — cut short, or left by another build — is put right, and the
    // half-written sidecar does not stay behind.
    std::fs::write(&file, b"not the list").unwrap();
    assert!(packs::food_search("oats", 3).unwrap().iter().any(|h| h.name.starts_with("Oat")));
    assert_eq!(std::fs::metadata(&file).unwrap().len(), FOOD_BYTES);
    assert!(!dir.path().join(format!("{FOOD_FILE}.part")).exists());

    std::env::remove_var("FM_PACKS_DIR");
}
