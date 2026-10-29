//! Looking a packaged food up by its barcode: the address to ask, and how the answer is read.
//!
//! `decisions.md#data`, 2026-10-12, *a barcode may be looked up, once, when the person asks*.
//! **No HTTP client lives here** (the calendar ruling of 2026-10-09): a shell makes the request
//! and hands the bytes over; this module says which address, with which name, and turns the
//! answer into the label form's fields. The person then saves it or does not — nothing here
//! writes.
//!
//! The source is Open Food Facts (terms in `sources-2026-10-11.md` §6c): one read of one product
//! per press, no account, version 2 of its API because version 3.6 answers an empty nutrition
//! block, and a User-Agent that says who is asking.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Where a product is read from. The only address of theirs this app knows (`ci/checks.sh`).
const READ: &str = "https://world.openfoodfacts.org/api/v2/product";
/// Where a person can see the same product, and what its note links to.
const PAGE: &str = "https://world.openfoodfacts.org/product";
/// What is asked for: the label, and nothing that is somebody's picture or score. Its name and
/// ingredients in English too, its categories (named in English) and where it is sold: a product first typed in by someone in another country comes back in their
/// language (2026-10-29, `decisions.md#data`).
const FIELDS: &str = "code,product_name,product_name_en,brands,serving_quantity,\
                      serving_quantity_unit,ingredients_text,ingredients_text_en,categories_tags,\
                      countries_tags,nutriments";
/// An answer larger than this is not one product.
pub const LIMIT: usize = 512 * 1024;

/// Who is asking, as Open Food Facts asks to be told: the app, its version, where it lives.
pub fn user_agent() -> String {
    format!(
        "formicaria/{} (https://github.com/formicaria-org/formicaria)",
        option_env!("FM_VERSION").unwrap_or("dev")
    )
}

/// The digits of a barcode, when they are one by GS1's check digit: weights 3 and 1 alternating
/// from the digit beside the last, and the last is what brings the sum to a multiple of ten.
pub fn barcode(text: &str) -> Option<String> {
    let d: String = text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if !matches!(d.len(), 8 | 12 | 13 | 14) || !d.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits: Vec<u32> = d.bytes().map(|b| u32::from(b - b'0')).collect();
    let (check, rest) = digits.split_last()?;
    let sum: u32 =
        rest.iter().rev().enumerate().map(|(i, v)| v * if i % 2 == 0 { 3 } else { 1 }).sum();
    ((10 - sum % 10) % 10 == *check).then_some(d)
}

/// The address to read a product from. **Refuses anything that is not a barcode**, so nothing
/// else a person typed is ever sent.
pub fn address(code: &str) -> Result<String, String> {
    let digits =
        barcode(code).ok_or("that is not a barcode number: its last digit does not check")?;
    Ok(format!("{READ}/{digits}?fields={FIELDS}"))
}

/// A product as its label would be typed in: what 100 g hold, by the keys every food uses.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Found {
    /// Its name as Open Food Facts has it, with the brand: in whatever language it was typed in.
    /// Which language that is is not said: their record of it is often wrong (an Italian name filed
    /// as Spanish, another as English).
    pub name: String,
    /// **What it is, in English**, when that says more than the name: their English name, else
    /// their most specific category, else the app's food list's food of that name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub english: Option<English>,
    /// Where they say it is sold, by country name.
    pub sold_in: Vec<String>,
    /// Where its barcode was issued (GS1's prefix): not where it was made, nor the brand's home.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub registered: String,
    pub barcode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_g: Option<i64>,
    pub per100: BTreeMap<String, i64>,
    pub ingredients: String,
    /// The ingredients in English, when they have them and the label's are in another language.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ingredients_en: String,
    /// The product's own page, to link to and to credit.
    pub page: String,
}

/// A product's name in English, and where that came from.
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct English {
    pub name: String,
    /// `name` (their English name), `category` (their category) or `list` (the app's food list).
    pub from: &'static str,
}

