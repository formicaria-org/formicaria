# formicarium

A local-first, single-user research notebook. Your notes are plain Markdown files
you own, search, organize, and back up; heavy media are content-addressed blobs
referenced from notes. It runs as a local web app — a tiny server (`fm-serve`)
serves the UI and fronts your vault over `http://127.0.0.1:8765`.

## Quick start

```sh
pixi run serve        # build + serve; open the printed URL
# or, opening the browser for you:
FM_OPEN=1 pixi run serve
```

A double-click launcher for Linux is in [`packaging/`](packaging/README.md).

## Documentation

The full manual (user + developer guide, including **how to add a feature**) is
an mdBook under [`docs/`](docs/):

```sh
pixi run docs         # renders docs/ to docs/book/ (also run in CI)
```

Start with `docs/src/introduction.md`, or the design spec in
[`formicarium/MASTERPLAN.md`](formicarium/MASTERPLAN.md).

## Development

Everything is pinned through [pixi](https://pixi.sh). One gate:

```sh
pixi run ci           # test + test-ui + deny + checks + docs
```

- `pixi run test` — Rust workspace tests · `pixi run test-ui` — Vitest (UI)
- `pixi run seam` — the zero-I/O query-engine suite · `pixi run perf` — search budget
- `pixi run checks` — architectural greps · `pixi run deny` — license gate

See [`docs/src/dev/`](docs/src/dev/) for architecture, the extension recipes, the
design system, and testing conventions.

## Layout

```text
crates/  fm-model · fm-query · fm-core · fm-app · fm-serve · fm-cli
ui/      Svelte 5 + Vite frontend (served by fm-serve)
docs/    the mdBook manual
vault/   your notes (its own git repo; git-ignored by this repo)
```

Backend and browser share one implementation: the command functions in
`crates/fm-app/src/commands.rs`, fronted over HTTP by `fm-serve`.
