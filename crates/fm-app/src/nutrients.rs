//! A day's vitamins and minerals, each beside the reference a named body gives for it.
//!
//! `decisions.md#data`, 2026-10-12, *food made easy to use*. The food list carries some thirty
//! values a food; the day screen names four. This answers all of them for one day, with:
//!
//! - **the reference** — EFSA's Population Reference Intake, or its Adequate Intake where it set
//!   no PRI (EFSA, *Dietary Reference Values for nutrients: Summary report*, 2017), for an adult
//!   of the person's sex and, for calcium, age; fibre is EFSA's 25 g and sodium WHO's limit, as in
//!   the plan library;
//! - **the upper level**, only where EFSA's applies to what is in food (EFSA, *Overview on
//!   Tolerable Upper Intake Levels*, v11, August 2025) — the ones that count only supplements or
//!   added forms (magnesium, folic acid, preformed vitamin A) are left out, since an entry cannot
//!   say which form it holds.
//!
//! **Nothing here is worked out by the app except two products the source defines**: thiamin and
//! niacin are given per megajoule of energy, so their reference follows the energy of the day.
//! Zinc is given at four levels of phytate intake and gets no single number. The register is
//! `sources-2026-10-11.md` §6c.

use crate::dto::ObjectMeta;
use crate::log::PLANNED;
use crate::plan::plan_profile;
use crate::plan::Profile;
use crate::thread::log_base;
use crate::views::{Metric, Role, TotalsDef};
use fm_core::{Store, StoreError};
use fm_model::{Object, PropertyValue};
use fm_query::{Query, SortKey};
use serde::Serialize;
use time::{format_description::well_known::Iso8601, Date};

/// What kind of figure a reference is, in EFSA's terms.
const PRI: &str = "PRI";
const AI: &str = "AI";

struct Ref {
    key: &'static str,
    label: &'static str,
    /// In the key's own unit: (men, women). The same twice when it does not differ.
    adult: Option<(i64, i64)>,
    /// Per megajoule of the day's energy, in the key's own unit, instead of a fixed figure.
    per_mj: Option<i64>,
    kind: &'static str,
    upper: Option<i64>,
    note: &'static str,
}

const fn fixed(
    key: &'static str,
    label: &'static str,
    men: i64,
    women: i64,
    kind: &'static str,
    upper: Option<i64>,
) -> Ref {
    Ref { key, label, adult: Some((men, women)), per_mj: None, kind, upper, note: "" }
}

/// In the order a label lists them: fibre and the two minerals a diet is most often held to, the
/// vitamins, then the minerals.
const REFS: &[Ref] = &[
    fixed("fiber_mg", "Fibre", 25_000, 25_000, AI, None),
    Ref {
        key: "sodium_ug",
        label: "Sodium",
        adult: None,
        per_mj: None,
        kind: "",
        upper: Some(2_000_000),
        note: "WHO: less than 2 g of sodium a day. EFSA's own figure was not read",
    },
    fixed("potassium_ug", "Potassium", 3_500_000, 3_500_000, AI, None),
    fixed("vitamin_a_rae_ug", "Vitamin A", 750, 650, PRI, None),
    fixed("vitamin_d_ng", "Vitamin D", 15_000, 15_000, AI, Some(100_000)),
    fixed("vitamin_e_ug", "Vitamin E", 13_000, 11_000, AI, None),
    fixed("vitamin_k1_ug", "Vitamin K", 70, 70, AI, None),
    fixed("vitamin_c_ug", "Vitamin C", 110_000, 95_000, PRI, None),
    Ref {
        key: "thiamin_ug",
        label: "Thiamin (B1)",
        adult: None,
        per_mj: Some(100),
        kind: PRI,
        upper: None,
        note: "0.1 mg for each megajoule of the day's energy",
    },
    fixed("riboflavin_ug", "Riboflavin (B2)", 1_600, 1_600, PRI, None),
    Ref {
        key: "niacin_ug",
        label: "Niacin (B3)",
        adult: None,
        per_mj: Some(1_600),
        kind: PRI,
        upper: None,
        note: "1.6 mg of niacin equivalents for each megajoule of the day's energy; the food \
               tables give niacin itself, so this reads a little low",
    },
    fixed("pantothenic_acid_ug", "Pantothenic acid (B5)", 5_000, 5_000, AI, None),
    fixed("vitamin_b6_ug", "Vitamin B6", 1_700, 1_600, PRI, Some(12_000)),
    fixed("folate_dfe_ug", "Folate", 330, 330, PRI, None),
    fixed("vitamin_b12_ng", "Vitamin B12", 4_000, 4_000, AI, None),
    fixed("choline_ug", "Choline", 400_000, 400_000, AI, None),
    fixed("calcium_ug", "Calcium", 950_000, 950_000, PRI, Some(2_500_000)),
    fixed("phosphorus_ug", "Phosphorus", 550_000, 550_000, AI, None),
    fixed("magnesium_ug", "Magnesium", 350_000, 300_000, AI, None),
    fixed("iron_ug", "Iron", 11_000, 16_000, PRI, None),
    Ref {
        key: "zinc_ug",
        label: "Zinc",
        adult: None,
        per_mj: None,
        kind: PRI,
        upper: Some(25_000),
        note: "EFSA gives 9.4 to 16.3 mg for men and 7.5 to 12.7 mg for women, by how much \
               phytate is eaten; no single figure",
    },
    fixed("copper_ug", "Copper", 1_600, 1_300, AI, Some(5_000)),
    fixed("manganese_ug", "Manganese", 3_000, 3_000, AI, None),
    fixed("selenium_ug", "Selenium", 70, 70, AI, Some(255)),
    fixed("iodine_ug", "Iodine", 150, 150, AI, Some(600)),
];

