# The study assistant

formicaria can run a small **local AI assistant** that answers questions in a note's discussion,
researches the web, and drafts note edits for you to review. It is **opt-in and off by default** — a
notebook that never turns it on pays nothing for it — and it runs **entirely on your own device**:
your notes never leave your machine, and the model itself runs locally, so there is no account, no
API key, and no cloud.

It is a *helper*, not an oracle. A small on-device model is good at summarizing, tidying notes, and
fetching-and-summarizing simple facts through a search tool; it will not match a large hosted model on
hard reasoning. That trade — small, private, local — is the point.

## Where it runs

Before anything else, the honest matrix.

| | Assistant | Audio → transcript | Reading images |
|---|---|---|---|
| **Linux**, from the download | **yes** — it fetches what it needs on first enable | yes | yes |
| **macOS**, from the download | **yes** | not yet | yes |
| **Windows**, from the download | **yes** | not yet | yes |
| **Linux**, from a checkout | yes | yes | yes |
| **Android** | yes, bundled in the app | yes, bundled | no — the phone's model cannot see |

**You no longer need the source code.** Turning the assistant on downloads the model and the runtime
it needs, having first told you how large they are and under what licence. The commands in "Turn it
on" below are the checkout route, which still works and is what a developer wants; a downloaded copy
needs none of them.

**Audio transcription is Linux-only for now.** Its runtime is a second, separate download and only
the Linux build of it has been verified; on macOS and Windows the switch simply does not appear
rather than offering something that would not work.

> **macOS and Windows are new here.** Before starting a model, formicaria checks it can read how
> much memory the machine has free, and refuses if that reading fails or looks implausible — it will
> not run a model it cannot watch. That check is what made those platforms wait, and it is still
> what protects them.

**Why macOS and Windows refuse.** Before starting a model, formicaria checks whether the machine
has room for it and keeps watching while it runs. That check reads Linux kernel counters and has no
macOS or Windows implementation yet, so on those systems the app **refuses rather than running a
model it cannot watch**. It is not a licence, a download or a setting: nothing you install will
change it, and the right fix is a monitor for those systems rather than a relaxed check. Settings
tells you which case you are in, and shows no switch instead of one that would fail.

Everything else in formicaria — notes, search, boards, backup, sharing — works normally without any
of this.

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

3. **(Optional) transcribing recordings.** `/transcribe` needs an audio runtime, which is a
   separate download from the model above:

   ```sh
   pixi run fetch-whisper
   ```

   Until it is there the transcription switch does not appear — it is a capability of its own, not
   part of the assistant, and the app will not offer a switch it cannot honour.

4. **(Optional) reading images.** `/transcribe` can read a photo of a page — handwriting, printed
   text, mathematics as LaTeX — but only with a model that can see. That needs a *projector* file
   beside the weights, and `fetch-model` collects it automatically for any model that has one:

   ```sh
   pixi run fetch-model qwen3-vl-4b   # weights + projector; the laptop pick for reading images
   ```

   A text-only model is not an error — the assistant says images are unavailable rather than asking
   a blind model to guess at a picture.

5. **(Optional) web search.** For `/search`, run the keyless proxy in another terminal:

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

It replies in the thread, attributed to the model's own name so you can see who said what. Four
commands refine a turn:

| Command | What it does |
|---|---|
| `/search` | Look the web up first, then answer from what it found (desktop; keyless SearXNG proxy). |
| `/research` | The thorough version: search, write with quotes, and check every claim against its source. Drops the ones it cannot support. |
| `/propose` | In a note's discussion, draft an **edit to that note** for you to review. |
| `/transcribe` | Turn this note's **recordings and images into text** — see below. |

Nothing the assistant writes lands in a note by itself. Every one of these produces a **suggestion
you review**: you see exactly what would change, and you accept it, edit it first, or turn it down.

Example:

```
@lfm2.5-1.2b summarize the key idea of Bayesian model selection in 3 bullet points /search
```

### Transcribing recordings and writing

`/transcribe` turns the media in a note into text: a voice memo becomes a transcript, and a photo of
a page, a whiteboard or a printout becomes something you can search and edit.

```
@qwen3-vl-4b /transcribe
```

Typed on its own it does **everything in the note that has not been done yet** — you never have to
name a file or copy an identifier. Ask twice and it skips what it already read.

For writing it aims at a *usable* transcript, not a description: mathematics comes back as LaTeX
(from the vision model itself — good for ordinary notation, and not a specialised formula reader),
code and pseudocode inside a code block with their indentation, and tables as Markdown tables. For a
chart it transcribes the title, the axis labels, the tick values and the legend.

Three things worth knowing:

- **The picture stays.** The text lands *beside* the image, never instead of it, and says which model
  read it. If the reading is wrong you can always look at the original.
- **It will misread things** — a handwritten symbol, a subscript, an O that is really a zero. Read
  the equations before you trust them. Where it genuinely cannot make something out it writes `[?]`
  rather than guessing, which is the mark to look for.
- **Reading images needs a model that can see.** Recordings need the audio runtime; images need a
  vision model with its projector file fetched. If one of those is missing the assistant says so
  instead of inventing an answer, and still does the half it can.

### What you change is remembered

When you edit or turn down a suggestion, formicaria keeps a record of it: what was suggested, what
you made of it, and — if you tell it — why. There is an optional one-line box for that, and a few
one-tap tags; leaving it empty is fine and costs nothing.

The record stays in your vault with your notes and **is not sent anywhere**. In *Settings* each vault
has two separate switches: whether to keep the record at all, and whether it may ever be shared
openly. The second is off unless you turn it on, and it is asked separately for a reason — agreeing
to keep a note of your own corrections is not agreeing to publish them, and sharing cannot be undone.

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
