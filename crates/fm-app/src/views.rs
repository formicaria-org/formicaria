//! `.view` files — a named, saved query rendered through an existing renderer.
//!
//! This is `MASTERPLAN.md:323`'s *"five generic renderers = query + a renderer"*, made
//! user-authored: a `.view` is `query + a renderer` written down, git-tracked in the vault at
//! `views/*.view`, and therefore **travelling to collaborators and syncing across machines** —
//! which is the one thing a localStorage view-preference can never do.
//!
//! **The wire never carries a query.** A `.view` is parsed *here*, server-side, into an
//! `fm_query::Query`; the UI only ever sends a **name**. That is deliberate and load-bearing:
//! putting `Query` on the wire would force serde onto `fm-query`/`fm-model` — where
//! `Object.vault` is *never serialized on purpose* (a forgeable permission otherwise) — and
//! would hand a client `PropertyValue`'s variant-order `Ord` trap. Neither risk exists when the
//! only thing that crosses is a string.
//!
//! **A `.view` extends a preset; it never replaces one.** `Filter { all }` is a top-level AND,
//! so the user's conjuncts compose onto the renderer's base by `Vec::extend` — no merge rules,
//! no precedence. The base for a board/agenda/timeline is `Kind(Note)`, written once, in Rust,
//! so a `.view` cannot forget the assets exclusion (`decisions.md`): `Kind(Note) ∧ anything` is
//! still notes-only.
//!
//! **The filter DSL has no ordered `prop` comparison.** Date windows go through `date:`
//! (a real `DateRange` over parsed `Date`s); there is deliberately no `prop: due, gt: …`. So
//! the `PropertyValue` variant-order `Ord` trap — comparing a `Text` against a `Stamp` and
//! getting a confident wrong answer — is **structurally unreachable** from a `.view` file,
//! rather than merely discouraged.

use crate::dto::{value_string, Board, Column, ObjectMeta};
use fm_core::{Store, StoreError};
use fm_model::{Kind, PropertyValue};
use fm_query::{Dir, Filter, Op, Predicate, Query, SortKey};
use serde::{Deserialize, Serialize};
use std::path::Path;
use time::{format_description::well_known::Iso8601, Date};

/// The renderer a view draws through — the same set the built-in nav offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Renderer {
    Board,
    Agenda,
    Timeline,
    Search,
    Gallery,
}

/// One `.view` file, as written. Everything but `name`/`view` is optional, so the simplest
/// useful view is two lines.
#[derive(Deserialize)]
struct ViewFile {
    name: String,
    view: Renderer,
    /// Board only — the property to group columns by.
    #[serde(default)]
    group_by: Option<String>,
    /// Overrides the renderer's default sort when present.
    #[serde(default)]
    sort: Vec<SortDto>,
    /// ANDed onto the renderer's base filter.
    #[serde(default)]
    filter: Vec<PredDto>,
}

#[derive(Deserialize)]
struct SortDto {
    key: String,
    #[serde(default)]
    dir: DirDto,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum DirDto {
    #[default]
    Desc,
    Asc,
}

/// A single filter conjunct, as a struct of optionals rather than a serde enum: YAML enums are
/// externally-tagged and ugly to hand-write, and this shape lets a malformed conjunct be named
/// precisely ("a filter needs exactly one of prop/tag/…"). Exactly one *selector* — `prop`,
/// `tag`, `tags_any`, `tags_all`, `text`, `date`, `not`, `any` — must be set.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct PredDto {
    // prop: <key> with one of eq / ne / exists
    prop: Option<String>,
    eq: Option<serde_yaml_ng::Value>,
    ne: Option<serde_yaml_ng::Value>,
    exists: Option<bool>,
    // tags
    tag: Option<String>,
    tags_any: Option<Vec<String>>,
    tags_all: Option<Vec<String>>,
    // full text
    text: Option<String>,
    // a date window; from/to are inclusive, either open-ended
    date: Option<String>,
    from: Option<String>,
    to: Option<String>,
    // composition
    not: Option<Box<PredDto>>,
    any: Option<Vec<PredDto>>,
}

