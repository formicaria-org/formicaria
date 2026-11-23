//! **The library's sections** (`docs/context/unifying-home-research-2026-11-22.md` §11; the shell,
//! `decisions.md` 2026-11-22, evening). A section is what a person navigates (Notes, Tasks, Food,
//! Music…): **a rule over notes**, the views that show them, and what ＋ makes. It is not a new
//! property written into notes: the rule reads the markers notes already carry, and the server's
//! note *types* ([`crate::kinds::kind_of`]) may be named in it.
//!
//! The shipped sections are files (`shipped/kinds/*.kind`, YAML, the `.view` filter language), so
//! adding one is a file, not code; a person's own come later (P4). Read by the `sections` command;
//! `section_summary` counts each and names its newest note, in one pass over the store.

use crate::dto::ObjectMeta;
use crate::kinds::kind_of;
use crate::views::{lower_pred, PredDto};
use fm_core::{Store, StoreError};
use fm_query::{Filter, Query, SortKey};
use serde::{Deserialize, Serialize};

/// One way a section can be seen: exactly one of the four is set.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ViewRef {
    /// A built-in view of the app (`timeline`, `board`, `agenda`, `search`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin: Option<String>,
    /// A shipped or saved `.view`, by its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// A screen the app ships (the music deck, the videos, discussions, collaboration). Only a
    /// shipped section may name one: it is code, and says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special: Option<String>,
    /// A generic renderer over the section's notes (`list`, `table`, `gallery`; the shell, P2a).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renderer: Option<String>,
    /// For a table: its columns, by property name.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
    /// What the switcher calls it, when not the view's own name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ViewRef {
    fn check(&self) -> Result<(), String> {
        let set = [&self.builtin, &self.view, &self.special, &self.renderer]
            .iter()
            .filter(|v| v.is_some())
            .count();
        if set != 1 {
            return Err("a view needs exactly one of builtin, view, special or renderer".into());
        }
        if let Some(b) = &self.builtin {
            if !["timeline", "board", "agenda", "search"].contains(&b.as_str()) {
                return Err(format!("no built-in view called “{b}”"));
            }
        }
        if let Some(r) = &self.renderer {
            if !["list", "table", "gallery"].contains(&r.as_str()) {
                return Err(format!("no renderer called “{r}”"));
            }
        }
        Ok(())
    }
}

/// What ＋ makes in a section.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct NewSpec {
    /// The properties a new note starts with.
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub props: serde_json::Map<String, serde_json::Value>,
    /// The fields the quick form asks for, in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<String>,
}

/// One `.kind` file, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionFile {
    key: String,
    label: String,
    icon: String,
    mark: String,
    purpose: String,
    order: i64,
    /// Today's page for it (`/f/…`), until the shell's own addresses replace them.
    page: String,
    /// The server's note types it holds (`kind_of`); empty for any.
    #[serde(default)]
    types: Vec<String>,
    /// The rule over notes, ANDed: the `.view` filter language.
    #[serde(default, rename = "match")]
    rule: Vec<PredDto>,
    views: Vec<ViewRef>,
    #[serde(default)]
    new: NewSpec,
}

/// A section, as the interface and the assistant read it.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Section {
    pub key: String,
    pub label: String,
    pub icon: String,
    pub mark: String,
    pub purpose: String,
    pub order: i64,
    pub page: String,
    pub types: Vec<String>,
    pub views: Vec<ViewRef>,
    pub new: NewSpec,
}

/// How many notes a section holds, and its newest.
#[derive(Debug, Serialize)]
pub struct Summary {
    pub key: String,
    pub count: usize,
    pub latest: Option<ObjectMeta>,
    /// When each of the section's notes made in the last [`RECENT_DAYS`] days was made, newest
    /// first: Home's figures count them per day on the app's clock (`appTime.ts`), so the zone is
    /// the UI's and the server only lists (Home, 2026-11-23).
    pub recent: Vec<String>,
}

