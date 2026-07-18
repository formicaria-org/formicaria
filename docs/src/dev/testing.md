# Testing & CI

Everything runs through pixi so the toolchain is reproducible. One gate:

```sh
pixi run ci        # = test + test-ui + check-ui + deny + checks + docs
```

## The suites

| Task              | What it runs                                                     |
|-------------------|-----------------------------------------------------------------|
| `pixi run test`   | `cargo test --workspace` — the Rust unit + integration tests.   |
| `pixi run test-ui`| Vitest (jsdom) — the markdown→HTML render seam and app flows.    |
| `pixi run check-ui`| svelte-check over every component. **`vite build` does not typecheck** — it strips types and emits — so without this a component calling a function it never imported builds clean and throws in the browser. |
| `pixi run seam`   | the zero-I/O `fm-query` suite (seam 1).                          |
| `pixi run perf`   | search-under-budget (< 100 ms at 10k notes).                    |
| `pixi run deny`   | `cargo deny` — license/advisory gate (permissive-only).         |
| `pixi run checks` | architectural greps (see below).                                |
| `pixi run docs`   | builds this book; broken links fail the build.                  |

## Conventions

- **Hermetic.** Tests use `tempfile::tempdir` or the in-RAM `MemoryStore`; no
  network, no shared state. Tests needing a subprocess tool (e.g. `restic`,
  `pdftotext`) probe for it and **skip** with a message when it's absent, so the
  suite stays green outside the full env while `pixi run` exercises it for real.
- **Contract equivalence.** `FileStore` is tested to behave identically to
  `MemoryStore`, so command tests written against the memory store hold in
  production.
- **Byte round-trip.** A note read → edited → written back is byte-for-byte
  identical; this invariant is pinned by a test.

## The architectural greps (`ci/checks.sh`)

Two invariants are enforced as build-failing greps:

1. **Seam 1** — `fm-query` must not reference a database crate, `std::fs`,
   `std::path`, or `File`. The query engine physically cannot do I/O.
2. **Generic renderers** — no `todo|doing|done` literal may appear under
   `ui/src/renderers/**`. Status names live only in data and CSS.

Run `pixi run checks` before committing a renderer or a query-engine change.

## UI dev without a backend

`pnpm -C ui dev` (or the vitest suite) runs the UI against the in-memory mock in
`ui/src/lib/mock.ts` — no server, no vault. `import.meta.env.PROD` selects the
real HTTP backend only in the built bundle.
