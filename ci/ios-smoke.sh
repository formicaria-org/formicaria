#!/bin/sh
# **Does the app boot on a Simulator, twice in a row, and say so?** — iOS rung 2.
#
# The analogue of `ci/android-smoke.sh`, and it inherits that file's central lesson: the owner's
# phone once opened to a gray screen on the first launch and worked on the second, so **every check
# here runs on TWO cold launches and requires both to pass.** A one-launch smoke test would have
# gone green through the whole outage.
#
# What it asserts, in order of how much it is worth:
#   C. two consecutive cold launches both report `vaults ready` — the store opened, on a real iOS
#   B. the screen is not blank            (screenshot standard deviation, via libvips)
#   A. the app is installed and launches  (necessary, and proves nothing about paint)
#
# **The diagnostic channel is stderr, and that is deliberate** (`decisions.md` 2026-09-03, *the iOS
# diagnostic channel is stderr, not `os_log`*). `simctl launch --console-pty` attaches a pty to the
# app's stdout/stderr, and `install_logger`'s iOS arm writes there with the same `formicaria` prefix
# Android's logcat tag uses — which is why both smoke tests grep for the same line. If the pty comes
# back empty the fallback is `xcrun simctl spawn <udid> log stream`, not a rewrite.
#
# **macOS only, and there is no Mac here.** This file is written blind, against the Tauri v2.11.4
# CLI source rather than against a run, so **every path and name it needs is discovered at runtime**
# — the scheme, the .app, the bundle id, the device. The one place that costs a line of explanation
# it gets one. It cannot be rehearsed: the first time it executes is inside a billed CI job.
#
# Usage (CI, or a Mac someone else owns):
#   sh ci/ios-smoke.sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
OUT="$root/target/ios-smoke/$(date +%Y%m%d-%H%M%S)"
# The blank-screen threshold, carried over from `android-smoke.sh` where it was measured: `vips
# deviate` is the standard deviation of the pixels, ~0 for a flat surface and tens for a painted app
# (102.9 on a real screenshot there). Every measured value goes to stats.txt so the number stays
# grounded in something re-readable rather than in a constant somebody guessed.
MIN_DEVIATION=10

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

say()  { echo "ios-smoke: $*"; }
# Only name the artifact directory once there is one. A precondition failure happens before
# `mkdir`, and pointing at an empty path is the kind of small lie that sends the next person
# looking for a log that was never written.
fail() {
    echo "FAIL: $*" >&2
    if [ -d "$OUT" ]; then echo "  artifacts: $OUT" >&2; fi
    exit 1
}

# ---------------------------------------------------------------------------
# 0) Preconditions. First, because everything below is Darwin-only verbs.
# ---------------------------------------------------------------------------
[ "$(uname -s)" = "Darwin" ] || fail "this needs macOS (xcrun, simctl, xcodebuild). There is no Mac
  in this project; the only one it has is a GitHub runner — see .github/workflows/ios.yml."
for t in xcrun xcodebuild plutil vips; do
    command -v "$t" >/dev/null 2>&1 || fail "$t is not on PATH (vips comes from pixi; run this as 'pixi run ios-smoke')"
done
mkdir -p "$OUT"

# **Disk, named up front.** The runner is 3 CPU / 7 GB RAM / 14 GB disk and this build compiles
# vendored OpenSSL, libgit2 and SQLite from source *and* an Xcode archive. Expect the first attempts
# to fail on disk rather than on code — so record it before and after rather than guessing later.
df -h / "$root" > "$OUT/disk-before.txt" 2>&1 || true

