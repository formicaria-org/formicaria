#!/bin/sh
# Move your notes into this newer formicaria. Double-click this.
#
# **Run this from the NEW folder, and it only ever copies INTO it.** The old folder is never
# written to and never deleted, so until you are satisfied it is a complete backup of everything
# you had. That is the property that makes this safe to run at all: the worst outcome of a mistake
# here is a folder you can throw away, not a notebook you cannot get back.
#
# Why this script exists. Each release unpacks into its own folder — `formicaria-v0.2.1-…`, then
# `formicaria-v0.2.2-…` — and your notes live in `vault/` INSIDE the one you have been using. So a
# new download starts empty, showing only the "Start here" note, and it looks as though your work
# is gone. It is not: it is still in the old folder. This copies it across.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

say() { printf '%s\n' "$*"; }

# Notes in a folder's vault, counted without depending on a shell glob that may match nothing.
count_notes() {
    find "$1/vault/notes" -maxdepth 1 -type f -name '*.md' 2>/dev/null | wc -l | tr -d ' '
}

say ""
say "  Moving your notes into this version of formicaria"
say "  ================================================"
say ""
say "  This folder:  $here"
say ""

# ---------------------------------------------------------------------------
# 1. Refuse if this folder has already been used.
#
# Three signs, any one of which means a real notebook: the vault list written on first start, the
# history folder git keeps, and the per-machine search index. A freshly unpacked download has none
# of them. Checking for those beats counting notes, because it distinguishes "never started" from
# "started, and you deleted the welcome note" — and copying a second notebook on top of a live one
# would mix two people's work with no way to tell them apart afterwards.
# ---------------------------------------------------------------------------
used=""
if [ -e "$here/vaults.json" ]; then used="vaults.json"; fi
if [ -d "$here/vault/.git" ]; then used="${used:+$used, }vault/.git"; fi
if [ -e "$here/vault/index.sqlite" ]; then used="${used:+$used, }vault/index.sqlite"; fi
if [ -e "$here/update.log" ]; then used="${used:+$used, }update.log"; fi

if [ -n "$used" ]; then
    say "  This folder has already been used ($used)."
    say ""
    say "  Nothing has been changed. Copying another notebook on top of this one would mix"
    say "  the two together, and there would be no way to separate them afterwards."
    say ""
    say "  If you meant to start again, unpack a fresh copy of the download and run this"
    say "  script in THAT folder instead."
    say ""
    exit 1
fi

# ---------------------------------------------------------------------------
# 2. Find the folder to copy from.
#
# An argument wins, because dragging a folder onto this script is how someone without a terminal
# says which one they mean. Otherwise look beside this folder for exactly one older formicaria
# that has notes in it. Exactly one — with several, this script says what it found and stops,
# because guessing which notebook is yours is not a decision a script should make.
# ---------------------------------------------------------------------------
old=""
if [ $# -ge 1 ] && [ -n "${1:-}" ]; then
    old=$(CDPATH= cd -- "$1" 2>/dev/null && pwd) || {
        say "  That is not a folder I can open: $1"
        exit 1
    }
fi

if [ -z "$old" ]; then
    parent=$(dirname -- "$here")
    found=""
    n=0
    for d in "$parent"/formicaria-*; do
        if [ ! -d "$d" ]; then continue; fi
        full=$(CDPATH= cd -- "$d" && pwd)
        if [ "$full" = "$here" ]; then continue; fi
        if [ "$(count_notes "$full")" -eq 0 ]; then continue; fi
        found="${found}${found:+
}$full"
        n=$((n + 1))
    done

    if [ "$n" -eq 0 ]; then
        say "  I could not find an older formicaria folder beside this one."
        say ""
        say "  Drag the old folder onto this script to say where it is. Or copy its \"vault\""
        say "  folder into this one by hand — that is all this script does."
        say ""
        exit 1
    fi
    if [ "$n" -gt 1 ]; then
        say "  I found more than one older folder with notes in it:"
        say ""
        printf '%s\n' "$found" | sed 's/^/      /'
        say ""
        say "  Drag the one you want onto this script, so the choice is yours and not mine."
        say ""
        exit 1
    fi
    old=$found
fi

if [ "$old" = "$here" ]; then
    say "  That is this same folder. Nothing to do."
    exit 1
