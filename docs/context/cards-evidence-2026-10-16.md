# The food and fitness cards: what each says, which overlap, what the science says, and who they are not for (2026-10-16)

> **Research only — nothing here is built.** Asked by the owner: *"we have a nice research under the
> cards but we need to guide better the user. What is each card saying, do we have alternative cards
> essentially tackling the same thing? What does nutrition science say? … indications for people with
> some deficiencies. Only a very thorough research based on science."*
>
> Builds on [`sources-2026-10-11.md`](./sources-2026-10-11.md) (the source register, which holds the
> primary sources each card was built from) and does not repeat it. Three research reviews (food,
> fitness, deficiencies and conditions) and one adversarial check of their consequential claims;
> the check's verdicts are in [§9](#9-what-the-adversarial-check-found). The cards themselves are
> `crates/fm-app/src/shipped/plans.json` (`plans`).
>
> **How sure:** *read* = the source page or PDF was opened; *summary* = an abstract or reputable
> summary of it; **UNVERIFIED** = not confirmed this session. No citation was invented.

---

## 1. The bottom line

1. **No two cards contradict each other; several are the same goal at different doses.** Food: WHO,
   EFSA and Nordic set nutrient targets for healthy adults, for different purposes (§3); Nordic food
   groups and EAT-Lancet are both plant-rich food patterns; the three fasting cards are three rhythms
   of one idea. Fitness: WHO 65+ is WHO adult plus balance; weight loss is WHO adult at a higher dose;
   the three strength cards are a ladder (minimal → health → growth). A person picks **one per
   goal**, and the app could say so (§4).
2. **Five numbers on the cards are wrong as shipped, for everyone or for a group** (§2, §6):
   - WHO card: **protein 15 % of energy is used as a ceiling** — WHO says 10–15 % is *"generally
     sufficient"*, a floor. A heart-healthy day (the NHLBI example at 23 %) is shown as a failure.
   - EFSA card: **iron 16 mg is applied to all women**; EFSA's 16 mg is for women *before the
     menopause*, 11 mg after.
   - WHO and EFSA: **potassium ≥ 3,500 mg is unsafe for some people with chronic kidney disease**;
     WHO's own 2025 salt-substitute guideline excludes people with kidney impairment (and children
     and pregnant women), and grades its potassium advice *conditional*.
   - EFSA: **folate 330 µg is wrong in pregnancy** (EFSA's own value is 600 µg; and everyone who could
     become pregnant is told to take 400 µg folic acid).
   - Energy and protein: **0.83 g/kg protein is too low over 65** (PROT-AGE 1.0–1.2 g/kg; Nordic 2023
     ~1.2 g/kg for food planning; ESPEN 2019 ~1 g/kg, adjusted individually); and the energy ceiling is wrong while breastfeeding (EFSA adds ~500 kcal/day).
3. **Every reference value is for healthy people, and no card says so.** EFSA: DRVs are *"for healthy
   individuals and populations"*. A one-line scope note on each food card is the minimum.
4. **The fasting cards are honest but incomplete.** Trials show them **about as effective as eating a
   little less every day** (Cochrane 2026; BMJ 2025); nothing beyond weight loss and nothing beyond
   1–2 years is shown; alternate-day fasting's small edge appears only in trials **under 24 weeks**.
   They lack a *"not intended for"* line (eating-disorder history, BMI < 18.5, children; diabetes on
   insulin or sulfonylureas needs the diabetes team; pregnancy, breastfeeding and over-70s not
   studied).
