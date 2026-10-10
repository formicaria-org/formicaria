# Image → text on the phone — every route, not only transformers (research, 2026-10-10)

**Status: research, nothing adopted.** Asked by the owner after two readers were measured on the
phone (`agents/bench/results.md`, 2026-10-10): PaddleOCR-VL-1.6 at 145–224 s per image and
LFM2.5-VL-450M at 3–5 s. The question: *what else exists for a phone, including engines that are
not vision-language models?* What is **measured here** and what is **only read** is marked on every
row; nothing read has been run on our hardware.

## The filter (unchanged, from `transcription-specialists-grounded-2026-08-31.md`)
- The phone is a Dimensity 7300, 8 cores, no usable GPU, about 3 GB free.
- **No native blob `cargo-deny` cannot audit** without a hand-written NOTICE: this is what has kept
  onnxruntime, Paddle and Tesseract out.
- The working pattern is the bundled `llama-server` sidecar. Anything that runs on it adds a model
  file, not a dependency.
- Same behaviour on every OS, and split by platform only where the platform truly differs.

## The five families

| Family | Examples | Speed on a phone | Reads | Cost to this project |
|---|---|---|---|---|
| **1. Small vision-language model on the engine we ship** | LFM2.5-VL-450M (**measured**) | **3–5 s, ~10 s for a large image**, 1.2 GB | print, tables as Markdown; weak on formulas, code nesting, charts | one model file; no new dependency |
| **2. The phone's own text recognition** | Google ML Kit v2 (Android), Apple Vision (iOS) | "real-time on most devices" for Latin (Google's words) | printed text in lines; no formulas; Apple's newer API also returns tables and lists | a native bridge per platform; Google's library and terms on Android |
| **3. Classic two-stage OCR (detect lines, then read them)** | PP-OCRv5 mobile, Tesseract, `ocrs` | tens to hundreds of ms per line on a desktop CPU; not measured on a phone | printed text; PP-OCRv5 claims handwriting | a new runtime (ONNX Runtime, MNN or Tesseract's C++), which is the ruled-out kind, **except `ocrs`, which is Rust** |
| **4. The same classic OCR, run inside the app's web view** | `@paddleocr/paddleocr-js`, tesseract.js | no phone numbers published | as family 3 | a WebAssembly runtime in the UI bundle instead of a native library |
| **5. One-job recognisers** | formulas: Texo, UniMERNet-T, PP-FormulaNet; handwriting lines: TrOCR-small | not measured | exactly one kind of content | a second runtime again; needs something else to find the region first |

## What each route is, with what its makers say

### 1. Small vision-language models (what we measured)
- llama.cpp's own guide (10 April 2026) lists the OCR models it runs: LightOnOCR, Qianfan-OCR,
  PaddleOCR-VL ("may have degraded performance"), GLM-OCR, DeepSeek-OCR, Dots.OCR, HunyuanOCR, and
  as general models that can read text **LFM2.5-VL-450M**, Qwen3-VL-2B and Gemma 4 E2B/E4B.
- Measured today: LFM2.5-VL-450M is the only one under 1 GB that both read correctly and ran in
  seconds. SmolVLM-256M invents content; granite-docling-258M is no faster than PaddleOCR-VL.
- **Not yet measured and worth one run each:** GLM-OCR (0.9 B, MIT, 95.2 on OmniDocBench in a
  September 2026 roundup) and Qwen3-VL-2B. Both are larger than LFM2.5-VL-450M, so slower; the
  question is whether they fix formulas and code at a time still worth waiting for.

### 2. The phone's built-in recognition
- **Android, ML Kit Text Recognition v2.** Two ways to ship it: *unbundled* (about 260 KB, the model
  downloaded and run by Google Play services) or *bundled* (about 4 MB per script, linked into the
  app). Scripts: Latin, Chinese, Devanagari, Japanese, Korean. Google asks for characters of at
  least 16×16 px. The page says nothing about handwriting; Google's handwriting product (Digital Ink)
  reads pen strokes, not photographs.
- **iOS, Apple Vision.** `RecognizeTextRequest` (accurate path by default, with language correction);
  from iOS 26, `RecognizeDocumentsRequest` also returns tables, lists and paragraphs as structure.
- **The cost here is not speed, it is the seam.** It is closed-source, differs per platform, exists
  on neither Linux nor Windows, and on Android is governed by Google's ML Kit terms. That is a
  platform split of exactly the kind the *same behaviour on every OS* rule exists to question.

### 3. Classic OCR engines
- **PP-OCRv5 mobile** (Baidu, Apache-2.0): the recogniser is 16 MB, and Baidu reports about 21 ms
  per line on a server CPU and "over 370 characters per second" for the pipeline. It is built for
  "handwriting, vertical text, pinyin, and rare characters", with no handwriting-only score
  published. **Every way to run it needs a runtime we have ruled out** (Paddle-Lite, ONNX Runtime,
  MNN). The Rust crates that wrap it (`oar-ocr`, `paddle-ocr-rs`, `ocr-rs`) all link one of those.
- **Tesseract**: small and fast on clean print, poor on handwriting (43 % character errors on one
  German handwriting set, against 9 % for a large vision model). C++, ruled out natively.
- **`ocrs`** (Rust, Apache-2.0/MIT): the one classic engine whose whole stack is Rust: its models
  run on `rten`, a Rust runtime. Latin script only, and by its own description "in an early preview.
  Expect more errors than commercial OCR engines." No speed or accuracy figures are published.
  **This is the only classic route that passes the native-blob rule as written**, so it is the one
  worth measuring.

### 4. Classic OCR inside the web view
- The phone app is a web view, so OCR can run there as WebAssembly with no native library at all.
  Baidu ships an official SDK, `@paddleocr/paddleocr-js` (PP-OCRv5 on ONNX Runtime Web, WASM with
  SIMD and threads); tesseract.js is the older equivalent.
- Nobody publishes phone timings for either. It also moves the dependency from Rust to the UI
  bundle (ONNX Runtime's WASM build and OpenCV.js), which `cargo-deny` does not audit at all: the
  rule's letter is met and its purpose is not.

### 5. One-job recognisers
- **Formulas:** UniMERNet-T (about 100 M parameters) leads open models; Texo (2026, about 20 M) claims
  comparable accuracy at a fifth of the size and in-browser use; `rapid_latex_ocr` is about 170 MB of
  ONNX. These would fix exactly what LFM2.5-VL-450M got wrong (`V^2` read as `≈2`).
- **Handwriting lines:** TrOCR-small is about 61 M parameters; the base model took about 2 s per line
  on an Apple M1 in one browser project.
- Each needs a detector to find the formula or the line first, and each needs ONNX Runtime or a
  port. This is the *Image → LaTeX* row already parked in `features.md` behind the NOTICE gate.

## Reading of it
1. **LFM2.5-VL-450M is already the cheap, fast answer**: seconds, one file, no new dependency, same
   on every platform the engine runs on. Nothing found here beats that combination.
2. **The built-in recognisers are faster still but cost a platform split** and Google's terms. They
   make sense only if "instant, printed text only" is wanted as a separate feature (for example
   live text in the camera), not as the reader.
3. **`ocrs` is the one unmeasured option that fits the house rules.** If it reads print as well as
   the small model and in well under a second, it could be the fast path for plain print, with the
   model kept for tables. It is early, Latin-only, and unproven on handwriting.
4. **Formulas and handwriting are the real gaps**, and no fast phone option closes them without
   ONNX Runtime. On the laptop PaddleOCR-VL already reads formulas exactly.
5. **Nothing here has been run on handwriting.** Every accuracy claim above is the maker's, on
   print or on mixed benchmarks.

## What to measure next, cheapest first
1. `ocrs` on the five fixtures on the laptop, then cross-compiled to the phone.
2. GLM-OCR and Qwen3-VL-2B on the laptop CPU, for the formula and code fixtures only.
3. The owner's own handwritten and photographed pages through whatever survives 1 and 2.

## Sources
- llama.cpp's OCR guide: <https://huggingface.co/blog/ggml-org/using-ocr-models-with-llama-cpp>
- ML Kit Text Recognition v2, Android: <https://developers.google.com/ml-kit/vision/text-recognition/v2/android>
- Apple Vision, `RecognizeDocumentsRequest`: <https://developer.apple.com/documentation/vision/recognizedocumentsrequest>
- PP-OCRv5: <https://www.paddleocr.ai/main/en/version3.x/algorithm/PP-OCRv5/PP-OCRv5.html> ·
  browser SDK: <https://www.paddleocr.ai/main/en/version3.x/inference_deployment/cross_platform/browser.html>
- `ocrs`: <https://github.com/robertknight/ocrs> · `rten`: <https://docs.rs/crate/rten>
- German handwriting comparison (Tesseract, TrOCR, a large VLM): <https://huggingface.co/naeyn/de-htr-web>
- UniMERNet: <https://arxiv.org/pdf/2404.15254> · Texo: <https://www.arxiv.org/pdf/2602.17189>
- September 2026 OCR roundup (secondary): <https://aitechmodel.com/best-open-source-ocr-models/>
