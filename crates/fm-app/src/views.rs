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
use std::path::{Path, PathBuf};
use time::{format_description::well_known::Iso8601, Date};

/// The renderer a view draws through — the same set the built-in nav offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Renderer {
    Board,
    Agenda,
    Timeline,
    Search,
    /// **Kept so old files still parse, but it no longer draws anything.** The gallery renderer was
    /// deliberately removed — assets open from the notes that reference them. A view asking for it
    /// used to fall through to the timeline in silence, which is the one thing `list_views` exists
    /// to prevent: it reports a view it cannot draw, it never quietly draws a different one.
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

impl PredDto {
    /// Is this predicate *only* a `tag:`? The one shape [`save_view`] can write, so it is also the
    /// one it may overwrite without losing anything the user put there.
    /// The tag this predicate restricts to, when it restricts to exactly one and nothing else.
    fn only_tag(&self) -> Option<String> {
        self.is_only_tag().then(|| self.tag.clone()).flatten()
    }

    fn is_only_tag(&self) -> bool {
        self.tag.is_some()
            && self.prop.is_none()
            && self.eq.is_none()
            && self.ne.is_none()
            && self.exists.is_none()
            && self.tags_any.is_none()
            && self.tags_all.is_none()
            && self.text.is_none()
            && self.date.is_none()
            && self.from.is_none()
            && self.to.is_none()
            && self.not.is_none()
            && self.any.is_none()
    }
}


