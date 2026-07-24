# Device resources — measured, not guessed

The actual hardware and per-component footprints for the two real devices, for **every** model we
have run — including ones not chosen as the default, because a real measurement is data worth
keeping. **Every number is measured** (with its source/command) or explicitly flagged as an
estimate. Follows the "external facts get a date + a re-verify command" rule.

_Last measured: 2026-07-24. Text-model numbers are from `model-benchmarks-2026-07-22.md`
(`llama-bench` + `/usr/bin/time -v` + on-device `/proc`) unless a newer date is given; whisper +
the live GPU/VRAM readings are from this session's `/proc` + `nvidia-smi`._

## The two devices (measured)

| | **Phone** | **Laptop** |
|---|---|---|
| Model | Redmi `24095PCADG` | (this machine) |
| SoC / CPU | MediaTek **MT6878** = Dimensity 7300, **8 cores (4 big + 4 little)** | Intel **i7-11800H**, 8C / 16T @ 2.30 GHz |
| RAM total | **7.4 GB** (7,607,064 kB) | **14.9 GB** (15,581,864 kB) |
| RAM free (typical) | ~2.9–3.4 GB | ~5–10 GB |
| GPU | none (CPU inference) | **RTX 3050 Laptop, 4 GB VRAM**, via **Vulkan** (`libvulkan.so.1`, no CUDA toolkit) |
| Optimal threads | **`-t 4`** (big cores only — using all 8 makes big cores wait on little) | **`-t 8`** (physical cores) |

Re-measure: `adb shell cat /proc/meminfo` · `adb shell getprop ro.board.platform` · `lscpu` · `nvidia-smi`.

## Text models — throughput (decode tok/s) and footprint, per device

Q4_K_M throughout. Decode t/s is the number that matters (reading speed ≈ 5–10 t/s, so all are
"smooth"). "peak RSS" = the model **process's own** peak resident RAM (`VmHWM`), which is well above
the file size once KV-cache + compute buffers are counted.

| Model | file | Phone `-t 4` decode | Phone resident (measured live) | Laptop CPU `-t 8` decode | Laptop peak RSS | Laptop **GPU (Vulkan)** decode | VRAM |
|---|---|---|---|---|---|---|---|
| **LFM2.5-230M** | 144 MB | **64 t/s** | ~300 MB *(est)* | 226 t/s | small | — | — |
| **LFM2.5-350M** | 216 MB | **42 t/s** | ~450 MB *(est)* | 148 t/s | small | — | — |
| **LFM2.5-1.2B** *(phone default)* | 695 MB | **13.5 t/s** (llama-bench) / **15.9 t/s** (real prompt) | **1.41 GB peak** (`VmHWM`, live serving); ~0.9 GB under llama-bench | 48 t/s | **1.19 GB** (measured) | **171.5 t/s** (3.6×) | fits easily |
| **Qwen3-4B-2507** *(laptop default)* | **2.38 GB** | — (too big to prefer on phone) | — | 14.0 t/s | **4.06 GB** (CPU) / 362 MB RSS + 2.64 GB peak (GPU build) | **51.9 t/s** (3.7×) | **2.53–2.75 GB** (97% util) |

Phone process names: `libllama-server.so` / `libwhisper-server.so` (children of `dev.formicaria.notes`,
run from `nativeLibraryDir`). Measured CPU load during generation: phone LFM2.5-1.2B **~385% avg**
(≈4 of 8 cores) — the basis for "doesn't block the UI"; laptop Qwen3-4B (CPU) **741%** (≈7.4/16).

**Live co-residence, measured on the phone 2026-07-24 (whisper actively transcribing):** the OS does
**not** hold both models fully resident — it **swaps the idle llama to zram**. Snapshot: whisper
resident 142 MB; llama resident 35 MB + **747 MB in swap** (peak was 1.41 GB when it was the active
model); `MemAvailable` steady at ~2.6 GB. So the phone runs audio+text together by paging whichever
model is idle — no admission-gate refusal, no stall observed.

