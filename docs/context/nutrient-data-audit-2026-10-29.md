# How the app computes nutrients, and what is wrong with it — an audit (2026-10-29)

> **Investigation, not a ruling.** Prompted by a user who logged eggs and saw no choline. Everything
> below was measured on the installed food pack (`foods@2026.10.3`, 18,338 foods) and its builder's
> source files, or read in the code. All six fixes (F1–F6) were made the same day: §5.

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

