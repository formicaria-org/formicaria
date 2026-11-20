# Open tools and science for keeping in shape and training — research (2026-11-20)

> **Research, not rulings.** Nothing here is built, installed or decided. It widens
> `training-coach-plan-2026-10-30.md`, which already covers the watch → Gadgetbridge (GB) → fm path
> (P1–P8) and, in its Part 2, the training-load methods (TRIMP, session RPE, the 42/7 chart, ACWR,
> critical speed, GAP, HRmax equations, the 10 % rule, taper, HRV-guided training). Those are not
> repeated here. Where this document and the code disagree, the code wins; where it and a dated
> decision disagree, the decision wins until the owner writes a new one.
>
> **Two parts.** Part 1 (§1–§11): keeping in shape, the tools, the watch data, AI coaching.
> Part 2 (§12–§19): training athletes for performance — endurance, strength and power, team and
> hybrid sport, nutrition and recovery, monitoring and models, and putting workouts and a calendar
> on the watch. Part 2 was added the same day at the owner's request: *"beyond in shape it is also
> about training beyond just staying healthy, like for athletes"*.
> Part 3 (§20–§27), the same day: cycling, swimming and triathlon, the gym, and nutrition coupled
> with training — the owner asked for *"not only running, maybe including cycling and swimming"*,
> *"also add fitness in gyms"* and for training *"coupled with nutrition (at the coaching level)"*.

**The question.** Watch data will reach fm. What should fm *do* with it — not only track, but help a
person keep in shape and train — using open tools and established, published science? The owner
asked for the survey to be as broad as possible and for research only, no code.

**How it was done.** Five research passes on 2026-11-20, each on one slice: (1) open apps and
platforms, (2) open libraries, datasets and learned models, (3) the wider training and health
science, (4) AI-assisted coaching, (5) what wrist data can be trusted for, and open data standards.
Repositories, licences and papers were read at source where marked. The passes used web fetches
that sometimes return a summary rather than the raw page; where a figure matters, re-read it before
it enters the register.

**Labels** (as in the coach plan). **VC** = code, licence file or repo metadata read. **VPD** =
primary paper or document read (abstract counts and is said so). **SH** = second-hand (search
summary, review site, another project's description). **UV** = unverified. Nothing marked SH or UV
may ship as a number, a definition or a claim (`decisions.md:9563-9566`); it is a lead to read.

**Device.** The data source is a Garmin running watch that GB supports, for example the Forerunner
165 class the coach plan already names. Wrist optical HR, GPS, barometer, accelerometer; no chest
strap assumed.

---

## 1. The short answer

1. **Most of the value is plain code over published formulas, not learned models.** GB hands over
   what the watch has already processed: 1 Hz HR, speed, GPS, altitude, cadence, and daily
   summaries (resting HR, a nightly RMSSD average, sleep stages, steps). It does not hand over raw
   PPG or raw accelerometer data (SH/UV for this export path). Almost every open learned model in
   this field needs those raw signals. One fitted model fits fm's inputs (§5.3).
2. **Keep three layers apart in every note and view:** *measured* (GPS distance, HR, steps,
   cadence, time), *the watch's estimate* (VO2max, sleep stages, stress, Body Battery, training
   effect, recovery time), and *fm's own* computation (with its method and source named). Vendor
   scores are proprietary Firstbeat/Garmin models; most are unvalidated or poorly validated (§2).
3. **The open science gives a clear, modest canon** for ordinary people (§3): weekly minutes by the
   WHO/UK/US guidelines, steps as a plain dial, strength on two or more days, balance for older
   adults, fitness as a vital sign to estimate and trend, mostly easy effort with some hard
   intervals, gradual progress, sleep of seven hours or more at regular times, and self-monitoring
   with goals and feedback. Much that apps sell is contested or hype (§3.3).
4. **No open tool closes the loop locally.** Plan → workout on the watch → FIT back → match to the
   plan → adjust, with no Garmin Connect and no intervals.icu, exists nowhere in the open (§4.3).
   That gap is fm's opening.
5. **The working coach pattern is "code decides, assistant explains".** Every coach whose method is
   documented (Runna, Garmin's plans, HRV4Training, the open Domestique) uses written plan templates
   plus deterministic adjustment rules; LLMs sit on top to explain. Research shows LLM arithmetic
   is unreliable (22 % vs 84 % with code, PHIA) and LLM-written plans raise load too fast. This
   matches fm's existing split: Rust computes, Myrme explains and proposes (§6).
6. **Licences sort the field.** Reusable in MIT fm: Apache-2.0 / MIT / BSD tools (Domestique,
   local-fitness, Apollo-Running, rustyfit, cardio-rs, …) and CC0/CC BY data. Black-box only:
   GPL/AGPL (GoldenCheetah, Gadgetbridge, wger code, Liftosaur, OpenAthlete, pyActigraphy).
   Content with conditions: WHO paper CC BY, NHS and UK CMO under OGL v3, wger exercises
   CC BY-SA. **Not embeddable without a licence: the Borg scales and PAR-Q+** (§7).

---

## 2. What the watch's data can be trusted for

**Rule this section proposes.** Each value carries one of four verdicts: *measured*, *the watch's
estimate* (shown, named as the watch's, never mixed with fm's numbers), *recompute openly* (fm's
own method, under its own name), or *ignore*.

### 2.1 What GB receives from this watch class

- **Support.** GB lists the FR165 / FR165 Music as "mostly supported"; it pairs without Garmin's
  app (VPD, gadgetbridge.org Garmin page). Sleep stages arrive only on watches that show them
  without a phone (VPD, GB Garmin topic page).
- **Files fetched** (VPD): Activity, Monitor, Metrics, Changelog (not parsed), HRV, Skin temperature
  (not parsed), Sleep, readiness files, Hydration — kept in the phone's internal storage.
- **Messages GB's importer reads** (VC, `FitImporter.java`, `GarminWorkoutParser.java` on Codeberg
  `master`): stress with Body Battery inside the same message (`getBodyEnergy()`); sleep stages,
  sleep assessment with about 12 sub-scores, restless moments, naps, raw sleep data; monitoring
  (HR, steps, distance, active calories, moderate/vigorous minutes, ascent/descent); SpO2;
  respiration; HRV status summary (weekly average, last-night average, 5-min high, baseline,
  status) and HRV values; resting HR; VO2max (`max_met_data`); training load, race prediction,
  training readiness, recovery, endurance and hill scores, load history. Per session: aerobic and
  anaerobic training effect, min/avg/max HR, session RMSSD/SDRR, running dynamics, power, RPE and
  "feel", time in HR zones with the zone bounds, lactate-threshold HR, recovery time.
- **Not on this watch class** (SH, review sites): training readiness, training status and training
  load. GB parses those messages when a watch sends them; for this class expect them empty (UV).
  So fm's own load (coach plan P3) would not duplicate anything the watch gives.
- **GB rewrites some values.** It may replace recorded altitude with imported elevation tiles (VC).
  Some newer devices do not sync all activity data; some workouts do not sync; activity sync can
  stick (VPD, GB known issues). **fm should read the original FIT, never GB's summary.**
- **GB can upload workouts and courses to Garmin watches** (VPD, GB Garmin topic), and it accepts
  files from another app: `FileInstallerActivity` is exported for `SEND`/`SEND_MULTIPLE` with
  `*/*` (VC, AndroidManifest). A calendar (FIT schedule file) is **not** handled — see §15.
  Whether the watch takes an fm-written workout end to end is still the coach plan's experiment E1.
- **GB exports to Android Health Connect** (VPD): steps, distance, calories, HR, resting HR, HRV,
  SpO2, respiration, sleep sessions, weight, VO2max, exercise sessions with route and streams.
  Export only.

### 2.2 Metric verdicts

| Metric | Raw or derived | Validity evidence | Openly recomputable? | Verdict |
|---|---|---|---|---|
| GPS track and distance | Raw | 8 GNSS watches: MAPE 3.2–6.1 %; distance may be underestimated by up to 9 %; worse in towns and forest (Gilgen-Ammann 2020, PMID 32396865, VPD abstract) | Yes | **Measured**, with about ±5 % context |
| Altitude, ascent | Raw (barometer) | none read | Yes | **Measured** from the original FIT; say so if GB corrected it |
| Activity HR | Raw (wrist optical) | Garmin within ±3 % in 49 % (23/47) of controlled comparisons (Fuller 2020, PMID 32897239, VPD); bias and lag already in the coach plan §2.4 | — | **Measured**, with the wrist-in-intervals caution |
| Steps | Device algorithm | best for Apple and Garmin, qualitative (Fuller 2020, VPD) | No (no raw IMU) | **Measured** (the device's count) |
| Cadence | Device algorithm | none read | From records | **Measured** |
| Resting HR | Derived, simple | no study specific to it read | Roughly (e.g. lowest sleeping 30-min mean; method UV) | **Watch's estimate**; an open recompute may sit beside it |
| Energy expenditure | Derived | "no brand was accurate"; Garmin under-estimated in 69 % (37/51) of comparisons (Fuller 2020, VPD) | Only with equally poor equations | **Ignore**, or grey with "known inaccurate" |
| Intensity minutes | Derived | none read | Partly: minutes above an HR band from the register | **Watch's estimate** |
| Sleep timing | Derived | total sleep overestimated by about 38 min, limits of agreement −83 to +160 min (Schyvens 2025, PMID 40303381, VPD full text, Vivosmart 4 vs PSG) | No | **Watch's estimate**; times are fair |
| Sleep stages | Derived | same study: κ 0.21 ± 0.13, wake specificity 29 %, deep sleep +44 min (MAPE 340 %), REM 33 % correct. Chinoy 2021 (PMID 33378539, VPD abstract): two Garmin models did worse than actigraphy. Walch 2019's open classifier: 3-class κ ≈ 0.3 (VPD) | No (raw sleep payload undocumented) | **Watch's estimate, trend only**, never a single night |
| Sleep score | Proprietary | none | No | Watch's estimate, or ignore |
| Nightly HRV, HRV status | Derived from optical beats | Casanova-Lizón 2026 (coach plan): reliable within a person, not ECG-equivalent | Not from summaries; needs beat intervals (whether this watch logs them overnight is UV) | **Watch's estimate**; status is proprietary |
| Stress 0–100 | Proprietary (Firstbeat) | Rosenbach 2025 (PMID 41292097, VPD abstract): "only marginal predictive value for subjective stress"; Siepe 2025 (PMID 40742761, VPD abstract, n = 781): device and self-reported stress "did not overlap for most individuals" | No | **Ignore**, or watch's estimate with the caveat |
| Body Battery | Proprietary | no peer-reviewed validation found; Firstbeat lists no white paper for it (VPD, list page) | No | **Ignore**, or "unvalidated" |
| VO2max | Proprietary (HR × speed) | meta-analysis: exercise-based bias −0.09, limits of agreement −9.92 to +9.74 ml/kg/min; Garmin/Fitbit running MAPE 8–10.2 % against Firstbeat's claimed 5 % (Molina-Garcia 2022, PMID 35072942, VPD full text; part-funded by Huawei). Fenix 6: MAPE 7.05 %, CCC 0.73 (Carrier 2025, PMID 39797066, VPD abstract). Firstbeat: 15 bpm error in HRmax gives 7–9 % error (white paper, VPD) | Concept only (Firstbeat's method is described, not specified, and called patented, VPD). An open HR–speed regression is possible but needs its own validation (UV) | **Watch's estimate, trend**; ±10 % for one person; ask for a measured HRmax |
| Training effect | Proprietary (peak EPOC → 0–5) | EPOC from HR vs measured r² ≈ 0.79, n = 32 on a cycle ergometer (Firstbeat white paper, VPD pages 1–3); Training Effect itself: no independent study | No closed form published (VPD) | **Watch's estimate**; fm's own load under a different name |
| Recovery time, race predictor, endurance/hill score | Proprietary | none read | Race prediction: yes, by published formula (§3) | Watch's estimate, or ignore; never mixed with fm's |
| SpO2 | Derived (wrist red/IR) | Fenix 6 vs medical oximeter: MAPE 4.29 % but **CCC 0.10** (Carrier 2025, VPD abstract) | No | **Ignore**, or "not clinical" |
| Respiration rate | Derived | none read | No | Watch's estimate, low priority |
| Wrist running dynamics (GCT, oscillation, stride), wrist power | Derived | no wrist-only Garmin validation read | No | Watch's estimate |
| Workout RPE, "feel" | Typed on the watch | — | — | **As entered** (RPE ÷ 10, coach plan §2.8) |

### 2.3 Cautions that follow

- **Name fm's numbers differently from the watch's.** "Load (HR method)", never "Training Effect".
  Never blend a vendor score with an fm score in one series.
- **Missing data is not a state.** A day without the watch is unknown, not rest (already D2 in the
  coach plan). Google's own coach was reported reading a non-wear day as "full recovery" (SH, §6.1).
- **Conflicts of interest.** The VO2max meta-analysis was part-funded by Huawei; one author of
  Fuller 2020 worked for Garmin; Firstbeat's own error figures were not reproduced independently
  (VPD for all three).
- **Verify on a real file before any UI.** Which of the messages in §2.1 this watch class actually
  writes is SH/UV. A local dump of real files settles it; nothing from that dump goes into the repo.

---

## 3. The science: what an open coach for ordinary people can stand on

### 3.1 Core canon — ten pillars

These are proposed as the backbone of "keep in shape". Each is read at source (VPD) unless marked;
each still needs a register row in `sources-2026-10-11.md` before it ships.

1. **Weekly minutes.** Adults: 150–300 min moderate, or 75–150 min vigorous, a week; bouts of any
   length count; limit sitting, no threshold set (WHO 2020, Bull et al., BJSM, PMID 33239350, VPD;
   US PAG 2018 dropped the 10-minute bout rule, VPD odphp page; UK CMO 2019, VPD gov.uk).
2. **Steps as a plain dial.** Against the lowest quartile (median 3,553 steps), mortality HR 0.60 /
   0.55 / 0.47; the benefit levels off at about 6–8k steps a day at 60 and over, 8–10k under 60
   (Paluch 2022, PMID 35247352, VPD; 15 cohorts, 47,471 adults). 7,000 vs 2,000 steps: HR 0.53;
   the curve bends around 5–7k (Ding 2025, PMID 40713949, VPD; 57 studies, GRADE mostly moderate).
   Observational: associations, not proof. 10,000 is not an evidence-based threshold.
3. **Strength on two or more days, for everyone.** WHO: strong recommendation, moderate certainty.
   Muscle-strengthening work: 10–17 % lower mortality, CVD, cancer and diabetes, J-shaped with the
   biggest gain at 30–60 min a week (Momma 2022, PMID 35228201, VPD). **ACSM 2026 resistance
   training position stand** (PMID 41843416, PMC12965823, VPD abstract and key passages; supersedes
   ACSM 2009): strength from loads ≥ 80 % 1RM, 2–3 sets, ≥ 2 sessions a week; size from ≥ 10 sets a
   week; training to failure, periodisation model and equipment type do not consistently matter;
   "insufficient evidence to quantify exact RIR targets". One set, 1–3 times a week, still raised
   1RM (Androulakis-Korakakis 2020, PMID 31797219, VPD; trained men). +0.37 % muscle per extra weekly
   set (Schoenfeld 2017, PMID 27433992, VPD).
4. **Balance and functional strength for older adults.** WHO: varied multicomponent work on ≥ 3
   days. Exercise cuts the rate of falls by 23 % (rate ratio 0.77, high certainty); balance and
   functional training 24 %, several types combined 34 % (Sherrington 2019, Cochrane, PMID
   30703272, VPD; 108 RCTs, 23,407 people, mean age 76). This is the best-evidenced effect in the
   whole survey.
