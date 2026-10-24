# Health

formicaria can keep a health diary beside your notes: how a headache was, a blocked nose, how you
slept, the painkillers you took, whether you washed the bedding, the days of your period. Each
day is one note in your own notebook, a plain file like everything else. You choose what to track.

**It records and counts. It never diagnoses, flags or advises.** Where it sets your numbers beside a
body's (the NHS, NICE, the International Headache Society, FIGO), it names the body and quotes it,
and it never puts a word to you. It is a record to bring to a doctor, not a doctor.

NICE is given **in our own words**, with the recommendation's number and a link: NICE allows its text
to be reused in the UK only, and formicaria is used everywhere. Such lines say *In our words*.

**Everything stays on your device.** The diary, the record and Mind are notes in your notebook;
nothing in Health sends anything anywhere. Your notes go only where your own backup sends them. The
assistant's model runs on your device too. If you turn its web research on, only the search questions
it writes go out (to Wikipedia, arXiv and GitHub), never your notes.

> **New, and not yet tried on a phone.** Built in October 2026 and tried on a computer only. The
> screens are laid out for a narrow window, but nobody has used them on a phone yet.

## Turning it on and off

**Health** is one of the app's [groups](./views.md#the-groups), beside Food and Fitness. It has three tabs: **Diary** to
write in, **Over time** to look back, and **Guidance** for the cards. It is shown unless you hide it, with the same switch:

**Settings → Food & reminders → Food and training → Show Food, Fitness and Health on this device.**

## What you track

Press **＋** in Health and choose **Track something**. Start from one of these, or name your own:

- **Headache**: how bad (none, mild, moderate, severe), whether it lasted 4 hours or more, and
  nausea, light that hurts, noise that hurts. These are the fields the International Headache
  Society's diaries keep and NICE asks for.
- **Nose and eyes**: a runny, blocked or itchy nose, sneezing, itchy and watery eyes, each from
  none to severe, as the US FDA's nasal symptom score counts them.
