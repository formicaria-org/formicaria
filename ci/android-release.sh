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

# ===============================================================================================
# **Building and signing are separable, so the key need not be present while the build runs.**
#
# `FM_ANDROID_STAGE` — `build`, `sign`, or unset for both. Unset is the default and is what a
# person types locally; the split exists for CI.
#
# **Why.** Building this APK compiles the whole Rust workspace — hundreds of crates, each free to
# run a `build.rs` — plus npm and a cmake cross-compile of whisper.cpp. Every one of those runs as
# you, on the machine where the keystore sits, and any of them could simply read the file. That is
# the largest exposure in the whole signing story, and it is not addressed by anything to do with
# how the secret is *passed*: the key is on disk because `apksigner` needs it there.
#
# Split, the release workflow builds with **no secret in the job at all**, and signs in a second
# job that runs `apksigner` and nothing else. Hundreds of third-party build scripts able to read
# the key becomes one Google-published binary. (`decisions.md`, 2026-09-09.)
#
# It does nothing for a *local* build, where the same key sits on the same disk while the same
# crates compile. That is worth knowing rather than pretending otherwise — but the runner is the
# machine that runs unreviewed code on a schedule, so it is the one worth separating.
# ===============================================================================================
stage="${FM_ANDROID_STAGE:-both}"
case "$stage" in
    build|sign|both) ;;
    *) echo "android-release: FM_ANDROID_STAGE must be build, sign or unset (got '$stage')" >&2; exit 1 ;;
esac
do_build=yes; do_sign=yes
[ "$stage" = sign ] && do_build=no
[ "$stage" = build ] && do_sign=no

# **Only the signing stage needs a key.** Checked before any work rather than after the build, so
# a missing keystore costs a second instead of a full compile.
if [ "$do_sign" = yes ] && [ ! -f "$props" ]; then
    echo "android-release: no keystore at $props" >&2
    echo "  Create one with:" >&2
    echo "    keytool -genkeypair -v -keystore ~/.config/formicaria/android-release.keystore \\" >&2
    echo "      -alias formicaria -keyalg RSA -keysize 4096 -validity 10000" >&2
    echo "  then write storeFile/storePassword/keyAlias/keyPassword into $props (chmod 600)." >&2
    exit 1
fi

# The launcher icons live in the regenerated tree, so they must be (re)written after any
# `android init` — otherwise the app ships Tauri's default icon and nobody notices until it is
# on a home screen. Cheap, idempotent, and the reason this runs every time.
# **A mobile-specific source, not the favicon.** Android masks launcher icons to a circle or
# squircle and only the inner ~66% is guaranteed to survive, so artwork drawn edge to edge loses
# its extremities — on a real phone the circle cut the ant's antennae and outer legs. The
# favicon fills its canvas because a browser tab is a 16px square with no mask and wants every
# pixel; the two requirements are opposite, so they are two files. See `mobile/icon-source.svg`.
if [ "$do_build" = yes ]; then
( cd mobile && pnpm exec tauri icon ./icon-source.svg >/dev/null )
# **…and then put back the one file it rewrites for no reason.** `tauri icon` emits byte-identical
# PNGs, but its `.icns` packer does not: the same 44 312 bytes come back with ~43 400 of them
# reordered, every run. `known-issues.md` has said so since 2026-08-30 — *"do not sweep it into a
# commit with `git add -A` without looking"* — and naming the hazard was not enough: it rode into
# `9a48a1f` that way, and into two more commits on 2026-09-09 while the very header you are reading
# was being corrected. A trap that is documented and still sprung is a trap the script should
# disarm.
#
# **Only `icon.icns`, deliberately.** It is the one file measured nondeterministic, and nothing here
# consumes it — `bundle.icon` is `icons/icon.png` alone, and this app ships to Android and iOS.
# Restoring the whole icon directory would silently revert a real change to `icon-source.svg`, which
# is the opposite mistake; the PNGs stay as generated, so an edited source still shows up.
#
# Whether these should be generated at build time rather than tracked is still the open design call
# `known-issues.md` records. This does not settle it — it stops the churn reaching a commit.
if git -C "$root" ls-files --error-unmatch mobile/src-tauri/icons/icon.icns >/dev/null 2>&1; then
    git -C "$root" checkout -- mobile/src-tauri/icons/icon.icns 2>/dev/null || true