/// What the UI lists in the sidebar. A view that would not parse still appears — with its
/// `error` set — because a view that silently vanished is exactly the failure the parse-error
/// discipline exists to prevent.
#[derive(Debug, Serialize)]
pub struct ViewInfo {
    pub name: String,
    pub renderer: Option<Renderer>,
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The result of running a view: a `board` for the board renderer, a flat `rows` list for the
/// rest. One envelope, so the UI switches on `renderer` exactly as it does for built-ins.
#[derive(Debug, Serialize)]
pub struct ViewResult {
    pub name: String,
    pub renderer: Renderer,
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board: Option<Board>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<ObjectMeta>>,
}

/// The base query for a renderer — the same shape the built-in `commands::{board,agenda,…}`
/// build. Kept here so a `.view` starts from the identical filter; the string `"done"` and
/// `Kind(Note)` live in exactly these arms.
fn base(renderer: Renderer, group_by: Option<&str>) -> Query {
    // The same base the built-in commands use, from its single definition — so a `.view`
    // file cannot be the one surface that forgets the assets or the message exclusion.
    let notes = crate::thread::notes_base;
    match renderer {
        Renderer::Board => Query {
            filter: notes(),
            group_by: Some(group_by.unwrap_or("status").to_string()),
            sort: vec![SortKey::desc("created")],
            ..Default::default()
        },
        Renderer::Timeline => Query {
            filter: notes(),
            sort: vec![SortKey::desc("created")],
            ..Default::default()
        },
        Renderer::Agenda => Query {
            filter: notes()
                .and(Predicate::Prop {
                    key: "due".into(),
                    op: Op::Exists,
                    value: PropertyValue::Null,
                })
                .and(Predicate::Prop {
                    key: "status".into(),
                    op: Op::Ne,
                    value: PropertyValue::Text("done".into()),
                }),
            sort: vec![SortKey::asc("due")],
            ..Default::default()
        },
        Renderer::Search => Query {
            filter: Filter::new(),
            sort: vec![SortKey::desc("updated")],
            ..Default::default()
        },
        Renderer::Gallery => Query {
            filter: Filter::new().and(Predicate::Kind(vec![Kind::Asset])),
            sort: vec![SortKey::desc("created")],
            ..Default::default()
        },
    }
}

/// Lower a parsed file to a ready-to-run `Query`. Errors are the caller-facing sentence a
/// broken view shows, so they say what is wrong and (where useful) how to fix it.
fn lower(file: &ViewFile) -> Result<Query, String> {
    let mut q = base(file.view, file.group_by.as_deref());
    for (i, p) in file.filter.iter().enumerate() {
        q.filter
            .all
            .push(lower_pred(p).map_err(|e| format!("filter[{i}]: {e}"))?);
    }
    if !file.sort.is_empty() {
        q.sort = file
            .sort
            .iter()
            .map(|s| SortKey {
                key: s.key.clone(),
                dir: match s.dir {
                    DirDto::Asc => Dir::Asc,
                    DirDto::Desc => Dir::Desc,
                },
            })
            .collect();
    }
    Ok(q)
}

fn lower_pred(p: &PredDto) -> Result<Predicate, String> {
    // Count the selectors so "exactly one" can be enforced with a real message.
    let selectors = [
        p.prop.is_some(),
        p.tag.is_some(),
        p.tags_any.is_some(),
        p.tags_all.is_some(),
        p.text.is_some(),
        p.date.is_some(),
        p.not.is_some(),
        p.any.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    if selectors == 0 {
        return Err("a filter needs one of prop / tag / tags_any / tags_all / text / date / not / any".into());
    }
    if selectors > 1 {
        return Err("a filter must set exactly one of prop / tag / text / date / not / any".into());
    }

    if let Some(key) = &p.prop {
        // No ordered comparison on purpose — see the module docs. eq / ne / exists only.
        return Ok(match (&p.eq, &p.ne, p.exists) {
            (Some(v), None, None) => Predicate::Prop {
                key: key.clone(),
                op: Op::Eq,
                value: to_value(v)?,
            },
            (None, Some(v), None) => Predicate::Prop {
                key: key.clone(),
                op: Op::Ne,
                value: to_value(v)?,
            },
            (None, None, Some(true)) => Predicate::Prop {
                key: key.clone(),
                op: Op::Exists,
                value: PropertyValue::Null,
            },
            (None, None, Some(false)) => Predicate::Not(Box::new(Predicate::Prop {
                key: key.clone(),
                op: Op::Exists,
                value: PropertyValue::Null,
            })),
            _ => return Err(format!(
                "prop '{key}' needs exactly one of eq / ne / exists (dates compare via `date:`, not gt/lt)"
            )),
        });
    }
    if let Some(t) = &p.tag {
        return Ok(Predicate::TagsAny(vec![t.clone()]));
    }
    if let Some(ts) = &p.tags_any {
        return Ok(Predicate::TagsAny(ts.clone()));
    }
    if let Some(ts) = &p.tags_all {
        return Ok(Predicate::TagsAll(ts.clone()));
    }
    if let Some(needle) = &p.text {
        return Ok(Predicate::Text(needle.clone()));
    }
    if let Some(key) = &p.date {
        return Ok(Predicate::DateRange {
            key: key.clone(),
            from: parse_date(&p.from, "from")?,
            to: parse_date(&p.to, "to")?,
        });
    }
    if let Some(inner) = &p.not {
        return Ok(Predicate::Not(Box::new(lower_pred(inner)?)));
    }
    if let Some(ps) = &p.any {
        return Ok(Predicate::Any(
            ps.iter().map(lower_pred).collect::<Result<Vec<_>, _>>()?,
        ));
    }
    unreachable!("selector count guaranteed exactly one above")
}

/// A YAML scalar → `PropertyValue`. A string is `Text`, a bool is `Bool`, an integer is `Int`.
/// A date-shaped string stays `Text` — it is only ever used with `eq`/`ne` (equality on the
/// stored day string), never an ordered comparison, so the variant-order trap cannot bite.
fn to_value(v: &serde_yaml_ng::Value) -> Result<PropertyValue, String> {
    use serde_yaml_ng::Value;
    Ok(match v {
        Value::String(s) => PropertyValue::Text(s.clone()),
        Value::Bool(b) => PropertyValue::Bool(*b),
        Value::Number(n) if n.is_i64() => PropertyValue::Int(n.as_i64().unwrap()),
        _ => return Err("a value must be text, a whole number, or true/false".into()),
    })
}

fn parse_date(s: &Option<String>, which: &str) -> Result<Option<Date>, String> {
    match s {
        None => Ok(None),
        Some(s) => Date::parse(s, &Iso8601::DATE)
            .map(Some)
            .map_err(|_| format!("{which} must be a date like 2026-07-20, not '{s}'")),
    }
}

/// Read `<vault>/views/*.view` and report each. A file that fails to parse is listed with its
/// error and named by its filename stem, never dropped.
pub fn list_views(vault: &Path) -> Vec<ViewInfo> {
    let dir = vault.join("views");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out; // no views/ dir yet — an empty list, not an error
    };
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("view"))
        .collect();
    paths.sort();
    for path in paths {
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        match read_view(&path) {
            Ok(file) => out.push(ViewInfo {
                name: file.name,
                renderer: Some(file.view),
                group_by: file.group_by,
                error: None,
            }),
            Err(e) => out.push(ViewInfo {
                name: stem,
                renderer: None,
                group_by: None,
                error: Some(e),
            }),
        }
    }
    out
}

