//! A vault's times are kept on the vault's clock and shown on the device's — `decisions.md`
//! 2026-10-23, *every device shows the agenda on its own zone* and *the vault's clock is read from
//! its first commit*.
//!
//! The device's clock is whatever this machine is set to (the owner's laptop: Singapore; GitHub's
//! runner: UTC), so these tests give the vault a clock no machine running them is on — UTC+14,
//! `Pacific/Kiritimati`, which has no daylight saving — and work the expectation out through the
//! same zone database rather than hard-coding the machine's.

use fm_core::descriptor::Descriptor;
use fm_core::zone::Zone;
use fm_core::{FileStore, Store};
use fm_model::{Kind, Object, Stamp};
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use time::macros::{date, time};

fn device() -> Zone {
    Zone::system().expect("this machine's own zone")
}

fn frontmatter_line(vault: &Path, id: &str, key: &str) -> String {
    let text = std::fs::read_to_string(vault.join(format!("notes/{id}.md"))).unwrap();
    text.lines().find(|l| l.starts_with(&format!("{key}:"))).unwrap_or_default().to_string()
}

/// Write a timed note through `store`, then check the file holds the vault's clock and a fresh
/// store reads it back on this device's.
fn round_trip(vault: &Path, clock: &Zone, open: impl Fn() -> FileStore) {
    if device() == *clock {
        return; // a machine already on that clock has nothing to convert
    }
    let shown = Stamp::at(date!(2026 - 10 - 27), time!(17:00));
    let mut o = Object::new(Kind::Note, "call with Nathan");
    o.start = Some(shown);
    o.due = Some(Stamp::day(date!(2026 - 10 - 30)));
    let id = o.id;
    open().put(&o).unwrap();

    // On disk: the vault's clock. A day without a time is the same day everywhere.
    let kept = clock.onto(shown, &device());
    assert_eq!(frontmatter_line(vault, &id.to_string(), "start"), format!("start: {kept}"));
    assert_eq!(frontmatter_line(vault, &id.to_string(), "due"), "due: 2026-10-30");

    // Read back — after a reopen, from the files — on this device's clock, unchanged; and through
    // a query, which is what the agenda and the board read.
    let s = open();
    assert_eq!(s.get(id).unwrap().unwrap().start, Some(shown));
    let rows = s.query(&fm_query::Query::default()).unwrap().rows;
    assert_eq!(rows.iter().find(|r| r.id == id).unwrap().start, Some(shown));
}

#[test]
fn a_clock_stated_in_vault_json_is_the_one_times_are_kept_on() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    std::fs::write(vault.join("vault.json"), "{\"notes_time_zone\": \"Pacific/Kiritimati\"}\n")
        .unwrap();
    let clock = Zone::parse("Pacific/Kiritimati").unwrap();
    round_trip(vault, &clock, || FileStore::open(vault).unwrap());
}

#[test]
fn without_one_the_clock_is_the_first_commits_and_a_stated_one_wins() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    let git = |args: &[&str], date: Option<&str>| {
        let mut c = Command::new("git");
        c.current_dir(vault).args(args);
        c.env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@x").env("GIT_COMMITTER_NAME", "t");
        c.env("GIT_COMMITTER_EMAIL", "t@x");
        if let Some(d) = date {
            c.env("GIT_AUTHOR_DATE", d).env("GIT_COMMITTER_DATE", d);
        }
        assert!(c.status().unwrap().success(), "git {args:?}");
    };
    git(&["init", "-q"], None);
    std::fs::write(vault.join("README"), "first").unwrap();
    git(&["add", "README"], None);
    // Begun at UTC+14; a later commit from elsewhere does not move it.
    git(&["commit", "-q", "-m", "first"], Some("2026-07-15T11:03:32+14:00"));
    std::fs::write(vault.join("README"), "second").unwrap();
    git(&["commit", "-q", "-am", "second"], Some("2026-10-20T09:00:00+02:00"));

    let seconds = fm_core::vcs::founding_offset(vault).unwrap().expect("a first commit");
    assert_eq!(seconds, 14 * 3600);
    let clock = Zone::fixed(seconds).unwrap();
    assert_eq!(clock.label(), "+14:00");
    round_trip(vault, &clock, || {
        let mut s = FileStore::open(vault).unwrap();
        s.default_clock(clock.clone());
        s
    });

    // A stated clock is never replaced by the first commit's.
    std::fs::write(vault.join("vault.json"), "{\"notes_time_zone\": \"Asia/Tokyo\"}\n").unwrap();
    let mut s = FileStore::open(vault).unwrap();
    s.default_clock(clock);
    assert_eq!(s.clock_label().as_deref(), Some("Asia/Tokyo"));
}

#[test]
fn the_time_zone_0_15_0_stamped_at_start_is_taken_out() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    // Alone in the file (a phone's stamp): the file goes.
    std::fs::write(vault.join("vault.json"), "{\"time_zone\": \"Europe/Paris\"}\n").unwrap();
    assert!(Descriptor::strip_legacy_time_zone(vault).unwrap());
    assert!(!vault.join("vault.json").exists());
    // Beside the person's own settings: only the stamp goes.
    std::fs::write(
        vault.join("vault.json"),
        "{\"git_assets_max\": \"50MB\", \"time_zone\": \"Asia/Singapore\"}\n",
    )
    .unwrap();
    assert!(Descriptor::strip_legacy_time_zone(vault).unwrap());
    let d = Descriptor::read(vault).unwrap();
    assert_eq!(d.git_assets_max, Some(50_000_000));
    assert_eq!(d.notes_time_zone, None);
    assert!(!Descriptor::strip_legacy_time_zone(vault).unwrap(), "nothing left to take out");
}
