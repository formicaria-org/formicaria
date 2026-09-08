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
//! properly for twenty years. Not writing a diff3 ourselves is the whole point: where there
//! is a `git` binary we shell out to it, and where there is not — a phone — we call libgit2's
//! own port of the same algorithm. Two engines, held byte-identical by
//! `tests/merge_differential.rs`; never a third implementation of our own.
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
//! **A genuinely divergent *field* — both sides set `status` differently — keeps both, by
//! demoting the loser into a `conflict-<field>` key beside it** (`decisions.md`, 2026-09-07).
//! Until then it dropped the whole file into a text merge, which put markers *inside* the YAML
//! fence and made the note vanish from every view; that was ruled loud-and-absent on 2026-07-19
//! and reversed once it became clear how quiet "absent" actually is — a phone sat on two of them
//! for 39 days. This is still not resolution by fiat, and the distinction is the whole point:
//! fiat picks between two pieces of content by **destroying one**, and nothing here is destroyed.
//! `status: done` with `conflict-status: [doing]` beside it has lost nothing, parses, indexes and
//! commits. The rule that keeps the two devices agreeing is that the winner is chosen from
//! **content only** — later `updated`, then `Ord` — so both compute the same file and the merge is
//! a fixed point.

use crate::frontmatter;
use crate::StoreError;
use fm_model::{Object, PropertyValue};
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

/// True when this text still carries git conflict markers.
///
/// **One definition, because two would eventually disagree about what "resolved" means.** It is used
/// by the list of conflicted notes (`fm_app::commands::conflicts`) *and* by the guard that refuses to
/// mark a conflict resolved while markers remain — and those two must never differ: a list that says
/// a note is clean while the guard refuses it (or worse, the reverse) is how `<<<<<<<` gets committed
/// as a note's content and pushed to a collaborator.
///
/// Requires **both** an opening and a closing marker, so a note that merely *writes about* merge
/// conflicts is not flagged. That tolerance is deliberate and load-bearing: this app's own notes
/// discuss merges.
pub fn has_conflict_markers(text: &str) -> bool {
    let mut opened = false;
    let mut closed = false;
    for line in text.lines() {
        if line.starts_with("<<<<<<<") {
            opened = true;
        } else if line.starts_with(">>>>>>>") {
            closed = true;
        }
    }
    opened && closed
}

/// Git's merge-driver entry point: read the three versions, write the result back over
/// `ours` (that is `%A`, which git takes as the answer), and report whether it is clean.
///
/// **This function is the driver ABI and nothing else.** Every decision lives in
/// [`merge_texts`], which takes and returns text. That split is not tidiness: a phone has no
/// merge driver for git to invoke, so the app must be able to run the *same* merge over three
/// strings it pulled out of an object database itself. Keeping the `%O %A %B` path shape here
/// and the logic there is what stops a phone and a desktop merging the same note differently
/// — the failure the `git2` rejection was written to avoid (`docs/context/decisions.md`).
pub fn merge_files(
    base: &Path,
    ours: &Path,
    theirs: &Path,
    marker_size: usize,
) -> Result<Merged, StoreError> {
    let (b, o, t) = (read(base)?, read(ours)?, read(theirs)?);
    let (text, outcome) = merge_texts(&b, &o, &t, marker_size)?;
    std::fs::write(ours, text).map_err(io)?;
    Ok(outcome)
}

/// Merge three versions of a note, as text in and text out. **The one engine**, callable
/// from a driver, from an in-process pull, or from a platform with no `git` binary at all.
///
/// Anything we cannot understand as a note — either side unparseable, which is exactly what a
/// *previous* bad merge leaves behind — falls back to a plain whole-file text merge. Being
/// unable to read a note must never be the reason a merge cannot happen.
pub fn merge_texts(
    base: &str,
    ours: &str,
    theirs: &str,
    marker_size: usize,
) -> Result<(String, Merged), StoreError> {
    let parsed = (
        frontmatter::from_file(base),
        frontmatter::from_file(ours),
        frontmatter::from_file(theirs),
    );
    let (Ok(bo), Ok(oo), Ok(to)) = parsed else {
        return text_3way(base, ours, theirs, marker_size);
    };

    let Some(mut merged) = merge_objects(&bo, &oo, &to) else {
        // The two sides are not the same note — different `id`s under one path. Nothing about
        // that is a merge, so it goes to the text engine and comes back with markers, which is
        // the honest answer. A *field* disagreement no longer reaches here: it is kept beside
        // the winner (`DEMOTED_PREFIX`), so the fence stays valid and the note stays readable.
        return text_3way(base, ours, theirs, marker_size);
    };

    let (body, outcome) = merge_body(&bo.body, &oo.body, &to.body, marker_size)?;
    merged.body = body;

    let text = frontmatter::to_file(&merged).map_err(|e| StoreError::Parse(e.to_string()))?;
    Ok((text, outcome))
}

