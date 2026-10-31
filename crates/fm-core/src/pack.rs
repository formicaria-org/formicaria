//! Reading a **reference pack** — a read-only SQLite file of facts a tool looks things up in.
//!
//! `decisions.md#toolchain`, 2026-10-11: a pack is fetched on request, like a model, lands outside
//! every vault, and is replaceable. This is the reading half, and it is here — not in an agent
//! crate — because a food log has to work with the assistant off. The file is *fetched* by a
//! transport shell; nothing in this crate reaches the network.
//!
//! The one pack so far is food: every value per 100 g as a whole number in the unit its key names
//! (`kcal`, `protein_mg`, `iron_ug`), from two European tables with USDA as the fallback, and
//! household portions in grams where a table gives them. Its layout is `pack.sql`,
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
pub const FOOD_FILE: &str = "foods.sqlite";

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
    /// The food's own identifier in its table, prefixed with the table: `ciqual:1000`.
    pub id: String,
    pub name: String,
    /// Which table it is from: `ciqual`, `bls`, `swiss`, `crea` or `usda`. Shown beside the name, because two
    /// tables may both list "banana" and they are not the same measurement.
    pub src: String,
    /// Energy per 100 g.
    pub kcal: i64,
    /// The food's name in its own table's language, when what was typed was found there and not
    /// in `name`: a person who typed *prosciutto* sees why *Ham, raw* is offered.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub alt: String,
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
    pub src: String,
    /// The food's group in its own table, by the name that table publishes. Empty where it
    /// publishes none — a group is never made up.
    pub grp: String,
    /// Every value per 100 g, keyed like a logged entry's properties.
    pub per100: BTreeMap<String, i64>,
    pub portions: Vec<FoodPortion>,
    /// `<name>@<version>` — with the id, what an entry records as where its numbers came from.
    pub pack: String,
    /// **The values its own table never measures, and the food each was borrowed from**
    /// (`decisions.md#data`, 2026-10-29): `{"choline_ug": "USDA: Egg, whole, raw"}`. Those values
    /// are in `per100` like the rest; this is what lets the app say so. Empty for a pack built before.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub borrowed: BTreeMap<String, String>,
    /// **Values estimated from similar foods** (2026-10-29, later that night), where no table
    /// measured them for this food or one close enough: `{"choline_ug": "5 similar foods (meat)"}`.
    /// In `per100` like the rest; shown as estimates. Empty for a pack built before.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub estimated: BTreeMap<String, String>,
}

pub struct FoodPack {
    db: Connection,
    info: PackInfo,
}

/// A table as a person reads its name where a value was borrowed from it.
fn table_name(src: &str) -> String {
    match src {
        "usda" => "USDA".into(),
        "ciqual" => "Ciqual (France)".into(),
        "bls" => "BLS (Germany)".into(),
        "swiss" => "the Swiss table".into(),
        "crea" => "CREA (Italy)".into(),
        other => other.to_uppercase(),
    }
}

/// Lower case, and the accents of the four languages taken off, as the index takes them off
/// (`remove_diacritics`): *biere* is *Bière*, and *kase* is *Käse*.
fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// How well a name answers the words typed, the smallest first: see `fit`.
type Fit = (u8, u8, u8, usize);
/// What a name none of whose words the words typed begin is worth: last.
const NO_FIT: Fit = (u8::MAX, u8::MAX, u8::MAX, usize::MAX);
/// **The words the tables use for a food as it comes** — raw, nothing added, or a table's own
/// average of its kinds — in the four languages the list is searched in: BLS writes *Apple raw*,
/// the Swiss table *Yogurt natural*, Ciqual *Oeuf cru* and *Milk (average)*, *Fromage (aliment
/// moyen)*, the Swiss table's Italian *Pomodoro, crudo*. Not *fresh*: *Fromage frais* and *Pasta
/// fresca* are other foods.
const AS_IT_COMES: &[&str] = &[
    "raw", "plain", "natural", "average", // English
    "cru", "crue", "crus", "crues", "nature", "moyen", "moyenne", // French
    "roh", "natur", // German
    "crudo", "cruda", "crudi", "crude", "naturale", "media", // Italian
];

