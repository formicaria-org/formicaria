# How agent-backed apps run the model — evidence for formicaria's runtime

> **Dated research, 2026-07-22 — a snapshot, not maintained.** An evidence layer about other
> people's apps, which move. Current rulings: `decisions.md#agent`.

*Research pass (4 clusters + synthesis, 2026-07-22): among real, popular, actively-maintained apps
shaped like formicaria — a **primary notes/knowledge app plus an opt-in LLM agent**, NOT a pure
chatbot — how do they run the model relative to their own process? This is the evidence layer under
`forward-plan-review-2026-07-22.md`, and it settles the in-process-FFI-vs-external question.*

## Headline verdict

Of the ~18 distinct primary-app analogs surveyed (deduped), **every single one keeps the generative
model in a separate process** — a user-run or app-spawned local server (Ollama / LM Studio /
llama.cpp-server / vLLM / LiteLLM), a remote API, or an external MCP client, reached over an
OpenAI/Ollama-compatible **HTTP** seam. **Zero embed a generative LLM in the notes-app's own
process.** The only apps that embed llama.cpp in-process via JNI/FFI are **single-purpose chatbots or
a vendor demo** (PocketPal, SmolChat, ChatterUI, Google AI Edge Gallery) — cited precisely for the
whole-app SIGSEGV/OOM/LMKD-kill crashes that embedding causes. The evidence strongly **confirms**
formicaria's current out-of-process posture and argues **against** the pending in-process-FFI switch.

## The closest twins, and what they do

| App | What it is | Traction | Model runtime | Crash isolation | Mobile |
|---|---|---|---|---|---|
| **Obsidian + Copilot / Smart Connections** | Flagship local-first MD notes; AI via plugins — closest structural twin | Obsidian millions; Copilot ~7.4k★ | external HTTP (+~25MB MiniLM embedder in-proc) | Ollama OOM leaves vault fully editable | On mobile points at a **remote/LAN** server; no on-device generation |
| **Reor** | Purpose-built local-first MD AI-notes app — single closest analog | ~8.6k★ (archived Mar 2026) | external HTTP; **bundles+spawns Ollama** | editor survives model crash; tried in-Electron llama.cpp, **shipped external instead** | Desktop-only — never attempted embedded phone generation |
| **AppFlowy + Local AI** | Rust+Flutter local-first Notion alt — closest tech-stack analog | ~74k★ | external HTTP (Ollama; AI in a separate LAI sidecar repo) | `local_ai_not_ready()` / `MissingModel` are **typed handled states**; workspace runs with no model | Ships mobile; Local AI desktop-only |
| **Trilium/TriliumNext** | Large hierarchical KB; AI built **into** the app | ~37k★ | external HTTP behind `AIServiceManager` | provider interface + availability checks; Experimental/off-by-default | no on-device mobile LLM |
| **Khoj** | Self-hostable "AI second brain"; phone clients | ~35.9k★ | multiple — external, or llama-cpp-python FFI **inside the server tier** | even its FFI model lives in the *server* process, isolated from the client | iOS/Android are **thin clients**; model never on phone |
| **AnythingLLM** | Private document/RAG workspace | ~63k★ | multiple — leads with external; **optional** bundled node-llama-cpp | external path isolated; docs push toward external even though it *can* embed | Desktop + Docker; no mobile |
| **Joplin — Jarvis** | Leading AI plugin for cross-platform local-first MD notes | ~352★ | external HTTP (any OpenAI-compatible backend) | model out-of-process; note app survives server death | runs on Joplin mobile, points at a **networked** server |
| **Standard Notes** | E2EE notes | ~6.6k★ | remote-api-only (exposes API/MCP) | app **never** runs inference — maximal separation | mobile = plain client, no model |
| **Continue.dev** | Agentic AI in the IDE (host = editor) | ~35k★ | external HTTP; ships no model | pure client; stop-the-server off-switch inherent | Desktop IDE |

*Cautionary non-analogs (pure chat / demo, in-process-FFI):* **Google AI Edge Gallery** (~9k★),
**PocketPal** (~6k★), **ChatterUI** (~1.5k★), **SmolChat** — model in app process; native crash/OOM
kills the whole app; LMKD reaps it. Single-purpose — **nothing to protect**. That is exactly what
differs for a notes app: it has a vault to protect, which changes the calculus.

## Why the apps like us chose external/HTTP (maintainers' own framing)

1. **Crash/OOM isolation is the whole point** — delegating converts a model crash into a recoverable
   HTTP error. Obsidian Copilot: keep "the plugin lightweight while delegating the computational
   burden to dedicated local server applications." Reor spawns Ollama "rather than linking llama.cpp,
   keeping the crash/OOM blast radius outside the notes app."
