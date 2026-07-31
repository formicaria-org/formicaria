# Features — the index (always-read)

One line per feature: **what it is · status · where the detail lives.** This is the discovery layer —
it names what exists and points at the on-demand doc, so nothing durable is invisible. Keep it
**brief**: a feature earns a dedicated `features/<slug>/` doc only when its lessons overflow a line
here. Status: ✅ shipped · 🟚 partial · 📋 planned. "the code wins" — verify a claim before trusting it.

| Feature | Status | One line | Detail / plan |
|---|---|---|---|
| **Notes & storage** | ✅ | files-as-truth: one note = one `<ulid>.md`; `Store`/`FileStore`/`MultiStore`, FTS5 index (disposable) | `overview.md` (seams) · `decisions.md#seams` |
| **Query & views** | ✅ | pure `fm-query` engine; board/agenda/timeline/search, generic literal-free renderers, `.view` files | `overview.md` · `decisions.md#ui` |
| **Editor & workspace** | ✅ | pane-grid workspace, `/` slash menu at caret, note↔note refs, double-click-to-edit | `decisions.md#ui` |
| **Media / blobs** | 🟚 | content-addressed blobs, ingest, streaming `GET /api/blob`; **video capped**; phone byte path is base64-over-IPC | `known-issues.md` · `decisions.md#vault` |
| **Whiteboard** | 🟚 | embedded Excalidraw, element-wise scene merge; **canvas-in-embed** still open, unverified in headless | `decisions.md#ui` · `known-issues.md` |
| **Collaboration (git sync)** | ✅ | per-vault git, `.md` merge driver, pull/push/squash, proposals (create/review/accept/reject), cross-user; **conflicts resolvable in-app for every git kind** (keep a side / mark resolved) | `decisions.md#git` · `decisions.md#sync` · `sessions/2026-07-24-proposals-on-the-phone.md` |
| **Share with a nearby device** | 🟚 | pair a tablet over the LAN; **per-vault scope**, code→`HttpOnly` token, host-bound commands denied; TLS (self-signed leaf, fingerprint checked by hand) unlocks the mic | `decisions.md#vault` (Scope) · `decisions.md#toolchain` (TLS) · `known-issues.md` |
| **Mobile / Android** | ✅ | one shared Rust core on the phone, git-coordinated via libgit2 (`native-git`); jniLibs + foreground service; startup is a contract (`pixi run android-smoke`) | `mobile-design.md` · `decisions.md#track-m` · `sessions/2026-07-31-the-gray-screen-on-first-open.md` |
| **Backup & acquisition** | ✅ | two-tier: git push (notes) + restic (media, opt-in); clone/restore via `acquire::naturalise` | `decisions.md#git` (Backup) |
| **Study assistant (agent)** | ✅ | on-device LLM answers `@name`, researches (opt-in web), drafts note edits; one-hop RAG; laptop+phone | `ai-agents-plan.md` · `model-selection-research-2026-07-22.md` |
| **Audio transcription** | 🟚 | `/transcribe` → whisper → proposal; laptop + phone (on-device whisper-server); multi-clip, skip-accepted | `audio-asr-research-2026-07-23.md` · `sessions/2026-07-24-proposals-on-the-phone.md` |
| **Image → LaTeX** | 📋 | a measured spike, behind an onnxruntime NOTICE gate; no defensible phone path yet | `ai-agents-plan.md` (Part III) · `outstanding.md` |

**Forward work queue:** [outstanding.md](./outstanding.md). **Why anything is the way it is:**
[decisions.md](./decisions.md) (grep a `#subject`). **Open gaps & traps:** [known-issues.md](./known-issues.md).
