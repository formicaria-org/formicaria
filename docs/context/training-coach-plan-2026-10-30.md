# Open training coach — plan (2026-10-30)

> **Owner's framing, 2026-10-30 — read before the reversals below.** *"Wearable and other sources of
> notes are congruent with each other … we take data files into a note management, which is already
> supported through assets, and we can visualize them and do things to them, all still in the scope
> of fm. So if we have data and public reputable knowledge, then we have what we need."* And:
> *"external tools that suggest things are already a de facto norm: myrme. Nothing stops us from
> entering coaching ability to that component, again based on known reputable sources."*
> So: a watch's FIT file is an **asset in a note**, like any other attached data file, not a new
> kind of input; and **coaching lives in Myrme** (through the contract), bound by the same
> read-at-source rule. The decisions this plan lists as reversals should be written in that light.
> **Order:** the F-Droid work (`fdroid-distribution-plan-2026-10-30.md`) comes first; this plan
> after it.

> **A proposal, researched, not built.** Nothing here is built, installed or decided. A coach fed by a watch **reverses** three standing lines: wearables were left out on purpose (`docs/context/tools-plan-2026-10-11.md:108-109`); *"No phone sensor: that is a separate decision"* (`decisions.md:11488-11489`); and *"There is no folder picker"* (`decisions.md:265`). It also extends several closed sets. So the dated decisions come first, each with a SUPERSEDED banner where it reverses something. Where this plan disagrees with the code, the code wins.

**What it is.** The aim is what TrainingPeaks offers, built only from open methods that have been read at the source: per-workout numbers, a load history, thresholds, workouts sent to the watch, plans, and adjustments the person sees and can override.

**Where the data comes from.** A Garmin watch that Gadgetbridge (GB) supports, for example the Forerunner 165 class (GB has a coordinator for it: `devices/garmin/watches/forerunner/GarminForerunner165Coordinator.java:11-14`). The route is GB on an Android phone, or USB to a laptop. Watch only: no chest strap, no second wearable.

**The plan is split.** fm's rule *"nothing ships as a number, a definition or a claim"* unless read at the source (`decisions.md:9563-9566`) blocks every session-load formula today (research §5). So this is a roadmap of **separate proposals**, P1 to P8. Only P1 is proposed for building now. Each later proposal is written, reviewed and decided on its own.

**Labels.** VC = code read. VPD = primary paper or doc read. SH = second-hand. UV = unverified. **(provisional)** = a number the planner set; the owner sets the real one.

**Snapshots.** GB `edbea98` (main, not a release), written `GB:` = `app/src/main/java/nodomain/freeyourgadget/gadgetbridge/`. fitparse-rs `fb82202`. Garmin `fit-python-sdk` `profile.py` (`FITP:`). GoldenCheetah `03e1666` (GPL-2.0+), used only as a black-box reference.

---

## The question first: when does data go from the watch to fm?

fm never talks to the watch. Data moves in five hops. Each hop has its own trigger and its own conditions.

| Hop | Trigger | Conditions (it does not happen unless…) | Typical delay |
|---|---|---|---|
| **1. Watch → GB** (Bluetooth) | The watch sends a synchronization request. GB answers when it covers workouts, activities, summaries or sleep, or is manual (`GB:service/devices/garmin/messages/SynchronizationMessage.java:55-59`; `…/FileTransferHandler.java:131-139`, VC). The person can also pull a sync in GB (`GB:service/devices/garmin/GarminSupport.java:675`, VC) | GB is running and connected. Android may stop GB in the background (UV) | Up to the watch's firmware (UV; experiment E0) |
| **2. GB → a phone folder** | **Activities:** GB's automatic FIT export runs inside its import of each new activity (`GB:service/devices/garmin/fit/FitImporter.java:733-738`, VC). It copies the original FIT byte for byte when it has it (`GB:export/AutoFitExporter.java:115-131`), and regenerates one only if not (`:133`). **Monitoring, sleep, HRV:** only in GB's ZIP backup, off by default, every N hours on WorkManager (`GB:util/PeriodicExporter.kt:40-72`) | An export folder is set, and the device is included ("all devices" is on by default; otherwise the device must be selected) (`AutoFitExporter.java:60-72`, VC). The activity has a track (`FitImporter.java:735-736`); whether indoor sessions have one is UV. Activities synced **before** export was turned on are not exported automatically. A write that fails part-way leaves a broken file that GB never overwrites (`AutoFitExporter.java:103-107, 109-140`) | Activities: seconds after hop 1 (by code path, not measured). ZIP: hours |
| **3. Folder → fm on the phone** | **Today:** the person picks the file by hand. **Proposed (P2):** fm reads the granted folder each time it opens or returns to the front | fm is opened | When fm is next opened |
| **4. fm → vault** | The import writes notes, one commit per import (`decisions.md:260-261`) | — | Immediate |
| **5. Phone vault → laptop** | Automatic backup: after writing has been still for about half a minute, at most every two minutes per vault, at once on leaving the app (`decisions.md:8994-8998`; amended 2026-10-17, `:11529`) | Never while someone is typing; **never a first backup** — a vault that was never sent waits for a person (`decisions.md:8999-9001`). The laptop then takes the notes at its own next sync | Usually under a minute on the phone, then the laptop's next sync |

**What reaches the laptop.** By default only the notes. The raw FIT travels only if the vault's `git_assets_max` lets git carry it (`decisions.md:2820-2831`). So every number fm shows must be in the note.

**Laptop over USB.** The watch mounts over MTP. The person copies files to a local folder and imports from a typed path. GB is not involved (P1).

---

## 0. Decisions the owner writes first

Each is a dated `decisions.md` entry answering `CLAUDE.md`'s four questions, with heading `## 2026-MM-DD — <title> \`#subject\``.

| # | Entry | For | Reverses or extends | What it settles |
|---|---|---|---|---|
| D1 | *Watch data enters fm as files another app wrote* | P1 | **Reverses** `tools-plan-2026-10-11.md:108-109`. **Settles** `decisions.md:11488-11489` | fm reads FIT files only. No Bluetooth in fm, no cloud reader (R14, `decisions.md:9401`; CSP `crates/fm-serve/src/main.rs:1693`). GB is an optional outside tool that declares itself (`decisions.md:2460-2483`) |
| D4 | *Reading FIT: licence, decoder, and what docs may say* | P1 | Extends the dependency policy (`deny.toml`; `ci/checks.sh:236-265`) | The FIT licence covers "documentation describing the FIT protocol" as well as code (LICENSE lines 4-7). §1 grants use "in any software created by Licensee"; §2(c) and §2(d) restrict making features available and modifiable derivatives. Legal status **UV**. D4 decides: (a) fitparser (MIT, profile generated from `Profile.xlsx`, `fitparser/src/profile/field_types.rs:4`) or a decoder written in fm, with a **provenance statement** of what it was written from; (b) whether tracked files may carry FIT message and field numbers; (c) whether to ask Garmin first. Until D4, no FIT numbers in tracked files and no code. fitparser also brings `chrono` |
| D2 | *In load arithmetic a day with no data is unknown, not rest* | P3 | Extends "nothing logged, never zero" (`decisions.md:10009`; `crates/fm-query/src/days.rs:4-8`) | Covered = rest marked, or an activity present; from P7 also a monitoring file spans it. No blanket "the watch records everything" declaration. Coverage measured over 126 days for long load, 21 for short |
| D3 | *The phone reads one folder the person grants* | P2 | **Reverses** "There is no folder picker" (`decisions.md:265`), phone only, read-only, one folder. Extends "Pictures only" (`decisions.md:12529-12530`) | Persistable grant in `MainActivity` via `ci/android-inject-service.sh`. Setting under Settings → *Connections* (`decisions.md:9054-9085`). Share menu accepts `.fit` and `.zip` only, checked by name and file magic in `takeShared`; anything else is refused with a sentence |
| D-S | *Training formulas wait for their sources* | P3 | Applies `decisions.md:9563-9566`; **not** an exception to it | Lists the texts of research §5 that must be read and registered before any load number, any 42/7 chart, any threshold rule or any safety screen ships. Anything that cannot be sourced is dropped. If the owner wants a model chart anyway, that is a separate dated exception, written as such |
| D6 | *A session's load is fixed when written, with its method and anchor* | P3 | Extends "a fact fixed when written" (`decisions.md:9452-9454`); keeps "totals are derived" (`:9337`) | Each workout note stores its loads, method and the threshold note in force. Long and short load are computed at read time. The chart marks anchor changes. Recompute is an explicit command |
| D7 | *Thresholds, bands and training checks* | P4, P5 | Extends closed log kinds (`decisions.md:9405-9436`) with `log: threshold`; closed check kinds (`:10005`); `rests_on` with `model`; the 13 `QUESTIONS` (`crates/fm-app/src/plan.rs:70`); "a new category needs a published classification" (`:9706-9707`); profile flags (`plan.rs:61-62`) with `hr_meds_or_rhythm` | Every formula, constant and band gets a *source* row. D7 says plainly that `rests_on: "model"` is weaker than anything Fitness carries today |
| D8 | *A Load view in Fitness* | P3 | Extends `Span` (`crates/fm-app/src/views.rs:164`); Health is the precedent (`decisions.md:12708-12713`) | One name on phone and desktop. Import is not an `AddAction` (`views.rs:153`) |
| D5 | *Writing FIT workout files* | P6 | New | Whether fm may write type-5 workout files, under D4's ruling |
| D9 | *Plans that move week to week, and adjustments the person accepts* | P5, P8 | Extends "the app suggests no loads" as relaxed (`decisions.md:10067`, `:10457-10471`); routines (nothing steps one forward, `outstanding.md:1239`); the Myrme allow-list (`crates/fm-app/src/commands.rs:980-1012`), a minor bump from `MYRME_CONTRACT = (1, 10)` (`crates/fm-app/src/lib.rs:34`) | Every adjustment cites a source, is a proposal, and never applies itself. Rules may lower or hold, never raise |
| D10 (optional) | *Effort from heart-rate bands* | P1 or later | Extends `moderate_minutes` (`plan.rs:663`) | Whether time in the register's ESC 2020 bands (`sources-2026-10-11.md:377`) may fill `effort`, so watch runs count toward the WHO card |

