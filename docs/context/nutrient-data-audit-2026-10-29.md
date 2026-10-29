# How the app computes nutrients, and what is wrong with it — an audit (2026-10-29)

> **Investigation, not a ruling.** Prompted by a user who logged eggs and saw no choline. Everything
> below was measured on the installed food pack (`foods@2026.10.3`, 18,338 foods) and its builder's
> source files, or read in the code. Six fixes (F1–F6) were made the same day (§5); **they did not
> solve it** — the same user still read no choline the next day. §6 measures why and what replaced
> F1.

## 1. How a number reaches the day screen

1. **The pack** (`ci/pack-foods.py` → `foods.sqlite`): five tables — Ciqual (FR), BLS (DE), the Swiss
   table, CREA (IT), and USDA FoodData Central as a fallback — each value converted to **a whole
   number per 100 g in the unit its key names** (`protein_mg`, `iron_ug`, `vitamin_b12_ug` …), 35
   nutrients. A nutrient a table does not give is **absent**, not zero (`amount` has no row).
2. **Finding a food** (`pack.rs::search`): European tables first, USDA last (`ORDER BY (src='usda'),
   bm25`).
3. **Logging** (`LogSheet.svelte::valuesAt`): every nutrient the food has, `round(per100 × grams /
   100)`, written as the entry note's properties. **Nothing is filtered**: if the food has it, the
   entry has it.
4. **Totals** (`fm-query::Tally`): whole numbers summed over the day's entries, with `n` = how many
   entries carried the nutrient. The nutrients sheet shows **"from n of m entries"** when some lack it.

**What is sound** (checked): macros agree across tables for the same food — a raw egg is 128–140
kcal, 12.4–13.2 g protein, 8.7–9.8 g fat in Ciqual, BLS and CREA; unit conversions (mg/µg) are right;
scaling by grams is right; a missing value is never silently written as 0 at logging time; totals say
how many entries they rest on.

## 2. What is wrong

### 2.1 Whole tables never measure some nutrients — the choline report

Share of each table's foods that carry a value:

| Nutrient | BLS (DE) | Ciqual (FR) | CREA (IT) | Swiss | USDA |
|---|---|---|---|---|---|
| **choline** | 0% | 0% | 0% | 0% | 94% |
| caffeine | 0% | 0% | 0% | 0% | 93% |
| vitamin B12 | 97% | 63% | **0%** | 99% | 95% |
| vitamin D | 98% | 64% | **2%** | 99% | 95% |
| vitamin A (RAE) | 100% | 56% | **0%** | 99% | 95% |
| vitamin E | 97% | 51% | **0%** | 98% | 95% |
| vitamin K1 | 90% | 46% | **0%** | **0%** | 95% |
| vitamin B6 | 98% | 74% | **0%** | **0%** | 98% |
| folate (DFE) | 100% | 26% | **0%** | **0%** | 93% |
| iodine | 87% | 63% | **0%** | 97% | **0%** |
| selenium | **0%** | 62% | 21% | 72% | 97% |
| copper | 99% | 72% | 34% | **0%** | 100% |
| manganese | 99% | 70% | 9% | **0%** | 6% |
| pantothenic acid | 98% | 68% | **0%** | 99% | 1% |

**Choline is in the USDA table only.** Because a European food is offered first, an egg logged from
any European table carries no choline — and from CREA it carries no B12, D, A, folate or choline
either. The day screen then says "from 0 of 3 entries", which is true and unhelpful: nothing says
the food's table never measured it, so it reads as the app failing to record it.

### 2.2 Whole micrograms are too coarse for B12 and vitamin D

Values are stored as whole µg per 100 g. **97% of B12 values and 98% of vitamin D values are ≤ 5
µg/100 g**, so rounding is a large share of each value: Ciqual's whole milk (~0.3–0.45 µg B12/100 g)
is stored as **0**; a value of 0.54 becomes 1, nearly double. Milk, yoghurt and eggs — the main B12
sources for many people — are exactly the foods hit. The same is true, less severely, for vitamin K1,
iodine and selenium in low-content foods. (69% of B12 values and 82% of vitamin D values are stored as
0; many are true zeros in plants, but the rounding cannot be told apart from them.)

