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

## Mechanism note

`models.toml` has a single `default`, and the mobile build embeds that same file (`include_str!`). To
run **350M on the phone and 1.2B on the laptop**, the two need to diverge: either a per-device default
(the shell/serve picks by platform) or a mobile-specific manifest. Simplest interim: set the shared
default to **350M** (safe and good on both; the laptop just isn't using its full headroom), with 1.2B
as the documented, user-selectable step-up.