**Order.** D1 and D4 before P1. D3 before P2's folder part. D-S, D2, D6 and D8 before P3. D7 before P4. D9 before P5. D5 before P6.

---

## P1. Import activity files on the desktop (proposed now)

**Ships.** A host-only `import_fit {path}` that reads activity FIT files into `log: workout` notes with plain measured facts. The existing cards count them. No load, no chart, no threshold.

### P1.1 Path

1. The person mounts the watch over USB (MTP) and copies files to a local folder. The watch's folder name is UV (SH: `GARMIN/Activity/`; E0c reads it). fm never writes to the watch's storage.
2. `import_fit {path}` with a typed path. It goes in `REMOTE_DENIED` (`crates/fm-serve/src/main.rs:462`), like the Logseq importer (`decisions.md:263-267`). The source folder is only read; a test fingerprints it before and after (`decisions.md:213-214`).
3. One commit per run (`decisions.md:260-261`). A re-run imports nothing and writes no commit.

### P1.2 Idempotent import

- **Identity key.** `source_key = fit:` + HMAC-SHA-256 over the FIT `file_id` tuple (type, manufacturer, product, serial, `time_created`), keyed with a random salt stored once per vault. The serial is a 32-bit field (`FITP:109-111`), so a plain hash could be reversed by search; the salt prevents that. Re-runs skip known keys (`decisions.md:229-235`).
- **Content key.** The blob, if kept, is named by the SHA-256 of the file (`crates/fm-core/src/blob.rs`). The note records it as `source_fit`.
- **Near-duplicates.** Whether a GB-regenerated file (`AutoFitExporter.java:133`) keeps the watch's `file_id` is UV. So a second check looks for an existing workout with start ± 60 s, the same sport and the same timer time. It **proposes** a merge; it never merges by itself. It prefers the copy whose `file_creator` is the watch.
- **Partial or corrupt files.** fm checks header size, data size and CRC. A failing file is **refused and named**, never half-imported. Because GB never overwrites and never retries a failed export (`AutoFitExporter.java:103-107, 138-140`), the message also says: *delete this file in the folder and export the activity again from GB, or copy it over USB.*

### P1.3 Time and days

The current rulings: one app clock per device (`decisions.md:13354`); the vault keeps the clock its times are on, read from its first commit (`:13041`); no zone is ever written into a note (`:12984-13003`).

1. Every FIT time is converted to UTC seconds at once. FIT time counts from 1989-12-31 00:00 UTC. fitparser's `local_timestamp` becomes `DateTime<Local>` on the **host's** zone (SH from earlier reading of `fitparser/src/profile/mod.rs:63-76`; re-read before use), so the reader takes the raw value.
2. `start` is written as the Stamp rules require, on the vault's clock.
3. **`day` is a property fixed at import**: the watch's own local date of the start, from the activity message's `local_timestamp` minus `timestamp` offset. It never changes with the viewing device.
4. All daily sums bin by `day`. The same vault gives the same daily series on every device (oracle O17).
5. DST and half-hour offsets are tested under `test-two-clocks`.

### P1.4 The note (`log: workout`)

Conventions: one note per entry; reserved `log`; integers with the unit in the key (`PropertyValue::Int` is `i64`, `crates/fm-model/src/lib.rs:58-66`); totals derived, never stored (`decisions.md:9314-9343`); entries kept out of planning views by `notes_base` (`crates/fm-app/src/thread.rs:209-217`).

| Key | Rule |
|---|---|
| `start`, `day` | §P1.3 |
| `activity`, `counts_as` | Through a shipped sport table, so watch sessions count toward existing cards (`plan.rs:648`). Raw `fit_sport`, `fit_sub_sport` kept. Unknown sport → `activity: other`. The table holds names only, with its "as at" date, under D4 |
| `minutes`, `timer_s`, `elapsed_s` | `minutes` in whole minutes for existing cards (`plan.rs:663`) |
| `distance_m`, `ascent_m`, `descent_m` | m |
| `hr_avg_bpm`, `hr_max_bpm` | bpm, labelled as the watch's numbers |
| `rpe` | Typed, CR-10. A FIT `workout_rpe`, if present, is divided by 10 (`FITP:7209-7215`) |
| `source_fit`, `source_key` | §P1.2 |

`effort` is never filled from heart rate unless D10 says so. If D10 allows it, the bands are the register's ESC 2020 row only (moderate 55–74 %, high 75–90 % of maximum; HRmax typed, or Nes 2013 marked *estimated*).

### P1.5 Code

| Module | Crate | Rule |
|---|---|---|
| `fit.rs` (decode) | fm-core | Bytes in, records out. No `Store`, no network. Builds under `fm-serve --no-default-features` (`ci/checks.sh:322-335`). Model: `crates/fm-core/src/events.rs:1-17` |
| `import_fit` arm | fm-app `dispatch.rs` | Five places per arm (`docs/src/dev/adding-features.md:12-48`). Precedent: `calendar_sync` (`dispatch.rs:2243`) |

### P1.6 Tests (all in pixi gates, `pixi.toml:492-505`)

| # | Test | Where | Gate |
|---|---|---|---|
| T2 | Decoder never panics, **for N seeded inputs** (plain tests; `proptest` is not in `Cargo.lock`) | fm-core | `test` |
| T4 | Synthetic FIT fixtures from a test-only writer: steady run, intervals, hill, over midnight, bad CRC, truncated, unknown message. Invented values only | fm-core | `test` |
| T4b | **Independent decode, pinned as data.** Expected field values for at least one file, produced **outside the repo** by a second decoder (fitdecode or GB), committed as numbers only. The file is one whose licence was read, or a synthetic file. This catches errors fm's writer and reader would share | fm-core | `test` |
| T7 | I1: importing twice adds 0 notes, 0 commits. I2: same identity, other bytes → "two versions", no overwrite. I2b: a GB-regenerated file and its raw file → a merge proposal, not two activities. I3: a bad file is refused and named, with the re-export sentence; **no good copy ever arrives** is covered. I4: source folder fingerprint unchanged | `crates/fm-app/tests/import_fit.rs` | `test`, `test-native-git` |
| T8 | Time: `day` from the watch offset; 23:50 start; 23- and 25-hour DST days; half-hour offset; **same vault, two TZ values, identical daily sums** | **fm-app** (runs under `test-two-clocks`, `pixi.toml:380`, which has no `-p fm-query`) | `test`, `test-two-clocks` |
| T10 | Privacy: `ci/checks.sh` fails if `fit.rs` names `std::net`, `TcpStream`, `ureq`, `reqwest` or a URL. No serial number in clear in any note; the key is salted (O20) | `ci/checks.sh`; fm-app | `checks`, `test` |
| T11 | Tests never touch real vaults: `FM_VAULTS` in a temp dir under the process lock, asserted first (`crates/fm-app/tests/blank_note.rs:8-18`, `:84`) | every fm-app test | `test` |
| T12 | Mock (`ui/src/lib/mock.ts`, `ci/checks.sh:1238-1255`), reference entry (`docs/src/reference/commands.md`), `REMOTE_DENIED` | — | `checks`, `test-ui`, `check-ui` |

