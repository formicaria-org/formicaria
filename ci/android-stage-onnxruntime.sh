#!/bin/sh
# Stage **ONNX Runtime** for android-arm64 into the APK's `jniLibs/arm64-v8a` as `libonnxruntime.so`
# — what the assistant's OCR opens on the phone (Myrme light, Myrme 0.7.0 / contract 1.6).
#
# WHY jniLibs: the OCR runs in-process (the app's own library loads it with dlopen), and an app's
# native-library dir is where Android lets it load code from. Myrme looks for it there under
# `fm_agent_run::host::BUNDLED_ONNXRUNTIME`; with it missing the assistant simply reads every page
# with the vision model.
#
# WHAT: Microsoft's own AAR from Maven Central, pinned by Myrme's catalogue
# (`runtime_url_onnxruntime_android_arm64` and its SHA-256), so formicaria keeps no copy of the
# version. Only `jni/arm64-v8a/libonnxruntime.so` is taken (~19 MB; it needs only libc/libm/libdl/
# liblog); the Java binding beside it is not.
#
# Pinned + arch-checked + idempotent. NOT in `pixi run ci` (it fetches ~30 MB); a depend-of
# `android-release` and `android-apk`, after `android-stage-runtime` (which leaves this file alone).
#
#   pixi run -e android android-stage-onnxruntime
set -eu

catalogue="$(sh "$(dirname -- "$0")/myrme-src.sh")/crates/fm-agent-run/models.toml"
key() { sed -n "s/^$1 = \"\(.*\)\"$/\1/p" "$catalogue"; }
URL=$(key runtime_url_onnxruntime_android_arm64)
SHA256=$(key runtime_sha256_onnxruntime_android_arm64)
[ -n "$URL" ] && [ -n "$SHA256" ] || {
    echo "android-stage-onnxruntime: no ONNX Runtime pinned in $catalogue (Myrme before contract 1.6?) — skipping, the OCR stays off"
    exit 0
}

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
dest="$root/mobile/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a"
stamp="$dest/.onnxruntime-version"
[ -d "$dest" ] || {
    echo "android-stage-onnxruntime: $dest is missing — run 'pixi run tauri android init' first" >&2
    exit 1
}
if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$SHA256" ] && [ -f "$dest/libonnxruntime.so" ]; then
    echo "android-stage-onnxruntime: already staged"
    exit 0
fi
for t in curl unzip readelf; do
    command -v "$t" >/dev/null 2>&1 || { echo "android-stage-onnxruntime: needs '$t' on PATH" >&2; exit 1; }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
echo "android-stage-onnxruntime: fetching $(basename "$URL")..."
curl -fL --retry 3 --progress-bar -o "$tmp/ort.aar" "$URL"
got=$( (sha256sum "$tmp/ort.aar" 2>/dev/null || shasum -a 256 "$tmp/ort.aar") | cut -d' ' -f1)
[ "$got" = "$SHA256" ] || { echo "android-stage-onnxruntime: checksum mismatch ($got)" >&2; exit 1; }
unzip -q -o -j "$tmp/ort.aar" 'jni/arm64-v8a/libonnxruntime.so' -d "$tmp"
m=$(readelf -h "$tmp/libonnxruntime.so" 2>/dev/null | sed -n 's/^ *Machine: *//p')
[ "$m" = "AArch64" ] || { echo "android-stage-onnxruntime: libonnxruntime.so is ${m:-unreadable}, wanted AArch64" >&2; exit 1; }
cp "$tmp/libonnxruntime.so" "$dest/libonnxruntime.so"
echo "$SHA256" > "$stamp"
echo "android-stage-onnxruntime: staged libonnxruntime.so ($(du -h "$dest/libonnxruntime.so" | cut -f1))"