/// **Water, caffeine and alcohol: the day's drinks, in Food's own numbers** (`food-values-plan-
/// 2026-10-30.md`, step 2; `decisions.md#data`, 2026-10-31). Every food in the list carries its
/// water, and many their caffeine and alcohol; Health already adds them up and compares the water
/// (*Water*, *Caffeine* in `shipped/health-cards.json`). Here they are beside the same figures:
///
/// - **water**: EFSA's adequate intake of *total* water, 2.0 L a day for women and 2.5 L for men,
///   from *"drinking water, beverages of all kind, and from food moisture"* — which is what the
///   food list measures (EFSA, *Summary report*, 2017);
/// - **caffeine**: EFSA's 400 mg a day, the intake that *"does not raise safety concerns for healthy
///   adults"*, and 200 mg in pregnancy (EFSA, *Scientific Opinion on the safety of caffeine*, 2015);
/// - **alcohol**: the amount alone. None of the app's sources gives an amount to aim for, and the
///   app does not advise.
const DRINKS: &[Ref] = &[
    Ref {
        key: "water_mg",
        label: "Water",
        adult: Some((2_500_000, 2_000_000)),
        per_mj: None,
        kind: AI,
        upper: None,
        note: "From food and drinks alike: EFSA's figure counts the water in food as well as in \
               drinks, as this does, and assumes moderate temperatures and activity",
    },
    Ref {
        key: "caffeine_ug",
        label: "Caffeine",
        adult: None,
        per_mj: None,
        kind: "",
        upper: Some(400_000),
        note: "EFSA: up to 400 mg a day raises no safety concerns for healthy adults, and up to \
               200 mg in pregnancy. Not a target",
    },
    Ref {
        key: "alcohol_mg",
        label: "Alcohol",
        adult: None,
        per_mj: None,
        kind: "",
        upper: None,
        note: "The amount alone: the app's sources give no amount to aim for",
    },
];

/// **At most six values on Day**, so it stays one calm screen (`food-values-plan-2026-10-30.md`,
/// step 4).
pub const MOST_ON_DAY: usize = 6;

