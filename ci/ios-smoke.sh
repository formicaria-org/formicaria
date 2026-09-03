#!/bin/sh
# **Does the app boot on a Simulator, twice in a row, and say so?** — iOS rung 2.
#
# The analogue of `ci/android-smoke.sh`, and it inherits that file's central lesson: the owner's
# phone once opened to a gray screen on the first launch and worked on the second, so **every check
# here runs on TWO cold launches and requires both to pass.** A one-launch smoke test would have
# gone green through the whole outage.
#
# What it asserts, and **in the order it asserts them**, which is load-bearing:
#   A. the app is installed and `simctl launch` stays alive  (necessary, proves nothing about paint)
#   C. it reports `vaults ready` on the pty — the store opened, on a real iOS
#   B. only *then*, that the screen is neither flat nor still the home screen (libvips)
#
# C precedes B deliberately. `android-smoke.sh` waits for `dumpsys window` to name our app before it
# measures a single pixel; iOS has no `dumpsys`, and the first screenshot after `simctl launch` is
# the **home screen** — wallpaper and icons, whose deviation sails past any blank-screen floor. A
# pixel check run first would pass at t+0 and prove nothing. `vaults ready` is the gate this
# platform does have, and a gray screen still fails B after passing it.
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

say()  { echo "ios-smoke: $*"; }
# Two sourced libraries, both shared with another iOS script, both for the same reason: the parts of
# an iOS script most likely to be quietly wrong should exist once and be self-tested once.
#   `simctl.sh`       — the device parsers and acquisition, shared with `ci/ios-https-probe.sh`
#   `ios-toolchain.sh` — TOOLCHAINS, the verifying rustup shim on PATH, `run_logged`, the Tauri CLI
#                        architecture check and `ios_project_ready`, shared with `ci/ios-package.sh`
# `--self-test` below exercises the parsers against captured output; `ci/checks.sh` runs it.
. "$root/ci/lib/simctl.sh"
. "$root/ci/lib/ios-toolchain.sh"

if [ "${1:-}" = "--self-test" ]; then
    t_fail=0
    check() {  # name expected actual
        if [ "$2" = "$3" ]; then
            echo "  ok   $1"
        else
            echo "  FAIL $1: expected '$2', got '$3'"; t_fail=1
        fi
    }
    # Captured from real `xcrun simctl` output. The `(3rd generation)` device is the regression.
    devices='== Devices ==
-- iOS 26.5 --
    iPhone SE (3rd generation) (7B2A1C4D-9E3F-4A55-B1C2-0D3E4F5A6B7C) (Shutdown)
    iPhone 17 (A1B2C3D4-1111-2222-3333-444455556666) (Shutdown)
    iPad Pro 11-inch (M4) (C0FFEE00-1111-2222-3333-444455556666) (Shutdown)'
    runtimes='== Runtimes ==
iOS 16.4 (16.4 - 20E247) - com.apple.CoreSimulator.SimRuntime.iOS-16-4 (unavailable, runtime profile not found using '"'"'System'"'"' match policy)
iOS 26.5 (26.5 - 23F79) - com.apple.CoreSimulator.SimRuntime.iOS-26-5'
    devtypes='iPhone 17 (com.apple.CoreSimulator.SimDeviceType.iPhone-17)
iPhone SE (3rd generation) (com.apple.CoreSimulator.SimDeviceType.iPhone-SE-3rd-generation)'

    echo "ios-smoke --self-test: the simctl parsers"
    check "udid ignores a bracketed device name" \
        "7B2A1C4D-9E3F-4A55-B1C2-0D3E4F5A6B7C" "$(printf '%s\n' "$devices" | pick_udid)"
    check "udid is empty when no iPhone is listed" \
        "" "$(printf '%s\n' "== Devices ==" | pick_udid)"
    check "runtime is an identifier, never 'policy)'" \
        "com.apple.CoreSimulator.SimRuntime.iOS-26-5" "$(printf '%s\n' "$runtimes" | pick_runtime)"
    check "devtype is an identifier, not '3rd generation'" \
        "com.apple.CoreSimulator.SimDeviceType.iPhone-SE-3rd-generation" "$(printf '%s\n' "$devtypes" | pick_devtype)"

    # Captured from `simctl list devices available` on the runner during the rung-3 dispatch of
    # 2026-09-03 — the run that spent 20 minutes booting a device that is *not in this list*.
    # **Several spellings on purpose.** The previous fixture was hand-written in one assumed
    # format and the parser agreed with it while disagreeing with the runner. These cover the
    # simctl shape, a `-showdestinations` shape, an unindented line and a `(Booted)` state; the
    # matcher must be indifferent to all of it. The runner's real list now ships in the artifact
    # as `devices-available.txt`, so this fixture can finally be replaced by a measurement.
    avail='== Devices ==
