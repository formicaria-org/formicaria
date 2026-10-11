//! **The food list, through the one door** — `decisions.md#toolchain`, 2026-10-11.
//!
//! A pack is a file outside every vault. These pin what the commands answer when it is there,
//! when it is not, and after it is removed.
//!
//! One test, on purpose: `FM_PACKS_DIR` is process-wide, and two tests pointing it at two folders
//! would race.

use fm_app::packs;
use fm_core::pack::{FOOD_FILE, FOOD_LAYOUT};

fn write_pack(dir: &std::path::Path) {
    let db = rusqlite::Connection::open(dir.join(FOOD_FILE)).unwrap();
    db.execute_batch(FOOD_LAYOUT).unwrap();
    db.execute_batch(
        "INSERT INTO meta VALUES ('name','usda-foods'),('version','test'),('title','Test foods'),
                                 ('licence','public domain'),('attribution','USDA');
         INSERT INTO nutrient VALUES (0,'kcal'),(1,'protein_mg');
         INSERT INTO food VALUES (7,'Banana, raw','survey');
         INSERT INTO amount VALUES (7,0,89),(7,1,1090);
         INSERT INTO portion VALUES (7,0,'1 medium',118);
         INSERT INTO names(rowid, name) SELECT id, name FROM food;",
    )
    .unwrap();
}

#[test]
fn the_food_list_is_read_where_it_is_kept_and_says_so_when_it_is_not_there() {
    let dir = tempfile::tempdir().unwrap();
    // The only test in this binary, so nothing else reads the environment meanwhile.
    std::env::set_var("FM_PACKS_DIR", dir.path());

    // Not there yet: searching says what to do, in words, and the status says what it would cost.
    let e = packs::food_search("banana", 5).unwrap_err();
    assert!(e.contains("not on this device") && e.contains("Settings"), "{e}");
    let s = packs::food_status();
    assert!(s.installed.is_none() && s.installed_bytes == 0);
    assert!(s.bytes > 1_000_000, "the offer states its size before anything is fetched");
    assert!(!s.licence.is_empty());

    // There.
    write_pack(dir.path());
    let hits = packs::food_search("ban", 5).unwrap();
    assert_eq!((hits[0].id.as_str(), hits[0].kcal), ("7", 89));
    let food = packs::food_get("usda-foods@test/7").unwrap().unwrap();
    assert_eq!(food.per100["protein_mg"], 1090);
    assert_eq!(food.pack, "usda-foods@test");
    let s = packs::food_status();
    assert_eq!(s.installed.as_ref().map(|i| i.foods), Some(1));
    assert!(s.installed_bytes > 0, "removing it must be able to say what comes back");

    // Removed — with a half-fetched file beside it, which goes too. Twice is not an error.
    std::fs::write(dir.path().join(format!("{FOOD_FILE}.part")), b"half").unwrap();
    packs::food_remove().unwrap();
    packs::food_remove().unwrap();
    assert!(packs::food_status().installed.is_none());
    assert!(!dir.path().join(format!("{FOOD_FILE}.part")).exists());
    assert!(packs::food_search("banana", 5).is_err());

    std::env::remove_var("FM_PACKS_DIR");
}
