# Plan (detailed): privacy switches, ad and tracker blocking, media from any site, kept as real artifacts in the library

> **Status, 2026-11-17: built, phases 0–9, released together.** What building it decided is
> `decisions.md`, *2026-11-17 (later) — the media and privacy plan, as built*; the phone engine's
> measures are `phone-blocking-2026-11-17.md`; what is not yet tried on a phone is in
> `known-issues.md`. Where this disagrees with the code, the code wins.

Owner, 2026-11-17: ad block and app tracker block; get media (video, audio, audio as MP3) from any
website, embedded players included; keep real artifacts, one copy of heavy things, only the final
product; organised in collections like songs, playable by one's own player; privacy switches like
DuckDuckGo, easy on and off. *"Implement whole plan then, if needed plan in detail first."*
The phases are built in order and released together (owner: one release, not ten).

## What exists (read 2026-11-17) — this builds on it

- **Music** (`music-plan-2026-11-13.md`, rulings M1–M5): a song is a note `log: music` whose `audio`
  is an asset note; Songs · Albums · Artists · Playlists (`crates/fm-app/src/music.rs`,
  `ui/src/lib/music/`); tags read at upload; `songDraft.ts`; playlists are tagged notes; **M3U8 out
  and in not built**; **on the phone music stops in the background** (M4(b): a Media3 player in a
  foreground media service, planned).
- **Assets**: content-addressed blobs under `vault/blobs/` (`ingest`, `ingest_chunk/finish/cancel`,
  no size limit), each with its asset note; same bytes = one blob. Blobs travel by restic snapshot,
  and by git only under the vault's `git_assets_max`.
- **The add-on** already has `downloads` and `<all_urls>`; every download asks where it goes, and a
  song downloaded into formicaria becomes a song note (`songDraft`, since 0.32.0).
- **Save page**: `lib/web/read.ts` (Readability, Turndown, a script-free HTML copy with `<base>`,
  images still remote) → `clip_save`. The phone has *Save this page* (0.50).
- **Phone shell**: page WebViews per tab (`pageFor`), `addDocumentStartJavaScript` in use,
  `androidx.webkit` 1.14, the foreground `AgentService`. **No JNI from Kotlin into Rust yet.**
- **Hard constraints found**:
  1. **Android 10+ forbids running a program downloaded into app storage** (W^X): native tools must
     ship inside the APK (`jniLibs`), as the agent's runtime already does (`agent.rs`). So *"yt-dlp
     as an optional download on the phone"* is **not possible** for ffmpeg/Python; only yt-dlp's
     own Python code can update itself once a Python runtime is in the APK.
  2. **Play Protect** flagged the APK once (`known-issues.md`; `ci/checks.sh` allows only
     `FOREGROUND_SERVICE`, `…_DATA_SYNC`, `POST_NOTIFICATIONS`, `RECORD_AUDIO`,
     `MODIFY_AUDIO_SETTINGS`). A **VPN service** (`BIND_VPN_SERVICE`) and a **media playback**
     service (`FOREGROUND_SERVICE_MEDIA_PLAYBACK`) widen that list; a sideloaded VPN app is exactly
     what Play Protect scrutinises.
  3. *A vault holds every byte it owns* (2026-10-15): no file outside the vault, no second copy.

## Part A — Privacy (≡ → Privacy; Settings → Privacy) — both devices

