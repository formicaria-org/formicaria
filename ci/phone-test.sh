#!/bin/sh
# Put a commit on the owner's phone, signed with the real key, in one step.
#
# **What it replaces.** Testing on the phone before a release (the rule since v0.6.2) was six acts by
# hand: open GitHub, run the `android` workflow, wait, download the artifact, say where it landed,
# and have it installed by cable. Every one but the last needed a browser. This does all of it:
#
#   1. asks GitHub to run `android.yml` on a branch (default `main`);
#   2. waits for that run, and only that run, to finish;
#   3. downloads its `android-apk` artifact into a fresh temporary directory;
#   4. checks the APK's signing certificate against `android/signing-certificate.sha256`;
#   5. saves the app now on the phone to `target/phone-rollback/`;
#   6. installs it with `adb install -r`, then disconnects.
#
# **It needs a token, and only for steps 1 and 3.** Starting a workflow and downloading an artifact
# both require a signed-in GitHub request, even on a public repository. The token is one the owner
# made for exactly this (`ci/phone-test-setup.sh`): this repository only, *Actions: read and write*,
# nothing else. It cannot publish a release, change a secret or push code. It is read from a file
# only the owner can read and is passed to `curl` through a header file, never on a command line,
# so it does not appear in the process list or in this script's output.
#
# **It never installs something signed by anyone else.** A certificate that does not match stops
# the script before `adb` is touched: Android would refuse it anyway, and the only way past that
# refusal is an uninstall, which deletes the notes on the phone.
#
#   pixi run -e android phone-test            # main
#   pixi run -e android phone-test my-branch  # another branch
#   FM_PHONE_TEST_KEEP=1 …                    # download and check, do not install
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
ref="${1:-main}"
repo="formicaria-org/formicaria"
api="https://api.github.com/repos/$repo"
token_file="${FM_GITHUB_TOKEN_FILE:-$HOME/.config/formicaria/github-actions-token}"
package="dev.formicaria.notes"

say() { printf 'phone-test: %s\n' "$*"; }
die() { printf 'phone-test: %s\n' "$*" >&2; exit 1; }

[ -s "$token_file" ] || die "there is no token at $token_file. Make one once with:
    sh ci/phone-test-setup.sh"

work=$(mktemp -d "${TMPDIR:-/tmp}/fm-phone-test.XXXXXX")
headers="$work/headers"
cleanup() { rm -f "$headers"; }
trap cleanup EXIT INT TERM
( umask 177; printf 'Authorization: Bearer %s\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2022-11-28\nUser-Agent: formicaria-phone-test\n' "$(tr -d '[:space:]' < "$token_file")" > "$headers" )
gh() { curl -fsS -m 60 -H @"$headers" "$@"; }
field() { python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"; }

# 1. The commit GitHub will build is the branch as GitHub has it, so say which one that is, and
#    refuse to test something that is only on this machine.
git fetch -q origin "$ref"
sha=$(git rev-parse "origin/$ref")
if [ "$(git rev-parse HEAD)" != "$sha" ] && [ "$ref" = "$(git rev-parse --abbrev-ref HEAD)" ]; then
    die "your $ref is not what GitHub has (local $(git rev-parse --short HEAD), GitHub $(git rev-parse --short "$sha")). Push first: the build is made from GitHub's copy."
fi
say "asking GitHub to build $ref at $(git rev-parse --short "$sha")"
started=$(date -u +%Y-%m-%dT%H:%M:%SZ)
gh -X POST "$api/actions/workflows/android.yml/dispatches" -d "{\"ref\":\"$ref\"}" >/dev/null \
    || die "GitHub refused to start the build. Is the token still valid, and does it have Actions: read and write?"

# 2. Find the run this request started — same commit, started by hand, not before the request —
#    and wait for it. Never "the latest run": that could be an older build of another commit.
run=""
tries=0
while [ -z "$run" ]; do
    tries=$((tries + 1))
    [ "$tries" -le 20 ] || die "GitHub did not show the new build within 100 seconds."
    sleep 5
    run=$(gh "$api/actions/workflows/android.yml/runs?event=workflow_dispatch&per_page=10" | field "next((str(r['id']) for r in d['workflow_runs'] if r['head_sha']=='$sha' and r['created_at']>='$started'), '')")
done
say "build $run started: https://github.com/$repo/actions/runs/$run"
waited=0
while :; do
    state=$(gh "$api/actions/runs/$run" | field "d['status']+' '+str(d['conclusion'])")
    case "$state" in
        completed\ success) break ;;
        completed\ *) die "the build ended as '${state#completed }'. See https://github.com/$repo/actions/runs/$run" ;;
    esac
    waited=$((waited + 30))
    [ "$waited" -le 3600 ] || die "the build has run for an hour. See https://github.com/$repo/actions/runs/$run"
    [ $((waited % 120)) -ne 0 ] || say "still building ($((waited / 60)) min)"
    sleep 30
