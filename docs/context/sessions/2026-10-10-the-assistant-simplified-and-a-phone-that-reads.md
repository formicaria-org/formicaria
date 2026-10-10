# 2026-10-10 — the assistant simplified, a phone that reads a picture, and a model that is offered

A snapshot of one long day and the release the next morning, as believed then. For current fact
read `features.md`, `known-issues.md`, `outstanding.md` (§2.17) and `decisions.md` (the three
2026-10-10 entries).

## The ask, and how it moved
It began as *improve the phone's model, which is quite bad*, and became a plan to simplify the
assistant: **one assistant, tool selection by file type in code, the model's one job being mail →
meetings, and image → text on every device**. The owner narrowed it three times: no model-based
selector for now; no specialised reader on the phone *if it is too slow*; and one general model
that can also see.

## What was measured, and what the measurements overturned
1. **LFM2.5-2.6B on the phone** looked like a regression (7.7 tok/s, no answers at the usual
   length). The owner doubted the test, and was right: the model always reasons first, its maker
   recommends it for extraction and tool use, and the test used neither its settings nor its
   tasks. The speed stood; the verdict was withdrawn in the log.
2. **PaddleOCR-VL-1.6** read the fixtures better than the laptop's model, and took **145–224 s an
   image on the phone**: the image encoder, not the text, is the cost.
3. **LFM2.5-VL-450M** read them in **3–5 s on the phone**. An estimate of 25 s, scaled from the
   first reader's phone-to-laptop ratio, was wrong by a factor of six.
4. Two smaller candidates failed for different reasons: SmolVLM-256M describes instead of
   transcribing; granite-docling is no faster than PaddleOCR-VL.

## What was built
- **P0 and P1 of the plan:** the unused code removed; one request helper, one model client, one
  quote-matcher, one prompts file, one replies file, one home for asset references; a closed list
  of commands.
- **Who reads an image:** a dedicated reader, else the main model if it can see, else nobody. The
  phone's launch had hard-coded "cannot see"; it now shares the desktop's code.
- **A newer model is offered:** tracing how the phone's new default would reach people found that a
  phone would download it unasked and a downloaded desktop app would never see it. Both now keep
  what they have and offer the new one in Settings.
- **An audit for stale code** found the code already tight, and half of its own candidates were
  wrong once opened (`2026-10-10-stale-code-audit.md`).
- **v0.6.4**, tagged on 2026-10-11 after the owner ran the test build on the phone.

## What the day taught
- **An estimate from another model's ratio is not a measurement.** The phone was connected; the
  owner had to say so.
- **A benchmark measures the question it asks.** Read the maker's card for what a model is for and
  how to run it before calling a result a verdict.
- **Stop only the process you started.** A `kill $(pgrep -x llama-server)` took down the owner's
  running assistant, which had been opened mid-benchmark.
- **Open a "dead" thing before deleting it.** A name search found a command with no caller; opening
  it found five tests and the building block of a planned feature.
- **Which model runs was decided in two places and recorded in none.** The model picked on the
  first-enable screen was downloaded and then not started.