/// What the UI lists in the sidebar. A view that would not parse still appears — with its
/// `error` set — because a view that silently vanished is exactly the failure the parse-error
/// discipline exists to prevent.
#[derive(Debug, Serialize)]
pub struct ViewInfo {
    pub name: String,
    pub renderer: Option<Renderer>,
    pub group_by: Option<String>,
    /// **Which vault holds this file.** `list_views` walks one vault at a time and cannot know its
    /// name, so it leaves this empty and `dispatch` stamps it while iterating the configs. Without
    /// it the UI has a list of views and no idea where any of them live — and "delete" resolves
    /// against the *default* vault, where a view belonging to another one is simply not found.
    /// A delete that silently succeeds while deleting nothing is the worst answer available.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub vault: String,
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
    /// **What this view leaves out, in words** — one phrase per `filter:` entry, from
    /// [`describe_pred`]. Empty for a view that filters nothing.
    ///
    /// It exists because a view draws through the *same renderer as the built-in it shadows*: a
    /// `view: board` and the Board pane are the same pixels, so a filter that removes a whole
    /// column removes it invisibly. The owner reported exactly that (2026-08-24, "my done column
    /// is not showing up") against a view whose filter is `not: {prop: status, eq: done}` — and a
    /// `.view` file can be neither written nor deleted from the UI, so the filter was unreachable
    /// as well as unseen. Same discipline as the parse `error` on [`ViewInfo`]: *say why, rather
    /// than let an absence masquerade as missing notes.*
    ///
    /// **Here and not on [`ViewInfo`]** — it belongs to the payload it describes. The view list is
    /// fetched once per vault change and can fail (it has, on the phone); these words arrive with
    /// the very rows they explain, from the same file that was just read, so they can be neither
    /// late nor stale.
    ///
    /// Always serialised, `[]` and not absent, so the client has one shape to handle.
    pub filters: Vec<String>,
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

/// One `filter:` entry **in words** — the sentence half of [`lower_pred`].
///
/// Written against the *file's own vocabulary* (the table in `docs/src/user/views.md`) rather than
/// the lowered [`Predicate`], on purpose: the DSL is what the author wrote, so describing it reads
/// the file back. Describing the lowered form would answer `tagged any of ["lab"]` for something
/// spelled `tag: lab`, which is the engine's business and not the reader's.
///
/// `neg` carries a surrounding `not:` **into** each arm instead of wrapping the result, so a
/// negation reads like English — `status is not done`, not `not (status is done)`. `not:` recurses
/// with it flipped; `any:` becomes *none of* under it, which is De Morgan rather than a paraphrase.
///
/// **Total by construction.** A `.view` that parses as YAML can still be nonsense `lower_pred`
/// rejects (`prop:` with both `eq:` and `ne:`, or with no selector at all), and [`list_views`]
/// deliberately does not lower — it lists what is there. So every path answers *something*; the
/// real diagnosis comes from `run_view`, which does lower and reports the error.
fn describe_pred(p: &PredDto, neg: bool) -> String {
    // A scalar as the reader would recognise it. Deliberately *not* `to_value`, which is fallible
    // and stricter than it needs to be here: a float under `eq:` is a value `lower_pred` will
    // reject, and "1.5" is still the honest thing to show while explaining what the file says.
    let scalar = |v: &serde_yaml_ng::Value| {
        use serde_yaml_ng::Value;
        match v {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            _ => "a value this app cannot read".to_string(),
        }
    };

    if let Some(key) = &p.prop {
        return match (&p.eq, &p.ne, p.exists) {
            (Some(v), None, None) if neg => format!("{key} is not {}", scalar(v)),
            (Some(v), None, None) => format!("{key} is {}", scalar(v)),
            (None, Some(v), None) if neg => format!("{key} is {}", scalar(v)),
            (None, Some(v), None) => format!("{key} is not {}", scalar(v)),
            // `exists: false` and a surrounding `not:` cancel, so the two flags decide together.
            (None, None, Some(want)) if want == neg => format!("{key} is unset"),
            (None, None, Some(_)) => format!("{key} is set"),
            _ => format!("{key} — an entry this app cannot read"),
        };
    }
    if let Some(t) = &p.tag {
        return if neg { format!("not tagged {t}") } else { format!("tagged {t}") };
    }
    if let Some(ts) = &p.tags_any {
        return if neg {
            format!("tagged none of {}", join_words(ts, "or"))
        } else {
            format!("tagged {}", join_words(ts, "or"))
        };
    }
    if let Some(ts) = &p.tags_all {
        // The negation of "has all of these" is "is missing at least one" — say that, rather than
        // "not tagged a and b", which a reader would take for "has neither".
        return if neg {
            format!("not tagged all of {}", join_words(ts, "and"))
        } else {
            format!("tagged {}", join_words(ts, "and"))
        };
    }
    if let Some(needle) = &p.text {
        return if neg {
            format!("text does not match “{needle}”")
        } else {
            format!("text matches “{needle}”")
        };
    }
    if let Some(key) = &p.date {
        return match (&p.from, &p.to) {
            (Some(a), Some(b)) if neg => format!("{key} outside {a} to {b}"),
            (Some(a), Some(b)) => format!("{key} between {a} and {b}"),
            (Some(a), None) if neg => format!("{key} before {a}"),
            (Some(a), None) => format!("{key} on or after {a}"),
            (None, Some(b)) if neg => format!("{key} after {b}"),
            (None, Some(b)) => format!("{key} on or before {b}"),
            (None, None) if neg => format!("{key} is not a date"),
            (None, None) => format!("{key} is a date"),
        };
    }
    if let Some(inner) = &p.not {
        return describe_pred(inner, !neg);
    }
    if let Some(ps) = &p.any {
        let parts: Vec<String> = ps.iter().map(|q| describe_pred(q, false)).collect();
        return if neg {
            // Commas, not "or": "none of: a or b" reads as though one of them were still allowed.
            format!("none of: {}", parts.join(", "))
        } else {
            join_words(&parts, "or")
        };
    }
    "an entry this app cannot read".to_string()
}

/// `["a"]` → `a`; `["a","b"]` → `a or b`; `["a","b","c"]` → `a, b or c`; `[]` → `nothing`.
/// An empty list is a real (if pointless) thing to write in a `.view`, and a phrase ending in a
/// dangling "tagged " would read as a bug in the app rather than a quirk of the file.
fn join_words(items: &[String], conj: &str) -> String {
    match items {
        [] => "nothing".to_string(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} {conj} {last}", rest.join(", ")),
    }
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
/// The file a view of this name lives in.
///
/// **The name is a label; the filename is derived from it and never trusted.** A view name reaches
/// this from the UI, so anything that is not a letter, digit or space becomes `-` and the result is
/// bounded — otherwise a name containing `../` chooses where in the filesystem we write. The label
/// itself is preserved verbatim inside the file, so the user still sees what they typed.
fn view_path(vault: &Path, name: &str) -> Result<std::path::PathBuf, String> {
    crate::vaultfile::path_in(vault, "views", name, "view", "view")
}

/// Write a saved view: a renderer, for a board the property its columns group by, and optionally
/// **one tag to narrow it to**.
///
/// **Still deliberately not a filter editor.** The `filter:` grammar is nine kinds of predicate and
/// a UI for it is a query builder, which is the thing a non-technical user was never going to use.
/// One tag is not that. "Show me the ones tagged `paper`" is a sentence a person says, it is the
/// narrowing they already perform by eye, and without it the app could offer *no* filtered view at
/// all — the documented way to get one was to author YAML in a text editor, in an app whose owner
/// works only through the UI. So the rule is unchanged in spirit and one question wider in
/// practice: an arrangement you keep, now including who is in it.
///
/// **A richer filter is still never flattened.** A view written by hand may carry anything the
/// grammar allows; saving over it from here can only express a single `tag`, so anything else is
/// refused rather than silently dropped. Re-saving a view whose filter *is* a single tag is fine —
/// that is the one this surface can faithfully rewrite.
pub fn save_view(
    vault: &Path,
    name: &str,
    view: Renderer,
    group_by: Option<&str>,
    tag: Option<&str>,
    // **Returns the path it wrote.** Not decoration: `dispatch` has to enrol this file in the
    // store's write list or `commit` will never stage it, and the app will go on telling the user
    // their view "travels with your notes" while it exists only on this machine.
) -> Result<PathBuf, String> {
    // The app must not author a view it cannot draw. `list_views` now reports a gallery view as
    // broken, and writing a fresh one would be manufacturing that breakage from inside the app.
    if view == Renderer::Gallery {
        return Err(GALLERY_GONE.into());
    }
    let path = view_path(vault, name)?;
    let mut tag_carried_over: Option<String> = None;
    if let Ok(existing) = read_view(&path) {
        // What this surface can faithfully round-trip: nothing, or exactly one `tag:`.
        let expressible = existing.filter.is_empty()
            || (existing.filter.len() == 1 && existing.filter[0].is_only_tag());
        // **An absent tag means "leave the filter alone", never "delete it".** The dialog opens
        // with the box empty and does not prefill, so re-saving a tag-filtered view to change its
        // grouping would otherwise silently drop the filter — the exact guarantee the ruling this
        // extends exists to protect. Clearing a filter is a deletion and needs its own gesture.
        if tag.map(str::trim).is_none_or(str::is_empty) {
            if let Some(existing_tag) = existing.filter.first().and_then(PredDto::only_tag) {
                tag_carried_over = Some(existing_tag);
            }
        }
        if !expressible {
            return Err(format!(
                "\"{}\" already exists and filters its notes in a way this screen cannot rewrite. \
                 Saving over it here would drop that filter, so it is refused — rename this one, or \
                 edit that view's file.",
                existing.name
            ));
        }
    }
    let renderer = match view {
        Renderer::Board => "board",
        Renderer::Agenda => "agenda",
        Renderer::Timeline => "timeline",
        Renderer::Search => "search",
        Renderer::Gallery => "gallery",
    };
    // Written as the same YAML a person would write by hand — this is a file they own, in their
    // vault, tracked by git and read by collaborators, not an opaque app artifact.
    let mut body = format!("name: {}\nview: {renderer}\n", name.trim());
    if let Some(g) = group_by.filter(|g| !g.is_empty()) {
        body.push_str(&format!("group_by: {g}\n"));
    }
    let tag = tag.map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).or(tag_carried_over);
    if let Some(t) = tag.as_deref() {
        // **Serialised, never interpolated.** A tag may contain any character since tags became
        // comma-separated, and `format!("tag: {t}")` writes a file YAML cannot read — or, worse,
        // one it reads *wrongly*: a tag of `#todo` starts a comment, so the view parses clean,
        // lists as healthy in the sidebar, and silently matches nothing.
        let quoted = serde_yaml_ng::to_string(&serde_yaml_ng::Value::String(t.to_string()))
            .map_err(|e| format!("could not write that tag: {e}"))?;
        body.push_str(&format!("filter:\n  - tag: {}\n", quoted.trim_end()));
    }
    let dir = path.parent().ok_or("no views directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    std::fs::write(&path, body)
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(path)
}

/// Remove a saved view. Missing is success — the user asked for it to be gone.
pub fn delete_view(vault: &Path, name: &str) -> Result<PathBuf, String> {
    let path = view_path(vault, name)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(path),
        // The path comes back on the missing branch too. A file git still tracks but disk no longer
        // has is exactly the case that must reach `commit` — otherwise the deletion is never
        // recorded, the file returns on the next pull, and the user deletes it again.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(e) => Err(format!("could not delete {}: {e}", path.display())),
    }
}

/// What a view asking for the removed gallery renderer is told — by the lister and by the writer,
/// so the app never explains the same refusal two different ways.
const GALLERY_GONE: &str =
    "gallery is no longer a way of showing notes — change this view to timeline or board";

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
            Ok(file) if file.view == Renderer::Gallery => out.push(ViewInfo {
                name: file.name,
                renderer: None,
                group_by: None,
                vault: String::new(),
                error: Some(GALLERY_GONE.into()),
            }),
            Ok(file) => out.push(ViewInfo {
                name: file.name,
                renderer: Some(file.view),
                group_by: file.group_by,
                vault: String::new(),
                error: None,
            }),
            Err(e) => out.push(ViewInfo {
                name: stem,
                renderer: None,
                group_by: None,
                vault: String::new(),
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
    // Described from the same parse that just ran, so the words and the rows can never disagree.
    let filters: Vec<String> = file.filter.iter().map(|p| describe_pred(p, false)).collect();

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
                filters,
                board: Some(Board { group_by, columns }),
                rows: None,
            }
        }
        _ => ViewResult {
            name: file.name,
            renderer: file.view,
            group_by: None,
            filters,
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

    /// **The column that vanished.** The owner's only `.view` is a board that filters `done` out,
    /// and a `view: board` draws through the same renderer as the Board pane — so the missing
    /// column read as missing notes (2026-08-24). `list_views` now carries what the view narrows
    /// to, in the file's own words, and the pane header says it.
    #[test]
    fn a_filtered_view_says_what_it_leaves_out() {
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(
            d.path(),
            "active.view",
            "name: Active\nview: board\ngroup_by: status\nfilter:\n  - not:\n      prop: status\n      eq: done\n",
        );
        let r = run_view(&s, d.path(), "Active").unwrap();
        // English, not an expression: the `not:` is carried into the phrase, not wrapped round it.
        assert_eq!(r.filters, vec!["status is not done".to_string()]);
        // And the words describe *this* board: the column they name is the one that is absent.
        let cols = r.board.unwrap().columns;
        assert!(
            !cols.iter().any(|c| c.value == "done"),
            "the filtered column is gone — which is exactly why the words have to travel with it",
        );
    }

    /// Every spelling the manual documents, so a view that narrows by tag, text or date is as
    /// legible as one that narrows by status — the phrase must never be status-shaped.
    #[test]
    fn each_filter_spelling_has_words() {
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(
            d.path(),
            "many.view",
            "name: Many\nview: timeline\nfilter:\n  - tag: lab\n  - tags_all: [a, b]\n  \
             - not:\n      tags_any: [x, y]\n  - text: kalman\n  - date: due\n    from: 2026-07-14\n    \
             to: 2026-07-27\n  - prop: due\n    exists: true\n  - any:\n      - prop: status\n        \
             eq: doing\n      - tag: urgent\n",
        );
        assert_eq!(
            run_view(&s, d.path(), "Many").unwrap().filters,
            vec![
                "tagged lab",
                "tagged a and b",
                "tagged none of x or y",
                "text matches “kalman”",
                "due between 2026-07-14 and 2026-07-27",
                "due is set",
                "status is doing or tagged urgent",
            ]
        );
    }

    /// A `.view` can be valid YAML and still be nonsense the engine rejects. `describe_pred` runs
    /// over the parsed file, not the lowered query, so it must answer *something* for those rather
    /// than panic — the refusal itself is `run_view`'s (the test below), and it must be the thing
    /// the user sees, not a panic from the describer racing it there.
    #[test]
    fn an_unreadable_entry_gets_a_phrase_instead_of_a_panic() {
        let bad = PredDto {
            prop: Some("status".into()),
            eq: Some(serde_yaml_ng::Value::String("doing".into())),
            ne: Some(serde_yaml_ng::Value::String("done".into())),
            ..Default::default()
        };
        assert!(describe_pred(&bad, false).contains("cannot read"));
        // Nothing set at all — `- {}` in a filter list.
        assert!(describe_pred(&PredDto::default(), false).contains("cannot read"));
        // A value `lower_pred` will reject is still shown for what it is, not swallowed.
        let float = PredDto {
            prop: Some("weight".into()),
            eq: Some(serde_yaml_ng::Value::Number(1.5.into())),
            ..Default::default()
        };
        assert_eq!(describe_pred(&float, false), "weight is 1.5");
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

    /// A view that hides nothing says nothing: `[]`, not a phrase — that empty list is what keeps
    /// the pane's "filtered" chip off an unfiltered view.
    #[test]
    fn an_unfiltered_view_has_nothing_to_disclose() {
        let d = tempdir().unwrap();
        let s = seed(d.path());
        write_view(d.path(), "all.view", "name: All\nview: timeline\n");
        assert!(run_view(&s, d.path(), "All").unwrap().filters.is_empty());
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
