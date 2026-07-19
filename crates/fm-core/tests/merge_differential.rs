//! The differential harness for the merge — step 2 of the mobile sequence.
//!
//! **What this is for.** `text_3way` is the last thing in the merge that shells out to
//! `git merge-file`, so it is exactly what a platform with no `git` binary has to replace.
//! Replacing the 3-way text merge on the one path that must never corrupt is not something to
//! do on a reading of the new engine's docs — the plan already tried that once and named a
//! `git2` function that does not exist. So: nothing swaps until a candidate is graded against
//! the implementation we have, over inputs nobody hand-picked.
//!
//! **Run against today's engine first, deliberately.** With one engine the comparison is close
//! to a tautology, and that is the point of doing it now: it proves the *harness* — that the
//! generator reaches conflicts as well as clean merges, that the comparison is exact, that the
//! verdict mapping is pinned — while there is still a known-good answer to check it against.
//! A harness first exercised on the day it is needed is a harness nobody trusts.
//!
//! **What a future engine must satisfy** is the `reference_*` comparisons below: identical
//! verdict, and identical bytes. Marker-byte divergence is the one thing that may end up
//! consciously waived (gitoxide, for instance, documents deviations in line-ending handling)
//! — but it is waived per-case, in writing, with the case recorded here. Never by loosening
//! an assertion until it passes.
//!
//! No `rand` dependency: a fixed-seed xorshift is four lines, and a *reproducible* failure
//! matters more here than statistical quality. The seed is printed on failure.

use std::process::Command;

use fm_core::merge::{merge_texts, Merged};

