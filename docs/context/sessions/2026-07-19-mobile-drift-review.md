# 2026-07-19 — Track M drift review: the record vs the rulings

**An adversarial review, not shipped code.** The owner asked whether the mobile app was ready,
then for a critical review of the *updated* plan against the *initial* one — to judge whether we
are diverging and how to stay on track — with the decisions **taken** under the owner-s stated
principles rather than deferred back.

Method: three reconstructed states (the owner-s first draft, recovered from the `(corrected)`
markers since it is not in git; the plan as committed 2026-07-18 in `9e01fd5`; reality at HEAD
`a7b34ce`), a divergence map, five adversarial lenses, four rulings each attacked by three
skeptics, plus a completeness critic. **All four rulings drew 3/3 refutations and were revised.**

Externally verified the same day, and folded in: `gix` push is still unimplemented; conda-forge
ships all four `rust-std-*-linux-android` targets but no NDK/SDK; PuppyGit and GitSync both use
libgit2 on Android (GitSync migrated JGit -> Rust `git2` and ships Android **and** iOS from one
Rust core); Termux is a dead end (impossible on iOS, unsupported for embedding on Android);
Syncthing-Android was discontinued in Oct 2024 and iOS background sync is structurally
unachievable — which **validates** git-as-coordinator rather than undermining it.

The full synthesis is kept verbatim below. It is the receipts behind the `decisions.md` entry and
the `mobile-design.md` repairs of the same date.

---

# TRACK M — FINAL SYNTHESIS

*Verification basis: working tree `/home/baljinder/formicaria`, clean, HEAD `a7b34ce`. Every anchor below re-checked directly in this session or by at least two independent lenses that corrected each other. Where a claim rests on external fact I say so; where I could not verify, I write UNVERIFIED.*

---

## 1. VERDICT

**Not diverging in judgement. Diverging badly in the record, and the record is now large enough to make a competent reader build the wrong thing. Severity: HIGH on documentation, LOW on decisions, and one live data-loss defect on desktop that the mobile port would make unrecoverable.** Every ruling made since the plan was committed on 2026-07-18 — rejecting `git2`, rejecting `Incremental`, rejecting the `fm-cli`-onto-`dispatch` migration, refuting the pragmatic-DnD pointer adapter, deferring Ruling B's image strip — traces to a stated principle and is, on the evidence, correct; `decisions.md:151-181` is the strongest single piece of reasoning in the repo and it chose the expensive answer over the convenient, already-sequenced one. The failure is that **the plan's sequence was never re-derived after its foundation was rejected**: 9 of 12 sequenced items and 7 of the 8 items inside the MVP cut line are still specified in terms of `git2`, and `plan.md:409`'s `⛔ BLOCKED` marker is doing the work of a rewrite it cannot do — it promises a foundation that is not coming back. Compounding it, three of the corpus's load-bearing sentences are false against the tree: `mobile-design.md:392`/`:395-398` assert the merge is in-process when `merge.rs:204` and `:233` are both `Command::new("git")`; `mobile-design.md:10` reports one untouched ruling and one shipped ruling as rejected; `plan.md:406-407` advertises a closed decision as open debt. And underneath all of it, `decisions.md:580-586` states the invariant that pays for the whole collaboration design — *"frontmatter is always emitted whole and valid, so a conflict lands in the body"* — four lines above its own counterexample, which reproduces today, is untested, and fires when two people drag one card to different columns.

---

## 2. THE DRIFT

### A → B: the owner's draft → the committed plan (2026-07-18)

The adversarial review was roughly **70% principled, 30% factual**, and **every principled change survives into today unreversed**:

| Change | Class | Authority |
|---|---|---|
| "nothing forks, two frontends already" → false; extract `fm_app::dispatch` | principled + factual | a Tauri bridge would be a *third* dispatch surface (`mobile-design.md:63-69`) |
| Added ruling 2B (`fm-serve` on device) as a documented retreat | principled | *"the browser is the product"* (`decisions.md:733`); the draft never weighed the cheaper architecture (`mobile-design.md:105`) |
| `diffy` rejected | principled | *"reaching for a niche crate on the one path that must never corrupt is the move the plan forbids everyone else"* (`mobile-design.md:166-168`) |
| OAuth-default → PAT-first | principled | OAuth *"vendors you to each host's OAuth-App program"* (`mobile-design.md:202-209`) |
| Keystore token: reversal owned in writing | principled | scoped to the one fact forcing it (`mobile-design.md:213-221`) |
| `git2` port surface 3 fns → all 13 | principled + factual | *"a red herring for a subprocess→library port"* (`mobile-design.md:279-284`) |
| `set_identity` before first commit | principled | `PLACEHOLDER_EMAIL` attributes a shared vault to one fake name (`mobile-design.md:236-245`) |
| Explicit `commit → push → pull → re-push` | principled | `push_squashed` is *designed* to reject and tell the user (`mobile-design.md:346-351`) |
| Path B is the destination, A is a demo | principled | Path A *"retreats toward the satellite-of-desktop model the owner overrode"* (`mobile-design.md:371-374`) |

**But the review did not hold its own facts to the standard it enforced.** Three defects originate in B, not A: `git2::merge_file` asserted as *"verified against the git2-rs docs"* and non-existent (`mobile-design.md:141-149`); the pragmatic-DnD pointer adapter (struck later, `plan.md:480-483`); and the "512 MB cap" error inherited from A and not caught (`mobile-design.md:322-324`). That is the exact self-exemption B diagnosed in A, reproduced one level up. It is the pattern to watch, because §3 shows it happened again.

### B → C: what shipped, what was rejected

**Shipped, all convergent:** `fm_app::dispatch` with 30 arms and the one-method `Host` seam (`crates/fm-app/src/dispatch.rs:81`, `:159`); the streaming blob route (`crates/fm-serve/src/blob.rs`); the pure-Rust whiteboard scene merge (`crates/fm-core/src/scene.rs`, wired at `merge.rs:183`) — which needed **none** of ruling 3 and refuted its premise as a side effect; the explicit sync loop with two hardenings beyond the ruling (`ui/src/lib/sync.svelte.ts`); the board tap→move fallback (`ui/src/renderers/Card.svelte`); the mobile CSS shell; the liveness/reindex split.

**Rejected on principle, all correctly:** `git2` (`decisions.md:151-181`), `Reindex::Incremental` (`plan.md:462-467`; `file.rs:106` is still `Reindex::Full`), `fm-cli`-onto-`dispatch` (`decisions.md:130-149`), the pointer adapter, Ruling B's ordering premise (`decisions.md:187-205`).

### The orphaning, quantified

Counting `mobile-design.md:536-569` — 3 spikes + M0–M8 = **12 numbered items**:

- **Name `git2`/`libgit2` explicitly (6):** spike (ii) `:539`, spike (iii) `:541-542`, M0 `:545`, M1 `:548`, M2 `:551`, M6 `:561`
- **Void by dependency without naming it (3):** M3 (`push_squashed`/`unpushed`/`remote_moved` are subprocess ops at `git.rs:580`, `:493`, `:695`), M4, M5
- **Free of it (3):** spike (i) — shipped — M7, M8

**→ 9 of 12 (75%). Restricted to M0–M8: 7 of 9 (78%). Inside the MVP cut line (`mobile-design.md:571`, end of M4): 8 items, 1 shipped, 7 void or partly void.**

### Three things that are true and appear in no project document

1. **`git.rs` has no `clone`.** Verified this session — 13 public fns at lines 66, 93, 244, 259, 303, 392, 406, 493, 523, 580, 695, 719, 803, none of them clone. M1 is the gate to the entire MVP, is framed as a `git2` **port**, and is **new code on any backend whatsoever**. This scope error survives every possible resolution of the backend question.
2. **M4 is blocked three times, not once.** No backend; no text-merge engine (`merge.rs:204` and `:233` are both `Command::new("git")` — verified); and — recorded nowhere — `merge_files` is *driver-shaped*: it takes three paths on disk and writes over `ours` (`merge.rs:58`, `:75`), which on desktop exist only because git materialized `%O %A %B`. A phone caller cannot produce them without an ODB reader, i.e. the backend it does not have.
3. **`fm_app::vaults::config_dir()` does not compile for Android.** Verified this session: three `#[cfg]` arms — `linux`, `macos`, `windows` — and no fallback (`crates/fm-app/src/vaults.rs:239-257`). Android's `target_os` is `"android"`, so every arm is skipped and the fn falls through to `()`. The first Android build fails in the file ruling 1 *moved into* `fm-app`.

