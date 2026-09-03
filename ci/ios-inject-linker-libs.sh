#!/bin/sh
# Tell Xcode to link **zlib and iconv**, without which the iOS app cannot link at all.
#
# ## Why this is needed
#
# The Rust side is built as `crate-type = ["staticlib"]` (`mobile/src-tauri/Cargo.toml`), and
# **rustc cannot put a system dylib inside a static archive.** libgit2's own C *objects* are in
# `libapp.a` — the `cc` crate emits `cargo:rustc-link-lib=static=git2`, and `+bundle` is the default,
# so they are packed in. But `libgit2-sys`'s build script also emits a plain
# `cargo:rustc-link-lib=z` and, for every Apple target, `cargo:rustc-link-lib=iconv`. Those are
# instructions for whoever performs the final link — and here that is **Xcode, not cargo**, which
# never sees them. Measured, rung 2 second firing (run 91351661719):
#
#     Undefined symbols for architecture arm64:
#       "_inflate", "_inflateInit_", "_deflate", "_crc32"   <- zlib
#       "_iconv", "_iconv_open", "_iconv_close"             <- libiconv
#         referenced from libapp.a(zstream.o, indexer.o, fs_path.o)
#     ** ARCHIVE FAILED **
#
# **Android never hits this** because it builds a `cdylib`, which rustc links itself and where those
# same directives are honoured. It is a consequence of the output shape, not of iOS.
#
# ## Why it is done *here*, and not in three tidier-looking places
#
# - **Not `tauri.conf.json`'s `bundle > iOS > frameworks`.** Read the CLI: entries with no extension
#   become `- sdk: {{this}}.framework`, and any other extension is treated as a *vendored* framework
#   path relative to `src-tauri`. There is no route from that field to a bare SDK library. The
#   template does have an `ios-vendor-sdks` bucket that renders `- sdk: …` — but tauri-cli never
#   populates it (`crates/tauri-cli/src/mobile/ios/mod.rs` sets only `frameworks` and
#   `vendor_frameworks`), so it is unreachable from our config.
# - **Not `cargo:rustc-link-arg` or `.cargo/config.toml`.** The Cargo book is explicit that
#   `rustc-link-arg` applies to binaries, examples, tests, benches and **cdylibs** — staticlib is
#   not in the list.
# - **Not by vendoring.** `libz-sys`'s `static` feature would bundle zlib and remove half the
#   problem, but `libgit2-sys` emits `rustc-link-lib=iconv` unconditionally on Apple targets and has
#   no feature to avoid it; Apple ships `libiconv` only as a system library. So iconv must be fixed
#   Xcode-side regardless, and doing both there is one mechanism instead of two — the second of
#   which would be a platform-gated Cargo feature edge, the exact shape that has bitten
#   `crates/fm-core/Cargo.toml` twice already.
#
# ## Why it must run every time, between init and build
#
# `gen/apple` is generated and **gitignored** — *"it embeds absolute paths, so committing it would be
# committing this machine"* — so a fresh checkout never has our edit. And `tauri ios build` never
# runs XcodeGen: only `tauri ios init` does. So this script must both patch `project.yml` *and*
# re-run `xcodegen generate` itself, because nothing else will. Same shape as
# `ci/android-inject-service.sh`, for the same reason.
#
#   sh ci/ios-inject-linker-libs.sh [path/to/project.yml]
#
# `FM_SKIP_XCODEGEN=1` patches without regenerating — used by the self-test in `ci/checks.sh`,
# which runs this against a fixture on Linux, where there is no Xcode.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
spec="${1:-$root/mobile/src-tauri/gen/apple/project.yml}"

[ -f "$spec" ] || {
    echo "ios-inject-linker-libs: $spec missing — run 'tauri ios init' first" >&2
    exit 1
}

# **The anchor is the last dependency the template emits.** `- sdk: WebKit.framework` is
# unconditional and sits immediately before `preBuildScripts:`, so appending after it lands inside
# the app target's `dependencies:` list with nothing conditional in between. Anchoring on a
# *structural* certainty rather than on line numbers or on a `dependencies:` key that other targets
# also have. Six-space indentation matches the template exactly.
#
# XcodeGen resolves a bare `.tbd` under `sdk:` to `usr/lib/<name>` with the SDKROOT source tree —
# i.e. precisely the "Link Binary With Libraries → libz.tbd" entry Xcode itself would create.
if grep -q '^      - sdk: libz\.tbd$' "$spec" && grep -q '^      - sdk: libiconv\.tbd$' "$spec"; then
    echo "ios-inject-linker-libs: already applied to $spec"
else
    awk '
        { print }
        /^      - sdk: WebKit\.framework$/ && !done {
            print "      - sdk: libz.tbd"
            print "      - sdk: libiconv.tbd"
            done = 1
        }
        END { if (!done) exit 3 }
    ' "$spec" > "$spec.new" || {
        rm -f "$spec.new"
        echo "ios-inject-linker-libs: no '- sdk: WebKit.framework' line in $spec." >&2
        echo "  That anchor comes from tauri-cli's own project.yml template and is unconditional," >&2
        echo "  so if it is gone the template changed. Re-read it before guessing a new anchor:" >&2
        echo "  crates/tauri-cli/templates/mobile/ios/project.yml at the pinned CLI version." >&2
        exit 1
    }
    mv "$spec.new" "$spec"
    echo "ios-inject-linker-libs: added libz.tbd + libiconv.tbd to $spec"
fi

# **Verify, do not assume.** A patch that silently landed in the wrong block would fail 6 minutes
# later as the same linker error, which is the expensive way to learn it.
for lib in libz.tbd libiconv.tbd; do
    grep -q "^      - sdk: $lib\$" "$spec" || {
        echo "ios-inject-linker-libs: $lib is not in $spec after patching — refusing to continue" >&2
        exit 1
    }
done

# `[ … ] && exit 0` would return 1 when the test is false and `set -e` would end the script right
# here — the classic form of this bug. An explicit `if` has no such edge.
if [ "${FM_SKIP_XCODEGEN:-0}" = "1" ]; then
    exit 0
fi

# `tauri ios build` does not run XcodeGen, so the edit above reaches the .xcodeproj only if we
# regenerate here. `xcodegen` is on PATH because `tauri ios init` installs it via Homebrew.
command -v xcodegen >/dev/null 2>&1 || {
    echo "ios-inject-linker-libs: xcodegen is not on PATH. 'tauri ios init' installs it (brew);" >&2
    echo "  if init was skipped because gen/apple already existed, install it or delete gen/apple." >&2
    exit 1
}
( cd "$(dirname "$spec")" && xcodegen generate --spec "$(basename "$spec")" >/dev/null ) || {
    echo "ios-inject-linker-libs: xcodegen generate failed for $spec" >&2
    exit 1
}

# The .xcodeproj is what xcodebuild actually reads; check the libraries survived the regeneration.
pbx=$(find "$(dirname "$spec")" -maxdepth 1 -name '*.xcodeproj' -type d | head -1)
if [ -n "$pbx" ] && [ -f "$pbx/project.pbxproj" ]; then
    for lib in libz.tbd libiconv.tbd; do
        grep -q "$lib" "$pbx/project.pbxproj" || {
            echo "ios-inject-linker-libs: regenerated $pbx but it does not reference $lib." >&2
            echo "  The project.yml edit did not reach the Xcode project; the link would fail." >&2
            exit 1
        }
    done
    echo "ios-inject-linker-libs: $(basename "$pbx") references both libraries"
fi
