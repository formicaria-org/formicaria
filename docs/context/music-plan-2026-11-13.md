# Music — a song is a logged note, playlists are notes, a player view (plan, 2026-11-13)

> **Status, 2026-11-13: M1–M5 ruled as proposed** (`decisions.md`, *a sixth group, Music*), with
> one change from Part 4: a song's file is uploaded **like any other attachment**, so it has its
> asset note, and the song note embeds it. **Phase 2 is built**: tags read at upload, the song
> note, `music_library`, Songs and *Add a song*. Phases 3–8 are not. Where this disagrees with the
> code, the code wins.

**The ask (owner, 2026-11-13):**
- *"a song is just an asset, so it can be viewed and played seamlessly … an efficient playlist
  creator (it's just a note) and basically a view common to any mp3 player"*;
- refined the same day: *"the music view would allow to add a song with the plus. It should try to
  get its metadata to fill it (a song is a special note, exactly as in food). I want a very fast and
  clear view that allows to use formicaria to play the music and organize it."*

**So the model is food's** (`decisions.md#data`, 2026-10-11):
- a song is a **note with the reserved `log` property** (`log: music`), the way a meal is `log: food`;
- the audio file is that note's **asset**;
- the plus opens an **Add a song** sheet that fills itself from the file, the way the food sheet
  fills itself from a label or a barcode.

## Part 1 — What music apps do (the survey, condensed)

A survey of ~40 players (desktop, Android, self-hosted, note-centric), read at source. Only what
shapes this plan is kept here; links are pages that were actually opened.

**The view set every player has.** Songs · Albums · Artists · Playlists (Folders is the most asked
addition), search, cover art, a visible queue with *Play next* / *Add to queue*, shuffle and repeat.
- **Phone:** tabs, a **mini-player bar** that opens a **now-playing sheet**, long-press
  multi-select with a per-row menu, drag handles to reorder, and controls on the lock screen and
  notification.
- **Desktop:** a sidebar of playlists, a sortable track table, a queue side panel, keyboard
  shortcuts, drag-and-drop and media keys.

**Playlists: one plain-text format travels — extended M3U in UTF-8 (`.m3u8`, `#EXTM3U`).** A
relative path resolves from the playlist file's own folder ([M3U](https://en.wikipedia.org/wiki/M3U)).
Paths are where every app breaks:
- Android players fight storage paths ([Auxio #932](https://github.com/OxygenCobalt/Auxio/issues/932),
  [Gramophone #736](https://github.com/FoedusProgramme/Gramophone/issues/736)).
- Android deprecated its own playlist table and points to M3U files
  ([MediaStore.Audio.Playlists](https://developer.android.com/reference/android/provider/MediaStore.Audio.Playlists)).
- Auxio's maintainer refused two-way file⇄database playlist sync as duplicated state
  ([Auxio PR 1411](https://github.com/OxygenCobalt/Auxio/pull/1411)).

**→ One source of truth (the note), M3U only as export and import.**

**What users complain about:**
- no gapless playback;
- playlists lost after moving files or devices;
- broken Android paths;
- poor multi-artist tags;
- slow scans;
- clunky playlist building.

**An efficient playlist builder is a set of habits, not one app.** The habits that recur:
- multi-select, then one *Add to playlist…*;
- add from anywhere in one step, the now-playing sheet included;
- drag into a playlist and drag to reorder;
- *save the queue as a playlist*;
- rule-based playlists kept as text
  ([Navidrome smart playlists](https://www.navidrome.org/docs/usage/features/smart-playlists/),
  [beets smartplaylist](https://beets.readthedocs.io/en/stable/plugins/smartplaylist.html));
- Quod Libet's "songs in no playlist" search as an inbox to sort
  ([searching](https://quodlibet.readthedocs.io/en/latest/guide/searching.html)).

**Web audio has two hard limits:**
1. **A plain `<audio>` element is not gapless.** The hybrid approaches use Web Audio or Media
   Source Extensions ([Gapless-5](https://github.com/regosen/Gapless-5)). Web-stack players
   (Feishin, Supersonic, [Musicat](https://github.com/basharovV/musicat), the last on Tauri +
   Svelte like us) moved playback to native code for exactly this.
2. **Android WebView has no Media Session API**
   ([browser-compat-data](https://raw.githubusercontent.com/mdn/browser-compat-data/main/api/MediaSession.json)).
   It cannot drive the lock screen or the notification, and its audio stops when the app goes to
   the background. Android's answer is a Media3 player in a foreground media service
   ([background playback](https://developer.android.com/media/media3/session/background-playback)).


**Where song details can come from** (official pages, read 2026-11-13):
- **The file's own tags** (ID3, Vorbis comments, MP4 atoms): offline, and usually right for
  bought or ripped music. Read with a tag library, no network.
- **MusicBrainz:**
  - **Access:** no API key, but a User-Agent naming the app and a contact is required, at most
    **one request per second** ([API](https://musicbrainz.org/doc/MusicBrainz_API),
    [rate limiting](https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting)).
  - **Search:** `https://musicbrainz.org/ws/2/recording?query=…&fmt=json` (Lucene syntax,
    [search](https://musicbrainz.org/doc/MusicBrainz_API/Search)).
  - **Licence:** core data (titles, artist credits, durations, releases and their track lists) is
    **CC0**. Tags and genres are **CC BY-NC-SA 3.0**
    ([data licence](https://musicbrainz.org/doc/About/Data_License),
    [database](https://musicbrainz.org/doc/MusicBrainz_Database)).
- **Cover Art Archive:** `coverartarchive.org/release/<mbid>/front-250`. *"All images are
  copyrighted by their respective copyright owners"* ([home](https://coverartarchive.org/),
  [API](https://musicbrainz.org/doc/Cover_Art_Archive/API)). No licence is granted.
- **AcoustID** (identify by sound):
  - needs a **registered application key** and is *"for non-commercial use only"*, at most 3
    requests per second ([web service](https://acoustid.org/webservice));
  - the client must decode the audio and fingerprint it with Chromaprint, which is LGPL 2.1
    ([licence](https://acoustid.org/license)), and its database is CC BY-SA 3.0.
- **LRCLIB** (synced lyrics): no key, an identifying User-Agent, about 90 requests a minute per IP
  ([docs](https://lrclib.net/docs.md)). No licence for the lyrics text was found.

## Part 2 — What formicaria already carries

| Need | Today | Gap |
|---|---|---|
| A song is a special note | logged entries: a note with `log`, whole-number values, `create_notes` / `set_properties` / `log_recent`; `not_logged()` already keeps them out of board, timeline and general search (`fm-app/src/thread.rs`) | `log_days` sums by day, which means nothing for songs; the library needs its own list (Part 4) |
| A group with a plus | view groups: views + one `log` filter + one `add` action (`decisions.md#ui`, 2026-10-12), shipped `.view` files | none: `log: music` + `add: song` fits the rule as written |
| The audio file | chunked upload → `ingest_finish` streams and hashes; blob under `sha256:`, served with `Range` | the upload today makes an *asset note*; a song wants the blob attached to the song note instead |
| Fill from the file | the food sheet fills from a label photo and a barcode | nothing reads audio tags |
| Look it up online | the barcode precedent: one fixed address, only on a press, the shells fetch, `fm-app` parses, a CI guard on the address (`decisions.md#data`, 2026-10-12) | the same, for MusicBrainz |
| Play it | `render.ts` draws `<audio controls>` per note | stops when the note closes; no queue |
| Phone in background | `AgentService.kt` foreground service, injected by `ci/android-inject-service.sh` | a media service needs `FOREGROUND_SERVICE_MEDIA_PLAYBACK`, outside the Play Protect list in `ci/checks.sh` |

## Part 3 — Questions to rule first

**M1. Where the audio lives.**
- **Recommended: imported into the vault's `blobs/`,** attached to the song note. It is
  deduplicated by hash and backed up by restic.
- **Cost:** it is a second copy of a library kept elsewhere. Blobs over `git_assets_max` (ceiling
  100 MB per file, notes-only by default) do not travel with git. So **the song notes reach the
  phone, the audio does not** until it is added there too; the phone shows such a song as *not on
  this device*.
- **The alternative,** a music folder read in place, is a new kind of thing (a path outside the
  vault, files changing under the index) and needs its own reversal entry.

**M2. How a song fills itself.** Two steps, food's pattern:
1. **On adding: the file's tags, offline.** Title, artist, album, album artist, track, disc, year,
   genre, duration and the embedded cover. Nothing leaves the device. This needs a tag library, the
   pure-Rust `lofty` crate (licence to confirm in `deny.toml`), which is a **new dependency and so a
   `decisions.md` entry**. Where there are no tags, the filename is read as `Artist - Title`.
2. **On a press: *Look up*, MusicBrainz.** One recording search, built from what the form holds
   (artist, title, duration), to one fixed address.
   - The person picks a match from a short list; the match fills only the empty or chosen fields;
     **the person saves**.
   - The shells fetch with `fm-fetch`, and `fm-app` only parses the answer; a `ci/checks.sh` guard
     allows that one address.
   - **Taken:** core CC0 fields only (title, artist, album, track, year, the recording's MusicBrainz
     id as `musicbrainz`).
   - **Not taken:** genres and tags (NC-SA), cover images (no licence granted), and nothing is sent
     back.
   - The sheet says before the press what is sent (artist and title, to musicbrainz.org) and keeps
     one request per second.
   - **Not proposed:** AcoustID. It needs an app key under a non-commercial-only term and an LGPL
     fingerprinter that decodes audio.

**M3. The player is app chrome.** The group stays inside its ruling (views + `log: music` + *Add a
song*). The player that keeps playing while the person moves through views does not belong to the
group. Proposed:
- the **player is the app's own**, like the undo strip;
- the **queue is device state** in `localStorage`, like board order, never vault data.

**M4. The phone in the background.**
- **(a) In v1 the phone plays while the app is in front,** and says so.
- **(b) Then a native Media3 player in a foreground media service,** with the web view as its
  remote control: lock screen, notification, headphone buttons.
- **(b) widens what ships:** a new permission, `androidx.media3` and a service Play Protect will
  read. So it needs a `decisions.md` entry, then the allowed list, then a phone test.

**M5. Gapless is not in v1.** A short gap between tracks is accepted at first. Gapless on the
desktop is a later phase; on the phone it comes with M4(b).

## Part 4 — The design

### The song note

```markdown
---
title: Song A
log: music
artist: Artist X
album: Album Y
album_artist: Artist X
track: 3
disc: 1
year: 2019
genre: Jazz
duration_s: 214
musicbrainz: 0a1b…        # only after a look-up the person kept
assets: [sha256:…audio, sha256:…cover]
---
![Song A](asset:sha256:…audio)
```

- **It is readable and playable without the Music view.** Opened as a note, its body already
  renders a player.
- **Edits go to the note, never to the file.** A blob is content-addressed: rewriting a tag would
  change its hash.
- **Adding the same file twice finds the existing song,** by its audio hash, and offers to open it
  rather than making a second note.
- **The existing `asset_note` path is not used** for songs: no second, asset-kind note per file.

### The Add a song sheet (the plus, on every Music view)

1. **Choose files.** One or many; on desktop also drag files onto the window.
2. **Each file is read at once:** tags, cover, duration. The sheet shows the filled form.
   - **One file:** the form, with *Look up* beside the title. Save.
   - **Many files:** a list, one line each (title · artist · album · ✓ or *needs a look*). *Save
     all* saves them as read. Any line opens its form, with *Look up*.
3. **Saved songs appear in the list at once,** while the bytes finish uploading behind them; the
   line shows progress.

It is food's `LogSheet` pattern, in `ui/src/lib/music/AddSong.svelte`: a form, not a renderer.

### The views — fast and clear

**Four tabs, the same names on phone and desktop: Songs · Albums · Artists · Playlists.**

**Songs (the opening view):**
- **a search field first,** filtering as you type, the way *Search a food to log* comes first in
  Food;
- **then the list:** title, artist, album, duration; sortable on desktop;
- **pressing a row plays it,** with the rest of the visible list as the queue;
- **long-press, or Ctrl/⌘-click, selects several.**

**Albums:** a grid of covers. Pressing one opens its track list in order, with *Play* and *Shuffle*.

**Artists:** a list. Pressing one shows its albums, then its songs.

**Playlists:** a list of playlist notes. Pressing one opens it: *Play*, *Shuffle*, drag to reorder.

**Fast, measured:**
- **One request per screen** (the `perf.rs` rule): a `music_library` command returns every song's
  display fields in one pass over the `log: music` notes.
- **The UI filters, sorts and groups that list in memory.** Typing never waits on the server.
- **Long lists are virtualised** (only visible rows drawn).
- **Target, measured in `perf.rs` and Vitest with 5,000 generated songs:** first paint of Songs
  under 300 ms on the desktop, filtering under 50 ms per keystroke.

**Generic where it can be:**
- The lists are drawn by one literal-free renderer, **`tracks`**. Its columns and grouping come
  from the `.view` file (`columns: [title, artist, album, duration_s]`, `group_by: album`).
- The domain parts live in `ui/src/lib/music/`: the sheet, the player, the queue, the playlist
  editor.

### Playlists

A playlist is a note tagged `playlist`. Its body is a numbered list of links to song notes:

```markdown
---
title: Long drive
tags: [playlist]
---
1. [[Song A]]
2. [[Song B]]
```

- **Order is the list's order.** Adding, removing and reordering are body edits through
  `undo.svelte.ts`, so Undo works.
- **Two devices adding to one playlist** merge like any note (`*.md merge=fm`).
- **The link syntax** (`[[…]]` or an id link) follows what notes use today for linking a note.
  To check before building.
- **Building a playlist, the efficient habits** (Part 1):
  - select several, then *Add to playlist…*, recent playlists first, with *+ New*;
  - *Add to playlist* in the now-playing sheet;
  - *Play next* / *Add to queue* on every row;
  - drag to reorder;
  - *Save queue as playlist*.
- **Out and in:** export writes a `.m3u8` (`#EXTINF`, relative paths, songs copied out by
  name). Import matches entries by audio hash, then filename, then artist + title, and lists what
  it could not match.

### The player (app chrome, M3)

- **Desktop:** a slim bar along the bottom (cover, title · artist, ⏮ ⏯ ⏭, progress, volume), a
  queue panel on demand, media keys through the browser's Media Session API, and Space / ← / →
  shortcuts.
- **Phone:** a mini-player above the app's bar. Tapping it opens the now-playing sheet (cover,
  progress, queue, *Add to playlist*).
- **The player bar appears only once something plays,** and nothing is laid over the existing
  views until then (`decisions.md#ui`, 2026-10-12: leave working views alone).

## Part 5 — Phases

Each phase ends green on `pixi run ci` and is **shown to the owner before the next builds on it**.

1. **Rulings.** M1–M5 as dated `decisions.md` entries.
2. **Songs in.**
   - Tag reading at ingest (`lofty`), the song note, an upload that attaches the blob to it, and
     duplicate detection by hash.
   - The Add a song sheet for one and for many files.
   - Tests with **generated** files (a second of silence with invented tags), so no music is
     committed.
3. **Songs view and the desktop player.**
   - The Music group (`log: music`, `add: song`), `music_library`, the `tracks` renderer and Songs.
   - The player bar, the queue, Media Session, and the performance targets measured.
4. **Albums, Artists, Playlists.** Multi-select, *Add to playlist*, reorder with Undo, *Save
   queue*, M3U8 out and in.
5. ***Look up* (MusicBrainz).** The shell fetch, the parser, the address guard and the attribution
   line.
6. **Phone, foreground.** The phone layout, playback in front, and a ranged fetch measured on a
   device.
7. **Phone, background** (after M4b). Media3 `MediaSessionService` injected like `AgentService`, the
   Play Protect list widened by its own entry, and a phone test.
8. **Later, each on request:** gapless on the desktop, smart playlists (a saved query, kept as
   text), synced lyrics (LRCLIB on a press, kept as an `.lrc` asset), a Folders view.

**Out of scope:**
- streaming services;
- writing tags back into files;
- transcoding;
- cover images from the Cover Art Archive;
- anything sent without a press.

## Part 6 — Risks

- **Disk.** M1 doubles any library already kept elsewhere. The sheet states the total size before
  it copies many files.
- **Index.** 5,000 song notes enter the store and the FTS index. `not_logged()` keeps them out of
  the general views, but every vault pass still reads them. Measure in `perf.rs` before phase 3
  ships.
- **Phone bytes.** The phone's byte path is base64 over IPC (`features.md`, *Media / blobs*). A
  4-minute song played through it must be measured; it may need a direct file URL.
- **A volunteer database.** A MusicBrainz match can be wrong. That is why the person picks the
  match and saves, as with Open Food Facts.
- **Play Protect (M4b).** A media service is ordinary for a player, but this APK's history makes a
  phone test mandatory before release.
