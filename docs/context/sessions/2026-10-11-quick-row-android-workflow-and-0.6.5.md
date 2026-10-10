# 2026-10-11 — a row above the keyboard, an Android workflow of its own, and v0.6.5

A snapshot of one day, as believed then. For current fact read `features.md`, `known-issues.md`,
`outstanding.md` (§2.18) and `decisions.md` (the four 2026-10-11 entries).

## What was asked
With a screenshot of the phone's own notes app: *fast buttons to add media and recording, and
shortcuts like bullet points, in a dedicated panel above the keyboard.* Then, after a test build:
*"this copy has no version to compare"*. Then: *a dedicated Android workflow, the same one the full
release calls.*

## What was built
1. **The quick row.** Record, add a photo or file, checklist, bullet, text style, while editing on a
   touch screen. It amends the 2026-09-11 ruling that removed touch's persistent strip, so the
   decision was written before the code.
2. **The app's bar steps aside while a note is typed.** Not asked for; it was the only way the row
   could be above the keyboard, and it nearly doubles the room for text.
3. **The updater's second half.** A copy with no version could check for a release since
   2026-10-09 and could not download one. One rule had lived in two functions.
4. **`android.yml`**, called by `release.yml` and runnable alone for a phone test.
5. **v0.6.5**, tagged after the owner confirmed the row on the phone.

## What went wrong, and what it taught
- **The row was under the keyboard on the real phone** after passing a browser check. The check
  simulated a keyboard and no navigation bar; the phone reports the keyboard *beyond* that bar, and
  the app's bar had been filling the difference. **Simulate every inset the device has, not the one
  being worked on.**
- **A fix that reached one of two enforcers.** The updater check and the updater download each
  compared versions. The test now pins both side by side.
- **Guards that named a file.** Moving jobs out of `release.yml` would have left three checks
  passing by finding nothing. Each was moved, and each was shown to fail on a broken file.
- **The README said nothing runs on a push.** It had been wrong for a month.

## Left unverified at the end of the day
Whether the v0.6.5 release run, the first through the new call, published its APK; and a test copy
updating itself end to end.