5. **A food log can show intake below a reference; only a blood test shows a deficiency.** The app
   may compare a log with a named published value and quote population advice ("vegans need a
   reliable B12 source"); it must not infer a deficiency or give a person a supplement dose — that
   is the step toward medical-device software (§6c).
6. **The strongest gaps** — well supported and searched for, with no card: **steps per day** (7,000, offered
   as *"a more realistic and achievable target for some"*, Lancet Public Health 2025), **DASH** (RCT-proven for blood pressure; its menus already ship),
   **Mediterranean** (the most-searched name; outcome RCTs), **protein for older adults**, a
   **vegan/vegetarian nutrient watch** (§7).
7. **Two cards cite nothing clickable** (ACSM 2026 strength, the 2025 stretching consensus): both
   now have DOIs (§2b). One line in the source register is unsupported (§8).

---

## 2. Each card: what it asks, how strong its basis is, what is off

### 2a. Food (9 cards)

| Card | What it asks of a person | Who, for whom, for what | Evidence | What is off / missing |
|---|---|---|---|---|
| **WHO healthy diet** | Fat ≤ 30 % and saturated fat < 10 % of energy; carbohydrate 45–75 %; protein 10–15 %; fibre ≥ 25 g; sodium < 2 g; potassium ≥ 3,510 mg | WHO, all countries; preventing non-communicable disease and *"unhealthy weight gain"*; numbers are **population-average goals** (WHO/FAO TRS 916, 2003; fat/carbohydrate guideline July 2023) | GRADE guideline. Strongest: saturated fat, salt, fibre/whole grains. **Weakest and contested: total fat ≤ 30 %** (its outcome is weight gain only; criticised by Willett, *summary*) | Protein max (point 2 above); present as "an average over days", not "a limit today"; potassium unsafe in CKD |
| **European reference values (EFSA)** | Protein ≥ 0.83 g/kg; fat 20–35 %, carbohydrate 45–60 %; fibre 25 g; potassium 3.5 g; sodium 2 g; floors for calcium, iron, magnesium, C, D, B12, folate; upper limits for calcium, vitamin D | EFSA, **healthy** people in the EU; purpose **adequacy** (enough of each nutrient), not disease prevention | Adequacy science (nitrogen balance etc.). **Adequate intakes** (fibre, potassium, D, B12, magnesium) are observed intakes where evidence is thin — falling below one is *not* a deficiency | Iron 16 mg after the menopause; folate in pregnancy; vitamin D 15 µg assumes little sun, so most users show "short" from food alone (EFSA 2016 basis, **UNVERIFIED**) |
| **Energy and protein for me** | Energy no higher than an estimate from sex, age, height, weight, activity; protein 0.83 g/kg | Henry 2005 equations (EFSA's choice); EFSA activity levels (*"a rough indication"*); WHO/FAO/UNU 2007 | Sound physiology; any equation is ±10 % or more for an individual | Too little protein over 65; wrong ceiling while breastfeeding; must not run for anyone flagged with an eating disorder |
| **Two fasting days a week** (5:2) | Two days a week at ≤ 25 % of energy need (~500–600 kcal) | A **definition** (Koppold et al., *Cell Metab* 2024, Delphi of 38 experts); recommended by no public body | RCTs: equal to continuous restriction (Harvie 2011; BMJ 2025 NMA: no advantage) | "Not intended for" line; "about as effective as eating a little less every day" |
| **Alternate-day modified fasting** | Every other day ≤ 25 % of need | Same consensus | RCTs: the only form slightly ahead of continuous restriction (−1.29 kg, BMJ 2025) — **only in trials under 24 weeks**; in the 17 trials of ≥ 24 weeks no form of fasting beat continuous restriction. And the **highest dropout** (38 % vs 29 %, Trepanowski 2017) | As above |
| **Time-restricted eating** | All eating within a window, ≥ 14 h without food | Same consensus (trials mostly used 8 h) | Works by making people eat less; **no benefit once calories are matched** (Liu, *NEJM* 2022; Lin, *Ann Intern Med* 2023) | As above; do not imply long-term safety is shown (a 2024 unreviewed AHA abstract on short windows and CV death is noted as caution only) |
| **Nordic recommendations 2023** | Food-group amounts (500–800 g fruit/veg, ≥ 90 g whole grain, nuts, fish, ≤ 350 g red meat/week, dairy, oil); the person says each day whether they kept to them | Nordic Council of Ministers, adults in the Nordic and Baltic countries; health first (*"No recommendation has been adjusted by environmental impact … in conflict with the health-based"*, read) | **The most transparently graded card**: ~100 graded systematic reviews; grades stated per food group (*convincing*, *probable*…) | **Measures nothing, but could**: fat 25–40 %, saturated fat < 10 %, carbohydrate 45–60 %, protein 10–20 %, fibre ≥ 25 g women / ≥ 35 g men, sodium 2.3 g; also recommends **avoiding alcohol** (read) |
| **Planetary health diet (EAT-Lancet 2025)** | A plant-rich ~2,400 kcal day: grains, vegetables, fruit, legumes, nuts, dairy, small meat/fish/eggs, oils, ≤ 30 g sugar | EAT-Lancet Commission (*Lancet* 2025, doi:10.1016/S0140-6736(25)01201-2), the **global average adult**, for health **and** environmental limits; *"not a one-size-fits-all prescription"* | Health benefit is **modelled** from cohort risks, not trialled | Card shows midpoints only — the table has **ranges starting at 0** (red meat 0–30 g, dairy 0–500 g), starchy roots 50 g, and **sodium < 2,000 mg** (the one measurable item); a smaller person should scale it down; **nutrient-adequacy caution** (Beal 2023: iron ~55 % of need for women 15–49) and the report's own advice on fortified foods or supplements for children, women of reproductive age and vegans |
| **My own** | Any numbers | The person | — | — |

### 2b. Fitness (11 cards)

| Card | What it asks | Who, for whom | Evidence | What is off / missing |
|---|---|---|---|---|
| **WHO weekly activity** | 150 min moderate or 75 vigorous a week; strength on 2 days | WHO 2020, adults 18–64 incl. chronic conditions and disability | **Strong** (GRADE; strength: *"strong recommendation … moderate-certainty"*, read) | 150 is the **floor of a range** — 150–300; beyond that *"does not appreciably further decrease"* mortality; *"some physical activity is better than none"* |
| **WHO, 65 and over** | The same, plus balance/strength-led multicomponent work on ≥ 3 days | WHO 2020, 65+ | Strong; falls: *"High-certainty evidence"* (read) | — |
| **Falls prevention** | Balance-challenging, functional exercise ≥ 3 sessions/week for ≥ 12 weeks | World Falls Guidelines 2022, older adults at risk | **Strong**: Cochrane 2019, falls rate −23 % (59 studies, high certainty) | Add: **walking alone is not a falls programme**; after a fall the guideline expects an assessment |
| **Strength, twice a week** | Every major muscle group ≥ 2 days, ≥ 2 sets, short of failure | **ACSM 2026** position stand (Currier et al., *MSSE* 58(4):851–72, doi:10.1249/MSS.0000000000003897) | Strong for "resistance training beats none"; moderate for the variables | **No URL on the card** — use the DOI; ACSM: failure *"not necessary"*, maybe *"inadvisable"* for older people; the exact "2–3 repetitions short" is not pinned down by evidence |
| **Muscle growth** | ≥ 10 sets/week per muscle group, any load | ACSM 2026 (hypertrophy) | Moderate (meta-regression; returns diminish ~18–20 sets) | — |
| **Minimal strength** | Leg, pull, push: ≥ 4 sets/week each | Iversen et al. 2021, a **narrative** review | Weak–moderate; ACSM 2026: *"insufficient evidence"* to recommend minimal-dose programmes | Label it a narrative review; done on one day it does not meet WHO's "2 days" |
| **Stretching for range of motion** | Daily, 2–3 sets per muscle, 30–120 s | Warneke et al. 2025 Delphi (20 experts) | **Expert consensus** — the lowest tier | **No URL** — doi:10.1016/j.jshs.2025.101067; "daily" is the panel's route to *"the highest possible weekly volume"*, so 7/7 is strict — "most days" fits the source better |
| **Norwegian 4×4** | Four 4-min hard efforts, 3 min easy between, 2–3×/week | NTNU CERG; trial-based | VO2max gain moderate–strong (Milanović 2015); **longevity not shown** (Generation 100, *BMJ* 2020: HR 0.63, CI 0.33–1.20) | The card's need for screening is the highest of all (§6b) |
| **Interval walking** | 3 min fast / 3 min slow ×5, ≥ 4 days/week | Nemoto, Nose et al. 2007, people ~63 | **Moderate–weak**: one RCT; fewer than half met targets; no meta-analysis found | "Fast" is defined against a lab measure — say "fast enough that talking is hard" |
| **Activity for losing weight** | > 250 min/week plus strength on 2 days, with eating less | ACSM 2009 + US 2018 | Moderate; **dated** | Add Jayedi et al., *JAMA Netw Open* 2024 (116 RCTs): ~0.5 kg per 30 min/week, > 150 min/week for clinically important change |
| **My own** | Any numbers | The person | — | — |

---

## 3. Why the food cards' numbers differ

They were written for different jobs, so different numbers are not errors:

| | WHO | EFSA | Nordic 2023 | Others |
|---|---|---|---|---|
| **Purpose** | prevent disease and weight gain, worldwide | enough of each nutrient, healthy Europeans | health for Nordic/Baltic adults (environment second) | EAT-Lancet: health **and** planet |
| Fat, % energy | ≤ 30 (weight gain only) | 20–35 | 25–40 | US 20–35 |
| Carbohydrate, % | 45–75 (*"the percentage … available after … protein and fat, hence the wide range"*) | 45–60 | 45–60 | US 45–65 |
| Protein | 10–15 % "sufficient" | 0.83 g/kg | 10–20 %; ~1.2 g/kg older adults | US RDA 0.8 g/kg; DGA 2025–30 1.2–1.6 g/kg (a policy choice WHO/EFSA do not share) |
| Saturated fat | < 10 % | "as low as possible" | < 10 % | consensus |
| Fibre | ≥ 25 g | 25 g | ≥ 25 g women, ≥ 35 g men | |
| Sodium | < 2 g | 2 g | 2.3 g | US 2.3 g (NASEM 2019 level) |

WHO's 10–15 % protein and EFSA's 0.83 g/kg say **the same thing in two units** for most adults (70 kg ×
0.83 ≈ 58 g ≈ 12 % of 2,000 kcal). None of WHO, EFSA or Nordic sets a protein **ceiling** for healthy
adults.

---

## 4. Which cards are alternatives, which combine

| Choose one (same goal) | Combine freely |
|---|---|
| WHO **or** EFSA **or** Nordic nutrient targets (WHO for prevention, EFSA for adequacy, Nordic for both with graded food groups) | any food card + Energy and protein |
| Nordic food groups **or** EAT-Lancet | any WHO activity card + one strength card |
| 5:2 **or** alternate-day **or** time-restricted | any card + stretching |
| WHO adult **or** WHO 65+ (by age) | WHO 65+ + falls prevention — **one balance session should count for both** |
| WHO adult **or** weight loss (the second contains the first) | WHO + 4×4 **or** interval walking (they fill its minutes) |
| strength minimal **or** health **or** growth (a ladder) | |
| 4×4 **or** interval walking (fitness; interval walking is the gentler entry) | |

---

## 5. What the science says, in one paragraph each

- **Nutrient targets.** The best-supported limits are saturated fat, salt, and fibre/whole grains (RCTs
  of lipids and blood pressure plus large cohorts). Total fat ≤ 30 % is the weakest and most contested.
  Macronutrient ranges are about adequacy, not optimisation.
- **Food patterns.** Patterns beat single nutrients. The strongest named pattern is **Mediterranean**
  (PREDIMED, republished 2018; CORDIOPREV 2022; though Cochrane 2019 calls event effects uncertain);
  **DASH** is proven on blood pressure; Nordic 2023 grades its food groups; EAT-Lancet is modelled.
- **Fasting.** Cochrane (Feb 2026, CD015610; 22 RCTs, ~2,000 adults): *"may result in little to no
  difference in weight loss"* versus regular advice. BMJ 2025 network meta-analysis (99 RCTs): *"similar
  benefits to continuous energy restriction"*; only alternate-day slightly ahead, and only in trials
  under 24 weeks. Not shown: anything beyond weight loss, beyond 1–2 years, in normal-weight people.
- **Activity.** The biggest drop in mortality risk is from **none to 150 min** (Garcia, *BJSM* 2023:
  RR 0.69); gains above are smaller. Steps: benefits level off ~5,000–7,000 **for all-cause death,
  heart disease incidence, dementia and falls**, but keep improving in a straight line for heart
  disease death, cancer, type 2 diabetes and depression (Ding 2025).
- **Strength.** ACSM 2026 (137 systematic reviews): resistance training *"is safe for healthy adults of
  all ages"*; ≥ 2 sets; failure unnecessary; ≥ 10 sets/week for growth; periodisation does not
  consistently matter; *"individualizing programs to increase RT participation is … more important"*.

---

## 6. People with deficiencies or conditions

### 6a. The common deficiencies, and what bodies advise

| Nutrient | Who is at risk | Advice from authoritative bodies | Upper limit that matters |
|---|---|---|---|
| **Iron** | Menstruating women and girls; pregnancy; under-5s (WHO: anaemia in 30 % of women 15–49, 37 % of pregnant women, read); vegetarians; female and endurance athletes. Singapore: ~¾ of women in a 2019 pregnancy cohort had low ferritin (*summary*) | EFSA 11 mg; **16 mg before the menopause**; NHS 14.8 mg women 19–49. No extra in pregnancy per EFSA (absorption rises) | NHS: *"17mg or less a day … unlikely to cause any harm"*; *"Very high doses … can be fatal, particularly if taken by children"* (read). Never suggest iron blindly (iron overload, **UNVERIFIED**) |
| **Vitamin B12** | Vegans without a source; pernicious anaemia; **metformin and acid-suppressant users** (MHRA 2022: up to 1 in 10 on metformin) | Vegan Society: fortified foods 2–3×/day for ≥ 3 µg, **or** ≥ 10 µg daily, **or** ≥ 2,000 µg weekly (read) | none set |
| **Vitamin D** | Little sun, dark skin, older people, winter at high latitude; Singapore 2010: 40 % deficient (MOH, read) | NHS: 10 µg a day **between early October and late March** for everyone, incl. pregnancy | **EFSA 2023: 100 µg adults, 50 µg ages 1–10** |
| **Folate** | Anyone who could become pregnant | NHS: **400 µg folic acid** until week 12 (5 mg for some, via a doctor); EFSA 600 µg in pregnancy | 1,000 µg added folic acid (masks B12 deficiency) |
| **Iodine** | No dairy or fish; unfortified plant milk; pregnancy | Fortified milk or a suitable supplement; **avoid kelp** (BDA); EFSA 150 µg, 200 µg in pregnancy and lactation | 600 µg |
| **Calcium** | Vegans, no dairy, post-menopause | EFSA 950 mg; UK osteoporosis 700–1,200 mg | 2,500 mg (on the card) |
| **Zinc** | High-phytate diets (vegetarian, vegan) | EFSA PRI rises with phytate: 7.5–12.7 mg women, 9.4–16.3 mg men | 25 mg |
| **Omega-3** | No fish | EFSA 250 mg EPA+DHA; +100–200 mg DHA in pregnancy | — |

### 6b. Conditions where a card's number is wrong or unsafe

| Card number | Condition | What the bodies say | Severity |
|---|---|---|---|
| Potassium ≥ 3,500 mg | **Chronic kidney disease**; potassium-raising medicines | WHO 2025 salt-substitute guideline: not for people with kidney impairment or conditions reducing potassium excretion (nor children, pregnancy); fact sheet: *"people not at risk of hyperkalaemia"*. KDIGO 2024 limits potassium-rich food **individually** — with a history of high potassium or in periods of risk — not for everyone with CKD; NHS: a dietitian decides | **Unsafe for some** — "eat more potassium" must never reach a CKD user; the app cannot know which CKD user is safe, so it stops showing the target |
| Protein ≥ 0.83 g/kg | **Age 65+** | PROT-AGE 1.0–1.2 g/kg; Nordic ~1.2 g/kg | Too low |
| Protein (any "more is better") | **CKD** | KDIGO 2024: 0.8 g/kg for stages G3–G5 (Rec. 3.3.1.1, grade 2C); avoid > 1.3 g/kg (Practice Point 3.3.1.1) | The US 1.2–1.6 is wrong here |
| Folate 330 µg; energy ceiling | **Pregnancy / breastfeeding** | EFSA: folate 600/500 µg, iodine 200, B12 4.5/5.0, protein +1/+9/+28 g by trimester; **+~500 kcal/day while breastfeeding** | Wrong |
| Fasting cards | Eating-disorder history; BMI < 18.5; children under 12 (Varady 2022 — full text paywalled, **secondary sources agree**); diabetes on insulin or sulfonylureas (IDF-DAR, *Diabetes Res Clin Pract* 2022); pregnancy, breastfeeding, over 70 (**not studied**) | Supervision or not at all | Unsafe for these |
| 4×4, interval walking | Known heart, metabolic or kidney disease; symptoms | WHO: clearance generally unnecessary for light–moderate; ACSM: clearance before **vigorous** with known disease or symptoms (algorithm wording **UNVERIFIED**); in supervised cardiac rehab, events rare but ~5× more per hour at high intensity (Rognmo 2012) | Screening link (PAR-Q+, link only — all rights reserved) |
| Sodium ≤ 2 g | Heart failure | SODIUM-HF 2022: < 1.5 g did not reduce death or admission | Harmless, but not a heart-failure regime |
| Training while iron-deficient | Fatigue, breathlessness | A reason to see a doctor, not to train harder | — |

### 6c. What the app may say, and what it must not

**May:** show a named, published value against the person's own log; quote advice addressed to a
population, with its source (*"The NHS advises 10 µg vitamin D in autumn and winter"*; *"A vegan diet
needs a reliable B12 source"*). Use the NHS's three-part wording: the safe amount, when to see a
professional, the specific harm — never a blanket "talk to your doctor".

**Must not:** say "you are deficient" (only a blood test shows that); suggest a supplement **dose** for
the person; compute **disease-specific** targets (a CKD potassium limit) — it should **stop showing** the
unsafe target, not invent a replacement. (EU MDCG 2019-11, UK MHRA and US FDA general-wellness guidance
draw the medical-device line at diagnosis, individual dosing and disease treatment; read only in
secondary commentary.)

**Screening as opt-in profile flags**, not questions everyone must answer: pregnant or breastfeeding;
kidney disease; diabetes medication; eating disorder, past or present; under 18; age. Each flag only
shows the matching caution and swaps in a named alternative number where a body publishes one. With the
eating-disorder flag set, hide the fasting and weight-loss cards and the energy ceiling — calorie
tracking is associated with eating-disorder symptoms (Levinson et al. 2017: 73 % of MyFitnessPal users
with an eating disorder felt it contributed; cross-sectional).

---

## 7. Gaps: what people look for that has no card, strongest evidence first

| Candidate | Evidence | Measurable? |
|---|---|---|
| **Steps per day** (7,000; 10,000 for the more active) | Ding et al., *Lancet Public Health* 2025 (57 studies): levels off ~5,000–7,000 for death from any cause, heart disease, dementia, falls; linear for others; Paluch 2022: 6–8k at ≥ 60, 8–10k below | Needs a step count |
| **DASH** | RCTs on blood pressure (meta-analysis GRADE moderate); NHLBI | Largely — sodium, potassium, saturated fat, fibre; its menus already ship |
| **Mediterranean** | PREDIMED 2018, CORDIOPREV 2022; Cochrane 2019 "uncertain" | Partly — the 14-item MEDAS screener as self-check |
| **Protein for older adults** | Nordic 2023, PROT-AGE, ESPEN | Trivially — a 1.0–1.2 g/kg variant |
| **Vegan/vegetarian nutrient watch** | Cohorts (less heart disease; EPIC-Oxford more strokes) | Yes — B12, iron, calcium, iodine from existing keys |
| **Sitting less** | WHO 2020 (conditional, no number); Ekelund 2016: 60–75 min/day moderate offsets heavy sitting | As guidance |
| **Pregnancy activity** | WHO 2020's own recommendation | Yes, with a clinician caveat |
| Not recommended as cards | Low-carb/keto (no agreed definition; DIETFITS equal), MIND (2023 trial null), "zone 2" (no definition), back pain (clinical; small effects — a linked guide instead) | — |

---

## 8. A correction to the source register

`sources-2026-10-11.md` §2 says, marked *search*, that EFSA cautions its values *"are not
recommendations for individuals"*. EFSA's own pages describe DRVs as what must be consumed *"to maintain
health in an otherwise healthy individual (or population)"*, used *"at the population and individual
level"*. The line should be corrected or re-sourced (not edited here: the register is a dated record).

---

## 9. What the adversarial check found

Sixteen consequential claims were checked against their primary texts (~40 lookups). **None refuted;
no citation fabricated.** Confirmed with the source's own words: WHO protein "sufficient" and TRS 916's
population-average framing; EFSA iron 16/11 mg; every Nordic 2023 figure (sodium 2.3 g = 5.75 g salt;
older adults 18 E% ≈ 1.2 g/kg "for food planning purposes"); the EAT-Lancet 2025 table and its
fortification line; Beal 2023's figures (a *Viewpoint*, not original research); Cochrane 2026; EFSA
pregnancy values; EFSA vitamin D UL (100 µg adults and ages 11–17, 50 µg ages 1–10); PROT-AGE; ACSM
2026; Warneke 2025; Paluch 2022; Jayedi 2024; WHO 2020's clearance line (via the Bull et al. *BJSM*
summary of the guideline).

**Corrected, and already applied above:** alternate-day fasting's edge is in trials under 24 weeks
only (BMJ 2025; its published correction, *BMJ* 390:r1737, not opened); the WHO 2025 salt-substitute
guideline is the stronger kidney source, and KDIGO restricts potassium individually; KDIGO's
limit above 1.3 g/kg is Practice Point 3.3.1.1; the NHS vitamin D line is seasonal; ESPEN 2019 says ~1 g/kg,
individually adjusted (1.0–1.2 is PROT-AGE's); Ding 2025's plateau holds for some outcomes only.

**Unverifiable:** Varady 2022 (paywalled; secondary sources agree, and add "not studied over 70").

**Citation details fixed:** Lin 2023 is doi:10.7326/M23-0052 (*Ann Intern Med* 176:885–95); Pelland
is *Sports Med* 2026;56:481–505; Nicol is *Br J Nutr* 2024, a narrative review; IDF-DAR appeared in
*Diabetes Res Clin Pract* 2022;185; Sim 2019 is a narrative review; Pengelly 2024 covers female
athletes only.

---

## 10. Sources (beyond the register)

**Food.** WHO healthy diet fact sheet, 26 Jan 2026 · WHO fat/carbohydrate guidelines, July 2023 ·
WHO/FAO TRS 916 (2003) · EFSA DRV summary 2017 · NNR 2023, doi:10.6027/nord2023-003 · EAT-Lancet 2025,
doi:10.1016/S0140-6736(25)01201-2 · Beal, Ortenzi & Fanzo 2023, doi:10.1016/S2542-5196(23)00006-2 ·
Koppold et al. 2024, doi:10.1016/j.cmet.2024.06.013 · Cochrane CD015610 (Garegnani et al. 2026) ·
Semnani-Azad et al., *BMJ* 2025, doi:10.1136/bmj-2024-082007 · Trepanowski 2017,
doi:10.1001/jamainternmed.2017.0936 · Liu 2022, doi:10.1056/NEJMoa2114833 · Lin 2023, doi:10.7326/M23-0052 ·
Cochrane Mediterranean 2019, doi:10.1002/14651858.CD009825.pub3 · CORDIOPREV,
doi:10.1016/S0140-6736(22)00122-2 · Tong 2019, doi:10.1136/bmj.l4897 · MIND trial,
doi:10.1056/NEJMoa2302368.

**Fitness.** WHO 2020 (PMC7719906) · ACSM 2026, doi:10.1249/MSS.0000000000003897 · Warneke 2025,
doi:10.1016/j.jshs.2025.101067 · Iversen 2021, doi:10.1007/s40279-021-01490-1 · Stensvold 2020,
doi:10.1136/bmj.m3485 · Milanović 2015, doi:10.1007/s40279-015-0365-0 · Nemoto 2007,
doi:10.4065/82.7.803 · Jayedi 2024, doi:10.1001/jamanetworkopen.2024.52185 · Sherrington 2019,
doi:10.1002/14651858.CD012424.pub2 · Ding 2025, doi:10.1016/S2468-2667(25)00164-1 · Paluch 2022,
doi:10.1016/S2468-2667(21)00302-9 · Garcia 2023, doi:10.1136/bjsports-2022-105669 · Ekelund 2016,
doi:10.1016/S0140-6736(16)30370-1 · Rognmo 2012, doi:10.1161/CIRCULATIONAHA.112.123117 · PAR-Q+,
https://eparmedx.com/.

**Deficiencies and conditions.** WHO anaemia fact sheet, Feb 2025 · NHS vitamins and minerals pages
(iron, vitamin B, vitamin D) · Vegan Society B12 · EFSA vitamin D UL 2023, doi:10.2903/j.efsa.2023.8145 ·
KDIGO 2024 CKD guideline, *Kidney Int* 105(4S) · PROT-AGE (Bauer et al. 2013) · ESPEN geriatrics 2019,
doi:10.1016/j.clnu.2018.05.024 · Varady et al. 2022, doi:10.1038/s41574-022-00638-x · IDF-DAR (*Diabetes Res Clin Pract* 2022;185),
doi:10.1016/j.diabres.2021.109185 · WHO salt-substitute guideline 2025 · MOH Singapore, vitamin D reply 2018 · Loy et al. 2019,
doi:10.1186/s12889-019-6736-y · Levinson et al., *Eat Behav* 2017.
