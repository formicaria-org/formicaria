# The study assistant

formicaria can run a small **local AI assistant** that answers questions in a note's discussion,
researches the web, and drafts note edits for you to review. It is **opt-in and off by default** — a
notebook that never turns it on pays nothing for it — and it runs **entirely on your own device**:
your notes never leave your machine, and the model itself runs locally, so there is no account, no
API key, and no cloud.

It is a *helper*, not an oracle. A small on-device model is good at summarizing, tidying notes, and
fetching-and-summarizing simple facts through a search tool; it will not match a large hosted model on
hard reasoning. That trade — small, private, local — is the point.

> **This chapter needs the source code, not the download.** Every step below runs from a checked-out
> copy of the project with its toolchain installed. **The release archive does not carry the
> assistant**, so on a downloaded copy the switch in Settings says so and stays off — nothing here
> will work from the folder you unpacked.
>
> The assistant also runs on **Linux and Android only** today: on Windows and macOS formicaria
> refuses to start it, because the safety check that decides whether the machine has room for a model
> has no implementation on those systems yet. Settings says which of those applies to you.
>
> Everything else in formicaria — notes, search, boards, backup, sharing — works normally without
> any of this.

## Turn it on

### On the desktop

1. **Get a model.** From the project directory:

   ```sh
   pixi run fetch-model              # the lightest default from agents/models.toml
   pixi run fetch-model lfm2.5-1.2b  # a specific, better one (recommended for research)
   ```

   This downloads a local model runtime and the model weights into `agents/` (both gitignored). It
   is the only setup step, and nothing is written outside that folder.

2. **Enable it in Settings** → *Study assistant* → on. It then starts automatically whenever you run
   formicaria, and stops when you close it — zero cost while off, no background process, no orphan.

3. **(Optional) web search.** For `/search`, run the keyless proxy in another terminal:

   ```sh
   pixi run search-proxy
   ```

### On Android

The assistant runs **on the phone itself** — the model is bundled in the app and downloads its
weights on first enable.

1. **Build and install the app** (from the project directory, with the Android toolchain set up via
   `pixi run android-init`):

   ```sh
   pixi run android-release          # a signed, 16 KB-aligned APK at mobile/formicaria-<abi>.apk
   ```

   Install that APK on your phone (`adb install -r mobile/formicaria-*.apk`, or copy it across and
   open it). A **notes-only build** that contains none of the assistant is the default when built
   `--no-default-features` — so people who only want the notebook can ship a lighter app.

2. **First launch** downloads the model (~150 MB–700 MB depending on the model) over your connection,
   shown by a persistent "study assistant" notification. It resumes if interrupted, and only happens
   once. After that the assistant is ready and runs offline.

## Use it

In **any discussion** — a note's discussion thread, or a first-class discussion — type `@` and the
picker suggests the assistant (and any collaborators in the vault). Pick it and ask:

```
@lfm2.5-1.2b what is the difference between mRNA and DNA vaccines?
```

It replies in the thread, attributed to the model's own name so you can see who said what. Two
commands refine a turn:

| Command | What it does |
|---|---|
| `/search` | Research the web first, then answer from what it found (desktop; keyless SearXNG proxy). |
| `/propose` | In a note's discussion, draft an **edit to that note**. The change lands on a review branch, never on `main` — you read the diff and merge or discard it. |

Example:

```
@lfm2.5-1.2b summarize the key idea of Bayesian model selection in 3 bullet points /search
```

### The mention picker

Typing `@` lists everyone you can address here: the assistants that are running, plus the vault's
git **collaborators** (anyone who has posted in a discussion). So `@` works whether you are mentioning
the AI or a person you share the vault with.

## Choosing a model

Each device has a sweet spot — small enough to stay responsive and not slow the rest of the system
down, large enough to answer well. Measured on real hardware:

| Device | Recommended | Why |
|---|---|---|
| **Phone** (mid-range, ~8 GB RAM) | `lfm2.5-1.2b` | Best instruction-following + tool-use in its size; ~16 tokens/s, ~1.4 GB — smooth, leaves the UI responsive. |
| **Laptop** (8-core, ≥8 GB RAM) | `qwen3-4b-2507` | A larger, stronger model the laptop can hold; the best small-model tool-caller. |
| **Any low-end device** | `lfm2.5-350m` | The fast, tiny floor — instant, ~0.45 GB — when responsiveness matters most. |

The assistant caps its own thread use so inference never starves the interface — on a phone this is
also, conveniently, the *fastest* setting.

## Privacy and control

- **Off by default.** Nothing runs, downloads, or watches until you enable it.
- **Your notes stay local.** The model runs on your device; notes are never uploaded. Only `/search`
  touches the internet — and only the search terms, never your notes.
- **You are always in control.** The assistant can only **reply** in a discussion or **propose** a
  change on a branch you review. It never writes to `main` on its own. Turn it off in Settings (or,
  on the desktop, simply don't run it) and formicaria is byte-for-byte the plain notebook.
