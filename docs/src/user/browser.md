# formicaria's browser

Browse the web inside formicaria and keep what you find as a note, in one press: a recipe, a paper,
an article, a video or a song, with a copy of the page as it was.

On a computer, this is **Linux for now**. The phone gets its own browser built in, in a later
version.

## Set it up

**Settings → Browser → Set up formicaria's browser.** formicaria downloads **Firefox ESR**, the
long-term-support Firefox, **from Mozilla**, and checks it against the checksum Mozilla publishes
before using it. It is about 90 MB to download and 320 MB on your computer. It goes in formicaria's
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
- **uBlock Origin**, from Firefox's add-on site, blocks ads. Turn off **Block ads with uBlock
  Origin** in Settings → Browser if you do not want it.

Your history, cookies and sign-ins stay on this computer, in the browser's profile. They are not
part of your notebook and are never synced.

Other Firefox add-ons work too: install them from Firefox's add-on site as in any Firefox.

## Keep a page

On any page, press the **formicaria button** in the toolbar, or right-click the page and choose
**Keep in formicaria**. formicaria reads what the page says it is and keeps it:

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

## What it does not do

- It does not download videos or music from streaming sites.
- It sends nothing anywhere but your own formicaria: the page is read in your browser and kept on
  your computer.
- Windows and macOS: not yet.
