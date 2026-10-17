//! A recipe saved from a web page, for the person who asked: which page may be read, and how the
//! recipe is read out of it.
//!
//! `decisions.md#data`, 2026-10-18, *a recipe can be saved from a link*. The owner: recipes from
//! *"popular food websites used in Italy"* — GialloZafferano, Cucchiaio d'Argento, Fatto in casa da
//! Benedetta, Misya, Sale&Pepe, Cookist and the rest. Those pages own their words and their pictures,
//! so the app **carries none of them**; this is a person keeping a copy of a page they chose, in
//! their own notes, as they could by copying it out by hand.
//!
//! **No HTTP client lives here** (the calendar ruling of 2026-10-09): a shell fetches the page and
//! hands the bytes over. This module refuses an address that is not a public `https` page, and reads
//! the recipe the page itself publishes for search engines — schema.org `Recipe` in JSON-LD, which
//! nearly every recipe site in Italy writes. **Nothing is guessed from the page's look**: a page
//! that does not say which part is the recipe is answered with a sentence, not a scrape.
use serde::Serialize;
use serde_json::Value;

/// A page larger than this is not one recipe.
pub const LIMIT: usize = 4 * 1024 * 1024;

/// What the page says the recipe is — each field the page's own text, or absent.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct WebRecipe {
    pub title: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    /// The page's own words for what it makes ("4 persone").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_text: Option<String>,
    /// Only when the page's yield is one whole number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servings: Option<u32>,
    /// Preparation, cooking and total time, as minutes read from the page's own durations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cook_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_minutes: Option<u32>,
    /// Where the page's picture is — a link, never fetched or kept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Who is asking, as a polite client says.
pub fn user_agent() -> String {
    format!(
        "formicaria/{} (a notes app saving a recipe for its user; https://github.com/formicaria-org/formicaria)",
        option_env!("FM_VERSION").unwrap_or("dev")
    )
}

/// The address to read, when it is one a recipe can be at: `https`, a public host. **Refuses**
/// anything else — another scheme, a bare name, this machine, a home network — so a link pasted
/// on a paired tablet cannot make the computer read something on the person's own network.
pub fn address(link: &str) -> Result<String, String> {
    let link = link.trim();
    let rest = link
        .strip_prefix("https://")
        .ok_or("Paste a link that starts with https:// — the address of the recipe's page.")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    let private = host.is_empty()
        || !host.contains('.')
        || host == "localhost"
        || host.ends_with(".local")
        || host.ends_with(".localhost")
        || host.starts_with('[')
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host.starts_with("0.")
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|o| o.parse::<u8>().ok())
                .is_some_and(|o| (16..=31).contains(&o)));
    if private {
        return Err(
            "That is not a public web page. Paste the recipe's address from the browser.".into()
        );
    }
    Ok(link.to_string())
}

/// Read the recipe out of a page. `Err` is a sentence for the person.
pub fn read(page: &[u8], url: &str) -> Result<WebRecipe, String> {
    let html = String::from_utf8_lossy(page);
    for block in json_ld_blocks(&html) {
        let Ok(v) = serde_json::from_str::<Value>(block.trim()) else { continue };
        if let Some(r) = find_recipe(&v) {
            let mut out = shape(r, url);
            if out.title.is_empty() {
                continue;
            }
            if out.site.is_none() {
                out.site = host_of(url);
            }
            return Ok(out);
        }
    }
    Err("This page does not say which part of it is the recipe, so nothing was taken from it. \
         Copy the recipe into a note by hand, or try the page of the recipe itself."
        .into())
}

/// The read, with the fetch handed in — what a shell calls: an address and a User-Agent in, the
/// answer's status and bytes out.
pub type Fetch<'a> = dyn Fn(&str, &str) -> Result<(u16, Vec<u8>), String> + 'a;
pub fn fetch(link: &str, get: &Fetch) -> Result<WebRecipe, String> {
    let url = address(link)?;
    let (status, page) = get(&url, &user_agent()).map_err(|e| {
        format!("The page could not be read ({e}). Check the link and the connection.")
    })?;
    if status != 200 {
        return Err(format!(
            "The site answered {status} for that address. Check the link — it may have moved."
        ));
    }
    read(&page, &url)
}

