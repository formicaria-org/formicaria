# On-device model benchmarks — picking the best model per device (2026-07-22)

Real `llama-bench` (b10081 arm64 / b10076 x64) measurements of the three LFM2.5 candidates on both
target devices, to choose a default that is *good* without blocking the interactive system.

## Numbers (Q4_K_M, prompt 128 / gen 64)

**Phone — Dimensity 7300 (MT6878), 8 cores big.LITTLE, ~3.4 GB free. `-t 4` (big cores):**

| Model | prefill t/s | decode t/s | model | ~RAM resident |
|---|---|---|---|---|
| LFM2.5-230M | 664 | **64** | 144 MB | ~300 MB |
| LFM2.5-350M | 374 | **42** | 216 MB | ~450 MB |
| LFM2.5-1.2B | 104 | **13.5** | 695 MB | ~1.4 GB |

**Laptop — i7-11800H, ~5 GB free, RTX 3050 4 GB (CPU build, GPU not wired). `-t 8` (physical cores):**

| Model | prefill t/s | decode t/s |
|---|---|---|
| LFM2.5-230M | 2340 | **226** |
| LFM2.5-350M | 1313 | **148** |
| LFM2.5-1.2B | 358 | **48** |

## The load-bearing finding: threads, not size, govern smoothness

On the phone **`-t 4` is *faster* than `-t 8`** (230M: 664/64 vs 424/45; 350M: 374/42 vs 293/35). The
Dimensity 7300 is 4 big + 4 little cores; using all 8 makes the big cores wait on the little ones and
contend for memory bandwidth. So the smoothest config — 4 threads, leaving 4 cores for the UI — is
*also* the fastest. `threads = 4` in `models.toml` is already optimal; it is why inference does not
stutter the interface. (On the laptop, `-t 8` of 16 leaves 8 for the system.)

## Recommendation

- **Phone → LFM2.5-350M.** The sweet spot: **42 tok/s** decode (far above reading speed, so answers
  stream smoothly), ~450 MB resident (safe headroom on 3.4 GB free, low thermal), and clearly better
  quality than the 230M (which is thin — the "say hi" wobble). The 1.2B *runs* (13.5 tok/s, 1.4 GB) but
  a long answer is slow and it is the one most likely to draw LMKD/thermal pressure under sustained use
  — keep it as an opt-in "slower, better" step-up, not the default.
- **Laptop → LFM2.5-1.2B.** 48 tok/s CPU is comfortably fast, quality is the best of the three, and it
  fits easily (695 MB model, ~5 GB free). The laptop has the headroom the phone doesn't; use it.
- **Keep `-t 4` on the phone, `-t 8` on the laptop** — the measured smoothness/speed optimum on each.
- Wiring the laptop's **RTX 3050** (a CUDA `llama-server`) would push the laptop far higher and free
  its CPU entirely — the biggest available smoothness win there — but needs a CUDA runtime, deferred.

## Verified on-device inference of the picked models (real prompts, not just llama-bench)

Tried each **research pick** on its device with a real study prompt ("Summarize the difference between
mRNA and DNA vaccines in exactly 3 bullet points"). Both produced correct, well-formatted, instruction-
following answers (3 clean bullets) — a night-and-day jump over the 230M's leaked-context garbage.

| Device | Model | decode t/s (real run) | prefill t/s | model on disk | ~RAM | quality |
|---|---|---|---|---|---|---|
| Phone (Dimensity 7300, t=4) | LFM2.5-1.2B-Instruct | **15.9** | 92 | 697 MB | ~1.4 GB | 3 correct bullets, followed "exactly 3" |
| Laptop (i7-11800H CPU, t=8) | Qwen3-4B-Instruct-2507 | **14.0** (llama-bench) | 99 | 2.32 GiB | ~2.6 GB | detailed, accurate, well-formatted |

**Key cost observation:** on **CPU**, the laptop's 4B (~14 t/s) is *no faster than* the phone's 1.2B
(~14-16 t/s) — the laptop's win is running a **bigger/better** model at an acceptable-but-not-fast
speed, not running the same model faster. **Wiring the RTX 3050 (CUDA llama.cpp) is the laptop's real
unlock** — it would take the 4B from ~14 t/s to comfortably interactive and free the CPU entirely.
Also observed: running large models in `/data/local/tmp` alongside the app can pressure Android LMKD
into reaping the app — reinforcing that the phone should stay at ~1.2B, not push toward 4B.

## Measured device-resource usage (per-process — the model's own load, not system absolute)

RAM figures elsewhere in this doc were *estimates*; these are **measured**. Each number is the model
**process's own** peak RSS and CPU% (laptop via `/usr/bin/time -v` on `llama-bench`; phone via
`/proc/<pid>/status` and `top` on the separate `libllama-server.so` process during a real generation).
Because they are per-process, they are the model's **incremental** footprint — independent of whatever
else is running (browser, etc.).

