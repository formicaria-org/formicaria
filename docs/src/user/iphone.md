# formicaria on an iPhone

There is no App Store version, and there will not be one. Instead you download the app file from
the release page and **sign it with your own Apple ID** — free, no developer programme, no payment.
The app then runs on your phone like any other.

This is a real route, used by a lot of open-source apps, and it has a real cost: **the app stops
opening after seven days unless it is refreshed.** Read that part before you start.

## Before you begin, know what is proven

The iPhone app is built and checked automatically — the file is verified to be the right shape for
your phone before it is published. But **nobody here owns an iPhone.** Everything below comes from
people who have done it and reported back, not from a machine that tests it. Two people have run
formicaria on an iPhone and said it worked.

If something does not match what you see, that is worth telling us — it is the only way this page
gets better.

## What you need

- An iPhone or iPad, and a computer to sign the app from.
- An **Apple ID**. The free kind is enough.
- A sideloading tool, depending on your computer:

  | Your computer | Tool |
  |---|---|
  | Windows or Mac | **SideStore** — refreshes over Wi-Fi, so you need the computer only for the first setup. Or **AltStore**, which needs the computer running each time it refreshes. |
  | Linux | **AltServer-Linux**, **Sideloader**, or `xtool install`, plus `libimobiledevice` and `usbmuxd`. |

- The file itself: **`formicaria-…ipa`**, from the
  [releases page](https://github.com/formicaria-org/formicaria/releases), beside the Windows, Mac
  and Linux downloads.

## The seven-day part

An app signed with a free Apple ID is trusted for **seven days**. After that it will not open until
it is refreshed — the same file, signed again. Nothing is wrong and nothing is broken; that is
Apple's limit on free signing, and every app installed this way lives with it.

**SideStore refreshes in the background over Wi-Fi**, which is what turns this from a weekly chore
into something you rarely think about. If you use AltStore instead, refreshing means opening
AltServer on your computer with the phone on the same network.

Two more limits worth knowing up front: a phone can hold **at most three** apps installed this way,
and a free Apple ID can register ten app identifiers in total.

## Your notes, and why backup matters more here

**Refreshing the app keeps everything.** Your notes stay exactly where they were.

**Deleting the app removes its notes with it**, as with any iPhone app — the notes live inside the
app's own storage, not in a folder you can browse. So set up **Back up** early, on the first day
rather than the day you need it. Once your notes have somewhere to go, the phone stops being the
only place they exist, and a phone you replace or an app you remove costs you nothing.

Backing up is under the **Back up** button: add a destination, and from then on backing up fetches
whatever your other devices have written and sends yours. See
[Backup & versioning](./backup.md).

## If it will not open

- **It has been more than seven days.** Refresh it with the tool you installed it from. This is by
  far the most common cause.
- **"Untrusted Developer" on first launch.** Settings → General → VPN & Device Management, find
  your Apple ID, and trust it. Once only.
- **Three apps already.** A phone holds three sideloaded apps at a time; remove one first.

## What this app does not have

The iPhone build carries the notebook: writing, searching, dates and boards, attachments, and
backing up.

**The study assistant is not on it**, and will not be. It runs a separate program alongside the app
to do its thinking, and iOS does not allow an app to start one. That is a rule of the platform, not
something waiting to be built. The Android app does have it.