/// The reserved frontmatter prefix that holds a value the merge demoted: `conflict-status` is
/// what the *other* device said `status` was, when the two disagreed and no rule could settle it.
///
/// **Public because it is a contract, not an implementation detail.** The UI lists these, a query
/// can group by one, and `ci/checks.sh` has no way to notice a second spelling appearing somewhere
/// else. One constant is what stops the app and the merge disagreeing about which keys are ours.
pub const DEMOTED_PREFIX: &str = "conflict-";

/// Merge everything except the body. `None` only when the two sides are not the same note at all —
/// every field-level disagreement is settled here, never escalated to the caller.
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

    // Every field that could not be settled by the three-way rule, as `field -> the loser`.
    // Collected rather than written straight into `m.extra`, because the winner has to exist
    // before a demoted value can be compared against it (see the end of this function).
    let mut d = Demote { ours, theirs, out: Vec::new() };

    m.kind = d.settle("type", &base.kind, &ours.kind, &theirs.kind);
    m.title = d.settle("title", &base.title, &ours.title, &theirs.title);
    m.status = d.settle("status", &base.status, &ours.status, &theirs.status);
    m.due = d.settle("due", &base.due, &ours.due, &theirs.due);
    m.start = d.settle("start", &base.start, &ours.start, &theirs.start);
    m.hard = d.settle("hard", &base.hard, &ours.hard, &theirs.hard);

    // Custom properties, by the same rule — including the ones only one side has.
    m.extra.clear();
    for key in ours.extra.keys().chain(theirs.extra.keys()) {
        // The record of a disagreement is a set, and merges as one — see `merge_demoted` below.
        // Running the scalar rule over it would let a divergence *in the record* demote itself
        // into `conflict-conflict-status`, unboundedly.
        if key.starts_with(DEMOTED_PREFIX) {
            continue;
        }
        let (b, o, t) = (base.extra.get(key), ours.extra.get(key), theirs.extra.get(key));
        // `clippy::single_match` wants an `if let` here. Kept as a `match` so the `None` arm has
        // somewhere to say what it means: dropping a key is a *decision* the three-way merge made,
        // not an absence of one, and an `if let` leaves nowhere to write that down.
        #[allow(clippy::single_match)]
        match d.settle(key, &b.cloned(), &o.cloned(), &t.cloned()) {
            Some(v) => {
                m.extra.insert(key.clone(), v);
            }
            // Both sides removed it, or it was never there.
            None => {}
        }
    }

    merge_demoted(&mut m, base, ours, theirs, d.out);
    Some(m)
}

/// The two sides, plus the losers collected so far. It exists so [`Demote::settle`] can stay
/// generic over each field's own type while still reading that same field out of both objects as a
/// `PropertyValue` — which is what a demoted value has to be, since it lands in `extra`.
struct Demote<'a> {
    ours: &'a Object,
    theirs: &'a Object,
    out: Vec<(String, PropertyValue)>,
}

impl Demote<'_> {
    /// The three-way rule, and — when it cannot answer — a winner plus a record of the loser.
    ///
    /// `key` is the **frontmatter** spelling of the field: `type`, not `kind`, because it becomes
    /// the second half of a key a user reads and types.
    ///
    /// **The winner is computed from content alone.** Later `updated` first, then `PropertyValue`'s
    /// own `Ord` to break a tie. Neither half can see which side is "ours", which is the property
    /// that matters: the same two commits merged on two devices produce the same winner, the same
    /// loser and the same bytes, so the merge is a fixed point and nothing is re-derived on the next
    /// pull. A rule that preferred "ours" would look identical in a single-device test and would
    /// leave the two devices trading the same edit forever.
    fn settle<T: PartialEq + Clone>(&mut self, key: &str, base: &T, ours: &T, theirs: &T) -> T {
        if let Some(v) = three_way(base, ours, theirs) {
            return v;
        }
        let (o, t) = (self.ours.get(key), self.theirs.get(key));
        let theirs_wins = (self.theirs.updated, &t) > (self.ours.updated, &o);
        let (winner, loser) = if theirs_wins { (theirs.clone(), o) } else { (ours.clone(), t) };
        self.out.push((format!("{DEMOTED_PREFIX}{key}"), crate::frontmatter::as_written(&loser)));
        winner
    }
}

