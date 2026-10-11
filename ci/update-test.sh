#!/bin/sh
# Prove that a downloaded copy can update itself from inside the app, on this computer.
#
# **What it does**, with the real published files and nothing faked:
#
#   1. downloads the release *before* the newest one and unpacks it into a temporary folder;
#   2. starts it the way its launcher does, on a spare port, with settings of its own;
#   3. presses what Settings presses: *Check now*, *Get*, *Restart*;
#   4. requires the copy to answer as the newest release afterwards, with the way back kept;
#   5. presses *Go back* and requires the earlier version to answer again;
#   6. requires a note written before all this to be byte-for-byte the same after each step.
#
# **Why it exists.** Every piece of the updater has unit tests, and none of them can say "a person
# who downloaded the last release can reach this one without opening GitHub". That sentence is only
# true of the published archives, the signed manifest and the launcher together, so this runs
# those. It was first done by hand on 2026-10-11, when the owner saw no update buttons on Linux.
#
# **What it does not cover.** It sets the launcher's variables itself instead of running
# `Start formicaria.*` (which opens a browser), and it tests releases that are already published:
# it cannot say whether the *next* one will be reachable until that one is out. The release
# workflow therefore runs it on all three desktop systems right after publishing.
#
# It needs the network, so it is not part of `pixi run ci`. It touches nothing outside its own
# temporary folder: the copy under test gets its own home, settings and notes.
#
# **`--from-source`** asks the same of the copy built in this checkout (`pixi run build` first):
# it has no version, so it must be offered the newest release, install it into a folder of its own
# (`FM_APP_DIR`, what `packaging/formicaria.sh` sets), and come back to the build made here.
#
#   pixi run update-test              # the release before the newest → the newest
#   pixi run update-test --from-source   # the build made here → the newest, and back
#   pixi run update-test v0.6.4      # start from this one instead
#   FM_UPDATE_TEST_TO=v0.6.6 …       # require this version at the end (the release workflow's use)
#   FM_UPDATE_TEST_PORT=8799 …       # another spare port
set -eu

repo="formicaria-org/formicaria"
port="${FM_UPDATE_TEST_PORT:-8799}"
base="http://127.0.0.1:$port/api"

say() { printf 'update-test: %s\n' "$*"; }
die() {
    printf 'update-test: FAIL: %s\n' "$*" >&2
    [ -z "${last:-}" ] || printf 'update-test: the app last answered: %s\n' "$last" >&2
    [ ! -f "${log:-/nonexistent}" ] || { echo "update-test: its own messages:" >&2; tail -15 "$log" >&2; }
    exit 1
}

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=linux-x86_64 archive=tar.gz exe=fm-serve ;;
    Darwin-arm64) target=macos-arm64 archive=tar.gz exe=fm-serve ;;
    MINGW*-x86_64 | MSYS*-x86_64 | CYGWIN*-x86_64) target=windows-x86_64 archive=zip exe=fm-serve.exe ;;
    *) die "no formicaria is published for $(uname -s) $(uname -m), so there is nothing to test here." ;;
esac

# The newest release is whatever the download page redirects to: the same address the app shows.
newest=$(curl -sI -m 30 "https://github.com/$repo/releases/latest" | tr -d '\r' | sed -n 's|^[Ll]ocation: .*/tag/||p')
[ -n "$newest" ] || die "could not ask GitHub which release is newest."
to="${FM_UPDATE_TEST_TO:-$newest}"
[ "$to" = "$newest" ] || die "the newest published release is $newest, not $to. Is $to published yet?"
from="${1:-}"
source_build=""
if [ "$from" = "--from-source" ]; then
    source_build="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/target/release/$exe"
    [ -x "$source_build" ] || die "there is no build here ($source_build). Run: pixi run build"
    from="the build made here"
elif [ -z "$from" ]; then
    from=$(git ls-remote --tags --refs "https://github.com/$repo" 'v*' | sed 's|.*refs/tags/||' \
        | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | grep -B1 -x "$to" | head -1)
fi
[ -n "$from" ] && [ "$from" != "$to" ] || die "found no release older than $to to start from."
say "$from → $to on $target"

work=$(mktemp -d "${TMPDIR:-/tmp}/fm-update-test.XXXXXX")
log="$work/app.log"
last=""
# The app restarts itself as a new process twice, so what is running at the end is not what was
# started here. It is asked to stop the way a closing tab asks, and it also stops by itself when
# nobody is looking; the folder is removed only once it has.
stop() {
    curl -s -m 5 -o /dev/null "$base/bye?tab=update-test" 2>/dev/null || true
    n=0
    while curl -s -m 2 -o /dev/null "$base/alive" 2>/dev/null; do
        n=$((n + 1))
        [ "$n" -le 60 ] || { say "the copy under test is still running; left $work in place"; return; }
        sleep 2
    done
    rm -rf "$work"
}
trap stop EXIT INT TERM

if curl -s -m 2 -o /dev/null "$base/alive" 2>/dev/null; then
    trap - EXIT INT TERM
    rm -rf "$work"
    die "something already answers on port $port. Choose another with FM_UPDATE_TEST_PORT."
fi

