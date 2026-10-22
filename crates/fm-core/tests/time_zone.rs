//! A vault's times are kept on the vault's clock and shown on the device's — `decisions.md`
//! 2026-10-23, *every device shows the agenda on its own zone*.
//!
//! The device's clock is whatever this machine is set to (the owner's laptop: Singapore; GitHub's
//! runner: UTC), so these tests give the vault a clock no machine running them is on —
//! `Pacific/Kiritimati`, UTC+14 with no daylight saving — and work the expectation out through the
//! same zone database rather than hard-coding the machine's.

use fm_core::descriptor::Descriptor;
use fm_core::zone::Zone;
use fm_core::{FileStore, Store};
use fm_model::{Kind, Object, Stamp};
use tempfile::tempdir;
use time::macros::{date, time};

const VAULT_ZONE: &str = "Pacific/Kiritimati";

fn device() -> Zone {
    Zone::system().expect("this machine's own zone")
}

fn frontmatter_line(vault: &std::path::Path, id: &str, key: &str) -> String {
    let text = std::fs::read_to_string(vault.join(format!("notes/{id}.md"))).unwrap();
    text.lines().find(|l| l.starts_with(&format!("{key}:"))).unwrap_or_default().to_string()
}

#[test]
fn a_time_is_written_on_the_vaults_clock_and_read_back_on_the_devices() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    assert!(Descriptor::anchor_time_zone(vault, VAULT_ZONE).unwrap());
    let kiritimati = Zone::parse(VAULT_ZONE).unwrap();
    if device() == kiritimati {
        return; // a machine set to UTC+14 has nothing to convert
    }

    let shown = Stamp::at(date!(2026 - 10 - 27), time!(17:00));
    let mut o = Object::new(Kind::Note, "call with Nathan");
    o.start = Some(shown);
    o.due = Some(Stamp::day(date!(2026 - 10 - 30)));
    let id = o.id;
    {
        let mut s = FileStore::open(vault).unwrap();
        s.put(&o).unwrap();
    }

    // On disk: the vault's clock. A day without a time is the same day everywhere.
    let kept = kiritimati.onto(shown, &device());
    assert_eq!(frontmatter_line(vault, &id.to_string(), "start"), format!("start: {kept}"));
    assert_eq!(frontmatter_line(vault, &id.to_string(), "due"), "due: 2026-10-30");

    // Read back — after a reopen, from the files — on this device's clock, unchanged.
    let s = FileStore::open(vault).unwrap();
    let got = s.get(id).unwrap().unwrap();
    assert_eq!(got.start, Some(shown));
    assert_eq!(got.due, Some(Stamp::day(date!(2026 - 10 - 30))));
    // And through a query, which is what the agenda and the board read.
    let rows = s.query(&fm_query::Query::default()).unwrap().rows;
    assert_eq!(rows.iter().find(|r| r.id == id).unwrap().start, Some(shown));
}

#[test]
fn a_vault_with_no_clock_converts_nothing_and_a_stated_clock_is_never_replaced() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    let shown = Stamp::at(date!(2026 - 10 - 27), time!(17:00));
    let mut o = Object::new(Kind::Note, "as before 2026-10-23");
    o.start = Some(shown);
    let id = o.id;
    FileStore::open(vault).unwrap().put(&o).unwrap();
    assert_eq!(frontmatter_line(vault, &id.to_string(), "start"), format!("start: {shown}"));

    assert!(Descriptor::anchor_time_zone(vault, VAULT_ZONE).unwrap(), "anchored once");
    assert!(!Descriptor::anchor_time_zone(vault, "Asia/Singapore").unwrap(), "and never moved");
    assert_eq!(Descriptor::read(vault).unwrap().time_zone.as_deref(), Some(VAULT_ZONE));
}
