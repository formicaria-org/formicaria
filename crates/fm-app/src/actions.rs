//! **What formicaria can do, declared once** (`decisions.md` 2026-11-17, *one server, many views*;
//! the owner's frame: a person and an agent sharing control of one system). Every action the person
//! can take — and, when they allow it, the assistant — is one entry here: what it takes, what it
//! makes, and where it runs. The UI draws from this table and the assistant will get its tools from
//! it, so neither has an ability the other cannot see. The assistant's use of each is **off** until
//! the person turns it on, action by action (the autonomy dial, wired last).
//!
//! The first action that runs on the server is [`put`]: **the one way content moves into a note**,
//! whether or not the note is open anywhere.
use fm_core::{Store, StoreError};
use serde::Serialize;

/// Where an action runs.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Runs {
    /// A command of this server (`dispatch`).
    Server,
    /// An address the screen opens with the content in it; the person finishes there (mail stays
    /// read-only: `handoff.ts`).
    HandOff,
    /// A place the screen opens: a feature, a note, a web page.
    Open,
}

/// One action.
#[derive(Serialize, Clone, Copy, Debug)]
pub struct ActionSpec {
    pub key: &'static str,
    pub label: &'static str,
    /// What it takes, in words.
    pub takes: &'static str,
    /// What it makes.
    pub makes: &'static str,
    pub runs: Runs,
    /// The assistant's starting permission: always `off`; the person turns each on.
    pub agent: &'static str,
}

const fn a(
    key: &'static str,
    label: &'static str,
    takes: &'static str,
    makes: &'static str,
    runs: Runs,
) -> ActionSpec {
    ActionSpec { key, label, takes, makes, runs, agent: "off" }
}

/// Every action, in the order a menu shows them.
pub const ACTIONS: &[ActionSpec] = &[
    a(
        "put",
        "Bring into a note",
        "a piece (text with its source)",
        "the note, with the piece in it",
        Runs::Server,
    ),
    a("new_note", "New note", "a title, and a piece to start it", "a new note", Runs::Server),
    a("place", "Place on a shelf", "a note and a shelf", "the note on that shelf", Runs::Server),
    a(
        "send_mail",
        "Send by mail",
        "a subject and text",
        "a mail ready to send, in Gmail",
        Runs::HandOff,
    ),
    a(
        "add_event",
        "Add to calendar",
        "a title, a time, text",
        "an event ready to save",
        Runs::HandOff,
    ),
    a("share", "Share", "text", "Android's share sheet, or a copy", Runs::HandOff),
    a("web_search", "Search the web", "words", "a web search in a tab", Runs::Open),
    a("open", "Open", "a feature, a note or a web address", "it, in a tab", Runs::Open),
];

/// Where a piece goes inside a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum At {
    /// At its end.
    End,
    /// At the end of the section under this heading (its text, without the `#`s); at the note's end
    /// when there is no such heading.
    AfterHeading(String),
}

impl At {
    /// `"end"` (or nothing) and `"after:<heading>"`.
    pub fn parse(s: &str) -> At {
        match s.strip_prefix("after:") {
            Some(h) if !h.trim().is_empty() => At::AfterHeading(h.trim().to_string()),
            _ => At::End,
        }
    }
}

/// A Markdown heading's level and text, if `line` is one.
fn heading(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    let level = t.chars().take_while(|c| *c == '#').count();
    (1..=6)
        .contains(&level)
        .then(|| t[level..].trim())
        .filter(|_| t[level..].starts_with(' ') || t[level..].is_empty())
        .map(|text| (level, text))
}

/// `body` with `md` put `at`, separated by one blank line.
pub fn insert(body: &str, md: &str, at: &At) -> String {
    let md = md.trim();
    if body.trim().is_empty() {
        return format!("{md}\n");
    }
    let end = |b: &str| format!("{}\n\n{md}\n", b.trim_end());
    let At::AfterHeading(want) = at else { return end(body) };
    let lines: Vec<&str> = body.lines().collect();
    let Some((i, level)) = lines.iter().enumerate().find_map(|(i, l)| {
        heading(l).filter(|(_, t)| t.eq_ignore_ascii_case(want)).map(|(lv, _)| (i, lv))
    }) else {
        return end(body);
    };
    // The section runs to the next heading of the same or a higher level.
    let stop = lines[i + 1..]
        .iter()
        .position(|l| heading(l).is_some_and(|(lv, _)| lv <= level))
        .map(|p| i + 1 + p)
        .unwrap_or(lines.len());
    let before = lines[..stop].join("\n");
    let after = lines[stop..].join("\n");
    if after.is_empty() {
        end(&before)
    } else {
        format!("{}\n\n{md}\n\n{after}\n", before.trim_end())
    }
}

