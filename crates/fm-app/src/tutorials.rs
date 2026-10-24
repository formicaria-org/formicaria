//! **The tutorials** (`decisions.md#ui`, 2026-10-18, *Home is the Library's shelves; tutorials are
//! notes*): a few short notes on the **Tutorials** shelf, the same on every device.
//!
//! They are ordinary notes — read, changed, deleted like any other — written in three cases:
//! **a notebook this person has just made** ([`write_into`], when its `notes/` folder is new and
//! empty), **at every start, to whoever has none** ([`give`], 2026-10-20, *everyone has the
//! tutorials*), and **when asked** ([`put_back`], Help → *Put the tutorials back*).
//!
//! **Fixed ids and fixed bytes**, so two devices that each make a notebook and are later joined
//! hold the *same* files, which merge as one — not each tutorial twice.
//!
//! **Written in the manual, never here** (2026-10-24, *the tutorials are pages of the manual*). The
//! files under `shipped/tutorials/` are generated from `docs/src/tutorials/` by
//! [`notes_from_pages`]: edit the page, then `pixi run tutorials`. `tests/tutorials_from_manual.rs`
//! fails while the two disagree, so a page cannot change without the note it ships as. A copy a
//! person never changed is brought up to the current text at start ([`update`]).

use crate::dto::ObjectMeta;
use fm_core::Store;
use std::path::Path;

/// The tutorials, in shelf order: (file name, file text).
pub const TUTORIALS: &[(&str, &str)] = &[
    ("01M191H1763EX62JM1DZMBK9A9.md", include_str!("shipped/tutorials/1-start-here.md")),
    ("01M5JA00000000000000000T02.md", include_str!("shipped/tutorials/2-write-and-find.md")),
    ("01M5JA00000000000000000T03.md", include_str!("shipped/tutorials/3-your-way.md")),
    ("01M5JA00000000000000000T04.md", include_str!("shipped/tutorials/4-keep-them-safe.md")),
    ("01M5JA00000000000000000T05.md", include_str!("shipped/tutorials/5-ask-myrme.md")),
];

/// Every tutorial text shipped before this one, as `<id> <sha256 of its body>` per line — appended
/// by `pixi run tutorials` whenever a page changes. A copy holding one of these is unchanged.
const EARLIER: &str = include_str!("shipped/tutorials/earlier.txt");

/// Whether `body` is a text this app once shipped for the tutorial `id`.
pub fn was_shipped(id: &str, body: &str) -> bool {
    let hash = crate::dto::version_of(body);
    EARLIER.lines().any(|l| l.split_once(' ') == Some((id, hash.as_str())))
}

/// **A copy nobody changed is brought up to date** (2026-10-24): a tutorial whose text is exactly
/// one shipped before gets the current title and text, and its `updated` the shipped one, so two
/// devices doing this write the same bytes. One the person changed is theirs and stays. Its tags and
/// properties are never touched. Answers the ones updated.
pub fn update(store: &mut dyn Store) -> Result<Vec<ObjectMeta>, fm_core::StoreError> {
    let mut updated = Vec::new();
    for (_, text) in TUTORIALS {
        let now = fm_core::frontmatter::from_file(text)
            .map_err(|e| fm_core::StoreError::Parse(format!("a tutorial does not read: {e}")))?;
        let Some(mut have) = store.get(now.id)? else { continue };
        if have.body == now.body || !was_shipped(&now.id.to_string(), &have.body) {
            continue;
        }
        have.title = now.title;
        have.body = now.body;
        have.updated = now.updated;
        store.put(&have)?;
        updated.push(ObjectMeta::from(&have));
    }
    Ok(updated)
}

/// The tag every tutorial carries, and the shelf it stands on (`shelf:Tutorials`).
pub const TAGS: [&str; 2] = ["tutorial", "shelf:Tutorials"];

/// **The tutorials, made from the manual's pages** (`docs/src/tutorials/`, as (file name, text)):
/// each page becomes the note it names in its header comment —
///
/// ```text
/// <!-- tutorial
/// id: 01M5JA00000000000000000T02
/// created: 2026-10-18T00:00:04Z
/// -->
/// ```
///
/// — titled by its `# heading`, tagged [`TAGS`], in exactly the bytes the app would write. Links are
/// made to work inside a note: `../images/x.png` → `manual:images/x.png`, `../user/views.md#board`
/// → `manual:user/views.html#board`, another tutorial's page → `note:<its id>`. Answers (the page's
/// file name, the note's text), in the order given.
pub fn notes_from_pages(pages: &[(&str, &str)]) -> Result<Vec<(String, String)>, String> {
    let mut heads = Vec::new();
    for (name, page) in pages {
        let rest = page
            .strip_prefix("<!-- tutorial\n")
            .ok_or_else(|| format!("{name}: must open with a `<!-- tutorial` header"))?;
        let (head, body) =
            rest.split_once("-->\n").ok_or_else(|| format!("{name}: its header never closes"))?;
        let field = |key: &str| {
            head.lines()
                .find_map(|l| l.strip_prefix(key).and_then(|v| v.strip_prefix(": ")))
                .map(str::trim)
                .ok_or_else(|| format!("{name}: its header has no `{key}:`"))
        };
        heads.push((*name, field("id")?.to_string(), field("created")?.to_string(), body));
    }
    let id_of = |page: &str| heads.iter().find(|h| h.0 == page).map(|h| h.1.clone());
    let mut out = Vec::new();
    for (name, id, created, body) in &heads {
        let body = body.trim_start_matches('\n');
        let title = body
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .ok_or_else(|| format!("{name}: has no `# title`"))?
            .trim();
        let body = relink(body, &id_of).map_err(|e| format!("{name}: {e}"))?;
        let tags: String = TAGS.iter().map(|t| format!("- {t}\n")).collect();
        let draft = format!(
            "---\nschema: 1\nid: {id}\ntype: note\ntitle: {}\ncreated: {created}\nupdated: {created}\ntags:\n{tags}---\n{body}",
            serde_json::to_string(title).expect("a string serializes"),
        );
        let obj = fm_core::frontmatter::from_file(&draft).map_err(|e| format!("{name}: {e}"))?;
        out.push((
            name.to_string(),
            fm_core::frontmatter::to_file(&obj).map_err(|e| e.to_string())?,
        ));
    }
    Ok(out)
}

