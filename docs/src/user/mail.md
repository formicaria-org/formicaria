# Mail (Gmail)

formicaria can read your Gmail and keep a note for each conversation you care about. Invitations
inside messages become meetings straight away. It reads **only** mail with a label you choose, from a
date you choose, and it can **only read**: Google gives it no way to send, delete or change mail.

## What it does to your notes

- **One note per conversation**, tagged `mail`, titled by its subject (without `Re:`, `R:`, `Fwd:`).
  It holds **the whole exchange**: every message with its sender, recipients, date and time,
  attachment names (the files themselves are not copied) and text. Quoted earlier messages are trimmed
  from replies, since they are already above. A link opens the conversation in Gmail.
- **New replies are added at the bottom.** Anything you write in the note is never changed or moved,
  and reading the same mail again adds nothing.
- **An invitation in a message becomes a meeting note**, exactly as if it came from your calendar.
  An updated invitation moves it, and a cancellation tags it `cancelled`. If the same meeting is also
  in your Google Calendar, it stays **one** note.
- Mail is read every 15 minutes while formicaria is open, up to 200 new messages each time.

**Where the text goes.** The exchange is part of your notes, so it goes wherever the chosen notebook
goes. If that notebook is copied online (for example to GitHub), the emails, including what other
people wrote to you, are copied there too. They stay in the notebook's history even after a note is
deleted, and reach anyone you share the notebook with. To keep mail on this computer only, choose a
notebook that has no online copy under *Into*.

## One-time set-up: your own Google Cloud client (about 10 minutes)

Google only lets an app read Gmail through a "client" registered with Google. You create your own, so
the arrangement stays between you and Google. If the console is in another language, add `?hl=en` to
its address to see the names used below.

1. Open <https://console.cloud.google.com>, choose the project menu at the top, then **New project**.
   Name it `formicaria` and create it.
2. Open **APIs & Services → Library**, search for **Gmail API**, and press **Enable**.
3. Open **Google Auth Platform** (also reachable as *OAuth consent screen*) and press **Get started**.
   App name: `formicaria`. Support email: your Gmail address. Audience: **External**. Contact
   email: yours. Agree and create.
4. In **Audience**, under **Test users**, add your own Gmail address.
5. In **Data access**, choose **Add or remove scopes**, filter for `gmail.readonly`, tick
   `…/auth/gmail.readonly`, and save. Add nothing else.
6. In **Clients**, choose **Create client**. Application type: **Desktop app**. Name: `formicaria`.
   Create it, then copy the **Client ID** and **Client secret**.

## Connect it

1. In Gmail, create a label called `formicaria`, and a filter that applies it to the mail you want
   read: a sender, a mailing list, or a subject. When you create the filter, tick *Also apply to
   matching conversations* so existing mail is included.
2. In formicaria, open **Settings → Mail**. Paste the Client ID and secret, check the label, choose
   **Read mail from** (two weeks ago by default) and the notebook, and press **Save**.
3. Press **Sign in with Google**. Google opens in a new tab and warns that it has not verified the
   app. That is because the app is your own project. Choose **Continue**. Google lists one
   permission, *read your email messages and settings*. Allow it, then close the tab.

## Things to know

- **Every 7 days Google asks you to sign in again**, while your Google Cloud project is in testing.
  Settings → Mail says when; press *Sign in with Google* again.
- **Changing the label or the start date** reads that mail from the beginning. Notes already made
  stay, and nothing is duplicated.
- **Disconnect** stops all reading and asks Google to cancel the sign-in. The notes already made
  stay.

## The assistant finds the meetings

With the study assistant switched on (Settings → Study assistant), it reads each conversation once a
new message arrives, and **proposes** the meetings the exchange fixes. You find the proposals in
*Collaboration*, and each one says the date, time and place it would set, in plain view.

- **The conversation note becomes the first meeting.** Accepting gives it the date, time and place,
  the tag `meeting`, and a short box at the top quoting the sentence the meeting came from. The emails
  below are untouched. It now shows in the Agenda.
- **Each further meeting in the same exchange is proposed as its own meeting note**, linked back to
  the conversation. If you reject it, nothing is created.
- **If a later email moves the meeting**, the assistant proposes moving the conversation note to the
  new time. A meeting that has already taken place is never moved.
- **The model never writes a date itself.** It copies the sentence and the words for the day and
  time; formicaria checks that the sentence really is in the emails and works out the date from the
  day that message was sent ("giovedì", "domani", "il 28", "16/10", "next Monday at 3pm"). If the
  words cannot be read exactly, the meeting is left out rather than guessed. A bare hour from 1 to 7
  ("alle 3") is read as the afternoon.
- **What you reject is not asked again**, and a meeting already in your Agenda (for example, from an
  invitation) is not proposed twice.
- Tasks are yours to create; the assistant only looks for meetings.
