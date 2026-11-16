# formicaria's browser

Browse the web inside formicaria and keep what you find as a note, in one press: a recipe, a paper,
an article, a video or a song, with a copy of the page as it was.

On a computer, this is **Linux for now**. The phone gets its own browser built in, in a later
version.

## Set it up

**Settings → Browser → Set up formicaria's browser.** formicaria downloads **Firefox ESR**, Mozilla's
long-term-support browser engine, **from Mozilla**, checks it against **Mozilla's signature**, and makes
it formicaria's: its name, its icon and its look, with formicaria beside the page you are reading. It is not a
fork of Firefox: Mozilla's own build, with formicaria's touches added on this computer. It is about 90 MB to download and 320 MB on your computer. It goes in formicaria's
settings folder (`~/.config/formicaria/browser/`), with a profile of its own, so it is separate from
any Firefox you already use.

From then on formicaria opens in it when it starts. To go back to your usual browser, turn off
**Open formicaria in its own browser when it starts**.

It starts out private:

- **strict tracking protection**, which also isolates each site's cookies;
- **HTTPS-Only**: pages are loaded over an encrypted connection;
- **Global Privacy Control** tells sites you do not want your data sold or shared;
- **cookie banners** are refused where Firefox knows how;
- **DuckDuckGo** is the search engine (change it in Firefox's settings);
- **Safe Browsing** is on, as in any Firefox: Firefox checks pages against Google's list of
  dangerous sites;
- **nothing is sent to Mozilla** (telemetry and studies are off);
- **uBlock Origin** is offered, not installed: turn on **Install uBlock Origin** in Settings →
  Browser to block ads as well. (It is off by default because, measured, it made the browser use
  much more memory; Firefox's own tracking protection is on either way.)
- Mozilla's **telemetry, studies, Pocket, sponsored content, Firefox Suggest and the AI chatbot are
  off**.
- **Sites cannot send you notifications while their tab is closed** unless you turn on that switch in
  Settings → Browser.

**Your settings in Firefox's own Settings page are yours**: formicaria only sets the starting values,
and what you change there stays.

**Your Mozilla account and Sync work**, if you want them: sign in (Settings → Account and sync) to
bring your bookmarks, passwords, history and add-ons from another Firefox.

Your history, cookies and sign-ins stay on this computer, in the browser's profile. They are not
part of your notebook and are never synced.

Other Firefox add-ons work too: install them from Firefox's add-on site as in any Firefox.

## Save a page

On any page, press the **ant in the address bar** (or **Alt+Shift+K**), or right-click the page and
choose **Save page in formicaria**. formicaria reads what the page says it is and saves it:

| The page is | It becomes |
|---|---|
| a **recipe** (the page publishes its recipe for search engines, as most recipe sites do) | a recipe note with its *Ingredients*, *Method* and *Times*, in **Food → Recipes**, its ingredient lines ready to match to foods |
| a **paper** (the page carries its citation, as journal and arXiv pages do) | a paper note with its authors, year, DOI and journal, like one made from a pasted BibTeX entry, with its abstract |
| a **video** (YouTube, Vimeo and others) | a note with the link and the title, on the **Web** shelf; the video itself is not downloaded |
| a **song or album page** (Bandcamp and others) | a note with the link, on the **Web** shelf; music you have bought is added with **Add a song** |
| anything else | an **article**: its text, read the way Firefox's reader view reads it, on the **Web** shelf |

**Every saved page ends with a link to a copy of the page as it was when you saved it**, stored in
your vault like any attachment. Large copies stay off git under the vault's size limit for
attachments, as every attachment does.

If you **select text** before saving, it goes at the top of the note as a quote, with a link that
reopens the page with that passage highlighted.

Then press **Open in formicaria** to go to the note. It is yours: edit it, tag it, move it.

## formicaria beside the page

- **formicaria sits beside the page**, and **the tabs are for the web**. Drag the line between them
  to make formicaria wider or narrower; the width is kept.
- **⇆** at the top of formicaria moves it around the page: **right, below, left, above**, in turn. **⤢** makes it **fill the
  window** (or double-click the line between them); choosing a page in the tab list puts it back
  beside. **✕ folds it away**; the **ant button** at the left of the toolbar, or **Alt+Shift+F**,
  brings it back.
- There is **one formicaria**, in one window: opening formicaria again (double-clicking it, say)
  shows it there rather than opening a second one, and a link to a note opens in it. A link in a
  note opens as a tab beside it.
- **Tabs are across the top**, as in any browser: click to switch, drag to reorder; **scroll over
  them** to move through them; **Ctrl+Tab** shows your recent tabs as cards to pick from. Prefer them
  down the side? **Settings → Browser → Tabs down the side** (always shown, never behind a button).
- **Your tabs come back** when the browser starts again.
- The window takes **formicaria's colours**, dark or light as your system is, and shows **formicaria**
  in the dock and Alt+Tab.

## Bring things in, or save the page

Two things you can do with what you find, and the same words everywhere:

| You want | Do | What happens |
|---|---|---|
| **this in what you are writing** | **Snip it**: press the **ant in the address bar** (or **Alt+Shift+S**, or hold **Ctrl+Shift** and drag on the page) and drag over what you want. Or **select text**: formicaria's icon appears right beside it; press it. Or right-click it → **Bring into "…"**, or drag it onto formicaria | it goes into **the note in front** in formicaria (its name is on the icon's tooltip and the snip's bar). With no note open, a new note named after the page is made and opened |
| **the whole page, for later** | **Save the whole page** in the snip's bar, or right-click the page → **Save page in formicaria**, or **Alt+Shift+K** | the page becomes **its own note**: a recipe in Food, a paper with its citation, anything else on the **Web** shelf, with a copy of the page |

**The snip's bar also offers what the page plainly has**, one press each: **The main text**, **The
images**, **The table**, or **Save as a recipe** / **Save as a paper** when the page is one. And when
your assistant is running, **Ask Myrme what matters here**: it reads the page and proposes the
passages that matter for the note in front, as proposals you accept; nothing is added until you do.

**A snip brings everything in the area, each as itself**, in reading order: paragraphs, headings,
lists, tables, code, images, links, a video at the moment it was playing. What has no text behind it
(a chart drawn by the page, a map) comes as a **picture of the area**. If you would rather have the
area as one picture, press **As a picture instead**.

**What happened is said where you did it**: a small bubble on the page, with **Undo** and the one
other choice.

**What arrives is what you pointed at:**

- **text** comes quoted, with its bold, italics, links and lists, and a link that reopens the page at
  that passage;
- an **image** is **kept in your vault** (large ones come as a link, with *Keep a copy* one press
  away);
- a **link** comes as a link;
- a **table** comes as a table, **code** as a code block;
- a **video or audio file** comes as a link, with *Keep the file* one press away; a **video on a
  streaming site** comes as a link to the page **at the moment it was playing**.

After each one, the bubble on the page (or, from a menu or a drop, a line in formicaria) says what
came in, with **Undo** and the one other choice (*Link only instead*, *Keep the file*).

**What is hard to select** (an image in a slideshow, a whole table): right-click the page → **Pick
from this page…**. What is under the pointer lights up and says what it is; **scroll** to widen it to
what contains it, or narrow it back; **click** to bring it in (Shift+click to bring several); **Esc**
to stop.

**After you select something on a page**, formicaria does what you choose in **Settings → Browser**:
**show formicaria's icon beside it** (the starting choice), **bring it in at once** (every selection
goes in; hold Alt to just select; a double-click on a word never does), or **nothing**.

**This page is in your notes**: when a note already names the page you are on, the ant in the
address bar is **filled red**; press it, and the snip's bar says **In 2 of your notes**: press that to
see them, each with the line that names the page, and open one beside the page.

**Search your notes from the address bar**: type **fm**, a space, and what you are looking for.

**Exactly where it came from.** A saved page ends with the **full address it was read at**, with the
date and time, and, when the page names a different address to cite (its canonical address, a
paper's abstract page), **that one too**; a paper's citation records the date it was read. Every
piece you bring in ends with a link to the page **at that passage**, and when. The note's **source**
is `web`, with the exact address and the time. Every source formicaria reads from (mail, calendars, files, and later a watch)
describes what it brings the same way.

## Bookmarks

After saving a page, press **Just the link instead** in the bubble: a note with the page's title
and link, on the **Bookmarks** shelf, without a copy of the page. Like every note, it travels to your
other devices with your notebook.

## Updates

formicaria keeps its browser up to date by itself. Mozilla's security releases are checked once a day,
fetched, checked against Mozilla's signature and prepared in the background, by formicaria itself while
it runs. If the browser is closed, the new copy goes in straight away; if it is open, formicaria shows
a line at the top: **close the browser's window and open formicaria again** (your tabs come back). If
a new copy does not start, the previous one stays. A formicaria update that changes the browser's
look works the same way.

## Downloads

Every download asks **where it should go**, and keeps **one copy**:

- **Keep in formicaria** (the default: press Enter). A song goes to **Music**. An album bought as a
  **zip** (from Bandcamp and the like) becomes its songs in Music; the zip itself is not kept. Any
  other file becomes a note with the file attached, on the **Web** shelf, saying where it came from.
  Nothing is left in your Downloads folder.
- **Downloads folder**: the download carries on as in any browser.
- **Choose a folder…**: Firefox asks where to save it.

Closing the question without choosing sends the file to your Downloads folder, so a download is
never lost. If formicaria cannot take a file, it is saved to Downloads and a notification says why.

## What it does not do

- It does not download videos or music from streaming sites.
- It sends nothing anywhere but your own formicaria: the page is read in your browser and kept on
  your computer.
- Windows and macOS: not yet.
