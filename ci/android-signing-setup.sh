#!/bin/sh
# Restore local release signing for the phone: write ~/.config/formicaria/android-release.properties.
#
# **Why this exists.** That file is what `ci/android-release.sh` reads to sign an APK on this machine,
# and signing locally is what lets a build go onto the phone by cable *before* it is released: the
# phone only accepts an update signed by the same key as the installed app. On 2026-10-09 the file
# was found to hold a byte-copy of the keystore instead of the four lines, so every local build came
# out unsigned (`docs/context/known-issues.md`).
#
# **The passwords never leave this terminal.** They are read with echo off, checked against the
# keystore with `keytool` (passed through the environment, not the command line, so they are not in
# the process list), and only then written, owner-only. A wrong password writes nothing. The old file
# is kept beside the new one, never deleted.
#
# Run it in your own terminal (it asks questions, so not through an assistant):
#     pixi run -e android sh ci/android-signing-setup.sh
set -eu
d="$HOME/.config/formicaria"
ks="$d/android-release.keystore"
props="$d/android-release.properties"

if [ ! -f "$ks" ]; then
    echo "There is no keystore at $ks — nothing to set up." >&2
    exit 1
fi

restore_echo() { stty echo 2>/dev/null || true; }
trap restore_echo EXIT INT TERM

printf 'Key alias (Enter for "formicaria"): '
read -r alias
alias=${alias:-formicaria}
stty -echo
printf 'Keystore password (ANDROID_STORE_PASSWORD): '
read -r FM_SP
printf '\nKey password (ANDROID_KEY_PASSWORD; Enter if it is the same): '
read -r FM_KP
restore_echo
printf '\n'
FM_KP=${FM_KP:-$FM_SP}
export FM_SP FM_KP

if ! keytool -list -keystore "$ks" -storepass:env FM_SP -alias "$alias" >/dev/null 2>&1; then
    echo "That password or alias does not open the keystore. Nothing was written." >&2
    exit 1
fi

if [ -f "$props" ]; then
    mv "$props" "$props.before-$(date +%Y%m%d-%H%M%S)"
fi
umask 177
{
    echo "storeFile=$ks"
    echo "storePassword=$FM_SP"
    echo "keyAlias=$alias"
    echo "keyPassword=$FM_KP"
} > "$props"
echo "Done: $props is written (readable only by you). Local release builds will be signed again."