### 2.3 Ciqual's "below the limit of detection" is stored as zero

`ciqual_number`: *"`traces` and `< x` are 0"*. "< x" means *less than x*, not nothing. In Ciqual's
own file:

| Nutrient | values | given as "< x" | "traces" |
|---|---|---|---|
| selenium | 2,101 | **46%** | 0% |
| iodine | 2,108 | **39%** | 0% |
| vitamin D | 2,174 | **28%** | 6% |
| vitamin C | 2,398 | 24% | 1% |
| vitamin K1 | 1,574 | 18% | 0% |
| riboflavin, manganese, copper, niacin, thiamin | ~2,450 each | 12–14% | ≤1% |

The raw egg's selenium in the pack is 0 for Ciqual; eggs are a recognised selenium source.

### 2.4 CREA's saturated fat is ~25% too high

The builder reads *"Acidi grassi Saturi (%)"* as a share of the food's fat (its comment quotes CREA:
"in % di lipidi"). But for a whole egg CREA prints saturated 45.55 %, monounsaturated 37.07 %,
polyunsaturated 18.1 % — **100.7 % together**, i.e. shares of the *fatty acids*, which are only part of
the fat (in an egg, about 83 %; phospholipids and cholesterol are the rest). Computed as fat × 45.55 %
the pack says **3.96 g/100 g**; Ciqual says 2.64, BLS 2.44, USDA ~3.1. The error is largest for eggs,
meat and dairy, small for oils. CREA's own statement needs reading in full before the fix
(a fatty-acid conversion factor by food group, as FAO/INFOODS publish).

### 2.5 CREA's folate is dropped

CREA prints *"Folati (μg)"* (51 µg for a whole egg); the pack's key is **folate DFE**, so it was not
mapped and every CREA food has no folate. For an unfortified food, dietary folate equivalents equal
food folate; only fortified foods (with folic acid) differ.

## 3. What would fix it — proposals, each a decision

| # | Fix | Where | Cost | Breaks? |
|---|---|---|---|---|
| F1 | **Fill a nutrient a table never measures from a matched food in another table**, labelled as borrowed (the entry says "choline from USDA's *Egg, whole, raw*"). Matching at pack build: same food group, name tokens, and macros within a tolerance; reviewed by hand for the commonest foods | `ci/pack-foods.py`, pack schema (a provenance column) | days, plus a review | a new pack version; old entries unchanged |
| F2 | **Finer units for B12 and vitamin D** (and possibly K1, iodine, selenium): keys in nanograms (`vitamin_b12_ng`), or store tenths. Old entries keep their `_ug` keys and are read as ×1000 | pack, `nutrients.rs`, UI units, a reader for old keys | a day or two | a data-format change: needs a decision entry |
| F3 | **"< x" as x/2** (a common convention for below-detection values), or keep 0 and mark the value as an upper bound | `ci/pack-foods.py` | hours | changes many Ciqual values |
| F4 | **CREA saturated fat × the fatty-acid fraction** of the food group | `ci/pack-foods.py` | hours, after reading CREA's notes | changes CREA's saturated fat |
| F5 | **CREA folate → folate DFE** for unfortified foods | `ci/pack-foods.py` | an hour | adds folate to CREA foods |
| F6 | **Say why a nutrient is missing**: "Choline: not measured in Ciqual, BLS, CREA or the Swiss table — 0 of 3 entries" and, on the food, which nutrients its table lacks; offer the USDA match | `NutrientsSheet.svelte`, `LogSheet.svelte` | a day | no |

**Suggested order:** F6 and F5 first (no format change, honest at once), then F3 and F4 (pack only),
then F2 (format change) and F1 (the real fix for choline, the largest). Each new pack is checked
against this document's numbers.

## 4. How this was measured

The installed pack read with SQLite, read-only; the builder's cached Ciqual and CREA sources parsed
for the "< x" and fatty-acid figures; the logging, totals and nutrients-sheet code read. No user data
was used: the report was about eggs, and eggs were looked up in the pack.

## 5. After the fixes (`foods@2026.10.4`, the same day)

Share of each table's foods with a value, and in brackets the share borrowed from USDA:

