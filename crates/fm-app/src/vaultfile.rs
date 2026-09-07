//! **A name a person typed, turned into a filename inside the vault.**
//!
//! Saved views and themes are both *files in someone's repository named after a label they chose*,
//! which means both need the same guard: the label reaches this from the UI, so anything that is
//! not a letter, digit or space becomes `-` and the result is bounded — otherwise a name containing
//! `../` chooses where in the filesystem we write. The label itself is preserved verbatim inside
//! the file, so the user still sees what they typed.
//!
//! It lives here rather than being written twice because a duplicated path-traversal guard that
//! drifts is a security bug with two homes, and only one of them gets fixed.

use std::path::{Path, PathBuf};

/// The longest stem we will produce. Bounded so a pathological name cannot approach a filesystem's
/// own limit, where the failure is an unhelpful OS error rather than a sentence.
const MAX_STEM: usize = 60;

/// Fold a user-supplied label into a safe filename stem, or say why it cannot be one.
/// `what` names the thing in the error, so the caller's vocabulary reaches the user ("a view needs
/// a name", "a theme needs a name").
pub fn slug(name: &str, what: &str) -> Result<String, String> {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase();
    let stem: String = stem.chars().take(MAX_STEM).collect();
    if stem.is_empty() {
        return Err(format!("a {what} needs a name"));
    }
    Ok(stem)
}

/// `<vault>/<dir>/<slug(name)>.<ext>` — the one place a label becomes a path.
pub fn path_in(
    vault: &Path,
    dir: &str,
    name: &str,
    ext: &str,
    what: &str,
) -> Result<PathBuf, String> {
    let stem = slug(name, what)?;
    Ok(vault.join(dir).join(format!("{stem}.{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_cannot_choose_where_we_write() {
        let vault = Path::new("/vault");
        // Every traversal attempt collapses to dashes and stays one level down.
        for hostile in ["../../etc/passwd", "..", "/etc/passwd", "a/../../b", "....//"] {
            let p = path_in(vault, "themes", hostile, "css", "theme").unwrap();
            assert_eq!(
                p.parent(),
                Some(vault.join("themes").as_path()),
                "{hostile:?} escaped to {p:?}"
            );
            assert!(!p.to_string_lossy().contains(".."), "{hostile:?} kept a traversal: {p:?}");
        }
    }

    #[test]
    fn a_name_with_nothing_usable_in_it_is_refused_by_its_own_word() {
        assert!(slug("   ", "view").unwrap_err().contains("view"));
        assert!(slug("", "theme").unwrap_err().contains("theme"));
    }

    #[test]
    fn the_stem_is_bounded() {
        assert_eq!(slug(&"a".repeat(500), "theme").unwrap().len(), MAX_STEM);
    }

    #[test]
    fn ordinary_names_survive_recognisably() {
        assert_eq!(slug("Writing Desk", "theme").unwrap(), "writing-desk");
        assert_eq!(slug("  Papers  ", "view").unwrap(), "papers");
    }
}
