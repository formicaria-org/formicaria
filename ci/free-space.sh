#!/bin/sh
# Give the disk back — `pixi run free-space`. The owner's laptop fills up again and again
# (2026-10-23: 1.9 GB left, a clean build failed with "No space left on device"; the owner: *"this is
# a recurring theme"*). Every path below is **regenerable build output** and nothing else:
#
#   target/debug                         ~38 GB  the next `pixi run test`/`ci` rebuilds it (minutes)
#   ../myrme/target/debug                ~15 GB  the same, in the sibling Myrme checkout
#   mobile/src-tauri/target              ~12 GB  the phone shell's builds (every Android ABI)
#   mobile/src-tauri/gen/android/app/build ~4 GB Gradle's output; the project around it is kept
#   ../myrme/.fm/<tag>                   ~0.4 GB each formicaria checkout `integration` made for a
#                                                tag Myrme no longer pins
#
# **Kept on purpose:** `target/release` — the desktop icon runs `target/release/fm-serve` and the
# assistant runs `target/release/agent-serve`, so deleting it breaks the running app and the next
# launch. The pixi environments, caches and node_modules are kept too: they are small next to the
# targets and slow to fetch again.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
myrme="$root/../myrme"
before=$(df -h "$root" | awk 'NR==2 {print $4}')

rm -rf "$root/target/debug" \
    "$root/mobile/src-tauri/target" \
    "$root/mobile/src-tauri/gen/android/app/build"
if [ -d "$myrme" ]; then
    rm -rf "$myrme/target/debug"
    # The formicaria Myrme's integration test builds against is the one `fm-fetch` is pinned at.
    pinned=$(sed -n 's/^fm-fetch = { git = "[^"]*", tag = "\([^"]*\)".*/\1/p' \
        "$myrme/crates/fm-agent-run/Cargo.toml" 2>/dev/null || true)
    for d in "$myrme"/.fm/*/; do
        [ -d "$d" ] || continue
        [ "$(basename "$d")" = "$pinned" ] || rm -rf "$d"
    done
fi

after=$(df -h "$root" | awk 'NR==2 {print $4}')
echo "free-space: $before free before, $after free now (target/release kept: the app runs from it)"
