# Where the food a person means lands in a food search (2026-10-31)

> Measured for `food-values-plan-2026-10-30.md`, step 1. The harness is
> `crates/fm-core/tests/food_search_ranks.rs` (ignored by default; it needs the real list), the
> words and the foods they mean are `crates/fm-core/tests/food_search_ranks.tsv`, and the rules
> are `fit` in `crates/fm-core/src/pack.rs`. Food list `foods@2026.10.6`, 18,338 foods.

## The set

66 typed words: 34 in English, then 32 in French, German and Italian, each meaning what its English
word does. The meant food is the plain one: the drink, or the unprepared food — raw where a table
lists raw, an *(average)* where a table gives one. It was chosen by reading the list before any
change, and any member of the food's group (`same`) counts, since a search shows a group once.
A word that is ambiguous is given every reading that is plain: *beef* is any raw cut or mince,
*juice* a fruit juice average or orange juice. (*beef* was widened once, to raw cuts, after the
first run showed the set had named only some; the baseline was measured again with the wider set.)

## The result

| | Before | After |
|---|---|---|
| The meant food first | 22 | **47** |
| In the first five | 48 | **61** |
| Not in the first 30 | 4 | 4 |

**Moved down, by one place each:** *café* 4 → 5, *caffè* 2 → 3, *beurre* 4 → 5 — among kinds of
coffee and of butter, where the meant one (brewed coffee, butter of choice) ties with ground coffee
or salted butter on every rule, and the order between them is a guess either way. *caffè* moved
because folding accents made CREA's own spelling, *Caffe'*, count as the whole word.

**Not found at all, before or after:** *wasser*, *ei*, *käse*, *formaggio*. The German names join
the food into a longer word (*Trinkwasser*, *Hühnerei*), and the index finds a word only by its
first letters; the meant cheese, Ciqual's *Cheese (average)*, has no German or Italian name at all.
No ordering reaches these. `known-issues.md`, *Search — what it cannot find*.

## The rules, and what each was measured to do

Applied in this order, each only among results the one before left tied (`fit`):

1. **The word typed is the name's first word, whole**, or its plural: *Water, municipal* before
   *Watermelon*, *Lemons* for "lemon".
2. **Nothing after the words typed makes it another food.** In an English name a second noun does
   (*Beer ham*, *Water ice*, *Apple tart*), so what follows must be a comma, a bracket, nothing, or
   a word of rule 3. The first version applied this to every language and moved six words down:
   in French and Italian a word after the food describes it (*Riz blanc*, *Oeuf dur*), and German
   joins the other food into one word. Kept for English names only.
3. **The food as it comes**: the name is the food alone, or the very next word (or the bracket
   right after it) is *raw*, *plain*, *natural*, *average* or the same in French, German or
   Italian (*cru*, *nature*, *moyen*, *roh*, *crudo*, *media* …). Counted anywhere in the name it
   lifted *Beer shandy (average)* and *Blue cheese (average)*; counted only right after the food,
   it does not. *Fresh* was in the list and came out: *Fromage frais* and *Pasta fresca* are other
   foods, and with it *fromage* fell from 8 to 20.
4. **The shorter name**, then the index's own relevance. Without length, seven words moved down
   (*pasta*, *onion*, *eau*, *bière*, *café*, *riz*, *beurre*). Breaking a tie by the index's
   relevance instead of alphabetically put two more words first (45 → 47) and moved *beurre* one
   place down.

Accents are taken off for the ordering as the index takes them off for finding (`fold`), so
*biere* is ordered like *bière*.

## Word by word

| Word | Before | After |
|---|---|---|
| water | 6 | 1 |
| beer | 4 | 3 |
| wine | 1 | 1 |
| red wine | 1 | 1 |
| coffee | 4 | 2 |
| tea | 3 | 2 |
| milk | 6 | 1 |
| juice | 13 | 10 |
| orange juice | 1 | 1 |
| egg | 3 | 1 |
| apple | 1 | 1 |
| bread | 9 | 1 |
| rice | 5 | 4 |
| banana | 1 | 1 |
| tomato | 1 | 1 |
| potato | 7 | 1 |
| chicken | 18 | 1 |
| cheese | 13 | 1 |
| butter | 1 | 1 |
| yogurt | 4 | 1 |
| pasta | 4 | 1 |
| oats | 1 | 1 |
| salmon | 2 | 1 |
| beef | 9 | 3 |
| onion | 1 | 1 |
| carrot | 1 | 1 |
| sugar | 2 | 1 |
| honey | 1 | 1 |
| olive oil | 1 | 1 |
| lemon | 1 | 1 |
| orange | 1 | 1 |
| cola | 1 | 1 |
| sparkling water | 1 | 1 |
| almond | 1 | 1 |
| eau | 2 | 2 |
| acqua | 4 | 2 |
| wasser | — | — |
| bière | 2 | 2 |
| bier | 3 | 1 |
| birra | 1 | 1 |
| vin | 4 | 1 |
| wein | 8 | 2 |
| vino | 1 | 1 |
| café | 4 | 5 |
| kaffee | 3 | 1 |
| caffè | 2 | 3 |
| thé | 2 | 2 |
| lait | 7 | 1 |
| milch | 3 | 1 |
| latte | 1 | 1 |
| pomme | 1 | 1 |
| apfel | 3 | 1 |
| mela | 2 | 1 |
| pain | 12 | 1 |
| brot | 2 | 1 |
| pane | 6 | 1 |
| riz | 1 | 1 |
| oeuf | 2 | 1 |
| ei | — | — |
| uovo | 2 | 2 |
| fromage | 8 | 1 |
| käse | — | — |
| formaggio | — | — |
| beurre | 4 | 5 |
| kartoffel | 10 | 1 |
| pomodoro | 2 | 1 |

— means not in the first 30.

## Not measured

- **The person's own foods first** (the second half of step 1) is not a ranking of the list: a food
  logged before is shown above the list, marked *Logged before*, and the list does not repeat it.
  It is tested with the demo vault (`LogSheet.svelte.test.ts`), not measured.
- Words that name a dish rather than a food (*pizza*, *lasagne*), and two-word searches beyond
  *red wine*, *orange juice*, *olive oil* and *sparkling water*.