fn have_git() -> bool {
    Command::new("git").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// xorshift64*. Deterministic, so a failure reproduces exactly from the printed seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// A base document, then two edits of it. Tuned so that conflicts, clean merges and
/// no-op sides all occur — asserted below, because a generator that only ever produces
/// clean merges would make every assertion here pass while testing nothing.
fn triple(rng: &mut Rng, eol: &str) -> (String, String, String) {
    // Long enough that two sparse edits usually land apart. A short document with busy edits
    // conflicts essentially always, which sounds like a stress test and is actually a worse
    // one: it never exercises the clean path, and the clean path is where a wrong engine
    // silently loses content instead of loudly marking it.
    let n = 12 + rng.below(12);
    let base: Vec<String> = (0..n).map(|i| format!("line {i} lorem ipsum")).collect();

    // Edit density varies per case, so the suite spans "barely touched" to "rewritten".
    let density = [6, 10, 15, 25][rng.below(4)];
    let edit = |rng: &mut Rng, density: u64| -> Vec<String> {
        let mut out = Vec::new();
        for line in &base {
            if rng.chance(density / 3) {
                continue; // deleted
            }
            if rng.chance(density) {
                out.push(format!("{line} — edited {}", rng.below(1000)));
            } else {
                out.push(line.clone());
            }
            if rng.chance(density / 3) {
                out.push(format!("inserted {}", rng.below(1000)));
            }
        }
        out
    };

    // A third of the time only one side moves. That is the commonest real shape — someone
    // pulls work they have no local edits against — and it is guaranteed-clean, so it keeps
    // the clean path well covered without weakening the conflicting cases.
    let one_sided = rng.chance(33);
    let ours = edit(rng, density);
    let theirs = if one_sided { base.clone() } else { edit(rng, density) };
    let join = |v: &[String]| {
        let mut s = v.join(eol);
        s.push_str(eol);
        s
    };
    (join(&base), join(&ours), join(&theirs))
}

/// What `text_3way` does, written out independently. This is the oracle: whatever replaces
/// the engine has to agree with *this*, not with our wrapper around it.
fn reference(base: &str, ours: &str, theirs: &str, marker: usize) -> (String, Merged) {
    let dir = tempfile::tempdir().unwrap();
    let p = |n: &str, t: &str| {
        let path = dir.path().join(n);
        std::fs::write(&path, t).unwrap();
        path
    };
    let (o, b, t) = (p("ours", ours), p("base", base), p("theirs", theirs));
    let out = Command::new("git")
        .arg("merge-file")
        .arg("-p")
        .arg(format!("--marker-size={marker}"))
        .args(["-L", "ours", "-L", "base", "-L", "theirs"])
        .args([&o, &b, &t])
        .output()
        .expect("git merge-file");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    match out.status.code() {
        Some(0) => (text, Merged::Clean),
        Some(n) if n > 0 && n < 128 => (text, Merged::Conflicted),
        other => panic!("git merge-file failed ({other:?})"),
    }
}

/// **The gate.** Byte-for-byte and verdict-for-verdict against the oracle, over generated
/// triples, across marker sizes and both line endings.
///
/// Plain text rather than notes on purpose: without frontmatter, `merge_texts` falls straight
/// through to the whole-file text merge, so this exercises the engine and nothing else — no
/// frontmatter rules, no scene merge, no fast paths.
#[test]
fn the_text_engine_agrees_with_git_merge_file_over_generated_triples() {
    if !have_git() {
        eprintln!("skipping differential test: git not on PATH");
        return;
    }
    const SEED: u64 = 0x5EED_1234_ABCD_0001;
    let mut rng = Rng(SEED);
    let (mut clean, mut conflicted) = (0, 0);

    for case in 0..400 {
        let eol = if case % 4 == 3 { "\r\n" } else { "\n" };
        let marker = [7, 7, 12, 32][case % 4];
        let (base, ours, theirs) = triple(&mut rng, eol);

        let (got, verdict) = merge_texts(&base, &ours, &theirs, marker).expect("merge_texts");
        let (want, want_verdict) = reference(&base, &ours, &theirs, marker);

        assert_eq!(
            verdict, want_verdict,
            "seed {SEED:#x} case {case}: verdict differs\nbase:\n{base}\nours:\n{ours}\ntheirs:\n{theirs}"
        );
        assert_eq!(
            got, want,
            "seed {SEED:#x} case {case}: bytes differ\nbase:\n{base}\nours:\n{ours}\ntheirs:\n{theirs}"
        );

        match verdict {
            Merged::Clean => clean += 1,
            Merged::Conflicted => conflicted += 1,
        }
    }

    // The generator has to reach both outcomes, or every assertion above is vacuous. This is
    // the assertion that makes the other two mean something.
    // Tuned to ~50/50 at this seed; the floors are well below that so ordinary drift does not
    // trip them, but a generator change that collapses one side of the split will.
    assert!(clean > 100, "generator produced too few clean merges ({clean}/400)");
    assert!(conflicted > 100, "generator produced too few conflicts ({conflicted}/400)");
    eprintln!("differential: {clean} clean, {conflicted} conflicted");
}

/// Notes, not plain text — the invariant the collaboration design actually promises.
///
/// A `Clean` result must be a *readable note*. That is what lets the app keep serving a vault
/// after a pull, and it is the property whose failure is the known cost recorded in
/// `decisions.md` (a divergent frontmatter field returns `Conflicted`, never a broken `Clean`).
#[test]
fn a_clean_note_merge_always_produces_a_note_that_parses() {
    if !have_git() {
        eprintln!("skipping differential test: git not on PATH");
        return;
    }
    const SEED: u64 = 0x5EED_1234_ABCD_0002;
    let mut rng = Rng(SEED);
    let mut checked = 0;

    for case in 0..300 {
        let (b, o, t) = triple(&mut rng, "\n");
        // Same id/created on all three (they are immutable in the merge rules); `updated`
        // differs, which is the collision every concurrent save really produces.
        let note = |updated: &str, status: &str, body: &str| {
            format!(
                "---\nid: 01JQ0000000000000000000000\ntype: task\ntitle: t\n\
                 created: 2026-07-17T10:00:00Z\nupdated: {updated}\nstatus: {status}\n---\n\n{body}"
            )
        };
        // Sometimes agree on status, sometimes diverge — the diverging case is the one that
        // must come back Conflicted rather than a Clean note that lost someone's column.
        let (os, ts) = if rng.chance(50) { ("doing", "doing") } else { ("doing", "done") };

        let base = note("2026-07-17T10:00:00Z", "todo", &b);
        let ours = note("2026-07-17T11:00:00Z", os, &o);
        let theirs = note("2026-07-17T12:00:00Z", ts, &t);

        let (got, verdict) = merge_texts(&base, &ours, &theirs, 7).expect("merge_texts");

        if verdict == Merged::Clean {
            fm_core::frontmatter::from_file(&got).unwrap_or_else(|e| {
                panic!("seed {SEED:#x} case {case}: a Clean merge did not parse: {e}\n{got}")
            });
            checked += 1;
        }
    }

    assert!(checked > 40, "too few clean note merges to be meaningful ({checked}/300)");
    eprintln!("note invariant: {checked} clean merges, all parsed");
}
