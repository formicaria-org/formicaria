//! **Splitting a backlog of attachments into pushes that can survive a phone connection.**
//!
//! The owner's phone had 65 attachments totalling 127.2 MB waiting the first time it could send
//! any of them, and the push died with a broken pipe — twice, identically. The rule that selects
//! attachments is a filter over the blob store, not a diff against the remote, so a device that
//! has never sent one stages the whole backlog at once.
//!
//! Batches are a *rising size limit* rather than an arbitrary partition, which is what makes them
//! work with no bookkeeping: each step re-stages what is already committed (a no-op) and adds only
//! what the raise newly admits, so each push carries the difference alone.

use fm_core::blob::asset_batches;
use std::path::Path;
use tempfile::tempdir;

fn blob(vault: &Path, n: usize, bytes: usize) {
    let hash = format!("{n:064x}");
    let p = vault.join("blobs/sha256").join(&hash[0..2]).join(&hash[2..4]).join(&hash);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, vec![7u8; bytes]).unwrap();
}

fn opt_in(vault: &Path, limit: &str) {
    std::fs::write(vault.join("vault.json"), format!(r#"{{"git_assets_max": "{limit}"}}"#))
        .unwrap();
}

#[test]
fn a_vault_that_has_not_opted_in_has_nothing_to_plan() {
    let d = tempdir().unwrap();
    blob(d.path(), 1, 5000);
    assert!(asset_batches(d.path(), 1000).unwrap().is_empty());
}

#[test]
fn the_backlog_is_split_smallest_first_and_the_caps_rise() {
    let d = tempdir().unwrap();
    opt_in(d.path(), "10MB");
    for (i, size) in [400usize, 300, 200, 100].into_iter().enumerate() {
        blob(d.path(), i + 1, size);
    }
    // Budget 500: 100+200 closes the first step at cap 200; 300 closes the second at 300 (it
    // reaches the budget on its own is false — 300 < 500 — so it continues into 400, which does).
    let plan = asset_batches(d.path(), 500).unwrap();
    assert!(plan.len() >= 2, "a backlog over the budget is more than one step: {plan:?}");

    // **Caps must strictly rise**, or a later step would stage a subset of an earlier one and the
    // loop would never finish.
    for pair in plan.windows(2) {
        assert!(pair[1].cap > pair[0].cap, "caps must rise: {plan:?}");
    }
    // Every attachment is accounted for exactly once, and the last cap admits the largest file.
    assert_eq!(plan.iter().map(|b| b.count).sum::<u32>(), 4);
    assert_eq!(plan.iter().map(|b| b.bytes).sum::<u64>(), 1000);
    assert_eq!(plan.last().unwrap().cap, 400);
}

#[test]
fn one_file_over_the_budget_still_gets_a_step_of_its_own() {
    // Splitting below a single file is impossible, and refusing to plan it would leave that file
    // permanently unsendable — the opposite of the point.
    let d = tempdir().unwrap();
    opt_in(d.path(), "10MB");
    blob(d.path(), 1, 5_000_000);
    let plan = asset_batches(d.path(), 1_000_000).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].count, 1);
    assert_eq!(plan[0].bytes, 5_000_000);
}

#[test]
fn a_file_over_the_vaults_own_limit_is_not_planned_at_all() {
    // It is never going to travel, so a step that stages it could never finish. This is also the
    // half the owner asked about: one oversized file must not impede the rest.
    let d = tempdir().unwrap();
    opt_in(d.path(), "1MB");
    blob(d.path(), 1, 500);
    blob(d.path(), 2, 9_000_000);
    let plan = asset_batches(d.path(), 100_000).unwrap();
    assert_eq!(plan.iter().map(|b| b.count).sum::<u32>(), 1, "only the one under the limit");
    assert_eq!(plan.last().unwrap().cap, 500);
}

#[test]
fn identical_sizes_close_a_step_together_because_a_cap_cannot_separate_them() {
    let d = tempdir().unwrap();
    opt_in(d.path(), "10MB");
    for i in 1..=4 {
        blob(d.path(), i, 300);
    }
    // A budget of 400 would like to stop after the second, but every file is the same size and a
    // cap of 300 admits all four. One step, honestly over budget, rather than a plan that lies.
    let plan = asset_batches(d.path(), 400).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].count, 4);
}