5. **Fitness is a vital sign to estimate and trend.** The AHA calls for routine assessment of
   cardiorespiratory fitness (Ross 2016, PMID 27881567, VPD). Each 1-MET higher fitness: RR 0.87
   all-cause mortality (Kodama 2009, PMID 19454641, VPD; 33 cohorts). Estimates for fm, each
   labelled an estimate with its error:
   - **Uth 2004** (PMID 14624296, VPD): VO2max ≈ 15.3 × HRmax / HRrest; SEE 2.7 ml/kg/min with
     measured HRmax, 4.7 with age-predicted. **Derived in 46 well-trained men only.** A women's
     factor (14.5) is SH.
   - **Rockport 1-mile walk** (Kline 1987, PMID 3600239, VPD, equation in the abstract):
     VO2max (L/min) = 6.9652 + 0.0091·weight(lb) − 0.0257·age + 0.5955·sex(male = 1)
     − 0.2240·time(min) − 0.0115·HR at the end; r 0.93, SEE 0.325 L/min, n = 343 aged 30–69. A
     guided field test the watch can time and record.
   - Nes 2011 non-exercise model (PMID 21502897, VPD abstract; R² 0.61 men / 0.56 women, SEE
     about 5.7 / 5.1): the coefficients were **not read** (paywalled). Cooper 12-min test: SH.
6. **Mostly comfortable effort, some hard intervals.** Norwegian 4×4 (4 × 4 min at 90–95 % HRmax,
   3 min at 70 % between, 3× a week for 8 weeks): VO2max +7.2 %; matched-work steady or threshold
   training did not raise it (Helgerud 2007, PMID 17414804, VPD; 40 moderately trained young men).
   HIIT vs moderate continuous in cardiometabolic patients: +3.03 ml/kg/min (Weston 2014, PMID
   24144531, VPD). 3 × 20 s sprints matched 45 min steady for VO2peak (+19 %, n = 25; Gillen 2016,
   PMID 27115137, VPD). Safety in cardiac rehab: 1 event per 23,182 h of HIIT vs 1 per 129,456 h of
   moderate training, no deaths or heart attacks (Rognmo 2012, PMID 22879367, VPD; 4,846 patients).
   Intensity can be guided without a licensed scale: the talk test's "equivocal" stage approximates
   the ventilatory threshold (Persinger 2004, PMID 15354048; review 2022, PMID 35507232, both VPD).
7. **Progress gradually; recovery is real; gains fade.** Overtraining is a continuum, diagnosed by
   exclusion, with no single marker (Meeusen 2013 ECSS/ACSM, PMID 23247672, VPD). Under four weeks
   off, VO2max falls fast in the highly trained and recent gains are lost; strength declines slowly
   (Mujika & Padilla 2000, PMIDs 10966148, 10999420, VPD). Specific running-injury rules are weakly
   supported (§3.3).
8. **Sleep ≥ 7 h at regular times.** Adults 18–60: "7 or more hours per night on a regular basis"
   (AASM/SRS 2015, PMC4434546, VPD). Sleep Regularity Index: the four more regular quintiles had
   20–48 % lower all-cause mortality than the least regular (Windred 2024, PMID 37738616, VPD
   abstract; UK Biobank, n = 60,977); SRI defined in Phillips 2017 (PMID 28607474; formula in the
   full text, UV). Both duration and regularity are computable from the watch's sleep *times*,
   which are fair even where stages are not.
9. **Self-monitoring, goals, feedback.** Self-monitoring plus at least one other control-theory
   technique: effect size 0.42 vs 0.26 (Michie 2009, PMID 19916637, VPD; 122 evaluations). Trackers
   and apps: about +1,800 steps a day (Ferguson 2022, PMID 35868813; Laranjo 2021, PMID 33355160,
   both VPD). Habits: 18–254 days to near-full automaticity, and one missed day did not matter
   (Lally 2010, SH; the often-quoted 66-day median is UV). Design vocabulary: BCT Taxonomy v1, 93
   techniques (Michie 2013, PMID 23512568, VPD; its licence UV).
10. **Safety and red flags.** Screening stays as in the coach plan (ACSM 2015 model, full text still
    to read). Sarcopenia cut-offs (EWGSOP2: grip < 27 kg men / < 16 kg women; 5 chair rises > 15 s;
    gait ≤ 0.8 m/s severe; Cruz-Jentoft 2019, PMID 30312372, VPD full text, CC BY-NC) only as "talk
    to a clinician", never a diagnosis.

### 3.2 Training-specific methods worth having

| Method | Source, read? | Use for fm |
|---|---|---|
| **Riegel** T₂ = T₁ (D₂/D₁)^k; k ≈ 1.06 recreational | Riegel 1981 (SH; not read). Vickers & Vertosick 2016 (PMID 27570626, VPD, **CC BY 4.0**): accurate to the half marathon; **marathon too fast by ≥ 10 min for half of runners**; their model adds weekly mileage (n = 2,303) | Use to the half, with the population; for the marathon prefer Vickers' model or none |
| Daniels/Gilbert "VDOT" equations | 1979 book, SH (calculator sites); "VDOT" trademark status UV | Describe only until a peer-reviewed reprint is read |
| Volume-adequacy discount on predictions | Runalyze "marathon shape" (SH, help pages) | An idea: a prediction should say when training volume does not support it |
| Couch to 5K | NHS terms page (VPD): web content under **OGL v3**, except logos, trademarks, stock images, third-party material; adapted text must carry the OGL attribution; no implied endorsement. No outcome RCT found | An adapted, unbranded beginner plan is allowed |
| Reps in reserve (RIR) | Helms 2016 (PMC4961270, VPD, CC BY-NC-ND). Mapping in the summary read disagreed with the usual "10 = 0 RIR" (the table is an image; UV) | Use fm's own wording of "reps left in the tank"; re-read the table |
| Estimated 1RM | Most accurate from a 5RM, use ≤ 10 reps (Reynolds 2006, PMID 16937972, VPD). Epley and Brzycki originals not read | Use ≤ 10 reps, labelled estimate, after the originals are read |
| ACSM 2009 progression | +2–10 % load when 1–2 reps over target are done (PMID 19204579, VPD) | Simple, readable progression rule for strength notes |
| Deload | One mid-programme deload week: slightly less strength, same size (Coleman 2024, PMID 38274324, VPD, n = 39); Delphi opinion (Bell 2023, CC BY) | Describe only |
| Nordic hamstring | RR 0.49 for hamstring injury (van Dyk 2019, PMID 30808663, VPD; 15 studies, 8,459 athletes) | One exercise card, sport context stated |
| Stretching | ≥ 60 s static per muscle: −4.6 % performance just after; no clear injury effect (Behm 2016, PMID 26642915, VPD). Strength training ≈ stretching for range of motion (Afonso 2021, PMID 33917036, VPD) | Use as stated |
| Compendium of Physical Activities 2024 | Free incl. commercial use; **MET values must not be changed**; must cite (Herrmann 2024, VPD pacompendium.com) | Energy cost of activities without HR, unmodified and cited |

### 3.3 Contested or hype — say so plainly

- **Zone 2 as the optimal intensity** for the general public: a 2025 review finds no support
  (PMID 40560504, VPD).
- **PAI** (Nes 2017, PMID 27984009, VPD abstract): the epidemiology is real (≥ 100 a week, CVD
  mortality −17 % men, −23 % women), but the algorithm is not in the abstract and is commercial,
  reportedly patent-pending (SH). Do not copy it or use its name.
- **Generation 100** (Stensvold 2020, PMID 33028588, VPD): the 5-year RCT in people aged 70–77 found
  **no mortality difference**. Do not claim "HIIT makes you live longer".
- **Running-injury prevention.** Stretching RR 0.85 (not significant) and graded build-up RR 1.02,
  mostly in military recruits (Cochrane, PMID 21735382, VPD). An online multifactorial programme
  (PMID 30954948) and a 10-minute strength routine before a marathon (PMID 31642726) showed no
  effect (both VPD). Gait retraining with feedback: HR 0.38 in one lab trial of 320 novices (PMID
  29065279, VPD). Across sports, strength training RR 0.32 (Lauersen 2014, PMID 24100287, VPD).
  fm must not promise injury prevention.
- **Uth's ratio, non-exercise VO2max, "fitness age"**: narrow cohorts, wide error.
- **"No upper limit" of fitness benefit** (Mandsager 2018): clinical cohort, estimated METs.
- **The J-shape for strength over an hour a week**: uncertain.
- **Wellness questionnaires** (Hooper 1995, PMID 7898325, VPD): 14 elite swimmers.
- **HRV, ACWR, "readiness" as injury predictors**: see the coach plan §2.3 and §2.5 (rejected or
  weak).

### 3.4 Instruments: what may be embedded

| Instrument | Status | Verdict |
|---|---|---|
| Borg RPE / CR-10 | Licence required; free only for non-commercial research and teaching; "electronic integration" by system providers must contact BorgPerception (VPD, borgperception.se) | **Do not embed the scale.** Use fm's own 0–10 effort question, RIR or the talk test. Note: the coach plan's `rpe` field is "CR-10"; its label and wording need this check (open point for D7) |
| PAR-Q+ | © PAR-Q+ Collaboration; written authorisation needed to put it in apps or software; fees possible; no modification (VPD, eparmedx.com terms) | **Do not embed.** Link out, or use the ACSM model once read |
| POMS | Commercially copyrighted (SH) | Avoid |
| Talk test | No instrument to license (VPD papers) | Use |
| WHO 2020 guideline paper | BJSM paper **CC BY 4.0** (VPD); the WHO book CC BY-NC-SA 3.0 IGO (SH) | Cite and paraphrase the paper |
| UK CMO 2019, NHS pages | OGL v3 (VPD) | Use with the attribution line |
| US PAG 2018 | US government work, public domain (UV) | Use |
| ACSM 2026 stand, EWGSOP2 | CC BY-NC-ND / CC BY-NC (VPD) | Facts and figures only, fm's own words |

### 3.5 Still to read in full before anything above ships

Nes 2011 coefficients; Nes 2017 PAI method and a patent search; Riegel 1981; a peer-reviewed
Daniels/Gilbert reprint; Zourdos 2016 and the Helms RIR table; Epley 1985, Brzycki 1993, LeSuer
1997; Lally 2010; Phillips 2017 SRI formula (PMC5468315); Cooper 1968; the rest of ACSM 2011
(flexibility and neuromotor doses, PMID 21694556); the WHO book and its licence page; the US PAG
2nd edition and its 2027 supplement; the Otago and FIFA 11+ manual licences; the BCTTv1 terms;
Hooper & Mackinnon 1995; Milanović 2015 in full; Ekelund 2016 in full. Plus the coach plan's own
list (§5 there).

---

## 4. Open apps and platforms

Dates are last push, commit or release as read on 2026-11-20 (VC unless marked).

### 4.1 Tools that help people *train*, not only track

| Tool | Licence | Activity | What it offers for training | Reuse in fm |
|---|---|---|---|---|
| **Domestique** (github.com/platypus45/domestique) | **Apache-2.0** (VC, LICENSE read) | v3.14.0, 2026-11-10 | The best-documented open adaptive engine. Readiness from HRV 30 %, form 20 %, how-you-feel 20 %, sleep 15 %, resting HR 15 %, re-normalised when parts are missing, "insufficient data" below three. Seven guardrails in fixed priority, the first that fires rewrites today and marks it adapted (G1 yesterday > 1.5 × planned → easy; G2 ≥ 25 min hard in 48 h → easy; G3 polarisation drift → next hard sessions drop a tier; G4 last week > 1.5 × planned → next week × 0.85; G5 soreness ≥ 6/7 → recovery; G6 check-in > 2 SD off baseline → easy; G7 3-day mean RPE ≥ 7 → hard session down a tier). Phases base → build → peak → taper with a ramp cap; regenerates after two missed weeks. **An honest caveats section** (weights are "a reasoned prior"; HRV does not detect overreaching) (VPD, docs/SCIENCE.md) | Code reusable with NOTICE and attribution. Cycling only, partly fed by intervals.icu. **Port the pattern, not every threshold**: some gates rest on ACWR-era sources (Hulin 2014, Gabbett 2016) that the coach plan rejected |
| **Apollo-Running** | MIT | 2026-11-09, 7 stars | Local-first marathon planner: 33 plans transcribed from books, late-join fitness check, adaptation with apply/undo, 10 % mileage cap, VDOT paces, Riegel/VDOT blend, decoupling, on-device FIT/GPX/TCX import, `.apollo-plan.json` (VPD) | Schema and code reusable; **book plan content may carry copyright** (UV). The 10 % cap is the rule Buist 2008 found ineffective |
| **local-fitness** | MIT | 2026-11-16 | Deterministic baselines, load, plan grading and per-workout "report cards" exposed as tools; "all synthesis … happens in the agent" (VPD) | Architecture match for Myrme |
| **PaceForge** | MIT | 2026-08, toy scale | Fully local (Ollama, 7B). Deterministic core with tested invariants (≤ 10 % weekly ramp, no back-to-back hard days, taper shape); "the LLM may not do arithmetic"; analysis works with no model; JSON tool-call fallback; device writes dry-run until approved; a pre-push check blocks invented numbers (VC README) | The closest open match to the Myrme contract |
| **garmin-local-mcp** | MIT | 9 stars | Offline FIT import into SQLite, 28-day baselines (mean ± 1 SD), z-score anomalies, holds back suspect resting HR; no LLM calls in the server (VC README) | Pattern for withholding suspect values |
| **OpenAthlete** | AGPL-3.0 | 2026-11-19 | Season calendar with A/B/C races and **illness or travel periods that move the sessions they cover**; structured workouts to Garmin/COROS/Suunto; per-interval grading against the plan; CS/CP; load from HR *or RPE*, projected over planned sessions; bring-your-own-key or local model (VPD) | Black box. Study calendar semantics and compliance grading |
| **GoldenCheetah** v3.8 | GPL (VC) | 2026-11-16 | Beyond the load chart: a plan calendar with phases, `.gcplan` export/import, **automatic linking of planned to actual by day and sport**, an "expected" load curve from planned sessions, user-defined metrics by formula (VPD wiki) | Black box. Copy the ideas |
| **Liftosaur** | AGPL-3.0 | 2026-11-20 | **Liftoscript**: progression and deload rules as a short readable script inside the program (e.g. add 5 lb when reps are hit; after 3 failures × 0.9); 5/3/1, GZCLP built in (VPD) | Black box. **The strongest idea for strength: rules as readable text in the plan** |
| **wger** | AGPL-3.0 code; exercise data per-entry CC (VC, live API: of 200 sampled, CC BY-SA 3 118, CC BY-SA 4 64, CC0 18) | 2026-11-19 | Routines with automatic progression rules, nutrition with Open Food Facts, REST API | Code: no. Data: with attribution and share-alike kept on the data |
| **avidididadi** (Android) | GPL-3.0-only | 2026-11-19, 4 stars | Strength, endurance and mobility generators; adapts to "how did it go", RPE, HRV, sleep, resting HR (rules undocumented); FTP test "only when needed"; **data as plain CSV files** | Black box. Interesting as files-as-data, multi-sport |
| **Coach Watts** | Apache-2.0 | 2026-09-01 | Load, power curves, daily push/rest from HRV and sleep, plans written by a cloud LLM | Less instructive: decisions sit in the LLM |
| **running-formulas-mcp** | MIT | UV | Daniels VDOT, training paces, equivalent times, HR zones | Reusable after checking against a primary text |
| **time-to-run** | MIT | 2024-02 | Pick a plan and race date, get a calendar | "Plan anchored to a date", small |
| **c2k** (Android) | GPL-3.0 | 2026-11-19 | Couch-to-5K/10K timers with spoken cues | Black box; the programme itself is public (§3.2) |
| **Auuki** | AGPL-3.0 | 2026-11-07 | Browser trainer control, `.zwo` workouts, RR intervals into FIT | Black box; ZWO is an open workout format |
| **intervals.icu** | Proprietary, free | active | The reference design: fitness/fatigue/form, eFTP and named power models (Morton 3P, Monod & Scherrer), interval detection, decoupling, custom zones on any stream, workout builder with ZWO/FIT export, REST API with key, an MCP connector (VPD) | Not open. A hub many open tools sync with; fm stays local |
| **Runalyze** | Open edition discontinued; archived 2019 (VC README) | — | Per-run effective VO2max corrected by a best race; "marathon shape" from 6 months of volume and long runs (SH) | Ideas only |

