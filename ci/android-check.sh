#!/bin/sh
# Does the Rust core actually build for a phone? — and prove it from the artifact.
#
# **Never trust the exit code here.** Inside a pixi env, `c-compiler`'s activation exports host
# `CC`/`CFLAGS`, so a misconfigured cross build can compile happily and emit x86-64 objects into
# the `aarch64-linux-android/` tree: green, wrong, and silent. `cargo check` cannot catch it
# either, because no workspace crate declares a `crate-type` and rlibs never link. So this
# builds for real and asserts the ELF machine of every vendored C blob.
#
# Opt-in, and NOT part of `pixi run ci`: it needs the NDK, and a contributor with no Android
# toolchain must still get a green gate (`docs/context/decisions.md`, ruling 3).
#
#   pixi run android-check              # aarch64 (the phone)
#   pixi run android-check x86_64-linux-android
set -eu

TARGET="${1:-aarch64-linux-android}"
case "$TARGET" in
    aarch64-linux-android) WANT="AArch64" ;;
    x86_64-linux-android) WANT="Advanced Micro Devices X86-64" ;;
    *) echo "android-check: unknown target $TARGET" >&2; exit 1 ;;
esac

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

# Refuse to run on a toolchain we did not provision. Without this the build can silently fall
# back to the host compiler, which is the exact failure this script exists to detect.
guard=$(eval "echo \${CC_$(echo "$TARGET" | tr - _)-}")
case "$guard" in
    "") echo "android-check: CC_$(echo "$TARGET" | tr - _) is unset — run inside 'pixi run -e android'" >&2; exit 1 ;;
    "$root"/.android/*) : ;;
    *) echo "android-check: the C compiler is not the one in .android/ ($guard)" >&2; exit 1 ;;
esac
[ -x "$guard" ] || { echo "android-check: $guard is not executable — run 'pixi run android-init'" >&2; exit 1; }

echo "[android] building fm-core (+native-git) for $TARGET"
cargo build -p fm-core --features native-git --target "$TARGET"

fail=0
# Every vendored C library, not just one: SQLite building for ARM says nothing about whether
# libgit2 or OpenSSL did, and each has its own build script and its own way to go wrong.
for pat in '*sqlite3.o' 'libgit2.a' 'libcrypto.a' 'libssl.a'; do
    f=$(find "target/$TARGET" -name "$pat" 2>/dev/null | head -1)
    if [ -z "$f" ]; then
        echo "  MISSING: no $pat under target/$TARGET — did the build really run?"
        fail=1
        continue
    fi
    case "$f" in
        *.a)
            tmp=$(mktemp -d)
            (cd "$tmp" && ar x "$root/$f" 2>/dev/null) || true
            obj=$(find "$tmp" -name '*.o' | head -1)
            machine=$(readelf -h "$obj" 2>/dev/null | sed -n 's/^ *Machine: *//p')
            rm -rf "$tmp"
            ;;
        *) machine=$(readelf -h "$f" 2>/dev/null | sed -n 's/^ *Machine: *//p') ;;
    esac
    if [ "$machine" = "$WANT" ]; then
        printf '  ok   %-14s %s\n' "$(basename "$f")" "$machine"
    else
        printf '  FAIL %-14s %s (wanted %s)\n' "$(basename "$f")" "${machine:-unreadable}" "$WANT"
        fail=1
    fi
done

if [ "$fail" -eq 0 ]; then
    echo "[android] $TARGET: the core and every vendored C library are $WANT."
fi
exit "$fail"
