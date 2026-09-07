//! A 3-way merge for whiteboard scenes — the body of a `view: board` note.
//!
//! A board's body is an Excalidraw scene: JSON, one big `elements` array, re-serialized
//! whole on every change. Handed to a *line* merge that is nearly the worst possible input
//! — two people drawing in opposite corners of the same canvas touch no common shape, but
//! their edits land in the same pretty-printed array and collide on the punctuation between
//! them. What comes back is either a spurious conflict or, worse, JSON that has been spliced
//! into something Excalidraw cannot parse: the whole board, gone, because two people drew at
//! once.
//!
//! The atom of a scene is the **element**, so merge elements. This is exactly the
//! frontmatter argument one level down: resolve structurally what is structurally
//! resolvable, and never let the text merge see something whose structure we understand.
//!
//! ## The rules, and the one that matters
//!
//! - An element only one side changed takes that side's version.
//! - Both changed it → **higher `version` wins**, ties broken by **lower `versionNonce`**.
//!   That is Excalidraw's own ordering, and it is deterministic on both machines, which is
//!   the property that matters: two people merging the same pair must land on the same
//!   scene or the next sync diverges again.
//! - **A deletion is honoured against the base.** An element present in the base and absent
//!   from one side was deleted by that side, and stays deleted — even if the other side
//!   moved it.
//!
//! That last rule is the whole reason this is not `reconcileElements`, Excalidraw's own
//! merge. `reconcileElements` takes *two* scenes and no base, so it cannot tell "you
//! deleted this" from "I added this" — and resolves both by keeping the element. Wire it
//! into a git merge and every shape either party deleted comes back from the dead on the
//! next sync. A 3-way merge has the base, so it can tell, and does.
//!
//! Losing a *concurrent edit to one shape* is accepted: that is last-writer-wins per
//! element, and it is what `version`/`versionNonce` are for. Losing a *deletion* would not
//! be a lost edit, it would be an undo nobody asked for.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Whether this text is a whiteboard scene we know how to merge structurally.
///
/// Deliberately strict: anything we are not sure about goes to the text merge, which is
/// the behaviour that exists today. A false positive here would mean silently rewriting a
/// note body that is merely JSON-shaped.
pub fn is_scene(text: &str) -> bool {
    parse(text).is_some()
}

fn parse(text: &str) -> Option<Map<String, Value>> {
    let v: Value = serde_json::from_str(text).ok()?;
    let obj = v.as_object()?;
    // Excalidraw stamps its own type; `elements` is the array we merge.
    if obj.get("type")?.as_str()? != "excalidraw" {
        return None;
    }
    obj.get("elements")?.as_array()?;
    Some(obj.clone())
}

/// One element's identity and precedence.
fn id_of(e: &Value) -> Option<&str> {
    e.get("id")?.as_str()
}
fn version_of(e: &Value) -> i64 {
    e.get("version").and_then(Value::as_i64).unwrap_or(0)
}
fn nonce_of(e: &Value) -> i64 {
    e.get("versionNonce").and_then(Value::as_i64).unwrap_or(0)
}

/// Which of two versions of the same element wins. Higher `version`; on a tie, lower
/// `versionNonce`; on a total tie they are the same edit and it does not matter, so take
/// `ours` and stay deterministic.
fn newer<'a>(ours: &'a Value, theirs: &'a Value) -> &'a Value {
    match version_of(ours).cmp(&version_of(theirs)) {
        std::cmp::Ordering::Greater => ours,
        std::cmp::Ordering::Less => theirs,
        std::cmp::Ordering::Equal => {
            if nonce_of(theirs) < nonce_of(ours) {
                theirs
            } else {
                ours
            }
        }
    }
}

fn by_id(elements: &[Value]) -> BTreeMap<String, Value> {
    elements.iter().filter_map(|e| id_of(e).map(|id| (id.to_string(), e.clone()))).collect()
}

