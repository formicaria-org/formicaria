# Agentic model evaluation & selection — methodology-first, benchmark-grounded (2026-07-24)

*The order matters: **how to evaluate** an on-device agentic model comes before **which model wins**.
Supersedes the *ranking axis* of `model-selection-research-2026-07-22.md` (keeps its data). Grounded
in the field's established methods + the most-used leaderboards; every candidate number → a source URL;
vendor-only claims flagged `reputation`. Filtered by our **measured** envelope (`agents/bench/results.md`),
with a stretch tier kept in the game. Product = a ~1–4B on-device **study/research assistant**: read a
note discussion → decide whether to web-search → emit a well-formed search call → read snippets → write
a **grounded, quote-first, format-following** note. Not code; non-thinking preferred.*

---

## Part 1 — How to evaluate our agentic models (the methodology)

Our four load-bearing axes and the **right measurement method** for each (borrow the *method* from the
big benchmarks; the *task content* must be ours — the frontier agent benchmarks are too hard to
separate 1–4B models):

| Axis | Method | Scoring | Method source |
|---|---|---|---|
| Emit a well-formed search/tool call; **decide whether to search** | BFCL-style **AST** + relevance/irrelevance case | deterministic AST match | BFCL v3/v4 |
| **Grounding / faithfulness** (answer only from sources, verbatim quote) | verbatim-quote **substring match** + **eligibility gate** + claim↔quote **entailment judge** | programmatic quote match; LLM judge for entailment only | FACTS Grounding, RAGAS, ARES |
| **Instruction / house-format** following | programmatic rule checks (KaTeX, no whole-fence, citation shape) | deterministic | eval-harness best practice |
| **Robustness + reliability** | paraphrase + malformed-snippet + injected-error variants; **N seeds → pass^k** | deterministic aggregate | BFCL v4, τ-bench (pass^k) |
| **Judge validity** | cross-vendor, order/length-controlled, gold-set-validated, **never self-judge** | — | LLM-judge-bias lit, AgentRewardBench |

**Validity threats the field warns about (and our guards):**
- **Contamination** (public benchmark text leaks into training) → **author our own source packs**, keep a private hold-out; a memorized answer must not pass as "grounded."
- **LLM-as-judge bias** — position, verbosity, **self-preference** → the judge must be a *different, larger* model (never our device model), order/length-controlled, validated against a small human gold set.
- **Trajectory judges are unreliable** (AgentRewardBench: no judge excels; rule-based *under*-reports) → prefer **end-state/programmatic** checks, which our quote-first contract makes unusually easy.
- **Single-run pass@1 hides variance** — small models are exactly where reliability collapses → run N seeds, report **pass^k**.
- **Vendor agent numbers** bundle undisclosed scaffolds/best-of-N/self-judging → reproduce under a **pinned local harness** (ours).

**Verdict on `agents/bench`:** its architecture is *right* and matches the lightweight-eval consensus
(real system prompts + fixed offline cases + programmatic checks, judge-sparingly, temp 0, thinking-trap
handled). It *was* a throughput harness with a counting proxy; **hardened 2026-07-24 into a correctness
eval** (deterministic checks, unit-tested without a model):
1. ✅ **Quote-verbatim + citation check** — each `> quote` is verified as a verbatim substring of the
   source its claim cited; reports `V/Q verbatim` + MIS-CITED + HALLUCINATED-quote. Catches a model
   that emits quote-*shaped* lines it never copied (0 verbatim) — invisible to the old counter.
2. ✅ **Tool/search-call cases** — `4-toolcall-search` (must emit a well-formed `web_search` call) and
   `5-toolcall-nosearch` (must *not* search when the note answers it — relevance detection).
3. ✅ **Abstain case** — `6-abstain`: novel sources that don't answer → must abstain, not fabricate.
4. ⏳ **Deferred** (methodologically noted): a cross-vendor LLM **entailment judge** (needs a judge
   model + gold set) and **pass^k** (N seeds at temp > 0). Deterministic checks cover the highest value first.

*(Done in `agents/bench/run.py` + `cases/`; see `agents/bench/README.md`. Run this hardened harness on
the Part 4 shortlist to make the final call.)*