-- iOS 26.5 --
    iPad Pro 13-inch (M5) (B29636C6-DD18-4550-B6F7-54EB38151CFA) (Shutdown)
    iPhone 17 (FC5FEF2A-E933-4515-AAEF-C9FC16651D0B) (Shutdown)
    iPhone 17 Pro (6EE862FE-93F2-4D55-946E-8745EE2B3A88) (Booted)
    iPhone 17 Pro Max (6300F6FD-611A-422C-B6F2-F8C162FBDDF7) (Shutdown)
iPhone 17e (A10B76FC-4115-4057-8550-967F510895A7) (Shutdown)
    iPhone Air (A8C66A3B-A7A0-4BD7-A222-B48E3A881425)'

    echo "ios-smoke --self-test: the available-device matcher"
    check "an exact hyphenated name" \
        "6300F6FD-611A-422C-B6F2-F8C162FBDDF7" "$(printf '%s\n' "$avail" | pick_device_by iPhone-17-Pro-Max)"
    check "'iPhone-17' is exact, not the Pro Max that merely contains it" \
        "FC5FEF2A-E933-4515-AAEF-C9FC16651D0B" "$(printf '%s\n' "$avail" | pick_device_by iPhone-17)"
    check "a substring with no exact match still resolves" \
        "A8C66A3B-A7A0-4BD7-A222-B48E3A881425" "$(printf '%s\n' "$avail" | pick_device_by Air)"
    check "a name with parentheses survives hyphenation" \
        "B29636C6-DD18-4550-B6F7-54EB38151CFA" "$(printf '%s\n' "$avail" | pick_device_by 'iPad-Pro-13-inch-(M5)')"
    check "**iPhone-SE is absent and must come back empty**, not as some other device" \
        "" "$(printf '%s\n' "$avail" | pick_device_by iPhone-SE)"
    # **The regression that cost a job.** The runner had this device; the first matcher returned
    # empty for it and the sweep fell through to create-and-cold-boot. Here it is unindented and
    # spelled with a space, against a hyphenated query — the mismatch the old parser could not see.
    check "iPhone-17e resolves on the runner that actually had one" \
        "A10B76FC-4115-4057-8550-967F510895A7" "$(printf '%s\n' "$avail" | pick_device_by iPhone-17e)"
    check "an absent size is empty rather than the first device on the list" \
        "" "$(printf '%s\n' "$avail" | pick_device_by iPad-mini)"
    [ "$t_fail" = 0 ] || { echo "ios-smoke --self-test: FAILED" >&2; exit 1; }
    echo "ios-smoke --self-test: all parsers ok"
    exit 0
fi

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
# **Features are left at their defaults, and here is the honest version of what that costs.**
# `build.rs`'s `agent_shell` gates `mod agent` — the *code* — so no iOS build runs an assistant
# however it is invoked, and no `--no-default-features` has to be remembered. It does **not** prune
# the dependency graph: the `agent` feature is still on, so `fm-agent`, `fm-agent-run` and their
# `download` tail (`ureq` → `rustls` → `ring`, plus `sha2`/`flate2`/`tar`/`zip`) are compiled for
# `aarch64-apple-ios-sim` here for the first time anywhere. `check-cross` covers `fm-agent` only —
# by design, since `ring`'s C build script needs an Apple SDK that Linux does not have — so **if
# this build fails inside `ring`, that is a known-unmeasured edge, not a mystery.** Pruning them
# needs `tauri ios build`'s feature plumbing, which travels over the CLI's own WebSocket rather than
# an env var, and this script deliberately does not depend on a flag nobody here can verify.
# ---------------------------------------------------------------------------
# `run_logged` and `tauri_ios` come from `ci/lib/ios-toolchain.sh`, sourced above. They are not
# simulator-specific and rung 5 needs them byte-identically; see that file for what each one is
# working around.

# The CLI architecture check, `tauri ios init` if `gen/apple` is absent, and the zlib/iconv linker
# injection that must follow it — all three in `ci/lib/ios-toolchain.sh`, because none of them is
# simulator-specific and rung 5 hits every one of them identically.
tauri_cli_arch_check
ios_project_ready