/// Merge three scenes element-wise. `None` when any of them is not a scene we understand —
/// the caller then falls back to the text merge, which is what happens today.
pub fn merge_scene(base: &str, ours: &str, theirs: &str) -> Option<String> {
    let (b, o, t) = (parse(base)?, parse(ours)?, parse(theirs)?);
    let (be, oe, te) = (
        b.get("elements")?.as_array()?.clone(),
        o.get("elements")?.as_array()?.clone(),
        t.get("elements")?.as_array()?.clone(),
    );
    let (bm, om, tm) = (by_id(&be), by_id(&oe), by_id(&te));

    let mut merged: Vec<Value> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    // Walk in ours-order first, then append what only theirs has. That keeps a scene that
    // nobody reordered byte-stable for the side that did the most recent work, and it is
    // the fallback ordering when elements carry no fractional index.
    let order = oe
        .iter()
        .filter_map(id_of)
        .chain(te.iter().filter_map(id_of))
        .map(String::from)
        .collect::<Vec<_>>();

    for id in order {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id.clone());
        let (in_base, mine, yours) = (bm.contains_key(&id), om.get(&id), tm.get(&id));
        match (mine, yours) {
            // Both still have it: the newer edit wins.
            (Some(a), Some(bv)) => merged.push(newer(a, bv).clone()),
            // Only one side has it. If the base had it too, the *other* side deleted it —
            // and a deletion is a decision, not an omission. If the base did not, this side
            // added it and it is new work.
            (Some(a), None) => {
                if !in_base {
                    merged.push(a.clone());
                }
            }
            (None, Some(bv)) => {
                if !in_base {
                    merged.push(bv.clone());
                }
            }
            // Deleted by both, or never existed.
            (None, None) => {}
        }
    }

    // Excalidraw orders the canvas by a fractional `index` string ("a0", "a1", "a0V"…), so
    // when every surviving element has one it *is* the z-order and must be respected —
    // otherwise a merge silently restacks the drawing. When they do not (older scenes), the
    // ours-then-theirs walk above is already a deterministic order, so leave it alone.
    if merged.iter().all(|e| e.get("index").and_then(Value::as_str).is_some()) {
        merged.sort_by(|a, b| {
            let k = |e: &Value| e.get("index").and_then(Value::as_str).unwrap_or("").to_string();
            k(a).cmp(&k(b))
        });
    }

    // `appState` is view state — scroll offset, zoom, which tool is selected. It is not
    // content and it is per-person, so there is nothing to merge: keep ours.
    //
    // `files` is the opposite: it is the image bytes the elements point at, keyed by
    // content id. Take the **union**, because dropping the side whose element lost the
    // version race would leave a surviving image element pointing at nothing.
    let mut out = o.clone();
    out.insert("elements".into(), Value::Array(merged));
    if let Some(theirs_files) = t.get("files").and_then(Value::as_object) {
        let mut files = out.get("files").and_then(Value::as_object).cloned().unwrap_or_default();
        for (k, v) in theirs_files {
            files.entry(k.clone()).or_insert_with(|| v.clone());
        }
        out.insert("files".into(), Value::Object(files));
    }

    // Two-space pretty, matching Excalidraw's own `serializeAsJSON` — so a merged scene is
    // byte-comparable with one the app wrote, and the next save is not a spurious diff.
    serde_json::to_string_pretty(&Value::Object(out)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(id: &str, version: i64, nonce: i64, x: i64) -> String {
        format!(
            r#"{{"id":"{id}","version":{version},"versionNonce":{nonce},"type":"rectangle","x":{x}}}"#
        )
    }
    fn scene(elements: &[String]) -> String {
        format!(
            r#"{{"type":"excalidraw","version":2,"source":"fm","elements":[{}],"appState":{{}},"files":{{}}}}"#,
            elements.join(",")
        )
    }
    fn ids(merged: &str) -> Vec<String> {
        let v: Value = serde_json::from_str(merged).unwrap();
        v["elements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect()
    }
    fn find(merged: &str, id: &str) -> Value {
        let v: Value = serde_json::from_str(merged).unwrap();
        v["elements"].as_array().unwrap().iter().find(|e| e["id"] == id).cloned().unwrap()
    }

    #[test]
    fn a_non_scene_body_is_left_to_the_text_merge() {
        assert!(!is_scene("just some prose"));
        assert!(!is_scene(r#"{"type":"something-else","elements":[]}"#));
        assert!(!is_scene(r#"{"type":"excalidraw"}"#), "no elements array");
        assert!(is_scene(&scene(&[])));
    }

    /// The everyday case: two people draw in different corners. Nothing collides.
    #[test]
    fn two_people_adding_different_shapes_both_keep_them() {
        let base = scene(&[el("a", 1, 1, 0)]);
        let ours = scene(&[el("a", 1, 1, 0), el("mine", 1, 5, 10)]);
        let theirs = scene(&[el("a", 1, 1, 0), el("yours", 1, 7, 20)]);

        let merged = merge_scene(&base, &ours, &theirs).unwrap();

        let got = ids(&merged);
        assert!(got.contains(&"a".to_string()));
        assert!(got.contains(&"mine".to_string()));
        assert!(got.contains(&"yours".to_string()));
        assert_eq!(got.len(), 3);
    }

    /// **The rule this module exists for.** `reconcileElements` has no base, so it keeps
    /// the element and the deletion is undone. With a base we can tell, and we honour it.
    #[test]
    fn a_deletion_is_not_resurrected_by_the_other_sides_edit() {
        let base = scene(&[el("doomed", 1, 1, 0), el("keep", 1, 2, 0)]);
        // We deleted it. They moved it (so their copy is *newer*).
        let ours = scene(&[el("keep", 1, 2, 0)]);
        let theirs = scene(&[el("doomed", 99, 3, 500), el("keep", 1, 2, 0)]);

        let merged = merge_scene(&base, &ours, &theirs).unwrap();

        assert_eq!(ids(&merged), vec!["keep"], "a deleted shape must stay deleted");
    }

    #[test]
    fn a_deletion_is_honoured_in_either_direction() {
        let base = scene(&[el("doomed", 1, 1, 0)]);
        let ours = scene(&[el("doomed", 99, 3, 500)]);
        let theirs = scene(&[]);

        let merged = merge_scene(&base, &ours, &theirs).unwrap();

        assert!(ids(&merged).is_empty());
    }

    #[test]
    fn the_higher_version_of_a_concurrently_edited_shape_wins() {
        let base = scene(&[el("a", 1, 1, 0)]);
        let ours = scene(&[el("a", 5, 10, 100)]);
        let theirs = scene(&[el("a", 9, 10, 900)]);

        let merged = merge_scene(&base, &ours, &theirs).unwrap();

        assert_eq!(find(&merged, "a")["x"], 900, "theirs is newer");
    }

    /// The tie-break has to be deterministic, because both machines run this merge
    /// independently and must land on the same scene.
    #[test]
    fn a_version_tie_breaks_on_the_lower_nonce_from_either_side() {
        let base = scene(&[el("a", 1, 1, 0)]);
        let ours = scene(&[el("a", 5, 800, 111)]);
        let theirs = scene(&[el("a", 5, 200, 222)]);

        let forward = merge_scene(&base, &ours, &theirs).unwrap();
        let backward = merge_scene(&base, &theirs, &ours).unwrap();

        assert_eq!(find(&forward, "a")["x"], 222, "lower nonce wins");
        assert_eq!(
            find(&backward, "a")["x"],
            222,
            "and wins the same way whichever side is 'ours'"
        );
    }

    /// z-order is a fractional index string. A merge that ignored it would silently
    /// restack the drawing.
    #[test]
    fn fractional_indices_decide_the_z_order() {
        let mk = |id: &str, idx: &str| {
            format!(r#"{{"id":"{id}","version":1,"versionNonce":1,"index":"{idx}"}}"#)
        };
        let base = scene(&[mk("a", "a0")]);
        let ours = scene(&[mk("a", "a0"), mk("top", "a9")]);
        let theirs = scene(&[mk("a", "a0"), mk("mid", "a5")]);

        let merged = merge_scene(&base, &ours, &theirs).unwrap();

        assert_eq!(ids(&merged), vec!["a", "mid", "top"], "sorted by index, not arrival");
    }

    /// An element that survives must not end up pointing at image bytes the merge threw
    /// away, so the file maps unite.
    #[test]
    fn image_files_from_both_sides_survive() {
        let with_files = |id: &str, file: &str| {
            format!(
                r#"{{"type":"excalidraw","version":2,"source":"fm","elements":[{}],"appState":{{}},"files":{{"{file}":{{"mimeType":"image/png","id":"{file}"}}}}}}"#,
                el(id, 1, 1, 0)
            )
        };
        let base = scene(&[]);
        let ours = with_files("mine", "f1");
        let theirs = with_files("yours", "f2");

        let merged = merge_scene(&base, &ours, &theirs).unwrap();

        let v: Value = serde_json::from_str(&merged).unwrap();
        assert!(v["files"]["f1"].is_object(), "ours kept");
        assert!(v["files"]["f2"].is_object(), "theirs kept too");
    }

    /// Nobody drew anything: the merge must be a no-op, not a reformat.
    #[test]
    fn an_unchanged_scene_round_trips() {
        let s = scene(&[el("a", 1, 1, 0)]);
        let merged = merge_scene(&s, &s, &s).unwrap();
        assert_eq!(ids(&merged), vec!["a"]);
    }
}