Public FIT files (fitparse-rs and fitdecode fixtures) are read in place by an opt-in local test and never vendored; their licences are UV.

**Exit test.** T2, T4, T4b, T7, T8, T10–T12 pass. Importing a folder twice adds nothing. Watch runs count on the existing cards.

---

## P2. The phone (separate proposal)

1. **By hand, without D3.** The file input already reaches the system picker (`ui/src/lib/NotePanel.svelte:2254-2266`).
2. **Transport.** The existing `ingest` route always writes an asset note: `ingest_unlocked` calls `commands::asset_note` (`crates/fm-app/src/dispatch.rs:3767-3768`), which always creates `Object::new(Kind::Asset, …)` (`commands.rs:2192`). So FIT files must **not** go through `ingest`. A new arm takes a chunk session, assembles it with `fm_core::chunked::assembled` (`crates/fm-core/src/chunked.rs:155`), decodes and imports, and writes **no** asset note. The blob is kept only if the person opts in.
3. The page sends chunks as base64 (`mobile/src-tauri/src/lib.rs:454-466`). That is fine for activity FITs, which are small. It is not fine for a GB ZIP (P7).
4. **D3 folder grant.** On open and resume, `MainActivity` lists the folder **off the UI thread** through the page's async path (listing a SAF folder can be slow, SH). New `*.fit` files by name + size + modified time. A file whose size or time changed between two listings a few seconds apart waits for the next resume. A revoked grant is shown as "folder no longer readable".
5. **Share menu.** `.fit` and `.zip` only, checked by name and magic; share cap 25 MB (`ci/android-inject-service.sh:616`).
6. **Tests.** Phone-route I1 (twice → 0 notes, 0 commits, **0 `Kind::Asset` notes**). Every Android IPC command is `(async)` (`ci/checks.sh:1050`). Device checklist **on a throwaway vault**: grant survives reboot; revoked grant shown; growing file left for later; folder on removable storage.

**Exit test.** A file GB auto-exports appears at the next resume; the laptop shows it after its next sync.

---

## P3. Load history (separate proposal, after D-S)

**Blocked today.** No load formula and no 42/7 constant has a *source* row. P3 starts only when research §5's texts are read and registered. What follows is the shape P3 would take; any part that cannot be sourced is dropped.

- **Default view: measured facts.** Weekly time, distance, and time in the register's bands. These need no model.
- **Methods, never mixed.** One method per person per sport in a series (Vermeire 2021, PMID 33691278, VPD). A session without that method's inputs lowers coverage; it is not swapped for another number. All sourced loads are stored on the note.
  - **HR load:** per-sample Banister TRIMP, only after the constants are read at source. Needs `sex` (`plan.rs:344-363`); without it, "needs sex". Replaced by RPE load when `hr_meds_or_rhythm` is set (P5).
  - **RPE load:** only after Foster 2001's full text confirms RPE × minutes.
  - **Pace load:** not offered until a published method is read. Its scale must name its anchor. It is never the default while CS is untested.
  - No "100 = 1 h at threshold" scale unless sourced (its only source today is GPL code).
- **Long and short load, freshness.** `1 − e^(−1/N)` form, documented; O2 separates it from `1/N`. Constants shown with whose they are, changeable by the person, with Vermeire 2022's caution quoted: general constants "should be avoided" (PMID 35320776, VPD). The chart reads *"a history of load, not a prediction"*. Anchor changes are marked; recompute is offered (D6).
- **Coverage (D2).** Covered = rest marked or activity present (monitoring from P7). Coverage over 126 days for long load, 21 for short. Below 80 % (provisional) the number is not shown. **Until P7, show weekly sums only**; the EWMA chart waits for monitoring data.
- **Load trend:** a plain number, no colour bands. Week-over-week distance change, plain.
- **Rejected:** ACWR (Impellizzeri 2020, PMID 32502973, VPD); injury-risk colours; Banister performance prediction (Busso & Chalencon 2023, PMID 36791017, VPD).
- **Code.** `load.rs`, `series.rs` in fm-query (no storage, no `std::fs` — `ci/checks.sh:46-65`). **Binning lives in fm-app**, or fm-query is added to `test-two-clocks`.
- **UI (D8).** A fourth Fitness view, **Load**, one picked series at a time (like `ui/src/renderers/Trends.svelte`). Not "Balance" (already "Balance days this week", `crates/fm-app/src/shipped/plans.json:448`) and not "Fitness". Nothing drawn over Week, Exercises, Regime, the board or the agenda (`decisions.md:9857-9882`, `:9910-9912`). Mockups for phone and desktop shown before the build (`decisions.md:9878-9879`). Renderers literal-free (`ci/checks.sh:179-188`); FIT names are untrusted text (`:189-204`). The UI reads the app clock (`ci/checks.sh:2483`).
- **Tests.** O1–O8, O16–O19 as applicable to what ships; GC parity (O15) as committed numbers only.

---

## P4. Thresholds (separate proposal)

- **Threshold note** (`log: threshold`, D7): one note per change; `measured`, `cs_mm_s`, `d_prime_m`, `hr_max_bpm`, `hr_rest_bpm`, each with `*_how` (`typed`, `fit`, or the named estimate) and `efforts` links.
- **CS.** Least squares on `D = D′ + CS·t` (Smyth & Muniz-Pumares 2020, PMID 32472926, VPD). Only efforts the person marks maximal (race or test). Refuses below 3 distances, or below Smyth's inclusion floor of 24 activities in 16 weeks. Shows SEE, efforts and dates, and "from training, not tested". A *proposed* note the person accepts.
- **HRmax.** Typed, or Nes 2013 (`sources-2026-10-11.md:377`) marked *estimated*. Tanaka only if added as a second named source, never blended (`decisions.md:9567-9569`). The observed maximum is shown as "highest seen: N bpm on date", for the person to adopt.
- **HRrest.** Typed; from P7 the watch's value, labelled as the watch's.
- **Not offered:** threshold HR "near CS" (no source); Conconi / HR deflection (Maffulli 2026, PMID 42872971, VPD).
- **Artefacts.** HR below rest or above 230 → no sample. Cadence lock (UV) flagged only. Counts shown on the session screen.
- **Tests.** O9, O10, O19.

---

## P5. Plans, safety (separate proposal)

- **Safety first, sourced.** The ACSM pre-participation text is read and registered before anything ships. The screen reuses the existing `kidney` and `diabetes_meds` flags (`plan.rs:61-62`) rather than asking twice.
- **The stop list** is shown at the start of every plan and with every proposal, not only after a tag. When the person records a listed symptom, every proposal stops and the screen says to seek medical care.
- **`not_for`.** Every training card carries `not_for: ["pregnant", "eating_disorder"]`; pregnancy routes to the existing pregnancy-activity card. A new flag `hr_meds_or_rhythm` (beta-blockers, arrhythmia, pacemaker) needs its own source row; when set, HR measures are replaced by RPE. A minimum age needs a source row.
- **`plans.json` additions:** `rests_on: "model"`; question "train for an event"; `anchors[]`; `weeks[]`; `progression {rule, source}`; `taper {weeks: 2, volume_cut_pct: [41, 60], keep: ["intensity","frequency"], source, population: "trained athletes"}` (Bosquet 2007, PMID 17762369, VPD); `adjust[]`; `screen`. New check kinds `load_sum`, `band_minutes`, `long_session`; a week below coverage shows *partial*.
- **Intensity templates.** Pyramidal default; polarised a choice; both studies shown (PMIDs 24550842, 39888556).
- **Adjustment rules.** Data with a `source`; a proposal showing what, why and evidence; lower or hold, never raise.
  - **(a) Distance jump.** When distance rose more than 30 % over a 2-week period: *"In one cohort of novice runners, larger jumps showed a non-significant trend (P = .07) toward more distance-related injuries."* Nielsen 2014, PMID 25155475 (VPD). Shown only where the population matches (new runners). Never "the 10 % rule" (Buist 2008, PMID 17940147).
  - **(b) Stale anchor.** Propose a re-test when CS efforts are old. The interval is **convention**, labelled so; Smyth's 16 weeks is a data window.
  - **(c) Taper** in the 2 weeks before `event_date`, with its population stated.
  - **No HRV rule.** Kiviniemi used morning HF power in 9 moderately fit men (PMID 17849143); the watch gives a nightly RMSSD average. Deferred unless the owner rules otherwise in D9.
  - Missed sessions are not piled onto later days (no source).
