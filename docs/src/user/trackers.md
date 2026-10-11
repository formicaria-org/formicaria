# Food and training

formicaria can keep a food log and a training log beside your notes: what you ate against a target,
how a week went, and the sets of a session. It is off until you turn it on, and what you log is
kept as notes in your own notebook — plain files, like everything else.

> **Early.** The screens were judged in a phone's browser against made-up data. The installed
> app, on a phone, with a real notebook, has not been tried yet.

## Turning it on

**Settings → Notes & backup → Food and training → Show Food and Training on this device.**

The choice is this device's own, like the theme that is on: turning it on at your desk does not
turn it on on your phone. **Food** and **Training** then appear among your views. On a phone, the
bar at the bottom becomes that group's own while you are in it — its views, and one control to
add. The first control on the left brings the usual bar back.

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
- **Remove this recipe** takes the recipe and its ingredients away in one step, and can be undone.
  What you already logged from it stays as it was.

What a recipe comes to is **the sum of its ingredients as you entered them, divided by its
servings**. Nothing is adjusted for cooking: the app does not estimate how much water a dish
loses or how much of a vitamin survives the heat. Choose the cooked form of a food where the list
has one ("Rice, cooked") if that is what you weighed. When an ingredient has no figure for
something — a food with no iron value, say — the recipe says so: *Iron from 3 of 4 ingredients*.

A recipe cannot be changed once saved, only removed and made again, and an ingredient has to be
a food from the list.

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

**Food → Trends** shows a week, four weeks or twelve: each day's energy against the target, and the
average of everything else. Averages are over the days you logged, and the screen says how many
those were. A target you changed shows as a step on the day it changed.

**Food → Weight** is a line of your weight over the same spans: where it is now and how far it has
moved. Press **＋** to write one down. Two readings on one day count as one, their average.

## Training

**Training → Session** is today's session: each exercise, each set, and what you did last time
beside it.

- **Add an exercise** — its name, how many sets, the reps and the load. Exercises you have done
  before are offered, with the numbers they had. Nothing is filled in for you otherwise: the
  app does not suggest sets, reps or loads.
- **Same as last time** — when nothing is planned for today, this plans the sets of your last
  session again, ready to change.
- Tap a set's numbers to change them or remove the set.
- **Tick a set** when it is finished; a clock starts counting the time since. It does not tell
  you how long to rest.

**Training → Volume** is the trend view over what you lifted. *Volume load* is repetitions ×
load, added up over the sets you ticked — the usual meaning of the term in the training
literature. It is a rough measure of work done, not a score.

There is no list of exercises to choose from, no best-lift history, and no way to keep a named
session plan.

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