/// **The values a person follows, on Food's own views** (`decisions.md#data`, 2026-10-31): the
/// keys pinned in their profile (`Profile::day_values`) stand in for the view's own details of the
/// food log — Day's *Also today*, and what Trends offers. A value from another log (a weight among
/// the trends) stays. Only a value of the nutrients sheet can be pinned, at most six; nothing
/// pinned, ever, leaves the view as it is. The same notes give the same view on every device.
pub fn pin(def: &mut TotalsDef, profile: &Profile) {
    let Some(keys) = &profile.day_values else { return };
    if def.log != "food" {
        return;
    }
    let ours = |m: &Metric| m.role == Role::Detail && m.log.is_none();
    let label = |key: &str, r: &Ref| {
        def.metrics.iter().find(|m| m.key == key).map_or(r.label.to_string(), |m| m.label.clone())
    };
    let mut seen = Vec::new();
    let pinned: Vec<Metric> = keys
        .iter()
        .filter_map(|k| REFS.iter().chain(DRINKS).find(|r| r.key == k))
        .filter(|r| {
            let first = !seen.contains(&r.key);
            seen.push(r.key);
            first
        })
        .take(MOST_ON_DAY)
        .map(|r| Metric {
            key: r.key.to_string(),
            label: label(r.key, r),
            role: Role::Detail,
            agg: Default::default(),
            log: None,
        })
        .collect();
    let at = def.metrics.iter().position(ours).unwrap_or(def.metrics.len());
    def.metrics.retain(|m| !ours(m));
    def.metrics.splice(at.min(def.metrics.len())..at.min(def.metrics.len()), pinned);
}

/// EFSA's calcium PRI is higher from 18 to 24.
const CALCIUM_YOUNG_UG: i64 = 1_000_000;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct NutrientRow {
    pub key: String,
    pub label: String,
    /// What the day's counted entries come to, and how many of them carried the value.
    pub sum: i64,
    pub n: usize,
    /// The reference for this person, in the key's own unit, when there is one figure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<i64>,
    /// `PRI` or `AI`, in EFSA's terms; empty when the row has only a limit.
    pub kind: String,
    /// A level not to go past, where the source's applies to food.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upper: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct Nutrients {
    pub day: String,
    /// How many entries the day has, planned ones left out.
    pub of: usize,
    pub rows: Vec<NutrientRow>,
    /// Water, caffeine and alcohol (`DRINKS`), shown apart from the vitamins and minerals.
    pub drinks: Vec<NutrientRow>,
    /// The person's sex is not known, so the references that differ by it are left out.
    pub needs_sex: bool,
    /// The values shown on Day now, by key: the person's pins, or the view's own.
    pub on_day: Vec<String>,
    /// How many values Day shows at most (`MOST_ON_DAY`).
    pub most_on_day: usize,
    /// The person's profile note, which keeps the pins; `None` before they have one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The entries, for a screen that wants to say which one a value came from.
    pub entries: Vec<ObjectMeta>,
}

/// **A key whose unit changed, and how many of the new unit one of the old makes** (2026-10-29,
/// `decisions.md#data`): vitamin B12 and D are kept in nanograms, since in whole micrograms milk's
/// B12 was 0. An entry logged before holds the old key; it is read in the new unit and never
/// rewritten.
pub(crate) const RENAMED: &[(&str, &str, i64)] =
    &[("vitamin_b12_ng", "vitamin_b12_ug", 1000), ("vitamin_d_ng", "vitamin_d_ug", 1000)];

/// An entry's whole number under `key`, or under the key it had before its unit changed, in the
/// new unit. The one reader of a logged value that a total uses.
pub(crate) fn int_of(o: &Object, key: &str) -> Option<i64> {
    match o.get(key) {
        PropertyValue::Int(v) => Some(v),
        _ => RENAMED.iter().find(|(new, ..)| *new == key).and_then(|(_, old, per)| {
            match o.get(old) {
                PropertyValue::Int(v) => Some(v.saturating_mul(*per)),
                _ => None,
            }
        }),
    }
}

