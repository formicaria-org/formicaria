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
