# 2026-07-20 — One plus, one gear, one lens

Same day as `2026-07-20-capture-on-a-real-phone.md`, different thread: the top bar.

## What was wrong

The owner's reading, and it was exactly right: *"new and the view and cmd palette are all calling
the same basically thing"*. Literally so — **`New` and `View` both ran `openSettings('commands')`**,
the same line, and the palette they opened restated the same actions a third time with different
wording (`Open Board` against `Board`). Three controls and a wordmark for one idea.

## What it is now

| Before | After |
|---|---|
| `formicaria` wordmark | gone — an app does not need to tell you its name inside its own window, and it was ~90px of a 411px bar |
| `＋ New` and a `View` button | **one red circular plus**, opening a short anchored menu |
| a permanently-open search field | **a lens**, which expands on tap and collapses when left empty |
| palette repeating all of it | the palette now **spreads the same list** |

The plus menu is **three items**: *New note*, *New board*, *New window*.

That last one is the second, larger simplification, and it came from the owner: *"The red plus
should have note, board and window. Then one should be able to quickly change the view of a
window."* A first pass had the plus list every view — Board, Agenda, Timeline, Search, Activity,
plus saved views — which meant a menu that **grows with the app** and asks you to decide what you
want to look at before you can see anything. A window opens on the board and is rotated
afterwards, which costs one scroll or one swipe.

**New vault is deliberately not in it** — you make a vault a handful of times ever, and it lives
in Settings. Every view is still reachable *by name* in the palette, which is the searchable
surface: typing "timeline" should work without knowing that a window is the thing that holds
one.

## Rotating a window, where the hand already is

The rotator existed — click the small label in the pane header, or scroll it — and it went
unfound, because it is one modest target among that header's controls with nothing saying
"scroll me". So the gesture moved to the whole header:

- **Wheel anywhere across the pane header** spins the view, either direction.
- **A horizontal swipe** does the same on a touch screen. Left is forward, the way a carousel
  moves.

**One gesture is one step.** The first version rotated on *every* wheel event, and the owner hit
it immediately: *"I barely move my hand, several views change too quickly."* A wheel does not emit
one event per notch — a mouse sends a burst and a trackpad sends a long stream plus inertia after
your fingers have left it. Three things together, all needed: deltas **accumulate to a 120px
threshold** (one notch), a **250ms cooldown** stops inertia banking a run of steps, and the
leftover is **discarded** rather than carried, because carrying it is what turns one emphatic
scroll into four views. A pause or a **reversal** resets — and a reversal also clears the cooldown,
since inertia never reverses, so scrolling back one answers at once instead of feeling stuck.

Deltas are normalised to pixels first: `deltaMode` is lines on Firefox and pages in some
configurations, so comparing the raw number against a pixel threshold would make the same gesture
~16x less sensitive there.

Both are ignored when the intent is clearly something else: a wheel event whose `deltaX` exceeds
its `deltaY` is a trackpad flick (that is how a board is read), a drag under 48px is a tap that
wandered, and a mostly-vertical drag is a scroll that began on the header. The header is *also*
the drag handle, so a trigger-happy threshold would change the view every time a pane was picked
up.

**Not the pane body.** A board scrolls horizontally by design and the agenda vertically; taking
those gestures from the content would trade one undiscovered action for two broken ones.

A note pane is exempt — the rotator's options are ways of looking at a *collection*, and a note
is one document.

## Two rulings changed on purpose

- **The codebase now has an anchored menu.** It had avoided one on the grounds that `.topbar` is
  a scroll container that would clip it. That is true of an `absolute` menu; this one is `fixed`,
  so no ancestor's overflow can reach it. The old answer — route every creation through a
  full-screen palette — is what made "new note" feel like a trip to settings.
- **The palette is derived, not written twice.** `CREATE_MENU` is spread into `commands`, so an
  item added to the plus appears in the palette for free and the two cannot disagree. Writing
  them separately is precisely how the *previous* palette drifted into a second Settings.

## The screenshot caught what the types could not

`svelte-check` was clean and the emulator showed **a red ellipse**. The `@media (pointer: coarse)`
block sets `min-height: 2.75rem` and horizontal padding — correct for a pill-shaped chip, wrong
for a round button, whose width stayed at `2rem`. A circle needs both axes set together.

Second near-miss, caught by reading rather than by eye: the collapsed search button was first
written as `.icon-btn`, and **narrow layouts hide every `.icon-btn` in the top bar** (those
controls live in the bottom `ViewBar` instead). It would have vanished on exactly the screen the
collapsing is for. It has its own class now.

This is the third session in a row where the emulator screenshot was the thing that found the
bug. `adb exec-out screencap -p` is the cheapest check available for anything visual, and driving
it with `input tap` exercises the menu itself — verified here: plus → menu → *Agenda* opens the
pane, and the lens expands and takes focus.
