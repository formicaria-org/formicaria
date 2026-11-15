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
