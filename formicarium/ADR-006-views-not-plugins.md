# ADR-006: Views are core. There is no plugin API.

**Status:** Proposed
**Date:** 2026-07-14
**Depends on:** ADR-001 (one model, many views)

---

## Context

Kanban and calendar *feel* like plugins. Every instinct says "make the views pluggable." Both baseline competitors tried exactly that, and both walked it back.

---

## The evidence: two projects, two architectures, one destination

### Obsidian ran the experiment twice

**Attempt 1 — kanban as a standalone plugin.**
The Kanban plugin stores boards in **its own markdown format**: each card is a line in a dedicated file.
https://medium.com/@geetduggal/tech-habits-lists-in-obsidian-kanban-vs-obsidian-bases-f613b1673d56

Excellent UX. And a **silo** — cards in a Kanban file are not the same objects as notes. You cannot query across them. The board is a separate world with a separate format.

**Attempt 2 — Bases as a core plugin.**
A database view over YAML frontmatter. Views over shared properties. Shipped in 1.9.0.
https://alternativeto.net/news/2025/5/obsidian-1-9-0-introduces-bases-plugin-for-database-style-note-management

And then the kanban problem returned immediately, because Bases shipped without a board view:
https://forum.obsidian.md/t/bases-kanban-view/101593

> Add a kanban view to Bases, grouping records by a status field into draggable columns — similar to the Projects plugin, which could serve as a reference. **Current workaround: using the Projects plugin when a kanban layout is needed, but the plugin is no longer maintained.**

**The plugin people depended on for kanban died.** That is plugin rot, and it is precisely what a plugin ecosystem buys you.

**Attempt 3 — the community rebuilt it, correctly, as a generic renderer.**
https://community.obsidian.md/plugins/kanban-bases-view

> A kanban-style drag-and-drop view for Bases. **Dynamic column generation: select any property from your base to generate kanban columns automatically.** Drag and drop moves cards between columns and updates the frontmatter. Optional swimlanes group the board by a second property.

They independently arrived at **the generic board renderer** — "group a query by any enum property" — not a kanban feature. This is the design this ADR mandates, arrived at by convergent evolution.

**Attempt 4 — Obsidian is pulling it into core.**
https://www.xda-developers.com/bases-plugins-you-should-be-using/

> According to the roadmap, Bases will be receiving calendar and Kanban views in the coming months.

### Logseq arrived at the same place from the opposite direction

The DB rewrite made **Kanban View, Calendar View and Gallery View first-class core views** (see ADR-001 research). In the file version, kanban was a query hack and a third-party plugin. In the DB version it is core.

### The verdict

**Two projects. Different architectures. Different failure modes. Identical destination.**

Views over the shared data model belong in **core**. A view with its own storage format is a silo. A view delivered as a third-party plugin is a dependency that will be abandoned.

Even the community's own rule agrees:
https://practicalpkm.com/building-a-content-calendar-in-obsidian-bases/

> As a general rule, if you can pull off what you want to do using core functionality, you should.

---

## Decision

> **1. Kanban, calendar, agenda, gallery and timeline are core views. Not plugins.**
>
> **2. There is no code-plugin API. Extensibility is declarative: themes and saved views.**

---

## Part 1 — Renderers are generic, or they are wrong

A **view** = a query + a renderer + a grouping/sort configuration.

| Renderer | Generic contract |
|---|---|
| `list` | ordered query result |
| `board` | **group by any enum property** → draggable columns; drop writes the value back |
| `calendar` | **place by any date property** |
| `table` | properties as columns |
| `gallery` | thumbnails, any query |
| `timeline` | chronological, any date property |

Six renderers cover everything anyone has ever asked a PKM tool for.

### The falsifiable test

> **The board renderer must not know what `status` is.**

Point it at `type` instead of `status`. You should get a board of notes / tasks / ideas, with drag-and-drop rewriting `type`.

- **It works** → you built a renderer.
- **It doesn't** → you built a kanban feature, and you will rebuild it the first time you want to group by priority.

**If the string `todo` or `doing` or `done` appears anywhere in renderer code, this ADR has been violated.** Those are values in *your data*, not concepts in *your code*. Put this test in CI.

### Agenda is not a feature

```yaml
name: Agenda
renderer: list
filter:
  type: task
  status: {not: done}
  due: {lte: today+7d}
sort: [due asc]
```

Zero new code. If "agenda" costs a weekend, ADR-001 is wrong and everything stops — this is the S4 checkpoint reappearing in the wild.

---

## Part 2 — Extensibility without a plugin API

> **A plugin API is a mechanism for letting strangers extend your software without touching your source. You have no strangers. You have git and a text editor.**

### What a code-plugin API actually costs a solo project

| Cost | Consequence |
|---|---|
| Permanent API surface | Every internal refactor breaks plugins → **you stop refactoring** |
| Security | A plugin can read your entire private corpus |
| Performance | One bad plugin makes the whole app feel slow, and users blame the app |
| Maintenance | Logseq's plugin API became a documented maintenance burden |
| Abandonment | See: the Projects plugin |

For one user who *is* the developer, this is nearly pure cost. You do not need an extension mechanism to change software you own. **You need an editor.**

### The three layers that give extensibility anyway

**Layer 1 — CSS themes.** Zero API surface. The entire Obsidian theming ecosystem exists on this alone.

**Layer 2 — declarative view definitions.** A `.view` file: filter, group, sort, renderer. **No code execution.** Created by clicking in the UI, saved as a tiny text file, versioned in git alongside the notes. This is exactly what Obsidian's `.base` is — and *the absence of code execution is why it is safe*.

**Layer 3 — the renderer set.** Fixed and small. Adding a seventh renderer is a source change by you, the only developer, in an afternoon. It is not an ecosystem; it is a switch statement.

### Revisit only if

You have **other users** who need to extend the tool without forking it. Until that day, a plugin API is a solution to a problem you do not have.

---

## Consequences

**Easier**
- No silos. Every object is visible to every view, forever.
- No abandoned dependencies. Nothing to rot.
- New views are cheap, which is the entire promise of ADR-001.
- Views are data, so they back up, version, and diff like everything else.

**Harder**
- Renderers must be genuinely generic, which is more thought up front than hardcoding three columns. The CI test is what keeps this honest.
- No community will write views for you. Correct — you are the community.

---

## Action items

1. [ ] `board` renderer groups by **any** enum property. Test in CI by grouping on `type`.
2. [ ] `calendar` renderer places by **any** date property. Test on `created`, not just `due`.
3. [ ] CI: grep renderer source for `todo`/`doing`/`done`. **Fail the build if found.**
4. [ ] `.view` files: declarative, no code execution, stored in the notes repo.
5. [ ] Agenda ships as a `.view` file, not as code. If it needs code, stop.
6. [ ] **Do not design a plugin API.** Not now, not in v2. Revisit only when a second person needs to extend the tool.
