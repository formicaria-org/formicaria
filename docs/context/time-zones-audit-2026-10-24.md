# Time zones — an audit of every clock in formicaria and Myrme (2026-10-24)

> **Research, not a ruling.** What reads, writes, compares or shows a time, in both repositories, and
> where a change of zone breaks it. Written before any fix, because fixes had been made one place at
> a time (the meeting pass, then the Collaboration dates) without this map. The rulings it builds on
> are `decisions.md` 2026-07-15 (*`Stamp`*), 2026-10-23 (three entries, *a time written in another
> zone…*, *every device shows the agenda on its own zone*, *the vault's clock is read from its first
> commit*).

**What prompted it.** A laptop moved to another zone. The app went on showing an email's arrival in
the old zone, and the owner asked for one app time, settable in Settings, with the
computer (or the internet) as its default — and for a search of the whole app, Myrme included, before
another fix.

## 1. The kinds of time the app holds

| Kind | Example | Where | Zone carried |
|---|---|---|---|
| **Instant** | `created`, `updated`, `shown`, `accepted`, `declined_at`, git commit times, Gmail's `internalDate` | frontmatter, git, caches | full: RFC 3339 with offset, or UTC seconds. **Sound.** |
| **Stamp** (naive) | `start`, `due` — `2026-11-02T16:30` | frontmatter | none in the file; means **the vault's clock** (`notes_time_zone`, else the first commit's offset) and is converted at the store seam (`FileStore::hydrate`/`put`) |
| **Text time** | an email heading `### Ana — 2026-10-07 09:12`; a conversation's `mail_last`; the assistant's words "Fri 23 Oct at 16:00" in a note or a verdict | note bodies, frontmatter, fm-serve state | **none, and never converted**: the zone of whichever setting was in force when it was written |
| **Day** | a log entry's day, a health diary day, the reminders window, `today` | arguments, stamps | the zone of whoever computed "today" |

## 2. Four answers to "which zone is the person in?"

The root cause. Each part of the app answers it on its own:

