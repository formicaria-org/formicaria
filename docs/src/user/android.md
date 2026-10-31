# formicaria on an Android phone

It takes about two minutes and a few taps. You need nothing but the phone.

## Installing it

1. **On the phone, open this link and tap it once:**
   **[Download formicaria for Android](https://github.com/formicaria-org/formicaria/releases/latest/download/formicaria-android.apk)**
   (about 30 MB). If the browser asks whether to keep the file, tap **Download** or **Download anyway**.
2. **When the download finishes, tap Open** (or find *formicaria-android.apk* in your Downloads and
   tap it).
3. **Allow the install.** Android says it cannot install apps from this source, because the app did
   not come from the Play Store. Tap **Settings**, switch on **Allow from this source**, then go back.
4. **Tap Install**, then **Open**. That is all.

Some phones add one more screen of their own — a "Play Protect" or virus check that asks whether you
are sure. Choose **Install anyway** (on some phones it is under **More details**). It is the phone being
careful about any app from outside its store, not a sign that something is wrong.

After that, formicaria tells you itself when there is a newer version, and one tap starts its
download — see [Updating](#updating).

There is no Play Store version yet.

**Installed from an app store instead?** If an app store such as F-Droid or Obtainium installed
formicaria, that store keeps it up to date, so formicaria's own check starts off and **Settings →
About & updates** says which store updates it. You can still turn the check on there. Installed from
this website, nothing changes: formicaria checks as described below.

## Updating

**From 0.5.2 on, formicaria tells you when there is a newer version.** Once a day it asks GitHub which
version is newest — nothing about you, the phone or your notes is sent. From 0.22.0 a line at the top
of the app says so; before, only **Settings → About & updates** did. **Check now** in Settings asks
straight away.

1. Tap **Update**. Your browser opens and downloads the newest version — the same file as the
   download link at the top of this page.
2. When the download finishes, tap **Open**, then **Update** on Android's own screen. If the browser
   does not offer **Open**, find *formicaria-android.apk* in your Downloads and tap it. Some phones
   add a "Play Protect" or virus check first, as on the first install.

Android installs it only if it is signed with the same key as the formicaria you have, so nobody can
slip you a fake update this way.

**Before 0.24.4** formicaria downloaded and installed updates itself, and the first time asked
whether it may install apps. It no longer needs that permission. If you allowed it once, you can
switch it off again in Android's **Install unknown apps** setting for formicaria.

Your notes stay where they are: an update keeps the app's storage. **Going back to an earlier version
is not offered on a phone**, because Android can only do that by uninstalling first, and uninstalling
deletes the notes kept inside the app.

A copy older than 0.5.2 cannot do this — install 0.5.2 or later by hand once, and from then on it can.
If you would rather have another app watch for new versions,
[Obtainium](https://github.com/ImranR98/Obtainium) still works: point it at
`github.com/formicaria-org/formicaria`.

### Updating through F-Droid

formicaria has its own F-Droid repository. It holds the same signed app as this website, so a copy
you already have updates from it **without reinstalling**, and your notes stay where they are.

1. On the phone, open
   **[formicaria-org.github.io/formicaria/fdroid](https://formicaria-org.github.io/formicaria/fdroid/)**,
   long-press the long link on that page and choose **Copy link address**.
2. In **F-Droid**, tap **Settings** → **Repositories** → **+**, paste the address and tap **Add**.
3. F-Droid shows the repository's fingerprint. Check that it is
   `5D3F5309005F299E32FBA61D27921AA773BD46675A595CD9C6515022D2AE30B6`, the same as on that page, and
   confirm.
4. Search for **formicaria** in F-Droid: it shows the copy you have as installed, and offers each new
   version from then on.

Once F-Droid has installed an update, **Settings → About & updates** says that F-Droid keeps
formicaria up to date, and formicaria's own check is off unless you turn it on. On Android 12 or
later, F-Droid can then install later updates without asking, if that is switched on in F-Droid's
own settings.

## Your notes, and why backup matters more on a phone

Your notes live inside the app's own storage, not in a folder you can browse. **Deleting the app
deletes them with it**, as with any Android app.

So set up **Back up** on the first day rather than the day you need it. Once your notes have
somewhere to go, the phone stops being the only place they exist, and a phone you drop, replace or
reset costs you nothing. Backing up fetches whatever your other devices have written and sends
yours, in one gesture — see [Backup & versioning](./backup.md).

## Attachments

**A screenshot goes into a note with Share**: tap Share under the screenshot, choose formicaria,
and pick the note (see [Assets & media](./assets.md#on-the-phone-share-from-another-app)). The 📎
button above the keyboard also opens your pictures, with the newest screenshots first.

Photos, recordings and files you attach on the phone travel with your notes, up to the size limit
that vault sets — **Settings → Vaults & backup**, *Send attachments under*. A large backlog is sent in steps
automatically, and a single file over the limit stays on the phone rather than blocking everything
behind it.

## What is on it

Reminders for dated notes show while the app is open; they do not arrive with it closed: see
[Reminders](./notes.md#reminders).

Everything the desktop has for writing: notes, search, dates and boards, attachments, whiteboards,
and backing up. **The study assistant is included too** — it downloads its model on first enable and
runs on the phone itself, with nothing sent anywhere. (That is the one thing the iPhone build cannot
have; iOS does not let an app start a second program.)

## For the careful: checking that you got the real thing

Every formicaria APK is signed with the same certificate. Its fingerprint is:

```text
de040b95933b491e3fb26966025001a8d7c052a526a56ea789e330c1f937bc20
```

You can read that back out of any file you download:

```sh
apksigner verify --print-certs formicaria-<version>-android-arm64.apk
```

The `SHA-256 digest` it prints should match, character for character. If it does not, do not
install it — tell us.

**Why bother.** Android already protects you *after* the first install: it refuses to replace the
app with one signed by a different key, so nobody can push you a fake update. What it cannot do is
protect the first install, because your phone has nothing to compare against yet. Downloads on a
release page can be replaced by anyone with write access to the project, and nothing about how the
app is built changes that. This fingerprint is the baseline that closes the gap, and it is the only
part of the chain that depends on you rather than on us.

*You do not need this to install or use formicaria.* It is for anyone who wants to confirm, with a
computer, that the file is the one we built.
