# Saving media from the web

On a web page, **Save media** keeps its video, or its sound, in your library: a song in **Music**, a
video in **Videos**, or an attachment of a note. Embedded players count too: a news article with a
YouTube video in it offers that video.

**Where:** on a computer, the ant's **Save media on this page**, or right-click a page, a video or
a sound → **Save media in formicaria…**; on the phone, ≡ → **Save media** while a web page is in
front.

## Choosing

The sheet lists what the page holds: its embedded players (YouTube, Vimeo, Dailymotion, SoundCloud,
Bandcamp, podcast players…), its own videos and sounds, the media files and streams it fetched,
and the page itself (which finds what is embedded in it). Choose one; formicaria looks at it (its
title, length, the heights it comes in), then:

- **Keep:** *The video* (choose the height), *The sound, as the site serves it* (the best
  quality, no conversion; AAC or Opus, which every current device plays) or *The sound as MP3*
  (about 190 kbps by default; 192, 256 or 320 kbps to choose).
- **Into:** *Music* for a sound (a playlist chosen, or a new one), *Videos* for a video (a
  collection chosen, or a new one), or *the note* you looked at last.

Press **Save**: the sheet shows it being fetched, converted and put in your library, then **Open**.

## One copy, the final product only

- **Only what you chose is fetched**: the sound alone never downloads the video.
- The work happens in a staging place inside the vault that is never synced or backed up; **only
  the final file enters your library**, and the staging place is emptied when the job ends, fails
  or is cancelled.
- **A file you already have is not kept twice**: the same file saved from two pages is one file.
- The title, artist, year and cover go **inside the file itself**, so any player shows them, and the
  note keeps where it came from (the media's address, and the page it was on).

## Your media in other players

- **On a computer**, formicaria writes each playlist and collection as a `.m3u8` file in the
  vault's `playlists/` folder, pointing at the vault's own files: point any player (VLC,
  foobar2000, Jellyfin…) at the vault and the collections are there.
- **Settings → Connections → Your music in other players** lets any **Subsonic** player
  (Symfonium, DSub, Tempo, Feishin…) stream your Music from this computer, with its own password.
  From a phone it reaches the computer while sharing with a nearby device is on.
- **On the phone**, *Open with…* on a video hands its file to another app, without copying it.

## The tools

formicaria uses **yt-dlp** (which knows over a thousand sites) and **ffmpeg** (which converts and
joins).

- **On a computer** they are fetched the first time you save, about 70–150 MB once, each checked
  against the fingerprint this release of formicaria carries. yt-dlp keeps itself current between
  releases, once a week and again when a site has changed.
- **On the phone** yt-dlp is inside the app (distributed under GPL-3.0 because of it; see the
  README), and updates itself there too. The phone needs no ffmpeg: it makes MP3s and writes the
  tags itself, and joins a video's picture and sound with Android's own tools.

**What cannot be kept:** media protected against copying (DRM: films and series from streaming
services, music services' songs), and live streams while they run. What you may keep from a site
is yours to judge.

When a site changes how it serves its media, saving from it fails until yt-dlp catches up, usually
within days; the sheet says so.
