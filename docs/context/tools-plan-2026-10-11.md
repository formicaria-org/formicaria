# Deterministic tools — a general plan, with food tracking as the first case (proposal, 2026-10-11)

> **A proposal, not a ruling.** Nothing here is decided or built. It was written from the code and
> `decisions.md` as they stood on 2026-10-11; where it disagrees with the code, the code wins. If the
> owner adopts it, the rulings go in `decisions.md` (`#agent`, `#data`, `#seams`) and this file
> becomes their receipt.

## The owner's direction

> *"We can define deterministic tools in fm and allow agents to use them, for instance the calorie
> tracking and computations. We need a very general plan that fits well with fm and uses its
> constructs, so an agent using a tool looks easy… this needs to coherently sit with the other
> design choices and long-term maintainability and clarity."*

Food tracking (a Lifesum-like log, totals and a day screen) is the motivating case. The plan must
not be about food.

## The claim: a tool is not a new construct

formicaria already has deterministic tools; it has never called them that.

| Already shipped | What it is |
|---|---|
| `create_paper` / `paper_bibtex` | A pasted citation becomes a note, offline, in Rust |
| `fm_core::events`, `fm_core::calendar` | A foreign format becomes notes, with the assistant off |
| `fm_core::import` | A Logseq or Obsidian graph becomes notes |
| The meeting pass | The model points at a sentence; Rust reads the date |

Each is the same shape, so the definition is taken from them and nothing is invented:

**A tool is a deterministic function in a non-agent crate, reached through one `dispatch` command,
whose only effect on a vault is notes and properties.**

Consequences that follow from existing rulings, with no new rule needed:

- **Everyone calls it the same way.** The UI, `fm-cli`, curl and the assistant all arrive at
  `fm_app::dispatch`. The assistant already does: `fm-agent-run/src/fmserve.rs` posts to
  `/api/<cmd>`. So "an agent using a tool" is a command call it can already make.
- **It works with the assistant off, and on iOS.** *Anything the notes-only build must be able to do
  cannot live in an agent crate* (2026-09-12). A tool lives in `fm-core`, beside `import.rs` and
  `events.rs`.
- **No network at call time.** *No HTTP client enters `fm-core` or `fm-app`.* Anything a tool needs
  from outside arrives beforehand through `fm-fetch`.
- **Testable like the rest.** Pure input to output, a fixture, no model.

## Who picks the tool: the three triggers that already exist

Selection stays deterministic (the 2026-10-10 narrowed scope). No model chooses a tool.

| Trigger | Today | A tool plugs in as |
|---|---|---|
| **A command** | the closed `Command` enum (`Chat`, `Propose`, `Research`, `Read`) | one more variant, or a UI action that calls the dispatch command directly |
| **A file's type** | MIME → reader (`resolve_blobs`, the planned `[specialists]` table) | a row |
| **A schedule** | the meeting pass; the planned Job registry | a job |

A tool that fits none of the three is not ready to be added.

## What the model may do around a tool

Only what the meeting pass taught: **the model points, Rust reads and computes.**

- The model's one job is to turn a person's words (or a transcript of a photo) into *pointers*: the
  phrases in the text that name the tool's inputs.
- Rust checks each pointer against the text it came from, resolves it through the tool, and does
  all arithmetic. A value the model supplies that is not in the text is dropped, as a date is today.
- **A person's own action writes directly; the assistant's result is a proposal.** Unchanged.

## What is actually new: three additions, each general

| Addition | Why it is needed | Where it goes | Serves beyond food |
|---|---|---|---|
| **1. A sum over a group** | The query engine groups, sorts and paginates but cannot total. This is `outstanding.md` §2.5b, the owner's "aggregate by tag" | `fm-query`, pure; no seam is touched | expenses, hours, training volume, reading |
| **2. A totals renderer** | A day screen is "group by day, sum these number properties, compare with an optional target". The `Renderer` enum is closed and has no such member | one new `Renderer` variant and one generic component; **labels and targets come from the `.view` file**, so the literal-free rule holds | any tracker |
| **3. A reference pack** | A tool may need a read-only table too large and too foreign to be notes | fetched by `fm-fetch`, checksum-verified, into `<config>/formicaria/tools`, listed in a catalogue that states size and licence before it spends disk, never downloaded unasked; read by `fm-core` (which already links SQLite) | units, exercise energy costs, any lookup table |

