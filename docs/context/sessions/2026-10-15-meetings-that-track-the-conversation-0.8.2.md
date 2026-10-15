# 2026-10-15 — meetings that track the conversation; proposals that read like notes; v0.8.2

> A snapshot, never updated. Current state is `features.md` and `known-issues.md`; the rulings are
> in `decisions.md` under 2026-10-15.

## How it started

The owner had emailed three students to move or cancel meetings set for 16 Oct, and the Agenda still
showed the old times. Each step of finding out why turned up a real fault, found on real data:

- **The proposal could never be accepted** — a backup's squash had folded away the commit the
  proposal branched from (add/add on a just-arrived email note). Fixed in both git backends.
- **The review showed none of the details** it would set, only text. Now Title/Start/Due/Place
  fields beside the note's current values, editable; and the note's own editor, not a textarea.
- **Collaboration flickered every ~15 s** — an open review reloaded on every refresh and its
  `proposal_shown` counted as a change, feeding the next refresh (220 bumps in 52 minutes, measured
  against the running server).
- **The emails never arrived** — Gmail labels sit on messages; today's replies carried none (read
  from the owner's mailbox, read-only). The reader now follows labelled conversations.
- **The model reported every moved meeting at its old time** — it re-read our own box first, and
  was asked which meetings an exchange *fixes*. Now: box hidden, "where do the meetings stand now",
  `fixed`/`cancelled`. A near-copied quote ("16th **of** October") stands as the email's own words;
  "next week on friday" is the following week.

## Then, at the owner's asking

- Reading on opening, mail and calendars every 5 minutes (an unchanged calendar skipped by hash),
  the meeting pass draining its backlog — *"I do not like that every time you need to trigger this
  manually."*
- Called-off meetings marked `cancelled`, struck through in Agenda and Calendar (calendar
  cancellations too — kept by the owner); on a re-read only when the call-off is the latest word.
  Calendar meetings are never touched by the email pass.
- Conversation titles "Person — who they are", once each, as proposals.
- The phone test dropped from the release requirements.

## How it was checked

A live harness (`crates/fm-agent/tests/live_meetings.rs`, ignored in CI) ran `qwen3-vl-4b` over the
owner's real conversations, kept in the session's scratch folder and never in the repo; the tests
use invented names. Final run: two moves with their Zoom links, a move to the following week, a
confirmation, a call-off, and five titles — all right; the roles plausible, not always exact.

## Left open

The `agreed` state is not read (the box names who said it); a conversation with several outside
people is titled after one of them; a meeting move phrased with both times ("not 2 — 5") reads as a
further meeting.
