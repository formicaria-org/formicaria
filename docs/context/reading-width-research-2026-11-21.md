# Reading width — how wide a note reads, and how a person changes it (research, 2026-11-21)

**Asked by the owner:** a note on a laptop reads *"super short in width"*; research what other apps and
the usability literature do about line length, and how to let a person resize it as conventionally as
possible. Built in 0.60.7 as recommended (`decisions.md` 2026-11-21 (evening)); it sits in the Settings plan's
Look category (`settings-plan-2026-11-21.md`).

## What fm does today (read off the code)

- A note in its own tab (`/f/note/…`) is *solo* and *wide*: its pane fills the window
  (`NotePanel.svelte`, `.panel.wide.solo`).
- The **text column is capped at `--measure: 68ch`** (`app.css`), centred, and the cap **includes** the
  24 px padding on each side (`.read`, `box-sizing: border-box`). The text itself is therefore ≈ 68ch − 48 px,
  about 600 px at the default 16 px. On a 1920 px-wide laptop that is under a third of the window, with
  two empty margins wider than the text.
- **There is no way to change it**: no setting, no per-note choice, no shortcut. Browser zoom
  (Ctrl +/−) makes the letters and the column bigger together, so the *characters per line* stay
  the same and the margins shrink.
- Tables, code blocks and images live inside the same 68ch column. Only a few elements scroll sideways
  (`overflow-x: auto`); nothing breaks out to the pane's width.
- Notes in the workspace grid get narrower still: up to 4 columns (`autoCols`, `panes.ts`), and a trail
  column of `clamp(32rem, 42vw, 44rem)`.

## What the research says

| Source | Finding |
|---|---|
| Baymard Institute | 50–75 characters per line is best for body text. Fatigue shows at 100+. |
| Dyson & Haselgrove / Dyson & Kipping (via *100 Things Every Designer Needs to Know*) | On screen, ~100 characters per line is read **fastest**, but people **prefer** ~45–72. |
| Nanavati & Bias (2005), literature review | 50–70 characters per line is easiest. Very short and very long lines are both disliked. |
| Bringhurst | 66 characters is the classic "ideal" for a single column. |
| WCAG 1.4.8 (AAA) + technique C20 | Text should be able to go to ≤ 80 characters (40 for CJK). The mechanism may be the browser's own (resizing the window, zoom). Relative units (`em`/`ch`) let it follow the font size. Authors are **not** asked to cap width in the default view. |

**Read together:** a cap is right, and 68ch sits inside the preferred band. But the band is a
*preference*, not a law. Wider is faster for skimming. Tables, code and boards want the full width.
The research consistently points to **a sensible default plus a person's own choice**, not a fixed number.

## What comparable apps do

| App | Default | How a person changes it |
|---|---|---|
| **Obsidian** | "Readable line length" **on**, ~700 px (`--file-line-width`) | One global **on/off toggle** in Settings → Editor. Off gives the full pane. Its most-installed width plugins all add the missing middle: a status-bar **slider** (slyde, Editor Width Slider), a number in px/ch (Width Ruler, Width Wizard), a **per-note** full-width switch (Page Width Toggle, Full Width Toggle, Per-Page Readable Width), and a toggle command with a hotkey. |
| **Notion** | Narrow, centred | **Per-page "Full width"** switch in the page's ⋯ menu. No global default. Full-page databases are always full width. |
| **iA Writer** | 64 characters | Settings → Editor → **Line width: 64 / 72 / 80 characters**. It only changes how text wraps on screen. |
| **Bear** | Medium | Preferences → Editor → **Line width** (steps of 8 em). Users ask for finer steps, and say "max" does not fill a wide window. |
| **Firefox Reader View** (fm's own engine) | ~40 em | The Reader View toolbar has a **Content width** slider (and font size, line height). It offers fewer steps in a narrow window. |

**The conventions that recur:**
1. a readable default (60–75 characters);
2. a **global setting** with a few named widths, the last of them "full";
3. a **per-note full-width switch** in the note's menu (Notion's pattern, and Obsidian's most-wanted plugin);
4. **width follows font size** (`ch`/`em`), so zoom keeps lines readable;
5. **wide content escapes**: tables and databases get the full width even when the text does not.

Drag handles on the text column itself exist only as niche plugins, and no mainstream app ships them.

## Recommendation for fm

1. **Settings → Look → Reading width**, four named choices, phone hidden (its pane is the screen):
   *Narrow* (60 characters) · **Comfortable (75, the default)** · *Wide* (100) · *Full* (the pane's width).
   Measure the **text**, not text plus padding: move the padding outside `--measure`, so "75" means
   75. Comfortable becomes slightly wider than today, still inside Baymard's band. Wide sits at
   Dyson's "fastest". iA Writer's 64/72/80 and Bear's steps show named steps beat a free number.
2. **Per note: "Full width" in the note's ⋯ menu**, Notion's word for it. It is kept per device
   (a layout choice, not the note's content), keyed by note id, so a phone never inherits it and
   the file is not rewritten.
3. **Wide content breaks out of the column** under any setting except Full: tables, code blocks,
   images, embedded boards and the calendar take the pane's width (minus a margin) and scroll
   sideways if still too wide. This is what Notion does for databases. It also removes the main
   reason people turn readable width off.
4. **Keep it in `ch`**, so Ctrl +/− (already there, Firefox's own) enlarges the letters while the
   characters per line stay the same. That is WCAG C20's technique and the most conventional
   "resize" there is.
5. **A command, no new shortcut**: "Reading width: …" in the command list. Obsidian's toggle plugin
   uses Ctrl/Cmd+Shift+E, but in Firefox that key opens the Network monitor, so it is not taken.
6. **The workspace grid** keeps its column count. A note shown in a narrow grid column just uses
   that column (any choice is capped by the pane), and the existing widen button stays the way
   to give one pane the whole screen.

**Tests when built:** a unit test of the width each choice yields. A usability-check persona at
1920×1080 measures the text block's width in characters for each choice, a table breaking out, and
per-note Full width surviving a reload.

## Sources

- Baymard Institute, *Line length readability*: https://baymard.com/blog/line-length-readability
- *100 Things Every Designer Needs to Know*, ch. 18 (Dyson): https://oreilly.com/library/view/100-things-every/9780132658638/ch18.html
- *The ideal line length for digital text* (Nanavati & Bias summary): https://dev.to/robole/the-ideal-line-length-for-digital-text-2d5i
- W3C, Understanding SC 1.4.8: https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation
- W3C, Technique C20: https://www.w3.org/WAI/WCAG21/Techniques/css/C20
- Obsidian width plugins: https://community.obsidian.md/plugins/width-ruler ,
  https://community.obsidian.md/plugins/page-width-toggle , https://community.obsidian.md/plugins/toggle-readable-line-length ,
  https://forum.obsidian.md/t/adjustable-readable-line-length/7564?page=2
- Notion, *Style & customize your page*: https://notion.com/help/customize-and-style-your-content
- iA Writer settings: https://ia.net/writer/support/basics/settings
- Bear line width: https://community.bear.app/t/need-finer-width-options/11763
- Firefox Reader View: https://support.mozilla.org/kb/firefox-reader-view-clutter-free-web-pages
