# Food and training

formicaria can keep a food log and a training log beside your notes: what you ate against a target,
how a week went, and the sets of a session. What you log is
kept as notes in your own notebook — plain files, like everything else.

> **Early.** New in 0.7.0, and used so far by one person on Linux and on an Android phone. On an
> iPhone, a Mac or a Windows computer it has not been tried. Following a diet or a regime (0.7.1),
> weight as a trend and routines (0.7.2), and meal times, the menu, packaged foods and the
> nutrient list (0.7.3), and the barcode look-up, the kitchen and the recipes that come with the
> app (0.7.4), and the week's timetable, the exercise cards and their films (0.7.5), and the three-tab Fitness, the pick-from-a-list add sheet and the weeks and days to
> start from (0.7.6), and labels from any country and millilitres (0.7.10), and a label read from a
> photograph, kept pictures, recipes that save as you go and forms that keep what you typed (0.8.0, 0.8.1) were released before being tried on a phone.

## Hiding it

**Food** and **Fitness** are two of the app's [groups](./views.md#the-groups) and are
shown unless you hide them:

**Settings → Food & reminders → Food and training → Show Food, Fitness and Health on this device.**

The choice is this device's own, like the theme that is on: hiding them at your desk does not
hide them on your phone. Nothing you logged is touched either way.

## The food list

To look a food up — its energy, protein, vitamins and minerals — formicaria uses a list of about
18,300 foods that **comes with the app**: there is nothing to get and no connection is needed. It
takes about 11 MB on the device.

It is made from four European food tables:

- **Ciqual 2025**, the French table, from Anses;
- **BLS 4.0**, the German table, from the Max Rubner-Institut;
- the **Swiss Food Composition Database 7.1**, from the Federal Food Safety and Veterinary
  Office, which has many Italian dishes;
- the **Tabelle di composizione degli alimenti** (2019 update) of **CREA**, Italy's research
  centre for food and nutrition: about 830 foods, the Italian cheeses, cured meats and breads by
  their own names, each with CREA's portion to pick.

**You can search in the table's own language.** Each food is named in English, and also by the
name its table gives it: French for Ciqual, German for BLS, Italian for CREA, and Italian, German and French
for the Swiss table. Type *prosciutto crudo*, *jambon sec* or *Haferflocken* and the food is found;
the name it was found by is shown under the English one. Nothing is translated by formicaria, so
a French-only name is not found by its Italian word.

A fifth, **FoodData Central** from the U.S. Department of Agriculture, is used only for what those
four do not have: foods they do not list, and household portions ("1 cup", "1 medium"), which
neither European table gives. A search shows European foods first, and each result says which
table it is from — two tables may both list a banana, and they are two measurements, not one.

A food from a European table is logged by weight. If you want to log "1 cup", pick the food from
the U.S. table, which is offered when the European ones leave room in the results.

Branded products are not in the list: a packaged food is written down from its label or looked up
by its barcode (see *A packaged food* below). Anything else can still be logged by name — see
*Quick add* below.

## Logging a food

Open **Food → Day** and press **＋**, or the **＋** beside a meal. From the bar's **＋** you say which meal it
belongs to; the app does not guess one from the time of day.

- **Recent** — what you have logged before, most often at this meal first. The **＋** on a row
  logs it again with the amount it had. Tapping the row lets you change the amount first.
- **Search** — a food from the list. Choose one, then say how much: the **−** and **＋** change the
  amount. Where the food's table gives household portions ("1 cup", "1 medium") they are offered
  as chips; the European tables give none.
- **Quick add** — any name and its energy. Protein, carbohydrate and fat are optional.

Tap a logged entry to change its amount. To take one entry out on its own, press the **✕** at the
end of its row — the rest of the meal stays. Removing one offers **Undo** at once; undo also works
from the ↶ menu, one step for each thing you did.

An empty meal offers **Same as yesterday** when yesterday had something there.

### Recipes

The fourth way to add is **Recipes**. A recipe is a name, the number of servings it makes, and
foods from the list by weight.

- **New recipe** — give it a name and its servings, then **Add an ingredient**: search for a food
  and set how many grams go in. What one serving comes to is shown as you go. **It is saved from
  the moment it has a name**, and every ingredient, amount and picture after that is saved as
  you add it: there is no Save button, and leaving half-way loses nothing. You find it under
  Recipes and carry on.
- **Log it** — tap a recipe, choose how much of a serving (in quarters), and add it to a meal.
  Every value the ingredients carry is logged, not only the ones on the day screen.