| Who asks | Answer it uses | Code |
|---|---|---|
| The store's seam (agenda, reminders, logs, every view, the phone) | the **operating system's** zone, asked through `jiff` (cached 5 min) | `fm-core/src/file.rs:685` `Zone::system()` |
| The calendar and mail readers | the **Settings zone** (Settings → Calendars, desktop only), else the system's | `fm-app/src/commands.rs:2776` `parse_zone` |
| Myrme (meeting pass) | the **Settings zone** through `/api/calendars`, else the system's; the phone has no setting | `myrme/crates/fm-agent-run/src/meetings.rs:275` `here()` |
| The UI (dates shown, "today", reminder moments, the Timeline's days, the calendar grid) | the **browser's** zone (`new Date()`, `getDate()`, `toLocale*`) — about 30 files; 3 pass a zone | `ui/src/lib/units.ts:105`, `reminders.svelte.ts:50`, … |

On one laptop with no setting these four agree. They disagree when:
- **the Settings zone is not the computer's** — readers write a meeting on one clock, the store reads
  it on another: every imported meeting and every proposal from Myrme moves by the difference;
- **the computer's zone just changed** — the store follows within five minutes, a browser that has not
  picked the change up does not: agenda times and reminder moments are off by the difference until it
  does;
- **a paired tablet is in another zone** — the host converts onto its own zone, the tablet's browser
  reads those times as its own (a known limit, `decisions.md` 2026-10-23);
- **text times were written before the change** — they stay in the old zone for ever.

### 2b. Two processes, two copies of the zone code

formicaria (`fm-serve`, `fm_core::zone::Zone`) and Myrme (`agent-serve` on the desktop, in-process on
the phone through the same `VaultAccess` facade; `fm_agent::zone::Here`) each wrap `jiff` and each ask
the system for its zone, each with its own five-minute cache. Myrme takes "the person's zone" from
`/api/calendars` (the Settings zone, else the computer's) — the **readers'** answer, not the store's —
and falls back to its own system zone when that call fails; the phone host's `zone()` answers `None`
(`fm-agent-run/src/fmserve.rs:101`), so there it is always its own. Its `today` is computed on its
side (`watch.rs:219`), the UTC date when no zone is known. Each side parses the setting itself.
Nothing checks that the two agree. **Proposed:** formicaria is the one authority and follows one
thing, the app clock; it serves that clock once (a contract addition, e.g. `/api/clock`: the resolved
zone, its current offset, today, now); Myrme takes its zone and today only from it, on both
platforms, and skips time-dependent work when it cannot be had; Myrme's zone code is then used only
for zones written inside an email.

## 3. Inventory — where a change of zone breaks something

Severity: **H** a wrong meeting, reminder or missed proposal; **M** a wrong word or day shown;
**L** an edge of a range.

| # | Where | Time | What goes wrong | Sev. |
|---|---|---|---|---|
| F1 | `fm-core/src/mail.rs:382-411`, `:596` | email headings, `mail_last` | written as text on the setting's zone at import, offset dropped; never converted. | H |
| F2 | readers vs store (`commands.rs:2776` vs `file.rs:685`) | imported meetings, invites | written on the Settings zone, read by the store as the system's. | H |
| F3 | Myrme `here()` vs store | Myrme's proposed starts | the same split as F2 for every meeting the assistant proposes. | H |
| F4 | Myrme `fm-agent/src/meetings.rs:855-859` | "began before the newest email" | compares a start on `here` with a heading on the import zone, as text. | H |
| F5 | Myrme `fm-agent-run/src/meetings.rs` verdict | "has taken place" | compares the note's start (system zone) with `mail_last` (import zone). | M |
| F6 | Myrme memory `mail-meetings.json` `proposed`, and the 0.9.0 ledger's `keep` starts | already-proposed meetings | stored as naive stamps on `here`; after a zone change the same meeting has another stamp, so it **can be proposed again**, and a kept email can leave the window early or late. Added by 0.9.0 too. | H |
| F7 | Myrme `fm-agent-run/src/watch.rs:220` | `today` | the UTC date when no zone is known. | L |
| F8 | UI `reminders.svelte.ts:46-50` | reminder moments | a stamp converted by the server (system zone) is turned into a moment in the browser's zone. | H |
| F9 | UI `units.ts:105` `today()` and its users (Food, Fitness, Health, the agenda's urgency bands, plans) | "today" | the browser's day; the server's stamps are on the system's. Disagree after a change until the browser follows. | M |
| F10 | `fm-app/src/health.rs:405` | a diary entry flagged *late* | `created.date()` is the UTC date of an instant, compared with a local day. Wrong near midnight. | L |
| F11 | UI `food/LogSheet.svelte:600`, `MailSection.svelte:45` | a date range, a suggested start | `toISOString().slice(0, 10)`: the UTC day as a local one. | L |
| F12 | fm-serve `mail.rs:356` | the mail reader's start day | `after:YYYY/MM/DD` is read by Gmail as midnight **Pacific time** (Gmail API, *Searching for messages*); seconds are exact. | L |
| F13 | UI, about 30 files | every date and time shown | the browser's zone; not the app's. | M |
| F14 | fm-serve `mail_heard` (Collaboration's verdicts) | "the meeting is still on Fri 23 Oct at 16:00" | the assistant's words, stored as text, frozen in the zone of the read. | M |
| F15 | Myrme chat | relative dates ("tomorrow") | no prompt states today's date or zone; the model cannot place "tomorrow". | M |
| F16 | the phone | the app zone | no setting: system only (consistent with itself, but cannot be chosen). | M |

**Not broken, checked:** instants everywhere (created/updated/proposal stamps, git, `internalDate`);
the calendar reader's `TZID` conversion and recurrence (`events.rs`, `recur.rs`); the vault's clock
(`founding_offset`); the Timeline's day buckets (browser-local, fixed 2026-08); git's
*Recently deleted* window; the update checker.

## 4. What a fix must hold to (proposal)

1. **One app clock, one source.** A per-device setting, *Automatic (this computer's zone)* by
   default, or a chosen zone; on the phone too. The server owns it (`fm_app::clock`), exposes it once,
   and everything asks it: the store's seam, the readers, Myrme (`/api`), and the UI (`appTime.ts`),
   which never asks the browser. This replaces the system-zone answer of 2026-10-23's *every device
   shows the agenda on its own zone* with *the app's zone*, whose default is the same thing.
2. **No new naive text time.** What the app writes from an instant keeps its offset: email headings
   `### Ana — 2026-10-07 09:12 +02:00`, `mail_last` as RFC 3339; shown converted to the app clock. The
   assistant's words are shown from the stamp at display, not stored as text.
3. **Myrme remembers instants** (UTC) for what it proposed and keeps in view, never naive stamps; it
   compares instants, and takes `here` and `today` from the app clock.
4. **The UI computes days and moments in the app zone** (`Intl.DateTimeFormat` with `timeZone`),
   never with `getDate()`/`new Date(y, m, d, h, mi)`: `today()`, reminders, the calendar grid,
   urgency, the Timeline.
5. **Correcting what is already written:** headings and `mail_last` from Gmail's own `internalDate`
   for each message id the note records (`<!-- gmail:<id> -->`); the local cache holds only some.
   Myrme's memory is dropped by a `PASS_VERSION` bump (nothing proposed is asked again: its starts are
   compared as instants from then on).
6. **An internet check, optional and only a warning:** the `Date` header of the Google requests the
   app already makes says whether this computer's clock is wrong.
7. **A two-clock test harness**: the suites run with the process zone (`TZ`), the app setting and the
   vault's clock all different, so a mix-up fails a test instead of a person's agenda.

Order: Myrme first (its note-format reading of headings is part of the contract), then formicaria.

## 5. Corrections from reading the code (two planning reviews, 2026-10-24)

Both reviews read the code at formicaria 0.18.0 / Myrme 0.9.0; each claim below was checked again.

- **§4.5 was wrong about Myrme's memory.** A `PASS_VERSION` bump keeps `proposed` (`Memory::load`):
  dropping it would ask again every meeting ever rejected. The bump must **convert** it to instants.
- **The phone never runs the meeting pass** (its `DispatchVault` implements neither `zone` nor
  `mail_conversations`), so on the phone Myrme's only clock need is chat's date. `/api/calendars` is a
  desktop route; the clock must be a `dispatch` command so both shells reach it.
- **`/api/calendars` answers `UTC` for a nameless system zone** (`calendar.rs` `effective`) while the
  store uses the real one. The new clock never uses UTC as a stand-in for "unknown".
- **Old readers and new headings.** Myrme 0.9.0 reads headings by position and ignores a suffix; an
  old formicaria's `place()` requires a 16-character time and passes a new heading over (misordering
  late mail, never damaging it). `mail_last` is parsed as a `Stamp` in `reconcile`, so its format and
  its parser change together.
- **The internet check** can only tell the clock's skew (the `Date` header), never the zone.
- **jiff's system zone** is cached five minutes and a `TZ` variable wins over the system's; Settings
  names the source.
- **"A conversation note only gains emails at its end"** (Myrme 2026-10-24) is false: `place()` puts a
  late older email in date order. The ledger's fingerprint resets on it; the sentence is corrected.

### New findings (add to §3)

| # | Where | What goes wrong | Sev. |
|---|---|---|---|
| **F17** | `commands.rs` `create_proposal_with`, `propose_new_note` | A proposal is built from `store.get` (already on the device's clock) and written with `to_file`, bypassing `put`'s conversion back; accepting merges those bytes. **Reproduced:** vault clock `+01:00`, process `TZ` +09:00, a title-only proposal on a note at 10:00 moved it by 8 hours. Live in 0.18.0 whenever the vault's clock is not the device's. | **H** |
| **F18** | `commands.rs` `restore_note` (Recently deleted) | Parses git's raw text (vault clock) then `put`s it, converting a second time. | **H** |
| F25 | `file.rs` `put` | Every write converts every timed stamp back; one nobody changed can be rewritten where vault→device→vault does not round-trip (a fold or gap), or when the clock moved between read and write. | M |
| F23 | `mail.rs` `place` | Orders emails by heading text: wrong across a zone change or a fall-back hour. | M |
| F24 | `proposal_props` | Myrme's proposed `start`/`due` are kept as naive text and re-applied on revise. | M |
| F21 | `ui/src/lib/appTime.ts` (2026-10-24) | An offset that is not a whole hour (`+05:30`) silently falls back to the browser's zone. | M |
| F19 | `fm-serve/src/calendar.rs` | The "already applied" key is the UTC day. | L |
| F20 | `ui/src/App.svelte` | Commit messages `backup: <toISOString().slice(0,16)>` read as local. | L |
| F22 | `zone.rs` `label` | A nameless zone is labelled by its offset in 1970. | L |
| — | `fm-serve/src/mail.rs` `since_ms` | A second UTC-midnight filter beside the `after:` query (F12). | L |

## 6. The refined plan

### 6.0 Hotfix first — formicaria 0.18.1 (F17, F18)

Independent of the rest and stopping live damage: `Store::encode(&Object)` (what `put` writes, on the
vault's clock) and `Store::decode(vault, &str)` (what `hydrate` reads); `MemoryStore`'s are identity.
`create_proposal_with`, `propose_new_note`, branch reads (`file_on_branch`), `restore_note` and
`delete_keeping` go through them. Test (two clocks): a title-only proposal on a timed note, accepted,
leaves the file's `start` byte-identical; a restored note's times do not move.

### 6.1 One app clock (formicaria 0.19.0)

- **Setting, per device, never synced:** `config_dir()/formicaria/clock.json` `{ "zone": "" }`
  (empty = *Automatic*, the zone the system reports; a name or an offset otherwise). Adopted once from
  the calendar panel's old setting when that was set. Settings → **Clock**, on the desktop and the
  phone: the choice, the time now with its offset, each vault's clock, the skew check.
- **One handle:** `fm_core::zone::Clock` (chosen, else system; **`None` when neither can be read** —
  never UTC), owned by `App`, cloned into every `FileStore`. The store's seam converts onto it, one
  snapshot per call; `put` writes back the file's raw value for a stamp nobody changed (F25).
- **Everyone reads it:** the calendar and mail readers (their own `zone` argument is ignored, F2),
  `/api/calendars.zone` (so Myrme 0.9.0 already agrees with the store, F3), the phone's
  `DispatchVault::zone`, `fm-cli`, reminders (the server answers each moment as an instant, `at_ms`,
  F8), the health diary's *late* (F10), the mail query in epoch seconds (F12), the calendar's `seen`
  key (F19).
- **The UI never asks the browser:** `clock` is read at boot and rides on `ping` (every 15 s, with a
  `generation` that moves when the resolved zone does); every date site goes through `appTime.ts` and a
  pure `civil.ts` (F9, F11, F13, F20, F21); a CI grep forbids `getDate()`/`getHours()`/`toLocale*`/
  `new Date(y, …)`/`toISOString().slice(0, 10)` elsewhere.
- **The contract call** (`dispatch("clock")`, `POST /api/clock`; `v: 1`, fields only added):
  `zone` (IANA name or `±HH:MM`, or `null` when unknown), `iana`, `offset`, `offset_seconds`, `now`
  (UTC), `today`, `source` (`automatic`/`chosen`), `next_change`, `generation`, `skew_seconds`,
  `instants: true`. With it: `apply_props` accepts RFC 3339 `start`/`due` (converted once onto the
  vault's clock), `mail_conversations` adds `start_at`/`taken_at`/`linked_at`/`last_at` instants
  beside the naive values, `mail_heard` accepts `latest_at` and `times` (verdict words filled at
  display, F14), `proposal_props` keep instants (F24).
- **Internet check:** the `Date` header of responses the app already receives (Google, GitHub); a
  warning past ±120 s; nothing adjusted, no new request.

### 6.2 Myrme follows formicaria (Myrme 0.10.0, contract 1.8)

> **Done** (Myrme 0.10.0, linked by formicaria 0.19.0). As built, two points differ from the plan
> below: `Here::today()` stays (it reads an instant, `Timestamp::now`, on a named zone; only the
> system-zone and local-time calls are refused, by Myrme's `pixi run clock`), and a day an email
> names is counted from the **sender's** day, which the offset headings of 0.20 make known. Myrme's
> `decisions.md`, 2026-10-24 (later), is the record.

- `VaultAccess::clock()` (defaulted: a minor). Zone and `today` come **only** from it;
  `Here::system()`/`Here::today()` are deleted and a CI grep keeps `try_system`/`Timestamp::now` out of
  `src`. No clock → no meeting pass (one log line), no date in chat.
- Every time becomes an instant at the start of a pass (headings in either form, `mail_last` in either
  form, formicaria's `*_at`); `plan` receives them rendered on the one clock, so its code and tests
  stay. The Past check, `answered`, `upcoming`, the window, `is_new`/`latest`, the verdict and
  `clash` compare instants (F4, F5).
- **Memory v9** stores UTC instants (`proposed_at`, the ledger's `read`/`keep`) and still writes the
  naive copies for a downgrade; the migration turns each old naive `proposed` into an instant on the
  zone 0.9 would have used, so nothing already proposed is asked again (F6).
- Proposes RFC 3339 times when `instants: true`, naive ones otherwise; writes its box text with the
  offset; tells the chat model today's date (no zone); adds `base` to a proposal so a stale one is
  refused (the migration rewrites every conversation note).
- Runs the meeting pass only where this device reads mail (two desktops would both propose).
- Evals: the 21 cases unchanged (legacy headings stay a supported input); new cases with offset
  headings, past/not past by instant, a mixed note, a relative day across offsets, a DST case.

### 6.3 An email's time keeps its offset (formicaria 0.20.0, links Myrme 0.10)

- **New headings:** `### Ana — 2026-10-07 09:12 +02:00`, the arrival (`internalDate`) shown at the
  **sender's offset** (their `Date:` header) when that parses and agrees with `internalDate` within a
  day, else the **vault's clock**: the same bytes on every device, and "tomorrow" counts from the
  sender's day. `mail_last` is RFC 3339. `place()` orders by instant (F23).
- **Display:** the read view shows a heading's time on the app clock (tooltip: as sent); the editor
  adds a widget without changing the text.
- **Existing headings** (`mail_offsets`, a pure command, one commit): for each `<!-- gmail:<id> -->`,
  the offset the heading's own text implies against Gmail's `internalDate` is **appended** (visible
  text unchanged), only when it is a whole quarter-hour within −12:00…+14:00; anything else is skipped
  and reported. Times from the old local cache first, then one `threads/<id>?format=minimal` per
  conversation (read-only scope, existing host). Idempotent, offline-tolerant (no token, no change),
  never guessing (an id Gmail cannot answer is listed, not filled), atomic per note; run at start
  **before** the assistant. Release note: update every device that reads Gmail.

### 6.4 Harness and guards

Three clocks, all different and awkward: process `TZ=America/St_Johns` (−03:30), app clock
`Asia/Kathmandu` (+05:45), vault clock `Pacific/Chatham` (+12:45/+13:45); folds and gaps on
`Australia/Lord_Howe` (a 30-minute change). `pixi run test-two-clocks` (the Rust suites) joins
`pixi run ci`, and `pixi run test-ui` itself runs on `America/St_Johns`. Myrme's suites and
its integration test (four clocks) the same way.

### 6.5 Open — the owner decides

1. **The hotfix (6.0) now, as 0.18.1?**
2. **A paired tablet:** times on the host's clock (planned, labelled), or the tablet's own (a
   per-request clock)?
3. **Food and health entries:** shown on the app clock like everything else (a 23:30 meal can land
   on another day after a zone change), or kept on the wall clock they were logged at?
4. **A vault clock read from the first commit** is a fixed offset: offer to state `notes_time_zone`
   as a named zone in Settings → Clock?
