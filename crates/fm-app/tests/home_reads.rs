//! **Home's two reads** (2026-11-23): the journal (days tagged `journal`, newest first) and each
//! section's `recent` (when its notes of the last month were made, for Home's figures). Invented
//! notes only.

use fm_app::sections::{journal, summary, RECENT_DAYS};
use fm_core::{MemoryStore, Store};
use fm_model::{Kind, Object};
use time::{Duration, OffsetDateTime};

fn put(store: &mut MemoryStore, title: &str, tags: &[&str], age_days: i64) {
    let mut o = Object::new(Kind::Note, format!("{title}\n"));
    o.title = Some(title.to_string());
    o.tags = tags.iter().map(|t| t.to_string()).collect();
    o.created = OffsetDateTime::now_utc() - Duration::days(age_days);
    store.put(&o).unwrap();
}

#[test]
fn the_journal_is_its_days_newest_first_from_a_day_back() {
    let mut s = MemoryStore::new();
    put(&mut s, "2026-11-21", &["journal"], 2);
    put(&mut s, "2026-11-23", &["journal"], 0);
    put(&mut s, "2026-11-22", &["journal"], 1);
    put(&mut s, "2026-11-24", &["journal"], 0); // a day after the one asked from: left out
    put(&mut s, "Reading notes", &["journal"], 0); // tagged, but not a day: left out
    put(&mut s, "2026-11-20", &[], 3); // a day, but not tagged: left out
    let days: Vec<_> =
        journal(&s, "2026-11-23", 10).unwrap().into_iter().map(|n| n.title.unwrap()).collect();
    assert_eq!(days, ["2026-11-23", "2026-11-22", "2026-11-21"]);
    assert_eq!(journal(&s, "2026-11-23", 1).unwrap().len(), 1, "the limit holds");
    assert!(journal(&s, "2026-11-19", 10).unwrap().is_empty(), "nothing before the first day");
}

#[test]
fn a_sections_recent_lists_only_the_last_month_newest_first() {
    let mut s = MemoryStore::new();
    put(&mut s, "Old idea", &[], RECENT_DAYS + 10);
    put(&mut s, "Yesterday's idea", &[], 1);
    put(&mut s, "Today's idea", &[], 0);
    let notes = summary(&s).unwrap().into_iter().find(|x| x.key == "notes").unwrap();
    assert_eq!(notes.count, 3, "the count is every note");
    assert_eq!(notes.recent.len(), 2, "recent is the last {RECENT_DAYS} days only");
    assert!(notes.recent[0] > notes.recent[1], "newest first: {:?}", notes.recent);
}
