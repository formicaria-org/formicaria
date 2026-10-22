//! **The health record**: what changes rarely and is asked about at every visit — medicines,
//! allergies, conditions, results, visits, immunizations, procedures, devices, contacts
//! (`health-plan-2026-10-22.md` §9; the International Patient Summary's sections, read at source).
//!
//! **Every item is an ordinary note**, made from a template the person asked for, shelved
//! `shelf: health` in the Library, and known by its `record` property. Nothing here is a log entry
//! and nothing is stored but the notes. A result carries **the lab's own range and flag as
//! printed**; the app never supplies, judges or compares one (MedlinePlus: *"You can't compare your
//! results to reference ranges that you might find elsewhere."*).

use fm_core::{Store, StoreError};
use fm_model::{Kind, PropertyValue};
use fm_query::{Filter, Op, Predicate, Query, SortKey};
use serde::Serialize;
use std::collections::BTreeMap;

/// The property a record note is known by, and its kinds.
pub const RECORD: &str = "record";
/// The last three are the person's own Mind notes (a thought record, worry time, a piece of
/// writing), kept on the `mind` shelf; the doctor's page never shows them.
pub const KINDS: [&str; 13] = [
    "medicine",
    "allergy",
    "condition",
    "result",
    "visit",
    "immunization",
    "procedure",
    "device",
    "contact",
    "none_known",
    "thought",
    "worry",
    "writing",
];
/// Whether an item is in force: a medicine *current · stopped · on hold*, an allergy or a condition
/// *active · inactive · remission · resolved* (IPS `status` and `clinicalStatus`). Not `status`, which
/// is the board's column: a medicine is not a task.
pub const RECORD_STATUS: &str = "record_status";
/// What a *none known* note is about: `medicine`, `allergy` or `condition` (IPS `emptyReason`).
pub const ABOUT: &str = "about";

/// Keys every item has that are not its own fields.
const OWN: [&str; 4] = [RECORD, RECORD_STATUS, ABOUT, "shelf"];

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RecordItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Its first and last day: a medicine's start and stop, a visit's day.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Its other properties as written: a dose, a reaction, a result's value and the lab's range.
    pub fields: BTreeMap<String, String>,
}

/// The record, and which of its kinds the person already has a template for, so starting a record
/// is offered once.
#[derive(Debug, Serialize)]
pub struct RecordAnswer {
    pub items: Vec<RecordItem>,
    pub templates: Vec<String>,
}

fn day(s: Option<fm_model::Stamp>) -> Option<String> {
    s.map(|s| format!("{:04}-{:02}-{:02}", s.date.year(), s.date.month() as u8, s.date.day()))
}

/// **Every record item**, in the order of [`KINDS`], then by title. Templates are left out: a
/// template is the shape of a note, not one of the person's medicines.
pub fn health_record(store: &dyn Store) -> Result<RecordAnswer, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Prop {
            key: RECORD.into(),
            op: Op::Exists,
            value: PropertyValue::Null,
        }),
        sort: vec![SortKey::asc("title")],
        ..Default::default()
    };
    let text = |v: PropertyValue| match v {
        PropertyValue::Text(t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => None,
    };
    let rows = store.query(&q)?.rows;
    let mut templates: Vec<String> = rows
        .iter()
        .filter(|o| o.kind == Kind::Note && crate::thread::is_template(o))
        .filter_map(|o| text(o.get(RECORD)))
        .collect();
    templates.sort();
    templates.dedup();
    let mut out: Vec<RecordItem> = rows
        .into_iter()
        .filter(|o| o.kind == Kind::Note && !crate::thread::is_template(o))
        .filter_map(|o| {
            let kind = text(o.get(RECORD)).filter(|k| KINDS.contains(&k.as_str()))?;
            let fields = o
                .extra
                .iter()
                .filter(|(k, _)| !OWN.contains(&k.as_str()) && !k.starts_with("conflict-"))
                .map(|(k, v)| (k.clone(), v.display()))
                .filter(|(_, v)| !v.trim().is_empty())
                .collect();
            Some(RecordItem {
                id: o.id.to_string(),
                kind,
                title: o.title.clone().unwrap_or_default(),
                status: text(o.get(RECORD_STATUS)),
                start: day(o.start),
                due: day(o.due),
                about: text(o.get(ABOUT)),
                fields,
            })
        })
        .collect();
    let rank = |k: &str| KINDS.iter().position(|x| *x == k).unwrap_or(KINDS.len());
    out.sort_by_key(|a| rank(&a.kind));
    Ok(RecordAnswer { items: out, templates })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::MemoryStore;
    use fm_model::Object;

    fn put(s: &mut MemoryStore, title: &str, kv: &[(&str, &str)], tags: &[&str]) {
        let mut o = Object::new(Kind::Note, "");
        o.title = Some(title.into());
        o.tags = tags.iter().map(|t| (*t).to_string()).collect();
        for (k, v) in kv {
            o.extra.insert((*k).into(), PropertyValue::Text((*v).into()));
        }
        s.put(&o).unwrap();
    }

    #[test]
    fn the_record_is_every_note_with_a_record_kind_in_order_and_templates_are_not_in_it() {
        let mut s = MemoryStore::new();
        put(&mut s, "Medicine", &[("record", "medicine"), ("shelf", "health")], &["template"]);
        put(
            &mut s,
            "House dust mite",
            &[("record", "allergy"), ("record_status", "active"), ("reaction", "blocked nose")],
            &[],
        );
        put(
            &mut s,
            "Fluticasone nasal spray",
            &[
                ("record", "medicine"),
                ("record_status", "current"),
                ("dose_text", "2 sprays each nostril, mornings"),
            ],
            &[],
        );
        put(
            &mut s,
            "IgE D. pteronyssinus",
            &[("record", "result"), ("value_text", "13.20 kUA/L"), ("ref_text", "< 0.35 kUA/L")],
            &[],
        );
        put(&mut s, "Not a record", &[("record", "recipe")], &[]);
        let answer = health_record(&s).unwrap();
        assert_eq!(answer.templates, vec!["medicine"]);
        let r = answer.items;
        assert_eq!(
            r.iter().map(|i| (i.kind.as_str(), i.title.as_str())).collect::<Vec<_>>(),
            vec![
                ("medicine", "Fluticasone nasal spray"),
                ("allergy", "House dust mite"),
                ("result", "IgE D. pteronyssinus")
            ]
        );
        assert_eq!(r[0].status.as_deref(), Some("current"));
        assert_eq!(r[0].fields["dose_text"], "2 sprays each nostril, mornings");
        // The lab's range is carried as written, never one of the app's.
        assert_eq!(r[2].fields["ref_text"], "< 0.35 kUA/L");
        assert!(!r[1].fields.contains_key("record_status"));
    }
}