fn host_of(url: &str) -> Option<String> {
    let rest = url.split("://").nth(1)?;
    let host = rest.split(['/', '?', '#']).next()?;
    Some(host.trim_start_matches("www.").to_string()).filter(|h| !h.is_empty())
}

/// Every `<script type="application/ld+json">` body in the page.
fn json_ld_blocks(html: &str) -> Vec<&str> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = lower[at..].find("<script") {
        let start = at + i;
        let Some(close) = lower[start..].find('>') else { break };
        let tag = &lower[start..start + close];
        let body_start = start + close + 1;
        let Some(end) = lower[body_start..].find("</script") else { break };
        if tag.contains("application/ld+json") {
            out.push(&html[body_start..body_start + end]);
        }
        at = body_start + end;
    }
    out
}

fn is_recipe(v: &Value) -> bool {
    match &v["@type"] {
        Value::String(s) => s.eq_ignore_ascii_case("Recipe"),
        Value::Array(a) => {
            a.iter().any(|t| t.as_str().is_some_and(|s| s.eq_ignore_ascii_case("Recipe")))
        }
        _ => false,
    }
}

/// The `Recipe` object, wherever the page put it: alone, in a list, or in an `@graph`.
fn find_recipe(v: &Value) -> Option<&Value> {
    if is_recipe(v) {
        return Some(v);
    }
    match v {
        Value::Array(a) => a.iter().find_map(find_recipe),
        Value::Object(o) => o.get("@graph").and_then(find_recipe),
        _ => None,
    }
}

fn shape(r: &Value, url: &str) -> WebRecipe {
    let text = |v: &Value| v.as_str().map(clean).filter(|s| !s.is_empty());
    let ingredients = match &r["recipeIngredient"] {
        Value::Array(a) => a.iter().filter_map(text).collect(),
        Value::String(s) => vec![clean(s)].into_iter().filter(|s| !s.is_empty()).collect(),
        _ => match &r["ingredients"] {
            Value::Array(a) => a.iter().filter_map(text).collect(),
            _ => Vec::new(),
        },
    };
    let mut steps = Vec::new();
    collect_steps(&r["recipeInstructions"], &mut steps);
    let yield_text = match &r["recipeYield"] {
        Value::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(clean).or(v.as_u64().map(|n| n.to_string())))
            .max_by_key(|s| s.len()),
        Value::String(s) => Some(clean(s)),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
    .filter(|s| !s.is_empty());
    let servings = yield_text.as_deref().and_then(|y| {
        let nums: Vec<u32> =
            y.split(|c: char| !c.is_ascii_digit()).filter_map(|n| n.parse().ok()).collect();
        // One number, and not a range ("4-6"): a range is said in words only.
        (nums.len() == 1 && nums[0] > 0 && nums[0] <= 100).then(|| nums[0])
    });
    let image = match &r["image"] {
        Value::String(s) => Some(s.clone()),
        Value::Array(a) => a.iter().find_map(|i| {
            i.as_str().map(str::to_owned).or_else(|| i["url"].as_str().map(str::to_owned))
        }),
        Value::Object(_) => r["image"]["url"].as_str().map(str::to_owned),
        _ => None,
    }
    .filter(|s| s.starts_with("https://"));
    let author = match &r["author"] {
        Value::String(s) => Some(clean(s)),
        Value::Array(a) => a.first().and_then(|x| x["name"].as_str().map(clean)),
        Value::Object(_) => r["author"]["name"].as_str().map(clean),
        _ => None,
    }
    .filter(|s| !s.is_empty());
    WebRecipe {
        title: r["name"].as_str().map(clean).unwrap_or_default(),
        url: url.to_string(),
        site: r["publisher"]["name"].as_str().map(clean).filter(|s| !s.is_empty()),
        author,
        ingredients,
        steps,
        yield_text,
        servings,
        prep_minutes: r["prepTime"].as_str().and_then(minutes),
        cook_minutes: r["cookTime"].as_str().and_then(minutes),
        total_minutes: r["totalTime"].as_str().and_then(minutes),
        image,
    }
}

