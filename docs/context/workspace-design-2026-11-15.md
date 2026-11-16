# formicaria as a person's workspace: the design (research, 2026-11-15)

_Research and design, not a ruling. Nothing here is built. It continues `browser-plan-2026-11-15.md`
(the browser is formicaria's window, §14–§16) and asks the larger question the owner put._

> **The owner:** *"we can study the UI of the app, it looks like an app that helps with the main
> workspace of a person. Even before this browser feature, it was resting in a browser, we are just
> making sure we can integrate more the web search, which is nowadays one of the main if not most used.
> So what I expect is to make sure we have a well designed interface, allowing the user to manage all
> the main workflow of a person. This often requires email checking, and searches and notes we take on
> the way while doing so, no code, just a design search. Notably, I want to make sure that once we have
> it we can optimize it later."*

Labels as in the browser research: **[doc]** a product's own page, **[secondary]** reviews and
articles found by search, **[not verified]**, **[proposal]** this document's own design.

---

## 1. What a person's day is made of

Five workflows carry most of a working day. formicaria already holds parts of each, scattered.

| Workflow | Its steps | In formicaria today | Friction today |
|---|---|---|---|
| **A. Mail in** | read new mail → decide (reply, act, plan, keep, drop) → turn the work it creates into tasks, notes or meetings | Gmail read-only: a note per conversation; the assistant proposes meetings; verdicts in Collaboration | replying happens in Gmail elsewhere; a conversation's tasks and notes have no single path; nothing says "this needs you" |
| **B. Search and read** | a question → web search → read several pages → keep what matters | 0.31–0.33: formicaria's own browser, *Keep*, *Bookmark*, downloads into the vault | search results, kept pages and the question that started them are not connected |
| **C. Write with sources** | a note open beside what it is about (a page, a paper, a mail) → quote, link, summarise | split view (0.33), quotes with links back to the passage | the note and its sources do not know about each other beyond links |
| **D. Plan the day** | what is due, what came in, what to do next | Agenda, reminders, Tasks board; the app opens on the Library's Timeline | no single "today": mail, captures and the agenda live in different places |
| **E. The life domains** | food, training, health, music, reading | five groups of their own | well served; they need only to stay one press away |

**The thread through all five is capture on the way**: the thought, quote, link or file met while doing
something else, kept without stopping, and found again in context.

## 2. Why one place, measured

- **Switching costs.** A study of 137 people in 20 teams at three large companies over five weeks:
  - about **1,200 toggles between apps and sites a day**;
  - each costs about **2 seconds to re-orient**, adding to roughly **four hours a week, about 9 % of
    working time** **[secondary: reported from Harvard Business Review]**.
- **Interruptions.** In Gloria Mark's field studies people worked about **10.5 minutes** on a task
  before an interruption, and took about **23 minutes** to return to it **[secondary]**. These are
  older, small studies, so they show a direction, not a model.
- **What that asks of the design:**
  - **fewer switches**: the next thing is in the same window;
  - **cheaper switches**: context kept, so a return costs nothing;
  - **capture without leaving**: one key, no destination to choose first.

## 3. What others did, and what to take

| Product | What it does | Take | Leave |
|---|---|---|---|
| **Vivaldi** | a browser with **built-in mail (IMAP/POP3, searchable locally), calendar, feed reader and notes**; side panels, including *web panels* kept open beside a page; **layout profiles**, from *Essentials* to *Fully Loaded* **[doc]/[secondary]** | the closest existing shape to "one window for the day"; **the person picks the density**; the calendar as a side panel while browsing | an IMAP client of its own (formicaria reads Gmail into notes and leaves sending to Gmail) |
| **Arc → Dia** (The Browser Company) | Arc reinvented tabs, bookmarks and spaces; its makers report **most users never used the features they valued most**, and that those were either too hard to learn or not valuable enough alone. Dia keeps core features **where people expect them** within a **"novelty budget"**, aiming at **one obvious way to do each thing** **[doc]/[secondary]** | **familiar first**: tabs, the address bar, Back work as everywhere; spend novelty only where it pays (capture, connection) | redesigning what people already know how to use |
| **Superhuman** | mail triage at speed: **Cmd+K for every action**, single keys (J/K to move, E to archive), *speed as the product* **[doc]**. A review notes it clears mail fast but "does nothing with the work email creates" **[secondary]** | single-key triage; one command line | a mail client of its own |
| **Sunsama / Akiflow** | the work mail creates goes into a plan. **Akiflow**: one inbox fed by Gmail and other tools, then onto the calendar; layout **inbox · Today/Week/Month columns · calendar**. **Sunsama**: a guided morning ritual **[secondary]** | **an inbox for everything that comes in**, and **today** as a view of its own | a ritual imposed every morning (offer it, never force it) |
| **Command palettes** (Linear, Raycast, Slack, GitHub) | Cmd/Ctrl+K: search and act. Guidance: **recents first** when empty; **search results and commands shown differently**; a no-match offers a next step; Escape leaves a sub-level first; keyboard-complete and accessible **[secondary]** | one command line that is also the address bar | – |
| **Readwise Reader, Zotero, Obsidian's clipper** (browser research §4) | capture from the page with templates; highlights back to the source | already taken: *Keep*, highlights as links back | – |

## 4. Design principles for formicaria **[proposal]**

1. **Three attentions, always one press away**:
   - **what comes in** (the Inbox): mail that needs you, kept pages, downloads, the assistant's
     proposals, reminders;
   - **what you are working on** (the work area): a note, and the page or mail it is about, beside it;
   - **what is planned** (Today): the agenda, what is due, what you chose for today.
2. **Familiar first, a novelty budget** (Dia's lesson). Tabs, the address bar, Back, right-click,
   Ctrl+T and Ctrl+L behave as everywhere. formicaria spends its novelty on capture and connection only.
3. **One command line for everything** (Ctrl+K, and the address bar is the same thing). Type, and
   see notes, places and mail conversations, then **actions**, then **the web**. Actions are drawn
   apart from results; recent items when empty; a no-match offers "search the web" and "new note".
4. **Capture never leaves.** From anywhere, with one key:
   - keep the page, a selection, a link or a download;
   - write a quick note.

   It lands in the Inbox with its context (where it came from, when, what you were doing), to be
   sorted later or never. Sorting is optional; finding is not.
5. **Mail becomes work, not a second home.**
   - A conversation opens as a note.
   - Single keys turn it into a task, a note or a meeting (the assistant's proposals already do part
     of this), or mark it done.
   - **Reply opens Gmail beside it** in formicaria's browser, so formicaria needs no mail client of
     its own, and Gmail stays read-only for formicaria.
6. **A place keeps its context.** A recipe page opened from Food belongs to Food; returning to Food
   restores its pages and notes (tab groups per place, `browser-plan` §14.9 Q2). Leaving a place
   costs nothing; coming back costs nothing.
7. **Density is the person's** (Vivaldi's lesson): *Essential* (a clean window) or *Full* (every panel).
   Both are layouts of the same parts, never two apps.
8. **Each device its own shape**, the same names (`decisions.md#ui`, *the layout may differ, the names
   never do*):
   - **Desktop**: keyboard first; side list, split view, command line.
   - **Phone**: one thing at a time; a bottom bar (Today · Inbox · Search · Places · Tabs); the share
     sheet as the way in; swipe between neighbours.
   - **Tablet**: the desktop's panes, with touch targets.
9. **Built to be measured, so it can be optimised** (§7).

## 5. The window **[proposal]**

```
Desktop, "Full" density
┌──────────────────────────────────────────────────────────────────────────────────┐
│ [≡] [ Ctrl+K  search notes, mail, places, actions … or the web              ]  ⬇ ⋯ │  one command line
├───────────────┬──────────────────────────────────────────┬──────────────────────┤
│ TODAY      3  │                                          │ CONTEXT              │
│ INBOX     12  │   the work area: one pane, or two        │  sources · links     │
│ ─ Places ───  │                                          │  backlinks           │
│ Library       │   ┌─────────────────┬─────────────────┐  │  the assistant       │
│  · Paper X    │   │ note            │ page / mail     │  │  (collapsible)       │
│ Tasks         │   │ (formicaria)    │ (web tab)       │  │                      │
│ Food          │   │                 │                 │  │                      │
│  · carbonara ↗│   └─────────────────┴─────────────────┘  │                      │
│ Fitness …     │                                          │                      │
│ Web           │                                          │                      │
│  · 3 tabs     │                                          │                      │
└───────────────┴──────────────────────────────────────────┴──────────────────────┘
"Essential" = the same window with the side list and the context pane folded away.
```

- **TODAY** is the first screen of the day: meetings and what is due (Agenda); conversations that need
  you (Mail); what you kept and have not sorted (Inbox, a count only); the notes you were in yesterday.
  No ritual, but one is offered: *Plan my day* walks Inbox → Today in two minutes, Sunsama-like.
- **INBOX** is one list of everything that came in, newest first, each with its source. One key each:
  file it to a place, make it a task, make it today's, open it, or drop it.
- **Places** are formicaria's groups, plus **Web**, each with the notes and pages open in it
  (`browser-plan` §16: notes are entries in the one formicaria tab; pages are real tabs).
- **The context pane** shows what the open item is connected to: the page a note quotes, the mail a
  task came from, the notes that link here, and the assistant. Folded by default in *Essential*.

```
Phone
┌──────────────────────┐
│ Today          🔍  ＋ │   ＋ = capture: note · photo · voice · link
│ ─────────────────────│
│ 10:00 Meeting …      │
│ 2 conversations need │
│   you                │
│ 4 kept, unsorted     │
│ …                    │
├──────────────────────┤
│ Today Inbox ⌕ Places ▣│   ▣ = tabs (web pages), GeckoView started only here (§16)
└──────────────────────┘
```

## 6. The workflows, walked through **[proposal; targets to measure, §7]**

| Workflow | Steps in the proposed design | Target |
|---|---|---|
| A. Mail | Today shows "2 need you" → open (Enter) → **T** task / **N** note / **M** meeting / **R** reply (Gmail beside) / **E** done | a conversation decided in ≤ 3 keys; no window switch to read |
| B. Search | Ctrl+K, type, **Tab** to "search the web" → results in a tab of **Web** → **Alt+Shift+K** keeps a page; the question becomes the Inbox item's title | from question to kept page with no app switch; the question kept with what was found |
| C. Write | a note open → drag or "open beside" the page → select on the page → **Keep selection** adds a quote with its link into the open note | quote into the note in one action, its link back intact |
| D. Plan | Today → *Plan my day* (optional) → Inbox items to Today with one key each | the day's list made in under 2 minutes |
| E. Domains | Places → Food / Fitness … as now; a page opened from Food stays in Food | no change in presses from today (§16 rule 7) |

## 7. Built to be optimised later

The owner asked that the design be optimisable once it exists. That means deciding **now** what is
measured, how, and how a change is compared, without breaking formicaria's privacy stance (nothing
sent anywhere, `site/privacy.html`).

