# 2026-10-09 — meetings from your calendar and your mail, undo, and Back that goes back

A snapshot of one long day, as believed that evening. For current fact read `features.md`,
`known-issues.md`, `outstanding.md` and `decisions.md` (all 2026-10-09 entries).

## The ask, and how it narrowed

*"Agenda points are updated through emails… a tool that parses emails (read only, it is very
sensitive) and creates deterministic lists of tasks, dates and times, plus a recap."* Over the day
the owner narrowed it three times: **local models only, and autonomous** (no paste from Gemini;
*"the user enters only when she needs to accept or not a proposal"*); **meetings only** (*"I can
create tasks by myself"*); **the conversation note is the first meeting**, with a further meeting
in the same exchange as its own note.

## What was built, in the order it was built

1. **Your own calendar.** The secret iCal address is read hourly. `UID`/`SEQUENCE` decide create,
   move and cancel, with no model involved (`fm_core::calendar`). It was run on the owner's real
   calendar the same evening.
2. **Repeating meetings** (`fm_core::recur`), one note per occurrence for 60 days ahead. A rule
   outside the subset is refused and counted, never guessed.
3. **Gmail, read-only by scope**, through the owner's own Google Cloud client. The set-up needed
   real hand-holding: Italian console labels (`?hl=en` helps), the Get-started form, scopes, a
   Desktop client.
4. **The text question.** It was first kept out of the vault for privacy. The owner reversed that:
   *why bother with a secret location?* The cost was stated (the vault is copied to GitHub) and
   accepted, and the note now holds the whole exchange with its metadata.
5. **The meeting pass.** Its **first real run proposed nothing**: the model found the right
   sentences but "copied" the date as a translation ("the 16th of October" became "16 ottobre"),
   or as a word from the instructions' examples. The checks correctly refused all of it. The fix
   was a design change, not a tweak: the model only points at the sentence, and Rust reads the date
   from it (or from earlier in the same email, for "that day"). A dry run on the real mail then
   proposed exactly the three upcoming meetings.
6. **The model's window**: 2048 → 8192 tokens, measured on the 4 GB GPU. It only starts with the
   image projector off the card.
7. **Releases 0.6.0 and 0.6.1** (security updates for the interface's libraries: 46 audit findings
   → 0, the last by lifting `chokidar` so `braces` left the tree), then **0.6.2**: undo, Recently
   deleted, and Back.
8. **0.6.2 broke window switching on the phone.** Closing the window list stepped history back onto
   the window just left. The test had checked right after the click, before that step arrived.
   **0.6.3** fixed it and was tested on the phone *before* tagging.

## What the day taught

- **A small model is good at finding things and bad at copying them.** Ask it to point; do the
  exact part in code.
- **A test of anything that goes through history must wait for `popstate`.**
- **Test on the phone before tagging.** The local signing file had been a copy of the keystore
  since 2026-09-09, so local builds were unsigned. The route that worked: run `release.yml` by
  hand on `main`, take its signed `android-apk` artifact, check the certificate, and install it by
  cable (`known-issues.md`).
- **Real data out of public tests.** Tests first written from the owner's mail carried names, a
  room and course codes. They were replaced with invented ones before anything was committed.