/// `recipeInstructions` in every form pages write it: one text, a list of texts, `HowToStep`s,
/// and `HowToSection`s holding steps.
fn collect_steps(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.extend(s.split('\n').map(clean).filter(|p| !p.is_empty())),
        Value::Array(a) => a.iter().for_each(|x| collect_steps(x, out)),
        Value::Object(o) => {
            if let Some(items) = o.get("itemListElement") {
                if let Some(name) =
                    o.get("name").and_then(Value::as_str).map(clean).filter(|s| !s.is_empty())
                {
                    out.push(format!("{name}:"));
                }
                collect_steps(items, out);
            } else if let Some(t) = o.get("text").and_then(Value::as_str) {
                let t = clean(t);
                if !t.is_empty() {
                    out.push(t);
                }
            }
        }
        _ => {}
    }
}

/// An ISO 8601 duration as minutes: `PT1H20M` → 80, `P0DT0H45M` → 45. `None` when it is not one.
fn minutes(d: &str) -> Option<u32> {
    let d = d.trim();
    let rest = d.strip_prefix('P')?;
    let (days, time) = match rest.split_once('T') {
        Some((a, b)) => (a, b),
        None => (rest, ""),
    };
    let mut total = 0u32;
    let mut num = String::new();
    for c in days.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else if c == 'D' {
            total += num.parse::<u32>().ok()? * 24 * 60;
            num.clear();
        } else {
            return None;
        }
    }
    for c in time.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
        } else {
            let n = num.parse::<f32>().ok()?;
            num.clear();
            total += match c {
                'H' => (n * 60.0) as u32,
                'M' => n as u32,
                'S' => 0,
                _ => return None,
            };
        }
    }
    (total > 0).then_some(total)
}