/// **How well a food's name answers the words typed**, compared smallest first; `None` when one of
/// the words typed begins no word of the name. `english` is whether it is the food's English name.
///
/// 1. **The first word typed is the name's first word, whole**, or its plural (*Water, municipal*
///    and *Lemons* — not *Watermelon*). Then a first word it only begins; then a name with the word
///    further in.
/// 2. **Nothing after the words typed makes it another food.** In English a second noun does:
///    *Beer ham*, *Water ice*, *Apple tart* — so in an English name what follows the food must be a
///    comma, a bracket, nothing, or a word of `AS_IT_COMES` (*Beer, dark*, *Coffee (infusion)*,
///    *Apple raw*). The tables' French, Italian and German names put the other food first or join
///    it into one word (*Succo di pomodoro*, *Bierschinken*), so what follows is said of the food
///    (*Riz blanc*, *Oeuf dur*) and any name they lead counts.
/// 3. **The food as it comes**: the name is the food alone, or the word after it is one of
///    `AS_IT_COMES` (*Butter*, *Egg, raw* and *Milk (average)* before *Egg, powder*). Only the next
///    word: *Beer shandy (average)* is the average of shandies, not of beers.
/// 4. The shorter name.
fn fit(name: &str, terms: &[String], english: bool) -> Option<Fit> {
    let low = fold(name);
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut start = None;
    for (i, c) in low.char_indices() {
        if c.is_alphanumeric() {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            words.push((s, &low[s..i]));
        }
    }
    if let Some(s) = start {
        words.push((s, &low[s..]));
    }
    if !terms.iter().all(|t| words.iter().any(|(_, w)| w.starts_with(t.as_str()))) {
        return None;
    }
    let whole_word =
        |w: &str, t: &str| w.strip_prefix(t).is_some_and(|s| s.is_empty() || s == "s" || s == "es");
    let head = words.first().map_or("", |(_, w)| *w);
    let whole = if whole_word(head, &terms[0]) {
        0
    } else if head.starts_with(terms[0].as_str()) {
        1
    } else {
        2
    };
    let leads = terms.len() <= words.len()
        && terms.iter().zip(&words).all(|(t, (_, w))| w.starts_with(t.as_str()));
    // What follows the words typed, and its first word.
    let (rest, next) = if leads {
        let (at, w) = words[terms.len() - 1];
        let rest = low[at + w.len()..].trim_start();
        (rest, words.get(terms.len()).map_or("", |(_, w)| *w))
    } else {
        ("", "")
    };
    // The next word, or any word of a bracket that comes next: *(aliment moyen)*.
    let bracket = rest.strip_prefix('(').map(|r| r.split(')').next().unwrap_or(r));
    let comes = rest.is_empty()
        || AS_IT_COMES.contains(&next)
        || bracket.is_some_and(|b| {
            b.split(|c: char| !c.is_alphanumeric()).any(|w| AS_IT_COMES.contains(&w))
        });
    let named_first = leads && (!english || comes || rest.starts_with([',', '(']));
    let as_it_comes = leads && comes;
    Some((whole, u8::from(!named_first), u8::from(!as_it_comes), name.chars().count()))
}

