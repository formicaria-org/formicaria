//! **A theme is a file in the vault, so it travels with the notes.**
//!
//! `MASTERPLAN.md` has listed `themes/*.css # CSS themes — git-tracked` in the vault layout since
//! the beginning, and names CSS themes as one of three sanctioned extensibility layers —
//! *"extensibility comes from CSS themes and declarative `.view` files — no code execution"*. This
//! is that layer, built; it amends nothing.
//!
//! Deliberately a **sibling of [`crate::views`]**, not a variation on it: both are "a file in a
//! vault subdirectory, named after a label a person typed", both are git-tracked so they arrive on
//! the other machine, and both must never let that label choose where in the filesystem we write —
//! which is why the path guard lives in [`crate::vaultfile`] and is shared rather than copied.
//!
//! **A theme is not a note**, and that is a decision rather than an accident. A stylesheet in
//! `notes/` would be indexed, full-text searchable and permanent noise on every board and timeline;
//! it would contradict the vault layout collaborators reason about; and note bodies are the app's
//! *designated untrusted input* — they arrive from other people through the `.md` merge driver, and
//! the whole sanitiser apparatus exists for them. Making note content into applied stylesheet text
//! would move a trust boundary for nothing.
//!
//! **The filename is the name.** There is no header format inside the file to carry a prettier
//! label, on purpose: a theme is picked from a short list where `writing-desk` reads perfectly
//! well, and inventing a mini-format for a CSS file is the kind of surface that becomes permanent.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// The directory, relative to the vault root, named by `MASTERPLAN.md`'s layout.
const DIR: &str = "themes";

/// **A ceiling, because this text is inlined into the document on every load and committed to git
/// forever.** The entire built-in design system — every token, both themes, all three layers — is
/// about 14 KB, so this is roughly nine times the whole thing: generous for a theme, and small
/// enough that a paste accident says so instead of bloating the repository.
pub const MAX_BYTES: u64 = 128 * 1024;

/// One theme file, as listed. `error` is set for a file we can see but cannot serve — a broken
/// theme names itself and never silently vanishes, the same discipline `views::list_views` applies.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ThemeInfo {
    pub name: String,
    /// Which vault holds it. `list_themes` walks one vault and has no name to report, so it leaves
    /// this empty and `dispatch` stamps it while iterating the configs — the same split, and for
    /// the same reason, as `views::ViewInfo::vault`: without it "delete" resolves against the
    /// default vault, where a theme belonging to another one is simply not found.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub vault: String,
    pub bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn theme_path(vault: &Path, name: &str) -> Result<PathBuf, String> {
    crate::vaultfile::path_in(vault, DIR, name, "css", "theme")
}

/// Every theme in this vault, by filename. No `themes/` directory yet is an **empty list, not an
/// error** — a vault that has never been themed is the ordinary case, not a fault.
pub fn list_themes(vault: &Path) -> Vec<ThemeInfo> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(vault.join(DIR)) else {
        return out;
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("css"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
        // Read it here rather than trusting the extension: a file that is not text cannot be
        // applied, and the person needs to be told that by the list, not by a blank screen.
        let error = match std::fs::read(&path) {
            Err(e) => Some(format!("could not read this theme: {e}")),
            Ok(raw) if std::str::from_utf8(&raw).is_err() => {
                Some("this file is not text, so it cannot be a theme".to_string())
            }
            Ok(_) if bytes > MAX_BYTES => Some(too_big(bytes)),
            Ok(_) => None,
        };
        out.push(ThemeInfo { name, vault: String::new(), bytes, error });
    }
    out
}

fn too_big(bytes: u64) -> String {
    format!("this theme is {} KB, and the limit is {} KB", bytes / 1024, MAX_BYTES / 1024)
}

/// The CSS of one theme, to show in the editor or to apply.
pub fn read_theme(vault: &Path, name: &str) -> Result<String, String> {
    let path = theme_path(vault, name)?;
    let raw = std::fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => format!("there is no theme called '{name}'"),
        _ => format!("could not read {}: {e}", path.display()),
    })?;
    String::from_utf8(raw).map_err(|_| "this file is not text, so it cannot be a theme".to_string())
}

/// Write a theme, returning the path so `dispatch` can record it. Refuses oversize input **before**
/// touching the disk, so a rejected save leaves nothing half-written.
pub fn save_theme(vault: &Path, name: &str, css: &str) -> Result<PathBuf, String> {
    let path = theme_path(vault, name)?;
    let len = css.len() as u64;
    if len > MAX_BYTES {
        return Err(too_big(len));
    }
    let dir = path.parent().ok_or("no themes directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    std::fs::write(&path, css).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(path)
}

/// Remove a theme. Missing is success — the user asked for it to be gone — and the path comes back
/// either way, because a file git still tracks but disk no longer has is exactly the case that has
/// to reach `commit`.
pub fn delete_theme(vault: &Path, name: &str) -> Result<PathBuf, String> {
    let path = theme_path(vault, name)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(e) => Err(format!("could not delete {}: {e}", path.display())),
    }
}