### Accidental vs principled — the honest split

Every **decision** was principled. Every **divergence** was accidental, and all of it is one kind: refutations were recorded in the file that holds them and never propagated to the files that depend on them. `plan.md` still says BLOCKED where `decisions.md:151` says rejected. `plan.md:406-407` still calls the `fm-cli` fork *"the open half of 'one command library'"* after `decisions.md:147` ruled *"one implementation of each command, not one entry point."* `crates/fm-cli/src/main.rs:7-8` still says migrating onto `dispatch` *"is what would finally make 'one command library' true"* — the source asserts the debt the ruling retired. `ui/vite.config.ts:14` still references a `tauri.conf.json` that has never existed in this era (verified: no such file anywhere in the tree).

---

## 3. THE RULINGS

**All four rulings were attacked by three skeptics each. All four drew 3/3 refutations. All four are REVISED below, and I say what changed and why.**

---

### RULING 1 — GIT BACKEND — **REVISED under attack (3/3)**

**Taken:** `gix` is the named Android direction. **Ship-a-git-binary is struck permanently. `git2`'s rejection stands. Path A is a demo, never an end state.** But the ruling's operative clause — swap the body merge engine on *both* platforms, plus own `send-pack` now — does not survive.

**What the skeptics broke, and I accept:**

1. **The desktop merge engine does not change.** All three refuted this. `merge.rs:14-16` states the principle at the site: *"Shelling out is the whole point: there is no diff3 here to get wrong."* `decisions.md:578-579` restates it as a standing ruling with **"no new dep"** in the sentence. `gix-merge` is permissive so it clears C10 — but C10 was one of four grounds; C8 and C12 do not care about the licence. `gix-merge` goes behind the same non-default `native` feature as everything else, exercised only by the conformance suite. **This also preserves `git merge-file` as a permanent oracle, which is the single most valuable structural property available and is exactly what a desktop swap would destroy.**
2. **The differential gate cannot assert byte-identity.** gitoxide's own `crate-status.md` lists `[ ] inter-hunk merging based on proximity` and `[ ] newline-related options` unimplemented, upstream's source carries `// DEVIATION: … probably is very different from what Git does` on the line-ending detection that formats conflict markers, and there is **no git-parity claim for blob text merge** (the `[x]` parity claim attaches to *tree* diff). Byte-identity is unattainable by design gap. The gate asserts **verdict identity** (clean vs conflict), **`frontmatter::from_file` parses both outputs**, **no `<<<<<<<` inside the YAML fence**, and **no field loss**. Marker-byte divergence is recorded as a bounded known difference, not pretended away.
3. **"Zero new C" is false.** Struck. There is no zero-C HTTPS path in `gix-transport`: `aws-lc-sys` (BoringSSL fork, C+asm) enters via rustls/reqwest and ships **no pregenerated bindings for `aarch64-linux-android`**, so Android needs libclang; the curl feature needs `curl-sys` + `libz-sys`. The true claim is still a good one: *`gix` adds C only for TLS, permissively licensed and visible in `cargo tree`, versus libgit2's ~230k lines of GPL C entering under a wrong licence.*
4. **The dependency cost was under-reported by an order of magnitude.** `gix-merge`'s closure is ~99 crates, **69 brand-new** against a 109-crate baseline (+63%), including `gix-command`+`shell-words` (subprocess spawning), `gix-index`+`memmap2`, and `jiff-tzdb` (a bundled IANA database) landing in `fm-core`. Transport adds ~110 more. This is the largest genuine objection and it must be stated at full size.
5. **`cargo deny` fails on the transport half today.** `webpki-root-certs 1.0.9` is `CDLA-Permissive-2.0`, not on the allowlist, reached via `rustls-platform-verifier → reqwest → gix-transport`. The ruling's own stop condition (*"if any transitive dep needs a waiver, that is a signal to stop"*) fires on first contact. It is fixable and honestly so — but it must be a conscious allowlist decision taken up front, because gate-visibility is the decisive argument.
6. **`send-pack` is not authorized by this ruling.** Two skeptics invoked `plan.md:66-69` — *"No CRDT library, no sync framework, no plugin API — those are the ones that would own us"* — and the precedent at `decisions.md:190-191` where a **blob mirror** was rejected as *"a sync framework by another name."* Hand-written pkt-line + thin-pack + `report-status` is more sync machinery than a mirror. It is reduced to a **spike with a written kill criterion**, and it needs its own ruling that cites and rebuts that precedent. Also: `push_squashed`'s rollback fires only on `!out.status.success()` (`git.rs:656-668`); with a hand-written client, "success" becomes our own parser's opinion, and a false success destroys granular history for a backup that never happened. **Make the rollback independent of the client's verdict — verify the remote ref moved via a separate `ls-remote` — before any spike.** That change is worth more than the spike and pays under every option.
7. **"The phone's `.git` is disposable" is overstated.** True for current bytes (A1); false for unpushed history, which is the entire product being bought. Restate honestly or delete.

**What survives, and the principle that carries it:** `deny.toml:1-4` (verified verbatim) scopes the GPL carve-out to tools *"invoked as subprocesses and never appear in this graph"*, and `ci/third-party.sh:25` is `for pkg in fm-serve fm-cli` (verified) — a `cargo tree` walk that produces **no row at all** for a bundled binary. Shipping git in an APK makes us the licence distributor *and* the CVE distributor of a TLS stack, invisible to both gates. That is structurally worse than the bug `decisions.md:163-168` rejected `git2` for. **Struck, not deferred.** One skeptic correctly notes the *reason* should be recorded as dated technical fact plus a packaging obligation, not as a new licence principle — GPLv2 §2 aggregation is real and `MASTERPLAN.md:262` already has the packaging answer for a `.deb`. Adopted: strike option 1 on the Android facts, do not rewrite `deny.toml`'s ratio.

**Owned cost:** two merge implementations for a period (the trap `mobile-design.md:306-307` names) — accepted because the alternative is *zero* on mobile and the second is continuously graded by an oracle that never retires; a 69-crate closure confined to a non-default feature; `gix-merge` is upstream-disclaimed on the path D22 protects; a Kotlin/JNI component for TLS trust roots (`rustls-platform-verifier`) that breaks the one-shared-Rust-core thesis at exactly one point and must be scoped in writing the way the Keystore reversal was.

**Reversal condition:** the differential harness cannot go green on **verdict** identity → stop; the merge stays subprocess, mobile stays merge-blocked, and steps 1/4/5 of §4 still ship and still pay. `cargo deny` needing a waiver to admit gitoxide → the answer is wrong. A maintained permissive pure-Rust git with push appears → swap and delete. gitoxide ships `send-pack` → delete our spike. **Does not reverse it:** `gix` push staying unimplemented (that is the assumption, not a risk), or Android toolchain difficulty (`rusqlite bundled` at `crates/fm-core/Cargo.toml:18` pays the NDK regardless — "avoid C cross-compilation" was never a live argument for any option).

---

### RULING 2 — THE FRONTMATTER CORRUPTION — **NEW, and unbundled from everything else**

This did not start as a separate ruling. It has to be one, because **two skeptics on the backend decision and two on the resequence decision all refuted the same clause**: *"ours wins the field, the loser goes into the body."*

**The bug is real and reproduced.** `merge_objects` returns `None` on any `three_way` failure over `kind/title/status/due/start/hard` (`merge.rs:104-122`); `merge_files:68` then calls `whole_file`, which line-merges the **entire file including the YAML fence** (`merge.rs:232`). Base `status: todo` / ours `doing` / theirs `done` puts `<<<<<<<` inside the fence; `yaml.safe_load` raises `ScannerError`; `frontmatter::from_file` takes the same path. The codebase already knows — `file.rs:417-420`: *"The commonest cause is a conflicted merge — git's markers land inside the YAML fence and `from_file` rightly rejects it."* **`decisions.md:580-586` asserts the invariant four lines above its own counterexample, and `crates/fm-cli/tests/merge.rs` has five tests and none of them covers it.** The trigger is two people dragging one card to different columns.

