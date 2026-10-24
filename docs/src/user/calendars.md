# Meetings from your calendar

formicaria can read your own calendar and keep a note for every upcoming meeting. New meetings
appear by themselves, a meeting that is moved moves, and a cancelled one is marked. You never have
to do anything after the first step.

## Add a calendar

1. **Copy your calendar's private address.** In Google Calendar, open *Settings*, choose the
   calendar under *Settings for my calendars*, then *Integrate calendar*, and copy **Secret address
   in iCal format**. In Outlook, *Settings → Calendar → Shared calendars → Publish a calendar* gives
   an ICS link (your organisation may have switched this off).
2. In formicaria, open **Settings → Connections → Calendars**. Give it a short name such as `work`, paste the
   address, choose which notebook the meetings go into, and press **Add calendar**.

The calendar is read when formicaria opens and then every 5 minutes while it is open. A calendar
that has not changed costs one download and touches nothing.

## What it does to your notes

- **Each upcoming meeting becomes a note** tagged `meeting` and the calendar's name, with its start
  and end, so it shows in the Agenda. The place and organiser are kept on the note.
- **When a meeting is rescheduled**, its note's start, end and place are updated. Its title and
  anything you wrote in it are **never changed**.
- **When a meeting is cancelled**, its note gains the tag `cancelled`. Notes are **never deleted**.
- **Meetings that are already over are not added.**
- **Two meetings at the same time** are marked **⚠ clash** in the Agenda's week, so an overlap does
  not pass for one meeting after another. A meeting with no end time is drawn as an hour.
- **A repeating meeting gets one note per occurrence** for the next two months. Later ones appear as
  time passes. If the organiser moves one week, that week's note moves. If they drop a week or move
  the whole series to another day, the dates it no longer has are tagged `cancelled`, never deleted.

Under each calendar, Settings says how the last read went: how many meetings were new, moved or
cancelled, and how many were left out and why.

## Your time zone

The whole app — the agenda, reminders, food and health days, the meetings found in mail — runs on
**one clock**, set in **Settings → Clock**. It is *Automatic* unless you choose: this device's own
zone, and it follows it — change the time zone of your computer or phone, and within a few minutes
every view shows the same meetings at the local hour. Choose a zone there instead when the
computer's own setting is wrong, or when you want to see your agenda on another clock while
travelling. Each device has its own setting. The same note shows at the right hour on every
device — 18:00 on a laptop in Tokyo, 10:00 on one in Berlin.

The small label beside the agenda's M / W / L switch says which zone you are looking at — for
example *Berlin · GMT+1*; hold the pointer on it to see the clock your notes are kept on.

That works because each notebook has one clock its times are kept on: the zone its first save was
made in (for most people, home), the same on every device. To set it yourself — useful if you live
where clocks change in summer — add `"notes_time_zone": "Europe/Rome"` to the notebook's
`vault.json`.

A meeting given on another clock is moved onto yours. An event your calendar holds in Berlin time,
at 10:00 on 27 October, is shown at 18:00 in Tokyo; the note still says which zone it was
written in. The same goes for an email that says *"10am Berlin time"* or *"15:00 CET"*: the
assistant proposes 18:00 and shows the email's own words beside it. A time with no zone written
after it is taken as yours.

Until version 0.19 the zone was set under Settings → Calendars; a zone set there is carried over
to Settings → Clock the first time 0.19 starts.

## Privacy

- The private address works like a password: anyone who has it can read your whole calendar. It is
  kept **only on this computer**, in a file only you can read, and is **never** saved in your notes,
  so it is never shared with anyone you share a notebook with.
- The address can only be read. formicaria cannot change your calendar through it.
- A tablet you have paired with this computer cannot see or change your calendars.

## Limits for now

- **A few unusual repeat patterns** (for example "the last weekday of the month") are not read yet.
  Those meetings show only their first date, and Settings counts how many.
- If you delete a meeting's note, it comes back at the next read while the meeting is still on the
  calendar. Tag it instead, or remove the meeting from the calendar.
- Calendars are read on the computer only, not on the phone app yet.
