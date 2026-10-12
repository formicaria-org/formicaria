# Food and training

formicaria can keep a food log and a training log beside your notes: what you ate against a target,
how a week went, and the sets of a session. What you log is
kept as notes in your own notebook — plain files, like everything else.

> **Early.** New in 0.7.0, and used so far by one person on Linux and on an Android phone. On an
> iPhone, a Mac or a Windows computer it has not been tried.

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

## Planning a meal ahead

Step to a day after today — the day screen reaches a week ahead — and add as usual. What you add
there is a **plan**: it is shown in a lighter hand, and is in no total until you press the tick
beside it. *Same as yesterday* on a day ahead plans it too.

## Trends

**Food → Trends** shows a week, four weeks or twelve. A menu at the top picks **what to show** —
energy, protein, carbohydrate, fat, or any other value the day screen has — and that one is drawn
day by day against its target; the averages of the others are listed below it. It says how many of
those days have anything logged, because an average over three logged days is not a week's
average.

There is no screen for your weight at the moment: it is to become one more thing to pick on this
menu, and until it is, the app has no way to log one. Weights you logged earlier are still in
your notebook.

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

There is no list of exercises to choose from, no best-lift history, and no way to keep a named
session plan.

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

Some things only you can say — whether you ate inside your hours, whether you kept to a diet
written in portions of food groups. For those there is **I kept to it today**.

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
