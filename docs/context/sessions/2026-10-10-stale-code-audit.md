# 2026-10-10 — what in the code is stale and could go (audit, nothing removed)

A snapshot, as found that evening, after the assistant's own cleanup landed (`3a9642c`). **Research
only: no file was changed for this.** Each finding says how it was found, so it can be re-checked
before anyone deletes on the strength of it.

## What was then done, the same evening
Asked to fix it, each candidate was opened before it was touched, and **half of them turned out
not to be stale**. Removed:
- the unused style rule in `BackupPanel.svelte`; `needsAttention`; the two unused type names;
  `with_seed` and `with_max_results`;
- **the `media` environment** (ffmpeg, tesseract): 153 packages left `pixi.lock`, none was added;
- **`fm-serve`'s dependency on `fm-query`**, which nothing in it used (the workspace still builds).

Kept, with the reason the audit below missed:
- **The developer route in group 1.** `search-proxy.py` is not a duplicate of the in-app search: it
  reaches the general web, which the in-app search deliberately does not, and the manual documents
  it. `fetch.sh` and `fetch-whisper.sh` are the only way to stage a model without the app, and the
  README documents them. Only the two launch wrappers duplicate anything, and removing them alone
  was not worth changing a documented route. **Group 1 is a design question, not stale code.**
- **The `stale` command.** It has five tests and is the piece a "forgotten things" digest would
  be built on (`agent-simplification-plan-2026-10-09.md`, P3). It lacks a caller, not a purpose.
- **The two "clear" commands.** Forgetting a backup password is a step with consequences; it wants
  a designed confirmation, not a button added in a cleanup.
- **`needs_migration`.** Its comment says it is the hook for a future format change.
- **The two "mock cases with no command"** were a false reading: `status` and `type` are property
  names in the mock's own `switch`, not commands.

## How it was done
- **Rust:** every `pub` item in the eleven crates and the phone crate, searched for by name across
  all sources, separating product code from tests. (Private dead code is already impossible:
  `clippy -D warnings` is in the gate.)
- **Commands:** all 100 `dispatch` arms checked for a caller in the interface, the CLI, the agent
  runner, the phone shell or `fm-serve`.
- **Interface:** every `export` in `ui/src/**/*.ts`, every component, every module, checked for a
  use outside tests and the mock. Lazy `import()`s were checked by hand.
- **Dependencies:** each crate's `Cargo.toml` and `ui/package.json` against what the sources name.
- **Scripts and tasks:** every file in `ci/`, `agents/` and `packaging/` checked for a reference;
  every path a task or workflow names checked to exist.
- **Words:** `TODO`, `FIXME`, `HACK`, "legacy", "deprecated", "no longer", read in context.

About 70,000 lines of Rust and 41,000 of interface code. **A name search cannot see a use built
from a string or a macro**, so every item below is a candidate to verify, not a verdict.

## The short answer
**The code is already tight.** Three unreferenced public functions in 70,000 lines of Rust, no real
`TODO`, no command without a caller, no script nothing runs, no tracked build output. What is
left falls into five groups, largest first.

## 1. A second way to run the assistant that only developers can reach (largest)
Since 2026-09-02 the app starts the assistant itself (`fm-serve` → `agent-serve --web-direct`), and
the phone has always done so in-process. The older shell route is still here:

| What | Size | Who still uses it |
|---|---|---|
| `agents/search-proxy.py` and the `search-proxy` task | 196 lines | only `agent-serve --searxng-port`, typed by hand |
| `fm_agent::search` (`SearxngSearch`), `Agent.searxng_port`, `--searxng-port` | about 230 lines + plumbing in 20 places | the same; the app always passes `--web-direct` |
| `agents/agent-serve.sh`, `agents/serve.sh` | 78 lines | the `agent-serve` and `serve-model` tasks |
| `agents/fetch.sh`, `agents/fetch-whisper.sh` | 237 lines | the `fetch-model` and `fetch-whisper` tasks; the app downloads in Rust (`fm_agent_run::fetch`) |

- **Why it is stale:** two search back ends and two download paths, where the shipped app uses one
  of each. This is exactly the "one way to do each thing" the simplification plan asked for.
- **What removing it costs:** the dev loop in `agents/README.md` would use the same path the app
  does. `agent-chat` sets `web_direct: false` and would need the other back end.
- **Check first:** whether the in-process search (`websearch.rs`) is behind the `download` feature
  for a reason a plain `cargo build` of the runner still needs.

