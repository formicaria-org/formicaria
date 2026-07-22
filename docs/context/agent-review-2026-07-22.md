# Study-agent adversarial review — 2026-07-22

Four lenses (portability/Android, seam-modularity, efficiency, philosophy/boilerplate) converged on one structural verdict: the *brain* of the agent is already portable (`StudyAssistant<L,S>` is pure, generic over `LlmStep`/`WebSearch`, needs no disk or network), but every *layer that runs it* is hard-bound to three localhost HTTP ports, a spawned x86 subprocess, a Python sidecar, and the git CLI — none of which exist on a phone. The findings below are deduplicated (the six-endpoint presence/activity channel, for instance, was independently flagged by three lenses and is merged) and ranked by impact × how much each unblocks Android.

## Executive summary

- **The single most important architectural change:** introduce one runner-level `VaultAccess` trait (get / thread / discussions / search / reply_as / create_proposal / present / activity) and make `Agent` generic over it — exactly the move `StudyAssistant` already made for the model and web. `FmServe` becomes the *desktop HTTP* impl; a second impl calls `fm_app::dispatch` **in-process** for the phone (and for the desktop one-shot path). This is the keystone: it collapses the localhost-:8765 dependency, routes proposal-creation through the single vault writer, and makes the identical agent run over HTTP on the laptop and over in-process dispatch on Android with **zero change to the orchestrator**. Almost every other portability fix either rides on this seam or becomes cheap once it exists.
- **Turn three ports and two languages into three traits.** Beyond `VaultAccess`: split a `ModelRuntime` trait (subprocess llama-server on desktop vs. in-process llama.cpp/LFM context on phone) out from the `LlmStep` *call* seam; and give `WebSearch` a real HTTPS-capable fetch (or route it through the app's existing Host/IPC HTTPS seam) so the Python search-proxy and SearXNG port vanish. These three seams are the whole portability story.
- **`fm-serve` has quietly become non-agnostic** — ~150 lines of agent machinery (4 `AppState` fields, six `/api/agent_*` endpoints, a bash-spawn path) now live in the "dumb transport shell" whose own header forbids it. The `rm -rf agents/ ⇒ byte-identical core` claim is false at the source and binary level. Extract it behind an `agent` cargo feature so the invariant is compiler-enforced, the way `fm-query`'s purity already is.
- **The presence/activity/@-mention channel exists only inside fm-serve's HTTP handlers**, so on mobile (no fm-serve) the working-wheel, @-picker, and heartbeat all silently no-op. Lift it into a transport-agnostic in-process `AgentRegistry` (two TTL boards) that desktop exposes over HTTP and the phone exposes over IPC.
- **The efficiency profile is the textbook Android-battery antipattern:** a resident model holding a full KV cache all day, a 1 Hz-forever poll with no backoff, presence heartbeats at 8× the needed rate, and a watchdog that guards `/proc` load-average (which a phone can't read and which lags bursts by ~a minute) instead of the thermal headroom the plan's own §15 mandates. Plus a **300 s wall-clock that SIGKILLs the "resident" model five minutes after launch** — a per-turn bound misapplied to a long-lived server, so the assistant reliably dies mid-session.
- **Recurring DRY/drift traps the repo has been bitten by before** reappear: `strip_echoed_prompt` re-types the prompt-assembly heading literals as a second silently-divergable copy; `models.toml`'s "single place" defaults are re-hardcoded in three Rust `Args` and re-parsed by three *already-divergent* awk copies; the @-mention picker is a hand-rolled second copy of the existing slash-menu autocomplete.
- **Cheap-but-high-value quick wins exist:** the 300 s cap fix is a one-line preset change; the presence-heartbeat rate and poll backoff are small timing edits; the `Board<V>` / `AgentRegistry` extraction is small and makes the untested "wheel never spins forever" expiry logic testable under `pixi run ci`.

---

## Theme 1 — Portability seams (the load-bearing refactors)

These are ranked first because each converts a desktop-only mechanism into a trait the phone can implement. Do them roughly in this order.

### 1.1 `VaultAccess` trait — the in-process dispatch seam *(the keystone)*
- **Problem** — `crates/fm-agent-run/src/lib.rs` (`Agent` struct + `llm()`/`web()`): `Agent` holds a concrete `FmServe` (hand-rolled TCP to fm-serve :8765); every store read/write goes over `TcpStream::connect`. On Android there is no fm-serve HTTP listener (external-fact #7 forbids ever shipping one), so the whole runner is structurally desktop-only even though the brain inside it is not.
- **Violates** — Seam 3 ("one door: `fm_app::dispatch`") and the "core never knows the agent exists" portability principle.
- **Refactor** — Make `Agent` generic over one `VaultAccess` trait. `FmServe` = the desktop HTTP impl; a second impl calls `fm_app::dispatch` in-process for mobile (and the desktop one-shot path). Smallest portable seam; unlocks 1.4 and Theme 2 for free.

### 1.2 `ModelRuntime` trait — split runtime lifecycle from the model call
- **Problem** — `crates/fm-agent-run/src/serve.rs` (`Command::new(llama-server)`) + `crates/fm-agent/src/launch.rs` (`SupervisedModel` over `std::process::Child`) + `watchdog.rs` (kill via `child.kill()`): `OpenAiStep` is a clean portable *call* seam, but the *runtime* is a prebuilt Ubuntu-x64 binary launched with `Command` and killed with SIGKILL. iOS forbids fork/exec (fact #8); Android only tolerates exec via the `lib*.so` trick from `nativeLibraryDir`, not a fetched x86 binary. `SupervisedModel` couples "the model" to "a Child," so there is no shape in which the model is an in-process context handle — the only shape that works on a phone.
- **Violates** — platform-abstracted model runtime.
- **Refactor** — Keep `LlmStep` as the call seam; add a `ModelRuntime` trait (start → healthy → stop) with `SubprocessRuntime` (desktop) and `InProcessRuntime` (llama.cpp/LFM via FFI, mobile). `StopFlag` already exists, so mobile "stop" drops the `llama_context` instead of SIGKILLing a pid. **Effort: large** — this is the one big lift.

### 1.3 Real HTTPS `WebSearch` — delete the Python sidecar and the third port
- **Problem** — `agents/search-proxy.py` + `agents/start-agent.sh` + `crates/fm-agent/src/search.rs` (`SearxngSearch` → TCP :8888): the `WebSearch` trait is right, but its only impl reaches a localhost port fed by a Python process that exists *purely because the hand-rolled Rust HTTP client has no TLS*. Android has no python3, no sidecar, no safe localhost listener. The whole `/search` leg is desktop-only for an implementation-shortcut reason, not an intrinsic one.
- **Violates** — platform-abstracted web-search / "nothing the phone cannot run."
- **Refactor** — Give the agent a real HTTPS-capable fetch behind `WebSearch`. Desktop: a TLS Rust client (keep the proxy as optional dev convenience). Android: route the outbound fetch through the Host/IPC seam already used for git-over-HTTPS and blobs. Collapses three ports to one seam and removes a language from the deploy story.

### 1.4 Route proposal-creation through dispatch — kill the one-shot's own FileStore + git CLI
- **Problem** — `crates/fm-agent-run/src/main.rs` (`FileStore::open` + `fm_core::git::commit_all`) vs. the resident path in `lib.rs` → `fmserve.rs` (`create_proposal` via dispatch): two divergent paths for one job. The one-shot binary opens its **own** FileStore and calls `git::commit_all` directly, which (a) races the running server's SQLite (known-issue: no `busy_timeout`) and (b) hard-binds proposal creation to the git *binary* — and there is no git binary on Android (fact #0). So on the phone the agent literally cannot emit its only output.
- **Violates** — "git is a capability, not a dependency" + single-vault-writer invariant.
- **Refactor** — Delete the direct-FileStore/`git::commit_all` path; route the one-shot through the same `VaultAccess`/dispatch seam (rides on 1.1). Proposal branches then inherit the platform's git capability (the libgit2-in-memory backend the app already uses on-phone) for free. One path, one writer.

### 1.5 `AgentLauncher` seam — replace the bash/awk/python launch chain with a Rust entry point
- **Problem** — `crates/fm-serve/src/main.rs` (`spawn_agent → Command::new("bash").arg("agents/start-agent.sh")`) + `agents/{start-agent,agent-serve,fetch,serve}.sh`: load-bearing decisions (which model file, which ports, proxy start/stop, log rotation, liveness-tied lifecycle) live in shell + awk parsers, forked via bash. Android has no bash and no freely-spawnable children. On Windows/Android `spawn_agent` silently no-ops via `script.exists()`, reading as "agent off" rather than "unsupported here" — the exact failure the `Host`/`open_external` trait was introduced to avoid.
- **Violates** — the Host-trait/no-`cfg`-ladder ruling; portability.
- **Refactor** — Move orchestration (parse `models.toml`, resolve model file/ports, own the run lifecycle) into a Rust library entry point in `fm-agent-run` that both frontends call; expose it behind an `AgentLauncher` seam (a trait, or reuse `Host`) where desktop-linux spawns and other platforms return `Unsupported`. Keep the shell scripts only as a thin dev convenience holding no logic a phone needs. "Enable the agent" becomes a function call, not a bash fork.

### 1.6 Android-aware `ResourceMonitor` — govern by thermal, not `/proc`, and by cancellation, not SIGKILL
- **Problem** — `crates/fm-agent/src/watchdog.rs` (`SystemMonitor`: linux `/proc` else `Err`; `Resources` = mem + `load_per_core`) + `preflight.rs` (`admit`): the only impl reads `/proc/meminfo`+`/proc/loadavg`, and the non-Linux default returns `Err` → fail-closed → **the agent never runs on Android**. Worse, the phone's real governor is the OS (LMKD, Doze, Thermal API), and load1 is a ~1-minute EWMA that can't see an inference burst. The whole watchdog is structured to SIGKILL a pid — meaningless for an in-process runtime.
- **Violates** — cross-platform device-safety (§15: thermal headroom, back off before the cliff; §16: CPU pressure not smoothed load).
- **Refactor** — Extend the `Resources` snapshot (the trait is already the clean OS seam) with `thermal_headroom: Option<f32>` and, on Linux, augment/replace load1 with instantaneous PSI CPU pressure (`/proc/pressure/cpu`, event-driven `poll()`). Android's impl fills memory from `ActivityManager.MemoryInfo` and headroom from `getThermalHeadroom()` via JNI. Separate "supervise a Child" from "govern a run": `StopFlag` gives the transport-neutral off-switch, so in-process is governed by cooperative cancellation + thermal back-off. Keep fail-closed, but give the phone an impl so fail-closed doesn't mean "never runs there."

---

## Theme 2 — fm-serve's agnostic-core violation and the presence/activity channel

All four lenses hit this file. Merged into three items.

### 2.1 Extract all agent machinery behind an `agent` cargo feature
- **Problem** — `crates/fm-serve/src/main.rs` (`35-131, 404-497, 226-233`): the "dumb HTTP shell that must never learn what a command *means*" now hard-codes 4 `AppState` fields, the `Activity` struct, six `/api/agent_*` branches, four helper fns, and an auto-start block. The `rm -rf agents/ ⇒ byte-identical` claim (comment at line 228, plan §14) is true only of *runtime behavior* (`spawn_agent` no-ops without the script); the source and compiled binary are permanently non-agnostic.
- **Violates** — CLAUDE.md "core never knows the agent exists" / fm-serve-is-a-shell doctrine; plan §14 extractability claim.
- **Refactor** — Move all of it into one `agent_channel` module behind a `agent` feature. `main.rs` keeps two touch-points: one `agent: AgentChannel` field on `AppState`, and one line in `handle` — `if let Some(resp) = agent_channel::route(&path, &body, &state.agent) { return … }`. Under `--no-default-features` the module isn't compiled, making "byte-identical pure build" literally true and the agnostic-core invariant compiler-enforced.

### 2.2 Lift presence + activity into a transport-agnostic `AgentRegistry` (two TTL `Board`s)
- **Problem** — merged from three findings. `crates/fm-serve/src/main.rs` (`AppState.agent_activity`/`agent_present` + the six endpoints, `35-80`, `404-497`): `agent_activity` and `agent_present` are two instances of *one* pattern — an ephemeral in-memory `key→(value,timestamp)` board the out-of-process agent writes and the browser reads, expiring on read (180 s vs 8 s the only real difference), hand-written twice with divergent rigor (these arms `.lock().unwrap()` and panic on poison; the rest of the file uses `if let Ok`). The expiry policy — "the wheel never spins forever," the failure every studied tool shares — lives nowhere testable. And because this channel exists *only* in fm-serve's HTTP handlers, on mobile the working-wheel (`NotePanel.svelte` `pollAgent`), the @-picker (`onlineAgents`), and the agent's heartbeat all silently no-op (`ipc.ts` even `.catch(()=>({active:false}))` to hide it).
- **Violates** — "a transport owns only its framing" (dispatch design); no-second-HTTP-port portability constraint; simplicity/DRY; the untested-expiry gap.
- **Refactor** — A generic `Board<V>{ map: Mutex<HashMap<String,(V,Instant)>>, ttl }` with `put`/`get`(expiring on read)/`keys`, and an `AgentRegistry { activity: Board<Activity>, presence: Board<()> }` owning both maps and the three durations as named consts. It's owned beside `App`, mutated in-process by the runner, and read by whichever transport is present: desktop fm-serve over its existing HTTP endpoints, mobile over IPC — no second HTTP port. Pure and `pixi run ci`-testable (drive the clock, assert expiry). Consistent lock-poison handling; ~65 lines of near-duplicate map juggling collapse to one tested type.

### 2.3 Give the transport endpoints typed parsing, `serde_json`, and a real constructor
- **Problem** — merged from three findings. `crates/fm-serve/src/main.rs`: each `/api/agent_*` block hand-parses with `from_slice(&body).unwrap_or(Value::Null)` then `v["field"].as_str().unwrap_or("")` and hand-builds replies with `format!("{{\"enabled\": {}}}")` — reintroducing the "API silently accepts malformed JSON → empty-string arg" defect known-issues.md:432 already flags, six times, none covered by CI. `wait_ready` (`serve.rs`) decides the model is up by a **substring match** on the JSON body (`contains("\"status\":\"ok\"")`). `AppState` is hand-constructed field-by-field in three places (`main.rs:172`, `main.rs:771`, `blob.rs:246` — the last re-imports `Mutex`/`HashMap` inline), so every new field edits three sites.
- **Violates** — the file's "commands live in dispatch" invariant; known-issues.md malformed-JSON trap; decisions.md "parse, don't substring"; the fm-serve test-coverage gap.
- **Refactor** — Parse each body into a `#[derive(Deserialize)]` struct (reject malformed input with a 400, don't coerce to empty); serialize with `serde_json::json!`. Parse the health body and check `v["status"] == "ok"`. Fold the self-initializing runtime state into `AgentChannel`/a `#[derive(Default)]` `Runtime`, and give `AppState::new(app, dist, origins, port)` — tests build through it with no knowledge of agent fields; a new field touches one `Default`, not three literals. Makes the whole set unit-testable through the existing `request()` harness.

---

## Theme 3 — Efficiency / battery (the Android idle-cost story)

The plan's §15/§16 device research is explicit: zero idle cost, launch-on-demand, no warm pool, event-driven self-throttle, thermal-not-load. The current runtime violates each. Thermal is folded into 1.6 above.

### 3.1 The 300 s wall-clock SIGKILLs the "resident" model five minutes after launch *(highest-value quick fix)*
- **Problem** — `crates/fm-agent-run/src/serve.rs:87` (`Limits::conservative`) + `watchdog.rs:61-70,161-164`: the resident @name watcher is meant to keep the model warm "on with the app, off with it," but launches it under `max_duration = 300s`, and `supervise` unconditionally SIGKILLs the child once `elapsed >= max_duration`. Five minutes in, the model dies, `finished()` flips true, the loop breaks, the assistant exits. Per-turn time is already bounded by `OpenAiStep` (120 s), so the cap buys no safety here — only a cliff.
- **Violates** — plan §8 "human in control + off-switch" (an *unintended* teardown); the design intent "model on with the app."
- **Refactor** — Separate continuous *resource* safety (memory/thermal, resident, no wall-clock) from per-*turn* time bounding (already on `OpenAiStep`). Give the resident launch a `Limits::resident()` preset with `max_duration` effectively unbounded; keep `max_duration` only for genuinely one-shot runs. **Effort: small.**

### 3.2 Idle-unload instead of warm-resident (the "warm pool" §16 says to avoid)
- **Problem** — `serve.rs:71-88,133-234`: the model is launched once and kept resident for the app's whole life with `-c 2048`. llama.cpp pre-allocates the full KV cache at load, so a resident server pins model RAM + a 2048-token KV cache continuously — while the agent is idle almost all the time (a user @-mentions it seconds per hour). On a phone this resident footprint is what LMKD reaps first; holding it warm all day is pure battery/RAM waste for a bursty workload.
- **Violates** — plan §15 "zero idle cost / launch-on-demand"; §16 "keep_alive=0 … no warm pool."
- **Refactor** — After N idle seconds with no pending mention, stop the model via the existing `stopper()` and relaunch on the next mention (preflight already gates the restart). Accept cold-load latency on the first post-idle mention (§16 already lists it as a number to measure). Keep warm-resident only for the interactive `agent-chat` REPL, gated behind a launch flag rather than hard-wired "always warm."

### 3.3 Adaptive backoff for the @name watcher (currently 1 Hz forever)
- **Problem** — `serve.rs:56-58,133-134`: the loop sleeps a fixed `poll_secs` (default 1) and every wake POSTs `present` + GETs `discussions` regardless of activity — 1 Hz permanently whether the last mention was 1 s or 6 h ago. Precisely the sustained background-wakeup pattern Doze/App-Standby suppress, and it contradicts §15's event-driven ideal.
- **Violates** — §15 "event-driven self-throttle, zero idle cost"; Android "lean into Doze/App-Standby."
- **Refactor** — Keep 1 s only while a turn is live or a mention arrived recently; after K quiet polls widen 1→2→5→30 s, snapping back on any change in `discussions` count. Pure timing, fully portable. Better: add a server-side long-poll (`discussions?wait=30s`) or a monotonic dirty-since cursor so the watcher blocks until something happens — event-driven, the plan's ideal.

### 3.4 Presence heartbeat beats 8× too fast; three transports all near 1 Hz
- **Problem** — `serve.rs:148` (present every poll) + `main.rs:492` (`retain < 8s`): the heartbeat fires at 1 Hz while the server only expires presence after 8 s, so 7 of every 8 heartbeats are redundant. The UI independently polls `agents` + `agent_activity_poll` + `thread` every 1.5 s (`NotePanel.svelte:858`). Nothing coalesces these.
- **Violates** — §15 "fewer round-trips / zero idle cost"; simplicity (one heartbeat, one source of truth).
- **Refactor** — Beat presence at ~5 s (inside the 8 s window, ~80 % cut) or fold liveness into the `discussions` response the agent already makes (zero extra round trips). On the read side, one UI poll returns activity + online-agents together. Tie heartbeat period and server expiry to one shared constant so they can't drift out of a safe ratio. **Effort: small.**

### 3.5 The 15 s force-scan defeats the count-skip; `handled` and `timing.jsonl` grow unbounded
- **Problem** — `serve.rs:151-164,176` (`force_scan`) + `:112` (`handled` HashSet) + `:276` (`timing.jsonl`): `last_count` exists to skip discussions whose count is unchanged (one cheap `discussions()` when idle), but every ~15 s `force_scan` bypasses it and fetches `thread(id)` for *every* discussion and re-scans *every* message, forever — O(D) full-thread fetches 4×/min on a battery device. `handled` accumulates every message id for the life of the process (a slow leak); `timing.jsonl` is appended every turn with no rotation (agent.log rolls at 5 MB, this doesn't).
- **Violates** — §15 "idempotent/resumable, zero idle cost"; the count-skip's own stated intent.
- **Refactor** — Trust the authoritative count and drop the periodic full sweep. If defense-in-depth is still wanted, have fm-serve expose a monotonic per-discussion high-water mark (last message id) in the `discussions()` payload the agent already fetches — O(1) per discussion, no thread re-reads. Replace the global `handled` set with a per-discussion high-water cursor (bounded by discussion count, not lifetime messages). Give `timing.jsonl` the same size-roll as agent.log, or gate it behind a debug flag.

### 3.6 (Low) Persistent keep-alive connections — do *after* cutting call count
- **Problem** — `crates/fm-agent/src/http.rs:15-31` + `fmserve.rs`, `openai.rs`, `search.rs` all set `Connection: close`: every call opens a fresh `TcpStream`, `read_to_end` to EOF, and drops it. At ≥2 calls/s forever, that churns thousands of ephemeral sockets/hour — syscall + TIME_WAIT bookkeeping that is pure battery on the target device.
- **Violates** — §15 "fewer round-trips."
- **Refactor** — Hold one persistent keep-alive connection per peer, framing responses by Content-Length/chunked (the `body()` decoder already parses chunked; single Content-Length is the small missing piece), behind the existing `http` seam so `openai`/`search`/`fmserve` inherit it. **Lower priority** — cut the *number* of calls (3.2–3.5) first, then reuse the connection for what remains.

---

## Theme 4 — Boilerplate, DRY, and drift traps

These don't block Android but are the repo's recurring failure mode (duplicated literals that drift with a green build). Cheap, and each removes a latent bug.

### 4.1 `strip_echoed_prompt` re-types the prompt-assembly heading literals *(highest correctness risk here)*
- **Problem** — `crates/fm-agent/src/lib.rs:313-369` vs `assemble_turn_context:374-388` / `assemble_prompt:392-412`: the stripper hard-codes `const HEADINGS: [&str;6]` — the same section titles the assemblers emit, re-typed as a separate lowercased array. Two sources of truth: rename a heading in the assembler and the stripper silently keeps matching the old string, stopping cleaning the exact shape it exists to clean, with a green build. Identical to the copy_note/merge-driver/renderer-literal drift already catalogued. The second pass (`361-369`) is an admitted "best remaining guess" on adversarial model output, covered by one hand-written example per shape.
- **Violates** — "enumerate, don't duplicate a literal" (decisions.md); the module's own "never guessing at the prose" claim.
- **Refactor** — Make section titles named constants and build *both* the prompt and the echo-boundary set from them — one source of truth, compile error if a heading is removed. Better: wrap the answer request in a sentinel the assembler owns (`<<<ANSWER>>>…<<<END>>>`) and extract between sentinels — a structural signal the model was given, not prose we pattern-match — falling back to raw text when absent. Add a property test driving arbitrary echo interleavings.

### 4.2 One `normalize_answer` seam for small-model output cleaning
- **Problem** — `crates/fm-agent/src/lib.rs`: `run()` (fence only), `summarize()` (fence only), `turn()` proposal (fence only), `turn()` chat (fence + echo + cap). The same class of workaround (`strip_wrapping_fence`, `strip_echoed_prompt`, `cap_reply`, `PROPOSAL_ACK`, the `truncated()` refusal) is applied in four different subsets, with no declared policy — so a `/propose` body the model echoes into ships un-stripped, and a fifth call site inherits whichever subset the author copied.
- **Violates** — simplicity/clarity; the "one engine, two call sites, unable to diverge" pattern decisions.md praises for `merge_texts`.
- **Refactor** — One `fn normalize_answer(raw, opts) -> Result<String, AgentError>` owning fence-strip, optional echo-strip, optional cap, and the truncation refusal; route all four sites through it with an explicit `opts` per path so the differences become a visible decision. Pure, fully under CI. **Effort: small.**

### 4.3 `models.toml` "single place" is re-hardcoded in three `Args` and re-parsed by three divergent awk copies
- **Problem** — `agents/models.toml:12-21` vs `serve.rs:40-51`, `chat.rs:24-34`, `main.rs:44-48`; parsers in `agents/{fetch,serve,agent-serve}.sh`: `default="lfm2.5-230m"`, `port=8081`, `ctx=2048`, `threads=4` are re-typed as clap defaults in three binaries, and the TOML is read by three hand-rolled awk parsers that **have already diverged** — `fetch.sh`'s `conf_get` scopes to the top-level table, the other two don't, so the same key can resolve differently depending on which script reads it.
- **Violates** — the manifest's own "single place" claim; duplicated-logic-drift (decisions.md fm-cli).
- **Refactor** — Either (a) one `models.toml`-reading function in a sourced `agents/_lib.sh` all three scripts use (kills the awk divergence, no toolchain change), or better (b) since the runner already depends on serde, add a `--manifest agents/models.toml` path so the runner reads the same file the fetch script does and clap defaults come from the manifest. Pairs naturally with 1.5 (Rust launch entry point). Also collapse the two binaries' shared clap args into a `#[command(flatten)]` `AgentOpts` + `impl From<AgentOpts> for Agent`, and define the default model name as one `pub const` in `fm-agent-run`.

### 4.4 The @-mention picker is a second hand-rolled copy of the slash-menu autocomplete
- **Problem** — `ui/src/lib/NotePanel.svelte:811-984` (`atMenu`/`onReplyInput`/`chooseAtMention`/`onReplyKeydown`) duplicates `177-186,1385-1480` (slash / `runSlashSearch` / `chooseSlash` / `onSlashKeydown` / `slashAnchor`): the `@`-picker re-implements the trigger regex, results list, index cycling, Enter/Tab choose, Escape close, click-to-choose, and caret anchoring — the keydown cascade mirrors `onSlashKeydown` almost line for line. Every future fix (caret-anchoring, touch-keyboard positioning, index wrap) must be made twice.
- **Violates** — simplicity-modularity-clarity (one primitive, two uses).
- **Refactor** — Extract one `CaretAutocomplete` primitive (Svelte component or `.svelte.ts` controller) parameterised by `{ triggerRegex, resultProvider, renderItem, onChoose }`, owning caret anchoring (shared `caret.ts`) and navigation once. Instantiate for `/` (note/embed search) and `@` (live agent names); generalises to future caret-triggered inserters.

### 4.5 (Low, but correctness) reply-author path guesses provenance by substring
- **Problem** — `crates/fm-app/src/dispatch.rs:296-309` (and `350-355, 793-798, 822-827`): the authored-commit change filters written paths with `p.to_string_lossy().contains(&meta.id)` to commit "just this one message" — a ULID substring match as a proxy for provenance the store already knows exactly (`commands::reply` wrote exactly one path); a latent over-commit if one id is a substring of another path. The `written → commit_all(_as) → clear_written` sequence is hand-inlined in four arms, so the authored variant became a fifth spelling.
- **Violates** — correctness (substring provenance); simplicity/clarity.
- **Refactor** — Have `commands::reply` return the path it wrote so the transport never guesses. Factor the sequence into one `Vaults::commit_written(cfg, msg, author: Option<(&str,&str)>)` helper; the reply arm passes `Some(author)`, the others `None`. Removes the fragile filter and unifies five commit sites.

### Dropped as weak
- **Duplicate `NoWeb`/`NoSearch` stubs** (`lib.rs:187-192`, `main.rs:52-57`) — a real observation (the `S: WebSearch` generic is vestigial on the conversational `turn()` path, satisfied only by a stub written twice), but low value on its own. Fold the trivial fix (one canonical `fm_agent::NoWeb`) into whatever touches the runner next rather than as its own task; the deeper "move the web seam off the turn-path generic" is not worth a dedicated change now.

---

## Sequenced plan

**First — cheap, high-value, no new architecture (do this week):**
1. **3.1** Give the resident launch `Limits::resident()` (unbounded `max_duration`). One-line preset; stops the assistant dying at 5 minutes. *Small.*
2. **3.4** Slow the presence heartbeat to ~5 s (or fold into `discussions`); tie period and expiry to one shared constant. *Small.*
3. **3.3** Adaptive poll backoff (1→2→5→30 s, snap back on change). *Small–medium, pure timing.*
4. **4.2** `normalize_answer` seam; **4.1** heading constants + property test. *Small; kills a real drift-with-green-build bug.*
5. **2.2/2.3** Extract `Board<V>` / `AgentRegistry` and add the `AppState::new` constructor + typed endpoint parsing. *Small–medium; makes the untested expiry logic CI-testable and fixes the malformed-JSON/`unwrap`-poison traps.*

**Second — the portability keystone and what rides on it (the core project):**
6. **1.1** `VaultAccess` trait with an in-process `fm_app::dispatch` impl. *Medium — the single most important change; everything below assumes it.*
7. **1.4** Route the one-shot proposal path through that seam; delete the direct-FileStore/git-CLI path. *Medium; rides on 1.1, fixes the SQLite race and the no-git-on-Android blocker.*
8. **2.1** Extract fm-serve's agent machinery behind an `agent` cargo feature. *Medium; makes "byte-identical core" compiler-true.*
9. **1.5** Rust launch entry point + `AgentLauncher` seam; **4.3** single manifest reader. *Medium; retire the bash/awk/python chain, kill the awk divergence.*
10. **1.3** Real HTTPS `WebSearch`; retire the Python proxy + SearXNG port. *Medium.*
11. **1.6** Android-aware `ResourceMonitor` (thermal headroom, PSI pressure, cancellation-not-SIGKILL). *Medium; without it Android fail-closes and never runs.*

**Third — the large lift and the deferrable polish:**
12. **1.2** `ModelRuntime` trait with an `InProcessRuntime` (llama.cpp/LFM via FFI). *Large — the one genuinely big piece; the phone's only viable model shape, but everything above can land and be validated on desktop first.*
13. **3.2** Idle-unload behind a launch flag; **3.5** drop the force-scan, bound `handled`, roll `timing.jsonl`. *Medium.*
14. **4.4** `CaretAutocomplete` primitive; **4.5** return-the-written-path + `commit_written` helper; **3.6** keep-alive connections. *Medium; do after call-count cuts and once the seams settle.*

**Honest note on scope:** items 6–11 are the real portability project and interlock — sequence them narrow-and-deep (one seam landed and green under `pixi run ci` before the next), not in parallel. Item 12 is the only *large* effort and can be deferred until the desktop-side seams are proven, because the same generic `Agent` will exercise them there first.