One sheet listing each protection with a switch, and what it did on the page in front. Per site:
*Protections off for this site* (remembered, listed, undoable). Settings are per device
(`localStorage` + the phone shell's prefs), not in the vault: privacy choices are the device's.

| Switch | Phone (WebView) | Computer (formicaria's Firefox) | Default |
|---|---|---|---|
| Private search | the web field's engine: DuckDuckGo, Startpage, Brave Search, Mojeek, Kagi, custom | the same list as Firefox's default engine (policy `SearchEngines`) + `/f/new`'s field | DuckDuckGo |
| Ad block | Part B | uBlock Origin switch (exists) | off until B's figures |
| Tracker block | Part B with EasyPrivacy | Firefox strict tracking protection (exists, shown here) | on |
| Cookie pop-ups | DuckDuckGo `autoconsent` rules injected at document start | Firefox's cookie-banner handling (`cookiebanners.service.mode`) | on |
| Clean links | strip `utm_*`, `fbclid`, `gclid`, `mc_eid`… on navigation (`shouldOverrideUrlLoading`) | Firefox's query stripping (`privacy.query_stripping.enabled`) | on |
| Global Privacy Control | `Sec-GPC: 1` header + `navigator.globalPrivacyControl` | `privacy.globalprivacycontrol.enabled` | on |
| HTTPS only | upgrade http→https; a page explaining before an http-only site | `dom.security.https_only_mode` | on |
| Fire button | clear WebView cookies, storage, cache, history (notes and kept files untouched) | Firefox's *Forget* offered in ≡ | button |
| App tracker block | Part C | — | off |

Licences to verify in step A0: `autoconsent` (DuckDuckGo, expected MPL-2.0); the link-parameter
list (our own short list if ClearURLs' LGPL rules are awkward).

## Part B — Ad and tracker block on phone pages

- **Engine**: `adblock` crate (Brave's `adblock-rust`, MPL-2.0) in a new small crate
  `crates/fm-block` (pure: rules in → verdicts out; no fs, no network), used by the phone shell.
- **The bridge**: a JNI function exported from the app's Rust library
  (`check(url, source_url, type) -> boolean`, `hide(url) -> String` for cosmetic selectors) via
  the `jni` crate; Kotlin `object Blocker { external fun check(…) }`. `shouldInterceptRequest`
  returns an empty `WebResourceResponse` for a block; a document-start script asks for the page's
  hide selectors and adds one `<style>`.
- **Lists**: subscribed, fetched on the phone (fm-fetch), weekly, into app storage; the compiled
  engine serialised next to them and loaded at start; EasyPrivacy for *Tracker block*, EasyList for
  *Ad block*; a custom list address allowed. Not shipped in the APK (licences, size).
- **Counts** per page for the Privacy sheet.
- **Measured (B0)**: APK growth, memory with both lists, time per check, start time; the default for
  ad block set by the figures (the computer's precedent: +66 % memory → off by default).

## Part C — App tracker block (whole phone)

- A `VpnService` in the shell that **only handles DNS**: the phone's DNS server is the VPN's own
  address; queries for tracker hosts get `0.0.0.0`, others are forwarded to the network's resolver.
  DNS-only keeps it light (no forwarding of all traffic as DuckDuckGo/TrackerControl do), at the
  cost of missing trackers reached by fixed IPs or by an app's own encrypted DNS (said in Settings).
- Host decisions from `fm-block`'s host rules (EasyPrivacy's domain rules + a mobile tracker hosts
  list, licence checked in C0). Per-app attribution (`getConnectionOwnerUid`, API 29+) → *which
  apps tried which trackers*; *let this app through* (`addDisallowedApplication`).
- **Costs said**: one VPN at a time (a real VPN cannot run alongside), Android's key icon, a
  permission asked once, battery (measured).
- **Play Protect**: `BIND_VPN_SERVICE` + a foreground service type widen the allowed list.
  **Owner decision** (below).

## Part D — Media from any site, kept as one final file

**The sheet** (≡ → *Save media* on a web page; computer: the ant menu and the page's context menu)
lists what the page holds:
1. its own `<video>`, `<audio>`, `<source>`, large images;
2. **embedded players**: iframes of YouTube (incl. `youtube-nocookie`), Vimeo, Dailymotion,
   SoundCloud, Bandcamp, podcast players, and any iframe yt-dlp recognises;
3. media requests seen while the page played (phone: `shouldInterceptRequest`; computer: the
   add-on's `webRequest`, a new permission) — files and HLS/DASH manifests.

Each entry offers **Video** (best / 1080 / 720 / 480), **Audio** (as served: `.m4a`/`.opus`, no
conversion), **Audio as MP3** (VBR V2 default; 192/256/320 in Settings), and **where it goes**:
*Music* (a playlist chosen or new), *Videos* (a collection), or *a note* (attached to the note
looked at last, as images are).

**How it is fetched** — one job runner (start, progress, cancel), two layers:
- **Direct** (a plain file): downloaded by the server (fm-fetch), streamed into an ingest session
  (`fm_core::chunked`): no temporary copy at all.
- **yt-dlp** (embeds, streams, the large sites): only the chosen format; `-x --audio-format mp3
  --audio-quality 2` for MP3; `--embed-metadata --embed-thumbnail` (tags and cover inside the
  file); separate streams joined by remux. Output into the app's cache, then **ingested** into the
  vault's blobs; the cache cleared on end, failure or cancel. Free space checked first.
- **No DRM**: yt-dlp refuses it; the sheet says so for those sites.

**The tools**:
- **Computer**: `yt-dlp` standalone release binaries (Linux, Windows, macOS) and `ffmpeg` per
  platform (yt-dlp's `FFmpeg-Builds` for Linux/Windows; a pinned static build for macOS), fetched
  on first use into formicaria's data dir, **pinned by SHA-256 in each release** (a `media-bump`
  step beside `browser-bump`), plus **yt-dlp self-update between releases** (its `SHA2-256SUMS`
  checked against its signature with yt-dlp's published key, verified in Rust so Windows needs no
  gpgv); the previous binary kept to fall back.
- **Phone**: decision 1 below.

**Into the library**:
- **Audio → a song note** (`songDraft`): title, artist (uploader/channel or the page's site name),
  album (the series, or the playlist), year, cover from the thumbnail, `source:` the media's address,
  `page:` the article; added to the playlist chosen.
- **Video → a video note** `log: video` with its `video` asset (Part E).
- **Never twice**: the source address is looked up among songs/videos before downloading; the blob
  hash dedupes after: *already in your library*.

## Part E — Collections and players

- **Videos**, a group beside Music, on Music's machinery: `crates/fm-app/src/video.rs`
  (`video_library`, `video_collections`), `ui/src/lib/video/` (thumbnails, search, collections as
  tagged notes like playlists, a `<video>` player with a queue). A ruling entry (`#data`: a video is
  a logged note, as a song).
- **M3U8 out** (music-plan phase 4 leftover): each playlist/collection written as
  `vault/playlists/<name>.m3u8` (relative paths into `blobs/`) on change; any player pointed at the
  vault shows the collections; tags are inside the files.
- **Phone background playback** (M4(b)): a Media3 `MediaSessionService` playing the queue natively,
  the web player its remote: lock screen, notification, headphones, Bluetooth, Android Auto.
- **Open with…**: a song or video handed to another app (`FileProvider` content URI; the desktop
  already has `open_external`).
- **Subsonic API** (last, optional): `fm-serve` answers the subset players use (`ping`,
  `getMusicFolders`, `getArtists`, `getAlbum`, `getPlaylists`, `stream`, `getCoverArt`,
  `search3`), behind the pairing tokens: any Subsonic app streams the library, one copy.

## Part F — Real artifacts: self-contained pages

- *Save page* (both devices) keeps **one self-contained HTML file**: images, styles and fonts
  inlined (fetched by the server, size-capped per page), scripts removed, **video and audio never
  inlined** (they point at the vault asset if kept, else the original address).
- Optional **WARC** (Settings → Web → *Also keep a web archive*), from the same fetches.

## Order of work

Each phase: tests, `pixi run ci`, `browser-check`, Kotlin compiled. Released at the end unless the
owner asks per phase.

| Phase | What | Tests |
|---|---|---|
| 0 | 0.50 released. This plan into `docs/context/web-media-privacy-plan-2026-11-17.md`; `decisions.md` entry (widens what ships: `adblock`, `jni`, signature crate; tool downloads; permissions) | `checks` |
| 1 | **A** Privacy sheet and switches (computer: Firefox prefs via the kit; phone: search choice, GPC, HTTPS only, clean links, cookie pop-ups, fire button, per-site off) | UI tests; Kotlin compiled; `browser-check` reads the prefs |
| 2 | **F** self-contained pages | Rust tests on fixture pages (images inlined, video referenced, scripts gone) |
| 3 | **D** on the computer: job runner, direct downloads, yt-dlp + ffmpeg fetched/pinned/self-updated, *Save media* (add-on), into Music / Videos / a note, MP3 | Rust tests with a local fixture server and a fake `yt-dlp` (no real site in CI); add-on tests; `browser-check` lists a fixture page's embed |
| 4 | **E** Videos group, M3U8 out, Open with… | UI and Rust tests |
| 5 | **B** ad/tracker block on phone pages (`fm-block`, JNI, lists) + B0 measurements | Rust tests on invented rules/URLs; Kotlin compiled; fixture-page UI test |
| 6 | **D** on the phone (per decision 1) + *Save media* on phone pages | Kotlin compiled; UI tests |
| 7 | **E** phone background playback (Media3) | Kotlin compiled; device test by hand |
| 8 | **C** app tracker block (per decision 2) | Rust host-rule tests; Kotlin compiled; device test by hand |
| 9 | Subsonic API | Rust tests against recorded client requests |

Each phase updates `features.md`, the manual (`browser.md`, `android.md`, `music.md`, new
`privacy.md` and `media.md`) and `decisions.md`.

## Decisions for the owner

1. **Phone media tools** (phase 6): (a) **ship yt-dlp + Python + ffmpeg in the APK** (+~35–60 MB;
   yt-dlp updates itself; works anywhere), or (b) **the phone asks the computer's formicaria** to run
   yt-dlp jobs (no APK growth; the phone fetches plain files itself; needs the computer on and
   paired; the result reaches the phone through attachment sync), or (c) both. *An optional download
   is impossible on Android 10+.*
2. **Play Protect** (phases 7–8): widen the allowed permissions for the media service and the VPN,
   accepting a possible Play Protect warning on install, or limit those two to the F-Droid build.
3. Defaults: ad block off until measured; tracker block, cookie pop-ups, clean links, GPC, HTTPS
   only on; DuckDuckGo.

## Risks

- Sites change; yt-dlp self-updates within days; the sheet says *this site changed, try after the
  next update*.
- Big files and sync: blobs travel by restic, and by git only under the size limit; a video saved on
  the computer is not on the phone unless attachment sync allows it. Said in the sheet.
- Legal: personal copies are the person's responsibility; no DRM circumvention.
- Size of the work: phases 1–4 are the computer and shared UI; 5–9 the phone's native work, the
  costlier half.

## Owner's answers (2026-11-17)

1. Phone media tools: **inside the phone app** (yt-dlp + Python + ffmpeg in the APK, +~35–60 MB;
   yt-dlp's Python code updates itself).
2. Play Protect: **media playback service for everyone; the app tracker block (VPN) only in the
   F-Droid build.**
3. Defaults as recommended.
