# Making the cards usable by anyone: a proposal with options (2026-10-16)

> **Built 2026-10-16 (night)**, with the departures recorded in `decisions.md#ui` (*the card library
> as built*). Kept as the record of the options and their sources. Originally: **A proposal — nothing here was built or decided.** The owner: *"some are not exact diets but
> indications and some overlap and for a non-trained user we do not have an easy way to understand how
> to use it. Essentially the cards offer a library and template that can be tracked. Make sure not to
> invent anything but rather offer options with their attached reference. The idea is to make it easy
> for anyone to track important things and try to assess the general trends."*
>
> **The rule this follows:** every number, label, grouping and threshold below is taken from a source
> in [`sources-2026-10-11.md`](./sources-2026-10-11.md) (the register) or
> [`cards-evidence-2026-10-16.md`](./cards-evidence-2026-10-16.md) (the evidence review), and says
> which. Where no source gives a number, none is offered. Where the choice is a product decision, it is
> offered as options for the owner, not made.

---

## 1. What is wrong today, from a newcomer's chair

The library (`PlanScreen.svelte`) shelves the cards by **what they rest on** — *From a guideline*, *From
a position stand or an expert consensus*, *From a review of trials*, *Defined by a consensus*, *Set out
by its own body*, *Yours*. That is honest, and it answers an expert's question ("how strong is it?").
A newcomer asks three other questions first, and the library answers none of them:

1. **What is this card *for*?** Nine food cards mix four different kinds of thing (§2) under one word,
   "diet".
2. **Which one do I pick?** Several cards are the same goal at different doses, or alternatives for one
   goal (`cards-evidence` §4); nothing says so.
3. **Is it for me?** Every food reference value is for healthy people; five numbers are wrong for
   identifiable groups (`cards-evidence` §1, §6b); nothing says so.

---

## 2. Say what kind of thing each card is

The cards are four kinds of thing for food and two for fitness. The kind decides **how it is tracked**,
which is the thing a person needs to know before choosing:

| Kind (plain words) | Cards today | How it is tracked | Source of the distinction |
|---|---|---|---|
| **Targets** — numbers for nutrients, measured from what you log | WHO healthy diet, European reference values, Energy and protein for me | Automatically, from the log | The checks are nutrient sums/shares (`plans.json`) |
| **A way of eating** — amounts of food groups | Nordic 2023, Planetary health | **You tick each day** whether you kept to it — the food tables cannot tell which food is in which group | Register §6b: food-group patterns *"cannot be checked"* |
| **When to eat** — a timing rule | Two fasting days, Alternate-day, Time-restricted | Automatically, from the times and energy you log | Koppold 2024 definitions |
| **Weekly amounts** of activity | WHO activity, WHO 65+, Activity for losing weight, the three strength cards | Automatically, from logged sessions | WHO 2020; ACSM 2026 |
| **A programme** — a set way to train | Norwegian 4×4, Interval walking, Falls prevention, Stretching | Sessions of that kind, counted | CERG; Nemoto 2007; World Falls Guidelines; Warneke 2025 |
| **Yours** | My own (food, fitness) | As you set | — |

**Option K1 (recommended):** shelve by kind, and keep *what it rests on* as a short badge on each card
— *Guideline* · *Expert consensus* · *Review* · *Definition, not advice* · *One trial* — the wording
already used, shortened. **Option K2:** keep the shelves as they are and add the kind as a badge. K1
answers the newcomer's first question; K2 changes less.

---

## 3. Say what each card is for, and which to pick

Group the cards under the **question a person brings**, with one card marked *Start here*, and the
others described by their relation to it. Every relation below is from `cards-evidence` §4.

### Food

| A person's question | Start here | Instead, if… | Relation (source) |
|---|---|---|---|
| *Am I eating healthily?* | **WHO healthy diet** | **Nordic 2023** if you live in the Nordic/Baltic countries (graded food groups; *"avoid alcohol"*); **Planetary health** if the environment matters to you too | Same goal, different authors and purposes (`cards-evidence` §3) |
| *Am I getting enough of each vitamin and mineral?* | **European reference values** | — | Adequacy, not disease prevention (EFSA) |
| *How much should I eat?* | **Energy and protein for me** | — | Henry 2005 × EFSA activity levels |
| *I want to try fasting* | **Two fasting days** (or Alternate-day, or Time-restricted) | All three are ways of eating less; in trials, about as effective as eating a little less every day | Cochrane 2026; BMJ 2025 |

**Why WHO as *Start here*:** it is the only food card written for all adults in all countries
(`cards-evidence` §2a). **Option S2:** no default, the person chooses. The owner decides.

### Fitness