Everything else is reuse: notes, tags, integer properties, `.view` files, `dispatch`, proposals,
`fm-fetch`, the model catalogue's first-enable pattern.

**A fourth, smaller question — numbers.** `PropertyValue` has `Int` and no decimal. The cheap answer
is integers in a fixed small unit (kcal; tenths of a gram), which needs no change to the model, the
merge driver or the index. A decimal type is a wider change and should be argued separately.

## Three rules that keep it clear over time

1. **The note carries its own numbers.** A pack is reference data, like a model: outside the vault,
   replaceable, absent on some devices. A logged entry copies the values it used into its own
   frontmatter and names the pack version. A note that only pointed into a pack would break
   *files-as-truth*.
2. **High-volume entries stay out of the planning views.** A few thousand log entries a year must
   not fill the board, the feed and search. The papers plan already names this disease and its
   cure (`thread::notes_base`); a tool that produces many small notes declares itself the same way.
3. **A tool earns its place by a short test.** Deterministic; outside the agent crates; one
   command; writes only notes and properties; no network when called; at least one of the three
   triggers; a fixture test. The `dispatch` match is the list of tools. No registry, no plugin
   system, no second table.

## Food tracking, expressed in those constructs

| Piece | Construct |
|---|---|
| A logged meal | a note tagged for it, integer properties, a `start` stamp, an optional photo blob |
| Food facts | a reference pack built from public-domain generic foods (USDA Foundation, SR Legacy, survey foods: tens of MB) |
| Search and portion arithmetic | one or two commands in `fm-core` |
| The day screen | a `.view` file shipped with the feature, drawn by the totals renderer |
| "Two eggs and toast" | the assistant points at items and amounts; Rust resolves and computes; a proposal |
| A photographed label | the existing image reader, then the same path |

**Left out on purpose:** barcodes and branded products (a ~3 GB table, or one network request per
scan, which is a different decision), wearables, and any nutrition value recalled by a model.

## The front end comes first (owner, 2026-10-11)

> *"It all depends on the usability of the front end, that's why I want to focus on the front end."*

So the order below is preceded by a step 0 that needs no Rust: **prototype the screens against the
in-memory backend** (`ui/src/lib/mock.ts`, which `pnpm dev` and Vitest already use) with invented
foods and entries, at phone width, and let that decide what the commands must return. A tracker
nobody can log into in a few seconds is not rescued by a correct sum.

**Scope for now (owner, same day): front end only, phone first, and it must still work on the other
systems.** That is reachable with almost no Rust, because a view row (`ObjectMeta`) already carries
every property in `props`: a day screen can total a day's few dozen rows in the UI, an entry is
`capture` plus `set_property`, and *recent* is the same rows. What a front-end-only pass cannot do:
a `.view` naming a new renderer (the `Renderer` enum is closed, in Rust), a `group:` key reaching
the UI (`ViewInfo` does not carry one), a food search, and totals over long spans, where summing in
the UI stops being honest. Phone first means: controls within thumb reach, no hover, the `--safe-*`
insets, `dvh`-bounded sheets, and a check on the device, since the phone runs the UI over IPC and
not HTTP (`asPhone()` in the harness covers the branches, not the feel).

The flows that decide it, in order of weight:

| Flow | What has to be true | fm construct it leans on |
|---|---|---|
| **Log something** | a few taps, one hand, most entries are repeats — so *recent and frequent* come first, from the person's own notes, before any search of a pack | `capture`; a sheet that joins `LAYERS` so Back closes it; the Android keyboard fix of 2026-10-08 (unverified on a device) |
| **See the day** | total against target at a glance, entries under their meal, move between days | the totals renderer; the one-row chrome and `ViewControls` |
| **Correct an entry** | tap it, change the amount, the total follows; a mistake is undoable | `set_property` through `undo.svelte.ts` |
| **Accept what the assistant proposes** | one tap per item, not a diff to read | `ProposalReview` is built for prose edits and is probably the wrong shape here |
| **See a trend** | a week of totals | the same renderer at a coarser grouping |

What the prototype should answer before anything is built underneath it: whether an entry is a note
per meal or a line in a day note (the screen looks the same; editing does not), where logging starts
from (the day screen, or a control always in reach), and how a target is set.

## Grouped views (owner, 2026-10-11)

> *"We might want to add the concept of grouped views: a person from the main view might go to
> another one, like one with graphs and progress assessment, or another where they track their
> fitness regimes, and anything we add."*

**The smallest form that fits:** a group is one optional key in a `.view` file (`group: Food`).
Views are flat files in `<vault>/views/*.view` today and `list_views` already reports each one; it
would report the group too, and the chrome would draw views under their group. No new file type, no
folder walk, and a group travels over git exactly as a view does. The built-in board, agenda,
timeline and search form the default group.

| In the chrome | Desktop (side panel) | Phone (bottom bar) |
|---|---|---|
| Groups | a heading per group, its views beneath | one switcher; the bar shows the current group's views as tabs |
| Moving between them | re-points the single window (`focusOrOpen`) | the same; Back returns to the previous group and view |

**What a group must not become without a ruling.** Three things attach themselves to it naturally,
and each is a step from "a label on views" towards "an app inside the app":

- **A shared filter** (every view in *Food* shows only food entries, and the default group leaves
  them out). This is also the cure for the volume problem above, so it is the one most worth having.
- **Its own "add" action** (in *Food* the add control logs a meal, with that group's tag and
  fields). The logging flow needs this, and it needs a home: a key in a view file, or a small file
  of its own.
- **Its own commands** (the tool behind it).

**Tensions with standing rulings:** view authoring was withdrawn from the UI on 2026-08-31 and the
owner said on 2026-08-30 to stop deepening view personalisation, so groups arrive as shipped or
synced files and are not built in the app; and the phone's bottom bar holds only a handful of
entries, so a group with many views needs an overflow.

## Order — each step ships and is useful alone

1. **The sum and the totals renderer.** Closes §2.5b. A person can already log by hand and see a
   day total. No pack, no assistant.
2. **The reference pack mechanism and the food commands**, with a quick-entry action in the UI.
   Still no assistant.
3. **The assistant's part**: a command that turns a sentence into a proposal through the same
   commands.

## What the owner must rule on before any code

| Question | Why it cannot be skipped |
|---|---|
| Is "a tool" recorded as a definition in `decisions.md`? | It is the one sentence every later addition is checked against |
| Does a reference pack ship? | It widens what the app downloads and spends disk on (the four questions, #4) |
| A new `Renderer` variant inside the notebook, or a separate app as ruled for papers? | Papers needed a PDF reader; this needs only a generic component. The plan argues *inside*, but the papers ruling is the precedent to distinguish from |
| May a proposal set number properties? | A proposal's property allowlist is `start`/`due`/`location` and tags today (*read before widening `apply_props`' allowlist*) |
| How is the totals view asked for without a query builder? | §2.5b: a builder was rejected twice. Shipping the `.view` with the feature avoids the question for food, not in general |
| Integers in small units, or a decimal type? | Decided once, for every tracker |

## Not verified

- Pack sizes are read from the USDA download page's table, whose columns were hard to parse.
- Nothing was measured on the phone: pack size on disk, search speed, or the model's accuracy at
  pointing at foods and amounts.
- Whether Lifesum keeps its food table on the device was not confirmed.
