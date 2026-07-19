//! A frontmatter-aware 3-way merge for notes — the thing that makes a shared vault
//! survivable rather than a thing you recover from.
//!
//! **The conflict this exists to kill is one we manufacture ourselves.** `updated:` is
//! rewritten on every save, so *any* two concurrent edits to one note collide on that
//! line — even when the two people touched entirely different paragraphs. Git's markers
//! then land inside the YAML fence, where [`crate::frontmatter::from_file`] rightly
//! refuses them, and the note drops out of the index. Every concurrent edit, by
//! construction. Nothing about that is the user's fault and nothing about it is
//! interesting to them.
//!
//! So: resolve structurally what is mechanically resolvable — `updated` is the later of
//! the two, `id`/`created` never move, `tags` unite, a field only one side touched takes
//! that side's value — and hand the body to git, which has done 3-way text merges
//! properly for twenty years. Shelling out is the whole point: there is no diff3 here to
//! get wrong.
//!
//! **One body is not prose, and gets the same treatment one level down.** A `view: board`
//! note's body is an Excalidraw scene — a single pretty-printed JSON array that the app
//! rewrites whole on every change, which a line merge mangles into either a spurious
//! conflict or unparseable JSON. Its atom is the *element*, so [`crate::scene`] merges
//! elements and [`merge_body`] tries that first. Same rule as above, applied recursively:
//! never hand the text merge something whose structure we understand.
//!
//! **The property worth stating:** when this driver conflicts, it conflicts *in the
//! body*. Frontmatter is always emitted whole and valid, so a conflicted note still
//! parses, still indexes, and still opens in the editor with the markers sitting in the
//! textarea where a human can see them. That is what turns "the app lost my note" into
//! "there are two paragraphs here, pick one" — and it is why the conflict-surfacing UI
//! can exist at all.
//!
//! Only a genuinely divergent *field* (both sides set `status` differently) falls back
//! to a whole-file `git merge-file`, marking the file conflicted exactly as git would
//! today. We never resolve that by fiat: silently dropping one side's status change is
//! the same data loss this whole phase exists to stop.

use crate::frontmatter;
use crate::StoreError;
use fm_model::Object;
use std::path::Path;
use std::process::Command;

/// What a merge did, from the caller's point of view. `Conflicted` still writes a
/// result — git's contract is that the driver leaves its best effort in `ours` and
/// signals with its exit code — the result just has markers in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merged {
    Clean,
    Conflicted,
}

/// Git's merge-driver entry point: read the three versions, write the result back over
/// `ours` (that is `%A`, which git takes as the answer), and report whether it is clean.
///
/// Anything we cannot understand as a note — either side unparseable, which is what a
/// *previous* bad merge leaves behind — falls back to a plain whole-file merge. The
/// driver must never be the reason a merge cannot happen at all.
pub fn merge_files(base: &Path, ours: &Path, theirs: &Path, marker_size: usize) -> Result<Merged, StoreError> {
    let (b, o, t) = (read(base)?, read(ours)?, read(theirs)?);

    let parsed = (frontmatter::from_file(&b), frontmatter::from_file(&o), frontmatter::from_file(&t));
    let (Ok(bo), Ok(oo), Ok(to)) = parsed else {
        return whole_file(base, ours, theirs, marker_size);
    };

    let Some(mut merged) = merge_objects(&bo, &oo, &to) else {
        // A real disagreement about a field's value. Not ours to resolve.
        return whole_file(base, ours, theirs, marker_size);
    };

    let (body, outcome) = merge_body(&bo.body, &oo.body, &to.body, marker_size)?;
    merged.body = body;

    let text = frontmatter::to_file(&merged).map_err(|e| StoreError::Parse(e.to_string()))?;
    std::fs::write(ours, text).map_err(io)?;
    Ok(outcome)
}

/// Merge everything except the body. `None` means the two sides disagree about a field
/// in a way no rule can settle — the caller falls back rather than picking a winner.
fn merge_objects(base: &Object, ours: &Object, theirs: &Object) -> Option<Object> {
    let mut m = ours.clone();

    // Identity never merges: a note is the file, and `id`/`created` are what say so.
    // If the two sides disagree about *which note this is*, this is not a merge.
    if ours.id != theirs.id {
        return None;
    }
    m.id = base.id;
    m.created = base.created;

    // The manufactured conflict, resolved: `updated` is a clock, and the later reading
    // is simply the true one. Never a conflict, whatever the two sides say.
    m.updated = ours.updated.max(theirs.updated);

    // Tags are a set, and adding one is never an act of removing another — so two
    // people tagging the same note concurrently both get their way. (The honest cost:
    // a tag one side *deleted* while the other kept it comes back. Union is still
    // right: re-deleting a tag is trivial, and losing someone's tag silently is not.)
    m.tags = union(&ours.tags, &theirs.tags);
    m.assets = union(&ours.assets, &theirs.assets);
    m.code = union(&ours.code, &theirs.code);

    m.kind = three_way(&base.kind, &ours.kind, &theirs.kind)?;
    m.title = three_way(&base.title, &ours.title, &theirs.title)?;
    m.status = three_way(&base.status, &ours.status, &theirs.status)?;
    m.due = three_way(&base.due, &ours.due, &theirs.due)?;
    m.start = three_way(&base.start, &ours.start, &theirs.start)?;
    m.hard = three_way(&base.hard, &ours.hard, &theirs.hard)?;

    // Custom properties, by the same rule — including the ones only one side has.
    m.extra.clear();
    for key in ours.extra.keys().chain(theirs.extra.keys()) {
        let (b, o, t) = (base.extra.get(key), ours.extra.get(key), theirs.extra.get(key));
        match three_way(&b.cloned(), &o.cloned(), &t.cloned())? {
            Some(v) => {
                m.extra.insert(key.clone(), v);
            }
            // Both sides removed it, or it was never there.
            None => {}
        }
    }
    Some(m)
}

