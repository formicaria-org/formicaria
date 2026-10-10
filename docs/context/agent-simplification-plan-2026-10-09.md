# The assistant, simplified — one assistant, two lanes, one secretary (plan, 2026-10-09)

**Status: partly done and shipped in v0.6.4 (2026-10-11).** P0, P1, the closed command list, the
image-reader rule and the newer-model offer are in; each dated section below says what. Not
started: the Myrme name and device identity, a question about a picture in chat, the secretary's
jobs. Its rulings are in `decisions.md` (`#agent`, 2026-10-10). Built from two read-only maps of the code taken today (fm-agent: 5,233
lines, 14 modules, 105 tests; fm-agent-run: 4,087 lines; fm-serve `agent.rs`: 1,097; mobile
`agent.rs`: 566), plus the owner's direction.

## Scope narrowed by the owner, 2026-10-10 — read this first

> *"I would keep the focus on the simplest and most deterministic agent design: tool selection by
> file type, and the LLM does the meeting update from email. I would then add to all the supported
> devices also the image to text."*

**In scope now, in this order:**
1. **Tool selection by file type, in code.** An image goes to the image reader and audio to the
   speech reader; no model chooses. A command still picks explicitly.
2. **The LLM's one job is mail → meetings** (today's pass: the model points, Rust reads the values).
3. **Image → text on every supported device**, the phone included. The phone's main model cannot see
   images, so this needs a small specialist reader. PaddleOCR-VL is the candidate; measure it first,
   one model at a time.

**Set aside until asked for again:** a model-based or similarity tool selector, fine-tuning, a
bigger general model for the phone, and the "forgotten things" digest (P3, job 2). The research
behind setting them aside: a selector only matters for plain-sentence requests, and fine-tuning
needs hundreds of examples per task where the app's own corpus held 23 records in August
(`outstanding.md` §2.0c).

**How the phases below map:** P0 and P1 (cleanup, one way to do each thing) still come first because
they make the rest small. From P2 keep the closed `Command` enum, the dispatch table and the Myrme
name; from P4 keep the image and audio rows and the `[specialists]` table. P3 keeps only job 1.

## What the owner asked for

> *"The user only sees the general agent: general requests go through a general AI (mostly tailored
> for online search, and things like email to task), and special requests go through dedicated tools
> deterministically… the study assistant will also be a general secretary-like tool, which helps
> with the things a user might forget (like the task updates)."*

It must do four things: **search online on well-grounded sites**, **find new tasks and meetings in
email**, **image → text**, and **audio → text**. The work is mostly in the agent backend.

## Where it stands (the evidence)

**How a person reaches it.** Type `@<model name>` (for example `@qwen3-vl-4b`) in a discussion, and
optionally one of four commands: `/search`, `/research`, `/propose`, `/transcribe`. The meeting pass
has no entry point; it runs on a timer, bolted onto the chat loop. Answers to research, propose and
transcribe arrive as proposals.

**The mess, concretely:**
- **Dead code.**
  - The whole `preference` module (`Embed`, `Memory`, `cosine`) and `OpenAiStep`'s `Embed` impl.
  - `summarize` and `SUMMARY_INSTRUCTION` (summarisation was removed).
  - `StudyAssistant::run`, with `assemble_prompt` and the one-shot `fm-agent-run` binary, which
    re-implements propose outside `Agent`.
  - `Agent.retrieve` (set everywhere, never read), `VaultAccess::search` (RAG was removed), and the
    `allow_propose` parameter (always `true`).
  - `agents/start-agent.sh`, and `agent-chat` / `fm-agent-run` defaulting to `lfm2.5-230m`, which is
    not in the catalogue.
- **Duplication.**
  - Two ways to call the model: `LlmStep::complete` and `ReadImage::read_image`, which rebuilds the
    request and has its own truncation check.
  - The same HTTP POST hand-formatted in five places.
  - Truncation checks in four places, each with a different message.
  - Two quote checks with different rules: `grounding::normalize_for_match` and `meetings::norm`.
  - Two fence-strippers.
  - Asset references parsed three ways.
  - The research and `run` pipelines repeat refine-and-search.
  - Launch code for llama and whisper is copied between `serve.rs` and the phone's `launch`.
  - On/off settings code is in both fm-serve and mobile.
  - `ggml-base.en.bin` is hard-coded in three places, while provisioning downloads something else.
- **Scattered prompts and replies.** Prompt constants live in lib.rs, grounding.rs, meetings.rs,
  imagetext.rs and preference.rs; user-facing reply strings are inline in the runner.
