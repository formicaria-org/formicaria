//! Reading a **reference pack** — a read-only SQLite file of facts a tool looks things up in.
//!
//! `decisions.md#toolchain`, 2026-10-11: a pack is fetched on request, like a model, lands outside
//! every vault, and is replaceable. This is the reading half, and it is here — not in an agent
//! crate — because a food log has to work with the assistant off. The file is *fetched* by a
//! transport shell; nothing in this crate reaches the network.
//!
//! The one pack so far is food: every value per 100 g as a whole number in the unit its key names
//! (`kcal`, `protein_mg`, `iron_ug`), and household portions in grams. Its layout is `pack.sql`,
//! which the builder (`ci/pack-foods.py`) runs too.
//!
//! **A pack is opened for each ask and never held.** It may be downloaded, replaced or removed
//! while the app runs; a handle kept open would keep serving a file that is gone.

use crate::StoreError;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The layout of a food pack. See `pack.sql`.
pub const FOOD_LAYOUT: &str = include_str!("pack.sql");
/// The food pack's file name inside the packs folder.
pub const FOOD_FILE: &str = "usda-foods.sqlite";

fn sql(e: rusqlite::Error) -> StoreError {
    StoreError::Io(format!("the food list could not be read: {e}"))
}

/// What a pack says about itself.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PackInfo {
    pub name: String,
    pub version: String,
    pub title: String,
    pub licence: String,
    pub attribution: String,
    pub foods: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FoodHit {
    pub id: String,
    pub name: String,
    /// Energy per 100 g.
    pub kcal: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FoodPortion {
    pub label: String,
    pub grams: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FoodDetail {
    pub id: String,
    pub name: String,
    /// Every value per 100 g, keyed like a logged entry's properties.
    pub per100: BTreeMap<String, i64>,
    pub portions: Vec<FoodPortion>,
    /// `<name>@<version>` — with the id, what an entry records as where its numbers came from.
    pub pack: String,
}

pub struct FoodPack {
    db: Connection,
    info: PackInfo,
}

impl FoodPack {
    /// Open a pack read-only. A file that is not a food pack is an error that says so, never a
    /// pack with nothing in it.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let db = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(sql)?;
        let meta = |key: &str| -> Result<String, StoreError> {
            db.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).map_err(sql)
        };
        let info = PackInfo {
            name: meta("name")?,
            version: meta("version")?,
            title: meta("title")?,
            licence: meta("licence")?,
            attribution: meta("attribution")?,
            foods: db
                .query_row("SELECT count(*) FROM food", [], |r| r.get::<_, i64>(0))
                .map_err(sql)? as u64,
        };
        Ok(FoodPack { db, info })
    }

    pub fn info(&self) -> &PackInfo {
        &self.info
    }

    /// Foods whose name has a word beginning with each word typed.
    ///
    /// **The index finds them; this orders them.** Relevance by word statistics puts *"Bananas,
    /// dehydrated, or banana powder"* above *"Banana, raw"* and *"Eggnog"* above *"Egg"*. What a
    /// person logging a meal wants first is the plain food: so a name that *starts* with what was
    /// typed leads, then foods as people report eating them, then the shorter name.
    pub fn search(&self, q: &str, limit: usize) -> Result<Vec<FoodHit>, StoreError> {
        let terms: Vec<String> = q
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(str::to_lowercase)
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        // Each term is letters and digits only, so quoting it is all the escaping there is.
        let expr = terms.iter().map(|t| format!("\"{t}\"*")).collect::<Vec<_>>().join(" ");
        let mut stmt = self
            .db
            .prepare(
                "SELECT f.id, f.name, f.src, coalesce(a.per100, 0)
                   FROM names
                   JOIN food f ON f.id = names.rowid
                   LEFT JOIN amount a
                     ON a.food = f.id AND a.n = (SELECT n FROM nutrient WHERE key = 'kcal')
                  WHERE names MATCH ?1
                  ORDER BY bm25(names)
                  LIMIT 400",
            )
            .map_err(sql)?;
        let mut found: Vec<(u8, u8, usize, FoodHit)> = stmt
            .query_map([&expr], |r| {
                let id: i64 = r.get(0)?;
                let name: String = r.get(1)?;
                let src: String = r.get(2)?;
                Ok((id, name, src, r.get::<_, i64>(3)?))
            })
            .map_err(sql)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql)?
            .into_iter()
            .map(|(id, name, src, kcal)| {
                let starts = u8::from(!name.to_lowercase().starts_with(&terms[0]));
                let origin = match src.as_str() {
                    "survey" => 0,
                    "foundation" => 1,
                    _ => 2,
                };
                let len = name.chars().count();
                (starts, origin, len, FoodHit { id: id.to_string(), name, kcal })
            })
            .collect();
        found.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)).then(a.3.name.cmp(&b.3.name)));
        Ok(found.into_iter().take(limit.max(1)).map(|f| f.3).collect())
    }

    /// One food, by its id — or by an entry's `source` (`usda-foods@2026.10/173944`), whatever
    /// version of the pack that entry was logged from: the id is the USDA's and does not change.
    pub fn get(&self, id: &str) -> Result<Option<FoodDetail>, StoreError> {
        let Ok(id) = id.rsplit('/').next().unwrap_or("").trim().parse::<i64>() else {
            return Ok(None);
        };
        let name: Option<String> = self
            .db
            .query_row("SELECT name FROM food WHERE id = ?1", [id], |r| r.get(0))
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(sql(e)),
            })?;
        let Some(name) = name else { return Ok(None) };

        let mut stmt = self
            .db
            .prepare(
                "SELECT n.key, a.per100 FROM amount a JOIN nutrient n USING (n) WHERE a.food = ?1",
            )
            .map_err(sql)?;
        let per100 = stmt
            .query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(sql)?
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(sql)?;
        let mut stmt = self
            .db
            .prepare("SELECT label, grams FROM portion WHERE food = ?1 ORDER BY seq")
            .map_err(sql)?;
        let portions = stmt
            .query_map([id], |r| Ok(FoodPortion { label: r.get(0)?, grams: r.get(1)? }))
            .map_err(sql)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql)?;
        Ok(Some(FoodDetail {
            id: id.to_string(),
            name,
            per100,
            portions,
            pack: format!("{}@{}", self.info.name, self.info.version),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pack of five foods, made with the same layout the builder uses.
    fn pack() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FOOD_FILE);
        let db = Connection::open(&path).unwrap();
        db.execute_batch(FOOD_LAYOUT).unwrap();
        for (k, v) in [
            ("name", "usda-foods"),
            ("version", "test"),
            ("title", "Test foods"),
            ("licence", "public domain"),
            ("attribution", "USDA"),
        ] {
            db.execute("INSERT INTO meta VALUES (?1, ?2)", [k, v]).unwrap();
        }
        db.execute_batch(
            "INSERT INTO nutrient VALUES (0, 'kcal'), (1, 'protein_mg'), (2, 'iron_ug');
             INSERT INTO food VALUES
               (1, 'Bananas, dehydrated, or banana powder', 'sr'),
               (2, 'Banana, raw', 'survey'),
               (3, 'Bananas, raw', 'sr'),
               (4, 'Eggnog', 'sr'),
               (5, 'Egg, whole, raw', 'survey'),
               (6, 'Crème fraîche', 'survey');
             INSERT INTO amount VALUES (2, 0, 89), (2, 1, 1090), (2, 2, 260), (5, 0, 143), (6, 0, 300);
             INSERT INTO portion VALUES (2, 0, '1 medium', 118), (2, 1, '1 cup, sliced', 150);
             INSERT INTO names(rowid, name) SELECT id, name FROM food;",
        )
        .unwrap();
        (dir, path)
    }

    #[test]
    fn a_pack_says_what_it_is() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        assert_eq!((p.info().name.as_str(), p.info().foods), ("usda-foods", 6));
    }

    #[test]
    fn the_plain_food_comes_first() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        let names = |q: &str| -> Vec<String> {
            p.search(q, 10).unwrap().into_iter().map(|h| h.name).collect()
        };
        // Starts with what was typed, as people report eating it, shorter — in that order.
        assert_eq!(
            names("banana"),
            ["Banana, raw", "Bananas, raw", "Bananas, dehydrated, or banana powder"]
        );
        assert_eq!(names("egg")[0], "Egg, whole, raw");
        // A word is found by its first letters, and by any of its words.
        assert_eq!(names("ra ban").len(), 2);
        // Folded like the app's own search.
        assert_eq!(names("creme"), ["Crème fraîche"]);
        assert!(names("").is_empty() && names("  ,,  ").is_empty());
        // What is typed is never syntax.
        assert!(names("\"banana\" OR *").len() <= 3);
    }

    #[test]
    fn a_hit_carries_its_energy_and_a_food_its_values_and_portions() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        assert_eq!(p.search("banana", 1).unwrap()[0].kcal, 89);
        let f = p.get("2").unwrap().unwrap();
        assert_eq!(f.per100["protein_mg"], 1090);
        assert_eq!(f.portions[0], FoodPortion { label: "1 medium".into(), grams: 118 });
        assert_eq!(f.pack, "usda-foods@test");
    }

    /// An entry logged from an older pack still finds its food: the id is the USDA's.
    #[test]
    fn a_food_is_found_by_an_entrys_source_whatever_version_it_names() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        assert_eq!(p.get("usda-foods@2025.04/2").unwrap().unwrap().name, "Banana, raw");
        assert!(p.get("999").unwrap().is_none());
        assert!(p.get("not a food").unwrap().is_none());
    }

    #[test]
    fn a_file_that_is_not_a_pack_is_an_error_that_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.sqlite");
        std::fs::write(&path, b"not a database").unwrap();
        assert!(FoodPack::open(&path).is_err());
        assert!(FoodPack::open(&dir.path().join("missing.sqlite")).is_err());
    }
}