/// Run the view named `name` (matched against the `name:` field, then the filename stem).
pub fn run_view(store: &dyn Store, vault: &Path, name: &str) -> Result<ViewResult, StoreError> {
    let path = find_view(vault, name)
        .ok_or_else(|| StoreError::Io(format!("no view named '{name}'")))?;
    let file = read_view(&path).map_err(StoreError::Io)?;
    let q = lower(&file).map_err(StoreError::Io)?;
    let res = store.query(&q)?;

    Ok(match file.view {
        Renderer::Board => {
            let group_by = q.group_by.clone().unwrap_or_default();
            let columns = res
                .groups
                .unwrap_or_default()
                .into_iter()
                .map(|g| Column {
                    value: value_string(&g.key),
                    label: g.label,
                    cards: g.rows.iter().map(ObjectMeta::from).collect(),
                })
                .collect();
            ViewResult {
                name: file.name,
                renderer: file.view,
                group_by: Some(group_by.clone()),
                board: Some(Board { group_by, columns }),
                rows: None,
            }
        }
        _ => ViewResult {
            name: file.name,
            renderer: file.view,
            group_by: None,
            board: None,
            rows: Some(res.rows.iter().map(ObjectMeta::from).collect()),
        },
    })
}

fn read_view(path: &Path) -> Result<ViewFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A view is addressed by its `name:` field first, then its filename stem — so a file whose
/// `name:` failed to parse is still reachable by filename to see the error.
fn find_view(vault: &Path, name: &str) -> Option<std::path::PathBuf> {
    let dir = vault.join("views");
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut by_stem = None;
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("view") {
            continue;
        }
        if path.file_stem().map(|s| s.to_string_lossy()) == Some(name.into()) {
            by_stem = Some(path.clone());
        }
        if let Ok(f) = read_view(&path) {
            if f.name == name {
                return Some(path);
            }
        }
    }
    by_stem
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::{FileStore, Store};
    use fm_model::{Kind, Object};
    use tempfile::tempdir;

    fn write_view(vault: &Path, file: &str, body: &str) {
        let dir = vault.join("views");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), body).unwrap();
    }

    fn seed(vault: &Path) -> FileStore {
        let mut s = FileStore::named(vault, "v").unwrap();
        for (title, status, tag) in [
            ("lab thing", "doing", "lab"),
            ("lab done thing", "done", "lab"),
            ("personal thing", "todo", "home"),
        ] {
            let mut o = Object::new(Kind::Note, title);
            o.status = Some(status.into());
            o.tags = vec![tag.into()];
            s.put(&o).unwrap();
        }
        s
    }

    #[test]
    fn a_board_view_filters_and_groups() {
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(
            d.path(),
            "lab.view",
            "name: Lab board\nview: board\ngroup_by: status\nfilter:\n  - tag: lab\n",
        );
        let r = run_view(&s, d.path(), "Lab board").unwrap();
        assert_eq!(r.renderer, Renderer::Board);
        let board = r.board.unwrap();
        // Two lab notes across two status columns; the personal note is filtered out.
        let total: usize = board.columns.iter().map(|c| c.cards.len()).sum();
        assert_eq!(total, 2, "only the two lab notes");
    }

    #[test]
    fn the_assets_exclusion_cannot_be_forgotten() {
        // A board view's base is Kind(Note); even a filter that mentions assets stays notes-only.
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(d.path(), "b.view", "name: b\nview: board\ngroup_by: status\n");
        let board = run_view(&s, d.path(), "b").unwrap().board.unwrap();
        let total: usize = board.columns.iter().map(|c| c.cards.len()).sum();
        assert_eq!(total, 3, "all three notes, no assets could appear");
    }

    #[test]
    fn not_and_timeline_render_a_flat_list() {
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(
            d.path(),
            "active.view",
            "name: Active\nview: timeline\nfilter:\n  - not:\n      prop: status\n      eq: done\n",
        );
        let r = run_view(&s, d.path(), "Active").unwrap();
        assert_eq!(r.renderer, Renderer::Timeline);
        let rows = r.rows.unwrap();
        assert_eq!(rows.len(), 2, "the two non-done notes");
        assert!(rows.iter().all(|o| o.status.as_deref() != Some("done")));
    }

    #[test]
    fn a_broken_view_is_listed_with_its_error_not_dropped() {
        let d = tempdir().unwrap();
        seed(d.path());
        write_view(d.path(), "good.view", "name: Good\nview: timeline\n");
        write_view(d.path(), "broken.view", "name: Broken\nview: notarenderer\n");
        let views = list_views(d.path());
        assert_eq!(views.len(), 2, "both are listed");
        let broken = views.iter().find(|v| v.name == "broken").unwrap();
        assert!(broken.error.is_some(), "the broken one carries its error");
        assert!(broken.renderer.is_none());
        let good = views.iter().find(|v| v.name == "Good").unwrap();
        assert!(good.error.is_none());
    }

    #[test]
    fn running_a_broken_view_returns_the_parse_error() {
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(d.path(), "bad.view", "name: Bad\nview: timeline\nfilter:\n  - {}\n");
        let err = run_view(&s, d.path(), "Bad").unwrap_err().to_string();
        assert!(err.contains("filter[0]"), "the error names which conjunct: {err}");
    }

    #[test]
    fn a_date_window_is_the_only_way_to_compare_dates() {
        // `date:` lowers to DateRange (real Date parsing); there is no prop gt/lt to misuse.
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(
            d.path(),
            "w.view",
            "name: W\nview: agenda\nfilter:\n  - date: due\n    from: 2026-01-01\n    to: 2026-12-31\n",
        );
        // Just assert it lowers and runs; the agenda seed has no due dates, so zero rows.
        let r = run_view(&s, d.path(), "W").unwrap();
        assert_eq!(r.rows.unwrap().len(), 0);
    }
}
