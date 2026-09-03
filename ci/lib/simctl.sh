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
    xcrun simctl bootstatus "$_udid" -b >"$_bootlog" 2>&1 \
        || fail "the simulator did not boot ($_bootlog)"
    echo "$_udid"
}
