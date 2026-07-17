# formicaria

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

## Build & run (debug vs release)

The app is a production UI bundle (`ui/dist`) plus the `fm-serve` binary. There's
no separate "install" — you run the binary from the repo root and it serves the
bundle and your vault.

```sh
# Run it (builds + serves in one step):
pixi run serve            # DEBUG build — fast to compile, unoptimized runtime (dev loop)
pixi run serve-release    # RELEASE build — optimized (~3 MB, faster); what you'd use daily

# Or just build the artifacts, without running:
pixi run build            # → target/release/fm-serve  (stripped + thin-LTO)  + ui/dist
pixi run build-debug      # → target/debug/fm-serve    (full debuginfo)       + ui/dist

# Then run a pre-built binary yourself (from the repo root, so it finds ui/dist):
FM_OPEN=1 ./target/release/fm-serve      # optimized, opens the browser
./target/debug/fm-serve                  # debug, prints the URL to open
```

`serve`/`serve-release` rebuild the UI every run; `build`/`build-debug` stop at the
compiled artifacts. `pixi run app` runs the already-built release binary **without
rebuilding** (instant start). Config is via env: `FM_VAULT` (default `vault`),
`FM_UI_DIST` (default `ui/dist`), `FM_ADDR` (default `127.0.0.1:8765`), `FM_OPEN`
(set → open the browser). The release profile lives in [`Cargo.toml`](Cargo.toml)
(`[profile.release]`).

> The [`packaging/`](packaging/README.md) `.desktop` launcher runs `pixi run app`
> — the **prebuilt release**, no rebuild, so a double-click is instant. It reflects
> your last `pixi run build`; rebuild to update what the icon launches. **Closing
> the tab quits the app** (the server auto-stops when the last tab's heartbeat
> stops — enabled by `FM_AUTO_SHUTDOWN`, which the launcher sets; a plain `pixi run
> serve` stays up until Ctrl-C).

## Documentation

The full manual (user + developer guide, including **how to add a feature**) is
an mdBook under [`docs/`](docs/):

```sh
pixi run docs         # renders docs/ to docs/book/ (also run in CI)
```

Start with `docs/src/introduction.md`, or the design spec in
[`formicaria/MASTERPLAN.md`](formicaria/MASTERPLAN.md).

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