# ---------------------------------------------------------------------------
# 1) Generate the Xcode project if it is not there, then build for the Simulator.
#
# **Through `tauri ios build`, not a bare `xcodebuild`** — and that is a finding, not a preference.
# The generated "Build Rust Code" phase shells out to `tauri ios xcode-script`, which unconditionally
# calls `read_options()`: it reads a `<bundle-id>-server-addr` file out of the temp dir and opens a
# WebSocket to a server that only exists inside a *currently running* `tauri ios build`/`dev`
# process, and `panic!`s when the file is missing. A bare `xcodebuild` with no cooperating tauri
# process alive therefore cannot get through the script phase at all.
#   (tauri-cli v2.11.4: crates/tauri-cli/src/mobile/ios/xcode_script.rs, mobile/mod.rs write_options)
#
# `--target aarch64-sim` is one of exactly three accepted values (`aarch64`, `x86_64`,
# `aarch64-sim`), and it is the Simulator triple on an Apple-silicon runner.
#
# `--no-sign` is **required, not tidiness**: the CLI runs `xcodebuild archive` for the simulator
# target too, on the same code path as a device build, and only `--no-sign` injects
# `CODE_SIGNING_ALLOWED=NO`. A headless runner has no keychain, no certificate and no team.
#
# Features are left at their defaults on purpose. `mobile/src-tauri/build.rs` derives `agent_shell`
# from *the feature and Android*, so an iOS build is agent-free however it is invoked — the flag is
# not what makes that true, and this script does not depend on a flag nobody here can verify.
# ---------------------------------------------------------------------------
# `run_logged <name> <cmd...>` — live output *and* an honest exit status.
#
# **`cmd | tee log || fail` is a lie**: a pipeline reports the status of its last command, so the
# `||` would fire on `tee` failing and never on the build. `pipefail` is not in POSIX sh. So the
# status is written to a file inside the subshell and read back afterwards. Live output matters
# here — this is a 25-45 minute build whose whole value is being watchable while it runs.
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

if [ ! -d mobile/src-tauri/gen/apple ]; then
    say "no gen/apple — running 'tauri ios init'"
    run_logged ios-init tauri_ios init --verbose
fi

say "building for the Simulator (aarch64-sim, unsigned)…"
run_logged build tauri_ios build --target aarch64-sim --no-sign --verbose

# The CLI renames the .app out of the .xcarchive into `gen/apple/build/<arch>/`, where <arch> is
# `arm64-sim`. Globbed rather than hardcoded: the name comes from tauri.conf.json's productName and
# the layout from cargo-mobile2, and neither is ours to pin.
app=$(find mobile/src-tauri/gen/apple/build -maxdepth 3 -name '*.app' -type d 2>/dev/null | head -1)
[ -n "$app" ] || fail "the build produced no .app under gen/apple/build ($OUT/build.log)"
say "built $app"

# The bundle id from what was actually built, not from tauri.conf.json — if those two ever disagree,
# the one that matters is the one on disk, and installing under the wrong id fails obscurely.
bid=$(plutil -extract CFBundleIdentifier raw "$app/Info.plist" 2>/dev/null || true)
[ -n "$bid" ] || fail "could not read CFBundleIdentifier from $app/Info.plist"
say "bundle id $bid"

# ---------------------------------------------------------------------------
# 2) A Simulator to run it on. Prefer one the image already has.
# ---------------------------------------------------------------------------
udid=$(xcrun simctl list devices available | awk -F'[()]' '/^ *iPhone/ {print $2; exit}')
if [ -z "$udid" ]; then
    say "no iPhone simulator available — creating one"
    runtime=$(xcrun simctl list runtimes | awk '/^iOS /{r=$NF} END{print r}')
    devtype=$(xcrun simctl list devicetypes | awk -F'[()]' '/iPhone/{d=$2} END{print d}')
    [ -n "$runtime" ] && [ -n "$devtype" ] || fail "no iOS runtime or iPhone device type on this machine"
    udid=$(xcrun simctl create fm-smoke "$devtype" "$runtime") || fail "simctl create failed"
fi
say "device $udid"
# `bootstatus -b` boots if needed and blocks until the system is actually up, which is the
# difference between "the process started" and "the springboard will accept an install".
xcrun simctl bootstatus "$udid" -b >"$OUT/bootstatus.txt" 2>&1 || fail "the simulator did not boot ($OUT/bootstatus.txt)"

