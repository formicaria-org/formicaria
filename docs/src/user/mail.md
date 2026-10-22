# Mail (Gmail)

formicaria can read your Gmail and keep a note for each conversation you care about. Invitations
inside messages become meetings straight away. It reads **only** conversations with a label you choose, from a
date you choose, and it can **only read**: Google gives it no way to send, delete or change mail.
**Label a conversation once and formicaria follows all of it** — the replies you send and receive
afterwards are read too, even though Gmail does not put the label on them.

## What it does to your notes

- **One note per conversation**, tagged `mail`, titled by its subject (without `Re:`, `R:`, `Fwd:`).
  It holds **the whole exchange**: every message with its sender, recipients, date and time,
  attachment names (the files themselves are not copied) and text. Quoted earlier messages are trimmed
  from replies (English, Italian and Chinese mail), since they are already above. A link opens the
  conversation in Gmail.
- **New replies are added in date order** — at the bottom, or above a later email if an older one
  arrives late. Anything you write in the note is never changed or moved,
  and reading the same mail again adds nothing.
- **An invitation in a message becomes a meeting note**, exactly as if it came from your calendar.
  An updated invitation moves it, and a cancellation tags it `cancelled`. If the same meeting is also
  in your Google Calendar, it stays **one** note.
- Mail is read when formicaria opens and then every minute while it is open, up to 200 new
  messages each time.

**Where the text goes.** The exchange is part of your notes, so it goes wherever the chosen notebook
goes. If that notebook is copied online (for example to GitHub), the emails, including what other
people wrote to you, are copied there too. They stay in the notebook's history even after a note is
deleted, and reach anyone you share the notebook with. To keep mail on this computer only, choose a
notebook that has no online copy under *Into*.

## Set up your Google Cloud client

Once, about 10 minutes. Google only lets an app read Gmail through a "client" registered with Google.
You create your own, so the arrangement stays between you and Google.

**Before you start:** sign in to <https://console.cloud.google.com> with the Gmail account you want
read. If you are signed in to several Google accounts, check the picture at the top right — every
link below opens in that account. If the console is in another language, add `?hl=en` to the address
to see the names used here.

Each step below is one page of the console, with its own link. **At the top of every page, check that
the project menu says `formicaria`**: a link opens whichever project you chose last.

1. **Create the project** — <https://console.cloud.google.com/projectcreate>.
   Project name: `formicaria`. Press **Create**, wait for the notification, and press **Select
   project** in it.
2. **Turn on the Gmail API** — <https://console.cloud.google.com/apis/library/gmail.googleapis.com>.
   Press **Enable**. (Do this before step 4: Google only lists the Gmail permission once its API is on.)
3. **Register the app** — <https://console.cloud.google.com/auth/overview>.
   Press **Get started** and fill in four short pages: App name `formicaria` and your Gmail as the
   support email → **Next**; Audience **External** → **Next**; your email as the contact → **Next**;
   tick the agreement → **Continue** → **Create**.
4. **Allow reading Gmail, and nothing else** — <https://console.cloud.google.com/auth/scopes>.
   Press **Add or remove scopes**, type `gmail.readonly` in the filter, tick
   `…/auth/gmail.readonly`, press **Update**, then **Save** at the bottom of the page. If it is not
   listed, paste `https://www.googleapis.com/auth/gmail.readonly` into **Manually add scopes**.