- **Never claimed:** injury risk or prevention; performance prediction; diagnostic meaning of HR, HRV or resting HR; that a plan is "safe". (Fitness already carries review-based cards; the Health-only ruling at `decisions.md:13513` is not borrowed.)
- **Tests.** O13 (numeric), T13 (every field, rule and check kind has a source row).

---

## P6. Workouts to the watch (separate proposal, after D5)

- **Steps** live in the planned note's body as a closed Markdown grammar (`steps.rs`, fm-core); a line it cannot read is refused. Planned entries are the existing ones (`crates/fm-app/src/log.rs:28-30`).
- **Encoder** `fit_write.rs`, own code (fitparser is decode-only), subject to D4/D5 provenance. Targets are **absolute only**: bpm with the +100 offset (`FITP:28580-28582`) or m/s at scale 1000. No % targets, because the watch would resolve them against its own settings (UV which).
- **Routes**, each a gated experiment:
  - **A. Share to GB.** `FileProvider` intent (precedent `ci/android-inject-service.sh:478-510`) → GB `FileInstallerActivity` (`SEND */*`) → `GarminFitFileInstallHandler` (magic check, ≤ 10 MB, `:107`; needs `WORKOUT_DOWNLOAD`, `:202-204`, reported at runtime, UV) → upload (`GarminSupport.java:1835-1846`). VC at main; **gate G1**: re-read at the installed GB release tag.
  - **B. USB copy** to the watch's workouts folder (SH/UV).
  - **C. GB's own encoder** (`GarminWorkoutFitEncoder.kt`, AGPL) only as a black-box oracle; files produced outside the repo; nothing of GB's code or tables is shipped.
- **Experiments** on a GB-supported watch with invented workouts: E1 (route A), E2 (route B), E3 (field-by-field match against GB). Until E1 or E2 passes, the UI offers *Save .fit file* only.
- **Tests.** T9 round trip; +100 offset; speed scale; repeats; **no step carries a % target**.

---

## P7. GB ZIP: resting HR, HRV, worn days (separate proposal)

- The ZIP holds GB's database, preferences and external files (`GB:util/backup/ZipBackupExportJob.java:85-100`, VC). fm reads only `*.fit` entries, **natively** (Kotlin or Rust), streaming, never through the page or `ingest`, and never stores the ZIP.
- `zip` enters fm-core only under its own D4 `#toolchain` line (today only `fm-fetch` links it, with waived advisories).
- Day note (`log: body`): `resting_hr_bpm`, `hrv_night_ms` (raw ÷ 128, `FITP:25614-25622`), `sleep_min`, `covered: 1`. A night belongs to the local date it ended (open question).
- HRV shown as a trend against the person's own baseline, no proposal.
- **Tests.** A ZIP read keeps only `*.fit` entries and stores nothing else; O14 scale check.

---

## P8. The assistant (separate proposal)

- Rust computes everything. Myrme only explains and points to sources.
- Allow-list only `minutes`, `distance_m`, `planned` on `apply_props` (`commands.rs:980-1012`). A proposal already carries a body (`commands.rs:976-978`), so steps are not a property. Contract bump to `(1, 11)` (`lib.rs:34-46`).
- "Never raise intensity" needs a defined ordering over the steps grammar, specified first. Rust refuses a raising proposal.
- Myrme never picks a tool and never runs import (R2, `decisions.md:9291-9312`).

---

## Review log

Spot-checks in this pass: `decisions.md:9563-9566`, `:8994-9001`, `:12984-13003`, `:13041`, `:13354`; `dispatch.rs:3767-3768`; `commands.rs:2186-2195`; `chunked.rs:155`; `mobile/src-tauri/src/lib.rs:454-466`; `pixi.toml:380`; `plans.json:448`; `plan.rs:61-62`; `lib.rs:34`; `sources-2026-10-11.md:377`; `outstanding.md:1213, 1239, 1370`; GB `AutoFitExporter.java` (export path); `profile.py` fields; FIT LICENSE §1-3; abstracts 35320776, 25155475, 17849143, 11708692; oracles O1, O4, O9 recomputed.

