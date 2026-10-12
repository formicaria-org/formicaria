//! **Reminders** — `decisions.md#ui`, 2026-10-13. The moments worked out from the notes' own
//! dates: at the time, at nine for a day with no time, earlier or never when the note says so.

use fm_app::log::{create_notes, NoteDraft};
use fm_app::reminders::reminders;
use fm_core::MemoryStore;
use serde_json::json;

fn note(title: &str, props: serde_json::Value) -> NoteDraft {
    serde_json::from_value(json!({ "title": title, "props": props })).unwrap()
}

#[test]
fn a_reminder_is_the_notes_own_date_said_at_its_time() {
    let mut s = MemoryStore::new();
    create_notes(
        &mut s,
        "",
        &[
            note("Dentist", json!({ "start": "2026-10-14T15:30" })),
            note("Tax return", json!({ "due": "2026-10-15" })),
            note("Call the bank", json!({ "due": "2026-10-14T10:00", "remind": 30 })),
            note("Quiet one", json!({ "due": "2026-10-14T11:00", "remind": "off" })),
            note("Already done", json!({ "due": "2026-10-14T12:00", "status": "done" })),
            note("Next year", json!({ "due": "2027-03-01" })),
            // A logged meal has a day and is nothing to be reminded of.
            note("Oats", json!({ "start": "2026-10-14T07:30", "log": "food", "kcal": 228 })),
        ],
    )
    .unwrap();
    let got = reminders(&s, "2026-10-13", 14).unwrap();
    let seen: Vec<(&str, &str, &str, &str)> = got
        .iter()
        .map(|r| (r.title.as_str(), r.at.as_str(), r.what.as_str(), r.when.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("Call the bank", "2026-10-14T09:30", "due", "10:00"),
            ("Dentist", "2026-10-14T15:30", "start", ""),
            // A day with no time: nine that morning.
            ("Tax return", "2026-10-15T09:00", "due", ""),
        ]
    );
    // From a later day the earlier ones are past, and a day that is no day is refused.
    assert_eq!(reminders(&s, "2026-10-15", 14).unwrap().len(), 1);
    assert!(reminders(&s, "soon", 14).is_err());
}
