#!/bin/sh
# **Does a device-target unsigned IPA build, and is it the shape a sideloader will accept?** — iOS rung 5.
#
# Rung 5 was struck from `ios-plan-2026-09-02.md` as *"a device install would cost $99/yr to produce
# an artifact nobody can run"*. That was true under the *"no iPhone"* ruling, which is withdrawn.
# The rung is back asking a **different question**: not *"can we ship?"* but *"does the unsigned
# device path produce a well-formed artifact?"* — see `decisions.md`, *iOS ships as an unsigned IPA
# that each user signs with their own Apple ID*.
#
# **Why this costs nothing.** `tauri-cli` v2.11.4 already emits exactly this artifact
# (`crates/tauri-cli/src/mobile/ios/build.rs:432-509`): with `--no-sign` it skips `build()`
# entirely, archives with `skip_codesign()`, then lifts the `.app` out of the `.xcarchive` and calls
# `create_ipa()`. **No `-exportArchive`, no `ExportOptions.plist`, no provisioning profile, no
# development team** — all of those live in the `else` branch reached only when signing is on. So
# the delta from rung 2, which has passed `--no-sign` since the day it was written, is one target
# triple.
#
# **What it asserts, and why each one is the check worth paying for:**
#   1. the `.ipa` exists and holds `Payload/<Name>.app/` — any other layout, a sideloader rejects
#   2. the Mach-O's build platform is **IOS**, not **IOSSIMULATOR** — the one thing four green
#      Simulator runs have never proven, and the whole reason this rung exists
#   3. the binary is `arm64` alone
#   4. the app carries **no `_CodeSignature/`** — unsigned is the requirement, not a side effect. A
#      signed one would mean `--no-sign` did not do what `build.rs` says it does
#   5. the generated entitlements name nothing a **free personal team cannot hold** — App Groups,
#      keychain sharing, push, iCloud, associated domains, Sign in with Apple, Apple Pay. Any one of
#      them makes the artifact unsignable by the people it is built for, and nothing else in this
#      repo would notice one being added
#   6. `CFBundleIdentifier`, `MinimumOSVersion`, `CFBundleShortVersionString` — recorded, not
#      asserted: the first honest reading of what we actually ship
#
# **Kill criterion, written before the money is spent.** If the device archive fails for a reason
# that needs a signing identity — a team, a certificate, a profile — stop. Record "the unsigned
# device path needs a team after all", and do not iterate it into a budget.
#
# **macOS only, and there is no Mac here.** Like `ci/ios-smoke.sh`, this is written blind. The parts
# that can be tested off a Mac — the Mach-O platform parser and the entitlements scanner — are, by
# `sh ci/ios-package.sh --self-test`, which `ci/checks.sh` runs on every commit.
#
# Usage (CI, or a Mac someone else owns):
#   sh ci/ios-package.sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
OUT="$root/target/ios-package/$(date +%Y%m%d-%H%M%S)"

say() { echo "ios-package: $*"; }
# Shared with `ci/ios-smoke.sh`: TOOLCHAINS, the verifying rustup shim on PATH, `run_logged`,
# `tauri_ios`, the CLI architecture check and `ios_project_ready`. Every one of them was learned
# inside a billed job; see that file.
. "$root/ci/lib/ios-toolchain.sh"

# ---------------------------------------------------------------------------
# The two parsers, and only the two. Everything else here is a file test.
# ---------------------------------------------------------------------------

# `macho_platform` — read the LC_BUILD_VERSION platform out of `vtool -show-build` (or `otool -l`)
# output on stdin, as a name.
#
# **Both spellings are handled deliberately.** `vtool` prints `platform IOS`; `otool -l` prints the
# raw constant `platform 2`. Which one a given Xcode gives is not worth discovering inside a billed
# job, and a parser that silently returned the empty string for the numeric form would turn the
# single most important assertion in this file — device, not simulator — into one that always
# passes. Constants from `<mach-o/loader.h>`: 2 is IOS, 7 is IOSSIMULATOR, and they are one digit
# apart, which is exactly how this goes wrong quietly.
macho_platform() {
    sed -n 's/^ *platform  *\([A-Za-z0-9]*\).*/\1/p' | head -1 | awk '
        /^[0-9]+$/ {
            p = $0 + 0
            n = "UNKNOWN-" p
            if (p == 1) n = "MACOS";     else if (p == 2) n = "IOS"
            else if (p == 3) n = "TVOS";      else if (p == 4) n = "WATCHOS"
            else if (p == 5) n = "BRIDGEOS";  else if (p == 6) n = "MACCATALYST"
            else if (p == 7) n = "IOSSIMULATOR"
            else if (p == 8) n = "TVOSSIMULATOR"; else if (p == 9) n = "WATCHOSSIMULATOR"
            print n; next
        }
        { print toupper($0) }'
}

