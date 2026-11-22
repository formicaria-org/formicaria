# One home that explains and connects everything (research, 2026-11-22)

**Asked by the owner:** *"one important aspect for usability that we are missing is a unifying UI …
the views we have now emerged from my need to show that the app can be pretty flexible, but it is not
clear at a glance when one opens the app how it works and what one can do; ideally there should be a
customizable general unifying interface and view that can link/connect to all the others. This is
beyond a single dependency like browser, we should consider any library that can fit. No code, just
research."*

**Builds on:**
- `workspace-design-2026-11-15.md` (its §3 surveyed Vivaldi, Arc/Dia, Superhuman, Sunsama/Akiflow and
  command palettes; not repeated here);
- the library model (a library of notes of kinds; views navigate kinds; sources acquire);
- `ui-audit-2026-11-22.md` (the first hour, measured).

The new ground: **apps whose home is built from kinds of objects**, **customizable dashboards**, and
**the libraries** that could build one.

## 1. The problem, in the audit's terms

A newcomer opens formicaria on **Notes → Timeline** (or the Library's shelves). Nothing on that screen
says formicaria also has:
- tasks, an agenda, food, fitness and health logs;
- music, videos, papers and saved web pages;
- discussions, collaboration and an assistant.

Each of those is a separate page, reached through a ▾ chooser or the new tab's buttons. The features
show the app is flexible; **no screen shows how they fit together**, and none can be shaped to the
person.

## 2. How others solve it

| App | The unifying screen | Customizable how | What to take |
|---|---|---|---|
| **Anytype** | A **sidebar of widgets**: Library, Bin, Favorites, Recent, and any Set/Query or Collection as a live list. **Object Types appear in the sidebar automatically once they hold something**, each listing its objects; draggable to reorder. On mobile, a gallery widget on the main screen | *Edit widgets*: add, remove, reorder; choose list or compact | **Kinds as the navigation, shown only when used**; any saved query becomes a widget |
| **Capacities** | **Object types in the left sidebar**; each type opens its **own dashboard** with customizable sections (untagged, not in a collection, any query or collection as a tab) | Sections per type; collections filter, sort and group (2026) | **Every kind has a dashboard of its own**: fm's feature pages, framed as kinds |
| **Notion** | *Home*: fixed widgets (recently visited, upcoming events, a database view); **hide only, no rearranging**. Separately, **dashboard views (2026)**: up to 12 widgets (tables, boards, calendars, charts, timelines), 4 a row, global filters | Home barely; dashboards fully | **A dashboard is a set of views side by side with one filter**; a home you cannot rearrange is a known complaint |
| **Obsidian** | **A homepage note** (Homepage plugin) holding **Bases code blocks**: live views written inside the note; *Dashboards* plugin for widget layouts | Edit the note | **The home is a note whose blocks are live views**: files-as-truth, and shareable as a template |
| **Home Assistant** | **Sections view**: a grid of sections holding cards, drag and drop; columns capped by screen width. Its makers found that **masonry, multi-screen and drag-and-drop cannot coexist**, and chose sections | Drag cards and sections; per-section theme | **Sections in a capped grid**, not a free canvas, so it fits a phone and a wide screen alike |

**What they share:**
1. **The kinds are the map**: what you can do is listed by kind, with what each holds.
2. **Any saved view can be placed on the home.**
3. **Customization is arranging blocks**, not designing pixels.
4. **A default that works before any customizing.**

## 3. Five shapes a unifying home could take

| Shape | For | Against | Fit with formicaria |
|---|---|---|---|
| **A. The Home note**: a note whose blocks are live views, notes and actions | Files-as-truth; versioned and synced by git; a template others can share (growth plan's gallery); edited with the editor people already use; the same note on the phone, one column | Needs *views embedded in notes* (fm embeds notes, not views); arranging by editing text is not obvious to everyone | **Highest.** Notes embed notes today (`render.ts`, `note-embed`); views are already declarative files (`*.view`). The missing piece is one embed kind |
| **B. A widget grid** (Home Assistant, Notion dashboards) | Drag and resize is instantly understood | Layout state is not a note unless stored as one; free 2-D layouts break on phones (Home Assistant's lesson) | Good *as the editing surface over A*: a list of blocks in sections, stored in the Home note |
| **C. Kinds as the navigation** (Anytype, Capacities) | Answers *"what can I do here?"* at a glance; mirrors the library model exactly | Alone, it is a menu, not a home | **Essential as the top of the home**: the kinds, each with what it holds and *＋ add* |
| **D. A docking workspace** (IDE panels: dockview, Lumino, Golden Layout) | Power users arrange many panes | Contradicts *one frame, minimal by default* (`decisions.md` 2026-11-17) and the novelty budget (Dia); Firefox already gives tabs | **Low.** Leave to the existing *tiled* option |
| **E. A map of the library** (node canvas: Svelte Flow, Excalidraw) | Shows how sources → notes → views connect; a good *explainer* | Graph homes are pretty and rarely used daily | **As a "How formicaria fits together" page**, linked from Help and the welcome, not the home |

## 4. Recommendation: one Home, made of a kinds strip and blocks, kept as a note

**What a person sees on first start**, on a computer, laid out as sections in a capped grid (one column
on a phone):

1. **What you can do here**: one row of the kinds, from `lib/features.ts` (Notes, Tasks, Agenda,
   Food, Fitness, Health, Music, Videos, Papers, Web pages, Discussions, Collaboration).
   - Each tile shows how many it holds and its last item, opens its page, and has **＋ add**: a new
     note, a meal, a song, a saved page.
   - A kind with nothing yet says in one line what it is for. *Show all* reveals the ones hidden.
2. **Getting started** (only until done or dismissed): five steps, each a real action:
   - write a note;
   - keep a web page;
   - log a meal or a workout;
   - add a song or a video;
   - connect your phone.

   Short, skippable and ticked by what the person actually did, read from the library itself.
   This is the research's pattern of an empty state as an invitation and a short checklist.
3. **Today**: the day's agenda and the tasks due, from the existing *agenda* renderer.
4. **Just in**: what arrived (saved pages, media, meetings from mail, imports): the library's
   *sources* made visible.
5. **Recent** and **Pinned**: the notes last opened, and what the person pinned.
6. **Any view or note** the person adds, from *Customize home → Add a block*: any `.view` (Menu,
   Training week, Playlists, a board), a single note, a tracker's today total, a playlist.

**Customizing:**
- **Customize home** switches to an edit mode: add a block from a list, drag to reorder, remove, and
  choose one or two columns for a section.
- The result is written to **a Home note in the vault** (`home.md`, Markdown, with one embed per
  block). It is versioned, synced, readable in any editor, and shareable as a template.
- Editing the note's text directly is the power-user path and gives the same result.
- **Reset to the default** is one press.

**Where it lives:**
- Home is the **start page and the new tab's first suggestion**.
- The ▾ chooser of every feature page gets *Home* first.
- On the phone, Home is the first tab.

**What it is not:**
- not a second Settings;
- not a command palette (still ruled out);
- not a canvas you must design before it is useful.

## 5. The libraries, for each piece (verified 2026-11-22)

| Piece | Candidate | License | Fit | Note |
|---|---|---|---|---|
| Live views inside a note | **none needed**: an embed kind in `render.ts` drawing an existing renderer | – | best | Obsidian's Bases code block is the model: the view's settings live in the note |
| Reordering blocks | **SortableJS** (vanilla) or **svelte-dnd-action**; or native HTML drag and drop | MIT | good | A list with sections needs no grid engine. *Licenses and current versions to be checked when chosen; not verified here* |
| A 2-D widget grid, if wanted later | **gridstack.js** 12.4.2 (Dec 2025), TypeScript, drag/resize/responsive; a Svelte 5 wrapper exists (v0.1.1, early) | MIT | possible | Only if sections prove too limiting; Home Assistant's lesson argues for sections first |
| Docking panes | **dockview-core** (vanilla TS, zero dependencies; floating, popout, serialization) | MIT, with a separate commercial *Enterprise* edition | low | Check which features are MIT-only; not for Home |
| The map of the library | **Svelte Flow 2** (`@xyflow/svelte`), by the React Flow team | MIT | good, for the explainer page | Bring your own layout; check Svelte 5 peer dependency |
| An optional guided tour | **driver.js** | MIT | good | **Not Shepherd.js or Intro.js**: both AGPL-3.0 with a paid licence, which does not suit an MIT app |
| Find-or-do | already built: Ctrl+K search (U1) | – | – | The home's search field is the same component |

Every dependency here would be a new front-end addition. Only the reordering library is needed for
the recommended Home, and native drag and drop may do.

## 6. What it rests on in formicaria today, and what is missing

| Needed | There today | Missing |
|---|---|---|
| The kinds, with counts and last items | `lib/features.ts` (the list); each feature's feed (`loadFeed`) | A cheap *summary* per kind (count, last item) in one request |
| Live views in a note | Views as declarative files (`crates/fm-app/src/shipped/*.view`), renderers (board, agenda, timeline, shelf, totals, trends, calendar, activity, discussions, search), note embeds (`render.ts`) | **A view embed** (`![](view:Menu)`, or a fenced block), drawn by the renderer the view names |
| A Home note | Notes, templates, the tutorials shelf | Where it lives (`home.md` at the vault's root, or a note tagged `home`) and its default content |
| Getting started, from real actions | The library itself (are there notes? a saved page? a log entry? a song? a paired device?) | One read answering those five questions |
| Arranging | The editor | The *Customize home* mode |

## 7. How it would be measured

The audit's harness, extended (`ci/usability-check.py`):
- **from a cold start, every kind reachable in one click** from Home;
- **the first note in under three actions**;
- **the checklist ticks** after each real action;
- **a block added, moved and removed survives a reload and a sync** (it is in `home.md`);
- the same Home on the phone (`phone-media-check`), one column.

Plus the audit's *first hour* rerun: Home should turn rows 6 (organise), 8 (own view: a view block)
and 19 (side by side: two blocks) toward ✅.

## 8. Decisions this needs (the owner's)

1. **Home replaces the Library's shelves as the start page**: it supersedes in part *"Home is the
   Library's shelves; tutorials are notes"* (2026-10-18). The shelves become a block on Home, and
   the tutorials a *Getting started* link.
2. **Views can be embedded in notes** (a new embed kind; `#ui` `#data`).
3. **The Home is a note in the vault** (`home.md`), so customizing is a file change that syncs.
4. **A kind with nothing yet**: shown with its one-line purpose, or hidden until used (Anytype hides;
   my recommendation is *shown, with a purpose line, and a "hide" per kind*, because hiding them
   is exactly what makes the app's range invisible today).
5. **The map of the library** as an explainer page (Svelte Flow), or not now.

## Sources

- Anytype, Sidebar & Widgets: https://doc.anytype.io/anytype-docs/basics/vault-and-key/customize-and-edit-the-sidebar ·
  widgets: https://doc.anytype.io/anytype/basics/sidebar/widgets · release notes: https://releases.any.org/desktop-0470
- Capacities, content types and dashboards: https://docs.capacities.io/reference/content-types ·
  https://capacities.io/whats-new/release-25 · https://docs.capacities.io/reference/user-interface
- Notion Home: https://www.notion.com/help/home-and-my-tasks · https://thomasjfrank.com/learn-notion/notion-home/ ·
  dashboards view: https://www.notion.com/sv/help/dashboards ·
  https://alternativeto.net/news/2026/3/notion-introduces-dashboard-views-to-turn-any-database-into-a-customizable-control-center
- Obsidian Bases in notes: https://help.obsidian.md/bases/create-base · Dashboards plugin:
  https://community.obsidian.md/plugins/dashboards · *My Home Base*: https://forum.obsidian.md/t/my-home-base-tutorial/108759
- Home Assistant sections view: https://home-assistant.io/blog/2024/03/04/dashboard-chapter-1 ·
  https://home-assistant.io/dashboards/sections/
- gridstack.js: https://www.jsdelivr.com/package/npm/gridstack · Svelte wrapper: https://sveltethemes.dev/mikolajbadyl/svelte-gridstack
- dockview: https://github.com/mathuo/dockview · https://www.npmjs.com/package/dockview-core
- Svelte Flow: https://svelteflow.dev · https://github.com/xyflow/xyflow
- Tours' licenses: https://npm-compare.com/driver.js,intro.js,shepherd.js ·
  Shepherd's licence file: https://unpkg.com/angular-shepherd@22.0.0/LICENSE.md
- Onboarding and empty states (vendor blogs, read with care): https://www.appcues.com/blog/user-onboarding-ui-ux-patterns ·
  https://userpilot.com/blog/progressive-disclosure/

## 9. Beyond a home: bigger changes (2026-11-22, later)

**The owner, on §4:** *"I think we need to go beyond using a note as a home, we need to think of possible
bigger changes."* A Home note fixes the front door, not the building.

**The building's real problem:** formicaria is a set of feature pages, each with its own controls and
its own way of listing, adding and opening. Each taught separately, so a person learns ten small apps.
The library model says something else: **one library of notes of kinds; views navigate the kinds**
(memory, 2026-11-16). The interface has not caught up with the model.

### 9.1 Six bigger directions

| # | Direction | What changes | Precedent | For | Against |
|---|---|---|---|---|---|
| D1 | **Object-first shell** (object-oriented UX) | One layout for everything: **the kinds** → **a view of the chosen kind** (list, board, calendar, table, gallery, or its special view) → **the item**. Every kind gets the **same verbs**: new, find, filter, group, view as…, open, link, export, history. Music's deck and Food's totals become *views of their kind*, not separate apps | Sophia Prater's OOUX (*"objects first, then actions"*); Capacities; Anytype; Finder and Mail | Learn once, use everywhere. It is the library model, made visible | The biggest rebuild. Special screens must fit a common frame |
| D2 | **Kinds as definitions the person can make** | A kind is a file (fields, icon, default views, actions, what *＋* asks). Food, Fitness, Music and Papers become **shipped kinds** a person can copy and change; a person adds *Book*, *Plant*, *Patient*. Forms and views are generated from the definition | Tana's supertags (a tag gives a node its fields; all tagged nodes are a collection with views); Anytype types; Ink & Switch, *Malleable software* (2025) | The unifying mechanism under D1: nothing is special-cased. The most customizable shape there is | Generated forms must still feel designed. Reverses *no view authoring* in spirit |
| D3 | **Spaces**: areas of a life | *Study*, *Health*, *Home*, *Lab*: each a chosen set of kinds, views and pinned notes, with its own Home; one gesture or key switches. Vaults (personal, lab, shared paper) fit naturally as a space's source | Arc's Spaces, now Dia's Profiles: what users missed most when they were gone | The app's range, cut to what matters in each context | Arc's makers found ambitious structure a barrier. Spaces must be optional, with one default space |
| D4 | **Time as the spine** | **Today** is the default view: what came in, what is planned, what is due, what was logged, what was written, on one line of time; zoom to week and month | fm's own Timeline and Agenda; Sunsama, Akiflow (§3 of the workspace research) | Everything in fm has a time; a day is how people think | Not everything is about today (a library, a paper). Good as a view, risky as the only door |
| D5 | **A spatial canvas** | The library as boards of cards arranged by hand | Heptabase (reviews: strong for research, a *steep learning curve*, impractical on a phone); fm's own Excalidraw boards | Shows how ideas connect | **Not as the shell**: learning curve, phone. As a kind of view, it exists already |
| D6 | **Assistant-first door** | One field, *"Ask or do"*, answering with views, creating, capturing, finding | Dia went AI-first; Superhuman's Cmd+K | One door for everything | A local model's limits; an answer cannot replace seeing your library. As part of Ctrl+K, yes |

### 9.2 The recommendation: one shell, built on kinds, cut by spaces, opening on today

A synthesis of D1 to D4, with D5 and D6 as tools inside it:

1. **One shell, the same on every device.**
   - **Where you go:** a rail on a computer, the bottom bar on a phone. It holds the **space** (one
     by default), then **Home**, **Today**, **the kinds of that space** (each with ＋), and **Find
     or do** (Ctrl+K: search, create, the assistant).
   - **What you see:** the chosen view of the chosen kind.
   - **The item:** it opens in place, or beside on a wide screen.
2. **Kinds are definitions (D2)**, and **every kind has the same verbs and the same view switcher
   (D1)**. A kind's special view (Music's deck, Food's totals, the Agenda) is its default view, not
   its own app. *Make your own view* is a verb, not a feature.
3. **Spaces (D3) are optional.** One space, *Everything*, until the person makes another. A space is
   a saved choice of kinds, views and pins, so it costs nothing to the model.
4. **Today (D4)** is a view every space has; **Home** (§4) becomes the space's first view, with the
   kinds strip, getting-started, today, just-in, recent, pinned and added blocks.
5. **Web pages stay where browsing is**: Firefox's tabs, hidden behind fm's frame as now. An open
   note or view can still become a tab of its own.

**Why this, rather than any one direction:** it is the library model drawn as an interface:
- **kinds** are what the library holds;
- **views** navigate them;
- **sources** fill them (*Just in*);
- **spaces** are how a person cuts the library;
- **today** is when.

Each piece already half exists:
- the kinds list (`lib/features.ts`);
- views as files (`*.view`);
- the renderers;
- Timeline and Agenda;
- vaults.

What changes is that they stop being separate pages.

### 9.3 What it collides with (the owner decides; no arguing it down)

| Standing ruling | The collision | Needed |
|---|---|---|
| *One frame* (2026-11-17): one row (≡ · tabs · +), *a tab is one feature*, **no left panel, no second bar** | The shell puts navigation in a rail (computer) and makes one fm view the place you move within, rather than a tab per feature | A **superseding** entry: the rail is fm's frame. Tabs remain for web pages and anything opened out |
| *No view authoring* (`features.md`) | *Make your own view* becomes a verb on every kind | **Reversal** (already listed in the audit, U3) |
| *Home is the Library's shelves* (2026-10-18) | Home becomes the space's first view; the shelves a block | **Superseded in part** (§8.1) |
| Kinds as code (each feature its own page and logic) | Kinds become definitions, the shipped ones included | A `#data` `#seams` entry: the kind file's format, and what stays code (a special renderer such as the music deck) |

### 9.4 How to get there without breaking a release

| Phase | What | Shown by |
|---|---|---|
| **P0** | The decisions above. **A clickable prototype** of the shell: static pages, outside the repo (`spikes/`), with the audit's 20 tasks walked on it by the same harness | the first-hour score on the prototype against today's (7 ✅ · 6 ⚠ · 7 ❌) |
| **P1** | **The kind registry** (workspace design §13.2): each kind declared once, with its verbs and views, the shipped ones written as definitions | every feature page drawn from its kind; no change a person sees yet |
| **P2** | **The shell**: rail or bar, Home and Today, Find or do, the common view switcher and verbs | the audit's rows 3–12 and 17–19 |
| **P3** | **Make your own view** as a verb; **Spaces** | rows 6, 8 and 19 |
| **P4** | **Kinds a person defines** | a new kind made, used, synced and shown on the phone, in `usability-check` and `phone-media-check` |

Each phase is a release through the full gate, and the first-hour audit is rerun at each. The
Settings plan (`settings-plan-2026-11-21.md`) fits inside P2: Settings becomes one more place in the
shell, not a card in a tab.

### 9.5 Libraries, revisited for the shell

Nothing in §5 changes. The shell needs **no new framework**: it is Svelte components over the existing
renderers. A rail with drag-to-reorder kinds uses the same small reordering library as Home.
Generated forms for kinds need none: they are fields from a definition, drawn by components fm
already has (`Details`, the property editors).

### Sources (§9)

- OOUX: https://alistapart.com/article/object-oriented-ux/ · https://blog.logrocket.com/ux-design/object-oriented-ux-ooux/
- Tana supertags: https://tana.inc/docs/supertags · https://tana.inc/articles/intro-to-supertags-search-nodes-and-views
- Ink & Switch, *Malleable software* (June 2025): https://www.inkandswitch.com/essay/malleable-software/ ·
  https://simonwillison.net/2025/Jun/11/malleable-software
- Heptabase reviews (anecdotal): https://dev.to/pickuma/heptabase-review-visual-note-taking-for-thinking-through-problems-5daa ·
  https://thesweetsetup.com/a-first-look-at-heptabase-a-pkm-app-for-research-and-learning/
- Arc → Dia: https://browsercompany.substack.com/p/the-strategy-behind-dias-design ·
  https://browsercompany.substack.com/p/designer-roundtable-building-swipeable ·
  https://tidbits.com/2025/06/20/dia-browser-debuts-with-contextual-ai-chat-but-arc-users-feel-left-behind/

## 10. What it takes: the plan from today's fm to the shell (2026-11-22, night)

**Asked:** *"what does it take to change fm to this? Only research and plan"*, after the prototype
(`~/formicaria-org/spikes/shell-prototype/index.html`, outside the repo). Sizes are relative (S, M,
L, XL) and counted in releases through the full gate; no dates.

### 10.1 Where fm is today (measured 2026-11-22)

| Part | Today | Size |
|---|---|---|
| **Two frames** | `ui/src/App.svelte`: the phone, a paired tablet, another browser; its bar `TabBar.svelte`. `ui/src/feature/Feature.svelte`: the computer's pages in formicaria's browser, one feature per tab, steered by the kit (`browser/kit/formicaria.cfg`) | 5,385 + 408 · 545 · 833 lines |
| **Per-feature screens** | `Pane.svelte`, `TrackerPane.svelte` (Food, Fitness, Health), `lib/music/` and `lib/video/` (14 files), `NotePanel.svelte`, `SettingsPanel.svelte` | 1,036 · 999 · — · 5,268 · 3,604 lines |
| **Kinds, twice** | Server: `crates/fm-app/src/kinds.rs` (`KINDS`: key, label, mark, views, sources, place, hand-offs; `kind_key` on every note; the `kinds` command). Interface: `ui/src/lib/features.ts` (ten features, their pages). **The two lists do not know each other** | 237 · 57 lines |
| **Views** | `.view` files (`views.rs`), renderers Board, Agenda, Timeline, Search, Totals; Gallery removed; components also for Calendar, Shelf, Trends, Activity, Discussions. **No Table, no Gallery** | 1,410 lines; 11 renderer components |
| **Tests bound to the frames** | `usability-check.py`: 52 `/f/` addresses. `phone-media-check.py`: 25 selectors of the phone's bar. UI tests: 51 of 174 files render `App` or `Feature` | — |

**The lesson to carry:** the move to the one frame (2026-11-17) silently dropped search, undo and
*Recently deleted* on the computer (`ui-audit-2026-11-22.md`). A frame change must come with **a
parity list**: every capability of the old frame, each tested in the new one before the old one goes.

### 10.2 The target, as parts

| Part | What it is | Built from |
|---|---|---|
| **Kind definitions** | One file per kind, the shipped ones in `crates/fm-app/src/shipped/kinds/*.kind` (YAML, like `.view`): key, label, icon, purpose line, how it is recognised (today's `KINDS` markers), its views (default first), its verbs, what ＋ asks (fields), its summary. Read by the server, served by `kinds`. Later a person's own in `<vault>/kinds/` (P4) | `kinds.rs` + `features.ts`, merged |
| **`kind_summary`** | One request: per kind, a count and the last item, from the index (cheap, low memory) | the FTS index, `kind_key` |
| **`KindView.svelte`** | The view switcher and the verbs (new, find, filter, group, sort, *make a view*) over any view of a kind | `Pane.svelte`'s feed loading; the renderers |
| **New generic views** | **List** (from Timeline's compact list and Search), **Table** (new), **Gallery** (back, thumbnails exist) | existing renderers, `Card.svelte` |
| **Special views** | A kind's own screen as one of its views: Music's player, Food's day and totals, Fitness's week, Health's cards, Agenda, Shelves, Videos | `MusicScreen`, `TrackerPane`, `VideoScreen`, `Calendar`, `Shelf` wrapped, not rewritten |
| **`Shell.svelte`** | One frame, all devices. A rail on a computer, the bottom bar on a phone, from one model (space, Home, Today, kinds, Find or do, Settings). Routes `/s/<space>/<kind>[/<view>][?item=<id>]`, `/s/home`, `/s/today`, `/s/settings`. Old `/f/…` addresses redirect | `Feature.svelte`, `TabBar.svelte`, `FindNotes`, `UndoDelete` |
| **The item, beside or in place** | `NotePanel` in a side slot on a wide screen, full on a narrow one; ⋯ with U2's Export and History | `Pane.svelte`'s note path |
| **Home and Today** | Home: kinds strip, getting started, blocks (§4), **customizable from the first shell release** (the owner, 2026-11-22: *"the initial home screen must be customizable"*): *Customize home* adds a block (any view, a note, a tracker's total, a playlist, Today, Just in, Recent, Pinned), drags it, removes it, and sets a section to one or two columns. It is saved to `home.md` in the vault (so it syncs and versions), with *Reset to the default*. This needs **views embedded in notes** (§8.2). Today: a composed feed (agenda, due, logged, arrived, written) | agenda/timeline commands, composed server-side as one `today` read; `render.ts`'s embeds plus a view embed |
| **Settings in the shell** | A place with its categories (`settings-plan-2026-11-21.md`), no longer a card in a tab | `SettingsPanel.svelte`, regrouped |
| **The kit** | One pinned fm tab, the shell; web pages in their own tabs as now; the window's ⚙ opens `/s/settings` in the shell; `keepToOurTabs` keeps the shell tab; *Open as a tab* still opens an item alone | `formicaria.cfg` (`notesTab`, `keepToOurTabs`, the + page) |
| **The phone** | The shell's bottom bar replaces `TabBar`'s feature tabs. Open web pages and items stay reachable through one *open* switcher (today's roller) | `App.svelte`'s phone path, `TabBar.svelte` |

### 10.3 The phases

Each phase is a release (or two) through the full gate, with the first-hour audit rerun.

| Phase | What ships | A person sees | Size | Gate adds | Decision entry |
|---|---|---|---|---|---|
| **P0** ✓ prototype · **P0b** | The prototype, then the audit's 20 tasks walked on it by the harness; the owner's review; changes to the design | nothing in fm | S | the walk's score on the prototype | none |
| **P1** | **Kinds as definitions**: shipped `.kind` files, merged from `kinds.rs` and `features.ts`; `kinds` returns them; `features.ts` reads them; `kind_summary` | nothing changes yet | M · 1 | Rust tests per kind file; every note recognised as before (a differential test on the tutorials vault) | the `.kind` format (`#data` `#seams`) |
| **P2a** | **One view switcher and the same verbs** on every feature page, inside today's frame: List, Table, Gallery and each kind's special view; U2's ⋯, `[[`, badge rule, Import/Export/History land here | every kind works the same way; ⋯ on every item | L · 1–2 | a persona per verb on three kinds; the audit's rows 5–12, 16–17 | U2's entry already written; Table/Gallery |
| **P2b** | **The shell on the computer**: rail, **a customizable Home** (add, move, remove blocks; saved to `home.md`; reset), Today, item beside, Find or do, Settings as a place (the Settings plan folds in); the kit's one shell tab; `/f/` redirects | the new interface | XL · 2 | **the parity list** checked in `usability-check`; a block added, moved and removed survives a reload and a sync; `browser-check`'s memory budget; the audit rerun | the shell entry (written), the kit's tab rules, view embeds in notes, `home.md` |
| **P2c** | **The shell on the phone**: bottom bar, the *open* switcher; `App.svelte`'s phone path retired; `phone-media-check` moved to the shell's selectors | the same interface on the phone | L · 1–2 | the phone's parity list; the music, media and privacy cases unchanged in what they prove | the phone's bar (supersedes parts of 2026-11-17) |
| **P2d** | **The old frames removed**: `App.svelte`'s desktop and tablet chrome, `Feature.svelte`'s strip, `TabBar`'s feature tabs. Thousands of lines out | nothing (only if the parity lists held) | M · 1 | the parity lists still green with the old code gone | none (follows P2b/P2c) |
| **P3** | **Make a view** as a verb (saved as a `.view` of the kind); **Spaces** (a saved choice of kinds, views, pins; one default) | own views; spaces | L · 1–2 | make, use, sync, reload a view; switch spaces | spaces' file and where it lives |
| **P4** | **A person's own kinds**: a kind made from a form, its fields, views and ＋, synced, on the phone | new kinds | L · 1–2 | make a kind, add items, see it on the phone | person-made kinds (`#data`) |

**In total: about 9 to 12 releases**, with P2b the largest and riskiest. U2 is no longer a separate
step: it lands in P2a. The Settings plan lands in P2b.

### 10.4 What stays the same

- **The server's job:** notes as files, the index, git, sync, the commands. It gains `kind_summary`,
  a composed `today` read and the kind loader. It stays a classic, low-memory server, with
  customization in the front (memory: *simple server, rich front*).
- **The renderers:** wrapped, not rewritten.
- **The note page** (`NotePanel`): moved into a slot, not redesigned. Its options move to ⋯ (U2).
- **The browser engine and the kit's other work:** privacy, Save media, updates.

### 10.5 Risks, and what answers each

| Risk | Answer |
|---|---|
| **Losing capabilities again** in the frame change | The parity list, written in P0b from both frames, each item a test before P2d deletes anything |
| **Scope growing** while it is built | One phase per release. Nothing from P3 or P4 enters P2 |
| **Special screens not fitting** a common frame | They are views, wrapped whole. The frame only gives them the switcher and the verbs around them |
| **The phone's memory** with a richer shell | `phone-media-check` measures memory per case already; the bottom bar is the same size as today's |
| **Novelty**: a new interface people must learn | Mainstream placements throughout (rail, ⋯, Ctrl+K, a view switcher); the audit's conventions as the test |
| **Tests rewritten wholesale** | `/f/` addresses redirect during the change, so old personas keep running while new ones are added; the phone's selectors move in one phase (P2c) |
| **The owner's review time** | A prototype or a screenshot set before each visible phase (P2a, P2b, P2c) |

### 10.6 The owner's answers (2026-11-22, night)

- **0.60.9 ships now** (the phone page fixes); the shell is the release after.
- **The owner refines the plan first**, and building waits for that.
- **"Full implementation" before the next release = P1 to P2d**: the whole shell on computer and
  phone, U2 and the Settings plan inside it, the old frames removed once the parity list passes. P3
  and P4 follow. Each phase is still taken through the full gate locally before the next begins;
  only the last is released.
- **Home must be customizable** from the first shell release (above).

### 10.7 The first steps

1. **P0b:**
   - walk the audit's 20 tasks on the prototype with the harness, and compare scores;
   - write the parity list from both frames;
   - the owner looks at the prototype and says what to change.
2. **P1:** kinds as definitions. Invisible, so safe to ship early, and everything after it reads them.
3. **Decision entries** as each phase starts (the `.kind` format first).

## 11. P1 in detail: kinds as definitions (design, 2026-11-22, night)

**What a rail entry is.** Two lists exist today, and they mean different things:
- The server's `KINDS` (`kinds.rs`) are **types of note**, recognised from markers: study note, web
  page, paper, recipe, log entry, song, video, meeting, conversation, discussion, proposal, file.
  Each note has exactly one (`kind_key`).
- The interface's `FEATURES` (`features.ts`) are **sections a person navigates**: Notes, Tasks,
  Agenda, Food, Fitness, Health, Music, Videos, Discussions, Collaboration. They are made of views
  (`BUILTIN_SPACES`, shipped `.view` groups).

A rail entry is the second, so a `.kind` file defines **a section chosen by a rule over notes**,
with its views and its ＋. *Tasks* is any note with a status or a due date, whatever its type; *Food*
is food entries and recipes. The server's note types stay as they are: the rule may name them.

**The file** (`crates/fm-app/src/shipped/kinds/<key>.kind`, YAML, embedded like the shipped
`.view`s; later a person's own in `<vault>/kinds/`, P4):

```yaml
key: music
label: Music
icon: play                 # an Icon.svelte name
purpose: Songs, albums and playlists you keep
order: 70                  # its place in the rail
match:                     # which notes belong: the same filter language as a .view
  - prop: kind_key
    eq: song
views:                     # the view switcher, the default first
  - special: music          # a screen the app ships (MusicScreen); only shipped kinds may name one
  - view: Songs             # a shipped or saved .view, by name
  - renderer: table
    columns: [title, artist, album, duration_s]
  - renderer: gallery
new:                       # what ＋ creates
  props: { log: music }
  asks: [title, artist]     # the fields the quick form shows
```

The first set of shipped kinds, mapping today's features one to one so P1 changes nothing a person
sees:
- **Notes:** study notes, web pages and papers. Shelves and Timeline are its views. Papers and Web
  pages may later become kinds of their own: a file each, no code.
- **Tasks:** status or due. Board and Agenda.
- **Food:** `log: food`, recipes. Its shipped views.
- **Fitness**, **Health**, **Music**, **Videos:** the same pattern.
- **Discussions**, **Collaboration:** their pages as special views.

*Agenda* stays a view (of Tasks, and later the heart of Today).

**What changes in code (P1, invisible):**
1. `kinds.rs` gains the loader. It parses the shipped `.kind` files with the parser `.view` uses
   (`views.rs`), keeps `kind_of` as is, and adds the sections to the `kinds` command's answer
   (`{ types: [...], sections: [...] }`, the old field kept for Myrme).
2. **`kind_summary`**: per section, how many notes match and the latest one. One pass over the
   index through `fm-query` (a pure filter, the seam respected). Bounded and cheap.
3. `ui/src/lib/features.ts` becomes a reader of `kinds.sections`, keeping its exported shape so
   `NewTab`, the chooser and `TabBar` change by one import. `BUILTIN_SPACES` and `groupsOf` give
   their views to the sections.
4. **Tests:**
   - each shipped `.kind` parses;
   - every note of the tutorials vault lands in the same feature as before (a differential test);
   - `kind_summary` counts match a direct count;
   - the UI's features list is unchanged.

**Decision entry at P1:** the `.kind` format, and that a section is a rule over notes, not a new
property written into them (`#data` `#seams`).
