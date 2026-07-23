#!/bin/sh
# Cross-compile whisper.cpp's `whisper-server` for **android-arm64** and stage it into the APK's
# `jniLibs/arm64-v8a` as `libwhisper-server.so` — the phone's audio→transcript runtime.
#
# WHY build, not download (cf. android-stage-runtime.sh, which downloads llama.cpp's prebuilt server):
# whisper.cpp publishes NO prebuilt Android binary, so we cross-compile with the NDK. Proven on-device
# (docs/context/sessions/2026-07-23-audio-transcript-substrate.md): it loads a ggml model and
# transcribes at ~2x real-time on a Dimensity 7300.
#
# WHY STATIC (`BUILD_SHARED_LIBS=OFF`): a shared build emits its own `libggml*.so`/`libwhisper.so` whose
# names COLLIDE with (but differ in version from) the ones llama-server already stages here — bundling
# both would break one runtime. Static links them into a single self-contained binary (only libc/libm/
# libdl external), so `whisper-server` rides in `jniLibs` alone, renamed `lib*.so` the same way and for
# the same reason as `libllama-server.so` (Android only exec's from nativeLibraryDir, populated from
# jniLibs matching `lib*.so`).
#
# Needs the NDK (the `android` pixi env sets ANDROID_NDK_HOME) + cmake/ninja, pulled EPHEMERALLY via
# `pixi exec` so the toolchain stays lean (no permanent cmake dep). Pinned + arch-checked + idempotent.
# NOT in `pixi run ci` (it compiles C++); a depend-of `android-release`.
#
#   pixi run android-stage-whisper
set -eu

VER="v1.9.1"   # pinned; a bump is deliberate, like android-stage-runtime's llama pin.

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
dest="$root/mobile/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a"
stamp="$dest/.whisper-runtime-version"
src="$root/.android/whisper-src"   # gitignored, beside the NDK/SDK

[ -n "${ANDROID_NDK_HOME:-}" ] || { echo "android-stage-whisper: ANDROID_NDK_HOME unset — run under the 'android' pixi env" >&2; exit 1; }
[ -d "$dest" ] || { echo "android-stage-whisper: $dest missing — run 'pixi run tauri android init' first" >&2; exit 1; }
toolchain="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"
[ -f "$toolchain" ] || { echo "android-stage-whisper: no NDK cmake toolchain at $toolchain" >&2; exit 1; }

if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$VER" ] && [ -f "$dest/libwhisper-server.so" ]; then
    echo "android-stage-whisper: $VER already staged"
    exit 0
fi

# Source (shallow, pinned). Re-clone if the checkout is a different tag.
if [ ! -d "$src/.git" ] || [ "$(git -C "$src" describe --tags 2>/dev/null)" != "$VER" ]; then
    rm -rf "$src"
    echo "android-stage-whisper: cloning whisper.cpp $VER…"
    git clone --depth 1 --branch "$VER" https://github.com/ggml-org/whisper.cpp "$src"
fi

build="$src/build-android-arm64-static"
echo "android-stage-whisper: configuring (arm64, static, CPU, no openmp)…"
# `pixi exec` runs cmake/ninja from an ephemeral, cached env; it inherits ANDROID_NDK_HOME.
pixi exec --spec cmake --spec ninja -- cmake -S "$src" -B "$build" -G Ninja \
    -DCMAKE_TOOLCHAIN_FILE="$toolchain" \
    -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-24 \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=OFF \
    -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=ON \
    -DGGML_OPENMP=OFF
echo "android-stage-whisper: building whisper-server…"
pixi exec --spec cmake --spec ninja -- cmake --build "$build" --target whisper-server -j

bin="$build/bin/whisper-server"
[ -f "$bin" ] || { echo "android-stage-whisper: build produced no whisper-server" >&2; exit 1; }

# Strip (unstripped ~26 MB → ~800 KB), keeping the dynamic symbols the loader needs — the same
# discipline android-stage-runtime uses on the llama libs.
strip=$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip 2>/dev/null | head -1 || true)
[ -n "${strip:-}" ] && "$strip" --strip-unneeded "$bin" 2>/dev/null || true

cp "$bin" "$dest/libwhisper-server.so"
chmod 0644 "$dest/libwhisper-server.so"

# Must be arm64, or the app installs then SIGSEGVs on exec (the wrong-arch failure ci/android-check.sh
# guards the core against).
readelf=$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-readelf 2>/dev/null | head -1)
m=$("$readelf" -h "$dest/libwhisper-server.so" 2>/dev/null | sed -n 's/^ *Machine: *//p')
[ "$m" = "AArch64" ] || { echo "android-stage-whisper: staged binary is $m, not AArch64 — refusing." >&2; rm -f "$dest/libwhisper-server.so"; exit 1; }

echo "$VER" > "$stamp"
sz=$(du -h "$dest/libwhisper-server.so" | cut -f1)
echo "android-stage-whisper: staged whisper-server $VER ($sz, AArch64) → jniLibs/arm64-v8a/libwhisper-server.so"