/// A field's text as a person reads it: tags out, entities decoded, whitespace made single.
fn clean(s: &str) -> String {
    let mut no_tags = String::with_capacity(s.len());
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => {
                inside = false;
                no_tags.push(' ');
            }
            _ if !inside => no_tags.push(c),
            _ => {}
        }
    }
    let decoded = decode_entities(&no_tags);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        let Some(semi) = after[..after.len().min(12)].find(';') else {
            out.push('&');
            rest = &after[1..];
            continue;
        };
        let name = &after[1..semi];
        let ch = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            "egrave" => Some('è'),
            "eacute" => Some('é'),
            "agrave" => Some('à'),
            "igrave" => Some('ì'),
            "ograve" => Some('ò'),
            "ugrave" => Some('ù'),
            "Egrave" => Some('È'),
            "deg" => Some('°'),
            "frac12" => Some('½'),
            "frac14" => Some('¼'),
            "rsquo" | "lsquo" => Some('’'),
            "ldquo" | "rdquo" => Some('"'),
            "ndash" => Some('–'),
            "mdash" => Some('—'),
            _ if name.starts_with("#x") || name.starts_with("#X") => {
                u32::from_str_radix(&name[2..], 16).ok().and_then(char::from_u32)
            }
            _ if name.starts_with('#') => name[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &after[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape Italian recipe sites publish (written here from the schema.org vocabulary, not
    /// copied from any of them): a `@graph`, HowToSteps, a yield in words, ISO durations.
    const PAGE: &str = r#"<html><head><title>x</title>
<script type="application/ld+json">{"@context":"https://schema.org","@graph":[
 {"@type":"WebPage","name":"page"},
 {"@type":["Recipe"],"name":"Spaghetti alla carbonara","author":{"@type":"Person","name":"Redazione"},
  "publisher":{"@type":"Organization","name":"Un sito di ricette"},
  "image":["https://example.org/carbonara.jpg"],
  "recipeYield":"4 persone","prepTime":"PT10M","cookTime":"PT15M","totalTime":"PT25M",
  "recipeIngredient":["Spaghetti 320 g","Guanciale 150 g","Tuorli 6","Pecorino romano 50 g","Pepe nero q.b."],
  "recipeInstructions":[{"@type":"HowToStep","text":"Tagliate il guanciale a listarelle."},
   {"@type":"HowToSection","name":"La crema","itemListElement":[{"@type":"HowToStep","text":"Sbattete i tuorli con il pecorino &amp; il pepe."}]}]}
]}</script></head><body>…</body></html>"#;

    #[test]
    fn a_page_s_own_recipe_is_read_and_nothing_else() {
        let r = read(PAGE.as_bytes(), "https://www.example.org/carbonara").unwrap();
        assert_eq!(r.title, "Spaghetti alla carbonara");
        assert_eq!(r.ingredients.len(), 5);
        assert_eq!(r.ingredients[0], "Spaghetti 320 g");
        assert_eq!(
            r.steps,
            vec![
                "Tagliate il guanciale a listarelle.",
                "La crema:",
                "Sbattete i tuorli con il pecorino & il pepe."
            ]
        );
        assert_eq!(r.servings, Some(4));
        assert_eq!(r.yield_text.as_deref(), Some("4 persone"));
        assert_eq!(
            (r.prep_minutes, r.cook_minutes, r.total_minutes),
            (Some(10), Some(15), Some(25))
        );
        assert_eq!(r.image.as_deref(), Some("https://example.org/carbonara.jpg"));
        assert_eq!(r.site.as_deref(), Some("Un sito di ricette"));
        assert_eq!(r.author.as_deref(), Some("Redazione"));
    }

    #[test]
    fn a_page_that_does_not_say_where_its_recipe_is_gives_a_sentence() {
        let e = read(
            b"<html><body><h1>Pasta</h1><ul><li>200 g</li></ul></body></html>",
            "https://x.org/a",
        )
        .unwrap_err();
        assert!(e.contains("does not say"), "{e}");
    }

    #[test]
    fn only_a_public_https_page_is_read() {
        assert!(address("https://www.giallozafferano.it/ricette/x").is_ok());
        for bad in [
            "http://example.org/r",
            "file:///etc/passwd",
            "https://localhost/r",
            "https://127.0.0.1/r",
            "https://192.168.1.10/r",
            "https://172.20.0.5/r",
            "https://router.local/r",
            "https://intranet/r",
            "https://[::1]/r",
        ] {
            assert!(address(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn durations_and_yields_read_as_the_page_wrote_them() {
        assert_eq!(minutes("PT1H20M"), Some(80));
        assert_eq!(minutes("P0DT0H45M"), Some(45));
        assert_eq!(minutes("PT0M"), None);
        assert_eq!(minutes("20 minuti"), None);
        let ranged = PAGE.replace("\"4 persone\"", "\"4-6 persone\"");
        assert_eq!(read(ranged.as_bytes(), "https://x.org/a").unwrap().servings, None);
    }

    /// Real pages, read from a folder named by `FM_WEB_RECIPE_PAGES` — never copied into the repo,
    /// since the pages are their sites' own. `cargo test -p fm-app web_recipe -- --ignored`.
    #[test]
    #[ignore]
    fn real_pages_in_a_folder_each_give_a_recipe() {
        let Ok(dir) = std::env::var("FM_WEB_RECIPE_PAGES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let page = std::fs::read(e.path()).unwrap();
            let r = read(&page, "https://example.org/r")
                .unwrap_or_else(|why| panic!("{:?}: {why}", e.path()));
            assert!(!r.ingredients.is_empty() && !r.steps.is_empty(), "{:?}: {r:?}", e.path());
            println!("{:?}: {} — {} ingredients, {} steps, servings {:?}, {:?}+{:?} min, by {:?} at {:?}",
                e.file_name(), r.title, r.ingredients.len(), r.steps.len(), r.servings,
                r.prep_minutes, r.cook_minutes, r.author, r.site);
        }
    }
}