impl FoodPack {
    /// Whether the pack has `name` — a pack built before a table was added has not.
    fn has_table(&self, name: &str) -> Result<bool, StoreError> {
        self.db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .map_err(sql)
    }

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
    /// **The index finds them; this orders them** (`fit`). Relevance by word statistics puts
    /// *"Bananas, dehydrated, or banana powder"* above *"Banana raw"* and *"Egg nog"* above *"Egg,
    /// raw"*. What a person logging a meal wants first is the plain food. Measured on the real list
    /// with 66 typed words in four languages (`tests/food_search_ranks.rs`,
    /// `food-search-ranks-2026-10-31.md`).
    ///
    /// **European tables first; USDA only where they leave room** (`decisions.md#data`,
    /// 2026-10-11). A USDA food is offered when the European tables do not fill the results — a
    /// food they do not list — and never ahead of one of theirs.
    pub fn search(&self, q: &str, limit: usize) -> Result<Vec<FoodHit>, StoreError> {
        let terms: Vec<String> =
            q.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(fold).collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        // Each term is letters and digits only, so quoting it is all the escaping there is.
        let expr = terms.iter().map(|t| format!("\"{t}\"*")).collect::<Vec<_>>().join(" ");
        // A pack built before groups has no `same`: every food is its own.
        let grouped = self.has_table("same")?;
        let (group, join) = if grouped {
            ("coalesce(s.first, f.id)", "\n                   LEFT JOIN same s ON s.food = f.id")
        } else {
            ("f.id", "")
        };
        let mut stmt = self
            .db
            .prepare(&format!(
                "SELECT f.code, f.name, f.src, coalesce(a.per100, 0), f.alt, {group}
                   FROM names
                   JOIN food f ON f.id = names.rowid{join}
                   LEFT JOIN amount a
                     ON a.food = f.id AND a.n = (SELECT n FROM nutrient WHERE key = 'kcal')
                  WHERE names MATCH ?1
                  ORDER BY (f.src = 'usda'), bm25(names)
                  LIMIT 600"
            ))
            .map_err(sql)?;
        let mut group_of = BTreeMap::new();
        let mut found: Vec<(u8, Fit, FoodHit)> = stmt
            .query_map([&expr], |r| {
                let hit = FoodHit {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    src: r.get(2)?,
                    kcal: r.get(3)?,
                    alt: r.get(4)?,
                };
                Ok((hit, r.get::<_, i64>(5)?))
            })
            .map_err(sql)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql)?
            .into_iter()
            .map(|(hit, group)| {
                group_of.insert(hit.id.clone(), group);
                hit
            })
            .map(|mut hit| {
                let fallback = u8::from(hit.src == "usda");
                // The name the person's words fit best: the English one, or one of the others —
                // the English one when they fit it as well.
                let english = fit(&hit.name, &terms, true);
                let other = hit
                    .alt
                    .split(" · ")
                    .filter_map(|n| fit(n, &terms, false).map(|f| (f, n)))
                    .min_by_key(|(f, _)| *f)
                    .filter(|(f, _)| english.is_none_or(|e| f < &e));
                let how = other.map(|(f, _)| f).or(english).unwrap_or(NO_FIT);
                // Only the name that was found is sent on, and only when it is not the first.
                hit.alt = other.map(|(_, n)| n.to_string()).unwrap_or_default();
                (fallback, how, hit)
            })
            .collect();
        // Stable: a tie keeps the index's own order, its relevance by word statistics.
        found.sort_by_key(|f| (f.0, f.1));
        // **One food, one result** (2026-10-29, later that night): the same food from several tables
        // holds one set of values, so it is offered once — under the name it was found by.
        let mut shown = std::collections::BTreeSet::new();
        Ok(found
            .into_iter()
            .filter(|f| shown.insert(group_of.get(&f.2.id).copied().unwrap_or(-1)))
            .take(limit.max(1))
            .map(|f| f.2)
            .collect())
    }

    /// One food, by its id — or by an entry's `source` (`foods@2026.10.1/ciqual:1000`), whatever
    /// version of the list that entry was logged from: the id is the table's own and does not
    /// change. A bare number is an entry from the first, USDA-only list, whose ids were FoodData
    /// Central's.
    pub fn get(&self, id: &str) -> Result<Option<FoodDetail>, StoreError> {
        let last = id.rsplit('/').next().unwrap_or("").trim();
        let code = if !last.is_empty() && last.bytes().all(|b| b.is_ascii_digit()) {
            format!("usda:{last}")
        } else {
            last.to_string()
        };
        let found: Option<(i64, String, String, String)> = self
            .db
            .query_row("SELECT id, name, src, grp FROM food WHERE code = ?1", [&code], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(sql(e)),
            })?;
        let Some((row, name, src, grp)) = found else { return Ok(None) };

        let mut stmt = self
            .db
            .prepare(
                "SELECT n.key, a.per100 FROM amount a JOIN nutrient n USING (n) WHERE a.food = ?1",
            )
            .map_err(sql)?;
        let per100 = stmt
            .query_map([row], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(sql)?
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(sql)?;
        let mut stmt = self
            .db
            .prepare("SELECT label, grams FROM portion WHERE food = ?1 ORDER BY seq")
            .map_err(sql)?;
        let mut portions = stmt
            .query_map([row], |r| Ok(FoodPortion { label: r.get(0)?, grams: r.get(1)? }))
            .map_err(sql)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql)?;
        // **A food without portions of its own offers the same food's from another table**
        // (`food-values-plan-2026-10-30.md`, step 3; `decisions.md#data`, 2026-10-31): Ciqual's red
        // wine is CREA's red wine (`same`), whose table gives Italy's standard portion, 125 g. Each
        // is named with its table, and only what a table published is offered — never a size of
        // ours. USDA's "Guideline amount per …" rows are a labelling rule, not something served.
        if portions.is_empty() && self.has_table("same")? {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT p.label, p.grams, g.src FROM same s
                       JOIN same t ON t.first = s.first
                       JOIN food g ON g.id = t.food
                       JOIN portion p ON p.food = g.id
                      WHERE s.food = ?1 AND g.id != ?1
                      ORDER BY (g.src = 'usda'), g.id, p.seq",
                )
                .map_err(sql)?;
            let theirs = stmt
                .query_map([row], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?))
                })
                .map_err(sql)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql)?;
            for (label, grams, src) in theirs {
                let label = format!("{label} · {}", table_name(&src));
                if !label.starts_with("Guideline amount")
                    && !portions.iter().any(|p| p.label == label)
                {
                    portions.push(FoodPortion { label, grams });
                }
            }
        }
        let has_borrowed = self
            .db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'borrowed'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map_err(sql)?
            > 0;
        let borrowed = if has_borrowed {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT n.key, f.src, f.name FROM borrowed b JOIN nutrient n USING (n)
                     JOIN food f ON f.id = b.from_food WHERE b.food = ?1",
                )
                .map_err(sql)?;
            let got = stmt
                .query_map([row], |r| {
                    let (key, src, name): (String, String, String) =
                        (r.get(0)?, r.get(1)?, r.get(2)?);
                    Ok((key, format!("{}: {name}", table_name(&src))))
                })
                .map_err(sql)?
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map_err(sql)?;
            got
        } else {
            BTreeMap::new()
        };
        let estimated = if self.has_table("estimated")? {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT n.key, e.basis FROM estimated e JOIN nutrient n USING (n)
                     WHERE e.food = ?1",
                )
                .map_err(sql)?;
            let got = stmt
                .query_map([row], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map_err(sql)?
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map_err(sql)?;
            got
        } else {
            BTreeMap::new()
        };
        Ok(Some(FoodDetail {
            id: code,
            name,
            src,
            grp,
            per100,
            portions,
            pack: format!("{}@{}", self.info.name, self.info.version),
            borrowed,
            estimated,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list of a few foods from three tables, made with the same layout the builder uses.
    fn pack() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FOOD_FILE);
        let db = Connection::open(&path).unwrap();
        db.execute_batch(FOOD_LAYOUT).unwrap();
        for (k, v) in [
            ("name", "foods"),
            ("version", "test"),
            ("title", "Test foods"),
            ("licence", "open"),
            ("attribution", "Anses; Max Rubner-Institut; USDA"),
        ] {
            db.execute("INSERT INTO meta VALUES (?1, ?2)", [k, v]).unwrap();
        }
        db.execute_batch(
            "INSERT INTO nutrient VALUES (0, 'kcal'), (1, 'protein_mg'), (2, 'iron_ug'),
                                         (3, 'choline_ug');
             INSERT INTO food(id, code, name, src, grp) VALUES
               (1, 'bls:F503400', 'Banana dried', 'bls', ''),
               (2, 'bls:F503100', 'Banana raw', 'bls', ''),
               (3, 'ciqual:13005', 'Banana, flesh without skin, raw', 'ciqual', 'fruits'),
               (4, 'usda:2709224', 'Banana, raw', 'usda', ''),
               (5, 'bls:P8A3000', 'Egg nog', 'bls', ''),
               (6, 'ciqual:22000', 'Egg, raw', 'ciqual', 'meat, egg and fish'),
               (7, 'ciqual:19024', 'Crème fraîche', 'ciqual', 'milk'),
               (8, 'usda:2705660', 'Poi', 'usda', '');
             INSERT INTO food(id, code, name, src, grp, alt) VALUES
               (9, 'swiss:1234', 'Ham, raw', 'swiss', 'Meat', 'Prosciutto crudo · Rohschinken · Jambon cru');
             INSERT INTO amount VALUES (2, 0, 95), (2, 1, 1150), (2, 2, 350), (4, 0, 97), (6, 0, 140),
                                       (7, 0, 300), (8, 0, 112), (6, 3, 335000);
             INSERT INTO borrowed VALUES (6, 3, 4);
             INSERT INTO estimated VALUES (2, 3, '5 similar foods (fruit)');
             INSERT INTO portion VALUES (4, 0, '1 medium', 118), (4, 1, '1 cup, sliced', 150);
             INSERT INTO names(rowid, name, alt) SELECT id, name, alt FROM food;",
        )
        .unwrap();
        (dir, path)
    }
    fn names(p: &FoodPack, q: &str, limit: usize) -> Vec<String> {
        p.search(q, limit).unwrap().into_iter().map(|h| h.name).collect()
    }

    /// A food is found by the name its own table gives it in another language, and says which
    /// name that was. Nothing is translated: only a name that was published is searched.
    #[test]
    fn a_food_is_found_by_its_name_in_its_tables_own_language() {
        let (_d, path) = pack();
        let pack = FoodPack::open(&path).unwrap();
        let hits = pack.search("prosciutto", 5).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!((hits[0].name.as_str(), hits[0].alt.as_str()), ("Ham, raw", "Prosciutto crudo"));
        // By its German name too, and by the first letters of a later word.
        assert_eq!(pack.search("rohsch", 5).unwrap()[0].alt, "Rohschinken");
        assert_eq!(pack.search("jambon cr", 5).unwrap()[0].alt, "Jambon cru");
        // Found by its English name, it shows no other.
        assert_eq!(pack.search("ham", 5).unwrap()[0].alt, "");
    }

    /// A value the food's own table never measures is borrowed, and the food says from where
    /// (2026-10-29: an egg from a European table had no choline at all).
    #[test]
    fn a_borrowed_value_says_where_it_came_from() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        let egg = p.get("ciqual:22000").unwrap().unwrap();
        assert_eq!(egg.per100["choline_ug"], 335_000);
        assert_eq!(egg.borrowed["choline_ug"], "USDA: Banana, raw");
        assert!(!egg.borrowed.contains_key("kcal"), "a measured value is the food's own");
        let banana = p.get("bls:F503100").unwrap().unwrap();
        assert!(banana.borrowed.is_empty());
    }

    /// One food, one result (2026-10-29, later that night): the same banana from three tables is
    /// one group, offered once — under the name it was found by.
    #[test]
    fn the_same_food_from_several_tables_is_offered_once() {
        let (_d, path) = pack();
        Connection::open(&path)
            .unwrap()
            .execute_batch("INSERT INTO same VALUES (2, 2), (3, 2), (4, 2);")
            .unwrap();
        let p = FoodPack::open(&path).unwrap();
        let raw: Vec<String> = names(&p, "banana raw", 10);
        assert_eq!(raw, ["Banana raw"], "one of the three");
        assert_eq!(names(&p, "banana", 10).len(), 2, "the dried one is another food");
        let b = p.get("bls:F503100").unwrap().unwrap();
        assert_eq!(b.estimated["choline_ug"], "5 similar foods (fruit)");
        // A food with no portions of its own offers the same food's, from the table that gave them.
        let shown: Vec<_> = b.portions.iter().map(|p| (p.label.as_str(), p.grams)).collect();
        assert_eq!(shown, [("1 medium · USDA", 118), ("1 cup, sliced · USDA", 150)]);
        let usda = p.get("usda:2709224").unwrap().unwrap();
        assert_eq!(usda.portions[0].label, "1 medium", "its own, unnamed as before");
        assert!(
            p.get("bls:F503400").unwrap().unwrap().portions.is_empty(),
            "dried is another food"
        );
        assert!(p.get("ciqual:22000").unwrap().unwrap().estimated.is_empty());
    }

    #[test]
    fn a_pack_says_what_it_is() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        assert_eq!((p.info().name.as_str(), p.info().foods), ("foods", 9));
    }

    #[test]
    fn the_plain_food_comes_first() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        // The food as it comes, then the shorter name.
        assert_eq!(
            names(&p, "banana", 3),
            ["Banana raw", "Banana, flesh without skin, raw", "Banana dried"]
        );
        assert_eq!(names(&p, "egg", 10), ["Egg, raw", "Egg nog"]);
        assert_eq!(names(&p, "egg r", 10), ["Egg, raw"]);
        // A word is found by its first letters, and by any of its words.
        assert_eq!(names(&p, "ra ban", 10).len(), 3);
        // Folded like the app's own search.
        assert_eq!(names(&p, "creme", 10), ["Crème fraîche"]);
        assert!(names(&p, "", 10).is_empty() && names(&p, "  ,,  ", 10).is_empty());
        // What is typed is never syntax.
        assert!(names(&p, "\"banana\" OR *", 10).len() <= 4);
    }

    /// `decisions.md#data`, 2026-10-11: USDA is the fallback. Its banana is offered only once the
    /// European tables have nothing more to give — and a food only it lists is still found.
    /// **What a person typing a word means is the plain food** (`fit`; measured on the real list in
    /// `tests/food_search_ranks.rs`). Invented foods, named the way each table names its own.
    #[test]
    fn the_word_typed_whole_names_the_food_and_the_food_as_it_comes_leads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FOOD_FILE);
        let db = Connection::open(&path).unwrap();
        db.execute_batch(FOOD_LAYOUT).unwrap();
        for k in ["name", "version", "title", "licence", "attribution"] {
            db.execute("INSERT INTO meta VALUES (?1, 'test')", [k]).unwrap();
        }
        db.execute_batch(
            "INSERT INTO food(id, code, name, src, alt) VALUES
               (1, 'bls:1', 'Watermelon raw', 'bls', ''),
               (2, 'bls:2', 'Water ice', 'bls', ''),
               (3, 'ciqual:3', 'Water, municipal', 'ciqual', 'Eau du robinet'),
               (4, 'bls:4', 'Beer ham', 'bls', 'Bierschinken'),
               (5, 'swiss:5', 'Beer shandy (average)', 'swiss', 'Bière panachée (moyenne)'),
               (6, 'ciqual:6', 'Beer, regular', 'ciqual', 'Bière blonde'),
               (7, 'ciqual:7', 'Egg, powder', 'ciqual', 'Oeuf, en poudre'),
               (8, 'ciqual:8', 'Egg, raw', 'ciqual', 'Oeuf cru'),
               (9, 'bls:9', 'Egg nog', 'bls', ''),
               (10, 'ciqual:10', 'Lemon tart', 'ciqual', ''),
               (11, 'crea:11', 'Lemons', 'crea', 'Limoni'),
               (12, 'swiss:12', 'Fresh cheese', 'swiss', 'Fromage frais'),
               (13, 'ciqual:13', 'Cheese (average)', 'ciqual', 'Fromage (aliment moyen)'),
               (14, 'bls:14', 'Apple tart', 'bls', 'Apfelkuchen'),
               (15, 'bls:15', 'Apple raw', 'bls', 'Apfel roh');
             INSERT INTO names(rowid, name, alt) SELECT id, name, alt FROM food;",
        )
        .unwrap();
        let p = FoodPack::open(&path).unwrap();
        let first = |q: &str| p.search(q, 10).unwrap().into_iter().next().unwrap();
        // The word whole before a word it begins; a second English noun makes another food.
        assert_eq!(names(&p, "water", 10), ["Water, municipal", "Water ice", "Watermelon raw"]);
        assert_eq!(names(&p, "beer", 10), ["Beer, regular", "Beer ham", "Beer shandy (average)"]);
        assert_eq!(names(&p, "apple", 10), ["Apple raw", "Apple tart"]);
        // A plural is the word whole; nothing after the food is the food as it comes.
        assert_eq!(names(&p, "lemon", 10), ["Lemons", "Lemon tart"]);
        // The food as it comes before what was done to it — in the table's own language too.
        assert_eq!(names(&p, "egg", 10), ["Egg, raw", "Egg, powder", "Egg nog"]);
        assert_eq!(first("oeuf").alt, "Oeuf cru");
        // A bracketed average counts, and only right after the food: *Beer shandy (average)* is the
        // average of shandies. *Frais* is not the food as it comes: *fromage frais* is another one.
        assert_eq!(first("fromage").alt, "Fromage (aliment moyen)");
        assert_eq!(first("biere").alt, "Bière blonde", "typed without its accent");
        assert_eq!(first("eau").alt, "Eau du robinet");
        // In German the other food is joined into the word: *bier* begins *Bierschinken*, whole
        // only in *Bière*.
        assert_eq!(first("apfel").alt, "Apfel roh");
    }

    #[test]
    fn european_tables_first_and_usda_only_where_they_leave_room() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        let three = p.search("banana", 3).unwrap();
        assert!(three.iter().all(|h| h.src != "usda"), "three European bananas fill three places");
        let four = p.search("banana", 4).unwrap();
        assert_eq!(four[3].src, "usda", "the fourth place is the fallback's");
        assert_eq!(names(&p, "poi", 5), ["Poi"]);
    }

    #[test]
    fn a_hit_says_which_table_it_is_from_and_a_food_carries_its_values() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        let hit = &p.search("banana", 1).unwrap()[0];
        assert_eq!((hit.id.as_str(), hit.src.as_str(), hit.kcal), ("bls:F503100", "bls", 95));
        let f = p.get(&hit.id).unwrap().unwrap();
        assert_eq!((f.per100["protein_mg"], f.grp.as_str()), (1150, ""));
        assert!(f.portions.is_empty(), "the European tables give no household portions");
        assert_eq!(f.pack, "foods@test");
        // Only the fallback has portions.
        let us = p.get("usda:2709224").unwrap().unwrap();
        assert_eq!(us.portions[0], FoodPortion { label: "1 medium".into(), grams: 118 });
    }

    /// An entry logged from an older list still finds its food: the id is the table's own. A bare
    /// number is from the first list, whose ids were FoodData Central's.
    #[test]
    fn a_food_is_found_by_an_entrys_source_whatever_version_it_names() {
        let (_d, path) = pack();
        let p = FoodPack::open(&path).unwrap();
        assert_eq!(p.get("foods@2026.10.1/ciqual:22000").unwrap().unwrap().name, "Egg, raw");
        assert_eq!(p.get("usda-foods@2026.10/2709224").unwrap().unwrap().name, "Banana, raw");
        assert!(p.get("usda-foods@2026.10/173944").unwrap().is_none(), "a food no longer carried");
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
