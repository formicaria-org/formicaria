//! **Fewer prose conflicts, by cutting the prose at a smaller seam** — `outstanding.md` §2.13.
//!
//! A line merge asks "did you both change this line?", and a Markdown paragraph is one line. So
//! two devices editing two *different sentences* of one paragraph collide over prose neither of
//! them touched, and the note freezes on a disagreement that does not exist.
//!
//! **Measured before it was built, on this repo's own vault (2026-09-08, 771 body lines):** 26%
//! of body lines carry more than one sentence; 54% of the collisions two independent edits can
//! have on a line are between *different* sentences, and **30% are between sentences far enough
//! apart that the merge settles them** — the rest touch, and a 3-way merge will not merge two
//! changed lines with nothing unchanged between them. On a bullet-shaped corpus (a 336-note
//! Logseq vault) the same figures are 67% and 48%. So: roughly one prose conflict in three stops
//! happening, and none of the ones that stop is a disagreement.
//!
//! **The risk is contained by control flow, not by these tests.** The ordinary line merge runs
//! first and its answer stands unless it conflicted; the finer pass is consulted only then, and
//! only its *clean* results are taken. So a merge that is clean today cannot change, and a
//! conflict either becomes clean or stays byte-for-byte what it is now. The tests below pin the
//! behaviour; the guarantee is that there is no third outcome to pin.

use fm_core::merge::{merge_texts, Merged};

/// One note, three versions, differing only in the body — the ordinary concurrent edit.
fn note(updated: &str, body: &str) -> String {
    format!(
        "---\nid: 01JQ0000000000000000000000\ntype: note\ntitle: t\n\
         created: 2026-07-17T10:00:00Z\nupdated: {updated}\n---\n\n{body}\n"
    )
}

fn merge(base: &str, ours: &str, theirs: &str) -> (String, Merged) {
    let (text, verdict) = merge_texts(
        &note("2026-07-17T10:00:00Z", base),
        &note("2026-07-17T11:00:00Z", ours),
        &note("2026-07-17T12:00:00Z", theirs),
        7,
    )
    .expect("a note merge");
    let body = fm_core::frontmatter::from_file(&text).expect("a merged note still parses").body;
    // The wrapper above contributes a blank line either side; the prose is what is under test.
    (body.trim_matches('\n').to_string(), verdict)
}

const PARA: &str = "The vault syncs over git. Every note is one file. \
                    The phone runs libgit2 and the desktop shells out.";

/// The headline: the case that is the majority of prose conflicts and is not a disagreement.
///
/// Proven red by making `sentence_rescue` return `None`: the two edits then come back as a
/// whole-paragraph conflict with markers, which is what shipped until 2026-09-08.
#[test]
fn two_devices_editing_two_sentences_of_one_paragraph_stop_conflicting() {
    let ours = PARA.replace("syncs over git", "syncs over git, which is the point");
    let theirs = PARA.replace("desktop shells out", "desktop shells out to the binary");

    let (body, verdict) = merge(PARA, &ours, &theirs);

    assert_eq!(verdict, Merged::Clean, "different sentences are not a disagreement:\n{body}");
    assert!(body.contains("which is the point"), "our edit survived:\n{body}");
    assert!(body.contains("to the binary"), "their edit survived:\n{body}");
    assert!(!fm_core::merge::has_conflict_markers(&body), "and nothing is marked:\n{body}");
    // The paragraph is still a paragraph: the transform is undone, not left in the file.
    assert_eq!(body.lines().count(), 1, "the split is an implementation detail:\n{body:?}");
}

/// The other half of the same rule, and the one that keeps this from being resolution by fiat:
/// when the two devices really did edit the *same* sentence, that is a disagreement and it is
/// still reported as one.
///
/// Proven red by taking the finer merge's result whatever its verdict: the markers then land
/// inside a sentence-split body and the note comes back without them at all.
#[test]
fn the_same_sentence_is_still_a_conflict() {
    let ours = PARA.replace("Every note is one file.", "Every note is one Markdown file.");
    let theirs = PARA.replace("Every note is one file.", "Every note is a single file.");

    let (body, verdict) = merge(PARA, &ours, &theirs);

    assert_eq!(verdict, Merged::Conflicted, "one sentence, two answers:\n{body}");
    assert!(fm_core::merge::has_conflict_markers(&body), "and it is marked:\n{body}");
    assert!(body.contains("one Markdown file"), "our text is there to choose:\n{body}");
    assert!(body.contains("a single file"), "and so is theirs:\n{body}");
}