/// The label's lines and the keys they are kept under, with how many of the key's unit a gram is.
const LINES: &[(&str, &str, f64)] = &[
    ("fat_100g", "fat_mg", 1_000.0),
    ("saturated-fat_100g", "saturated_fat_mg", 1_000.0),
    ("carbohydrates_100g", "carbs_mg", 1_000.0),
    ("sugars_100g", "sugars_mg", 1_000.0),
    ("fiber_100g", "fiber_mg", 1_000.0),
    ("proteins_100g", "protein_mg", 1_000.0),
    // They give sodium in grams beside salt; the tables here are in sodium.
    ("sodium_100g", "sodium_ug", 1_000_000.0),
];

fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        // Some of their fields are numbers written as text.
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|n| n.is_finite() && *n >= 0.0)
}

/// Read their answer. `None` when they have no such product.
pub fn read(body: &[u8]) -> Result<Option<Found>, String> {
    let v: Value = serde_json::from_slice(body)
        .map_err(|_| "Open Food Facts answered something that is not a product".to_string())?;
    let Some(p) = v.get("product").filter(|_| v.get("status").and_then(Value::as_i64) == Some(1))
    else {
        return Ok(None);
    };
    let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let (name, brand) = (text("product_name"), text("brands"));
    let tags = |k: &str| -> Vec<String> {
        p.get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
            .unwrap_or_default()
    };
    let (name_en, ingredients, ingredients_en) =
        (text("product_name_en"), text("ingredients_text"), text("ingredients_text_en"));
    // Their English name counts only when it is another name: some records hold the label's own
    // words in the English field ("CECI SOTTOVUOTO").
    let english = if !name_en.is_empty() && !same_words(&name_en, &name) {
        Some(English { name: name_en, from: "name" })
    } else {
        // A category whose words are all in the name already (*Hummus* for "Hummus ceci") says
        // nothing more.
        category(&tags("categories_tags"))
            .filter(|c| !within(c, &name))
            .map(|c| English { name: c, from: "category" })
    };
    let n = p.get("nutriments").cloned().unwrap_or(Value::Null);
    let mut per100 = BTreeMap::new();
    if let Some(kcal) = n.get("energy-kcal_100g").and_then(number) {
        per100.insert("kcal".to_string(), kcal.round() as i64);
    }
    for (theirs, ours, per) in LINES {
        if let Some(g) = n.get(*theirs).and_then(number) {
            per100.insert((*ours).to_string(), (g * per).round() as i64);
        }
    }
    let code = text("code");
    let grams = text("serving_quantity_unit");
    Ok(Some(Found {
        english,
        sold_in: tags("countries_tags")
            .iter()
            .filter_map(|t| t.strip_prefix("en:"))
            .map(country)
            .collect(),
        registered: registered(&code).unwrap_or_default().to_string(),
        ingredients_en: if same_words(&ingredients_en, &ingredients) {
            String::new()
        } else {
            ingredients_en
        },
        name: match (name.is_empty(), brand.is_empty()) {
            (false, false) if !name.to_lowercase().contains(&brand.to_lowercase()) => {
                format!("{name} ({brand})")
            }
            (true, false) => brand,
            _ => name,
        },
        serving_g: p
            .get("serving_quantity")
            .and_then(number)
            .filter(|q| *q >= 1.0 && (grams.is_empty() || grams == "g"))
            .map(|q| q.round() as i64),
        per100,
        ingredients,
        page: format!("{PAGE}/{code}"),
        barcode: code,
    }))
}

/// Two texts that are the same words, case and accents aside.
fn same_words(a: &str, b: &str) -> bool {
    let fold = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    fold(a) == fold(b)
}

