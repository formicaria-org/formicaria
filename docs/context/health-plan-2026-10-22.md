# A Health group — research and a proposal (2026-10-22)

> **A proposal, not a ruling.** Nothing here is built and nothing is decided. A fifth group
> **extends** `decisions.md#ui` *four groups over notes* (2026-10-12), and `log: health` extends the
> closed set of log kinds (*the tracker's domain parts are a closed set*, 2026-10-11). So a dated
> `decisions.md` entry comes first, per `CLAUDE.md`'s four questions. The owner's four answers
> (§6) are binding on this proposal. **Revised the same day** after three source-level reviews:
> clinical definitions, instrument licences, and similar apps with their published evaluations.

## 1. What prompted it

The owner: *"these were all actually ways for you to do a research that I need to improve formicaria
with a new view: health"*, then *"refine with grounded research and research on similar apps."* The
conversation went from house-dust-mite IgE results to treatments, then to three years of headache,
nose and eye symptoms that ease when the bedding is washed, and ended at **which standard diaries a
person keeps to bring to a doctor**. That last question is the feature.

## 2. The rule it must meet

`decisions.md#data` *nothing is invented*, with its register `sources-2026-10-11.md`. For health
this means **the app records and counts; it never diagnoses, flags, advises or treats.** Every scale,
definition or wording it shows names a source that was read **at the source**. *What the person types
is theirs.* On ruling, the rows in §3 move into the register as a new section.

Checked column, as in the register: **source** = read on the holder's own page or document on
2026-10-22. **search** = secondary or snippet. **memory** = recall. Only *source* rows may ship.

## 3. What the sources say

### 3.1 What a headache diary records, and what a day is

| Claim | Source | Checked |
|---|---|---|
| "Characterization of frequently recurring headache generally requires a headache diary to record information on pain and associated symptoms day-by-day for at least one month." | ICHD-3 1.3, note 2: [ichd-3.org/…/1-3-chronic-migraine](https://ichd-3.org/1-migraine/1-3-chronic-migraine/) | source |
| A diary records "for a minimum of 8 weeks: frequency, duration and severity of headaches / any associated symptoms / all prescribed and over the counter medications taken to relieve headaches / possible precipitants / relationship of headaches to menstruation" (1.1.4) | NICE CG150: [recommendations](https://www.nice.org.uk/guidance/cg150/chapter/Recommendations) | source |
| "Headache days: Number of days during an observed period of time (commonly one month) affected by headache for any part or the whole of the day." | ICHD-3: [definition of terms](https://ichd-3.org/definition-of-terms/) | source |
| An episode under 24 h spanning midnight is **one** day ("a headache starting at 20:00 and ending at 01:00 … counted as a single migraine or headache day") | IHS, Tassorelli 2018, preventive trials in chronic migraine: [PDF](https://ihs-headache.org/wp-content/uploads/2025/04/Clinical-Trials-Controlled-trials-of-preventive-treatment-of-chronic-migraine-in-adults-CEP.pdf) | source |
| A diary needs only: headache yes/no, peak severity mild/moderate/severe, duration < 4 h or ≥ 4 h, type of acute medication, associated symptoms | same | source |
| "Minimize the response burden … the time needed to complete each daily set of questions is similar regardless of whether subjects experience an attack." Electronic diaries "with time stamps (to reduce recall bias)" are preferred | same ([accepted manuscript](https://iris.unipv.it/handle/11571/1239666)) | source |
| Severity: "a 4-point scale where 0 = no headache; 1 = mild headache; 2 = moderate headache; 3 = severe headache. Alternatively, a 100-mm VAS or an 11-point numerical rating scale (NRS) can be used." | IHS, Diener 2019, acute-treatment trials, 4th ed.: [PDF](https://ihs-headache.org/wp-content/uploads/2025/04/Clinical-Trials-Controlled-trials-of-acute-treatment-of-migraine-attacks-in-adults-4th-ed-CEP.pdf) | source |
| NRS end anchors ("0 = no pain … 10 = worst pain imaginable") | IMMPACT (Dworkin 2005) is closed access; not read | memory: **the 0–10 option shows numbers only** |

### 3.2 Definitions a doctor applies to those counts (for the manual, not for computed flags)

| Claim | Source | Checked |
|---|---|---|
| Chronic migraine: headache "on ≥15 days/month for >3 months", with migraine features "on ≥8 days/month" | ICHD-3 1.3 (above) | source |
| Chronic tension-type headache: "≥15 days/month on average for >3 months"; where both fit, "only the diagnosis 1.3 Chronic migraine should be given" | [ICHD-3 2.3](https://ichd-3.org/2-tension-type-headache/2-3-chronic-tension-type-headache/) | source |
| Medication overuse: triptans, opioids, ergots or combination analgesics "on 10 days per month or more", or "paracetamol, aspirin or an NSAID, either alone or in any combination, on 15 days per month or more", for 3 months or more | NICE CG150 1.2.7; [ICHD-3 8.2](https://ichd-3.org/8-headache-attributed-to-a-substance-or-its-withdrawal/8-2-medication-overuse-headache-moh/) agrees, and adds that the thresholds "are based on expert opinion rather than on formal evidence" | source |
| Chronic rhinosinusitis (symptom definition): two or more symptoms, one being blockage or discharge, "for ≥12 weeks" | EPOS 2020 §1.2.2.6: [Rhinology Suppl 29](https://www.rhinologyjournal.com/Documents/Supplements/supplement_29.pdf) | source |
| Allergic rhinitis "persistent" = symptoms "more than 4 days a week, and for more than 4 weeks" | ARIA, as reproduced in [PMC1936314](https://pmc.ncbi.nlm.nih.gov/articles/PMC1936314); secondary wordings differ | search: **not shipped** |
| When to call 999, get urgent care, or see a GP | [nhs.uk/symptoms/headaches](https://www.nhs.uk/symptoms/headaches/); NICE CG150 1.1.1 red flags | source. NHS text is OGL v3.0, so it may be quoted verbatim with its date (§5.2) |

**Decision proposed: no computed flags** (*refined in §5.2*: a followed card's check may show a count
beside a body's number, as Fitness does, never with a diagnosis word). The month shows counts. The manual quotes these
definitions with their source, so a person can compare the counts themselves. This follows lesson 9
in §4, and keeps the app clear of the medical-device question: MASK-air was judged not a device
*because* it gives no diagnosis or treatment advice ([PMC8449952](https://pmc.ncbi.nlm.nih.gov/articles/PMC8449952/)).

### 3.3 Nose and eyes

| Claim | Source | Checked |
|---|---|---|
| TNSS: rhinorrhea, congestion, itching, sneezing, each 0–3, with anchors (e.g. "1 = mild symptoms (sign/symptom present, but minimal awareness; easily tolerated)"). A US federal work | FDA, *Allergic Rhinitis: Developing Drug Products for Treatment* (2018): [PDF](https://www.fda.gov/media/71158/download) | source |
| A daily 0–100 "bother" slider per area (overall, nose, eyes, asthma, sleep, work): 0 "not at all bothersome", 100 "extremely bothersome". Validated in 17,780 users | Sousa-Pinto 2021, *Clin Transl Allergy*, **CC BY 4.0**: [PMC8449952](https://pmc.ncbi.nlm.nih.gov/articles/PMC8449952/). "MASK-air®" is a registered name: don't use it | source |
| EAACI daily symptom score (6 symptoms × 0–3) and combined symptom-medication score | Pfaar 2014, *Allergy*: closed access, read only through secondary papers | search: **not shipped** |

### 3.4 Instruments a person may bring a score from

| Instrument | Holder and terms | May the app show the items? | Checked |
|---|---|---|---|
| SNOT-22 | Washington University in St. Louis. "You may not publish the instrument"; the free tier is non-commercially-funded research only: [licensing](https://sinonasaltest.wustl.edu/licensing-snot) | **No.** Store the person's total (0–110) and link out | source |
| HIT-6 | IQVIA (formerly QualityMetric). A signed licence is needed for all use, with fees for electronic use: [COA licensing](https://coas.iqvia.com/COAs/headache-impact-test-standard) | **No.** Score + link | source |
| MIDAS | © Innovative Medical Research 1997 / AstraZeneca 2007, "all rights reserved" printed on the form; no reuse terms found | **No.** Score + link | search |
| Epworth Sleepiness Scale | M. W. Johns; licences through Mapi Research Trust. "A license is required to use the ESS" | **No.** Score + link | search (holder site and Mapi unreachable on 2026-10-22) |
| PHQ-9 / GAD-7 (mood, later) | Pfizer: "no permission is required to reproduce, translate, display or distribute them": [phqscreeners.com](https://www.phqscreeners.com/select-screener) | Yes, if mood is ever added | source |
| Migraine Trust diary; NHS BNSSG diary | No licence stated on either, so treat as all rights reserved. Their **fields** are generic | Link to them; copy neither | source |

## 4. What similar apps and their evaluations teach

Apps reviewed: Migraine Buddy, N1-Headache, Migraine Monitor, MASK-air, Bearable, Flaredown
(GPL-3.0), CareClinic, Daylio, Track & Graph (GPL-3.0, offline), Apple HealthKit, Android Health
Connect. Most are cloud services with an account. The two open-source ones are the nearest in
spirit: Flaredown's daily check-in uses a 0–4 integer that may be left blank; Track & Graph keeps
person-defined trackers and nothing online. Unless marked otherwise, the evidence below is
*source*-checked.

1. **Few diaries are built on expertise.** [Hundert 2014, JMIR mHealth](https://mhealth.jmir.org/2014/3/e36)
   looked at 38 apps: 18 % had headache expertise, none reported psychometrics, and 18 % captured all of
   severity, triggers, medication, associated symptoms, frequency, disability and duration. →
   *The starter template carries the IHS/NICE fields and nothing more; the person adds the rest.*
2. **People misjudge their own month.** Against a daily e-diary, patients' estimates were off by
   4.7 ± 5.0 migraine days, 6.2 other headache days and 4.3 medication days per month
   ([van Casteren 2021, Cephalalgia](https://doi.org/10.1177/03331024211010306)). → *The month leads
   with counts.*
3. **Paper diaries are filled in after the fact.** In one study 90 % compliance was reported and 11 %
   was real ([Stone 2002, BMJ](https://www.bmj.com/content/324/7347/1193); *search*). →
   *A note's own `created` already shows when a day was written. The month marks days entered late,
   and never locks a file.*
4. **People drop off.** Pooled dropout from chronic-disease apps is 43 %
   ([Meyerowitz-Katz 2020, JMIR](https://doi.org/10.2196/20283)). → *A missed day is shown plainly,
   with no streaks or nags. Filling in yesterday is one tap.*
5. **Blank is not zero.** HealthKit separates `notPresent` from `unspecified`; Flaredown allows blank;
   IHS counts more than 6 missing days in 28 as low compliance. → *A recorded 0 is data. The month
   says "24 of 31 days recorded".*
6. **Trigger beliefs mostly fail when tested.** In N1-Headache, 80 % believed neck pain was a trigger
   and 32 % showed a link (*search*); trigger profiles were unique in 85 % of people
   ([Peris 2017, Cephalalgia](https://doi.org/10.1177/0333102416649761)). → *If co-occurrence is ever
   shown, it is arithmetic with n ("headache on 6 of 9 days after bedding washed; 5 of 20
   otherwise"), same day and next day together, never called a cause. Not in v1.*
7. **Users want customisation, trends and export**
   ([Minen 2020, Headache](https://doi.org/10.1111/head.13859), 945 reviews of 15 apps). →
   *Person-named properties; one picked trend; the month as a printable page and a CSV of the same
   numbers.*
8. **Most health apps leak.** 33 of 36 apps sent data to third parties
   ([Huckvale 2019, JAMA Netw Open](https://doi.org/10.1001/jamanetworkopen.2019.2542)); 79 % of
   medicines apps shared with 55 entities ([Grundy 2019, BMJ](https://doi.org/10.1136/bmj.l920)); 55 %
   of headache diaries with a policy used data for ads ([Minen 2018, Headache](https://doi.org/10.1111/head.13341)).
   → *formicaria already has no account, no analytics and no SDK. Say so in one line on the group, and
   let the person choose which properties go into the doctor's page.*
9. **A doctor reads medication days, by medicine.** The Migraine Trust diary asks about a second dose
   (*source*). → *Doses are counted per medicine per day; the month counts medication days per
   medicine. It does not flag a threshold.*
10. **Vocabulary for later.** HealthKit has 39 symptom identifiers (headache, runnyNose,
    sinusCongestion, fatigue …) and a severity scale `notPresent/mild/moderate/severe`, which matches
    the IHS 0–3 one-to-one. Android's Health Connect has **no** symptom type. → *Choosing 0–3 keeps a
    future HealthKit export possible. CSV is the portable path.*

**What formicaria has that they lack:** files that outlive the app (Migraine Monitor, unmaintained
since 2023), no third-party data flow, triggers named by the person (such as *bedding washed*, which
no standard diary has), and headache and allergy in one diary. **What they have that this
deliberately leaves out:** automatic weather and sleep, statistical trigger inference (N1 is CE
Class I because of it), clinician remote monitoring, community features.

## 5. The proposal

**Health is a fifth group over notes.** It needs no new `Kind`, no new file type, no new renderer
beyond what `totals` does, and no dependency.

**An entry is a day note, `log: health`, dated by day** (*an entry is dated by day*, 2026-10-11).
Values are whole numbers with the unit in the key. A missing key means *not recorded*; 0 means
*recorded as none*.

**The diary is general (owner, §6): the person names what they track.** Three things ship to start
from, each a starter template (*a template carries structure*, 2026-10-12), with every scale sourced:

| Starter | Keys | Scale and source |
|---|---|---|
| **Headache** | `headache_0to3`, `headache_hours`, `nausea`, `light_hurts`, `noise_hurts` (0/1) | IHS 4-point (Diener 2019); fields from IHS/NICE |
| **Nose and eyes** | `runny_nose_0to3`, `blocked_nose_0to3`, `itchy_nose_0to3`, `sneezing_0to3`, `itchy_eyes_0to3`, `watery_eyes_0to3` | FDA TNSS anchors for the nose (2018); eye items on the same 0–3 scale. **Alternative:** a 0–100 "bother" slider per area, adapted from Sousa-Pinto 2021 (CC BY), with citation |
| **Medication** | `<name>_doses` per medicine, named by the person | a count; nothing to source |

The person adds their own keys, such as `bedding_washed` (0/1), the owner's own clue, with whatever
0–3, 0–10 or 0/1 scale they choose. The 0–10 option shows numbers only, since its anchors are unread
(§3.1). A severity choice of 0–3 or 0–10 is the person's; 0–3 is the default because it is the IHS
default and maps to HealthKit.

**Views, at most four for the phone's bar:**
1. **Day.** One screen; the same fields every day, whether or not anything happened (Tassorelli).
   It opens on today, with a one-tap switch to yesterday.
2. **Month.** A grid of days, plus counts: days recorded, headache days (`headache_0to3 > 0`, the
   ICHD-3 glossary sense), moderate-or-severe days, medication days per medicine, and days of each
   0/1 key the person added (*bedding washed*). Late-entered days are marked. Printable, with a CSV
   of the same numbers. The person picks which keys go on it.
3. **Trends.** One series picked from a menu (*a trend is picked from a menu*, 2026-10-12).
4. **Cards**: grounded cards like Fitness's *Regime* and Food's *Diet* (§5.2). The tab's name is
   open (§7).

Questionnaire totals (SNOT-22, HIT-6, MIDAS, Epworth) are not a view of their own. Each is a tracker
of kind *score* (§5.1), typed on the day it was taken, named, linked to its holder's page, and drawn
in Trends like anything else. **Never the items.**

**Engine.** Counting "days where a key > 0" and "days recorded" is one new aggregation in pure
`fm-query`. It stays inside seam 1.

**Manual page, `docs/src/user/health.md`.** What a diary is for (ICHD-3, NICE, quoted); how long to
keep it (ICHD-3 at least a month, NICE at least 8 weeks); the definitions in §3.2, quoted with their
source; a link to the NHS page for when to get help; links to the Migraine Trust and NHS diaries;
and one line saying nothing leaves the device except by the vault's own backup.

**Not in v1:** co-occurrence of triggers (lesson 6), reminders (`reminders.rs` could later take a
daily evening one; Leiden used 18:00), HealthKit export, and any assistant involvement
(`decisions.md#agent`).

**Process rules that stand:** nothing is laid over an existing view; phone and desktop are drawn
apart and **shown to the owner before shipping**.

### 5.1 Trackers: the interface, and the person's own

The owner (2026-10-22): *"a nice interface to track different things (the user might also want to
add custom ones)."*

**A tracker is a key, and its suffix says what kind it is.** This is the same rule as *the unit in
the key* (2026-10-11), taken one step further. The file stays readable without the app, and no
schema is stored anywhere:

| Kind | Suffix | Control on the Day screen | Wording source |
|---|---|---|---|
| Severity | `_0to3` | four chips: *none · mild · moderate · severe* | IHS, Diener 2019 (source) |
| Rating | `_0to10` | 0–10, numbers only | IHS names the NRS; anchors unread |
| Bother | `_0to100` | a slider from *not at all bothersome* to *extremely bothersome* | Sousa-Pinto 2021, CC BY (source) |
| Yes / no | `_yes` | a switch | — |
| Count | `_doses`, `_count` | − / ＋ | — |
| Amount | a unit: `_min`, `_ml`, `_mg`, `_mmhg`, `_g` … | a number field with its unit | SI; the closed unit list grows only with a source |
| Score | `_score` (e.g. `snot22_score`) | a number field, a range and a link to the holder | §3.4 |

**Adding one.** On the Day screen, the phone's one plus (*the plus means what the view in front
makes*) opens **Track something**: a name, a kind from the table, a unit if it is an amount, and a
section (Headache · Nose and eyes · Medication · Sleep · Yours). It writes the key into the person's
own Health template, which is a note (*a template carries structure*). **Nothing hidden is stored.**
Removing a tracker takes it off the template; past days keep their values. A key is never renamed,
because renaming would orphan the days already written. Only the label changes, and the label is
read from the key (`bedding_washed_yes` → *Bedding washed*).
*Open:* a name with accents or other scripts needs a label that is not derived from the key.

**This is not view authoring.** The 2026-08-31 withdrawal is about saved views. A tracker edits a
template note, which a person can already do by hand. The form keeps its unsaved work on the device
(`decisions.md#ui`, 2026-10-14).

**The Day screen** is one column of sections, every row a label with its control, the same rows
every day. A row left untouched stays blank, which means *not recorded*. On a phone it is one
scroll; on a desktop the same column, wider, with nothing added beside it. The date sits at the top
with ‹ › to step back a day. Touching any control creates the day's note, so there is no separate
"new entry" step.

**Month and Trends show one tracker at a time**, picked from a menu (*one picked thing per
screen*). Month colours its grid by the value (shades for 0–3, a dot for yes/no, the number for a
count) and puts the counts under it. **For the doctor** builds the printable page and its CSV from
the trackers the person ticks. Trends may mark the days of **one** yes/no tracker on the chosen
series (*bedding washed* on *headache*). That is a mark, not a second series; the owner should
confirm it doesn't break the one-thing rule.

### 5.2 Cards, as in Fitness

Same structure as `plans.json` (`decisions.md#ui`, *the card library as built*, 2026-10-16): one
card per question; a summary in the source's own words; `rests_on` shown as a badge (*Guideline*,
*Expert consensus*, *Review*, *One trial*, *Definition, not advice*); sources with URLs; optional
**checks** read from the diary, each with a `basis` quoting its source; and `unmeasured` for what the
diary cannot tell. Opt-in flags (e.g. pregnancy) only show a caution or swap in a body's own number.

**What a health check is, and is not.** As in Fitness, a check shows the person's count or weekly
average **beside the body's number, with its name**, for example *"Medication days this month: 6.
NICE CG150 names 10 (triptans, opioids, combinations) or 15 (paracetamol, aspirin, NSAIDs) over 3
months."* It never uses a diagnosis word, and it never computes a medical target. This replaces the
blanket *no computed flags* of §3.2 with the Fitness pattern. A medicine's class (simple or
triptan, opioid or combination) is the person's own choice on the medication tracker. The app never
infers it from a name.

**A card can bring its trackers.** Following *fewer dust mites in the bedroom* adds
`bedding_washed_yes` to the template if it is missing. Its check then counts washes per week. This
joins the custom trackers and the cards into one model.

**NHS text may be carried** (read at source: [nhs.uk terms](https://www.nhs.uk/our-policies/terms-and-conditions/)
§3.3–3.4). It is released under the Open Government Licence v3.0. **Verbatim** copy is credited
*"Information from the NHS website"*, carries *"licensed under the Open Government Licence v3.0"*,
and is dated *"as at DDMMYY"* (or refreshed every 7 days). **Reworded** copy may not be credited to
the NHS and carries *"Contains public sector information licensed under the Open Government Licence
v3.0."* So NHS lines on a card are quoted word for word and dated. Logos and images are excluded.
That terms page was last reviewed in 2021, with its next review due in 2024.

### 5.3 The cards, read at source (2026-10-22)

Every quote below was read at the source unless marked. Shelves follow Fitness: shelved by kind.
**Bold keys** are the trackers a card brings to the template if they are missing.

**Shelf: Keeping a diary**

1. **Keep a headache diary.** *Guideline.* NICE CG150 1.1.4 (the list in §3.1); 1.3.1 *"as a basis
   for discussion with the person about their headache disorder"*; NHS migraine page: *"It can help
   to keep a migraine diary to help you work out what might trigger your migraine attacks."* (as at
   date). **Check:** days recorded across the last 8 weeks (*"for a minimum of 8 weeks"*). With the
   *Cycle* starter (§5.4), `period_start_yes` adds NICE 1.2.6's *"at least 2 menstrual cycles"*. **Never** points out
   the 1.2.5 menstrual pattern: *"Suspect"* is a clinician's word.
2. **Painkiller days.** *Guideline.* NICE CG150 1.2.7 (quoted in §3.2); NHS: *"Try not to take
   painkillers for more than 2 days a week as this can cause you to get more headaches and make
   migraine harder to treat."* **Checks**, kept apart and never merged: days a month taking
   *triptans, opioids, ergots or combinations* beside **10**; days a month taking *paracetamol,
   aspirin or an NSAID* beside **15**; days a week taking any painkiller beside NHS's **2**. The
   class comes from the person's medication tracker (§5.2). **Never:** "you have medication overuse
   headache" (NICE also requires 3 months and a headache that *"developed or worsened"*), and never
   advice to stop a medicine (1.3.43 applies after a diagnosis).

**Shelf: Nose and allergy**

3. **Fewer dust mites in the bedroom.** *Review: weak evidence*, plus *Expert consensus*. The
   summary is Cochrane's own conclusion ([Sheikh 2010, CD001563](https://pubmed.ncbi.nlm.nih.gov/20614426/),
   9 trials, 501 people): *"acaricides and extensive bedroom-based environmental control programmes
   may be of some benefit in reducing rhinitis symptoms"*, *"Isolated use of house dust mite
   impermeable bedding is unlikely to prove effective"*, and trials *"small and of poor methodological
   quality"*. For asthma, [CD001187](https://pubmed.ncbi.nlm.nih.gov/18425868/): *"cannot be
   recommended"*. Its editorial note (*"should not be used for clinical decision-making"*) is shown
   with it. **Checks:** **`bedding_washed_yes`** at least once a week. Basis: AAAAI, *"Bedding should
   be washed weekly in hot water (130° F)"* ([page](https://www.aaaai.org/tools-for-the-public/conditions-library/allergies/managing-indoor-allergen)).
   **`bedding_wash_c`** ≥ 60. Basis: Choi 2008, *"using the 60 degrees C … all HDMs were killed"*;
   this is a laboratory laundry test, and the card says so. Optional: **`humidity_pct`** < 50, from
   ACAAI, *"less than 50%"*. **Never:** that these measures relieve symptoms; that covers work on
   their own (a CUH NHS leaflet says they are *"the single most effective measure"*, which conflicts
   with Cochrane, so it is shown as a disagreement per *where sources disagree, say whose*); 54 °C
   killing all mites (untested); ACAAI's *"no more than 120°F"*, which contradicts the kill
   temperature.
4. **Rinsing the nose with salt water.** *Review: low or very low certainty.*
   [Cochrane CD012597](https://pubmed.ncbi.nlm.nih.gov/29932206/) (14 studies, 747 people): *"may
   reduce patient-reported disease severity … at up to three months"*, *"no reported adverse
   effects"*; quality *"low or very low"*. Shown with NHS's home recipe verbatim (as at date).
   **Check:** **`nose_rinse_count`** ≤ 3 a day (*"up to 3 times a day (making a new solution each
   time)"*). **Never:** a benefit past 3 months, as an add-on to medicines (*"uncertain"*), or
   compared with steroid sprays.
5. **Hay fever self-care.** *Expert consensus* (NHS patient information; no trials cited). NHS do's
   and don'ts verbatim (as at date). **Check:** a decongestant spray on at most 5 days in a row
   (*"should not be used for longer than 5 days as this can make a blocked nose worse"*). This needs
   a "days in a row" count, a second new aggregation. **Never:** that any single measure is proven.
6. **When to see someone.** *Expert consensus.* NHS's GP lists verbatim, from the allergic rhinitis
   page (*"your symptoms are affecting your sleep and everyday life"* …) and the headache and migraine
   pages (*"you have migraine attacks more than once a week"*), with NHS's 999 list from §3.2 kept
   **at the top**, and ARIA's pharmacist referral list ([Lourenço 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7355936/)).
   **No check.** No source gives a number, and the NHS labels most of these *non-urgent*.

**Shelf: Every day**

7. **Sleep.** *Expert consensus.* AASM/SRS ([Watson 2015](https://www.aasm.org/resources/pdf/adultsleepdurationconsensus.pdf)):
   *"Adults should sleep 7 or more hours per night on a regular basis to promote optimal health"*,
   for ages *"18 to 60"*. CDC's table gives 61–64: 7–9 h and 65+: 7–8 h, offered under an
   age flag. **Check:** nights with **`sleep_min`** ≥ 420, shown as a count. *"On a regular basis"*
   is not turned into "most nights". **Never:** calling more than 9 h harmful (the source says it is
   *"uncertain"*).
8. **Caffeine.** *Review* (EFSA risk assessment, [topic page](https://www.efsa.europa.eu/en/topics/topic/caffeine)):
   *"Intakes up to 400mg per day … do not raise safety concerns for healthy adults … except pregnant
   women"*; 200 mg a day under the pregnancy flag. **Check:** days with **`caffeine_mg`** over 400,
   beside EFSA's line. **Never:** 400 shown as a target; a 200 mg single-dose check worked out from a
   daily total; milligrams worked out from cups. NHS lists *"too much caffeine"* as a migraine
   trigger, quoted, unmeasured.
9. **Water.** *Definition, not advice.* EFSA: adequate total water 2.0 L/day (women) and 2.5 L/day
   (men), counting *"food moisture"* and assuming moderate temperature and activity. **No check.** A
   log of drinks cannot be compared with a total-water figure, so the card shows the definition and
   puts that limit under `unmeasured`.
10. **Blood pressure at home.** *Guideline.* NICE NG136 1.2.7: 2 readings at least 1 minute apart,
    seated, twice daily, for at least 4 days, ideally 7, *"Discard the measurements taken on the first
    day"*; 1.2.8 home average *"135/85 mmHg or higher"*, together with a clinic reading of 140/90 or
    above. **Check:** complete days logged (≥ 4) and the average without day 1, beside 135/85.
    **Never:** a diagnosis (that needs the clinic figure too); clinic thresholds applied to home
    readings; treatment targets for someone not on treatment. *Open:* four readings a day do not fit
    one whole-number key. Either use eight keys (`bp_sys_am1_mmhg` …) or log each reading as its own
    entry. To decide before building.

**Held**

- **Mood (PHQ-9).** May be carried in full (Pfizer: *"no permission is required"*). The manual's bands
  are 5/10/15/20, and item 9 is *"counted whenever it is present"*. **Held for the owner's ruling**:
  a self-harm answer must open urgent-help contacts, and the numbers read (UK 999, Samaritans
  116 123) are not Singapore's. Singapore's have not been read. NICE NG225 1.6.1 also forbids using
  any scale to *"predict future suicide"*.
- **Intermittent or persistent rhinitis.** Held until ARIA's own guideline text is read. The wording
  varies between the secondary sources read.

### 5.4 The menstrual cycle (owner, 2026-10-22: *"For women also the menstrual cycle?"*)

Two more reviews: clinical sources, and apps with their privacy record. Checked as before.

**What the apps and the evidence teach** (source unless marked):
- **Calendar fertility is wrong, and it makes an app a device.** Of 10 apps, 8 % of ovulation
  guesses were exact and 67 % were 2–9 days early; period dates were off by up to 8 days for anyone
  without a 28-day cycle ([Worsfold 2021](https://pmc.ncbi.nlm.nih.gov/articles/PMC8504278/)). Only
  20 of 108 apps met basic accuracy criteria (Moglia 2016, *Obstet Gynecol* 127:1153, abstract read on Europe PMC).
  Natural Cycles is an FDA De Novo contraceptive device and Clue Birth Control went through 510(k)
  (*search*). Apple says its predictions should not be used *"as birth control or to diagnose"*.
- **Cycles vary.** 612,613 cycles: mean 29.3 days (SD 5.2), and **13 %** were 28 days
  ([Bull 2019](https://pmc.ncbi.nlm.nih.gov/articles/PMC6710244/)).
- **Gaps in logging fake long cycles.** About 42 % of Clue users had at least one
  ([Li 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7250828/)). Hormonal and IUD cycles were
  excluded there. Apple pauses its deviation alerts while a *cycle factor* (pregnancy, lactation,
  contraception) is set.
- **Period data has leaked through SDKs.** The [FTC's 2021 order against Flo](https://www.ftc.gov/legal-library/browse/cases-proceedings/192-3133-flo-health-inc)
  found cycle and pregnancy data sent to Facebook, Google and others, against Flo's own promise.
  Privacy International (2019) and Mozilla (2022: 18 of 25 apps warned; Euki, local-only, the one
  "Best Of") found the same pattern (*search*). Since Dobbs (2022) the threat is any **server-held,
  subpoenable record** (*search*).
- **Design.** Epstein 2017 (*search*): users resent misgendering, heteronormative icons and "pink"
  design; Clue's neutral wording worked; some encode their entries to hide them; contraception and
  pregnancy break naive statistics. drip (GPL, local, CSV) and Mensinator (MIT, local) are the
  nearest in spirit.

**Clinical sources** (source unless marked):

| Claim | Source |
|---|---|
| Cycle *"from the first day of bleeding in one cycle to the first day of the next"*; normal *"24 to 38 days"*; bleeding *"up to eight consecutive days"*, spotting included; spread shortest–longest ≤ 9 days at 18–25 and 42–45, ≤ 7 at 26–41; *"below the age of 18 and after 45 … norms are not well defined"*; *"based on the previous 6 months, provided there has been no pregnancy or puerperium"*; volume *"per the patient's determination"*; *"any bleeding between cyclically regular menses is considered abnormal"* | FIGO System 1 as restated by its authors, [Jain, Munro, Critchley 2023](https://pmc.ncbi.nlm.nih.gov/articles/PMC10952771/) (CC BY). Munro 2018 itself is closed: *search* |
| *"every 21 days to every 35 days"*, *"between 2 and 7 days"*; *"Bleeding between periods, bleeding after having sex, or bleeding after the menopause needs to be checked by a doctor"*; *"Using an app or a diary to keep track of your periods can help"*; *"Day 1 … is the first day of your period"* | NHS [Periods](https://www.nhs.uk/conditions/periods/), [Irregular periods](https://www.nhs.uk/symptoms/irregular-periods/). **FIGO's and the NHS's ranges differ, so each is shown with its own name, never blended** |
| HMB *"excessive menstrual blood loss which interferes with a woman's physical, social, emotional and/or material quality of life"*; treatment *"aims to improve this rather than focusing on blood loss"* | NICE [NG88](https://www.nice.org.uk/guidance/ng88/chapter/Context) |
| Signs: pad or tampon *"every 1 to 2 hours"*, *"periods lasting more than 7 days"*, clots *"larger than about 2.5cm"*, *"bleed through to your clothes or bedding"* …; and when to see a GP | NHS [Heavy periods](https://www.nhs.uk/conditions/heavy-periods/) |
| *"keep a diary of your symptoms for at least 2 menstrual cycles – you can take this to a GP appointment"*; *"Call 999 or go to A&E if: you have symptoms of … PMDD and are feeling suicidal"* | NHS [PMS](https://www.nhs.uk/conditions/pre-menstrual-syndrome/) |
| PMDD criterion F: *"confirmed by prospective daily ratings during at least two symptomatic cycles"* | DSM-5-TR, as reproduced in [Naik 2023](https://pmc.ncbi.nlm.nih.gov/articles/PMC10711063/) (the manual itself not read). **DRSP form: licence unknown, so not carried** |
| Pure menstrual migraine: *"day 1 ± 2 (ie, days −2 to +3) … in at least two out of three menstrual cycles"*; *"there is no day 0"*; a diary *"is recommended, but this is not mandatory for clinical diagnosis"* | [ICHD-3 A1.1.1 / A1.1.2](https://ichd-3.org/appendix/a1-migraine/a1-1-migraine-without-aura/a1-1-1-pure-menstrual-migraine-without-aura/) |
| Endometriosis: suspect on period pain affecting daily life, deep pain with sex, cyclical bowel or urinary symptoms …; *"keeping a pain and symptom diary can aid discussions"* | NICE [NG73](https://www.nice.org.uk/guidance/ng73/chapter/Recommendations) 1.3.1, 1.3.3; NHS [Endometriosis](https://www.nhs.uk/conditions/endometriosis/) |
| PBAC blood-loss chart: cut-offs *"ranged from 50 to 185"*; *"lack of standardization precludes widespread use"* | [Magnay 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7011238/). **Not used** |
| Flow vocabulary: HealthKit `HKCategoryValueVaginalBleeding` (iOS 18+, replaces the deprecated `…MenstrualFlow`): unspecified · none · light · medium · heavy; a *cycle start* flag; `intermenstrualBleeding`. Health Connect `MenstruationFlowRecord`: UNKNOWN 0 · LIGHT 1 · MEDIUM 2 · HEAVY 3 (no "none"); `IntermenstrualBleedingRecord` | Apple and Android developer references (raw HealthKit integers: *memory*) |

**The proposal.**
- **A starter, *Cycle*, that a person adds. It is never turned on from their sex.** Its words are
  neutral (*bleeding*, *cycle*), with no theme of its own. Keys, in §5.1's suffixes:
  `period_start_yes` (HealthKit's cycle-start flag); `period_flow_0to3` (0 none · 1 light · 2
  medium · 3 heavy, mapping by name to HealthKit and as 1–3 to Health Connect); `spotting_yes`
  (intermenstrual bleeding); optional `period_pain_0to3`, `clots_large_yes`,
  `products_changed_count`. Every field is independent and optional. Sex, contraception, temperature
  and cervical mucus are **not** in the starter, so the tracker never assumes someone is trying to
  conceive. A person can add them as their own trackers.
- **A *cycle context*:** a text property set on the day it begins (contraception, pregnancy,
  postpartum, perimenopause, in the person's own words) and cleared on the day it ends. **Cycle
  statistics leave out any cycle that overlaps it**, as FIGO, Apple and Li do.
- **Only arithmetic on recorded first days.** *"Last 6 cycles: 27, 31, 29 …; shortest 27, longest
  31."* **No prediction, no fertile window, no ovulation, no pregnancy inference** from a late
  period. A next-period *range* is left for the owner to decide later (§7).
- **A gap is not a long cycle.** Each cycle shows how many days in it have a note. The person can
  mark a cycle *missed logging* (`cycle_gap_yes` on any day in it) to leave it out.
- **The headache cards join up:** card 1 reads `period_start_yes` for NICE's *"at least 2 menstrual
  cycles"*, and card 14 below counts migraine days around it.

**Cards** (shelf: *Cycle*):

11. **Your cycle.** *Definition, not advice.* FIGO's and the NHS's ranges, each named. **Checks:**
    cycle lengths over the last 6 months, beside FIGO 24–38 **and** NHS 21–35 as two separate lines;
    bleeding days in a row, beside FIGO 8 and NHS 7; the shortest-to-longest spread, beside FIGO's 7
    or 9, **only** if the person has entered an age between 18 and 45; spotting days, with the NHS's
    *"needs to be checked by a doctor"*. **Never** the word "irregular" on its own, and never
    "outside the normal range" without the body's name.
12. **Heavy periods.** *Guideline.* NICE's quality-of-life definition and the NHS signs, quoted.
    **Checks:** days at flow 3; bleeding days beside NHS 7 and FIGO 8; optional large-clot days.
    **Never** a diagnosis, a PBAC score, or millilitres worked out from anything.
13. **Premenstrual symptoms.** *Expert consensus* (NHS). The diary is the point: *"at least 2
    menstrual cycles"*. **Check:** complete cycles with daily entries for the person's chosen
    symptoms. **Never** the words "PMDD", "meets criteria" or "likely". The NHS urgent-help line is
    always shown: **held for local numbers**, as for PHQ-9 (§7).
14. **Periods and migraine.** *Definition, not advice* (ICHD-3 A1.1). **Check:** *"headache logged on
    days −2 to +3 in X of your last 3 cycles"* (no day 0), and whether headaches also came at other
    times. **Never** names a migraine type: the diary cannot establish ICHD-3's 1.1 criteria.

Card 6, *When to see someone*, gains the NHS's period, heavy-period and endometriosis GP lists,
quoted.

**Privacy, raised again for the owner: cycle data is a different risk.** formicaria has no vendor,
no account and no SDK. That is the strongest position any of the reviewed apps could reach. But **a
vault pushed to a git remote is a server-held record**, and git history makes deletion hard. The
owner chose *any vault, no steering* (§6). That choice stands unless the owner changes it. Proposed
for this starter only, as information rather than steering: one line on the Cycle starter saying
backups and remotes are copies, and a manual section on keeping cycle notes out of a shared remote
and removing them from history.

**Engine, again:** "days between successive first days", and a count over the days around a marked
day (−2 to +3). Both are pure and inside seam 1.

**Shelf: Yours**, as in Food and Fitness: a card the person writes, with their own trackers and no
source required.

**Engine, revised:** two pure `fm-query` aggregations. The first counts days where a key is above 0
(Month, and checks 1–3, 7, 8, 10). The second finds the longest run of days in a row (check 5). Both
stay inside seam 1.

## 6. The owner's answers (2026-10-22)

Asked as four questions, each with a recommendation:

- **Scope: a general symptom diary.** *(Recommended was headache + allergy.)* That is why §5 ships
  starters, not a fixed list.
- **Entry shape: one note per day.** This matches IHS, NICE and the ICHD-3 headache-day.
- **Shown by default, like Food and Fitness**, with the Settings switch kept. *(Recommended was off by
  default.)* This must be checked against *a Home a newcomer starts on* (`decisions.md#ui`,
  2026-10-18): a first launch will list five groups.
- **Any vault, no steering.** *(Recommended was steering to a private vault.)* The one line in the
  manual and on the group that states where the notes travel is information, not steering. It is
  proposed for the decision entry, and the owner can strike it.

## 7. Open, still to read before anything it supports is built

- **The Cards tab's name.** Food says *Diet*, Fitness says *Regime*; Health needs one word, the same
  on every screen. The owner's to choose.
- **Labels for non-ASCII tracker names** (§5.1).

- ARIA's own wording of *persistent* (whiar.org unreachable): until read, "persistent" is not used.
- Pfaar 2014's symptom-medication score (closed access): not used.
- NRS 0–10 anchors (IMMPACT, closed access): the 0–10 option shows numbers only.
- ~~The NHS website's reuse terms~~: **read** (§5.2). NHS text is OGL v3.0: quoted verbatim with
  the date it was taken from the site, or reworded and not credited to the NHS.
- The nhs.uk page on dust-mite prevention no longer exists (it redirects to *Allergies*), so card 3
  uses AAAAI, ACAAI and Cochrane instead.
- Singapore urgent-help numbers, before PHQ-9 or card 13 can ship.
- **Cycle:** whether to show a next-period *range* (min–max of recent lengths) or nothing; whether
  the Cycle starter carries the privacy line and the manual section (§5.4); the ACOG 651 adolescent
  ranges and the DRSP licence (both unread); HealthKit raw values (before any export code).
- Epworth's licence with Mapi (PROinformation@mapi-trust.org) and MIDAS's terms: until read, score
  + link only. That would be the rule even if the terms allowed more.

## 8. Order — taken by the owner, 2026-10-22

The owner: *"go with your recommendations and write the decision entry and start coding."* Taken
recommendations:
- the cards' tab is **Guidance**;
- Over time may mark one yes/no tracker;
- PHQ-9 and the premenstrual card are held;
- no next-period range;
- the privacy line goes on the Cycle starter only.

**Views are renamed Diary, Month, Over time and Guidance** wherever this plan says Day and Trends,
because a view's name is unique across groups and Food already has *Day* and *Trends*.

Progress:
- ✅ Decision entry: `decisions.md`, 2026-10-22, *a fifth group, Health*.
- ✅ Day arithmetic: `fm_query::days` (`count`, `longest_run`, `per_week`, `intervals`,
  `ichd_menstrual_window`, `in_window`, `outside_windows`), 7 tests.
- ✅ Trackers: `fm_app::health` (`parse_key`, `label_of`, `trackers` from Health templates, `series`
  of recorded days, with a peak or a sum when a day has two notes), 6 tests.
- ✅ Wiring: `AddAction::Health`, `Span::{Diary, Month}`, a Health view needing no `metrics:`,
  `shipped/health-{diary,month}.view`, dispatch arms `health_trackers` and `health_days` (it
  returns `template`, `trackers`, every day's `values`/`id`/`late`, and `counts`), the command
  reference, and the mock's twin in `mockTracker.ts`.
- ✅ Screens (desktop first; one column, so it reads on a phone too, but the phone layout is not
  designed yet): `ui/src/lib/health/HealthScreen.svelte` (Diary: ‹ day ›, one row per tracker;
  Month: one tracker picked, a grid shaded by value, dashed where not recorded, a dot where written
  in late, and the counts), `TrackSheet.svelte` (the plus: two starters, or a name, a kind and a
  unit), `starters.ts` (the starters and the scale words, each with its source). 4 component tests.
- ✅ **For the doctor** (`DoctorPage.svelte`, from Month): *This month* or *Last 8 weeks* (NICE's
  least). The counts come first, then a row a day. The person ticks which trackers go on it. A
  blank means not recorded, `*` means written in later, and it says *"a record, not a diagnosis"*.
  **Print** prints that page alone (a print rule keyed to a body class only while it is open).
  **Save as CSV** writes the same numbers, one row a day. 1 test, including the CSV.
- ✅ **Over time** (`shipped/health-over-time.view`, span `range`): one tracker, a week a bar over 4
  weeks, 12 weeks, 6 months or a year. A severity or a yes/no shows the days in the week; a count
  shows the times in the week; anything else shows the average of the days recorded. Each bar
  says how many days were recorded, and a week with none says *nothing recorded*. One yes/no
  tracker may be marked under the bars. `health_days` gained `weeks`, from `fm_query::days::per_week`,
  whose `DayCount` gained `sum`. 1 test.
- ⏳ **The owner looks**: `pixi run bash -c 'pnpm -C ui dev'` on the mock writes nothing, while
  `pixi run serve` creates a `vault/` folder in the repo. The screens are under Health → Diary,
  Month and Over time.
- **Unverified on a phone:** whether Android's WebView prints at all (it may ignore
  `window.print()`) and whether a CSV download lands anywhere. The phone may need the share sheet
  for both.

Open, found while building:
- A Health template is a template, so it is offered wherever templates are offered. Making a
  note from it copies tracker keys at 0 into an ordinary note: harmless, but untidy.
- Pressing a day in Month does not open it in Diary yet.
- Trackers are listed in key order. Sections (Headache · Nose and eyes · …) are not built yet.
- The first launch now lists five groups: still to check against *a Home a newcomer starts on*.
- ✅ **Guidance** (`shipped/health-cards.json`, `guidance.rs`, `GuidanceScreen.svelte`,
  `shipped/health-guidance.view`): ten cards from §5.3 on three shelves. Following one writes a
  `program: health` note; the followed checks are counted on today beside each body's number.
  Painkiller days asks which trackers are which class. 6 Rust tests (every card parses, every check
  is a known kind with a basis, no check's label names a condition, counting, not chosen, stopped)
  and 1 screen test. As built: `decisions.md`, 2026-10-22 (later). The mock carries three cards
  (`health/mockGuidance.ts`).
- ✅ **The Cycle** (`cycle.rs`, 4 tests; `guidance.rs` measures six new check kinds, 1 more test):
  - The *Cycle* starter (`period_start_yes`, `period_flow_0to3` in HealthKit's words none · light ·
    medium · heavy, `spotting_yes`, `period_pain_0to3`) says before it is added that backups and
    remotes are copies, with a link to the manual's privacy section.
  - The Diary has a **Cycle context** row (`cycle_context`: the person's words on the day a context
    begins, *none* where it ends).
  - Cycles that end in the last six months count; one overlapping a context, or marked *missed
    logging* (`cycle_gap_yes` on its first day, from *Your cycle*), is left out and says why.
  - Cards: *Your cycle* (lengths beside FIGO 24–38 **and**, apart, NHS 21–35; bleeding days beside
    FIGO 8 and NHS 7; the spread beside FIGO's 9/7/9, only with a year of birth from the profile;
    spotting days), *Heavy periods* (heavy days; bleeding days beside NHS 7) and *Periods and
    migraine* (of the last 3 periods, how many had a headache on days −2 to +3, and the headache
    days at other times).
  - Nothing is predicted. The screen test checks that no word like irregular, fertile, ovulation or
    predict appears.
- ✅ **The manual**: `docs/src/user/health.md` (in SUMMARY after *Food and training*), with *Keeping
  cycle notes to yourself*, *What doctors use these numbers for* (the quoted definitions, which the
  app never applies) and *When to get help now*. `views.md` now has five groups (its anchor
  `#the-groups`, and `trackers.md`'s link to it followed). There are *How this works* links on Diary
  and Guidance.
- ✅ **Yours** (`guidance.rs` `own_card`, 1 test; *Write your own* in Guidance, 1 screen test): a
  following note with `plan: own`, the person's own title, words and one check.
- ✅ **Lean: three tabs** (owner, *"only the necessary ones … to give a view and to insert"*):
  **Diary** to write in, **Over time** to look back (Month | Weeks, *For the doctor* at its foot),
  **Guidance**. `health-month.view` and `Span::Month` are removed. `decisions.md`, 2026-10-22
  (evening).
- ✅ **Released in 0.12.0** (2026-10-22), with the phone layout by width and *Keep as a note* on
  the doctor's page.
- ✅ **Sections in Diary**: Headache · Nose and eyes · Cycle · Medication · Every day · Yours,
  read off the key (`starters.ts` `sectionOf`): a starter's keys are its own, a card's trackers sit
  on its shelf, `_doses` is Medication (a new *Doses of a medicine* kind), and anything else is
  Yours. Nothing is stored. The pickers in Over time and the doctor's page follow the same order.
- ✅ **A day pressed in the month opens in Diary**, through `TrackerPane`'s `openDay` (Food's own
  path, now also Health's). Back returns to the month; a day to come cannot be pressed.
- Next: blood pressure's four readings a day; Singapore crisis numbers before PHQ-9 or the
  premenstrual card; tried on a phone.

The original order, for reference:

1. Decision entry: a fifth group; `log: health`; a general diary on starter templates; shown by
   default; no computed flags; scores only as typed totals.
2. Register: the *source* rows of §3 become `sources-2026-10-11.md` §10.
3. Engine: day-count aggregation in `fm-query`, with tests.
4. Day + Month on the desktop → owner looks → the phone layout → owner looks.
5. Trends and the manual page.
6. Cards 1–10 in `plans.json` with their checks, then *Yours*; then the Cycle starter and cards
   11, 12 and 14. PHQ-9 and the rhinitis definition
   follow only once §7 is settled.

## 9. Broader: the record on a Library shelf, the diary widened, and what fm already knows (2026-10-22, late)

> **A proposal, researched, not built.** The owner: *"it needs to be broader and talk with the other
> information in fm (everything stays a note, at most structured after all). What would a person
> need to track in this sort of view is the first question"*, then *"run the research round,
> Library shelf for the record"*. Two reviews, read at source unless marked.

### 9.1 What a health record holds

**HL7 FHIR International Patient Summary**, v2.0.1 STU 2
([structure](https://hl7.org/fhir/uv/ips/en/Structure-of-the-International-Patient-Summary.html)):
*"(R) denotes a required section … (S) denotes a recommended section; others are optional."*

| Tier | Sections |
|---|---|
| Required | Problem List, Allergies and Intolerances, Medication Summary |
| Recommended | Immunizations, Diagnostic Results, History of Procedures, Medical Devices |
| Optional | Vital Signs, Past Problems, Pregnancy, Social History, Plan of Care, Functional Status, Advance Directives, Alerts, Patient Story |

An empty required section is stated, not left blank (*"emptyReason"*). A medicines list may say
*"known not to be under any relevant medication"*.

Patient-facing records group the same things:
- **Apple HealthKit:** allergy, condition, immunization, lab result, medication, procedure and vital
  sign records.
- **Android Health Connect:** the same plus visits, practitioners and pregnancy.
- **The NHS App:** prescriptions, appointments, test results, vaccinations, conditions and
  allergies, documents.

**What patients are told to keep and bring:**
- **FDA's *My Medicine Record*** (form FDA 3664, [PDF](https://www.fda.gov/media/73856/download)):
  what, what it looks like, how much, how and when, start and stop, why, who told me to; allergies
  to *"any medicine, dietary supplement, food, skin cleaner, medical tape"*; *"My Medical Conditions
  and Operations"*; *"These are my medicines as of [date]"*.
- **NHS, *What to ask your doctor*:** *"List or bring all your medicines and pills – including
  vitamins and supplements. Write down details of your symptoms"*; afterwards *"Write down what you
  discussed and what happens next. Keep your notes."*
- **AHRQ:** *"Bring all the medicines you take … Know your current medical conditions, past
  surgeries, and illnesses."*

**What people use and drop** (PubMed abstracts):
- Ancker 2015 ([PMID 26290186](https://pubmed.ncbi.nlm.nih.gov/26290186/)): *"tracking this data
  feels like work"*; *"physicians trust technologically measured data such as lab reports over
  patients' self-tracked data"*.
- Krebs & Duncan 2015 ([PMID 26537656](https://pubmed.ncbi.nlm.nih.gov/26537656/)): 45.7 % stopped
  using some health apps, *"primarily due to high data entry burden"*.
- Clinicians judge self-tracked data by *"accuracy and reliability, completeness, context"*
  ([West 2017, PMID 29209601](https://pubmed.ncbi.nlm.nih.gov/29209601/)).

**Results belong to their lab** ([MedlinePlus](https://medlineplus.gov/lab-tests/how-to-understand-your-lab-results/)):
*"labs often use different testing methods, they often use different reference ranges … You can't
compare your results to reference ranges that you might find elsewhere"*, and *"It's common for
healthy people to sometimes have results outside the reference range."* Apps interpreting numbers
cause *"anxiety, confusion"* ([van der Mee 2024, PMID 39133906](https://pubmed.ncbi.nlm.nih.gov/39133906/)).

### 9.2 The record: notes on the Library shelf `health`

**Everything is an ordinary note, made from a template, shelved `shelf: health`**, linked with
`[[links]]`. Few fields are required (a name, a status, a date) and the rest are optional, because
data entry is what makes people stop. Keys follow the diary's rules: whole numbers, unit in the key.

| Note (template) | Fields (IPS name) |
|---|---|
| **Medicine** | name (medication), strength and form, `dose_text` (dosage.text), schedule or `as_needed` (dosage.timing), `start`/`due` as start and stop (effective), `status`: current · stopped · on hold (status), reason → [[condition]], prescriber → [[contact]], `looks_like` (FDA) |
| **Allergy or intolerance** | substance (code), `type`: allergy · intolerance, reaction, severity, onset, `status`: active · inactive · resolved; food, tape and skin products too (FDA) |
| **Condition** | name (code), `status`: active · remission · resolved (clinicalStatus), onset, resolved, severity |
| **Result** | test (code), value with its unit in the key (`ige_dpt_kua_per_l_x100: 1320` for 13.20), **the lab's own** `ref_low`/`ref_high`/`ref_text` and `lab_flag` as printed, date, lab → [[contact]], the report attached (PDF or photo), status: preliminary · final · amended |
| **Visit** | date (on the Agenda), with → [[contact]], questions to ask, what was said, next steps |
| **Immunization**, **procedure**, **device** | name, date, status (and dose number, body site, since) |
| **Contact** | name, role (GP, allergist, lab, pharmacy), phone |
| **None known** | a note per required section saying so, so *none* is told apart from *not recorded* (IPS emptyReason) |

**Rules carried from the research:**
1. **The lab's range, never the app's.** A result shows its value beside the lab's own printed range
   and flag. Nothing is coloured *normal/abnormal*, explained or compared across labs, and the
   assistant is not asked to read it.
2. **A different unit is a different key.** Nothing is converted silently.
3. **A medicine stopped is kept**, with status *stopped* and a last day: archived, not deleted (Apple,
   FDA).
4. **Doses stay in the diary.** The diary's `<medicine>_doses` tracker and the medicine note link to
   each other; the note holds the plan and the diary holds what was taken.

**What it connects to in fm:**
- **The Agenda:** visits are dated notes; a medicine's `start`/`due` place it.
- **Reminders:** a dose time is a note's own date, said at its time (`reminders.rs`).
- **Meetings from calendar and mail:** a clinic appointment already becomes a meeting note; *Make it
  a visit* adds the shelf and the template's fields.
- **Image → text:** a photographed lab report is read by the same OCR as a food label, and the
  person checks every figure. Nothing is guessed.
- **For the doctor:** the page opens with the record, *"as of"* today. It lists current medicines
  (including over-the-counter medicines and supplements), allergies, active conditions and past
  operations, then the diary's counts. This is the one-page summary FDA, NHS and AHRQ ask a person
  to bring.

### 9.3 Read, not typed twice: what fm already logs

| Value | Where fm has it | Used by |
|---|---|---|
| Water | Food's log, `water_mg`: the water in foods **and** drinks, which is EFSA's *total* water | the *Water* card can now count (it could not from drinks alone) |
| Caffeine | Food's log, `caffeine_ug` (USDA foods; Ciqual and BLS do not carry it, so it is partial) | the *Caffeine* card, with the gap said |
| Alcohol | Food's log, `alcohol_mg` | a trigger row, or a card when one is sourced |
| Weight, steps | the body log (`weight_g`, `steps`) | Over time, the doctor's page |
| Activity | Fitness sessions | Over time |
| Period days | the Cycle starter | Food and Fitness flags, later |

In the Diary these appear as read-only rows that name where they come from, and open there.

### 9.4 The diary widened: what to add, from the research

| Diary | Holder | Ship as | Notes |
|---|---|---|---|
| **Home blood pressure** | NICE NG136; BIHS and NHS Lothian diaries | **starter** | 4 readings a day: `bp_am1_sys_mmhg`, `bp_am1_dia_mmhg` … `pm2`, over 7 days, the first discarded (NICE 1.2.7). The card quotes nhs.uk: *"135/85 or higher when checked at home"*. A target only if the person types the one they were given (BIHS has a *"Target BP (if advised)"* field) |
| **Peak flow** | Asthma + Lung UK; NHS | **starter** | `peak_flow_am_lmin`, `peak_flow_pm_lmin`, each *"the highest of"* 3 blows; the person's own best. **No zones**. ACT needs a QualityMetric licence and fee (**hold**); GINA's questions need copyright approval (**link only**) |
| **Blood glucose** | NICE NG17, NG28 | **kinds only** | Readings named by the person (before breakfast …), in tenths of mmol/L. **No targets**: NICE: *"Agree an individualised HbA1c target with each adult"* |
| **Bladder diary** | NICE NG123: *"a minimum of 3 days"* | **starter** | Voids (ml), urgency 0–4 in our own words (ABUHB's wording needs permission), leaks, pads, drinks (ml) |
| **Bowel** | Bristol Stool Form Scale, © Norgine | **kinds only** | A type 1–7 number. Norgine's descriptions and pictures are not carried |
| **Temperature** | NHS: *"A high temperature is usually considered to be 38C or above."* | **kind** | Tenths of °C. The quote may sit beside it, attributed |
| **Pain** | British Pain Society (digitising needs permission); BPI (licence) | **kind only** | 0–10 in our own words |
| **Eczema (POEM)** | University of Nottingham: free for non-commercial use | **link only** | formicaria is MIT, so anyone may use it commercially, and carrying POEM would pass on a right we do not have |
| **Wellbeing (WHO-5)** | WHO, CC BY-NC-SA 3.0 IGO | **link only** | Same reason: non-commercial and share-alike, inside an MIT app |
| **Mood (PHQ-9)** | Pfizer: *"No permission is required"* | **owner's choice** | Item 9 is about self-harm. Singapore contacts read at source: **995** ([SCDF](https://www.scdf.gov.sg/)), **mindline 1771**, 24 hours, WhatsApp +65 6669 1771 ([mindline.sg](https://www.mindline.sg/)). SOS 1767 and the IMH helpline were not reachable to read. Either PHQ-9 with these always shown beside it, or PHQ-8, which leaves item 9 out (unread at source) |

**Unread at source:** ACT and GINA terms; the ICIQ bladder diary terms; Norgine's own page; the
NHS and NICE reuse terms for the diary layouts; SOS and IMH; peak-flow units (L/min, from memory).

### 9.5 Order, proposed

1. **The record**:
   - the templates and the `shelf: health` shelf;
   - result notes with the lab's own range and an attached report;
   - medicine notes linked to the diary's doses;
   - *none known* notes.
2. **The doctor's page opens with the record**, as of today.
3. **Read, not typed**: water, caffeine and alcohol from Food; weight and steps from the body log.
   The *Water* card gains its check.
4. **The diary widened**: the blood-pressure starter (which settles §5.3 card 10), peak flow, the
   bladder diary, and the temperature, glucose, bowel and pain kinds.
5. **Visits** from calendar meetings; a photographed report read by the OCR.
6. **PHQ-9 or PHQ-8**, once the owner chooses.

**Built 2026-10-22 (night)**, per `decisions.md` of that night:
- the record (`record.rs`, `health_record`; *Start a health record*; the record first on the doctor's
  page);
- values read from Food and the body log (`health::elsewhere`); *Water* counted (`food_mean`);
- the *Blood pressure* (`bp_home`) and *Bladder* starters; tenths units;
- PHQ-9 and *Premenstrual symptoms*, with Singapore's contacts.

Held: peak flow, ACT, GINA, POEM, WHO-5; visits from meetings and the OCR for reports are next.


## 10. Mind: diaries for mental health (2026-10-22, research; a proposal, not built)

> The owner: *"What about mental health and diaries, nothing in that department?"*, then *"requires
> a research … grounded on reputable sources and research"*. Two reviews, read at source unless
> marked.

### 10.1 What the evidence says (PubMed abstracts and regulators' own pages)

| Practice | Evidence | Checked |
|---|---|---|
| **Activity planning (behavioural activation)** | Cuijpers 2026, 105 trials ([PMID 42492146](https://pubmed.ncbi.nlm.nih.gov/42492146/)): SMD 0.67 against control; *"Self-guided BA was also effective (k = 15; SMD = 0.36)"*. COBRA ([Lancet 2016, PMID 27461440](https://pubmed.ncbi.nlm.nih.gov/27461440/)): delivered by junior workers, *"non-inferior to CBT"* | source |
| **Thought records** | Ciharova 2021 ([PMID 34264703](https://pubmed.ncbi.nlm.nih.gov/34264703/)): *"no evidence of a difference"* between challenging thoughts, activity planning and full CBT | source |
| **Guided against unguided** | Karyotaki 2021 ([PMID 33471111](https://pubmed.ncbi.nlm.nih.gov/33471111/), n = 9,751): guided is better right after treatment, with no difference at 6–12 months; unguided works about as well for mild symptoms. Unguided self-help left fewer people worse than control: 5.8 % against 9.1 % ([PMID 29540243](https://pubmed.ncbi.nlm.nih.gov/29540243/)) | source |
| **Mood tracking on its own** | Kauer 2012 ([PMID 22732135](https://pubmed.ncbi.nlm.nih.gov/22732135/)): raises emotional self-awareness, with no direct effect on depression. MONARCA I ([PMID 26220802](https://pubmed.ncbi.nlm.nih.gov/26220802/)): *"no significant effects"*, and a *"tendency towards more sustained depressive symptoms in the intervention group"*. MONARCA II: no effect on symptoms, with higher quality of life | source |
| **Expressive writing** | Frattaroli 2006, 146 trials ([PMID 17073523](https://pubmed.ncbi.nlm.nih.gov/17073523/)): *"average r-effect size of .075"*. Guo 2023 ([PMID 36536513](https://pubmed.ncbi.nlm.nih.gov/36536513/)): the benefit *"emerged after a delay"*, and was larger with days between sessions. People often feel worse right after writing (Pennebaker & Beall 1986, *search*) | source / search |
| **Engagement** | Baumel 2019 ([PMID 31573916](https://pubmed.ncbi.nlm.nih.gov/31573916/)): 3.3 % still use a mental-health app at 30 days | source |
| **Privacy** | Huckvale 2019: 81 % of depression apps sent data to third parties. FTC against [BetterHelp](https://www.ftc.gov/news-events/news/press-releases/2023/03/ftc-ban-betterhelp-revealing-consumers-data-including-sensitive-mental-health-information-facebook-others-targeted) (2023: health questionnaires sent to Facebook and others) and [Cerebral](https://www.ftc.gov/news-events/news/press-releases/2024/04/proposed-ftc-order-will-prohibit-telehealth-firm-cerebral-using-or-disclosing-sensitive-data) (2024: data on about 3.2M people). Mozilla 2023: 19 of 32 apps carried its privacy warning | source |
| **Regulation** | MHRA, digital mental-health tools (Feb 2025): a tool that *"tracks 'anxiety levels' over time"* has a medical purpose. Storing, showing and an easily checked calculation are *low functionality*, which is **not a regulated device**. Processing the entries with **AI is high functionality, which is a device**. APA's app model asks how a tool responds to safety concerns and who it shares data with | source |

### 10.2 What may be carried (licences read at source)

- **GAD-7**: carried word for word, as Pfizer allows (*"no permission is required to reproduce,
  translate, display or distribute"*). It has 7 items over 2 weeks, scored 0–21, with the manual's
  *"Scores of 5, 10, and 15 represent cutpoints for mild, moderate, and severe anxiety"*. The 0–4
  band is left unnamed, and the manual's yellow-flag and red-flag wording is not used.
- **No other template may be carried**, so each is **our own wording, citing its source**:
  - Every Mind Matters, though on nhs.uk, has its own terms: *"You must not use any part of the
    content on our site for commercial purposes without obtaining a licence"*. The NHS website's open
    licence does not cover it.
  - NHS trusts' guides, Get Self Help and Psychology Tools all forbid reposting or commercial use.
  - The Consensus Sleep Diary is *"free for not-for-profit use"*.
  - formicaria is MIT, so its users may reuse it commercially, and those terms would not pass on.
- **NICE: a finding that reaches back to 0.12 and 0.13.** NICE's terms (18.1–18.2) allow reuse in
  the UK under its UK Open Content Licence. Outside the UK, *"Except for personal use, study or
  personal research"*, use needs an agreement and *"may be subject to a licence and fee"*. The
  shipped cards quote CG150, NG136, NG88, NG73, NG123 and NG225 word for word. Short quotations may
  fall under copyright's quotation exceptions, but by this project's own rule that is a gap. **For
  the owner:** paraphrase NICE in our own words with the recommendation number and a link
  (recommended), or ask NICE.

### 10.3 Proposed: a *Mind* section of the diary, and *Mind* notes on the shelf

**Framed as a private wellbeing notebook, never as monitoring a condition** (MHRA): no *"track
your depression"* and no *"prevents"*.

| Piece | Built as | Rests on |
|---|---|---|
| **Mood today** | a 0–10 tracker in our own words, unanchored (no free, validated single item was found) | NG222 1.4.11, self-monitoring (cited, not quoted) |
| **Activity and mood** (behavioural activation) | a *Mind* starter: *planned something*, *did it*, *pleasure 0–10*, *achievement 0–10*. Our own wording; a day's activities are lines in the day note's text | Cuijpers 2026, COBRA; NG222's BA *"link between an individual's activities and their mood"* (cited) |
| **Thought record** | a template note on the record shelf (`shelf: mind`): what happened · the thought · how it felt and how strong (0–100) · what else could be true · how it feels now. Our own wording; the columns are common CBT practice and are cited to NICE and Ciharova, not copied from any sheet | Ciharova 2021; NG222 Table 1 (cited) |
| **Worry time** | a template note: the worries · which can be acted on · the plan. Our own wording | CG113 self-help (cited) |
| **Sleep** | a starter: in bed, minutes to fall asleep, times awake, minutes awake, up, quality 0–10. Our own wording | Carney 2012 (cited, not copied, and not given its name) |
| **Writing** | a template note for expressive writing, saying beforehand that people often feel worse right after writing and any good comes later, and suggesting days between sessions. The Timeline is already the journal | Frattaroli 2006; Guo 2023 |
| **GAD-7** | a card beside PHQ-9, carried word for word | Pfizer |

**Safety and privacy, from the research:**
1. **Contacts on every Mind screen**, in the same place, all the time: 995 and mindline 1771. They
   are not triggered by a score, and no risk is predicted (NG225 1.6.1).
2. **Never score, classify or flag a journal or a thought record.** Charts show only the person's
   own numbers.
3. **Myrme stays out of Mind notes** unless the person asks of one note. AI over these entries is
   what MHRA counts as high functionality. **This needs a Myrme-side rule**, an exclusion by
   `shelf: mind`, which goes through Myrme's contract.
4. **A line on the Mind starter**: *"if keeping this makes you feel worse, stop or take a break"*
   (the MONARCA I trend). No streaks and no reminders unless asked.
5. **Privacy**, as for the cycle: backups and remotes are copies, and a vault of its own is the
   simplest way to keep these notes private.
6. **When low mood lasts, see someone.** Guided help beats unguided for moderate and severe
   symptoms. Said once, in our own words, citing Karyotaki 2021.

**Held:** a panic diary (no open template, and no source for its fields); a bipolar mood chart
(−3 to +3, a signed scale this app does not have yet; the NIMH-LCM's terms are unknown; CG185 is
cited only).

**Built 2026-10-22 (late night)**, per `decisions.md` of that night: §10's Mind (starters, Mind
notes, the three cards with GAD-7), contacts on every Mind screen, NICE paraphrased on every card
and in the manual (with a test), and the manual's *everything stays local*.

