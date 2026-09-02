#!/bin/sh
# Build, align and sign a release APK for the phone.
#
# Release rather than debug because debug is ~180 MB of unstripped symbols for libgit2 and
# OpenSSL against ~19 MB here — a 9x difference you feel when sideloading.
#
# **Signing is deliberately done here rather than in Gradle.** Tauri regenerates
# `gen/android/` on every `android init`, so a `signingConfig` written into its build files is
# lost the next time anything about the identifier or config changes. Signing the finished
# artifact with `apksigner` survives that, and keeps the key entirely out of the repo.
#
# The keystore lives in ~/.config/formicaria/, mode 0600, NOT in the project: a signing key in
# a git working tree is a signing key that eventually gets committed. **Back it up** — losing
# it means the app can never be updated in place, only uninstalled and reinstalled.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
props="$HOME/.config/formicaria/android-release.properties"
target="${1:-aarch64}"

[ -f "$props" ] || {
    echo "android-release: no keystore at $props" >&2
    echo "  Create one with:" >&2
    echo "    keytool -genkeypair -v -keystore ~/.config/formicaria/android-release.keystore \\" >&2
    echo "      -alias formicaria -keyalg RSA -keysize 4096 -validity 10000" >&2
    echo "  then write storeFile/storePassword/keyAlias/keyPassword into $props (chmod 600)." >&2
    exit 1
}

# The launcher icons live in the regenerated tree, so they must be (re)written after any
# `android init` — otherwise the app ships Tauri's default icon and nobody notices until it is
# on a home screen. Cheap, idempotent, and the reason this runs every time.
# **A mobile-specific source, not the favicon.** Android masks launcher icons to a circle or
# squircle and only the inner ~66% is guaranteed to survive, so artwork drawn edge to edge loses
# its extremities — on a real phone the circle cut the ant's antennae and outer legs. The
# favicon fills its canvas because a browser tab is a 16px square with no mask and wants every
# pixel; the two requirements are opposite, so they are two files. See `mobile/icon-source.svg`.
( cd mobile && pnpm exec tauri icon ./icon-source.svg >/dev/null )

# **Three env vars Tauri needs that the pixi feature does not supply**, and their absence is
# not a clear error: `tauri android build` fails with "failed to ensure Android environment:
# Skipping Android Studio command line tools installation", which names neither the variable
# that is missing nor the fact that the SDK is present and complete.
#
#   NDK_HOME   — the feature exports ANDROID_NDK_HOME (what cargo and `cc` read); Tauri reads
#                this one. Both must be set, and they are the same path.
#   JAVA_HOME  — pinned to the environment's JDK 21. Gradle 8.14 refuses conda's default 25
#                with "Unsupported class file major version 69", naming neither Java nor Gradle.
#   PATH       — our `rustup` shim first. Tauri shells out to `rustup target add`, and this
#                project has no rustup (targets come from conda-forge, pinned in pixi.lock);
#                the shim verifies rather than pretends.
#
# `android-apk` sets all three inline and this script did not, which is why the debug build
# worked and the release build did not.
# **The version the app reports about itself**, baked in exactly as the desktop release workflow
# does it: `fm-app` reads `option_env!("FM_VERSION")` and falls back to `dev`. Without this the
# phone said `dev` while the desktop of the same release said `v0.3.1` — two builds of one release
# disagreeing about what they are, which is the confusion the version display exists to end.
#
# Taken from the tag on HEAD when there is one, so cutting a release and building the phone need
# not be kept in step by hand; `FM_VERSION=... pixi run android-release` still wins for a one-off.
: "${FM_VERSION:=$(git -C "$root" describe --tags --exact-match 2>/dev/null || echo dev)}"
export FM_VERSION
echo "android-release: building as FM_VERSION=$FM_VERSION"

( cd mobile \
    && PATH="$root/.android/bin:$PATH" \
       NDK_HOME="${NDK_HOME:-$ANDROID_NDK_HOME}" \
       JAVA_HOME="${JAVA_HOME:-$CONDA_PREFIX/lib/jvm}" \
       FM_VERSION="$FM_VERSION" \
       pnpm exec tauri android build --apk --target "$target" )

bt=$(ls -d "$root"/.android/sdk/build-tools/*/ | sort -V | tail -1)
unsigned="$root/mobile/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"
out="$root/mobile/formicaria-$target.apk"

# `-P 16`: Google states 16 KB page size as a *device* property — without it the app will not
# run on future Android releases at all. Verified afterwards rather than assumed, because
# Tauri sets its own CARGO_TARGET_*_RUSTFLAGS and overwrites ours.
"${bt}zipalign" -P 16 -f 4 "$unsigned" "$out"

# Read the four values rather than sourcing the file: `. <(...)` is a bashism that dies under
# dash, and sourcing a properties file would *execute* whatever is in it. Parsing is both
# portable and the safer of the two for a file holding a signing password.
storeFile=$(sed -n 's/^storeFile=//p' "$props")
storePassword=$(sed -n 's/^storePassword=//p' "$props")
keyAlias=$(sed -n 's/^keyAlias=//p' "$props")
keyPassword=$(sed -n 's/^keyPassword=//p' "$props")
"${bt}apksigner" sign --ks "$storeFile" --ks-pass "pass:$storePassword" \
    --key-pass "pass:$keyPassword" --ks-key-alias "$keyAlias" "$out" 2>/dev/null
"${bt}apksigner" verify "$out" >/dev/null 2>&1 || { echo "android-release: signature did not verify" >&2; exit 1; }

# Assert the artifact, not the flag: this is the check that catches Tauri having replaced the
# alignment rustflag.
tmp=$(mktemp -d); unzip -o -j "$out" "lib/*/*.so" -d "$tmp" >/dev/null 2>&1
for so in "$tmp"/*.so; do
    align=$(readelf -lW "$so" 2>/dev/null | awk '/LOAD/{print $NF; exit}')
    [ "$align" = "0x4000" ] || echo "  WARNING: $(basename "$so") LOAD align $align, not 16 KB" >&2
done
rm -rf "$tmp"

printf 'android-release: %s (%.0f MB), signed and 16 KB aligned\n' "$out" \
    "$(stat -c%s "$out" | awk '{print $1/1048576}')"
