# Appearance

Formicaria ships with a dark look and a warm light one, switched in **Settings → Theme**. Beyond
that you can change how it looks yourself — and because the result is a file in your vault, it
travels with your notes.

## The quick way

**Settings → Appearance** asks three questions:

- **Accent** — the colour of links, buttons and the focus ring. Four presets, or any colour you like.
- **Text size** — small, normal or large.
- **Font** — the system font, a serif, or a monospace.

Answering any of them creates a theme called *My appearance* in your vault and switches to it. There
is no save button: the change is written as you make it.

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

## The names you can set

These are the app's own design tokens. **They are a promise**: they will keep meaning the same thing
as the app changes.

`--bg` · `--surface` · `--surface-elevated` · `--surface-hover` · `--border` · `--border-strong` ·
`--text` · `--text-muted` · `--text-subtle` · `--accent` · `--accent-hover` · `--accent-contrast` ·
`--accent-subtle` · `--link` · `--focus-ring` · `--shadow-sm` · `--shadow-md` · `--shadow-lg` ·
`--tint-a` · `--tint-b` · `--tint-c` · `--u-overdue` · `--u-soon` · `--u-week` · `--u-later` ·
`--u-none` · `--danger-bg` · `--danger-fg` · `--ok-bg` · `--ok-fg` · `--font-sans` · `--font-mono` ·
`--measure` · `--text-xs` · `--text-sm` · `--text-base` · `--text-md` · `--text-lg` · `--text-xl` ·
`--text-2xl` · `--lh-xs` · `--lh-sm` · `--lh-base` · `--lh-md` · `--lh-lg` · `--lh-xl` · `--lh-2xl` ·
`--radius-sm` · `--radius-md` · `--radius-lg` · `--radius-pill` · `--space-1` · `--space-2` ·
`--space-3` · `--space-4` · `--space-5` · `--space-6` · `--space-7` · `--space-8`

You can of course write any CSS you like, including rules aimed at the app's own elements. That
works — but those are internal and may change between versions, and the names above will not.