/// How far back [`Summary::recent`] reaches: a month's figure and its trend, with room for a zone.
pub const RECENT_DAYS: i64 = 35;
/// At most this many in [`Summary::recent`]: a figure past it says "2000+", it never stalls a read.
const RECENT_MAX: usize = 2000;

const SHIPPED: &[(&str, &str)] = &[
    ("notes", include_str!("shipped/kinds/notes.kind")),
    ("tasks", include_str!("shipped/kinds/tasks.kind")),
    ("agenda", include_str!("shipped/kinds/agenda.kind")),
    ("food", include_str!("shipped/kinds/food.kind")),
    ("fitness", include_str!("shipped/kinds/fitness.kind")),
    ("health", include_str!("shipped/kinds/health.kind")),
    ("music", include_str!("shipped/kinds/music.kind")),
    ("videos", include_str!("shipped/kinds/videos.kind")),
    ("discussions", include_str!("shipped/kinds/discussions.kind")),
    ("collaboration", include_str!("shipped/kinds/collaboration.kind")),
    ("web", include_str!("shipped/kinds/web.kind")),
];

struct Loaded {
    section: Section,
    filter: Filter,
}

fn parse(file: &str, text: &str) -> Result<Loaded, String> {
    let f: SectionFile = serde_yaml_ng::from_str(text).map_err(|e| format!("{file}.kind: {e}"))?;
    if f.key != file {
        return Err(format!("{file}.kind: its key is “{}”, not its file's name", f.key));
    }
    if f.views.is_empty() {
        return Err(format!("{file}.kind: a section needs at least one view"));
    }
    for v in &f.views {
        v.check().map_err(|e| format!("{file}.kind: {e}"))?;
    }
    let mut filter = Filter::new();
    for (i, p) in f.rule.iter().enumerate() {
        filter.all.push(lower_pred(p).map_err(|e| format!("{file}.kind: match[{i}]: {e}"))?);
    }
    Ok(Loaded {
        section: Section {
            key: f.key,
            label: f.label,
            icon: f.icon,
            mark: f.mark,
            purpose: f.purpose,
            order: f.order,
            page: f.page,
            types: f.types,
            views: f.views,
            new: f.new,
        },
        filter,
    })
}

fn loaded() -> Vec<Loaded> {
    let mut out: Vec<Loaded> = SHIPPED
        .iter()
        // A shipped file that does not parse is a build error in spirit: the test below fails on it,
        // and a release never carries one. Skipped here rather than taking the server down.
        .filter_map(|(file, text)| parse(file, text).ok())
        .collect();
    out.sort_by_key(|l| l.section.order);
    out
}

/// The sections, in their order.
pub fn sections() -> Vec<Section> {
    loaded().into_iter().map(|l| l.section).collect()
}

/// A section's notes, newest first, at most `limit` (the generic views: list, table, gallery).
pub fn notes(store: &dyn Store, key: &str, limit: usize) -> Result<Vec<ObjectMeta>, StoreError> {
    let Some(l) = loaded().into_iter().find(|l| l.section.key == key) else {
        return Err(StoreError::Io(format!("no section called “{key}”")));
    };
    let q = Query { filter: l.filter, sort: vec![SortKey::desc("updated")], ..Default::default() };
    Ok(store
        .query(&q)?
        .rows
        .into_iter()
        .filter(|o| l.section.types.is_empty() || l.section.types.iter().any(|t| t == kind_of(o)))
        .take(limit)
        .map(|o| ObjectMeta::from(&o))
        .collect())
}

/// The notes carrying `tag`, newest first, at most `limit`: the Home note (tagged `home`), and any
/// other one place a screen keeps as a note.
pub fn tagged(store: &dyn Store, tag: &str, limit: usize) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: Filter::new().and(fm_query::Predicate::TagsAny(vec![tag.to_string()])),
        sort: vec![SortKey::desc("updated")],
        limit: Some(limit),
        ..Default::default()
    };
    Ok(store.query(&q)?.rows.iter().map(ObjectMeta::from).collect())
}

