#!/bin/sh
# Stage the bundled model runtime — llama.cpp's prebuilt **android-arm64** `llama-server` + its shared
# libs — into the APK's `jniLibs/arm64-v8a`, so the on-device study agent has a binary it can `exec`.
#
# WHY jniLibs, and WHY renamed: Android's W^X/SELinux let an app `exec` only from its
# `nativeLibraryDir`, which is populated from `jniLibs`. So the server binary rides there as a
# `lib*.so` (Android only packages/extracts files matching that pattern) and is renamed to
# `libllama-server.so`; `mobile/.../src/agent.rs` finds the dir via `/proc/self/maps` and runs it. The
# shared libs are already `lib*.so` and go alongside it (the server dlopens them via LD_LIBRARY_PATH).
#
# Pinned + arch-checked + idempotent. NOT in `pixi run ci` (it fetches ~30 MB); run it before an
# android build — `android-apk`/`android-release` depend on it.
#
#   pixi run android-stage-runtime
set -eu

# Pinned llama.cpp release. A bump here is deliberate (matches the desktop runtime_url's discipline in
# agents/models.toml). android-arm64 is the phone; there is no prebuilt android-x86_64, so the emulator
# build simply has no runtime and the agent stays off there — the notebook still runs.
VER="b10081"
URL="https://github.com/ggml-org/llama.cpp/releases/download/$VER/llama-$VER-bin-android-arm64.tar.gz"

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
dest="$root/mobile/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a"
stamp="$dest/.llama-runtime-version"

[ -d "$dest" ] || {
    echo "android-stage-runtime: $dest is missing — run 'pixi run tauri android init' first" >&2
    exit 1
}
if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$VER" ] && [ -f "$dest/libllama-server.so" ]; then
    echo "android-stage-runtime: $VER already staged"
    exit 0
fi
for t in curl tar readelf; do
    command -v "$t" >/dev/null 2>&1 || { echo "android-stage-runtime: needs '$t' on PATH" >&2; exit 1; }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
echo "android-stage-runtime: fetching llama.cpp $VER (android-arm64)..."
curl -fL --retry 3 --progress-bar -o "$tmp/rt.tgz" "$URL"
tar xzf "$tmp/rt.tgz" -C "$tmp"
src="$tmp/llama-$VER"
[ -f "$src/llama-server" ] || { echo "android-stage-runtime: no llama-server in the tarball" >&2; exit 1; }

# Clear any prior staging (keep the tauri-generated app lib, and the whisper runtime staged by
# android-stage-whisper.sh — a separate, self-contained binary that must survive a llama re-stage).
find "$dest" -maxdepth 1 -name 'lib*.so' ! -name 'libformicaria_mobile_lib.so' ! -name 'libwhisper-server.so' -delete

# Copy the server (renamed so Android exec-extracts it) then the **transitive closure** of its NEEDED
# libs that ship in the tarball — resolving deps rather than guessing which to include (a guess just
# missed libllama-server-impl.so). System libs (libc/libm/libdl/liblog…) aren't in the tarball and are
# provided by Android, so they're skipped naturally.
cp "$src/llama-server" "$dest/libllama-server.so"
queue="$dest/libllama-server.so"
while [ -n "$queue" ]; do
    f=${queue%% *}
    case "$queue" in *' '*) queue=${queue#* } ;; *) queue= ;; esac
    for dep in $(readelf -d "$f" 2>/dev/null | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p'); do
        [ -f "$src/$dep" ] || continue      # a system lib → Android provides it
        [ -f "$dest/$dep" ] && continue     # already staged
        cp "$src/$dep" "$dest/$dep"
        queue="$queue $dest/$dep"
    done
done
# ggml picks a CPU backend at runtime via dlopen (not a NEEDED entry, so the closure won't find it) —
# ship them all; the device loads the one matching its ISA.
for so in "$src"/libggml-cpu-android_*.so; do [ -f "$so" ] && cp "$so" "$dest/"; done

# The prebuilt libs ship unstripped — ~235 MB staged, most of it symbol tables the device never needs.
# Strip with the NDK's llvm-strip (keeps the dynamic symbols the loader uses; drops debug/local ones),
# which is the difference between a ~40 MB and a ~235 MB payload — decisive for the low-resource goal.
strip=$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip 2>/dev/null | head -1 || true)
if [ -n "${strip:-}" ]; then
    for f in "$dest"/lib*.so; do
        [ "$(basename "$f")" = libformicaria_mobile_lib.so ] && continue
        "$strip" --strip-unneeded "$f" 2>/dev/null || true
    done
    echo "android-stage-runtime: stripped staged libs with $(basename "$strip")"
else
    echo "android-stage-runtime: WARNING no llvm-strip found — libs left unstripped (large APK)" >&2
fi

# Every staged blob must be arm64, or the app installs and then SIGSEGVs on exec — the silent
# wrong-arch failure ci/android-check.sh guards the core against.
fail=0
for f in "$dest"/lib*.so; do
    [ "$(basename "$f")" = libformicaria_mobile_lib.so ] && continue
    m=$(readelf -h "$f" 2>/dev/null | sed -n 's/^ *Machine: *//p')
    if [ "$m" = "AArch64" ]; then
        printf '  ok   %s\n' "$(basename "$f")"
    else
        printf '  FAIL %s: %s (wanted AArch64)\n' "$(basename "$f")" "${m:-unreadable}"
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo "android-stage-runtime: a staged lib is not arm64 — refusing." >&2; exit 1; }

# The renamed server binary can only be `exec`'d if Android EXTRACTS it to nativeLibraryDir; modern
# packaging loads libs straight from the APK instead (extractNativeLibs=false), leaving nothing to
# exec. `useLegacyPackaging = true` restores extraction. gen/android is regenerated by `tauri android
# init`, so patch the generated gradle idempotently here — one `android-stage-runtime` makes the
# project ready, libs + flag together.
gradle="$root/mobile/src-tauri/gen/android/app/build.gradle.kts"
if [ -f "$gradle" ] && ! grep -q useLegacyPackaging "$gradle"; then
    tmpf=$(mktemp)
    awk '1; /^android \{/ && !done { print "    packaging { jniLibs { useLegacyPackaging = true } }"; done=1 }' \
        "$gradle" > "$tmpf" && mv "$tmpf" "$gradle"
    echo "android-stage-runtime: set useLegacyPackaging=true (extract native libs) in build.gradle.kts"
fi

echo "$VER" > "$stamp"
n=$(find "$dest" -maxdepth 1 -name 'lib*.so' ! -name 'libformicaria_mobile_lib.so' | wc -l | tr -d ' ')
echo "android-stage-runtime: staged $VER — $n libs into arm64-v8a (all AArch64), extract-native-libs on."