**Shipped defaults (laptop updated 2026-07-24):** phone = `lfm2.5-1.2b` (unchanged — the sweep found no
≤2B model beats it); laptop = **`qwen3-vl-4b`** on the **GPU** (40.1 t/s our-case, weights in VRAM so
system RAM stays free — RSS 791 MB), which grounds better than the prior `qwen3-4b-2507` (kept as the
fallback) at the same speed. See the full matrix above + `model-selection-research-2026-07-24-grounded.md`.
The GPU is the decisive laptop win — 3.2–3.7× over CPU. Non-default rows are kept because the measurement
is the asset.

**Can we push the phone bigger? Measured 2026-07-24 — a real speed↔quality tradeoff, not a free win.**
The `agents/bench/` harness (our real StudyAssistant workload, not synthetic) run on-device:
`lfm2.5-1.2b` = **12.4 tok/s our-case, 1.5 GB RSS**, but **0 verbatim quotes** on the grounded-research
contract; `Qwen3-1.7B` = **8.9 tok/s (~28% slower), 2.4 GB RSS**, but it **follows the quote-first
contract** (1 quote where the 1.2B managed none). Both loaded safely (MemAvailable held 2.5 GB), so
**RAM is not the wall — phone CPU throughput is** (bigger = slower). So the bigger model is *smarter but
slower*, and the pick is a speed-vs-quality call. **Caveat that cost a wrong first verdict:** Qwen3 is a
thinking model — every model has its own special tokens + chat template, and a thinking model needs
`enable_thinking:false` (or the app reads back an empty answer). To *ship* a thinking model the app's
OpenAiStep needs that too. Full numbers + the reusable harness: `agents/bench/results.md` +
`agents/bench/README.md` (`pixi run model-bench`).

## Full resource-cost matrix — our-case sweep, measured 2026-07-24

Every model we ran through the hardened `agents/bench` harness (real StudyAssistant workload, temp 0,
thinking-off, `-c 2048`, `n_predict` ≤256), one at a time, with live `/proc` + `nvidia-smi` sampling.
**This is the "what will it cost me" reference** — pick a row, read across. `decode t/s` = generation
speed (reading pace ≈5–10, so all are usable); `prompt t/s` = context-ingest speed (range across the 6
cases — noisy, depends on prompt length); `peak RSS`/`VmHWM` = the model process's own resident/peak
RAM; `VRAM` = GPU memory held; `MemAvail min (before)` = system free RAM at the tightest point (the
headroom that proves it fit). Raw per-case blocks: `agents/bench/results.md`.

### 💻 Laptop — i7-11800H, RTX 3050 4 GB (Vulkan), 14.9 GB RAM

| Model | GGUF (Q4_K_M) | backend | decode t/s | prompt t/s | peak RSS | VmHWM | VRAM | MemAvail min (before) |
|---|---|---|---|---|---|---|---|---|
| **Qwen3-VL-4B** *(NEW default)* | 2.4 GB | GPU `-ngl 99` | **40.1** | ~510–700 | **791 MB** | 2594 MB | **2782 MB** | 4505 MB (4770) |
| Qwen3-4B-2507 *(fallback)* | 2.4 GB | GPU `-ngl 99` | 39.8 | ~260–675 | 511 MB | 2578 MB | 2769 MB | 9757 MB (9930) |
| Qwen3-4B-2507 | 2.4 GB | CPU `-t 8` | 12.6 | ~70–222 | 3029 MB | 3029 MB | — | 5766 MB |
| Nemotron-3-Nano-4B | 2.7 GB | GPU `-ngl 99` | 35.4 | ~19–362 | 1773 MB | 2933 MB | 2986 MB | 5645 MB (6532) |
| Gemma-4-E4B | 4.7 GB | **CPU only** ¹ | 10.9 | ~21–54 | 5363 MB | 5363 MB | — | 9953 MB (10110) |

¹ Gemma-4-E4B's 4.7 GB Q4 exceeds the 4 GB VRAM, so it cannot GPU-offload on this card — CPU-bound at ~11 t/s.
The GPU rows hold weights in VRAM, so the model's **system-RAM RSS stays under 800 MB** — the decisive laptop win.

### 📱 Phone — Dimensity 7300 (8-core), 7.4 GB RAM, CPU `-t 4 -ngl 0`

