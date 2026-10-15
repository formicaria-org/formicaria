//! Moving a vault's folder somewhere else — the file half of `move_vault`.
//!
//! **The guarantee this module exists for** (`decisions.md#vault`, 2026-10-15): when
//! [`move_tree`] returns an error, the source is exactly as it was and the destination does not
//! exist. When it returns `Ok`, the destination holds every file the source held, byte for byte.
//!
//! - **One disk: a single `rename`.** The folder is at one place or the other, never half.
//! - **Another disk** (`rename` refuses with `EXDEV`): every file is copied, then **re-read and
//!   compared by SHA-256**. Any failure — a file that will not copy, a byte that does not match —
//!   removes the partial copy. The source is never touched on this path; the caller removes it
//!   with [`remove_source`] only once it has switched to the copy.
//!
//! Nothing here knows what a vault is: it moves a directory tree. Which folder, and what the app
//! does around it (taking the vault out of the live set first, saving the new path), is
//! `fm-app`'s.

use crate::StoreError;
use std::fs;
use std::path::{Path, PathBuf};

/// What a move did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    /// `true` when the files were copied (another disk); the source still exists then, and is the
    /// caller's to remove. `false` for a rename, after which the source is gone.
    pub copied: bool,
    pub files: u64,
    pub bytes: u64,
}

/// How to move. `Copy` takes the cross-disk path even on one disk — what a test needs to reach it,
/// and a fallback should a rename be refused for a reason that a copy can get round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Copy,
}

fn io(what: &str, p: &Path, e: std::io::Error) -> StoreError {
    StoreError::Io(format!("{what} {}: {e}", p.display()))
}

/// Move the tree at `src` to `dest`. `dest` must not exist, or be an empty directory.
pub fn move_tree(src: &Path, dest: &Path, mode: Mode) -> Result<Moved, StoreError> {
    if !src.is_dir() {
        return Err(StoreError::Io(format!("{} is not a folder", src.display())));
    }
    if dest.exists() {
        let empty = fs::read_dir(dest).map(|mut d| d.next().is_none()).unwrap_or(false);
        if !empty {
            return Err(StoreError::Io(format!(
                "{} already has files in it — choose an empty or a new folder",
                dest.display()
            )));
        }
        fs::remove_dir(dest).map_err(|e| io("could not use", dest, e))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| io("could not create", parent, e))?;
    }
    let (files, bytes) = count(src)?;

    if mode == Mode::Auto {
        match fs::rename(src, dest) {
            Ok(()) => return Ok(Moved { copied: false, files, bytes }),
            // Another disk: copy instead. Anything else is a real refusal, and a failed rename
            // changed nothing, so it can simply be reported.
            Err(e) if crosses_devices(&e) => {}
            Err(e) => return Err(io("could not move to", dest, e)),
        }
    }

    let copied = copy_tree(src, dest).and_then(|()| verify(src, dest));
    if let Err(e) = copied {
        // The partial copy goes; the source was never touched.
        let _ = fs::remove_dir_all(dest);
        return Err(e);
    }
    Ok(Moved { copied: true, files, bytes })
}

/// Remove the source after a verified copy — the last step, taken only once the caller uses the
/// copy. Separate from [`move_tree`] precisely so it cannot happen before that.
pub fn remove_source(src: &Path) -> Result<(), StoreError> {
    fs::remove_dir_all(src).map_err(|e| io("could not remove", src, e))
}

fn crosses_devices(e: &std::io::Error) -> bool {
    // `EXDEV` (18) on Linux and macOS; `ERROR_NOT_SAME_DEVICE` (17) on Windows.
    if cfg!(windows) {
        e.raw_os_error() == Some(17)
    } else {
        e.raw_os_error() == Some(18)
    }
}

/// Every entry under `root`, relative to it, directories before their contents.
fn walk(root: &Path) -> Result<Vec<PathBuf>, StoreError> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let dir = root.join(&rel);
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .map_err(|e| io("could not read", &dir, e))?
            .collect::<Result<_, _>>()
            .map_err(|e| io("could not read", &dir, e))?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let r = rel.join(e.file_name());
            let ft = e.file_type().map_err(|err| io("could not read", &e.path(), err))?;
            if ft.is_dir() {
                stack.push(r.clone());
            }
            out.push(r);
        }
    }
    Ok(out)
}

/// How many files a folder holds and how many bytes they take — what a move will carry, stated
/// before it starts.
pub fn size(root: &Path) -> Result<(u64, u64), StoreError> {
    count(root)
}

