# formicaria's own browser — the plan (2026-11-15)

_A plan, not a ruling: nothing here is built and no `decisions.md` entry exists yet (§9 lists the
ones to write first). The evidence, alternatives and sources are in
[`web-browsing-research-2026-11-15.md`](./web-browsing-research-2026-11-15.md) (§ references below
point there as **R§**). Labels: **[measured]** in this repo · **[doc]/[secondary]** from the
research · **[estimate]** to be replaced by a P0 measurement · **[proposed]** a number for the
owner to confirm._

---

## 1. What the owner asked for

- **An own browser inside formicaria**, so that finding something on the web and keeping it in the
  vault is one place and one press: recipes, papers, articles, fitness pages, music bought on
  Bandcamp, video links. No copy and paste, least of all on a phone.
- **Built on an existing open project**, judged on size, efficiency, smoothness and maintainability.
- **The add-ons people already use work** (popular Firefox add-ons), and **protections like
  DuckDuckGo's** are there.
- **Android first, built into the app, on by default**: that is where it should have the biggest
  effect.
- The assistant's part stays **opt-in**, as the assistant is.

## 2. The path, in one picture

```
                    ┌──────────────────────── Android APK (one app) ────────────────────────┐
                    │                                                                        │
 the web ──► GeckoView Lite (Firefox's engine, ESR)        formicaria screens (Tauri WebView) │
             + Mozilla Android Components (tabs,           ▲                                  │
               toolbar, downloads, add-ons screen)         │ __fmShare-style bridge:          │
             + our page add-on (built in, unsigned)        │ "here is a page to save"         │
             + Firefox's protections (ETP Strict, HTTPS-Only, GPC, built-in adblock-rust)      │
                    │  Save / download / selection                                            │
                    └─────────► BrowserEngine (our thin seam) ────────────────────────────────┘
                                         │                                   │
                                         ▼                                   ▼
                              a "Save" sheet in the UI  ──►  fm_app readers (Rust, pure)
                              (the person sees, edits,          recipe · paper · article ·
                               saves)                           video/song link · exercise
                                                                      │
                                                                      ▼
                                                     a note (+ PDF/audio blobs) in the vault

 Desktop (P3): formicaria downloads and keeps its own Firefox (ESR), with policies.json:
 our add-on force-installed, formicaria as the pinned home tab, add-ons kept off 127.0.0.1:8765.
```

**Why this path** (R§10–§13, in one line each):
- **Gecko** is the only engine that runs Firefox's add-on catalogue, on Android and desktop alike.
- **GeckoView**, not a fork: Firefox for Android is itself built on it, so Mozilla cannot drop it
  without dropping its own browser. Forks die of rebasing (Mull).
- **The Lite build** is the variant meant for third-party apps; it leaves out Mozilla's telemetry and
  crash reporting **[doc]**.
- **ESR, not the monthly release**: Tor Browser and Waterfox ship GeckoView ESR **[secondary]**. The
  API and features change once a year, while security fixes still arrive in point releases.
- **A `BrowserEngine` seam**: Wolvic moved off Gecko in 2024, and Mozilla depends on one funder
  (R§13). A second implementation over Android System WebView is the exit, and P0's fallback.
- **One page add-on, written once** to the standard WebExtension API: built into the Android
  browser, force-installed in desktop Firefox, and portable to a Chromium engine if ever needed.
- **The readers stay in Rust** (`fm-app`), pure and tested, extending `web_recipe.rs`. The page side
  only serialises; it decides nothing.

## 3. The pieces, and where each would live

