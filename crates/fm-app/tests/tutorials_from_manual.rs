//! **The tutorials are the manual's pages** (`decisions.md#ui`, 2026-10-24): the notes the app ships
//! under `src/shipped/tutorials/` must be exactly what `docs/src/tutorials/` makes of them. This
//! fails while they differ, which is what keeps a page from changing without its note.
//!
//! **To regenerate:** `pixi run tutorials` — this same test with `FM_BLESS_TUTORIALS=1`, which
//! writes the notes and, for each whose text changed since the last commit, appends the committed
//! text's hash to `earlier.txt`, so a copy nobody changed is brought up to date
//! (`tutorials::update`). The committed text, not the file on disk: regenerating while a page is
//! being written must not record each draft as a text that shipped.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The manual's tutorial pages, in order: `1-start-here.md`, `2-…`.
fn pages() -> Vec<(String, String)> {
    let dir = root().join("docs/src/tutorials");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let numbered = name.ends_with(".md") && name.starts_with(|c: char| c.is_ascii_digit());
            numbered.then(|| (name, std::fs::read_to_string(e.path()).unwrap()))
        })
        .collect();
    out.sort();
    out
}

fn body(text: &str) -> String {
    fm_core::frontmatter::from_file(text).expect("a tutorial reads").body
}

/// A file as the last commit holds it, or None (not in git yet, or no git).
fn committed(path: &str) -> Option<String> {
    let out = std::process::Command::new("git")
        .current_dir(root())
        .args(["show", &format!("HEAD:{path}")])
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn the_shipped_tutorials_are_what_the_manual_pages_make() {
    let pages = pages();
    let borrowed: Vec<(&str, &str)> = pages.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    let made = fm_app::tutorials::notes_from_pages(&borrowed).unwrap();
    let shipped = root().join("crates/fm-app/src/shipped/tutorials");

    if std::env::var_os("FM_BLESS_TUTORIALS").is_some() {
        let earlier = shipped.join("earlier.txt");
        let mut log = std::fs::read_to_string(&earlier).unwrap_or_default();
        for (name, text) in &made {
            let path = shipped.join(name);
            if let Some(old) = committed(&format!("crates/fm-app/src/shipped/tutorials/{name}")) {
                let (old_body, new) = (body(&old), fm_core::frontmatter::from_file(text).unwrap());
                let line = format!("{} {}\n", new.id, fm_app::dto::version_of(&old_body));
                if old_body != new.body && !log.contains(&line) {
                    log.push_str(&line);
                }
            }
            std::fs::write(&path, text).unwrap();
        }
        std::fs::write(&earlier, log).unwrap();
        for e in std::fs::read_dir(&shipped).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") && !made.iter().any(|(n, _)| *n == name) {
                std::fs::remove_file(e.path()).unwrap();
            }
        }
        eprintln!("tutorials: wrote {} note(s) from docs/src/tutorials/", made.len());
        return;
    }

    let compiled = fm_app::tutorials::TUTORIALS;
    assert_eq!(
        compiled.len(),
        made.len(),
        "docs/src/tutorials/ has {} page(s) and `tutorials::TUTORIALS` lists {}",
        made.len(),
        compiled.len()
    );
    for ((name, text), (file, have)) in made.iter().zip(compiled) {
        let id = fm_core::frontmatter::from_file(text).unwrap().id;
        assert_eq!(*file, format!("{id}.md"), "{name}: TUTORIALS lists it under another id");
        assert!(
            *have == text.as_str(),
            "docs/src/tutorials/{name} changed and its note did not: run `pixi run tutorials`"
        );
    }
}

/// A copy holding the text shipped before is brought up to date; one the person changed is not,
/// and neither loses its tags. The fixture is *Write, and find it again* as 0.17 shipped it.
#[test]
fn an_unchanged_copy_is_updated_and_a_changed_one_is_left() {
    use fm_core::{MemoryStore, Store};
    let old = fm_core::frontmatter::from_file(include_str!(
        "fixtures/tutorial-2-as-shipped-2026-10-18.md"
    ))
    .unwrap();
    assert!(
        fm_app::tutorials::was_shipped(&old.id.to_string(), &old.body),
        "earlier.txt knows the text 0.17 shipped"
    );
    let (_, now_text) = fm_app::tutorials::TUTORIALS
        .iter()
        .find(|(f, _)| f.trim_end_matches(".md") == old.id.to_string())
        .expect("it still ships");
    let now = fm_core::frontmatter::from_file(now_text).unwrap();
    assert_ne!(old.body, now.body, "the fixture is an earlier text");

    let mut s = MemoryStore::new();
    let mut unchanged = old.clone();
    unchanged.tags.push("mine".into());
    s.put(&unchanged).unwrap();
    assert_eq!(fm_app::tutorials::update(&mut s).unwrap().len(), 1);
    let got = s.get(old.id).unwrap().unwrap();
    assert_eq!(got.body, now.body, "brought up to date");
    assert_eq!(got.title, now.title);
    assert!(got.tags.contains(&"mine".to_string()), "its tags are the person's");
    assert!(fm_app::tutorials::update(&mut s).unwrap().is_empty(), "once");

    let mut changed = old.clone();
    changed.body.push_str("\nMy own line.\n");
    s.put(&changed).unwrap();
    assert!(fm_app::tutorials::update(&mut s).unwrap().is_empty(), "a changed copy is theirs");
    assert_eq!(s.get(old.id).unwrap().unwrap().body, changed.body);
}
