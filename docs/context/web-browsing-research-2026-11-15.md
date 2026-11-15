# Browsing the web without leaving formicaria — research, 2026-11-15

> **The plan built from this research is [`browser-plan-2026-11-15.md`](./browser-plan-2026-11-15.md).**
> This file keeps the evidence, the alternatives weighed and the sources.

_Research, not a ruling. Nothing here is decided or built. It answers three questions: how
formicaria could get a browsing view that saves what is found into the vault; what other products
do, closely or loosely alike; and whether there is a workable plan, what it costs, and which
standing decisions it would break. Read with `decisions.md#ui`, `#data`, `#seams`, `#vault`,
`papers-plan.md` §3.7 and `music-plan-2026-11-13.md`._

**Evidence labels.** **[code]** read in this repo at `8b77410`. **[doc]** a vendor's own docs or
the project's own page, found by search. **[secondary]** a review, a news article or a forum, found
by search. **[not verified]** from memory or inferred, not checked in this pass. No page was opened
in full; every web claim comes from search results. Check a claim before building on it.

---

## TL;DR

0. **Owner's direction, later the same day: formicaria should have its own browser, built on an
   existing open project, because chains of separate tools ask too many steps of a person (no one
   copies and pastes on a phone).** §10 answers that, §11 says what happens to the system browser, and **§12 is the refined plan** (Gecko on every platform, so popular Firefox add-ons work; phases P0–P5). Together they **replace item 1 below as the direction**.
   **Owner, later still: the browser ships built into the Android app, on by default**, because
   that is where it should have the biggest effect (§12.10). This settles §12.9 Q5 against the
   companion-APK recommendation in §12.7c.
   Items 2–5 still hold: the readers, opt-in and the YouTube line apply to an own browser too.
1. *(Superseded as the direction by §10; kept for the reasoning.)* **Do not build or fork a browser.** On a desktop, formicaria already runs *inside the person's
   own browser* (`decisions.md`, *Browser is the product*, 2026-07-15). The person is already in a
   browser. What is missing is a **bridge** from the page they are on to the vault, not a second
   engine. Embedding an engine (Electron, CEF, Tauri child webviews, Servo) would reverse that ruling
   and turn the project into a browser vendor, with a Chromium security release roughly every four
   weeks to keep up with.
2. **Build a capture layer with three front doors and one set of readers.**
   - **Readers** (pure, in `fm-app`, like `web_recipe.rs` today): page bytes in, a proposed note out.
     There would be one reader per kind of content: recipe, paper, article, video/audio link,
     exercise.
   - **Door 1, link or share**: paste a link, or use Android's share menu, which the app already
     handles. The shell fetches the page once. This is today's `recipe_from_link`, made general.
   - **Door 2, a companion browser extension** (Firefox and Chromium, desktop, plus Firefox on
     Android). It sends the page *as the person sees it* (signed in, after scripts, past the cookie
     banner) to the local server. A side panel puts a slim formicaria beside the page. Zotero,
     Joplin, Trilium, Logseq and Obsidian all work this way.
   - **Door 3, optional and later, an in-app browser on Android only**: a native Android WebView
     screen with a *Save* button, like Paprika's built-in browser. Only if Door 2 does not reach
     enough phone users.
3. **Most of your list is feasible: recipes, papers, articles, fitness pages, Bandcamp purchases,
   notes and annotations.** **Downloading songs from YouTube is the exception.** YouTube's terms
   forbid it, Google Play rejects apps that do it, and a built-in downloader would endanger the
   phone build's distribution. The recommendation is to keep a YouTube *link* (and optionally an
   embed) as a song or exercise note. You would decide whether to go further (§5.3).
4. **It can be opt-in like the assistant**: one Settings switch, *Save from the web*, off by
   default. The extension's key is minted only while it is on, and every capture command refuses
   when it is off.
5. **It breaks four standing assumptions**, each needing a dated `decisions.md` entry before code
   (§6): the loopback auth model (an extension needs a new, command-scoped key, which `papers-plan.md`
   §3.7 dropped); "carry none of those sites' words" (what a saved page snapshot may hold, and
   whether it is synced); the papers *separate app* ruling (where web-paper annotation lives); and
   the CSP's `frame-src` (only if embeds are wanted).

---

## 1. What exists today that this builds on

| Piece | Status | Where |
|---|---|---|
| A page fetched once, on a press, by the **shell**; `fm-app` only parses | shipped for recipes and barcodes | `fm-serve/src/products.rs` → `fm_app::web_recipe` **[code]**; `decisions.md` 2026-10-12 and 2026-10-18 |
| Only public `https`; localhost and private ranges refused; refused to paired devices | shipped | `web_recipe::address`, `REMOTE_DENIED` **[code]** |
| Reads **only what the page publishes for machines** (schema.org `Recipe` JSON-LD), "nothing guessed from the page's look" | shipped | `web_recipe.rs` header **[code]** |
| Android share menu (`SEND`, `SEND_MULTIPLE`) | shipped 0.10.0 | `AndroidManifest.xml` **[code]** |
| Chunked ingest of large files, blobs content-addressed, `Range` serving | shipped | `features.md` *Media / blobs* |
| A paper from a pasted BibTeX entry, DOI, arXiv link or title, **offline**; DOI recognised, never resolved | shipped | `paper.rs`; `decisions.md` *A paper is a note* |
| Song notes filled from file tags; *Add a song* | in progress (the other session) | `music.rs`; `music-plan-2026-11-13.md` |
| Assistant research over **listed** reputable hosts, a guarded fetch, `dom_smoothie` for article text | shipped in Myrme | `myrme/docs/research/2026-10-29-web-research.md`, `-reputable-sources.md` |
| Loopback callers get `Scope::All`; the CSRF guard refuses a foreign `Origin` | shipped | `fm-serve/src/main.rs` `authorize`, `allowed_origin` **[code]** |

**So the hard parts already exist in miniature.** There is a press-to-fetch path, an address guard,
a pure reader, a share entry point, large-file ingest and a sanitising renderer. A general capture
layer extends these rather than adding a subsystem.

---

## 2. How a "dedicated browser" could be built: five options

### O1. A browser inside the app page (iframe), possibly through a proxy: **rejected**

- Most large sites forbid being framed (`X-Frame-Options: DENY/SAMEORIGIN` or CSP
  `frame-ancestors`). The browser will not show them in a cross-origin iframe **[secondary]**.
  YouTube, Google, banks and most publishers set these **[not verified per site]**.
- The workaround is a proxy in `fm-serve` that rewrites and strips those headers. It would serve
  third-party script **from the app's own origin** (`127.0.0.1:8765`), where it could call every
  `/api/*` command with `Scope::All`. That means a full takeover of the vault from any page
  visited. It also breaks sign-ins, turns the server into a general fetcher (against the
  *no HTTP client in the core* line) and runs against sites' terms. **Not viable.**
- The app's own CSP is `frame-src` limited to its own blobs; widening it is a ruling of its own
  (`decisions.md`, *the read view may frame its own blob*).

### O2. A native desktop shell embedding a browser engine: **possible, very expensive, reverses a ruling**

| Engine | State (as found) | Cost |
|---|---|---|
| **Electron** `WebContentsView` | the supported way since `BrowserView` was deprecated in Electron 30; `<webview>` "not recommended" **[doc]**. Obsidian's core *Web viewer* (1.8) uses Electron's webview and is **desktop-only** **[doc]** | a Node/Chromium runtime of 100+ MB **[not verified]**; a second toolchain beside pixi/Rust; Electron's frequent major releases |
| **CEF** via `cef-rs` (Tauri team) | tracks Chromium 146–150 **[doc]**; ships a helper binary and large runtime files **[secondary]** | same Chromium size; security updates are ours to ship |
| **Tauri 2 child webviews** (`add_child`) | behind the `unstable` feature; **"Android / iOS: Not supported"** **[doc]** | on Linux this is **WebKitGTK, the engine that would not paint here** (`decisions.md` 2026-07-15) |
| **Servo** | first crates.io release 0.1.0 on 2026-04-13, "not a 1.0" **[doc]**; the Tauri-based Verso reported stalled **[secondary]** | site compatibility (YouTube, Bandcamp players, sign-ins) unproven **[not verified]** |

**Verdict.** Every route reverses *Browser is the product; the native window is removed*. It brings
back a window, which on this project's Linux desktop never painted. It adds a second desktop
toolchain, and it makes formicaria a browser vendor: Chrome ships a major release about every four
weeks, and each one closes security holes that an embedded copy must take too **[not verified:
cadence]**. That is a permanent tax on a one-person project, paid to recreate something the person
already has open.

### O3. A companion browser extension (WebExtension): **recommended for desktop**

The pattern every comparable notes or reference app uses:

| Product | How the extension reaches the app | Notes |
|---|---|---|
| **Zotero Connector** | desktop app runs an HTTP server on `127.0.0.1:23119`; the extension starts every call **[doc]** | site **translators** (JS) are matched by URL, then `detectWeb()` runs in the page **[doc]**; translators are AGPL **[not verified]** |
| **Joplin Web Clipper** | `localhost:41184`, a **token** copied from the desktop app's settings **[secondary]** | exactly the key model §6.1 proposes |
| **Trilium** | scans a local port range for the desktop app; clips land in a *clipper inbox* note **[doc]** | |
| **Logseq** (third-party) | the desktop HTTP API at `127.0.0.1:12315`, enabled in settings **[secondary]** | |
| **Obsidian Web Clipper** | writes through an `obsidian://` URI; MIT; extraction by **Defuddle**, Markdown by Turndown; **per-site templates** (YouTube, Reddit, recipes…) **[doc]** | the best design reference for templates |
| **Mealie** (recipes) | a bookmarklet sends the URL; extensions such as *Mini Mealie* send the page **HTML** so signed-in or script-built pages work **[doc]/[secondary]** | the same failure the server-side fetch hits: anti-bot pages **[secondary]** |

**Why it fits formicaria better than an engine:**
- **The app adds no new outbound path.** The person's browser has already fetched the page; the
  extension hands the bytes to `127.0.0.1`. `privacy.html`'s list of outbound paths does not grow
  (`decisions.md` 2026-10-16).
- **It sees what the person sees**: signed-in pages, paywalled papers through their institution,
  pages built by script. A server-side fetch gets the anonymous or bot view.
- **Downloads can be caught.** The `downloads` API sees a Bandcamp purchase zip or a paper PDF the
  person chose to download, and can offer *Add to vault* **[not verified: per-browser API detail]**.
- **A side panel** (Chrome `sidePanel`, Firefox `sidebar_action` **[not verified: API names]**) can
  host a narrow formicaria view next to the page: the target note, tags, *Save*. That is as close
  to "never leave the app" as a desktop gets without an engine.
- **Phone:** Firefox for Android supports the general add-on catalogue **[secondary]**, so the same
  extension reaches some phone users. Chrome for Android has no extensions **[not verified]**.

**What it costs:** an MV3 extension (TypeScript; it can share the readers' JS-side types); two store
listings or self-hosted signed builds (a new publishing channel, and so a `#toolchain` decision); and
**the new auth axis §6.1 describes**.

### O4. A native in-app browser on Android: **optional, later**

- Android's **WebView** gives the embedding app the DOM, cookies and `evaluateJavascript`. **Custom
  Tabs** deliberately give none of that **[doc]**. So a *Save* button needs a WebView screen.
- Tauri's child webviews do not run on Android **[doc]**, so this would be a native
  Activity/Fragment injected the way `AgentService` and `MainActivity`'s inset bridge already are.
- **Costs:** Chromium's team says WebView "is not intended as a framework for building browsers" and
  lacks some of a browser's security features **[doc]**. **Google sign-in refuses embedded
  WebViews** (`disallowed_useragent`) **[not verified]**, so signed-in YouTube or Google Scholar will
  not work there. Every page visited is hostile content inside our own process.
- **Paprika** proves the experience is liked for recipes: a built-in browser, a *Download* button on
  the toolbar, preloaded cooking sites, and a clipboard fallback when import fails **[doc]**.

### O5. Fork an open-source browser (Firefox, Chromium, Brave, Zen, Floorp, Min, Nyxt, qutebrowser, Ladybird): **rejected**

Same costs as O2, multiplied: rebasing on upstream for life. Nothing in this use case needs control
of the engine; it needs the page's bytes and a way to send them home.

---

## 3. The capture layer: readers, one per kind of content

One rule, extended from `web_recipe.rs`: **read what the page publishes for machines first, then the
article text, and guess nothing from the page's look.**

| Kind | Machine-readable source on the page | Fallback | Becomes |
|---|---|---|---|
| **Recipe** | schema.org `Recipe` JSON-LD (shipped) | Readability text → `@Myrme /recipe` proposal | a recipe note; ingredient lines matched in Food (shipped) |
| **Paper** | Highwire `citation_*` meta tags (`citation_title`, `citation_doi`, `citation_pdf_url`…), which Google Scholar asks publishers for, so they are near-universal on publisher pages **[doc]**; JSON-LD `ScholarlyArticle`; arXiv and DOI in the URL | a pasted BibTeX entry (shipped) | `create_paper` + the PDF ingested, if the person's browser could fetch it |
| **Article / blog / guide** | `og:*`, JSON-LD `Article` | Readability-style extraction. Myrme chose `dom_smoothie` (MIT) after measuring; Obsidian uses Defuddle; Firefox Reader View and Obsidian's Web viewer use Mozilla Readability **[doc]** | a Markdown note with `source_url`, the author and date, and a credit line |
| **Video** (YouTube, Vimeo) | `VideoObject` JSON-LD, `og:video` | — | a note holding the **link** and the title; optional timestamped notes the person writes |
| **Song / album** (Bandcamp page) | `MusicRecording`/`MusicAlbum` JSON-LD, `og:audio` **[not verified per site]** | — | a song note with `source_url`; the **audio** only from a file the person owns (§5.3) |
| **Exercise** | rarely structured (no adoption figures found for `ExercisePlan`/`HowTo`) | Readability text → an agent proposal, *"how an exercise is done is its source's own words"* | an exercise note with its source named (`decisions.md#data`, 2026-10-12) |

**Annotation, almost for free.** Two open standards fit files-as-truth:
- **Text fragments** (`https://page#:~:text=start,end`) reopen the page in the person's own browser
  with the passage highlighted. Chrome, Edge and Safari support them, and Firefox does since about
  version 131 **[not verified]**. A highlight is then just a link in a note, needing no stored
  anchor and no copy of the page.
- **W3C Web Annotation `TextQuoteSelector`** (exact text + prefix + suffix), which Hypothesis anchors
  on, with fuzzy re-anchoring when the page changes **[secondary]**. It is the robust form for a
  *saved* copy. It is the same idea as the PDF `asset:…#page=&rect=` reference `papers-plan.md`
  already uses.

**Keeping a copy of the page**, as an option per save: the *text* goes into the note, and
optionally a single-file HTML snapshot becomes a blob. **SingleFile** and **ArchiveWeb.page/WACZ**
are the reference tools, and **both are AGPL-3.0** **[doc]**, so they cannot be vendored into an MIT
app without a licence decision. The extension can serialise the DOM itself. A stored HTML blob
already gets `Content-Disposition: attachment` and a `sandbox` CSP (`blob.rs`), so it cannot run
inside the app. Viewing it in-app would need a sandboxed viewer, which is a `frame-src` decision.

---

## 4. Related work, close and distant

**Notes apps with a browser or clipper.** Obsidian added a core **Web viewer** in 1.8 (desktop only,
Electron): it opens external links inside Obsidian, and a *Save to vault* command converts the page
with Mozilla Readability **[doc]**. Community plugins (**Surfing**, Web Browser) do the same, and
Surfing lets browser extensions run in its tabs and exposes the browser to AI agents over MCP
**[doc]**. Notion, Evernote, Joplin, Trilium, Logseq, Anytype, AFFiNE and SiYuan ship clippers. The
pattern across them is a local-port or URI handoff plus templates.

**Read-it-later.** Pocket shut down in July 2025; Omnivore's hosted service closed in November 2024
**[secondary]**. Readwise Reader is the main survivor: it saves articles, PDFs and **YouTube videos
with their transcripts**, with highlights synced across sources **[doc]**. Wallabag (self-hosted) and
Raindrop also remain. The lesson from the two closures is that a hosted reading list can vanish. A
local vault cannot, which is formicaria's pitch.

**Archiving.** **ArchiveBox** is the closest *engine-side* analogue to the whole vision: one URL in,
then SingleFile, Readability, yt-dlp (audio/video with subtitles), gallery-dl and papers-dl
extractors run over it **[doc]**. It shows the full pipeline is buildable, and also that its media
half rests on scrapers that need constant upkeep.

**Reference managers.** Zotero's connector and translators are the most complete page-to-record
system, with per-site translators that are community-maintained and auto-updated from a central
repository **[doc]**. Paperpile and Mendeley follow the same extension pattern **[not verified]**.

**Recipes.** Paprika (built-in browser + *Download*), Mealie and Tandoor (`recipe-scrapers`, a
bookmarklet), and Obsidian's recipe templates. All lean on schema.org `Recipe`, which formicaria
already reads.

**AI browsers.** Dia (from Arc's makers, bought by Atlassian in 2025), ChatGPT Atlas (October 2025,
opt-in "browser memories", an assistant side panel), Perplexity Comet, and the AI features in Edge,
Chrome and Brave **[secondary]**. The warning they carry: **indirect prompt injection** through page
content let researchers make agent browsers leak credentials and act on the user's behalf (Brave on
Comet, August and October 2025; similar results against Atlas). Brave called it "a systemic challenge
facing the entire category" **[secondary]**. Perplexity has since published a detection model
(BrowseSafe) **[doc]**.

