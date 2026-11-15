# Bringing the world into the workspace: the web and every other data source (research, 2026-11-15)

> **Research, not a ruling.** Nothing here is decided; §9 lists what the owner decides. No code was
> changed for it. It extends `browser-plan-2026-11-15.md` §14/§16 and `workspace-design-2026-11-15.md`,
> and it reframes the browser work as one case of a general idea.

## 0. The question, in the owner's words

- *"Does it make sense to have a new tab per each note and view one wants to open? Maybe that requires
  more RAM? … we can have the fm part resizable and can open anything like now, but then the tabs are
  to browse the internet, and not the notes. However, we would need to clearly link the web with the
  notes, through simple commands … easy to access and select and to easily connect with a note."*
- *"formicaria already runs in a browser, without Firefox. The browser comes for one and only one
  reason: better integration."*, sharpened to: *"bring the web to the workspace, which already has
  local vault search."*
- *"This is very similar in structure to how I plan to bring health data from Gadgetbridge … The idea
  is the same: make it easy to connect with data sources. The web is a data source, same as health
  info from a smartwatch. Some other day we might have a different source."*
- *"We can have dedicated layers and a thin general structured one. This helps define the infra
  connecting the app and the environment … it also helps agents like Myrme have structured
  interfaces."*

So there are three layers to design, and this document takes them in that order. **The test for each
proposal: does it bring more of the world into the vault, linked and searchable, for its cost?**
1. **The web's own part** (§1–§4): where formicaria sits in its browser window, and the commands that
   link a page to a note.
2. **The general source layer** (§5–§6): one thin contract every source goes through, with a
   dedicated layer per source (the web, Gadgetbridge, mail, calendars, files, whatever comes next).
3. **The agent's view** (§7): the same layer, offered to Myrme as structured, permissioned commands.

## 1. Measured: what a formicaria tab costs

**Method.** Firefox ESR 153.7.0, unmodified (the gate's baseline copy, so the kit's one-tab rule did
not interfere). It ran headless against a debug `fm-serve` serving an **invented vault of 2,000
notes** (40–400 words each, two tags each). Each case was started fresh, measured after 35 s (memory
as the sum of PSS over all Firefox processes), then for 60 s idle (CPU), twice, averaged. Every
formicaria tab showed the app's opening screen. Script: a scratch copy built on
`ci/browser-check.py`'s helpers; not in the repo.

| Case | Memory (PSS) | Processes | Idle CPU |
|---|---:|---:|---:|
| 1 formicaria tab | 687 MB | 12 | 10 % |
| 3 formicaria tabs | 755 MB | 14 | 10 % |
| 5 formicaria tabs | 830 MB | 15 | 10 % |
| 1 formicaria tab + 4 web pages | 952 MB | 18 | 15 % |
| 5 formicaria tabs + 4 web pages | 994 MB | 21 | 22 % |

The four pages were Wikipedia (two articles), arXiv and a recipe site.

**Reading it.**
- **Each extra formicaria tab costs about 35 MB** on its own (687 → 830 MB for four more). With four
  web pages open the increment measured smaller (952 → 994 MB), within run-to-run noise. **A web page
  costs about 65 MB** (687 → 952 MB for four).
- **The cost is modest because the app's tabs share processes**: they are one site, so Firefox puts
  them in the same content processes (12 → 15 processes for four more tabs, not +4).
- **But this is the floor.** Each copy of the app has its own state, its own timers and its own
  polling of the server. Each copy held only the opening screen; a board, a big note or the Music
  deck in each would cost more. A copy per note also means a copy per note **to keep in sync** (an
  edit in one tab, stale in another), which the app's single-instance design has never had to do.
- **Idle CPU** rose with the web pages (their scripts and ads), not with formicaria's tabs; with
  two runs a case, the 15 → 22 % step is not separable from noise.

**Verdict.** RAM alone does not forbid a tab per note (a person with ten notes open would pay about
350 MB). The stronger arguments against it are **correctness and responsiveness**: one app holding
one state, and one place where the person looks for their notes. `browser-plan` §16.1's ruling
("formicaria runs once, not once per tab") stands on those grounds, and these numbers put a size on
it.

## 2. Verified in ESR 153: Firefox's split view

Checked in the gate's copy of formicaria's browser through Marionette (chrome context), 2026-11-15:
- **Split view exists** (`gBrowser.addTabSplitView([a, b])` makes a `tab-split-view-wrapper`), and
  there is a **draggable divider** (`split-view-splitter`).