- **The meeting pass is a special case.** It builds its own model client, keeps its own memory file,
  uses UTC for "today", and runs only when the minute rescan finds nothing else to do.
- **A desktop lifecycle bug.** `set_agent false` never stops `agent-serve`, and `running` is reset
  only on a failed spawn, so off-then-on in one session does not respawn. The child is unsupervised.
- **Windows risk.** `serve.rs` looks for `llama-server` and `whisper-server` without `.exe`.
- **Stale words.** `/describe` is mentioned but does not exist; comments call the phone and audio
  "future"; `models.toml` says whisper is "manual-only".

## The shape to aim for

```
            person ── @Myrme ── one name; each device runs its own copy (Myrme · laptop / · phone)
                         │
                 ┌───────┴────────┐
                 │  Router (pure) │  parse → Command (closed enum), deterministic
                 └───────┬────────┘
       ┌─────────────────┼──────────────────────────────┐
  general lane      deterministic lane             secretary (background)
  (the LLM)         (specialist tools)             Job registry, own cadence
  chat · research   /read: image → OCR specialist   mail → meetings (today's pass)
  (grounded web) ·  or VLM, audio → ASR, by MIME     calendar feed (already core)
  mail → meetings   — no model decides routing       "forgotten things" digest
       └─────────────────┴───────────── every write is a PROPOSAL (insertion-only) ──┘
```

**Rules carried over unchanged** (each already a ruling):
- **The model proposes; the person accepts.** Every write is a proposal; insertion-only; nothing is
  deleted.
- **Routing is deterministic.** The file type or the command picks the tool. A model may *suggest*
  a chain for an ambiguous request, but the router decides (the July "media-type judge as actuator"
  cut stands).
- **The model points; Rust reads exact values** (dates, times, quotes): the meeting-pass lesson,
  generalised.
- **Grounded web research** keeps `grounding::verify`: a quote must be in its source, or the claim
  is dropped.
- **The notes-only build stays agent-free.** Phone parity: the same runner on the desktop and the
  phone.

## Plan, in phases — each one shippable and tested before the next

### P0 — done 2026-10-10 (uncommitted at the time of writing)
Each "dead" claim was checked against its callers before anything was deleted. What happened:
- **Deleted:** the `preference` module and `OpenAiStep`'s `Embed` impl; `summarize` and
  `SUMMARY_INSTRUCTION`; `StudyAssistant::run` with `assemble_prompt`; the one-shot `fm-agent-run`
  binary, its `agent-propose` task and the `smoke` example; `Agent.retrieve` and the `--retrieve`
  flag; `VaultAccess::search`; the `allow_propose` parameter; `agents/start-agent.sh`. The four
  behaviours `run`'s tests covered (seed fallback, fence stripping, refusing a cut-off write, refusing
  a request with no host note) are now tested through `research` and `turn`.
- **Fixed:** `agent-serve` looked for `whisper-server` with no `.exe`, so its `exists()` check could
  never pass on Windows. `agent-chat` no longer defaults to a model that is not in the catalogue.
- **Not done, because the plan was wrong:** "off never stops `agent-serve`" is **not a bug**. It is
  the designed behaviour, and Settings says so (*"A change takes effect at the next launch"*;
  `known-issues.md` calls it the `agent.rs` precedent). Off-then-on in one session leaves the same
  process running, which is correct. Stopping it at once would be a behaviour change and needs the
  owner's decision.
- **Kept:** `StudyAssistant` itself (it carries `turn` and `research`), and `agent-chat`.

### P0 as first planned — Clean up, with no change in behaviour (small, safe)
- Delete the dead code listed above. Keep the dev chat binary, rewired through `Agent`.
- Fix the desktop lifecycle: off stops `agent-serve`; on respawns; the child is supervised.
- Fix the Windows binary names. Remove the stale comments and the `/describe` mentions.
- Gate: the full suite, plus a test that off then on respawns.

### P1 — done 2026-10-10 (uncommitted at the time of writing)
No change in behaviour; the gate is green and the phone crate compiles.
- **One request helper:** `fm_agent::http::post` writes the request line and framing headers for the
  model, whisper and `fm-serve` seams. `OpenAiStep::chat` is the one chat-completion call; the text
  step and the image step both use it, and the image step's cut-off check is `truncated()`.
- **One model client:** `Agent::llm()` is the only place the client is built (chat, research, image
  reading, the meeting pass).
