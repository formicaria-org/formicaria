# Shared iOS build setup, sourced by `ci/ios-smoke.sh` (rung 2/3) and `ci/ios-package.sh` (rung 5).
#
# **Every line here was paid for by a billed macOS job.** Rung 2 took four firings to go green and
# three of them died in this preamble — not in iOS, and not in our code: a Rosetta-mismatched CLI, a
# missing linker library, a toolchain search order. A second iOS script that re-typed these would be
# a second copy of hard-won knowledge, drifting from the first the moment either is fixed. So they
# live once, and both callers get the fixes.
#
# **Callers must define, before sourcing:** `root` (the repo root).
# **Callers must define, before calling anything here:** `say`, `fail`, and `OUT` (the artifact
# directory) — the same contract `ci/lib/simctl.sh` uses, for the same reason: an artifact path and
# a message prefix belong to the script that owns the run, not to a library shared by two of them.

# **tauri#15066 / actions/runner-images#13135.** Under Xcode 26 the default toolchain search order
# puts `MetalToolchain` ahead of `XcodeDefault`, and MetalToolchain ships no Swift back-deployment
# libraries — so any Swift-linking build dies with `ld: library not found for -lswiftCompatibility56`.
# The runner-images maintainers' own fix is this variable. Tauri's `xcodebuild` invocations have no
# way to inject it (`cargo-mobile2/src/apple/target.rs` passes no `-toolchain`), so it has to be in
# the environment before the CLI is called. Applied defensively: the issue is documented against the
# device SDK, and nothing found says the Simulator SDK is immune.
export TOOLCHAINS="${TOOLCHAINS:-com.apple.dt.toolchain.XcodeDefault}"

# Our verifying `rustup` shim first, exactly as `pixi.toml`'s `android-apk` and
# `ci/android-release.sh` do it. Tauri's mobile commands shell out to `rustup target add`, and this
# project has no rustup — targets are conda packages pinned in `pixi.lock`. The shim reports what is
# installed and refuses to invent anything; without it, `tauri ios build` either fails or finds a
# real rustup and installs an unpinned toolchain, which is what the pinning exists to prevent.
PATH="$root/ci/bin:$PATH"
export PATH

# `run_logged <name> <cmd...>` — live output *and* an honest exit status.
#
# **`cmd | tee log || fail` is a lie**: a pipeline reports the status of its last command, so the
# `||` would fire on `tee` failing and never on the build. `pipefail` is not in POSIX sh. So the
# status is written to a file inside the subshell and read back afterwards. Live output matters
# here — this is a long build whose whole value is being watchable while it runs.
run_logged() {
    name=$1; shift
    # `set +e` inside the subshell: with errexit on, a non-zero `$@` exits the subshell *before*
    # `echo $?` runs, and the real code is lost — the failure is still caught, but every build
    # failure would be reported as rc=1. Measured, not assumed.
    ( set +e; "$@"; echo $? > "$OUT/$name.rc" ) 2>&1 | tee "$OUT/$name.log"
    rc=$(cat "$OUT/$name.rc" 2>/dev/null || echo 1)
    [ "$rc" = 0 ] || fail "$name failed (rc=$rc) — $OUT/$name.log"
}

tauri_ios() { ( cd "$root/mobile" && pnpm exec tauri ios "$@" ); }

# **Is the Tauri CLI the right architecture?** Checked because the symptom is otherwise
# unrecognisable. `@tauri-apps/cli` is a thin JS wrapper over a per-platform native binary that pnpm
# selects as an optional dependency; if the wrong one is installed, `tauri` runs under Rosetta 2 and
# the *first* thing it does — shell out to `brew` for `xcodegen` — fails with **"Cannot install
# under Rosetta 2 in ARM default prefix (/opt/homebrew)"**. That names Rosetta and Homebrew and says
# nothing about pnpm, which is what it actually is. It cost rung 2 its first job on 2026-09-03; see
# the pins in `pixi.toml`'s `[dependencies]`. One `ls` is cheaper than reading that error again.
tauri_cli_arch_check() {
    _arch=$(uname -m)
    case "$_arch" in arm64) _want=darwin-arm64 ;; x86_64) _want=darwin-x64 ;; *) _want= ;; esac
    [ -n "$_want" ] || return 0
    _have=$(ls -d "$root"/mobile/node_modules/@tauri-apps/cli-darwin-* 2>/dev/null | sed 's|.*/cli-||' | tr '\n' ' ')
    case " $_have " in
        *" $_want "*) say "tauri CLI: $_want (matches $_arch)" ;;
        "  ")         say "tauri CLI: no darwin binary resolved yet — 'tauri ios init' will fetch one" ;;
        *)            fail "the tauri CLI installed is '$_have' but this machine is $_arch, so it would
  run under Rosetta 2 and 'brew install xcodegen' would refuse with a message about /opt/homebrew.
  This means two pixi environments resolved different pnpm versions — see the nodejs/pnpm pins in
  pixi.toml [dependencies], and run every pnpm step of this job in the same environment." ;;
    esac
}

# `ios_project_ready` — generate `gen/apple` if it is absent, then patch it. **Both callers need
# both halves, and the second half is not optional.**
#
# The generated project does not link zlib or iconv, which libgit2 needs and which rustc cannot
# bundle into a `staticlib` — rung 2's second firing died at the link step on exactly those twelve
# symbols. That is a consequence of the crate type, **not of the Simulator**, so a device build hits
# it identically. `gen/apple` is gitignored so a fresh checkout never carries the fix, and `tauri
# ios build` never re-runs XcodeGen, so `ios-inject-linker-libs.sh` both patches `project.yml` and
# regenerates. See its header for why no tidier mechanism works.
ios_project_ready() {
    if [ ! -d "$root/mobile/src-tauri/gen/apple" ]; then
        say "no gen/apple — running 'tauri ios init'"
        run_logged ios-init tauri_ios init --verbose
    fi
    # **Two patches, one regeneration.** The plist injection runs first with `FM_SKIP_XCODEGEN=1`
    # so it only edits `project.yml`; the linker-libs script then patches *and* regenerates, so
    # `xcodegen` runs exactly once with both edits in place. Reversing this order would silently
    # drop the usage descriptions from the generated Info.plist.
    run_logged inject-plist env FM_SKIP_XCODEGEN=1 sh "$root/ci/ios-inject-plist.sh"
    run_logged inject-linker-libs sh "$root/ci/ios-inject-linker-libs.sh"
}