/// Rename a theme, returning `(old, new)` so both reach the write list. Simpler than the view
/// version — there is no label inside a CSS file, the filename *is* the name — but the same rule
/// applies: **move the bytes, never rewrite them**, so whatever the author wrote survives intact.
pub fn rename_theme(vault: &Path, from: &str, to: &str) -> Result<(PathBuf, PathBuf), String> {
    let src = theme_path(vault, from)?;
    let dst = theme_path(vault, to)?;
    if !src.exists() {
        return Err(format!("there is no theme called '{from}'"));
    }
    if src == dst {
        return Ok((src.clone(), src));
    }
    if dst.exists() {
        return Err(format!(
            "there is already a theme called '{}' — pick another name",
            dst.file_stem().unwrap_or_default().to_string_lossy()
        ));
    }
    std::fs::rename(&src, &dst).map_err(|e| format!("could not rename {}: {e}", src.display()))?;
    Ok((src, dst))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn a_theme_round_trips() {
        let d = tempdir().unwrap();
        save_theme(d.path(), "Writing Desk", ":root { --bg: #f4f1ea; }").unwrap();
        assert_eq!(read_theme(d.path(), "Writing Desk").unwrap(), ":root { --bg: #f4f1ea; }");
        let listed = list_themes(d.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "writing-desk");
        assert!(listed[0].error.is_none());
    }

    #[test]
    fn a_hostile_name_cannot_escape_the_themes_directory() {
        let d = tempdir().unwrap();
        save_theme(d.path(), "../../escape", "/* x */").unwrap();
        // The only file written is inside `themes/`, and nothing landed beside the vault.
        let inside: Vec<_> = std::fs::read_dir(d.path().join(DIR)).unwrap().flatten().collect();
        assert_eq!(inside.len(), 1, "exactly one file, and it is in themes/");
        assert!(!d.path().parent().unwrap().join("escape.css").exists());
    }

    #[test]
    fn an_oversize_theme_is_refused_and_writes_nothing() {
        let d = tempdir().unwrap();
        let huge = "a".repeat((MAX_BYTES + 1) as usize);
        let err = save_theme(d.path(), "Huge", &huge).unwrap_err();
        assert!(err.contains("128"), "the message carries the limit: {err}");
        assert!(!d.path().join(DIR).join("huge.css").exists(), "nothing half-written");
    }

    #[test]
    fn a_vault_that_has_never_been_themed_is_empty_not_broken() {
        let d = tempdir().unwrap();
        assert!(list_themes(d.path()).is_empty());
    }

    #[test]
    fn deleting_a_theme_that_is_not_there_is_success() {
        let d = tempdir().unwrap();
        assert!(delete_theme(d.path(), "never-existed").is_ok());
    }

    #[test]
    fn renaming_moves_the_bytes_and_changes_nothing_in_them() {
        let d = tempdir().unwrap();
        let css = "/* mine */\n:root { --bg: #f4f1ea; }\n";
        save_theme(d.path(), "Writing Desk", css).unwrap();
        rename_theme(d.path(), "Writing Desk", "Reading Room").unwrap();

        assert!(read_theme(d.path(), "Writing Desk").is_err(), "the old name is gone");
        assert_eq!(read_theme(d.path(), "Reading Room").unwrap(), css, "byte for byte");
    }

    #[test]
    fn renaming_onto_a_name_already_taken_is_refused() {
        let d = tempdir().unwrap();
        save_theme(d.path(), "One", "/* one */").unwrap();
        save_theme(d.path(), "Two", "/* two */").unwrap();
        assert!(rename_theme(d.path(), "Two", "One").unwrap_err().contains("already"));
        // Neither is damaged by the refusal.
        assert_eq!(read_theme(d.path(), "One").unwrap(), "/* one */");
        assert_eq!(read_theme(d.path(), "Two").unwrap(), "/* two */");
    }

    #[test]
    fn renaming_something_that_is_not_there_says_so() {
        let d = tempdir().unwrap();
        assert!(rename_theme(d.path(), "ghost", "other").unwrap_err().contains("ghost"));
    }

    #[test]
    fn a_file_that_is_not_text_lists_with_its_reason_and_does_not_vanish() {
        let d = tempdir().unwrap();
        std::fs::create_dir_all(d.path().join(DIR)).unwrap();
        std::fs::write(d.path().join(DIR).join("broken.css"), [0xff, 0xfe, 0x00]).unwrap();
        let listed = list_themes(d.path());
        assert_eq!(listed.len(), 1, "it must still be listed");
        assert!(listed[0].error.is_some(), "and it must say what is wrong");
    }
}