**The proposed fix is refuted.** `merge.rs:32-35` verbatim: *"**We never resolve that by fiat: silently dropping one side's status change is the same data loss this whole phase exists to stop.**"* `decisions.md:584-586` restates it. "Ours wins the field" is last-writer-wins on the field Board, Agenda and Calendar query, and it is worse than it looks: bodies are identical in the card-drag case, so `merge.rs:165-167` returns **Clean**, `commit_all` auto-commits at 5s, and `sync.svelte.ts` pushes it. Silent loss on the sync path. It also breaks parity with a driver-less collaborator (`merge.rs:229`: *"What git would have done without us"*), which is `decisions.md:169-171`'s reason for convicting `git2`, self-inflicted.

**The "silent" framing is also wrong.** Skipped notes are surfaced by name today: `file.rs:426`, `:471` collect with reason → `file.rs:140` exposes → `dispatch.rs:125`, `:342`, `:560` carry it on `Ping` → `App.svelte:477-487` reports *"N note(s) could not be read and are missing from every view — usually a conflicted merge."* Today's failure is **loud-and-absent**. The proposed fix makes it **quiet-and-present** — the card sits in our column looking resolved.

**RULING:** the semantic change does not ship under a resequencing or hygiene mandate. What ships now:

1. **A characterization test** in `crates/fm-cli/tests/merge.rs`: divergent `status` → `Merged::Conflicted`, fence broken, note name appears in `skipped`, notification fires. Locks the designed behaviour so nobody changes it by accident.
2. **The UI surface** — a "needs attention" place, not a toast, listing skipped notes with an editor that opens them raw. This is F28 applied: *"A phase that ships only Rust shipped nothing"* (`plan.md:70-73`). It is desktop-first, verifiable by eye, and violates nothing.
3. **If the frontmatter behaviour is to change, it goes to the owner as its own ruling with its own adversarial pass**, and the acceptance criteria are: both values survive in the file, the file parses, and the result is `Merged::Conflicted` (never Clean — otherwise each machine keeps its own value by fiat, both commit, and `three_way(base, doing, done)` re-derives `None` forever, accreting a duplicate body block per round).

**Reversal condition:** none — this ruling is a refusal to decide under the wrong mandate. It expires when the owner takes the decision.

---

### RULING 3 — TOOLCHAIN — **REVISED under attack (3/3)**

**Taken and surviving:** Android is a pixi **feature and two environments, never a platform** (closed by fact — pixi's manifest enumerates no android subdir). The tier split is the correct line and it is the ruling's spine: **a hash-pinned versioned artifact is compatible with the house rule; a second package manager resolving against a mutable vendor manifest is not.** So the NDK enters as a single zip pinned by our own SHA-256 in a committed `android/toolchain.lock`; `sdkmanager` and everything it resolves is quarantined into an opt-in `android-apk` environment declared non-hermetic. The four noarch `rust-std-*-linux-android` targets, `openjdk` and `gradle` are all conda-forge-native, so **the gap is NDK + SDK only** — `plan.md:502`'s *"the toolchain escapes pixi"* is overstated (`mobile-design.md:581`'s *"not cleanly conda-packaged"* is precise). `pixi run ci` gains no NDK dependency, ever; the expensive gate is opt-in; two cheap NDK-free checks go into the always-on gate.

**What the skeptics broke, and I accept:**

1. **There is no "the project's restic repo," and the ruling invented one to carry its decade claim.** `pixi.toml:21` is a package declaration, not a provisioning fact. Restic here is **per-vault, caller-supplied, optional** (`backup.rs:41-45`, `dispatch.rs:636`, `:691`), the owner personally corrected an agent for the one-repo-for-the-set error (`decisions.md:541-546`), and `backup.rs:10-14` — verified verbatim this session — deliberately narrows the snapshot set because a vault's root *"holds your source, your `.env` and a `.git`, none of which belong in a restic repo that may be a lab's rather than yours."* A 1 GB Google NDK zip is that same fixed bug at one order of magnitude. **Replace with: our SHA-256 in the committed lock is the durable, project-owned assertion; custody is the owner's choice**, resolved by `$FM_NDK_MIRROR` (same shape as `FM_VAULTS`/`FM_RESTIC_REPO`) — which is `decisions.md:466-468` applied to the build layer.
2. **The mirror must be the read path, not the fallback.** "Mirror first, `dl.google.com` fallback" means an empty mirror falls through to Google silently for years — the ruling's own reversal condition describes its day-one state. Vendor fetch becomes an explicit `ndk-fetch --from-vendor` that *populates* the mirror.
3. **The C-half check is blind, demonstrated empirically.** `cargo check -p fm-core --target aarch64-linux-android` **exits 0 with no NDK at all**, because pixi's `c-compiler` activation always sets `CC`, so the `cc` crate finds the host compiler and deposits an **x86-64** `sqlite3.o` into the `aarch64-linux-android/` tree. `cargo build` doesn't save it either — no workspace crate declares `crate-type`, so `fm-core`/`fm-app` are rlibs and rlibs never link. **Use a linking target (`cargo build -p fm-cli --target …`) and assert the ELF machine is AArch64, not the exit code.** Add a guard refusing to run unless `CC_aarch64_linux_android` resolves inside `.android/ndk/`. And record the trap: pixi converts rusqlite#503 from a loud absence into a **silent wrong-architecture build** — in this repo, a misconfigured Android toolchain produces green builds.
4. **`config_dir()` must gain an android arm before the gate exists** (`crates/fm-app/src/vaults.rs:239-257`) — verified this session. It is a precondition, not a follow-up.
5. **Strike "sideload has no 16 KB deadline."** Google states it as a **device** property: without recompiling, apps won't work on 16 KB devices in future Android releases. Play deferral buys relief from submission floors and Console overhead only. Risk #1 moves down one notch, not off the podium. **The alignment flag still ships, explicitly (`-Wl,-z,max-page-size=16384`), and the assertion moves to the first linked `.so` — tier 1 as specified produces no linked artifact for `llvm-readelf` to read.**
6. **Do not declare F31 un-violated.** Keep `mobile-design.md:581-583` and **narrow it in place**: pixi remains the only package manager for everything it reaches; the NDK is the one artifact it cannot, acquired by ~60 lines we own. Deleting an honest admission and replacing it with a compliance claim is precisely what F32 forbids.
7. **Scope the quarantine grep** to `crates/ ui/src ci/ pixi.toml` — corpus-wide it reddens on day one against `mobile-design.md:584`, and every existing guard in `ci/checks.sh` is narrowly scoped by design.
8. **`.cargo/config.toml` is an explicit committed output** (it does not exist today), since that is where `rustflags` and the linker actually live and what the drift check assumes.

**Correction the corpus needs regardless, and none of the critique lenses had it:** the claim that the NDK floor *"cannot be independently pinned because Tauri hard-codes it"* is **partly false**. Tauri hard-codes `ndkVersion`/`compileSdk`/`buildToolsVersion` in the `cargo-mobile2` **template**, whose output is a generated `gen/android/build.gradle.kts` that lands in our tree and is ours to commit and edit. The coupling is at `tauri android init` time. The honest statement is *pinnable; the cost is re-fighting the template on every Tauri upgrade.*

**Owned cost:** ~60 lines of shell owned forever; the expensive gate is opt-in and will rot (mitigated only by the two cheap always-on checks); tier 2 is archived, never pinned, and an AGP bump can break packaging with no test; a permanent tax that the Rust core stays `--target`-clean, so every future C dependency justifies itself against 2-ABI cross-compilation (today one unit: `crates/fm-core/Cargo.toml:18`); the owner accepts Google's SDK Terms once, by hand, never via a scripted `yes |`.

**Reversal condition:** conda-forge ships a real `android-ndk` package → the fourth lockfile collapses into `pixi.lock`. Tier 1 stops being SDK-free (JNI codegen needs `android.jar` to compile Rust) → the `libfm.so` boundary no longer holds and the ruling is re-derived. Play becomes required → the `targetSdk` floor returns. A second maintainer arrives → `MASTERPLAN.md:225`'s Nix hatch goes live on its own stated terms, and `android/toolchain.lock` is exactly the input an `androidenv` derivation wants, so nothing here is wasted.

---

### RULING 4 — RESEQUENCE — **REVISED under attack (3/3)**

**Surviving:** strike spikes (ii) and (iii); **M0 is redefined as "the phone paints and is a complete no-git notebook"**; the **MVP cut line moves from end-of-M4 to end-of-M0-plus-M2's-non-git-half**; M1/M3/M4/M5 are **parked on a named upstream event**, not marked BLOCKED; M8 stays parked; the first actionable step is a zero-code observation on real hardware.

