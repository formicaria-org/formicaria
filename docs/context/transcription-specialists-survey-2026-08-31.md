# Specialist vs. general models for transcription — a literature survey (2026-08-31)

**The question, as asked:** *are there open-source specialized tools competitive against
general-purpose multimodal LLMs at image→text (transcription) and voice→text (transcription)?*

**This doc is a survey, not a ruling.** It was commissioned **unconstrained** — no filter for our
hardware, licences or packaging rules — so it makes **no recommendation and no pick**, and it earns
no `decisions.md` entry. It is a reference for what the literature claims and how well each claim is
evidenced. The decision-shaped companion for audio is
[`audio-asr-research-2026-07-23.md`](./audio-asr-research-2026-07-23.md); the image side has no
decision doc, which is part of why this exists.

## The short answer

| Modality | Are specialists competitive? | The honest shape of it |
|---|---|---|
| **Image → text** | **Yes — overwhelmingly, on efficiency; and the current SOTA *is* a specialist.** | But "specialist" has quietly come to mean *a small VLM trained on one task*. Against a **frontier** general VLM the accuracy gap is small or even reversed; against **cost** it is 2–3 orders of magnitude. The durable specialist edge is **hallucination resistance** and **localisation**, not raw edit distance. |
| **Voice → text** | **Yes, and they own the efficiency frontier by 10–100×.** | But the framing "specialists vs. general multimodal LLMs" is **now wrong at the top**: general omni-modal LLMs (Qwen3.5-Omni) post ASR numbers at or below the dedicated leaders. Specialists win throughput, size and licence — not, any longer, accuracy. |

The single most useful finding in this survey is that **the dichotomy in the question is dissolving
in both modalities, in opposite directions.** For images, the winners are specialists that are
architecturally general VLMs. For audio, the winners are generalists that have absorbed the
specialist's job.

## Method, and how much to trust what follows