| Nutrient | BLS | Ciqual | CREA | Swiss |
|---|---|---|---|---|
| choline | 16% (16%) | 19% (19%) | 28% (28%) | 24% (24%) |
| vitamin B12 | 97% | 63% | 29% (29%) | 99% |
| vitamin D | 98% | 64% | 31% (29%) | 99% |
| vitamin A | 100% | 56% | 29% (29%) | 99% |
| vitamin K1 | 90% | 46% | 30% (30%) | 25% (25%) |
| vitamin B6 | 98% | 74% | 32% (32%) | 27% (27%) |
| folate (DFE) | 100% | 26% | 11% (its own, F5) | 24% (24%) |
| selenium | 17% (17%) | 62% | 21% | 72% |
| saturated fat | 100% | 95% | 25% (F4: no value where no factor fits) | 100% |

- **The reported case:** an egg from any of the five tables now has choline, 335 mg/100 g, from USDA's
  *Egg, whole, raw*; CREA's whole egg also borrows B12, D, A, E, K1 and B6 from it, and its saturated
  fat is 3.29 g (was 3.96).
- **Milk's B12** (Ciqual) is 200 ng/100 g — 0.2 µg — where it was 0.
- **Matching is deliberately conservative** (`ci/pack-foods.py`, *borrowing*): a shared head word
  (the food itself, before the first comma), a third of the name's words (half for a dish), no
  macronutrient far apart, fried never matched to not fried. About 2,500 European foods borrow; the
  rest of a table's gap stays a gap. Matches for twenty common foods are printed at every build and
  were read: eggs, milk, yoghurt, butter, cheese, beef, pork, lard, salmon, tuna, potato, rice, bread,
  fruit juices, banana, spinach, broccoli, lentils, almonds. Borderline ones remain (*White bean stew
  with pork ← Stew, pork*; *Pasta with egg ← Pasta, cooked*): each is shown as borrowed, from that food.
- **What no fix reaches:** CREA's iodine (USDA has none either); a food with no confident match. For
  those the nutrients sheet now says *why* — "not measured in CREA (Italy)" — instead of a bare count.
- **Old entries** keep what they were logged with; their B12 and D in micrograms are read as ×1000.

## 6. The first fix fell short (2026-10-29), and what replaced it (`foods@2026.10.5`)

The user logged eggs after 0.22.1 and read no choline; she also found no selenium in macadamia nuts.
Measured on `foods@2026.10.4` with the builder's own code (single foods only — no dish — and per
table):

| Single foods with a value | Ciqual | BLS | Swiss | CREA |
|---|---|---|---|---|
| choline | 23% | 18% | 29% | 27% |
| selenium | 64% | 20% | 73% | 22% |
| vitamin B12 | 66% | 96% | 98% | 28% |
| magnesium | 80% | 99% | 99% | 35% |
| iodine | 64% | 83% | 96% | 0% |

**Why**, three causes:
1. F1 filled a nutrient only for a table that measures it for under 5% of its foods. Ciqual measures
   selenium for 62% of its foods, so none of the other 38% — its macadamia nuts among them — was
   filled, nor CREA's magnesium and zinc.
2. The matcher compared the English names' words. CREA's "Chicken Egg, whole, boiled, a la coque or
   hard-boiled" and USDA's "Egg, whole, boiled or poached" shared 33% of them, against a bar of 34%:
   the boiled egg had no choline, nor CREA's fried egg, the Swiss hard-boiled one or BLS's fried one.
   Of 223 European egg foods, 66 had choline.
3. "＋ again" in the logging sheet copied the old entry's numbers, so a food the list now completed
   still logged without it; entries already written were never told.

**What replaced F1** (`ci/pack-foods.py`, *borrowing*; `decisions.md#data`, 2026-10-29): every value a
food lacks is borrowed, first from USDA's closest food, then from the closest food of another European
table (measured, or lent by USDA — the value keeps its origin); names are read alike first; the
macronutrients weigh more. Rules each found in a sample of matches read by eye keep apart a dish and
a single food, fried and not, preserved and fresh, dried, smoked or cured, a juice and its fruit, and
keep American fortification (enriched grain, vitamin D in milk) out.

