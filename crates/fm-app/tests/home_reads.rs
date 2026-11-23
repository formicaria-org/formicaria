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
    let notes =
        summary(&s, &[], RECENT_DAYS).unwrap().into_iter().find(|x| x.key == "notes").unwrap();
    assert_eq!(notes.count, 3, "the count is every note");
    assert_eq!(notes.recent.len(), 2, "recent is the last {RECENT_DAYS} days only");
    // A heatmap asks for a year: the older note is in it.
    let year = summary(&s, &[], 371).unwrap().into_iter().find(|x| x.key == "notes").unwrap();
    assert_eq!(year.recent.len(), 3);
    assert!(notes.recent[0] > notes.recent[1], "newest first: {:?}", notes.recent);
}

#[test]
fn a_hidden_vault_is_left_out_of_the_counts() {
    let mut s = MemoryStore::new();
    for (title, vault) in [("Lab idea", "lab"), ("Home idea", "home"), ("Lab plan", "lab")] {
        let mut o = Object::new(Kind::Note, format!("{title}\n"));
        o.title = Some(title.to_string());
        o.vault = vault.to_string();
        s.put(&o).unwrap();
    }
    let count = |hidden: &[String]| {
        summary(&s, hidden, RECENT_DAYS)
            .unwrap()
            .into_iter()
            .find(|x| x.key == "notes")
            .unwrap()
            .count
    };
    assert_eq!(count(&[]), 3);
    assert_eq!(count(&["lab".to_string()]), 1);
    assert_eq!(count(&["home".to_string()]), 2);
}

/// **A plan is not a task, and a meeting is not one either** (2026-11-23): a training plan's `start`
/// and `due` are the days it was in force, and a meeting's `due` is when it ends. Neither belongs
/// in the agenda's deadlines (a plan) or in Tasks (both); a real task does.
#[test]
fn plans_and_meetings_are_not_tasks() {
    use fm_app::commands::{agenda, set_property};
    let mut s = MemoryStore::new();
    let mut add = |title: &str, props: &[(&str, &str)]| {
        let mut o = Object::new(Kind::Note, format!("{title}\n"));
        o.title = Some(title.to_string());
        s.put(&o).unwrap();
        for (k, v) in props {
            set_property(&mut s, &o.id.to_string(), k, v).unwrap();
        }
    };
    add(
        "Strength, invented plan",
        &[
            ("program", "training"),
            ("plan", "strength-x"),
            ("start", "2026-10-01"),
            ("due", "2026-10-11"),
        ],
    );
    add("Lab meeting", &[("start", "2026-10-05T10:00"), ("due", "2026-10-05T11:00")]);
    add("Send the draft", &[("due", "2026-10-20")]);
    add("Order reagents", &[("status", "todo")]);
    let in_agenda: Vec<_> = agenda(&s).unwrap().into_iter().filter_map(|n| n.title).collect();
    assert!(
        !in_agenda.iter().any(|t| t.contains("plan")),
        "a plan is not a deadline: {in_agenda:?}"
    );
    assert!(in_agenda.iter().any(|t| t == "Lab meeting"), "a meeting stays on the calendar");
    assert!(in_agenda.iter().any(|t| t == "Send the draft"));
    let tasks = fm_app::sections::notes(&s, "tasks", 50).unwrap();
    let mut titles: Vec<_> = tasks.into_iter().filter_map(|n| n.title).collect();
    titles.sort();
    assert_eq!(titles, ["Order reagents", "Send the draft"]);
}
