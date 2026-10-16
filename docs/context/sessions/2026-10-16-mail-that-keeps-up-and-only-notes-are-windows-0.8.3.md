# 2026-10-16 — mail that keeps up and says so; only notes are windows; v0.8.3

> A snapshot, never updated. Current state is `features.md` and `known-issues.md`; the rulings are
> in `decisions.md` under 2026-10-16.

## How it started

The owner, a third time in two days: replies under the label were not reaching the meetings, and every
check looked like it needed a manual nudge. Measured on the running machine each time, it was a
different fault each time — and none was the model's:

- **A reply that only confirmed a meeting changed nothing, and said nothing.** It was read within a
  minute; the verdict went to the agent's terminal only. Now each conversation's verdict is shown in
  Collaboration under *Mail*, "nothing to change — the meeting stays at …" included.
- **Two timers stacked** (mail every 5 min, the pass every minute). Now mail is read every minute and
  a read that writes something raises a counter the assistant asks for at every poll.
- **Google ended the sign-in on its 7th day**, and the only word of it was in Settings — the new Mail
  section hid itself while disconnected. Now a red *Mail stopped* card with the sign-in. The lasting
  remedy is the owner's Google Cloud *Publish app*; Google asked for a home page, a privacy policy and
  terms first, so the website gained `privacy.html` and `terms.html`, and the manual links every
  console page.
- **The model was right and our check dropped it** — a quote shortened with "…" was looked for whole
  — and **another conversation's meeting counted as "already on the agenda"** at the same hour. Both
  fixed; both moves proposed on the live machine after the rebuild.
- **Emails read as out of order**: Chinese Outlook/Gmail quote headers and a wrapped "On … wrote:"
  were not trimmed, so a reply carried the email it answered. Trimmed now; the four affected notes
  were cleaned once at the owner's word. An email read late is now placed by when it was sent.

## And the windows

*"We open even searches as windows, which to me makes no sense … on the phone to close a note I need
to do many actions."* Asked, the owner chose all three: only notes are windows (one view, re-pointed);
search is a passing panel over the view; a note closes with × at the top right of its header.

## How it was checked

Unit and component tests for each fault (the UI suite at 945), the live harness over the owner's five
real conversations (scratch only, deleted after; the repo's tests use invented names), and the running
app after each rebuild. Not seen: the new search panel and × on a phone screen, and Google accepting
the website as the app's authorized domain.

## Also

The disk filled to 100% mid-session (`target/debug` at 55 GB); the debug caches were cleared, 52.6 GB.
