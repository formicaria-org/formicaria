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

**Shipped defaults:** phone = `lfm2.5-1.2b` (quality worth the 15.9 t/s; 350M is the lighter sweet
spot at 42 t/s if ever needed); laptop = `qwen3-4b-2507` on the **GPU** (51.9 t/s, weights in VRAM so
system RAM stays free). The GPU is the decisive laptop win — 3.6–3.7× over CPU. Non-default rows are
kept because the measurement is the asset.

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
`agents/bench/README.md` (`pixi run model-bench`). *(Laptop-GPU ladder — its 4 GB VRAM caps it near the
current 4B — is the remaining measurement.)*

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
