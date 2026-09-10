#!/bin/sh
# Start formicaria on macOS from an unpacked release archive.
#
# **`.command` rather than a plain script** because that is the extension Finder will run on a
# double-click. A Terminal window appears alongside the app and must stay open — that is a
# property of `.command`, not something this script can avoid, and the README says so rather than
# pretending otherwise.
#
# **The first launch needs right-click -> Open, not a double-click.** These binaries are not
# signed or notarised, so Gatekeeper refuses a plain double-click. Right-click -> Open offers an
# "Open anyway" button; after once, double-click works. The alternative is a paid Apple developer
# account, which is why the README documents the click instead of shipping a signature.
#
# `dirname $0` is load-bearing here: Finder launches with the working directory set to the user's
# home, so anything relative would resolve against the wrong folder entirely.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# See formicaria.sh — both vault variables are set together, deliberately.
FM_VAULT="$here/vault"
FM_VAULTS="$here/vaults.json"
FM_OPEN=1
FM_AUTO_SHUTDOWN=1
# **The generation of this launcher**, read by the updater. 2 means "this folder can put the
# previous version back if an update fails" — the block below. The app refuses to update itself
# when this is unset, because a folder with an older launcher has no way back.
FM_LAUNCHER=2
export FM_VAULT FM_VAULTS FM_OPEN FM_AUTO_SHUTDOWN FM_LAUNCHER

# **If the program is missing, put back the one that was working.** This is the whole safety net
# behind updating in place (`decisions.md`, 2026-09-10).
#
# The updater renames `program/` aside to `.fm-backup-<version>` and then renames the new one in, so
# the only moment `program/fm-serve` can be absent is *between those two renames* — or after an
# update that stopped partway. Either way the version that was working is sitting right here, and
# putting it back is a move, not a repair.
#
# **It lives in the launcher rather than in the app because the app is what is missing.** A person
# with no terminal has exactly one gesture — double-clicking this file — so that gesture has to be
# the recovery too. It costs one test on every ordinary start.
#
# **Two ways an update can leave you stuck, and this handles both.** The program can be *missing* —
# interrupted between the two renames — or it can be *there and unable to start*, which no amount of
# looking at the folder reveals. The second is why there is a counter: this script adds one on every
# start, and formicaria removes it once it has been serving for a moment, so three starts that never
# get that far mean the new version does not run on this computer.
fm_restore() {
    for b in "$here"/.fm-backup-*; do
        if [ -x "$b/fm-serve" ]; then
            rm -rf "$here/program"
            mv "$b" "$here/program"
            rm -f "$here/.fm-attempts"
            return 0
        fi
    done
    return 1
}

attempts=0
if [ -f "$here/.fm-attempts" ]; then
    attempts=$(cat "$here/.fm-attempts" 2>/dev/null || echo 0)
fi
# Anything that is not a plain number counts as none: a corrupted counter must not roll anyone back.
case "$attempts" in '' | *[!0-9]*) attempts=0 ;; esac

#
# **Only reset the count when something was actually put back.** Zeroing it regardless turns "there
# is nothing to go back to" into an endless 1, 2, 3, "putting the previous version back", 1, 2, 3 —
# a sentence that is false, changing nothing, offered forever to somebody with no terminal. That
# state is one deliberate button press away now that going back is a thing a user can choose.
if [ ! -x "$here/program/fm-serve" ] || [ "$attempts" -ge 3 ]; then
    if fm_restore; then
        echo "formicaria: putting the previous version back."
        attempts=0
    elif [ ! -x "$here/program/fm-serve" ]; then
        echo "formicaria: this copy is damaged and there is no earlier version here to go back to."
        echo "Download formicaria again: https://github.com/formicaria-org/formicaria/releases/latest"
        echo "Your notes are in the 'vault' folder beside this file and have not been touched."
        exit 1
    else
        echo "formicaria: this version will not start properly and there is no earlier one here."
        echo "Download formicaria again: https://github.com/formicaria-org/formicaria/releases/latest"
        echo "Your notes are in the 'vault' folder beside this file and have not been touched."
    fi
fi

# Count this start. Cleared by formicaria itself once it is up, so it only ever accumulates across
# starts that failed.
if [ -x "$here/program/fm-serve" ]; then
    echo $((attempts + 1)) > "$here/.fm-attempts" 2>/dev/null || true
fi

mkdir -p "$FM_VAULT"

exec "$here/program/fm-serve"