/// Fold every demoted value — the ones already in either side's file, and the ones this merge just
/// produced — into `m.extra` as one sorted set per field.
///
/// **A set, merged as a set**, by the rule that governs `tags` and `manifest.json`: the key is the
/// content, so agreement is structural and duplication is idempotent. Running the scalar
/// [`three_way`] over these would be a category error that bites twice — a divergence *in the record
/// of a divergence* would demote itself into `conflict-conflict-status`, unboundedly.
///
/// **But a plain union would make the key immortal, and the key is the user's undo.** Removing a
/// `conflict-status` line is how someone says "I have looked at this". Under a union that removal is
/// re-added by the first pull from a device that has not looked yet, and then re-added by that
/// device pulling back — for ever, with no way out. So it is the three-way *set* merge: an element
/// survives if either side still has it and **neither side that had it in the base deleted it**.
/// That is the one place this deliberately differs from `tags`, where the cost of resurrection is a
/// tag you re-delete in a second rather than a warning you cannot dismiss.
///
/// **Sorted**, because ours-then-theirs order is the last remaining thing that would differ between
/// two devices merging the same pair of commits, and a merge that is not byte-identical on both
/// sides is not a fixed point.
///
/// A demoted value equal to the winning one is dropped: that is agreement, and leaving it would show
/// a disagreement that no longer exists. That rule is also what makes *promoting* the loser stick —
/// once the user sets the field to it, it stops being a loser everywhere, on every device.
fn merge_demoted(
    m: &mut Object,
    base: &Object,
    ours: &Object,
    theirs: &Object,
    fresh: Vec<(String, PropertyValue)>,
) {
    let mut sets: std::collections::BTreeMap<String, Vec<PropertyValue>> = Default::default();
    let keys = base.extra.keys().chain(ours.extra.keys()).chain(theirs.extra.keys());
    for key in keys.filter(|k| k.starts_with(DEMOTED_PREFIX)).cloned().collect::<Vec<_>>() {
        let (b, o, t) = (set_at(base, &key), set_at(ours, &key), set_at(theirs, &key));
        let kept = o
            .iter()
            .chain(t.iter())
            .filter(|v| {
                // Deleted by a side that had it: that side has seen it, and said so.
                let deleted_by_us = b.contains(v) && !o.contains(v);
                let deleted_by_them = b.contains(v) && !t.contains(v);
                !deleted_by_us && !deleted_by_them
            })
            .cloned()
            .collect();
        sets.insert(key, kept);
    }
    // A value this very merge demoted was never in the base, so it cannot have been "deleted".
    for (k, v) in fresh {
        sets.entry(k).or_default().push(v);
    }

    for (key, mut losers) in sets {
        let field = key.strip_prefix(DEMOTED_PREFIX).unwrap_or(&key);
        let winner = crate::frontmatter::as_written(&m.get(field));
        losers.retain(|v| *v != winner);
        losers.sort();
        losers.dedup();
        if losers.is_empty() {
            m.extra.remove(&key);
        } else {
            m.extra.insert(key, PropertyValue::List(losers));
        }
    }
}

/// One side's demoted set for one key. A hand-typed `conflict-status: doing` is a set of one —
/// tolerated rather than ignored, because this is a plain-text file and a human is allowed to have
/// edited it.
fn set_at(o: &Object, key: &str) -> Vec<PropertyValue> {
    match o.extra.get(key) {
        Some(PropertyValue::List(items)) => items.clone(),
        Some(scalar) => vec![scalar.clone()],
        None => Vec::new(),
    }
}