### 4.2 Tracking, logging and content

| Tool | Licence | Note |
|---|---|---|
| Gadgetbridge | AGPL-3.0 | §2.1. 2026.11.1 released 2026-11-07 |
| GarminDB | GPL-2.0 | Garmin → SQLite; **keeps raw FIT and treats the database as rebuildable** — fm's files-as-truth idea |
| garmin-grafana | BSD-3 | Needs Garmin's cloud API |
| FitTrackee, Endurain, Dreeve (formerly Statistics for Strava), wanderer | AGPL-3.0 | Tracking only; Dreeve has motivation views (Eddington number, milestones, gear) |
| workout-tracker | MIT (assets/vendor may differ) | Route segments for progress over time; manual strength records |
| OpenTracks | Apache-2.0 | Recorder |
| FitoTrack | GPLv3 (SH) | Recorder |
| openScale | GPL-3.0 | Body measures, CSV export → an import source for fm notes |
| Flex | MIT | Offline strength/cardio log, simple |
| openGym | AGPL-3.0 | Data "in a folder you control"; its star count looks implausible (UV) |
| Workout.cool | MIT | Exercise catalogue; media licence separate (UV) |
| Feeel | AGPL-3.0+ code; **exercise text and illustrations CC BY-SA 4.0** (VC) | Mobility and stretching content with attribution |
| free-exercise-db | **Unlicense** (VC) | 800+ exercises. **Provenance caution:** restructured from someone else's exercises.json of unstated origin, so image and text rights are UV |
| OpenWorkoutTracker | MPL-2.0 with a contradictory "commercial use prohibited" note | Avoid |
| MyTourbook, SportsTracker, pytrainer | GPL-2.0 | Legacy logging |

### 4.3 Gaps no open tool fills

1. **A local closed loop for Garmin.** Plan → workout on the watch → FIT back → matched to the plan
   → compliance → next proposal, without Garmin Connect or intervals.icu. GB's workout upload is
   the missing link (coach plan P6, experiment E1).
2. **Adaptive engines for running and strength.** Only Domestique documents its rules with sources,
   and it is cycling-only.
3. **Hybrid weeks.** Endurance tools and lifting tools are separate worlds; none plans strength,
   running and mobility in one weekly budget.
4. **Keeping in shape over years.** For non-racers there are fixed C25K timers and generic
   routines; nothing does maintenance over months: minimum effective dose, detraining noticed,
   strength and balance added with age.
5. **Honest confidence.** Almost no tool says how sure it is or checks its predictions against
   later races.
6. **Plans as editable files.** Plans as text with their own readable rules exist nowhere; fm's
   files-as-truth model is a natural fit.

---

## 5. Building blocks: libraries, datasets, learned models

### 5.1 Libraries

| Name | Licence (VC) | Status (VC) | Gives | Fit for fm |
|---|---|---|---|---|
| **rustyfit** | BSD-3 | v0.11.2, 2026-11-17 | `no_std` FIT decode **and encode** | A second decoder for the T4b oracle, and an encoder candidate for P6 (fitparser decodes only). Still subject to D4 (the FIT licence question) |
| fitparser (fitparse-rs) | MIT | — | Decoder, already in the coach plan | Whether its profile is generated from Garmin's Profile.xlsx matters for D4 (coach plan says yes, `field_types.rs:4`) |
| fitdecode (Python) | MIT | 2025-08 | Streaming reader | Test oracle outside the repo |
| Garmin FIT SDK | FIT Protocol License, not OSI; §2(d) forbids copyleft-style redistribution; §2(f) forbids benchmarking or competitive analysis (VPD, fit-javascript-sdk LICENSE) | — | Official | Do not vendor. §2(f) is a new point for D4 |
| gpx (georust) | MIT | 2025-08 | GPX read/write | Route export |
| tcx | MIT | 2021, stale | TCX | Usable |
| srtm_reader | BSD-3 | 2026-07 | Elevation from SRTM tiles | Optional elevation correction, offline after one download |
| **cardio-rs** | MIT | 2025-03 | RMSSD, SDNN, pNN50, LF/HF, peaks | Best Rust HRV block if beat intervals exist |
| hrv-algos | MIT | 2024-12, "unstable" | Ectopic removal, DFA-α1, Poincaré | DFA-α1 for a later aerobic-threshold idea (Maffulli 2026 in the coach plan) |
| NeuroKit2, pyHRV, HeartPy, systole | MIT / BSD-3 / MIT / Apache-2.0 | active | HRV and peak detection | Local oracles to check a Rust port |
| hrv-analysis, pyActigraphy | **GPL-3.0** | active | HRV; sleep regularity, IS/IV, Cole-Kripke | Read only; re-implement SRI from Phillips 2017 |
| GGIR (R) | Apache-2.0 | active | Raw-accelerometer pipeline | Not applicable (no raw IMU) |
| sweatpy | MIT | 2022-12, stale | Endurance helpers | Low value |
| Minetti GAP | Paper (coach plan) | — | Polynomial | Implement from the paper body once read. **Strava's HR-based GAP is patented** (US 11,623,121, SH): do not copy |

### 5.2 Datasets

| Dataset | Licence | Gives | Use |
|---|---|---|---|
| **GoldenCheetah OpenData** | **CC0 1.0** (VC, OSF API) | Anonymised 1 s HR, cadence, power, distance, altitude; no GPS; mostly cycling; collection off since about 2021 | Committable fixtures for load and HR features |
| Lövdal running injury | CC0 (SH) | 7 years of logs from 74 high-level runners with injuries | Local measurement; not a model source |
| sleep-accel (Walch) | ODC-By 1.0 (VPD) | 31 people, Apple Watch motion and HR with PSG | Validating sleep logic |
| MMASH | ODbL 1.0 (VPD) | 22 men, RR intervals, actigraphy, sleep | RR-based HRV tests (share-alike on derived databases) |
| LifeSnaps | CC BY 4.0 (VPD) | 71 people over 4+ months, Fitbit daily data plus surveys | Closest open match to fm's daily summaries |
| PMData | CC BY on Simula's page vs CC BY-NC on a mirror (SH) | 16 people, Fitbit plus sRPE, wellness, injuries | Confirm the licence at source first |
| PPG-DaLiA | CC BY 4.0 | Raw PPG | Not relevant |
| WESAD | Non-commercial (SH) | Stress | Avoid |
| Endomondo / FitRec | **No licence** (VPD) | 253k scraped workouts | Local research only; nothing derived ships |
| UK Biobank, MESA/NSRR | Access agreements | — | Not usable |
| Compendium 2024 | Free, unmodified MET values, cite (VPD) | METs | §3.2 |
| **No public Garmin FIT dataset was found** (SH) | — | — | Real-file testing stays local, outside the repo |

### 5.3 Learned and fitted models

- **The one that fits fm's inputs: a per-person HR-response model.** Nazaret et al. 2023 (Apple,
  PMC10651837, VPD): a hybrid ODE in which an oxygen-demand state driven by workout intensity
  (speed, grade) moves HR between per-person HRmin and HRmax; 270,707 workouts from 7,465 people,
  median error 6.1 bpm; the learned parameters correlate with VO2max. Code under Apple's sample-code
  licence (permissive, **no patent grant**, no weights). The plain two-state ODE, fitted per run in
  Rust, is small enough for a phone; its parameter drift is a candidate "fitness from HR at pace"
  trend. **Not externally validated** (data from Apple's own study only). Re-implement from the
  paper, not the code. Plain fallback: aerobic decoupling (HR drift against pace).
- **Not applicable (need raw PPG or IMU):** PaPaGei (code BSD-3, weights licence unstated,
  archived), Pulse-PPG (MIT with weights), BeliefPPG, OxWearables asleep/stepcount/ssl-wearables
  (**Oxford academic licence, "not for use by consumers"**), Walch's sleep classifiers (MIT, but
  motion input and κ ≈ 0.3 for 3 classes).
- **Not available:** Apple's biosignal and behaviour foundation models, Google's LSM/SensorLM —
  no weights released (VPD/SH). Apple's WBM (arXiv 2507.00191) shows daily aggregates carry signal,
  but there is nothing to use.
- **Do not build: an injury-risk model.** The best open one (Lövdal 2021, bagged XGBoost) reached
  AUC 0.72 by day, 0.68 by week, in 74 elite runners of one team, not externally validated (SH).
  At most describe facts ("weekly distance +40 % on your 4-week average").
- **Not needed:** activity classification (the FIT session carries the sport); PPG artefact
  cleaning (fm only gets HR output; plain outlier rules suffice, coach plan §P4).

### 5.4 Step map: how each candidate feature would be built

| Feature | Build as | Evidence |
|---|---|---|
| Resting HR trend | Plain code: rolling baseline and deviation | Descriptive; strong as description |
| HRV trend | Plain code over the watch's nightly value: ln(RMSSD), 7-day mean against a 30–60-day baseline (Plews/Buchheit approach, SH; coach plan already allows "trend, no proposal") | Method SH; vendor RMSSD a black box |
| Sleep summary | Plain code over the watch's times: duration, timing, mid-sleep, SRI. Never re-stage | Duration/timing fair; stages weak |
| VO2max | Show the watch's; optional Uth ratio or Rockport test as fm's own, each labelled | §3.1 pillar 5, §2.2 |
| Race prediction | Riegel to the half; Vickers' model for the marathon | §3.2 |
| GAP, elevation | Plain code from Minetti once read; optional SRTM | Coach plan §1 |
| Fitness from HR at pace | Per-person HR-response ODE (experimental) or decoupling | §5.3 |
| Energy cost of non-HR activities | Compendium lookup, unmodified, cited | §3.2 |
| Strength library | free-exercise-db (Unlicense, provenance caution) or wger/Feeel entries with attribution | §4.2 |
| Injury risk | **Not built** | §5.3 |

Nothing in this map needs a Python sidecar on the phone. Python libraries serve only as local test
oracles.

---

## 6. AI-assisted coaching

### 6.1 Research

| Work | What it did | Result | Read? |
|---|---|---|---|
| **PHIA** (Google; arXiv 2406.06464) | Agent writing code over wearable data | Objective questions: **84 % with code vs 22 % for LLM numeric reasoning without code** (code baseline 74 %); experts rated 83 % of open answers favourable. Question set released under **CC BY-NC 4.0** | Figures SH; repo VC |
| PH-LLM (Google, Nature Medicine 2025) | Gemini fine-tuned on daily wearable aggregates | Exams: fitness 88 % vs experts 71 %, sleep 79 % vs 76 %; experts still better at sleep coaching | VPD (Google page) |
| Personal Health Agent (arXiv 2508.20148) | Data-science, domain-expert and coach sub-agents with an orchestrator; ~1,200 users | Preferred over single agents (blog, SH) | VPD abstract |
| Google/Fitbit coach in public use | Field reports | **Invented a run, then suggested the user forgot to log it**; read a non-wear day as full recovery; stale memory | OSnews quoting 9to5Google VPD; rest SH |
| ChatGPT running plans (J Sports Sci Med 2024, 23:56-72) | 10 coaches rated plans on 22 criteria | Better with richer prompts, but **the best plan raised load too fast**; weak screening; never asked clarifying questions | VPD |
| **Bloom** (Stanford, arXiv 2510.05449) | GPT-4o coach, plan edits only through tools, deterministic workout-to-plan linking; 4-week RCT, n = 54 | Both arms doubled the share meeting guidelines; **no activity advantage from the LLM**; better mindsets. Safety taxonomy with 5 harm classes, 600-example benchmark (prompts and taxonomy under a proprietary licence) | VPD |
| GPTCoach (CHI 2025) | Motivational interviewing with tool calls; 16 people | MI-consistent or neutral > 93 % of the time | SH |
| HealthSLM-Bench (arXiv 2509.07260) | 9 models of 1–3.8B on PMData and others | Competitive on classification; collapse to the majority class on imbalanced tasks; 4-bit on a phone: 1.8–7.6 s per answer | VPD |
| WearableQA (arXiv 2609.05405) | 4,084 questions over 200 users' data, 14 LLMs | 19.6–72.9 % accuracy; most under 60 % | VPD abstract |
| JMIR scoping review 2025 (e79217) | 20 studies of LLM exercise coaching | 55 % low rigour; no long-term outcomes; recommends RAG over guidelines, yes/no rubrics, red-teaming, local models | VPD |
| Boolean rubrics (arXiv 2503.23339) | Yes/no rubric items for health answers | Higher agreement, about half the time | VPD abstract |
| Fitness-fatigue optimisation, Bayesian Banister, RL plans | Model-based planning | Ill-conditioned; "not yet a practical predictive tool"; RL gains only in simulation | SH |

**Reading.** Arithmetic in the LLM fails; LLM-written plans ramp load unsafely; deployed coaches
invent data; the one RCT shows attitude gains, not activity gains; no model-based planner beats
transparent rules on evidence.

### 6.2 How commercial coaches adapt (documented parts only)

- **Garmin Coach:** plans from three named coaches, set up from weekly distance, pace and a goal;
  "adapt based on your progress" (VPD, Garmin blog). Split between rules, ML and LLM undocumented.
  Garmin's acute/chronic load ratio bands (below 0.8 low, 0.8–1.4 optimal) are SH and ACWR-like.
- **Runna:** coach-written plans adjusted by an algorithm; paces change only on **repeated** misses
  or beats in tempo and interval sessions, outliers ignored; a skipped session is dropped and the
  plan continues (VPD support page); after 3+ misses a realignment offer (SH).
- **HRV4Training:** normal range from the last 60 days; below range → "consider easing off"
  (VPD, FAQ). Athletica: 7-day vs 60-day traffic light (SH).
- **AI Endurance (2023, VPD):** a Banister-style model plus rules, with ChatGPT in the adaptation
  loop — the only documented case of an LLM deciding.
- **TrainAsONE, WHOOP Coach, Garmin "Active Intelligence":** marketing only (SH).

**Pattern:** templates written by coaches, plus deterministic rules, plus an LLM on top to explain.

### 6.3 Open AI-coach projects

Most pair a local server with a cloud LLM and an unofficial Garmin login (garmin_mcp MIT,
1.5k stars, 110+ tools — too many for a small model; garmin-connect-mcp MIT, 22 tools;
intervals-mcp-server **GPL-3.0**, write tools, SSE mode without auth by default; strava-mcp MIT;
garmin-ai-coach MIT, cloud LLMs, LangGraph agents; claude-coach MIT, cloud despite "local"
wording). **Local end to end:** PaceForge, garmin-local-mcp, Domestique (no LLM). **No Gadgetbridge
MCP server was found** (SH). All VC from repo pages unless marked.

### 6.4 Step map for Myrme

| Step | Build as |
|---|---|
| Import, dedupe, units | Rust (coach plan P1) |
| Data quality, non-wear, suspect values withheld | Rust |
| All metrics, baselines, z-scores, load, intensity spread | Rust |
| Match activity to plan; missed or extra sessions | Rust |
| "Should I train today?" | **Rust rules with a source on each**, giving a verdict and reason codes; Myrme phrases it |
| Next week, pace targets | Rust planner producing a **proposal** under invariants (hard-day spacing, a ramp cap that is a stated *convention* until sourced, Runna-style "repeated miss or beat" before a pace change); the person accepts |
| Season plan | Rust templates with parameters |
| Personal response model | Later, advisory only |
| Explain why | Myrme, quoting tool outputs and register passages only |
| Questions over own data | Myrme choosing from a **small, fixed set of typed query tools** (not free code on a phone) |
| Free-text check-in → soreness, illness, effort | Small model extracts; the person confirms before it is written |
| Goals, motivation | Myrme, motivational-interviewing style, no numbers of its own |
| Safety triage | **Code first** (keywords plus fixed scripted stop and referral); the LLM is never the only gate |
| Every number in a reply | Code checks it traces to a tool result; otherwise block |
| Evaluation | Yes/no rubric test set, red-team prompts, invented people only |

