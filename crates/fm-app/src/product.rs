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
/// What is asked for: the label, and nothing that is somebody's picture or score.
const FIELDS: &str = "code,product_name,brands,serving_quantity,serving_quantity_unit,\
                      ingredients_text,nutriments";
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
    pub name: String,
    pub barcode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serving_g: Option<i64>,
    pub per100: BTreeMap<String, i64>,
    pub ingredients: String,
    /// The product's own page, to link to and to credit.
    pub page: String,
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
        ingredients: text("ingredients_text"),
        page: format!("{PAGE}/{code}"),
        barcode: code,
    }))
}

/// The shell's side of a look-up: given the address and the User-Agent, the status and the body,
/// capped at [`LIMIT`].
pub type Fetch<'a> = dyn Fn(&str, &str) -> Result<(u16, Vec<u8>), String> + 'a;

/// Ask for one product.
pub fn lookup(code: &str, fetch: &Fetch) -> Result<Option<Found>, String> {
    let (status, body) = fetch(&address(code)?, &user_agent())?;
    match status {
        200 => read(&body),
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
}