| Single foods with a value, after | Ciqual | BLS | Swiss | CREA |
|---|---|---|---|---|
| choline | 33% | 22% | 35% | 39% |
| selenium | 77% | 36% | 84% | 66% |
| vitamin D | 80% | 97% | 99% | 65% |
| vitamin B12 | 81% | 97% | 99% | 64% |
| folate | 59% | 99% | 58% | 61% |
| magnesium | 88% | 99% | 99% | 78% |
| zinc | 87% | 99% | 99% | 79% |
| iodine | 75% | 88% | 98% | 53% |

- **The reported foods**: CREA's boiled egg has 268 mg of choline per 100 g, from USDA's *Egg, whole,
  boiled or poached*; its fried egg, the Swiss hard-boiled egg and BLS's fried egg likewise. Ciqual's
  macadamia nuts have selenium (12 µg per 100 g, from USDA's *Macadamia nuts*). USDA's own two records
  for macadamia nuts disagree (12 and 33 µg; its 2023 analysis of eight samples ranged 9–54):
  selenium follows the soil, and a borrowed value is one country's measurement.
- **Entries**: the nutrients sheet names each entry without a value and why, and offers to update
  entries logged before the list had values it has now; "＋ again" reads the list as it is.
- **Still not covered**: choline for most foods USDA has no close match for (it alone measures it);
  dishes their table did not analyse; CREA's iodine where no table has a close food. Each is said per
  entry, never shown as zero.

**How this was measured.** The builder's readers over its cached sources, the coverage counted per
table for foods without a dish word, before and after; samples of 50–60 matches drawn at random and
read by eye at each step (the rules above each answer a wrong match seen in one). No user data was
used.

## 7. Entries read from the list, one food one set of values, tested estimates (`foods@2026.10.6`)

Later the same night a user planned macadamia nuts for the next day and read no selenium; the owner:
*"this is a symptom of an underlying problem … the views are not using a simple sum from lookup
tables"*, and *"that same food in different languages means different things"*. Three changes
(`decisions.md#data`, 2026-10-29, later that night):

1. **Every view sums from the list.** An entry is its food and its amount; its numbers are worked
   out from the list when read. Planned days count their plan. Reproduced and checked on the real
   app: the planned macadamia nuts now give the next day 4 µg of selenium.
2. **One food, one set of values.** 706 groups of the same food across tables (1,571 foods), strictly
   formed (same kind, state, numbers, organ or part, nested names, macronutrients within a
   quarter, one food per table). Each nutrient comes from the first table in the order BLS, Swiss,
   Ciqual, CREA, USDA that measured it. Every macadamia nut of the list now carries CREA's measured
   4 µg of selenium per 100 g, except USDA's separately analysed *raw* nuts (33 µg); a value lent
   from a grouped food carries the group's.
3. **Estimates only where tested.** At every build each measured value of a nutrient and a kind of
   food is hidden in turn and estimated from the five most similar foods of that kind (never of the
   same name); an estimate is used only where the typical error is within 20% and three in five
   are within 30%. **30 pairs held**: choline in meat (19%); magnesium, potassium and phosphorus in
   meat, offal, fish or cheese (16–20%); and the absence of vitamin B12, vitamin D, cholesterol and
   selenium in plant foods. **Selenium, iodine, folate and vitamins A, E and K held for no real kind
   of food** (typical errors 30–270%), so they are not estimated. 1,969 values for 1,516 foods are
   estimated, shown as "≈ … estimated from 5 similar foods (meat)". A zero is given to plant foods
   only, never to mushrooms, algae, yeast, anything fortified or a plant drink.

**Result**, European foods with a value:

| | Single foods (8,882) | Dishes (3,647) |
|---|---|---|
| choline | 39% | 6% |
| selenium | 57% | 25% |
| vitamin D, B12 | 92% | 88–89% |
| iodine | 82% | 84% |
| folate | 81% | 73% |
| vitamin K1 | 78% | 78% |
| magnesium, zinc | 94% | 90–91% |
| **every vitamin and mineral** | **26%** | **5%** |

**What remains a gap, and why.** Only USDA measures choline, and a value needs a close USDA food or a
tested estimate; selenium varies with the soil (macadamia nuts measured 4, 12 and 33 µg) and no
estimate from similar foods held; a dish's values depend on its recipe, which no table publishes.
These are said per entry, never counted as zero.

