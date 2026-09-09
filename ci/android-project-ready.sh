#!/bin/sh
# Generate `mobile/src-tauri/gen/android` if it is not there yet.
#
# **Why this exists.** `gen/android` is generated and gitignored — `.gitignore` forbids committing
# it in terms, because "it embeds absolute paths, so committing it would be committing this
# machine". Everything that stages into the APK therefore requires it to have been generated first,
# and three scripts said so and none did it:
#
#   android-stage-runtime: …/jniLibs/arm64-v8a is missing — run 'pixi run tauri android init' first
#   android-stage-whisper: … missing — run 'pixi run tauri android init' first
#   android-inject-service: … missing — run 'tauri android init' first
#
# Which is fine advice for a person and useless to a runner. The APK had only ever been built on the
# machine where somebody had once typed that by hand, so the first CI attempt died in 60 seconds
# (2026-09-09). **This is the same shape as the `rustup` shim**, which lived in a gitignored
# `.android/bin` until 2026-09-03 and made `android-apk` a task that "worked on one machine and
# nowhere else" — `pixi.toml` still says so, one comment above the task this fixes.
#
# iOS has had the equivalent since it was written: `ios_project_ready()` in `ci/lib/ios-toolchain.sh`
# runs `tauri ios init` when `gen/apple` is absent. Android simply never grew one.
#
# **Idempotent, and deliberately not a regeneration.** If the directory exists this does nothing:
# `tauri android init` rewrites the generated tree, which would discard the foreground-service
# injection and the launcher icons that `android-inject-service` and `android-release` put there —
# and on a developer's machine it would silently undo local state mid-build.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
gen="$root/mobile/src-tauri/gen/android"

if [ -d "$gen" ]; then
    echo "android-project-ready: gen/android already present"
else
    echo "android-project-ready: no gen/android — running 'tauri android init'"

    # The same three variables the build needs, for the reasons `ci/android-release.sh` documents at
    # length: Tauri reads `NDK_HOME` while cargo reads `ANDROID_NDK_HOME`; Gradle refuses conda's
    # default JDK; and `ci/bin` puts our verifying `rustup` shim first, because `android init` is
    # one of the things that shells out to `rustup target add`.
    ( cd "$root/mobile" \
        && PATH="$root/ci/bin:$PATH" \
           NDK_HOME="${NDK_HOME:-$ANDROID_NDK_HOME}" \
           JAVA_HOME="${JAVA_HOME:-$CONDA_PREFIX/lib/jvm}" \
           pnpm exec tauri android init )

    [ -d "$gen" ] || {
        echo "android-project-ready: 'tauri android init' finished but $gen still does not exist." >&2
        exit 1
    }
    echo "android-project-ready: generated $gen"
fi

# **And the staging destination, which a freshly generated project does not have.** `jniLibs/<abi>`
# is where `android-stage-runtime` and `android-stage-whisper` put the two model runtimes, and both
# refuse outright if it is absent — with the same "run 'tauri android init' first" advice, which by
# then has already been followed. It exists on this machine only because a build once created it,
# so the second CI attempt failed here *after* the init this script had just done (2026-09-09).
#
# Gradle packages whatever is in `jniLibs`, so creating it early is free; unconditional rather than
# inside the init branch, because a tree that was generated but never built is in exactly the same
# state as a fresh one.
mkdir -p "$gen/app/src/main/jniLibs/arm64-v8a"
