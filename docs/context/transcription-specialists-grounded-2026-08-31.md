# Specialist transcription models, filtered through *our* hardware (2026-08-31)

The grounded companion to
[`transcription-specialists-survey-2026-08-31.md`](./transcription-specialists-survey-2026-08-31.md).
The survey asked *what exists*; this asks **what we could actually run**, against the measured
devices in [`device-resources.md`](./device-resources.md) and the house rules in `CLAUDE.md`.

**This doc proposes; it does not decide.** No default is changed and no file is fetched by anything
here. If a pick below is adopted, *that* earns the `decisions.md` entry — it would widen what ships.

## The filter

| Constraint | Where it comes from | What it kills |
|---|---|---|
| Laptop GPU = **RTX 3050, 4 GB VRAM**, Vulkan (no CUDA toolkit); `qwen3-vl-4b` already holds **2.78 GB** | `device-resources.md` | ~1.2 GB VRAM free — no second vision model co-resident with the current one |
| Phone = **Dimensity 7300, 8 core, no GPU, 7.4 GB RAM**, `MemAvailable` ~2.9–3.4 GB; CPU throughput is the wall, not RAM | `device-resources.md` | anything whose peak RSS (measured at **1.7–2.4× the GGUF file**) exceeds ~3 GB |
| Admission gate refuses when `MemAvailable < model_file + headroom` (phone llama 1.0 GB, whisper 300 MB) | `fm_agent::preflight::admit()` | fail-safe, so an over-large pick is a refusal, not a crash |
| **No native blob `cargo-deny` cannot audit** without a hand-written NOTICE | `CLAUDE.md`, `audio-asr-research-2026-07-23.md` | onnxruntime, Paddle, Tesseract, `whisper-rs`/`ort`/`sherpa-rs` |
| Shipping into the **MIT Android app is distribution**, not invocation | `outstanding.md` §2.4, `deny.toml` line 1 | AGPL/GPL weights or code bundled in the APK |
| The working pattern is a **prebuilt sidecar binary + loopback HTTP**, bytes by value never a blob path | `models.toml`, the `Transcribe` seam | anything needing a Python runtime in the shipped app |

Everything below is judged against that, not against a leaderboard.

## What we run today, measured

| | Laptop | Phone |
|---|---|---|
| **Image→text** | `qwen3-vl-4b` + `mmproj` — 2.4 GB GGUF, **2.78 GB VRAM**, 40.1 t/s | **nothing — the model is blind and says so** |
| **Voice→text** | `whisper base.en`, 142 MB, 150 MB RSS | `whisper base.en`, 142 MB resident |

The image row is the gap: one general-purpose 4B VLM doing OCR on the desktop only, chosen (rightly,
on grounding and abstention) for the *study-assistant* job and inheriting the transcription job
because the projector was one file away.

---

# Part I — Image → text

## 1. The finding that decides this: our runtime already supports the specialists

