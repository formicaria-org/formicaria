# Stronger privacy on every platform — plan and progress (2026-11-19)

The owner, 2026-11-19, after a comparison with the state of the art (Tor and Mullvad Browser, Brave,
DuckDuckGo's browser and App Tracking Protection, uBlock Origin, TrackerControl, RethinkDNS): *"I
guess we can still improve … draft a plan"*, then *"finish the privacy plan after replanning it in
detail"*, *"and then release"*. Stage 0 came first, a fault found the same day.

Rules it serves: the browser's memory budget (+10 % over unmodified Firefox, warm, `browser-check`),
the APK under 100 MB, the computer-wide cap (~80 %), *systematic fixes* (a class closed at its root,
tested with invented cases), *an update brings everything at once* (2026-11-19), and nothing sent off
the device that a switch does not say.

## The owner's decisions (2026-11-19)

| Question | Answer |
|---|---|
| A list made for apps | **Open lists, and DuckDuckGo's too** where its licence allows: it is CC BY-NC-SA 4.0 (non-commercial), so it is **fetched by the phone at run time**, never shipped in the APK (GPL-3.0) |
| Encrypted lookups (a resolver then sees them) | **On by default**, Quad9; Mullvad DNS or the network's own as choices |
| Refuse apps' own encrypted-lookup services | **A switch, off** |
| Ads blocked by default on the computer | **Yes, if measured within +10 %**; otherwise a switch, off |

## What the research found (2026-11-19)

- **`adblock-rust` 0.13.5 already does what Brave does**: `BlockerResult.redirect` (a stand-in's
  data URL), `rewritten_url` (`$removeparam`), `UrlSpecificResources.injected_script` (scriptlets)
  and `procedural_actions` (JSON). We used `should_block` and plain hide selectors only.
- **Firefox ESR 153's strict mode already includes** fingerprinting protection (`fpp`), bounce-tracking
  protection (`btp`), query stripping (`qps`), known fingerprinters (`fp`), email trackers and Total
  Cookie Protection (`browser.contentblocking.features.strict`). The first draft's 2.8 is a check,
  not a change.
- **Firefox ESR 153 carries Brave's engine** (`ContentClassifier`, prefs
  `privacy.trackingprotection.content.protection.*`), fed from Remote Settings
  (`content-classifier-lists`: Disconnect's lists only) or from `…test_list_urls`. Tried for ads in 5.2.
- **The iPhone shows no page inside the app**: the page view is Android's (`__fmPage`); on iOS a link
  leaves for Safari, whose own protection (Intelligent Tracking Prevention) applies.
- **Lists and licences** (fetched 2026-11-19):

| List | Address | Licence | Use |
|---|---|---|---|
| EasyList, EasyPrivacy | `easylist.to/easylist/{easylist,easyprivacy}.txt` | GPL-3.0 / CC BY-SA 3.0 | pages (as before) |
| uBlock filters, Privacy, Unbreak, Quick fixes, Badware | `ublockorigin.github.io/uAssetsCDN/filters/<name>.min.txt` (no `!#include`) | GPL-3.0 | pages |
| URL tracking (`$removeparam`) | `ublockorigin.github.io/uAssetsCDN/filters/privacy-removeparam.txt` | GPL-3.0 | links |
| Peter Lowe's | `pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&mimetype=plaintext` | non-commercial | pages, apps; fetched, never shipped |
| uBO resources (stand-ins, scriptlets) | assembled from a pinned uBlock tag (`pixi run block-resources`) | GPL-3.0 | in the APK |
| Brave debounce | `raw.githubusercontent.com/brave/adblock-lists/master/brave-lists/debounce.json` | MPL-2.0 | links |
| DuckDuckGo app trackers | `raw.githubusercontent.com/duckduckgo/tracker-blocklists/main/app/android-tds.json` | CC BY-NC-SA 4.0 | apps; fetched, never shipped |
| Encrypted-DNS services | `raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/doh-onlydomains.txt` | GPL-3.0 | apps, behind its switch |

## Stage 0 — Settings that cannot go missing — **done**

The kit lagged one browser start behind the program; the restart notice was on a page no longer
shown; nothing checked the update path. Every start now refreshes the kit before the browser opens,
pages show their own ⚙ while the window cannot, the notice is on every page, and `usability-check`'s
*someone who just updated* guards it (fails without the fix, tried). `decisions.md` 2026-11-19.

## Stage 1 — measure first

| # | Step | Where |
|---|---|---|
| 1.1 | **A privacy test site**, served by the checks: a page loading real trackers and ads, a link with tracking parameters, `/echo` returning the request's headers (GPC), a service worker fetching a tracker | `ci/privacy_site.py` |
| 1.2 | `browser-check`: on the test site, the content-blocking log names the trackers, tracking parameters are stripped, `Sec-GPC: 1` arrives, strict's features (`fp`, `fpp`, `btp`, `qps`) are on, DNS over HTTPS is the chosen resolver | `ci/browser-check.py` |
| 1.3 | The phone check: the same site in a page view (requests refused, a stand-in answered, GPC, a link cleaned), then **the app blocker end to end**: on (consent by `appops`), a tracker's name refused for a shell lookup, an ordinary one answered, off restoring all | `ci/phone-media-check.py` |
| 1.4 | **Coverage offline**: a corpus of real tracker and ad requests by kind, plus ordinary ones; the share each list set refuses, and the ordinary ones wrongly refused | `crates/fm-block/tests/`, `docs/context/privacy-baseline-2026-11.md` |

privacytests.org's suite is not run: it needs its own hosted multi-domain servers and harness. Its
published results are cited in the baseline instead.

## Stage 2 — the cheap holes

| # | Step | Platform |
|---|---|---|
| 2.1 | Service-worker requests checked (`ServiceWorkerControllerCompat`), against the page that made them (`Referer`, else the page in front) | phone pages |
| 2.2 | A request's type from `Sec-Fetch-Dest` when given, otherwise a better guess | phone pages |
| 2.3 | The app's package name not sent (`X-Requested-With`: an empty allow list); third-party cookies refused explicitly | phone pages |
| 2.4 | **The app blocker's own list**: EasyPrivacy's whole-site rules + DuckDuckGo's app trackers + Peter Lowe's, as names matched by suffix (`fm-block::HostSet`), not the pages' engine | phone apps |
| 2.5 | The answer's CNAME chain: refused when any name in it is listed (`fm-block::dns`) | phone apps |
| 2.6 | **Private DNS set to a provider**: the sheet says the app blocker cannot see lookups, and where to change it | phone apps |
| 2.7 | Firefox's query stripping widened (`privacy.query_stripping.strip_list`) with the URL-tracking list's plain names | computer |
| 2.8 | One **list registry** (`fm-block::lists`): name, address, licence, refresh, platforms, shipped or fetched | all |
| 2.9 | The user's Privacy page: what is **not** protected (the address, the phone's fingerprint, Private DNS), the iPhone, and every contact formicaria makes | docs |