- **A pinned tab cannot join a split view**: `addTabSplitView([pinned, other])` makes nothing, and
  the tab stays pinned. Mozilla's help says the same [secondary, via search].
- **A pair is a lasting object**: switching to a third tab and back leaves both tabs in their
  wrapper. Whether both panes are drawn again on return was not checked by eye (headless).
  Bug 2003991 implies that selecting either member shows both [secondary].
- **Consequence for formicaria today**: 0.33's formicaria tab is **pinned**, so "a page beside a note"
  through split view, as the manual and plan describe it, **does not work as built**. Either the
  formicaria tab stops being pinned, or "beside" is made another way (§3).

## 3. The web's part: where formicaria sits in its window

| Option | What the person sees | Copies of the app | Verdict |
|---|---|---|---|
| **A. Now (0.33):** formicaria a pinned tab | formicaria *or* a page, switching between them; Alt+Shift+F | 1 | works, but no page beside a note (pinned tabs cannot split) |
| **B. A tab per note and view** | notes and pages mixed in the tab list | 1 per open note | rejected (§1): more memory, state duplicated, two lists of "what's open" (the app's panes and the tabs) |
| **C. A docked formicaria panel (recommended)** | formicaria in a **resizable panel** at the left or right of the window, beside **whichever page is shown**; tabs are only web pages; the panel can be widened to fill the window (today's look) or folded away | 1 | the owner's proposal; one copy, always beside the page, the app unchanged inside it |
| D. formicaria an ordinary (unpinned) tab in a split | formicaria beside *one* page; switching to another page leaves the pair | 1 | a stopgap: the pair binds one page, so reading three pages means re-pairing |

**How C would be built (to verify in a spike).**
- The kit's startup script adds **one content `<browser>` element** to the window, beside the tab
  panels, loading formicaria's address, with a splitter. This is how **Floorp's panel sidebar** and
  Vivaldi's web panels work [secondary]. It is a top-level browsing context, so formicaria keeps its
  normal storage. That avoids 0.32's sidebar problem, a framed page whose storage was partitioned.
- **One copy across windows**: Firefox can move a live page between windows (the same "swap" that
  moves a tab to a new window), so the one panel could follow the window in front instead of each
  window loading its own copy. **Unverified**; the fallback is a panel in the first window only.
- **Never unloaded** (`browser-plan` §16.1 point 2) and loaded first at start.
- **Folded, maximised, resized**: a shortcut (Alt+Shift+F, today's) toggles it; a double-click on
  the divider widens it to the whole window, which is today's full formicaria, so no feature becomes
  harder to reach (§16.1 point 7).
- **Risks**: it is privileged code reaching into the window layout, the part most likely to move
  between ESR majors (Floorp moved its scripted UI into its own build for that reason [secondary]).
  Keep it to one element and one splitter, and have `browser-check` test it like the one-tab rule.
- **The phone** has no room for a side panel: there, formicaria and the page are two screens with
  one swipe or one button between them, and the linking commands of §4 are the same.

**If C fails its spike**, D (unpinned formicaria tab in a split, re-paired by a shortcut "beside this
page") is the fallback, and A remains as is.

## 4. Linking a page and a note: the commands

Patterns taken from tools that do this (sources in §10). Each one costs memory only while used.

**From a page to the vault**

| Command | What it does | Patterned on |
|---|---|---|
| **Keep page** (exists, Alt+Shift+K) | the page becomes a note: a clip, a recipe, a paper | MarkDownload, Zotero |
| **Add to the open note** (new; a shortcut and the context menu on a selection) | appends the selection, or the page's link, to the note open in the formicaria panel, with a quote anchor (the quoted text and some text around it) so the passage can be found again | Obsidian Web Clipper's "append to note", Hypothesis anchors |
| **Add to a note…** (new) | a small picker (recent notes first, type to search the vault) when the open note is not the target; also "today's note" | Obsidian destinations |
| **"This page is in your notes"** (new) | the add-on asks `fm-serve` only when a tab is shown or finishes loading, by normalised address. The toolbar button shows a count. Pressing it lists the notes and the passage around each link, and opens one in the panel | Promnesia, Firefox's Tab Notes, Hypothesis's badge |
| **Search the vault from the address bar** (new) | a keyword (`fm …`) in the address bar searches the vault through the extension omnibox API; Enter opens the note in the panel | Firefox's omnibox API |

**From a note to the web**
- **A link in a note opens as a tab next to the panel**, so the note stays in view (C), and the tab
  remembers which note opened it (a "back to the note" button).
- **A project's tabs are a tab group.** Opening several links from a note can put them in a Firefox
  tab group named after the note. *Keep this group* writes its pages into the note as a list, so the
  session can be reopened later.

**Address matching** decides whether "this page is in your notes" is believed. Match on the canonical
link, the DOI and a normalised address (no tracking parameters, no `www.`/mobile host differences, no
trailing slash), in that order (Promnesia's and Hypothesis's lessons).

## 5. The general source layer

Every source formicaria has, or plans, already does the same five things in its own way:

| Source | Status | Its identity today | Raw kept? |
|---|---|---|---|
| Gmail | shipped, read-only by token scope | `gmail_thread` | no: the conversation note is the copy |
| Calendars | shipped | the event's id, `source` label | no |
| Web pages (clips, recipes, papers) | shipped (0.31–0.33) | the address; a paper's DOI | **yes**: each clip keeps its snapshot |
| Files (ingest, imports) | shipped | content hash (blobs) | yes |
| Smartwatch via Gadgetbridge | planned (`training-coach-plan-2026-10-30.md`) | FIT files | to decide |
| *the next one* | — | — | — |

**The contract every source would meet.** Each element below is found again and again in the
systems surveyed; the examples are in brackets.
1. **Raw first, notes derived from it.** Keep the original (the FIT file, the page snapshot, the
   message) as a blob in the vault, and derive the note from it; a fixed reader then re-derives,
   without anything lost. Each reader has a **version**, and a higher version re-derives (HPI's
   export/DAL split; Perkeep's importer version; Gadgetbridge's own docs: *"most of the understanding
   of the activity data are actually assumptions"*).
2. **A stable identity and an upsert.** Every item is `(source, its own id)` plus a version or a
   change time. A second arrival updates the note instead of adding a copy. Deletions come by that id
   alone, so it is stored (Health Connect `clientRecordId`/`clientRecordVersion`; HealthKit's sync
   identifier; Airbyte's primary key; Readwise).
3. **One provenance block on every item**, in the note's properties: the source, the device or tool
   and its version, **how it was recorded** (sensed / typed / derived), when it was taken, and a
   pointer back to the original (Health Connect metadata; HealthKit's source revision; Open mHealth's
   modality; Promnesia's locator).
4. **A cursor per source and stream, with a way back**: where the last read stopped, kept by the
   source; when it is lost or expired, re-read a window and let (2) remove the duplicates (Airbyte's
   per-stream state; Health Connect's 30-day change tokens).
5. **A self-description**: what the source needs to be set up, a check that it works, and the list
   of kinds of item it produces with their fields. This **catalog** is what Settings shows, what
   permissions are granted on, and what Myrme reads (Airbyte's spec/check/discover; Singer's
   catalog; Health Connect's data types).
6. **A thin shared shape for search and linking**: every item, whatever its source, is findable by
   key, kind, title, time, text and link. The rich per-source fields stay in the note
   (dogsheep-beta's search index; Promnesia's `Visit`). formicaria has this already: **a note**, and
   the vault's search.
7. **Failures stay local**: a broken source reports its error and the rest go on (HPI's
   `@import_source`; Home Assistant's "unavailable" and "sign in again" states).
8. **Who owns a figure** when two sources give the same one (watch steps vs phone steps): one source
   is named its owner, and the person can change it (Exist.io's attribute ownership).

**Pitfalls the surveyed projects report.**
- **Connector rot**: every adapter is upkeep; Singer taps and HPI's sources decayed when unattended.
  Prefer stable file formats (FIT, schema.org) over private APIs, and keep the list of adapters
  short.
- **Normalising at capture** loses data for good (HPI). The raw copy prevents that.
- **Appending into notes** duplicates and goes stale (Readwise → Logseq). A generated part of a note
  needs its identity and a marked region that a re-sync may rewrite, never the person's own text.
- **Echo loops**: writing to a hub and reading your own writes back (Health Connect filters by
  origin).
- **Device-shaped schemas** leaking into the general layer (Gadgetbridge's per-device tables):
  normalise to quantities (heart rate, sleep minutes), not to device models.

**What formicaria already has of this:** the note as the shared shape (6), the vault's search, the
blob store for raw copies (1), and one dispatcher that the UI, the CLI and the phone share. What it
lacks is the **contract written down once**: today each source has its own id key, its own
properties and its own sync loop. Writing it down is mostly a naming exercise over shipped code,
plus the catalog (5).

## 6. The dedicated layer for the web: adapters

How the web's own layer would sit on top of the general one:
- **A short, ordered list of adapters**, each with a match (an address pattern or a schema.org
  type), a cheap test, and an extraction, **lowest priority number first**. The generic chain runs
  last: schema.org JSON-LD, then citation metadata, then OpenGraph, then the readable text
  (Readability + Turndown, today's clip). Modelled on Zotero's translators.
- **An adapter corrects the generic result** rather than starting over: the generic layer extracts,
  the adapter fixes the fields it gets wrong (Zotero's `itemDone` pattern). A site that changes
  then falls back to the generic result, not to an error.
- **Only the address and the title are required**; every other field is optional, with a list of
  places to try (yt-dlp's rule). A partial clip beats a refusal.
- **Adapters as data, not code**: rules (selectors, field names) bundled in the add-on, not scripts
  fetched from anywhere (ftr-site-config; Obsidian's triggers). Easy to review, nothing remote runs.
- **Tests**: saved pages for repeatable tests; a live check of each adapter's example page, run by
  hand, as a warning signal and never a gate (live pages throw consent walls and A/B layouts).
- **Add an adapter only when the generic layer is shown to fail** on real pages, so the list stays
  short (Zotero has about 700 translators, yt-dlp more than 1,800 extractors, and both pay for it).

Today's recipe and paper readers are already two such adapters; the contract names what they share.

## 7. The agent's view: the same layer, structured

What the surveyed systems (MCP, Apple's App Intents, Android's AppFunctions and Health Connect, Home
Assistant's LLM API) agree on, applied to Myrme:
1. **One address per item, the same for everyone**: the UI, the CLI, the phone and Myrme name an item
   the same way (an `fm://source/kind/id` form, or the note id). An agent's answer points at items
   rather than copying them (Apple's one entity for Spotlight, Shortcuts and Siri; MCP's
   `resource_link`).
2. **The catalog of §5 is the agent's menu**: kinds of item with their fields, and a **small set of
   commands per task**, not every command at once. One Zotero MCP server grew to 62 tools (about 23k
   tokens of context) and had to be cut down [secondary].
3. **Reads apart from writes, labelled by the server**: each command says whether it reads, writes,
   deletes, or reaches the network, with the worst case as the default. The dispatcher enforces it;
   the model's claims count for nothing (Home Assistant's tool labels; MCP: such labels are
   untrusted).
4. **A switch per source and direction, off for the agent by default**: "Myrme may read health",
   "may write notes", and a separate grant for history or bulk reads. A new source starts
   unexposed (Health Connect's per-type, per-direction permissions; Home Assistant's opt-in exposure,
   and its users' complaint that new entities were exposed by default).
5. **What a source brings in is untrusted text.** A clipped page, a mail or a PDF can carry
   instructions aimed at an agent. A turn that has read such content does not write or reach the
   network without the person confirming (Simon Willison's "lethal trifecta": private data,
   untrusted content and a way out; a local model with the vault and clipped pages already has two).
6. **An audit trail** of what the agent read and wrote, inputs and outputs (MCP's tools guidance).
7. **MCP later, as a thin translation** of the dispatcher, served only on loopback with a token, not
   a second interface (Home Assistant serves its LLM API as MCP; Android calls AppFunctions "the
   mobile equivalent of MCP tools").

**Avoided on purpose**: capturing the screen after the fact (Microsoft Recall, Screenpipe). It
produces text with no structure, no provenance and no consent per source; Recall's plaintext store
drew the sharpest criticism of the survey. formicaria's structured capture is the opposite design.

## 8. What this changes in the browser plan

- `browser-plan` §14.3's "*Beside* = the formicaria tab and a web tab in split view" **does not work
  as built** (pinned tabs cannot split, §2). §3's option C replaces it, pending a spike.
- §14.4's *shell mode* (the app hides its own chrome when the window's frame replaces it) becomes
  smaller: in C the window keeps Firefox's tab list for pages, and formicaria keeps its own
  navigation inside the panel.
- The add-on's role grows from "keep a page" to **the web's source adapter**: adapters (§6), the
  linking commands (§4), and the "this page is in your notes" lookup. Its background script stays
  idle between events.

## 9. For the owner to decide

> **Status (2026-11-15, late night):** the owner said *"plan and execute"*. Built: 1 (option C, the
> docked panel), 2 (the contract, applied to the web, and the `sources` catalog), and of 5: *Add to
> the open note*, *This page is in your notes* and `fm` in the address bar. Not yet: 3 (Gadgetbridge)
> and 4 (Myrme's switches). `decisions.md` 2026-11-15 (late night).

1. **The window**: C (a docked, resizable formicaria panel; tabs are web pages only), with D as the
   fallback. A spike would check one copy across windows, never unloaded, and `browser-check` tests.
2. **The general contract** (§5): write it down as a decision, and name the existing sources' fields
   to match it (identity, provenance, raw copy, reader version). It would be a `decisions.md` entry,
   `#data` `#seams`.
3. **Gadgetbridge as the second source built on the contract**, with FIT files kept raw in the vault.
4. **Myrme's access**: per-source switches off by default; a read-only first stage; writes only with
   the person confirming.
5. **Which of §4's commands come first.** Suggested: *Add to the open note*, then *This page is in
   your notes*, then the address-bar search.

## 10. Sources

Labels: **[primary]** the doc or code was read; **[secondary]** a summary, press or forum.

**Measured or checked here**: §1 (memory) and §2 (split view), in this repository's browser-check
copy of Firefox ESR 153.7.0.

**Source connectors**
- HPI exports and layering: https://beepb00p.xyz/exports.html; module design:
  https://github.com/karlicoss/HPI/blob/master/doc/MODULE_DESIGN.org [primary]
- Promnesia record model: https://github.com/karlicoss/promnesia (`src/promnesia/common.py`,
  `doc/SOURCES.org`) [primary]
- Dogsheep / dogsheep-beta: https://github.com/dogsheep/dogsheep-beta [primary]
- Perkeep importers: https://perkeep.org/pkg/importer/ [primary]
- Exist.io attribute ownership: https://developer.exist.io/reference/attribute_ownership/ [primary]
- Health Connect metadata and sync:
  https://developer.android.com/health-and-fitness/guides/health-connect/develop/metadata,
  https://developer.android.com/health-and-fitness/guides/health-connect/develop/sync-data [primary]
- HealthKit sync identifier:
  https://developer.apple.com/documentation/healthkit/hkmetadatakeysyncidentifier [primary]
- Gadgetbridge data management: https://gadgetbridge.org/internals/development/data-management
  [primary]; Health Connect sync and auto-export pages [secondary]
- Airbyte protocol: https://docs.airbyte.com/platform/understanding-airbyte/airbyte-protocol
  [primary]; Singer spec: https://hub.meltano.com/singer/spec/ [primary]; tap upkeep complaints:
  https://hub.meltano.com/tap-target-maintenance [secondary]
- Home Assistant data fetching:
  https://contributors.home-assistant.io/core/integrations/fetching-data/ [primary]
- Readwise API: https://readwise.io/api_deets [primary]; Logseq export duplicates:
  https://docs.readwise.io/readwise/docs/exporting-highlights/logseq [secondary]

**The web's adapters and linking**
- Zotero translators: https://www.zotero.org/support/dev/translators/coding [primary],
  https://www.zotero.org/support/dev/translators [secondary]
- yt-dlp extractor rules: https://github.com/yt-dlp/yt-dlp/blob/master/CONTRIBUTING.md [primary]
- recipe-scrapers: https://pypi.org/project/recipe-scrapers/ [secondary]
- ftr-site-config: https://github.com/fivefilters/ftr-site-config [primary]
- Obsidian Web Clipper templates: https://obsidian.md/help/web-clipper/templates [primary]
- Zen's Glance: https://docs.zen-browser.app/user-manual/glance [secondary]
- Floorp's panel sidebar: https://docs.floorp.app/docs/features/panel-sidebar [secondary]
- Vivaldi panels and memory: https://forum.vivaldi.net/post/810119 [secondary]
- Firefox 149 split view and Tab Notes:
  https://www.firefox.com/en-US/firefox/149.0/releasenotes/,
  https://bugzilla.mozilla.org/show_bug.cgi?id=2003991 [secondary]

**Agents**
- MCP resources, tools, security:
  https://modelcontextprotocol.io/specification/2025-06-18/server/resources,
  https://modelcontextprotocol.io/specification/2025-06-18/server/tools,
  https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices [primary]
- The lethal trifecta: https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/ [secondary]
- Zotero MCP's tool count: https://github.com/54yyyu/zotero-mcp [secondary]
- Apple App Intents / IndexedEntity:
  https://developer.apple.com/documentation/appintents/indexedentity [primary]
- Android AppFunctions: https://developer.android.com/ai/appfunctions [primary]; Health Connect
  data types and permissions:
  https://developer.android.com/health-and-fitness/guides/health-connect/plan/data-types [primary]
- Home Assistant LLM API: https://contributors.home-assistant.io/core/internals/llm-api/ [primary];
  exposure complaints:
  https://community.home-assistant.io/t/wth-are-all-new-entities-exposed-to-assist-by-default/803889
  [secondary]
- Playwright MCP: https://github.com/microsoft/playwright-mcp [primary]; WebMCP draft:
  https://webmachinelearning.github.io/webmcp/ [secondary]
- Microsoft Recall criticism:
  https://www.techrepublic.com/article/news-signal-blocks-windows-recall-privacy/ [secondary]

## 11. "Bring this in": one action, many ways to start it (plan, 2026-11-15, not built)

> **The owner:** *"select parts of a content in the web tab and seamlessly bring it into the note … if
> it is an image we bring the image, if it is a link a link, if text the text, if audio and video then
> audio and video … the selection gets the right content, then the user should be able to bring the
> actual content through assets or just a link for the heavy ones … more than one action done in
> different ways doing the same could help."*

### 11.1 What exists (searched 2026-11-15)

- **Firefox's screenshot tool** picks the element under the pointer and climbs to a sensible ancestor
  (`overlayHelpers.mjs`; by-element selection since Firefox 106). Its known weakness is choosing the
  wrong block; users asked for an "Expand to parent" control [primary: source; secondary: Mozilla
  Connect].
- **Obsidian Web Clipper's highlighter** selects text, images and elements and clips only those; its
  images stay web links until a separate "download attachments" command [primary: help pages].
- **Notesnook** clicks blocks one by one; **Obsidian's Select to Note** clicks an element or drags a
  rectangle and appends to a note [secondary].
- **Edge's Web Select** (now in Web Capture) kept a table a table when pasted [secondary].
- **Evernote** clips only a selection made beforehand; **Trilium** and **Memos** save an image from
  the right-click menu; **video and audio are barely handled anywhere** [secondary].

None is aware of the note open beside the page, keeps media in the person's own store by choice,
*and* records the source of each piece. formicaria has the three: the panel's open note, the vault,
the source contract (§5).

### 11.2 The pipeline (one, whatever started it)

1. **Resolve what was pointed at** into a *piece* of a known kind: text (with its formatting), image,
   link, table, code, formula, audio, video, embedded post, or a structured item (recipe, paper). The
   richer the starting point, the better the guess (11.3).
2. **Choose its form: the content or a link.**
   - Text, tables, code, formulas: always the content (small).
   - Images: **the image, kept in the vault**, up to a size the person sets; above it, ask.
   - Audio and video **files**: **a link by default**, with *Keep the file (N MB)* one press away.
   - **Streams** (most video sites): a link at the current moment, the title, the thumbnail, and the
     transcript when the page offers captions; there is no single file to keep. **Protected media**:
     the link only; its protection is not circumvented.
   - It lands at once by these defaults; the message that follows offers the one alternative
     (*Link only instead* / *Keep a copy instead*), and the defaults per kind are in Settings →
     Browser (11.3, rule 2).
3. **Fetch** what is kept: the add-on downloads it with the page's own access (so an image behind a
   sign-in works) and hands the bytes to the server's existing `ingest` route; the vault stores it as
   an attachment. The app never fetches from other sites.
4. **Deliver** to the target: the note open in the panel (at the cursor, else the end); with none
   open, a new note named after the page, or a picked note. Through the bridge and the note's own
   editor: **one undo** for the whole bring-in.
5. **Say where it came from**, per piece: a link that reopens the page at that passage (a text
   fragment, as clips already make), and the date. The page's "in your notes" count then includes it.

### 11.3 The person's view: two things they want, two verbs

> **The owner:** *"plan better for usability and clarity from the user point of view, minimal and
> intuitive actions … we need also to make it work on the phone."*

Every action a person takes on a page is one of two intents. Everything else (clip, keep, bookmark,
add to note, download into the vault) is merged into them:

| Intent | Verb (same word everywhere) | What happens |
|---|---|---|
| *"I want this piece in what I'm writing"* | **Bring in** | what is selected or pointed at goes into **the note in front**, named on every button (*Bring into "Reading"*) |
| *"I want this page for later"* | **Save page** | the page becomes **its own note**: a recipe in Food, a paper in Papers, anything else on the Web shelf; *link only* is the old bookmark |

Rules that keep it clear:
1. **The destination is always named** before the press: the right-click item, the drop outline and
   the chip say *into "Reading"*. With no note in front, **Bring in** makes a new note named after
   the page and opens it, so the person can keep writing.
2. **No questions when a good default exists; corrections after, not dialogs before.** What comes
   in follows the defaults of 11.2 at once, and a short message in formicaria says what happened with
   the one alternative: *Brought an image into "Reading" · **Link only instead** · Undo*. A question
   comes first only when it costs something real: a file above the size limit, or a large download on
   the phone's mobile data.
3. **Feedback where the eyes are**: the chosen part of the page flashes briefly, the note scrolls to
   what arrived, and the message sits in formicaria (not a system notification).
4. **The page's own icon is about the page; the selection's tools are about the selection.** The ant
   in the address bar (where the bookmark star is, and read the same way) is **Save page**; it is
   filled, with a count, when notes already name the page, and pressing it lists them. Showing or
   hiding formicaria stays on its toolbar button and the panel's own header. **Bring in** lives where
   the selection is.
5. **One undo** for every bring-in; nothing a person did not point at is ever written.
6. **Learnable in one go**: the first time a person selects text in formicaria's browser, one hint:
   *"Press the ant to bring this into your note."* Shown once.

### 11.4 The same two verbs, on each device

| | Computer (formicaria's browser) | Phone (formicaria's own browser, GeckoView) | Phone (any other app) |
|---|---|---|---|
| **Bring in a selection** | the **ant chip** beside the selection, or right-click → *Bring into "Reading"*, or drag it onto formicaria; Alt+Shift+A | **"Bring in" in the text selection toolbar** (beside Copy and Share: GeckoView lets the app add its own selection actions **[to verify in G0]**) | **"formicaria" in the selection toolbar** of every app (Android's text-processing entry); text only, no address |
| **Bring in an image, link, video** | right-click it → *Bring into "Reading"*; or drag it | **long-press** it → *Bring into "Reading"* (GeckoView reports what was pressed: image, link, video **[to verify in G0]**) | **Share → formicaria** |
| **Save page** | the ant in the address bar; Alt+Shift+K | the ant in the address bar | **Share → formicaria** → *Save page* |
| **See the note while reading** | formicaria beside the page (left, right, top, bottom: §11.6) | **the note as a sheet over the page**: a strip at the bottom with the note's name; pull it up to half height to read and write while the page stays visible, all the way up for the note alone. A tablet in landscape gets the side panel | — |
| **Where it goes** | the note in front, in the panel | the note last open in formicaria, named on the sheet; a tap changes it | a small sheet: *Into "Reading" ▾ · Bring in* |
| **Hard to select** (an image in a carousel, a table, a video) | **Pick** (right-click → *Pick from this page*): hover lights up blocks, scroll widens or narrows, click brings in | **Pick** from the page menu: tap a block, pinch out or in to widen or narrow, tap *Bring in* | — |

**The phone's own gesture for it** (owner, 2026-11-15: *"a quick tap plus a sustained one opens the
selection tool that selects the thing to bring, then the sustained tap is like copying it into the
note"*): **tap, then press and hold still** on a page opens Pick at that spot, with the block under the
finger lit and named; moving the finger (or pinching) widens or narrows it; **holding on** past a
short count brings it into the note (a ring fills around the finger, so letting go early cancels).
Firefox on Android uses *tap, then press and drag* for one-finger zoom, so the gesture is told apart
by the finger staying still; **[to verify on a device in G0]**.

Other phone gestures for the same act (brainstorm, owner: *"or similar"*), all feeding Pick and the
same pipeline:
- **The ant as an eyedropper** (recommended as the main one): press the ant on the note sheet, slide
  onto the page (the block under the finger lights up, named), let go to bring it in. Visible, so
  learnable, and it never competes with the page's own gestures.
- **Tap, then hold still** (above): quick and buttonless, but hidden.
- **Fling to the note**: select with the usual handles, flick toward the sheet.
- **Pull it in**: long-press an image or a link and drag it down into the sheet (Android's own drag
  and drop).
- **Two-finger press** on a block takes the whole block (rare on pages, but undiscoverable).
- **Circle it with a stylus** (the gesture Android's Circle to Search made familiar), in formicaria's
  browser only.

**What similar apps teach, and the simplest choice** (owner: *"intuitive and simple, minimal effort by
the user"*; searched 2026-11-15):
- **Readwise Reader highlights as soon as text is selected**, by default; a menu turns it off, and on
  a computer holding Alt (Option) selects without highlighting
  ([docs](https://docs.readwise.io/reader/docs/faqs/highlights-tags-notes)) [primary]. So selecting
  *is* the save there, which is formicaria's *bring in at once*; Alt held while selecting skips it.
- **iPhone's "lift subject"**: press and hold an image until it lifts, drag it into Notes (iOS 16)
  [secondary: MacRumors, iPhoneLife]. Press, hold, drag to the note is already learnt.
- **So the phone needs two gestures, both known**: *select text* (it comes in, or one tap on *Bring
  in* in the selection toolbar), and *press, hold and drag anything into the note sheet*. The
  eyedropper ant and tap-then-hold are shortcuts on top, not things to learn first. What happens after a selection is
formicaria's to decide on the phone as on the computer (11.3): offer, bring at once, or nothing.

On the phone nothing depends on drag and drop or on keyboard shortcuts, and the long-press, the
selection toolbar and the share sheet are the gestures Android users already know.

### 11.5 What changes from 0.34

- The menus collapse to **two items**: *Bring into "…"* and *Save page in formicaria* (and *Pick from
  this page* on a right-click of the page itself). *Keep in formicaria*, *Add to the open formicaria
  note* and *Bookmark in formicaria* go.
- The toolbar's keep button moves into the address bar as the page's ant (Save page, with the
  count); the toolbar's ant stays for showing formicaria.
- The add-on's system notifications give way to the message inside formicaria (with *Undo* and the
  one alternative).

### 11.6 Placing formicaria (computer)

Left, right, top or bottom, from a **Place** item in the panel's header or by dragging the header to
an edge (an outline shows where it will dock), as Vivaldi does; double-click the divider to switch
between beside and fill. Left and right reuse the panel's ordering; top and bottom need the window's
row laid out as a grid and are tested before they are offered (moving the page area itself would
reload every page).

### 11.7 How usability is checked

Tasks, each with a target count of presses, run by `browser-check` on its test page (computer) and
by a phone test (GeckoView, when R2 starts):

| Task | Target |
|---|---|
| Bring a paragraph into the note in front | select + 1 press |
| Bring an image in, then make it a link instead | 1 right-click + 1 press, then 1 press |
| Save a recipe page | 1 press |
| See which notes name this page and open one | 1 press + 1 press |
| On the phone: bring a selection from another app | select + 2 taps |
| On the phone: read the note while reading the page | 1 pull |

The computer's test page holds one of each kind (a paragraph with formatting, an image, a table,
code, a formula, an audio file, a video file, a link); each must arrive in the note in front as
itself, with its source, and *Link only* must leave no attachment.

Order to build: the two verbs and their menus (with the content-or-link defaults and the message with
its alternative); the address-bar ant; the selection chip (default by measurement, 11.8); drag; Pick;
placing the panel. The phone's half comes with its browser (R2), on the same pipeline.

### 11.8 Risks

- **Choosing the right block** is the hard part (Firefox's own picker misses on some sites); widening
  and narrowing is the remedy, and the label says what will be brought before the click.
- **A script on every page is light, not free** (owner, 2026-11-15: *"it should be a lightweight
  option … why are scripts heavy?"*): a few KB, idle until a selection, but Firefox keeps a small
  isolated scope for it in every page it runs in (and in every frame, unless limited to the top
  frame). It is measured in `browser-check` against the budget, not assumed. Its other limits: a page
  can see and occasionally clash with the chip, and the script acts only on the person's gestures,
  never on what a page says. Right-click, drag, paste, the keyboard and the strip need no page script;
  pick mode injects its own only while it is on.
- **Large media** fills the vault: the size limit, the chip's choice, and the attachment rules already
  in place (large files stay off git).
- **Copyright**: a personal copy, as every clipper keeps; formicaria never shares it.