/// **Put a piece into a note** — under the note's version guard: `base`, when given, must be the
/// version the caller read, or nothing is written (`StoreError::Conflict`). Answers the new version.
pub fn put(
    store: &mut dyn Store,
    id: &str,
    md: &str,
    at: &At,
    base: &str,
) -> Result<String, StoreError> {
    let parsed: fm_model::Id =
        id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let obj = store.get(parsed)?.ok_or(StoreError::NotFound(parsed))?;
    if !base.is_empty() && crate::dto::version_of(&obj.body) != base {
        return Err(StoreError::Conflict(parsed));
    }
    let body = insert(&obj.body, md, at);
    crate::commands::update_body(store, id, &body, "")
}

/// **Swap what an earlier put wrote** (its first occurrence) for `to` — empty for Undo. Nothing is
/// written when it is no longer there (edited since), and the version guard holds as for [`put`].
pub fn replace(
    store: &mut dyn Store,
    id: &str,
    from: &str,
    to: &str,
    base: &str,
) -> Result<String, StoreError> {
    let parsed: fm_model::Id =
        id.parse().map_err(|_| StoreError::Parse(format!("invalid id: {id}")))?;
    let obj = store.get(parsed)?.ok_or(StoreError::NotFound(parsed))?;
    if !base.is_empty() && crate::dto::version_of(&obj.body) != base {
        return Err(StoreError::Conflict(parsed));
    }
    let Some(at) = obj.body.find(from) else { return Ok(crate::dto::version_of(&obj.body)) };
    let mut body = String::with_capacity(obj.body.len());
    body.push_str(&obj.body[..at]);
    body.push_str(to);
    body.push_str(&obj.body[at + from.len()..]);
    // Undo leaves no gap where the piece was.
    let body = if to.is_empty() {
        let mut b = body;
        while b.contains("\n\n\n") {
            b = b.replace("\n\n\n", "\n\n");
        }
        b.trim_end().to_string() + "\n"
    } else {
        body
    };
    crate::commands::update_body(store, id, &body, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_goes_at_the_end_after_one_blank_line() {
        assert_eq!(insert("", "> A passage", &At::End), "> A passage\n");
        assert_eq!(insert("Notes.\n\n\n", "> A passage", &At::End), "Notes.\n\n> A passage\n");
    }

    #[test]
    fn a_piece_goes_at_the_end_of_its_heading_section() {
        let body = "# Ants\n\nIntro.\n\n## Reading\n\nOne.\n\n## Questions\n\nWhy?\n";
        let got = insert(body, "> Leaves.", &At::parse("after:reading"));
        assert_eq!(
            got,
            "# Ants\n\nIntro.\n\n## Reading\n\nOne.\n\n> Leaves.\n\n## Questions\n\nWhy?\n"
        );
        // The last section: at the end.
        let got = insert(body, "> More.", &At::parse("after:Questions"));
        assert!(got.ends_with("## Questions\n\nWhy?\n\n> More.\n"), "{got}");
        // No such heading: at the end, never lost.
        let got = insert(body, "> Kept.", &At::parse("after:Nowhere"));
        assert!(got.ends_with("Why?\n\n> Kept.\n"), "{got}");
    }

    #[test]
    fn a_line_that_only_starts_with_a_hash_is_not_a_heading() {
        assert_eq!(heading("#tag at the start"), None);
        assert_eq!(heading("## Reading"), Some((2, "Reading")));
    }

    #[test]
    fn every_action_starts_off_for_the_assistant() {
        assert!(ACTIONS.iter().all(|a| a.agent == "off"));
        let keys: std::collections::HashSet<_> = ACTIONS.iter().map(|a| a.key).collect();
        assert_eq!(keys.len(), ACTIONS.len(), "each action once");
    }
}