5. **Make the sign-in last** — two pages, described in
   [Stop the weekly sign-out](#stop-the-weekly-sign-out): fill in **Branding**, then press **Publish
   app** in **Audience**. Skip this and Google signs you out **every 7 days**, and mail stops being
   read until you sign in again. If you would rather stay in testing, add your own Gmail address
   under **Test users** at <https://console.cloud.google.com/auth/audience> instead.
6. **Create the client** — <https://console.cloud.google.com/auth/clients>.
   Press **Create client**. Application type: **Desktop app**. Name: `formicaria`. Press **Create**,
   then copy the **Client ID** and the **Client secret** from the window that opens (or press
   **Download JSON** to keep them).

## Connect it

1. In Gmail, create a label called `formicaria`, and a filter that applies it to the mail you want
   read: a sender, a mailing list, or a subject. When you create the filter, tick *Also apply to
   matching conversations* so existing mail is included.
2. In formicaria, open **Settings → Connections → Mail**. Paste the Client ID and secret, check the label, choose
   **Read mail from** (two weeks ago by default) and the notebook, and press **Save**.
3. Press **Sign in with Google**. Google opens in a new tab and warns that it has not verified the
   app. That is because the app is your own project. Choose **Continue**. Google lists one
   permission, *read your email messages and settings*. Allow it, then close the tab.

## Things to know

- **If mail stops, Collaboration says so** — a red *Mail stopped* card with Google's reason and a
  **Sign in with Google** button — and so does Settings → Connections → Mail. Nothing new reaches your
  notes until you sign in again.
- **Changing the label or the start date** reads that mail from the beginning. Notes already made
  stay, and nothing is duplicated.
- **Disconnect** stops all reading and asks Google to cancel the sign-in. The notes already made
  stay.

## Stop the weekly sign-out

While your Google Cloud project is in **testing**, Google ends formicaria's sign-in after 7 days
([Google's rule](https://developers.google.com/identity/protocols/oauth2#expiration)). Mail then stops
being read, and replies stop reaching your notes, until you sign in again. To stop it for good,
publish the app. Google first asks for a home page, a privacy policy and terms for it; formicaria's
website has all three, so you can give those. (Check on each page that the project menu at the top
says `formicaria`.)

1. **Branding** — <https://console.cloud.google.com/auth/branding>. Fill in, and press **Save** at
   the bottom:

   | Field | What to put |
   |---|---|
   | Application home page | `https://formicaria-org.github.io/formicaria/` |
   | Application privacy policy link | `https://formicaria-org.github.io/formicaria/privacy.html` |
   | Application terms of service link | `https://formicaria-org.github.io/formicaria/terms.html` |
   | Authorized domains → **Add domain** | `formicaria-org.github.io` |

   App name, support email and developer contact are already there from set-up. Leave the logo
   empty: Google shows one only after it has reviewed the app.
2. **Publish** — <https://console.cloud.google.com/auth/audience>. Under *Publishing status*, press
   **Publish app**, then **Confirm**. The status reads **In production**.
3. In formicaria, press **Sign in with Google** once more — in Collaboration's *Mail stopped* card, or
   in Settings → Connections → Mail. A sign-in made while the project was in testing still ends on its
   7th day; the new one does not.

**What publishing changes, and what it does not.** Nobody else can use your client: they would need its
ID and secret, which only you have. Google keeps showing its *"Google hasn't verified this app"*
screen at sign-in — choose **Continue**, as before — and an unverified app may be used by at most 100
Google accounts in its lifetime
([Google's page on this](https://support.google.com/cloud/answer/15549945)), which one person is far
inside. You do **not** need to send the app to Google for verification.

## The assistant finds the meetings

With the study assistant switched on (Settings → Assistant → Study assistant), it reads each conversation once a
new message arrives, works out **where its meeting stands now** — after the last message — and
**proposes** whatever changed. You find the proposals in
*Collaboration*. Each one shows the date, time and place it would set as fields, next to what the
note has now — and you can **correct them before accepting**: a wrong hour or room is fixed there and
then, not by turning the whole proposal down.

- **The conversation note becomes the first meeting.** Accepting gives it the date, time and place,
  the tag `meeting`, and a short box at the top quoting the sentence the meeting came from. The emails
  below are untouched. It now shows in the Agenda.
- **Each further meeting in the same exchange is proposed as its own meeting note**, linked back to
  the conversation. If you reject it, nothing is created.
- **A time given in another zone is moved onto yours.** "Tuesday 27th at 10am Paris time" is
  proposed at 17:00 if your time zone is Singapore's, and the box shows *17:00 (10:00 Paris time)*.
  The zone must be written right after the time — a city or country and "time", an abbreviation
  such as CET or PT, or an offset such as UTC+2. With none written, the time is taken as yours. Your
  time zone is the one in Settings → Calendars ([Your time zone](./calendars.md#your-time-zone)).
- **If a later email moves the meeting**, the assistant proposes moving the conversation note to the
  new time — even though the old time is still written in the earlier emails above it. A meeting
  that has already taken place is never moved.
- **If a later email keeps the time but sends a place** — a room, or a meeting link such as Zoom —
  the assistant proposes that place for the meeting, and leaves the time as it is. A vaguer word
  ("online") never replaces a link the meeting already has.
- **If a later email calls the meeting off** — "I won't be able to make it", "it will not take place"
  — and no new time is set, the assistant proposes marking it **cancelled**. Nothing is deleted: the
  meeting stays in the Agenda and the Calendar, struck through, so you can still see what was
  planned. If a new time is agreed later, it is proposed as a move, and the meeting is on again.
- **Each conversation gets a proper title, once:** the person you are writing with and who they
  are to you — "Sara Lodi — Master student, robotics project" — in place of the email subject. It is a
  proposal like the rest, with a Title field you can correct; the name must appear in the emails, and
  the description is kept to a few words. A title you gave yourself in the form "Name — …" is left
  alone.
- **Each proposal quotes the sentence it came from, with who wrote it and when**, so you can see at
  a glance whether a time is one you proposed and nobody has answered yet.
- **It keeps up on its own.** Mail is read as soon as formicaria opens and every minute after; new
  mail wakes the assistant at once, and it goes through the changed conversations one after another.
  A reply reaches Collaboration within a minute or two, with nothing to press.
- **Collaboration says what it made of each reply**, under **Mail**: when mail was last checked, and
  for each conversation the email it read up to and its verdict — a proposal, or *nothing to change —
  the meeting stays at …* when a reply only confirms what is already on your Agenda. Tap a line to
  open the conversation.
- **The model never writes a date itself.** It copies the sentence and the words for the day and
  time; formicaria checks that the sentence really is in the emails and works out the date from the
  day that message was sent ("giovedì", "domani", "il 28", "16/10", "next Monday at 3pm"; "next week
on Friday" is the Friday of the following week). If the
  words cannot be read exactly, the meeting is left out rather than guessed. A bare hour from 1 to 7
  ("alle 3") is read as the afternoon.
- **What you reject is not asked again**, and a meeting already in your Agenda (for example, from an
  invitation) is not proposed twice.
- Tasks are yours to create; the assistant only looks for meetings.