`llama.cpp` ships first-class support for exactly the family-B specialists the survey identified as
state of the art — [its own guide](https://huggingface.co/blog/ggml-org/using-ocr-models-with-llama-cpp)
lists **LightOnOCR, Qianfan-OCR, PaddleOCR-VL, GLM-OCR, DeepSeek-OCR, dots.ocr and HunyuanOCR**, run
through the same `llama-server --mmproj` path we already use, on the same `/v1/chat/completions`
endpoint, from the same pinned tarball.

**PaddleOCR-VL support merged 2026-02-19 ([PR #18825](https://github.com/ggml-org/llama.cpp/pull/18825)),
five months before the build we pin (`b10076`, 2026-07-21).** So for at least that model this is
**not a runtime change** — no new sidecar, no new port, no new NOTICE, no `pixi.lock` movement.
It is a `models.toml` entry and a fetch, which is the *exact* shape of the already-ruled
"one verb, two specialists" change (`decisions.md`, 2026-08-30) that enabled vision at all.

## 2. The candidate, with our arithmetic

**PaddleOCR-VL-1.6** — 0.5 B language decoder, **Apache-2.0**, official
[`PaddlePaddle/PaddleOCR-VL-1.6-GGUF`](https://huggingface.co/PaddlePaddle/PaddleOCR-VL-1.6-GGUF),
model card claims **96.33 % on OmniDocBench v1.6**. Sizes from the HF API (the same
`?blobs=true` provenance method `models.toml` already uses):

| Build | LM | mmproj | **Total** | Phone gate needs | Projected peak RSS (1.7–2.4×) |
|---|---|---|---|---|---|
| Q4_K_M | 300 MB | 882 MB | **1.18 GB** | ~2.18 GB — **passes** (2.9–3.4 GB avail) | 2.0–2.8 GB — **tight but plausible** |
| Q8_0 | 498 MB | 882 MB | 1.38 GB | ~2.38 GB — passes | 2.3–3.3 GB — marginal |
| official F16 | 936 MB | 882 MB | 1.82 GB | ~2.82 GB — marginal | 3.1–4.4 GB — **fails the phone** |

**The engineering fact that matters: the 882 MB mmproj is F16-only and is 75 % of the Q4 footprint.**
Quantising the language model below Q4 buys almost nothing. Any sizing argument about this model is
really an argument about its vision encoder.

**The alternative already measured on our own phone.** `LFM2.5-VL-1.6B` appears in
`device-resources.md`'s sweep (698 MB file, **11.8 t/s, 1518 MB RSS on the Dimensity 7300**) — the
only vision-capable model we have *ever run on the phone*. But that was the LM alone; its
[mmproj is 854 MB](https://huggingface.co/unsloth/LFM2.5-VL-1.6B-GGUF), so the real total is
Q4_K_M 731 MB + 854 MB = **1.59 GB**, projecting to 2.7–3.8 GB peak — **worse than PaddleOCR-VL and
probably past the phone.** It is also licensed "other" (LFM Open Licence), not Apache-2.0, and
llama.cpp carries [a known GGUF OCR-quality defect](https://github.com/ggml-org/llama.cpp/issues/17290)
for LFM2-VL on images with a ~1024 px side — precisely the photographed-page case.

**So the general small VLM is both bigger and worse at this job than the 0.5 B specialist.** That is
the survey's thesis landing on our actual hardware.

## 3. Verdicts

**Laptop — adopt-shaped, and cheaper than it looks.** PaddleOCR-VL-1.6 Q8_0 is 1.38 GB. It
**cannot co-reside in VRAM** with `qwen3-vl-4b` (2.78 GB of 4 GB leaves ~1.2 GB), but at 0.5 B it
does not need to: **run the OCR specialist on CPU and leave the GPU to the study model.** That
sidesteps the VRAM contention entirely and keeps the agent's 40.1 t/s untouched — which the
present design cannot do, because today the *same* model does both jobs and both jobs want the GPU.

**Phone — the first credible path to an image→text feature at all.** Q4_K_M at 1.18 GB clears the
admission gate with ~0.7–1.2 GB to spare and projects inside the envelope, and the measured
co-residence behaviour is favourable: the OS **swaps the idle model to zram** rather than holding
both (whisper 142 MB + llama paged to 35 MB resident + 747 MB swap, `MemAvailable` steady ~2.6 GB).
Unverified until run, and the phone's wall is throughput, not RAM — a 0.5 B decoder should be fast,
but the 882 MB vision encoder's prefill cost on 4 big cores is the number nobody has.

## 4. A premise in `outstanding.md` §2.4 that our own shipping has falsified

The parked **Image→LaTeX** entry says the licence constraint "decides the design" because *"the
target includes the phone, and **Android has no subprocesses** — so the model must run inside the
WebView"*, forcing ONNX Runtime Web / transformers.js and ruling out AGPL Texo.

**The subprocess half is no longer true.** Since 2026-07-24 the phone runs
`libllama-server.so` and `libwhisper-server.so` as **child processes of `dev.formicaria.notes`**
from `nativeLibraryDir` (`device-resources.md`). The phone has had native sidecars for five weeks.

This does **not** rescue AGPL Texo — distribution is distribution, and bundling an AGPL model in an
MIT APK is still out. But it removes the in-WebView requirement, and with it the forced
ONNX-Runtime-Web design. Any formula work can ride the sidecar we already ship. §2.4 should be
re-read before it is picked up, not applied as written.

Two of its other findings survive intact and are corroborated by the survey: **evaluate with CDM,
not BLEU** (arXiv 2409.03643), and *train-our-own dissolves the licence problem*. A third is worth
revisiting — the survey's Family C section shows **general VLMs now match specialists on printed
formulae** (GPT-4o 86.8 CDM ≈ Mathpix 86.6 ≥ UniMERNet 85.0), while PaddleOCR-VL-1.6 advertises
formula recognition as a first-class capability. The 20 M-param bespoke model may be solving a
problem a 0.5 B Apache-2.0 download now solves.

---

# Part II — Voice → text

## 5. `whisper base.en` stands — and nothing in the survey dislodges it

At 142 MB and 150 MB RSS it is, as `device-resources.md` puts it, "genuinely cheap next to the
llama". Against our filter the survey's leaders mostly disqualify themselves:

| Candidate | Survey standing | Our verdict |
|---|---|---|
| Qwen3.5-Omni | competitive general omni-modal, 1.7 % LibriSpeech | **30B-A3B MoE — not on either device.** Ignore. |
| Canary-Qwen-2.5B / Granite-Speech-4.1-2B | best WER (5.63 / 5.33) | laptop-only at ~2.5 GB+, and it would contend with the llama for the 4 GB card for a job base.en already does acceptably. **No.** |
| Moonshine v2 (34 M, MIT, streaming) | the edge specialist | still the model we'd most want on the phone — but reaching it means **sherpa-onnx / onnxruntime**, i.e. a NOTICE gate and a Python or native dependency. The July doc's plan (strategy B) is unchanged and still unbuilt. **Deferred, not dismissed.** |
| Parakeet-TDT-0.6B-v3 | 6.32 WER at **RTFx 3333** | **newly reachable — see below.** |

## 6. Parakeet is now a zero-new-runtime option, with one integration catch

The July doc ruled Parakeet out on phone RAM (~1.2 GB int8). Two things changed:

1. **`whisper.cpp` v1.9.0 added NVIDIA Parakeet support**, and `models.toml` already records that the
   tarball we pin *"also bundles a `parakeet-cli`, a possible future faster/edge leaf."* **The binary
   is already in the archive we fetch.**
2. **Parakeet-TDT-0.6B-v3 int8 is ~680 MB, 25 languages** — not the 1.2 GB the July doc assumed.

On the **laptop** that is a straight upgrade candidate over `base.en` at no runtime cost. On the
**phone**, 680 MB clears the 300 MB-headroom whisper gate (~980 MB needed) but is **4.8× base.en**
and would sit beside the 1.4 GB llama; the measured zram-swap behaviour suggests it *might* hold,
and it is exactly the kind of claim this repo requires to be measured rather than reasoned.

**The catch, and it is real:** `parakeet-cli` is a **CLI, not a server**. Our `Transcribe` backend
speaks HTTP to `whisper-server`'s `/inference`. Adopting Parakeet means either a second sidecar
shape (spawn-per-clip) or waiting for server support — **verify before promising**, because the
model-agnostic seam is agnostic about the *model*, not about the *transport*.

---

# Part III — What to measure before anything is adopted

The house rule is measured-not-guessed, and every number in Part I §2–3 marked "projected" is
arithmetic over a measured ratio, not an observation. The reusable harness already exists
(`agents/bench/`, `pixi run model-bench`, `agents/bench/README.md`).

1. **Laptop, PaddleOCR-VL-1.6 Q8_0 on CPU** — fetch, serve on a second port, run a handful of real
   photographed pages and handwriting through `/transcribe`. Record decode t/s, prompt t/s (the
   vision encoder's prefill is the unknown), peak RSS, and — the one that matters — **whether it
   beats `qwen3-vl-4b` on our own inputs**, not on OmniDocBench.
2. **Confirm `b10076` actually loads PaddleOCR-VL-*1.6*.** The merged PR is for PaddleOCR-VL; 1.6 is
   a later revision and may need a newer build. If it does, that is a **pin bump**, which is a
   `decisions.md`-worthy change to what ships, not a footnote.
3. **Phone, PaddleOCR-VL-1.6 Q4_K_M** — only after (1). Watch `MemAvailable`, `VmHWM` and whether the
   admission gate refuses; re-measure llama co-residence under zram.
4. **Laptop Parakeet spike** — does `whisper-server` serve a Parakeet model over `/inference`, or is
   `parakeet-cli` the only entry point? That single answer decides whether §6 is cheap or a rebuild.
5. **Hold the honest comparison.** The survey's §11 applies to us: OmniDocBench is saturated and
   PaddleOCR-VL's 96.33 % is a vendor number on a saturated benchmark. Our acceptance test is
   handwriting and photographed pages from this vault, scored by eye — and the property to test
   hardest is the one the survey says specialists actually win, **refusing to hallucinate**, which
   is also the property `decisions.md` already made load-bearing ("an unreadable symbol becomes
   `[?]` rather than a guess").

## Summary

| Question | Grounded answer |
|---|---|
| Is there a specialist worth switching to for **image→text**? | **Yes, one: PaddleOCR-VL-1.6.** 0.5 B, Apache-2.0, official GGUF, already supported by the runtime we pin, 300 MB + 882 MB mmproj at Q4. Nothing else clears the filter as cleanly. |
| Does it fix the **phone's blindness**? | Plausibly — the first candidate that fits the gate with margin. Unmeasured. |
| Should **voice→text** change? | **No.** `base.en` stands. Parakeet is newly reachable and worth a laptop spike; Moonshine stays the wanted-but-gated edge pick; everything else is too big. |
| Does any of this change a decision? | Not yet. Adoption would — and `outstanding.md` §2.4 needs its falsified subprocess premise corrected regardless. |
