# Food and training

formicaria can keep a food log and a training log beside your notes: what you ate against a target,
how a week went, and the sets of a session. What you log is
kept as notes in your own notebook — plain files, like everything else.

> **Early.** New in 0.7.0, and used so far by one person on Linux and on an Android phone. On an
> iPhone, a Mac or a Windows computer it has not been tried. Following a diet or a regime (0.7.1),
> weight as a trend and routines (0.7.2), and meal times, the menu, packaged foods and the
> nutrient list (0.7.3) were released before being tried on a phone.

## Hiding it

**Food** and **Fitness** are two of the app's [four groups](./views.md#the-four-groups) and are
shown unless you hide them:

**Settings → Notes & backup → Food and training → Show Food and Fitness on this device.**

The choice is this device's own, like the theme that is on: hiding them at your desk does not
hide them on your phone. Nothing you logged is touched either way.

## The food list

To look a food up — its energy, protein, vitamins and minerals — formicaria uses a list of about
16,000 foods that **comes with the app**: there is nothing to get and no connection is needed. It
takes about 9 MB on the device.

It is made from two European food tables:

- **Ciqual 2025**, the French table, from Anses;
- **BLS 4.0**, the German table, from the Max Rubner-Institut.

A third, **FoodData Central** from the U.S. Department of Agriculture, is used only for what those
two do not have: foods they do not list, and household portions ("1 cup", "1 medium"), which
neither European table gives. A search shows European foods first, and each result says which
table it is from — two tables may both list a banana, and they are two measurements, not one.

A food from a European table is logged by weight. If you want to log "1 cup", pick the food from
the U.S. table, which is offered when the European ones leave room in the results.

There are no branded products and no barcode reading. Anything not in the list can still be
logged by name — see *Quick add* below.

## Logging a food

Open **Food → Day** and press **＋**, or the **＋** beside a meal. From the bar's **＋** you say which meal it
belongs to; the app does not guess one from the time of day.

- **Recent** — what you have logged before, most often at this meal first. The **＋** on a row
  logs it again with the amount it had. Tapping the row lets you change the amount first.
- **Search** — a food from the list. Choose one, then say how much: the **−** and **＋** change the
  amount. Where the food's table gives household portions ("1 cup", "1 medium") they are offered
  as chips; the European tables give none.
- **Quick add** — any name and its energy. Protein, carbohydrate and fat are optional.

Tap a logged entry to change its amount or remove it. Removing one offers **Undo** at once; undo
also works from the ↶ menu, one step for each thing you did.

An empty meal offers **Same as yesterday** when yesterday had something there.

### Recipes

The fourth way to add is **Recipes**. A recipe is a name, the number of servings it makes, and
foods from the list by weight.

- **New recipe** — give it a name and its servings, then **Add an ingredient**: search for a food
  and set how many grams go in. What one serving comes to is shown as you go.
- **Log it** — tap a recipe, choose how much of a serving (in quarters), and add it to a meal.
  Every value the ingredients carry is logged, not only the ones on the day screen.
- **Change it** — tap a recipe and it is all there to change: the servings it makes, **Add an
  ingredient**, and each ingredient — tap one to change its amount or **Take out**. Each change
  can be undone. What you already logged from the recipe stays as it was.
- **Remove this recipe** takes the recipe and its ingredients away in one step, and can be undone.

