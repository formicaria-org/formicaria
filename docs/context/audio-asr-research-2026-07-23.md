# Audio→transcript: ASR solution research (2026-07-23)

Deep research (4 parallel agents, cited) into how to run the **audio→transcript** specialist leaf,
framed by formicaria's constraints: **out-of-band loopback** (bytes-in/text-out, never a blob path),
**no native/opaque blob linked into the shipped Rust app** unless declared in a hand-written NOTICE
(cargo-deny can't see a `-sys` C lib), **stdlib-over-pip** preference, laptop v1 + Android later,
**short English clips** the primary case. The `Transcribe` seam already shipped is **model-agnostic**,
so the backend below is swappable without touching the substrate/runner/UI.

## The correction that reframes everything

whisper.cpp **ships a prebuilt, CPU-only, portable Linux x64 `whisper-server`** in its GitHub
releases (v1.9.1: `whisper-bin-ubuntu-x64.tar.gz`, + an arm64 variant). Verified by direct
download+inspection: `whisper-server` (1.3 MB) + `libwhisper.so` + 14 `libggml-cpu-*` variants, **no
CUDA**, ~24 MB unpacked, links only stock system libs. GitHub is reachable from our sandbox.
→ [releases API](https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest) ·
[the asset](https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.1/whisper-bin-ubuntu-x64.tar.gz) ·
[server README](https://github.com/ggml-org/whisper.cpp/blob/master/examples/server/README.md)

This means the shipped `WhisperServer` client + a `fetch.sh`-style tarball fetch (exactly the
`llama-server` pattern) is a **zero-build, zero-Python** path to a real run — no Moonshine or conda
needed to close the gap. Only the ggml `.bin` weights are HF-hosted, but a `.bin` is one
self-contained file (bundleable/mirrorable; nothing HF-specific runs at inference).

## The four strategies (each a clean swap behind the seam)

### A. Prebuilt whisper.cpp `whisper-server` — the pragmatic v1
- **What**: fetch the github tarball, run `whisper-server -m ggml-base.en.bin`, POST multipart audio to
  `/inference` → `{"text":…}`. Our `WhisperServer` already speaks this.
- **Pros**: no build, no Python, ~24 MB; **unified laptop+phone** (same ggml `.bin` both places; arm64
  build cross-compiles like `llama-server`); MIT; clean kill semantics (sidecar + commit-on-200).
- **Cons**: Whisper's fixed 30 s framing wastes compute on very short clips (minor on a laptop); ggml
  weights from HF (one file). Multilingual (a plus, not needed here).
- Fit: **best immediate path**; closes the gap today with the code already shipped.

### B. sherpa-onnx (Python wheel + a stdlib HTTP shim) — the strategic substrate
- **What**: `pip install sherpa-onnx` (CPU wheel **~2–12 MB**, onnxruntime bundled inside it, Apache-2.0,
  Python 3.7–3.14); wrap its **offline recognizer** in our existing stdlib `http.server`.
  → [PyPI](https://pypi.org/project/sherpa-onnx/) · [models](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html)
- **Pros**: **weights come from GitHub releases, not HuggingFace** — the only option that avoids HF
  outright; a *runtime hub* that can host **Moonshine, zipformer, paraformer, Parakeet, Whisper** as
  ONNX; **prebuilt Android AAR + tiny `.so` (libonnxruntime ~15 MB + libsherpa-jni ~3.7 MB) + ready
  demo APKs** → the cleanest Android story. tiny zipformer int8 <200 MB, 3.88% LibriSpeech test-clean.
- **Cons**: introduces a **pip + onnxruntime** out-of-band helper (not stdlib-only like search-proxy);
  its shipped server example is WebSocket, so we write a small HTTP shim; on Android the idiom is a JNI
  lib, not the sidecar-binary pattern we standardized on.
- Fit: **best medium-term** if we want Moonshine-class quality + a no-HF, prebuilt-Android path.

### C. Moonshine via `useful-moonshine-onnx` (Python) — best short-English model, heavy deps
- **What**: `pip install useful-moonshine-onnx`; `transcribe(wav, 'moonshine/base')`; wrap in stdlib HTTP.
  → [PyPI](https://pypi.org/project/useful-moonshine-onnx/) · [paper](https://arxiv.org/html/2410.15608)
- **Pros**: purpose-built for **short clips** (compute ∝ audio length, no 30 s pad → ~3× lower latency);
  **beats Whisper base.en** (base 3.23% vs 4.25% test-clean) at 61 M params; MIT code+weights (~26/57 MB);
  **official Android SDK** (`ai.moonshine:moonshine-voice` on Maven, mmap `.ort`).
- **Cons**: heaviest Python deps (`numba`+`librosa`+`llvmlite`+`scipy`); weights from HF; **English-only**;
  hallucinates on sub-second/silence (gate with VAD). Note: sherpa-onnx can host Moonshine ONNX too —
  so B is a lighter way to *run* Moonshine than this package.
- Fit: the model we most want; run it **via sherpa-onnx (B)** rather than this heavy package.

### D. Pure-Rust (candle CPU-only, or tract) — the architectural ideal, bigger lift
- **What**: `candle` (CPU default = pure Rust, no onnxruntime) running whisper-tiny/base — a maintained
  `candle-transformers` example; or `tract` (pure-Rust ONNX, production ASR at Sonos) running a
  whisper/zipformer ONNX. → [candle](https://github.com/huggingface/candle) ·
  [candle whisper docs](https://docs.rs/candle-transformers/latest/candle_transformers/models/whisper/index.html) ·
  [tract](https://github.com/sonos/tract)
- **Pros**: **no native blob linked into the app** — cargo-deny sees the whole graph (Apache-2.0/MIT);
  no Python, no sidecar; in-process, no loopback needed.
- **Cons**: slower than whisper.cpp; more implementation work (tract needs whisper graph surgery — only
  `base` community-proven); **Moonshine only in the immature single-maintainer `whisper-apr` crate**.
  Must NOT enable candle's `cuda`/`mkl`/`accelerate` features or a native link returns —
  verify with `cargo tree -e no-dev` (no `*-sys`).
- Fit: **the end-state** we'd migrate to if we want zero-Python + zero-native-blob; not v1.

### Avoid
- **`whisper-rs` / `ort` / `sherpa-rs` (Rust bindings)**: each **links a native blob cargo-deny can't
  audit** (whisper.cpp / onnxruntime / sherpa-onnx), and ort+sherpa-rs *download* it at build time —
  each needs a hand-written NOTICE, defeating the seam's purpose.
- **faster-whisper**: most accurate but heaviest install, HF-bound → not lean.
- **Parakeet-0.6B / Canary / Kyutai on a phone**: Parakeet int8 ≈ **1.2 GB RAM** — too big for a
  mid-ranger. → [issue #2626](https://github.com/k2-fsa/sherpa-onnx/issues/2626)
- **Android built-in `SpeechRecognizer`**: OEM-fragmented, may run in cloud, no file/batch control —
  opportunistic fallback only, never the guarantee. → [Picovoice 2026](https://picovoice.ai/blog/android-speech-recognition/)
- **Silero CE**: non-commercial license. **MediaPipe**: has no STT task.

## Model quick-reference (short English, CPU)

| Model | Size | test-clean WER | Short-clip | Lang | License |
|---|---|---|---|---|---|
| Moonshine base | 61 M | **3.23%** | Excellent | En | MIT |
| Moonshine tiny | 27 M | 4.52% | Excellent | En | MIT |
| Whisper base.en (ggml) | 74 M / ~142 MB | 4.25% | OK (30 s pad) | En | MIT |
| Whisper tiny.en | 39 M / ~75 MB | 5.66% | OK | En | MIT |
| sherpa zipformer int8 | <200 MB | 3.88% | Excellent | En | Apache-2.0 |
| Parakeet-TDT-0.6B-v2 | 600 M | 1.69% | Good (1.2 GB RAM) | En | CC-BY-4.0 |

Sources: [Moonshine paper](https://arxiv.org/html/2410.15608) ·
[moonshine-base card](https://huggingface.co/UsefulSensors/moonshine-base) ·
[sherpa zipformer](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/zipformer-transducer-models.html) ·
[Parakeet card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2)

## Recommendation

Because the seam is model-agnostic, do it in **two moves, not one big bet**:

1. **Now — ship + prove with prebuilt whisper.cpp `whisper-server` (A).** It closes the gap today with
   the code already written, no build, no Python; fetch the github tarball like `llama-server`. Run
   `base.en` on the laptop. This gives you a real, working feature immediately.
2. **Then — migrate the backend to sherpa-onnx (B) for the edge/tiny + Android direction**, running a
   tiny zipformer or **Moonshine** ONNX: no HuggingFace, prebuilt Android artifacts, one ONNX model on
   both devices. The substrate/runner/UI don't change — only the loopback backend does.
   **Pure-Rust candle/tract (D)** stays the eventual zero-Python/zero-native-blob end-state.

This honors the plan's "measure before safe" and the owner's edge/tiny vision, while the whisper.cpp
step means we're never blocked on a real run.