**Media.** NewPipe (F-Droid), Seal (a yt-dlp front end), cobalt. They exist, and they are absent from
Google Play for a reason (§5.3).

---

## 5. Feasibility, use case by use case

### 5.1 Recipes: **feasible now, mostly built**
Door 1 already works for any page with JSON-LD. Door 2 adds pages that block a bot or need sign-in,
because the bytes come from the person's browser into the **same** `web_recipe` parser.
*Modify* already exists: a recipe note is the person's own; *Make it mine*; ingredient matching.
The new parts are a reader that takes **supplied HTML** instead of fetching, and a Readability +
`/recipe` fallback for pages without JSON-LD.

### 5.2 Papers: **feasible for metadata and the PDF; the reader is the known open item**
- Metadata from `citation_*` tags is a small, pure reader beside `paper.rs`.
- The PDF: through Door 2 the person's browser downloads it with their own access, and it is
  ingested as today. Through Door 1, `citation_pdf_url` would be a **second fetch**, which is a
  widening of *one press, one page*.
- **Open access look-up** (Unpaywall: `GET api.unpaywall.org/v2/{doi}?email=…`, free, no
  registration **[secondary]**) would be a new outbound address and reverses *a DOI is recognised,
  never resolved*. It is its own decision, under the barcode ruling's terms.
- **Summarise**: `@Myrme` over the extracted text, as a proposal (exists). **Annotate**: `papers-plan.md`
  Phase C, under the owner's *separate app* ruling. A browser does not change that ruling but makes
  it more pressing (§6.3).

### 5.3 Music: **Bandcamp purchases yes; YouTube downloads no (recommendation)**
- **Bandcamp**: every purchased download is **DRM-free**, in MP3, FLAC, AAC, Ogg, ALAC, WAV or AIFF
  **[doc]**. The flow is: buy on Bandcamp → the download (often a zip) → the extension or a drop
  offers *Add to vault* → the existing *Add a song*. The new piece is **zip unpacking at ingest**
  (several tracks plus a cover image). Ripping Bandcamp's *stream* is a different matter and is not
  proposed **[not verified: Bandcamp's terms text was not retrieved]**.
- **YouTube**: the terms forbid downloading except through YouTube's own features **[secondary,
  consistent across sources]**. Google Play does not accept YouTube downloaders **[secondary]**. The
  official captions API is for the video's owner only **[doc]**; transcripts come only through an
  undocumented endpoint **[secondary]**. yt-dlp itself is a maintained tool, but it is a
  cat-and-mouse dependency **[not verified: current runtime requirements]**. Bundling it would:
  (a) put the Android build's Play Protect standing and the F-Droid plan at risk, (b) make the
  project an ongoing scraper maintainer, and (c) bring copyright exposure to a public project.
  **Recommended instead:** a song or video note that **keeps the link** (and, if wanted later, a
  `youtube-nocookie.com` embed, which needs a `frame-src` decision). Audio enters the vault only from
  a file the person already has. If the person runs a downloader of their own, the resulting file
  goes through *Add a song* like any other. **This is your call** (§8 Q2).
- `music-plan-2026-11-13.md` lists *streaming services* as out of scope. This recommendation agrees
  with it.

### 5.4 Fitness: **feasible, agent-assisted**
Exercise pages rarely carry structured data, so this route is Readability text, then a proposed
exercise note (a `/exercise` command shaped like `/recipe`) the person edits. The *source's own
words, named* rulings apply: the note must link and credit its page, and nothing joins the shipped
library this way. Video demonstrations are a **link** (§5.3).

### 5.5 Articles and everything else: **feasible**
A note with the extracted text, `source_url` and a credit line, plus text-fragment highlights. It is
searchable by the existing FTS, synced by git, and summarised by Myrme on request.

### 5.6 The AI side (opt-in, as today)
- The assistant reads **only the extracted text of a page the person chose to save**. It never
  browses with the person's cookies and never acts on a page. That keeps out the Comet/Atlas failure
  class.
- Everything it produces stays a **proposal** the person accepts (existing). That is the main
  defence against injected instructions in a page.
- No written prompt-injection threat analysis exists yet for the assistant. Web capture makes
  that gap larger, so the analysis and hostile-page eval cases should come **before** the assistant
  reads clipped pages.

---

## 6. What it breaks: assumptions and rulings