**A recipe is a note.** It is in the Library, tagged `recipe`, and you can open it, write the
method in it, and tag it further like any other note. You can also start one from the **Recipe**
[starter template](./notes.md#templates) — it arrives empty, and you put the ingredients in from
here. The ingredients themselves are kept out of your notes, the way logged meals are.

What a recipe comes to is **the sum of its ingredients as you entered them, divided by its
servings**. Nothing is adjusted for cooking: the app does not estimate how much water a dish
loses or how much of a vitamin survives the heat. Choose the cooked form of a food where the list
has one ("Rice, cooked") if that is what you weighed. When an ingredient has no figure for
something — a food with no iron value, say — the recipe says so: *Iron from 3 of 4 ingredients*.

An ingredient has to be a food from the list.

### Recipes that come with the app

Under your own recipes, **From the NHLBI** lists 54 recipes — search them by dish or by
ingredient — from the U.S. National Heart, Lung,
and Blood Institute, which publishes them in the public domain. Open one to read its ingredients
directions, tips and notes exactly as published, with a link to its own page. **Add a
serving** logs it — or plans it, from the Menu — with the Institute's own figures for a serving.

Their amounts are in cups and spoons, so the app does not add their ingredients up: on the Menu
and on a shopping list they are listed as the recipe gives them.

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

### A packaged food

A packet of pasta is not in a table of generic foods. Under **Search**, press **A packaged food?
Write it down from its label** and copy the label's lines for 100 g — energy is the only one
needed. Salt is typed as the label states it. You can add the number under the barcode and the
ingredient list as well.

It is saved as a note of yours and found from then on by its name, or by its barcode number, in
the same search. Logging some of it works like any other food.

**Photograph the barcode** takes or chooses a picture and reads the number off it, so you need
not type it. The picture is read on your device and is not kept. If nothing can be read, try
again closer and in good light.

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

## Fitness

**Fitness → Session** is today's session: each exercise, each set, and what you did last time
beside it.

- **Add an exercise or an activity** opens one sheet with two sides.
  - *Exercise, in sets* — its name, how many sets, the reps and the load. Exercises you have done
    before are offered, with the numbers they had. Nothing is filled in for you otherwise: the
    app does not suggest sets, reps or loads. *It trains* says which part of the body the
    exercise works (upper or lower body, pushing or pulling); a regime that counts sets per part
    needs it, and nothing else does.
  - *Activity, in minutes* — a walk, a swim, a yoga class. Say what it was, what it counts as
    (aerobic, strengthening, balance or stretching), for how many minutes, and how hard it felt
    from 0 to 10. The sheet tells you what your number means: 5 or 6 is moderate (you can talk,
    not sing), 7 and above is vigorous. Lighter than that is recorded and does not count towards
    a weekly total, which is how the World Health Organization counts it.
- **Same as last time** — when nothing is planned for today, this plans the sets of your last
  session again, ready to change.
- Tap a set's numbers to change them or remove the set.
- **Tick a set** when it is finished; a clock starts counting the time since. It does not tell
  you how long to rest.

**Fitness → Volume** is the trend view over what you lifted. *Volume load* is repetitions ×
load, added up over the sets you ticked — the usual meaning of the term in the training
literature. It is a rough measure of work done, not a score.

### Routines

A routine is a session kept under a name.

- **Keep this session as a routine** — under today's sets. Give it a name and, if you like, the
  days of the week it is meant for.
- On a day with nothing planned, the routines meant for that day are offered first — **Plan
  Legs** — and the others after them. Planning one puts its sets on today, ready to change and
  tick.
- **Routines**, at the bottom, lists them with their days. Press one to rename it, change its
  days or remove it.
- To change a routine's sets, plan it, change the session, and keep it again under the same name.

There is no list of exercises to choose from and no best-lift history, and the app never
suggests sets, reps or loads: a routine holds only what you put in it.

## Following a diet or a regime

**Food → Diet** and **Fitness → Regime** let you choose something to follow and then show how
your week measures up to it.

**Choosing.** Each option says where it comes from, and they are shelved by that: a public
guideline, an expert position, a single research review, a definition, a diet described by the
body that wrote it, or your own. Press one to see, before anything is saved:

- **What you will be held to** — each number, with whose number it is. You can change any of
  them; that is how you fit an option to yourself. **My own** is the same form with nothing
  filled in.
- **About you**, when the option needs it: sex, year of birth, height, weight and how active you
  are. They are kept in one note of yours and used only to work out numbers that depend on you —
  an energy estimate, protein for your weight, iron for your sex. The energy figure is an average
  for people like you, not a prescription, and says so.
- **What formicaria cannot measure** about it. Free sugars, for instance, are not in the food
  tables; hours without eating cannot be told from entries that have a day and no time.
- Its **sources**, as links.

**Follow this** starts it today.

| In Food | In Fitness |
|---|---|
| WHO healthy diet | WHO weekly activity |
| European reference values (vitamins and minerals too) | WHO weekly activity, 65 and over |
| Energy and protein for me | Falls prevention |
| Two fasting days a week | Strength, twice a week |
| Alternate-day modified fasting | Muscle growth |
| Time-restricted eating | Minimal strength |
| Nordic recommendations 2023 | Stretching for range of motion |
| Planetary health diet | My own |
| My own | |

The fasting patterns are given as an international group of researchers defines them. No public
health body was found recommending them, and the screen says that.

**Being measured.** Once you follow something, the tab shows one row for each thing it asks:
what was asked, where you are today or so far this week, and the seven days from Monday. A day
you logged nothing on reads *nothing logged*, not zero. A weekly amount is only called short
once the week is over.

The hours you went without eating are measured from the times of what you logged: from the last
thing with energy the day before to the first that day. Some things only you can say — whether
you kept to a diet written in portions of food groups. For those there is **I kept to it today**.

A diet's energy and protein numbers also become the targets on **Food → Day**.

**How to do it** links to pages and videos by the body itself, such as the NHS's strength
exercises. They open in your browser; nothing of theirs is kept in the app.

**Change its numbers** starts a new set from today and leaves earlier weeks measured against what
they had. **Choose another** replaces it. **Stop following** ends it; undo brings it back.

Not offered: Mediterranean, DASH, vegetarian, vegan and low-carbohydrate diets. The food tables
cannot say how many servings of a food group a day held, or that a mixed dish contains no meat,
and nothing here is measured by guessing.

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
