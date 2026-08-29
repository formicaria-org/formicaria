==============================================================================
  FORMICARIA - your notes, as plain files you own
==============================================================================

A notebook for research and writing. Your notes are ordinary text files on
your own computer. No account, no cloud, nothing is sent anywhere.

This folder contains the whole application. Nothing gets installed.


------------------------------------------------------------------------------
  1. START IT
------------------------------------------------------------------------------

  *** UNZIP THIS FOLDER FIRST. ***

  Do not run it from inside the .zip file. Windows opens zips in a temporary
  folder that it deletes afterwards - and it would take your notes with it.
  Right-click the .zip, choose "Extract All...", and pick a real folder such
  as your Documents.

  Then double-click:

      Windows      Start formicaria.vbs
      macOS        Start formicaria.command   (see the note below)
      Linux        Start formicaria.sh

  Your web browser opens by itself. That is the app.

  TO STOP IT:  close the browser tab. It shuts down a few seconds later.

  TO START IT AGAIN:  double-click the same file, any time. Your notes are
  still in the "vault" folder, exactly as you left them.


  YOUR COMPUTER WILL WARN YOU THE FIRST TIME
  ------------------------------------------

  We have not paid Microsoft or Apple to sign this software, so both of them
  warn about it. The warning is about the missing signature - not about
  anything found in the files.

      Windows      Your browser stops you first, at the bottom of the window:
                   "...was blocked" or "...is not commonly downloaded".
                   Click the three dots beside it, then "Keep", then
                   "Keep anyway".

                   Then Windows stops you: a blue box, "Windows protected
                   your PC". There is no visible button. Click the small grey
                   "More info" text, then "Run anyway".

      macOS        macOS refuses to open it and offers you only "Done" and
                   "Move to Trash".

                   *** DO NOT CLICK "MOVE TO TRASH". ***

                   1. Double-click "Start formicaria.command". Click Done.
                   2. Apple menu -> System Settings -> Privacy & Security.
                   3. Scroll to the bottom. A line says formicaria was
                      blocked. Click "Open Anyway" beside it.
                   4. Confirm with Touch ID or your Mac password.
                   5. Double-click "Start formicaria.command" again, then
                      click "Open".

                   Once only. After that a double-click works.

                   Step 3 only appears for a short while after step 1, so do
                   them one after the other.

                   A black Terminal window stays open next to the app. That is
                   normal. Closing it quits formicaria.


------------------------------------------------------------------------------
  2. READ THE MANUAL
------------------------------------------------------------------------------

  Double-click:   Manual.html

  It opens in your browser and explains how to use everything. It works
  offline - it is part of this folder, not a website.

  The manual is also inside the app itself: click the "?" button in the top
  right corner at any time.

  (The "manual" folder next to it holds the pages and pictures the manual is
  built from. You never need to open that folder - use Manual.html.)


------------------------------------------------------------------------------
  3. WHERE YOUR NOTES ARE KEPT
------------------------------------------------------------------------------

  In the "vault" folder, right here inside this one.

  That is what makes this portable: copy this whole folder to a USB stick or
  another computer, and your notes come with it. Nothing is left behind on
  the computer you were using.

  Your notes are the files in  vault\notes\  - one plain text file per note.
  You can open them in any editor. This app is not required to read them.


------------------------------------------------------------------------------
  4. WHAT YOU NEED
------------------------------------------------------------------------------

  A web browser, and a 64-bit computer. That is all.

    - Windows 10 or 11
    - macOS on Apple silicon
    - Linux (Ubuntu 22.04 or newer, or anything of similar age)

  There is nothing else to install. No runtime, no account, no network.


------------------------------------------------------------------------------
  5. OPTIONAL EXTRAS
------------------------------------------------------------------------------

  Writing notes, tasks, dates, the board and search all work with nothing
  else installed at all.

  Two further things are possible if your computer has the right program for
  them. Settings -> "This machine" always tells you which of these this
  computer can currently do.

    KEEPING A HISTORY, AND BACKING UP
      Go back to how a note was last week, back your notes up, or share a
      notebook with someone else. This is the one worth having.
      On Windows it already works - nothing to install.
      On macOS and Linux it needs "git", which most machines already have.

    SEARCHING INSIDE PDFs
      Find a paper by a phrase that is in it, not just by its title.
      PDFs are stored, opened and shown either way; without this their
      contents simply are not searched.
      Needs "poppler".

    BACKING UP PHOTOS AND ATTACHMENTS
      An encrypted, space-efficient backup of the heavy files, separate from
      the notes themselves. Needs "restic", and a place to put the backup.

  Without any of them your notes are still perfectly safe - they are ordinary
  files on your disk - you just do not get that particular extra.


------------------------------------------------------------------------------
  6. IF SOMETHING GOES WRONG
------------------------------------------------------------------------------

  Nothing happens when I double-click
      Windows: try "Start formicaria (show messages).bat" instead. It shows
      what went wrong instead of hiding it.
      Linux: some file managers open scripts in a text editor. Choose "Run",
      or open a terminal in this folder and type:  ./Start\ formicaria.sh

  It says the port is already being used
      Something else on your computer is using address 8765. If it is another
      copy of formicaria, it will just open your browser at the one already
      running.

  The page is blank
      Close the tab and start it again.


------------------------------------------------------------------------------
  ABOUT
------------------------------------------------------------------------------

  No account, no cloud, nothing phones home. The app listens only on your own
  computer (127.0.0.1) and is meant for you alone. Do not expose it to a
  network - it has no password, because it was never meant to need one.

  MIT licensed - see the LICENSE file. THIRD-PARTY.md lists the open-source
  libraries built into these programs, and their notices.

  Source code:  https://github.com/formicaria-org/formicaria