/// **The journal** (Home, 2026-11-23): notes tagged `journal` whose title is a day (`2026-11-23`),
/// the form an imported Logseq or Obsidian journal already takes (`fm_core::import`). Newest day
/// first, from `on_or_before` (inclusive) back, at most `limit`. Today's entry is the first when its
/// title is today; the UI makes it on the first keystroke, so no empty day is ever written.
pub fn journal(
    store: &dyn Store,
    on_or_before: &str,
    limit: usize,
) -> Result<Vec<ObjectMeta>, StoreError> {
    let q = Query {
        filter: Filter::new().and(fm_query::Predicate::TagsAny(vec!["journal".to_string()])),
        ..Default::default()
    };
    let mut days: Vec<ObjectMeta> = store
        .query(&q)?
        .rows
        .iter()
        .map(ObjectMeta::from)
        .filter(|n| n.title.as_deref().is_some_and(|t| is_day(t) && t <= on_or_before))
        .collect();
    days.sort_by(|a, b| b.title.cmp(&a.title));
    days.truncate(limit);
    Ok(days)
}

/// `2026-11-23`, and nothing else: a journal's title is the day it is for.
fn is_day(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Each section's count and newest note, from one read of the store.
pub fn summary(store: &dyn Store) -> Result<Vec<Summary>, StoreError> {
    let (objects, _) = store.candidates(&Filter::new())?;
    let since = time::OffsetDateTime::now_utc() - time::Duration::days(RECENT_DAYS);
    Ok(loaded()
        .into_iter()
        .map(|l| {
            let q = Query {
                filter: l.filter,
                sort: vec![SortKey::desc("updated")],
                ..Default::default()
            };
            let rows: Vec<_> = fm_query::run(&q, &objects)
                .rows
                .into_iter()
                .filter(|o| {
                    l.section.types.is_empty() || l.section.types.iter().any(|t| t == kind_of(o))
                })
                .collect();
            let mut made: Vec<_> = rows.iter().map(|o| o.created).filter(|t| *t >= since).collect();
            made.sort_unstable_by(|a, b| b.cmp(a));
            Summary {
                key: l.section.key,
                count: rows.len(),
                latest: rows.first().map(ObjectMeta::from),
                recent: made.into_iter().take(RECENT_MAX).map(crate::dto::stamp).collect(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_section_parses_and_names_its_views() {
        for (file, text) in SHIPPED {
            let l = parse(file, text).unwrap_or_else(|e| panic!("{e}"));
            assert!(!l.section.views.is_empty(), "{file}");
            assert!(l.section.page.starts_with("/f/"), "{file}");
        }
        let keys: Vec<_> = sections().into_iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            [
                "notes",
                "web",
                "tasks",
                "agenda",
                "food",
                "fitness",
                "health",
                "music",
                "videos",
                "discussions",
                "collaboration"
            ]
        );
    }

    #[test]
    fn a_broken_section_is_named_precisely() {
        let e = parse("x", "key: x\nlabel: X\nicon: pen\nmark: x\npurpose: p\norder: 1\npage: /f/x\nviews:\n  - builtin: nowhere\n")
            .err()
            .unwrap();
        assert!(e.contains("no built-in view called “nowhere”"), "{e}");
        let e = parse("y", "key: z\nlabel: X\nicon: pen\nmark: x\npurpose: p\norder: 1\npage: /f/x\nviews:\n  - builtin: board\n")
            .err()
            .unwrap();
        assert!(e.contains("not its file's name"), "{e}");
    }

    /// **The interface's copy** (`ui/src/lib/sections.json`) is these files' JSON: the mock and the
    /// interface's tests read it, so the files stay the one truth. Run with `FM_WRITE_SECTIONS=1` to
    /// write it after changing a `.kind` file.
    #[test]
    fn the_interface_copy_matches_the_files() {
        let want = serde_json::to_string_pretty(&sections()).unwrap() + "\n";
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/lib/sections.json");
        if std::env::var("FM_WRITE_SECTIONS").is_ok() {
            std::fs::write(&path, &want).unwrap();
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            have == want,
            "ui/src/lib/sections.json is out of date: FM_WRITE_SECTIONS=1 pixi run cargo test -p fm-app sections"
        );
    }
}