fi
fi

# **Three env vars Tauri needs that the pixi feature does not supply**, and their absence is
# not a clear error: `tauri android build` fails with "failed to ensure Android environment:
# Skipping Android Studio command line tools installation", which names neither the variable
# that is missing nor the fact that the SDK is present and complete.
#
#   NDK_HOME   — the feature exports ANDROID_NDK_HOME (what cargo and `cc` read); Tauri reads
#                this one. Both must be set, and they are the same path.
#   JAVA_HOME  — pinned to the environment's JDK 21. Gradle 8.14 refuses conda's default 25
#                with "Unsupported class file major version 69", naming neither Java nor Gradle.
#   PATH       — our `rustup` shim first (`ci/bin`). Tauri shells out to `rustup target add`, and
#                this project has no rustup (targets come from conda-forge, pinned in pixi.lock);
#                the shim verifies rather than pretends. It lived in the gitignored `.android/bin`
#                until 2026-09-03, i.e. on one machine only — a fresh clone could not build.
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
echo "android-release: stage=$stage, FM_VERSION=$FM_VERSION"

if [ "$do_build" = yes ]; then
( cd mobile \
    && PATH="$root/ci/bin:$PATH" \
       NDK_HOME="${NDK_HOME:-$ANDROID_NDK_HOME}" \
       JAVA_HOME="${JAVA_HOME:-$CONDA_PREFIX/lib/jvm}" \
       FM_VERSION="$FM_VERSION" \
       pnpm exec tauri android build --apk --target "$target" )
fi