### 6.5 What Myrme must never do

1. State a number not taken from a tool result.
2. Read missing data as a body state.
3. Change a plan, the calendar or the watch on its own — proposals only (D9 already says so).
4. Exceed code-enforced limits, even when asked.
5. Diagnose or play down red-flag symptoms; give diet or weight targets on at-risk signals.
6. Present ACWR, HRV or "readiness" as an injury predictor.
7. Cite a source not read at source.
8. Send health data off the device by default.
9. Let remembered context override fresh data.
10. Carry GPL, AGPL or CC BY-NC material (OpenAthlete, intervals-mcp-server, PHIA data, Bloom
    prompts) into the MIT codebase.

---

## 7. Licence map

| Can go into MIT fm (with notices) | Black-box reference only | Content with conditions | Do not use |
|---|---|---|---|
| Domestique (Apache-2.0), Coach Watts (Apache-2.0), local-fitness, PaceForge, garmin-local-mcp, Apollo-Running code, running-formulas-mcp, Flex (MIT), garmin-grafana (BSD-3), rustyfit (BSD-3), fitparser, gpx, tcx, cardio-rs, hrv-algos (MIT), srtm_reader (BSD-3), systole (Apache-2.0) | GoldenCheetah, Gadgetbridge, wger code, Liftosaur, OpenAthlete, Auuki, c2k, avidididadi, FitTrackee, Endurain, GarminDB, openScale, intervals-mcp-server, pyActigraphy, hrv-analysis | WHO paper (CC BY), UK CMO and NHS (OGL v3, attribution line, no endorsement), Vickers 2016 (CC BY), wger and Feeel exercises (CC BY-SA, kept apart from code), Compendium (unmodified, cited), ACSM 2026 / EWGSOP2 / Helms (NC-ND / NC: facts in own words), GC OpenData (CC0), LifeSnaps (CC BY), MMASH (ODbL) | Borg scales and PAR-Q+ (licence required), POMS, PHIA data (NC), Bloom prompts, OxWearables models, FitRec code and Endomondo data, Strava GAP (patent), Garmin FIT SDK files (proprietary) |

Ideas are not licensed: fm may adopt any idea above (plan↔actual linking, rules as text, readiness
that admits missing data) and write it from its own sources.

---

## 8. What this suggests for fm — directions for the owner, none decided

The owner's framing in the coach plan stands: a watch file is an asset in a note; coaching lives in
Myrme; everything is read at source. Within that, the research points to two tracks and a few
structural pieces.

### 8.1 "Keep in shape" — for everyone, before any training plan

