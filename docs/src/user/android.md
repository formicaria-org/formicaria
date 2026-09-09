# formicaria on an Android phone

The Android app is on the
[releases page](https://github.com/formicaria-org/formicaria/releases), as a `.apk` file beside the
Windows, Mac and Linux downloads. There is no Play Store version.

Android is one of the two systems formicaria is used on every day, so of everything here it is the
best tested — alongside Linux, and unlike the rest.

## Installing it

Download `formicaria-…-android-arm64.apk` on the phone and open it. Android will ask whether to
allow installing apps from wherever you downloaded it — your browser, usually — because the app did
not come from the Play Store. Allow it, and the install proceeds normally.

Updating is the same gesture: download the newer file and open it. Your notes stay where they are.

## There is no automatic update

Nothing tells you when a new version exists — you have to look. If that sounds like something you
will forget, [Obtainium](https://github.com/ImranR98/Obtainium) is an app that watches a GitHub
project for you and offers the update when one appears. Point it at
`github.com/formicaria-org/formicaria` and it will track releases from then on.

## Your notes, and why backup matters more on a phone

Your notes live inside the app's own storage, not in a folder you can browse. **Deleting the app
deletes them with it**, as with any Android app.

So set up **Back up** on the first day rather than the day you need it. Once your notes have
somewhere to go, the phone stops being the only place they exist, and a phone you drop, replace or
reset costs you nothing. Backing up fetches whatever your other devices have written and sends
yours, in one gesture — see [Backup & versioning](./backup.md).

## Attachments

Photos, recordings and files you attach on the phone travel with your notes, up to the size limit
that vault sets — **Backup options → attachment settings**. A large backlog is sent in steps
automatically, and a single file over the limit stays on the phone rather than blocking everything
behind it.

## What is on it

Everything the desktop has for writing: notes, search, dates and boards, attachments, whiteboards,
and backing up. **The study assistant is included too** — it downloads its model on first enable and
runs on the phone itself, with nothing sent anywhere. (That is the one thing the iPhone build cannot
have; iOS does not let an app start a second program.)