## Stage 3 — the phone's pages at Brave's level

| # | Step |
|---|---|
| 3.1 | The lists: + uBlock filters, Privacy, Unbreak, Quick fixes, Badware, Peter Lowe's |
| 3.2 | **Stand-ins**: uBO's resources in the engine; a request with a `redirect` answered with the stand-in instead of nothing |
| 3.3 | **Scriptlets**: the engine's `injected_script` for a site, registered at document start for that origin before the page opens (the page view runs it, so the site's CSP cannot refuse it) |
| 3.4 | **Procedural hiding**: `procedural_actions` applied by a small interpreter (`:has-text`, `:has`, `:upward`, `:remove()`, `:style()`), again as the page changes |
| 3.5 | **Links**: `$removeparam` through the engine (the 28 names kept as fallback), and **debouncing** of redirect wrappers (Brave's `debounce.json`, `fm-block::Debouncer`) |

## Stage 4 — trackers in other apps, done properly

| # | Step |
|---|---|
| 4.1 | The list of 2.4, refreshed weekly on the phone; *let through* per app as before |
| 4.2 | **Encrypted upstream, on by default**: allowed lookups by DNS over HTTPS to Quad9 (or Mullvad DNS, or the network's own); the network's resolver only when the encrypted one fails |
| 4.3 | **Stop apps going around the block** (switch, off): public encrypted-DNS services refused |
| 4.4 | Robust lookups: a small cache by TTL; IPv6 lookups |
| — | Not done: all traffic through the VPN to read server names (battery, speed, much code) |

## Stage 5 — the computer, beyond Firefox's own

| # | Step |
|---|---|
| 5.1 | **More sites** without extensions: DuckDuckGo's cookie-banner engine in formicaria's own add-on, the wider strip list (2.7) instead of ClearURLs |
| 5.2 | **Ads**: Firefox's built-in engine fed EasyList tried first; uBlock Origin measured warm; whichever blocks and fits +10 % is the default, on |
| 5.3 | **Encrypted lookups**: Firefox's DNS over HTTPS with the chosen resolver (mode 2: the system's as fallback), on by default |

## Stage 6 — the iPhone

Nothing to build: the manual says pages open in Safari, whose protection applies.

## Each step's proof

Unit tests with invented cases for every Rust piece; UI tests for the sheet's new switches; stage 1's
checks before and after; then CLAUDE.md's release order. A `decisions.md` entry for the defaults
(encrypted lookups to Quad9, ads if in budget), the lists fetched, and what the APK ships.

## Progress (2026-11-19)

| Stage | Status |
|---|---|
| 0 | **done**: the kit refreshed before the browser opens; ⚙ on the page while the window cannot; update-path check |
| 1 | **done**: `ci/privacy_site.py`; `usability-check`'s privacy persona (9 of 9); the phone's cases in `phone-media-check`; `pixi run privacy-coverage` (`privacy-baseline-2026-11.md`). Found: the browser was never in strict mode (fixed) |
| 2 | **done**: 2.1–2.9. 2.8 lives in `fm_block::lists` |
| 3 | **done**: lists, stand-ins and 153 scriptlets (`pixi run block-resources`, owner OK), procedural hiding, `$removeparam` and debouncing |
| 4 | **done**: app lists (17 of 20 app trackers, from 9), DNS over HTTPS upstream, *around*, cache, IPv6, Private DNS said |
| 5 | **done**: *More sites* without extensions; ads stay a switch (uBlock Origin +47.8 %, Firefox's engine refused nothing); DNS over HTTPS |
| 6 | **done**: nothing to build (Safari) |
| + | **done**: the phone's pages answer fingerprinting with made-up detail (`ui/page/fingerprint.ts`), the last gap of the comparison |
