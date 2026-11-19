# Privacy: what the protections do, measured (2026-11-19)

The measures behind `privacy-plan-2026-11-19.md`. Run again with `pixi run privacy-coverage` (lists
as published that day), `pixi run usability-check` (its *someone who counts on the protections*),
`pixi run -e android phone-media-check` (the phone's) and `FM_CHECK_PRIVACY='{"ads": true}' pixi run
browser-check` (memory with a switch on).

## What each list set refuses, offline (`privacy-coverage`, 2026-11-19)

The corpus (`crates/fm-block/tests/data/`): 22 analytics, 7 social and 16 ad requests of well-known
services, and 21 ordinary ones (libraries, fonts, pictures, players, payments, captchas), as loaded by
a news page; 20 names app SDKs look up, and 12 ordinary app names.

| Pages: lists | analytics | social | ads | ordinary refused |
|---|---|---|---|---|
| 0.55, *Block trackers* (EasyPrivacy) | 21 of 22 | 6 of 7 | 4 of 16 | 0 of 21 |
| 0.55, both (+ EasyList) | 21 of 22 | 7 of 7 | 16 of 16 | 0 of 21 |
| now, *Block trackers* (+ uBO Privacy, Unbreak, Quick fixes, Badware) | **22 of 22** | 6 of 7 | 6 of 16 | 0 of 21 |
| now, both (+ uBO filters, Peter Lowe's) | **22 of 22** | 7 of 7 | 16 of 16 | 0 of 21 |

| App blocker | trackers refused | ordinary refused |
|---|---|---|
| 0.55 (EasyPrivacy through the pages' engine) | 9 of 20 | 0 of 12 |
| EasyPrivacy's whole-site rules | 9 of 20 | 0 of 12 |
| + DuckDuckGo's app list | 14 of 20 | 0 of 12 |
| + Peter Lowe's (**now**) | **17 of 20** | 0 of 12 |

What it says: the pages' network blocking was already near the ceiling on well-known services (the
gains of stage 3 are in what the corpus cannot show: stand-ins, scriptlets, procedural hiding, which
decide whether pages break and whether ads on YouTube-like sites go); the **app blocker almost
doubled**. Not measured here: lesser-known trackers, regional ad networks, first-party disguises.

## The computer, on the privacy test site (`usability-check`, 2026-11-19)

Before the fix of the day, formicaria's browser reported Firefox's category **"custom"**, not strict:
the kit had made strict a default, and Firefox sets a category's protections only when the category
changes. After: strict, with `fp`, `fpp` (fingerprinting protection), `btp` (bounce tracking), `qps`;
Facebook's pixel refused and Google Analytics answered by Firefox's stand-in (Firefox's own blocking
log); `Sec-GPC: 1` sent; `id=7&utm_source=mail&fbclid=…&gclid=…` on a link to another site arriving as
`id=7`; lookups by DNS over HTTPS to Quad9; *More sites* installing no extension. 9 of 9.

## Memory with ads on (`browser-check`, warm, 2026-11-19)

| Browser | Memory | Against unmodified Firefox |
|---|---|---|
| formicaria, default switches (0.55.0 release gate, strict not really on) | — | +4.3 % |
| formicaria, default switches (now: strict on, DNS over HTTPS) | 768 MB | **+1.7 %** (755 MB) |
| formicaria, *Block ads* on (uBlock Origin) | 1127 MB | **+47.8 %** (762 MB) |
| formicaria, *Block ads* on (uBlock Origin Lite), three starts | — | **+1.2 %, +5.9 %** (median of the three +4.8 %) |

Full uBlock Origin is over the +10 % budget; uBlock Origin Lite (the same author's lists, applied by
Firefox's own declarative blocking) is within it, so since 0.57.0 *Block ads* is on by default on the
computer with Lite, and full uBlock Origin is removed where formicaria had installed it (decisions,
2026-11-19 evening). Firefox ESR 153's
built-in `adblock-rust` refused nothing with the preferences it exposes (tried with EasyList, first-
and third-party requests).

## Where formicaria stands against the state of the art (2026-11-19)

| | formicaria | Comparison |
|---|---|---|
| Computer, trackers | Firefox strict (Disconnect lists, Total Cookie Protection, fingerprinting protection, bounce tracking) | the *Firefox strict* tier; privacytests.org's published results rank Tor and Mullvad Browser first, Brave close, standard Firefox mid-field on fingerprinting |
| Computer, ads | uBlock Origin Lite, on by default | full uBlock Origin is the best there is but +48 % memory; Lite +1–6 % |
| Phone pages | Brave's engine with uBlock Origin's lists, stand-ins, scriptlets, procedural hiding; DuckDuckGo's cookie rules; debouncing | Brave's level of blocking; canvas, WebGL and audio fingerprinting answered with made-up detail per site (Brave's approach), not a uniform fingerprint |
| Phone, other apps | DNS: app lists, CNAME, encrypted upstream | between TrackerControl/RethinkDNS (lookups and lists) and DuckDuckGo's App Tracking Protection (which also reads server names) |
| Address (IP) | not hidden | Tor, a VPN |

The phone's measures (blocked on a page made to be blocked, links cleaned, the app blocker end to end
on the emulator, memory with the lists) are in the release's `phone-media-check` log and the plan's
progress table.