---

## Part 2 — The most-used leaderboards, and "popular ≠ right for us"

| Board / index | What it measures | Trust for our axes |
|---|---|---|
| **BFCL v3/v4** | tool/function-calling (AST/executable, relevance, multi-turn, web-search, format-sensitivity) | **High** — the closest public proxy for our search-call axis |
| **FACTS Grounding** · **Vectara HHEM** · RAGAS/ARES | grounding/faithfulness to provided context | **High method, thin coverage** — barely lists ≤4B models (only Google Gemmas on FACTS) |
| **IFEval / IFBench / Multi-IF** | verifiable instruction-following | **High** — our primary, densely published |
| **Artificial Analysis Intelligence Index (v4.1)** | 9-eval aggregate: hard reasoning, PhD-knowledge, coding, **1** agentic (τ³-Banking) | **Low for us** — **dropped instruction-following entirely**, barely touches tool-use, ignores faithfulness; over-weights knowledge we offload to search. A coarse capability floor, not a ranking on our axes |
| LMArena Elo · Open LLM Leaderboard (archived) | human-pref chat / static academic | floor / sanity only |

**The key reconciliation:** the *most-used aggregate* (Artificial Analysis) is now the *least aligned*
with our use — it rewards exactly the reasoning/knowledge headroom a search-grounded assistant doesn't
need, and no longer measures instruction-following at all. So **the leaderboards narrow the field; our
own hardened harness makes the call** — especially below 4B, where grounding is essentially unmeasured
publicly.

---

## Part 3 — Candidate landscape (envelope-filtered, llama.cpp-gated, reputation-flagged)

The field moved a lot since our defaults: **successors** (Qwen3.5, Gemma-4, Granite-4.1, Nemotron-3),
**agentic-tuned** entrants (Nanbeige4.1, MiniCPM5), and **viable multimodal** options appeared. Hard
gates: a real **Q4_K_M GGUF + llama.cpp support** and the **measured envelope**.