| Piece | Where (proposed) | Notes |
|---|---|---|
| **Android browser module** (Kotlin) | a **committed Gradle module**, e.g. `mobile/browser/`, wired into the generated project by an extended inject script (as `ci/android-inject-service.sh` does for `AgentService.kt`) **[measured: `gen/android` is gitignored, Kotlin is injected]** | not more heredocs: the browser is too large to live in a shell script. Holds the `BrowserEngine` interface, `GeckoEngine`, `SystemWebViewEngine`, the browser screen, downloads and the Fire button |
| **The page add-on** (MV3, TypeScript) | e.g. `browser-addon/`, built by a pixi task | detect a recipe/paper/video/song badge; serialise the page or a selection; text-fragment highlights; catch downloads. Its Android copy talks to the module by GeckoView's built-in-extension messaging **[doc]** |
| **Readers** (Rust) | `crates/fm-app/src/web/` (moving `web_recipe.rs` in as the first) | pure: page bytes + address → a proposed note. No HTTP client (the shells fetch, R§1) |
| **One capture command** | through `fm_app::dispatch` (the one door), e.g. `clip_page` | takes the page bytes the browser already has; **fetches nothing**. `recipe_from_link` stays for paste/share |
| **The Save sheet + Browser place** (Svelte) | `ui/src/lib/` | the person sees the proposed note and saves it; a *Browser* place in the phone's bar |
| **The hand-off** browser → formicaria | reuse the **share bridge pattern** (`MainActivity.kt`'s `__fmShare`) **[measured]** | the browser hands the page to the formicaria screen exactly as Android's share menu does today, so no new JNI path into Rust |
| **Desktop Firefox (P3)** | downloaded into `<config>/formicaria/tools` like the assistant's runtime; `distribution/policies.json` written by the app | a launcher change in `fm-serve` (open our Firefox instead of the default browser; fall back to the default) |
| **Desktop add-on key (P3)** | `fm-serve`, a new key type scoped to the capture commands | the one new authorisation axis (R§6.1) |

## 4. What a saved page becomes

A **note**, files-as-truth as always: the person's own copy, edited freely afterwards.

- **Frontmatter**: the title; `source_url`; `site`; `author` and `published` when the page states
  them; `saved` (the stamp); the kind's own fields (a recipe's servings, a paper's `doi`, `authors`,
  `year` via `create_paper`, a song's tags via the music fields).
- **Body**: the extracted text as Markdown (recipe: *Ingredients*, *Method*; article: the text);
  a credit line linking the page; **highlights as text-fragment links**
  (`source_url#:~:text=…`), which reopen the page highlighted in any modern browser.
- **Blobs**: a PDF the person downloaded (paper); an audio file they own (a Bandcamp purchase,
  unpacked from its zip); **a snapshot of the page, with every save** (owner, §8 Q5: *"nothing kept
  out of the vault … all stays in"*). A snapshot is an asset like any other: in the vault, travelling
  by git under the vault's existing `git_assets_max` rule like every blob, and kept out of the
  planning views as assets already are. Better vault filtering is the answer to clutter, never
  leaving something out.
- **Never stored**: video files; anything from a page the person did not choose to save.
- Sorted **by kind** into the existing groups (recipe → Food, paper → Library, song → Music), with
  an **Inbox** shelf (`shelf:` tag) for anything unsorted.

## 5. The user-visible flows (Android)

1. **Browse**: the *Browser* place opens on a start page (formicaria's recent saves + a search box).
   Tabs, back/forward, downloads, a private tab, the **Fire button** (closes tabs and wipes site
   data in one press).
2. **Save**: a *Save* button in the toolbar, with a badge when the page *is* something (a recipe, a
   paper…). Press → the Save sheet opens over the browser with the proposed note → *Save* → done.
   One press to look, one to keep.
3. **Save a selection**: long-press text → *Save to formicaria* → appended to the note being
   built, as a quote with its text-fragment link.
4. **Download → vault**: a PDF or a zip downloaded in the browser offers *Add to vault*. A zip of
   audio becomes songs through *Add a song*.
5. **Add-ons**: an *Add-ons* screen lists Firefox's Android catalogue (Android Components'
   add-ons feature **[doc]**); first run offers uBlock Origin. Installs need AMO's signature
   **[doc]**.
6. **Search**: the address bar searches with a chosen engine (§8 Q3); no search deal, nothing earned.

## 6. Privacy defaults (on from first run, each changeable)

ETP **Strict** (Total Cookie Protection, fingerprinting and bounce-tracking defence, query
stripping) · **HTTPS-Only** · **Global Privacy Control** on · cookie pop-ups handled by DuckDuckGo's
`autoconsent` (MPL-2.0) · the engine's **built-in adblock-rust** with EasyList/EasyPrivacy if P0
finds it active on Android, else uBlock Origin offered · the **Fire button** · **GeckoView Lite**
(no Mozilla telemetry) · **Safe Browsing on** (owner, §8 Q6) · **DuckDuckGo** as the search engine,
changeable (§8 Q3) · **history, cookies and sign-ins stay on the device** (owner, §8 Q7).
Not offered: DuckDuckGo's *App Tracking Protection* (a device VPN, not a browser feature, R§12.7b).

## 6b. When memory is short, the assistant yields (owner, §8 Q4)

The browser is what the person is looking at; the assistant can wait. **Rule [proposed detail]:**
when Android signals memory pressure while the browser is in front (`onTrimMemory` at
`RUNNING_LOW` or worse, or available memory under a floor set from P0.2's measurement), the
assistant's model process is stopped. A question in progress says so (*"paused to keep the browser
smooth; ask again"*), and nothing is lost. The model starts again on the next question. It is never
the browser's tabs that are dropped first. On a phone with plenty of memory nothing changes; both
run.

## 7. Phases

Each phase ends with `pixi run ci` green, its doc rows updated, and (from P2) a phone test.

### P0: spikes and measurements (no product code)

| # | Question | How | Pass when **[proposed thresholds]** | If it fails |
|---|---|---|---|---|
| 0.1 | APK size with GeckoView Lite (ESR) | build the APK with a minimal browser screen; compare with the ~28 MB today **[measured]** | **the whole APK ≤ 100 MB** (owner). Focus grew ~34 MB **[doc]**, so ~65 MB is expected | try only the needed Android Components; then the System WebView engine |
| 0.2 | RAM with the assistant | `dumpsys meminfo` on the test phone (`device-resources.md`): model answering + browser on a video page; then a 4 GB-class emulator profile | the browser never stutters or is killed because of the model | **the assistant yields** (owner, §8 Q4): §6b's rule stops the model under pressure |
| 0.3 | The rest of the app stays as fast | app cold start and view switches, with and without the module; the runtime is created lazily | **no measurable change** (owner: *"the rest of the app is fast"*) | fix the lazy start; it is a bug |
| 0.4 | Opening the browser, and feel | cold and warm open time (`adb shell am start -W`-style timing and a screen recording), against **opening Firefox for Android and Chrome cold and warm on the same phone**; scroll and load on five sites | **no slower than opening another browser on the same device** (owner), cold and warm; no scroll a person calls janky side by side | System WebView engine as the default, Gecko later |
| 0.5 | Sign-in and media | Google sign-in, YouTube playback, a Bandcamp purchase download | all three work | note the gap; Bandcamp and downloads are the must-have |
| 0.6 | Add-ons | install from AMO in our build: uBlock Origin, Privacy Badger, Bitwarden, Dark Reader, ClearURLs, SponsorBlock, Firefox Relay | each installs and works on three sites | list what does not, and why |
| 0.7 | Built-in blocking | is `adblock-rust` present and switchable in GeckoView Lite on Android | ads blocked with EasyList on | uBlock Origin offered; our own `adblock` crate link last |
| 0.8 | Page add-on ↔ app | built-in add-on → module → `__fmShare`-style hand-off → a note | a round trip on a recipe page | — (design bug) |
| 0.9 | Desktop D2 | official Firefox ESR unpacked with `policies.json` on Linux, Windows, macOS; host blocking keeps add-ons off `127.0.0.1:8765`; the update path; Mozilla's trademark terms read | all four hold | a rebranded Gecko build others publish |
| 0.10 | Zotero bridge | a stub answering `/connector/ping` and `/connector/saveItems`; the real connector saves through it | one paper lands | drop it; nobody else has built it |

P0 ends with a **results section appended to the research doc** against the owner's budgets (§8)
and the go/no-go on Gecko as the default engine.

### P1: readers (Rust)

- Move `web_recipe.rs` under `fm_app::web`; add readers: **paper** (`citation_*` meta, JSON-LD
  `ScholarlyArticle`, arXiv/DOI in the address → `create_paper`), **article** (`dom_smoothie`, already
  Myrme's pick), **video/song link** (`VideoObject`/`MusicRecording`, `og:*`), **exercise** (article
  text + credit; the agent part is P4).
- The `clip_page` command through `dispatch`: bytes in, proposed note out; refused to paired
  devices like `recipe_from_link`.
- Tests on invented pages in the repo; real pages read only from a folder given at test time
  (pages kept nowhere, as the recipe reader's ignored test does).
- **Done when** each reader passes its tests and the eval on saved real pages (kept outside the repo).

### P2: the Android browser (the headline phase)

- The `mobile/browser/` module: `BrowserEngine` + `GeckoEngine` (+ `SystemWebViewEngine` behind the
  same interface, kept building in CI so the exit stays open).
- The browser screen from Android Components: toolbar, tabs, private tab, downloads, Fire button.
- The page add-on, built in; the Save sheet and Browser place in the UI; download → vault;
  Bandcamp zip → songs.
- §6's defaults; the add-ons screen; uBlock Origin offered; a snapshot kept with every save.
- §6b's rule: the assistant yields to the browser under memory pressure.
- **Settings → Browser**: hide it (the engine then never starts), the search engine, the defaults.
- `pixi run android-smoke` extended: app start must not create the `GeckoRuntime`.
- **Done when** the owner uses it for a week on the phone without reaching for another browser, and
  the P0 budgets still hold in the release build.

### P3: desktop (D2)

- The app downloads, verifies and keeps **Firefox ESR** in `<config>/formicaria/tools`, with a
  progress bar and its size stated, as the assistant's runtime does.
- `policies.json`: our add-on force-installed (AMO-signed, unlisted), formicaria as the home tab,
  add-ons blocked from formicaria's address, the §6 defaults.
- The launcher opens formicaria in it; the default browser stays the fallback; the system browser is
  no longer needed.
- The desktop add-on key (the capture commands only, minted in Settings).
- **Done when** a desktop session never needs the system browser, and an "all sites" add-on
  provably cannot read the formicaria tab.

### P4: the assistant over saved pages (opt-in, as the assistant is)

- The **prompt-injection threat analysis** written first, with hostile-page eval cases (R§5.6).
- Summarise a saved page; *adapt this recipe*; `/exercise` from a fitness page; all as proposals.

### Later, each on request

Zotero bridge as a feature (if 0.10 passed) · a Tor private tab via Arti · iOS (unchanged until
asked) · snapshots viewed in-app (needs a `frame-src` decision).

## 7b. The release path (owner, 2026-11-15: *"it shall end with a release we can try, first here on the desktop and then on the phone"*)

The phases above are regrouped into **two releases**, each one something to try:

**R1, the desktop release (Linux first)**: P0.9 → P1 → P3, then a release.
- Settings → *Browser* → **Set up formicaria's browser** downloads **Firefox ESR straight from
  Mozilla** into `<config>/formicaria/tools/firefox` (formicaria redistributes nothing), verified
  against Mozilla's published SHA-512 sums, size stated first.
- formicaria then opens in it: its own profile (`user.js` with the §6 defaults; history on the
  device), `policies.json` (our add-on, the add-on's key via managed storage, DuckDuckGo default
  search, Safe Browsing on, add-ons kept off formicaria's address).
- Our add-on: a *Save* button on every page → a proposed note (recipe, paper, article, video or song
  link) → *Save* → a note with a snapshot of the page, in the vault.
- **Linux only in R1** (the only desktop this project can test). Windows and macOS say so in
  Settings instead of offering a setup that was never run.

**R1 as built (2026-11-15):** `fm-serve` `browser.rs` (setup, policies, key, launch, routes; the
narrow extension gate), `fm-app` `clip.rs` + `clip_save` + `recipe_from_page`, the add-on in
`ui/addon/` (built into `ui/dist/addon/formicaria.xpi` by `pnpm -C ui build`, baked into the
binary), the shared `ui/src/lib/web/clip.ts`, Settings → Browser, `#note=<id>`. Run end to end on
Linux, headless: setup from Mozilla (153.7.0esr), the gate with forged requests (7/7 as designed),
and the real add-on keeping four real sites correctly (research §14). Not yet pressed by a person;
what to check is in `known-issues.md`.

**R2, the phone release**: P0.1–0.8 → P2, then a release with the browser built in and on by
default, tried on the phone after R1 has been tried on the desktop. Built by `release.yml`
(this machine has no Android SDK or device attached), tried by the owner on the phone.

**Recommended options taken** where the owner left them open (the owner asked for this):
the add-on unsigned and installed through the ESR profile, which allows it (AMO signing later,
with an account); the Zotero bridge left out of R1 and R2 (a spike only); downloads → vault in R2;
the desktop add-on key carried by Firefox's managed storage for extensions, written by formicaria.

## 8. The owner's answers (2026-11-15)

| # | Question | Answer | Where it lands |
|---|---|---|---|
| 1 | Budgets | **The whole APK stays within 100 MB.** The rest of the app stays as fast as it is. **Opening the browser costs no more than opening another browser on the same phone or device** | P0.1, P0.3, P0.4 |
| 2 | ESR or monthly | **Long-term-support (ESR)** | §2, §10 |
| 3 | Search engine | **DuckDuckGo**, changeable | §5, §6 |
| 4 | Phone memory | **The assistant goes lower in priority** when resources are very constrained | §6b, P0.2 |
| 5 | Page snapshots | **Nothing is kept out of the vault**; improve vault filtering at most, but all stays in | §4: a snapshot with every save, an asset like any other |
| 6 | Safe Browsing | **On by default** | §6; a `privacy.html` line (§9) |

| 7 | Browsing history and cookies | **History stays on the device**; so do cookies and sign-ins | §6 |

## 9. Decision entries to write first (when `decisions.md` is free)

- `#track-m`: GeckoView Lite (ESR) in the main APK, on by default, lazy runtime; **the APK within
  100 MB**; the assistant yields under memory pressure;
  the browser module as a committed Gradle module.
- `#toolchain`: Mozilla's Maven repository; the ESR bump in the release flow; AMO signing keys in
  the release pipeline (P3); the F-Droid route (R§12.10: IzzyOnDroid or the project's own repository).
- `#ui`: a *Browser* place from install; desktop opening in our Firefox (P3).
- `#data`: what a page may be read for (the readers); **a snapshot with every save, nothing kept out
  of the vault**; the Inbox shelf.
- `#vault`: the desktop add-on key; add-ons kept off formicaria's address.
- `#seams`: `clip_page` through the one door, fetching nothing.
- `#agent`: the assistant over saved pages (P4).
- **`privacy.html`** gains its lines: the browser fetches what the person visits; AMO when they
  install an add-on; filter-list updates; **Safe Browsing, on by default** (what the engine sends
  to check a page **[not verified: GeckoView Lite's exact Safe Browsing traffic; read it in P0]**);
  DuckDuckGo as the default search.

## 10. Maintenance, after it ships

| What recurs | Cadence | Who does the work | Ours |
|---|---|---|---|
| GeckoView ESR security point releases | as Mozilla ships them | Mozilla | a bot PR bumping the version → `pixi run ci` → the agent-driven release |
| GeckoView ESR major (API changes) | yearly | Mozilla, with a 3-release deprecation window **[doc]** | one planned upgrade a year |
| Android Components | pinned to the GeckoView version | Mozilla | same bump |
| Desktop Firefox ESR | Firefox updates itself **[not verified for this install]** | Mozilla | the policy file; check once a year |
| The page add-on | with our releases | — | AMO signing for desktop (automated) |
| Readers | when a site's published data changes | — | low: schema.org, `citation_*`, Readability |
| Filter lists, ETP lists | continuous | list authors, Mozilla | none |
| `autoconsent` rules | with its releases | DuckDuckGo | a dependency bump |

## 11. Risks, and what limits each

| Risk | Limit |
|---|---|
| Mozilla deprioritises GeckoView for third parties (Wolvic's experience) or loses funding | the `BrowserEngine` seam with a System WebView implementation kept building in CI; the add-on is standard WebExtension |
| The APK roughly doubles or more | the owner's **100 MB ceiling** (P0.1); Lite build; only the needed components; System WebView engine if it cannot be met |
| Memory beside the assistant's model | lazy runtime; P0.2; **the assistant yields** (§6b) |
| F-Droid main repo refuses a prebuilt engine | IzzyOnDroid or the project's own repository, decided in the `#toolchain` entry |
| A popular add-on breaks on Android | Mozilla's own Android limits are known (no sidebar, no native messaging) **[doc]**; P0.6 says which, before launch |
| Snapshots make vaults grow, and a shared vault carries the pages saved | snapshots are assets: big ones stay off git by the existing per-vault limit; the Save sheet states the size; the person chooses what to save into a shared vault |
| Hostile pages | Gecko's sandboxing; pages never reach the formicaria screen except as text through the Save sheet; the existing sanitiser on every body; the assistant reads only what was saved, and only proposes |
| An add-on reads formicaria on desktop | policy host-blocking (P0.9) or a separate profile; P3 is not done until proven |
| YouTube downloads expected | not offered (terms, store policy, R§5.3); a video is saved as a link with notes |

## 12. Out of scope

A browser engine or fork of our own · Chrome Web Store add-ons · downloading YouTube or ripping
streams · App Tracking Protection (device VPN) · syncing browsing history or cookies through the
vault · iOS changes.