/// A page's links, as a note needs them (see [`notes_from_pages`]). A link to a page that is
/// neither in the manual nor a tutorial is refused: in a note it would lead nowhere.
fn relink(body: &str, id_of: &dyn Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find("](") {
        out.push_str(&rest[..at + 2]);
        rest = &rest[at + 2..];
        let end = rest.find(')').ok_or("a link never closes")?;
        let target = &rest[..end];
        let (path, section) = match target.split_once('#') {
            Some((p, s)) => (p, Some(s)),
            None => (target, None),
        };
        let to = if let Some(inside) = path.strip_prefix("../") {
            let page = inside.strip_suffix(".md").map(|p| format!("{p}.html"));
            let page = page.unwrap_or_else(|| inside.to_string());
            match section {
                Some(s) => format!("manual:{page}#{s}"),
                None => format!("manual:{page}"),
            }
        } else if path.contains(':') || path.is_empty() {
            target.to_string() // a web link, a scheme of ours, or a section of this page
        } else if let Some(id) = id_of(path) {
            format!("note:{id}")
        } else {
            return Err(format!("`{target}` is neither a tutorial nor a page of the manual"));
        };
        out.push_str(&to);
        rest = &rest[end..];
    }
    out.push_str(rest);
    Ok(out)
}

/// **Everyone has the tutorials** (`decisions.md#ui`, 2026-10-20): each tutorial that no open
/// notebook holds, and that no notebook's history shows deleted, is written into the **first**
/// notebook. Once per person, not per notebook: their ids are fixed, and two notebooks holding one
/// id is what `MultiStore` assumes cannot happen. A notebook with no history cannot tell a deletion
/// apart, so it is never written to here. Answers the ones written.
pub fn give(
    store: &mut dyn Store,
    vaults: &[crate::vaults::VaultConfig],
) -> Result<Vec<ObjectMeta>, fm_core::StoreError> {
    let Some(first) = vaults.first() else { return Ok(Vec::new()) };
    let mut missing = Vec::new();
    for (_, text) in TUTORIALS {
        let obj = fm_core::frontmatter::from_file(text)
            .map_err(|e| fm_core::StoreError::Parse(format!("a tutorial does not read: {e}")))?;
        if store.get(obj.id)?.is_none() {
            missing.push(obj);
        }
    }
    if missing.is_empty() || !first.path.join(".git").exists() {
        return Ok(Vec::new());
    }
    // Deleted anywhere, ever: a deletion synced from another device counts as this person's.
    let mut deleted = std::collections::HashSet::new();
    for v in vaults {
        // Twenty years: further back is before 1970 for git's `--since`, which then finds nothing.
        for d in fm_core::vcs::deleted_notes(&v.path, 365 * 20)? {
            deleted.insert(d.id);
        }
    }
    let mut given = Vec::new();
    for mut obj in missing {
        if deleted.contains(&obj.id.to_string()) {
            continue;
        }
        obj.vault = first.name.clone();
        store.put(&obj)?;
        given.push(ObjectMeta::from(&obj));
    }
    Ok(given)
}

/// Into a notebook **just made**: write the tutorials into `<vault>/notes/` when that folder holds
/// no note yet. Anything already there means it is not new, and nothing is written. Returns how
/// many were written.
pub fn write_into(vault: &Path) -> std::io::Result<usize> {
    let notes = vault.join("notes");
    std::fs::create_dir_all(&notes)?;
    let has_a_note = std::fs::read_dir(&notes)?
        .flatten()
        .any(|e| e.path().extension().is_some_and(|x| x == "md"));
    if has_a_note {
        return Ok(0);
    }
    for (name, text) in TUTORIALS {
        std::fs::write(notes.join(name), text)?;
    }
    Ok(TUTORIALS.len())
}

/// **Put the tutorials back**: each one missing from this notebook is written again, as shipped;
/// one that is here — changed or not — is left as it is. Answers the ones put back.
pub fn put_back(
    store: &mut dyn Store,
    vault: &str,
) -> Result<Vec<ObjectMeta>, fm_core::StoreError> {
    let mut back = Vec::new();
    for (_, text) in TUTORIALS {
        let mut obj = fm_core::frontmatter::from_file(text)
            .map_err(|e| fm_core::StoreError::Parse(format!("a tutorial does not read: {e}")))?;
        if store.get(obj.id)?.is_some() {
            continue;
        }
        obj.vault = vault.to_string();
        store.put(&obj)?;
        back.push(ObjectMeta::from(&obj));
    }
    Ok(back)
}
