# Getting started

Fifteen minutes from a clone to a running app and a green gate. If you would rather read about the
design first, [Architecture](./architecture.md) is next door — but the build is quick and it makes
the rest concrete.

## 1. Install pixi

[pixi](https://pixi.sh) pins the entire toolchain: the Rust compiler, Node, pnpm, mdBook, and the
optional media tools. **Nothing else is a prerequisite**, and nothing here expects `cargo`, `node`
or `pnpm` on your `PATH` — if you have them, they are not the ones this project uses.

```sh
curl -fsSL https://pixi.sh/install.sh | sh      # or: brew install pixi / winget install prefix-dev.pixi
```

## 2. Clone and set up

```sh
git clone https://github.com/formicaria-org/formicaria.git
cd formicaria
pixi run setup      # points git at .githooks
```

`pixi run setup` is one line — `git config core.hooksPath .githooks` — and it installs a pre-commit
hook that rejects any staged file over 5 MB. This repo is text; media belongs in the vault's
content-addressed blob store. The hook is what keeps a clone small a decade from now, and it is
easy to forget, which is why it has a task.

The first `pixi run` of anything downloads the toolchain. That is the slow one.

## 3. Run it

```sh
pixi run serve      # builds the UI, then serves it against ./vault
```

Open <http://127.0.0.1:8765>. `serve` points at a scratch `./vault` in the checkout (gitignored),
not at your real notes, and it turns off the close-the-tab-quits behaviour — under a dev loop a
closed tab means *"I am about to reload"*, not *"I am finished"*.

Some things to know about the loop:

- **The UI is compiled into the binary.** `crates/fm-serve/build.rs` bakes `ui/dist` in, so after a
  frontend change you rebuild the binary — checking `ui/dist`'s timestamp will fool you.
  `pixi run serve` does both.
- **Restart after a backend change.** There is no watcher.
- `pixi run build` produces the real thing at `target/release/{fm-serve,fm}`, and `pixi run app`
  runs an already-built one without rebuilding.

## 4. The gate

```sh
pixi run ci
```

**This is the whole gate, and it runs locally.** Every GitHub workflow in this repo is manual; none
of them gates a push. If `pixi run ci` is green, you are done — see
[Testing & CI](./testing.md) for what is inside it, which environments exist, and the four opt-in
suites that are not in it.

One trap, from `docs/context/known-issues.md`: **`pixi run ci | tail` reports `tail`'s exit code**,
which is always 0. Read the run's own status, not a pipe's.

## 5. Where things are

```text
crates/fm-model     the data model — no I/O
crates/fm-query     the query engine — CANNOT touch the filesystem (seam 1)
crates/fm-core      FileStore, BlobStore, ingest, git, backup
crates/fm-app       dispatch — the one command surface — and the commands themselves
crates/fm-serve     an HTTP shell over dispatch, and the embedded UI
crates/fm-cli       the `fm` binary; also what git invokes as the .md merge driver
crates/fm-agent     the study assistant's deterministic orchestrator
ui/                 Svelte 5 + Vite
mobile/             the Tauri shell for Android and iOS
docs/src/           this manual
docs/context/       the maintainer's working notes — not documentation
```

## 6. Before you add anything

Read [How to add a feature](./adding-features.md) — but the short version, and the part this
project has actually been bitten by, is that **every addition is justified locally and nothing
checks the sum**. So before adding a dependency, a file to the release archive, a relaxed guard or
a changed default, ask:

1. **What subject is this?** Grep the `#subject` index at the top of `docs/context/decisions.md`.
2. **Read the entries it names — and read past them.** The index is a starting point, not a
   contents page.
3. **Classify honestly: permitted / extends an exception / contradicts a standing decision.** If
   everything you ever check comes back *permitted*, you are not applying this.
4. **Does it widen what ships?** If so it earns a `decisions.md` entry whatever the verdict.

If it contradicts a standing decision, **write the dated reversal first**, with a `> SUPERSEDED`
banner on the entry it supersedes. Writing it first forces the argument while changing course is
still cheap.

`decisions.md` is append-only. A decision is superseded, never edited away — the value of *"we
tried X, then Y"* is the whole chain.