**What to measure.** Google's **HEART** framework (Happiness, Engagement, Adoption, Retention, Task
success, with *Goals → Signals → Metrics*) **[doc/secondary]**, kept to the categories that fit a
personal tool:

| Category | Signal in formicaria | Metric |
|---|---|---|
| **Task success** | the five workflows of §6 | completion without a detour; **time and number of switches per workflow**; Undo right after an action (a mistake) |
| **Responsiveness** (formicaria's own, §16) | the measures that already exist | view switch, note open, typing, capture to saved (target ≤ 1 s), command line to first result (target ≤ 100 ms) |
| **Findability** | Ctrl+K and search | median time from Ctrl+K to opening the thing; share found in the top 3 |
| **Happiness** | an occasional one-tap question, opt-in ("did that go the way you wanted?") | a running score per workflow |
| *Adoption, Retention* | dropped: these are growth metrics for a service, not a personal tool | – |

**How, keeping privacy:**
- a **local, opt-in measurement log on the device** (not in the vault, never synced, never sent);
- each workflow has named start and end points, so the time and the switches between them are counted
  without recording content;
- the person can see it (*Settings → How formicaria is doing for you*) and **export it only if they
  choose**, for example to send the maintainer a report.

**How a change is compared:**
- **Baseline first**: before any redesign, measure today's five workflows: by hand once, scripted where
  possible.
- **Scripted workflows in CI** (headless, the real engine, as `browser-check` does): each of §6 run as
  a script, timed, so a release that makes one slower is caught, like the memory budget.
- **Alternatives as switches**, not as A/B tests across people, since there is no telemetry. A layout
  or a flow under study ships behind a switch; the owner, or anyone who opts in, compares the two on
  their own work with their own numbers.
- **The design as data**: places, panels, densities and the command line's sources are configuration
  (the way views and themes already are files), so optimising is changing data, not rewriting screens.
  The existing design tokens (`ui/src/app.css`) already do this for the look.

## 8. What formicaria lacks today, against this design

1. **One Inbox** for mail needing action, kept pages, downloads, proposals and reminders, which today
   live in four places.
2. **Today** as the first screen (the app opens on the Library's Timeline, `decisions.md#ui`
   2026-11-12).
3. **One command line** that searches notes, mail, places and actions, and falls through to the web
   (search is a passing panel today).
4. **Mail actions by single keys**, and **Reply beside** in Gmail.
5. **Places that keep their tabs** (tab groups per place in the browser's list).
6. **The context pane** (links, sources, backlinks, the assistant in one place).
7. **Densities** (*Essential* / *Full*).
8. **The measurement log** and **scripted workflows** (§7).

Items 1–3 carry most of the value; 4–6 depend on the browser's shell mode (`browser-plan` §14.4);
7–8 make it optimisable.

## 9. A proposed order (design work first, no code until agreed)

| Step | What | Output |
|---|---|---|
| W0 | **Baseline**: time and count today's five workflows on the desktop and the phone | a table in this doc |
| W1 | **Wireframes**: desktop *Full* and *Essential*, phone, for Today, Inbox, the command line, a mail triage and a research session | images in this doc |
| W2 | **A clickable prototype** behind a switch (shell mode) | the owner uses it on real work for a week |
| W3 | **Measure against W0**; keep what is better | the decision entry |
| W4 | **The measurement log and scripted workflows**, so every later change is measured the same way | `ci/` scripts, a Settings page |

## 10. Questions for the owner

1. **The first screen**: *Today* (what is due, what needs you, what came in), replacing the Library's
   Timeline as the default? Recommended: yes.
2. **One Inbox** for mail-that-needs-you, kept pages, downloads, proposals and reminders? Recommended:
   yes.
3. **Reply in Gmail beside** (formicaria stays read-only on mail), or should formicaria send mail
   itself one day? Recommended: beside.
4. **Measurement**: the local, opt-in log plus scripted workflows in CI? Recommended: yes.
5. **Densities**: *Essential* and *Full*, chosen per device? Recommended: yes, *Essential* by default on
   the phone.

## 11. One library, its sections, and the collection you work with (plan, 2026-11-16; build after 0.37)

> **The owner:** *"the overall fm view can use a mixture of techniques in the back but appear uniform
> in the front (like tabs) … a web feature will require a browser tab but a note feature can have a
> more lean tab … food, fitness, music have dedicated views but can appear as separate tabs … we
> retain the customization … we need a cohesive model that allows them all to be accessed, swapped,
> edited from the same view."* Then, settling the words: *"fm is the library (in the broadest
> sense), it has sections (music, food, video…) and every section has study rooms as well (notes
> with collaboration, Myrme…). Each section has catalogues (their views). At a given time, a user
> has a collection made of many different things (food, web, emails…) it uses to study or work."*
> And: *"the mail would stay in the mail client, not go into the notes, unless one wants to bring it
> there to use it to study. Every section has its own location and no need to bring it into the study
> notes necessarily."*

> **Status (2026-11-16, 0.38):** built D0 (the words; *Study rooms*), D1–D2 (lean tabs for study
> rooms and sections), D4 (the library search), D5 (sections described once; Mail as its own section)
> and D7's formicaria side (the assistant's access per section, all off). Not yet: D3 (one-drag
> *beside* for any two tabs), D6 (the phone), D7's assistant side. `decisions.md` 2026-11-16 (night,
> later).

