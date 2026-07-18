# 2026-07-17 — Create vault: a first-run gate, and the vault list gains a writer

**Outcome:** you can create a vault from inside the app, and with none configured the app
says so instead of inventing one. `pixi run ci` green; verified end-to-end against a real
`fm-serve` (first-run → create → capture, with no restart), not just in tests.

Shipped: `check_path` / `create_vault` / `list_vaults`; the first-run screen
(`NewVault.svelte`, one component, two mounts — the gate and a BackupPanel dialog);
`fm-serve/src/vaults.rs` (the extraction **plus the first writer this config has ever
had**); `MultiStore::open(&[])` legal + `NoVaults` + `add`; `commands::inspect_path`; the
`AppState` one-mutex refactor. fm-serve went from **zero tests to six**.

## Why

`load_vaults()` read `vaults.json`; **nothing wrote it**. It was hand-edited JSON, so there
was no path from "I want a vault" to a configured, opened, listed vault. And it could never
return empty — `FM_VAULT` defaulted to the *relative* `"vault"`, so a typo, or the launcher
started from another cwd, silently `create_dir_all`ed a working empty vault named after the
mistake while your notes appeared to have vanished. Configuration masquerading as
capability: the `restic_ready` shape, again.

## Rulings made while building (full entries in decisions.md)

- **Do not `git init` at creation.** `commit_all` already calls `ensure_repo` on the first
  auto-commit. Doing it eagerly buys an empty `.git` five seconds early — and if the path
  sits inside a repo the user owns, `ensure_repo` probes only `<path>/.git`, finds none, and
  `git init`s a **nested repo shadowing theirs**, writing the placeholder identity into it.
- **One mutex, not two.** `api()` already took the store and the vault list in *opposite
  orders* (`commit`/`push` lock-then-resolve; `ingest` the reverse). A second mutex would
  have been AB/BA. Worse, one mutex makes `lock(state)` + `state.vault()` a **self-deadlock**
  — `std::sync::Mutex` is not reentrant — so those arms were rewritten to resolve through
  the guard they already hold, not merely re-typed.
- **`vaults::save` writes the whole live list, and merges into the parsed `Value` tree.**
  Not a typed serde round-trip: `#[serde(flatten)] extra` would reformat a human's file and
  turn "I don't understand this" into "I dropped it". Appends only; refuses a file it could
  not parse. *The sharp bug:* `FM_VAULT` set with no `vaults.json`, then create vault #2 —
  write only the new entry and `load` prefers the file, ignores `FM_VAULT`, and **vault #1
  vanishes on the next start.**
- **JSON before memory.** The config write is the commit point. Reverse it and a failed
  write leaves an in-memory vault that vanishes on restart *while you capture notes into
  it*. Partial failure is reported as partial, and a failed open never deletes the
  directory.
- **`writable` is probed, not inferred.** Create and remove a temp entry. Mode bits are
  configuration; the probe is capability.
- **`allVaults` moved off loaded notes onto `list_vaults`.** It derived from fetched cards,
  so an *empty* vault did not exist as far as the sidebar was concerned — "empty vault" and
  "no vault" looked identical, which is the exact confusion the first-run screen exists to
  end.

## Left not-working / deliberately out

- **`main.rs`'s `.expect("open vaults")` still panics** when a configured path fails to
  open (an unmounted drive costs you the other vaults). Real, pre-existing, and *uncoupled*:
  making `open(&[])` legal does not change it. An `open_lossy` was planned and cut as scope.
- **The first-run screen is unverified in a real browser.** The data path is verified
  (`list_vaults` → `[]`, `refresh()` early-returns at zero); the rendering is not.
- No git clone / join-a-remote-vault, no vault removal, no descriptor, no native dialog.
  This session is **Track V's V1**; V2–V4 are in [plan.md](../plan.md) — and **V2 is four
  live co-tenancy bugs, two of them silent**, found while building this. They gate V3–V4.

## Also

`.github/workflows/cross.yml` is **manual-only** (`workflow_dispatch` alone): this month's
Actions minutes are spent and at macOS 10× / Windows 2× it was essentially the whole bill.
`ci.yml` + `docs.yml` are ubuntu-latest (1×) and still gate every push. The cost is real and
recorded in that file — Linux cannot see the CRLF class of bug the first Windows run caught.
Restore by putting the `push:`/`pull_request:` blocks back; `release.yml` still builds all
three on a `v*` tag.
