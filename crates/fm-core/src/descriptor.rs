//! `<vault>/vault.json` — the vault's own description of itself.
//!
//! **Bounded by one rule: every field must be a fact git cannot supply.** That rule is what
//! keeps this from becoming the config file that eats the project. Git already knows the
//! authorship and the history (`git log`), the audience (`git remote`, plus who can clone),
//! and — neatly — `.gitignore` is *already* a truth-versus-cache declaration. So there is no
//! `author`, no `collaborators`, no `remote`, no `created`. Adding one later means answering
//! why git's answer was wrong, which is a bar worth having to clear.
//!
//! Three facts survive it:
//!
//! - **`name`** — the audience label. A vault list entry can already carry one, but a repo
//!   you clone has nobody to type it, and the directory name is whatever git chose.
//! - **`description`** — what this vault is for, in the vault, so it travels.
//! - **`notes`** — where the notes are, relative to the vault root. The reason this exists at
//!   all: adopting a repo you already own means the notes are in `docs/` or `notes/` or
//!   nowhere in particular, and demanding they move is demanding you restructure your project
//!   to suit a notebook.
//!
//! **Every field is optional, and an absent file is exactly today's behaviour** — that is the
//! property that makes this safe to add to a format people already have on disk.
//!
//! **Rejected:** a descriptor declaring *two* locations (read from here, write to there).
//! It reads as flexibility and behaves as a trapdoor: the first edit silently relocates a
//! note, and "where is my file" is the one question a files-as-truth notebook must never
//! make hard to answer.

use crate::StoreError;
use std::path::{Path, PathBuf};

/// What `vault.json` can say. Absent file, absent field and empty string are all "no
/// opinion" — the caller keeps its default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Descriptor {
    pub name: Option<String>,
    pub description: Option<String>,
    /// Where the notes live, relative to the vault root. `None` means `notes/`.
    pub notes: Option<PathBuf>,
}

impl Descriptor {
    /// Read `<root>/vault.json`. **An absent file is `Ok(default)`, never an error** — the
    /// overwhelming majority of vaults will not have one, and a notebook that refused to
    /// open a folder because it lacked a config file would have missed the point entirely.
    ///
    /// A file that exists but does not parse *is* an error, and deliberately so: it was put
    /// there on purpose, so silently ignoring it would apply settings the user believes are
    /// in force. Same stance as `vaults::load`'s malformed-JSON warning.
    pub fn read(root: &Path) -> Result<Self, StoreError> {
        let path = root.join("vault.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(StoreError::Io(format!("{}: {e}", path.display()))),
        };
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| StoreError::Parse(format!("{} is not valid JSON: {e}", path.display())))?;

        let field = |k: &str| {
            v.get(k)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
        };

        let notes = match field("notes") {
            None => None,
            Some(rel) => {
                // **The one thing worth refusing.** `notes` is joined onto the vault root, so
                // an absolute path or a `..` would put the notes directory outside the vault
                // — which means outside the repo that defines the audience, and audience is
                // decided by *location* in this design. A vault that could point its notes
                // into another vault's tree would make "which audience is this note in"
                // unanswerable.
                let p = PathBuf::from(&rel);
                if p.is_absolute() || p.components().any(|c| c.as_os_str() == "..") {
                    return Err(StoreError::Parse(format!(
                        "{}: `notes` must stay inside the vault — {rel:?} does not",
                        path.display()
                    )));
                }
                Some(p)
            }
        };

        Ok(Descriptor { name: field("name"), description: field("description"), notes })
    }

    /// Where this vault's notes live, given its root. The single question `FileStore` asks.
    pub fn notes_dir(&self, root: &Path) -> PathBuf {
        root.join(self.notes.clone().unwrap_or_else(|| PathBuf::from("notes")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn with(json: &str) -> (tempfile::TempDir, Descriptor) {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), json).unwrap();
        let desc = Descriptor::read(d.path()).unwrap();
        (d, desc)
    }

    /// The common case by a wide margin, and it must be silent.
    #[test]
    fn an_absent_descriptor_is_todays_behaviour() {
        let d = tempdir().unwrap();
        let desc = Descriptor::read(d.path()).unwrap();

        assert_eq!(desc, Descriptor::default());
        assert_eq!(desc.notes_dir(d.path()), d.path().join("notes"));
    }

    #[test]
    fn it_reads_the_three_facts_git_cannot_supply() {
        let (d, desc) = with(r#"{"name":"lab","description":"Ravi's group","notes":"docs"}"#);

        assert_eq!(desc.name.as_deref(), Some("lab"));
        assert_eq!(desc.description.as_deref(), Some("Ravi's group"));
        assert_eq!(desc.notes_dir(d.path()), d.path().join("docs"));
    }

    /// Absent, null and empty all mean "no opinion" — a half-filled file must not turn into
    /// a vault named "".
    #[test]
    fn empty_and_missing_fields_are_both_no_opinion() {
        let (d, desc) = with(r#"{"name":"","description":null}"#);

        assert_eq!(desc.name, None);
        assert_eq!(desc.description, None);
        assert_eq!(desc.notes_dir(d.path()), d.path().join("notes"), "still the default");
    }

    /// Unknown keys are the user's, or a newer version's. Neither is ours to reject.
    #[test]
    fn an_unknown_key_is_not_an_error() {
        let (_d, desc) = with(r#"{"name":"lab","somethingElse":{"nested":true}}"#);
        assert_eq!(desc.name.as_deref(), Some("lab"));
    }

    /// Audience is decided by *location*, so notes escaping the vault would make "who can
    /// see this note" unanswerable.
    #[test]
    fn notes_may_not_escape_the_vault() {
        for bad in [r#"{"notes":"/etc"}"#, r#"{"notes":"../elsewhere"}"#, r#"{"notes":"a/../../b"}"#] {
            let d = tempdir().unwrap();
            std::fs::write(d.path().join("vault.json"), bad).unwrap();
            assert!(Descriptor::read(d.path()).is_err(), "must refuse: {bad}");
        }
    }

    /// It was put there on purpose, so applying "defaults" the user thinks are overridden is
    /// worse than saying the file is broken.
    #[test]
    fn a_malformed_descriptor_is_an_error_not_a_shrug() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), "{ not json").unwrap();
        assert!(Descriptor::read(d.path()).is_err());
    }
}
