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

To look a food up — its energy, protein, vitamins, and how much "one cup" weighs — formicaria uses
a list of about 13,600 generic foods from the U.S. Department of Agriculture. It is a file of about
8 MB that is kept on the device, so looking things up afterwards needs no connection.

It is **not fetched until you ask**. The same Settings section shows its size and where it comes
from, and a control to get it. Removing it gives the space back and changes nothing you have
logged: every entry carries its own numbers.

Without the list you can still log anything by name — see *Quick add* below.

There are no branded products and no barcode reading.

## Logging a food

Open **Food → Day** and press **＋**, or the **＋** beside a meal.

- **Recent** — what you have logged before, most often at this meal first. The **＋** on a row
  logs it again with the amount it had. Tapping the row lets you change the amount first.
- **Search** — a food from the list. Choose one, then say how much: the **−** and **＋** change the
  amount, and the chips are household portions ("1 cup", "1 medium").
- **Quick add** — any name and its energy. Protein, carbohydrate and fat are optional.

Tap a logged entry to change its amount or remove it. Removing one offers **Undo** at once; undo
also works from the ↶ menu, one step for each thing you did.

An empty meal offers **Same as yesterday** when yesterday had something there.

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
  before are offered, with the numbers they had.
- **Same as last time** — when nothing is planned for today, this plans the sets of your last
  session again, ready to change.
- Tap a set's numbers to change them or remove the set.
- **Tick a set** when it is finished; a rest timer starts.

**Training → Volume** is the trend view over what you lifted.

There is no list of exercises to choose from, no best-lift history, and no way to keep a named
session plan.

## What is kept, and where

Each logged food or set is a note with a `log` property (`food`, `workout`), the day it belongs
to, and whole numbers whose unit is in the name — `kcal`, `protein_mg`, `iron_ug`:

```yaml
title: Banana, raw
start: 2026-10-11
amount_g: 118
kcal: 105
log: food
protein_mg: 1286
slot: breakfast
source: usda-foods@2026.10/2709224
```

These notes are kept out of the board, the agenda, the timeline and search, so a year of meals
does not bury your notes. They are backed up and shared with a notebook like any other note, and a
removed one is in *Recently deleted*.

Targets are an ordinary note carrying `program: food` — it is on your agenda from the day it
starts.
