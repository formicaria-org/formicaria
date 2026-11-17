# The structure of formicaria, for the long run (2026-11-17)

A map of where things are and the rules that keep them apart. Read before adding a crate, a
module, a route, a phone shell feature or anything to the release. Plan and reasons:
`decisions.md`, *2026-11-17 (night) — structure, a size budget, the computer's privacy extensions*. The phone's
Kotlin is in `mobile/android/kotlin/`, copied by `ci/android-inject-service.sh`.

## The layers

| Layer | Where | What it may do | Must not |
|---|---|---|---|
| **Model** | `crates/fm-model` | notes, properties, ids | anything else |
| **Query** | `crates/fm-query` | pure filtering/sorting of notes | `std::fs`, a database, paths (CI grep) |
| **Core** | `crates/fm-core` | files, the index, blobs, git, import, chunked ingest | HTTP |
| **App** | `crates/fm-app` | every command (`dispatch.rs`), the kinds and their logic (`music`, `video`, `media`, `privacy`, `log`, `plan`…) | an HTTP client (`fm-fetch` stays out) |
| **Fetch** | `crates/fm-fetch` | HTTPS: downloads (resumable, checked), small gets, one file of a page | knowing what a vault is |
| **Block** | `crates/fm-block` | the phone's ad and tracker engine (adblock-rust), pure | fetching or keeping lists |
| **Audio** | `crates/fm-audio` | the phone's MP3 (Symphonia + LAME) and tags (lofty), in place of ffmpeg | being linked by a computer crate (LAME is LGPL; `checks.sh`) |
| **Server** | `crates/fm-serve` | the computer: HTTP routes over `dispatch`, the browser kit, updates, media tools and jobs, Subsonic | logic a phone also needs (that goes in `fm-app`) |
| **Phone shell** | `mobile/src-tauri/src` (Rust: commands the phone answers itself, JNI for blocking) · `mobile/android/kotlin/` (Kotlin: page views, privacy, VPN, media runner, player service) · `ci/android-inject-service.sh` (copies the Kotlin, patches the generated manifest and Gradle) | what only a phone can do | duplicate `fm-app` logic |
| **UI** | `ui/src` (Svelte 5) | every screen, one bundle for every device | — |
| **Add-on** | `ui/addon` | the computer's browser: saving, bringing in, media, the ant's panel | call routes outside `EXTENSION_ROUTES` |
| **Browser kit** | `browser/kit` | Firefox's startup script and style sheet | lock any preference but the add-on line |

**One door**: every surface reaches notes through `fm_app::dispatch` (the server's routes, the
phone's commands, the CLI, the assistant). **Shared logic lives once**: what both devices need is in
`fm-app` (e.g. `media::keep` files a saved file on both) or in `ui/src/lib` (e.g. `web/read.ts`,
`web/inline.ts` for the add-on and the phone).

## The UI's two shells

- **The phone's one bar** (`TabBar.svelte`, `App.svelte` in tab-bar mode): tabs of features, notes
  and pages. The page tabs' glue is `lib/phone/pages.ts` (what the shell says, saving, media,
  attachments, Back), talking to the app through `PageHost`.
- **The computer's feature pages** (`ui/feature.html`, `ui/src/feature/`): each feature, note,
  Settings, Privacy and Save media a page of its own (`/f/…`), in formicaria's browser's tabs.
- `App.svelte` also keeps the older whole-app chrome for a paired tablet and other browsers.

## Where a feature lives

| Feature | Core | UI | Phone | Computer |
|---|---|---|---|---|
| Notes, views | `fm-app` dispatch, `views.rs` | `NotePanel`, renderers | — | feature pages |
| Music, Videos | `music.rs`, `video.rs`, `media.rs` (playlists, `.m3u8`) | `lib/music/`, `lib/video/` | `PlaybackService.kt` | `subsonic.rs` |
| Save media | `media.rs` (args, `phone_runs`, notes, `keep`) | `SaveMedia.svelte`, `mediaRun.ts` | `MediaRun.kt` (yt-dlp in the APK; streams joined by `MediaMuxer`), `fm-audio` (MP3, tags) | `fm-serve/media.rs` (tools pinned in `media/tools.json`) |
| Privacy | `privacy.rs` (per device) | `PrivacySheet.svelte` | `PagePrivacy.kt`, `fm-block`, `TrackerVpn.kt` | `formicaria.cfg` (Firefox prefs, extensions), `browser.rs` (`PRIVACY_ADDONS`) |
| Pages | — | `PageItem.svelte`, `lib/phone/pages.ts`, `lib/web/` | `MainActivity.kt` (`Pages`) | Firefox |

## The size budget

| Artifact | 0.50.0 | Budget |
|---|---|---|
| Android APK | 48.4 MB (0.50) · 104.2 MB (0.51, ffmpeg in) · ~69 MB expected (0.52, ffmpeg out) | **under 100 MB**, decimal (`android-release.sh` refuses above, warns above 90) |
| Computer archives | 27–29 MB | keep media tools a first-use download (they would add 70–110 MB) |

Whatever is added next to the APK must replace something or be data fetched later: Android 10+
runs no program downloaded after install.

## The next structural steps (not done yet)

- `dispatch.rs` (5.7k lines) and `fm-serve/main.rs` (3k): split the long `match`es by domain when a
  domain grows, not as a sweep; each arm is tested.
- `NotePanel.svelte` (5.2k) and `App.svelte` (5.3k): move pieces out with the next change that
  touches them, as `lib/phone/pages.ts` was.
- The Kotlin has no unit tests: the next change to `TrackerVpn.kt`'s packets should bring a JVM test.