- **A guidelines view** from the canon (§3.1): weekly moderate/vigorous minutes, strength days,
  steps against one's own baseline, balance days for older adults, sleep duration and regularity.
  Measured facts plus register-backed bands; no model needed. Most of it extends cards Fitness
  already carries (the WHO card, D10's HR bands).
- **Fitness as a vital sign:** the watch's VO2max as its estimate, plus an optional guided field
  test (Rockport walk) recorded as fm's own, each with its error.
- **Maintenance over years:** minimum effective dose (one set, 2× a week), detraining noticed after
  breaks, strength and balance raised in priority with age. No open tool does this (§4.3 gap 4).

### 8.2 "Train" — plans that move with life

- **Plans as notes with readable rules.** Liftosaur's idea in fm's grammar: a plan note carries its
  own progression rule as text (e.g. ACSM 2009's +2–10 % when 1–2 reps over target), each rule with
  a source row. Fits files-as-truth, and the person can read and edit it.
- **Plan ↔ actual.** Imported workouts link to planned notes by day and sport (GC, OpenAthlete,
  Bloom); per-session compliance; missed weeks handled explicitly (Runna: drop and continue;
  Domestique: regenerate after two; OpenAthlete: illness periods move sessions).
- **A rule chain in Rust** on Domestique's pattern — first rule that fires produces a proposal with
  its reason and source; lower or hold, never raise (D9). Domestique's thresholds that rest on ACWR
  sources are not carried over.
- **Readiness that admits missing data**, if ever: inputs dropped and re-weighted when absent;
  "insufficient data" below a floor. Its weights are a prior and must be labelled so.
- **The local loop:** fm writes a FIT workout → GB uploads it → the watch records → FIT back → link
  → proposal. P6 and experiment E1 decide whether this is possible.

### 8.3 Structure

- **Three layers in every note** (§2): measured, the watch's estimate, fm's own — each value with a
  source and method tag. Open mHealth (Apache-2.0) and IEEE 1752.1 (schemas Apache-2.0, VPD page)
  are models for field names; LOINC codes (e.g. HR 8867-4, daily steps 41950-7, resting HR
  40443-4; VPD, FHIR R4 and the Physical Activity IG) may be optional tags for later export.
- **Vendor-neutral sources.** A BLE chest strap gives ECG-grade beat intervals (true RMSSD); Polar's
  BLE SDK streams RR and ECG (custom permissive licence, MIT compatibility UV); Bangle.js exposes
  raw PPG (SH). The note model should not assume Garmin.
- **Test data.** GC OpenData (CC0) as committed fixtures; invented people for everything else; real
  files only locally.

### 8.4 Decision entries this would need (in addition to the coach plan's D1–D10)

| Entry | What it settles |
|---|---|
| *Watch estimates are shown as the watch's, never as fm's* | The three-layer rule; which vendor fields are shown, greyed or ignored (§2.2) |
| *A guidelines view for keeping in shape* | §8.1; which register rows it needs (WHO/UK/US, Paluch/Ding, Momma, Sherrington, AASM, Windred) |
| *Plans carry readable progression rules* | A closed rule grammar for plan notes, each rule with a source |
| *Planned and done are linked* | Matching rules, compliance, missed-week handling |
| *Effort is asked in fm's own words* | Replaces any Borg CR-10 wording; also affects the coach plan's `rpe` field (§3.4) |
| *Content licences* | Whether CC BY-SA exercise data (wger, Feeel) may ship beside MIT code, and how attribution shows; free-exercise-db's provenance |
| *Field tests* | Rockport (and others once read) as guided tests whose results are fm's own estimates |
| *Myrme's coaching tools* | The fixed tool set, the number-trace check, the safety gate in code (§6.4–6.5); a contract bump |

---

## 9. Points that conflict or need checking

- **GB's Garmin charts vs this watch class.** GB displays training load, readiness and race
  predictor *when the device sends them*; reviews say this watch class has none of training
  readiness, status or load (SH). Confirm on a real file.
- **Domestique's licence** was read as Apache-2.0 in its LICENSE (VC); its author once said MIT in a
  forum (SH). The file wins.
- **Helms' RIR mapping** read through a summarising fetch disagreed with the usual mapping; the
  table is an image. Re-read before any RIR wording.
- **Uth's women's factor (14.5)** is SH; the paper studied men only.
- **The FIT licence §2(f)** (no benchmarking or competitive analysis) is new to D4's list; the coach
  plan cited §1, §2(c), §2(d).
- **Garmin's acute/chronic load bands** are ACWR-like; the coach plan rejected ACWR. If the watch
  ever sends them, they are the watch's estimate, never an fm rule.
- **The 10 % weekly cap** used by Apollo and PaceForge is the rule Buist 2008 found ineffective; a
  ramp cap in fm can only be a stated convention.

## 10. What this run did not do

- No code was read in fm and nothing in fm was changed; no real watch file was opened.
- Full texts were not read for most of §3's items (abstracts and key passages only; §3.5 lists the
  gaps). Several repo activity dates and one star count are UV.
- No legal reading of any licence; licence verdicts are a reading of the text, not advice.
- Not covered: nutrition (beyond noting wger's Open Food Facts link), pregnancy and children
  (the coach plan routes these to existing cards), mental health (see `health-plan-2026-10-22.md`
  §10), swimming and cycling specifics.

---

## 11. Sources

**PubMed (VPD; abstract unless "full text").** 33239350 (+ PMC7719906 passages), 35247352,
40713949, 31434697, 27475271, 35228201, 31685526, 25082581, 27881567, 19454641, 30646252,
14624296, 21502897, 3600239, 27984009, 17414804, 33028588, 24144531, 26243014, 27115137,
22879367, 40560504, 36900796, 27570626 (+ PMC5000509 passages), 30954948, 26221015, 29065279,
24100287, 21735382, 31642726, 40407382, 41122014, 41843416 (+ PMC12965823 passages), 19204579,
21694556, 27531969, 31797219, 27433992, 12741856, 16937972, 38274324, 37730925, 26642915,
33917036, 31792067, 30703272, 20817938, 30312372 (+ PMC6322506 table), 29209504, 30808663,
23247672, 10966148, 10999420, 7898325, 26039963, 29073398, 37738616, 39405349, 28607474,
23512568, 19916637, 35868813, 33355160, 15354048, 35507232, 32396865, 32897239 (PMC7509623,
partly), 33378539, 40303381 (PMC12038347, full text), 38557808, 35072942 (PMC9213394, full
text), 39797066, 41292097, 40742761, PMC4434546 (AASM/SRS), PMC4961270 (Helms), PMC10511399
(Bell), PMC10651837 (Nazaret).

**Papers and preprints (VPD unless marked).** arXiv 2508.20148, 2510.05449, 2503.23339,
2509.07260, 2609.05405, 2507.00191; J Sports Sci Med 2024 23:56-72; JMIR 2025 e79217
(PMC12520646); research.google PH-LLM page; Google "personal health coach" blog; Walch 2019
(Sleep zsz180). SH: arXiv 2406.06464 figures, 2405.06061, 2505.20859, 2204.00961; Lally 2010;
Lövdal 2021; Vickers' Riegel exponents; INTERLIVE meta-analysis.

**Guideline, terms and vendor pages (VPD).** gov.uk UK CMO guidelines; odphp.health.gov current
guidelines and "top 10"; nhs.uk terms; borgperception.se licence page; eparmedx.com terms;
ntnu.edu/cerg/vo2max; pacompendium.com; Firstbeat VO2max white paper (full) and EPOC white paper
(pp. 1–3) and publication lists; Garmin adaptive-plan blog; Runna support article;
the5krunner on Runna (2026) and AI Endurance (2023); athletica.ai; hrv4training.com FAQ;
FIT SDK licence (garmin/fit-javascript-sdk); hl7.org FHIR R4 vital signs; HL7 Physical Activity
IG measures; developer.android.com Health Connect data types; standards.ieee.org 1752.1;
github.com/openmhealth/schemas; github.com/polarofficial/polar-ble-sdk; gadgetbridge.org Garmin
gadget, Garmin topic, features, activities and Health Connect pages.

**Code and repositories (VC).** Gadgetbridge (Codeberg `master`: `FitImporter.java`,
`GarminWorkoutParser.java`, LICENSE); repo metadata, LICENSE or README for every tool in §4 and
§5; crates.io API for rustyfit, cardio-rs, hrv-algos, srtm_reader, tcx, fitparser; wger live
licence and exercise APIs; OSF API for GC OpenData; Zenodo records for LifeSnaps and Pulse-PPG;
PhysioNet pages for sleep-accel and MMASH; UCI pages for PPG-DaLiA and WESAD.

**Failed or not reached.** iris.who.int (403), NCBI Bookshelf (captcha), Wiley (403), USPTO (404),
jcsm.aasm.org (404), the Garmin FIT licence web page (403), datasets.simula.no (TLS), PMC for some
items and TechRadar, Android Authority and OpenAI pages (403); the GitHub API in one pass.

---

# Part 2 — Training athletes for performance (2026-11-20)

**Scope.** Competitive amateurs to sub-elite: endurance first (running), then strength and power,
team and hybrid sport, nutrition and recovery, monitoring and performance models, and getting
workouts and a calendar onto the watch. Three research passes, same labels as Part 1;
**VPD-abs** below means the PubMed abstract only, which is the case for most rows. One pass read
abstracts through a summarising fetch told to quote numbers verbatim; **every figure must be
re-read in the original abstract before it enters the register.**

Already covered elsewhere and not repeated: the coach plan's Part 2 (load methods, CS, taper,
intensity distribution, HRV-guided training) and Part 1 §3 here (4×4, HIIT reviews, ACSM 2026,
Riegel/Vickers, deload, Nordic hamstring, overtraining, detraining).

## 12. Endurance training

### 12.1 Evidence

| Topic | Finding | Source, read | Strength | From the watch? | fm use |
|---|---|---|---|---|---|
| Volume and intensity at the top | ≥ 80 % of volume at low intensity; 11–14 sessions a week; build over 8–12 weeks; taper 7–10 days; a 6-zone table (zone 3 ≈ 83–87 % HRmax). The authors: results-proven practice, not proven "best" practice | Haugen 2022, PMID 35362850 (VPD, most of the full text) | Expert, descriptive | Zones from %HRmax: yes | The zone table, labelled descriptive; **not** its volumes for amateurs |
| Volume and level in amateurs | Fastest marathoners ran > 3× the volume, mostly easy; > 80 % of the fastest trained pyramidal; mean 45.1 km a week (151,813 marathons). Grouped R² ≥ 0.90 overstates individual effects | Muniz-Pumares 2025, PMID 39616560 (VPD-abs) | Observational | Yes (CS-based zones) | "Runners like you" description, never causal |
| Training determinants of the marathon | Weekly distance, runs a week, longest run and training pace all relate to time; R² 0.38–0.81 | Doherty 2020, PMID 31704026 (VPD-abs; 85 studies) | Observational | Yes | Describe |
| Big-data performance indices | Aerobic-power and endurance indices fitted to season-best 5 km–marathon; 2.0 % mean prediction error with ≥ 3 races a season; pace and time only; cause and effect "remained open" | Emig & Peltonen 2020, PMID 33024098 (VPD, part of the full text; ~14,000 runners, 1.6 M sessions) | Observational | Yes | A model candidate, with its stated limits |
| Consistency | A break of ≥ 7 days in the 16 weeks before a marathon: 5–8 % slower | Feely 2023, PMID 36704260 (VPD-abs; ~300,000 runners) | Observational | Yes (gap detection) | A plain descriptive flag |
| Periodisation models | Block slightly better for VO2max (SMD 0.40, CI 0.02–0.79) in small, low-quality studies (PEDro 3.7/10). Reverse periodisation no better. Order of interval types did not matter when load was matched (RCT, 63 cyclists) | Mølmen 2019, PMID 31802956; González-Ravé 2022, PMID 35445953; Sylta 2016, PMID 27300278 (all VPD-abs) | Low; one RCT | Plan structure only | **Total load over clever ordering**; do not sell models |
| Threshold-zone intervals | 4 × 8 min (~90 % HRmax) beat 4 × 16 and 4 × 4 min for VO2peak: +11.4 % vs +4.2 % (35 recreational cyclists, 7 weeks). Rest of ~2 min between 4-min reps suggested | Seiler 2013, PMID 21812820; Seiler & Hetlelid 2005, PMID 16177614 (VPD-abs) | Moderate, single RCTs | Yes | Use, cited as one RCT |
| Short vs long intervals | 30/15 s beat 5-min reps for VO2max (+8.7 % vs +2.6 %, small groups, one lab); but with self-pacing the time-near-VO2max advantage vanished | Rønnestad 2015, PMID 24382021; 2021, PMID 33735833; Appelhans 2025, PMID 40328438 (VPD-abs) | Contested | Structure from pace | Describe as contested |
| Billat vVO2max intervals | Reps at vVO2max for 50 % of time-to-exhaustion; n = 8, VO2max unchanged | Billat 1999, PMID 9927024 (VPD-abs) | Low | With a field test | Describe only |
| HIIT programming | Aim for time ≥ 90 % VO2max; up to 9 variables | Buchheit & Laursen 2013, PMIDs 23539308, 23832851 (VPD-abs) | Expert | Partly | Describe |
| Strength → running economy | Economy improves 2–8 %; explosive −4.83 %, heavy −3.65 %; 1.5–10 km time trials improve; VO2max unaffected; 2–3 sessions a week | Balsalobre-Fernández 2016, PMID 26694507; Denadai 2017, PMID 27497600; Blagrove 2018, PMID 29249083 (VPD-abs) | Moderate | Logged strength sessions | **Use** |
| Durability | Critical power falls ~10 % after 2 h of heavy cycling (range < 1 % to ~32 %); a "fourth dimension" | Maunder 2021, PMID 33886100; Jones 2024, PMID 37606604 (VPD-abs) | Emerging concept | See next row | Describe |
| HR–pace decoupling in the marathon | %HRmax ÷ (speed as %CS), km 35–40 against km 5–10; mean 1.16, onset (> 1.025) at 25.2 km; low vs high decouplers 217 vs 238 min; adding it cut prediction error 6.45 % → 5.16 % (82,303 runners). Bands < 1.1, 1.1–1.2, ≥ 1.2 | Smyth 2022, PMID 35511416 (VPD-abs) | Observational, large | **Yes**: HR + GPS + CS | **Use** with this definition; the coaching "< 5 % Pa:HR" rule is lore |
| Pacing | Starting a marathon above ~94 % CS predicts > 25 % second-half slowing; world records trend to even pacing | Smyth & Muniz-Pumares 2020, PMID 32472926; Díaz 2018/2019, PMIDs 29557279, 30922191 (VPD-abs) | Observational | Yes | **Use** as a pacing caution |
| Heat | Acclimatise with repeated exercise-heat exposure over 1–2 weeks; longer regimens adapt most | Racinais 2015 consensus, PMID 26069301; Tyler 2016, PMID 27106556 (VPD-abs) | Consensus | Not from wrist temperature (UV) | Guidance |
| Altitude | Natural live-high train-low +4 % in open-label studies; double-blind trials found **no effect** | Bonetti & Hopkins 2009, PMID 19203133; Siebenmann 2012, PMID 22033534; Robach 2018, PMID 29469995 (VPD-abs) | Contested | No | Describe as contested |
| Footwear | Advanced shoes: VO2 g −0.44, time trials g −0.23 (17 crossover RCTs); the "4 %" figure is from a Nike-funded study | Xiao 2025, PMID 40527489; Hoogkamer 2018, PMID 29143929 (VPD-abs) | Moderate | Shoe tag only | Describe, funding noted |
| Cadence | No general economical technique; shortening stride up to ~3 % may help | Moore 2016, PMID 26816209 (VPD-abs) | Low–moderate | Cadence: yes | No cadence targets ("180" is lore) |
| Masters | Performance held to ~35, modest decline to 50–60, mainly via VO2max; economy unchanged with age | Tanaka & Seals 2008, PMID 17717011 (VPD-abs) | Review | — | Describe |
| Menstrual cycle | Trivially lower performance in the early follicular phase, very low certainty; authors advise individualising | McNulty 2020, PMID 32661839 (VPD-abs) | Very low | Cycle logging exists in Health | No phase-based prescription; logging only |
| REDs | IOC 2023 consensus and its clinical tool (REDs CAT2) | Mountjoy 2023, PMID 37752011 (VPD-abs; licence UV) | Consensus | No | Point to clinicians; never a score |

### 12.2 Lore to label as lore

The "Norwegian method" and double-threshold days (the only peer-reviewed source, Casado 2023,
PMID 36900796, is a model description with untested mechanisms, guided by blood lactate the watch
cannot measure); runner threshold HR from a 30-min time trial (no validation found); 3:1 load and
recovery weeks (none found); cadence 180; DFA-α1 from a wrist watch (§14).

## 13. Strength, power, team and hybrid sport

| Topic | Finding | Source, read | Licence | fm use |
|---|---|---|---|---|
| Periodisation for strength | Periodised beats non-periodised for 1RM (ES 0.31, CI 0.04–0.57); undulating beats linear only in trained lifters (ES 0.61, CI 0.00–1.22); hypertrophy unaffected. Linear = undulating in mostly untrained (Harries 2015) | Moesgaard 2022, PMID 35044672; Harries 2015, PMID 25268290 (VPD-abs) | © | Paraphrase |
| Autoregulation | RPE/RIR and velocity-based loading ≈ percentage loading for strength (1RM MD 2.07 kg, NS); velocity loss ≤ 25 % better for strength, > 25 % for size; a network meta-analysis ranking APRE first conflicts and is fragile | PMID 35038063 (CC BY); Helms 2018, PMID 29628895 (CC BY); PMID 40791980 (VPD-abs) | Mixed | **Use**; rankings as contested |
| Velocity loss and failure | Higher velocity loss: more size, less jump and sprint; strength flat across reps in reserve, size rises closer to failure | PMIDs 36178597 (CC BY), 38970765 (VPD-abs) | — | Use, matches ACSM 2026 |
| Strength → speed and jump | Squat strength gains track sprint gains (r = −0.77, ~3 % faster); plyometrics +8.7 % CMJ | Seitz 2014, PMID 25059334; Markovic 2007, PMID 17347316 (VPD-abs) | © / free | Use |
| Power | Strength first; ballistic at 0–50 % 1RM, weightlifting at 50–90 % | Cormie 2011, PMID 21244105 (VPD-abs) | © | Describe |
| Lifting taper | What lifters **report**: peak volume 4–6 weeks out, ~40–45 % volume cut over 5–9 days, last heavy deadlift ~1 week out — practice, not efficacy | PMID 42611173 (VPD-abs, CC BY) | CC BY | "What athletes report" |
| Repeated sprints | 3 × 6 × 30 m, 2× a week for 6 weeks improves VO2max, Yo-Yo, sprint and repeated-sprint ability; more volume added nothing (541 athletes) | PMID 38041768 (VPD-abs) | © | Use (protocol is a fact) |
| Team-sport tests and games | Yo-Yo IR1/IR2 valid and sensitive (Bangsbo 2008, PMID 18081366); small-sided games ≈ HIIT for endurance, very low certainty (PMID 37678559) | VPD-abs | © | Use the test; describe the games |
| In-season maintenance | 1 session a week, 1 set, intensity kept, holds strength up to 32 weeks in young adults; **no athlete data** | Spiering 2021, PMID 33629972 (VPD-abs) | © | Use with the caveat |
| Concurrent training | No loss of max strength or size; explosive strength blunted (SMD −0.28), worse in the same session than ≥ 3 h apart; running interferes more than cycling; lifting first gives +6.9 % lower-body strength | Schumann 2022, PMID 34757594 (CC BY); Wilson 2012, PMID 22002517; Eddens 2018, PMID 28917030 (CC BY) | CC BY | **Use** for hybrid weeks |
| What the watch can do for lifters | **Garmin wrist rep counts not valid** — no device met MAPE ≤ 10 % and CCC ≥ 0.7 on any exercise (lunges 44–67 % error); wrist HR in a free-weight circuit −3.3 % ± 16.7 % | IJES conference abstract 2023 (VPD, n = 20, low); PMID 30530451 (CC BY) | — | Strength sets are typed, not counted by the watch |
| Phone tools | My Jump valid for jump height (meta-analysis, PMID 37978338, CC BY; h = g·t²/8 from flight time, formula SH); barbell-velocity apps vary (Qwik ≈ a linear transducer; Metric and MyLift miss reps; PMIDs 39561127, 39071120, CC BY); Kinovea video alone disagrees with Open Barbell System (PMID 35291635) | VPD-abs | CC BY | A jump test fm can time from a slow-motion video is plain code; velocity tracking is not fm's job |

## 14. Monitoring and performance models

| Method | Evidence | Inputs | Open code (licence, VC) | Verdict |
|---|---|---|---|---|
| **Monitoring framework** | Internal vs external load side by side; no validated injury marker (Bourdon 2017, PMID 28463642; Soligard 2016, PMID 27535989; Impellizzeri 2019, PMID 30614348; Jones 2017 SR, PMID 27677917; all VPD-abs) | — | — | The design spine; **no risk score** |
| **Subjective measures** | Track load more sensitively and consistently than objective ones, and the two mostly do not correlate (Saw 2016, PMID 26423706, VPD-abs; 56 studies) | A phone question | — | **A daily check-in is the main readiness input**, ahead of any device score |
| Wellness questionnaire | Hooper (4 items, 1–7: sleep, stress, fatigue, soreness; item format SH) explained 76 % of staleness in 14 swimmers (PMID 7898325); McLean (5 items, SH) in 12 rugby players (PMID 20861526) | Questionnaire | — | One questionnaire, fm's own wording, scored against a 28-day personal baseline |
| **Monotony and strain** | Monotony = weekly mean daily load ÷ SD; strain = weekly load × monotony; illness clustered above each athlete's **own** strain threshold (Foster 1998, PMID 9662690, VPD-abs, n = 25) | Load per day | trivial | Plain code, own history only, guard SD = 0; no population cut-off |
| Mean-maximal curve, best efforts | Standard | Speed stream | GC (GPL) as reference | Plain code; reject GPS spikes; it feeds CS and the models below |
| **3-parameter model** (Morton) | Standard; needs ~3 maximal efforts (SH) | Speed or power | GC `CP3Model` (GPL, VC) | Plain code, extends the coach plan's CS fit; equation from the paper, not GC |
| Omni-domain model | One curve from sprint to ultra; prediction no better than 3-parameter (Puchowicz 2020, PMID 32131692, VPD-abs, n = 9) | Same | none | Only after the equation is read |
| **W′bal / D′bal** | Integral form: recovery time constant 546·e^(−0.01·DCP) + 316 s (Skiba 2012, PMID 22382171; field AUC 0.914, PMID 24509723); the integral form recovered most accurately in Black 2023 (PMID 36094337); in running D′bal separated exhaustive from non-exhaustive sessions (157 vs 200 m) but **did not reach 0 at exhaustion** (Bellenger 2025, PMID 40132598). All VPD-abs | Speed (or power) and CS/D′ | **sweatpy, MIT** (`w_prime_balance.py`: integral, Waterworth O(n), differential) | Plain code (port from MIT or the paper); shown as **relative depletion**, never a blow-up warning |
| Race prediction from training | Big-data case-based models: 5–8 % error (Feely/Smyth, CEUR 3418, CC BY; ~300k Strava runners); no open code or data | History | none | Treat 5–8 % as the honest band for any fm prediction |
| eFTP-style estimates | Curve families built from proprietary data; users report inflation from short efforts (SH, intervals.icu forum) | Power or pace | proprietary | Avoid; a "best efforts in N days" envelope instead |
| **DFA-α1 (aerobic threshold from HRV)** | α1 = 0.75 ≈ VT1 at group level (Rogers 2021, PMID 33519504, n = 15); individual limits about −12 to +10.5 bpm fresh, bias −12.3 bpm when fatigued (Van Hooren 2025, PMID 37916488); even a Polar H10 shows ±40–58 % LoA at high intensity (Schaffarczyk 2022, PMID 36081005); HR reliability ICC 0.52 (Sempere-Ruiz 2024). **No validation of wrist optical DFA-α1 found** | **Chest strap RR intervals** | FatMaxxer (Apache-2.0), pgalko's estimator (MIT), cardio-rs (MIT; DFA, ectopic filter) | **Strap only**; fresh-state ramps only; ±10 bpm stated; never from the wrist |
| HR acceleration, orthostatic test | rHRI falls with overreaching (SMD −0.48, PMID 26888648); HR recovery and post-exercise HRV rise with both good and bad adaptation, so alone they are ambiguous; orthostatic test not read | Strap preferred | none | Later, research only |
| Running power | Stryd underestimates metabolic power 2–4 % on 6–8 % grades (PMID 41722548, n = 10); comparisons SH | Footpod | proprietary | Do not build; read it from FIT if present; prefer GAP |
| Automatic interval detection | HR-only detection not sufficient without a person in the loop (thesis, SH) | — | — | Use FIT laps and workout steps; fall back to slicing on pace with confirmation |

### 14.1 Step map for an athlete mode

- **Plain code, wrist only:** best efforts and pace–duration curve; GAP (after Minetti is read);
  CS/D′ and the 3-parameter model; D′bal (relative); Smyth decoupling for long runs and races;
  monotony and strain on own history; the daily check-in with a personal baseline; Riegel to the
  half and Vickers for the marathon, with a 5–8 % band; consistency gaps (Feely); the pacing
  caution (start below ~94 % CS); intervals from FIT laps and steps.
- **Needs a chest strap or other hardware:** DFA-α1; strap-grade HRV and HR-kinetics tests; power
  (read, not computed). With a power meter, the same W′bal code runs on power. A BLE chest strap
  sends RR intervals by the standard Heart Rate Service (UV that this watch writes strap RR into
  FIT).
- **Learned model:** none worth shipping. Big-data predictors have no open code or data; a
  per-athlete nearest-neighbour model over one person's history would be weak.
- **Avoid:** ACWR and injury-risk scores; eFTP-style curve families; home-made running power;
  Xert-like signatures; DFA-α1 from fatigued sessions or the wrist; any absolute cut-off for
  monotony, wellness scores or decoupling.

## 15. Workouts and a calendar on the watch

**The owner's question:** can training sessions and calendars be loaded onto the Garmin, as
Connect does?

**What Connect does (VPD, the FR165-series manual, "About the Training Calendar").** The watch's
calendar extends the Connect calendar. Scheduled workouts sent to the watch show in a calendar
glance and can be started from it; if one is scheduled for today the watch prompts to start it;
a workout stays on the watch whether done or skipped; **"Sending workouts from Garmin Connect
overwrites the existing training calendar."** Training plans need a Connect account and a paired
phone. The transfer goes through the phone over Bluetooth; once on the watch, workouts run with no
phone nearby (by implication of the manual). Wi-Fi is reported on the Music model only (SH,
contradictory sources). How Connect fills the calendar (a FIT schedule file or a proprietary
message) is **UV**.

**What Gadgetbridge does (VC, Codeberg `master`).**
- Uploads FIT **workouts** since 0.82.0, plus courses, locations and segments; "manage workout
  templates" since 0.95.0 (CHANGELOG).
- `GarminFitFileInstallHandler.java` handles COURSES, WORKOUTS, LOCATION, SEGMENTS. **A FIT schedule
  file is not handled**: it installs only with the per-device "install unsupported files" setting
  (0.88.0, with a warning), and whether the watch then honours it is untested (UV).
- Accepts files from other apps by the Android share menu (`FileInstallerActivity`, `SEND` with
  `*/*`).
- Its "calendar sync" pushes phone calendar events, not a training calendar.

**USB.** Copying a workout FIT into `GARMIN/NewFiles` (some models `GARMIN/Workouts`) is reported in
forums, along with failures; one report of dated schedule files appearing in the calendar. Nothing
documented by Garmin for current Forerunners (SH/UV).

**FIT writing in Rust.** rustyfit (BSD-3, VC) decodes and encodes, generated from Garmin's profile;
its README warns that use may require the FIT Protocol License — so D4/D5 still decide.
intervals.icu schedules workouts but reaches the watch through Connect (SH).

**Answer.** Workouts without Connect: **yes, by a verified chain** — fm writes a type-5 workout →
share → Gadgetbridge → the watch's workout list. A training calendar and "today's workout" without
Connect: **not verified anywhere**. Two untested routes (a schedule file through GB's "unsupported"
setting, or USB) need one test on a real watch on a throwaway setup. Until then fm holds the
calendar and sends each session as a workout a day or two ahead.

## 16. Nutrition and recovery

| Topic | Finding | Source, read | Licence | fm use |
|---|---|---|---|---|
| Supplements with good evidence | Caffeine 3–6 mg/kg ~60 min before (≥ 9 mg/kg no extra gain); creatine 20 g/day for 5–7 days then 3–5 g/day (1–2 kg mass gain); nitrate 310–560 mg 2–3 h before; beta-alanine ~65 mg/kg/day for 10–12 weeks (~0.2–3 %); bicarbonate. Contamination and doping risk for all | IOC 2018, PMID 29540367 (VPD, Tables 3, 5, 6) | **CC BY-NC** | Paraphrase facts, cite; no tables copied |
| Nitrate limits | d = 0.174 overall; **no effect in women-only studies or well-trained endurance athletes** | Senefeld 2020, PMID 32936597 (VPD-abs) | CC BY-NC-ND | State the caveat |
| ISSN position stands | Protein 1.4–2.0 g/kg/day (2.3–3.1 when cutting), 20–40 g doses every 3–4 h; caffeine 3–6 mg/kg; bicarbonate 0.2–0.3 g/kg 60–180 min before (GI warning); creatine safe to 30 g/day for 5 years in healthy people | PMIDs 28642676, 33388079, 34503527, 28615996 (VPD-abs) | **CC BY 4.0** | **Use**, with attribution |
| Protein ceiling | No extra fat-free mass beyond ~1.62 g/kg/day (49 RCTs) | Morton 2018, PMID 28698222 (VPD-abs) | CC BY-NC | Paraphrase |
| Train low | Signalling improves; performance improved in 37 % of 11 studies (Impey 2018, PMID 29453741); pooled SMD 0.17 (CI −0.15 to 0.49) in trained endurance athletes (Gejl & Nybo 2021, PMID 34001184) | VPD-abs | **CC BY** | A hypothesis, not a rule |
| AND/DC/ACSM 2016 | Carbohydrate and protein ranges **not read** (abstract has no numbers) | PMID 26891166 | © | Full text needed |
| Hydration | Sweat rate from body mass before and after plus fluid drunk minus urine, per hour; avoid losing > 2 % body mass | ACSM 2007, PMID 17277604; Baker 2017, PMID 28332116 (CC BY) | Formula is a fact | **Use**: a field calculation fm can do |
| Low energy availability | LEAF-Q cannot classify high risk; only rules out (Rogers 2021, PMID 34284349) | VPD-abs | © | Never a score; refer |
| Cold-water immersion | After strength sessions it blunts strength and size (Roberts 2015, PMID 26174323; regular use: strength SMD −0.60, endurance ~0, Malta 2021, PMID 33146851); helps 24-h power recovery (SMD 0.34, PMID 35157264) | VPD-abs | — | **Use**: not after strength days |
| Massage, compression, foam rolling | Massage best for soreness and fatigue (Dupuy 2018, PMID 29755363, CC BY); compression ES 0.38 (PMID 28434152); foam rolling minor, more a warm-up (PMID 31024339, CC BY) | VPD-abs | Mixed | Describe; subjective outcomes |
| NSAIDs | 1200 mg/day ibuprofen halved quadriceps growth vs low-dose aspirin (+3.7 % vs +7.5 %) | Lilja 2018, PMID 28834248 (VPD-abs) | © | A caution in strength blocks |
| Sleep extension | ≥ 10 h in bed improved sprint and shooting in **11** players with no control group (Mah 2011); a 2023 review ranks extension and naps best (PMID 37462808, CC BY) | VPD-abs | Mixed | Use, with Mah's limits |
| Youth | NSCA long-term athletic development: all youth are athletes, individualised development (PMID 26933920); UKSCA youth consensus not read (PMID 24055781) | VPD-abs | © | Describe; full text needed |

## 17. Athlete canon (proposed)

1. **Most volume is easy, and volume is the strongest training correlate of performance** —
   consistent across every dataset, but observational.
2. **Pyramidal or polarised both work**; marathoners run more pyramidal.
3. **Total load beats clever ordering**; periodisation models are not performance levers, though
   planned variation beats none for strength.
4. **Accumulate time at high but sustainable intensity** (4 × 8 min beat 4 × 4 and 4 × 16 in one
   RCT); short vs long intervals is contested.
5. **Strength 2–3× a week for endurance athletes** (economy 2–8 %, VO2max unharmed); for hybrid
   athletes, separate sessions by ≥ 3 h or lift first.
6. **Autoregulate strength by reps left or bar speed** — as good as percentages, not magic; stop
   further from failure for strength and speed, closer for size.
7. **Consistency**: long breaks before a race cost 5–8 %.
8. **Durability is measurable** from watch data (Smyth decoupling) and **start races
   conservatively** (below ~94 % CS for the marathon).
9. **Fuel and recover by evidence**: ~1.6 g/kg protein; five supplements with evidence, small
   effects, trial first; train-low a hypothesis; sleep first; no cold water after strength days;
   weigh before and after for sweat rate.
10. **Subjective check-in first, device scores second**; no injury-risk prediction; low energy
    availability and red flags go to a clinician.

## 18. What Part 2 adds for fm — directions, none decided

- **An athlete mode on the same notes.** The plain-code list in §14.1 needs nothing beyond the
  watch, and most of it sits on the coach plan's P1 (import) and P4 (thresholds). D′bal,
  decoupling, monotony, best efforts and pacing caution are each a small Rust function with one
  primary source.
- **The daily check-in** (fm's own wording, personal baseline) is the readiness input the evidence
  favours. It also feeds strength autoregulation and the Domestique-style rule chain (§8.2).
- **Hybrid weeks** — endurance and strength in one budget with session spacing — are both
  evidence-backed and missing from every open tool (§4.3 gap 3).
- **Strength sessions are typed**, because the watch's rep counts and wrist HR in lifting are not
  valid; jump height from a phone video is a cheap, valid performance test.
- **The watch loop:** workouts by Gadgetbridge now; the calendar after one test (§15).
- **Optional hardware tier:** a chest strap unlocks DFA-α1 and true HRV; a power meter or footpod is
  read, never imitated.

**More decision entries this would need** (adding to §8.4): *An athlete mode* (which of §14.1 ships
and with which sources); *A daily check-in* (items, wording, baseline); *Hybrid weeks* (spacing
and order rules); *A chest strap as an optional source* (RR intervals, DFA-α1 with its limits);
*Nutrition facts* (whether fm carries supplement and protein figures at all, and only from CC BY
sources or paraphrased); *The watch calendar* (after the schedule-file test).

## 19. Part 2: not done, and sources

**Not done.** Abstracts only for nearly every row; full texts listed below. Not reached: 30-15 IFT
for runners beyond one team-sport study, weightlifting-derivative meta-analyses, GPS external load
in team sport, the orthostatic test, Peronnet–Thibault, the omni-domain equation, Skiba 2015 in
full, the Feely/Smyth full papers, ISSN beta-alanine.

**Full texts still needed.** Smyth 2022 (exact decoupling formula, HRmax source, devices);
Casado 2023; Muniz-Pumares 2025 (the 82.3 % CS zone boundary); Mølmen 2019 (which studies were
pooled); Tyler 2016 (effects by regimen length); Racinais 2015 and Périard 2021; Maunder 2021 and
Jones 2024 (field durability tests); Rothschild 2025 (decoupling vs VT1, PMID 40402269); Mountjoy
2023 (REDs CAT2, licence); AND/DC/ACSM 2016; Lloyd 2014 youth consensus; Weakley 2021 VBT
(technology section; NSCA "unauthorized reproduction prohibited"); Malta 2021; Emig 2020 and
Haugen 2022 (remainder); Bonetti 2009; the Garmin manual's rep-counting wording; a Garmin-specific
rep-counting study beyond the 2023 abstract.

**PubMed read (abstracts unless noted).** Endurance: 31802956, 20199119, 35445953, 27300278,
35418513, 35362850 (most of full text), 36900796, 21812820, 16177614, 29745765, 22233784,
32472926, 39616560, 31704026, 33024098 (part of full text), 36704260, 9927024, 11219499,
23539308, 23832851, 18550949, 24382021, 33735833, 40328438, 26694507, 27497600, 29249083,
23914932, 40527489, 29143929, 26816209, 40964543, 33886100, 37606604, 35511416, 40402269,
40150840, 34010308, 33519504, 35615679, 36081005, 29557279, 30922191, 18278984, 26069301,
27106556, 25943654, 19203133, 22033534, 29469995, 35157264, 28679683, 26041108, 32661839,
37752011, 17717011; Sempere-Ruiz 2024 (doi 10.3389/fphys.2024.1329360, part of full text).
Strength, sport, nutrition, recovery: 25268290, 35044672, 29786623, 29628895, 42611173, 32172680,
25059334, 17347316, 40791980, 35038063, 36178597, 38970765, 21244105, 38041768, 37678559,
18081366, 33629972, 22002517, 34757594, 28917030, 29540367 (Tables 2–6), 28698222, 28642676,
33388079, 34503527, 28615996, 32936597, 29453741, 34001184, 26891166, 17277604, 28332116,
34284349, 26174323, 33146851, 42816732, 29755363, 28434152, 31024339, 28834248, 21731144,
37462808, 24055781, 26933920, 34564323, 30530451, 37978338, 39561127, 39071120, 38015739,
35291635. Monitoring and models: 9662690, 28463642, 27535989, 30614348, 26423706, 27677917,
7898325, 20861526, 25555023, 26888648, 26209427, 22382171, 24509723, 34686611, 36094337,
40132598, 41931241, 32131692, 41722548, 12183501, 37916488, 33530473.

**Other pages.** IJES conference abstract (digitalcommons.wku.edu/ijesab/vol14/iss3/143); CEUR
Vol-3418 ICCBR 2022 DC paper 36; Garmin FR165-series manual, training calendar page; Gadgetbridge
CHANGELOG, `GarminFitFileInstallHandler.java`, AndroidManifest, gadgetbridge.org Garmin topic;
sweatpy `w_prime_balance.py` (MIT); GoldenCheetah `PDModel.h` (GPL); FatMaxxer (Apache-2.0);
pgalko DFA-α1 estimator (MIT); cardio-rs (lib.rs); rustyfit; Europe PMC full text of Emig 2020 and
Haugen 2022. SH only: intervals.icu eFTP thread, TrainingPeaks decoupling article, Stryd
comparisons, a Univie thesis on interval detection, Garmin forum on NewFiles, FR165 spec reviews.

---

# Part 3 — Cycling, swimming and triathlon, the gym, and nutrition (2026-11-20)

Four research passes, same labels. Most rows are **VPD-abs**; numbers came through summarising
fetches and must be re-read in the original before they enter the register. What Parts 1–2 and
the coach plan already hold is not repeated.

## 20. Cycling

### 20.1 What this watch class records on a bike

- **Profiles:** "Bike" and "Bike Indoor" only (SH, review sites).
- **Sensors** (VPD, FR165-series manual, "Wireless Sensors"): external HR, foot pod, headphones,
  bike lights and radar, a bike **speed or cadence** sensor, a temperature sensor, over ANT+ or
  Bluetooth. **No power meters and no smart trainers** in that list; a review states the watch
  supports neither (SH). The radio is there; the firmware feature is not.
- **In the FIT file** (inference, UV until a real file is read): HR, GPS position and speed,
  barometric altitude and distance on every ride; cadence and wheel speed only with a sensor; never
  power. An indoor ride with no sensor is HR and time only.
- **Gadgetbridge** passes the raw FIT through, and can upload courses and workouts; a bike workout
  on this watch can only target HR, speed, cadence or time (UV which).

**So on this watch, cycling is HR plus GPS and altitude (plus cadence with a sensor). Every
power-based feature needs another device** — a watch or bike computer that records power.

### 20.2 Evidence

| Topic | Finding | Source, read | fm use |
|---|---|---|---|
| **"FTP" (20 min × 0.95)** | Group means sit near MLSS or the 4 mmol lactate threshold, but individual limits of agreement are about ±40–60 W (Borszcz 2018, PMID 29801189; Jeffries 2021, PMID 31269000); MLSS ≈ 91–93 % of FTP, not 95 % (Lillo-Beviá 2022, PMID 31714462; Inglis 2019, PMID 31689684); FTP **missed a 12 W rise in MLSS** over 7 months (Inglis); time to exhaustion at FTP 35–51 min by level (Sitko 2022, PMID 35835698); FTP ≠ critical power (Karsten 2021, PMID 33551839, CC BY; Morgan 2019, PMID 30387374). All VPD-abs | A practical benchmark, **not** a physiological threshold; power meter only |
| Test reliability | 20-min test CV 2.9 %, ICC 0.97 (PMID 31952081); depends on warm-up (PMID 34749416); 4- and 20-min tests track 60-min power, R² 0.95/0.92 (MacInnis 2019, PMID 29809063) | VPD-abs | Protocol cards |
| **Critical-power field protocol** | 3–5 trials of 2–15 min, ≥ 30 min apart or on separate days; non-linear 2-parameter fit; avoid 2-trial fits, single-effort FTP and the 3-min all-out test; race-derived W′ is poor | Leo 2022, PMID 34708276 (**VPD full text, CC BY 4.0**) | **The protocol card** for anyone with power |
| 3-min all-out test | Overestimates CP (meta-analysis g = 0.41, n = 284; Quittmann 2026, PMID 42479066); fine in active, not trained riders (Vanhatalo 2007, PMID 17473782; Bartram 2017, PMID 27834562) | VPD-abs | Avoid as a CP source |
| Coggan levels, sweet spot, ramp test, normalised power | Levels are a practitioner convention with no peer-reviewed validation (TrainingPeaks page, VPD); no cycling study of "sweet spot" found; no ramp-test validation found; the NP algorithm is from a book, and its load tracked fitness change like iTRIMP (r .75–.79 vs .77–.81; Sanders 2017, PMID 28095061) | — | Convention or lore, labelled so |
| **Trademarks (new)** | NORMALIZED POWER reported as US Reg. 4450849 (2013) and "NP" with the same status; TrainingPeaks claims ® for TSS, IF, NP and their long forms; **FTP is not claimed** (SH via a registry mirror; check USPTO TSDR) | SH | fm keeps neutral names (coach plan) |
| Wrist HR on a bike | Mid accuracy; Garmin among the better devices; error rises with intensity, BMI and darker skin; latency explains part of the error (Gillinov 2017, PMID 28709155; Budig 2021, PMID 35009723; Støve 2020, PMID 32404012; Kostrna 2026, PMID 42197732) | VPD-abs | Flag low-confidence stretches; suggest a strap for intervals |
| Separate zones per sport | Cycling and running HR and thresholds differ (Millet 2009, PMID 19290675) | VPD-abs | **Per-sport zones** |
| Indoor ≠ outdoor | Outdoor power higher at 1–14 min; CP +19 W outdoors in 14 pros (Lipski 2022, PMID 35361736) | VPD-abs | **Never pool indoor and outdoor bests** |
| Cadence | Elite riders most efficient around 80–100 rpm (Foss 2005, PMID 15503124); second-half cadence decline tracks HR drift (+0.61 % per rpm; one study, PMID 41923151) | VPD-abs | Describe; drift hint experimental |
| Durability | Better pros hold power after 2000 kJ or 10–30 kJ/kg (Spragg 2023, PMID 35239466; Mateo-March 2025, PMID 40148210; Muriel 2022, PMID 34586952) | VPD-abs, observational | With power: "best power after X kJ/kg" |
| Sprint intervals | VO2max g = 0.63 (Sloth 2013, PMID 23889316), equal to endurance training (Gist 2014, PMID 24129784), in untrained to recreational people | VPD-abs | Use |
| Pacing | Even pacing for time trials > 10 min in steady conditions (Atkinson 2007, PMID 17645369) | VPD-abs | Describe |
| Power without a meter | The physics model (Martin 1998, PMID 28121252) gives R² 0.97 **with measured inputs**; with guessed drag, wind and drafting it is unreliable (Strava estimates 10–30 % off, SH) | VPD-abs | Experimental only, long-segment averages with a range; or not at all |
| Aero and bike fit | Drag area best predicts flat time-trial time (Peterman 2015, PMID 26290797, CC BY); bike-fit evidence mostly poor — only saddle height, trunk angle and saddle fore-aft clear (Husband 2024 SR, PMID 39285616) | VPD-abs | Describe |
| Health cards | Low bone density in road cyclists, mainly the spine (Nagle 2011, PMID 22164312; Abrahin 2016, PMID 27476628) → add loading or impact work; helmets: head injury OR 0.49, fatal 0.35 (Olivier 2017, PMID 27450862); active travel's benefits outweigh crash and pollution risk, median ratio 9 (Mueller 2015, PMID 25900805); saddle pressure eased by noseless saddles (PMID 33074460) | VPD-abs | Content cards |

### 20.3 Open cycling tools

GoldenCheetah (GPL; one pass read GPL-3.0 from the repo API, the coach plan read GPL-2.0-or-later
from file headers — either way black box) has Aerolab virtual elevation and CP models. Trainer
bridges: qdomyos-zwift (GPL-3.0), SmartSpin2k (GPL-2.0), FortiusANT (GPL-3.0), Gymnasticon (MIT,
stale 2022). pycycling (MIT) is a Python BLE reference. BRouter (MIT) is an offline, elevation-aware
bike router. Virtual-elevation scripts are GPL or AGPL. No open Rust cycling-physics crate worth
depending on; implement from papers. All VC (repo API).

### 20.4 Step map

- **Wrist only:** per-sport cycling HR zones; time in zone split by Bike and Bike Indoor; wrist-HR
  confidence flags (first ~10 min, sharp transitions); HR drift on steady flat or indoor rides
  (experimental); cadence analytics with a sensor; climb detection and climbing rate (vertical
  metres per hour) from the barometer; content cards.
- **With power (another device):** CP/W′ by the Leo 2022 protocol; zones as % CP; mean-maximal power
  curve (indoor and outdoor apart); durability panel; W′bal (Part 2 §14); virtual elevation.
- **Avoid:** NP/TSS/IF names; 3-min-test W′; ramp-test FTP as a threshold; "sweet spot" as
  evidence; anything implying power on this watch; GPL code.

## 21. Swimming

### 21.1 What the watch records, and what reaches fm

- **Pool swim** (VPD, FR165 owner's manual pp. 8–10): pool size set by the person; distance =
  counted lengths × pool size; intervals and lengths recorded automatically; a "stroke" is one cycle
  of the watch arm; SWOLF = length time (s) + strokes; stroke type per length (free, back, breast,
  fly, mixed, drill); auto rest after 15 s; a drill log where the person types the distance.
- **Open water:** GPS distance, pace, stroke rate.
- **HR:** wrist HR on by default for swims; a strap takes priority.
- **Not on this watch:** no triathlon/multisport mode and no CSS test in the manual (sibling models
  have both). Swim workouts on this watch: UV.
- **FIT** (VPD, Garmin FIT cookbook): one **Length** message per length (time, strokes, stroke
  type, active or idle); laps group lengths; idle lengths are rest; `pool_length` in metres.
- **Gadgetbridge drops Length messages** (VC, `GarminWorkoutParser.java`): it reads only pool
  length, average SWOLF and strokes per length from the session, and stroke, distance, time and HR
  from laps. It keeps the raw FIT path, so **fm reads Length messages from the original file.**
  Its workout encoder (AGPL) writes pool-swim workouts with drills, stroke targets and rest.

### 21.2 How far the swim data can be trusted

| Metric | Evidence | Verdict |
|---|---|---|
| Lengths and distance, steady freestyle | Two Garmin watches: 0 % error over 400 m freestyle (Jacko 2024, PMC11281193, **VPD full text, CC BY**); lap detection 99.7–100 % (Delgado-Gonzalo 2016) | **Measured** |
| Distance with stroke changes | 200 m medley: **91.7 % error** for both Garmins; stroke changes mid-length counted as new lengths (Jacko 2024) | Flag mixed-stroke intervals |
| Stroke count | Garmin differed from video on all but 2 occasions (Mooney 2017, PMID from PMC5298290, VPD-abs); 0.4–29.5 % per-length error across watches (Jacko 2024) | Watch's estimate; one-arm count |
| **Wrist HR in water** | Garmin vs chest strap in water: bias −44 bpm, range −117 to +30, MAPE 30 % (on land 4 %) (Cosoli 2022, PMC9269005, **VPD full text, CC BY**); another wrist watch −53 to +34 bpm (Olstad 2020, Polar-funded) | **Not valid.** Shown with a warning; never used for zones or load |
| Open-water GPS distance | No consumer-watch validation found; evidence limited (Seelhöfer 2026 SR) | "Not validated for this device" |
| SWOLF | No validation as an efficiency index; depends on pool length and stroke definition | Descriptive, same pool only |

### 21.3 Swim training science

| Topic | Finding | Source, read | fm use |
|---|---|---|---|
| **Critical swim speed (CSS)** | Slope of distance against time from maximal swims; two-point form CSS = (400 − 200) / (t₄₀₀ − t₂₀₀) m/s ≈ maximal lactate steady state (Wakayoshi 1992, PMID 1555562; 1993, PMID 8425518); but Dekerle 2005 (PMID 16195984) found CSS a little **above** it (92.7 % vs 88.3 % of maximal aerobic speed); 30-min speed ≈ CSS, which overestimates it by 3.2 % (Dekerle 2002, PMID 11842355) | VPD-abs | **Use**, saying it sits slightly above the steady state |
| Swim zones | Pace per 100 m relative to CSS, time in zone from Length data; brand "T-pace" boundaries are lore | — | Plain code; boundaries labelled convention |
| Session RPE in swimming | r = 0.55–0.94 against HR methods in 12 competitive swimmers (Wallace 2009, PMID 19002069) | VPD-abs | **The load method for swims** (HR is invalid in water) |
| Intensity distribution | Depends on event distance; mostly retrospective (González-Ravé 2021, PMID 33952709) | VPD-abs | Describe |
| Dryland strength | Resistance training improves 25–200 m front crawl via stroke rate (Jin 2024, PMID 39129754, 13 studies); all three types ~2–2.5 % (Fone 2022, PMID 35099631); plyometrics g 0.38, resistance 0.29, core 0.20 in developing swimmers (Mou 2026, PMID 43000402) | VPD-abs | Swim plus strength beats swim alone |
| Taper | About +2 % over the last 3 weeks in Olympic swimmers (Mujika 2002, PMID 12439774) | VPD-abs | Describe |
| Shoulder | Pain prevalence up to 91 % in adolescent swimmers, linked to volume (Feijen 2020, PMID 31935141); load–injury links poor quality (Barry 2021, PMID 33486408) | VPD-abs | A volume-spike note, descriptive; no thresholds |
| Swimming for health | All-cause mortality HR 0.72 (Oja 2017, PMID 27895075; 80,306 adults), 0.76 in a 2024 meta-analysis (PMID 38658416); observational. Aquatic exercise for knee and hip osteoarthritis: pain SMD −0.31 (Bartels 2016 Cochrane, PMID 27007113) | VPD-abs | Describe; "swimmers outlive runners" (Chase 2008) is SH and pool-funded — avoid |
| Open-water safety | Cold water causes exhaustion before hypothermia (Tipton 2014, PMID 23377278) | VPD-abs | Safety text |

## 22. Triathlon and load across sports

| Topic | Finding | Source, read | fm use |
|---|---|---|---|
| Run after bike | Running energy cost +1.6 % to +11.6 %, varying by ability (Millet & Vleck 2000, PMID 11049151); individual (Bonacci 2010, PMID 20359948); no change in 7 elites (Bonacci 2011, PMID 21560752) | VPD-abs | Describe |
| Brick sessions | Acute effects only (Olcina 2019, PMID 31100906); **no training study shows bricks improve race performance** | VPD-abs | No such claim |
| Intensity distribution | Ironman athletes 69/25/6 % across three zones (Neal 2011, PMID 21967604, n = 10) | VPD-abs | Describe |
| Swim pacing | Swim at ~80–90 % of a maximal swim test to protect bike and run (Peeling 2009, PMID 19847691) | VPD-abs | Describe |
| Injuries | Short course 15.7–24.3 per 1000 exposures, 45–92 % while running (Guevara 2024, PMID 36898525) | VPD-abs | Describe |

**Can load be added up across sports? Not on a validated common scale.**
1. Session RPE is valid **within** many sports (Haddad 2017 review, PMC5673663, CC BY; Wallace 2009
   for swimming), but no study found tests whether one unit in the pool equals one on the bike or
   the run. Every triathlon study found sums load without validating the sum.
2. HR load does not carry across: maximal HR swimming was 13 % lower than running (162 vs 186 bpm,
   Meadley 2021, PMID 33821772), and swim HR on this watch is invalid anyway.
3. Sport weighting factors (run 1.0, swim 0.75, bike 0.5) are asserted without derivation
   (Esteve-Lanao 2017, PMC5447032, VPD full text).
4. "Swim TSS" (IF³ × hours × 100) and SwimScore/TriScore are lore or a white paper; platforms differ
   by about 2× (SH); GoldenCheetah's `SwimScore.cpp` (VC, GPL) is a black-box reference only.

**Proposed design:** one panel per sport with its own anchor (run CS, swim CSS, bike HR — or power
with another device); per-sport volume and time in zone; a combined panel that shows **only time per
sport and RPE × minutes**, captioned "not validated across sports"; **no single cross-sport fitness
line.** With no multisport mode on this watch, a triathlon day is same-day sessions grouped.

## 23. The gym, for athletes

### 23.1 Evidence

| Topic | Finding | Source, read | fm use |
|---|---|---|---|
| **Strength training prevents injury** | Sports injuries RR 0.32 (0.21–0.48); stretching 0.96; proprioception 0.55; overuse 0.53 (25 RCTs, 26,610 people) | Lauersen 2014, PMID 24100287 (VPD-abs) | **The headline** |
| ACL programmes | All ACL −50 %; non-contact ACL in women −67 %; male data insufficient | Webster & Hewett 2018, PMID 29737024 | **Use** |
| Copenhagen adduction | Groin problems 13.5 % vs 21.3 %, OR 0.59 (cluster RCT, 652 footballers); 3×/week preseason then 1×/week; dose table SH from the trial registry | Harøy 2019, PMID 29891614 | **Use**; full text for the dose |
| Shoulder (OSTRC) | Shoulder problems 17 % vs 23 % in 660 elite handball players | Andersson 2017, PMID 27313171 | Describe; own exercise cards |
| Tendons | Heavy slow resistance = eccentric for Achilles, better compliance (92 % vs 78 %) and satisfaction (Beyer 2015, PMID 26018970); good for patellar tendon (Kongsgaard 2009, PMID 19793213) | VPD-abs | Rehab-framed cards, not diagnosis |
| Hip strength in runners | Did not reduce injury in a pilot RCT (Nguyen 2024, PMID 38251299) | VPD-abs | Not claimed |
| Free weights vs machines | No difference in dynamic or isometric strength or size; gains specific to the mode trained (Haugen 2023, PMID 37582807, 13 studies) | VPD-abs | Choose by goal and test |
| One- vs two-legged | Size the same; strength follows specificity (Kassiano 2025, PMID 39794667; Liao 2022, PMID 35959319) | VPD-abs | Use |
| Range of motion | Full beats partial for strength (ES 0.56) and leg size (0.88) (Pallarés 2021, PMID 34170576); lengthened partials an option (SH) | VPD-abs | Full ROM default |
| Order, rest, tempo | Priority lift first (Nunes 2021, PMID 32077380); rest ≥ 90 s (Singer 2024, PMID 39205815); rep speed 0.5–8 s does not matter, > 10 s worse (Schoenfeld 2015, PMID 25601394) | VPD-abs | Defaults; tempo optional |
| Isometrics | Long muscle lengths for size; ≥ 70 % MVC for tendon (Oranchuk 2019, PMID 30580468); dosing by goal (Lum 2019, PMID 30943568) | VPD-abs | Use |
| Flywheel | ES 0.63 vs weight stack (Maroto-Izquierdo 2017, PMID 28385560) | VPD-abs | Describe (equipment) |
| Blood-flow restriction | 20–40 % 1RM, 40–80 % of measured occlusion pressure, 30-15-15-15, 2–3×/week; adverse events rare in small, mostly young-male samples; one author with a commercial interest | Patterson 2019, PMID 31156448 (**VPD part full text, CC BY**) | Describe with a safety card |
| Sport-specific | Cyclists: heavy strength improves efficiency (ES 0.35) and performance (0.46), low certainty (Llanos-Lagos 2026, PMID 40632222); swimmers §21.3; runners Part 2 §12 | VPD-abs | Use |
| Treadmill | 1 % grade matches road running cost at 3.75 m/s (Jones & Doust 1996, PMID 8887211; 9 trained men, short runs) | VPD-abs | Use with its limits |
| Rower VO2max | Concept2's equations: unpublished data, no peer-reviewed citation, "estimate only" (VPD page) | — | Describe only |
| CrossFit-style classes | No significant pooled benefit (only 2 of 31 studies high quality, PMID 29484512); injury prevalence 35 %, shoulder and spine most (PMID 33322981); newcomers at higher risk. HYROX: no evidence found | VPD-abs | Describe honestly |
| Tests | Isometric mid-thigh pull reliable (ICC ~0.96, PMID 35309521), but isometric tests under-report dynamic gains (Saeterbakken 2025, PMID 40314751); sit-and-reach measures hamstrings, not the lower back (PMID 24570599) | VPD-abs | Use with labels |

### 23.2 Logging gym sessions

- **The watch records sets** (VPD, FR165 manual, "Recording a Strength Training Activity"): a
  strength profile; reps counted (shown after 4); a set ends with BACK, then a rest timer; reps and
  **weight** can be edited during rest; one move per set counted; workouts built in Connect.
- **FIT `set` messages** (VC via rustyfit's generated profile): duration, repetitions, weight (kg,
  scale 16), set type (active or rest), category and exercise name, workout step index.
- **Gadgetbridge reads sets** (VC, `GarminWorkoutParser.java`, `FitActivityTrackProvider.java`):
  active sets with exercise name, reps, weight and duration; it can also **send strength workouts**
  with exercise steps, weights and rest (`GarminWorkoutFitEncoder.kt`, AGPL).
- **So gym sessions can arrive as files.** Wrist rep counts are not valid (Part 2 §13), so reps stay
  editable in the note and the note's values win.
- **Other apps** (SH; no official specs): Strong and Hevy CSV exports carry date, exercise, set
  order, weight, reps, RPE; Hevy imports Strong's format. fm could detect the format by header.

### 23.3 Programme names and licences

Copyright does not cover "any idea, procedure, process, system, method of operation" (US Copyright
Office Circular 33, VPD). A set-and-rep scheme is a method; books, prose and tables are protected;
names may be trademarks. "STARTING STRENGTH" is reported as registered and "JIM WENDLER 5/3/1" has
filings (SH, registry mirrors). **fm offers generic schemes in its own words** ("linear progression
3 × 5", "three rising weeks then a deload") and never uses a programme's name as a label. US Army
FM 7-22 is marked "approved for public release" (SH); as a US government work it may be a source of
exercise descriptions, each figure checked for third-party credit.

### 23.4 Gym canon for athletes

1. Strength training prevents injury (RR 0.32); it is the strongest case for the gym.
2. Specificity: pick free weight or machine, one leg or two, dynamic or isometric by the task.
3. Full range of motion by default; priority lift first; rest ≥ 90 s; tempo is not a lever.
4. Targeted prevention for the sport: ACL programmes, Copenhagen adduction, shoulder programme.
5. Heavy slow loading for tendons.
6. Endurance athletes lift heavy too (runners, cyclists, swimmers); preseason 3×/week, in season
   1×/week.

## 24. Nutrition coupled with training

### 24.1 Evidence

| Topic | Finding | Source, read | Licence | fm use |
|---|---|---|---|---|
| **Daily carbohydrate by load** | Light 3–5 g/kg/day; moderate (~1 h/day) 5–7; 1–3 h/day 6–10; > 4–5 h/day 8–12. Targets matter on days of key work; other days can follow energy goals | AND/DC/ACSM 2016 joint position, PMID 26891166 (full text read **from an unofficial copy**, the journals being paywalled — re-read the official text before shipping) | © Academy: numbers with citation, never the tables | **Use**: Rust picks the band from planned hours |
| Around sessions and races | Before (> 60 min): 1–4 g/kg 1–4 h before. During: < 45 min none; 45–75 min small amounts or a mouth rinse; 1–2.5 h 30–60 g/h; > 2.5–3 h up to 90 g/h from glucose–fructose mixes. Loading for > 90 min events: 10–12 g/kg for 36–48 h. Two sessions < 8 h apart: 1–1.2 g/kg/h for 4 h | Same; Burke 2011, PMID 21660838; Jeukendrup 2014, PMID 24791914 (open) | — | **Use** |
| Protein | 1.2–2.0 g/kg/day; ~0.3 g/kg (15–25 g) after key sessions and every 3–5 h; higher in a deficit or injury | AND/DC/ACSM 2016 | — | Use (Part 2 §16 has the ISSN and Morton figures) |
| Fluids | 5–10 mL/kg 2–4 h before; keep losses < 2 % body mass; sodium when sweat > 1.2 L/h, salty sweat or > 2 h; **over-drinking is the main cause of exercise-associated hyponatraemia** | AND/DC/ACSM 2016 | — | Use, with the sweat-rate method |
| 90 → 120 g/h | Upper limit "could increase"; field doses "not yet substantiated" (Morton 2026, PMID 41759826); tolerable in 9 trained men (Hearris 2022, PMID 35446596); a 2026 critique | VPD-abs | — | Opt-in, labelled contested |
| Gut training | Gut discomfort −47 % after 2 weeks of feeding in training (Martinez 2023, PMID 37061651) | VPD-abs | open | "Rehearse race fuelling in long sessions" |
| Periodised nutrition | "Specific workouts accompanied by specific nutritional goals"; ketogenic diets: "not a single study has demonstrated performance benefits" (Jeukendrup 2017, PMC5371625, CC BY, sports-drink-industry funded) | VPD part full text | CC BY | Use with attribution |
| **Energy availability** | EA = (intake − exercise energy) / fat-free mass; 45 ≈ balance, < 30 impairs function (AND/DC/ACSM 2016). The 30 comes from 3–5-day studies in young lean sedentary women; debated in women, unknown in men; field measurement "difficult and prone to errors" (Areta 2021, PMID 33095376, CC BY) | VPD | — | Concept only; **never a computed verdict** |
| REDs tools | The IOC clinical tool has "no singular validated diagnostic test" (Stellingwerff 2023, PMID 37752002); resting-metabolism ratios depend on the equation, 1–68 % prevalence (Heikura 2025, PMID 40262739) | VPD-abs | © | Never implement in the app; refer |
| Weight loss for athletes | ~0.7 %/week kept lean mass (+2.1 %), faster lost it (Garthe 2011, PMID 21558571, n = 24); ~2.3 g/kg protein in a deficit (Mettler 2010, PMID 19927027, n = 20); 250–500 kcal/day for 3–6 weeks in the base phase (AND/DC/ACSM) | VPD-abs | — | A rate caution, not a target |
| Diet type | Low-fat vs low-carb: no difference at 12 months (DIETFITS, n = 609, PMID 29466592); adherence decides (ISSN 2017, PMID 28630601, CC BY) | VPD-abs | — | No macro dogma |
| Weight changes slowly | Half-time about a year (Hall 2011, PMID 21872751) | VPD-abs | — | Show trends; never forecast kg from kcal |
| **Calorie tracking and eating disorders** | Associations with symptoms (Simpson & Mazzeo 2017, PMID 28214452; Linardon 2019, PMID 30772765; Messer 2021, PMID 34543856 — worse when the motive is weight or shape); ~75 % of eating-disorder patients used a tracker (Levinson 2017, SH). All cross-sectional | VPD-abs | — | A hard gate (§24.3) |
| Screening | SCOFF ≥ 2: sensitivity 84 %, specificity 80 %; no study of benefits or harms of screening (USPSTF review, PMID 35289875) | VPD-abs | items © (UV) | Opt-in self-check plus referral only |
| Self-report | Athletes under-report intake by 19 % (PMID 29207495); adults 12–38 % (PMID 12835280) | VPD-abs | — | Logs are "logged", never "true intake" |

### 24.2 Healthy eating and food data

| Item | Licence | fm use |
|---|---|---|
| WHO healthy diet fact sheet (2026): ≥ 400 g fruit and vegetables, free sugars < 10 % (≤ 5 % better), saturated fat ≤ 10 %, salt < 5 g | WHO publications CC BY-NC-SA 3.0 IGO (VC) | Cite and paraphrase numbers |
| UK Eatwell Guide | OGL v3 except where stated (VC) | Use with attribution; check the image |
| US Dietary Guidelines 2025–2030 | US government work (UV); content read only second-hand; a large departure from 2020 | One national view, describe |
| Nordic Nutrition Recommendations 2023 | Open licence not confirmed (UV) | Hold |
| **USDA FoodData Central** | **CC0** (VC) | Bundle generic foods; never ship an API key |
| **UK CoFID** | OGL v3 (VC) | Use |
| CIQUAL (France) | 2020: Licence Ouverte (VC); 2025 UV | Use 2020 |
| **Open Food Facts** | Database ODbL, contents DbCL, images CC BY-SA (VC) | Barcode lookups; keep the derived database separate under ODbL, attributed |
| OpenNutriTracker, Waistline (GPL-3.0), wger (AGPL) | VC | Exchange data, take no code |
| Photo calorie estimation | 0.1–38 % error, worse for mixed meals (PMID 38060823); Nutrition5k data CC BY | Avoid as a number source |

### 24.3 The coupling: training events → nutrition proposals

Rust computes every number from the plan, body mass and logs, with its source; Myrme words it; the
person accepts or dismisses. **Watch calorie estimates are never an input** (Part 1 §2.2).

| Trigger (code) | Proposal | Source |
|---|---|---|
| Planned hours per day → load band | Daily carbohydrate as a **range** × body mass; lower on easy days | AND/DC/ACSM 2016 |
| Key session > 60 min tomorrow | 1–4 g/kg, 1–4 h before | same |
| Session 1–2.5 h | 30–60 g/h × hours = grams to carry | same; Burke 2011 |
| Session or race > 2.5 h | Up to 90 g/h glucose–fructose; 120 g/h only as an opt-in contested note | same; Jeukendrup 2014; Morton 2026 |
| Race ≥ 90 min within 3–6 weeks | Rehearse race fuelling in long sessions | Martinez 2023 |
| Race ≥ 90 min in 2 days | 10–12 g/kg for 36–48 h | AND/DC/ACSM 2016 |
| Two hard sessions < 8 h apart | 1–1.2 g/kg/h for 4 h, plus ~0.3 g/kg protein | same |
| Strength or key session done | ~0.3 g/kg protein soon after | same |
| Hot day, or high measured sweat rate | Fluids before; losses < 2 %; sodium rule; **and "don't drink so much you gain weight"** | same |
| Weight falling > 1 %/week | Rate caution; protein | Garthe 2011; Mettler 2010 |
| Weight-loss goal during a heavy block or before a race | Suggest moving the loss to the base phase; EA as a concept, no verdict | AND/DC/ACSM 2016; Areta 2021 |
| Logged warning signs (missed periods, bone stress injury, falling performance with weight loss) | Fixed referral text to a sports physician or dietitian; no diagnosis | IOC REDs 2023 |
| **At-risk flag** (eating-disorder history declared, opt-in SCOFF ≥ 2, under 18, pregnant) | **Hard switch:** no calories, deficits, weight targets or weight-trend prompts; only fuelling adequacy and referral | §24.1 |
| No training plan | WHO / Eatwell paraphrases, not targets | WHO; Eatwell |

### 24.4 What fm and Myrme must never do in nutrition

1. Invent or estimate a calorie, gram or nutrient number (LLM estimates drift up to 45 %, SH).
2. Use watch energy expenditure to set intake, deficits or energy availability.
3. Present energy availability, "REDs risk", a resting-metabolism ratio or a clinical-tool colour as
   a finding.
4. Give calorie targets, deficits or weight pacing to anyone at risk, under 18 or pregnant; answer
   requests for restriction or hiding behaviours.
5. Forecast kilograms from calories with a fixed rule.
6. Push single body-composition targets, or weight loss during competition or above 1 %/week.
7. Recommend > 90 g/h by default, ketogenic diets for performance, or train-low as a booster.
8. Recommend supplements beyond Part 2 §16.
9. Treat logged intake as true intake.
10. Copy copyrighted tables or text (AND/ACSM tables, WHO NC text, NICE); mix ODbL data into the MIT
    bundle without its terms.

## 25. What Part 3 adds for fm — directions, none decided

- **One library, many sports.** Every sport is a note with its own anchor and zones; the combined
  view is time and RPE × minutes only. The watch gives running, cycling (HR and GPS), pool and open
  water swims, and strength sets — all through the same FIT import, with fm reading what
  Gadgetbridge drops (swim lengths).
- **Per-sport honesty labels** become a pattern: wrist HR invalid in water, power absent on this
  watch, mixed-stroke distance unreliable, rep counts editable.
- **The gym is first-class**: sets from the watch, strength workouts to the watch, generic programmes
  in fm's words, prevention programmes as cards, and the injury-prevention case stated plainly.
- **Nutrition follows the calendar**: a fuelling proposal is computed from the planned session, the
  same way a training proposal is — with a hard safety gate that removes weight and calorie features
  for anyone at risk. This touches the Health group (`health-plan-2026-10-22.md`), which already
  carries a Mind section; the gate and the referral texts belong in both.
- **Open food data** can ship: USDA (CC0) and CoFID (OGL) in the bundle; Open Food Facts as a
  separate ODbL database for barcodes.

**More decision entries** (adding to §8.4 and §18): *Sports and their anchors* (per-sport zones,
the combined view's limits); *Swim import* (Length messages, quality flags, CSS); *Strength sets
from the watch* (import and the note-wins rule); *Programme names* (generic schemes only); *Fuelling
proposals* (triggers, sources, the 90 g/h default); *The nutrition safety gate* (who it covers,
what it hides, who sets it); *Food data* (which databases ship, ODbL separation).

## 26. Part 3: not done and full texts needed

**Not done.** FR165 swim-workout support, how RPE lands in FIT, and whether rustyfit emits workout
and set messages correctly — all need a real file. No open triathlon planner found. Compound vs
isolation exercises, 1RM-testing safety, grip norms and low-back programmes not sourced.

**Full texts.** Martin 1998 cycling power model; Chung's virtual-elevation note; Gillinov 2017,
Lipski 2022, Inglis 2019, Karsten 2021; Spragg 2023 and Mateo-March 2025 methods; Nagle 2011;
Mooney 2017, Olstad 2020, Wallace 2009, Dekerle 2005, Wakayoshi 1993, Haddad 2017, Jacko 2024
Table 8; Skiba's SwimScore paper; Harøy 2019 dose table, Andersson 2017 exercises, Beyer 2015 and
Kongsgaard 2009 protocols, Lauersen 2014 included trials, Patterson 2019 remainder, Llanos-Lagos
2026; the **official** AND/DC/ACSM 2016 text; Jeukendrup 2014, Podlogar 2022, Morton 2026,
Martinez 2023, Areta 2021 remainder, Mountjoy 2023 and Heikura 2024; Hew-Butler 2015 hyponatraemia
consensus; Levinson 2017; the NNR 2023 and CIQUAL 2025 licences; the US DGA 2025–2030 primary text;
SCOFF reuse rights; USPTO records for the cycling and programme marks; FIT Profile.xlsx for
`set`, `length` and workout-step targets.

## 27. Part 3: sources

**Cycling (VPD-abs unless noted).** 29801189, 30676826, 31952081, 29809063, 33551839, 35835698,
36803419, 34127613, 31269000, 31714462, 28930880, 31839854, 31689684, 30387374, 32604072,
34749416, 17473782, 17951327, 42479066, 28388783, 27834562, 34586952, 35239466, 37259247,
41295789, 40148210, 36875044, 41923151, 28095061, 28121252, 28282753, 26290797, 21936289,
40910034, 39285616, 23287833, 35361736, 19417224, 15503124, 19290675, 42560532, 23889316,
24129784, 28709155, 32404012, 35009723, 42197732, 40968160, 39928630, 22164312, 23256921,
27476628, 32147498, 33074460, 27450862, 25900805, 36629524, 17645369; Leo 2022 (PMC8783871, full
text). FR165 manual "Wireless Sensors"; TrainingPeaks power-levels page; GitHub API for the tools
in §20.3.

**Swimming and triathlon.** FR165 owner's manual (pp. 4–10) and webhelp pages; Garmin FIT cookbook
(decoding activities, encoding workouts); Gadgetbridge `FitImporter.java`,
`GarminWorkoutParser.java`, `GarminWorkoutFitEncoder.kt`, LICENSE; GoldenCheetah `SwimScore.cpp`;
full texts Jacko 2024 (PMC11281193), Cosoli 2022 (PMC9269005), Esteve-Lanao 2017 (PMC5447032);
abstracts 1555562, 1521952, 8425518, 16195984, 11842355, 26473520, 31991399, 42009301, 19002069,
33952709, 39129754, 28497283, 12439774, 8775162, 31935141, 33486408, 37515375, 27895075, 38658416,
27007113, 23377278, 26825101, 11049151, 20359948, 21560752, 31100906, 21967604, 31827355,
19847691, 36224433, 41343068, 25258562, 36898525, 35291633, 21916794, 33821772, and the Mooney,
Olstad, Delgado-Gonzalo, Seelhöfer, Kits and Beanland records.

**Gym.** 37582807, 28641044, 30580468, 28385560, 31156448 (part full text, CC BY), 25601394,
29891614, 9617396, 27313171, 23914932, 8887211, 29484512, 24100287, 34170576, 32077380, 30943568,
30547232, 29737024, 39205815, 26018970, 19793213, 39794667, 35959319, 33322981, 32774557, 36174660,
40314751, 24570599, 35309521, 28631257, 35099631, 40632222, 38251299, 43000402; FR165 manual
strength page; concept2.com VO2max calculator; US Copyright Office Circular 33; Gadgetbridge
`GarminWorkoutParser.java`, `FitActivityTrackProvider.java`, `GarminWorkoutFitEncoder.kt`; rustyfit
`set.rs`.

**Nutrition.** AND/DC/ACSM 2016 (unofficial copy, pp. 504–511, 514–515); Jeukendrup 2017
(PMC5371625); Areta 2021; abstracts 24791914, 24092765, 19927027, 28214452, 41759826, 35446596,
37061651, 38059288, 32679728, 40262739, 39164063, 37752002, 34543856, 33095376, 30772765, 24864135,
21872751, 29466592, 12835280, 29207495, 35289875, 31705473, 21660838, 30632439, 31141408, 24667305,
31353783, 21185970, 38060823, 28630601, 21558571; licence pages of Open Food Facts, USDA FoodData
Central, gov.uk CoFID and Eatwell, data.gouv.fr CIQUAL, WHO, Nutrition5k; GitHub API and F-Droid
for OpenNutriTracker, Waistline, wger.
