# 2026-10-12 — food, fitness and the four groups, cut back by a user's phone; v0.7.0

> A snapshot, never updated. Current state is `features.md` and `known-issues.md`; the rulings are
> in `decisions.md` under the dates named here.

## What was built (2026-10-11 and 2026-10-12)

- **Logged entries, totals and the tracker screens** (`fm-app/src/log.rs`, `fm-query`'s tally,
  the `totals` renderer, the food and training sheets), shipped as views that come with the app.
- **A food list carried in the app**: Ciqual 2025 and BLS 4.0, USDA only for what they lack
  (`ci/pack-foods.py`, `fm-core/src/pack.rs`). Every nutrient key tied to one FAO/INFOODS
  identifier, after the first build was found to have matched three columns by name and got them
  wrong. A source register (`sources-2026-10-11.md`) and the rule *nothing is invented*.
- **Recipes**: first as hidden notes, then with the recipe's own note visible and everything
  about it changeable.
- **Four groups over notes** — Library, Tasks, Food, Fitness — planned by a draft, a critic, an
  advocate and a third pass that settled them (`groups-plan-2026-10-12.md`), then built in five
  phases in one sitting. Templates that carry structure and are inert.

## What the owner took out, the same day

He installed two builds on his phone and each time said less was wanted:

| He said | What went |
|---|---|
| "a weird stats above the board… board and agenda were fine" | the ring over the board, the fortnight strips |
| "Remove: everything (what does it even mean?)" | the *Everything* view and the view keys made for it |
| "remove weight (it is just one of the trends eventually)" | the *Weight* view; Trends got a menu to pick what to draw |
| "I tried the template but looks like empty notes?" | starters got a body; the two program starters went |
| "The bottom of the phone screen should be as the bottom before, then the plus means different things" | the group's own bar, its list of groups, its add sheet |
| "the names of the options… should be the same between OSs" | the phone's separate list of views; one list now, one registry of names |

## What that cost, and what it teaches

- **Phase 3 was built, measured, documented, ruled on and removed inside a few hours.** So were
  half of phase 0 and most of phase 5. The plan's "phone quality" list was the weakest part of it
  and nobody — not the critic either — asked whether a ring over a board meant anything.
- **Everything visual was built unseen.** A screenshot was asked for once and declined, and the
  work went on regardless. The ruling since: nothing visual is added unseen, and the owner's own
  test for a screen is *it shows what was picked, and at most scrolls*.
- **A three-agent review settles what can be checked in code** (it was right about the recipe's
  list property, the template copy rule, the byte budget) **and nothing about taste.**
- **The owner's rules arrived as corrections, one at a time.** They are in `decisions.md` now;
  the next plan for a screen should start from them and from his phone, not from a list.

## Releasing

