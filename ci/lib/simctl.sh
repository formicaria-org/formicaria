# Shared `xcrun simctl` helpers, sourced by `ci/ios-smoke.sh` and `ci/ios-https-probe.sh`.
#
# **These parsers are where an iOS script is most likely to be wrong and least likely to look
# wrong**, which is why they live in one file with one self-test rather than in each caller.
# An `awk -F'[()]' '{print $2}'` reads "the second bracketed group": correct for
# `iPhone 17 (UDID) (Shutdown)`, and it returns the string `3rd generation` for
# `iPhone SE (3rd generation) (UDID) (Shutdown)` — a stock member of the default device set. That
# bug shipped here and was caught by review, having been about to hand `simctl bootstatus` a device
# *name*, 45 minutes into a paid job. Match the identifier by its **shape**, never by its position.
#
# `sh ci/ios-smoke.sh --self-test` exercises all three against captured `simctl` output. It needs no
# Mac, no simulator and no network, and `ci/checks.sh` runs it on every commit.

pick_udid()    { grep -m1 '^ *iPhone' | grep -oE '[0-9A-Fa-f]{8}-([0-9A-Fa-f]{4}-){3}[0-9A-Fa-f]{12}' || true; }

# `pick_device_by <hyphenated-substring>` — read `simctl list devices available` on stdin and echo
# the UDID of a device whose **name** matches. Empty when nothing does.
#
# **Why this exists, and it cost 20 minutes of a billed job on 2026-09-03.** The size sweep used to
# resolve a name against `simctl list devicetypes` and then `simctl create`. `devicetypes` is what
# Xcode *knows about*, not what this runner can *run*: `iPhone-SE-3rd-generation` is still listed
# under Xcode 26, so create succeeded — and then `bootstatus -b` sat on a device that cannot pair
# with any installed iOS 26 runtime. The guard above it only ever handled "no such device type".
# **`devices available` is the list that has already paired a device with a runtime**, so a match
# here is a device that boots, and a miss is a fast, honest skip.
#
# Names print with spaces (`iPhone 17 Pro`, `iPad Pro 13-inch (M5)`) while device *type* identifiers
# use hyphens (`…SimDeviceType.iPhone-17-Pro`). The name is hyphenated before matching so one
# spelling in `FM_IOS_SIZES` works whichever list a future caller reaches for.
#
# Exact name first, then the first substring match — deliberately, because `iPhone-17` is a prefix
# of `iPhone-17-Pro-Max` and "the last line that matched" would silently hand back the largest
# device every time you asked for the smallest.
device_index() {
    sed -n 's/^[[:space:]]*\(.*\) (\([0-9A-Fa-f]\{8\}-[0-9A-Fa-f]\{4\}-[0-9A-Fa-f]\{4\}-[0-9A-Fa-f]\{4\}-[0-9A-Fa-f]\{12\}\)) (.*)$/\1|\2/p' \
        | awk -F'|' '{ n=$1; gsub(/ /,"-",n); print n "\t" $2 }'
}
pick_device_by() {
    _want=$1
    _idx=$(device_index)
    _u=$(printf '%s\n' "$_idx" | awk -F'\t' -v w="$_want" '$1==w{print $2; exit}')
    [ -n "$_u" ] || _u=$(printf '%s\n' "$_idx" | awk -F'\t' -v w="$_want" 'index($1,w){print $2; exit}')
    printf '%s' "$_u"
}