/// The ordinary 3-way rule for a single value: whoever changed it wins, and if both
/// changed it to the same thing there was never a disagreement. `None` when both moved
/// it somewhere different — the one case a merge cannot invent an answer for, and the one
/// [`settle`] answers by keeping both.
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

    let (text, verdict) = text_3way(base, ours, theirs, marker_size)?;

    // A line merge asks "did you both change this line?", and a Markdown paragraph is one
    // line — so two devices editing two different sentences of it collide over prose neither
    // of them touched. Measured over this repo's own vault (2026-09-08): 26% of body lines
    // carry more than one sentence, and **30% of the collisions two edits can have on a line
    // are between sentences far enough apart for the merge to settle them** — one prose
    // conflict in three, none of which is a disagreement. Sentences that *touch* still
    // conflict, and deliberately: see `tests/sentence_merge.rs`.
    //
    // **A rescue, never a policy.** The line merge above has already run and its answer stands
    // unless it conflicted, so nothing that merges cleanly today can change — the finer pass
    // can only turn a conflict into a clean merge, never the reverse, and that is a property of
    // the control flow rather than of a test. Its result is taken only when it comes back
    // clean, so markers are always the line merge's, in the shape the app and the user already
    // know.
    //
    // This is not a third merge engine, which this module forbids by name: it is the same
    // `text_3way`, handed the same prose cut at a smaller seam.
    if verdict == Merged::Conflicted {
        if let Some(finer) = sentence_merge(base, ours, theirs, marker_size) {
            return Ok(finer);
        }
    }
    Ok((text, verdict))
}

/// Re-run the text merge with a **sentence** as the unit instead of a line, and hand back the
/// result only if it is clean. `None` when there was nothing finer to try or it conflicted too.
///
/// The transform is exactly reversible — [`join_sentences`] undoes [`split_sentences`] byte for
/// byte — which is what makes it safe on the one path that must never corrupt: a clean merge is
/// a concatenation of whole units taken from the three inputs, so decoding it is the same
/// operation as decoding an input.
fn sentence_merge(
    base: &str,
    ours: &str,
    theirs: &str,
    marker_size: usize,
) -> Option<(String, Merged)> {
    let (b, o, t) = (split_sentences(base), split_sentences(ours), split_sentences(theirs));
    // `split_sentences` adds exactly one byte per cut, so equal lengths mean no line held a
    // second sentence and the finer pass is the same merge that just failed.
    if b.len() == base.len() && o.len() == ours.len() && t.len() == theirs.len() {
        return None;
    }
    let (merged, verdict) = text_3way(&b, &o, &t, marker_size).ok()?;
    Some(match verdict {
        // Clean: the exact inverse, so the paragraph comes back as the one line it was.
        Merged::Clean => (join_sentences(&merged), Merged::Clean),
        // Conflicted: the same join, except that a marker line keeps its whole line to itself —
        // and it may decline, in which case the line merge's answer stands untouched.
        Merged::Conflicted => (join_conflicted(&merged, marker_size)?, Merged::Conflicted),
    })
}

/// Put each sentence on its own line, so the text merge treats it as its own unit.
///
/// **The cut is `[.!?]` + at least one space, and only where at least two word characters come
/// before the punctuation.** That last clause is what keeps `1. ` — a Markdown ordered-list
/// marker — and `e.g. ` from becoming units of their own: tiny units that repeat across a
/// document are exactly what makes a diff align two unrelated places.
///
/// The spaces stay with the sentence that ends, which is what makes the split reversible: a line
/// of the output ends in punctuation-then-spaces **only** when this function cut there, because
/// a line that genuinely ended that way would have been cut at the same point and left the
/// newline to the empty unit after it.
fn split_sentences(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len() + text.len() / 32);
    let mut cut = 0;
    // Where the current *output* line starts, in source coordinates: after the last newline we
    // passed, or the last cut we made, whichever is later. `structured` is asked about this and
    // nothing else, because a line is exactly what the joiner gets to look at.
    let mut line = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\n' {
            line = i + 1;
            i += 1;
            continue;
        }
        if matches!(b[i], b'.' | b'!' | b'?') && words_before(b, i) >= 2 && !structured(&b[line..])
        {
            let mut j = i + 1;
            while j < b.len() && b[j] == b' ' {
                j += 1;
            }
            if j > i + 1 {
                // Only ASCII is ever matched above, so every index here is a char boundary.
                out.push_str(&text[cut..j]);
                out.push('\n');
                cut = j;
                line = j;
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&text[cut..]);
    out
}

/// Where a marker line sits in a conflicted merge, found by **scanning** rather than by asking each
/// line about itself. `A rule follows. =======` is a sentence somebody wrote; the same seven `=`
/// are a separator only *between* an opening marker and a closing one, and only git puts them
/// there. Testing lines in isolation reads the first as the second and tears a paragraph in half
/// around content nobody edited.
#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Open,
    Sep,
    Close,
}

