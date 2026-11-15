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

**R1.1 (0.32.0), after the owner tried R1:** the button pinned to the toolbar; a sidebar and the new
tab showing formicaria (the app framed by exactly our add-on); formicaria's colours; Alt+Shift+K /
Alt+Shift+F; every download asks where it goes, formicaria by default, one copy (a song or an album
zip → Music, any other file → a note); the launcher says when it is building. `decisions.md`
2026-11-15 (later).

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

---

## 13. formicaria's browser, fully its own: rebuilt automatically, stripped to its foundation (plan, 2026-11-15)

_Plan only, no code (owner: *"plan and research only until I say so"*). Evidence:
`web-browsing-research-2026-11-15.md` §15 (the C spike)._

**The owner's requirements.** *"We need to have an automated process that does this every time we
bump releases and use a new firefox. I would strip away all the Firefox Sync and other stuff, we can
have dedicated fm options for that based on our own infra."* Earlier: full control of the interface,
Firefox as the engine and foundation, the system Firefox untouched.

### 13.1 The pipeline (recommended: a signed kit applied to Mozilla's own build)

Three ways to get Mozilla's ESR plus formicaria's layer onto a computer:

| Way | What a release carries | For | Against |
|---|---|---|---|
| **K: a kit** (recommended) | formicaria's layer only: branding files, icons, policies, the interface layer (B), the add-on, and the **pinned ESR version with Mozilla's SHA-512 per platform**, all signed with formicaria's release key | Mozilla's binaries come from Mozilla (nothing of theirs redistributed); small release assets; exactly the ESR the release tested | the layer is applied on each computer, so it needs a self-check there |
| P: a prebuilt package | the whole modified browser per platform (~90 MB each), signed by formicaria | the computer only unpacks | formicaria redistributes a modified Firefox; heavy assets each release |
| L: local live (today's R1) | nothing pinned; the app follows Mozilla's latest ESR | security fixes at once | an untested combination can reach users |

**K, step by step:**
1. **In the repo**: `browser/` holds the kit's sources. That is branding templates (every locale's
   name terms, generated, not hand-edited), icons generated from `packaging/formicaria.svg`, the
   policy template (§13.2), the interface layer (B), and `browser/esr.toml` pinning the ESR version
   and SHA-512 per platform.
2. **`pixi run browser-bump`** (like `myrme-bump`): asks Mozilla for the newest ESR and verifies
   Mozilla's `SHA512SUMS` signature against Mozilla's pinned release key. It writes the pin, and says
   loudly when the **major** changed (the once-a-year adaptation).
3. **`pixi run browser-check`** downloads the pinned ESR, applies the kit, launches it headless and
   checks:
   - no visible "Firefox" left (an allowlist keeps the attribution line);
   - no account or Sync anywhere;
   - formicaria's add-on and interface present;
   - startup time against stock ESR;
   - screenshots kept as evidence.

   Linux runs in the local gate; Windows and macOS run in `release.yml`'s matrix on the tag (where
   every platform is already built).
4. **Release order** gains two steps: docs → `myrme-bump` → **`browser-bump`** → `check-references` →
   **`browser-check`** → `pixi run ci` → version commit → push `main` and the tag. Still agent-driven,
   nothing waits for a person.