/// **Nothing that merges cleanly today may change.** The finer pass sits behind a verdict check,
/// so this is a property of the control flow; the golden is what would notice if someone ever
/// moved it in front, and the bytes are the whole contract between two devices.
///
/// Proven red by running the rescue ahead of the line merge *and* returning its split form.
/// Worth recording that moving it in front **on its own changes no test in this file**: a
/// split only ever pushes two changes further apart, so a merge that was clean at line
/// granularity is clean at sentence granularity and reconstructs the same bytes. The ordering
/// is therefore not what makes the answers right — it is what makes the blast radius nil
/// without anyone having to trust that argument.
#[test]
fn a_clean_line_merge_is_returned_exactly_as_it_was() {
    let base = "First line. Second sentence here.\n\nA second paragraph. With two.\n";
    let ours = base.replace("First line.", "First line, edited.");
    let theirs = base.replace("A second paragraph.", "A second paragraph, edited.");

    let (body, verdict) = merge(base, &ours, &theirs);

    assert_eq!(verdict, Merged::Clean);
    assert_eq!(
        body, "First line, edited. Second sentence here.\n\nA second paragraph, edited. With two.",
        "a clean line merge is byte-for-byte what it was before the finer pass existed"
    );
}

/// A rescued merge has to be a **fixed point**, like every other merge here: the two devices
/// both compute it, both commit it, and re-merging it must return it. A rescue that drifted by
/// a byte would have the two devices re-deriving the same conflict forever.
#[test]
fn a_rescued_merge_is_a_fixed_point() {
    let ours = PARA.replace("syncs over git", "syncs over git, which is the point");
    let theirs = PARA.replace("desktop shells out", "desktop shells out to the binary");
    let (body, verdict) = merge(PARA, &ours, &theirs);
    assert_eq!(verdict, Merged::Clean);

    let (again, verdict) = merge(&body, &body, &body);
    assert_eq!(verdict, Merged::Clean);
    assert_eq!(again, body, "merging a rescued body with itself must return it unchanged");

    // And the merge is symmetric, which is what makes it the *same* fixed point on both
    // devices rather than two of them.
    let (swapped, verdict) = merge(PARA, &theirs, &ours);
    assert_eq!(verdict, Merged::Clean);
    assert_eq!(
        swapped.replace(", which is the point", "").replace(" to the binary", ""),
        PARA,
        "whichever side is 'ours', the merge keeps both edits and invents nothing:\n{swapped}"
    );
}

/// §2.13 asks for a **measurement, not an assertion**: over a corpus of two-device edits, the
/// count of conflicting notes goes down and no merge that was clean becomes conflicted.
///
/// **And it pins the boundary, which is the more useful half.** Three outcomes, all deliberate:
/// two edits to the same sentence still conflict; two edits to sentences with a sentence between
/// them now merge; and two edits to *adjacent* sentences still conflict, because a 3-way merge
/// treats two changed lines with nothing unchanged between them as one region. That last one is
/// left alone on purpose — separating the units with blank scaffolding lines would make git
/// merge them, and that would be this module overriding git's judgement about whether two
/// touching edits interact, which is the "third implementation" it forbids by name. A conflict
/// git would report is not ours to talk it out of.
///
/// The corpus is generated rather than read from a vault, because a test that reads the author's
/// notes is a test that only passes on the author's machine. The real-vault numbers that
/// justified the work are in the module header above and in `decisions.md`.
#[test]
fn over_a_corpus_of_two_device_edits_only_the_real_disagreements_are_left() {
    let sentences = [
        "The vault syncs over git.",
        "Every note is one file.",
        "The phone runs libgit2.",
        "A merge that loses a side is a bug.",
        "Frontmatter is merged structurally.",
        "The body is handed to a text merge.",
    ];
    let (mut settled, mut apart) = (0, 0);
    let (mut conflicted, mut touching_or_same) = (0, 0);

    for i in 0..sentences.len() {
        for j in 0..sentences.len() {
            // One paragraph, every sentence on one line — how the editor writes prose.
            let base = sentences.join(" ");
            let edit = |s: &str, who: &str| format!("{}, {who}.", s.trim_end_matches('.'));
            let ours = base.replace(sentences[i], &edit(sentences[i], "ours"));
            let theirs = base.replace(sentences[j], &edit(sentences[j], "theirs"));
            let (body, verdict) = merge(&base, &ours, &theirs);

            if i.abs_diff(j) > 1 {
                apart += 1;
                assert_eq!(
                    verdict,
                    Merged::Clean,
                    "edits to sentence {i} and sentence {j} are not a disagreement:\n{body}"
                );
                settled += 1;
                assert!(body.contains(", ours."), "case {i}/{j} kept our edit:\n{body}");
                assert!(body.contains(", theirs."), "case {i}/{j} kept theirs:\n{body}");
            } else {
                touching_or_same += 1;
                assert_eq!(
                    verdict,
                    Merged::Conflicted,
                    "edits to sentence {i} and sentence {j} touch, and a merge that settles \
                     them is this module inventing a policy rather than running one:\n{body}"
                );
                conflicted += 1;
                assert!(fm_core::merge::has_conflict_markers(&body), "and it is marked:\n{body}");
            }
        }
    }

    eprintln!(
        "sentence rescue over {} two-device edits: {settled}/{apart} that do not touch now \
         merge clean, {conflicted}/{touching_or_same} that touch still conflict",
        apart + touching_or_same
    );
}
