# Water, drinks and the values a person follows — the plan (2026-10-30)

> **A proposal, not adopted.** Written after a user asked to track water, with the owner's
> condition: *"definitely we should track it but it should not add more complexity"*. Nothing here
> is built. Where it disagrees with the code, the code wins; a ruling goes in `decisions.md`.

## What prompted it

A user wants to track water in Food. The owner wants no new categories. Wine, beer and cocktails
came up in the same conversation, and then whether a person can choose which values Food shows.

## What the app already has (read in the code and the food list 2026.10.6)

- **Every food in the list carries water** (`water_mg`), measured by its tables; 1,275 carry
  alcohol (`alcohol_mg`); caffeine is a key too (`caffeine_ug`). Plain water is a food: *Drinking
  water* (BLS), *Water, municipal* and *mineral* (Ciqual), *Acqua potabile* (Swiss). Wine, beer,
  cider, spirits and about 50 cocktails are listed, each with its alcohol and energy.
- **Health already adds them up.** The day diary reads water, caffeine and alcohol from Food
  (`health_days`' `elsewhere`), and the *Water* card compares the day's water with EFSA's
  adequate intake (2.0 L women, 2.5 L men, food and drinks together), once the profile gives a sex.
- **Food does not show them.** The all-nutrients sheet lists only nutrients with an EFSA reference
  (`nutrients.rs::REFS`); water, alcohol and caffeine are not among them. Day shows what
  `food-day.view` names: energy, protein, carbohydrate and fat, then fibre, calcium, iron and
  vitamin C as details.
- **A vault's own view file replaces the shipped one whole** (`views::totals_named`). A copy made
  to change one line would also stop every later change to the shipped file from reaching it.
- **No glass, no can.** Household portions come only from USDA; the European tables give none.
  Recent already makes a repeat one tap: the row's ＋ logs it again with its amount.
- **Search puts the plain drink last.** Measured on the list with the app's own ranking (a name
  starting with the typed word first, then the shorter name):

  | Typed | First results today |
  |---|---|
  | water | Water ice · Watermelon · Watermelon raw · Watercress · Water cress · *Water, municipal* |
  | beer | Beer ham · Beer sauce · Beer, dark · Beer, lager |
  | coffee | Coffee, ground · Coffee cream · Coffee ice cream · *Coffee (infusion)* |
  | tea | Tea leaf · Tea, leaves · *Tea, no sugar added* |
  | wasser | Wassereis · Wassermelone · Wasserbiskuitboden |

  The cause: "watermelon" starts with "water", and a short name wins. A whole word is not told
  from the start of a longer one.

## What other apps do (looked up 2026-10-30)

| App | What a person chooses | Limit |
|---|---|---|
| MacroFactor | pins nutrients to the dashboard; hides the rest everywhere | 8 pins |
| Cronometer | a *Visible* switch per nutrient; highlighted targets | — |
| MyFitnessPal | the diary's nutrient columns | 5 plus calories |
| Lose It! | nutrient goals and favourites on the dashboard | Premium |
| Yazio | a separate water tracker, on or off | its users ask for it to count food's water |