# `forbidden_entitlements` — echo, one per line, every entitlement key on stdin that a **free
# personal team** cannot hold. Not a style check: any one of these makes the `.ipa` unsignable by
# the users it exists for, and they arrive by accident — enabling a capability in Xcode writes one.
FREE_TEAM_FORBIDS='com.apple.security.application-groups
keychain-access-groups
aps-environment
com.apple.developer.icloud-container-identifiers
com.apple.developer.icloud-services
com.apple.developer.ubiquity-kvstore-identifier
com.apple.developer.associated-domains
com.apple.developer.applesignin
com.apple.developer.in-app-payments'
forbidden_entitlements() {
    _text=$(cat)
    printf '%s\n' "$FREE_TEAM_FORBIDS" | while IFS= read -r k; do
        [ -n "$k" ] || continue
        # Match the key in its `<key>…</key>` element, which is how it appears in a plist whether
        # the file is hand-written or Xcode-generated. Substring matching would fire on prose.
        #
        # `if`, not `grep -q … && echo`: the AND-list would be the loop body's last command, so
        # under `set -eu` a *clean* file — every grep failing, which is the expected case — leaves
        # a non-zero status behind. It survives today only because of where this is called from,
        # and that is not a property to depend on inside a job that costs money.
        if printf '%s' "$_text" | grep -q "<key>$k</key>"; then echo "$k"; fi
    done
    return 0
}

# ---------------------------------------------------------------------------
# `--self-test` — the parsers, on Linux, for free. Runs before any precondition.
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--self-test" ]; then
    t_fail=0
    check() {  # name expected actual
        if [ "$2" = "$3" ]; then
            echo "  ok   $1"
        else
            echo "  FAIL $1: expected '$2', got '$3'"; t_fail=1
        fi
    }
    vtool_device='/path/to/formicaria.app/formicaria:
Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform IOS
    minos 13.0
      sdk 26.0'
    otool_sim='Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform 7
    minos 13.0'
    otool_device='Load command 10
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform 2
    minos 13.0'
    ents_clean='<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>com.apple.security.get-task-allow</key><true/>
</dict></plist>'
    ents_dirty='<plist version="1.0"><dict>
  <key>keychain-access-groups</key><array><string>$(AppIdentifierPrefix)dev.formicaria.notes</string></array>
  <key>com.apple.developer.associated-domains</key><array/>
</dict></plist>'

    echo "ios-package --self-test: the Mach-O platform parser"
    check "vtool's symbolic name" "IOS" "$(printf '%s\n' "$vtool_device" | macho_platform)"
    check "otool's numeric 2 is IOS" "IOS" "$(printf '%s\n' "$otool_device" | macho_platform)"
    check "otool's numeric 7 is IOSSIMULATOR (the assertion that must not pass)" \
        "IOSSIMULATOR" "$(printf '%s\n' "$otool_sim" | macho_platform)"
    check "no LC_BUILD_VERSION at all is empty, not a false IOS" \
        "" "$(printf '%s\n' "Load command 1" | macho_platform)"

    echo "ios-package --self-test: the free-team entitlements scanner"
    check "a clean entitlements file names nothing" \
        "" "$(printf '%s\n' "$ents_clean" | forbidden_entitlements)"
    check "keychain sharing and associated domains are both caught" \
        "keychain-access-groups
com.apple.developer.associated-domains" "$(printf '%s\n' "$ents_dirty" | forbidden_entitlements)"
    check "prose mentioning a key is not a match" \
        "" "$(printf '%s\n' "we must never add aps-environment here" | forbidden_entitlements)"

    [ "$t_fail" = 0 ] || { echo "ios-package --self-test: FAILED" >&2; exit 1; }
    echo "ios-package --self-test: all parsers ok"
    exit 0
fi

# Only name the artifact directory once there is one — pointing at an empty path is the kind of
# small lie that sends the next person looking for a log that was never written.
fail() {
    echo "FAIL: $*" >&2
    if [ -d "$OUT" ]; then echo "  artifacts: $OUT" >&2; fi
    exit 1
}