| A person's question | Start here | Instead, if… | Relation (source) |
|---|---|---|---|
| *Am I moving enough?* | **WHO weekly activity** | **WHO, 65 and over** from 65 (adds balance); **Activity for losing weight** when weight loss is the aim (contains WHO's dose) | WHO 2020; ACSM 2009 / Jayedi 2024 |
| *I want to get stronger* | **Strength, twice a week** | **Minimal strength** when short of time; **Muscle growth** for size — a ladder, pick one | ACSM 2026; Iversen 2021 |
| *I want to be fitter (heart and lungs)* | **Interval walking** (gentler, tested at ~63) | **Norwegian 4×4** (harder, better studied for fitness) — alternatives | Nemoto 2007; Milanović 2015 |
| *I'm worried about falling* | **Falls prevention** | — (one balance session counts for WHO 65+ too) | World Falls Guidelines 2022 |
| *I want to be more flexible* | **Stretching for range of motion** | — | Warneke 2025 (expert consensus) |

**Why WHO as *Start here*:** WHO 2020 covers adults including those with chronic conditions or a
disability; the others are doses for narrower goals. Same option S2 applies.

---

## 4. One card, or a base and add-ons?

Today a person follows **one** food card and **one** fitness card. The evidence shows several cards
that *combine* rather than compete (`cards-evidence` §4: a strength card *with* a WHO card; stretching
with anything; WHO 65+ *with* falls prevention).

- **Option C1 — keep one card each**, and say in the chooser which other card it already contains
  (*"WHO 65+ includes the WHO weekly activity"*). Least change.
- **Option C2 — one base and add-ons**: one card per question in §3 (e.g. *Moving enough* + *Stronger*
  + *Flexible*), each measured on its own line. Closer to how the sources are written (WHO itself is
  "aerobic **plus** strength"); a larger change to `plan.rs` (one followed note per question instead of
  per group).

A session should **count toward every card it satisfies** in either option (`cards-evidence` §2b, §4).

---

## 5. Tracking the important things, and seeing the trend

The owner's aim — *"make it easy for anyone to track important things and … assess the general
trends"* — needs no new card. It needs the cards' existing numbers shown **as a trend**, which is also
what their sources intend:

- **WHO's numbers are population averages, "an average to aim at over days"** (WHO/FAO TRS 916,
  confirmed by the adversarial check). So a **week's average**, and how it moves week to week, is the
  reading closest to the source — not a pass/fail per day.
- **Activity is already weekly** in every source (WHO, ACSM).

**Option T1 — a trend line per check**, for the card being followed: this week, and the last few weeks,
against the card's own bound. No new numbers.

**Option T2 — also a short "worth watching" set** that anyone sees, whichever card they follow, chosen
by the evidence review's ranking of what is **both strongest-evidenced and measurable from a log**:

| Watch | Number and source | Why this one |
|---|---|---|
| Saturated fat, share of energy | < 10 % — WHO, EFSA "as low as possible", Nordic < 10 % | Agreed by every body; RCT-backed (`cards-evidence` §5) |
| Sodium | < 2 g — WHO, EFSA (Nordic and US 2.3 g; named, not averaged) | RCT-backed (blood pressure) |
| Fibre | ≥ 25 g — WHO, EFSA (Nordic ≥ 35 g for men) | Cohort meta-analyses; every body |
| Protein | ≥ 0.83 g/kg — EFSA (1.0–1.2 g/kg from 65, PROT-AGE) | Adequacy; the most common shortfall in older age |
| Activity minutes | 150–300 a week — WHO | Strong recommendation; biggest gain is from none to some (Garcia 2023) |
| Strength days | ≥ 2 a week — WHO | Strong recommendation |

Each line names its source; where bodies differ, both are named, never averaged (the house rule,
register §7). **What is deliberately not on it:** potassium (unsafe to push in kidney disease without a
flag — `cards-evidence` §6b), total fat (the most contested limit), anything from a food-group pattern
(cannot be measured).

---

## 6. Filling the gaps — cards that could be added, each with its source

From `cards-evidence` §7, ordered by evidence; **status** says what stands between it and the library.