fn marks(lines: &[&str], marker_size: usize) -> Vec<Option<Mark>> {
    let run = |l: &str, c: u8| {
        let b = l.as_bytes();
        b.len() >= marker_size
            && b[..marker_size].iter().all(|&x| x == c)
            && (b.len() == marker_size || b[marker_size] == b' ')
    };
    let mut out = vec![None; lines.len()];
    let mut open = false;
    for (i, l) in lines.iter().enumerate() {
        if !open && run(l, b'<') {
            out[i] = Some(Mark::Open);
            open = true;
        } else if open && run(l, b'=') {
            out[i] = Some(Mark::Sep);
        } else if open && run(l, b'>') {
            out[i] = Some(Mark::Close);
            open = false;
        }
    }
    out
}

/// Undo [`split_sentences`] for a merge that came back **conflicted** — or decline, and let the
/// line merge's answer stand.
///
/// **`None` is a first-class answer here, and it is what keeps this change honest.** Narrower
/// markers are an improvement offered only where it is provably free; everywhere else the caller
/// falls back to the whole-paragraph markers that shipped before, byte for byte. So the guarantee
/// stays what §2.13's was — the finer pass can improve a conflict or leave it exactly alone, and
/// there is no third outcome — even though the conflicted output is now used.
///
/// Nothing is ever glued onto the front of a marker line: a marker that does not start its own line
/// is not a marker, and `has_conflict_markers`, the guard that refuses to commit marked-up text and
/// the user would all miss it.
///
/// **Three repairs, because a marker's own line break has to replace what the cut was carrying.**
/// A tear line keeps its newline and gives up its trailing spaces — two of them at a line end is a
/// Markdown hard break, and the sentence spacing they came from is what the newline now supplies.
/// An empty line **immediately after** a marker contributes nothing: it is the source's own line
/// ending, and the closing marker has already ended the line, so keeping both invents a blank line
/// — which splits one paragraph into two, permanently, and compounds, because every line this
/// transform tears ends in `". "` and so triggers it on the *next* conflict.
///
/// **A separate function from [`join_sentences`], not a flag.** That one is the exact inverse of the
/// split and must stay so: a body may legitimately contain a line of `=` characters and the clean
/// path must not read it as a marker. Here exactness is not required, because the text is already
/// being restructured around a conflict.
fn join_conflicted(text: &str, marker_size: usize) -> Option<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let m = marks(&lines, marker_size);
    if !narrowing_is_safe(&lines, &m, text) {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        let last = i + 1 == lines.len();
        // The source's own line ending, already supplied by the marker above it.
        if !last && line.is_empty() && i > 0 && m[i - 1].is_some() {
            continue;
        }
        let torn = !last && m[i + 1].is_some() && ends_where_split_cuts(line);
        out.push_str(if torn { line.trim_end_matches(' ') } else { line });
        if last {
            break;
        }
        if torn || !ends_where_split_cuts(line) {
            out.push('\n');
        }
    }
    Some(out)
}

/// Whether the narrower markers can be handed over without restructuring the document around them.
///
/// Each rule is a shape somebody found by trying to break this, and every one of them survives the
/// user resolving the conflict — which is what makes them worth a fallback rather than a comment.
fn narrowing_is_safe(lines: &[&str], m: &[Option<Mark>], text: &str) -> bool {
    if !m.iter().any(|x| x.is_some()) {
        return false;
    }
    // A body whose line endings are CRLF would come back with bare-LF tears among them. Mixed
    // endings are byte churn on the next Windows checkout, and this is not the change that
    // relitigates `eol=lf`.
    if text.contains('\r') {
        return false;
    }
    let mut fenced = false;
    for (i, line) in lines.iter().enumerate() {
        if m[i].is_some() {
            // A fenced code line torn across a marker is invalid code, and stays invalid after the
            // conflict is resolved. `structured` cannot see a fence — it may only read one line —
            // so the whole-output scan is where this is caught. The line merge hands the user two
            // intact candidate lines instead, which is the better answer for code.
            if fenced {
                return false;
            }
            // A sentence that begins `# `, `- ` or `> ` is prose in the middle of a paragraph and a
            // heading, list item or quote at the start of a line.
            if lines.get(i + 1).is_some_and(|l| starts_a_block(l)) {
                return false;
            }
            continue;
        }
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fenced = !fenced;
        }
    }
    true
}

/// A line that Markdown reads as opening a block rather than continuing a paragraph. `#tag` is
/// prose — the space is what makes a heading — which is why every case here requires one.
fn starts_a_block(line: &str) -> bool {
    let t = line.trim_start();
    let b = t.as_bytes();
    matches!(b.first(), Some(b'#' | b'-' | b'+' | b'*' | b'>')) && matches!(b.get(1), Some(b' '))
        || t.split_once(". ")
            .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
}