fn count(root: &Path) -> Result<(u64, u64), StoreError> {
    let (mut files, mut bytes) = (0, 0);
    for rel in walk(root)? {
        let m = fs::symlink_metadata(root.join(&rel)).map_err(|e| io("could not read", &rel, e))?;
        if m.is_file() {
            files += 1;
            bytes += m.len();
        }
    }
    Ok((files, bytes))
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), StoreError> {
    fs::create_dir(dest).map_err(|e| io("could not create", dest, e))?;
    let mut entries = walk(src)?;
    // Parents first: a sorted relative path list puts every directory before what is inside it.
    entries.sort();
    for rel in entries {
        let (from, to) = (src.join(&rel), dest.join(&rel));
        let m = fs::symlink_metadata(&from).map_err(|e| io("could not read", &from, e))?;
        if m.is_dir() {
            fs::create_dir_all(&to).map_err(|e| io("could not create", &to, e))?;
        } else if m.file_type().is_symlink() {
            let target = fs::read_link(&from).map_err(|e| io("could not read", &from, e))?;
            link(&target, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| io("could not copy", &from, e))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) -> Result<(), StoreError> {
    std::os::unix::fs::symlink(target, at).map_err(|e| io("could not copy the link", at, e))
}
#[cfg(not(unix))]
fn link(_target: &Path, at: &Path) -> Result<(), StoreError> {
    Err(StoreError::Io(format!("{} is a link, which cannot be copied here", at.display())))
}

/// Every file in `src` exists in `dest` with the same bytes, every link with the same target.
fn verify(src: &Path, dest: &Path) -> Result<(), StoreError> {
    for rel in walk(src)? {
        let (a, b) = (src.join(&rel), dest.join(&rel));
        let m = fs::symlink_metadata(&a).map_err(|e| io("could not read", &a, e))?;
        let same = if m.is_dir() {
            b.is_dir()
        } else if m.file_type().is_symlink() {
            fs::read_link(&a).ok() == fs::read_link(&b).ok()
        } else {
            fs::symlink_metadata(&b).map(|mb| mb.len() == m.len()).unwrap_or(false)
                && crate::blob::sha256_file(&a)? == crate::blob::sha256_file(&b)?
        };
        if !same {
            return Err(StoreError::Io(format!(
                "the copy of {} did not match the original, so nothing was moved",
                rel.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(root: &Path) {
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::create_dir_all(root.join(".git/objects/ab")).unwrap();
        fs::create_dir_all(root.join("empty-dir")).unwrap();
        fs::write(root.join("notes/01A.md"), "---\nid: 01A\n---\nhello").unwrap();
        fs::write(root.join(".git/objects/ab/cdef"), [0u8, 1, 2, 3, 255]).unwrap();
        fs::write(root.join("vault.json"), "{}").unwrap();
    }

    fn snapshot(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        walk(root)
            .unwrap()
            .into_iter()
            .map(|r| {
                let p = root.join(&r);
                let bytes = p.is_file().then(|| fs::read(&p).unwrap());
                (r, bytes)
            })
            .collect::<std::collections::BTreeMap<_, _>>()
            .into_iter()
            .collect()
    }

    #[test]
    fn on_one_disk_a_vault_is_renamed_whole() {
        let t = tempfile::tempdir().unwrap();
        let (src, dest) = (t.path().join("vault"), t.path().join("elsewhere/vault"));
        vault(&src);
        let before = snapshot(&src);
        let m = move_tree(&src, &dest, Mode::Auto).unwrap();
        assert!(!m.copied);
        assert_eq!(m.files, 3);
        assert!(!src.exists(), "a rename leaves nothing behind");
        assert_eq!(snapshot(&dest), before, "every file and folder, the empty one included");
    }

    #[test]
    fn across_disks_every_file_is_copied_verified_and_the_source_kept_for_the_caller() {
        let t = tempfile::tempdir().unwrap();
        let (src, dest) = (t.path().join("vault"), t.path().join("other/vault"));
        vault(&src);
        let before = snapshot(&src);
        let m = move_tree(&src, &dest, Mode::Copy).unwrap();
        assert!(m.copied);
        assert_eq!(snapshot(&dest), before);
        assert_eq!(snapshot(&src), before, "the source is untouched until the caller removes it");
        remove_source(&src).unwrap();
        assert!(!src.exists());
    }

    /// **A copy that fails halfway changes nothing.** A file that cannot be read stops the copy;
    /// the partial destination is removed and the source is exactly as it was.
    #[cfg(unix)]
    #[test]
    fn a_copy_that_fails_halfway_leaves_the_source_alone_and_no_destination() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let (src, dest) = (t.path().join("vault"), t.path().join("other/vault"));
        vault(&src);
        let locked = src.join("notes/01A.md");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        // Running as root reads anything, and then there is no failure to provoke.
        if fs::read(&locked).is_ok() {
            return;
        }
        let err = move_tree(&src, &dest, Mode::Copy).unwrap_err();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(err.to_string().contains("01A.md"), "{err}");
        assert!(!dest.exists(), "no half-copied vault is left behind");
        assert_eq!(fs::read_to_string(&locked).unwrap(), "---\nid: 01A\n---\nhello");
    }

    #[test]
    fn a_folder_with_files_in_it_is_never_a_destination() {
        let t = tempfile::tempdir().unwrap();
        let (src, dest) = (t.path().join("vault"), t.path().join("busy"));
        vault(&src);
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("mine.txt"), "someone else's").unwrap();
        assert!(move_tree(&src, &dest, Mode::Auto).is_err());
        assert!(src.join("notes/01A.md").exists());
        assert_eq!(fs::read_to_string(dest.join("mine.txt")).unwrap(), "someone else's");
    }

    #[test]
    fn an_empty_folder_is_a_destination() {
        let t = tempfile::tempdir().unwrap();
        let (src, dest) = (t.path().join("vault"), t.path().join("ready"));
        vault(&src);
        fs::create_dir_all(&dest).unwrap();
        move_tree(&src, &dest, Mode::Auto).unwrap();
        assert!(dest.join("notes/01A.md").exists());
    }
}