xcrun simctl install "$udid" "$app" || fail "simctl install failed"
say "installed"

# ---------------------------------------------------------------------------
# 3) The launch check, run twice. Launch 1 is the one that used to be gray on Android.
# ---------------------------------------------------------------------------
: > "$OUT/stats.txt"
launch() {
    n=$1
    xcrun simctl terminate "$udid" "$bid" >/dev/null 2>&1 || true

    # `--console-pty` blocks for the life of the app, so it goes to the background with its output
    # in a file. That file is the entire diagnostic channel this platform has here.
    xcrun simctl launch --console-pty "$udid" "$bid" > "$OUT/console-$n.log" 2>&1 &
    console_pid=$!

    # B — painted. Polled rather than slept-and-hoped, so the number in stats.txt is the time the
    # app actually took, which is the number worth watching across releases.
    k=0
    while :; do
        xcrun simctl io "$udid" screenshot "$OUT/open$n.png" >/dev/null 2>&1 \
            || fail "launch $n: screenshot failed"
        dev=$(vips deviate "$OUT/open$n.png" 2>/dev/null || echo 0)
        printf 'launch %s: t+%ss deviation=%s\n' "$n" "$k" "$dev" >> "$OUT/stats.txt"
        awk -v d="$dev" -v m="$MIN_DEVIATION" 'BEGIN{exit !(d+0 > m+0)}' && break
        k=$((k + 1))
        [ "$k" -lt 60 ] || fail "launch $n: the screen never painted (deviation $dev <= $MIN_DEVIATION after 60s) — $OUT/open$n.png"
        sleep 1
    done
    say "launch $n painted at t+${k}s (deviation $dev)"

    # C — the vaults opened, and the shell got to say so. The same assertion `android-smoke.sh`
    # makes, against the same log line, which is why the iOS logger carries the same prefix.
    j=0
    until grep -q 'vaults ready' "$OUT/console-$n.log" 2>/dev/null; do
        j=$((j + 1))
        if [ "$j" -ge 30 ]; then
            echo "--- console-$n.log ---" >&2; cat "$OUT/console-$n.log" >&2 || true
            fail "launch $n: no 'vaults ready' line in 30s. Either the store never opened, or stderr
  is not reaching the pty — in which case try 'xcrun simctl spawn $udid log stream' before
  concluding anything about the app ($OUT/console-$n.log)."
        fi
        sleep 1
    done
    say "launch $n reported 'vaults ready'"

    if grep -q 'formicaria ERROR' "$OUT/console-$n.log"; then
        fail "launch $n: an error was logged at startup ($OUT/console-$n.log)"
    fi
    kill "$console_pid" >/dev/null 2>&1 || true
}
launch 1
launch 2
say "both launches painted and reported ready — the 'blank first, fine second' signature is absent"

# ---------------------------------------------------------------------------
# 4) What `app_data_dir()` actually resolved to. Not an assertion — an answer.
#
# One of the open questions rung 2 exists to close (`ios-plan-2026-09-02.md`): where the vault lands
# decides both iCloud-backup exposure and whether the Files app can ever reach it. Recorded, not
# judged, because the right answer is a decision and this script does not get to make it.
# ---------------------------------------------------------------------------
container=$(xcrun simctl get_app_container "$udid" "$bid" data 2>/dev/null || true)
if [ -n "$container" ]; then
    echo "$container" > "$OUT/app-container.txt"
    find "$container" -maxdepth 3 >> "$OUT/app-container.txt" 2>/dev/null || true
    say "data container: $container (tree in $OUT/app-container.txt)"
fi

# ---------------------------------------------------------------------------
# 5) Teardown. Simulator-only verbs, on a device this script may itself have created.
# ---------------------------------------------------------------------------
df -h / "$root" > "$OUT/disk-after.txt" 2>&1 || true
xcrun simctl terminate "$udid" "$bid" >/dev/null 2>&1 || true
xcrun simctl uninstall "$udid" "$bid" >/dev/null 2>&1 || true
say "PASS — artifacts in $OUT"