| # | Objection | Severity | Outcome |
|---|---|---|---|
| 1 | No load formula or 42/7 may ship under "nothing is invented" | blocker | **Fixed.** Confirmed at `decisions.md:9563-9566`. Plan split; P3 blocked behind D-S and the reading list (research §5); load methods marked research-only; "100 at threshold" scale dropped |
| 2 | `ingest` writes an asset note per file | blocker | **Fixed.** Confirmed (`dispatch.rs:3767-3768`, `commands.rs:2192`). P2 uses a new chunk-session arm via `chunked::assembled` with no asset note; ZIP handled natively; tests for 0 `Kind::Asset` notes |
| 3 | Time model cites a superseded ruling; chart depends on device | major | **Fixed.** Current rulings cited (`:12984-13003`, `:13041`, `:13354`). `day` fixed at import from the watch's local date; binning by `day`; O17 |
| 4 | Second HRmax formula against the register; unsourced HRmax/threshold rules | major | **Fixed.** Nes 2013 (register row) is the estimate; Tanaka only as a named second source; observed max shown as a fact; threshold HR dropped |
| 5 | Vermeire 2022 misstated | major | **Fixed.** Abstract re-read; the paper advises against general constants. Quoted on the chart; constants named and changeable |
| 6 | Coverage rule restores silent zero-fill | major | **Fixed.** Declaration dropped; covered = rest or activity (monitoring from P7); 126/21-day windows; weekly sums only until P7; O16 |
| 7 | Base64 transport; SAF listing cost | major | **Fixed.** Confirmed base64 at `lib.rs:454-466`. FIT only through the page; ZIP native; listing off the UI thread |
| 8 | Ambiguous §2.26; backup conditions omitted | minor | **Fixed.** Cited `outstanding.md:1239` (the line that says nothing steps a routine forward; the objection's 1236 is a nearby line). Backup conditions added to hop 5 |
| 9 | "Balance" collides | minor | **Fixed.** "Freshness" (provisional); grep found no existing use |
| 10 | Health "settled" ruling borrowed | minor | **Fixed.** Not cited; D7 states `model` is weaker |
| 11 | Username leaks through a dashed scratchpad path | major | **Fixed** in both documents: no scratchpad paths. Widening the guard regex (`ci/checks.sh:1144`) is a separate change for the owner |
| 12 | Private-research wording; device reads as the owner's | minor | **Fixed in part.** Private references removed. The FR165 class stays as the example because the product brief names it; it is now framed as one entry in GB's coordinator list, and experiments run "on a GB-supported watch" |
| 13 | Rule (a) overstates Nielsen 2014 | major | **Fixed.** Confirmed: 874 novices, P = .07, 2-week window. Reworded, distance only, population-matched |
| 14 | HRV rule uses a different metric | major | **Fixed.** Confirmed: Kiviniemi used morning HF power; the FIT field is a nightly RMSSD average at scale 128. Rule deferred; trend only |
| 15 | CS from training biased low | major | **Fixed.** Maximal-marked efforts only; 24-activity floor; "from training, not tested" |
| 16 | Pace load default, least sourced, from GPL code | major | **Fixed.** Not default; not offered until a primary method is read; anchor must be named; no GC structure reproduced |
| 17 | Chart sums across anchor changes | major | **Fixed.** Anchor changes marked; explicit recompute; O19 |
| 18 | Rule (b) window; taper population; `workout_rpe` ×10; `workout_hr` % | minor | **Fixed.** All four confirmed in `profile.py` and the abstracts and applied |
| 19 | No % HR targets | major | **Fixed.** Absolute targets only; T9 assertion |
| 20 | FIT licence covers the documentation | blocker | **Fixed.** Confirmed (LICENSE lines 4-7, §2(c), §2(d)). Also noted §1's grant "to use the FIT protocol in any software created by Licensee", which the objection left out; legal status stays UV. D4 now decides docs content and provenance; no FIT numbers in tracked files until then |
| 21 | GB AGPL; G1 list | minor | **Fixed.** Nothing of GB shipped; oracle files out of tree; `AutoFitExporter` added to G1 |
| 22 | Serial hash reversible; regenerated file may not match | major | **Fixed.** Confirmed `uint32z` (`FITP:109-111`). Salted HMAC; near-duplicate merge proposal; test I2b |
| 23 | A failed GB export is permanent | major | **Fixed.** Confirmed (`createFile` then stream; log only; no overwrite). Refusal message tells the person to delete and re-export or use USB; I3 covers "no good copy ever" |
| 24 | Hop table missing preconditions | major | **Fixed, one correction.** Conditions added. Device selection matters only when "all devices" is off; it defaults on (`AutoFitExporter.java:60-62`) |
| 25 | Share filter breadth; removable storage | minor | **Fixed.** Name and magic check; removable storage in the device checklist |
| 26 | Fixtures share writer and reader errors | major | **Fixed in part.** T4b pins an independent decode as data in CI for P1. **Rejected:** making E3 a gate for M1, because E3 compares *encoders* (workout files) and says nothing about the activity decoder; it stays a P6 gate |
| 27 | Two-clock gate does not run fm-query; missing oracles | major | **Fixed.** Confirmed `pixi.toml:380`. Binning tests in fm-app; O16–O20 added; O13 numeric |
| 28 | Seeded loops, fuzz tooling, rejected-feature oracles, real vault | minor | **Fixed.** "For N seeded inputs"; fuzz target dropped (no pixi task, needs nightly); O11/O12/O14 marked documentation only; throwaway vault |
| 29 | `not_for` and screening miss groups | major | **Fixed.** Confirmed flags at `plan.rs:61-62`. `not_for` on every card; `hr_meds_or_rhythm` with a source row; RPE replaces HR; minimum age needs a source |
| 30 | Screening and stop list SH; stop reactive | major | **Fixed.** Behind the reading list; stop list shown up front and with every proposal; existing flags reused |
| 31 | Too wide for one proposal | major | **Fixed.** Split into P1–P8; only P1 proposed now |
| 32 | Myrme `steps` via `apply_props`; intensity ordering | minor | **Fixed.** Allow-list without `steps`; ordering specified before P8 |

---

## Residual risks

| Risk | State |
|---|---|
| **FIT licence** | Unresolved (legal UV). P1 cannot start without D4. Asking Garmin may be the only clean answer |
| **No load formula is sourced yet** | P3 may never ship if the primary texts cannot be read. P1's measured facts still give value |
| **GB read at main, not a release** | Gate G1 before P2 and P6 |
| **Indoor sessions may not export** | UV until E0b; fallback is USB or the ZIP route |
| **GB stopped in the background** | Hop 1 delay unknown until E0 |
| **Regenerated vs raw file identity** | UV; mitigated by the near-duplicate proposal |
| **Wrist HR in intervals** | Stated as a limit; RPE stays available |
| **Phone blobs die with an uninstall** (`docs/context/features.md:15`) | Notes carry every number |
| **Personal data in fixtures** | Synthetic only; `ci/checks.sh:1143-1182` guards identifiers, but misses the dashed `-home-<name>` form until widened |
| **Trademark status of TSS etc.** | SH; neutral names used |
| **Scope creep across P2–P8** | Each is its own proposal and decision |

---

# Part 2 — Open training coach: research (2026-10-30)

**Scope.** This document covers the methods fm could compute for a training coach. For each it gives the evidence and its limits, and how GoldenCheetah (GC) implements it. The output is formulas and test oracles that fm can check its numbers against. The data source is a Garmin watch that Gadgetbridge (GB) supports, for example the Forerunner 165 class. The data arrives as raw FIT files. No chest strap is assumed.

**Labels.**
- VC = verified code: the source was read.
- VPD = verified primary document: the abstract or full text was read.
- FH = first-hand report.
- SH = second-hand.
- UV = unverified.

**Shipping rule (fm, `docs/context/decisions.md:9563-9566`).** *"A source counts when it has been read at the source … nothing marked search or memory ships as a number, a definition or a claim."* Every method below marked SH or UV is **research only**. It cannot ship as a number in fm until a register row marked *source* exists in `docs/context/sources-2026-10-11.md`. Section 5 lists what must be read first.

**Snapshots.**
- GoldenCheetah: commit `03e1666` (2026-10-28). Licence GPL-2.0-or-later (`COPYING`; file headers such as `src/Metrics/PMCData.cpp:1-18`).
- Garmin FIT Python SDK, `garmin_fit_sdk/profile.py` (written `FITP:` below).
- Gadgetbridge `edbea98` (main, 2026-10-30). This is not a release.
- PubMed abstracts came from NCBI E-utilities on 2026-10-30.

**Trademarks.** TSS, rTSS, hrTSS, CTL, ATL, TSB, NP, IF and "Performance Management Chart" are TrainingPeaks terms. That they are registered marks is SH; no registry was checked. fm uses neutral names:

| TrainingPeaks term | fm name |
|---|---|
| TSS / hrTSS | **session load**, with a method suffix (`hr`, `pace`, `rpe`) |
| CTL | **long load (42 days)** |
| ATL | **short load (7 days)** |
| TSB | **freshness** (provisional; "balance" and "fitness" are taken in fm) |
| ramp rate | **load trend** |

GC labels the same series LTS, STS and SB (`src/Metrics/PMCData.cpp:978-988`, VC).

---

## 1. Methods table

| Method | Formula (units) | Inputs from the watch via FIT | Evidence label and citation | Limits | Verdict for fm |
|---|---|---|---|---|---|
| **Banister TRIMP** (HR reserve, exponential weighting) | `TRIMP = t_min × x × 0.64 × e^(b·x)`, `x = (HR − HRrest)/(HRmax − HRrest)`; b = 1.92 male, 1.67 female (AU) | `record.heart_rate` (FITP:9989); `session.avg_heart_rate` (FITP:5069); resting HR from monitoring; HRmax typed or observed | Formula and constants: **SH** (Banister 1991, a book chapter, not read). Morton 1990, PMID 2246166 (VPD, abstract): the abstract gives the model but **no formula and no constants**. GC: VC `TRIMPPoints.cpp:34-36, 91-97` | Sex constant is a group fit. The average-HR form underestimates load in variable sessions (oracle O5). Wrist-HR lag. Needs correct HRmax and HRrest | **Research only** until a primary text stating the constants is read. Then per-sample, as the HR method |
| **Edwards zone TRIMP** | `Σ minutes_in_zone_i × i`, i = 1..5 for 50-60 / 60-70 / 70-80 / 80-90 / 90-100 % HRmax | `time_in_zone.time_in_hr_zone` (FITP:2684), or computed from records | Formula: **SH** (Edwards 1993, a book). Vermeire 2021, PMID 33691278 (VPD): load methods "cannot be used interchangeably because they evolve differently" | Step weights. Bands tied to %HRmax, and fm's register has no 5-band scheme | **Research only.** Needs the book and a registered band scheme |
| **Lucia TRIMP** | `Σ minutes_in_phase_k × k`, k = 1/2/3 below VT, VT to RCP, above RCP | Needs ventilatory thresholds, which a watch cannot measure | Phases: Lucia 2003, PMID 12750600 (VPD). Weights 1/2/3: SH | Needs lab thresholds | **Avoid** |
| **Session RPE** (Foster 2001) | `sRPE = RPE(CR-10) × minutes` (AU) | Typed RPE. The FIT profile defines `session.workout_rpe` (FITP:7209-7215: comment "multiplied 10x", scale 1), so a raw value must be divided by 10. Whether the watch writes it is UV | Foster 2001, PMID 11708692 (VPD, abstract): the session RPE method is valid across exercise types and scores higher than the HR method. The abstract does **not** state "RPE × minutes": the formula is **SH**. GC: VC `TRIMPPoints.cpp:359, 372` | Subjective. Different scale from HR load | **Research only** until Foster's full text is read. Then the no-sensor fallback and the method for strength work |
| **Pace load, GOVSS-like** (Skiba) | GC: power from 120-s rolling speed and slope; `LNP` = 4th-power mean of 30-s rolling power; `GOVSS = secs·(LNP/RTP)²/36` | `record.enhanced_speed` (FITP:10709), `enhanced_altitude` (FITP:12855); weight, height, CS | Method: **UV** (Skiba's text is not peer-reviewed and was not read). The only description read is GPL code (GC `GOVSS.cpp:41-56, 96-158, 400-418`, VC) | No validation study read. Its "100" is anchored to a one-hour effort at a threshold, but CS is not a one-hour pace (Poole 2016, below). The 30-s/4th-power structure is known here only from GPL code | **Avoid** until Skiba or an equivalent primary text is read. fm must not reproduce the GC structure from the code |
| **Grade-adjusted pace (GAP)** | `v_GAP = v × Cr(i)/Cr(0)`, `Cr(i) = 155.4i⁵ − 30.4i⁴ − 43.3i³ + 46.3i² + 19.5i + 3.6` J·kg⁻¹·m⁻¹ | speed and altitude records | Minetti 2002, PMID 12183501 (VPD, abstract: level Cr 3.40 ± 0.24 J/kg/m). The polynomial is **SH** (in the paper body, not read; also in GC `GOVSS.cpp:51`) | The polynomial gives 3.6 on the level, against the measured 3.40. Downhill values come from trained runners. GPS and barometer noise | **Research only** until the paper body is read. Needed for CS from training data |
| **Long/short load chart** (42/7 EWMA) | `L_d = L_{d−1}·e^(−1/42) + load_d·(1 − e^(−1/42))`; same with 7; freshness = long − short | Daily summed session load | GC VC `PMCData.cpp:59-68, 132-136, 521-556`. The 42/7 constants are convention: **SH**. Vermeire 2022, PMID 35320776 (VPD): *"the use of general constants should be avoided since they do not account for interindividual differences and differences between training-load methods"* | The paper advises against exactly the general 42/7 constants. Descriptive only | **Research only** until a published text stating 42/7 is read. If shipped, show it as history, name whose constants they are, let the person change them, and show Vermeire's caution |
| **Banister impulse-response, fitted per person** | `p_t = p0 + k_a Σ e^(−(t−s)/τ_a) w_s − k_f Σ e^(−(t−s)/τ_f) w_s` | Daily load plus repeated performance tests | Morton 1990 (VPD); Hellard 2006, PMID 16608765 (VPD, full text); Busso 2003, PMID 12840641 (VPD); Busso & Chalencon 2023, PMID 36791017 (VPD); Borresen & Lambert 2009, PMID 19691366 (VPD) | **Contested / weak.** τ_a–τ_f correlation 0.99 (Hellard). Prediction "not satisfactory for individual training planning" (Busso 2023). Accuracy "has proven poor" (Borresen 2009) | **Avoid** |
| **ACWR** | acute ÷ chronic load | Daily load | Lolli 2019, PMID 29101104 (VPD, title only); Impellizzeri 2020, PMID 32502973 (VPD) | Inaccurate even when uncoupled; no causal basis | **Avoid** |
| **Critical speed (CS) and D′** | `D = D′ + CS·t` (least squares, ≥ 3 distances); `t_lim = D′/(v − CS)` | Best efforts from training records | Poole 2016, PMID 27031742 (VPD). Smyth & Muniz-Pumares 2020, PMID 32472926 (VPD, full text) | Maximal effort "was not possible to verify". The sample was marathon runners with ≥ 24 activities in 16 weeks. D′ CV 14.6 %. A sub-maximal "best" lowers CS | **Use only** from efforts the person marks as maximal (a race or test), labelled "from training, not tested" |
| **LTHR field test** | For example, average HR over the last 20 min of a 30-min solo effort | HR and pace | **SH** (coaching literature). Maffulli 2026, PMID 42872971 (VPD): CS and DFA-α1 correlate with lab criteria; HR deflection "not consistently identifiable" | Group correlation is not individual agreement | **Avoid** until sourced. Never offer Conconi / HR-deflection tests |
| **HRmax estimate** | Nes 2013: `211 − 0.64·age` (SE 10.8). Tanaka 2001: `208 − 0.7·age`. Gellish 2007: `207 − 0.7·age` | Age from the profile; observed maxima | **Nes is already in fm's register** (`sources-2026-10-11.md:377`, with ESC 2020: *"must be individualized after maximal exercise testing"*). Tanaka, PMID 11153730 (VPD); Gellish, PMID 17468581 (VPD) | Population equations, a prior only | **Use Nes** (the register's row), marked *estimated*. Tanaka only as a second named source, never blended. Show the observed maximum as a plain fact for the person to adopt |
| **HRrest** | Nightly or daily resting HR from the watch | Monitoring resting HR; `user_profile.resting_heart_rate` (FITP:1317) | Zhang 2020, PMID 32552580 (VPD): −0.01 bpm at rest, −0.40 bpm in sleep | The vendor's definition of "resting" is UV. A "rolling median" has no source | **Use** typed, or the watch's value shown as the watch's |
| **Nightly HRV** | Watch nightly value. FIT `hrv_status_summary.last_night_average` is "Last night RMSSD average over sleep", uint16, **scale 128**, ms (FITP:25614-25622) | GB import of HRV status | Casanova-Lizón 2026, PMID 42944412 (VPD; a different Garmin model with the same sensor family): wrist PRV reliable within a person, not ECG-equivalent | The vendor's averaging is UV | **Optional, later:** a trend against the person's own baseline, no proposal |
| **HRV-guided adjustment** | Kiviniemi rule: low intensity or rest if HRV < (10-day mean − SD) or falls 2 days running | — | Kiviniemi 2007, PMID 17849143 (VPD): 26 healthy, moderately fit males, 9 in the HRV group; **high-frequency R-R oscillations measured every morning**. Javaloyes 2019, PMID 29809080; meta-analyses 34639599, 34489178, 33143175 (VPD) | The rule was built on morning HF power, not a nightly RMSSD average. Pooled performance benefit small or not significant | **Avoid as a rule.** Documented only (oracle O14) |
| **Progression jumps** | — | Weekly distance | Buist 2008, PMID 17940147 (VPD): the 10 % rule had no effect (20.8 % vs 20.3 %). Nielsen 2014, PMID 25155475 (VPD): 874 healthy **novice** runners; no significant difference overall; for distance-related injuries, >30 % progression **over a 2-week period** gave HR 1.59 (0.96–2.66), **P = .07** | A non-significant trend in novices | **Optional caution only**, worded as a non-significant trend in novices, computed over 2 weeks on distance |
| **Intensity distribution** | Time in HR or pace zones | `time_in_zone` or records | Stöggl & Sperlich 2014, PMID 24550842 (VPD). Rosenblat 2025, PMID 39888556 (VPD) | **Contested** | Describe; let the person choose |
| **Taper** | 2 weeks; volume −41 to −60 %; intensity and frequency kept | Plan volume | Bosquet 2007, PMID 17762369 (VPD) | Trained athletes; group effect | **Use** in event plans, stating the population |
| **Recovery weeks** | e.g. 1 lighter week in 3-4 | — | UV | — | Convention only; no number ships |
| **Pre-participation screening** | ACSM 2015: current activity × signs/symptoms or known disease × intended intensity | Person's answers | Riebe 2015, PMID 26473759 (VPD, abstract only; full text blocked) | The branch actions and the symptom list are **SH** | **Research only** until the ACSM text is read |

---

## 2. Detail

### 2.1 Session load

**Banister TRIMP.**
- Formula and constants: SH. The Morton 1990 abstract has the model but not the constants.
- GC (VC) implements the **session-average** form: `secs/60 × x × 0.64 × e^(ksex·x)` (`TRIMPPoints.cpp:96-97`). It uses the sex constant at `:91-93` and moving time at `:84`. It returns 0 if HR is below rest, and NIL with no HR zones (`:63-66`). `TRIMP100` (`:157-158`) rescales so that 60 min at LTHR = 100. That scale has **no source except GC**, so fm cannot ship it.
- Per-sample integration and the average form differ on variable sessions (O5). If HR load is ever shipped, per-sample is primary, and the average form is kept for parity tests only.

**Zonal TRIMP in GC (VC).** GC's defaults are 0.9 / 1.1 / 1.2 / 2.0 / 5.0 on zones from 0 / 68 / 83 / 94 / 105 % LTHR (`HrZones.cpp:37-44`). These are not Edwards' 1–5. Edwards appears only in a comment (`TRIMPPoints.cpp:168`).

**Session RPE.** Foster 2001 (VPD, abstract) supports the method's validity across exercise types. The multiplication rule is SH until the full text is read. GC (VC `:359, 372`): `secs/60 × RPE`, and 0 if RPE is 0.

**Pace load.** GC computes GOVSS (VC `GOVSS.cpp`):
- Running power (`:41-56`) from aerodynamic, kinetic and slope terms. The slope term is the Minetti polynomial.
- 120-s rolling speed and slope; 30-s rolling power, 4th power, mean, 4th root (`:96-158`).
- RTP is power at CV on the flat (`:298-306`).
- The final value reduces to `secs·(LNP/RTP)²/36` (`:403-417`).

The structure is known here only from GPL code. Skiba's own text was not read (UV). fm must not ship a pace load until a primary text is read. Any 100-point scale must name its anchor. CS marks the heavy/severe boundary (Poole 2016, VPD), so "100 = 1 h at CS" is not the same anchor as an HR "100 = 1 h at threshold". Whether a one-hour effort at CS is sustainable is SH.

### 2.2 Long and short load

**GC PMC (VC).**
- Decay: `lte = e^(−1/LTS_days)`, `ste = e^(−1/STS_days)` (`PMCData.cpp:132-136`). Defaults 42 and 7 (`:59-68`).
- Recursion (`:521-539`): `LTS[d] = stress[d]·(1 − lte) + LTS[d−1]·lte`. Day 0 starts from 0.
- Seeds (`:525-530`): a season seed resets both series; 0 is a valid seed.
- Stress balance `SB[d+1] = LTS[d] − STS[d]`, shifted to the next day unless "SB today" is on (`:136, 555`).
- Ramp rate: change in LTS over the last `STS_days` (`:541-550`).
- Binning: by the local calendar date of the start (`:303`). Same-day loads sum (`:316-327`). NaN/inf skipped (`:313-315`). Zero-filled (`:425-426`). Range from first activity − 1 day to last + 365 days (`:228, 233-236`). Planned and actual series kept apart (`:316-360`).

**Other tools.** Some use `k = 1/N` instead of `1 − e^(−1/N)` (SH, forum posts, UV). Over 42 days of constant load the two differ by about 0.7 % (O2).

**Constants.** 42/7 are convention (SH). Vermeire 2022 (VPD) advises **against** general constants. If fm ever ships the chart, the person can set the constants, the screen says whose defaults they are, and the screen quotes the caution.

**Coverage.** A recursion needs a number for every day. A silent 0 for a day with no data drags long load down. Facts:
- With τ = 42, the last 42 days carry only `1 − e^(−1) ≈ 63 %` of long load's weight; 126 days (3τ) carry about 95 %.
- Activity FIT files say nothing about days without activity. Only monitoring files (via GB's ZIP) show that the watch was worn.

**Mixing anchors.** A load fixed against one threshold and a load fixed against a later threshold are not on the same scale. Vermeire 2021 (VPD) found that methods "evolve differently". The same caution applies across anchor changes (an inference, not a finding of that paper).

**Banister in GC (VC).** `Banister.cpp:475-482`: `g_t = g_{t−1}·e^(−1/τ1) + w_t`, `perf = p0 + k1·g − k2·h`, not normalised, and including the same day's load. Defaults τ1 = 50, τ2 = 11 (`:114-115, 213-215`). Only k1, k2, p0 are fitted, by Levenberg–Marquardt (`:520, 534`).

**Predictive validity (VPD).** Hellard 2006: fit R² 0.79 ± 0.13 but wide CIs; parameters "inappropriate" for monitoring elite swimmers. Busso & Chalencon 2023: prediction "not satisfactory for individual training planning". Borresen & Lambert 2009: accuracy "has proven poor".

### 2.3 ACWR

- Lolli 2019 (VPD, title only): coupling causes spurious correlation.
- Impellizzeri 2020 (VPD): inaccurate even when uncoupled; no causal basis.
- Williams 2017 EWMA variant (PMID 27650255): title only; SH.
- Gabbett 2016 (PMID 26758673, VPD) is historical; Impellizzeri rebuts it.
- Instead: show acute and chronic load as plain numbers, the week-over-week change in distance as a plain number, and no risk colours.

### 2.4 Thresholds

**Critical speed (Smyth & Muniz-Pumares 2020, VPD full text).**
- Fastest grade-adjusted times over 400, 800, 1000, 1500, 3000, 5000 m within the 16 weeks before the target. **At least 24 activities** were required. Sampled every 100 m.
- `D = D′ + CS·t` with ≥ 3 distances; linear, R = 0.9999.
- Pooled CS 3.74 ± 0.08 m/s, D′ 149 ± 24 m. D′ CV 14.56 %.
- Maximal effort "was not possible to verify".
- The 16 weeks is a **data window**, not a re-test interval. A re-test interval is convention (UV).

**GC (VC).** Fits CP/CV with an iterative envelope over the mean-maximal curve (`PDModel.cpp:366-430`), default windows 120–200 s and 720–1200 s (`:490-493`), and applies it to speed (`Charts/CPPlot.cpp:819-835`). That differs from Smyth's least squares.

**Bias.** Recreational training rarely holds maximal 400 or 800 m efforts. A sub-maximal "best" lowers the fitted CS. Every pace-based load then rises with the square of the ratio. fm therefore accepts only efforts the person marks as maximal.

**HRmax.** Nes 2013 is the register's row. Tanaka 2001 (VPD): `208 − 0.7·age`, 18,712 subjects. Gellish 2007 (VPD): `207 − 0.7·age`. Any "99.5th percentile" rule is UV, so fm shows "highest seen: N bpm on date" and lets the person adopt it.

**Threshold HR.** "HR at sustained efforts near CS" has no source (UV). Not shipped.

**Wrist HR.**
- Zhang 2020 (VPD): bias −0.51 bpm in treadmill walking/running, −4.55 in cycling, −7.26 in resistance training.
- Bent 2020 (VPD): absolute error about 30 % higher in activity than at rest.
- Lag for any given watch: UV.
- Consequences: HR measures underweight short hard repetitions (SH/UV). The cadence-lock heuristic (HR within ±3 bpm of cadence) is UV and may only flag, never drop.
- People on HR-limiting medication or with rhythm disorders: HR-based numbers may be meaningless for them (SH; a register row is needed).

### 2.5 Plan principles

- **Progression.** The 10 % rule did not prevent injury (Buist 2008). Nielsen 2014: a non-significant trend (P = .07) for distance-related injuries in novices progressing >30 % over 2 weeks. Never "10 % keeps you safe".
- **Intensity distribution.** Contested. Stöggl & Sperlich 2014 favoured polarised in well-trained athletes over 9 weeks. Rosenblat 2025 (13 studies, 348 athletes, IPD) found no overall difference. Pyramidal "mostly easy" is a default; polarised is a choice.
- **Taper.** Bosquet 2007: 2 weeks, volume −41–60 %, intensity and frequency kept; ES 0.59 (2-week duration), 0.72 (volume). Population: trained athletes.
- **HRV-guided training.** Kiviniemi's rule used morning HF power in 9 moderately fit men. The watch gives a nightly RMSSD average at scale 128. Pooled effects: Manresa-Rocamora 2021 SMD 0.20 (−0.09 to 0.48); Düking 2021 g = 0.079 (performance); Granero-Gallegos 2020 VO2max ES 0.402 (VPD). Not shipped as a rule.
- **Sleep.** No primary source read. Context only.

### 2.6 What fm can and cannot claim

**Can claim:** descriptive facts (time, distance, HR averages, time in a registered band); "studies found…" with citations; the taper structure with its population.

**Cannot claim:** injury-risk prediction or prevention (Impellizzeri 2020; Buist 2008); performance prediction (Busso 2023); diagnostic meaning of HR, HRV or resting HR; that a plan is "safe".

**Safety.** The ACSM 2015 model (Riebe 2015, abstract) uses current activity; signs or symptoms, or known cardiovascular, metabolic or renal disease; and intended intensity. The branch actions and the stop list (chest pain or pressure; fainting or near-fainting; unusual breathlessness; palpitations; dizziness during exercise) are **SH** and must be read in the ACSM text before anything ships.

### 2.7 The GPL boundary for GoldenCheetah

GC is GPL-2.0-or-later; fm is MIT.
1. Do not copy, transliterate or closely paraphrase GC code, structure, comments or constant tables.
2. Formulas cite their papers. GC file and line references appear only in test docs, as "behaviour compared against".
3. GC is a black-box oracle. A maintainer runs GC on synthetic activities and commits only the exported numbers.
4. Parity fixtures name the GC version and settings (LTS/STS days, SB-today flag, sex, rest HR, max HR).
5. Where a method's only read description is GC code (pace load), fm does not implement it from that code.

### 2.8 FIT facts the plan depends on (VC in `profile.py`)

- `workout_rpe` (session field 193): comment "multiplied 10x", scale 1 (FITP:7209-7215). Divide by 10.
- `hrv_status_summary.last_night_average`: "Last night RMSSD average over sleep", uint16, scale 128, ms (FITP:25614-25622).
- `workout_hr`: the only named value is `100: 'bpm_offset'` (FITP:28580-28582). That values below 100 mean % is **SH**.
- `file_id.serial_number`: `uint32z` (FITP:109-111). A 32-bit space: an unsalted hash that includes it can be reversed by search.
- **Licence.** The FIT Protocol License defines the Licensed Technology as the SDK "that includes documentation describing the FIT protocol and related source code files" (`fit-python-sdk` LICENSE, lines 4-7). §1 grants use for internal business purposes, "including to use the FIT protocol in any software created by Licensee". §2(c) forbids making available "any features or functionality of the Licensed Technology, to any third party". §2(d) forbids derivatives under a licence that lets others modify them. How these interact for a public MIT project is **UV (legal)**. This note does not settle it.

---

## 3. Test oracles

Numbers marked **derived** were computed by hand from the formulas above (a small script outside the repo). They are not GC output. Tolerance 1e-6 unless stated. Oracles for methods marked *research only* are kept so the numbers are ready if the method is ever sourced; they are not acceptance tests for anything shipped in the first proposal.

**O1. Single impulse, normalised EWMA.** Load 100 on day 0, from 0.

| Day | Long | Short |
|---|---|---|
| 0 | 2.352831 | 13.312210 |
| 6 (no further load) | 2.039617 | 5.649340 |

Freshness on day 1 = −10.959379 (shown the next day, as GC's default).

**O2. Constant load 100 per day, from 0.**

| After | Long | Short |
|---|---|---|
| 7 days | 15.351828 | 63.212056 |
| 42 days, `1 − e^(−1/N)` | 63.212056 | 99.752125 |
| 42 days, `1/N` | 63.654405 | 99.845747 |

**O3. Binning.** Two sessions on one `day` sum. A session starting 23:50 local counts on its start date. A day with no data is **unknown** in the coverage count even though the arithmetic enters 0 (see O16).

**O4. Banister TRIMP, average form.** 60 min, HR 150, rest 50, max 190: male **108.0954**, female **90.4179**. HR below rest gives 0.

**O5. Convexity.** 30 min at 120 + 30 min at 180, male: per-segment **131.0956** ≥ 108.0954.

**O6. A 100-at-threshold scale.** 60 min at LTHR 170 (rest 50, max 190) gives 100.0. Documentation only: the scale has no source outside GC.

**O7. Session RPE.** 45 min × RPE 6 = **270 AU**. RPE 0 gives 0. A raw FIT `workout_rpe` of 65 reads as **6.5**.

**O8. GOVSS reduction.** `secs × (LNP/RTP)²/36`: 3600 s at ratio 1 gives **100**; 1800 s at 1.05 gives **55.125**. GC's first sample uses `initial_speed = 0`, so a GC-exported fixture reads slightly above 100; set that tolerance from the fixture.

**O9. Critical speed, least squares (derived).** 400 m/80 s, 800 m/170 s, 5000 m/1200 s → CS **4.094861 m/s**, D′ **87.484 m**; t_lim at 4.5 m/s **215.94 s**. Fewer than 3 distances → "insufficient data". Efforts not marked maximal → refused.

**O10. HRmax at age 40.** Nes **185.4**, Tanaka **180**, Gellish **179**. Each is returned with its source name; no blend.

**O11. Banister impulse response (documentation only).** k1 = 1, k2 = 2, τ1 = 42, τ2 = 7: back to baseline at **5.822 days**, peak at **20.873 days**.

**O12. ACWR coupling (documentation only).** Weekly [100, 100, 100, 400]: coupled **2.2857**, uncoupled **4.0**.

**O13. Taper (numeric).** Base 50 km/week, 2-week taper, cut 41–60 %: total taper distance must lie in **[40, 59] km** (2 × 50 × 0.40 to 2 × 50 × 0.59). The share of time in each band must equal the base week's.

**O14. Kiviniemi rule (documentation only).** 10-day mean 60 ms, SD 5: today 54 → "low/rest"; 62 → 59 → 56 → "low/rest"; otherwise unchanged. A raw FIT value of 7680 reads as **60 ms** (scale 128).

**O15. GC parity fixtures** (a maintainer runs GC): steady 60-min run; 6 × 2 min intervals; ±8 % hill; a 90-day calendar with gaps and two-a-days. Settings recorded.

**O16. Coverage (D2).** 42 days, loads only on days 0, 7, 14…; no rest marks; no monitoring data. Covered days = activity days only. Coverage over 126 days (long) and 21 days (short) is reported. Below the threshold, long load is not shown as a number.

**O17. Two zones, one series.** The same vault read under `TZ=UTC` and `TZ=America/St_Johns` gives an identical daily series, because days come from the stored `day`, not the viewing zone.

**O18. DST.** A session on a 23-hour day and one on a 25-hour day each count once on their `day`.

**O19. Anchor change.** Loads fixed against CS₁ for days 0-20 and CS₂ for days 21-41: the series marks day 21; a recompute gives the same values as a series computed against CS₂ throughout.

**O20. Import key.** The key is an HMAC with a per-vault random salt; the same file under two vaults gives two different keys; the serial does not appear in any note.

---

## 4. Sources

**PubMed abstracts read (VPD).** 11708692, 2246166, 16608765, 29101104 (title only), 32502973, 17940147, 24550842, 17762369, 17849143, 29809080, 11153730, 26473759, 32472926, 25155475, 30614348, 39888556, 34639599, 34489178, 33143175, 26758673, 27650255 (title only), 19691366, 12840641, 36791017, 27031742, 23728131, 35320776, 33691278, 17468581, 32047863, 12750600, 32552580, 42944412, 42872971, 12183501. Re-checked in this pass: 35320776, 25155475, 17849143, 11708692, 2246166.

**Full text read.** PMC1974899 (Hellard 2006), PMC7664951 (Smyth 2020).

**Already in fm's register.** Nes 2013 HRmax and the ESC 2020 heart-rate bands (moderate 55–74 %, high 75–90 % of maximum), `sources-2026-10-11.md:377`.

**Not read (second-hand).** Banister 1991; Edwards 1993; Skiba (GOVSS); Clarke & Skiba 2013 (403); Williams 2017 (403); Riebe 2015 full text (402); ACSM Guidelines text; any primary text stating 42/7. intervals.icu: forum posts only, UV.

**Code read (VC).** GoldenCheetah `03e1666`: `src/Metrics/PMCData.cpp`, `TRIMPPoints.cpp`, `HrZones.cpp`, `GOVSS.cpp`, `Banister.cpp`, `PDModel.cpp`, `src/Charts/CPPlot.cpp`. Garmin `fit-python-sdk/garmin_fit_sdk/profile.py` and its LICENSE. Gadgetbridge `edbea98`: `export/AutoFitExporter.java`, `service/devices/garmin/fit/FitImporter.java`, `devices/garmin/GarminFitFileInstallHandler.java`, `service/devices/garmin/GarminSupport.java`.

## 5. Reading list before any load number ships

1. A primary text stating the Banister TRIMP constants (0.64, 1.92, 1.67).
2. Foster 2001 full text (the RPE × minutes rule).
3. A published text that states the 42/7 constants as a method.
4. Skiba's GOVSS text, or another published pace-load method, with its anchor.
5. Minetti 2002 body (the polynomial).
6. Edwards 1993, if zonal load is wanted.
7. The ACSM pre-participation text and its symptom list.
8. A source on HR-based measures under beta-blockers and rhythm disorders.