/// The ordinary 3-way rule for a single value: whoever changed it wins, and if both
/// changed it to the same thing there was never a disagreement. `None` when both moved
/// it somewhere different — the one case a merge cannot invent an answer for.
fn three_way<T: PartialEq + Clone>(base: &T, ours: &T, theirs: &T) -> Option<T> {
    if ours == theirs {
        return Some(ours.clone());
    }
    if ours == base {
        return Some(theirs.clone()); // we didn't touch it; they did
    }
    if theirs == base {
        return Some(ours.clone()); // they didn't touch it; we did
    }
    None
}

/// Order-preserving union: ours first, then whatever they added.
fn union(ours: &[String], theirs: &[String]) -> Vec<String> {
    let mut out = ours.to_vec();
    for t in theirs {
        if !out.contains(t) {
            out.push(t.clone());
        }
    }
    out
}

/// Hand the three bodies to `git merge-file`, which is the 3-way text merge everyone
/// already has and nobody should write twice. Returns the merged text and whether it
/// came back clean; on conflict the markers are in the returned body, and *only* in the
/// body, which is what keeps the note parseable.
fn merge_body(
    base: &str,
    ours: &str,
    theirs: &str,
    marker_size: usize,
) -> Result<(String, Merged), StoreError> {
    // Fast path, and the common one for a metadata-only edit: nobody touched the prose.
    if ours == theirs {
        return Ok((ours.to_string(), Merged::Clean));
    }
    if ours == base {
        return Ok((theirs.to_string(), Merged::Clean));
    }
    if theirs == base {
        return Ok((ours.to_string(), Merged::Clean));
    }

    // A whiteboard's body is not prose, it is a scene — one big JSON array that the app
    // re-serializes whole on every change. A line merge is close to the worst tool for it:
    // two people drawing in opposite corners share no shape but do share the punctuation
    // between them, so it returns either a spurious conflict or spliced JSON that
    // Excalidraw cannot parse — the entire board lost because two people drew at once.
    //
    // The atom there is the element, so merge elements. Same argument as the frontmatter
    // one directly above, one level down: never hand the text merge something whose
    // structure we understand. Anything we cannot parse as a scene falls straight through.
    if let Some(merged) = crate::scene::merge_scene(base, ours, theirs) {
        return Ok((merged, Merged::Clean));
    }

    // git merge-file works on paths, so the bodies have to land somewhere. **Not** in
    // the repo: the driver's working directory is the vault, and the auto-commit's
    // `git add -A` fires every 5s — a scratch file living there for the length of a
    // merge is a scratch file that can end up in someone's history.
    // The name has to be unique per *call*, not per process. As the `fm merge-md` driver
    // that is the same thing — one merge per one-shot subprocess — but the app is meant to
    // call this in-process (a phone has no driver to invoke), and a threaded server merging
    // two notes at once would otherwise have both writes race over the same three paths and
    // hand one note a body assembled from the other. Clean exit, wrong content, no error.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir();
    let stamp = std::process::id();
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let paths: Vec<_> = ["ours", "base", "theirs"]
        .iter()
        .map(|n| dir.join(format!("fm-merge-{stamp}-{seq}-{n}")))
        .collect();
    for (path, text) in paths.iter().zip([ours, base, theirs]) {
        std::fs::write(path, text).map_err(io)?;
    }

    // `-L` because the labels are what the *user* reads in the conflict markers, and
    // without them git names the scratch files above — leaking `fm-merge-31337-ours`
    // into someone's note. "ours"/"theirs" is git's own vocabulary for a merge.
    let out = Command::new("git")
        .arg("merge-file")
        .arg("-p")
        .arg(format!("--marker-size={marker_size}"))
        .args(["-L", "ours", "-L", "base", "-L", "theirs"])
        .args(&paths)
        .output();
    for path in &paths {
        let _ = std::fs::remove_file(path);
    }
    let out = out.map_err(|e| StoreError::Io(format!("could not run git merge-file: {e}")))?;

    // Exit code is the number of conflicts; negative (>128 as u8) means it failed
    // outright, and then we have no merged text to trust.
    match out.status.code() {
        Some(0) => Ok((String::from_utf8_lossy(&out.stdout).into_owned(), Merged::Clean)),
        Some(n) if n > 0 && n < 128 => {
            Ok((String::from_utf8_lossy(&out.stdout).into_owned(), Merged::Conflicted))
        }
        _ => Err(StoreError::Io(format!(
            "git merge-file failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
    }
}

/// What git would have done without us. The fallback for a note we cannot read as a
/// note, and for a field disagreement no rule settles.
fn whole_file(base: &Path, ours: &Path, theirs: &Path, marker_size: usize) -> Result<Merged, StoreError> {
    let out = Command::new("git")
        .arg("merge-file")
        .arg(format!("--marker-size={marker_size}"))
        .arg(ours)
        .arg(base)
        .arg(theirs)
        .output()
        .map_err(|e| StoreError::Io(format!("could not run git merge-file: {e}")))?;
    match out.status.code() {
        Some(0) => Ok(Merged::Clean),
        Some(n) if n > 0 && n < 128 => Ok(Merged::Conflicted),
        _ => Err(StoreError::Io(format!(
            "git merge-file failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
    }
}

fn read(path: &Path) -> Result<String, StoreError> {
    std::fs::read_to_string(path).map_err(io)
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}
