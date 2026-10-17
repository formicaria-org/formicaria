# Appearance

Formicaria ships with a dark look and a warm light one, switched in **Settings → Look → Theme**. Beyond
that you can change how it looks yourself — and because the result is a file in your vault, it
travels with your notes.

## The quick way

**Settings → Look → Appearance** asks three questions:

- **Accent** — the colour of links, buttons and the focus ring. Four presets, or any colour you like.
- **Text size** — small, normal or large.
- **Font** — the system font, a serif, or a monospace.

Answering any of them creates a theme called *My appearance* in your vault and switches to it. There
is no save button: the change is written as you make it.

## Starting from a suggestion

Under **Start from a suggestion** are four ready-made themes. **Use** copies one into your vault as a
theme of your own and switches it on — from then on it is yours, and **Edit** changes it like any
other. Using one again, or after you have changed your copy, makes a second copy (*Paper 2*); it
never overwrites the first.

| Theme | What it is | Dark / light |
|---|---|---|
| **Paper** | warm off-white, brown accent, a serif — like a printed page | always light |
| **Night ink** | deep blue-black with a soft blue accent, for working at night | always dark |
| **High contrast** | black on white (or white on black), clear borders, a strong accent | follows your choice |
| **Large and calm** | bigger text, more space between lines, a shorter line to read | keeps your colours |

They are also the quickest way to learn the format: open one with **Edit** and change a line.

## Writing your own

Under the same heading is a list of your themes, each with **Edit** and **Delete**. Edit opens the
theme's text, and it is applied **as you type** — so you can see a change before you commit to it.
Nothing is written to your vault until you press **Keep**; **Cancel** puts back exactly what was
there.

A theme is ordinary CSS that sets the app's own names:

```css
:root {
  --bg: #f4f1ea;
  --surface: #ffffff;
  --text: #2b2b2b;
  --accent: #8a5a2b;
  --font-sans: Georgia, 'Iowan Old Style', serif;
  --radius-md: 4px;
}
```

If a theme contains anything the three questions cannot describe — a rule of your own, a media
query, a name outside the list below — the questions switch themselves off rather than overwrite
what you wrote. Edit it as text instead.

## If a theme makes the app unusable

It can happen: a theme is CSS, and CSS can hide things.

- While a theme is on but you have not yet clicked anything, a plain **Turn off** button sits in the
  bottom-right corner. It is styled not to inherit your theme.
- If that fails, **close formicaria and open it again**. A theme that was on when you last ran the
  app, and which you were never able to click or type through, is not switched on again — you get a
  message saying so, and the built-in look back.

Nothing about this can lose a note. A theme only changes how things are drawn.

## Where themes live

`<your vault>/themes/*.css`, one file per theme, versioned with your notes. Copy your vault to
another computer and your themes come with it.

**Which theme is switched on is not part of that.** Like the dark/light choice, it is remembered per
device, so a laptop and a phone can look different, and a collaborator who pulls your vault gets
your theme files without being forced to wear them.

A theme is capped at 128 KB — the entire built-in design system is about 14 KB, so this is not a
limit you will meet by accident.

## Themes and dark/light

A theme that sets colours under `:root` uses them whatever **Settings → Look → Theme** says — *Paper*
stays light even when the app is set to dark. To follow the choice instead, give each mode its own
block, as *High contrast* does:

```css
:root[data-theme='dark']  { --bg: #000000; --text: #ffffff; }
:root[data-theme='light'] { --bg: #ffffff; --text: #000000; }
```

A theme that sets no colours at all — only sizes and fonts, like *Large and calm* — keeps whichever
built-in colours are in use. (Before version 0.8.9, colours in a theme were ignored in light mode.
They now apply in both.)

## The names you can set

These are the app's own design tokens. **They are a promise**: they will keep meaning the same thing
as the app changes. Anything you leave out keeps its built-in value.

**Backgrounds and text**

| Name | What it colours |
|---|---|
| `--bg` | the page behind everything |
| `--surface` | panels, the side bar, board columns |
| `--surface-elevated` | cards, menus, dialogs — anything that sits on top |
| `--surface-hover` | a row or button under the pointer; also tags, counts and dates |
| `--border` | ordinary dividing lines |
| `--border-strong` | the edges of inputs and buttons |
| `--text` | the main text |
| `--text-muted` | secondary text — dates, counts, descriptions |
| `--text-subtle` | the faintest text — hints and placeholders |

**Accent and links**

| Name | What it colours |
|---|---|
| `--accent` | the main buttons, chosen items and highlights |
| `--accent-hover` | the same, under the pointer |
| `--accent-contrast` | text drawn on the accent (keep it readable against it) |
| `--accent-subtle` | a faint wash of the accent behind a chosen item |
| `--link` | links in notes |
| `--focus-ring` | the outline round whatever the keyboard is on |
| `--shadow-sm` · `--shadow-md` · `--shadow-lg` | the shadow under cards, menus and dialogs (any CSS `box-shadow` value) |

**Highlights and meaning**

| Name | What it colours |
|---|---|
| `--tint-a` · `--tint-b` · `--tint-c` | three highlight colours — coloured text and callouts in notes, charts; `--tint-c` is also the highlighter |
| `--u-overdue` | a date that has passed (in the agenda and on the calendar) |
| `--u-soon` | due in the next few days |
| `--u-week` | due this week |
| `--u-later` | due later |
| `--u-none` | no date |
| `--danger-bg` · `--danger-fg` | warnings and anything that deletes: background and text |
| `--ok-bg` · `--ok-fg` | confirmations: background and text |

**Type**

| Name | What it sets |
|---|---|
| `--font-sans` | the font for everything you read and write |
| `--font-mono` | the font for code |
| `--measure` | how long a line of a note may grow (for example `68ch`, about 68 characters) |
| `--text-base` | the size of ordinary text — the one to change first |
| `--text-xs` · `--text-sm` · `--text-md` · `--text-lg` · `--text-xl` · `--text-2xl` | the smaller and larger sizes, smallest first: labels, then headings |
| `--lh-xs` · `--lh-sm` · `--lh-base` · `--lh-md` · `--lh-lg` · `--lh-xl` · `--lh-2xl` | the space between lines for each size (a number such as `1.6`) |

**Shape and spacing**

| Name | What it sets |
|---|---|
| `--radius-sm` · `--radius-md` · `--radius-lg` | how rounded corners are: small controls, cards, dialogs (`0` for square) |
| `--radius-pill` | fully rounded chips and tags |
| `--space-1` … `--space-8` | the spacing steps, from 4px to 64px (`--space-2`, `--space-3`, `--space-4`, `--space-5`, `--space-6` and `--space-7` between) |

Larger spacing takes room from a phone's narrow screen; change the small steps a little rather than
the large ones a lot.

You can of course write any CSS you like, including rules aimed at the app's own elements. That
works — but those are internal and may change between versions, and the names above will not.