2. **Offload model-management** (download, quant, GPU offload, keep-alive, limits). AppFlowy models
   "no model available" as a typed state; Cherry Studio's stance: **"be the UI, never the runtime."**
3. **One wire contract → swap local↔remote for free.** Jarvis/Khoj/SurfSense treat Ollama/Jan/LM
   Studio/remote API as interchangeable behind OpenAI HTTP — a config swap, not an architecture change.
4. **Keep the core app unburdened, feature opt-in.** Trilium gates AI behind a provider interface +
   Experimental toggle; AppFlowy keeps AI in a **separate repo** so the core has zero hard dependency
   on a model runtime.

Even the apps that *can* embed refuse to default to it: AnythingLLM leads with external; Khoj's FFI
model sits **behind a server boundary** so an OOM "kills the Khoj server, not the editor." The lesson:
**FFI is acceptable only if it stays behind a process boundary — which is the out-of-process
architecture with extra steps.**

## Failures to learn from

**From embedding in-process (the risk we were weighing):**
- **A bad model file = a dead app.** One corrupt GGUF (Falcon3-1B) produced `SIGSEGV` during init
  *simultaneously* across ChatterUI, PocketPal and SmolChat. If we embed, every upstream
  llama.cpp/model bug becomes a full-app crash that loses the user's notes session.
- **No graceful degradation under memory pressure.** Native heap isn't GC-reclaimable; Android LMKD
  OOM-kills the entire foreground app when a multi-GB model is resident. An oversized model doesn't
  slow down — it reaps the process.
- **The host-crash class is real on desktop too** (Electron/V8 OOM, node-llama-cpp OOM, LM Studio
  V8-OOM-on-startup). Reor sidestepped all of it by delegating to Ollama.
- **You lose the OS off-switch** — killing the model means killing the app.

**From depending on an external server (the cost we'd inherit):**
- **The "is the server up?" tax** — the dominant *visible* failure is connectivity, not stability
  (CORS, wrong port, model-name mismatch). Budget real UX for a clear "server not reachable" state;
  treat model-absent as a first-class **typed, non-fatal** state (AppFlowy), never a crash.
- **Large-corpus ingest can still crash the external model** — guard batch/embedding jobs and surface
  fallback explicitly.
- **Setup friction is the price of isolation** — and the fix is to *smooth* it, not abandon it: **Reor
  bundles and spawns the server binary itself**, getting crash isolation AND turnkey UX.

## The mobile reality (Android)

Among non-chat, agent-backed apps, **no one embeds a generative model in a notes app's process.** The
consistent choices: point the phone at a **networked server** (Obsidian Copilot, Khoj, Jarvis — the
phone is a thin client, model never on the phone); keep AI **desktop-only** and leave the mobile core
untouched (AppFlowy, Logseq, Joplin's plugin runtime doesn't exist on mobile); reserve in-process for
something **too small to OOM** (a ~25MB MiniLM *embedder*, not generation); and when a model *does* run
on-device, it's a **separate process** (`com.micklab.llama` ships llama.cpp as its own on-device HTTP
server — proving crash-isolation + an OS kill-switch are achievable on Android **without** embedding).

## What this means for formicaria

**The evidence CONFIRMS the review and challenges the in-process-FFI switch.** Every primary-app
analog keeps the generative model out of the notes-app process behind an OpenAI/Ollama-compatible HTTP
seam; the closest twins (Reor, Obsidian+Copilot, AppFlowy, Joplin+Jarvis, Trilium, Khoj) do so
deliberately and by name. The apps that embed in-process are chatbots/demos, cited for exactly the
whole-app SIGSEGV/OOM/LMKD failures — and loss of the OS off-switch — that we feared.

**Honest nuance:** in-process FFI is *technically viable* (LiteRT-LM, AnythingLLM's optional engine),
so "impossible" would overstate it. But everyone who *can* embed still defaults to the external
server, and the one that uses FFI at scale (Khoj) hides it behind a server boundary.

**Clearest recommendation the evidence supports:**
- Keep the model **out-of-process behind the OpenAI/Ollama-compatible HTTP seam as the default and
  load-bearing path** — preserving crash isolation, the OS kill-switch, and free local↔remote swap.
- Do **not** make in-process FFI the runtime. If convenience ever demands a bundled engine, follow
  **Reor's playbook: bundle and spawn the server binary as a supervised child process** (turnkey UX
  *with* isolation) — which is exactly what `SupervisedModel`/`Watchdog` already do.
- Model "no model available" as a **typed, non-fatal state** (AppFlowy's pattern).
- On Android, keep the model **networked or in a separate on-device server process** — never in the
  vault-bearing app process.
