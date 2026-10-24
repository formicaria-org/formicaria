//! **The `shelf` property becomes a `shelf:` tag** (`decisions.md#ui`, 2026-10-24, *a shelf is a
//! `shelf:` tag*). Run at every start, from `App::load_with`: each note still carrying `shelf: X`
//! is rewritten to carry the tag `shelf:X` instead, so the Library finds it where it was. Every
//! start, not once, because an older build on another device can still write the property.
//!
//! **`updated` is left as it was.** This is not the person's edit, and two devices rewriting the
//! same note then write the same bytes, which merge as one.

use crate::views::SHELF_TAG;
use fm_core::{Store, StoreError};
use fm_model::PropertyValue;
use fm_query::{Filter, Op, Predicate, Query};

/// The property shelves used to be kept in.
const OLD: &str = "shelf";

/// Rewrite every note's `shelf` property as a `shelf:` tag. Answers how many notes were rewritten.
pub fn migrate(store: &mut dyn Store) -> Result<usize, StoreError> {
    let q = Query {
        filter: Filter::new().and(Predicate::Prop {
            key: OLD.into(),
            op: Op::Exists,
            value: PropertyValue::Null,
        }),
        ..Default::default()
    };
    let rows = store.query(&q)?.rows;
    let mut n = 0;
    for mut obj in rows {
        let Some(old) = obj.extra.remove(OLD) else { continue };
        let name = old.display();
        let name = name.trim();
        if !name.is_empty() {
            let tag = format!("{SHELF_TAG}{name}");
            if !obj.tags.contains(&tag) {
                obj.tags.push(tag);
            }
        }
        store.put(&obj)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_core::MemoryStore;
    use fm_model::{Kind, Object};

    #[test]
    fn a_shelf_property_becomes_a_shelf_tag_once_and_updated_is_kept() {
        let mut s = MemoryStore::new();
        let mut a = Object::new(Kind::Note, "Medicine");
        a.tags = vec!["template".into()];
        a.extra.insert("shelf".into(), PropertyValue::Text("health".into()));
        let mut b = Object::new(Kind::Note, "Already tagged");
        b.tags = vec!["shelf:Reading".into()];
        b.extra.insert("shelf".into(), PropertyValue::Text("Reading".into()));
        let c = Object::new(Kind::Note, "On no shelf");
        for o in [&a, &b, &c] {
            s.put(o).unwrap();
        }

        assert_eq!(migrate(&mut s).unwrap(), 2);
        let a2 = s.get(a.id).unwrap().unwrap();
        assert_eq!(a2.tags, ["template", "shelf:health"], "the case is kept");
        assert!(!a2.extra.contains_key("shelf"), "the property is gone");
        assert_eq!(a2.updated, a.updated, "not the person's edit");
        let b2 = s.get(b.id).unwrap().unwrap();
        assert_eq!(b2.tags, ["shelf:Reading"], "never added twice");
        assert_eq!(migrate(&mut s).unwrap(), 0, "nothing left to rewrite");
    }
}