| # | Standing assumption | What the plan does to it | Verdict |
|---|---|---|---|
| 6.1 | **Loopback = `Scope::All`; a foreign `Origin` is refused** (`authorize`, `allowed_origin` **[code]**). `papers-plan.md` §3.7: an extension "does not work", since there is no command-level scope, and pairing is the LAN path. *The browser extension* was **dropped** for a paste box | An extension needs a **new auth axis**: a key minted in Settings (Joplin's model), sent in a header, **accepted only for a short command allowlist** (`clip_page`, `ingest`, `create_paper`, read-only target-note lookup), refused for everything else, and only while *Save from the web* is on. The extension's `Origin` is accepted **only with** that key | **Contradicts** §3.7's drop, so it needs a dated reversal. It is the main design risk, so it deserves the adversarial review the CLAUDE.md reserves for security |
| 6.2 | **The app carries none of those sites' words**; links, never pictures (recipe ruling, 2026-10-18) | A person's own saved copy is consistent with "a person keeping a copy … as they could by hand". **But** a vault can be shared with collaborators and pushed to a git host, so a saved full page becomes redistribution | **Extends**, with a rule needed: the text goes in the note, a snapshot is an opt-in blob, and the person is told what a shared vault will carry |
| 6.3 | **Papers: a separate app** (owner, 2026-08-29) | A browser front door produces papers and highlights *inside* the notebook | **Tension, not a contradiction**: capture can live in the notebook while the annotating reader stays the separate app. Worth an explicit line |
| 6.4 | **No HTTP client in `fm-app`/`fm-core`; the shells fetch one fixed address per press** | Door 2 adds **no** fetch at all. Door 1 widens from *recipe pages* to *any public https page*, still one page per press | **Extends** the 2026-10-18 exception. A PDF follow-on fetch, Unpaywall or YouTube transcripts would each be a further decision |
| 6.5 | **Browser is the product; no native window** (2026-07-15) | Kept by O3/O1/O5's rejection. O2 would reverse it. O4 adds a native screen on Android only | **Permitted** on the recommended path |
| 6.6 | **CSP `frame-src`**: the app frames only its own blobs | Embeds (YouTube, a saved-page viewer) need widening | **Only if wanted**; a decision of its own |
| 6.7 | **Distribution** (Android Play Protect list, the F-Droid plan, store review) | A YouTube downloader would threaten it. An extension adds store listings | YouTube download **not recommended**; listings are a `#toolchain` entry |
| 6.8 | **Disk / git**: blobs over `git_assets_max` (≤100 MB) do not travel | Albums and PDFs are fine. Video files are large | Already handled; the save sheet should state size, as *Add a song* does |
| 6.9 | **Untrusted bodies are sanitised** (`render.ts`, DOMPurify) | Clipped HTML becomes Markdown and goes through the same sanitiser | **Permitted**, but add a CI check that no clip path writes raw HTML into a body |
| 6.10 | iOS is agent-free and has only a share sheet | A share extension would be native code | Out of scope for now |

**The four questions** (CLAUDE.md), applied to the recommended path. Subjects: `#seams` `#vault`
`#data` `#ui` `#toolchain` `#agent`. Verdict: **contradicts one ruling** (6.1, the dropped extension)
and **extends three** (6.2, 6.4, 6.7). It **widens what ships**: new commands, a Settings switch and
a new distributed artifact (the extension). So it needs `decisions.md` entries first.

---

## 7. A phased plan, and what each phase requires

| Phase | What | Requires | Rough size **[estimate]** |
|---|---|---|---|
| **W0** | Generalise `recipe_from_link` into **`page_from_link`** with a reader registry: recipe (exists), paper (`citation_*`), article (Readability, reusing Myrme's `dom_smoothie` pick), video/audio link. Android share of a URL routes here | a decision widening 6.4; one dependency (`dom_smoothie`, MIT) in `fm-app`, so the four questions | days |
| **W1** | The same readers accept **supplied HTML** (no fetch): a paste-HTML box, and the extension's endpoint later | none beyond W0 | small |
| **W2** | **Companion extension** (Firefox + Chromium; Firefox Android): *Save page*, *Save selection*, side panel, downloads → *Add to vault* (PDFs, Bandcamp zips), text-fragment highlights. **Settings → Save from the web** (off by default) mints the key | the 6.1 reversal with an adversarial security review; zip ingest; store listings (`#toolchain`) | 2–4 weeks |
| **W3** | Agent over clipped notes: summarise (exists), `/exercise`, *adapt this recipe*; all as proposals | the written prompt-injection analysis + hostile-page evals, **first** | in Myrme's cadence |
| **W4** (optional) | **Android in-app browser** screen with *Save*: a native WebView injected like `AgentService` | a `#track-m` entry; a phone test; acceptance that Google sign-in will not work there | 2–3 weeks + device testing |
| **Not planned** | Own engine or fork; proxy iframe; bundled YouTube downloader | — | — |

Opt-in behaviour, end to end: with the switch off, nothing exists. No key, the capture commands
answer *turned off*, and the extension shows *"Turn on Save from the web in formicaria's Settings"*.
Turning it off revokes the key. This mirrors how the assistant and the barcode look-up are each
behind a person's choice.

---

## 8. Open decisions for the owner

1. **Companion, not browser.** Accept that on a desktop the browser stays the person's own, with an
   extension and side panel as the bridge, and a native browser only on Android, later, if needed?
2. **YouTube.** (a) a link only; (b) a link plus a privacy-enhanced embed (`frame-src` decision);
   (c) also accept a downloader the person installs themselves (never shipped). Recommendation: **(b)**.
3. **The extension key** (6.1). Reverse `papers-plan.md` §3.7's drop with a command-scoped key, after
   an adversarial review?
4. **Page snapshots.** Text only, or also an opt-in HTML snapshot blob, and may it travel with git?
5. **Where it lives.** A group of its own (a *Web* or *Inbox* view of saved pages), or just notes
   landing in the existing groups by kind (recipe → Food, paper → Library, song → Music)?
   Recommendation: by kind, with an *Inbox* shelf for unsorted clips.

## 9. Not done in this pass

- No page was opened in full; all web claims are from search summaries (labels above).
- No prototype or measurement: extension API details, Firefox Android behaviour, and the claim that
  Google sign-in is refused in WebViews are unverified.
- Bandcamp's terms of use, Google Play's policy text and F-Droid's anti-feature labels for
  downloaders were not retrieved.
- Licences of Defuddle and Zotero translators were not confirmed.

---

## 10. Our own browser, built on an existing open project (owner's direction, 2026-11-15)

**The ask.** An own browser, not a bridge, because it allows custom actions a bridge cannot: a
*Save* that knows the vault, catching downloads, detecting a recipe or paper on the page, blocking.
Built on an existing open project, keeping maintainability and privacy in view. Android first, since
that is where today's paste-a-link route costs most.

**The key distinction: our own browser does not mean our own engine.** Every small browser keeps
up by putting its own interface over an engine someone else patches: Google (Chromium/WebView),
Mozilla (Gecko) or Apple (WebKit). The engine is the part with the monthly security releases. The
interface (address bar, tabs, *Save*, the side panel) is small, and that is the part we would own.

### 10.1 Android: the candidate bases

| Base | Engine, and who patches it | Licence | What we would get | Costs |
|---|---|---|---|---|
| **GeckoView + Mozilla Android Components** | Gecko (Firefox's engine), patched by Mozilla. GeckoView is a self-contained library pulled from Mozilla's Maven repository; no compiling Gecko **[doc]** | MPL-2.0 **[doc]** (file-level copyleft; fine beside MIT code) | Mozilla describes Android Components as "libraries for building browsers and browser-like applications". Its **Reference Browser** is offered as "a starting point for your own new browser-like applications" **[doc]**. It is the stack under Firefox for Android, Focus, Fennec F-Droid, IronFox and **Tor Browser for Android** **[doc]/[secondary]**. **Firefox add-ons run inside it**: `WebExtensionController.install()` takes an AMO-signed `.xpi`, and `installBuiltIn()` loads our own add-on unsigned **[doc]** (see §12.2), so uBlock Origin, and our own capture logic written once as an extension, could run in our browser *and* in desktop Firefox/Chrome | the engine is bundled, so the APK grows by tens of MB **[not verified: measure]**; a GeckoView version bump each Firefox release (four-weekly) **[not verified: cadence]**, which is a dependency bump, not a rebase |
| **Android System WebView shell** (the DuckDuckGo route) | Chromium, **updated by Google through the Play Store**; zero engine upkeep | DuckDuckGo's Android app: **Apache-2.0**, on GitHub; it ships no engine of its own and uses the system WebView **[doc]/[secondary]** | the smallest APK and the least work: the formicaria app is already a WebView app, so a browser screen is a native view beside it, with the Rust core in the same process. DuckDuckGo's code can be read and borrowed (Apache-2.0 is compatible) | **Google sign-in refuses embedded WebViews** (`disallowed_useragent`, enforced since 2021-09-30) **[doc]**, so no signed-in YouTube or Google Scholar; Chromium says WebView "is not intended as a framework for building browsers" **[doc]**; no extensions |
| **Fork a whole Firefox for Android** (Fennec, IronFox, Tor Browser) | Gecko | MPL-2.0 (+ BSD for Tor's parts **[not verified]**) | everything at once | rebasing a fork of the whole of Firefox, every release, forever. **Mull, the hardened fork IronFox continues, died of exactly this** **[secondary]**. Not recommended |
| **Tor Browser for Android as the base** | Gecko + the Tor network | open | proves the GeckoView/Fenix line can be rebranded and hardened | everything routed through Tor: slow, CAPTCHAs, many sites and payment pages block exit nodes **[not verified per site]**. Better as an **optional private tab** later, via **Arti**, the Tor Project's Rust Tor **[not verified: licence MIT/Apache, maturity]**, which fits a Rust core |

**Privacy, from existing parts:** Brave's **adblock-rust** is MPL-2.0, Rust, parses EasyList and
EasyPrivacy, and handles uBlock Origin-compatible resources **[doc]**. It fits the Rust core and
works under either engine (WebView: `shouldInterceptRequest`; Gecko: as itself, or just ship uBlock
Origin as an extension). No telemetry, no account, and the history stays on the device.

**Recommendation for Android: GeckoView + Android Components, starting from the Reference
Browser's patterns, not a fork of a finished browser.** It is the only base that gives an engine
someone else patches, Google sign-in that is not refused as a WebView **[not verified: test it]**,
extensions, and a lineage (Firefox, Tor Browser, IronFox) that shows it can carry a privacy browser.
The System WebView route is the fallback if the measured APK growth is unacceptable.

### 10.2 Desktop: the candidate bases (second, after Android)

| Base | Notes |
|---|---|
| **Min** (Apache-2.0, Electron) **[doc]** | a small, readable browser to start from; Chromium patched through Electron updates; a second toolchain (Node). **Ruled out by §12's goal**: Electron supports only a *subset* of Chrome's extension APIs and loads only unpacked extensions **[doc]**, so popular add-ons would not work seamlessly |
| **Tauri with the system webviews** (WebView2 / WKWebView / WebKitGTK) | smallest; embedded views are behind `unstable` **[doc]**; WebKitGTK is the engine that would not paint here, and a blank WebKitGTK window is often fixed by a renderer setting **[not verified on this machine]**, so that is worth one test before ruling it out |
| **Firefox with our extension preinstalled** | Firefox's enterprise policies can install an extension at first run **[not verified]**. This is *our browser* in all but the build, with zero engine work. A good first desktop step |
| qutebrowser (GPL, QtWebEngine) **[doc]** | GPL; fine as a separate program, but its code cannot move into the MIT app |
| **Ladybird** (BSD-2, own engine), **Servo** (0.1.0) | alpha not yet shipped as of September 2026 **[secondary]** / not 1.0 **[doc]**. Watch, do not build on yet |

### 10.3 How it changes the plan in §7

- **One capture codebase, three places.** The readers stay in `fm-app` (Rust). The *page side*
  (detect a recipe or paper, serialise the DOM, highlight, catch a download) is written once as a
  WebExtension. It is built into the Android browser (GeckoView) and installable in desktop
  Firefox/Chrome (§2 O3). The browser calls the Rust core in-process, so **the extension-key problem
  (6.1) disappears on Android**: there is no network hop to authorise.
- **Revised order:** W0–W1 as before (readers) → **B1: Android browser spike**: a GeckoView screen
  inside the existing APK with an address bar and *Save*; **measure** the APK growth and test Google
  sign-in and YouTube playback → **B2**: tabs, downloads → vault, adblock, the built-in capture
  extension, the side sheet showing the target note → **B3**: desktop (Firefox with the extension
  first, an own window later) → W3 agent work.
- **Still true under an own browser:** opt-in (*Save from the web* also turns the browser on), the
  assistant reads only what the person saves, and YouTube audio is not downloaded (the browser can
  play YouTube and save its link and notes).
- **Decision entries this needs** (the owner knows the rulings; listed so none is missed): a
  `#track-m` entry for a second engine in the APK and its size; `#toolchain` for Mozilla's Maven
  repository and the release cadence; `#ui` for a Browser place in the app; `#vault` for the desktop
  extension key if B3 uses it.

---

## 11. With our own browser, is the system browser still needed to run formicaria?

**Android: it never was.** The app is its own window (Tauri over Android System WebView), not a
page in the phone's browser app. With our browser added, the APK holds **two engines**: the
formicaria screens stay on System WebView (Tauri's Android runtime, wry, is built on it and cannot be
pointed at Gecko **[not verified: no Gecko backend found]**), and the browser screen runs on
GeckoView. The person sees one app. What it costs is size, and memory beside the assistant's model
(§12.6).

**Desktop: today yes, afterwards no.** `fm-serve` starts and opens the *default* browser on
`127.0.0.1:8765` (`FM_OPEN`; *Browser is the product*, 2026-07-15). Once formicaria has its own
desktop browser, the launcher opens formicaria **in it**, as a pinned home tab with the sidebar, and
the system browser is optional. `fm-serve` stays the backend unchanged; only the window changes.
Three conditions:
- **Pages and add-ons must not reach the app.** Web pages in the same browser must not drive
  `127.0.0.1:8765`, and an add-on granted "all sites" must not read or script the formicaria tab
  (§12.5).
- **The fallback stays.** Opening the address in any ordinary browser keeps working, for anyone who
  does not take our browser.
- **Other devices are unchanged.** A paired tablet still opens formicaria in its own browser over the
  share listener.

**iPhone:** unchanged; one app on Apple's WebKit, no separate browser planned.

---

## 12. The refined plan: a formicaria browser where a person's usual add-ons just work

_No code. Research and plan only, written after the owner's note: the point is to let people bring
the options they use every day, so popular Firefox add-ons should work seamlessly._

### 12.1 The decision this plan rests on: Gecko on every platform that gets our browser

Making add-ons work means picking **one add-on ecosystem** and an engine that runs it fully:

| Engine route | Add-on support | Verdict |
|---|---|---|
| **Gecko**: GeckoView on Android, Firefox on desktop | Android: installs **AMO-signed** add-ons, minus a few desktop-only APIs (§12.3) **[doc]**. Desktop: everything AMO has | **Chosen.** One catalogue (addons.mozilla.org) and one capture add-on of ours, everywhere |
| Android System WebView | none | no add-ons at all |
| Electron (Min) / CEF | a subset of Chrome APIs, unpacked only **[doc]** | most popular add-ons break |
| A Chromium fork (as Brave or Vivaldi are) | full Chrome Web Store | a whole-browser fork, rebased every four weeks; ruled out in §10 |

**Gecko cannot be embedded on desktop** (GeckoView is Android-only; Mozilla's desktop embedding was
retired long ago **[not verified]**). So on desktop, "our browser" means **a dedicated, configured
Firefox installation**, not a new program (§12.4).

### 12.2 What we build, and what we reuse

| Part | Reuse | Ours |
|---|---|---|
| Engine | GeckoView (Android), Firefox (desktop), both patched by Mozilla | nothing |
| Browser shell on Android | **Mozilla Android Components**: toolbar, tabs, sessions, downloads, an add-ons manager (`feature-addons`, what Firefox for Android uses **[not verified: reuse by a third party]**); the Reference Browser as the pattern **[doc]** | the formicaria look; a *Save* button; the opt-in switch |
| Browser shell on desktop | Firefox's own interface | policies, a profile, and our add-on's toolbar button and **sidebar** (desktop Firefox has `sidebar_action`) |
| **The formicaria add-on** (one codebase, MV3) | — | the page side: detect a recipe/paper/video/song on the page and show a *Save* badge, serialise the page or a selection, highlights (text fragments), catch downloads (PDF, Bandcamp zip) and offer *Add to vault*. Android: loaded with `installBuiltIn()`, unsigned **[doc]**. Desktop: AMO-signed (unlisted signing) and force-installed by policy **[doc]** |
| Page → note | `dom_smoothie` (Myrme's measured pick) | the readers in `fm-app` (Rust), §3 |
| Talking to the vault | — | Android: the add-on → **GeckoView's privileged messaging to the embedder** (open to built-in add-ons; third-party native messaging is not) **[doc]** → the Rust core in-process; no network hop, no key. Desktop: the add-on → `fm-serve` with a key minted while *Save from the web* is on (6.1) |
| Ad and tracker blocking | **uBlock Origin from AMO**, offered on first run, no own engine needed. `adblock-rust` (MPL-2.0) is kept as the fallback if a built-in blocker is wanted **[doc]** | the first-run offer |

### 12.3 Which add-ons will work: what is known, and what to measure

**Android limits** (Mozilla's own list of differences) **[doc]**: no `sidebar_action` (shown in a tab
instead), no third-party **native messaging** (so anything needing a companion desktop program does
not work there), no `managed` storage, no developer-tools add-ons. Unsigned add-ons cannot be
installed persistently **[secondary]**. That is a security feature, and ours is exempt because it is
built in.

**Desktop limits:** none beyond Firefox's own.

The add-ons people most often ask for, to be checked one by one in the P0 spike (*works* = installs
from AMO in our GeckoView build and does its job on three sites):

| Add-on | Why people use it | Known for Android? |
|---|---|---|
| uBlock Origin (and uBlock Origin Lite) | ads, trackers | **yes**: AMO marks it *Recommended* and *Available on Firefox for Android*, ~10 M users **[doc]**. GPLv3, so **offered from AMO at first run, not bundled** (bundling = we redistribute GPL code). P0: confirm GeckoView keeps AMO add-ons updated |
| Privacy Badger · Ghostery · AdGuard · ClearURLs | privacy | **on AMO's Android listings** **[doc]** |
| Bitwarden / Proton Pass | passwords | check |
| Dark Reader | dark pages | check |
| SponsorBlock · Return YouTube Dislike | YouTube | check |
| Firefox Relay | email masks | on AMO's Android listings **[doc]** |
| Proton VPN / Mullvad-style proxies | VPN | check (proxy API on Android) |
| LanguageTool · a translator | writing, reading | check |
| **Zotero Connector** | papers | desktop: yes; Android: check. **See §12.5** |
| Video DownloadHelper and other downloaders | media | needs a native companion → **not on Android** **[doc: no native messaging]**; and §5.3 |

### 12.4 Desktop: a dedicated formicaria Firefox, three steps up

1. **D1: a formicaria profile in the person's Firefox.** The launcher starts their Firefox with a
   dedicated profile; our add-on is installed into it. Zero shipping. But Firefox becomes a
   prerequisite, and enterprise policies are per *installation*, not per profile **[doc]**, so we
   could not lock down the formicaria tab without touching their main Firefox.
2. **D2 (recommended target): our own Firefox installation.** The app already downloads, verifies
   and keeps runtimes in `<config>/formicaria/tools` (the assistant's runtime). It would do the same
   for an **official Firefox (or ESR) build**, with a `distribution/policies.json` in it:
   - `ExtensionSettings` force-installs our add-on and pre-approves the add-ons the person picks
     **[doc]**;
   - the home page is the formicaria tab;
   - add-ons are kept off formicaria's own address (§12.5).

   Firefox's own updater keeps it patched **[not verified: for an unpacked install on each OS]**.
   macOS needs `EnterprisePoliciesEnabled` **[doc]**. **To check before building:** Mozilla's
   trademark terms for redistributing an *unmodified* Firefox with a policies file **[not verified]**.
   If they forbid it, use a rebranded Gecko build that others already publish (LibreWolf-style)
   rather than building Firefox ourselves.
3. **D3, only if needed: our own branded Gecko build.** Building Firefox from source in CI is heavy:
   hours and tens of GB per platform **[not verified]**, against a disk that already fills (see the
   project memory). Not planned.

### 12.5 Bridges to tools people already use

- **Zotero Connector → formicaria.** The connector talks to a desktop server on `127.0.0.1:23119`
  with documented endpoints (`/connector/ping`, `/connector/saveItems`, `/connector/saveSnapshot`,
  …) **[doc]**. If formicaria answered that protocol, the **real Zotero Connector**, installed in
  our browser, would save papers into the vault using Zotero's whole library of site translators.
  We would maintain none of them and copy no AGPL code (we implement a protocol, the add-on runs its
  own code) **[not legal advice]**. Risks: the published docs may lag the code (a December 2025
  forum post found an endpoint gone) **[secondary]**; the port collides if Zotero itself runs. It is
  worth a spike because it brings the best paper capture there is for free.
- The same idea could later cover other clippers whose local protocol is open (Joplin's
  `127.0.0.1:41184` with a token **[secondary]**), so a person's existing habits keep working.

### 12.6 Security and resources: what must be true

- **Add-ons must not reach formicaria.** On Android this holds by construction: the formicaria
  screens are a separate engine (System WebView) with no HTTP server; the phone talks over IPC.
  On desktop the formicaria tab is a page at `127.0.0.1:8765`, which an "all sites" add-on can
  match. Firefox's `ExtensionSettings` policy can block hosts from add-ons
  (`runtime_blocked_hosts`) **[not verified: name and behaviour]**, which would cover it. Otherwise
  the formicaria window opens in a separate container or profile. **This is the one hard
  requirement for D2.**
- **Pages must not drive the app.** The CSRF guard already refuses foreign origins **[code]**. Keep
  it, and keep capture on the add-on's privileged channel, never on an endpoint a page can call.
- **The assistant reads only what is saved** and only proposes (§5.6); the prompt-injection
  analysis comes first.
- **Phone memory.** GeckoView runs beside the app's WebView *and*, when it is on, the assistant's
  model in the same process (`device-resources.md`). Measure peak RAM with both before shipping;
  the browser may need to stop the model while it is open.
- **Distribution.** GeckoView comes as a prebuilt library from Mozilla's Maven repository. F-Droid's
  main repository builds everything from source **[not verified: policy detail]**, which is why
  Gecko browsers such as IronFox ship through their own F-Droid repository or other stores
  **[secondary]**. This could change the F-Droid route in `fdroid-distribution-plan-2026-10-30.md`;
  IzzyOnDroid, which ships the developer's own APK, is the likely route **[not verified]**.
- **Signing our add-on for desktop** needs an AMO developer account and API keys in the release
  pipeline: a new release secret.

### 12.7 Phases (no code until P0 has answered)

| Phase | Goal | Done when | Stop if |
|---|---|---|---|
| **P0: spikes, measured** | answer the unknowns above | a written results section here: APK growth (in-app vs a companion APK); RAM with the model (`dumpsys meminfo`); engine cold start against the smoothness bars; Google sign-in, YouTube, Bandcamp in GeckoView; the §12.3 add-on table filled; built-in add-on ↔ Rust messaging round trip; D2 on Linux, Windows, macOS with `policies.json`; the blocked-host rule; Zotero `ping` + `saveItems` answered by a stub | APK or RAM unacceptable on the target phones, or sign-in blocked → fall back to the System WebView browser (no add-ons) and say so |
| **P1: readers** | page bytes → a proposed note, for recipe, paper, article, video/song link, exercise (§3) | each reader tested on saved pages from real sites (pages kept out of the repo) | — |
| **P2: Android browser** | a *Browser* place in the app: tabs, *Save*, downloads → vault, our built-in add-on, **an add-ons screen installing from AMO**, uBlock Origin offered | used for a week on a phone without falling back to Firefox | — |
| **P3: desktop browser (D2)** | the launcher opens formicaria in its own Firefox, with the sidebar, add-ons and *Save* | the default browser is no longer needed; the fallback still works | the trademark check fails → a rebranded Gecko build |
| **P4: bridges** | the Zotero Connector protocol | a paper saved by the real connector lands as a paper note with its PDF | protocol churn too high |
| **P5: assistant** | summarise, adapt a recipe, `/exercise`, over saved pages, as proposals | hostile-page evals pass | — |
| **Later** | a Tor private tab (Arti); D3; iOS | on request | — |

**Opt-in throughout:** *Save from the web* in Settings turns on the browser, the add-on's key and
the capture commands together; off, none of them exist.

### 12.7b DuckDuckGo's anti-tracking features, and how a Gecko browser gets each

The owner asked whether our browser could have DuckDuckGo's protections. Most come **built into
Gecko** (Firefox's own protections, which GeckoView exposes to an embedder **[not verified: each
setting's API]**). A few come from add-ons, two are small features of our own, and one is not a
browser feature at all.

| DuckDuckGo feature | In our browser | How |
|---|---|---|
| Tracker blocking in pages | **yes** | Firefox's **Enhanced Tracking Protection** (Standard/Strict/Custom on Android **[doc]**) + uBlock Origin. **Not** DuckDuckGo's own list: Tracker Radar's data is under a *NonCommercial* Creative Commons licence (sources disagree on SA vs ND) **[secondary]**, which an MIT project should not ship |
| Cookies kept per site | **yes** | **Total Cookie Protection**; on Android it comes with Strict **[doc]** |
| Fingerprinting defence, bounce tracking | **yes** | ETP Strict **[secondary]** |
| Tracking parameters stripped from links | **yes** | Firefox's query stripping in Strict **[secondary, one source]**; ClearURLs add-on is on Android **[doc]** |
| HTTPS upgrade (*Smarter Encryption*) | **yes** | Firefox's HTTPS-Only / HTTPS-First mode **[not verified in GeckoView]** |
| Global Privacy Control signal | **yes** | a Firefox setting **[not verified]** |
| Cookie pop-ups handled | **likely** | DuckDuckGo's open `autoconsent` library or the Consent-O-Matic add-on **[not verified: licences, Android]** |
| **Fire Button** (burn tabs and data in one press) | **ours, small** | GeckoView can clear site data and run private sessions **[not verified: API names]**; one button |
| **Duck Player** (YouTube without personalised ads) | **ours, small** | open a video through YouTube's privacy-enhanced embed; same as §5.3's recommendation |
| Email Protection (`@duck.com` aliases) | **via add-on** | a service, not a browser feature; Firefox Relay is on Android **[doc]**, or DuckDuckGo's own |
| **App Tracking Protection** (trackers in *other* apps) | **no** | a device-wide **local VPN**, not part of a browser **[secondary]**. Android allows one VPN at a time and Play restricts VPN apps **[not verified]**. Point people to DuckDuckGo's app or TrackerControl (open, on F-Droid **[secondary]**) |

**Default proposed for P2:** ETP Strict, HTTPS-Only, GPC on, a Fire button, uBlock Origin offered.
Each is a setting the person can change, and each is checked in the P0 spike.

### 12.7c Size, efficiency, smoothness, maintainability: the whole plan weighed

_Measured here where a number is ours; otherwise labelled. P0 replaces every estimate with a
measurement._

**Size.**

| Item | Today | With the plan | Basis |
|---|---|---|---|
| Android APK (arm64) | **~28 MB** | **~75–85 MB** if GeckoView is inside the app | ours: `fdroid-distribution-plan-2026-10-30.md`; the whole of Firefox for Android (engine + Android Components + UI) lists at **~56 MB** for recent arm64 builds **[secondary, listings vary]**, so the engine and components alone are an estimate |
| Desktop download | `fm-serve` ~3 MB + UI | + a Firefox installation for those who opt in | Firefox's size not checked in this pass **[not verified]** |
| Our own code | ~2,000 lines of Kotlin in the shell today **[code]** | + the Android browser screen (thin: Android Components does tabs, downloads, add-ons), + one MV3 add-on, + Rust readers | estimate in P0 |

> **Overruled by the owner (§12.10):** the browser ships inside the main APK, on by default. The
> reasoning below is kept for the costs it names, which §12.10 now has to absorb.

**This changes one recommendation: ship the Android browser as a separate, opt-in companion app.**
Tripling the APK for everyone, so that the people who turn the browser on can have it, runs against
how the assistant is shipped (weights come on first enable). A **formicaria Browser** APK, installed
by the app's existing updater when *Save from the web* is turned on:
- keeps the main app at ~28 MB;
- runs its engine in its own process, so it never shares a heap with the assistant's model;
- talks to formicaria through an Android permission with `protectionLevel="signature"`, which only an
  app signed with the same key can hold **[not verified: wiring]**. That is secure with no tokens, and
  still in-process for the Rust core on formicaria's side.

The cost: two APKs to sign, list and update. The in-app variant stays the alternative if P0 measures
the growth well under the estimate.

**Efficiency (CPU, battery, memory).**
- Nothing new runs in the background. The readers run once per *Save*, in Rust. The page side runs
  only on the page in front. No crawling, no polling.
- **Memory** is the real budget. Firefox for Android has been reported at roughly 100 MB for one simple
  page, and 45–70% above Chrome during video, where the gap is mostly GPU driver memory **[secondary;
  a Mozilla bug report]**. Beside the assistant's **~1.5 GB peak** on the test phone (`device-resources.md`)
  (`device-resources.md`), that fits there. On a 4 GB phone Android would likely kill one of them
  **[not verified]**. A separate APK lets Android reclaim the browser on its own. P0 measures
  `dumpsys meminfo` with both running.
- Saved pages stay small: text into the note by default; snapshots are opt-in blobs; video is never
  stored (link only).

**Smoothness.**
- One `GeckoRuntime` per process, created when the browser first opens and kept while it lives
  **[doc: a singleton]**. Cold start of the engine is the visible cost; P0 measures it against
  `gui-smoothness-plan-2026-10-18.md`'s bars.
- *Save* answers at once (a sheet with the proposed note) while the reader runs. A slow reader never
  blocks scrolling.
- Moving between formicaria and the browser is an app switch (companion APK) or a screen change
  (in-app). Both are native transitions. The formicaria WebView is never re-created for it.
- Desktop: Firefox *is* the window, so it is as smooth as Firefox; formicaria's pinned tab is a
  normal tab.

**Maintainability: what recurs, and who carries it.**

| Recurring work | Cadence | Carried by | Our part |
|---|---|---|---|
| Engine security fixes (Android) | each Firefox release, about every four weeks, plus point fixes **[not verified: cadence]** | Mozilla writes them | **a GeckoView + Android Components version bump and a release**. GeckoView keeps a deprecated API **3 releases** before removing it **[doc]**, so a bump rarely needs code changes, but the security fix only reaches people when we release. Automate the bump (a bot PR + `pixi run ci`); the agent-driven release flow already ships without waiting |
| Engine security fixes (desktop) | same | Mozilla, **including delivery**: Firefox updates itself | none, if D2 uses Firefox's updater. **Firefox ESR** (one feature change a year, security fixes in between) is the low-churn option for the policy file **[not verified: ESR cadence]** |
| Add-on store signing | each release of our add-on | AMO, automated (`web-ext sign`) | API keys in the release pipeline |
| Site changes breaking a reader | when a site changes | — | low by design: readers read schema.org / `citation_*` / Readability, not a site's layout |
| Zotero protocol drift | rare, unannounced **[secondary]** | — | a contract test against the real connector |
| Tracking protection lists | continuous | Mozilla (built in), uBlock's authors (add-on) | none |

**Compared with the alternatives**, on the same four axes:

| Route | Size | Memory | Upkeep | Add-ons |
|---|---|---|---|---|
| Our own Gecko browser (this plan) | +~50 MB, opt-in APK | moderate–high | a monthly bump + release (Android); none (desktop) | yes |
| System WebView browser (DuckDuckGo-style) | +~1 MB | low (shared with the system) | **none for the engine**: Google updates it | **no** |
| Electron/Min desktop | +~100 MB **[not verified]** | high | Electron bumps | partial |
| Fork of a whole browser | large | as the base | **a rebase every release**: what killed Mull **[secondary]** | yes |

**Verdict.** The plan is sound on all four axes **if** (a) the Android browser is a separate opt-in
APK, (b) engine bumps are automated and shipped promptly, and (c) desktop leans on Firefox's own
updater. The **System WebView browser** is the fallback that wins on size, memory and upkeep, and
loses only add-ons. If P0 shows the Gecko costs are too high on the target phones, take that route
and keep the built-in protections (§12.7b) that do not need Gecko: HTTPS upgrade, the Fire button,
adblock-rust.

### 12.8 Decision entries this needs (a checklist)

`#track-m` (a second engine in the APK; its size and RAM) · `#toolchain` (Mozilla's Maven repository;
Firefox downloaded as a runtime; AMO signing in the release pipeline; the F-Droid route) · `#ui` (a
*Browser* place; desktop opening in our Firefox instead of the default browser) · `#vault` (the
desktop add-on key; add-ons kept off formicaria's address) · `#data` (readers widening what may be
read from a page; snapshots) · `#agent` (the assistant over saved pages).

### 12.9 Questions for the owner

1. **Gecko everywhere** (one add-on ecosystem: Firefox's), accepting that Chrome-only add-ons are out?
2. **Desktop D2**: formicaria downloads and keeps its own Firefox, as it does the assistant's
   runtime?
3. **The Zotero Connector bridge**: worth a P0 spike?
4. **Phone resources**: if P0 shows the browser and the assistant cannot run together, which one
   gives way while the browser is open?
5. ~~**Android packaging**~~ **Answered (§12.10): inside the main APK, on by default.** Was: the browser as a **separate opt-in companion APK** (main app stays ~28 MB;
   recommended, §12.7c), or inside the main APK (~75–85 MB for everyone)?

---

## 12.10 Owner's decision, 2026-11-15: the browser is built into the Android app, on by default

> *"I would like to ship the built in browser as a default on android, where I suspect it will have
> the biggest effect."*

**What it means.** GeckoView and the browser screen are part of the main APK. The *Browser* place is
in the app from install, with nothing to turn on. *Save* works out of the box. **Still opt-in:** the
assistant reading saved pages (as the assistant itself is), page snapshots, and any add-on beyond
the first-run offer. Desktop (P3) and the Zotero bridge (P4) are unchanged. A Settings switch can
**hide** the browser for people who do not want it; the engine is then never started, so it costs
disk but no memory.

**What changes in the plan.**
- **§12.7c's companion APK is dropped.** The IPC between two apps goes too: *Save* calls the Rust
  core in-process.
- **P2 becomes the headline phase**, and P0 decides how, not whether. P0's stop condition becomes
  "fall back to System WebView *inside the main app*" (no add-ons), not "do not ship".
- **The opt-in switch in §7 and §12.7 now governs only the desktop key and the assistant.**

**The costs this now carries for every Android user, and how each is contained.**

| Cost | Size of it | Containment |
|---|---|---|
| **APK size** | ~28 MB → **~65–85 MB** (estimate: Focus grew 4 → 38 MB with GeckoView; Lite build, §13; P0 measures) | arm64 only, as today; no bundled add-ons (uBlock is offered from AMO, not shipped); measure what GeckoView's omni.ja and native libs actually add, and strip what the app does not use **[not verified: what is strippable]** |
| **Every update is a bigger download** | the full APK each release **[not verified: whether Android's installer path here does delta updates]** | releases already batch; state the size in the update offer, as the assistant does for weights |
| **Memory** | the Gecko parent lives in the app's process beside the Tauri WebView and, when on, the model (~1.5 GB peak) | **create the `GeckoRuntime` only when the browser is first opened**, never at app start; pages render in Gecko's own content processes **[not verified on Android]**; P0 measures `dumpsys meminfo` with model + browser + a video page on the test phone and a 4 GB-class device or emulator profile |
| **Cold start of the app** | none if the runtime is lazy | a CI check that app start does not touch GeckoView (the startup contract `pixi run android-smoke` already exists) |
| **Security releases become the whole app's** | a Gecko security fix means a formicaria release, about monthly **[not verified: cadence]** | automated version-bump PR → `pixi run ci` → the existing agent-driven release; GeckoView's 3-release deprecation window **[doc]** keeps most bumps code-free |
| **F-Droid** | the whole app, not just a companion, now carries a prebuilt GeckoView, which F-Droid's main repository likely refuses **[not verified]** | revisit `fdroid-distribution-plan-2026-10-30.md`: IzzyOnDroid (ships the developer's APK) or the project's own F-Droid repository, as IronFox does **[secondary]**; or build GeckoView from source in the F-Droid recipe, as Fennec F-Droid does (very heavy) |
| **Play Protect / store review** | a browser is a normal app category; no new permission is needed for browsing itself **[not verified]** | no downloader for YouTube (§5.3) keeps it clean |

**Decision entries to write when `decisions.md` is free** (the music session has it open): `#track-m`
(GeckoView in the main APK, on by default, lazy runtime; APK size accepted), `#toolchain` (Mozilla's
Maven repository, the monthly bump, the F-Droid route), `#ui` (a *Browser* place from install).

---

## 13. The libraries, vetted: track record, maintenance, and the case against each

_Asked by the owner: have these been used before, are they actively maintained, and what do other
apps and users say against them? Search summaries only; labels as above._

| Library / dependency | Used in the past by | Maintenance signal | The case against (contrastive) | Verdict |
|---|---|---|---|---|
| **GeckoView** (engine) | **Mozilla's own Firefox for Android and Focus** (Focus moved to it and grew **4 → 38 MB**) **[doc]**; Tor Browser for Android (rebased to ESR 153 in July 2026) **[secondary]**; Waterfox Android (Gecko 153, July 2026) **[secondary]**; IronFox; Fennec F-Droid; Wolvic | ships with every Firefox release; apilint-checked API; a 3-release deprecation window **[doc]** | **Wolvic (Igalia, the most serious outside embedder) moved to Chromium in 2024**: Gecko's GPU-process integration was weak and Mozilla "no longer prioritises" features embedders need (WebXR there) **[secondary]**. Users report Firefox for Android feeling slower than Chrome on some sites, with open Mozilla bugs on it **[secondary]**, and higher memory during video **[secondary: Mozilla bug]**. Mozilla **dropped desktop embedding before** (XULRunner, 2015; "frozen" embedding APIs removed without notice, 2009) **[secondary]** | **Use, with an exit.** The deciding fact is that Mozilla's own Android browser *is* a GeckoView app, so it cannot be dropped without dropping Firefox for Android. Wolvic's complaints were about XR/GPU, not page browsing. Keep the engine behind a thin interface of ours (§13.1) |
| **GeckoView "Lite"** build | the variant Mozilla intends for third-party apps; Waterfox switched to it **[doc]/[secondary]** | built alongside the full one since Firefox 92 **[doc]** | none found; omits telemetry and crash-reporting libraries, with no intended content differences **[doc]** | **Use Lite**: fewer Mozilla services inside our app, which is a privacy plus |
| **Mozilla Android Components** (tabs, toolbar, downloads, add-ons UI) | Firefox, Focus, Reference Browser, Iceraven, Tor Browser **[doc]/[secondary]** | moved into the Firefox monorepo in 2024; artifacts still published to `maven.mozilla.org` each release **[doc]** | built first for Mozilla's own apps; some docs are stale after the monorepo move **[doc]**; volunteer forks such as Iceraven warn of "no guarantees of security or updates" **[secondary]**. Waterfox stopped using the prebuilt components and compiles everything itself **[secondary]** | **Use selectively**: take the components the plan needs (session, toolbar, downloads, add-ons), not the whole Fenix app. Pin to the same version as GeckoView |
| **Firefox desktop + enterprise policies** (D2) | thousands of organisations; Mozilla's own policy docs use uBlock Origin as the example **[doc]** | policies documented and versioned by Mozilla **[doc]** | macOS needs `EnterprisePoliciesEnabled`; a registry key can override the policy on Windows **[doc/secondary]**; the trademark question is open (§12.4) | **Use** |
| **Mozilla as an organisation** | — | — | **~85% of its income comes from Google's search deal**. The September 2025 antitrust remedy lets Google keep paying, non-exclusively; the current deal ends late 2026; the liability finding was under appeal **[secondary]** | **A real, external risk** for every Gecko choice. A one-person project cannot hedge it except by keeping the engine swappable (§13.1) |
| **uBlock Origin** (offered, not bundled) | ~10 M Firefox users; *Recommended* on AMO **[doc]** | actively maintained by its creator **[secondary]** | **essentially one maintainer** (a long-standing community concern) **[secondary]**; a 2024 clash with Mozilla's automated review over uBO Lite **[secondary]**. Firefox has said repeatedly, most recently in **August 2026**, that it keeps Manifest V2 and blocking `webRequest`, so uBO keeps working there, but gave **no end date** **[secondary]** | **Offer it**; do not depend on it. The built-in protections (§12.7b) stand on their own if it ever stalls |
| **adblock-rust** (Brave) | Brave; **Firefox ships it inside the engine since 149, off by default**, behind `privacy.trackingprotection.content.protection.enabled` **[secondary]**; Waterfox; Ladybird adopting it **[secondary]** | Brave rewrote it in January 2026 for **75% less memory** **[doc]**; many releases **[secondary]** | experimental inside Firefox: no UI, no lists bundled **[secondary]**; **not confirmed active in GeckoView on Android** | **Strong candidate for a built-in blocker with no add-on**: if P0 finds it in GeckoView, switching it on with EasyList/EasyPrivacy costs no new dependency. Otherwise link the crate (MPL-2.0) into the Rust core |
| **dom_smoothie** (article text) | Myrme already uses it, after measuring it against rivals (F1 0.865) | versions to 0.18 on docs.rs **[doc]**; release cadence not confirmed | one maintainer **[not verified]**; a port of Readability.js, so it inherits that method's limits | **Keep** (already a dependency in Myrme). P1's eval can compare rs-trafilatura on our own saved pages, as Myrme's doc proposed |
| **DuckDuckGo `autoconsent`** (cookie pop-ups) | DuckDuckGo's browsers **[doc]** | MPL-2.0; 17 authors, ~400 commits, frequent releases (14.84.0 seen) **[secondary]** | one company's project; rules need constant upkeep as sites change (inherent) | **Use** if cookie handling is wanted; as a built-in content script it updates with our releases |
| **Zotero Connector protocol** (bridge) | Zotero desktop only; **no third-party app found that answers it** **[secondary: none found for JabRef, Juris-M, Paperlib]** | published docs, "Zotero 6.0 and newer" **[doc]** | docs lag the code (an endpoint removed, December 2025) **[secondary]**; the port can be taken by other programs **[secondary]**; being first means nobody has shown it works | **Spike only** (P0); no commitment until a real save works end to end |
| **Android System WebView** (the fallback engine) | DuckDuckGo (Apache-2.0, open), countless apps **[doc]** | updated by Google through the Play Store | not meant as a browser framework; Google sign-in refused **[doc]**; no add-ons | **Keep as the fallback** behind the same interface |

### 13.1 What the vetting changes in the plan

1. **An engine seam from the start.** The browser screen talks to *our* small `BrowserEngine`
   interface (load, navigate, run our page script, receive *Save*, downloads), with a Gecko
   implementation and a System WebView one. Wolvic shows engines do get swapped (it kept both). Our
   add-on is written to the standard WebExtension API, so it also runs on a Chromium-based engine if
   ever needed. Cost: a little more code now; it buys an exit from both Mozilla risks above.
2. **GeckoView Lite, not the full build.**
3. **Blocking: try the engine's own adblock-rust first** (no add-on, no new dependency), uBlock
   Origin as the person's optional choice, our own adblock-rust link only if neither works.
4. **P0 gains three checks:** (a) adblock-rust active in GeckoView Lite on Android; (b) scrolling and
   page-load feel against System WebView on the same phone and the same five sites (the
   Firefox-vs-Chrome complaints, measured on our device rather than taken from forums); (c) the size
   GeckoView Lite really adds (Focus's 4 → 38 MB suggests ~35 MB, so ~28 → ~65 MB, below §12.10's
   estimate **[estimate]**).
5. **The Zotero bridge drops from a phase to a spike**: no one else has built it.

---

## 14. P0 results

### P0.9: desktop Firefox ESR, Linux (2026-11-15) **[measured]**

Run outside the repo, headless, with two unsigned probe add-ons ("ours", and "other" standing in for
a third-party add-on with all-sites access), a stand-in formicaria at `127.0.0.1:8799` and a
stand-in web page at `127.0.0.2:8798`.

| Question | Result |
|---|---|
| Latest ESR, resolved via `download.mozilla.org/?product=firefox-esr-latest-ssl&os=linux64` | **153.7.0esr**; `.tar.xz` **86 MB**, unpacked **311 MB** |
| Verified | SHA-512 matched `archive.mozilla.org/…/153.7.0esr/SHA512SUMS` (a `.asc` signature is published beside it) |
| An **unsigned** add-on force-installed by `policies.json` (`ExtensionSettings`, `file://` URL) with `xpinstall.signatures.required=false` in the profile's `user.js` | **installed and ran** (ESR honours the pref). AMO signing is not needed for R1 |
| The add-on's key through managed storage (`3rdparty.Extensions.<id>`) | **ours read `{"key":…}`; the other add-on got "Managed storage manifest not found"** |
| `runtime_blocked_hosts` on `*` for `127.0.0.1`/`localhost`, `runtime_allowed_hosts` for ours | the other add-on's **content script did not run** on the formicaria page; ours did |
| …does it stop an add-on's **background** fetch to formicaria? | **No**: the other add-on's background GET read the stand-in's data. Host blocking covers page access, not an add-on's own requests |
| What `Origin` such requests carry | **POST: `moz-extension://<uuid>`** for both add-ons; GET: sometimes `moz-extension://…`, sometimes none |

**What follows for the design.**
- Every formicaria command is a POST, and `fm-serve`'s CSRF guard already **refuses a foreign
  Origin** **[code]**, so another add-on cannot call commands. Our add-on is let through by a narrow
  rule: a `moz-extension://` Origin **and** the key, **and** only the capture routes.
- **Residual:** a GET with no Origin is accepted on loopback (`/api/blob/<sha256>`), so an add-on
  that already knows a content address could read that blob. The address is unguessable, and the
  same is true of any add-on in any browser on this machine today. Recorded in `known-issues.md`,
  not new to this feature.
- The ESR route needs no Mozilla account, no AMO listing and no redistribution: the app downloads
  Firefox from Mozilla and verifies it.

### R1 end to end: the built feature on Linux (2026-11-15) **[measured]**

A throwaway vault and settings folder, `fm-serve` on its own port, Settings' own route doing the
setup, and the real add-on in the real ESR, headless, with a test-only copy that presses *Keep* by
itself when a tab finishes loading (never shipped).

| Step | Result |
|---|---|
| `browser_setup` | resolved **153.7.0esr**, downloaded 86 MB with progress, SHA-512 matched, unpacked; `key` and `policies.json` written `0600` |
| The key gate, forged requests | `moz-extension` + key → `clip_save` **200**; wrong key, no key, another route (`delete`, `recent`), or a web page carrying the key → **403**; the app's own origin unchanged |
| A recipe page (a recipe site) | first run: kept as an **article**: the copy had lost its JSON-LD with the scripts. Fixed (JSON-LD kept); then a recipe note with *Ingredients* (5 lines), *Method*, 4 servings |
| An arXiv abstract page | a **paper** note: title, eight authors, year, abstract, from `citation_*`. First run took Readability's "[Submitted on …]" line as an author; fixed (a byline is an article's only) |
| A Wikipedia article | an **article** note with its text as Markdown; first run credited "Wikimedia Foundation, Inc." (Readability's guess); fixed (the page's own site name, else its host) |
| A YouTube video page | a **video** note with the link; first run carried *another video's* description from the page's loose meta tags; fixed (only the item's own structured description is used) |
| Reading the page | first run, some tabs answered *"Receiving end does not exist"*: the background messaged the page reader before it listened. Fixed: the background injects the reader and calls it directly |

Every kept note ended with a link to its copy, stored as an asset.