### 11.0 Scope: the core stays the same

> **The owner:** *"in essence fm stays in the core the same, I am trying just to more cohesively link
> it with the places the knowledge and data comes from."*

Nothing in formicaria's core changes: notes as files, the notebook and its sync, each section's
views and settings, the assistant. This plan adds **a layer of access and linking** on top:
- **one way to have anything open** (the collection, as tabs, whatever the engine behind);
- **one search over the library** (every section asked at once);
- **one way to bring any source into a study room** (the web's *Bring in*, for every section);
- **one description per section** (11.5), so a new source plugs into all of these;
- later, **places where Myrme acts**: each section can offer the assistant its own way to search and
  read it (the owner: *"so we have data and knowledge sources, but in the future also places where
  Myrme can act to actively search them"*).

Where it touches an existing part, the least change is taken: no item moves, no store is added, a
name changes only where the words would otherwise clash.

### 11.1 The words (settled by the owner, 2026-11-16)

| Word | In a library | In formicaria |
|---|---|---|
| **The library** | the building and everything in it | **formicaria** itself, in the broadest sense |
| **Section** | the books, the films, the records, each kept its own way | **Music, Food, Video, Web, Mail, Fitness, Health, Calendar…**, and whatever comes next (a watch). Each has **its own location**: its items live there and need not go anywhere else |
| **Catalogue** | a section's card index and shelves | **a section's views**: its own search, layouts, saved views, shelves and settings (all kept as they are) |
| **Study room** | the rooms where you sit, read and write, with others | **notes**, with **collaboration** (discussions, proposals) and **Myrme**; every section has them: a note can be written beside any section's items |
| **Item** | one book, one film | one song, recipe, page, mail conversation, workout, note… |
| **Collection** | what you have taken out and spread on your table right now | **what is open now**: things from any section (a recipe, two pages, a mail, a note), the tabs, to study or work with |
| **Source** | where the library acquires from | where items come from (web, mail, calendars, files, a watch): the source contract (`data-sources-research` §5) |

**Consequences of the words:**
- Today's **Library** view (the notes) is the **study rooms'** catalogue; its name in the app is to
  settle (11.8, Q1).
- **Mail stays in the Mail section** (its client and its catalogue); it enters a study room only when
  the person **brings it in** (a quote, a link, a passage), as a web page does. This reverses part of
  *the email exchange lives in the conversation note* (2026-10-09); `decisions.md` 2026-11-16 (night).
- **Each section's items stay in their section.** Bringing something into a study room is a
  deliberate act (the *Bring in* of the web, for every section).

### 11.2 The model: uniform in front, a fitting engine behind

**Everything open is a tab in the collection**, and every tab behaves alike: a title and its
section's icon; open, close, reorder by dragging, pin, group, restore after a restart, put beside
another. What differs is the **engine** behind it, chosen by what the item needs:

| Item | Engine behind its tab | Cost |
|---|---|---|
| **A web page** | a real browser tab (Gecko) | a web page's (~65 MB measured) |
| **A study room** (a note), **a catalogue** (Music, Food, Fitness, Health, a saved view…) | a **lean tab**: a tab that loads nothing (`about:blank`, never a copy of the app); selecting it shows **the one formicaria** at that item | about nothing per tab; formicaria itself runs once (`browser-plan` §16) |
| **A mail conversation** | the Mail section's own view (its client), in formicaria | as a catalogue; Reply in Gmail beside it, a web tab |
| **A future section** (a watch, another mail client) | whichever fits: a lean tab onto its catalogue in formicaria, or a web tab onto its service | declared by the section (11.5) |

**Each section keeps its catalogues and its ways** (the Music deck, Food's recipes and day, Fitness,
saved views, shelves, themes): the collection does not replace them, it **holds** them. A tab onto a
catalogue remembers its state (filters, layout), so two tabs can show the same section two ways.

### 11.3 How it looks and behaves (computer)

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│ [♪ Music] [▣ Food · Today] [✉ Mail · Lab] [📝 Reading] [🌐 Ant – Wikipedia] [+]        │ the collection
│  ←  →  ⟳   [ search the library, or an address …                         ] 🐜  ☰      │
├─────────────────────────────────────────────┬────────────────────────────────────────┤
│  📝 Reading (a study room: formicaria)       │  🌐 Ant – Wikipedia (a web page)        │
└─────────────────────────────────────────────┴────────────────────────────────────────┘
```

- **Opening** anything opens it as a tab: an item from a catalogue (or *Open as a tab*), a section
  from formicaria's side list, a search result, a link. Middle-click and Ctrl+click open a new tab.
- **Beside**: drag a tab onto the left or right half of the window to put it beside the one shown:
  a study room beside a page (today's panel), a page beside a page (Firefox's split view), a study
  room beside a catalogue. One gesture, whatever the engines.
- **Bring in**, from any section into a study room: the web's way (select, snip, drag) extended to
  every section: a passage of a mail, a recipe, a song, a workout, each arriving as itself with its
  source.
- **Selecting a lean tab** shows formicaria at that item, filling the content area (or beside the
  page it was paired with).
- **Closing a tab** returns the item to its section; nothing is deleted.
- **Restarting** restores the collection: web tabs as the browser does, lean tabs by their item's
  address.
- **Searching the library** (Ctrl+K, or typing in the address bar) asks every section at once,
  grouped (Study rooms, Music, Food, Mail, saved pages…), then offers the web; each section's own
  catalogue keeps its own, richer search.
- **Keeping a collection**: tab groups (Firefox's) for a project (*Thesis: 3 study rooms, 4 pages, 1
  mail*); a group can be **kept** and reopened whole later, like a reserved shelf.
- **formicaria's own list of open windows** (its panes) shows only outside formicaria's browser (a
  plain browser, a paired tablet); inside it, **the collection is the list**, never two lists.

### 11.4 The phone

The same words in a phone's shape (`browser-plan` §14.5, §16): the collection is a **grid of cards**
(one per open item, of any section), swiped away to close, dragged to reorder; one item at a time,
full screen; **swipe sideways** between neighbours; a study room **beside a page** is the note as a
sheet over the page (`data-sources-research` §11.4); web pages run in GeckoView, started only when a
page is opened.

### 11.5 What each section declares (the cohesive part)

One small description per section, in code, so a new one (Mail's own client, a watch) plugs in with
no new machinery:
- its **name and icon**, and **its location** (where its items live);
- its **catalogues** (views) and how a tab onto one saves its state;
- its **item kinds** and their **address** (`fm://music/<song>`, `fm://web/<url>`, `fm://mail/<thread>`:
  the same addresses the assistant uses, `data-sources-research` §7);
- its **engine** (lean tab, or a web tab and onto what);
- how **the library search** asks it;
- how its items are **brought into a study room** (what a passage of it looks like there, with its
  source);
- its **sources** (the source contract's catalog);
- **what Myrme may do there** (later): the section's own search and reads, offered to the assistant as
  structured commands (search the web, look through mail, query a watch's workouts, the music
  library), each section **off for the assistant until the person turns it on**, reads before any
  write, and what it finds coming back as **proposals the person accepts**; content a section brings
  in is material, never instructions (`data-sources-research` §7).

### 11.6 Build order (after 0.37), each step measured and in `browser-check`

| Step | What | Done when |
|---|---|---|
| D0 | **The words** in the app and manual (11.1); the study rooms' catalogue named | `decisions.md`; the manual's terms |
| D1 | **Lean tabs for study rooms**: *Open as a tab*, middle-click on a note link; a tab that loads nothing, its item kept across restarts (Firefox's session store keeps a tab attribute); selecting shows formicaria at the note | memory per lean tab ≈ 0 and switching under 100 ms, measured; restored after a restart |
| D2 | **Sections' catalogues as tabs**: Music, Food, Fitness, Health, saved views; each keeps its state; formicaria's own window list hidden inside its browser | each opens as a tab and comes back as it was |
| D3 | **Beside, one gesture**: drag a tab to a half of the window, whatever the two engines | room+page, page+page, room+catalogue |
| D4 | **Searching the library**: every section answers, in Ctrl+K and the address bar | results open as tabs |
| D5 | **The section description** (11.5) in code; **Mail as its own section** (its client view; brought into study rooms only when chosen); its files stay where they are (11.0) | a new section needs no new machinery; mail no longer appears among study notes |
| D6 | **The phone's collection** (11.4), with its browser (R2) | the same names, cards, swipe |
| D7 | **Myrme acting in sections**: each section's search offered to the assistant, a switch per section (off by default), results as proposals; with Myrme's repository | the assistant searches a section the person opened to it, and nothing else |

### 11.7 Risks

- **Firefox's tab internals** (lazy tabs, session attributes) can move between ESR majors: few hooks,
  each exercised by `browser-check`.
- **Two lists of what is open** is the failure to avoid: inside formicaria's browser the collection is
  the only one.
- **Moving mail out of the notes** touches what reads it today: the search, the assistant's meeting
  proposals (`fm_agent::meetings`) and sync. D5 keeps each working against the Mail section.
- **A rename confuses** people who know *Library*: the manual and a one-time line say it.
- **Memory**: lean tabs must stay lean (measured per step).

### 11.8 Questions for the owner

1. **The study rooms' catalogue** (today's *Library* view of the notes): call it **Study rooms**, or
   keep **Library** for it? Recommended: **Study rooms**, since formicaria is the library.
2. **Selecting a study room's tab**: fill the window, or stay beside the page it came from?
   Recommended: fill; *beside* is one drag.
3. **Tab groups**: only when the person makes one (recommended), or one per section automatically?
4. **The phone's bottom bar**: **Today · Collection · Search · Sections**? Recommended: yes.
5. **Where Mail's items live**: by 11.0, **where they are today** (files in the notebook); what changes
   is only where they are *shown* (the Mail section's own view, not among the study notes). A
   separate store would be a core change, so it is not proposed.

## 12. What is left, in the order it is built (plan, 2026-11-16)

> **Status (0.40):** 1 (D3), 2 (Mail as a client) and 3 (Today and its Inbox, the first screen unchanged)
> are built. 4 (the phone's browser, phase A): a first version as a separate screen was withdrawn
> before release (§12.1: a page is a layer of formicaria); 0.41 ships the layer. Next: 8 (Windows and macOS). **Last, by the owner's
> priority (2026-11-16): 5 (a watch, Gadgetbridge) and 6 (Myrme acting in sections)**; 7 waits on the owner.

> **The owner:** *"plan D3 and the phone browser next"*, then *"build all that is left to build, plan
> it if necessary"*. Each step ships when `browser-check` and `pixi run ci` pass; what cannot be
> verified on this machine says so.

| # | What | How | Verified by |
|---|---|---|---|
| 1 | **D3: any two tabs beside each other, one drag** | drag a tab onto the left or right half of the page area: two pages → Firefox's split view; a study room/catalogue and a page → formicaria's panel beside the page, on that side; two study rooms → formicaria's own two panes (`#pair=`) | `browser-check` (the pairing, by its function) |
| 2 | **Mail as a client view** | the Mail section as a list and a reading pane (the conversation), *Reply* opening the thread in Gmail beside it (a web tab, formicaria stays read-only on mail), *Bring in* of a passage into a study room | app tests |
| 3 | **Today and one Inbox** (§10, Q1–Q2 on their recommendations) | *Today* (meetings and what is due, conversations that need you, what came in unsorted) as the first screen; *Inbox*: everything that came in (kept pages, passages, shared items, downloads, proposals), each with its source, one press to file it | app tests |
| 4 | **The phone's browser, phase A**: pages inside formicaria on Android's own engine (System WebView: 0 MB) | a page screen beside the collection's cards; formicaria's page script (the selection icon, the snip) injected into pages; the note as a **sheet over the page**; the ant as an **eyedropper**; tap-then-hold. No Firefox add-ons in this phase | compiled with the Android SDK (bootstrapped by the project's own pinned script), the app's tests; on a device: by the owner |
| 5 | **A watch as a source** (Gadgetbridge's FIT files, `training-coach-plan` P1) | a FIT file shared or added becomes a workout in Fitness, the file kept raw, the source contract's fields | tests on public sample FIT files |
| 6 | **D7: Myrme searching sections** | Myrme's side, in its repository: each section's search as a tool, the per-section switch in Settings (off), findings as proposals | Myrme's own evals |
| 7 | **The phone's browser, phase B**: GeckoView with formicaria's add-on as a built-in extension (Firefox add-ons on the phone) | **needs the owner's decision**: a separate optional *formicaria browser* app (keeps formicaria under 100 MB), or a raised budget; and G0's measurements on a device. GeckoView is about 60 MB for arm64 ([bug 1727150](https://bugzilla.mozilla.org/show_bug.cgi?id=1727150): *"a big ask for non-browser apps"*); it loads built-in extensions with content scripts ([docs](https://firefox-source-docs.mozilla.org/mobile/android/geckoview/consumer/web-extensions.html)) | a device |
| 8 | **Windows and macOS for formicaria's browser** | the kit applied to Mozilla's installers there (an unpack on Windows; a re-sign on macOS) | those systems (not here) |

### 12.1 Minimal by default: one at a time, a quick switch, both only when there is room (2026-11-16)

> **The owner:** *"I would keep things incredibly minimal. While browsing, I might want the note right
> there, or I can have it only at command through a simple formicaria-like icon that makes it
> apparent it is there. So yes, it can be over but only if space allows. In a phone, we can have a
> quick mechanism to switch between them … too many things in a screen is not what people want on a
> phone. Similar things apply to the desktop: we can also allow a quick switch on top of the
> concurrent view."*

- **Phone**: the page **fills the screen**; a small **floating formicaria icon** says the note is
  there. **Tap** it: the note, at once; a small icon there goes back to the page where it was (both
  stay alive, nothing reloads). The icon **follows the moment**: with text selected, a tap **brings
  it in**; **pressed and dragged** onto the page, it is an **eyedropper** (the block under the finger
  comes in); otherwise a tap **switches**. **The note over the page only when space allows** (a
  tablet, a large phone in landscape). To switch instantly, the page lives **inside formicaria's own
  screen** (a layer over it), not a separate screen.
- **Why not a separate screen** (the owner: *"the entire app is now formicaria, you should not have two
  copies"*): a page is an item of the collection; a separate page screen put it outside formicaria,
  out of reach of the note. One formicaria, the page a layer of it.
- **Computer**: *beside* stays; **a quick switch on top of it**: one key (and formicaria's icon)
  flips between the page alone and formicaria alone; when formicaria is folded away, the same small
  floating icon on the page shows it is there.

### 12.2 Switching between formicaria's features, not between formicaria and a browser (2026-11-16)

> **The owner:** *"We are not switching between fm and the browser. We are simply switching between fm
> features."*

The web is one of formicaria's features, as notes, Music or Food are. So:
- **A web page is an item of the collection**, like a note or a section: on the phone among the
  collection's cards, on the computer a tab among the others.
- **The quick switch goes between the last two items, whatever their kinds** (a page and a note, two
  notes, a note and Music). Its small floating button shows **the icon of the item it goes back to**.
  Never "show formicaria" or "hide the browser".
- **Together** is any two items side by side where there is room (D3 on the computer; a tablet or
  landscape on the phone).
- **Words on screen** name items, never "formicaria" as if it were a separate thing: the panel's head
  names the item it shows; folding it is *close this side*.

## 13. One library of notes (the detailed plan, 2026-11-16; approved, refined)

> **The owner:** *"formicaria contains many features like browsing, it is not contained in them … fm
> should accommodate many information sources … views were there exactly for this, to navigate in
> intuitive ways different sorts of assets … Music, food, fitness, calendar and so on all rely on notes,
> of special kinds, structured, just like special kinds of books … we can increase the library, improve
> it by editing it, but also use it to study and produce other notes, which can themselves end up in
> some place in the library"*; content goes **both ways** (a discussion with Myrme helps write an
> email). This section supersedes §12.1's *page layer* wording and the separate phone page screen.

> **Status (0.41):** step 1 (the kind registry, `kind_key` on every note) and step 2 (one piece, `put`;
> ⇥ on every note brings a passage into another) are built; the computer's quick switch (§13.7) too.

### 13.1 The cycle

**Sources acquire → notes of kinds ← views navigate → study (alone, with Myrme) → new notes → filed in a
place; hand-offs go out where the library ends.** Nothing here adds a store: notes stay files, views
stay views, `dispatch` stays the one surface.

### 13.2 The kind registry (step 1)

One table, in Rust (`crates/fm-app/src/kinds.rs`, growing from `source.rs`), **recognising a note's
kind once, on the server**; `ObjectMeta` carries it (`kind_key`), so the UI never guesses (today's
`ui/src/lib/sections.ts` heuristics go). A read-only `kinds` command returns the table (for the UI and
Myrme; it replaces `sources`).

| Kind | Recognised by (today's markers) | View(s) | Source(s) | A passage of it, in another note | Out (hand-off) | Default place |
|---|---|---|---|---|---|---|
| **study note** | none of the below | Study rooms | — | quote + `note:` link | Mail (new), Share | Study rooms |
| **web clip** | `source: web` / tag `web` | Study rooms (Web shelf), Today | the web (browser) | quote + page link at the passage | — | shelf *Web* |
| **paper** | tag `paper` / `doi` | Study rooms (Papers) | the web, BibTeX | quote + citation | — | shelf *Papers* |
| **recipe** | `recipe` | Food (Recipes) | the web, typed | ingredients/steps as text + link | — | Food |
| **log entry** | `log: food\|workout\|health\|music` | Food, Fitness, Health, Music | typed, a watch (later) | the entry's values as a line | — | its view |
| **song** | `log: music` + audio asset | Music | files, downloads | title, artist + link | — | Music |
| **meeting** | tag `meeting` / `ics_uid` | Calendar (Agenda), Tasks | calendars, mail | title, time, place + link | Calendar (event) | Calendar |
| **conversation** | `gmail_thread` | Mail | Gmail | the message, quoted + link | Mail (reply) | Mail |
| **discussion** | `thread_of` / discussion root | Collaboration | Myrme, people | the message + link | Mail (new/reply) | Collaboration |
| **asset** | `Kind::Asset` (a file, a photo, a video) | the notes using it | phone gallery (Share, 📎), files, downloads | embedded (`asset:`) | Share | where used |

### 13.3 One piece (step 2)

`Piece` (`ui/src/lib/web/bring.ts`, generalised to `ui/src/lib/piece.ts`): `kind` (text, image, link,
table, code, audio, video, file, event, note) · `html` / `text` / `md` · `src` / `asset` · `time` · and its
**source**: `{ kind, address, place, title, when }` (`address`: a page's URL, `note:<id>`, `asset:`).
**One conversion** `pieceMarkdown(piece)` (today's `textPiece` / `imageKept` / `mediaLinked` …, plus
note and event readings from §13.2); **one consumer** `put(noteId, pieces)` → `bridgeInbox.insert`
(through the note's own pane, one undo, as now).

Producers, re-expressed (their screens unchanged): the add-on (chip, snip, Pick, drop, Alt+Shift+A) →
pieces; the phone's Share and selection menu (`SharedText`) → pieces; the phone page painter's bring
(`Brought`) → pieces; the mail passage (`ui/src/lib/web/mail.ts`) → a piece of kind text with a
conversation source; a discussion message (Myrme) → a piece. **New**: between two open notes (side by
side), drag a selection from one into the other, or *Send to the note beside*.

### 13.4 Filing as placing (step 3)

A **place** = a section (from the kind) and, inside it, a **shelf** (`shelf:` tags, as today).
- *File it* becomes **Place…**: a small picker in the note's head: the suggested place (§13.2's
  default), the shelves of that section, a new shelf. It sets the `shelf:` tag and `filed`.
- **Today's Inbox** = notes with a `source` and no `filed`: the new acquisitions.
- A note produced while studying has **Place…** too; unplaced it stays in the Study rooms' timeline.

### 13.5 Views and their sources (step 3)

A view whose kind has a source shows one quiet line with that source's state and an action, through
the existing commands: Mail (*Gmail · synced 2 min ago · Sync now* → `mail_sync`), Calendar
(*3 calendars · refreshed … · Refresh* → `calendar_sync`), Music (*Add songs from files*), Study rooms'
Web shelf (*n pages saved · open the browser*), the phone (📎 *from the gallery*). A generic
`source_status` read for each.

### 13.6 Out of the notes (step 4)

**Send to…** in a note's head and on a selection in it (and on a discussion message):
- **another open note** → `put` (§13.3);
- **Mail, new** → Gmail's compose opened with the subject and body
  (`https://mail.google.com/mail/?view=cm&fs=1&su=…&body=…`); in formicaria's browser as a tab beside;
- **Mail, reply** (the note is a conversation, or quotes one) → the thread opened (`mail_link`) and the
  text **copied**, said plainly (*Copied; paste it in the reply*): Gmail takes no prefilled reply;
- **Calendar** (the note has `start`/`due`) → Google Calendar's new-event page prefilled
  (`…/calendar/render?action=TEMPLATE&text=…&dates=…&details=…&location=…`), or an `.ics` file for other
  calendars;
- **Share** → the phone's Android share sheet (a new shell call `__fmShare.out(text, subject)`); the
  computer copies.
Mail stays read-only (`gmail.readonly`); nothing is sent without the person. The add-on opens only
`mail.google.com` and `calendar.google.com` beside.

### 13.7 The collection and the quick switch (step 5)

- **Computer** (in formicaria's browser; written, uncommitted): Alt+Shift+X to the previous item, any
  kind; the panel's head names the item (the app titles its document by it); *close this side*; a small
  floating mark when that side is closed. To add: `browser-check` checks.
- **Phone**: a web page is a pane kind (`page`: `url`, `pageTitle`) of the collection; the Android shell
  only **paints** a WebView in the rect the app gives (`__fmPage.show/hide/close/bring/pickAt`), replacing
  the withdrawn `PageActivity`; the bar's switch button (the previous item's icon); with a page in front
  it also brings the selection, and as an eyedropper when dragged; cards list pages; two together when a
  card is dragged to a side.
- **Firefox kept bare**: the home and account buttons leave the toolbar; the tab row and the address
  bar stay; formicaria's actions are in formicaria's own UI.

### 13.8 Releases

| Release | Steps | Checked by |
|---|---|---|
| 0.41 | 1 (kinds, `kind_key`) + 2 (one piece; note → note) | Rust tests per kind; UI tests per piece kind; `browser-check`'s bring/snip/chip unchanged |
| 0.42 | 3 (Place…, Inbox, views' source line) + 4 (Send to…) | UI tests: placing, each hand-off's address and text |
| 0.43 | 5 (collection + switch on both; Firefox bare) | phone app tests, the Kotlin compiled (pinned SDK), `browser-check` quick-switch checks |
| later | the watch (a source of log entries), Myrme across the library | — |

### 13.9 Risks

- **Recognising kinds once** changes what `sections.ts` guessed: tests pin each marker (§13.2).
- **Hand-offs depend on Google's URLs** (compose, calendar template): tested by their built addresses;
  other mail or calendar services get the copy / `.ics` path.
- **The phone painter** draws native views over the app: positions follow the app's layout (resize,
  keyboard); checked in the app's tests by the calls it makes, on a device by the owner.

## Sources

[Vivaldi 4.0 mail, calendar, feeds](https://liliputing.com/vivaldi-web-browser-gains-translation-mail-calendar-and-feed-reader-functions-in-version-4-0/) ·
[Vivaldi Mail 1.0](https://www.computerbase.de/2022-06/vivaldi-mail-1-0-final-release/) ·
[The strategy behind Dia's design](https://browsercompany.substack.com/p/the-strategy-behind-dias-design) ·
[Arc's lessons (ifanr)](https://www.ifanr.com/1625124) ·
[Why they killed Arc to build Dia](https://every.to/podcast/inside-the-browser-company-why-they-killed-arc-to-build-dia) ·
[Superhuman Cmd+K](https://help.superhuman.com/article/478-superhuman-command) ·
[Superhuman review (dev.to)](https://dev.to/pickuma/superhuman-review-is-a-premium-email-client-worth-it-for-developers-in-2026-5d81) ·
[Sunsama vs Akiflow](https://www.morgen.so/blog-posts/sunsama-vs-akiflow) ·
[Akiflow vs Sunsama review](https://dhruvirzala.com/?p=8469) ·
[Toggling cost](https://www.physicianleaders.org/articles/how-much-time-and-energy-do-we-waste-toggling-between-applications) ·
[Gloria Mark on interruptions](https://newsadvance.com/exclusive/article_3c20b021-323a-575d-8b5f-0cae56eeb67f.html) ·
[HEART framework](https://ixdf.org/literature/article/google-s-heart-framework-for-measuring-ux) ·
[Command palette patterns](https://www.saasui.design/blog/saas-search-command-palette-ux-patterns) ·
[Designing the command palette (Retool)](https://retool.com/blog/designing-the-command-palette)