done

# 3. The signed APK. `android-sign` is allowed to fail without failing the run, so a green run with
#    no `android-apk` means signing did not happen, and that is worth saying in those words.
artifact=$(gh "$api/actions/runs/$run/artifacts" | field "next((a['archive_download_url'] for a in d['artifacts'] if a['name']=='android-apk'), '')")
[ -n "$artifact" ] || die "the build finished but made no signed APK (the signing job failed or was skipped). See https://github.com/$repo/actions/runs/$run"
gh -L -m 600 -o "$work/apk.zip" "$artifact"
mkdir "$work/apk"
unzip -q -o "$work/apk.zip" -d "$work/apk"
apk=$(find "$work/apk" -name '*.apk' | head -1)
[ -n "$apk" ] || die "the artifact held no APK."
say "downloaded $(basename "$apk")"

# 4. Signed by the published key, or nothing further happens.
signer=$(ls "$root"/.android/sdk/build-tools/*/apksigner 2>/dev/null | sort | tail -1)
[ -x "$signer" ] || die "apksigner is not here. Run: pixi run android-init"
want=$(sed -n 's/^sha256[[:space:]]*//p' android/signing-certificate.sha256 | tr -d '[:space:]')
got=$("$signer" verify --print-certs "$apk" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | tr -d '[:space:]')
[ -n "$got" ] && [ "$got" = "$want" ] || die "the APK is not signed with formicaria's published key (got '${got:-nothing}'). It was not installed."
say "signed with the published key"

if [ -n "${FM_PHONE_TEST_KEEP:-}" ]; then
    say "kept at $apk (FM_PHONE_TEST_KEEP is set, so it was not installed)"
    exit 0
fi

# 5 and 6. The phone. Only the documented steps: read where the app is, copy it out, install over it.
adb="$root/.android/sdk/platform-tools/adb"
[ -x "$adb" ] || adb=$(readlink -f "$root/.android/sdk/../platform-tools/adb")
[ -x "$adb" ] || die "adb is not here. Run: pixi run android-init"
disconnect() { "$adb" kill-server >/dev/null 2>&1 || true; cleanup; }
trap disconnect EXIT INT TERM
[ "$("$adb" devices | awk 'NR>1 && $2=="device"' | wc -l)" = 1 ] \
    || die "exactly one phone must be connected with USB debugging allowed. The APK is at $apk"
installed=$("$adb" shell pm path "$package" | head -1 | sed 's/^package://' | tr -d '\r')
if [ -n "$installed" ]; then
    mkdir -p target/phone-rollback
    was=$("$adb" shell dumpsys package "$package" | sed -n 's/.*versionName=//p' | head -1 | tr -d '\r')
    keep="target/phone-rollback/formicaria-${was:-unknown}-installed-$(date +%Y-%m-%d-%H%M).apk"
    "$adb" pull "$installed" "$keep" >/dev/null
    say "saved what was on the phone ($was) to $keep"
fi
"$adb" install -r "$apk" | tail -1
say "on the phone now: $("$adb" shell dumpsys package "$package" | sed -n 's/.*versionName=//p' | head -1 | tr -d '\r') built from $(git rev-parse --short "$sha")"
