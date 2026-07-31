#!/bin/sh
# **Does the app actually paint, twice in a row?**
#
# The test that did not exist when it was needed. The owner's phone opened to a gray screen on the
# first launch and worked on the second (2026-07-31); nothing in `pixi run ci` could see it, because
# no test ever starts the mobile shell and every UI test runs against `ui/src/lib/mock.ts`. The
# signature of that bug — *fine the second time* — is why every check below runs on TWO launches and
# requires both to pass. A one-launch smoke test would have gone green through the whole outage.
#
# What it asserts, in order of how much it is worth:
#   D. two consecutive cold launches both paint and both report the same startup line
#   B. the screen is not blank        (screenshot standard deviation, via libvips)
#   C. the vaults were opened, and BEFORE the cert store and the model  (our own logcat tag)
#   A. the Activity is resumed       (necessary, and proves nothing about paint — the whole lesson)
#
# EMULATOR ONLY. It installs, force-stops and uninstalls, so it refuses to run against anything that
# is not an `emulator-*` serial: the phone this project is developed against is the owner's personal
# device (docs/context/mobile-design.md § "Device safety — the phone is not a test rig"), and a
# guard that is a comment is not a guard.
#
# Usage:
#   pixi run android-init && pixi run android-avd     # once
#   pixi run android-emu &                            # headless; screencap needs no display
#   pixi run android-smoke
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ADB="$root/.android/platform-tools/adb"
PKG=dev.formicaria.notes
ACT="$PKG/.MainActivity"
OUT="$root/target/android-smoke/$(date +%Y%m%d-%H%M%S)"
# The blank-screen threshold. `vips deviate` is the standard deviation of the pixels: a flat gray or
# white surface is ~0, a painted app (top bar, text, a card) is tens — measured 102.9 on a real
# screenshot and ~0 on a synthetic flat one. Every measured value is written to stats.txt so the
# threshold stays grounded in numbers somebody can re-read rather than a constant somebody guessed.
MIN_DEVIATION=10

[ -x "$ADB" ] || { echo "android-smoke: no adb at $ADB — run 'pixi run android-init' first" >&2; exit 1; }

# ---------------------------------------------------------------------------
# 0) The safety gate: exactly one target, and it must be an emulator.
#
# **First, before every other precondition.** A guard that runs after an environment check is a
# guard that a missing tool can make you skip past while you fix the environment — and the whole
# point of it is to be the thing that always ran.
# ---------------------------------------------------------------------------
serials=$("$ADB" devices | awk '/\tdevice$/{print $1}')
count=$(printf '%s\n' "$serials" | grep -c . || true)
if [ "$count" -ne 1 ]; then
    echo "android-smoke: expected exactly one attached device, got $count:" >&2
    printf '  %s\n' $serials >&2
    echo "  Start the emulator ('pixi run android-emu &') and 'adb disconnect' anything else." >&2
    exit 1
fi
S=$serials
case "$S" in
    emulator-*) : ;;
    *)  echo "android-smoke: REFUSING — '$S' is not an emulator." >&2
        echo "  This script installs, force-stops and uninstalls. It must never touch a real phone;" >&2
        echo "  the real-device pass is observation only (docs/context/mobile-design.md)." >&2
        exit 1 ;;
esac
# From here on every call is "$ADB" -s "$S". `grep -n 'adb ' ci/android-smoke.sh` should show no
# bare invocation — that is the property that keeps the guard above meaningful.
adbs() { "$ADB" -s "$S" "$@"; }

command -v vips >/dev/null 2>&1 || { echo "android-smoke: vips not on PATH — run it as 'pixi run android-smoke'" >&2; exit 1; }

mkdir -p "$OUT"
fail() { echo "FAIL: $*" >&2; adbs logcat -d > "$OUT/logcat.txt" 2>/dev/null || true; echo "  artifacts: $OUT" >&2; exit 1; }
say() { echo "android-smoke: $*"; }

# ---------------------------------------------------------------------------
# 1) Wait for the emulator to be a working Android, not just a booted process.
# ---------------------------------------------------------------------------
adbs wait-for-device
i=0
while [ "$(adbs shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" != "1" ]; do
    i=$((i + 1))
    [ "$i" -lt 180 ] || fail "emulator did not finish booting in 180s"
    sleep 1
done
say "emulator $S up"

