//! What the assistant's meeting pass is told is already on the agenda (`mail_conversations`).

use fm_app::commands::mail_conversations;
use fm_core::{MemoryStore, Store};
use fm_model::{Kind, Object, PropertyValue, Stamp};

fn meeting(start: &str, extra: &[(&str, &str)]) -> Object {
    let mut o = Object::new(Kind::Note, "");
    o.tags = vec!["meeting".into()];
    o.start = Some(start.parse::<Stamp>().unwrap());
    for (k, v) in extra {
        o.extra.insert((*k).into(), PropertyValue::Text((*v).into()));
    }
    o
}

/// 2026-10-16: Li wrote to move her meeting to Friday 23rd at 16:00, the hour Siyu's conversation
/// already met at — and the move was never proposed, because every conversation's own meeting
/// counted as "already on the agenda" for every other conversation.
#[test]
fn a_conversations_own_meeting_is_not_taken_from_the_others() {
    let mut s = MemoryStore::new();
    let siyu = meeting("2026-10-23T16:00", &[("gmail_thread", "t-siyu")]);
    let proposed = meeting("2026-11-02T10:00", &[("meeting_of", &fm_model::note_ref(siyu.id))]);
    let invite = meeting("2026-10-27T09:00", &[]);
    for o in [&siyu, &proposed, &invite] {
        s.put(o).unwrap();
    }
    let out = mail_conversations(&s).unwrap();
    assert_eq!(out.taken, vec!["2026-10-27T09:00".to_string()], "only what no conversation holds");
    assert_eq!(out.conversations.len(), 1);
    assert_eq!(out.conversations[0].start.as_deref(), Some("2026-10-23T16:00"));
    assert_eq!(out.linked[&siyu.id.to_string()], vec!["2026-11-02T10:00".to_string()]);
}
