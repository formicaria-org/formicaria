# Testing & CI

Everything runs through pixi so the toolchain is reproducible. One gate:

```sh
pixi run ci        # = test + test-agent-download + test-ui + check-ui
                   #   + deny + checks + third-party-check + docs
```

## The suites

| Task              | What it runs                                                     |
|-------------------|-----------------------------------------------------------------|
| `pixi run test`   | `cargo test --workspace` — the Rust unit + integration tests.   |
| `pixi run test-agent-download` | the in-app model fetcher's hermetic (localhost) tests. Separate because the `download` feature is off by default, so `test` above never builds it. |
| `pixi run test-ui`| Vitest (jsdom) — the markdown→HTML render seam and app flows.    |
| `pixi run check-ui`| svelte-check over every component. **`vite build` does not typecheck** — it strips types and emits — so without this a component calling a function it never imported builds clean and throws in the browser. |
| `pixi run seam`   | the zero-I/O `fm-query` suite (seam 1).                          |
| `pixi run perf`   | search-under-budget (< 100 ms at 10k notes).                    |
| `pixi run deny`   | `cargo deny` — license/advisory gate (permissive-only).         |
| `pixi run checks` | architectural greps (see below).                                |
| `pixi run third-party-check` | `THIRD-PARTY.md` still describes the tree it claims to. Regenerate with `pixi run third-party`. |
| `pixi run docs`   | builds this book; broken links fail the build.                  |

## Environments

`pixi.toml` defines four, and a task belongs to exactly one. Running a task from the wrong one
is a real hazard rather than a tidiness point: the environments resolve their own `nodejs`/`pnpm`,
and a mobile CLI run from the wrong one has already re-resolved a dependency and broken a build.

| Environment | For |
|---|---|
| `default` | everything above. |
| `media` | the optional media tools (`pdftotext`, `vipsthumbnail`, `restic`) — so the tests that probe for them actually exercise them instead of skipping. |
| `android` | the Android toolchain: `pixi run -e android android-release`, `android-smoke`. |
| `cross` | the cross-compilation targets. `pixi run -e cross check-cross` type-checks `fm-agent` for Windows, macOS and both iOS targets. |

## Opt-in suites — not in the gate, and each for a reason

These are cheap, they run locally, and none of them needs CI. They are outside `pixi run ci`
because they are slow, need a toolchain the gate does not, or are for one platform.

| Task | Run it when |
|---|---|
| `pixi run test-native-git` | **Whenever `commit_all` or `git_native.rs` changes.** `cargo test --workspace` `cfg`s out `merge_differential`'s native gate and skips `merge_on_a_device_without_git` entirely, so the libgit2 backend — the one the phone uses — is *untested by the gate*. Nine test binaries. |
| `pixi run -e cross check-cross` | after touching anything per-OS in `fm-agent`. It found a no-op `std::mem::forget` on a raw pointer and a deprecated `libc::mach_host_self` on its first run, in code that had shipped without ever being compiled for either target. |
| `pixi run check-pins` | when a model or runtime pin moves. Downloads ~110 MB and checks every pinned runtime is really there. |
| `pixi run -e android android-smoke` | after an Android change, with a device or emulator attached. |

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