v0.7.0 is the first release made docs-first (the owner's standing instruction of 2026-10-11):
this note, the manual and the context notes were pushed before the version commit.

## The same day, later: something to follow (v0.7.1)

The owner, away from the keyboard: a library of diets and regimes, a custom one fine-tuned, and
being monitored against whichever is picked; release 0.7.1 without the phone check.

- **Built**: `plan.rs` and `shipped/plans.json` (17 options, 11 named activities), four read
  commands, the *Diet* and *Regime* tabs (`ui/src/lib/plan/`), activities by minutes and effort
  and a body region on a set (`WorkoutSheet`, `SessionScreen`). Ruling: `decisions.md#data`.
- **Seen**: in headless Chromium against the built program, 1280 and 360 wide — follow *Energy
  and protein for me* (2,798 kcal from a profile), the Day screen taking its targets, a 30-minute
  walk counted by *WHO weekly activity*, stop following. **Not seen on a phone.**
- **Found on the way**: `kind` on an entry is the note's own kind (hence `counts_as`); two views
  cannot share a name (hence *Diet* and *Regime*); the register's WHO fat range was a different
  document from the fact sheet, which was re-read and used.
- **Left**: routines on weekdays, weight over time — `outstanding.md` §2.23, item 10.

## Later still: what 0.7.1 left open (v0.7.2)

The owner: *"finish this without asking me, add anything you deem necessary for a full, seamless
user experience"*, then *"I want the release and try it myself on the phone after it"*.

- **Weight** is on the food Trends menu, written down from there, and is the weight every
  estimate uses. **Routines** are kept from a session, meant for weekdays, and planned from the
  session screen. Ruling: `decisions.md#data`, the last 2026-10-12 entry.
- **Seen** in headless Chromium against the built program at 360 wide: a weight written down and
  drawn; a session kept as *Legs* for Mon and Thu, today emptied, *Plan Legs* putting it back.
  **Not seen on a phone.**
- **Found on the way**: reading a second log in `log_days` pulled in other groups' program notes
  as targets — caught by the new test before it shipped.

## Last of the day: food made easy to use (v0.7.3)

The owner brainstormed usability — a packet with a barcode, the time of a meal, whether the day
follows the diet, micronutrients, simple feedback, a kitchen, recipes from known sources — then a
menu for several days and amounts as ranges; asked for it to be grounded in sources, then:
*"execute the plan"* and *"release 0.7.3"*.

- **Research first**: three agents (about 370k tokens) read primary pages; the register's §6c
  holds what they read, with what was only excerpted marked for re-reading. It changed the plan:
  Open Food Facts has 11,883 products for Singapore, so a typed label became half the feature;
  every camera reader but one needs Google's services; no carried recipe collection has grams.
- **Built**: a time on every entry and the night's fast measured from it; the diet on the Day
  screen; `log_nutrients` and its sheet; eating rhythm on Trends; an amount as a range; the
  *Menu* view with a shopping-list note; packaged foods from their labels. Three decision
  entries of 2026-10-12 under `#data` and `#ui`.
- **Not built, and why**: the barcode look-up (the first thing in food to reach the network — the
  owner was asked and said "execute the plan", which is not the same as a yes to that; and it
  reverses a ruling, so the reversal is written first); the camera; the kitchen; recipes from
  outside; proposing meals. All in `outstanding.md` §2.24.
- **Seen** in headless Chromium against the built program at 360 wide: each new screen. **Not on
  a phone**; released on the owner's word as before.
- **Found on the way**: a timed `start` broke every place that compared it to a day; a title
  crammed into a third of a phone row on the first Menu; a test that passed only because a
  reload used to be faster.

## And v0.7.4: the look-up, the kitchen, recipes from outside

The owner: *"continue with the barcode lookup, the kitchen and outside recipes"*, then *"continue
with what is missing"*, *"tackle all the remaining open points"*, *"release 0.7.4"*.

- **Built**: one read of one product at Open Food Facts when asked (the 2026-10-11 ruling against
  barcodes reversed, dated, first); a barcode read from a photograph with `zxing-wasm`, so no
  camera permission; a kitchen of names on the Menu; the NHLBI's 54 recipes carried word for
  word with their notes (one agent, about 97k tokens, fetched and checked them); dishes for what
  a day is short of; each Menu day against the diet as planned. Six decision entries.
- **Checked**: the look-up live from the desktop build; the reader on a drawn EAN-13 in a
  browser; the phone shell compiled for Android by hand. **Not on a phone**, and never a real
  photograph of a pack.
- **What went wrong, and is written down**: a new advisory turned the gate red (waived, with a
  migration queued); three test locks over one environment failed one run in six (fixed); a
  test-runner teardown error seen once (open); and twice an edit step failed while its gate ran
  on — once into a commit whose message claimed docs it did not have.

## And v0.7.5: when to do what, and how to do it

The owner: a clear timetable for the week; how each exercise is done, with cards for tool-free
options and for a gym; research for people who want less weight, more definition and long-term
health; videos from well-known sources. Then, twice, a correction that changed the work.

- **Built**: *Week* (a timetable, a session for any day, the CDC's example weeks); *Exercises*
  (117 cards word for word — the NHS website's and the U.S. Army's manual — with 157 linked
  films); three options to follow by name (Norwegian 4×4, interval walking, activity for losing
  weight); *How* from a session's exercise to its card. Seven agents over three batches read
  sources, built the exercise file and found the films (about 1.1 million tokens across the
  three batches, each batch under the cap).
- **"Why did you not find the Norwegian protocol?"** It was in the register as one unnamed trial.
  Every brief had been framed by issuing body. Searches for what people follow now go both
  ways, and the lesson is in memory.
- **"I would avoid the army as the primary source."** The first film search fell back to
  trainers only where no body had a film, so gym cards got Army clips. A second search found
  trainers whose qualification could be read on their own site; they now come before the Army.
  The *text* of the gym cards is still the Army's: nothing else may be carried.
- **Slips**: an agent put the owner's email address in a request header twice (both requests
  failed; later briefs forbid it). A gate was read as the result of an edit that had not applied,
  again, and caught before the commit. A test run with a name filter hid the one test that
  mattered. A weekly count had always summed the whole week, whatever the docs said.
- **Not seen on a phone.** No film was watched.

## And v0.7.6: a minimal set of views that make sense

The owner, after using 0.7.5: nothing to see after following a regime; *"what is volume in
fitness? and session? why are they there?"*; *"what are exercises there for if in the session I
have to invent and fill all again"*; and, leaving: *"a cohesive minimal set of views that make
sense and help a person build their routine based on established and clear research and practice,
same for food."*

- **Built**: Fitness is three views — Week, Exercises, Regime; a day opens in the week's place;
  the add sheet is a list to pick from, arriving with sourced numbers to change; every regime
  has a week to start from and every diet a day, each said to be a body's own or the app's
  arrangement; "plan the days left as last week went". Two agents found the examples (about
  340k tokens).
- **The lesson**: 0.7.5 added views beside the ones that were there. Session and Volume had been
  built first and were never questioned; the Week was put between them. The owner's test — *what
  is this for?* — removed two of five. A view that exists because it was built first is not a
  reason.
- **What bent**: "the app suggests no number of its own" (2026-10-11). The add sheet and the
  examples now do, on the owner's word, each number naming its source and each example saying it
  is only an indication. Recorded as a contradiction, first.
- **Slips**: a `git checkout` of one file to undo a failed insertion also undid an uncommitted
  change in it (restored, caught by the next step). Not seen on a phone.

## And v0.7.7: reminders

The owner: *"Can we add notifications to the app on all the OSs? Like a reminder for an event or
a deadline?"*, then *"go ahead and build it and then release, I am afk"*.

- **Built**: one read-only command, `reminders`, works out the moments from the notes' own
  `start` and `due`; nothing is stored. On Android the page hands the list to `__fmRemind` and
  alarms show them with the app closed and after a restart; everywhere the open page shows a
  line and, where allowed, the system's notification. Off until switched on in Settings.
- **Not built, said so**: the desktop does not remind when closed (the owner's decision);
  iOS has the in-app line only.
- **Checked**: the core by test; the page by test and in a headless browser; the Kotlin only
  by compiling. **Not seen on a phone.**

### After 0.7.7: the computer stays for a reminder

The owner: *"We can keep it running but we need to avoid to have also the AI models in the
background, that is a heavier task."*

- **Built**: the page hands the program the list it hands Android; when the last page leaves
  and a reminder waits, the program rests instead of stopping, refuses the assistant (which
  stops its model by the path it already had), says each reminder through the system's own
  notification, and stops after the last. No second switch.
- **A race found by running it**: the watchdog stopped the program at the reminder's moment,
  two seconds before the thread that was to say it. Unit tests passed; only the end-to-end run
  showed nothing had been said. Whoever says the last one now stops the program.
- **The docs, read whole** on the owner's word that the README did not mention food or
  fitness. It did, in one 190-word table cell eight rows down, which is the same as not. The
  README now opens with it; the manual lost a sentence saying there was no barcode reading,
  written two releases before there was. What is left is `outstanding.md` §2.28. Two agents,
  about 215k tokens.

### After 0.7.8: the barcode nobody could find

The owner, on the phone: *"I do not see scan the bar code option in the latest release."* It was
there since 0.7.4 — as *Photograph the barcode*, inside a form reached from a line of small
print at the foot of the Search tab. Built, tested, documented, and two screens from where a
person holding a packet looks. Now **Scan a barcode** and **Type in a label** sit at the top of
the add sheet. The lesson is 0.7.5's again: a feature is where it is found, not where it was
built.

### After 0.7.9: food for more than one country (2026-10-13)

Asked: would labels and barcodes work across countries; are
there open datasets, or governments'; *"are you sure there is nothing else for Italy?"*. Six
research agents over the day (about 880k tokens) wrote `sources-2026-10-11.md` §6f to §6h. Then,
on *"go ahead and start with that order"*:

- **A label is typed as it is printed**: per 100 g, per 100 ml or per serving; kcal or kJ; salt
  or sodium. A food labelled by volume is measured in millilitres (`amount_ml`), and no density
  is assumed anywhere.
- **The Swiss table is carried** (1,216 foods, Italian dishes among them): 17,506 foods.
- **Italy's own table is carried** (CREA, 832 foods, read off its pages into a snapshot):
  18,338 foods. Taleggio is found at last.
- **Each table's own-language names are searched**: French, German, Italian. Nothing translated.
- **No letters.** Two were drafted, to CREA and to Singapore's Health Promotion Board; the owner:
  *"we do not need the letters"*, and: *"but they do not require letters to be used."* He was
  right: the terms ask for none. CREA's table is being carried, read off its pages. Singapore's
  is not, because its site's `robots.txt` forbids every robot — found only when I went to
  collect it, a file I should have read the day I read the terms.
- **Not done**: the UK's table (its carbohydrate is in monosaccharide equivalents), the label
  read from a photograph, anything from Singapore's database.
- **Slips**: the gate was started twice before the size of the always-read docs had been read
  off the screen, and a third time before the third-party notice was regenerated: three runs
  for what one look each would have saved. One Swiss food stopped the build, rightly: its
  English row has no reference unit.

