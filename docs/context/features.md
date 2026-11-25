# Features — the index (always-read)

One line per feature: **what it is · status · where the detail lives.** This is the discovery layer —
it names what exists and points at the on-demand doc, so nothing durable is invisible. Keep it
**brief**: a feature earns a dedicated `features/<slug>/` doc only when its lessons overflow a line
here. Status: ✅ shipped · 🟚 partial · 📋 planned. "the code wins" — verify a claim before trusting it.

| Feature | Status | One line | Detail / plan |
|---|---|---|---|
| **Notes & storage** | ✅ | files-as-truth: one note = one `<ulid>.md`; `Store`/`FileStore`/`MultiStore`, FTS5 index (disposable); renders as you write | `overview.md` (seams) · `decisions.md#seams` |
| **The shell** (computer) | ✅ | one frame on every page: the rail (＋ New, Find or do, Home, Journal, sections with counts, sync; Settings only outside formicaria's browser, whose ⚙ is top right), folding away with « or Ctrl+\; a note beside the page; sections are `.kind` files (`sections.rs`); a section hides from the rail; a red dot on Collaboration for proposals never shown | `decisions.md` 2026-11-22 (night, latest) · `unifying-home-research-2026-11-22.md` |
| **Home & Journal** | ✅ | Home is a note tagged `home`: every card (figures and links too) placed on a 12-column grid by ⠿ drag, arrows or ⋯, sized by its edges, settling upward, Tidy up; the phone reorders by a held ⠿ (glance/summary/full, show on the phone), heatmaps; generated from use until edited; Pin to Home. Journal: a day's entry (`journal`, titled with the day) made on the first keystroke; the Journal page holds only the journal (entry, earlier days); on the phone it takes the screen while you write | `decisions.md` 2026-11-23 (all four) · `unifying-home-research-2026-11-22.md` §12 |
| **Web & Find or do** | ✅ | Web is a section (pages kept); Find or do opens a typed address, offers *Search the web* first, shows what Enter does; Ctrl+Enter searches the web | `decisions.md` 2026-11-23 |
| **Vaults shown** | ✅ | the rail's vault chooser (two or more vaults) hides a vault's notes from every page and the counts; a view preference, never an entitlement | `decisions.md` 2026-11-23 (evening) · `decisions.md#vault` |
| **Query & views** | ✅ | pure `fm-query`; board/agenda/timeline/search/table/gallery through literal-free renderers; *View as* on every section; no shelves or tag row (2026-11-25, search narrows); `.view` files; only a task is overdue | `decisions.md#ui` |
| **Appearance / themes** | ✅ | `<vault>/themes/*.css`, git-tracked, chosen per device; four starters, a form, a CSS box; every built-in theme kept legible by a contrast test | `decisions.md#ui` · `themes.contrast.test.ts` |
| **Editor & workspace** | ✅ | one live editor; `/` menu, `[[` links, note refs, ⋯ (Export, History, Pin to Home); touch row over the keyboard; the phone's bar steps aside while anything is typed | `decisions.md#ui` |
| **Undo & Back** | ✅ | ↶ menu and Ctrl+Z for the session's own writes; Recently deleted from git; Back is one step everywhere | `decisions.md#ui` |
| **Media / blobs** | 🟚 | content-addressed blobs; chunked ingest (no size refusal); ranged serving on the phone; the phone cannot show a PDF inline (handed to the phone's app) | `known-issues.md` · `decisions.md#vault` |
| **Whiteboard** | 🟚 | embedded Excalidraw, element-wise scene merge; canvas-in-embed open | `decisions.md#ui` · `known-issues.md` |
| **Collaboration (git sync)** | ✅ | per-vault git, `.md` merge driver, proposals (create/review/accept/reject), conflicts resolved in-app; the phone merges prose too | `decisions.md#git` · `decisions.md#sync` |
| **Share with a nearby device** | 🟚 | pair a tablet over the LAN, per-vault scope, `HttpOnly` token, self-signed TLS | `decisions.md#vault` |
| **Mobile / Android · iOS** | 🟚 | one Rust core on the phone (libgit2); Home and Journal as tabs; iOS an unsigned `.ipa` (not verified here) | `mobile-design.md` · `decisions.md#track-m` · `known-issues.md` |
| **Backup & acquisition** | ✅ | git push (attachments under a 100 MB ceiling) + optional restic of notes and blobs; backs up by itself, gets their changes first; a vault moves in-app | `decisions.md#git` · `decisions.md#vault` |
| **Updating the app** | 🟚 | one press: gets, verifies, installs in place (desktop) or via Android's installer; browser kit and add-on with it | `decisions.md#toolchain` |
| **Papers / library** | 🟚 | a paper is a note tagged `paper`, from BibTeX, DOI, arXiv link or title, offline | `decisions.md#data` |
| **Study assistant (agent)** | ✅ | **Myrme**, own repo, pin 0.13.0: answers `@name`, researches listed sources, drafts edits as proposals; reads meetings from mail (time ranges, the sender's clock) | Myrme `decisions.md` · `decisions.md#agent` |
| **Audio transcription** | 🟚 | `/transcribe` → whisper → proposal; installs itself (Linux, Windows; no macOS build) | `decisions.md#agent` |
| **Image → text (in `/transcribe`)** | 🟚 | photographed pages → transcript (math as LaTeX, code fenced) as a proposal | `decisions.md#agent` |
| **Meetings from calendar & mail** | ✅ | iCal every 5 min; Gmail read-only, one note per conversation, headings with the sender's clock; the assistant proposes meetings | `decisions.md#agent` · the manual's Mail chapter |
| **Import from Logseq / Obsidian** | ✅ | a graph becomes notes: links resolved, tags lifted, attachments copied, journals dated | `logseq-import-plan-2026-10-14.md` · `decisions.md#data` |
| **Trackers: food, programs** | 🟚 | a logged item is a note with `log`; totals in `fm-query`; diets and regimes followed, plans in force by `start`/`due` | `decisions.md#data` · `cards-evidence-2026-10-16.md` · `cards-guidance-proposal-2026-10-16.md` |
| **Reminders** | 🟚 | a task's or meeting's time, said; a computer rests for them; on a phone only in the open app | `reminders.rs` · `known-issues.md` |
| **Health** | 🟚 | trackers you name, a doctor's page, cards, a cycle that counts; phone untried | `health-plan-2026-10-22.md` |
| **Music** | 🟚 | songs as logged notes; a deck and Songs/Albums/Artists/Playlists | `music-plan-2026-11-13.md` |
| **Privacy · Save media · Videos** | 🟚 | phone: adblock with uBO's lists, app blocker, DoH; computer: Firefox strict, DoH; media from any site → Music/Videos/note | `privacy-plan-2026-11-19.md` |
| **formicaria's browser** | 🟚 | Firefox ESR with formicaria's kit: the ant, feature and note tabs, Bring in; about +5–8 % memory | `browser-plan-2026-11-15.md` |
| **Release checks** | ✅ | browser-check, usability-check (personas; layout guard at laptop and phone width), phone-media-check (emulator; keyboard-up Home, the ＋ sheet, a held-⠿ move), check-references, `ci` | `decisions.md#toolchain` · `decisions.md` 2026-11-23 (late) |
| **Image → LaTeX** (a *dedicated* recogniser) | 📋 | a measured spike; no defensible phone path yet | `archive/ai-agents-plan-superseded-2026-09-02.md` · `outstanding.md` |

**Forward work queue:** [outstanding.md](./outstanding.md). **Why anything is the way it is:**
[decisions.md](./decisions.md) (grep a `#subject`). **Open gaps & traps:** [known-issues.md](./known-issues.md).
