# Meetings from your calendar

formicaria can read your own calendar and keep a note for every upcoming meeting. New meetings
appear by themselves, a meeting that is moved moves, and a cancelled one is marked. You never have
to do anything after the first step.

## Add a calendar

1. **Copy your calendar's private address.** In Google Calendar, open *Settings*, choose the
   calendar under *Settings for my calendars*, then *Integrate calendar*, and copy **Secret address
   in iCal format**. In Outlook, *Settings → Calendar → Shared calendars → Publish a calendar* gives
   an ICS link (your organisation may have switched this off).
2. In formicaria, open **Settings → Calendars**. Give it a short name such as `work`, paste the
   address, choose which notebook the meetings go into, and press **Add calendar**.

The calendar is read straight away, and then every hour while formicaria is open.

## What it does to your notes

- **Each upcoming meeting becomes a note** tagged `meeting` and the calendar's name, with its start
  and end, so it shows in the Agenda. The place and organiser are kept on the note.
- **When a meeting is rescheduled**, its note's start, end and place are updated. Its title and
  anything you wrote in it are **never changed**.
- **When a meeting is cancelled**, its note gains the tag `cancelled`. Notes are **never deleted**.
- **Meetings that are already over are not added.**
- **A repeating meeting gets one note per occurrence** for the next two months. Later ones appear as
  time passes. If the organiser moves one week, that week's note moves. If they drop a week or move
  the whole series to another day, the dates it no longer has are tagged `cancelled`, never deleted.

Under each calendar, Settings says how the last read went: how many meetings were new, moved or
cancelled, and how many were left out and why.

## Your time zone

Some calendars give times in UTC. formicaria shows them at the clock you set under *Your time
zone* (for example `+08:00` for Singapore). A meeting written in another zone keeps the time it was
written in and says which zone on its note.

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
