#!/bin/sh
# Give the iOS app the **permission usage descriptions** it cannot run without.
#
# ## Why this is not cosmetic
#
# On iOS a missing `NS*UsageDescription` is **not** a denied permission. The system **terminates the
# app** the moment the API is touched. Our `＋ Media` menu (`ui/src/lib/NotePanel.svelte`) is
# rendered on every note being edited, with no platform gate, and offers four things that reach
# permission-gated hardware:
#
#   Record audio    -> getUserMedia({audio:true})           -> NSMicrophoneUsageDescription
#   Take a photo    -> <input capture="environment">        -> NSCameraUsageDescription
#   Record a video  -> camera + microphone                  -> both of the above
#   From the library / Any file -> the photo picker         -> NSPhotoLibraryUsageDescription
#
# So without this file, **the first tester to edit a note and tap ＋ Media crashes the app** — and
# it looks like our app being broken, because it is. Found by audit on 2026-09-03, before anyone
# was asked to install anything.
#
# `NSLocalNetworkUsageDescription` is here for a different failure: without it, *Share with a nearby
# device* does not crash, it fails **silently**, because iOS 14+ gates every local-network
# connection behind it.
#
# **`NSBonjourServices` is deliberately absent.** An earlier note in `known-issues.md` said it was
# needed; reading the code says otherwise — `fm-serve/src/share.rs:357-370` advertises
# `<hostname>.local` and the phone *resolves* that name. The key is required for **browsing**
# services (`NWBrowser`/`NSNetServiceBrowser`), which nothing here does, and inventing a service
# type to satisfy a key we do not need is exactly the guess this project keeps paying for.
#
# ## Why here, and not in a tidier place
#
# - **Not `tauri.conf.json`.** The template *does* have a hook — `{{#each apple.plist-pairs}}` right
#   after `CFBundleVersion` — but tauri-cli never populates it. Read
#   `crates/tauri-cli/src/mobile/ios/mod.rs` at the pinned version: the only `plist` it touches is
#   `export_options_plist`, which is the signing export, not `Info.plist`. Same dead end as
#   `ios-vendor-sdks`, and for the same reason.
# - **Not by hand in `gen/apple`.** That directory is generated and gitignored, so a fresh checkout
#   never carries the edit.
#
# ## Why it is a second script rather than part of `ios-inject-linker-libs.sh`
#
# That one is named for its job and `decisions.md` — which is append-only — refers to it by name.
# Renaming it to cover a second job would leave dated entries pointing at a file that no longer
# exists. So: two scripts, one honest name each, and **`xcodegen` still runs exactly once** —
# `ios_project_ready` calls this one with `FM_SKIP_XCODEGEN=1` and lets the linker-libs script
# regenerate after both patches are in.
#
#   sh ci/ios-inject-plist.sh [path/to/project.yml]
#
# `FM_SKIP_XCODEGEN=1` patches without regenerating — used above, and by the self-test in
# `ci/checks.sh`, which runs this against a fixture on Linux where there is no Xcode.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
spec="${1:-$root/mobile/src-tauri/gen/apple/project.yml}"

[ -f "$spec" ] || {
    echo "ios-inject-plist: $spec missing — run 'tauri ios init' first" >&2
    exit 1
}

# **The anchor is the last unconditional property the template emits.** `CFBundleVersion` sits at
# the end of `info: properties:`, immediately before the (never-populated) `plist-pairs` loop, so
# appending after it lands inside the properties map with nothing conditional in between. Anchored
# on a structural certainty rather than on line numbers — the same reasoning, and the same eight
# spaces of indentation, as the template itself.
#
# **The wording is shown to the user in the system prompt**, so each one says what is accessed and
# when. A vague reason is both a worse prompt and an App Store rejection, and this is the text a
# tester reads before deciding whether to trust the app.
KEYS='        NSMicrophoneUsageDescription: formicaria records audio only when you tap Record audio in a note. The recording is saved into that notebook, on this phone.
        NSCameraUsageDescription: formicaria opens the camera only when you choose Take a photo or Record a video in a note. What you capture is saved into that notebook, on this phone.
        NSPhotoLibraryUsageDescription: formicaria opens your photo library only when you pick a file to attach to a note. Nothing is read from it otherwise.
        NSLocalNetworkUsageDescription: formicaria connects to a computer on your network when you pair this phone with one, so your notes sync directly rather than through the internet.'

if grep -q '^        NSMicrophoneUsageDescription: ' "$spec"; then
    echo "ios-inject-plist: already applied to $spec"
else
    # **Through the environment, not `awk -v`** (2026-09-09). `$KEYS` is four lines, and POSIX does
    # not allow a literal newline in a `-v` assignment: the value is processed for escape sequences,
    # and a raw newline is a syntax error. gawk, mawk and busybox awk all accept it anyway, so this
    # ran green here and on every runner image until `macos-latest` moved to macOS 26, whose awk
    # enforces the rule — `awk: newline in string   NSMicrophone... at source line 1`, three times,
    # then a failed job. `ENVIRON` is not escape-processed and carries newlines untouched.
    if KEYS="$KEYS" awk '
        { print }
        /^        CFBundleVersion: / && !done { print ENVIRON["KEYS"]; done = 1 }
        END { if (!done) exit 3 }
    ' "$spec" > "$spec.new"; then
        mv "$spec.new" "$spec"
        echo "ios-inject-plist: added 4 usage descriptions to $spec"
    else
        # **Two failures, told apart** — because conflating them is what made the first one cost a
        # job to understand. `exit 3` is ours and means the anchor is genuinely gone; anything else
        # is awk itself refusing, and the old code reported both as "the template changed", sending
        # the reader to re-read tauri-cli's template when the template was fine.
        rc=$?
        rm -f "$spec.new"
        if [ "$rc" = 3 ]; then
            echo "ios-inject-plist: no '        CFBundleVersion: ' line in $spec." >&2
            echo "  That anchor comes from tauri-cli's own project.yml template, inside the app" >&2
            echo "  target's 'info: properties:' map, and is unconditional. If it is gone the template" >&2
            echo "  changed — re-read it before guessing a new anchor:" >&2
            echo "  crates/tauri-cli/templates/mobile/ios/project.yml at the pinned CLI version." >&2
        else
            echo "ios-inject-plist: awk failed (exit $rc) reading $spec — this is awk refusing, not" >&2
            echo "  a missing anchor. Read its message above before touching the anchor or template." >&2
        fi
        exit 1
    fi
fi

# **Verify, do not assume.** A key that landed in the wrong block would produce a build that runs
# right up until someone taps ＋ Media, which is the expensive way to find out.
for k in NSMicrophoneUsageDescription NSCameraUsageDescription \
         NSPhotoLibraryUsageDescription NSLocalNetworkUsageDescription; do
    grep -q "^        $k: " "$spec" || {
        echo "ios-inject-plist: $k is not in $spec after patching — refusing to continue" >&2
        exit 1
    }
done

if [ "${FM_SKIP_XCODEGEN:-0}" = "1" ]; then
    exit 0
fi

command -v xcodegen >/dev/null 2>&1 || {
    echo "ios-inject-plist: xcodegen is not on PATH. 'tauri ios init' installs it (brew)." >&2
    exit 1
}
( cd "$(dirname "$spec")" && xcodegen generate --spec "$(basename "$spec")" >/dev/null ) || {
    echo "ios-inject-plist: xcodegen generate failed for $spec" >&2
    exit 1
}
