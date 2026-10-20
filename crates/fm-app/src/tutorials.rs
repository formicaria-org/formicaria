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