| Model | GGUF (Q4_K_M) | decode t/s | prompt t/s | peak RSS | VmHWM | MemAvail min (before) |
|---|---|---|---|---|---|---|
| **LFM2.5-1.2B** *(default — stands)* | 695 MB | **12.4** | ~100–106 | 1503 MB | 1503 MB | 2950 MB (3018) |
| MiniCPM5-1B | 657 MB | **13.5** | ~94–99 | 1213 MB | 1266 MB | 3242 MB (3475) |
| LFM2.5-VL-1.6B | 698 MB | 11.8 | ~70–72 | 1518 MB | 1520 MB | 3188 MB (3274) |
| Qwen3-VL-2B | 1.1 GB | 7.8 | ~48–49 | 2611 MB | 2611 MB | 2474 MB (2786) |
| Qwen3.5-2B | 1.2 GB | 7.5 | ~30–63 | 2861 MB | 2861 MB | 2367 MB (2475) |
| Qwen3-1.7B *(earlier)* | 1.1 GB | 8.9 | — | 2432 MB | 2432 MB | ~2500 MB |

**Reading the phone rows:** every candidate **fit** (MemAvailable never dropped below ~2.3 GB — the
admission gate never refused). The wall is **CPU throughput, not RAM**: the 1–1.2B models run 12–13.5 t/s,
the 2B models drop to ~7.5–8 (bigger = slower). Peak RSS runs ~1.7–2.4× the GGUF file (KV-cache + compute
buffers). Quality did **not** improve with size — no ≤2B model grounded a verbatim quote — so the phone
default stays at the fast, well-behaved 1.2B. (Grounding/behaviour per model: `agents/bench/results.md`.)

## Whisper (audio → transcript)

| Model | file | Device | RSS / peak | Source |
|---|---|---|---|---|
| **base.en** *(both defaults, 2026-07-24)* | **142 MB** | Laptop (CPU) | **150 MB / 196 MB peak** | measured `/proc/<pid>/status` this session |
| **base.en** | 142 MB | **Phone** | **142 MB resident** (`VmRSS`=`VmHWM`, RssAnon 142 MB) | **measured live on device 2026-07-24** (pid `libwhisper-server.so`) |
| tiny.en *(old phone pick, fall-back)* | ~75 MB | Phone | ~90 MB *(est)* | estimate |

whisper.cpp v1.9.1; runtime is the prebuilt x64 server (laptop) / cross-compiled arm64 static
`whisper-server` bundled in the APK (phone). base.en is genuinely cheap next to the llama — which is
why the phone moved tiny→base (`models.toml`, 2026-07-24).

## Admission-gate budget (the hard limit)

`fm_agent::preflight::admit()` refuses to launch when `MemAvailable < model_file_bytes + headroom`
(fail-safe: too big → clear refusal, never an OOM crash). Constants:

| Launch | headroom | gate needs free |
|---|---|---|
| phone llama (`agent.rs`) | 1.0 GB | ~1.75 GB |
| phone whisper (`agent.rs`) | 300 MB | ~0.45 GB |
| laptop (`serve.rs` `--headroom`) | CLI-set | file + headroom |

Phone reality (measured, not additive): the OS **swaps the idle model to zram** rather than holding
both — whisper 142 MB + llama paged down to 35 MB resident (+747 MB swap, 1.41 GB peak when active) +
app shell ~0.2 GB, with `MemAvailable` steady ~2.6 GB. Comfortable, and the gate never refused.

## Re-measure commands

```
# Phone, while a transcription/agent turn is ACTIVE (models loaded):
adb shell 'for p in llama-server whisper-server; do echo $p; cat /proc/$(pidof $p)/status 2>/dev/null | grep -E "VmRSS|VmHWM"; done'
adb shell cat /proc/meminfo | grep MemAvailable
adb shell dumpsys meminfo dev.formicaria.notes | grep -E "TOTAL (PSS|RSS)"

# Laptop, with agent-serve running:
for p in llama-server whisper-server; do echo $p; grep -E "VmRSS|VmHWM" /proc/$(pgrep -x $p)/status; done
nvidia-smi --query-gpu=name,memory.total,memory.used,utilization.gpu --format=csv,noheader

# Throughput (either device): agents/llama.cpp llama-bench -m <model.gguf> -t <4|8> -p 128 -n 64
```