/// Undo [`split_sentences`]: drop the newline after any line this module cut, keep every other.
fn join_sentences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(k) = rest.find('\n') {
        let line = &rest[..k];
        out.push_str(line);
        if !ends_where_split_cuts(line) {
            out.push('\n');
        }
        rest = &rest[k + 1..];
    }
    out.push_str(rest);
    out
}

/// The decision in [`split_sentences`], asked of a finished line. The two must agree exactly or
/// the transform stops being reversible, so they are written to be read side by side.
fn ends_where_split_cuts(line: &str) -> bool {
    let b = line.as_bytes();
    if structured(b) {
        return false;
    }
    let mut k = b.len();
    while k > 0 && b[k - 1] == b' ' {
        k -= 1;
    }
    // No trailing space means no cut: the split always carries its spaces with it.
    k != b.len() && k > 0 && matches!(b[k - 1], b'.' | b'!' | b'?') && words_before(b, k - 1) >= 2
}

/// A line the split leaves whole: an indented code block, or a Markdown table row. Breaking either
/// across a conflict marker turns a thing with structure into a thing without one — a table stops
/// being a table — and unlike a paragraph, rejoining it is not something the renderer does for you.
///
/// **Decidable from the line's own first bytes, and that is the whole design constraint.** The
/// splitter sees three inputs and the joiner sees one merged output; a guard that depended on
/// context (*"am I inside a ``` fence?"*) could reach opposite verdicts on the two sides and stop
/// them being inverses. Asking only about the bytes in front of it cannot. **The cost, stated:** a
/// *fenced* code line is not recognisable this way, so it is not guarded — see the ruling.
///
/// It is asked about a **line**, never about a source line. `Intro. |ab. cd.` cuts once, and the
/// `|ab. cd.` left behind is a table row as far as the joiner can tell — so the splitter has to ask
/// the same question at the same place, which is the start of each *unit*, not of each source line.
fn structured(line: &[u8]) -> bool {
    // A short line's fourth byte is a newline, never a space, so this cannot read past its end.
    line.starts_with(b"    ") || matches!(line.first(), Some(b'\t') | Some(b'|'))
}

/// How many word characters run backwards from `i`, counting no further than two — which is all
/// either caller asks.
fn words_before(b: &[u8], i: usize) -> usize {
    let mut n = 0;
    let mut k = i;
    while k > 0 && b[k - 1].is_ascii_alphanumeric() && n < 2 {
        n += 1;
        k -= 1;
    }
    n
}

/// The 3-way text merge itself, in whichever engine this device can actually run.
///
/// Serves two callers: [`merge_body`] for a note's prose, and [`merge_texts`] for the
/// whole-file fallback when a note cannot be read as a note. One engine for both, so the
/// fallback cannot drift away from the ordinary path.
///
/// **The selection rule is [`crate::vcs`]'s, for [`crate::vcs`]'s reasons.** A real `git`
/// binary wins; libgit2 is the fallback. The desktop therefore keeps shelling out — which is
/// what keeps the `fm merge-md` driver and the app's own merge the *same* engine, so a
/// collaborator's terminal `git pull` cannot disagree with ours — and a device with no binary
/// gets the same merge out of the library it already links.
///
/// **This was the last thing in the merge that shelled out, and the phone had no answer for
/// it.** Until 2026-09-02 `git_native::pull` reached here on any note whose prose had diverged
/// and failed with `could not run git merge-file` — mid-merge, leaving a `MERGE_HEAD` that
/// freezes the vault (`git_native::commit_all` refuses while one stands). The two engines are
/// held byte-identical by `tests/merge_differential.rs`, which is the entire reason a second
/// one is allowed to exist.
fn text_3way(
    base: &str,
    ours: &str,
    theirs: &str,
    marker_size: usize,
) -> Result<(String, Merged), StoreError> {
    #[cfg(feature = "native-git")]
    if crate::vcs::native() {
        return text_3way_native(base, ours, theirs, marker_size);
    }
    text_3way_git(base, ours, theirs, marker_size)
}

/// One `git_merge_file_input`, borrowing `text` for the duration of the call.
///
/// # Safety
/// The returned struct holds a raw pointer into `text`; it must not outlive it, and
/// `git_merge_file` must be the only thing that reads it.
#[cfg(feature = "native-git")]
unsafe fn merge_input(text: &str) -> libgit2_sys::git_merge_file_input {
    let mut input: libgit2_sys::git_merge_file_input = std::mem::zeroed();
    libgit2_sys::git_merge_file_input_init(&mut input, 1);
    input.ptr = text.as_ptr() as *const std::os::raw::c_char;
    input.size = text.len();
    input
}