5. **On the person's computer**, Settings → Browser and every formicaria update:
   - download the pinned ESR from Mozilla and check it against the pin inside the signed kit
     (stronger than reading Mozilla's sums live);
   - apply the kit, self-check, swap;
   - keep the previous copy if anything fails (the updater's G2).

   Firefox's own updater is off (`DisableAppUpdate`); formicaria is the updater.
6. **Security point releases between formicaria releases**: §13.5 Q1.

### 13.2 Stripped to the foundation: what goes, by Firefox's own enterprise policies

Policies are Mozilla's supported, release-stable switches, so stripping by policy survives updates
where interface hacks would not ([policy reference](https://firefox-admin-docs.mozilla.org/reference/policies/)).

| Goes | Policy |
|---|---|
| Telemetry, studies, feedback | `DisableTelemetry`, `DisableFirefoxStudies`, `DisableFeedbackCommands` |
| Pocket, sponsored tiles, Firefox Suggest | `DisablePocket`, `FirefoxHome`, `FirefoxSuggest` |
| Mozilla's messages, Labs, "More from Mozilla", recommendations | `UserMessaging` (incl. `FirefoxLabs`, `MoreFromMozilla`) |
| The AI chatbot and AI features | `GenerativeAI` (since Firefox 148; not every AI control is covered by it **[secondary]**) |
| Firefox's own updater | `DisableAppUpdate` |
| Firefox's default bookmarks, first-run and what's-new pages | `NoDefaultBookmarks`, `OverrideFirstRunPage`, `OverridePostUpdatePage` |

**Kept, by the owner's choice: Firefox's Sync and Mozilla account** (*"maybe we can leave also the sync from firefox, to allow users to smoothly start with formicaria"*): signing in brings a person's bookmarks, passwords, history and add-ons over. formicaria's own options (§13.3) sit beside it, not instead of it.

**Kept** (the engine and the local foundation): browsing, Firefox's settings and
privacy controls, add-ons from addons.mozilla.org (the owner wants popular add-ons; AMO is the one
Mozilla service kept, Q3), Safe Browsing (owner: on), history on the device (owner's ruling).

What policies cannot remove (visible leftovers such as the fox picture) is the interface layer's job (B).

### 13.2b What formicaria maintains, and what it keeps for free

**The owner:** *"We should also understand what we want to maintain. I do like the extension infra of
firefox, but need nothing else."* So formicaria maintains **one thing: its layer** (the add-on, the kit,
the interface). Everything Mozilla maintains is free to keep. Removing costs nothing when a policy
does it, and costs upkeep when the interface has to hide it. **The rule: keep what runs on the
device, strip Mozilla's cloud services and promotions, except the three that are security.**

| Keep (Mozilla services, because they are security) | Why |
|---|---|
| **Remote Settings** | carries the blocklist of malicious add-ons, certificate revocation (CRLite) and tracking-protection lists; stripping it quietly weakens the browser **[not verified: exact contents for ESR 153]** |
| **Safe Browsing** | owner: on |
| **Add-on updates from addons.mozilla.org** | the extension infrastructure the owner wants |

| Keep (on the device, free, and useful to formicaria) | formicaria could build on it |
|---|---|
| **PDF reader with highlights and notes** (pdf.js) | an annotated paper kept in the vault (the papers plan's reader, for free) |
| **Reader View** | clean articles; *Keep* already uses the same Readability |
| **Local translation** (on-device models, downloaded once) | a translated copy kept beside the original |
| **Picture-in-picture** | a fitness video on screen while writing the note |
| **Screenshots, Save as PDF** | more things *Keep* can take |
| **Containers, private windows, vertical tabs, tab groups** | the interface layer (B) builds on them instead of rebuilding them |
| **Developer tools** | for development |
| **Local password manager and autofill** (no sync) | Q2 |

| Keep only if wanted (services some sites need) | |
|---|---|
| **DRM playback (Widevine)** | Spotify, Netflix and similar web players need it; Bandcamp does not |
| **Web push** | notifications from sites whose tab is closed (web chat, mail) |

### 13.3 formicaria's own replacements, on formicaria's own infrastructure

formicaria's infrastructure is the vault, its git sync and the LAN share; there is no cloud.

| Firefox had | formicaria offers | How |
|---|---|---|
| Bookmarks synced | **bookmarks are notes**: *Keep* already makes them; a "bookmark" button that keeps the link without the page copy; a Bookmarks view | vault notes, synced by git like every note |
| Open tabs on other devices, reading list | a **reading list** shelf; "open on my other device" | notes on a shelf; git sync |
| Add-ons on every device | **formicaria's add-on list**: the add-ons you install are written to the vault and installed by policy on each of your computers | a vault file read into `policies.json` at launch |
| Settings on every device | the browser's formicaria-side choices in the vault; Firefox's own settings stay per device | a vault file |
| Passwords synced | **not rebuilt by formicaria**: Firefox's password manager stays, and Firefox's own Sync carries it for whoever signs in (owner, Q2) | security-critical; formicaria should not become a password manager |
| History synced | **nothing**: history stays on the device (owner, 2026-11-15) | n/a |

### 13.4 Costs and risks

- **Once a year**, the new ESR major moves things the layer depends on. `browser-check` turns red at
  `browser-bump`; that release carries the adaptation.
- **macOS**: changing files inside the app breaks Apple's seal. An **ad-hoc re-sign** after the kit
  is applied makes it run on that Mac (it is not valid on other Macs, which does not matter here,
  since it is applied on each Mac) **[secondary: Mozilla's signing docs]**. To test on `release.yml`.
- **Windows**: the program files are signed; the packed interface is not, so it is expected to work.
  Untested.
- **Languages**: the spike renamed en-US only. Each locale has its own branding terms, so the kit
  generates them for all locales the ESR carries. Language packs: unknown, to check.
- **Startup speed**: the spike's repacked interface was compressed (17 MB against 54 MB); repack the
  way Mozilla does (stored) unless measured fine.
- **The add-on catalogue (AMO)** remains a Mozilla service; removing it removes popular add-ons.

### 13.5 The owner's answers (2026-11-15)

| # | Question | Answer |
|---|---|---|
| 1 | Security fixes between formicaria releases | **Point releases arrive by themselves**: the app takes Mozilla's newer point release of the same ESR major, checks Mozilla's signature, applies the kit, self-checks, falls back on failure. A new major comes only with a formicaria release |
| 2 | Passwords | **Firefox's Sync stays available** (the strip list keeps it) so a person can start smoothly; the password manager stays |
| 3 | Add-ons from addons.mozilla.org | kept (the extension infrastructure the owner wants) |
| 4 | Firefox's own Settings pages | open: keep reachable, hide what the policies removed (recommended) |
| 5 | DRM, web push, translation | **DRM playback kept**; **web push a switch, off by default**; translation: see below |

**Translation (owner: *"we should have myrme doing translations, no? How big are Mozilla models?"*).**
Mozilla's translation models are small, dedicated neural translators: **about 17–35 MB per language
direction** (tiny models; Japanese, Korean, Chinese the largest). They translate through English
between two other languages, so two models for, say, Italian to French. A distilled 17 MB model
scored BLEU 50.7 against its 798 MB teacher's 52.5, about 35× faster on one CPU core. Engine and
models are MPL-2.0 ([Mozilla Hacks](https://hacks.mozilla.org/2022/06/neural-machine-translation-engine-for-firefox-translations-add-on/),
[models repo](https://github.com/mozilla/firefox-translations-models)) **[secondary for sizes; to
measure]**. Recommendation, consistent with Myrme's own "a small dedicated tool per task" rule:
- **pages in the browser** use Firefox's built-in translation (free, maintained by Mozilla, on the
  device);
- **notes, kept pages and mail** use a **Myrme translation tool on the same engine and models**
  (`bergamot-translator`), so a language fetched once serves both, on the computer and the phone;
- **the language model** only for rewording or explaining.

That is a plan item for **Myrme's repository**, not this one.

### 13.6 Phases (when the owner says go)

| Phase | What |
|---|---|
| K0 (spike) | B on the scratch copy: restyle the window and add one built-in formicaria control; an ESR point-update round trip with the kit re-applied; a visible window checked by the owner |
| K1 | the kit in `browser/`; `browser-bump`, `browser-check` (Linux), the release-order change |
| K2 | the app applies the signed kit at setup and on updates; Firefox's updater off; fallback |
| K3 | §13.2's policies; §13.3's replacements (bookmarks, reading list, add-on list) |
| K4 | `browser-check` on Windows and macOS in `release.yml`; ad-hoc re-sign on macOS |

**Decision entry to write first** (K1): *formicaria's browser is a kit applied to Mozilla's ESR,
stripped of Mozilla's services by policy* (`#toolchain` `#ui` `#vault`). It extends the 2026-11-15
entries and reverses 0.31.0's "follow Mozilla's latest ESR live".

### 13.7 Refined, and what 0.33.0 carries (owner: *"refine plan and execute it till new release"*)

**Measured before refining** (2026-11-15):
- `gpgv` verifies Mozilla's signature on an ESR's `SHA512SUMS`. Mozilla's release key is
  `14F2 6682 D091 6CDD 81E3 7B6D 61B7 B526 D98F 0353` ("Mozilla Software Releases"); it signs with
  a subkey rotated every two years, the current one valid to 2028.
- Point releases 153.5.0–153.7.0 are on Mozilla's archive, so an update round trip can be tried.

**Refinement: the kit is files beside Firefox, not a repacked Firefox.** Firefox's *autoconfig*
(`defaults/pref/autoconfig.js` pointing at a `.cfg` in the install folder) runs a privileged script at
every start, from plain files **outside** the packed `omni.ja`. If that script can rename (override
`chrome://branding/` and register formicaria's branding strings), restyle (a chrome style sheet) and
add formicaria's own controls, then:
- the kit is a folder copied beside Firefox, and re-copied after an update;
- no packed-file format to handle and no `zip` tool needed on the person's computer;
- no startup cost.

K0 proves or refutes this. **The fallback is the C spike's repack**, which already worked
(research §15).

**0.33.0 (desktop, Linux), in order:**
1. **K0, the spike**: on a scratch ESR, the autoconfig kit renames, restyles and adds one formicaria
   control; then an update 153.6.0 → 153.7.0 with the kit re-applied; and the policy strip list
   checked (§13.2).
2. **The decision entry** (§13.6) is written before code.
3. **K1, the kit in the repo**: `browser/kit/` (branding, icons, styles, the startup script), the policy
   template, `browser/esr.toml` (pinned ESR, SHA-512 per platform, Mozilla's key fingerprint),
   `browser/mozilla-release-key.asc`. The kit is **compiled into `fm-serve`**, so it is signed with the
   app itself (no separate asset).
4. **K1, the tools**: `pixi run browser-bump` (newest ESR, its signed sums, the pin) and
   `pixi run browser-check` (download the pin, apply the kit, launch headless, check names, Sync
   still present, the strip list absent, the add-on loaded; screenshots). Both go into the release
   order.
5. **K2, on the person's computer**:
   - setup and every start check for a newer **point** release of the pinned major (owner Q1),
     verified with `gpgv` against the pinned key;
   - apply the kit, self-check with a headless start, swap, keep the previous copy;
   - Firefox's updater off.
6. **K3**: the strip list by policy (Sync kept); **settings that stick** (the privacy defaults
   become policy defaults, so the person's changes win; only the add-on's own line stays forced);
   the window named formicaria (already prepared); a **Bookmark in formicaria** entry (a note with
   the link, no copy, on a *Bookmarks* shelf); DRM kept; web push behind a switch, off.
7. **Release 0.33.0**: docs → `myrme-bump` → `browser-bump` → `check-references` →
   `browser-check` → `pixi run ci` → version commit → push `main` and the tag.

**Later, not in 0.33.0:** the add-on list kept in the vault; Windows and macOS (`release.yml`
matrix, macOS re-sign); other languages; a fuller formicaria interface built on the K0 control;
Myrme's translation tool (its own repository).

---

## 14. formicaria is the app; web pages are its tabs (owner, 2026-11-15; plan, supersedes the "formicaria in a tab" shape)

> **The owner**, stopping K0: *"fm is not something in one tab, it is the entire ecosystem. So we
> need not anymore open tabs inside a fm tab in a browser, we can actually open other tabs in the
> overall app."* Back to plan mode; nothing of K0 ran.

### 14.1 What changes

Until now the shape was **a browser with formicaria inside it**: formicaria in a tab, in a sidebar, on
the new tab. The new shape: **formicaria is the window**. Its own navigation (Library, Tasks, Food,
Fitness, Health, Music, and now Web) is the frame, and **everything opens as a tab of formicaria**:
- a note;
- a view (a board, a day of food);
- a web page.

A note can sit beside the page it is about. There is no "formicaria tab" in a browser any more.

### 14.2 How (recommended: formicaria's shell built into Firefox's own window)

| Option | | Verdict |
|---|---|---|
| **S1. A formicaria shell in the browser's own window** (§13's B layer, privileged startup scripts and styles on ESR) | formicaria draws the window's frame: its navigation panel, its look, its actions. **Firefox's tab machinery is reused**: vertical tabs, tab groups and **split view**, all in Firefox since 149, so in ESR 153 **[secondary]**; session restore; add-ons. formicaria items are tabs showing the formicaria app in *shell mode*; web pages are ordinary tabs | **recommended** |
| S2. web pages inside the formicaria page, in frames | the add-on would strip sites' "do not frame me" headers | **rejected**: many sites still break (sign-ins refuse frames, sign-in pop-ups fail), and framed sites get separate, partitioned storage, so you are signed out of everything inside formicaria; it also weakens a protection |
| S3. a native shell (Electron, CEF, Tauri, Servo) | | rejected (§2): a second engine to keep secure |
| S4. a compiled fork (Zen, Floorp) | the same UI, built in | not needed now. Zen and Floorp show this kind of interface is buildable on Firefox, and **Floorp moved its scripted components into its own build** **[secondary]**, a warning to keep the shell small and its hooks few |

### 14.3 The window, concretely (desktop)

> **Revised (2026-11-15, late night):** a pinned tab cannot join Firefox's split view, so formicaria
> became a **docked panel** of the window, beside whichever page is shown; tabs are web pages only
> (`decisions.md`; research: `data-sources-research-2026-11-15.md` §2–§4).

- **The left panel is formicaria's**: the places (Library, Tasks, Food, Fitness, Health, Music, Web)
  on top, and below them **everything open**, notes, views and web pages in one list. It is built on
  Firefox's vertical tabs and tab groups, restyled: a tab group per place.
- **The content area shows the chosen tab.** **Split view** puts a note beside a page (Firefox's,
  two tabs at a time).
- **The address bar** goes to an address, or searches the web (DuckDuckGo) and your notes in one
  list (to try).
- **The actions are the window's own**: *Keep*, *Bookmark*, *Add to note beside*, downloads into
  formicaria, all in the frame and no longer only behind an add-on button.
- **Opening**: pressing a note in a formicaria view opens it as a tab (or beside the page); a link
  in a note opens a web tab. formicaria's own app pages are top-level tabs on `127.0.0.1`, so they keep
  their normal storage (none of the frame-partitioning problem of 0.32.0's sidebar).

### 14.4 What the formicaria app (the Svelte UI) needs: a *shell mode*

- **Every place, view and note has an address** (`/#view=…`, `/#note=…`), so each can be a tab, be
  restored, and be opened beside.
- **In shell mode the app hides its own chrome** (side panel, top bar): the window's frame replaces
  them. In any ordinary browser, and on a paired tablet, the app is unchanged.
- **The app asks the shell** to open a tab, open beside, or switch place, over one narrow channel:
  only formicaria's own top-level tabs may send, and the shell never acts on anything a web page
  says.

### 14.5 The phone

The same shape: formicaria's Android app is the shell, and web pages are tabs among formicaria's
screens. GeckoView sessions sit beside the formicaria screens in one tab list, not in a separate
*Browser* place (§12.10 changes accordingly when R2 is planned in detail).

### 14.6 What formicaria maintains

The shell script and styles, the app's shell mode, the kit (§13), and the add-on's page-reading part.
Everything else is Firefox's. **Yearly** (ESR major) the shell's hooks into Firefox's window are
re-checked; `browser-check` exercises them.

### 14.7 Risks

- **Privileged code**: the shell runs with the browser's own rights. It must never take input from a
  web page; its only input is formicaria's own tabs and the person.
- **Firefox's window internals change between majors**: keep the shell's hooks few and named, and
  test them in `browser-check`.
- **Accessibility and keyboard**: Firefox's own tab list is accessible; a restyle must keep it so.

### 14.8 Revised order

1. **S0, spike** (scratch ESR, the owner's Firefox untouched):
   - the autoconfig kit renames without repacking;
   - a formicaria panel with places and the tab list, built on vertical tabs and groups;
   - a note opened as a tab, and a note beside a page in split view;
   - an update round trip.
2. **The decision entry**: *formicaria is the window; web pages are its tabs*, superseding the
   sidebar/new-tab shape of 0.32.0 for the shell.
3. **S1**: the app's addresses and shell mode.
4. **S2**: the shell (panel, actions, opening rules) and the kit pipeline (§13.7, K1–K3).
5. **Release.**

### 14.9 Questions for the owner

1. **The tab list**: Firefox's own vertical tabs, restyled as formicaria's panel (recommended: less to
   maintain, accessible, sessions restored), or a list formicaria draws itself?
2. **Places as tab groups**: one group per formicaria place, with web pages in *Web* or in the place
   they were opened from (a recipe page in Food)? Recommended: in the place opened from.
3. **The address bar searching notes too**: yes (recommended, to try in S0), or web only?
4. **0.32.0's sidebar and new tab**: retire them once the shell exists (recommended), or keep them for
   formicaria in a plain Firefox?

---

## 15. Exploit the dependency: what formicaria hands to the engine (owner, 2026-11-15; plan)

> **The owner:** *"My idea is to exploit this dependency to offload things that were necessary before.
> Obviously, different devices will have to do it slightly differently since screen and inputs are
> different … The dependency is strong and can be helpful also on the mobile apps."*

Until now Gecko was a **cost**: a download and a pin. Committed to, it is also a **platform**. Much of
what formicaria built, or worked around because of an engine it did not control, can be handed to it,
**differently per device**:
- desktop: Firefox ESR plus formicaria's shell (§14);
- Android: GeckoView in formicaria's app;
- iOS: WebKit, which Apple requires outside the EU, so it gains none of this.

### 15.1 The inventory: formicaria's own work → the engine's

| formicaria does today | The engine gives | Desktop | Android | Effect |
|---|---|---|---|---|
| Tabs, panes, session restore, side-by-side | vertical tabs, tab groups, **split view**, session restore | ✅ (§14) | Android Components' tabs | the shell, not the Svelte app, owns windows |
| **PDFs**: an iframe on desktop; on Android **never rendered**, only `open_external` (`known-issues`) | **pdf.js**, with highlights and notes | ✅ | **to confirm** in GeckoView **[not verified]** | the papers plan's reader (Phase C) largely given; a long-standing Android gap closed |
| **Music in the background on a phone**: needs a Media3 service because Android's WebView has no Media Session (`music-plan` M4) | GeckoView's **MediaSession** API (wired to Android's media controls by the app, as Firefox does) **[doc]** | browser media keys | ✅ with wiring | M4 shrinks to wiring, not a player of our own |
| Translation (planned in Myrme) | **on-device translation**: Bergamot, `TranslationsController` in GeckoView since 121 **[doc]** | ✅ | ✅ | pages and the app's own text translated by the engine; Myrme's tool only for notes or mail outside a page (§13.5, revised) |
| Article extraction (Readability bundled in the add-on) | Reader View, the same Readability | ✅ | Android Components' reader | the add-on can call the engine's |
| Snapshot of a page (the add-on serialises the DOM) | **Save as PDF / page**, screenshots | ✅ | GeckoView print/PDF **[to check]** | richer copies for *Keep* |
| Export a note to PDF (not offered) | **print to PDF** with the app's print styles | ✅ | ✅ **[to check]** | a feature for free |
| Spelling in the editor | the engine's spell checker and dictionaries | ✅ | ✅ | for free |
| Android WebView workarounds: insets bridge (`--safe-*`), the keyboard, a WebView that forwards no console, base64-over-IPC blobs, a ranged `fmblob:` scheme | GeckoView's own handling of insets and the keyboard, its console, and loading from the same local server as the desktop | – | **if the app itself runs on GeckoView** (15.2) | Android-only code deleted; one transport on both |
| Ad and tracker blocking, add-ons | Firefox's protections, the add-on catalogue | ✅ | ✅ (GeckoView add-ons) | for free |
| UI tests in jsdom, which applies no CSP and no layout (`decisions.md`: a broken PDF and layouts hid there for weeks) | **the real engine headless**, the same Gecko the person runs | ✅ | ✅ | tests that see what people see |

### 15.2 The structural question: formicaria's own screens on GeckoView on Android?

Today the Android app is **Tauri over Android's System WebView** (Chromium), and §12 planned GeckoView
only for *web pages*. Exploiting the dependency fully means **formicaria's own screens run on
GeckoView too**, with one engine on desktop and Android:
- the same rendering, the same tests, the same add-ons;
- the WebView workarounds deleted.

**What it costs:**
- Tauri cannot drive GeckoView, so the Android shell becomes a **native Android app** (Kotlin with
  Android Components) hosting GeckoView and the Rust core, as it already hosts `AgentService` and the
  share bridge.
- The command channel moves from Tauri's IPC to either the desktop's local HTTP server, guarded by a
  per-launch token because other apps on the phone can reach loopback, or a built-in add-on's
  native-messaging port.
- **iOS keeps Tauri and WebKit**.

**This is the largest decision in the plan.** It makes the R2 phone release a rebuild of the Android
shell rather than a screen added to it. Recommended: decide after a spike that runs formicaria's
built UI in a bare GeckoView app and measures:
- start time against today's app;
- the 100 MB ceiling;
- memory beside the assistant;
- the keyboard and insets.

### 15.3 Per device: the same idea, shaped by screen and input

| | Desktop | Android | iOS |
|---|---|---|---|
| Engine | Firefox ESR (kit, §13) | GeckoView (in the APK) | WebKit (Apple's rule) |
| Shell | formicaria's frame in Firefox's window (§14) | a native formicaria app: tabs of notes, views and pages (15.2) | today's Tauri app |
| Tabs | vertical list, split view | a tab switcher sized for a thumb; one at a time, *beside* as a sheet | as today |
| Input | keyboard shortcuts, right-click | long-press, share sheet, a bottom bar | as today |
| Offloaded | §15.1, all | §15.1, all on GeckoView | none |

### 15.4 Revised order (replaces §14.8's steps 1–2)

1. **S0 (desktop spike)**: the autoconfig kit, the formicaria panel over vertical tabs, note beside page.
2. **G0 (Android spike)**: formicaria's built UI in a bare GeckoView app, the four measurements
   (15.2); pdf.js and print in GeckoView checked.
3. **Decision entries**: *formicaria is the window* (§14); *Gecko is formicaria's engine on desktop
   and Android* (with the measurements, and the Tauri question on Android).
4. Then §14.8 S1–S2 on desktop, and the Android shell plan from G0's results.

---

## 16. Lean and quick: the budgets the dependency must meet (owner, 2026-11-15; governs §13–§15)

> **The owner, final refinement:** *"make sure we keep minimal the memory usage and maximal the
> responsiveness of the app. Adding this dependency should not add lags and difficulty accessing the
> current features."*

This section **overrides** anything above that conflicts with it. Nothing is adopted until it meets
the budgets in 16.3, measured.

### 16.1 Design consequences

1. **formicaria runs once, not once per tab.** Each formicaria tab running its own copy of the app
   would multiply memory by the number of open notes. So the panel's formicaria items (notes,
   views) are **entries inside the one formicaria tab**: choosing one switches to that tab and tells
   the app which note to show (the shell channel, §14.4). Only **web pages** are real browser tabs.
   *Beside* = the formicaria tab and a web tab in split view. This revises §14.3; it also retires
   0.32.0's sidebar and new tab, which are each a further copy of the app.
2. **The formicaria tab is never unloaded and never waits**: pinned, excluded from Firefox's
   tab unloading, loaded first at start.
3. **Web tabs cost nothing until looked at**: restored on demand (Firefox's default), unloaded under
   memory pressure (Firefox's tab unloading).
4. **The shell's code is small and never busy.** It runs on the browser's main thread, so anything
   slow there slows every tab. Event handlers do no more than route; work goes to the add-on's
   background script or to `fm-serve`.
5. **The phone keeps formicaria's screens on Android's System WebView, and starts GeckoView only
   when a web page, a PDF or a translation needs it** (lazy, as §12.10 already planned).
   - System WebView is shared and preloaded by Android, so it is cheap to start.
   - GeckoView loads its whole engine on the first use.
   - §15.2's "formicaria's screens on GeckoView" is adopted **only if** G0 measures it within the
     phone budgets below; otherwise the hybrid stands.
6. **Built-in before add-on, where equal.** Firefox's own protections (and its built-in Rust blocker,
   if it can be switched on) before uBlock Origin, if they block comparably: one less process of
   work and memory. uBlock stays an offer.
7. **No feature moves behind the browser.**
   - Every place and action the app has today is reachable in the shell in the same number of
     presses or fewer.
   - The keyboard shortcuts stay.
   - The app keeps working in any browser and on a paired tablet.
   - If formicaria's browser is not set up or fails to start, the app opens in the default browser,
     as today.
   - In shell mode the app hides a control of its own **only** where the shell shows the same one (a
     check lists them).

### 16.2 What is measured, and how

- **Desktop memory**: total resident memory (PSS) of every process of the browser, with formicaria
  open and 0, 3 and 10 web tabs. Compared with **today**: formicaria in a tab of the person's own
  Firefox, the same tabs.
- **Desktop start**: from the double-click to formicaria drawn, cold (nothing running) and warm.
  Compared with today's launcher opening the default browser.
- **Responsiveness**: the app's own measures stay where they are: view switch, note open, typing
  latency (`gui-smoothness-plan-2026-10-18.md`, `crates/fm-app/tests/perf.rs`). Shell actions (*Keep*,
  *Beside*, switching place) acknowledge within one frame budget.
- **Phone**: §12's measures (APK, cold start, `dumpsys meminfo` with the model), plus the first web
  page's delay (GeckoView's lazy start).

### 16.3 Budgets **[proposed; the owner confirms]**

| Measure | Budget |
|---|---|
| Desktop memory, formicaria + 3 web tabs | **no more than** formicaria + the same 3 tabs in the person's own Firefox today (+10 % tolerance for the shell) |
| Desktop cold start to formicaria drawn | ≤ today's cold start of formicaria in the default browser |
| Desktop warm start (browser already open) | formicaria in front in ≤ 300 ms |
| View switch, note open, typing | **unchanged** (within measurement noise of today) |
| A shell action's first visible response | ≤ 100 ms |
| Phone cold start of the app | **unchanged** (GeckoView not started) |
| Phone memory, no web page open | **unchanged** |
| Phone first web page | ≤ opening another browser on the device (owner, §8) |
| Phone APK | ≤ 100 MB (owner) |

**Any budget missed blocks adoption** of the piece that misses it, not the whole plan. The piece
waits, or takes the cheaper route: the hybrid on the phone, uBlock instead of the built-in blocker,
fewer shell features.

### 16.4 Where the budgets are enforced

- **S0 and G0** (the spikes) report every measure against today's baseline before any decision entry.
- **`browser-check`** (the release gate, §13.1) records desktop start and memory, and fails a release
  that regresses them by more than the tolerance.
- **The existing perf tests** stay the guard for the app's own responsiveness.

### 16.5 Revised order (final)

1. **S0 (desktop spike)**, with §16.2's measures:
   - the autoconfig kit;
   - one formicaria tab plus the panel over vertical tabs;
   - note beside page;
   - **today's baseline measured first**.
2. **G0 (Android spike)**, with §16.2's measures:
   - the hybrid (System WebView screens, lazy GeckoView) against GeckoView for everything;
   - pdf.js, print and media session checked.
3. **Decision entries**, with the measurements.
4. **Build** what passed: the kit pipeline (§13), the shell and the app's shell mode (§14), the
   offloads that met the budgets (§15). Then the release.

### 16.6 Status (2026-11-15, 0.33.0)

**Built and gated** (`pixi run browser-check` green):
- the kit (`browser/kit/`, compiled into `fm-serve`): renaming, look, the formicaria button, the
  pinned formicaria tab, the privacy defaults as defaults;
- the pinned ESR (`browser/esr.toml`, `pixi run browser-bump`) and Mozilla's signature (`gpgv`);
- point releases prepared in the background and swapped in at a start; Firefox's updater off;
- Mozilla's services stripped by policy, Sync kept; web push behind a switch; uBlock Origin an
  offer, off (measured: +66 % memory);
- 0.32.0's sidebar and new tab retired (one formicaria);
- *Bookmark in formicaria*.

**Budgets**: memory +6.4 %, start unchanged (§16.3 holds).

**Next**:
- the app's shell mode and the window's own list (§14.4);
- G0 on a phone (§15.4);
- Windows and macOS;
- other languages.

