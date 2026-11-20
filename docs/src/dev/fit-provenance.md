# Reading watch files (FIT): provenance

formicaria reads a sports watch's activity files (FIT) — on the phone, files shared to it or found in one granted folder (`import_fit_files`); on a computer, a folder the person names is
read into the training log (`import_fit`, host-only). This page says what that reading is built
from, as `decisions.md` (2026-11-20, *Reading and writing FIT: open crates, a provenance note*)
requires. The same note heads the decoder, `crates/fm-core/src/fit.rs`.

| | |
|---|---|
| Decoder | the `rustyfit` crate, **0.11.2**, BSD-3-Clause — <https://github.com/muktihari/rustyfit> |
| Message and field numbers | from **that crate's generated profile** (FIT profile 21.221, which the crate generates from the FIT SDK's `Profile.xlsx`). formicaria carries no profile of its own |
| Why not `fitparser` | the decision named `fitparser` (MIT) for reading. It brings `chrono`, which the advisory database lists as unmaintained with no safe upgrade (RUSTSEC-2026-0543), so the licence and advisory gate (`pixi run deny`) refuses it. `rustyfit` is the decision's other crate, named there as the writer; it reads too |
| Written in formicaria | the frame check before decoding (header size, the `.FIT` signature, the data size, the header and file CRC-16), the scales applied to a few raw values, and the message numbers the reader names (`fit::mesg`), from the protocol's published description |
| Brought in with it | `embedded-io` (MIT OR Apache-2.0). No time-zone library: the crate returns raw counts, and formicaria converts on the app's own clock |
| Writer | the same crate, **0.11.2**: `crates/fm-core/src/fit_write.rs` writes a *workout* file (type 5: `file_id`, `workout`, `workout_step`) from a planned Coach session (`workout_fit`). Scales are the profile's: time in ms, distance in cm, speed in mm/s, weight in kg × 100, heart rate as bpm + 100 (a custom value of 100 or below would mean "% of maximum", which is never written). `embedded-io` is named directly for the in-memory `Write + Seek` the encoder needs; it was already in the build |
| Licence | formicaria relies on the FIT Protocol License's **§1** grant to use the FIT protocol in software the licensee creates. §2(c), §2(d) and §2(f) were read; the residual legal question is accepted, not settled |

## What is tested, and how

- Every file in the tests is **synthetic**: written by a small test-only writer
  (`crates/fm-core/src/fit_synth.rs`) from the protocol's description, with invented values and
  invented serial numbers. It is deliberately not `rustyfit`, so a mistake the reader made would
  not be hidden by a writer that shares its code. No real person's file is in the repository.
- The decoder is run over seeded random and mutated inputs and must never panic; a damaged file is
  refused with a sentence that names what is wrong.
- `ci/checks.sh` fails if the decoder, the writer, the import or the workout builder names a way to
  the network.
- **T18**: a workout written by the writer is read back by the decoder step for step (time,
  distance, repetitions, weight, the repeat, heart rate and pace ranges), and no step carries a
  percentage or zone target. Writer and reader share the crate here, so the file must also pass
  formicaria's own frame check (sizes and both CRCs), written independently of the crate.

## Not yet verified

The reader has decoded synthetic files only. That it reads `length` (swim) and `set` (strength)
messages, and `workout_rpe`, from a real watch's files as it reads the synthetic ones is still to be checked on real files
locally (the coach plan's experiment E0); see `docs/context/known-issues.md`.