say "building for the Simulator (aarch64-sim, unsigned)…"
run_logged build tauri_ios build --target aarch64-sim --no-sign --verbose

# The CLI renames the .app out of the .xcarchive into `gen/apple/build/<arch>/`, where <arch> for
# `--target aarch64-sim` is `arm64-sim`. The *name* is globbed — it comes from tauri.conf.json's
# productName via cargo-mobile2 — but the **directory is pinned**: a reused checkout can hold a
# leftover `build/arm64/<Name>.app` from a device build at the same depth, and installing that on a
# Simulator fails in a way that looks like the app is broken rather than like the wrong file.
app=$(find mobile/src-tauri/gen/apple/build/arm64-sim -maxdepth 1 -name '*.app' -type d 2>/dev/null | head -1)
[ -n "$app" ] || fail "the build produced no .app in gen/apple/build/arm64-sim ($OUT/build.log)"
say "built $app"

# The bundle id from what was actually built, not from tauri.conf.json — if those two ever disagree,
# the one that matters is the one on disk, and installing under the wrong id fails obscurely.
bid=$(plutil -extract CFBundleIdentifier raw "$app/Info.plist" 2>/dev/null || true)
[ -n "$bid" ] || fail "could not read CFBundleIdentifier from $app/Info.plist"
say "bundle id $bid"

# ---------------------------------------------------------------------------
# 2) A Simulator to run it on. Prefer one the image already has.
# ---------------------------------------------------------------------------
# **Match the identifier by its shape, never by which bracket it is in.** `simctl` lines look like
#     iPhone 17 (A1B2C3D4-…) (Shutdown)
#     iPhone SE (3rd generation) (A1B2C3D4-…) (Shutdown)
# so "the second parenthesised group" is the UDID on the first line and the string `3rd generation`
# on the second — and an `(Nth generation)` iPhone is an ordinary member of the default device set.
# That parse would have handed `simctl bootstatus` a device name, and it would have done so *after*
# the 25-45 minute build was already paid for.
udid=$(simctl_device "$OUT/bootstatus.txt")
say "device $udid"

xcrun simctl install "$udid" "$bid" >/dev/null 2>&1 || true
xcrun simctl install "$udid" "$app" || fail "simctl install failed"
say "installed"

# **Where the app writes its own log, which is the channel that actually works.** stderr reaches
# nobody here: run 91364602829 produced a byte-empty pty and not one of our records in the unified
# log, from a process that was demonstrably running. `install_logger`'s iOS arm now also writes to
# `std::env::temp_dir()/formicaria.log`, which on iOS is inside this container. Resolved once, here,
# because the poll loop needs it — `tmp/` itself only appears at first launch, so it is searched for
# each time rather than assumed.
container=$(xcrun simctl get_app_container "$udid" "$bid" data 2>/dev/null || true)
[ -n "$container" ] && say "data container: $container" || say "no data container yet (first launch will make one)"

# ---------------------------------------------------------------------------
# 3) The launch check, run twice. Launch 1 is the one that used to be gray on Android.
# ---------------------------------------------------------------------------
: > "$OUT/stats.txt"