bt=$(ls -d "$root"/.android/sdk/build-tools/*/ | sort -V | tail -1)
unsigned="$root/mobile/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"

# **Named the way it is attached, not the way it is built.** The desktop archives arrive from
# `release.yml` as `formicaria-v0.5.0-linux-x86_64.tar.gz`; the phone's APK is attached to the same
# release by hand, and until 2026-09-09 this wrote `formicaria-aarch64.apk` — a name with no version
# in it and an ABI spelling nothing else in the project uses. So every release since v0.2.0 was
# renamed by hand to `formicaria-<version>-android-arm64.apk` before being uploaded, five times,
# and a rename done by hand before publishing is a rename that is eventually forgotten — leaving a
# release carrying an asset that cannot say which version it is.
#
# `aarch64` is the Rust target triple's word; `arm64` is Android's and the one the other five files
# already use. The `case` rather than a `&&` substitution because `set -e` turns a failed test at
# the end of an `&&` chain into an exit.
case "$target" in
    aarch64) abi=arm64 ;;
    *) abi=$target ;;
esac
# `$FM_VERSION` is `v0.5.0` on a tagged commit and `dev` otherwise, so a scratch build is named
# `formicaria-dev-android-arm64.apk` and can never be mistaken for a release artifact.
out="$root/mobile/formicaria-$FM_VERSION-android-$abi.apk"

# `-P 16`: Google states 16 KB page size as a *device* property — without it the app will not
# run on future Android releases at all. Verified afterwards rather than assumed, because
# Tauri sets its own CARGO_TARGET_*_RUSTFLAGS and overwrites ours.
#
# Alignment is the build's business, and `apksigner` preserves it — so the two stages meet at
# `$out`: build leaves an aligned, unsigned APK there, sign picks that same path up.
if [ "$do_build" = yes ]; then
    "${bt}zipalign" -P 16 -f 4 "$unsigned" "$out"
fi

if [ "$do_sign" = yes ] && [ ! -f "$out" ]; then
    echo "android-release: nothing to sign at $out" >&2
    echo "  FM_ANDROID_STAGE=sign expects the build stage's aligned APK to be there already." >&2
    echo "  Check FM_VERSION matches the build ($FM_VERSION) — the filename carries it." >&2
    exit 1
fi

# Read the four values rather than sourcing the file: `. <(...)` is a bashism that dies under
# dash, and sourcing a properties file would *execute* whatever is in it. Parsing is both
# portable and the safer of the two for a file holding a signing password.
#
# **This is the only part of the script that touches the key**, which is the point of the split:
# in CI it runs in a job with no compiler, no npm and no cmake — one Google-published binary
# instead of hundreds of build scripts.
if [ "$do_sign" = yes ]; then
    storeFile=$(sed -n 's/^storeFile=//p' "$props")
    storePassword=$(sed -n 's/^storePassword=//p' "$props")
    keyAlias=$(sed -n 's/^keyAlias=//p' "$props")
    keyPassword=$(sed -n 's/^keyPassword=//p' "$props")
    "${bt}apksigner" sign --ks "$storeFile" --ks-pass "pass:$storePassword" \
        --key-pass "pass:$keyPassword" --ks-key-alias "$keyAlias" "$out" 2>/dev/null
    "${bt}apksigner" verify "$out" >/dev/null 2>&1 || { echo "android-release: signature did not verify" >&2; exit 1; }

    # **The published fingerprint is an assertion, not a note.** `android/signing-certificate.sha256`
    # is what users are told to check a download against; if this build carries a different
    # certificate then either the file is wrong or the wrong key just signed the app, and both are
    # worth stopping for. A mismatched APK would fail to install as an update on every phone that
    # already has formicaria, and would contradict the digest published beside it.
    want=$(sed -n 's/^sha256[[:space:]]*//p' "$root/android/signing-certificate.sha256" | tr -d '[:space:]')
    got=$("${bt}apksigner" verify --print-certs "$out" 2>/dev/null \
          | sed -n 's/.*certificate SHA-256 digest:[[:space:]]*//p' | head -1 | tr -d '[:space:]')
    if [ -z "$want" ]; then
        echo "android-release: no sha256 line in android/signing-certificate.sha256" >&2
        exit 1
    fi
    if [ "$got" != "$want" ]; then
        echo "android-release: SIGNED BY THE WRONG KEY" >&2
        echo "  expected $want" >&2
        echo "  got      $got" >&2
        echo "  Users are told to check a download against the expected digest, and every phone" >&2
        echo "  that already has formicaria will refuse this as an update. Either the wrong" >&2
        echo "  keystore signed it, or the app's identity really changed — in which case read" >&2
        echo "  decisions.md before editing android/signing-certificate.sha256." >&2
        exit 1
    fi
    echo "android-release: certificate matches the published fingerprint"
fi

# Assert the artifact, not the flag: this is the check that catches Tauri having replaced the
# alignment rustflag.
tmp=$(mktemp -d); unzip -o -j "$out" "lib/*/*.so" -d "$tmp" >/dev/null 2>&1
for so in "$tmp"/*.so; do
    align=$(readelf -lW "$so" 2>/dev/null | awk '/LOAD/{print $NF; exit}')
    [ "$align" = "0x4000" ] || echo "  WARNING: $(basename "$so") LOAD align $align, not 16 KB" >&2
done
rm -rf "$tmp"

# Say what actually happened. "signed" printed after a build-only run would be the kind of
# cheerful overstatement this project spends most of its guards preventing.
if [ "$do_sign" = yes ]; then
    what="signed and 16 KB aligned"
else
    what="16 KB aligned, NOT signed (FM_ANDROID_STAGE=build)"
fi
printf 'android-release: %s (%.0f MB), %s\n' "$out" \
    "$(stat -c%s "$out" | awk '{print $1/1048576}')" "$what"
