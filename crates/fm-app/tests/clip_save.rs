//! **A page kept from formicaria's own browser** (`decisions.md` 2026-11-15).
//!
//! `clip_save` is what the browser's add-on calls. What is pinned here:
//!
//!  1. **The note and its copy land together**: one note, its text ending in a link to the copy,
//!     and the copy stored as an asset (*"nothing kept out of the vault"*).
//!  2. **A paper kept from its page is the same note a pasted citation makes**, tagged `paper` with
//!     its metadata as flat properties, plus the page's text and copy.
//!  3. **Anything that is not a web page is refused before anything is written.**
//!  4. **A recipe is read from a page the browser already has**, with no fetch (`recipe_from_page`).

use fm_app::{dispatch, vaults::VaultConfig, App, Host};
use fm_core::MultiStore;
use serde_json::{json, Value};
use std::path::Path;
use tempfile::{tempdir, TempDir};

struct NoHost;
impl Host for NoHost {
    fn open_external(&self, _p: &Path) -> Result<(), String> {
        Err("not in a test".into())
    }
}

fn app() -> (TempDir, TempDir, App) {
    let home = tempdir().unwrap();
    let vault = tempdir().unwrap();
    let store = MultiStore::open(&[("notes".to_string(), vault.path().to_path_buf())]).unwrap();
    let app = App::new(
        store,
        vec![VaultConfig { name: "notes".into(), path: vault.path().to_path_buf(), restic: None }],
        Some(home.path().join("vaults.json")),
        true,
    );
    (home, vault, app)
}

fn call(app: &App, cmd: &str, args: Value, body: &[u8]) -> Result<Value, String> {
    dispatch(cmd, &args, body, app, &NoHost)
        .map(|o| serde_json::from_slice(&o.into_bytes()).unwrap_or(Value::Null))
}

fn get(app: &App, id: &str) -> Value {
    call(app, "get", json!({ "id": id }), b"").unwrap()
}

const PAGE: &str = "<html><head><title>A walk in the hills</title></head>\
                    <body><p>The path climbs gently.</p></body></html>";