- **One quote-matcher:** `fm_agent::textmatch` — `fold` (verbatim: research) and `loose` (`fold` plus
  lower case and accents off: meetings). The meeting reader gains dash and odd-space folding from
  this; the research check gains `«»`. Both are supersets of what they did.
- **One prompts file:** `fm_agent::prompts` holds all six model instructions.
- **One replies file:** `fm_agent_run::replies` holds the assistant's fixed replies.
- **One home for asset references:** the three forms a note names a blob in were known to three
  files (`adjunct::hash_of`, `convo::asset_ref`, the runner's `asset_ref_candidates`). All three
  functions now live in `fm_agent::adjunct`, beside the block that writes one. Finding a note's
  media was already one function, `resolve_blobs`, parameterised by MIME prefix, so **file-type
  routing is already in place** for `/transcribe`.
- **Left as they are, with reasons:**
  - The plan's "five hand-formatted POSTs" were three; the two `GET`s stay as they are.
  - Meeting blocks already use `adjunct`'s marks and `defang`. `adjunct::block` carries a
    specialist/model/blob provenance line a meeting does not have, so forcing it would add fields,
    not remove code.
  - The meeting pass's UTC "today" is a documented limit (`watch.rs`), not duplication.
  - `backup_status_reports_the_password_as_a_bool_and_never_its_value` (`fm-app`) failed once in a
    full run and passed alone and in the next full run: a flaky test unrelated to this work.

### P1 as first planned — One way to do each thing
- **One model client** (`fm_agent::model`): chat, JSON answers (fences and `<think>` handled once),
  image input, truncation as a typed outcome, and one HTTP helper. It replaces the five POSTs and
  the four truncation checks.
- **One text-matching module** for quotes and normalisation, used by research and meetings alike
  (case, accents, dashes, whitespace).
- **One prompts module** with every system prompt, and one reply-strings table, so plain-words
  review is one file.
- **Asset references** parsed in one place; meeting blocks written through `adjunct::block`.
- Gate: today's tests unchanged, plus parity tests where two copies merge.

### Progress on the narrowed scope, 2026-10-10
- **The closed command list is in** (`fm_agent::convo::Command`: `Chat`, `Propose`, `Research`,
  `Read`), with precedence tested; `Agent::handle` dispatches on it. The Myrme name and device
  identity are not started.
- **PaddleOCR-VL-1.6 measured on the laptop** (`agents/bench/results.md`, 2026-10-10). It reads the
  typeset fixtures better than `qwen3-vl-4b`, and b10076 loads it, so no runtime pin bump is needed.
  Two findings shape the design:
  1. **CPU only, an image takes about 25 s** (the image encoder); with the encoder on the GPU it
     takes about 2 s. Measure the phone before promising image → text there.
  2. **Its task prompts cannot be chosen by file type.** Default to `OCR:`; tables, formulas and
     charts need either a second deterministic choice or the person's.
- **Measured on the phone the same day:** the readings match the laptop's exactly, at **about 2½
  minutes per small image** on the processor (2.0 GB peak, alone). It works, as a background job
  that ends in a proposal; it is not something to wait for on screen. A photographed page is larger
  and will take longer unless shrunk first.
- **A faster reader for the phone, measured there the same day: LFM2.5-VL-450M.** 3–5 s per small
  image and about 10 s for a full-size one, 1.2 GB peak, against PaddleOCR-VL's 145–224 s. Correct
  on typed notes and tables; it misread one formula and flattened a code block's nesting, so
  PaddleOCR-VL stays the more exact reader where there is time for it. It needs its own one-line
  instruction; the app's long one halves its accuracy. granite-docling and SmolVLM-256M were tried
  and set aside (`agents/bench/results.md`).
- **The reader rule is built** (`decisions.md`, 2026-10-10): dedicated reader → main model if it
  can see → nobody. One function decides it, one function passes a projector for desktop and phone,
  and a model can carry its own reading instruction. **Done since:** the phone's model was chosen and released (v0.6.4).
  **Not built:** a question about a picture in chat.
- **The phone's default is now `lfm2.5-vl-450m`** (`models.toml`), and **a changed model is
  offered, not pushed** (`decisions.md`, 2026-10-10): the downloaded model keeps running, Settings
  offers the new one with its size, and the catalogue now reaches downloaded desktop apps too.
  Released in v0.6.4 after the owner ran it on the phone.
- **The owner's ruling for the laptop (2026-10-10):** the reader stays on the processor for now.
- **Still needed:** the owner's own handwritten and photographed pages; a phone run beside the main
  model; and a size cap for photos.

### P2 — The router and the skills
- `Command` becomes a closed enum: `Chat`, `Research`, `Propose`, `Read` (with `/transcribe` kept
  as an alias), `Meetings` (run the mail pass now).
- Each command is a **skill** behind one small trait: `run(ctx, input) -> Outcome`, where `Outcome`
  is a reply, proposals, or new-note proposals.
- `Agent::handle` becomes a lookup in a dispatch table instead of a 90-line branch.
- **The assistant gets one stable name: Myrme** (the owner's choice, from *myrmecologist*, the
  keeper of a formicarium). The model is configuration, not identity; the old `@<model>` name stays
  as an alias for a while.
- **One identity per device, because one name on two devices answers twice.** Today the model names
  keep the laptop's and the phone's assistants apart by accident. Under one name, a message synced
  to both would be answered by both. So:
  - **Instances:** each running copy is *Myrme · laptop* or *Myrme · phone*. The device word comes
    from the OS (Linux, macOS or Windows → "laptop"; Android or iOS → "phone"), and a Settings
    field lets the person rename it ("office desktop", "tablet").
  - **Who answers:** the UI stamps each message it sends with the device it was written on (a
    `device:` field in the message's frontmatter, set by `reply`). A plain `@Myrme` is answered only
    by that device's Myrme. `@Myrme·laptop` addresses one copy explicitly, which is needed when only
    that device can do the job (mail, reading images on the laptop).
  - **In history:** each copy signs its own commits and proposals (*Myrme (laptop)*,
    `myrme+laptop@fm-agents.local`), so contributor chips tell them apart. The person's own identity
    is unchanged: *a contributor is an email, everywhere* (`decisions.md`). Which device a person
    wrote from, if wanted, goes in a commit trailer (`Device: phone`), never a second identity.
  - **Gate:** a test where one message reaches two devices' runners and exactly one answers.
- Gate: every command has a dispatch test; the UI chips come from one list the backend also serves.

### P3 — The secretary
- A **Job registry** in the watch loop: each job has a cadence, its own state (the runtime-folder
  JSON pattern the meeting pass already uses, with a version), and a budget of one unit of model
  work per tick, so chat never waits.
- **Job 1** is today's meeting pass, moved out of the special case.
- **Job 2**, a "forgotten things" digest (to be designed with the owner first): overdue `due` notes
  with no change, meetings tomorrow with no agenda note, proposals waiting more than N days. It
  lands as **one** proposal or note per week, never a stream of nags.
- Gate: the jobs are deterministic in tests (an injected clock and a fake vault); never more than
  one job's model call per tick.

### P4 — The specialists (measure, then adopt)
- **Image → text:** PaddleOCR-VL-1.6 (0.5B, Apache-2.0, llama.cpp-native;
  `transcription-specialists-grounded-2026-08-31.md`). Measure it against the current VLM on the
  owner's pages, then route images to it by MIME; the VLM stays the fallback. It runs on the
  laptop's CPU, leaving the GPU to the main model, and is the first path for the phone to read
  images at all.
- **Audio → text:** multilingual Whisper (Italian), configuration only; Parakeet or Qwen3-ASR only
  after a spike.
- **The phone's main model:** LFM2.5-2.6B was measured on 2026-10-09: it always thinks and runs at 7.7 tok/s against 12.4. Its quality is undecided, because the test did not use the settings or the tasks its makers recommend (`agents/bench/results.md`, correction of 2026-10-10). Re-measure it on extraction and tool calls, beside qwen3-1.7b.
- Routing becomes declarative: a `[specialists]` table in `models.toml` (mime → model → runtime),
  read by the router. A new specialist is a row plus a measurement, not new code.

### P5 — What a person sees
- One assistant, one name. Command chips: Ask · Research the web · Read this file · Check my mail
  for meetings.
- One "What the assistant did" list (the proposals it made, the jobs it ran), so the secretary is
  visible, not magic.
- Plain-words review; the manual's assistant chapter is rewritten around the four abilities.

## Order and cost
P0 → P1 → P2 can each land in a day or two of focused work, with tests. P3 needs one design
conversation (what counts as "forgotten"). P4 is measurement-led, one model at a time per the
benchmarking rule.

## Open questions for the owner
1. ~~**The assistant's name**~~: answered — **Myrme**, with one identity per device (P2).
2. **"Forgotten things":** which signals matter — overdue tasks, meetings without notes, stale
   proposals, unanswered email threads?
3. **Web research sources:** keep today's set (Wikipedia, GitHub, arXiv in-process; SearXNG when
   running), or add a curated "well-grounded" allowlist (for example official docs, journals)?