/// The same 3-way merge from libgit2's own port of it — `git_merge_file`, the buffer-shaped
/// API that takes three texts and hands back one.
///
/// **Why the raw `-sys` crate and not `git2`.** `git2` 0.21 wraps only
/// `Repository::merge_file_from_index`, which wants `IndexEntry`s and would write into the
/// ODB — the opposite of what a merge driver needs — and it imports `libgit2-sys` privately,
/// so the buffer API is unreachable through it. `libgit2-sys` is already an optional
/// dependency of this crate (for the CA-store option), so this adds no dependency, only a
/// second use of one. That is what makes this cheap now and expensive in July, when the
/// rejected proposal assumed a `git2::merge_file` that does not exist
/// (`mobile-design.md`, ruling 3, marked refuted).
///
/// **The options mirror the subprocess call exactly, because the harness compares bytes:**
/// the same three labels in the same order, the same marker size, `favor = NORMAL` (never
/// resolve by fiat — that is the data loss this module exists to stop) and default flags,
/// which is git's own two-sided marker style rather than diff3.
#[cfg(feature = "native-git")]
fn text_3way_native(
    base: &str,
    ours: &str,
    theirs: &str,
    marker_size: usize,
) -> Result<(String, Merged), StoreError> {
    use std::os::raw::{c_char, c_ushort};

    // These are raw `libgit2-sys` calls, which skip the `git2` wrappers' own `init()`.
    // Opening a path that cannot exist is the cheapest way to run it — the same idiom, and
    // the same reason, as `git_native::add_certs_from_pem`.
    let _ = git2::Repository::open(Path::new("/nonexistent/formicaria-libgit2-init"));

    // The labels a human reads inside the markers, so they are part of the output and part of
    // what the differential test compares. NUL-terminated in place: the call only borrows them.
    const ANCESTOR: &[u8] = b"base\0";
    const OURS: &[u8] = b"ours\0";
    const THEIRS: &[u8] = b"theirs\0";

    // Safety: every pointer handed to libgit2 is either a NUL-terminated literal above or a
    // borrow of one of the three `&str` parameters, all of which outlive this block. The one
    // allocation libgit2 hands back is read and freed before returning. Note the merged bytes
    // are *not* required to be valid UTF-8 — `result.len` carries the length, and the lossy
    // conversion mirrors what the subprocess arm does with its stdout.
    unsafe {
        let ancestor = merge_input(base);
        let our = merge_input(ours);
        let their = merge_input(theirs);

        let mut opts: libgit2_sys::git_merge_file_options = std::mem::zeroed();
        if libgit2_sys::git_merge_file_options_init(&mut opts, 1) < 0 {
            return Err(StoreError::Io("could not initialise a libgit2 merge".into()));
        }
        opts.ancestor_label = ANCESTOR.as_ptr() as *const c_char;
        opts.our_label = OURS.as_ptr() as *const c_char;
        opts.their_label = THEIRS.as_ptr() as *const c_char;
        opts.marker_size = marker_size as c_ushort;

        let mut result: libgit2_sys::git_merge_file_result = std::mem::zeroed();
        let code = libgit2_sys::git_merge_file(&mut result, &ancestor, &our, &their, &opts);
        if code < 0 {
            return Err(StoreError::Io(format!(
                "libgit2 could not merge the body: {}",
                git2::Error::last_error(code).message()
            )));
        }

        // A merge whose result is empty legitimately hands back a null pointer, and
        // `from_raw_parts` on null is undefined behaviour even for a zero length.
        let merged = if result.ptr.is_null() || result.len == 0 {
            String::new()
        } else {
            let bytes = std::slice::from_raw_parts(result.ptr as *const u8, result.len);
            String::from_utf8_lossy(bytes).into_owned()
        };
        // `automergeable` is libgit2's word for the exit code the subprocess arm reads: zero
        // means it left markers behind.
        let verdict = if result.automergeable != 0 { Merged::Clean } else { Merged::Conflicted };
        libgit2_sys::git_merge_file_result_free(&mut result);
        Ok((merged, verdict))
    }
}