/// One day's nutrients, each beside its reference.
pub fn log_nutrients(
    store: &dyn Store,
    def: &TotalsDef,
    day: &str,
    planned: bool,
) -> Result<Nutrients, StoreError> {
    let date = Date::parse(day, &Iso8601::DATE).map_err(|_| {
        StoreError::Parse(format!("day must be a day like 2026-07-20, not '{day}'"))
    })?;
    let q = Query {
        filter: log_base(&def.log),
        sort: vec![SortKey::asc("created")],
        ..Default::default()
    };
    let entries: Vec<Object> = store
        .query(&q)?
        .rows
        .into_iter()
        .filter(|o| o.start.is_some_and(|s| s.date == date))
        // A plan is in no total until it is eaten — except on a day still to come, which is its
        // plan (`planned`): macadamia nuts planned for tomorrow read no selenium (2026-10-29).
        .filter(|o| planned || !matches!(o.get(PLANNED), PropertyValue::Int(n) if n != 0))
        .collect();

    let profile = plan_profile(store)?;
    let female = profile.sex.as_deref().map(|s| s == "female");
    let age = profile.born.map(|b| i64::from(date.year()) - b);
    let kcal: i64 = entries.iter().filter_map(|o| int_of(o, "kcal")).sum();

    let row = |r: &Ref| {
        let got: Vec<i64> = entries.iter().filter_map(|o| int_of(o, r.key)).collect();
        let reference = match (r.adult, r.per_mj) {
            // kcal × 4.184 kJ, ÷ 1000 to MJ, times the figure per MJ.
            (_, Some(per)) => (kcal > 0).then(|| per * kcal * 4184 / 1_000_000),
            (Some((men, women)), _) if men == women => {
                Some(if r.key == "calcium_ug" && age.is_some_and(|a| (18..25).contains(&a)) {
                    CALCIUM_YOUNG_UG
                } else {
                    men
                })
            }
            (Some((men, women)), _) => female.map(|f| if f { women } else { men }),
            _ => None,
        };
        NutrientRow {
            key: r.key.into(),
            label: r.label.into(),
            sum: got.iter().sum(),
            n: got.len(),
            reference,
            kind: r.kind.into(),
            upper: r.upper,
            note: r.note.into(),
        }
    };
    Ok(Nutrients {
        day: day.to_string(),
        of: entries.len(),
        rows: REFS.iter().map(row).collect(),
        drinks: DRINKS.iter().map(row).collect(),
        needs_sex: female.is_none(),
        on_day: {
            let mut shown = def.clone();
            pin(&mut shown, &profile);
            shown
                .metrics
                .iter()
                .filter(|m| m.role == Role::Detail && m.log.is_none())
                .map(|m| m.key.clone())
                .collect()
        },
        most_on_day: MOST_ON_DAY,
        profile: profile.id.clone(),
        entries: entries.iter().map(ObjectMeta::from).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each figure against the table it was read from (EFSA 2017, Tables 5 to 11), in the key's
    /// own unit — a slip of a thousand here would tell someone they are short of a vitamin.
    /// An entry logged before B12 and D moved to nanograms keeps its microgram key, and is read in
    /// the new unit beside a new one: 2 µg and 450 ng are 2,450 ng (2026-10-29).
    #[test]
    fn an_entry_from_before_a_unit_changed_is_read_in_the_new_one() {
        let mut old = Object::new(fm_model::Kind::Note, "");
        old.extra.insert("vitamin_b12_ug".into(), PropertyValue::Int(2));
        let mut new = Object::new(fm_model::Kind::Note, "");
        new.extra.insert("vitamin_b12_ng".into(), PropertyValue::Int(450));
        let total: i64 = [&old, &new].iter().filter_map(|o| int_of(o, "vitamin_b12_ng")).sum();
        assert_eq!(total, 2_450);
        assert_eq!(int_of(&old, "vitamin_d_ng"), None, "a key nobody wrote is still nothing");
        assert_eq!(int_of(&old, "vitamin_b12_ug"), Some(2), "the old key reads as it was");
    }

    #[test]
    fn the_references_are_the_figures_read() {
        let of = |key: &str| REFS.iter().find(|r| r.key == key).unwrap();
        assert_eq!(of("vitamin_c_ug").adult, Some((110_000, 95_000)));
        assert_eq!(of("vitamin_a_rae_ug").adult, Some((750, 650)));
        assert_eq!(of("iron_ug").adult, Some((11_000, 16_000)));
        assert_eq!(of("vitamin_b12_ng").adult, Some((4_000, 4_000)));
        assert_eq!(of("calcium_ug").upper, Some(2_500_000));
        assert_eq!(of("selenium_ug").upper, Some(255));
        assert!(of("zinc_ug").adult.is_none() && !of("zinc_ug").note.is_empty());
        let mut keys: Vec<_> = REFS.iter().map(|r| r.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), REFS.len(), "a key is listed twice");
        for r in REFS {
            assert!(["_ug", "_mg", "_ng"].iter().any(|u| r.key.ends_with(u)), "{}", r.key);
        }
    }

    /// **Pinned values stand in for Food's own details**, in the person's order, at most six, only
    /// values the sheet has; another log's value (weight on Trends) stays; nothing pinned changes
    /// nothing, and every value taken off shows none.
    #[test]
    fn the_values_a_person_pins_are_the_ones_day_and_trends_show() {
        let day = crate::views::totals_named(&[], "Day").unwrap();
        let trends = crate::views::totals_named(&[], "Trends").unwrap();
        let details = |d: &TotalsDef| -> Vec<String> {
            d.metrics.iter().filter(|m| m.role == Role::Detail).map(|m| m.key.clone()).collect()
        };
        let pinned = |keys: Option<&[&str]>, d: &TotalsDef| {
            let mut d = d.clone();
            let profile = Profile {
                day_values: keys.map(|k| k.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            };
            pin(&mut d, &profile);
            d
        };
        assert_eq!(details(&pinned(None, &day)), details(&day), "never chosen: the view's own");
        let mine = pinned(Some(&["water_mg", "iron_ug", "kcal", "nonsense_mg", "water_mg"]), &day);
        assert_eq!(details(&mine), ["water_mg", "iron_ug"], "known values, once each, in order");
        assert_eq!(mine.metrics[0].key, "kcal", "energy and the macros stay where they were");
        let iron = mine.metrics.iter().find(|m| m.key == "iron_ug").unwrap();
        assert_eq!(iron.label, "Iron", "the view's own name where it has one");
        let water = mine.metrics.iter().find(|m| m.key == "water_mg").unwrap();
        assert_eq!(water.label, "Water");
        let many = [
            "water_mg",
            "iron_ug",
            "zinc_ug",
            "folate_dfe_ug",
            "iodine_ug",
            "caffeine_ug",
            "alcohol_mg",
        ];
        assert_eq!(details(&pinned(Some(&many), &day)).len(), MOST_ON_DAY);
        assert!(details(&pinned(Some(&[]), &day)).is_empty(), "all taken off: none");
        // Trends offers the same values, and keeps the body log's.
        assert_eq!(
            details(&pinned(Some(&["water_mg"]), &trends)),
            ["water_mg", "weight_g", "steps"]
        );
        // Another log's view is not Food's.
        let mut body = day.clone();
        body.log = "body".into();
        assert_eq!(details(&pinned(Some(&["water_mg"]), &body)), details(&day));
    }

    /// Food and Health say the same figures for the day's drinks: the Water card's 2.0 and 2.5
    /// litres (ml there, mg here: a millilitre of water is 1,000 mg), and the Caffeine card's
    /// 400 mg (mg there, µg here). One source read, so one number.
    #[test]
    fn the_drinks_figures_are_healths_own() {
        let cards: serde_json::Value =
            serde_json::from_str(include_str!("shipped/health-cards.json")).unwrap();
        let check = |card: &str| {
            cards["cards"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == card)
                .map(|c| c["checks"][0].clone())
                .unwrap()
        };
        let of = |key: &str| DRINKS.iter().find(|r| r.key == key).unwrap();
        let water = check("water");
        let ml = |k: &str| water[k].as_i64().unwrap() * 1_000;
        assert_eq!(of("water_mg").adult, Some((ml("min_male"), ml("min_female"))));
        assert_eq!(
            of("caffeine_ug").upper,
            Some(check("caffeine")["value"].as_i64().unwrap() * 1_000)
        );
        assert!(of("alcohol_mg").adult.is_none() && of("alcohol_mg").upper.is_none(), "no target");
        assert!(DRINKS.iter().all(|d| REFS.iter().all(|r| r.key != d.key)), "shown once");
    }
}
