# formicaria's browser

Browse the web inside formicaria and keep what you find as a note, in one press: a recipe, a paper,
an article, a video or a song, with a copy of the page as it was.

On a computer, this is **Linux for now**. The phone gets its own browser built in, in a later
version.

## Set it up

**Settings → Browser → Set up formicaria's browser.** formicaria downloads **Firefox ESR**, Mozilla's
long-term-support browser engine, **from Mozilla**, checks it against **Mozilla's signature**, and makes
it formicaria's: its name, its icon and its look, with formicaria always the first tab. It is not a
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

## Keep a page

On any page, press the **formicaria button** in the toolbar (or **Alt+Shift+K**), or right-click
the page and choose **Keep in formicaria**. formicaria reads what the page says it is and keeps it:

| The page is | It becomes |
|---|---|
| a **recipe** (the page publishes its recipe for search engines, as most recipe sites do) | a recipe note with its *Ingredients*, *Method* and *Times*, in **Food → Recipes**, its ingredient lines ready to match to foods |
| a **paper** (the page carries its citation, as journal and arXiv pages do) | a paper note with its authors, year, DOI and journal, like one made from a pasted BibTeX entry, with its abstract |
| a **video** (YouTube, Vimeo and others) | a note with the link and the title, on the **Web** shelf; the video itself is not downloaded |
| a **song or album page** (Bandcamp and others) | a note with the link, on the **Web** shelf; music you have bought is added with **Add a song** |
| anything else | an **article**: its text, read the way Firefox's reader view reads it, on the **Web** shelf |

**Every kept page ends with a link to a copy of the page as it was when you kept it**, stored in
your vault like any attachment. Large copies stay off git under the vault's size limit for
attachments, as every attachment does.

If you **select text** before keeping, it goes at the top of the note as a quote, with a link that
reopens the page with that passage highlighted.

Then press **Open in formicaria** to go to the note. It is yours: edit it, tag it, move it.

## formicaria is the first tab

- **formicaria is the pinned first tab** of the window, always there. The **formicaria button** at the
  left of the toolbar, or **Alt+Shift+F**, brings it forward from any page.
- **A page beside a note**: right-click a page's tab and put it in **split view** with the formicaria
  tab, so you read on one side and write on the other.
- **Alt+Shift+K** keeps the page you are on, like the toolbar button. (Change either in the add-ons'
  *Manage Extension Shortcuts*.)
- The window takes **formicaria's colours**, dark or light as your system is, and shows **formicaria**
  in the dock and Alt+Tab.

## Bookmarks

Right-click a page → **Bookmark in formicaria**: a note with the page's title and link, on the
**Bookmarks** shelf, without a copy of the page. Like every note, it travels to your other devices with
your notebook.

## Updates

formicaria keeps its browser up to date by itself. Mozilla's security releases are checked once a day,
fetched, checked against Mozilla's signature and prepared in the background; the new copy goes in the
next time the browser starts. If a new copy does not start, the previous one stays.

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