The common shape is **pin from the app's own list, with a small cap**; nobody's good answer is a
second log. Yazio's separate tracker shows the cost: its users ask for it to be joined with the
diary. Sources: [MacroFactor](https://help.macrofactorapp.com/en/articles/129-pin-nutrients-to-the-dashboard),
[Cronometer](https://support.cronometer.com/hc/en-us/articles/360019864311),
[MyFitnessPal](https://support.myfitnesspal.com/hc/en-us/articles/360032621991-How-can-I-change-the-nutrients-that-are-tracked-in-my-diary),
[Lose It!](https://loseit.zendesk.com/hc/en-us/articles/47649705562004-How-to-Create-Nutrient-Goals),
[Yazio](https://help.yazio.com/hc/en-us/articles/360000671858-Why-is-my-Water-Tracker-gone),
[Yazio feedback](https://feedback-en.yazio.com/suggestions/600981/include-water-intake-from-foods-and-distinguish-it-from-bottled-water-for-a-more).

## The plan: four steps, each one release, each useful alone

### 1. Search finds the plain food first

The fix that helps everything, not only drinks.

- **Measure first.** A set of typed words, each with the food a person means, in the four
  languages the list is searched in: water, beer, wine, coffee, tea, milk, juice, egg, apple,
  bread, rice… (about 60). Record today's rank of the meant food; the change must raise it and
  move none down. The set and the numbers go in a dated research note.
- **Candidate rules**, in order: a whole word matched before a word that only starts with it
  (*Water, municipal* before *Watermelon*); then, among whole-word matches, the name the typed
  word heads (*Beer, dark* before *Beer ham*: in the tables' naming a comma follows the food, and
  a second noun makes another food). Kept only if the measurement holds.
- **What the person has logged ranks first.** A food from their own Recent that matches leads the
  results, so the second time is never a search through the list.
- Tested in `pack.rs` with invented foods, as the search tests are now.

### 2. Water, alcohol and caffeine in Food's own numbers

- **Three rows in the all-nutrients sheet**, under a heading of their own (*Drinks and water*):
  - **Water, from food and drinks**, beside EFSA's adequate intake for the profile's sex, the
    figure Health's *Water* card already uses, read from the same place. The words say it
    counts the water in food, as EFSA's figure does.
  - **Alcohol**, the amount alone. No target: none of the app's sources gives a safe amount,
    and the app does not advise.
  - **Caffeine**, beside EFSA's 400 mg a day for healthy adults, and its 200 mg in pregnancy,
    in the same 2015 opinion that Health's *Caffeine* card already quotes.
- No new log, no new screen, no number Health does not already show.

### 3. A drink is logged by the glass

- **Portion chips for drinks**, from a named source, never typed in by us
  (`decisions.md#data`, 2026-10-11, *nothing is invented*). Candidates to read: Italy's LARN
  standard portions (SINU), and the container a drink is sold in (a 330 ml can, a 500 ml bottle)
  where the label says it. The source and its figures are settled before anything ships.
- **Millilitres for a drink from the list.** The list is per 100 g. Water, beer and wine are
  within a few per cent of 1 g/ml, so a glass can be entered in ml and stored as grams, saying so;
  a drink much denser (a syrup, a liqueur) keeps grams.

### 4. Pin the values you follow

- **"Show on Day"** on each row of the all-nutrients sheet. A pinned value joins Day's details;
  unpinning one takes it off.
- **The defaults are today's**: fibre, calcium, iron, vitamin C. A person who pins nothing sees
  nothing new. **At most six**, so Day stays one calm screen.
- **Kept in the notebook, not on the device**, so a phone and a computer show the same values:
  as a property of the profile note, where the sex and the year of birth already are
  (`plan.rs`). Not as a copy of the view file, which would stop shipped changes reaching it.
- **Trends offers what is pinned first** in its picker.

## What this does not do

- No water tracker, glass counter, streak or reminder to drink. Each is one more thing to learn,
  and a counter of glasses would leave out the water in coffee and soup and disagree with Health.
- No value a person invents. Food shows what the food list measures; a mood or a step count
  belongs to Health and the body log, which already have places for them.
- No target for alcohol, and no advice about drinking.

## Before building: decisions to read

`decisions.md#data` — *nothing is invented* (2026-10-11, for any portion size), *every nutrient key
has one international definition*, *a logged entry is a note … its unit in the key* (ml against g).
`decisions.md#ui` — *after the owner used it on the phone … putting more than one thing on a
screen* (2026-10-12, for Day's details), *the layout may differ between a phone and a desktop; the
names never do*. `decisions.md#seams` — *search ranks by relevance* (2026-10-29), for step 1.
Step 4 changes what Day shows from a person's choice, so it earns its own entry.