## 2. Things built and never wired to a caller
| What | Where | Evidence |
|---|---|---|
| The `stale` command ("notes untouched since …") | `commands.rs:189`, `dispatch.rs:1096`, `ipc.ts:559` (`staleNotes`), a row in the command reference | the only caller is the `staleNotes` wrapper, which nothing imports |
| `clearResticPassword`, `clearGitCredential` | `ipc.ts:904`, `ipc.ts:1035` | the commands exist; no screen calls the wrappers. **A gap as much as dead code:** by the UI-only rule there is no way to forget a stored password from the app |
| `needsAttention` | `sync.svelte.ts:300` | exported, never read |
| `TextToken`, `CalloutType` | `render-vocab.ts:13,18` | exported types, never referenced |
| `needs_migration` | `fm-model/src/schema.rs:11` | never called; its comment says it exists for a future format change |
| `with_seed`, `with_max_results` | `fm-agent/src/openai.rs:65`, `search.rs:56` | builder methods nothing calls |

For the first two rows the decision is **wire it or remove it**, and it is the owner's: `stale` is a
feature without a surface, and the two "clear" commands are a surface the app is missing.

## 3. Tools installed that no code runs
- **The `media` environment** in `pixi.toml` installs **ffmpeg and tesseract**. Nothing in the
  Rust, the scripts or CI runs either, and no task uses `-e media`. They are mentioned only in old
  design notes. Tesseract is also one of the engines the project ruled out.
- **`fm-serve` depends on `fm-query`** in its `Cargo.toml`, and no file in `fm-serve/src` names it.
  **Verify before touching:** the seam rule is that `fm-query` stays pure, and a dependency edge
  may be there on purpose for a check; if not, it is a line to delete.
- `@types/react` and `@types/react-dom` are not imported by name. They are almost certainly needed
  to type-check the embedded Excalidraw, so **probably not stale**; listed only because the search
  flagged them.

## 4. Code that exists only for tests, living in product files
Not dead, but worth a look when the file is next open:
`is_chat` (`convo.rs`; superseded today by `Intent::command`), `verified_quote_rate`
(`grounding.rs`), `runtime_keys` (`manifest.rs`), `is_all` (`scope.rs`), `is_scene` (`scene.rs`),
and in the interface `placeValue`, `isoDate`, `resetReachable`, `clearSync`. The four
`*_for_test` helpers and `force_native` are test seams by design and should stay.

## 5. Small leftovers
- **One unused style rule:** `BackupPanel.svelte:1361`, `input[type='text']` (the only warning
  `svelte-check` gives).
- **Two mock cases with no command behind them:** `status` and `type` in `ui/src/lib/mock.ts`.
- **Test fixtures still named after a model that is not in the catalogue:** `lfm2.5-230m` in
  `manifest.rs` and `convo.rs` tests. Harmless; confusing to read.
- **Fourteen "legacy alias" style properties** in `app.css` that the theme notes say "will be
  retired". They are **not** dead: about 100 uses. Retiring them is a rename, not a deletion.
- **`ggml-tiny.en` and `qwen3-4b-2507`** are catalogue entries nothing selects by default. Under
  the rule adopted today, an entry is also what keeps an already-downloaded model supported, so
  **removing one ends support for it**. Not a cleanup to do casually.

## Documents
`docs/context/` holds several dated papers the router already calls "an earlier decision's
receipts". Candidates to move into `archive/`, since each says of itself that it is superseded or
unmaintained: `model-selection-research-2026-07-22.md`, `model-benchmarks-2026-07-22.md`,
`transcription-specialists-survey-2026-08-31.md`, `forward-plan-review-2026-07-22.md`,
`agent-review-2026-07-22.md`, `agent-backed-apps-runtime-2026-07-22.md`. Moving one means updating
its router pointer, which the gate checks.

## What was looked for and not found
- No public Rust function, type or constant unused outside the three in group 2.
- No `dispatch` command without a caller (the two used only by `fm-serve` are its timers).
- No component or module the interface never loads (the panels are loaded lazily).
- No script in `ci/`, `agents/` or `packaging/` that nothing references, and no task or workflow
  pointing at a file that is not there.
- No real `TODO`, `FIXME` or `XXX`. One `HACK`, one `#[allow(deprecated)]` with its reason beside it.
- No build output, cache or backup file tracked by git.

## A suggested order, if any of it is done
1. Group 5 and the three unused builders and types: minutes, no behaviour change.
2. Group 3: the `media` environment, then the `fm-query` edge once checked.
3. Group 2: decide `stale` and the two "clear" commands.
4. Group 1: the dev-only assistant route, as the simplification plan's next step. It is the only
   one that changes how someone works.
