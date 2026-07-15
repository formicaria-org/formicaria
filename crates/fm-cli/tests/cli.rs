//! The `fm` binary end-to-end: parse → index → query → stdout. Drives the built
//! binary (`CARGO_BIN_EXE_fm`, provided by cargo — no extra dependency) against a
//! throwaway `--vault`, covering the arg parsing and output formatting the GUI's
//! command functions don't exercise.

use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn fm(vault: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_fm"));
    c.arg("--vault").arg(vault);
    c
}

#[test]
fn capture_list_search_and_set_round_trip() {
    let vault = tempdir().unwrap();

    // Capture a note whose body carries a distinctive marker.
    let out = fm(vault.path()).arg("capture").arg("marker_alpha meta-rl note").output().unwrap();
    assert!(out.status.success(), "capture failed: {}", String::from_utf8_lossy(&out.stderr));

    // It appears in `list` (the header line is "N note(s):", so find the note row).
    let out = fm(vault.path()).arg("list").output().unwrap();
    let list = String::from_utf8_lossy(&out.stdout);
    let note_line = list.lines().find(|l| l.contains("marker_alpha")).expect("captured note listed");
    let id = note_line.split_whitespace().next().expect("an id begins the note line").to_string();

    // Full-text search finds the marker.
    let out = fm(vault.path()).arg("search").arg("marker_alpha").output().unwrap();
    let hits = String::from_utf8_lossy(&out.stdout);
    assert!(hits.contains("marker_alpha"), "search finds the note:\n{hits}");

    // Set a property, then confirm `show` reflects it (round-trips through disk).
    let out = fm(vault.path()).args(["set", &id, "status", "doing"]).output().unwrap();
    assert!(out.status.success(), "set failed: {}", String::from_utf8_lossy(&out.stderr));

    let out = fm(vault.path()).args(["show", &id]).output().unwrap();
    let shown = String::from_utf8_lossy(&out.stdout);
    assert!(shown.contains("doing"), "show reflects the set property:\n{shown}");
}

#[test]
fn verify_exits_zero_on_a_clean_vault() {
    let vault = tempdir().unwrap();
    fm(vault.path()).arg("capture").arg("a clean note").output().unwrap();
    let out = fm(vault.path()).arg("verify").output().unwrap();
    assert!(out.status.success(), "verify should exit 0 on a clean vault");
}