# ---------------------------------------------------------------------------
# 0) Preconditions.
# ---------------------------------------------------------------------------
[ "$(uname -s)" = "Darwin" ] || fail "this needs macOS (xcrun, xcodebuild, otool, lipo). There is no
  Mac in this project; the only one it has is a GitHub runner — see .github/workflows/ios.yml."
for t in xcrun xcodebuild plutil unzip; do
    command -v "$t" >/dev/null 2>&1 || fail "$t is not on PATH (run this as 'pixi run ios-package')"
done
mkdir -p "$OUT"

# **The version the artifact reports about itself.** Same line as `ci/android-release.sh`, and it
# was missing here entirely: nothing on the iOS path set `FM_VERSION`, so `fm-app`'s
# `option_env!` fell back to `dev` and Settings said `dev` while the bundle's
# `CFBundleShortVersionString` said `0.4.0` from `tauri.conf.json`. Two halves of one build
# disagreeing about what they are — the exact confusion `android-release.sh:64-70` was written to
# close, reintroduced on a third platform and unnoticed until the `.ipa` became publishable
# (2026-09-09).
#
# The workflow sets it from the dispatched ref; this default covers a local run on a Mac, and
# `dev` is the honest answer for a build made from no tag.
: "${FM_VERSION:=$(git -C "$root" describe --tags --exact-match 2>/dev/null || echo dev)}"
export FM_VERSION
say "building as FM_VERSION=$FM_VERSION"

# The runner is 3 CPU / 7 GB RAM / 14 GB disk and this compiles vendored OpenSSL, libgit2 and
# SQLite *and* runs an Xcode archive. Record disk rather than guess at it later.
df -h / "$root" > "$OUT/disk-before.txt" 2>&1 || true

tauri_cli_arch_check
ios_project_ready

# ---------------------------------------------------------------------------
# 1) Build for the device.
#
# `--target aarch64` is one of exactly three accepted values (`aarch64`, `x86_64`, `aarch64-sim`)
# and is the **device** triple — `aarch64-apple-ios`, not the `-sim` one every green run so far has
# exercised. `--no-sign` is what makes this free; see this file's header.
#
# **Clear the output directory first.** The name is globbed (it comes from `productName` via
# cargo-mobile2) but the directory is pinned, and a stale `.ipa` from an earlier attempt in a reused
# checkout would let every assertion below pass against an artifact this run did not build.
# ---------------------------------------------------------------------------
outdir="mobile/src-tauri/gen/apple/build/arm64"
rm -f "$outdir"/*.ipa 2>/dev/null || true

say "building for the device (aarch64, unsigned)…"
run_logged build tauri_ios build --target aarch64 --no-sign --verbose

ipa=$(find "$outdir" -maxdepth 1 -name '*.ipa' -type f 2>/dev/null | head -1)
[ -n "$ipa" ] || fail "the build produced no .ipa in $outdir ($OUT/build.log).
  If the log asks for a signing identity, team or provisioning profile, that is this rung's kill
  criterion: record it and stop, do not iterate."
say "built $ipa"

stats="$OUT/stats.txt"
: > "$stats"
record() { echo "$*" | tee -a "$stats"; }
record "fm-version     $FM_VERSION"
record "ipa            $ipa"
record "ipa-bytes      $(wc -c < "$ipa" | tr -d ' ')"

# ---------------------------------------------------------------------------
# 2) Is it the shape a sideloader accepts?
# ---------------------------------------------------------------------------
work="$OUT/extracted"
mkdir -p "$work"
unzip -q "$ipa" -d "$work" || fail "the .ipa is not a readable zip"
unzip -l "$ipa" > "$OUT/ipa-listing.txt" 2>&1 || true

app=$(find "$work/Payload" -maxdepth 1 -name '*.app' -type d 2>/dev/null | head -1)
[ -n "$app" ] || fail "no Payload/<Name>.app inside the .ipa — a sideloader will reject it
  ($OUT/ipa-listing.txt)"
appname=$(basename "$app")
record "payload-app    $appname"

# The executable named by the bundle, not the one whose name happens to match the directory.
exe=$(plutil -extract CFBundleExecutable raw "$app/Info.plist" 2>/dev/null || true)
[ -n "$exe" ] && [ -f "$app/$exe" ] || fail "could not resolve CFBundleExecutable inside $appname"
bin="$app/$exe"

# --- 2. device, not simulator. The reason this rung exists. ---------------------------------
if command -v vtool >/dev/null 2>&1; then
    vtool -show-build "$bin" > "$OUT/machO-build.txt" 2>&1 || true
else
    otool -l "$bin" > "$OUT/machO-build.txt" 2>&1 || true
fi
platform=$(macho_platform < "$OUT/machO-build.txt")
record "platform       ${platform:-<none>}"
[ "$platform" = "IOS" ] || fail "the binary's build platform is '${platform:-<none>}', not IOS.
  IOSSIMULATOR here would mean this rung built the same thing rungs 2-3 already build, and proved
  nothing new ($OUT/machO-build.txt)."

# --- 3. arm64 alone --------------------------------------------------------------------------
archs=$(lipo -archs "$bin" 2>/dev/null | tr -s ' ' | sed 's/^ *//;s/ *$//' || true)
record "archs          ${archs:-<unreadable>}"
[ "$archs" = "arm64" ] || fail "expected a single arm64 slice, got '${archs:-<unreadable>}'"