### 💻 Laptop (~4B on the 4 GB GPU; + stretch)
| Model | IFEval | BFCL | Grounding | Multimodal | Fit (llama.cpp) | Read |
|---|---|---|---|---|---|---|
| **Qwen3-4B-Instruct-2507** *(current)* | 83.4 | **61.9** | Vectara **94.3%** | no | ✓ GPU (39.8 t/s measured) | best-balanced *validated* baseline; AA marks it "deprecated → Qwen3.5-4B" |
| **Nemotron-3-Nano-4B** | 82.8/88.0 | **61.1 (v3)** | none pub. | no | ✓ (`nemotron_h`, official Q4_K_M) | **measured rival** on our axes; run thinking-**off**; NVIDIA custom license |
| **Qwen3-VL-4B-Instruct** | chart-only (verify) | native agent/tool | none pub. | **image (best OCR, DocVQA 95.3)** | ✓ (mmproj) | **multimodal 2-for-1** — joint-pretrain *preserves* text; Apache-2.0 |
| **Gemma-4-E4B** | **unpub.** | **unpub.** (reputation) | unpub. | **image ✓ / audio ✗ in llama.cpp** | ✓ (PR #21309) | strong general scores, unproven on our axes; Apache-2.0 (verify LICENSE) |
| Gemma-3-4B-it | **90.2** | weak/unpub. | FACTS 0.701 (⚠67% answer rate) | image (weaker OCR) | ✓ | top IF, weak tool-calling — note-writer not search-router |
| Nanbeige4.1-3B · Granite-4.1-3B · Qwen3.5-4B | agentic-tuned / concise / successor | — | — | no | ✓ Q4_K_M | worth measuring; on-axis but unvalidated by us |
| **Stretch: Qwen3-8B / Qwen3.5-9B / Granite-4.1-8B** | ↑ | ↑ | Vectara 95.2% (8B) | no | **CPU only (~7–10 t/s)** — >4 GB VRAM | quality-over-speed / future GPU |

**Reputation — not ranked / excluded:** Phi-4-mini, Ministral-3B (API/license), InternLM2.5;
**Nemotron 30B-A3B MoE excluded** (`nemotron_h_moe` unsupported in llama.cpp); Nemotron-9B excluded (>4 GB VRAM).

### 📱 Phone (~2B; + 3B stretch)
| Model | IFEval | BFCL | Multimodal | Fit | Read |
|---|---|---|---|---|---|
| **LFM2.5-1.2B-Instruct** *(current)* | 86.2 | 49.1 | no | ✓ (12.4 t/s measured) | best-balanced non-thinking; AA can't discriminate at 1.2B |
| **LFM2.5-1.2B-Thinking** | **88.4** | **57.0** | no | ✓ (731 MB) | only *published* upgrade — gated on affording `<think>` |
| **MiniCPM5-1B** | — | vendor "tool-use SOTA" (reputation) | no | ✓ (verify arch) | new tool-use claimant — measure head-to-head |
| **Qwen3.5-2B (non-reasoning)** | — | — | no | ✓ | successor to our Qwen3 line; non-thinking variant on-axis |
| **Qwen3-VL-2B-Instruct** | chart-only | native | **image (strong OCR)** | ✓ (mmproj) | multimodal brain if we want image-notes; verify text numbers |
| **Gemma-4-E2B** | unpub. | unpub. | image ✓ / audio ✗ | ✓ | only multimodal phone option; unproven on our axes |
| Granite-3.3-2B-Instruct | 65.8 | unpub. | no | ✓ | clean non-thinking control |
| **Stretch: Llama-3.2-3B / Qwen3-4B-CPU** | 77.4 | 67 (v2) | no | ~5–7 t/s edge | patient-quality / future faster phone |

**The multimodal ruling (your point, resolved):** multimodality **can** coexist — but it's a *recipe*
question. **Jointly-pretrained** VLMs (Qwen3-VL, Gemma-4) keep text/tool quality and are a real 2-for-1;
**bolt-on** VLMs regress — **LFM2.5-VL-1.6B measures IFEval 71.89 vs 86.23** for its text-only sibling
(−14 pts). So: *prefer Qwen3-VL / Gemma-4; do not adopt LFM2.5-VL as the single brain* (fine as a vision
sidecar). Reject the bolt-on recipe, not the modality.

## Part 4 — Shortlist to benchmark (the download list; all confirmed GGUF + llama.cpp)

**Harden `agents/bench` first (Part 1), then measure — one model at a time, memory-capped, never
alongside the running agent (the crash rule, [[benchmarking-safety-one-model-at-a-time]]).**

*All repos/files **verified on HF 2026-07-24** (exact filenames below — note Qwen's `Qwen3VL` has no dash).*

| Tier | Model | GGUF repo → file (verified) | Why it's on the list |
|---|---|---|---|
| Laptop | **Nemotron-3-Nano-4B** | `nvidia/NVIDIA-Nemotron-3-Nano-4B-GGUF` → `NVIDIA-Nemotron3-Nano-4B-Q4_K_M.gguf` | *measured* IFEval/BFCL rival to Qwen3-4B; run thinking-off |
| Laptop | **Qwen3-VL-4B-Instruct** | `Qwen/Qwen3-VL-4B-Instruct-GGUF` → `Qwen3VL-4B-Instruct-Q4_K_M.gguf` (+ `mmproj-Qwen3VL-4B-Instruct-F16.gguf`) | multimodal 2-for-1, best OCR, Apache-2.0 |
| Laptop | Gemma-4-E4B *(if vision valued)* | `unsloth/gemma-4-e4b-it-GGUF` → `gemma-4-E4B-it-Q4_K_M.gguf` (ggml-org has Q4_0 only; +mmproj for vision) | image bonus; tool-calling unproven → harness decides |
| Laptop *(stretch)* | Qwen3-8B | `unsloth/Qwen3-8B-GGUF` → `Qwen3-8B-Q4_K_M.gguf` | quality-over-speed (CPU ~7–10 t/s) / future GPU |
| Phone | **LFM2.5-1.2B-Thinking** | `LiquidAI/LFM2.5-1.2B-Thinking-GGUF` → `LFM2.5-1.2B-Thinking-Q4_K_M.gguf` (731 MB) | published upgrade; measure `<think>` cost |
| Phone | **MiniCPM5-1B** | `openbmb/MiniCPM5-1B-GGUF` → `MiniCPM5-1B-Q4_K_M.gguf` | new tool-use claimant (confirm arch runs in our llama.cpp b10081) |
| Phone | Qwen3-VL-2B / Gemma-4-E2B *(if vision)* | `Qwen/Qwen3-VL-2B-Instruct-GGUF` → `Qwen3VL-2B-Instruct-Q4_K_M.gguf` (+mmproj) / `unsloth/gemma-4-e2b-it-GGUF` → `gemma-4-E2B-it-Q4_K_M.gguf` | multimodal phone brain |
| Phone *(stretch)* | Llama-3.2-3B | `unsloth/Llama-3.2-3B-Instruct-GGUF` → `Llama-3.2-3B-Instruct-Q4_K_M.gguf` | slow-edge quality |
| Baselines (already local) | Qwen3-4B-Instruct-2507 · lfm2.5-1.2b · Granite-3.3-2B (`ibm-granite/granite-3.3-2b-instruct-GGUF`) · Gemma-3-4B (`unsloth/gemma-3-4b-it-GGUF`) · Lucy (`Menlo/Lucy-gguf`) | — | anchors for the head-to-head |

## Bottom line
Our current picks (Qwen3-4B-Instruct-2507 laptop, LFM2.5-1.2B phone) remain **well-chosen and the only
ones we've *validated***. The field now offers real challengers — **Nemotron-3-Nano-4B** (measured, on
our axes), **Qwen3-VL** (the multimodal 2-for-1 that preserves text), and the successor/agentic-tuned
lines — but the honest position is: **published data below 4B can't decide, the popular index (AA) no
longer measures our axes, and multimodal quality is recipe-dependent.** So the deliverable is a
*measurement plan*: harden the harness (Part 1), then run this shortlist through it (Part 4). Leaderboards
narrow; our hardened, quote-verifying, tool-call-checking harness decides.

### Sources
Methodology: [BFCL](https://sky.cs.berkeley.edu/project/berkeley-function-calling-leaderboard/) · [BFCL v4 web-search](https://gorilla.cs.berkeley.edu/blogs/15_bfcl_v4_web_search.html) · [τ-bench 2406.12045](https://arxiv.org/abs/2406.12045) · [FACTS Grounding](https://deepmind.google/blog/facts-grounding-a-new-benchmark-for-evaluating-the-factuality-of-large-language-models/) · [RAGAS](https://docs.ragas.io/en/stable/concepts/metrics/available_metrics/) · [ARES 2311.09476](https://arxiv.org/abs/2311.09476) · [RAGTruth 2401.00396](https://arxiv.org/pdf/2401.00396) · [LLM-judge bias](https://llm-judge-bias.github.io/) · [AgentRewardBench 2504.08942](https://arxiv.org/pdf/2504.08942) · [contamination 2406.04244](https://arxiv.org/abs/2406.04244) · [AA Index v4.1](https://artificialanalysis.ai/articles/artificial-analysis-intelligence-index-v4-1) · [GAIA](https://ar5iv.labs.arxiv.org/html/2311.12983)
Models: [Qwen3-4B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507) · [Nemotron-3-Nano-4B](https://huggingface.co/nvidia/NVIDIA-Nemotron-3-Nano-4B-BF16) · [Gemma 4 card](https://ai.google.dev/gemma/docs/core/model_card_4) · [Gemma-4 vision PR](https://github.com/ggml-org/llama.cpp/pull/21309) · [Qwen3-VL-4B](https://huggingface.co/Qwen/Qwen3-VL-4B-Instruct-GGUF) · [Qwen3-VL report 2511.21631](https://arxiv.org/pdf/2511.21631) · [LFM2.5-VL-1.6B](https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B) · [LFM2.5-1.2B-Thinking](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Thinking) · [Granite-3.3-2B](https://huggingface.co/ibm-granite/granite-3.3-2b-instruct) · [AA tiny board](https://artificialanalysis.ai/models/open-source/tiny)