/// The 3-way text merge as a subprocess — `git merge-file`, which every desktop already has.
///
fn text_3way_git(
    base: &str,
    ours: &str,
    theirs: &str,
    marker_size: usize,
) -> Result<(String, Merged), StoreError> {
    // git merge-file works on paths, so the texts have to land somewhere. **Not** in
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

fn read(path: &Path) -> Result<String, StoreError> {
    std::fs::read_to_string(path).map_err(io)
}

fn io(e: std::io::Error) -> StoreError {
    StoreError::Io(e.to_string())
}

/// The one property the sentence rescue rests on, so it is tested where the two halves of it
/// live rather than through the public merge: **`join_sentences(split_sentences(x)) == x`, for
/// every `x`.** If that ever stops holding, a clean rescue silently rewrites someone's prose —
/// which is the exact failure this module exists to prevent, arriving through the door built to
/// prevent it.
#[cfg(test)]
mod sentence_transform {
    use super::{join_sentences, split_sentences};

    fn roundtrips(text: &str) {
        let there = split_sentences(text);
        let back = join_sentences(&there);
        assert_eq!(back, text, "split/join is not the identity\nsplit form:\n{there:?}");
    }

    #[test]
    fn the_split_is_reversible_on_the_shapes_that_have_an_answer_to_argue_about() {
        for case in [
            "",
            "\n",
            "one sentence",
            "One sentence. Another one.",
            "One sentence. Another one.\n",
            // Punctuation at a line end, with and without the trailing space that makes the
            // decode ambiguous if the two halves of the transform ever drift apart.
            "Ends here.\nNext line.",
            "Ends here. \nNext line.",
            "Ends here.  \nTwo spaces is a Markdown hard break.",
            "Trailing spaces and nothing else.   ",
            // The guard: a list marker and an abbreviation are not sentence ends.
            "1. first item\n2. second item",
            "Uses e.g. this and i.e. that in one line.",
            // Not ASCII, so the byte scan must not cut inside a character.
            "Une phrase. Déjà vu. 日本語の文。 Fin.",
            "3.14 is not a sentence. 20. is a list marker.",
            // The structural guard, and the fragment case that decides where it is asked:
            // `Intro. ` cuts, and what is left begins with a pipe, so the joiner sees a table row
            // where the *source line* was prose. Both sides must ask at the same place.
            "Intro. |ab. cd.",
            "Intro.     code. more.",
            "| Kind. Sort. | Meaning. Sense. |",
            "    print(\"Hello. World.\")",
            "\tprint(\"Hello. World.\")",
            "    trailing spaces after code. ",
            // A setext heading underline is a line of `=`, which the *conflicted* joiner treats as
            // a marker. The exact joiner must not: this is the clean path.
            "A heading. Underlined.\n=======\nbody. More.",
            "Question? Yes! Really.  Three ways to end.",
            "\r\nCRLF. Two sentences.\r\nSecond line.\r\n",
            "...\n. \n.. \nheading\n",
        ] {
            roundtrips(case);
        }
    }

    /// Same property over text nobody chose, since the hand-picked cases above are exactly the
    /// ones I thought of. Fixed-seed xorshift, in the style of `tests/merge_differential.rs`:
    /// a reproducible failure is worth more here than statistical quality.
    #[test]
    fn the_split_is_reversible_on_text_nobody_picked() {
        const SEED: u64 = 0x5EED_0000_5E17_0001;
        let mut x = SEED;
        let mut next = move || {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        };
        // Weighted towards the characters the transform actually looks at.
        // Weighted towards what the transform looks at, and it must include the bytes the
        // structural guard and the marker test key on — `|`, tab, `=`, `<`, `>` — or those two
        // rules are only ever exercised by cases somebody thought of.
        let alphabet: Vec<char> = "aB1 ...!?.\n\r\t.|= <>é—".chars().collect();
        for _ in 0..2000 {
            let len = (next() % 60) as usize;
            let text: String =
                (0..len).map(|_| alphabet[(next() % alphabet.len() as u64) as usize]).collect();
            roundtrips(&text);
        }
    }

    /// The cut itself, stated rather than inferred from the round trip — which passes just as
    /// happily when nothing is ever split.
    #[test]
    fn a_paragraph_becomes_one_line_per_sentence_and_a_list_does_not() {
        assert_eq!(split_sentences("One. Two. Three."), "One. \nTwo. \nThree.");
        assert_eq!(split_sentences("1. one\n2. two"), "1. one\n2. two");
        assert_eq!(split_sentences("Written e.g. like this."), "Written e.g. like this.");
        assert_eq!(split_sentences("no sentence ends here"), "no sentence ends here");
    }
}
