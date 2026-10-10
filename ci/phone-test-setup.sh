#!/bin/sh
# Store the GitHub token `ci/phone-test.sh` uses, once. Run it in your own terminal: it asks a
# question, and what you paste must not pass through an assistant or a recorded screen.
#
# **Make the token first** (two minutes, in a browser):
#   GitHub → your picture → Settings → Developer settings → Personal access tokens →
#   Fine-grained tokens → Generate new token
#     - Resource owner:      formicaria-org
#     - Repository access:   Only select repositories → formicaria
#     - Permissions → Repository permissions → Actions: Read and write
#     - everything else:     No access
#     - Expiration:          your choice; 90 days is reasonable
#
# **That is all it can do**: start a workflow and read its results in this one repository. It cannot
# push code, publish or edit a release, or read or change a secret. Starting `android` builds an APK
# and publishes nothing, so the worst a leaked copy of this token can do is spend runner minutes.
#
# The token is read with echo off, checked against GitHub, and only then written, readable by you
# alone. A token that does not work writes nothing.
#
#   sh ci/phone-test-setup.sh
set -eu
dest="${FM_GITHUB_TOKEN_FILE:-$HOME/.config/formicaria/github-actions-token}"
api="https://api.github.com/repos/formicaria-org/formicaria"

restore_echo() { stty echo 2>/dev/null || true; }
work=$(mktemp -d "${TMPDIR:-/tmp}/fm-token.XXXXXX")
cleanup() { restore_echo; rm -rf "$work"; }
trap cleanup EXIT INT TERM

printf 'Paste the fine-grained token (it will not be shown), then Enter: '
stty -echo
read -r token
restore_echo
printf '\n'
token=$(printf '%s' "$token" | tr -d '[:space:]')
[ -n "$token" ] || { echo "Nothing was pasted. Nothing was written." >&2; exit 1; }

( umask 177; printf 'Authorization: Bearer %s\nAccept: application/vnd.github+json\nUser-Agent: formicaria-phone-test\n' "$token" > "$work/h" )
# Reading the workflow proves the token is valid and reaches this repository. Whether it may also
# *start* one is only provable by starting one, which `phone-test.sh` will say plainly if it cannot.
code=$(curl -s -m 30 -o /dev/null -w '%{http_code}' -H @"$work/h" "$api/actions/workflows/android.yml")
if [ "$code" != 200 ]; then
    echo "GitHub answered $code for that token, so it was not stored. Check it is for formicaria-org/formicaria" >&2
    echo "with Actions: Read and write, and that it has not expired." >&2
    exit 1
fi

mkdir -p "$(dirname "$dest")"
[ ! -f "$dest" ] || mv "$dest" "$dest.before-$(date +%Y%m%d-%H%M%S)"
( umask 177; printf '%s\n' "$token" > "$dest" )
echo "Stored at $dest, readable only by you. Test a build on the phone with:"
echo "    pixi run -e android phone-test"
