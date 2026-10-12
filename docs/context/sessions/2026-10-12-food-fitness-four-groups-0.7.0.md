# 2026-10-12 — food, fitness and the four groups, cut back by the owner's phone; v0.7.0

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