# ---------------------------------------------------------------------------
# 2) Build + install the debug APK.
# ---------------------------------------------------------------------------
apk=mobile/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
if [ "${SMOKE_SKIP_BUILD:-0}" != "1" ]; then
    say "building the debug APK (x86_64)…"
    ( cd "$root" && pixi run android-apk ) || fail "android-apk build failed"
fi
[ -f "$root/$apk" ] || fail "no APK at $apk (set SMOKE_SKIP_BUILD=1 only when one is already built)"
adbs install -r "$root/$apk" >/dev/null || fail "install failed"
say "installed $(basename "$apk")"

# ---------------------------------------------------------------------------
# 3) Seed one note, so "it has data" is a decidable question.
#
# The first launch creates the vault tree (`configure_paths`), then `run-as` writes a note into it —
# which works only because the debug APK is debuggable, and is exactly why this layer needs an
# emulator rather than a phone. The title carries a nonce so a stale index cannot fake a pass.
# ---------------------------------------------------------------------------
NONCE="SMOKE-$(date +%s)"
adbs shell am start -n "$ACT" >/dev/null || fail "could not launch $ACT"
sleep 12
adbs shell am force-stop "$PKG" >/dev/null
note="---\nid: 01SMOKE0000000000000000000\ntype: note\ntitle: $NONCE\ncreated: 2026-07-31T00:00:00Z\nupdated: 2026-07-31T00:00:00Z\n---\n\nseeded by ci/android-smoke.sh\n"
for dir in files/vaults/notes/notes files/vault/notes; do
    if adbs shell run-as "$PKG" sh -c "[ -d $dir ]" >/dev/null 2>&1; then
        adbs shell run-as "$PKG" sh -c "printf '$note' > $dir/01SMOKE0000000000000000000.md" \
            || fail "could not seed a note into $dir"
        say "seeded $NONCE into $dir"
        seeded=1
        break
    fi
done
[ "${seeded:-0}" = "1" ] || fail "the app created no vault directory on first launch — that is itself the bug"

# ---------------------------------------------------------------------------
# 4) The launch check, run twice. `open1` is the one that used to be gray.
# ---------------------------------------------------------------------------
: > "$OUT/stats.txt"
launch() {
    n=$1
    adbs shell am force-stop "$PKG" >/dev/null
    # Only new lines, so launch 2 cannot pass on launch 1's log.
    adbs logcat -c >/dev/null 2>&1 || true
    adbs shell am start -n "$ACT" >/dev/null || fail "launch $n: am start failed"

    # A — resumed. Necessary, and deliberately not sufficient: a resumed Activity over an unpainted
    # WebView is precisely what the owner was looking at.
    #
    # **`mFocusedApp`, not `mCurrentFocus`.** They answer different questions: `mCurrentFocus` is the
    # focused *window*, which any transient system window takes — on the owner's phone
    # 2026-07-31 it read `NotificationShade` while our Activity was plainly the front app, and the
    # first version of this check duly reported "never took focus" for a running, painted app. The
    # question here is which *app* is in front, and that is `mFocusedApp`.
    j=0
    until adbs shell dumpsys window 2>/dev/null | grep -q "mFocusedApp.*$PKG"; do
        j=$((j + 1)); [ "$j" -lt 30 ] || fail "launch $n: $PKG never became the focused app"
        sleep 1
    done

    # B — painted. Poll rather than sleep-and-hope, so the number in stats.txt is the time the app
    # actually took, which is the number worth watching over releases.
    k=0
    while :; do
        adbs exec-out screencap -p > "$OUT/open$n.png" 2>/dev/null || fail "launch $n: screencap failed"
        dev=$(vips deviate "$OUT/open$n.png" 2>/dev/null || echo 0)
        printf 'launch %s: t+%ss deviation=%s\n' "$n" "$k" "$dev" >> "$OUT/stats.txt"
        awk -v d="$dev" -v m="$MIN_DEVIATION" 'BEGIN{exit !(d+0 > m+0)}' && break
        k=$((k + 1))
        [ "$k" -lt 40 ] || fail "launch $n: the screen never painted (deviation $dev <= $MIN_DEVIATION after 40s) — $OUT/open$n.png"
        sleep 1
    done
    say "launch $n painted at t+${k}s (deviation $dev)"

    # C — the vaults opened, and before the optional work. This is the emulator's version of the
    # host-side ordering test in crates/fm-app, run on a real Android runtime.
    log=$(adbs logcat -d -s formicaria 2>/dev/null || true)
    printf '%s\n' "$log" > "$OUT/formicaria-$n.log"
    printf '%s\n' "$log" | grep -q 'vaults ready' \
        || fail "launch $n: no 'vaults ready' line — the shell never finished opening the vaults, or it never got to say so ($OUT/formicaria-$n.log)"
    ready_at=$(printf '%s\n' "$log" | grep -n 'vaults ready' | head -1 | cut -d: -f1)
    for later in 'ca-bundle' 'study agent'; do
        at=$(printf '%s\n' "$log" | grep -n "$later" | head -1 | cut -d: -f1 || true)
        if [ -n "$at" ] && [ "$at" -lt "$ready_at" ]; then
            fail "launch $n: '$later' was logged before the vaults were ready — slow work has moved back onto the critical path, which is what made the first open gray"
        fi
    done
    if printf '%s\n' "$log" | grep -q '^E/\|E formicaria'; then
        fail "launch $n: an error was logged at startup ($OUT/formicaria-$n.log)"
    fi
}
launch 1
launch 2
say "both launches painted — the 'blank first, fine second' signature is absent"