| Card | Kind | What it would track | Numbers, and from whom | Status |
|---|---|---|---|---|
| **Protein for older adults** | Targets | Protein per kg, from 65 | 1.0–1.2 g/kg (PROT-AGE 2013); ~1.2 g/kg "for food planning" (Nordic 2023); ESPEN 2019 ~1 g/kg — **named side by side** | **Ready**: numbers confirmed. Not with a kidney flag (KDIGO) |
| **Steps per day** | Weekly amounts | Daily steps | Risk flattens at 6,000–8,000 from 60, 8,000–10,000 below 60 (Paluch 2022); 7,000 *"a more realistic and achievable target for some"* (Ding 2025). *"10,000 is in no guideline read"* (register) | **Needs a step count** — the app has none; a phone pedometer or manual entry is a separate decision |
| **Vegan or vegetarian: what to watch** | Targets | B12, iron, calcium, iodine, against EFSA's own values | EFSA values already in the app; Vegan Society's three B12 options; NHS and BDA lines (quoted, not doses for a person) | **Ready** as a watch: no new numbers |
| **DASH** | A way of eating (+ some targets) | Servings ticked; sodium and fibre measured | NHLBI servings by calorie level (register — *its two documents disagree at 1,600 kcal*); NHLBI's nutrient goals **not yet read** | **Needs reading** (NHLBI nutrient table) |
| **Mediterranean** | A way of eating | MEDAS, 14 yes/no questions, weekly | Schröder et al. 2011 (register) | **Needs a licence check** — MEDAS's reuse terms are unsettled (register) |
| **Pregnancy** (food and activity) | Targets / weekly amounts | EFSA's pregnancy column (folate 600 µg, iodine 200 µg, B12 4.5 µg, protein add-ons); NHS folic acid and vitamin D lines; WHO 2020's pregnancy activity | EFSA 2017; NHS; WHO 2020 | **Ready**, behind an opt-in flag (§7) |
| *Sitting less* | Guidance only | — | WHO 2020 gives **no number** (conditional); Ekelund 2016's 60–75 min/day is one pooled analysis | **Not a card** — a line of guidance |
| *Low-carb, keto, MIND, "zone 2"* | — | — | No agreed definition, or a null trial (`cards-evidence` §7) | **Not offered** |

---

## 7. "Is it for me?" — opt-in flags, and what each does

From `cards-evidence` §6. Asked as an optional *"Anything we should know?"* list, never required. Each
flag does only two things: **show the matching caution, and swap in a named alternative number where a
body publishes one** — it never computes a medical target.

| Flag | What changes | Source |
|---|---|---|
| Pregnant / breastfeeding | EFSA's pregnancy or lactation values; NHS folic acid (to week 12) and vitamin D (October–March); fasting cards hidden; breastfeeding: the energy ceiling not used (+~500 kcal, EFSA) | EFSA 2017; NHS |
| Kidney disease | Potassium target **not shown**; protein shown as KDIGO's 0.8 g/kg with "set with your kidney team" | WHO 2025 salt-substitute guideline; KDIGO 2024 |
| Diabetes medication (insulin, sulfonylurea) | Fasting cards carry "with your diabetes team first" | IDF-DAR 2022 |
| Eating disorder, past or present | Fasting and weight-loss cards and the energy ceiling hidden | Varady 2022; Levinson 2017 |
| Age 65 or over (from the birth year already asked) | PROT-AGE beside EFSA; *WHO 65+* suggested | WHO 2020; PROT-AGE |

And a line on every food card: *"Values for healthy adults (EFSA). A log can show intake below a
reference; only a blood test shows a deficiency."*

---

## 8. Fix first: five numbers that are wrong as shipped

Independent of every option above (`cards-evidence` §1, confirmed by the adversarial check):

1. WHO protein: drop the 15 % maximum (WHO: *"generally sufficient"*).
2. EFSA iron: 11 mg after the menopause, 16 mg before (needs age, which the card already asks).
3. Potassium: hidden with a kidney flag; a caution line without one.
4. Folate: EFSA's pregnancy value with the pregnancy flag.
5. Protein over 65 and energy while breastfeeding: as §7.

Plus the small ones: the two missing DOIs (ACSM 2026, Warneke 2025); Nordic's measurable nutrient
numbers; EAT-Lancet's ranges and sodium; Jayedi 2024 on the weight-loss card; "fasting: in trials, about
as effective as eating a little less every day"; stretching "most days".

---

## 9. Decisions for the owner — all taken 2026-10-16 (`decisions.md#ui`)

| # | Question | Decided |
|---|---|---|
| 1 | Shelving | **By kind** (K1); what a card rests on becomes a badge |
| 2 | *Start here* | **Yes — WHO** healthy diet and WHO weekly activity |
| 3 | One card or base and add-ons | **Base and add-ons** (C2): one card per question, each measured on its own line |
| 4 | Trends | **T1 + T2**: weekly trends for the followed cards, and the "worth watching" set for everyone |
| 5 | New cards | **All four lines**: the ready three (protein 65+, vegan/vegetarian watch, pregnancy) first; then **steps** (needs a step-count source), **DASH** (needs NHLBI's nutrient table read), **Mediterranean** (needs MEDAS's reuse terms settled) |
| 6 | Opt-in flags | **As §7**, opt-in, never required |
| 7 | The five fixes (§8) | **Yes**, first |