#[test]
fn an_article_and_its_copy_land_together() {
    let (_h, vault, app) = app();
    let made = call(
        &app,
        "clip_save",
        json!({
            "url": "https://example.org/walks/hills",
            "title": "A walk in the hills",
            "body": "The path climbs gently.\n\n---\n\nFrom [example.org](https://example.org/walks/hills).",
            "tags": ["web", "shelf:Web"],
            "props": { "source_url": "https://example.org/walks/hills", "saved": "2026-11-15" },
            "snapshot": PAGE,
        }),
        b"",
    )
    .unwrap();
    let id = made["id"].as_str().unwrap();
    let note = get(&app, id);
    assert_eq!(note["title"], "A walk in the hills");
    let body = note["body"].as_str().unwrap();
    assert!(body.starts_with("The path climbs gently."), "{body}");
    assert!(body.contains("[A copy of the page as it was saved](asset:sha256-"), "{body}");
    let tags: Vec<&str> =
        note["tags"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert!(tags.contains(&"web") && tags.contains(&"shelf:Web"), "{tags:?}");

    // The copy is a blob in the vault, byte for byte.
    let hex = body.split("asset:sha256-").nth(1).unwrap().trim_end_matches([')', '\n']);
    let blobs = vault.path().join("blobs");
    let found = walk(&blobs).into_iter().find(|p| p.to_string_lossy().contains(hex));
    let found = found.unwrap_or_else(|| panic!("no blob {hex} under {}", blobs.display()));
    assert_eq!(std::fs::read(found).unwrap(), PAGE.as_bytes());
}

#[test]
fn a_paper_is_the_note_a_pasted_citation_makes() {
    let (_h, _v, app) = app();
    let bib = "@article{lovelace1843,\n  title = {Notes on the Analytical Engine},\n  \
               author = {Lovelace, Ada},\n  year = {1843},\n  doi = {10.1000/xyz123}\n}";
    let made = call(
        &app,
        "clip_save",
        json!({
            "url": "https://journal.example/article/1",
            "title": "Notes on the Analytical Engine",
            "body": "Abstract text.",
            "tags": ["web"],
            "props": { "source_url": "https://journal.example/article/1" },
            "bibtex": bib,
            "snapshot": PAGE,
        }),
        b"",
    )
    .unwrap();
    let note = get(&app, made["id"].as_str().unwrap());
    let tags: Vec<&str> =
        note["tags"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert!(tags.contains(&"paper") && tags.contains(&"web"), "{tags:?}");
    assert_eq!(note["title"], "Notes on the Analytical Engine");
    let text = note.to_string();
    assert!(text.contains("10.1000/xyz123"), "the DOI is kept: {text}");
    assert!(text.contains("1843"), "the year is kept: {text}");
    assert!(note["body"].as_str().unwrap().contains("asset:sha256-"));
}

#[test]
fn only_a_web_page_is_kept() {
    let (_h, vault, app) = app();
    for url in ["file:///etc/passwd", "javascript:alert(1)", "about:blank", ""] {
        let r = call(
            &app,
            "clip_save",
            json!({ "url": url, "title": "x", "body": "y", "snapshot": PAGE }),
            b"",
        );
        assert!(r.is_err(), "{url} was accepted");
    }
    // Refused before anything was written: no blob, no note.
    assert!(walk(&vault.path().join("blobs")).is_empty());
    let recent = call(&app, "recent", json!({}), b"").unwrap();
    assert_eq!(recent.as_array().map(Vec::len).unwrap_or(0), 0, "{recent}");
}

#[test]
fn a_recipe_is_read_from_the_page_without_a_fetch() {
    let (_h, _v, app) = app();
    let page = r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@type":"Recipe","name":"Pasta e ceci",
         "recipeIngredient":["200 g pasta","400 g ceci"],
         "recipeInstructions":[{"@type":"HowToStep","text":"Cook."}],
         "recipeYield":"4"}
        </script></head><body></body></html>"#;
    let r = call(
        &app,
        "recipe_from_page",
        json!({ "url": "https://ricette.example/pasta-e-ceci" }),
        page.as_bytes(),
    )
    .unwrap();
    assert_eq!(r["title"], "Pasta e ceci");
    assert_eq!(r["ingredients"].as_array().unwrap().len(), 2);
    assert_eq!(r["servings"], 4);

    let not = call(&app, "recipe_from_page", json!({ "url": "file:///x" }), page.as_bytes());
    assert!(not.is_err());
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

/// **The source contract and "this page is in your notes"** (`decisions.md` 2026-11-15, late night):
/// a kept page says where it came from, and the page is found again by another form of its address,
/// in a kept page and in a note that only mentions it.
#[test]
fn a_kept_page_carries_its_source_and_is_found_again() {
    let (_h, _vault, app) = app();
    let made = call(
        &app,
        "clip_save",
        json!({
            "url": "https://example.org/walks/hills",
            "title": "A walk in the hills",
            "body": "The path climbs gently.",
            "tags": ["web"],
            "props": { "source_url": "https://example.org/walks/hills" },
        }),
        b"",
    )
    .unwrap();
    let id = made["id"].as_str().unwrap().to_string();
    let note = get(&app, &id);
    let props = note["props"]
        .as_object()
        .or_else(|| note["extra"].as_object())
        .cloned()
        .unwrap_or_default();
    let text = serde_json::to_string(&note).unwrap();
    assert!(text.contains("\"source\"") && text.contains("\"web\""), "{props:?} {text}");
    assert!(text.contains("\"recorded\"") && text.contains("\"copied\""), "{text}");
    assert!(text.contains("\"captured\""), "{text}");

    let other = call(
        &app,
        "create_notes",
        json!({ "notes": [{ "title": "Weekend", "body": "Try [this walk](http://www.example.org/walks/hills/?utm_source=x) soon." }] }),
        b"",
    );
    assert!(other.is_ok(), "{other:?}");
    let found =
        call(&app, "page_notes", json!({ "url": "https://example.org/walks/hills#top" }), b"")
            .unwrap();
    let found = found.as_array().unwrap();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().any(|n| n["id"] == id.as_str()));
    assert!(found.iter().any(|n| n["context"].as_str().unwrap().contains("Try [this walk]")));
    let none =
        call(&app, "page_notes", json!({ "url": "https://example.org/walks/other" }), b"").unwrap();
    assert_eq!(none.as_array().unwrap().len(), 0);
    let kinds = call(&app, "kinds", json!({}), b"").unwrap();
    assert!(kinds.as_array().unwrap().iter().any(|k| k["key"] == "web"));
    // The kept page is recognised as a web page, and says so with every listing of it.
    let listed = call(&app, "search", json!({ "query": "walk" }), b"").unwrap();
    assert!(listed.as_array().unwrap().iter().any(|n| n["kind_key"] == "web"), "{listed}");
}

/// **Mail is its own section** (`decisions.md` 2026-11-16, night): a conversation is not in the study
/// rooms' timeline, and the library's search still finds it. Its file stays where it is.
#[test]
fn a_conversation_stays_in_mail_and_is_still_found() {
    let (_h, _vault, app) = app();
    call(
        &app,
        "create_notes",
        json!({ "notes": [
            { "title": "Field trip plans", "body": "Meet at the gate.", "props": { "gmail_thread": "t-1" } },
            { "title": "Field notes", "body": "Ants at the gate." },
        ] }),
        b"",
    )
    .unwrap();
    let recent = call(&app, "recent", json!({}), b"").unwrap();
    let titles: Vec<_> =
        recent.as_array().unwrap().iter().map(|n| n["title"].as_str().unwrap_or("")).collect();
    assert!(titles.contains(&"Field notes"), "{titles:?}");
    assert!(!titles.contains(&"Field trip plans"), "{titles:?}");
    let found = call(&app, "search", json!({ "query": "gate" }), b"").unwrap();
    assert_eq!(found.as_array().unwrap().len(), 2);
}