# **Both questions are polled together, and neither gates the other.**
#
# The first version asserted them in sequence — `vaults ready` on the pty, then the pixel check —
# because iOS has no `dumpsys window` to say "our app is frontmost", so a pixel check run too early
# measures the **home screen** (wallpaper and icons, deviation far above any blank-screen floor) and
# passes at t+0 proving nothing. Ordering fixed that and introduced a worse problem: rung 2's third
# firing (run 91357302585) reached `no 'vaults ready' line in 60s` with an **empty pty** and
# therefore **no screenshot at all** — so "the app runs but cannot speak" and "the app never
# painted" were indistinguishable, which is the one distinction the job was bought to make.
#
# So both are now watched in one loop, and the home-screen baseline is what keeps the pixel check
# honest: a frame identical to it, or flat, is not our app. The verdict below names which of the two
# failed, because they have opposite meanings — a silent-but-painting app is a *logging* problem
# (the `os_log` question this project deferred), while a mute black screen is a *boot* problem.
launch() {
    n=$1
    xcrun simctl terminate "$udid" "$bid" >/dev/null 2>&1 || true

    # What this screen looks like with no app on it. Evidence, and the reference the pixel check
    # compares against; its deviation goes to stats.txt so the numbers below can be read.
    xcrun simctl io "$udid" screenshot "$OUT/springboard-$n.png" >/dev/null 2>&1 \
        || fail "launch $n: could not screenshot the home screen"
    base_dev=$(vips deviate "$OUT/springboard-$n.png" 2>/dev/null || echo 0)
    printf 'launch %s: home screen deviation=%s  (context, NOT a threshold)\n' "$n" "$base_dev" \
        >> "$OUT/stats.txt"

    # **The unified log, captured from before the launch.** `install_logger`'s iOS arm writes to
    # stderr on the reasoning that `--console-pty` prints it (`decisions.md` 2026-09-03). Run
    # 91357302585 produced an empty pty from a process that stayed alive for 60s, so that reasoning
    # is now in question — and this is the fallback the kill criterion names, run automatically
    # rather than left as an instruction for a human who would need another job to act on it.
    xcrun simctl spawn "$udid" log stream --style compact \
        --predicate 'processImagePath CONTAINS "formicaria"' > "$OUT/oslog-$n.log" 2>&1 &
    oslog_pid=$!

    # `--console-pty` blocks for the life of the app, so it goes to the background with its output
    # in a file.
    xcrun simctl launch --console-pty "$udid" "$bid" > "$OUT/console-$n.log" 2>&1 &
    console_pid=$!

    ready=no; painted=no; ready_at=-1; painted_at=-1; died=no
    k=0
    while [ "$k" -lt 90 ]; do
        if [ "$painted" = no ]; then
            xcrun simctl io "$udid" screenshot "$OUT/open$n.png" >/dev/null 2>&1 || true
            dev=$(vips deviate "$OUT/open$n.png" 2>/dev/null || echo 0)
            same=no
            if cmp -s "$OUT/springboard-$n.png" "$OUT/open$n.png"; then same=yes; fi
            printf 'launch %s: t+%ss deviation=%s identical-to-home=%s ready=%s\n' \
                "$n" "$k" "$dev" "$same" "$ready" >> "$OUT/stats.txt"
            if [ "$same" = no ] && awk -v d="$dev" -v m="$MIN_DEVIATION" 'BEGIN{exit !(d+0 > m+0)}'; then
                painted=yes; painted_at=$k
                cp "$OUT/open$n.png" "$OUT/painted-$n.png" 2>/dev/null || true
            fi
        fi
        # **Three channels, and which one spoke is itself a result.** The pty and the unified log
        # are both known to have carried nothing on 2026-09-03; the app's own file is the fix. If a
        # future run reports `via=pty`, stderr started working and this note can go.
        if [ "$ready" = no ]; then
            applog=""
            if [ -n "$container" ]; then
                applog=$(find "$container" -maxdepth 3 -name 'formicaria.log' 2>/dev/null | head -1)
            fi
            for src in "$OUT/console-$n.log:pty" "$OUT/oslog-$n.log:unified-log" "${applog:-/nonexistent}:app-file"; do
                f=${src%:*}; via=${src##*:}
                if [ -f "$f" ] && grep -q 'vaults ready' "$f" 2>/dev/null; then
                    ready=yes; ready_at=$k; ready_via=$via
                    break
                fi
            done
        fi
        [ "$painted" = yes ] && [ "$ready" = yes ] && break
        # A launch that exited is decided; keep whatever was captured and stop waiting for it.
        if ! kill -0 "$console_pid" 2>/dev/null; then died=yes; break; fi
        k=$((k + 1))
        sleep 1
    done
    kill "$oslog_pid" "$console_pid" >/dev/null 2>&1 || true

    # The app's own log, kept whatever happened — on a silent run it is the only thing that can say
    # how far startup got, and it outlives the process the way the two stream captures do not.
    if [ -n "$container" ]; then
        found=$(find "$container" -maxdepth 3 -name 'formicaria.log' 2>/dev/null | head -1)
        [ -n "$found" ] && cp "$found" "$OUT/applog-$n.log" 2>/dev/null || true
    fi

    # Crash reports, if the app died on its own. Cheap, and the only place a dyld or TCC failure
    # says anything at all.
    find "$HOME/Library/Logs/DiagnosticReports" -name '*formicaria*' -newermt '-10 minutes' \
        -exec cp {} "$OUT/" \; 2>/dev/null || true

    if [ "$painted" = yes ] && [ "$ready" = yes ]; then
        say "launch $n: painted at t+${painted_at}s (deviation $dev vs home $base_dev), ready at t+${ready_at}s via ${ready_via:-?}"
        # **`applog-$n.log` is in this list, and its absence was a hole.** On iOS stderr was
        # measured to reach nobody (`decisions.md`, *the iOS log is a file in the app container*),
        # so the app's own file is the channel that actually carries a startup error — and it was
        # copied here for a human to read while the assertion looked at the two channels that are
        # empty on this platform. Now that the WebView forwards `console.error` as `web: …`
        # through the same `log` sink (`decisions.md`, *the WebView gets a voice*), this grep is
        # what makes a frontend failure fail the job instead of being filed as an artifact.
        if grep -q 'formicaria ERROR' \
            "$OUT/console-$n.log" "$OUT/oslog-$n.log" "$OUT/applog-$n.log" 2>/dev/null; then
            fail "launch $n: an error was logged at startup — see $OUT/applog-$n.log,
  $OUT/console-$n.log. A 'web:' prefix means it came from the page, not the shell."
        fi
        return 0
    fi

    # **Name which half failed, because they mean opposite things.**
    echo "--- console-$n.log (pty) ---" >&2; cat "$OUT/console-$n.log" >&2 2>/dev/null || true
    echo "--- applog-$n.log (the app's own file) ---" >&2; cat "$OUT/applog-$n.log" >&2 2>/dev/null || true
    echo "--- oslog-$n.log (unified log, tail) ---" >&2; tail -40 "$OUT/oslog-$n.log" >&2 2>/dev/null || true
    if [ "$painted" = yes ]; then
        fail "launch $n: **the app painted but never said 'vaults ready'** (painted t+${painted_at}s,
  deviation $dev vs home $base_dev; launch process $( [ "$died" = yes ] && echo exited || echo 'still alive' )).
  The app runs. This is the diagnostic channel, not the app: if the unified log above is also empty,
  stderr does not reach either channel on iOS and \`install_logger\` needs the os_log backend that
  \`decisions.md\` deferred — that entry names this as its reversal trigger. Not a kill criterion."
    elif [ "$ready" = yes ]; then
        fail "launch $n: **reported ready but never painted** (ready t+${ready_at}s, deviation $dev
  vs home $base_dev, identical-to-home=$same). The store opened and the screen did not — this is the
  gray-screen signature \`android-smoke.sh\` exists for, now on iOS. $OUT/open$n.png"
    else
        fail "launch $n: **neither painted nor spoke** in 90s (deviation $dev vs home $base_dev,
  launch process $( [ "$died" = yes ] && echo exited || echo 'still alive' )). If the launch process
  exited, read the pty dump above and any crash report copied into $OUT. If it is still alive with a
  home-screen frame, the app is not coming up at all. **This is the kill criterion** — record it and
  stop rather than iterating."
    fi
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
# 4b) **Rung 3 — the same app at other screen sizes.** Off unless `FM_IOS_SIZES` is set.
#
# Additive on purpose: the single-device path above took four billed jobs to get right, and this
# must not be able to break it. It also reuses the `.app` already built rather than rebuilding —
# the build is 360s of a ~460s job, so a size sweep that rebuilt per device would cost more than
# every other rung put together.
#
# `FM_IOS_SIZES` is a space-separated list of **substrings** matched against `simctl list
# devicetypes`, e.g. "iPhone-SE iPhone-17-Pro-Max". Substrings rather than identifiers because the
# runner image's device set changes without notice: a name that no longer exists should cost a
# skipped size and a note, not a failed job. Screenshots only — this rung asks what the UI *looks*
# like, and the assertions that matter (paint, `vaults ready`) were already made above.
# ---------------------------------------------------------------------------
if [ -n "${FM_IOS_SIZES:-}" ]; then
    # **Record the raw list before parsing it.** The first `pick_device_by` was validated against a
    # fixture written by hand from a *different command's* output and returned empty on a runner
    # that had the device — costing a create-and-cold-boot per size. Nobody here can run `simctl`,
    # so the only way to stop guessing at its format is to ship it in the artifact. Cheap, and it
    # turns the next parser question into a read rather than another billed dispatch.
    xcrun simctl list devices available > "$OUT/devices-available.txt" 2>&1 || true
    say "device list recorded ($(grep -c . "$OUT/devices-available.txt" 2>/dev/null || echo 0) lines) -> devices-available.txt"
    say "matcher sees: $(device_index < "$OUT/devices-available.txt" | wc -l | tr -d ' ') device(s)"
    device_index < "$OUT/devices-available.txt" > "$OUT/devices-parsed.txt" 2>&1 || true

    runtime=$(xcrun simctl list runtimes available | pick_runtime)
    [ -n "$runtime" ] || fail "FM_IOS_SIZES is set but no iOS runtime is available"
    for want in $FM_IOS_SIZES; do
        # **An already-available device first, and only then create one.** The runner image ships
        # several iPhones already paired with a runtime and pre-created; using one skips a create
        # *and* a first cold boot, and — the expensive part — it cannot select a device that will
        # never boot. `simctl list devicetypes` lists what Xcode knows about, which on Xcode 26
        # still includes an iPhone SE that no iOS 26 runtime will pair with: created fine, then sat
        # in `bootstatus` until the job was 20 minutes old. See `pick_device_by` in lib/simctl.sh.
        created_size=0
        sud=$(xcrun simctl list devices available | pick_device_by "$want")
        if [ -n "$sud" ]; then
            say "size '$want': using the runner's own device $sud"
        else
            dt=$(xcrun simctl list devicetypes \
                | grep -F "$want" | tail -1 \
                | grep -oE 'com\.apple\.CoreSimulator\.SimDeviceType\.[A-Za-z0-9._-]+' || true)
            if [ -z "$dt" ]; then
                say "size '$want': no available device and no such device type — skipped"
                continue
            fi
            say "size '$want': no available device; creating one from $dt"
            sud=$(xcrun simctl create "fm-size-$want" "$dt" "$runtime" 2>/dev/null || true)
            if [ -z "$sud" ]; then
                say "size '$want': simctl create failed — skipped"
                continue
            fi
            created_size=1
        fi
        # Bounded, always. Even a pre-paired device can stall on a loaded 3-core runner, and the
        # whole point of this rung is screenshots — none of them is worth an unbounded wait.
        if boot_with_deadline "$sud" "$OUT/bootstatus-$want.txt" "${FM_IOS_BOOT_TIMEOUT:-240}" \
           && xcrun simctl install "$sud" "$app" >/dev/null 2>&1; then
            xcrun simctl launch "$sud" "$bid" >/dev/null 2>&1 || true
            # No polling loop: the assertions live above. Give the WebView a moment, then record.
            i=0
            while [ "$i" -lt 20 ]; do
                xcrun simctl io "$sud" screenshot "$OUT/size-$want.png" >/dev/null 2>&1 || true
                d=$(vips deviate "$OUT/size-$want.png" 2>/dev/null || echo 0)
                awk -v d="$d" -v m="$MIN_DEVIATION" 'BEGIN{exit !(d+0 > m+0)}' && break
                i=$((i + 1)); sleep 1
            done
            printf 'size %s: deviation=%s at t+%ss (%s)\n' "$want" "$d" "$i" "$dt" >> "$OUT/stats.txt"
            say "size '$want': screenshot at t+${i}s (deviation $d)"
        else
            say "size '$want': would not boot or install — skipped ($OUT/bootstatus-$want.txt)"
        fi
        xcrun simctl shutdown "$sud" >/dev/null 2>&1 || true
        # **Delete only what this run created** — the same rule `created_device` follows at the end
        # of this script. Now that the sweep prefers the runner's own pre-created devices, deleting
        # unconditionally would destroy part of the image's device set: harmless on a throwaway
        # runner, wrong on the "a Mac someone else owns" this file's header offers.
        # `if`, not `&&`: under `set -eu` a trailing AND-list whose test fails is a non-zero status
        # on the loop body's last command.
        if [ "$created_size" = 1 ]; then xcrun simctl delete "$sud" >/dev/null 2>&1 || true; fi
    done
fi

# ---------------------------------------------------------------------------
# 5) Teardown. Simulator-only verbs, on a device this script may itself have created.
# ---------------------------------------------------------------------------
df -h / "$root" > "$OUT/disk-after.txt" 2>&1 || true
xcrun simctl terminate "$udid" "$bid" >/dev/null 2>&1 || true
xcrun simctl uninstall "$udid" "$bid" >/dev/null 2>&1 || true
# Only a device *this run* created is deleted. A CI runner is thrown away either way, but the
# header offers "a Mac someone else owns" and on one of those an undeleted `fm-smoke` accumulates
# every run — and deleting a simulator the machine's owner made would be the worse mistake.
if [ "${created_device:-0}" = 1 ]; then
    xcrun simctl shutdown "$udid" >/dev/null 2>&1 || true
    xcrun simctl delete "$udid" >/dev/null 2>&1 || true
    say "deleted the simulator this run created"
fi
say "PASS — artifacts in $OUT"