| Device | Model | peak RSS (measured) | CPU during generation | cores used / total | % of RAM |
|---|---|---|---|---|---|
| Laptop (t=8) | Qwen3-4B-2507 | **4.06 GB** | **741%** | ~7.4 / 16 threads | ~4.0 of ~5 GB free |
| Laptop (t=8) | LFM2.5-1.2B | **1.19 GB** | 624% | ~6.2 / 16 threads | small |
| Phone (t=4) | LFM2.5-1.2B | **~0.90–0.95 GB** | **~340–450% (avg ~385%)** | ~4 of 8 cores | ~12% of 8 GB |

**What this means for "good performance without blocking":**
- **Phone is comfortable.** The 1.2B holds ~0.9 GB (of ~3.4 GB free) and its 4 threads use ~4 of 8
  cores — **4 cores stay free for the UI**, confirming the interface isn't starved. This is the
  measured basis for the "doesn't block" claim, not an assumption.
- **Laptop 4B is memory-tight.** Its **real peak is ~4.06 GB** (not the ~2.5 GB the model file
  suggests — KV cache + compute buffers add up), against ~5 GB free. With a browser and other apps
  running this leaves little headroom; if the laptop feels pressured, the fallback is `lfm2.5-1.2b`
  there too (1.19 GB) or wiring the RTX 3050 (moves weights to VRAM, frees system RAM).
- **CPU at the benchmark-optimal thread count is high** (7.4 cores on the laptop, ~4 on the phone) —
  fine while a reply streams for a few seconds, but it is a *burst*, not steady load; the agent only
  runs during a turn. *(Observation to revisit: the phone's model process runs at `nice -10` — elevated
  priority; the 4-core cap protects the UI regardless, but a neutral/positive nice would be safer.)*

## Laptop GPU (RTX 3050) — measured, and it is the decisive laptop win

The laptop was running CPU-only. llama.cpp ships no CUDA binary, but its **Vulkan** build runs on the
RTX 3050 through the proprietary driver's Vulkan ICD (driver 595, the GPU exposes `NV_coopmat2` matrix
cores) — no CUDA toolkit needed, just the already-present `libvulkan.so.1`. Measured with all layers
offloaded (`-ngl 99`, `GGML_VK_VISIBLE_DEVICES=1` to pin the NVIDIA GPU over the Intel iGPU):

| Model | CPU t=8 (decode) | **RTX 3050 Vulkan (decode)** | speedup | GPU memory |
|---|---|---|---|---|
| Qwen3-4B-2507 | 14.0 tok/s | **51.9 tok/s** | **3.7×** | 2.53 GB VRAM (of 3.7 GB free), 97% util |
| LFM2.5-1.2B | 48.1 tok/s | **171.5 tok/s** | 3.6× | fits easily |

**Why this is the laptop's answer, not just "faster":**
- The 4B goes from sluggish (14 t/s) to **comfortably interactive (52 t/s)**.
- It moves the model **off system RAM onto VRAM** — the ~4.06 GB CPU-RSS pressure against ~5 GB free
  (flagged above) is **replaced by ~2.5 GB VRAM + minimal system RAM**, so the laptop stops being
  memory-tight *and* the CPU cores are freed (the GPU does the compute → the interface stays smooth).

**To wire it in** (not yet done — a runtime change): the desktop agent runs `agents/runtime/llama-server`
from the pinned **CPU** build (`runtime_url`). Point it at the **vulkan** asset
(`llama-b10076-bin-ubuntu-vulkan-x64.tar.gz`) and pass `-ngl 99` (+ `GGML_VK_VISIBLE_DEVICES` to pick
the discrete GPU). Caveat: the vulkan binary needs `libvulkan.so.1` present, and it still carries CPU
backends so it falls back when no GPU is found — but a machine with no Vulkan loader at all would need
the CPU build, so GPU should be an **opt-in runtime**, not the silent default.

## Mechanism note

`models.toml` has a single `default`, and the mobile build embeds that same file (`include_str!`). To
run **350M on the phone and 1.2B on the laptop**, the two need to diverge: either a per-device default
(the shell/serve picks by platform) or a mobile-specific manifest. Simplest interim: set the shared
default to **350M** (safe and good on both; the laptop just isn't using its full headroom), with 1.2B
as the documented, user-selectable step-up.