# --- 4. unsigned -----------------------------------------------------------------------------
if [ -d "$app/_CodeSignature" ]; then
    record "signature      PRESENT"
    fail "$appname carries a _CodeSignature/. It must be unsigned: the whole route is that each
  user re-signs it with their own Apple ID. A signature here means --no-sign did not take the
  create_ipa path in tauri-cli's build.rs, and the artifact should not be published."
fi
record "signature      absent (as required)"

# --- 5. nothing a free personal team cannot hold ----------------------------------------------
# The *generated project's* entitlements file, not the bundle's: the `--no-sign` path deliberately
# leaves entitlements off the binary (`build.rs:511`), which is correct here — the sideloader
# injects its own on re-sign — so the bundle is not where a forbidden capability would show up.
found=""
for ent in mobile/src-tauri/gen/apple/*/*.entitlements; do
    [ -f "$ent" ] || continue
    hits=$(forbidden_entitlements < "$ent" | tr '\n' ' ' | sed 's/ *$//')
    record "entitlements   $ent -> ${hits:-none forbidden}"
    [ -z "$hits" ] || found="$found $ent:$hits"
done
[ -z "$found" ] || fail "entitlements a free personal team cannot hold:$found
  Every user of this app signs it with a free Apple ID. Any of these makes that impossible — see
  decisions.md, 'iOS ships as an unsigned IPA that each user signs with their own Apple ID'."

# --- 5b. the permission keys, and this one is an assertion ---------------------------------------
# **A missing `NS*UsageDescription` does not deny a permission on iOS, it terminates the app.** The
# `＋ Media` menu is rendered on every note being edited with no platform gate, so without these
# the first tester to tap Record audio crashes the artifact we just built. `ci/ios-inject-plist.sh`
# puts them into `project.yml`; this is the check that they survived XcodeGen and the archive, on
# the actual `.ipa` rather than on the spec that was supposed to produce it.
missing=""
for k in NSMicrophoneUsageDescription NSCameraUsageDescription \
         NSPhotoLibraryUsageDescription NSLocalNetworkUsageDescription; do
    v=$(plutil -extract "$k" raw "$app/Info.plist" 2>/dev/null || true)
    if [ -z "$v" ]; then missing="$missing $k"; else record "$(printf '%-28s' "$k")present"; fi
done
[ -z "$missing" ] || fail "the built app declares no usage description for:$missing
  On iOS that is not a denied permission — the system terminates the app the moment the API is
  touched, and ＋ Media reaches all of them. See ci/ios-inject-plist.sh."

# --- 6. what we actually ship, recorded ---------------------------------------------------------
for k in CFBundleIdentifier CFBundleShortVersionString CFBundleVersion MinimumOSVersion; do
    v=$(plutil -extract "$k" raw "$app/Info.plist" 2>/dev/null || echo "<absent>")
    # `%-28s`: the longest key here is `CFBundleShortVersionString` at 26 characters, and `printf`
    # pads but never truncates — at `%-14s` the first run printed `CFBundleIdentifierdev.formicaria.notes`
    # with no separator at all. An artifact nobody can regenerate cheaply should be readable first time.
    record "$(printf '%-28s' "$k")$v"
done

df -h / "$root" > "$OUT/disk-after.txt" 2>&1 || true

say "an unsigned device .ipa exists and is well-formed:"
sed 's/^/  /' "$stats"
say "artifacts: $OUT"
say "NOT PROVEN, and it cannot be proven from here: that this .ipa re-signs, installs, launches or
  syncs on a physical iPhone. There is no iPhone in this project."