- **Cycle**: see [The cycle](#the-cycle).
- **Blood pressure**: two readings in the morning and two in the evening, systolic and diastolic, in
  mmHg, as NICE asks for a home reading: for at least 4 days and ideally 7.
- **Bladder**: times passing urine (and at night), urine measured, leaks, pads and drinks, for at
  least 3 days, as NICE asks of a bladder diary.

**Your own**: give it a name ("Bedding washed", "Ibuprofen", "Sleep") and say how it is recorded:

| Recorded as | Kept as | On the Diary |
|---|---|---|
| None to severe | `_0to3` | four words: none, mild, moderate, severe |
| A number from 0 to 10 | `_0to10` | 0 to 10 |
| How much it bothered you | `_0to100` | a slider, from *not at all bothersome* to *extremely bothersome* |
| Yes or no | `_yes` | Yes, No |
| How many times | `_count` | − and ＋ |
| Doses of a medicine | `_doses` | − and ＋, under *Medication* |
| An amount | the unit: `_min`, `_ml`, `_mg`, `_g`, `_mmhg`, `_pct`, `_c` | a number with its unit |
| An amount to a tenth | `_c10` (°C), `_mmoll10` (mmol/L) | typed as 38.1, kept as 381 |
| A questionnaire score | `_score` | a number, and a link to the questionnaire's owner |

What you track is kept on one note, **Health diary**, tagged `template`. Each thing you track is a
property on it, and the end of the property's name says how it is recorded. You can open that note
and change it like any other.

**Questionnaires** (SNOT-22, HIT-6, MIDAS, the Epworth Sleepiness Scale) are not carried in the app:
their owners do not allow it. Fill one in where its owner publishes it and type your total here.

## The diary

**Diary** shows one day, with the same rows every day whether or not anything happened. Step back a
day with **‹**. The rows are in sections: *Headache*, *Nose and eyes* (the bedding is here too),
*Cycle*, *Medication* (anything recorded as doses), *Every day* (sleep, caffeine) and *Yours*
(anything else you named). The section comes from what a thing is, so there is nothing to set.

- **A row you leave alone is "not recorded".** "None" or "No" is recorded. The month counts the two
  apart, and a doctor reading it can tell them apart too.
- **Pressing the chosen word again** takes the value away, back to not recorded.
- **The first row you touch makes the day's note.** There is nothing to save, and ↶ undoes it.
- **A day written in more than a day later is marked.** People remember their own month less well
  than they think, so the mark helps whoever reads the diary.

## Over time

**Over time** shows one thing you track at a time, chosen from **Showing**, in one of two ways.

**Month** is a grid of the month (press a day to open it in Diary; Back returns to the month),
shaded by the value, dashed where nothing was recorded, with a dot where a day was written in
later. Under it are the counts: days recorded, days at 1 or more and, for none to severe, the days
that were moderate or severe.

**Weeks** shows a bar a week, from Monday, over 4 weeks, 12 weeks, 6 months or a
year. For none to severe or yes or no, a bar is the days in the week; for how many times, the times
in the week; for anything else, the average of the days recorded. Every week says how many of its
days were recorded, and a week with none says *nothing recorded* rather than showing a zero.
**Mark** puts the days of one yes/no tracker under the bars, for example *Bedding washed* under
*Headache*.

## From Food and your body log

What you already log elsewhere is shown in the Diary, under *From Food and your body log*, and not
asked for again: the **water** in what you ate and drank, **caffeine** and **alcohol** from Food,
and your **weight** and **steps**. They are changed where they were logged. Caffeine is only as
complete as the food list: some of its tables do not carry it.

## The record

Some things change rarely and are asked about at every visit. In **Health → ＋ → Start a health
record**, formicaria writes templates for them into your notebook, once: **Medicine**, **Allergy or
intolerance**, **Condition**, **Result**, **Visit**, **Immunization**, **Procedure**, **Device**,
**Contact** and **No known allergies**. Make one from **＋** like any note. Each is kept on the
Library's **health** shelf (the tag `shelf:health`), and is an ordinary note you can link to and from.

- **A medicine** has a status, current, stopped or on hold, and its first and last day. Keep a
  stopped one: it is history worth having.
- **A result** keeps the value, and the range and flag **as the lab printed them**, with the report
  attached. Ranges belong to the lab that printed them. The app never looks one up, colours a result
  or explains it.
- **No known allergies** says so, so it is not mistaken for allergies not written down.

The sections are the International Patient Summary's, the standard for a record a person carries;
a medicine's lines follow FDA's *My Medicine Record*.

## For the doctor

In **Over time**, **For the doctor** makes a page of the diary. It opens with your record, **as of
today**: current medicines, allergies, conditions, past operations and devices, which is what NHS,
FDA and AHRQ guidance tells people to bring. Then:

- for **this month**, or for **the last 8 weeks**, which is the least NICE asks a headache diary to
  cover;
- the counts first, then a row a day;
- with the things you tick, and nothing else;
- ending with what its scales are and where they come from, and *"A record, not a diagnosis"*.

**Print** prints the page alone. **Save as CSV** saves the same numbers as a spreadsheet file, one
row a day. **Keep as a note** keeps the page as a note, tagged `health`, with its tables written out:
it reaches your other devices with your notes, and a computer can print it.

**On a phone** only **Keep as a note** is offered: Android's web view, which the app runs in, does
not print or save files by itself.

## Guidance

**Guidance** holds cards, one per question, as in Food and Fitness. Each is in its sources' own
words, with whose they are, and a badge for what it rests on: **Guideline**, **Expert consensus**,
**Review**, or **Definition, not advice**. Each card also says **what it does not say**, so it is
not read as more than it is. Where reputable sources disagree, the card says so and names both.

- **Keeping a diary**: *Keep a headache diary*; *Painkiller days*.
- **Nose and allergy**: *Fewer dust mites in the bedroom*; *Rinsing the nose with salt water*;
  *Hay fever self-care*; *When to see someone*.
- **Every day**: *Sleep*; *Caffeine*; *Water* (counted from Food, beside EFSA's 2.0 L for women or
  2.5 L for men, by the sex in your profile); *Blood pressure at home* (your 7-day average, the
  first day left out as NICE asks, beside the NHS's *"135/85 or higher when checked at home"*).
- **Cycle**: *Your cycle*; *Heavy periods*; *Periods and migraine*; *Premenstrual symptoms* (the
  cycles your diary covers, beside the NHS's *"at least 2 menstrual cycles"*).
- **Mind**: *Mood (PHQ-9)*: the questionnaire itself, which its makers allow anyone to carry,
  answered on the card. It is counted when every question is answered, as its instructions score
  it, and never named as a diagnosis.
- **Yours**: the cards you write.

**Follow** a card, and what it counts appears at the top of Guidance. It also adds the things the
card counts to what you track. The count is shown beside the body's number, with the body's words,
for example *"Days this week with any painkiller: 2 · at most 2 · NHS: 'more than 2 days a week'"*.
A ✓ means your count is within the number. Nothing else is said about it.

*Painkiller days* asks which of your medicines are paracetamol, aspirin or an NSAID, and which are a
triptan, an opioid, an ergot or a combination. NICE gives a different number for each, and the app
does not guess a medicine's class from its name.

**Stop following** ends it from yesterday. The note stays, and so do the days it counted.

**Yours**: **Write your own** makes a card of your own. Give it a name, your own words if you like
(why, and who said so), one thing you track, how it is counted (days this week, days this month,
days above or at a number, most days in a row), and your own aim if you have one: at least or at
most. Its number is yours, so it needs no source, and it is badged **Yours** and says so.

Following a card is a note too, with `program: health`, so it is on the agenda from the day you
started it, like a diet or a regime.

## The cycle

The **Cycle** starter tracks the first day of each period, the flow (none, light, medium, heavy:
the words Apple Health and Android's Health Connect use), spotting, and period pain. It is added
only when you add it.

- **A cycle runs from one first day to the next**, as FIGO, the international federation of
  gynaecologists, defines it. Mark **Period start** on the first day of each period.
- **Nothing is predicted**: not the next period, not fertile days, not ovulation. A late period is
  not read as anything.
- **Cycle context**: on the day something changes your cycles (you started or stopped the pill, are
  pregnant, gave birth, are in perimenopause), write it in your own words; write *none* on the day
  it ends. Cycles while a context is in force are left out of the counts, as FIGO's are, and they say
  so.
- **Missed logging**: if a long cycle was really a gap in your diary, press **Missed logging** beside
  it in *Your cycle*. It is left out of the counts, and says so. **Count it again** undoes that.
- *Your cycle* shows your cycle lengths beside FIGO's 24 to 38 days **and**, separately, the NHS's
  21 to 35. It shows your days of bleeding beside FIGO's 8 and the NHS's 7. FIGO also gives how much
  cycles may vary by age, and that is compared only if your year of birth is in your profile (the
  one Food uses).

## Keeping cycle notes to yourself

Everything you track goes wherever your notebook goes. For a cycle diary that matters more than for
a food log, so be clear about where that is:

- **A backup is a copy.** If the vault has a git remote, every day you write is pushed to it. A
  restic snapshot holds a copy too. See [Backup & versioning](./backup.md).
- **Deleting a note does not remove it from history.** The vault's history, and every copy already
  pushed or snapshotted, still has it. Taking it out of history means rewriting that history with
  git tools outside the app, and then dealing with every other copy (the remote, other devices,
  snapshots). That is hard, so it is easier not to send the notes anywhere in the first place.
- **The simplest way to keep them to yourself is a vault of their own**, with no remote, shared
  with nobody. Create one in **Settings → Vaults & backup** and keep your Health diary there.
- formicaria has no account, no analytics and nothing that sends your notes anywhere but where
  you back them up. That is your choice to make, and this is what it means.

## What doctors use these numbers for

These are quoted so you can compare your own counts. The app never applies them to you, and only a
clinician can.

- **Headache days**: *"Number of days during an observed period of time (commonly one month)
  affected by headache for any part or the whole of the day."* (ICHD-3, definition of terms)
- **Chronic migraine**: headache *"on ≥15 days/month for >3 months"*, with migraine features *"on
  ≥8 days/month"*. (ICHD-3, 1.3)
- **Painkillers**, in our words: clinicians should think of medication overuse headache when a
  headache began or got worse while someone took triptans, opioids, ergots or combination
  painkillers on 10 or more days a month, or paracetamol, aspirin or an NSAID on 15 or more days a
  month, for 3 months or longer (NICE CG150, 1.2.7). The thresholds *"are based on expert opinion
  rather than on formal evidence"*. (ICHD-3, 8.2)
- **A long-lasting blocked or runny nose**: two or more symptoms, one a blockage or a discharge,
  *"for ≥12 weeks"*. (EPOS 2020, 1.2.2.6)
- **How long to keep a diary**: *"for at least one month"* (ICHD-3); at least 8 weeks (NICE
  CG150, 1.1.4); for a link with periods, at least 2 menstrual cycles (NICE CG150, 1.2.6).

## Mind

A private notebook, not a test: nothing you write in it is scored, judged or read by the app.

- **Mood and activity** (a starter): your mood from 0 to 10, the things you planned and did, and
  how much pleasure and achievement they brought. Planning activities and doing them (behavioural
  activation) is the self-help with the strongest research behind it: across 105 trials it helped
  with low mood, also when people did it on their own (Cuijpers 2026; the COBRA trial, 2016).
- **Sleep** (a starter): how long you slept, how long it took, times awake, how well.
- **Mind notes**: **＋ → Start Mind notes** writes templates for a **thought record**, **worry
  time** and a piece of **writing**, kept on the Library's **mind** shelf (`shelf:mind`). They are our own wording:
  the well-known worksheets are not free to carry.
- **Writing it down**: people often feel worse straight after writing about something hard; any good
  tends to come later, and with a few days between sessions (Frattaroli 2006; Guo 2023).
- **Mood (PHQ-9)** and **Anxiety (GAD-7)**: the questionnaires themselves, which their makers let
  anyone carry, answered on their cards and never named as a diagnosis.

If keeping any of this makes you feel worse, stop or take a break: in one trial, people who rated
their mood every day did no better. When low mood lasts, see someone: help from a person does
better than help on your own once it is more than mild (Karyotaki 2021). The contacts below are on
every Mind screen and at the top of every Mind note.

## When to get help now

A diary is for the weeks ahead, not for an emergency. For a sudden, very bad headache, weakness or
numbness, trouble speaking or seeing, confusion, a seizure, or a high temperature with a stiff neck,
call your local emergency number now. *When to see someone* in Guidance quotes the NHS's full lists.
999 is the UK's number; where you live, use your own.

In Singapore: **995** for an emergency ambulance, and **National mindline 1771**, a 24-hour helpline
(WhatsApp +65 6669 1771). *Mood (PHQ-9)* and *Premenstrual symptoms* show them on the card, and at
the top of what is followed whenever PHQ-9's last question is answered other than *Not at all*.

## Where the words come from

Every quote names its source on its card. The research behind them, with what was read and what was
left out, is in the project's notes (`docs/context/health-plan-2026-10-22.md`). Text from the NHS
website is used under the Open Government Licence v3.0, as at 22 October 2026.