fi
if [ ! -d "$old/vault" ]; then
    say "  There is no \"vault\" folder in:"
    say "      $old"
    say ""
    say "  That does not look like a formicaria folder."
    exit 1
fi

notes=$(count_notes "$old")
history="no"
if [ -d "$old/vault/.git" ]; then history="yes"; fi

# ---------------------------------------------------------------------------
# 3. Say what will happen, then ask.
#
# With no terminal attached there is nothing to ask with, so print the command and stop rather
# than acting unasked on someone's only copy.
# ---------------------------------------------------------------------------
say "  Copy FROM:    $old"
say "                $notes note(s), history: $history"
say "  Copy INTO:    $here"
say ""
say "  The old folder is not changed and not deleted. Keep it until you are sure."
say ""

if [ ! -t 0 ]; then
    say "  Run this in a terminal to confirm:"
    say ""
    say "      \"$here/$(basename -- "$0")\" \"$old\""
    say ""
    exit 1
fi

printf '  Type yes to continue: '
read -r answer
if [ "$answer" != "yes" ]; then
    say ""
    say "  Nothing was changed."
    exit 1
fi
say ""

# ---------------------------------------------------------------------------
# 4. Copy.
#
# `vault/.` copies the folder's CONTENTS including the dotted ones, so `.git` — your history —
# comes too. `blobs/`, `views/`, `themes/` and `manifest.json` all live inside the vault and are
# carried by the same copy.
#
# **`index.sqlite` is deliberately not kept.** It is a per-machine search index, rebuilt from the
# notes on the next start; `fm_core::acquire::naturalise` removes it for the same reason whenever a
# vault arrives from somewhere else. Copying it is not dangerous, only pointless — and a stale
# index that looks current is worse than none.
#
# **`vaults.json` is deliberately NOT copied**, and this is the subtle one. It records each vault's
# ABSOLUTE path, so the old folder's copy points at the old folder's vault. Bring it across and this
# newer formicaria would quietly keep writing into the folder you are about to delete. Left alone,
# the launcher's own `FM_VAULT` points at the vault right here, and the file is written afresh on
# the first start.
# ---------------------------------------------------------------------------
mkdir -p "$here/vault"
cp -R "$old/vault/." "$here/vault/"
rm -f "$here/vault/index.sqlite"

# ---------------------------------------------------------------------------
# 5. Check it actually arrived, and say so in numbers rather than reassurance.
# ---------------------------------------------------------------------------
copied=$(count_notes "$here")
if [ "$copied" -lt "$notes" ]; then
    say "  SOMETHING WENT WRONG."
    say ""
    say "  Expected $notes note(s) here, found $copied."
    say "  Your old folder has not been touched — everything is still in:"
    say "      $old"
    say ""
    exit 1
fi

{
    printf 'updated %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'from    %s\n' "$old"
    printf 'notes   %s\n' "$copied"
} > "$here/update.log"

# Two numbers, because they differ and the difference is confusing if you meet it unexplained:
# the download brought its own "Start here" note, so this folder holds one more than was copied.
say "  Done. $notes note(s) copied. This folder now holds $copied."
if [ "$copied" -gt "$notes" ]; then
    say "  (The extra is the \"Start here\" note this download came with. Delete it whenever.)"
fi
say ""

# Vaults kept OUTSIDE the old folder were registered in its `vaults.json` by absolute path, and
# that file is not brought across. The notes themselves are untouched wherever they live — but the
# app will not know about them until they are added again, so name them rather than let someone
# discover the absence later.
if [ -e "$old/vaults.json" ]; then
    others=$(grep -o '"path"[[:space:]]*:[[:space:]]*"[^"]*"' "$old/vaults.json" 2>/dev/null \
        | sed 's/.*"\([^"]*\)"$/\1/' \
        | grep -v "^$old/vault$" || true)
    if [ -n "$others" ]; then
        say "  Your old setup also knew about these notebooks, which live outside that folder:"
        say ""
        printf '%s\n' "$others" | sed 's/^/      /'
        say ""
        say "  They are untouched on disk. Add them again in the app: Back up -> New vault."
        say ""
    fi
fi

say "  Now start this version, and check your notes are here before deleting the old folder."
say ""