/// A tag as words: `canned-chickpeas` → `Canned chickpeas`.
fn words(tag: &str) -> String {
    let w = tag.replace('-', " ");
    let mut c = w.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// A country tag as its name: `bosnia-and-herzegovina` → `Bosnia and Herzegovina`.
fn country(tag: &str) -> String {
    tag.split('-')
        .enumerate()
        .map(|(i, w)| {
            if i > 0 && matches!(w, "and" | "of" | "the") {
                w.to_string()
            } else {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Their most specific category in English (`en:canned-chickpeas` → `Canned chickpeas`): the last of
/// their own taxonomy's English tags, which are slugs. A tag in another language (`sk:cicer`) is
/// one nobody has translated yet, and free text someone typed (`en:Vegetables - Prepared/Processed
/// (Shelf Stable)`) is not theirs; both are passed over.
fn category(tags: &[String]) -> Option<String> {
    tags.iter()
        .rev()
        .filter_map(|t| t.strip_prefix("en:"))
        .find(|t| {
            !t.is_empty()
                && t.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        .map(words)
}

/// Whether every word of `part` is in `whole`, case aside.
fn within(part: &str, whole: &str) -> bool {
    let w = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|x| !x.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let whole = w(whole);
    w(part).iter().all(|x| whole.contains(x))
}

/// **Where a barcode was issued**, by GS1's prefix: the country of the GS1 office that gave the
/// brand its numbers. Not where the food was made, nor where the brand is from — a German chain's
/// own brand sold in Italy carries a German number. `None` for a prefix not listed here, for
/// in-store codes (20–29) and for books and magazines.
pub fn registered(code: &str) -> Option<&'static str> {
    // A 12-digit UPC is an EAN-13 with a leading 0.
    let code = if code.len() == 12 { format!("0{code}") } else { code.to_string() };
    if code.len() != 13 {
        return None;
    }
    let p: u32 = code.get(..3)?.parse().ok()?;
    Some(match p {
        0..=19 | 30..=39 | 60..=139 => "the United States or Canada",
        300..=379 => "France",
        380 => "Bulgaria",
        383 => "Slovenia",
        385 => "Croatia",
        400..=440 => "Germany",
        450..=459 | 490..=499 => "Japan",
        460..=469 => "Russia",
        471 => "Taiwan",
        489 => "Hong Kong",
        500..=509 => "the United Kingdom",
        520..=521 => "Greece",
        540..=549 => "Belgium or Luxembourg",
        560 => "Portugal",
        569 => "Iceland",
        570..=579 => "Denmark",
        590 => "Poland",
        594 => "Romania",
        599 => "Hungary",
        600..=601 => "South Africa",
        640..=649 => "Finland",
        690..=699 => "China",
        700..=709 => "Norway",
        729 => "Israel",
        730..=739 => "Sweden",
        750 => "Mexico",
        760..=769 => "Switzerland",
        789..=790 => "Brazil",
        800..=839 => "Italy",
        840..=849 => "Spain",
        858 => "Slovakia",
        859 => "the Czech Republic",
        860 => "Serbia",
        868..=869 => "Turkey",
        870..=879 => "the Netherlands",
        880 => "South Korea",
        885 => "Thailand",
        888 => "Singapore",
        890 => "India",
        893 => "Vietnam",
        899 => "Indonesia",
        900..=919 => "Austria",
        930..=939 => "Australia",
        940..=949 => "New Zealand",
        955 => "Malaysia",
        _ => return None,
    })
}

/// **The food list's English name for a product named in another language**: its first food found
/// by that language's own name (the list keeps each table's own names beside the English one, and
/// says which was matched). A food found only by its English name is no translation and is not
/// taken. The brand, in brackets, is left out of the search.
fn from_list(name: &str, search: &dyn Fn(&str) -> Vec<fm_core::pack::FoodHit>) -> Option<English> {
    let q = name.split(" (").next().unwrap_or(name).trim();
    if q.is_empty() {
        return None;
    }
    search(q)
        .into_iter()
        .find(|h| !h.alt.is_empty())
        .map(|h| English { name: h.name, from: "list" })
}

/// The shell's side of a look-up: given the address and the User-Agent, the status and the body,
/// capped at [`LIMIT`].
pub type Fetch<'a> = dyn Fn(&str, &str) -> Result<(u16, Vec<u8>), String> + 'a;

/// Ask for one product. When their record names it in English nowhere, the app's food list is
/// asked (on the device, nothing sent) for a food of that name.
pub fn lookup(code: &str, fetch: &Fetch) -> Result<Option<Found>, String> {
    let (status, body) = fetch(&address(code)?, &user_agent())?;
    match status {
        200 => Ok(read(&body)?.map(|mut f| {
            if f.english.is_none() {
                f.english =
                    from_list(&f.name, &|q| crate::packs::food_search(q, 5).unwrap_or_default());
            }
            f
        })),
        404 => Ok(None),
        429 | 503 => Err("Open Food Facts is asking for fewer requests just now. Try again in a \
                          minute, or type the label in."
            .into()),
        other => Err(format!("Open Food Facts answered {other}. You can type the label in.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_barcode_is_ever_asked_about() {
        // GS1's worked example.
        assert_eq!(barcode("6291041500213").as_deref(), Some("6291041500213"));
        assert_eq!(barcode(" 3017 6240-10701 ").as_deref(), Some("3017624010701"));
        assert_eq!(barcode("96385074").as_deref(), Some("96385074"));
        assert!(barcode("6291041500214").is_none());
        assert!(barcode("my lunch").is_none() && barcode("12345").is_none());
        let a = address("3017624010701").unwrap();
        assert!(
            a.starts_with("https://world.openfoodfacts.org/api/v2/product/3017624010701?fields=")
        );
        assert!(address("../search?q=x").is_err());
        assert!(user_agent().starts_with("formicaria/") && user_agent().contains("https://"));
    }

    /// The shape their version 2 answered on 2026-10-12, cut to what is read.
    #[test]
    fn their_answer_becomes_a_label_per_100_g() {
        let body = br#"{"code":"3017624010701","status":1,"status_verbose":"product found",
          "product":{"code":"3017624010701","product_name":"Nutella","brands":"Ferrero",
            "ingredients_text":"sugar, palm oil, hazelnuts","serving_quantity":"15",
            "serving_quantity_unit":"g",
            "nutriments":{"energy-kcal_100g":539,"energy-kj_100g":2228,"fat_100g":30.9,
              "saturated-fat_100g":10.6,"carbohydrates_100g":57.5,"sugars_100g":56.3,
              "proteins_100g":6.3,"salt_100g":0.1075,"sodium_100g":0.043}}}"#;
        let p = read(body).unwrap().unwrap();
        assert_eq!(p.name, "Nutella (Ferrero)");
        assert_eq!((p.barcode.as_str(), p.serving_g), ("3017624010701", Some(15)));
        assert_eq!(p.per100["kcal"], 539);
        assert_eq!(p.per100["fat_mg"], 30_900);
        assert_eq!(p.per100["sodium_ug"], 43_000);
        assert!(!p.per100.contains_key("fiber_mg"), "what they do not state is not made up");
        assert_eq!(p.page, "https://world.openfoodfacts.org/product/3017624010701");
        assert!(read(br#"{"code":"1","status":0,"status_verbose":"product not found"}"#)
            .unwrap()
            .is_none());
        assert!(read(b"<html>").is_err());
    }

    #[test]
    fn a_look_up_sends_one_request_and_says_plainly_when_it_cannot() {
        let asked = std::cell::RefCell::new(Vec::new());
        let got = lookup("3017624010701", &|url, agent| {
            asked.borrow_mut().push((url.to_string(), agent.to_string()));
            Ok((404, Vec::new()))
        });
        assert_eq!(got, Ok(None));
        assert_eq!(asked.borrow().len(), 1);
        assert!(lookup("3017624010701", &|_, _| Ok((429, Vec::new())))
            .unwrap_err()
            .contains("minute"));
        // Not a barcode: refused before anything is sent.
        assert!(lookup("hello", &|_, _| panic!("nothing may be sent")).is_err());
    }

    /// A tin bought in Italy whose record a German contributor typed in (2026-10-29): its name is
    /// German, the English comes from their category, and where it is sold and where its barcode was
    /// issued are said.
    #[test]
    fn a_product_typed_in_another_language_says_so_and_what_it_is_in_english() {
        let body = br#"{"status":1,"product":{"code":"4056489123457","lang":"de",
          "product_name":"Kichererbsen","product_name_en":"","brands":"Freshona",
          "ingredients_text":"Kichererbsen, Wasser, Salz","ingredients_text_en":"Chickpeas, water, salt",
          "categories_tags":["en:plant-based-foods","en:legumes","en:chickpeas","sk:cicer"],
          "countries_tags":["en:germany","en:italy","en:bosnia-and-herzegovina"],
          "nutriments":{"energy-kcal_100g":112}}}"#;
        let p = read(body).unwrap().unwrap();
        assert_eq!(p.name, "Kichererbsen (Freshona)");
        assert_eq!(p.english, Some(English { name: "Chickpeas".into(), from: "category" }));
        assert_eq!(p.sold_in, ["Germany", "Italy", "Bosnia and Herzegovina"]);
        assert_eq!(p.registered, "Germany");
        assert_eq!(p.ingredients_en, "Chickpeas, water, salt");

        // Their English name, when it is another name, comes first.
        let named = br#"{"status":1,"product":{"code":"8001234567890","lang":"it",
          "product_name":"Ceci lessati","product_name_en":"Boiled chickpeas","categories_tags":["en:chickpeas"]}}"#;
        let p = read(named).unwrap().unwrap();
        assert_eq!(p.english, Some(English { name: "Boiled chickpeas".into(), from: "name" }));
        assert_eq!(p.registered, "Italy");

        // As three real records were on 2026-10-29: the label's own words in the English field are
        // no English name, free text among the categories is not one of theirs, and a category
        // whose words are all in the name says nothing more.
        let same = br#"{"status":1,"product":{"code":"3083681003949","lang":"en",
          "product_name":"CECI SOTTOVUOTO","product_name_en":"CECI SOTTOVUOTO",
          "categories_tags":["en:legumes","en:chickpeas","en:canned-chickpeas",
            "en:Vegetables - Prepared/Processed (Shelf Stable)","sk:cicer"]}}"#;
        let p = read(same).unwrap().unwrap();
        assert_eq!(p.english.unwrap().name, "Canned chickpeas");
        assert_eq!(p.registered, "France");
        let hummus = br#"{"status":1,"product":{"code":"8017596128164","lang":"en",
          "product_name":"Hummus ceci","categories_tags":["en:dips","en:hummus"]}}"#;
        assert_eq!(read(hummus).unwrap().unwrap().english, None);
        let filed_as_spanish = br#"{"status":1,"product":{"code":"8003712010376","lang":"es",
          "product_name":"Sfogliette ai ceci","categories_tags":["en:snacks","en:appetizers"]}}"#;
        assert_eq!(read(filed_as_spanish).unwrap().unwrap().english.unwrap().name, "Appetizers");
    }

    #[test]
    fn the_food_list_names_it_only_when_found_by_its_own_language_s_name() {
        let hit = |name: &str, alt: &str| fm_core::pack::FoodHit {
            id: "x:1".into(),
            name: name.into(),
            src: "bls".into(),
            kcal: 0,
            alt: alt.into(),
        };
        let search = |q: &str| {
            assert_eq!(q, "Kichererbsen", "the brand is not searched");
            vec![hit("Chickpea, ripe seeds, dried", "Kichererbse reif getrocknet")]
        };
        assert_eq!(
            from_list("Kichererbsen (Freshona)", &search),
            Some(English { name: "Chickpea, ripe seeds, dried".into(), from: "list" })
        );
        assert_eq!(from_list("Hummus", &|_| vec![hit("Hummus", "")]), None);
    }

    #[test]
    fn a_barcode_says_where_it_was_issued_not_where_it_was_made() {
        assert_eq!(registered("4056489123457"), Some("Germany"));
        assert_eq!(registered("8001234567890"), Some("Italy"));
        assert_eq!(registered("3017624010701"), Some("France"));
        assert_eq!(registered("036000291452"), Some("the United States or Canada"));
        assert_eq!(registered("2001234567893"), None, "an in-store code");
        assert_eq!(registered("9780306406157"), None, "a book");
        assert_eq!(registered("96385074"), None);
    }
}