# A resume is not a launch: the process is alive, so the setup hook must NOT run again. If it does,
# every return to the app pays a full vault open and FTS rebuild — and two `App::load()`s in one
# process would index every vault twice.
adbs logcat -c >/dev/null 2>&1 || true
adbs shell input keyevent KEYCODE_HOME >/dev/null
sleep 5
adbs shell am start -n "$ACT" >/dev/null
sleep 5
boots=$(adbs logcat -d -s formicaria 2>/dev/null | grep -c 'vaults ready' || true)
[ "$boots" = "0" ] || fail "resume re-ran the startup hook ($boots 'vaults ready' lines) — a return to the app now pays a full vault open"
say "resume did not re-boot the shell"

# Backgrounded and then reclaimed by the OS: the task survives, the process does not. On a phone
# this is not an edge case, it is what every relaunch after a few hours actually is.
adbs shell am kill "$PKG" >/dev/null || true
sleep 2
launch 4
say "relaunch after an OS background-kill painted"

# ---------------------------------------------------------------------------
# 5) Closure. What the ordinary ways of leaving the app do to the process.
# ---------------------------------------------------------------------------
# **Force-stop, not Back.** Back looked like the interesting gesture — `TauriActivity` sets
# `handleBackNavigation = false`, so a Back that reaches the Activity finishes it, and tao's event
# loop then calls `process::exit` on the last window closing. But measured on the owner's phone
# 2026-07-31, Back did **not** end the app: this frontend pushes history (`NotePanel` handles
# `onpopstate`), so the WebView had somewhere to go back to and absorbed the key. A closure test whose
# first step may be a no-op tests nothing, so kill it outright — which is also what a swipe-away and an
# LMKD reclaim do.
#
# What must be true afterwards: nothing of ours is left holding memory. `PR_SET_PDEATHSIG` is the
# backstop that makes it pass; if this ever fails, that is what broke.
adbs shell am force-stop "$PKG" >/dev/null || true
sleep 4
# **Compare the parent, do not just count processes.** The first version called any surviving
# `llama-server` an orphan and duly cried wolf at two processes that were children of a perfectly
# healthy app. An orphan is one whose parent is gone — i.e. reparented to init (ppid 1).
left=""
for p in $(adbs shell "ps -A -o PID,NAME | grep -E 'llama-server|whisper-server' | grep -v grep | awk '{print \$1}'" 2>/dev/null | tr -d '\r'); do
    ppid=$(adbs shell "cat /proc/$p/stat 2>/dev/null | awk '{print \$4}'" 2>/dev/null | tr -d '\r')
    [ -n "$ppid" ] && [ "$ppid" = "1" ] && left="$left $p(ppid=1)"
done
[ -z "$left" ] && say "closure: no model process was orphaned" || fail "closure: orphaned model process holding RAM:$left"

# A relaunch after that kill must be as good as the first one. This is the loop the owner was
# actually stuck in, expressed as an assertion.
launch 3
say "relaunch after closure painted too"

# ---------------------------------------------------------------------------
# 6) Teardown. Emulator-only verbs, which is what the step-0 guard buys.
# ---------------------------------------------------------------------------
adbs shell am force-stop "$PKG" >/dev/null || true
adbs uninstall "$PKG" >/dev/null || true
say "PASS — artifacts in $OUT"