- **Metrics.** Image: normalised **edit distance** (lower better), **TEDS** (tables), **CDM**
  (formulae — a render-and-compare metric designed because BLEU/edit distance mis-score LaTeX,
  [CDM paper](https://arxiv.org/html/2409.03643v1)), **Hmean** (detection). Handwriting: **CER/WER**.
  Audio: **WER** and **RTFx** (inverse real-time factor — higher is faster).
- **Benchmarks leaned on.** [OmniDocBench](https://github.com/opendatalab/OmniDocBench) (CVPR 2025)
  and its 1.5–1.7 revisions · [olmOCR-Bench](https://arxiv.org/abs/2510.19817) (binary unit-test
  rules rather than fuzzy reference matching) · OCRBench v2 · IAM / LAM / READ2016 (handwriting) ·
  [Open ASR Leaderboard](https://arxiv.org/abs/2510.06961) (86 systems, 12 datasets).
- **Provenance warning, applied throughout.** Most numbers below are **from the proposing paper's
  own table**. Where a vendor benchmarks on a benchmark it curated, this doc says so inline. A few
  entries are flagged `[partially verified]` where the PDF's tables did not extract.
- **A standing caution the field has written down about itself:**
  [*Position: State-of-the-Art Claims Require State-of-the-Art Evidence*](https://arxiv.org/abs/2605.17273)
  finds that across ten cross-domain benchmarks, **in more than half of top-model comparisons at
  least one assumed property of superiority fails** (meaningful effect size, consistency across
  tasks, robustness to dropping a dataset), and that "aggregate gains are frequently driven by
  outlier datasets". Read every table below with that in hand.

---

# Part I — Image → text

## 1. The taxonomy the word "specialist" now hides

Three different things get called "a specialist OCR model", and conflating them is what makes the
question hard to answer:

**Family A — pipeline specialists.** Detect-then-recognise, CNN/CTC, no language model, millions
not billions of parameters. Tesseract, **PaddleOCR / PP-OCRv5 / PP-OCRv6**, docTR, EasyOCR, Surya,
RapidOCR. These are specialists in the old sense: a different *kind* of model.

**Family B — task-specialised small VLMs.** End-to-end page → Markdown, ViT + LLM decoder.
**dots.ocr**, **olmOCR-2**, **HunyuanOCR**, **LightOnOCR-2**, **GLM-OCR**, PaddleOCR-VL, MinerU2.5,
DeepSeek-OCR, FireRed-OCR, Qianfan-OCR, OvisOCR2, Nougat, Marker. **These are multimodal LLMs.**
What makes them "specialist" is training data and output contract, not architecture.

**Family C — sub-domain specialists.** UniMERNet / pix2tex / Texify / DocTron-Formula / Uni-MuMER
(maths → LaTeX), TrOCR / HTR-VT / HTR-ConvText (handwriting), table and layout models.

Family B is where the current state of the art lives, and it is the reason a naive reading of
"specialist beats LLM" is misleading: the winning specialist *is* an LLM, just a small one pointed
at one job.

## 2. Family A — the strongest form of the "small beats huge" claim

[**PP-OCRv5**](https://arxiv.org/abs/2603.24373v1) (CVPR 2026) is the paper that states the thesis
most baldly — its title is *"A Specialized 5M-Parameter Model Rivaling Billion-Parameter
Vision-Language Models on OCR Tasks"*. Its own OmniDocBench table (overall normalised edit distance,
**lower is better**):

| System | Params | Overall ↓ | English ↓ | Chinese ↓ |
|---|---|---|---|---|
| **PP-OCRv5** | **5 M** | 0.067 | 0.058 | 0.076 |
| Qwen3-VL | 235 B | **0.026** | **0.016** | — |
| InternVL2 | 76 B | 0.115 | — | — |
| GPT-4o | — | 0.122 | — | 0.125 (mixed) |
| Qwen2-VL | 72 B | 0.173 | — | 0.274 |

**Read this table against the title.** PP-OCRv5 at 5 M parameters beats GPT-4o and two 72–76 B open
VLMs — but it **loses to Qwen3-VL-235B by a factor of 2.6**. The defensible claim is *five million
parameters land you within striking distance of models tens of thousands of times larger*, which is
an efficiency result, not a superiority result. The paper's qualitative claims of "superior
localization precision and reduced hallucinations" are **not quantified anywhere in it**; the
authors attribute the gains to a data-centric programme (difficulty, accuracy, diversity) rather
than architecture.

[**PP-OCRv6**](https://arxiv.org/pdf/2606.13108) supplies the missing quantification, on a
**benchmark the authors curated themselves** — weigh accordingly. Variants: tiny **1.5 M**, small
**7.7 M**, medium **34.5 M**.

| Model | Text-hallucination accuracy ↑ | Recognition (weighted) ↑ | Detection Hmean ↑ |
|---|---|---|---|
| **PP-OCRv6_medium (34.5 M)** | **93.2 %** | **83.2 %** | **86.2 %** |
| Kimi-K2.6 | 85.0 % | — | — |
| Qwen3-VL-235B | 80.6 % | 74.9 % | — |
| GPT-5.5 | 78.0 % | — | — |
| MiniMax-M3 | 72.6 % | — | — |
| Gemini-3.1-Pro | — | — | 46.8 % |

Speed, Intel Xeon + OpenVINO, end-to-end: **PP-OCRv6_medium 1.40 s/image vs. PP-OCRv5_server
7.30 s/image**; tiny **0.20 s/image**; ~6.1× speedup over PP-OCRv5_mobile on an Apple M4. The
detection gap (86.2 vs. 46.8 Hmean) is the most striking number in the image half of this survey and
the least independently corroborated one.

The paper's own framing is modest and worth quoting: lightweight specialised systems "offer a
**practical and effective alternative** for production OCR deployment in the large-model era" — an
alternative, not a conquest.

## 3. Family B — where the state of the art actually is

| Model | Params | Headline result | Notes |
|---|---|---|---|
| [**HunyuanOCR**](https://arxiv.org/html/2511.19575v1) | **1 B** (0.4 B ViT + 0.5 B LM + adapter) | OmniDocBench **94.10** overall; text-spotting **70.92** vs. Qwen3-VL-235B **53.62**, Gemini-2.5-Pro **23.44** | But **loses OCRBench IE/VQA**: 860 vs. Qwen3-VL-235B's **920**. Beats a 235 B model at transcription, loses to it at understanding. |
| [**dots.ocr**](https://arxiv.org/html/2512.02498v1) | 2.9 B (1.2 B ViT + 1.7 B dec.) | OmniDocBench **0.125 EN / 0.160 ZH** vs. Gemini-2.5-Pro 0.148/0.212, Qwen2.5-VL-72B 0.214/0.261, GPT-4o 0.233/0.399 | **126 languages**; on its XDocParse benchmark, 0.177 vs. Gemini-2.5-Pro 0.251 (29.5 % relative). |
| [**olmOCR-2**](https://arxiv.org/abs/2510.19817) | 7 B (Qwen2.5-VL-7B fine-tune) | olmOCR-Bench **82.4** vs. GPT-4o **68.9 ± 1.1**, Gemini Flash 2 **57.8 ± 1.1** | Trained by **unit-test rewards**; the benchmark's binary rules dodge fuzzy-reference and LLM-judge noise. |
| [**LightOnOCR-2-1B**](https://arxiv.org/html/2601.14251v1) | **1 B** | olmOCR-Bench **83.2 ± 0.9** vs. Chandra-9B 81.7, olmOCR-2-8B 80.4, Qwen2.5-VL-8B 64.3 | **5.71 pages/s on one H100** (~20.6 k pages/h) vs. olmOCR-2 3.28, Chandra 1.70. **Apache-2.0.** |
| GLM-OCR | — `[partially verified]` | reported ~94.6 on the OmniDocBench composite | CC-BY-4.0; tables did not extract from the PDF. |
| OvisOCR2 | — `[partially verified]` | reported **96.58** on OmniDocBench v1.6 | Search-summary only. |

**LightOnOCR-2 is the most theoretically interesting entry in this survey.** It is trained by
**knowledge distillation from a general VLM teacher** — upgraded from Qwen2-VL-72B to
**Qwen3-VL-235B-A22B** — over 43 M rendered pages. So a 1 B specialist that outperforms 8–9 B
specialists is, mechanically, **a compressed general-purpose multimodal LLM**. That is the
convergence in its purest form: the specialist is not a rival lineage, it is a distillate.

## 4. Family C — the sub-domains, where the answer splits

**Handwriting — the clearest architectural specialist win by CER.**
[HTR-ConvText](https://arxiv.org/html/2512.05021v1) (**65.9 M** params, ResNet-18 + MobileViT +
ConvText encoder):

| Method | IAM CER / WER | LAM CER / WER | READ2016 CER / WER |
|---|---|---|---|
| **HTR-ConvText** | **4.0 / 12.9** | **2.7 / 7.0** | **3.6 / 15.7** |
| HTR-VT | 4.7 / 14.9 | 2.8 / 7.4 | 3.9 / 16.5 |
| TrOCR | 7.3 / 37.5 | 3.6 / 11.6 | — |
| DAN | — | — | 4.1 / 17.6 |

**But this table has no LLM in it** — the specialist HTR literature does not benchmark against
multimodal LLMs at all. The paper that does,
[*Benchmarking LLMs for Handwritten Text Recognition*](https://arxiv.org/abs/2503.15195) (English,
French, German, Italian; modern and historical), reaches a deliberately unheroic conclusion:
comparisons with Transkribus show **"no consistent advantage for either approach"**, MLLMs do well
on *modern* handwriting, they carry an **English language bias**, and — the finding that cuts
against open source specifically — **proprietary models, especially Claude 3.5 Sonnet, outperform
open-source alternatives in zero-shot settings.** Also: LLMs show "limited ability to autonomously
correct errors in zero-shot transcriptions."

**Mathematics — genuinely mixed, and it depends on printed vs. handwritten.** On OmniDocBench's
CDM metric, **GPT-4o 86.8 ≈ Mathpix 86.6 ≥ UniMERNet 85.0** — the general model *matches the
specialist*. Yet [UniMERNet](https://arxiv.org/html/2404.15254v1) crushes the older specialists on
its own UniMER-Test (ExpRate 0.4799 / CDM 0.968 vs. Texify 0.2288 / 0.755; render success 97.62 %
vs. Texify 94.97 %, pix2tex 86.17 %), DocTron-Formula reportedly beats UniMERNet while general VLMs
*generalise better than UniMERNet* on hard sets, and on **handwritten** maths
[Uni-MuMER](https://arxiv.org/html/2505.23566) reports beating Gemini-2.5-flash by **24.42 %**
zero-shot. Current CDM leaders are Family B: HunyuanOCR 93.28, PaddleOCR-VL 91.11, dots.ocr 90.77,
Gemini 3 90.27.

**Multilingual.** [MORE](https://arxiv.org/pdf/2607.02956) (22 languages) concludes that
**"neither specialist nor general approaches uniformly dominate"**: specialists are stronger on
high-resource languages with established training data; general VLMs adapt better across diverse
writing systems and low-resource languages. `[partially verified — exact tables did not extract]`

## 5. Where general VLMs still win, and the failure-mode shift

**The specialist's structural advantage is not accuracy — it is the failure mode.** Generative OCR
"shifts the core risk from *misrecognition* to *hallucination*": a pipeline model that cannot read a
word emits garbage you can see, while a VLM emits fluent plausible text you cannot. That is what the
PP-OCRv6 hallucination column is measuring, and it is why the advantage survives even where the
edit-distance advantage does not.

**The general VLM's advantages**, from the same literature: instruction-following on messy or mixed
input; anything past transcription into semantics (HunyuanOCR losing OCRBench IE/VQA 860 to 920 is
the clean demonstration); low-resource scripts (MORE); zero-shot handwriting (2503.15195); and no
pipeline to maintain — LightOnOCR's own pitch is a "single unified model … without fragile
multi-stage pipelines."

**And Family B has its own documented failures**: MinerU2-VLM and dots.ocr are criticised for low
visual-token efficiency, **reading-order confusion**, and difficulty with long or complex documents;
visual-token cost scales with page resolution, which is what makes them expensive at volume.

**A benchmark-integrity caveat that undercuts several rows above.** LlamaIndex argues
[OmniDocBench is saturated](https://www.llamaindex.ai/blog/omnidocbench-is-saturated-what-s-next-for-ocr-benchmarks)
(2026-02-24): top models cluster above 94 % — GLM-OCR 94.6, PaddleOCR-VL-1.5 > 94, against Gemini 3
Pro at 90.3 — so gains are "edge-case fixing"; the 1,355-page / 9-document-type corpus is skewed to
academic papers and omits handwriting, visuals and form fields; and edit distance / TEDS penalise
"small, harmless differences like punctuation, spacing, and line breaks" where several
representations are semantically valid. **Differences of a few points on OmniDocBench should not be
treated as real.**

---

# Part II — Voice → text

## 6. The framing correction

The audio half of the question was set up expecting the image answer: plucky specialists against
bloated generalists. **That is not what the 2026 literature shows, and this survey's own working
hypothesis was wrong here.** Two distinct things must be separated:

- **Speech-LLMs** — a Conformer/FastConformer encoder bolted to an LLM decoder (Canary-Qwen-2.5B =
  FastConformer + an unmodified Qwen3-1.7B; Granite Speech; Phi-4-Multimodal). These are *specialist
  systems that use an LLM as a component*. They now **lead English accuracy**.
- **General omni-modal LLMs** — Qwen3.5-Omni, Gemini, GPT-4o-Transcribe. Genuinely
  general-purpose. These are **no longer outclassed**: Qwen3.5-Omni-Plus reports **6.6 % average WER
  on FLEURS**, beating Gemini-3.1-Pro (7.3 %) and GPT-4o-Transcribe (10.4 %), **1.7 % on
  LibriSpeech**, and 113 languages — with large margins on tonal and low-resource languages
  (Cantonese 2.2 % vs. Gemini-3.1-Pro 6.3 %). `[vendor-reported]`

So on **accuracy**, general multimodal models are competitive at ASR. The specialist case now rests
almost entirely on **efficiency, size, licence and streaming** — where it is not close.

## 7. The Open ASR Leaderboard, and the shape of the frontier

[The leaderboard paper](https://arxiv.org/abs/2510.06961) (86 systems, 12 datasets; English
short-form, long-form, multilingual) is the field's reproducibility anchor. English short-form:

| Model | Avg. WER ↓ | RTFx ↑ |
|---|---|---|
| NVIDIA Canary-Qwen-2.5B | **5.63** | 418 |
| IBM Granite-Speech-3.3-8B | 5.74 | 145 |
| IBM Granite-Speech-3.3-2B | 6.00 | 260 |
| Microsoft Phi-4-Multimodal-Instruct | 6.02 | 151 |
| **NVIDIA Parakeet-TDT-0.6B-v2** | 6.05 | **3386** |
| NVIDIA Parakeet-TDT-0.6B-v3 | 6.32 | 3333 |
| Mistral Voxtral-Mini-3B | 7.05 | 110 |
| OpenAI Whisper-large-v3 | 7.44 | 146 |

**This table is the whole argument in eight rows.** Parakeet-TDT-0.6B-v2 gives up **0.42 WER** to
the leader and returns **8× the throughput at a quarter of the parameters**. The paper's own
conclusion: Conformer+LLM decoders "achieve the strongest English WER but at the cost of higher
latency, whereas CTC/TDT decoders offer faster inference with only modest accuracy trade-offs" —
elsewhere quantified as **10–100× faster throughput**.

Long-form inverts the accuracy order and widens the efficiency gap: Whisper-large-v3 **6.43 WER at
RTFx 68.56**, Parakeet-CTC-1.1B **6.68 at RTFx 2794** (~41×). Multilingual is a near-tie —
Phi-4-Multimodal, Canary-1B-v2, Whisper-large-v3 and Parakeet-TDT-0.6B-v3 all land in **3.2–6.6 %**
across DE/FR/IT/ES/PT with no consistent winner — and the paper notes English fine-tuning
"often [comes at] the cost of multilingual coverage."

## 8. Delta against `audio-asr-research-2026-07-23.md`

| That doc's position | Status on 2026-08-31 |
|---|---|
| Whisper `base.en` via prebuilt `whisper-server` is the pragmatic path | **Holds.** Nothing here disturbs it; Whisper remains the multilingual/portability baseline. |
| Moonshine is the short-clip/edge model to want | **Holds and strengthened.** [Moonshine v2](https://arxiv.org/html/2602.12241v1) adds a **position-free sliding-window streaming encoder** for latency-critical use; the *Flavors of Moonshine* line goes down to **34 M params (~130 MB)**, claiming quality on par with models 6× their size; English model **MIT**. |
| Parakeet-0.6B is too big for a mid-range phone (~1.2 GB int8) | **Holds as a memory claim.** Independently, Parakeet-TDT is reported **slow on CPU** — it is a GPU/bulk model. "Parakeet wins bulk cloud transcription, Moonshine wins live edge." |
| sherpa-onnx as a multi-model runtime hub | **Holds** — unchallenged by anything in this survey. |
| Model table topping out at Parakeet 1.69 % LibriSpeech | **Superseded at the top.** New entrants since: **ARK-ASR-3B** (5.04 % avg WER / RTFx 491, reported SOTA on the English short-form track), **Granite-Speech-4.1-2B** (5.33 %, shipped May 2026, with a **2BN** variant at RTFx 1820 — an hour of audio in ~2 s), **MOSS-Transcribe-preview-2B** (reported lower still). |
| *(not addressed)* general multimodal LLMs | **New**: they are now competitive on accuracy — see §6. |

## 9. When rank stops discriminating

The top of the English track spans **5.04–6.05 WER** — under one point. At that spread, and given
the claim-evidence critique in *Method*, **rank is no longer the deciding variable**: licence,
language coverage, streaming support, CPU vs. GPU viability, and cost per audio-hour are. This is
the audio field's version of OmniDocBench saturation, and it arrived at roughly the same time.

---

# Part III — Cross-cutting

## 10. Two different convergences

- **Images: the generalist is being distilled into the specialist.** Family B *is* the VLM
  architecture, trained on one task; LightOnOCR-2-1B is literally distilled from Qwen3-VL-235B. The
  specialist wins by being a compressed generalist, and what it keeps that the teacher lacks is
  **refusal to hallucinate** and **spatial grounding**.
- **Audio: the specialist is being absorbed into the generalist.** Speech-LLMs put an LLM inside the
  ASR system; omni-modal LLMs put ASR inside the LLM. What survives as irreducibly specialist is the
  **encoder-only CTC/TDT design** — 10–100× throughput, sub-100 MB footprints, streaming.

**The property that does not converge, in either modality, is efficiency.** Every accuracy claim in
this survey is contestable within a point or two; none of the efficiency gaps are close. 5 M vs.
235 B; 0.6 B at RTFx 3386 vs. 8 B at 145; 34 M running on a phone. If "competitive" means *matches
the best number*, the answer is often "nearly". If it means *delivers that quality per unit of
compute*, specialists win by orders of magnitude that no benchmark dispute can erase.

## 11. Reading these numbers responsibly

1. **Most tables here are self-reported by the proposing paper.** Independent re-runs are rare.
2. **Version drift is rampant** — OmniDocBench v1.0 / 1.5 / 1.6 / 1.7 scores are quoted
   interchangeably across sources and are not comparable.
3. **Two benchmarks are saturated or near it** (OmniDocBench above 94; Open ASR's top under 1 WER
   point apart), so small deltas are noise.
4. **Metric choice changes the winner** — CDM was created precisely because edit distance mis-ranks
   formula recognition.
5. **The strongest specialist claims and the weakest evidence coincide.** The two most dramatic
   numbers in this survey — PP-OCRv6's 93.2 % vs. 72.6 % hallucination spread, and 86.2 vs. 46.8
   detection Hmean — both come from a **vendor-curated benchmark with no independent replication**.

## Bibliography

**Image → text.** [PP-OCRv5, CVPR 2026](https://arxiv.org/abs/2603.24373v1) ·
[PP-OCRv6](https://arxiv.org/pdf/2606.13108) · [HunyuanOCR](https://arxiv.org/html/2511.19575v1) ·
[dots.ocr](https://arxiv.org/html/2512.02498v1) · [olmOCR-2](https://arxiv.org/abs/2510.19817) ·
[LightOnOCR-2](https://arxiv.org/html/2601.14251v1) · [GLM-OCR](https://arxiv.org/pdf/2603.10910) ·
[DharmaOCR](https://arxiv.org/pdf/2604.14314) `[partially verified]` ·
[OmniDocBench](https://github.com/opendatalab/OmniDocBench) ·
[OmniDocBench saturation](https://www.llamaindex.ai/blog/omnidocbench-is-saturated-what-s-next-for-ocr-benchmarks) ·
[MORE](https://arxiv.org/pdf/2607.02956) · [AWESOME-OCR-LLM](https://github.com/Yuliang-Liu/AWESOME-OCR-LLM) ·
[Modal's 8-model comparison](https://modal.com/blog/8-top-open-source-ocr-models-compared)

**Handwriting & maths.** [HTR-ConvText](https://arxiv.org/html/2512.05021v1) ·
[LLMs for HTR](https://arxiv.org/abs/2503.15195) · [UniMERNet](https://arxiv.org/html/2404.15254v1) ·
[CDM metric](https://arxiv.org/html/2409.03643v1) · [Uni-MuMER](https://arxiv.org/html/2505.23566) ·
[DocTron-Formula](https://arxiv.org/html/2508.00311)

**Voice → text.** [Open ASR Leaderboard](https://arxiv.org/abs/2510.06961) ·
[HF leaderboard blog, 2025-11-21](https://huggingface.co/blog/open-asr-leaderboard) ·
[Moonshine v2](https://arxiv.org/html/2602.12241v1) ·
[Flavors of Moonshine](https://arxiv.org/pdf/2509.02523) ·
[On-device streaming ASR](https://arxiv.org/abs/2604.14493) ·
[Qwen3.5-Omni](https://arxiv.org/html/2604.15804v1) ·
[Granite Speech 4.1](https://huggingface.co/ibm-granite/granite-speech-4.1-2b) ·
[ARK-ASR-3B](https://huggingface.co/Audio8/ARK-ASR-3B)

**Method.** [SOTA claims require SOTA evidence](https://arxiv.org/abs/2605.17273)

**Re-verify:** the two leaderboards move monthly —
`https://huggingface.co/spaces/hf-audio/open_asr_leaderboard` and the OmniDocBench repo's
leaderboard — and the arXiv IDs above dated `26xx` postdate this survey's model cutoff, so they were
read from the papers rather than recalled.