- **Change it** — tap a recipe and it is all there to change: the servings it makes, **Add an
  ingredient**, and each ingredient — tap one to change its amount or **Take out**. Each change
  can be undone. What you already logged from the recipe stays as it was.
- **Write the method, times and notes** — opens the recipe as an ordinary note. Write anything
  there: the method, how long it takes, the oven temperature, where it came from, what to do
  differently next time; add pictures; format it as you like. None of it is counted — only the
  ingredients are — and all of it is found by search.
- **Save a recipe from a link** — paste the address of a recipe page (GialloZafferano, Cucchiaio
  d'Argento, Fatto in casa da Benedetta, Misya, Sale&Pepe, Cookist and most recipe sites) and it
  becomes a recipe of your own: its ingredients as the page lists them, the method, the times, and a
  link back to the page and to its photo. On a computer or a phone — not from a device paired to a
  computer, which cannot ask the computer to read a page.
- **Dictate a recipe** — say it: its name, what goes in and how it is made, as you would tell
  someone. It is a recipe straight away, here under Recipes, with what you said in its notes; and
  Myrme is asked to sort it into ingredients and method **in that same recipe** — a proposal in
  **Collaboration** to accept or change. Needs the assistant with *Audio transcription* on
  ([Dictating](./notes.md#dictating)); if what you said cannot be written down, the recipe keeps the
  recording, and nothing is lost.
- **Choose the food for each ingredient** — a recipe whose note lists its *Ingredients* (saved from a
  link, made from one that comes with the app, or typed in yourself) shows those lines here until
  each is matched. Tap one: the food search opens on what it names, and the amount on the grams it
  gives. Pick the food, check the amount, **Add to the recipe** — and that line counts. A line you
  leave, such as salt to taste, is simply not counted.
- **Remove this recipe** takes the recipe and its ingredients away in one step, and can be undone.

**A recipe is a note, and the same one from both sides.** It is in the Library, tagged `recipe`, so
searching the Library finds it by anything you wrote in it, and you can tag it further like any
other note. Opened as a note, it has **Ingredients and servings** at the top, which takes you here,
to that recipe. Started here or from the template, it begins the same way: a *Method*, *Times* and
*Notes* for you to fill in. You can also start one from the **Recipe**
[starter template](./notes.md#templates) — it arrives empty, and you put the ingredients in from
here. The ingredients themselves are kept out of your notes, the way logged meals are.

What a recipe comes to is **the sum of its ingredients as you entered them, divided by its
servings**. Nothing is adjusted for cooking: the app does not estimate how much water a dish
loses or how much of a vitamin survives the heat. Choose the cooked form of a food where the list
has one ("Rice, cooked") if that is what you weighed. When an ingredient has no figure for
something — a food with no iron value, say — the recipe says so: *Iron from 3 of 4 ingredients*.

An ingredient has to be a food from the list.

### Recipes that come with the app

**One search finds every recipe** — yours and the 345 that come with the app — by dish, by
ingredient or by region, with or without accents (*tiramisu* finds *Tiramisù*). Each row says where
the recipe is from: **Yours**, **NHLBI** or **Wikibooks** (with its region). With nothing typed, the
list is your own recipes, and a line says how many more the search reaches.

- **NHLBI** — 54 recipes from the U.S. National Heart, Lung, and Blood Institute, in the public
  domain. They come with the Institute's figures for a serving, so **Add a serving** logs one
  straight away — or plans it, from the Menu. Their amounts are in cups and spoons, so on the Menu
  and on a shopping list they are listed as the recipe gives them.
- **Wikibooks** — 291 Italian recipes from the Italian Wikibooks cookbook (*Libro di cucina*), each
  filed there under an Italian region, shown word for word, credited to their authors and under
  their licence (Creative Commons BY-SA 4.0), with a link to their page and, where there is one, to a
  photo. Their pages give no nutrition figures, so one counts once its ingredients are matched to
  foods — the one extra step, below.

Open any of them to read it as published. **Make it mine** copies it into a recipe of your own —
the same text, credited — to change, photograph and time as you like, and to match its ingredients
to foods so it counts in your own amounts.

## The day, and what it is measured against

The large number is what is left of your energy target for the day. Going past it is shown plainly
— how far over — and not as an alarm.

Press **Set a target** (or **Change**) to say what a day should come to. A change applies from
today on; earlier days keep the targets they had.

A set of targets can have a last day (*Until*). After it, the targets you had before apply again —
so a few weeks of eating less, then back to normal, is two saves and nothing to remember.

Below the meals, *Also today* lists fibre, calcium, iron and vitamin C. When a value comes from
only some of the day's entries it says so — *from 3 of 4* — because an entry typed in by hand has
energy and nothing else, and a total that left it out without saying would look complete.

### Nothing you typed is lost

Leaving a form, changing view or closing the app does not throw away what you were doing.

- A **training session**, a **recipe**, and a **diet or regime you follow** are saved step by
  step: each set, each ingredient, each change is written as you make it.
- A form you have **not finished** — a label, a food typed in by hand, an exercise being
  planned, a routine being named, a diet or regime you are fine-tuning — is kept on the device.
  So is a **change you had not saved** to a set, a routine, a packet's label or something you
  logged.
  Open it again and it says *Carried on from where you left it*, with **Start afresh** beside it.

A kept form is on this device only: it is not in your backup and not on your other devices
until you save it.

### A packaged food

A packet of pasta is not in a table of generic foods. Three buttons at the top of the add sheet
are for it. **Scan a barcode** takes a picture of the barcode: a packet you have written down
before opens straight away, and any other starts a form with its number filled in, from which
you can look it up. **Photograph a label** has the assistant read the nutrition table off a
picture and fill the form for you to check (see below; on a computer, not yet on a phone).
Each of these opens the camera; **or choose one from the gallery**, beside it, uses a picture you
already took.
**Type in a label** opens the same form empty: copy the label as it is printed — energy is the
only line needed.

Labels differ by country, so the form asks three things instead of assuming them:

- **What the figures are for**: *per 100 g*, *per 100 ml* (drinks), or *per serving* — then say
  how big a serving is, in g or ml. A European packet is per 100; an American one is per serving.
- **Energy in kcal or kJ.** If the label prints both, type the kcal.
- **Salt in grams, or sodium in milligrams**, whichever the label prints.

Whatever you choose, the food is kept per 100 g or per 100 ml, worked out from what you typed.
A food typed per 100 ml is **measured in millilitres** from then on: you log 250 ml of it, and a
shopping list adds its millilitres up apart from grams. formicaria never turns a millilitre into
a gram, because that depends on the food. Salt is typed as the label states it. You can add the number under the barcode and the
ingredient list as well.

It is saved as a note of yours and found from then on by its name, or by its barcode number, in
the same search. Logging some of it works like any other food.

**Photograph the barcode** takes or chooses a picture and reads the number off it, so you need
not type it. The picture is read on your device and is not kept. If nothing can be read, try
again closer and in good light.

**Photograph a label** (also **Photograph the label** in the form) takes or chooses a picture
of the packet's nutrition table. The assistant on your device reads the table, and the form is
filled with what it found: what the figures are for, the serving if one is printed, energy, fat,
saturates, carbohydrate, sugars, fibre, protein, and salt or sodium. Nothing is saved until you
press **Save it**.

- **Check every figure against the packet.** The assistant can misread a digit or mix two lines
  up, and the form cannot always tell. When the figures do not add up (sugars above
  carbohydrate, energy that does not match the rest) it says so in red, but a wrong figure can
  also look perfectly reasonable.
- **Take the picture upright, close, and in good light**, with the table filling it. A table
  photographed sideways is not read; **Turn the picture and read it again** turns it a quarter
  turn each press. **What was read** shows the assistant's own words.
- **It needs the assistant, with a model that reads labels well.** On a computer that is the
  model the assistant installs by default; a reading takes from a quarter of a minute to a
  minute. **On a phone it is not read yet**: the phone's small model got two packets right
  out of nine when it was measured. The picture is taken and kept with the food all the same,
  so you can type the label in now, or save it and have it read on your computer later (next
  section). The barcode still works there.
- The picture goes to the assistant on your own device and nowhere else.
- Labels are read in English, Italian, French, German, Spanish and Portuguese. A table printed
  sideways on (nutrients across the top) or two tables side by side may be read wrongly or not
  at all.

**Pictures, and settling a label later.** A packet keeps its pictures. The one you take of
the label is kept, and **Add a picture of the pack** adds more: the ingredient list, the
front. They are stored at full size in the food's own note and go into your backup with it. A
recipe of your own keeps pictures the same way (**Add a picture of the dish**).

With a name and a picture you can **save a packet before its figures are known**. Search then
shows it as *label not read yet*. You can log some of it straight away: the entry keeps its
amount and is not counted in the day's totals, which say so, until the label is filled in.

To settle it, find the packet, and press **Fill in its label**. The form opens with its
pictures; where the assistant can read labels (your computer, once a backup has brought the
packet there), **Read it** under a picture reads it into the form. Check it against the
picture, and save. What you logged before is then filled in with its share, and the screen
says how many entries. An entry logged in grams of something that turns out to be measured in
millilitres is not filled in, and you are told: open it and give its amount again.

**Change its label** does the same for a packet that already has figures, to put one right.

**Looking it up.** Once the form has a barcode number, **Look it up at Open Food Facts** asks
that public database for the product and fills the form with what it has. Check it against the
pack before you save: volunteers typed it in, it can be wrong, and for many products — more so
outside Europe and North America — there is nothing. This is the one thing in Food that reaches
the internet. It happens only when you press the button, and it sends the barcode number and
nothing else. The saved note says where its figures came from.

### When you ate

A food you log today is marked with the time you logged it; you are never asked. Open the entry to
change the time or clear it. Under the day's numbers the screen says how many times you ate and
when the first and last were — entries less than 15 minutes apart count as one. It is a
description of your day, not a score: no health body recommends a number of meals.

### Not sure how much?

Under the amount, **Not sure of the amount?** lets you give a least and a most. The entry keeps
your best guess, and the day then also says the least and the most it could have come to.

### Are you following your diet?

If you follow a diet (see below), the Day screen names it and says how many of the things it asks
of a day are met. Only the ones that are off are listed, each with how far: *13 g to go*, *4%
over*. Press one to see which of today's foods put most into it. When you are short of
something, it also lists up to three dishes — from your recipes and the ones that come with the
app — a serving of which has most of it, and what your day would then come to; press a dish to
open it. If a serving would take something else past its limit, it says so. That is addition,
not advice.

### All nutrients

**All nutrients**, under *Also today*, lists every vitamin and mineral of the day beside the
European reference value for an adult of your sex. Press one to see which foods it came from.
Where the reference differs for men and women and the app does not know which you are, it is
left out until you say so in *About you*. The references describe what covers a population, not
what you in particular need.

## Planning a meal ahead

Step to a day after today — the day screen reaches a week ahead — and add as usual. What you add
there is a **plan**: it is shown in a lighter hand, and is in no total until you press the tick
beside it. *Same as yesterday* on a day ahead plans it too.

## Menu

**Food → Menu** is the days ahead, to fill in: today and the days after it, each with its meals.
Set how many days with the minus and plus at the top.

- The **+** beside a meal plans something for it, with the same sheet you log with: a food, or a
  serving of one of your recipes. It is a plan until the day comes; on that day's screen you
  confirm it when you eat it.
- **next day** beside a planned meal plans the same again for the day after — for a dish you
  cook once and eat twice.
- **Dishes on this menu** says, for each of your recipes on it, how many of the servings it makes
  you have planned.
- **What it needs** adds up what the planned meals take: a recipe's ingredients for the servings
  planned, and plain foods by their amount. Mark what you already have with **need it / have
  it**. **Make a shopping list** saves the rest as a note with a box to tick on each line.
- **In your kitchen** is the list of what you said you have — names only, no amounts to keep up.
  Add to it by hand, or take something off when you run out.

Each day shows what it would come to against your energy target and, if you follow a diet, how
many of the things it asks of a day the plan would meet and how far off the others are. The app does not say how long a
cooked dish keeps.

## Trends

**Food → Trends** shows a week, four weeks or twelve. A menu at the top picks **what to show** —
energy, protein, carbohydrate, fat, or any other value the day screen has — and that one is drawn
day by day against its target; the averages of the others are listed below it. It says how many of
those days have anything logged, because an average over three logged days is not a week's
average.

**Weight** is the last thing on that menu. With it showing, the **+** beside the menu writes down
today's weight, and the screen draws it as a line: where it is now, and how far it moved over the
days shown. Two weighings in a day count as their average. The weight you give in *About you*
(under **Diet**) is written down the same way, and the numbers worked out for you always use
your latest weighing.

**Steps** are written down the same way: pick **Steps** on the menu and press **+** to note the
day's count from any phone or step counter. formicaria counts no steps itself. The *Steps a day*
card reads what you write here.

## Fitness

Fitness has three tabs: **Week**, **Exercises** and **Regime**.

### Week

Your timetable, Monday to Sunday. Each day shows the routines you meant for it and what you did.
At the top is the regime you follow and how the week stands against it, and what you lifted this
week beside the week before.

- **Open** on a day shows that day: its exercises and activities, to add to, change and tick.
  **Back to the week** returns.
- A day still to come with a routine and nothing on it has **Plan …** to put the routine there.
- **Plan the days left as last week went** copies what you actually did last week onto the same
  weekdays of this one.
- If you follow a regime, its **week to start from** is shown here: an example, as a health body
  published it where one did, and otherwise arranged by the app from that regime's own numbers —
  the screen says which. It is only an indication. **Use it as my week** turns it into routines
  of your own, which you then change freely.
- With nothing followed and no routines yet, three example weeks from the U.S. CDC are shown.

### Adding to a day

Press **+** (or *Add an exercise or an activity* on a day). You pick from a list — what you did
before, activities such as walking, and the exercise cards (all, no equipment, or gym) — and it
arrives with numbers to start from:

- an **exercise** with sets and repetitions: what you did last time, or the figure on its card,
  or a general starting point from the NHS (2 sets of 8);
- an **activity** with minutes and effort, where a source gives them (a 30-minute brisk walk).

The sheet says where the numbers come from. Change them to what you do, then add. Something not
in the list can still be typed. *It trains* says which part of the body an exercise works, which
a regime that counts sets per part needs. Effort runs from 0 to 10: 5 or 6 is moderate (you can
talk, not sing), 7 and above is vigorous; lighter than that is recorded and does not count
towards a weekly total.

Tick a set, or a planned activity, when it is done.

### Routines

A routine is a day's exercises kept under a name, meant for some weekdays; it comes back every
week. **Keep this session as a routine** makes one from a day; **Routines**, at the bottom of a
day, lists them to rename, change their days or remove. To change a routine's exercises, plan
it, change the day, and keep it again under the same name.

### Exercises

How each exercise is done, in the words of the body that published it. Choose what you have —
**No equipment**, **A bar**, **Free weights**, **Machines** — and the part of the body, then open
a card for its steps.

- The no-equipment cards are mostly the NHS website's; the others are the U.S. Army's exercise
  manual, which is public. The Army's are written for soldiers and shown exactly as written.
- **To watch** links to films on YouTube, often several: a health body's own channel first; then
  a trainer whose science or clinical qualification could be read on their own site, shown with
  it; the Army's own clips last. On the Army's cards the films come above the text.
- **Safety tips for lifting**, on every card that needs a bar, free weights or a machine (and at
  the top of those lists), opens the US National Institute on Aging's safety tips, whole and as
  written.
- **How it is done, on other sites** links to pages elsewhere — mostly the American Council on
  Exercise's library — that explain the exercise and what to avoid. Only their titles are shown
  here; open them to read.
- **Add to today's session** starts the add sheet with that exercise. On a day, **How** beside an
  exercise opens its card.

## Following a diet or a regime

**Food → Diet** and **Fitness → Regime** let you choose what to follow and then show how your weeks
measure up to it. Everything they hold you to is a number a named health body or study published;
nothing is made up, and each number says whose it is.

**Worth watching.** At the top of both tabs, whatever you follow or if you follow nothing, are the
same few numbers, read over the week, with the last four weeks beside them, oldest first:

| In Food | In Fitness |
|---|---|
| Saturated fat under 10% of energy (WHO, Nordic 2023; EFSA: as low as possible) | Activity, 150–300 minutes a week (WHO) |
| Sodium under 2 g (WHO, EFSA; Nordic 2023: 2.3 g) | Strengthening on 2 or more days (WHO) |
| Fibre at least 25 g (WHO, EFSA; Nordic 2023: 35 g for men) | |
| Protein for your weight: 0.83 g per kg (EFSA); 1.0–1.2 g from 65 (PROT-AGE) | |

They were chosen because they are both the best-supported and measurable from what you log.
Potassium is not among them on purpose: pushing it is not safe with kidney disease.

**Choosing.** The cards are shelved by **what kind of thing they are**, which says how they are
measured:

- **Targets** — numbers for nutrients, measured from what you log.
- **A way of eating** — amounts of food groups. What the food list can measure is measured; for the
  servings, you say each day whether you kept to them (**I kept to it today**).
- **When to eat** — measured from the times of what you log.
- **Weekly amounts** and **Programmes** — measured from the sessions you log.
- **Yours** — every number set by you.

Each card shows the **question** it answers (*Am I eating healthily?*, *Getting stronger*, …), what
it **rests on** (a guideline, an expert consensus, a review of trials, a definition), and **how it
relates to the others** — *"Instead of WHO, for adults in the Nordic and Baltic countries"*, *"The
middle of three: pick one"*. **Start here** marks WHO's two cards, the only ones written for all
adults.

**One card for each question.** You can follow WHO's weekly activity *and* a strength card *and*
stretching, because they answer different questions; one session you log counts for every card it
fits. Following a second card for the **same** question replaces the first — WHO 65 and over
instead of WHO's weekly activity, say.

| In Food | In Fitness |
|---|---|
| **Eating healthily:** WHO healthy diet · DASH eating plan · Nordic recommendations 2023 · Planetary health diet | **Moving enough:** WHO weekly activity · WHO, 65 and over · Activity for losing weight · Activity in pregnancy and after |
| **Enough of each nutrient:** European reference values · Vegan or vegetarian: what to watch | **Walking more:** Steps a day |
| **How much to eat:** Energy and protein for me | **Getting stronger:** Strength, twice a week · Muscle growth · Minimal strength |
| **When to eat:** Two fasting days · Alternate-day modified fasting · Time-restricted eating | **Fitter heart and lungs:** Norwegian 4×4 · Interval walking |
| **My own** | **Steadier on my feet:** Falls prevention · **More flexible:** Stretching · **My own** |

Press a card to see, before anything is saved, **what you will be held to** (every number
changeable — that is how you fit a card to yourself), **about you** where it needs it, **what
formicaria cannot measure**, its **sources** as links, and a day or a week to start from. **Follow
this** starts it today.

**Anything we should know?** Optional, never required: pregnant, breastfeeding, after the
menopause, kidney disease, taking insulin or a sulfonylurea, an eating disorder now or in the past.
A box you tick does only two things — shows the card's note for it, and uses a number a health body
publishes for you, or leaves a number out:

- Pregnant or breastfeeding: EFSA's folate (600 / 500 µg) and iodine; the energy figure gains
  EFSA's 500 kcal while breastfeeding, and is left out in pregnancy; the fasting cards are left out.
- After the menopause: EFSA's 11 mg of iron instead of 16.
- Kidney disease: potassium and protein are left out — with kidney disease your kidney team sets them.
- From 65 (from your year of birth): protein 1.0 g per kg (PROT-AGE) instead of 0.83.
- An eating disorder: the fasting and weight-loss cards and the energy ceiling are left out.
- Insulin or a sulfonylurea: the fasting cards say to talk to your diabetes team first.

The library says how many cards it left out. A log can show intake below a reference; only a blood
test shows a deficiency, and formicaria never suggests a dose of anything.

**Being measured.** Each card you follow shows one row for each thing it asks: what was asked,
today or the week so far, the seven days from Monday, and **the last four weeks** — a day's number
as the week's average, a weekly one as the week's total. WHO's numbers are averages for whole
populations, so the trend over weeks is the reading closest to what they mean. A day you logged
nothing on reads *nothing logged*, not zero; a weekly amount is only called short once the week is
over.

The fasting cards: in trials, about as effective as eating a little less every day (Cochrane 2026;
BMJ 2025). The interval cards are followed by logging the session under its name. A diet's energy
and protein numbers also become the targets on **Food → Day**.

**Change its numbers** starts a new set from today and leaves earlier weeks measured against what
they had. **Add a card, or change one** opens the library. **Stop following** ends a card; undo
brings it back.

Not offered: a Mediterranean card (its 14-question check is a research instrument whose reuse terms
are not settled), low-carbohydrate and keto (no agreed definition), and "zone 2" (none either).

## What is kept, and where

Each logged food or set is a note with a `log` property (`food`, `workout`), the day it belongs
to, and whole numbers whose unit is in the name — `kcal`, `protein_mg`, `iron_ug`:

```yaml
title: Banana raw
start: 2026-10-11
amount_g: 118
kcal: 112
log: food
protein_mg: 1357
slot: breakfast
source: foods@2026.10.1/bls:F503100
```

These notes are kept out of the board, the agenda, the timeline and search, so a year of meals
does not bury your notes. They are backed up and shared with a notebook like any other note, and a
removed one is in *Recently deleted*.

Targets are an ordinary note carrying `program: food` — it is on your agenda from the day it
starts.
