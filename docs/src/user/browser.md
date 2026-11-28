# formicaria's browser

Browse the web inside formicaria and keep what you find as a note, in one press: a recipe, a paper,
an article, a video or a song, with a copy of the page as it was.

**It is how formicaria runs on a computer** (Linux, Windows and macOS): formicaria brings its own
browser, so it needs no other. On the phone, web pages open inside the app (see the phone's page).

## The first start

The first time formicaria starts, it gets its browser ready by itself, then opens in it: about a
minute or two, said in the window formicaria starts from and, where the computer can, in a
notification. formicaria downloads **Firefox ESR**, Mozilla's long-term-support browser engine,
**from Mozilla**, checks it (against **Mozilla's signature** where the computer can check one,
otherwise against the sum this formicaria release recorded), and makes it formicaria's: its name,
its icon and its look. It is not a fork of Firefox: Mozilla's own build, with formicaria's touches
added on this computer. It is about 90 MB to download and 320 MB on your computer, in formicaria's
settings folder (`~/.config/formicaria/browser/` on Linux), with a profile of its own, so it is
separate from any Firefox you already use.

If it cannot be set up (no connection, say), formicaria opens in the computer's own browser for that
time, and **Settings → Browser** says why, with **Try again**.

It starts out private:

- **strict tracking protection**, which also isolates each site's cookies;
- **secure first**: every page is tried over an encrypted connection, and a site that has none
  opens over plain http, marked *not secure*, as in Firefox (Privacy → **Always https** asks before
  such a site instead);
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
| a **video** (YouTube, Vimeo and others) | a note with the link and the title, tagged `shelf:Web`; the video itself is not downloaded |
| a **song or album page** (Bandcamp and others) | a note with the link, tagged `shelf:Web`; music you have bought is added with **Add a song** |
| anything else | an **article**: its text, read the way Firefox's reader view reads it, tagged `shelf:Web` |

**Every saved page ends with a link to a copy of the page as it was when you saved it**, stored in
your vault like any attachment. Large copies stay off git under the vault's size limit for
attachments, as every attachment does.

If you **select text** before saving, it goes at the top of the note as a quote, with a link that
reopens the page with that passage highlighted.

Then press **Open in formicaria** to go to the note. It is yours: edit it, tag it, move it.

## One row: your tabs

- **On formicaria's own tabs there is one row**: **← and →** to go back and forward, formicaria's
  button (the ant, at the left end of the tab row on every tab: Save media, Privacy, Settings
  and Help, and a search of your notes and the web), your tabs, **+**, and **⚙ Settings** at the right
  end. The address row appears only on web pages, which need one, and has its own ← and →.
- **Settings is always at the top right** (⚙, at the right end of the tab row), on every tab: a note,
  a feature, a web page. It opens Settings in a tab of its own, or goes to the one already open.
- **🔥 beside ⚙ forgets the web**: it asks first, then closes the web's tabs and forgets their
  cookies, stored data, cache and history, keeping formicaria's tabs and the sites you keep signed in.
  [Forget web data](./privacy.md#forget-web-data-).
- **A private window**: the ant's **New private window**, or **Ctrl+Shift+P**. It is Firefox's own:
  nothing from it is remembered once it closes. formicaria's protections, ad blocking and its button
  work in it as in any window.
- **Every tab is one thing**: a feature (Notes, Tasks, Agenda, Food journal, Fitness, Health, Music), a
  single note, or a web page. A feature's tab starts with its line: the feature's name (**Food ▾**)
  switches *this* tab to another feature (Back returns), and its views follow (Day · Menu · Trends).
- **+ (or Ctrl+T) suggests**: a new tab offers the features, a new note, and a field for a web address
  or a search. What you choose fills the tab.
- **Opening a note from a feature opens it in a tab of its own**, as a link would.
- **Select text inside formicaria** and its icon appears beside it: bring the passage into the note you
  looked at last, any note, or a new one. (On web pages, the same icon; see below.)
- **Two tabs side by side**: drag a tab onto the left or right half of the page (Firefox's split
  view), for any two tabs.
- **Switch back in one press**: **Alt+Shift+X** goes to the tab you had before, and again to come back.
- Close any tab as you would any tab: its **×**, **Ctrl+W**, or a middle-click.
- **Your tabs come back** when the browser starts again, unless **Forget when closing** is on (then
  only formicaria's own tabs do). An old whole-app tab becomes Notes.
- **Updates restart the browser by themselves.** After formicaria updates, its browser saves what you
  were typing in every open note, closes, and opens again on the same tabs within a minute, with the
  update in place. Nothing to do by hand.
- Prefer tabs down the side? **Settings → Browser → Tabs down the side**.

## Bring things in, or save the page

Two things you can do with what you find, and the same words everywhere:

| You want | Do | What happens |
|---|---|---|
| **this in what you are writing** | **Snip it**: press the **ant in the address bar** (or **Alt+Shift+S**, or hold **Ctrl+Shift** and drag on the page) and drag over what you want. Or **select text**: formicaria's icon appears right beside it; press it, and choose where it goes: **the note in front** (first, so Enter is enough), **another note** (type to find it; it goes in without the note opening, so you stay on the page), or **a new note**. Or right-click it → **Bring into "…"** | it goes into **the note in front**: the note whose tab you looked at last (its name is on the icon's tooltip and the snip's bar), straight into the note, open or not. With no note looked at yet, a new note named after the page is made |
| **the whole page, for later** | **Save the whole page** in the snip's bar, or right-click the page → **Save page in formicaria**, or **Alt+Shift+K** | the page becomes **its own note**: a recipe in Food, a paper with its citation, anything else tagged `shelf:Web`, with a copy of the page |

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

**Bring into a note** is the page-with-an-arrow button in the address bar; on a page with a video
or a sound it offers **Save media…** first. **This page is in your notes**: when a note already names
the page you are on, that button is **filled red**; press it, and the snip's bar says **In 2 of your notes**: press that to
see them, each with the line that names the page, and open one beside the page.

**Search the whole library from the address bar**: type **fm**, a space, and what you are looking
for. Each result says its section (✎ Notes, ♪ Music, ◍ Food, ◎ Web…) and opens as a
tab.

**Exactly where it came from.** A saved page ends with the **full address it was read at**, with the
date and time, and, when the page names a different address to cite (its canonical address, a
paper's abstract page), **that one too**; a paper's citation records the date it was read. Every
piece you bring in ends with a link to the page **at that passage**, and when. The note's **source**
is `web`, with the exact address and the time. Every source formicaria reads from (mail, calendars, files, and later a watch)
describes what it brings the same way.

## Bookmarks

Press **☆** in the address bar, or **Ctrl+D**, as in any Firefox. Firefox keeps its bookmark (so
your Mozilla account still carries it to your other Firefoxes), and formicaria makes a **bookmark
note**: the page's title and link, tagged `shelf:Bookmarks`, without a copy of the page. The panel
that opens says **Also saved in formicaria, as a note**, with **Open the note**. **Remove** there
takes both back. A page that already has its note is not saved twice: the panel says **Already in
formicaria**.

After saving a whole page, **Just the link instead** in the bubble swaps the copy for the same kind
of note. Like every note, a bookmark travels to your other devices with your notebook, and search
finds it. Bookmarks that arrive by your Mozilla account stay Firefox's and do not become notes.

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
  other file becomes a note with the file attached, tagged `shelf:Web`, saying where it came from.
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

## Privacy and media

- The ant's **Privacy** opens formicaria's privacy switches (search engine, trackers, ads, cookie
  pop-ups, tracking links, Global Privacy Control, secure connections, forgetting web data, sites kept
  signed in, forgetting when closing); they
  are Firefox's own protections, set from one place. [Privacy](./privacy.md).
- The ant's **Save media on this page**, or right-click → **Save media in formicaria…**, keeps a
  page's video or sound in your library. [Saving media](./media.md).
- **Save page** keeps the whole page as **one self-contained file**: its pictures, styles and fonts
  inside it, so it opens the same offline (its videos and sounds keep their addresses). Its link in
  the note opens the copy in a new tab, as it was saved, with nothing in it able to run.