# `boot_with_deadline <udid> <logfile> <seconds>` — `simctl bootstatus -b`, but bounded.
#
# **`bootstatus -b` has no timeout of its own**, and a simulator that will never come up is
# indistinguishable from one that is slow. On a billed 3-core runner that difference is money: the
# unbootable iPhone SE above turned a ~7 minute job into 20+. A skipped size costs a note; an
# unbounded wait costs the job.
boot_with_deadline() {
    _bd_udid=$1; _bd_log=$2; _bd_max=$3
    # **Background `xcrun` itself, not a subshell wrapping it.** `kill` must land on the process
    # that is actually blocked; killing a wrapper subshell leaves the real `bootstatus` orphaned and
    # still holding the device. Its output goes to a file, never to the job's stdout — an orphan on
    # the log pipe would keep the *step* alive, which is the symptom this whole function exists to
    # prevent.
    xcrun simctl bootstatus "$_bd_udid" -b >"$_bd_log" 2>&1 &
    _bd_pid=$!
    _bd_n=0
    while kill -0 "$_bd_pid" 2>/dev/null; do
        if [ "$_bd_n" -ge "$_bd_max" ]; then
            kill -TERM "$_bd_pid" 2>/dev/null || true
            sleep 1
            kill -KILL "$_bd_pid" 2>/dev/null || true
            # **Fire and forget.** Unsticking the device matters — a killed `bootstatus` leaves it
            # mid-boot and a later `simctl delete` on a booting device is itself slow — but
            # `simctl shutdown` has no timeout either, and a wedged CoreSimulator can hang it. An
            # escape hatch that can hang is not an escape hatch, and the runner is discarded anyway.
            ( xcrun simctl shutdown "$_bd_udid" >/dev/null 2>&1 & ) 2>/dev/null || true
            echo "(gave up waiting after ${_bd_max}s)" >> "$_bd_log"
            wait "$_bd_pid" 2>/dev/null || true
            return 1
        fi
        _bd_n=$((_bd_n + 1)); sleep 1
    done
    wait "$_bd_pid"
}
pick_runtime() { grep '^iOS ' | tail -1 | grep -oE 'com\.apple\.CoreSimulator\.SimRuntime\.[A-Za-z0-9._-]+' || true; }
pick_devtype() { grep 'iPhone' | tail -1 | grep -oE 'com\.apple\.CoreSimulator\.SimDeviceType\.[A-Za-z0-9._-]+' || true; }

# `simctl_device <bootlog-path>` — echo the UDID of a booted iPhone simulator, creating one only if
# the image has none. Sets `created_device=1` when it made one, so the caller can delete only what it
# created: a CI runner is thrown away either way, but on a Mac someone else owns, deleting a
# simulator its owner made would be the worse mistake.
#
# Callers must define `say` and `fail`.
simctl_device() {
    _bootlog=$1
    _udid=$(xcrun simctl list devices available | pick_udid)
    if [ -z "$_udid" ]; then
        say "no iPhone simulator available — creating one"
        # `runtimes available`, not `runtimes`: an unavailable runtime prints a trailing
        # "(unavailable, runtime profile not found … match policy)", so the last field of the
        # unfiltered list can be the word `policy)`.
        _runtime=$(xcrun simctl list runtimes available | pick_runtime)
        _devtype=$(xcrun simctl list devicetypes | pick_devtype)
        [ -n "$_runtime" ] && [ -n "$_devtype" ] \
            || fail "no available iOS runtime or iPhone device type on this machine"
        _udid=$(xcrun simctl create fm-smoke "$_devtype" "$_runtime") || fail "simctl create failed"
        created_device=1
    fi
    # `bootstatus -b` boots if needed and blocks until the system is actually up, which is the
    # difference between "the process started" and "the springboard will accept an install".
    #
    # **Bounded, generously.** This device came from `devices available`, so it is already paired
    # with a runtime and the pathology that cost 20 minutes on 2026-09-03 cannot arise here — but
    # "cannot arise" is what was believed about the sweep too, and an unbounded wait on a billed
    # runner has no upper cost. 600s is far above any observed boot and only fires on a hang.
    boot_with_deadline "$_udid" "$_bootlog" "${FM_IOS_BOOT_TIMEOUT_PRIMARY:-600}" \
        || fail "the simulator did not boot ($_bootlog)"
    echo "$_udid"
}