The M0 redefinition rests on the strongest unexploited asset in the repo, verified verbatim this session at `git.rs:53-58`: *"**Git is optional, and this is the function that says so out loud.** … Someone with no git has a complete, working, single-PC notebook. What they do not have is **history**."* Because `decisions.md:155` **defended** C12 rather than reversing it, an Android build with no git backend is already a coherent product at zero code cost. This is not Path A — Path A is rejected because it creates a **dependency on a second machine** (`decisions.md:81-83`); a no-git phone depends on nothing. The plan collapsed *degraded-and-honest* with *dependent-on-a-satellite* and rejected both.

**What the skeptics broke, and I accept:**

1. **The promotion of ruling 2B to *the* transport is withdrawn.** Two skeptics refuted it on the same platform fact, with Google's own guidance verbatim: *"Some applications use localhost network ports for handling sensitive IPC. **Don't use this approach, because these interfaces are accessible by other applications on the device.**"* `fm-serve` is unauthenticated by design and says so twice (`crates/fm-serve/src/main.rs:259`, `:553`), and **both guards pass by omission, with tests asserting it**: `main.rs:241-242` (*"A missing Host is HTTP/1.0 or a hand-rolled client"*), `main.rs:265-268` (*"No `Origin` at all means a non-browser client … there are no ambient credentials to abuse"*), tests at `main.rs:574` and `:598`. That reasoning is correct on desktop, where a local process already has your files, and **inverts** on Android, where the sandbox *is* the boundary. A no-Origin `POST` reaches `capture:192`, `set_property:200`, `update_body:208`, `delete:214`, `open_external:254`, `set_git_remote:291` — and `set_git_remote` plus the shipped sync loop is a complete exfiltration primitive using the user's own credentials. `GET /api/blob/` is unguarded entirely (the CSRF guard is POST-only). **Nobody in the chain had ever priced this**, and the ruling's headline argument — "zero lines changed" — is precisely the property that makes it a hole.
2. **The transport returns to undecided and joins the parked list**, per `decisions.md:178`. What is recorded instead: the 2A cost table (five coupling points, verified this session — `ipc.ts:24`, `:102`, `:109`, `:130` plus **`assetUrl` at `ipc.ts:116`**, an unconditional URL string no `invoke` shim can intercept, plus `App.svelte:411`, `:436`, `:496`; `mobile-design.md:131`'s *"three spots"* undercounts by ~60%), **and the symmetric 2B trap**. The leading candidate to evaluate when the transport is decided is **2C — in-process interception via `register_uri_scheme_protocol`**, which on Android serves an `http://<scheme>.localhost/` origin, so relative URLs resolve, `streamsBlobs` stays correct, `assetUrl` is untouched, and there is no socket. Its named residual: `blob.rs:127-141` streams 64 KiB chunks into a socket, and whether a custom-protocol responder can stream incrementally rather than buffering a range is **UNVERIFIED**.
3. **Step 1's frontmatter fix is unbundled** — see Ruling 2. What remains in the sequence: extract `merge_texts`, fix the PID-only temp naming, land the differential harness, add the characterization test.
4. **The differential harness lands *before* any behaviour change, not alongside it.**

**Owned cost:** D14's headline promise — a phone that clones and pushes — is deferred visibly, and that must be written into `decisions.md` as a stage with a named re-entry point. Deferring it silently would be the exact self-exemption the 2026-07-18 review diagnosed.

**Reversal condition:** gitoxide ships `send-pack`/`receive-pack` (read `crate-status.md`, **never** the `gix::push` module name) → M1/M3/M4/M5 un-park and the original destination is restored unchanged. A WebView blanks on a real phone → M0's kill criterion fires and the transport question is decided from the shell up.

---

## 4. THE CORRECTED SEQUENCE

### STEP 0 — TODAY. ONE HOUR. ZERO LINES OF CODE. This is the single first actionable step.

```
pixi run serve                    # builds the PROD bundle: real dispatch, real blob.rs, real Range
adb reverse tcp:8765 tcp:8765     # device localhost → host localhost; no listener on the phone
# phone browser: http://127.0.0.1:8765
```

The device's browser sends `Host: 127.0.0.1:8765` and `Origin: http://127.0.0.1:8765`, so **every guard at `main.rs:242-257` and `:268-279` passes by construction — zero lines changed, and no loopback exposure**, because the server runs on the developer's host. Because `pixi run serve` runs `pnpm -C ui build`, `import.meta.env.PROD` is true: this exercises real `dispatch` and real `blob.rs`, **not `ui/src/lib/mock.ts`**.

Observe and record into `docs/context/sessions/`: single-column workspace (`ui/src/App.svelte:1198`), 2.75rem coarse-pointer targets (`:1217`), Board scroll-snap vs finger drag (`ui/src/lib/Board.svelte:196`), the tap→move menu (`ui/src/renderers/Card.svelte:114`, `:126`), an Excalidraw canvas under a finger, an inline image, and a `<video>` seeking mid-file — the only thing that exercises `blob.rs`'s `Range` path on real hardware.

**Why this is first:** `outstanding.md` §1.1 calls the volume of unobserved UI *"the largest single risk in the project"*, and `known-issues.md:63-66` documents Chrome touch emulation as unable to reproduce Android's `dragstart` suppression — *"emulation can hide the very bug the menu"* exists to fix. This is the only gate that can retire that item, and it needs no NDK, no backend, no phone build. `adb` is outside pixi (not on PATH, not on conda-forge) — acceptable because it enters neither the build graph nor the artifact, and F31 requires saying so rather than letting it pass as hermetic.

### STEP 1 — `crates/fm-core/src/merge.rs`, desktop-only, no new deps

- Extract `merge_texts(base: &str, ours: &str, theirs: &str, marker_size: usize) -> Result<(String, Merged), StoreError>`. Keep `merge_files` (`merge.rs:58`) as the thin driver-ABI wrapper — it is git's `%O %A %B` contract and `crates/fm-cli/src/main.rs:110` is its only caller. **Required under every resolution**, including "wait" and including Path A.
- Fix the PID-only temp naming at `merge.rs:192`. `crates/fm-serve/src/blob.rs:105` already uses PID + `AtomicU64`; `merge.rs` uses PID alone and is safe today only because its sole caller is a one-shot subprocess. `mobile-design.md:179` plans in-process calls into a thread-per-connection server, at which point two concurrent merges write the same three `/tmp` paths and produce a note whose body was merged from another note's content — clean exit, silent.
- **No semantic change.** Add the characterization test (Ruling 2).

### STEP 2 — the differential harness, green against **today's** engine first

Property test over randomized `(base, ours, theirs)` triples — including near-adjacent hunks, CRLF bodies, marker sizes 1 and 255, and markers from a prior bad merge — asserting **verdict identity** and the parse invariant. Run it against the existing subprocess implementation before touching anything: that proves the harness, not the engine. Re-home it in `decisions.md` as a standing precondition on any change to the body engine, independent of backend. **Risk #2 — *"wrong markers = silent data loss"* (`mobile-design.md:594-595`) — currently has no gate at all, because its only mitigation was parented to refuted ruling 3.**

### STEP 3 — the skipped-note surface (`ui/src/`)

A place, not a toast. This is the highest-value visible work available and it is the correct answer to the frontmatter problem under F28.

### STEP 4 — `crates/fm-core/src/git.rs`: write `clone`

On the subprocess backend, where it is testable today. Sequence `ensure_repo` → `set_identity` (`git.rs:259`) before the first commit, which finally makes ruling 5 real rather than theoretical. New arm in `dispatch.rs` beside `set_git_remote` (`:291`), plus a "clone a vault" affordance in the desktop vault picker — so the step ships visible UI, not only Rust. **Prerequisite on every backend.** Test: offline clone of a `file://` remote, asserting the first commit's author is not `PLACEHOLDER_EMAIL`.

### STEP 5 — harden `push_squashed`'s rollback (`git.rs:656-668`)

Make it independent of the client's own success verdict — verify the remote ref actually moved via a separate `ls-remote` before treating a push as landed. Pays under every option and is a precondition for any future push client.

### STEP 6 — the record (§5 below)

### STEP 7 — `gix-merge` as an **experiment**, feature-gated, mobile-only

Add behind the same non-default `native` feature. Run against the harness. Publish the divergence list **before deciding anything**. Treat `Resolution::CompleteWithAutoResolvedConflict` as an automatic **stop** until someone demonstrates what content it discards — it is resolution-by-fiat baked into the engine, and neither mapping is safe (`Conflicted` tells the user to resolve a file with no markers, which is `decisions.md:590-596`'s documented silent-deletion failure; `Clean` publishes the fiat). Decide `CDLA-Permissive-2.0` in `deny.toml` up front, deliberately.

### STEP 8 — toolchain, per Ruling 3

`[feature.android]` + two environments; `android/toolchain.lock`; the linking probe with an ELF-machine assertion; the `CC` provenance guard; the android arm in `vaults.rs:239`; the scoped quarantine grep; `.cargo/config.toml` committed.

### STEP 9 — M0

Only after step 8 is green, and only after the transport is decided.

**Not in this sequence, deliberately:** M8. Replacing mature `pdftotext` (`crates/fm-core/src/ingest.rs:101`) and `vipsthumbnail` (`ingest.rs:147`) with niche crates is structurally the `diffy` move C8 rejected. The divergence hazard it would close is real — identical PDF bytes yield extracted text on desktop and an empty body on a phone, both committed, both hitting the text merge — but the minimal correct fix is two lines of policy: **declare the extractors as capabilities on the heartbeat** (today `ingest.rs:101`'s `.ok()?` cannot distinguish a missing binary from a text-free PDF, which C11/F33 forbid), and **rule that a device without an extractor writes no body rather than an empty one** — *an empty body merges cleanly into a lie.*

---

## 5. PLAN HYGIENE

**The rule:** refuted **arguments** stay verbatim and marked — they teach. Refuted **instructions** are deleted — they recruit. When a line is both, keep the reasoning and strike the imperative clause.

**Do not cut `mobile-design.md:536-574` or `:590-606`.** The proposal to re-home the sequence into `plan.md` is withdrawn: `plan.md:393-394` says the opposite of `mobile-design.md:5` (verified both this session — *"Full design, code audit, and staged sequence (spikes → M0–M8) in mobile-design.md. This entry carries only the sequence-defining rulings"*), and the house precedent is the receipts file carrying the sequence — `collaboration-design.md:304` is `## Sequencing (do not reorder)`. Repair in place instead, and **fix the charter contradiction explicitly** — it is a genuine defect, and 3 of 4 sources point the same way.

### `docs/context/mobile-design.md`

| Line | Action |
|---|---|
| `:5` | Amend to match `plan.md:393` and the Track C precedent: this file carries the why, the why-not, the code, **and the staged sequence**. |
| `:8-11` | Rewrite. Name rulings **by subject, not number** — the two documents' numbering schemes are incompatible and always were. *"The `git2` backend and the `git2::merge_file` body engine were rejected; **the transport ruling is untouched, not rejected**."* State **four** blockers, not one: toolchain, git backend, **body-merge engine**, transport. |
| `:392`, `:395-398` | **Highest blast radius.** Strike `+3`. Rewrite as a requirement, not an achievement: *the app-owned merge is what would make a dumb transport safe; it does not exist — `merge_files` shells `git merge-file` (`merge.rs:204`, `:233`) and takes driver-shaped path inputs, so Path A's safety case is contingent, not established.* |
| `:131` | Correct "three spots" → **eight transport-coupling points, five in `ipc.ts`** (`:24`, `:102`, `:109`, `:116`, `:130`; `App.svelte:411`, `:436`, `:496`), and name the two that no shim can intercept. Anchors have drifted; fix them. |
| `:139`, `:225`, `:539`, `:541-542`, `:548`, `:551`, `:561` | Delete the refuted **mechanisms** (`git2::merge_file`, `git2::Cred`, `git2` read/`commit_all`/`revwalk`). Mark the surrounding **arguments** and keep them. |
| `:545` (M0) | Delete both dead lines — `Reindex::Incremental` (rejected, `plan.md:462-467`; `file.rs:106` is still `Reindex::Full`) and *"libgit2 cross-compiled"* (M0 is explicitly no-git). Keep the kill criterion. |
| `:548` (M1) | **M1 is new code on any backend** — `git.rs` has no `clone`. Do not frame it as a port. |
| `:555` (M4) | **Blocked three times:** backend, engine, and materialization of the three sides. |
| `:551-553` | **Keep** — the Android-PDF/WebView why-not is unrefuted and is exactly this file's charter. |
| `:563` (M7) | Ordering premise refuted — `decisions.md:194-196`. |
| `:565` (M8) | Keep, marked as blocked by nothing and deliberately not re-sequenced (see §4). |
| `:576-588` | **Keep — best paragraph in the corpus.** Sharpen: the gap is **NDK + SDK only**; *"pin the whole matrix"* is unachievable as written and the reason (Tauri template constants) is a maintenance cost, not an impossibility; **narrow the F31 admission in place, do not delete it**; Play deferral is a **ruling**, not a parenthesis. |
| `:594-595` (risk 2) | Re-anchor the differential test off dead ruling 3; state that until an engine is chosen, **M4 has no fidelity gate**. |
| `:596` (risk 3) | Rewrite: resolved **differently** — one command *library*, not one command *door* (`decisions.md:147`). |
| `:604` (risk 7) | Shipped (`scene.rs`, wired at `merge.rs:183`), and **there is no `.excalidraw` file** — `*.md merge=fm` routes boards in. |
| `:605-606` (risk 8) | **Keep, separate from risk 4**, plus one line: its foreground-service mitigation is the documented trigger for risk 4 (Tauri #15671, blank Android WebView on current versions), and its *"don't commit attachments"* clause is **superseded for whiteboard-embedded blobs** by `decisions.md:187-192`, open for everything else. |
| `:592-593` (risk 1) | Its mitigation *"pin the matrix in CI"* has no executor — all four workflows are `workflow_dispatch`-only. Replace with the control that does run. |
| `:64` | The self-contradicting parenthetical reads as though the fork closed. It did not — only `commands::asset_note` is shared (`crates/fm-cli/src/main.rs:144`). |

### `docs/context/plan.md`

- `:409`, `:425`: **BLOCKED → REJECTED.** Blocked invites a reader to wait for a foundation that is not coming back.
- `:406-407`: delete — false `fm-cli` debt, closed the other way at `decisions.md:130-149`.
- `:24-25`: Ruling B is **decided** (`decisions.md:187-205`), not *"blocked on owner decisions."*
- `:502`: → **NDK + SDK only**.
- `:393-394`: leave as-is once `mobile-design.md:5` is amended to agree with it.
- Track M gains **three lines and no re-planning**: M0 and M2's non-git half need no backend (cite `git.rs:53-58`); M1 is new code on any backend; M4 is blocked three times.

### `docs/context/decisions.md`

- `:153-154`: the numbering error propagated into the **binding** file. Fix by subject.
- `:176-181`: **keep the deferral, fix the menu.** Option 1 struck on dated Android facts (the `execve` restriction, the `nativeLibraryDir`/`.so` naming shape, git's multi-binary `libexec/git-core` + curl + TLS form, strictly more C cross-compile than the libgit2 already rejected) **plus** the packaging obligation — **do not rewrite `deny.toml`'s linked-vs-invoked ratio**, which is correct and whose expansion would foreclose a future `.deb`/AppImage shipping `pdftotext` (`MASTERPLAN.md:262` already has that answer). Option 2 **confirmed closed**, assume-never-ships. Option 3 qualified as demo-only in **both** places (`:81-83` and `:181` are ~100 lines apart and only the second qualifies itself). **Add option 4** — a narrow push-only layer over `gix-transport`/`gix-pack` — marked **spike-level and unevaluated**, with the C8 objection stated alongside it. **Delete any completeness stamp**: "the option set as of 2026-07-19," never "evaluated."
- Add: *we will not **build** a backend for a platform that does not exist; we have **evaluated** what exists.*
- On C12: **narrow, do not suspend.** Only one skeptic attacked this clause, and the attack is right on the mechanism: what D14 makes constitutive on mobile is **sync**, not git. A phone without a git backend is a C12-compliant degraded notebook that reports `git: false` and hides the collaboration surfaces — `git.rs:53-58` and `mobile-design.md:404-406` already specify exactly that. Record *that*, not a suspension.
- Add: **the body-merge engine is a decision distinct from the backend**, and the differential test is a standing precondition on any change to `merge_files`.
- `:580-586`: the invariant is **asserted, not held**. Mark it, cite `file.rs:417-420`, and point at Ruling 2.

### `docs/context/known-issues.md`

A dated **"External facts"** section — not a new file; the Traps section at `:167` is the right container. Verified this session: the file contains **no `gix`/`gitoxide`/NDK trap at all** (its only two Android matches, `:47` and `:66`, are about relaunch cost and touch emulation). Seed it with, each carrying a re-verify command:

1. `gix` push unimplemented — verify via gitoxide's `crate-status.md`, **never** the `gix::push` module name, which is the `push.default` config enum and will fool the next checker.
2. conda-forge ships four `rust-std-*-linux-android` and **no** NDK/SDK.
3. `bundled` rusqlite (`crates/fm-core/Cargo.toml:18`) needs an NDK sysroot regardless of any git decision.
4. **Inside a pixi env, a misconfigured Android toolchain produces green builds** — `c-compiler`'s activation sets `CC` to the host compiler, so `cargo check -p fm-core --target aarch64-linux-android` exits 0 with no NDK and emits an x86-64 `sqlite3.o`.
5. **Android loopback is not sandboxed.** Any app with `INTERNET` reaches a localhost listener. `fm-serve`'s Host/Origin guards are a browser threat model (`main.rs:241-242`, `:265-268`) and do not apply. **Never ship `fm-serve` as a TCP listener on a phone.**
6. Tauri couples the NDK/SDK floor via `cargo-mobile2` template constants — pinnable in our tree, at the cost of re-fighting the template on each upgrade.
7. Play's 16 KB page-size requirement, and the fact that it is a **device** property, not only a submission gate.

Also fix the `fm-cli` trap's false expiry — *"until it lands"* → permanently, by decision.

### `docs/context/outstanding.md`

Split §2.3: **M0 and M8 blocked on toolchain only** (M8: on nothing); M1/M3/M5 on toolchain + backend; M4 on toolchain + backend + engine. Add: write a `clone` primitive; correct `crates/fm-cli/src/main.rs:7-8`, which still asserts in **source** the migration debt `decisions.md:147` retired — under F32 a cold reader who checks the code finds it reasserted.

### `docs/context/README.md`

`:28` stands as-is once the charter is fixed in `mobile-design.md:5` rather than by cutting the sequence.

### Outside `docs/context/`

`ui/vite.config.ts:14` references a `tauri.conf.json` that does not exist anywhere in the tree (verified). Pre-pivot fossil; delete or amend. `ui/src/lib/render.ts:172` creates an `<iframe class="asset-pdf">` with **no `sandbox` attribute** while `render.ts:163`'s comment claims *"only a sniffed `application/pdf` reaches a sandboxed iframe"* — and PDF is on the inline allowlist (`blob.rs:164`) while blobs arrive from collaborators. File it independently of Track M.

---

## 6. HOW TO STAY ON TRACK — TRIPWIRES

`docs/book.toml:5` is `src = "src"`, so `docs/context/` has **zero** CI coverage, and all four GitHub workflows are `workflow_dispatch`-only by standing order. Local `pixi run ci` is the only gate. Design accordingly: mechanical where possible, and where not, a checklist that survives a cold read.

**Mechanical — add to `ci/checks.sh`, which states its own charter at `:2-4` (*"not style checks — they defend the invariants"*). Each needs the owner to widen that charter in writing, once:**

1. **Quarantine grep** — `ANDROID_HOME|NDK_HOME|sdkmanager|gradlew` must not appear under `crates/ ui/src ci/ pixi.toml`. Scoped, never corpus-wide. **Test it by breaking it**: add the string under `crates/`, confirm red, remove, confirm green.
2. **`native`-module purity** — no `Command::new` under any `git/native*` module. The third instance of the pattern `fm-query` and the renderers already established.
3. **Toolchain drift** — if `android/toolchain.lock` exists, its version string must match `.cargo/config.toml`.
4. **CC provenance** — `android/check.sh` refuses to run unless `CC_aarch64_linux_android` resolves inside `.android/ndk/`. This is the only thing that makes the pixi-`CC` trap loud.
5. **ELF machine assertion** — the Android probe asserts `readelf -h` reports AArch64 on the emitted `sqlite3.o`, and the first linked `.so` asserts `LOAD` alignment `0x4000`. **Never assert an exit code where an artifact fact is available.**

**Tests that are tripwires:**

6. **The differential harness** must be in `pixi run ci` and must fail on **any** unpinned divergence. Per-divergence written waivers are the mechanism by which this becomes the wrong engine, one reasonable paragraph at a time.
7. **The frontmatter characterization test** — divergent `status` → `Conflicted`, note appears in `skipped`, notification fires. It locks the current designed behaviour; if someone changes it, this test tells them they are reversing a ruling.
8. **The byte round-trip** (`MASTERPLAN.md:327`) re-run over **conflicted** output with CRLF bodies, since that is where the one upstream line-ending deviation lives.

**Doc-staleness — the cold-read test, quarterly and after any ruling, as a literal checklist at the bottom of `docs/context/README.md` so it is re-run rather than re-derived.** Fresh session, README's protocol, seven questions:

1. Was the Tauri transport ruling reversed? *(No — untouched. Corpus says yes today, in the binding file.)*
2. Is the merge in-process? *(No — `merge.rs:204`, `:233`. Corpus says yes today.)*
3. Does `fm-cli` owe a migration onto `dispatch`? *(No — `decisions.md:147`. `plan.md:406-407` and `crates/fm-cli/src/main.rs:7-8` say yes today.)*
4. Does M1 port an existing primitive? *(No — `git.rs` has no `clone`. Corpus is silent today.)*
5. What is executable today with no phone and no NDK? *(Steps 0–5 of §4. Corpus says "blocked" today.)*
6. What does the corpus say about viewing a PDF on Android, and about foreground services? *(Guards against the class of fact most likely to be lost in an edit pass.)*
7. Is `fm-serve` safe to run on a phone? *(No. Nothing in the corpus says so today.)*

**Six of seven answer wrong right now.** That number is the measurement that justifies the work in §5, and re-measuring it is the gate afterwards.

**Standing rules, because they are cheap and they are what actually failed:**

9. **Cite rulings by subject, never by number, across document boundaries.** The two numbering schemes are incompatible and the hybrid at `mobile-design.md:10` propagated into `decisions.md:153`.
10. **Any external claim gets a date and a re-verify command, or it does not go in.** The corpus carried *"once `gix` push ships"* as a schedule for a four-year-stalled upstream issue.
11. **A mitigation that names a mechanism must name an executor that exists.** Risk #1's mitigation requires a CI that is switched off for cost reasons orthogonal to mobile; risk #2's was parented to a refuted ruling. Both read as managed and are not.
12. **The rule the corpus keeps proving it needs:** the 2026-07-18 review's own finding was that *"the draft applied the project's rules to others and exempted its own proposals."* It then did the same. So did the ruling set attacked above — three of four rulings smuggled in a decision the owner had already taken the other way, under a mandate that was not "decide this." **Before any ruling ships, grep `decisions.md` for the thing it is about, and quote what you find.**

---

# Appendix — completeness critic (what the review itself missed)

All verified. Here is the completeness audit.

---

# GAPS THE REVIEW MISSED — ranked

**Verification basis:** tree at `/home/baljinder/formicaria`, HEAD `a7b34ce`, clean. Every anchor below re-checked directly.

---

## G1 — CRITICAL: the plan's own declared *next* item is absent from the entire review, and it structurally collides with M0

**`plan.md:35`, verbatim: "Next: Track V4 — adoption"** — and `outstanding.md:38`: *"**The largest unbuilt item in the plan**, and the natural next feature now V1–V3 are in."*

Across four recon documents, five critique lenses, four rulings and twelve attacks, **Track V4 is never mentioned once.** The `resequence` ruling produced a "corrected sequence" (S1–S4, M0, M2′, cut line) without ever checking it against the item the sequencing document itself names as next.

This is not a scheduling oversight — V4 **changes the shape of the thing M0 opens**. `plan.md:361-376`:

> *"`path_for(id) = notes.join("{id}.md")` — the filename *is* the id… So **the index records each note's path and `put` writes back where it found it**… which is also what makes a recursive walk safe (`reindex`'s `read_dir` is flat today, so nested Markdown is invisible)."*

Verified: `crates/fm-core/src/file.rs:166` is `fn path_for(&self, id: Id) -> PathBuf`. V4 changes `FileStore`'s path model, the index schema, and `reindex`'s walk. M0 is defined as "open the store, list notes." **If V4 lands first, M0 opens a different store; if M0 lands first, V4 re-does it on two platforms.** Nobody sequenced them against each other, in either direction.

V4 also silently subsumes an M-band item: `plan.md:376` requires *"`FileStore::skipped()` must reach the GUI first"* — which the corruption lens spent a finding on (the skipped-note path) without noticing V4 already owns it.

**Closes it:** one line in `plan.md` Track M and Track V stating the ordering constraint between V4's path-model change and M0's store-open, and a ruling on which goes first. Given V4 is desktop-executable today and M0 is toolchain-blocked, the answer is probably V4 — which would make the resequence ruling's "first actionable step" wrong.

---

## G2 — CRITICAL: iOS does not exist anywhere in the corpus, and "Android-first" is a promise nobody priced

`grep -cin "ios\b" docs/context/mobile-design.md` → **0**. No `iPhone`, `iPad`, `App Store`, or `Apple` anywhere in `docs/context/`. The only hits in the repo are `macOS` platform strings and WebKit**GTK** (Linux).

`mobile-design.md:576` is titled **"Toolchain (Android-first)"**. *First* is a sequencing word: it implies a second. That second platform is never named, never deferred, never rejected. This matters concretely, not rhetorically:

- **It changes the transport ruling.** iOS has no `fork`/`exec` at all and no user-servicable browser fallback — ruling 2B (`fm-serve` on device, which the `resequence` ruling promoted to *the* transport) has a much weaker story on iOS, and the "open your own Safari" retreat is materially different from Chrome-on-Android.
- **It changes the git-backend ruling.** Every option in `decisions.md:179-181` was scored on Android constraints only. "Ship a git binary" is refuted on Android's `execve` rule; on iOS it is refuted harder and for different reasons. The `git-backend` ruling's licence argument (APK → we become the distributor) applies to an IPA too, but nobody said so.
- **It changes the toolchain ruling.** The whole `toolchain` ruling is an NDK/SDK pinning design. Xcode cannot be pinned by pixi *or* by a SHA-256'd zip, and requires macOS hardware. The `toolchain` ruling's "F31 is not violated, it was mis-stated" reframe does not survive contact with a platform whose toolchain is legally undistributable.

**Closes it:** one ruling in `decisions.md` — *"iOS is out of scope for Track M; Android-first means Android-only until stated otherwise"* — or, if iOS is intended, a named row in the risk register with the Xcode hermeticity cost. Either is cheap. The current silence lets a future reader assume parity work is a port when it is a second program.

---

## G3 — HIGH: nobody asked what a person actually *does* with the phone, and the corpus has no answer

Grep of `mobile-design.md` for `share`/`intent`/`widget`/`notification`/`camera`/`photo`/`voice`/`keyboard`/`paste` returns **zero** workflow hits (the `share` matches are all "shared core"/"shared vault").

The plan is 606 lines of transport, git, merge, credentials and toolchain. It contains **no statement of what the phone is for.** Meanwhile the project's own product rule is `plan.md:70-73`: *"The UI is the product… A phase that ships only Rust shipped nothing."* Three lenses invoked that rule to score the *shipped band* — none applied it to the *plan itself*.

The omission is load-bearing because the dominant mobile notes workflow is **capture** — share-sheet from a browser, a photo, a voice note, a lock-screen widget — and every one of those is an Android-integration surface that `Host` (one method, `open_external`) does not cover. `dispatch.rs:192` has a `capture` arm, so the *command* exists; the *gesture* that reaches it from outside the app does not, and is not in any phase. M0–M8 deliver a phone that can only be used by opening the app and typing.

There is a related, concrete consequence nobody traced: `ingest` (`dispatch.rs:386`) on a phone means photos, and `ingest.rs:147`'s `vipsthumbnail` is absent on Android — so the single most natural mobile capture gesture lands on the one subprocess dependency with no capability declaration (see G5).

**Closes it:** a short "what the phone is for" section in `mobile-design.md` naming the two or three gestures that justify the port (capture-from-share-sheet, read/search offline, resolve a conflict), and a check that each has a phase. If capture-from-share-sheet is in, it is an M-phase item and a `Host` trait method, and neither exists.

---

## G4 — HIGH: auto-commit is a browser `setTimeout` that dies with the tab — accepted on desktop, *dominant* on a phone, never revisited

`ui/src/App.svelte:645`: `commitTimer = setTimeout(() => { … }, 5000)`. `outstanding.md:65-69` files this under **"Known and accepted — do not 'fix' without deciding"**:

> *"**Auto-commit dies with the tab.** A browser `setTimeout`… so 'edit, then close' can skip that commit. Files are never at risk (atomic temp+rename); our commits lag. Fixing it means `beforeunload`, which is unreliable by design."*

That reasoning is a *desktop* judgement: on a laptop, closing the tab is deliberate and rare. **On Android, backgrounding is how you leave every app, and the OS freezes timers and kills the process.** So the accepted edge case becomes the normal path, and "our commits lag" becomes "the phone's edits are never recorded" — which on a device with no terminal and no Vim is indistinguishable from data loss to the user.

I verified the mitigation does not exist: `App.svelte:503-507`'s `visibilitychange` handler calls `refresh()` only — it does **not** flush the pending commit. The efficiency work (15 s beat, 90 s watchdog, `POST /api/alive`) all went to *reading*; nothing went to *durably recording* on suspend.

The review discussed mobile lifecycle three times (the `Reindex::Full` cold start, the watchdog, the foreground service) and never connected it to the write path.

**Closes it:** a decision entry re-scoring this known-and-accepted item for mobile, and one line in M2′'s scope — on `visibilitychange → hidden`, flush the commit before the timer would have fired. That is desktop-testable today and improves desktop too.

---

## G5 — HIGH: the "never put the token in the remote URL" rule has no enforcement anywhere, and the URL round-trips to the UI

`mobile-design.md:219-220` states the rule: *"**never** in the remote URL (it leaks into `.git/config`)."* I checked what enforces it. Nothing does.

`crates/fm-core/src/git.rs:406-432` — `set_remote` validates **only that the URL is non-empty** and that an identity exists. No scheme check, no credential-in-URL check. `dispatch.rs:291-298` passes `s("url")` straight through from the wire.

Worse, it comes back out: `dispatch.rs:685` populates `VaultStatus.remote` from `git::remote()`, and `ui/src/lib/BackupPanel.svelte:85` does `remoteDrafts[v.name] ??= v.remote ?? ''` — the full URL is loaded into an editable input and rendered via `shortDest`. So a token pasted into a remote URL is stored in plaintext in `.git/config`, returned on every `backup_status`, and displayed.

On desktop this is latent (ambient credential-helper means nobody needs to do it). **On a phone, `https://user:TOKEN@host/repo.git` is the path of least resistance** — it is what every "paste your token" tutorial does, it needs no Keystore, no `CredentialSource`, no OAuth. Ruling 4's whole Keystore design exists to prevent exactly this, and the codebase has no guard that would stop a user or a future implementer from short-circuiting it.

The `resequence` attacks found the *transport* exposure of `set_git_remote` (exfiltration via an attacker-set remote). Nobody looked at the reverse direction: **credentials flowing out through the same field.**

**Closes it:** reject a URL containing userinfo in `set_remote` with a message naming the reason, and add the negative test. Desktop-executable today, ~10 lines, and it is a precondition for ruling 4 meaning anything.

---

## G6 — HIGH: no agent asked how any M-phase would be *tested*, and the answer is "it cannot be"

The corpus has a genuine testing culture — 5 integration tests in `crates/fm-cli/tests/merge.rs` driving real `git merge`, 13 in `blob.rs`, 8 in `scene.rs`, 14 UI suites. Every ruling in this review specifies verification. **None of them asks what the mobile CI story is**, and the pieces are all in the corpus already:

- All four workflows are `workflow_dispatch`-only by standing order (maintenance lens verified this).
- `pixi.toml`'s `ci` task has no cross-compile step and no android platform.
- `known-issues.md:63-68` rules Chrome touch emulation *"actively misleading"* for the exact bug the touch menu fixes.
- `outstanding.md:18-28` says the unobserved UI is *"the largest single risk in the project."*

Compose those and the conclusion is unavoidable: **every M-phase acceptance test requires a physical device, in the owner's hand, with no automation possible.** M4's stated acceptance test is a *two-device* demo — so it requires a phone *and* a desktop *and* a remote, manually, every time it regresses. That is not a testing gap; it is a **structural ceiling on how much mobile work can be maintained**, and it belongs in the risk register at or near #1, next to toolchain hermeticity, because it has the same decade-scale shape.

**Closes it:** a "how this gets verified" row per M-phase, and an explicit ruling on what is accepted as untestable. The honest version probably shrinks the mobile scope — which is useful information, not a defeat.

---

## G7 — MEDIUM-HIGH: `collaboration-design.md` (400 lines) was never read, and it is the sibling document mobile-design.md names as its template

`mobile-design.md:3-4` declares its own model: *"the same way [`collaboration-design.md`](./collaboration-design.md) sits behind Track C."* Only one agent (attack 1 on plan-hygiene) opened it — and found that `collaboration-design.md:304` is `## Sequencing (do not reorder)`, carrying a full staged Phase 0–3 sequence **in the receipts file**.

That single fact refuted the `plan-hygiene` ruling's second pillar, which had cut `mobile-design.md`'s sequence on the theory that sequences don't belong in receipts files. The precedent says they do. **A 400-line sibling document, named in the target file's second sentence, was read by 1 of ~20 agents** — and when it was read it overturned a ruling. Whatever else is in it (the C8 dependency text at `:57-58`, the F29 moat argument at `:96`) was never scored against Track M.

**Closes it:** read it. Specifically, check Track C's Phase 0–3 sequencing conventions against Track M's, since the two tracks share the git substrate and Track C actually shipped.

---

## G8 — MEDIUM: two of the owner's stated principles were catalogued and then applied to nothing

The `principles` recon built a 37-item rubric. Two were never used by any lens, ruling or attack:

- **F30 — "Three pillars, one atom — resist the fourth thing"** (`plan.md:55-57`): *"Resist every urge to add a fourth thing. The moment scheduling gets its own store… the tool has three products to maintain."* Its own RULES-OUT clause reads *"a mobile-specific store, format, or subsystem; a second index; a 'mobile mode' data model."* This is the principle that should have been aimed at the `git-backend` ruling's proposal to **write and own a smart-HTTP `send-pack` client** — a fourth subsystem by any reading. Two attacks got there via C8 instead; F30 is the sharper instrument and nobody picked it up.
- **E27 — "Vaults are audiences; hiding is a view, not a permission"** (`decisions.md:563-567`). Never applied. It bears directly on the un-examined question of what happens when a phone holds several vaults with different audiences and one shared credential store — the "vault A on github/alice, vault B on gitlab/bob" case ruling 4 names as a headline goal.

**Closes it:** score the `send-pack` proposal against F30 explicitly before it is adopted, and apply E27 to the multi-vault credential model.

---

## G9 — MEDIUM: `known-issues.md`'s actual structure was never audited, only grepped

Every agent grepped `known-issues.md` for `gix|android|NDK` (correctly finding zero) and concluded "the trap file holds no mobile traps." Nobody read its four sections — `## Known gaps` (:17), `## Deferred` (:148), `## Historical` (:160), `## Traps for whoever works here next` (:167) — to ask which *existing* entries change meaning on a phone. At least one does: `known-issues.md:162` is a **Historical** entry retiring the WebKitGTK blank-window problem. Tauri #15671 (blank Android WebView, current versions) is the same failure class returning on a new platform — so a "Historical / no longer relevant" entry is about to become live, and the file has no mechanism for that transition.

**Closes it:** a pass over all four sections asking "does mobile change this?" per entry, rather than a keyword grep for mobile terms.

---

# FACTUAL CONTRADICTIONS BETWEEN AGENTS — resolved against the tree

| # | Contradiction | Resolution (verified) |
|---|---|---|
| 1 | **PROD gate count.** Divergence map §5.4: *"Eight PROD gates across `ui/src`"*. Architecture F6: *"seven code sites"*. Executability F8: *"7, plus `assetUrl`"* | **Architecture and executability are right; the map is wrong.** `grep -rn "import.meta.env.PROD" ui/src/` → 8 hits, of which **`ipc.ts:21` is a comment**. Code gates = `ipc.ts:24, 102, 109, 130` + `App.svelte:411, 436, 496` = **7**. `assetUrl` (`ipc.ts:116`) is a 5th `ipc.ts` coupling point with **no** PROD gate. So: 7 gates + 1 ungated URL constant = 8 coupling points, 5 in `ipc.ts`. The map's "8" is right by coincidence, wrong by composition. |
| 2 | **`ci/third-party.sh` anchor.** Maintenance F1 cites `:24`; git-backend ruling §0.3 says *"P3's anchor is right; P2 cited `:24`"* and uses `:25` | **`:25` is correct.** `grep -n "for pkg in"` → `25:    for pkg in fm-serve fm-cli; do`. The maintenance lens — which built its strongest finding (G-gate blindness to APK artifacts) on this line — cited it one off. The finding survives; the anchor doesn't. |
| 3 | **`Cargo.lock` crate count.** git-backend ruling: *"112 crates today"*. Attack 1 on git-backend: *"formicaria today: 109"* | **112 by `grep -c '^name = ' Cargo.lock`.** Attack 1's 109 is a different metric (almost certainly `cargo tree -e normal` dedup, excluding some workspace/dev entries). Not a factual conflict, but attack 1's headline *"off by 69 crates (+63%)"* is computed against the smaller base and should be restated against a stated methodology. Both agree the direction: the dependency growth is large and the ruling under-reported it. |
| 4 | **Test count in `crates/fm-cli/tests/merge.rs`.** Corruption lens lists 4 (`:63, :128, :215, :273`); resequence ruling says 5 (`:63, :128, :172, :215, :273`) | **5 is correct.** The corruption lens omitted `:172 pull_brings_their_work_home_and_says_so`. Its actual claim — *no divergent-frontmatter-field test exists* — is **confirmed**; none of the 5 covers it. |
| 5 | **"M4's second blocker is recorded nowhere."** Divergence map §3/§6 says so; principle-fidelity F7 says it *is* recorded in `mobile-design.md` | **Principle-fidelity is right, and so is the map's narrower version.** `mobile-design.md:284` verbatim: *"shells `git merge-file` at `184`/`213`, outside the `git(vault)` helper — ruling 3 replaces those."* So the fact is recorded — with **stale anchors** (now `merge.rs:204`/`:233`, verified) and a **dead resolution clause** (ruling 3 is refuted). The map's §3 headline *"recorded nowhere"* is overstated; its §5 phrasing (*"nothing in `plan.md`, `decisions.md` or `outstanding.md`"*) is accurate. This matters: the map presents it as one of three novel findings, inflating confidence in the other two. |
| 6 | **`dispatch` arm count.** code-reality says 30; architecture F6 flags a naive grep giving 31 | **30 confirmed.** `grep -c '^\s*"[a-z_]*" =>'` → 30. Architecture's correction is right (`dispatch.rs:788` is a string literal). |

**Surviving from the map's three headline "recorded nowhere" findings:** #1 (the `mobile-design.md:10` hybrid numbering error) — **confirmed**. #2 (`git.rs` has no `clone`) — **confirmed**, 13 `pub fn`, none is clone. #3 (M4 blocked twice) — **partially refuted**, see row 5.

---

# WHAT THE REVIEW GOT RIGHT AND SHOULD NOT BE RELITIGATED

So the gap list is read in proportion: the `git2` rejection reasoning, the `merge.rs:204`/`:233` finding, the `mobile-design.md:392` false-in-process-merge premise, the licence-gate blindness to non-Rust artifacts, the missing `clone` primitive, and the frontmatter-fence corruption reproduction are all **verified correct** and are genuinely valuable. The corruption lens's F1 in particular — reproduced empirically, contradicted by `file.rs:414-421` in the codebase's own words, untested in the one suite built to test it — is the single best finding in the corpus, and G4 above compounds rather than displaces it.

**The pattern across G1–G3:** the review audited the plan's *mechanisms* exhaustively and its *purpose* not at all. Nobody asked what the phone is for, whether iOS exists, or how this sequences against the work the plan itself calls next. Twenty agents went four levels deep on a git backend for a product whose mobile user story is unwritten.
