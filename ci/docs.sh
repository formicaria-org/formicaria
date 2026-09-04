#!/bin/sh
# Render the manual for ONE operating system — the one this build is for.
#
# **Why the manual is not the same on every platform.** We already ship a separate archive per OS,
# so the Windows zip knows it is Windows. A setup chapter that lists three platforms makes every
# reader skip two-thirds of it and work out which third is theirs — and the one thing a first-time
# reader must not have to do is choose. So `user/setup.md` is *assembled*, not written: the three
# variants are tracked, and this script picks one.
#
# **The assembled file is COMMITTED, not gitignored** (changed 2026-09-04). It used to be ignored,
# which was tidy and wrong: `SUMMARY.md` and `introduction.md` both link to it, and those links are
# the first thing a reader follows on GitHub — where a gitignored file is a 404, and it was the
# *first* link in the manual's first list. The committed copy is whichever variant this machine
# builds (Linux, in practice, since that is where `pixi run ci` runs); each release archive still
# carries its own OS's variant, assembled on that OS's runner. Regenerating on Linux is a no-op, so
# the tree stays clean; regenerating on another OS shows up as a diff, which is the correct signal
# rather than a silent one.
#
# The OS is detected rather than passed, because the release job for each target runs ON that
# target — so `pixi run build` and `pixi run docs` on the Windows runner produce the Windows manual
# with nobody having to remember to say so. `FM_DOCS_OS` overrides it for previewing another
# platform's text without a machine of that kind.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

os="${FM_DOCS_OS:-}"
if [ -z "$os" ]; then
    case "$(uname -s 2>/dev/null || echo unknown)" in
        Darwin)                     os=macos ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT) os=windows ;;
        *)                          os=linux ;;
    esac
fi

src="docs/src/user/setup-$os.md"
[ -f "$src" ] || { echo "docs: no setup chapter for '$os' ($src)" >&2; exit 1; }

# Generated, and stamped as such: an editor who does not know that would lose their work to the
# next build. The banner is the only warning they will get — and since the file is now committed,
# it also has to tell a GitHub reader why they may be looking at another platform's chapter.
{
    printf '<!-- GENERATED from %s by ci/docs.sh — do not edit. Edit that file instead. -->\n' "$src"
    printf '<!-- The committed copy is one platform'"'"'s chapter (whichever machine last built the\n'
    printf '     manual). Each release archive carries its own. The other two are beside this file:\n'
    printf '     setup-linux.md, setup-macos.md, setup-windows.md. -->\n'
    cat "$src"
} > docs/src/user/setup.md

echo "docs: manual for $os"
mdbook build docs
