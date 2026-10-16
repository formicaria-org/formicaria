# 2026-10-16 (night) — the card library, rebuilt for anyone; v0.8.4

> A snapshot, never updated. Current state is `features.md` and `known-issues.md`; the rulings are
> in `decisions.md` under 2026-10-16.

## How it started

The owner: the cards hold good research, but *"some are not exact diets but indications and some
overlap and for a non trained user we do not have an easy way to understand how to use it … make it
easy for anyone to track important things and try to assess the general trends."* Research came
first — three reviews and an adversarial check (`cards-evidence-2026-10-16.md`), then a proposal of
options (`cards-guidance-proposal-2026-10-16.md`); the owner chose every recommended option and said
to finish without waiting.

## What changed for a person

- **Shelved by kind** (targets, a way of eating, when to eat, weekly amounts, programmes, yours),
  each card with its question, what it rests on, how it relates to the others; **Start here** on
  WHO's two.
- **One card per question**, so a base and add-ons; a session counts for every card it fits.
- **Worth watching** — six sourced numbers for everyone — and a **four-week trend** on every check.
- **Anything we should know?** — six opt-in flags that only show a note, use a body's number, or
  leave a number out.
- **Fixed:** WHO's protein ceiling (it is "generally sufficient", not a limit), EFSA iron after the
  menopause, potassium with kidney disease, folate in pregnancy, protein from 65.
- **New cards:** DASH (read at source, NIH 06-4082), Vegan or vegetarian: what to watch, Activity in
  pregnancy and after (WHO 2020), Steps a day (written down from Trends).

## How it was checked

Engine tests for every flag, the per-question rule and its legacy case, trends and the watch set;
UI tests for the new flow and a flag leaving cards out; the full gate. Not seen on a phone screen.
Not shipped: Mediterranean (MEDAS's reuse terms unconfirmed).
