# Best on-device research model — evidence-based pick

*Grounded in our measured llama-bench envelope (Dimensity 7300 phone; i7-11800H + RTX 3050 laptop), Q4_K_M, 2026-07-22. Vendor-neutral. Only models with a real Q4_K_M GGUF are considered.*

## 1. Which scores decided it

For an on-device **research assistant that routes to web search, summarizes faithfully, and writes tidy notes — explicitly not code**, the deciding metrics, most-decisive first, are: **(1) BFCL-v3 tool-calling** ([Berkeley Function-Calling Leaderboard](https://gorilla.cs.berkeley.edu/leaderboard.html) / [llm-stats](https://llm-stats.com/benchmarks/bfcl-v3)) — the agent wins or loses on whether it reliably emits a well-formed search call; **(2) IFEval** ([HF Open LLM Leaderboard v2](https://huggingface.co/spaces/open-llm-leaderboard/open_llm_leaderboard)) — verifiable-constraint obedience, i.e. "summarize in 3 bullets, cite sources"; **(3) agentic factual-QA-with-search** (tau-bench + SimpleQA-with-search) — the real call-tool→read→answer loop, where 1.7B models reach ~78% vs single digits without a tool; **(4) faithful summarization** (IFBench, FACTS Grounding — no single canonical board); **(5) chat/prose quality** (Arena-Hard-v2, Creative-Writing-v3, LMArena Elo) as a readability sanity check. We **deliberately ignore and actively penalize coding** (HumanEval/MBPP/LiveCodeBench) since a code-tuned model is a minus here, and we **de-emphasize parametric knowledge** (MMLU-Pro, GPQA, MATH, MUSR, BBH) to a floor only — a search tool covers factual gaps, so knowledge-from-weights is not the axis. Net rule: rank by **BFCL-v3 × IFEval**, confirm with tau-bench/SimpleQA-with-search, then filter by the hardware envelope (**phone ≤ ~1.5B Q4 / ~1.5 GB; laptop ≤ ~4B Q4**).

## 2. Shortlist (deduped across clusters, best-first within tier)

### Phone tier (≤ ~1.5B Q4, ~1.5 GB, must stream at 4 threads on the Dimensity 7300)

| Candidate | Params | Vendor | Q4_K_M GGUF? | Key scores (source) | Phone / Laptop | Research strength | Code-tuned? |
|---|---|---|---|---|---|---|---|
| **LFM2.5-1.2B-Instruct** | 1.2B | Liquid AI | Yes — `LiquidAI/LFM2.5-1.2B-Instruct-GGUF` | IFEval **86.2**, IFBench 47.3, BFCL-v3 **49.1**, Multi-IF 61.0, MMLU-Pro 44.4 ([model card](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct)) | measured **13.5 tok/s** phone / 48 laptop, ~1.4 GB | Best-in-tier IF **and** tool-calling; purpose-built on-device agent; Tool/RAG siblings | No |
| **Lucy** (Qwen3-1.7B fine-tune) | 1.7B | Menlo Research | Yes — `Menlo/Lucy-gguf` | **SimpleQA 78.3% with search** ([arXiv 2508.00360](https://arxiv.org/pdf/2508.00360)); IFEval/BFCL unpublished | ~10-12 tok/s phone (borderline, ~1.1 GB) / comfy laptop | Trained purely to Google-and-browse on mobile; the most on-point search agent | No |
| **Qwen3-1.7B** | 1.7B | Alibaba (Qwen) | Yes — `Qwen/Qwen3-1.7B-GGUF` | IFEval 73.7, BFCL-v3 46.3, Multi-IF 56.5, IFBench 21.3 ([LFM2.5 card head-to-head](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct); [arXiv 2505.09388](https://arxiv.org/pdf/2505.09388)) | ~10-12 tok/s phone (borderline) / comfy laptop | Solid generalist + native FC; loses to LFM2.5-1.2B on IF & tools despite being larger | No |
| **Qwen3-0.6B** | 0.6B | Alibaba (Qwen) | Yes — `Qwen/Qwen3-0.6B-GGUF` | Per-model IFEval/BFCL unpublished; 1.7B sibling 73.7/46.3 as family proxy ([card](https://huggingface.co/Qwen/Qwen3-0.6B)) | >40 tok/s phone (core sweet spot) / 200+ laptop | Fast fallback with real FC; lean on search for facts | No |
| **Gemma 3 1B-it** | 1B | Google DeepMind | Yes — `google/gemma-3-1b-it-qat` | IFEval 63.3 (re-measured), BFCL-v3 **16.6**, IFBench 20.5 ([LFM2.5 card](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct); [arXiv 2503.19786](https://arxiv.org/html/2503.19786v1)) | ~15-18 tok/s phone / laptop | Clean prose/summaries but BFCL ~17 is a liability for a tool-router | No |
| **LFM2.5-350M** | 0.35B | Liquid AI | Yes — `LiquidAI/LFM2.5-350M-GGUF` | BFCL-v3 **44.1**, IFBench 40.7, CaseReportBench 32.5 ([blog](https://www.liquid.ai/blog/lfm2-5-350m-no-size-left-behind)) | measured **42 tok/s** phone / 148 laptop, ~0.45 GB | Remarkable tool-caller for 350M; weak free-form writer/knowledge | No |
| SmolLM2-1.7B-Instruct | 1.7B | Hugging Face | Yes — `HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF` | IFEval 56.7, BFCL 27, MT-Bench 6.13 ([card](https://huggingface.co/HuggingFaceTB/SmolLM2-1.7B-Instruct)) | ~9-11 tok/s phone / laptop | Behind the tier on IF & tools; pick only if fully-open data matters | No |
| Qwen2.5-1.5B-Instruct | 1.5B | Alibaba (Qwen) | Yes — `Qwen/Qwen2.5-1.5B-Instruct-GGUF` | IFEval 42.5, BFCL overall ~48.8 ([arXiv 2412.15115](https://arxiv.org/pdf/2412.15115)) | ~11-12 tok/s phone / laptop | Superseded by Qwen3; low IFEval | No |
| Llama-3.2-1B-Instruct | 1B | Meta | Yes — `bartowski/Llama-3.2-1B-Instruct-GGUF` | IFEval 52.4, BFCL-v3 21.4 ([LFM2.5 card](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct)) | ~15 tok/s phone / laptop | Weakest 1B for this use; 2024 baseline | No |

### Laptop tier (≤ ~4B Q4, fits 5 GB RAM / 4 GB VRAM)

| Candidate | Params | Vendor | Q4_K_M GGUF? | Key scores (source) | Phone / Laptop | Research strength | Code-tuned? |
|---|---|---|---|---|---|---|---|
| **Qwen3-4B-Instruct-2507** | 4B | Alibaba (Qwen) | Yes — `Qwen/Qwen3-4B-Instruct-2507-GGUF` | IFEval **83.4**, BFCL-v3 **61.9**, TAU1-Retail 48.7, Arena-Hard-v2 43.4, Creative-Writing 83.5, Multi-IF 69.0 ([card](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507)) | laptop only, ~15-20 tok/s, ~2.5 GB | Top small-model IF **and** BFCL simultaneously + real tau-bench; the base under Jan-nano/Jan-v1 | No |
| **Jan-v1-4B** (Qwen3-4B-Thinking fine-tune) | 4B | Menlo/Jan | Yes — `janhq/Jan-v1-4B-GGUF` | **SimpleQA 91.1% with search** ([Jan docs](https://www.jan.ai/docs/desktop/jan-models/jan-v1)); BFCL/IFEval unpublished | laptop only, ~15-20 tok/s | Highest agentic factual-QA in the list; open Perplexity-Pro alternative; thinking-budget latency | No |
| **Gemma 3 4B-IT** | 4B | Google DeepMind | Yes — `google/gemma-3-4b-it-qat-q4_0` | IFEval **90.2**, FACTS Grounding 70.1 ([arXiv 2503.19786](https://arxiv.org/pdf/2503.19786)); **BFCL unpublished** | laptop only, ~15-20 tok/s | Best IF + faithful-summarization; but weakest documented tool-caller (no native FC template) | No |
| **Jan-nano-4B** (Qwen3-4B-Base fine-tune) | 4B | Menlo | Yes — `Menlo/Jan-nano-gguf` | SimpleQA 83.2% via MCP ([arXiv 2506.22760](https://arxiv.org/html/2506.22760v1)) | laptop only | Non-thinking MCP deep-research; lighter/faster than Jan-v1 | No |
| **Llama-3.2-3B-Instruct** | 3B | Meta | Yes — `bartowski/Llama-3.2-3B-Instruct-GGUF` | IFEval 77.4, BFCL-v2 tool-use 67.0 ([Meta evals](https://huggingface.co/datasets/meta-llama/Llama-3.2-3B-Instruct-evals)) | laptop; **borderline phone** (~2 GB, ~5-6 tok/s) | Mature FC template, widest tooling; older (2024), no clean BFCL-v3 | No |
| SmolLM3-3B | 3B | Hugging Face | Yes — `ggml-org/SmolLM3-3B-GGUF` | Dual-mode evals on card; **no BFCL/SimpleQA row** ([card](https://huggingface.co/HuggingFaceTB/SmolLM3-3B)) | laptop, ~20-25 tok/s | Strongest fully-open (weights+data+recipe), long context; general workhorse | No |
| Phi-4-mini-instruct | 3.8B | Microsoft | Yes — `unsloth/Phi-4-mini-instruct-GGUF` | IFEval ~73.8, MMLU 67.3; **BFCL claimed, unpublished** ([card](https://huggingface.co/microsoft/Phi-4-mini-instruct)) | laptop, ~15-20 tok/s | Skews math/reasoning (axes we don't weight); brittle open-ended | No (math-leaning) |
| Qwen2.5-3B-Instruct | 3B | Alibaba (Qwen) | Yes — `Qwen/Qwen2.5-3B-Instruct-GGUF` | IFEval 58.2; BFCL unpublished ([blog](https://qwenlm.github.io/blog/qwen2.5/)) | laptop | Superseded by Qwen3-4B-2507 | No |

## 3. The pick — PHONE: **LFM2.5-1.2B-Instruct** (Liquid AI)

It is the only model that tops **both** deciding axes in the phone tier: **IFEval 86.2** (highest of any model in this study, above Qwen3-4B's 83.4) and **BFCL-v3 49.1** — far above every other ~1-1.7B option (Qwen3-1.7B 46.3, Gemma-3-1B 16.6, SmolLM2 27, Llama-3.2-1B 21.4) ([Liquid card](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct)). That combination — reliably format a search call **and** obey "3 bullets, cite sources" — is exactly the research-note loop. It is in **our own measured set**: **13.5 tok/s** decode at 4 big cores (streams faster than reading) and **~1.4 GB resident**, sitting *inside* the 1.5B/1.5 GB ceiling, not against it. Dedicated `LFM2-1.2B-Tool` / `-RAG` GGUF siblings exist if we later want to specialize. Parametric knowledge is weak (MMLU-Pro 44) — precisely the trade this use accepts, because search covers facts.

**Runner-up: Lucy** (Menlo, 1.7B). Its **SimpleQA 78.3% with a search tool** ([arXiv 2508.00360](https://arxiv.org/pdf/2508.00360)) is the most direct evidence of real on-phone research ability, and it was *built* to Google-and-browse on mobile CPU. It slips to second only because it is borderline on the envelope (1.7B, ~10-12 tok/s, just over the smooth 1.5B band) and publishes no IFEval/BFCL — a specialized search agent rather than a well-rounded note-taker. Keep it in reserve if in-practice search routing ever beats LFM2.5's numbers.

## 4. The pick — LAPTOP: **Qwen3-4B-Instruct-2507** (Alibaba)

The best-documented all-rounder at the 4B ceiling and the only laptop model strong on **both** deciding axes: **IFEval 83.4** and **BFCL-v3 61.9** — the highest openly-published tool-calling of the sub-4B general field — plus real tau-bench agentic numbers (TAU1-Retail 48.7) and genuinely good prose (Arena-Hard-v2 43.4, Creative-Writing 83.5) for readable summaries ([card](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507)). It is the **non-thinking Instruct** variant — predictable streaming latency, no wasted `<think>` budget for a search-router. At ~4B Q4 it runs **~15-20 tok/s on the i7 CPU (8 threads)** at ~2.5 GB resident, inside the measured 4B/4GB ceiling. *(BFCL caveat: a public board shows ~33, likely BFCL-v4; treat 61.9 as vendor-reported v3 — still the strongest v3 figure here.)*

**RTX 3050 / CUDA upside:** a ~2.5 GB Q4 fits the 4 GB VRAM with headroom, so a future CUDA llama.cpp build (a known pending win) would move this from ~15-20 tok/s to comfortably interactive — the single highest-leverage laptop improvement available.

**Runner-up: Jan-v1-4B** (Menlo, Qwen3-4B-Thinking fine-tune). Its **SimpleQA 91.1% with web search** ([Jan docs](https://www.jan.ai/docs/desktop/jan-models/jan-v1)) is the highest agentic factual-QA number in the entire study and it is purpose-built as a local Perplexity-Pro. It is runner-up rather than the pick because it is a *thinking* model — slower first token and heavier per-answer cost — and publishes no IFEval/BFCL, making it less predictable for tidy-note formatting. If deep multi-source research outweighs latency, swap it in; it is built on the very same Qwen3-4B-2507 tool skill. *(Gemma 3 4B-IT is the honorable third: IFEval 90.2 and a real FACTS-Grounding 70.1 make it the faithful-summarization champion, but its unpublished BFCL and prompt-engineered-only tool use disqualify it on the decisive axis.)*

## 5. vs. our current LFM2.5 default

Our default is **`lfm2.5-230m`**, stepping up to 350M then 1.2B (`agents/models.toml`). Honest read: **the LFM2.5 line is already the right vendor for the phone — but the default is one or two sizes too small for a *research* agent.**

- **230M / 350M** have no published IFEval; the 350M's headline is **BFCL-v3 44.1 / IFBench 40.7** ([blog](https://www.liquid.ai/blog/lfm2-5-350m-no-size-left-behind)) — genuinely strong tool-routing for its weight, and it runs effectively instant (42 tok/s, ~0.45 GB). It is an excellent *always-on tool-caller*, but a weak free-form writer and knowledge source. For "decide to search, format the call, extract a field, write one tidy line" it is fine; for faithful multi-source summarization and instruction-following it is under-powered.
- **LFM2.5-1.2B** is the same family and still comfortably on-device (13.5 tok/s, ~1.4 GB) but adds **IFEval 86.2 and BFCL-v3 49.1** — a large, measured jump on exactly the two axes that decide this use, at ~3× the RAM and ~3× slower (still faster than reading).

So the recommendation is **not** "abandon LFM2.5" — it is **"promote the phone default from 230M to the 1.2B within the same line."** No other phone-tier model beats LFM2.5-1.2B on IF+BFCL together. The only thing LFM2.5 does *not* cover is the laptop tier: there is no 4B LFM2.5, so the laptop step-up genuinely goes to **Qwen3-4B-Instruct-2507**, which outscores everything at that size on tool-calling. Keep 230M/350M as the fast always-on floor; make 1.2B the phone research default; add Qwen3-4B-2507 as the laptop entry.

## 6. Action — `agents/models.toml`

The 1.2B entry **already exists** and its repo/filename are correct. Two concrete changes: (a) point the phone research default at it, and (b) add the laptop-tier Qwen3-4B entry. **Phone and laptop should differ** — no single model spans both the 1.5 GB phone ceiling and the 4B laptop headroom, and the owner's "lightest that works" stance means the phone should not carry a 4B.

Keep `default = "lfm2.5-230m"` as the instant floor (it is intentionally the lightest), but the **research** picks are:

**Phone research pick — already present, no file change needed:**
```toml
[[models]]
name = "lfm2.5-1.2b"
repo = "LiquidAI/LFM2.5-1.2B-Instruct-GGUF"
file = "LFM2.5-1.2B-Instruct-Q4_K_M.gguf"
note = "PHONE research default — IFEval 86.2 / BFCL-v3 49.1, top of tier; measured 13.5 tok/s, ~1.4 GB on the Dimensity 7300."
```

**Laptop research pick — add this entry:**
```toml
[[models]]
name = "qwen3-4b-2507"
repo = "Qwen/Qwen3-4B-Instruct-2507-GGUF"
file = "Qwen3-4B-Instruct-2507-Q4_K_M.gguf"
note = "LAPTOP research pick — IFEval 83.4 / BFCL-v3 61.9 (best small-model tool-calling); ~15-20 tok/s CPU on the i7, ~2.5 GB. Fits the RTX 3050 4 GB for a future CUDA build. Non-thinking Instruct (not Qwen3-Coder, not Thinking-2507)."
```

**Optional laptop runner-up** (deep-research, thinking): `janhq/Jan-v1-4B-GGUF` → `Jan-v1-4B-Q4_K_M.gguf` — SimpleQA 91.1% with search, at the cost of thinking-budget latency.

*Verify the exact Q4_K_M filenames at fetch time against the repo file lists ([Qwen3-4B-2507-GGUF](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507-GGUF)); the LFM2.5-1.2B filename is confirmed against the existing working entry. Do not use the `Qwen3-Coder`, `Qwen3-4B-Thinking-2507`, or any `-Coder` sibling — code-tuning is a negative for this use.*