if [ -n "$source_build" ]; then
    # No download: the program is the one built here, started where it is, and told where it may
    # install. Its notes live in the temporary folder like the other mode's.
    app="$work/copy"
    mkdir -p "$app"
    program="$source_build"
    FM_APP_DIR="$work/installed"
    export FM_APP_DIR
    current_from=""      # a build with no version answers `"current":null`
else
name="formicaria-$from-$target"
curl -fsSL -m 600 -o "$work/app.$archive" "https://github.com/$repo/releases/download/$from/$name.$archive" \
    || die "could not download $name.$archive."
mkdir "$work/unpack"
case "$archive" in
    zip) unzip -q "$work/app.zip" -d "$work/unpack" ;;
    *) tar -xzf "$work/app.tar.gz" -C "$work/unpack" ;;
esac
app="$work/unpack/$name"
[ -x "$app/program/$exe" ] || die "$name.$archive holds no program/$exe."
program="$app/program/$exe"
current_from="$from"
fi

# A note of the user's, to be found untouched at every step.
mkdir -p "$app/vault/notes"
printf -- '---\ntitle: kept\n---\nThese words must survive an update and the way back.\n' > "$app/vault/notes/kept.md"
cp "$app/vault/notes/kept.md" "$work/kept.md"
kept() { cmp -s "$app/vault/notes/kept.md" "$work/kept.md" || die "the note changed $1."; }

# What `Start formicaria.*` sets, minus the browser; and a home of its own, so the copy under test
# neither reads nor writes this computer's formicaria settings.
mkdir -p "$work/self/config"
HOME="$work/self" USERPROFILE="$work/self" XDG_CONFIG_HOME="$work/self/config" APPDATA="$work/self/config" \
    FM_ADDR="127.0.0.1:$port" FM_VAULT="$app/vault" FM_VAULTS="$app/vaults.json" FM_LAUNCHER=2 FM_OPEN=0 \
    "$program" > "$log" 2>&1 &

press() { last=$(curl -s -m 60 -X POST -H 'content-type: application/json' -d '{}' "$base/$1" || true); }
# One value out of the status the Settings screen reads: `"key":"value"` or `"key":value`.
value() { printf '%s' "$last" | sed -n "s/.*\"$1\":\"\\{0,1\\}\\([^\",}]*\\).*/\\1/p" | sed 's/^null$//'; }
# wait_for <seconds> <key> <wanted> <what a person would say is happening>
wait_for() {
    waited=0
    while :; do
        press update_status
        # No answer is not an answer: while the copy restarts, an empty reply must not pass for an
        # empty value (a build with no version answers `"current":null`).
        [ -z "$last" ] || [ "$(value "$2")" != "$3" ] || return 0
        [ "$(value stage)" != failed ] || die "$4: the app reports a failure."
        waited=$((waited + 2))
        [ "$waited" -le "$1" ] || die "$4: \"$2\" was still '$(value "$2")' after $1 seconds, wanted '$3'."
        sleep 2
    done
}

wait_for 30 can_check true "starting $from"
[ "$(value current)" = "$current_from" ] || die "expected $from to be running, not '$(value current)'."
[ "$(value can_check)" = true ] || die "$from shows no way to look for a newer version (can_check is false)."
[ "$(value can_install)" = true ] || die "$from says it cannot install an update: $(value why)"
say "$from is running and offers to look for a newer version"

# **Check now, allowing for GitHub refusing the question.** Versions up to 0.6.9 ask GitHub's
# listing, which answers anonymous callers sixty times an hour per address; GitHub's own Linux
# build machines share addresses and are often past that, and then the answer is *status code
# 403*. That says nothing about the release (the same run passed on macOS and Windows, and passed
# on Linux when run again, v0.6.9, 2026-10-11). So: ask again for a few minutes, and if the
# refusal persists, say plainly that this system could not be tested and do not call it a failure.
# Any other outcome is judged as before. Versions after 0.6.9 ask the release page first and do
# not meet this.
tries=0
while :; do
    press update_check
    sleep 3
    press update_status
    [ "$(value available)" != "$to" ] || break
    case "$last" in
        *"status code 403"* | *"status code 429"*) ;;
        *) wait_for 60 available "$to" "Check now"; break ;;
    esac
    tries=$((tries + 1))
    if [ "$tries" -ge 10 ]; then
        msg="NOT TESTED on $target: GitHub refused $from's question about the newest version (too many anonymous questions from this machine's address) for five minutes. Run this job again."
        say "$msg"
        [ -z "${GITHUB_ACTIONS:-}" ] || echo "::warning title=update test not run::$msg"
        exit 0
    fi
    say "GitHub refused the question (rate limit); asking again in 30 seconds ($tries of 10)"
    sleep 30
done
say "Check now found $to"

press update_start
wait_for 900 stage ready "Get $to"
kept "while $to was downloaded"
say "Get downloaded and checked $to"

press update_apply
wait_for 120 current "$to" "Restart into $to"
[ "$(value previous)" = "${current_from:-previous}" ] || die "after the update, the way back to $from is missing."
kept "during the update to $to"
say "Restart: the copy now answers as $to, and keeps $from to go back to"

press update_rollback
wait_for 120 current "$current_from" "Go back to $from"
[ -z "$source_build" ] || [ "$(value available)" = "" ] || die "the release that was turned down is offered again."
kept "on the way back to $from"
say "Go back: the copy answers as $from again"

say "PASS: $from updated itself to $to from inside the app, went back, and the note is unchanged"
