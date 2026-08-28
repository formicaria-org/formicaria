#!/bin/sh
# Render the manual for ONE operating system — the one this build is for.
#
# **Why the manual is not the same on every platform.** We already ship a separate archive per OS,
# so the Windows zip knows it is Windows. A setup chapter that lists three platforms makes every
# reader skip two-thirds of it and work out which third is theirs — and the one thing a first-time
# reader must not have to do is choose. So `user/setup.md` is *assembled*, not written: the three
# variants are tracked, the assembled file is generated and gitignored.
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

# Generated, and stamped as such: this file is in .gitignore, and an editor who does not know that
# would lose their work to the next build. The banner is the only warning they will get.
{
    printf '<!-- GENERATED from %s by ci/docs.sh — do not edit. Edit that file instead. -->\n' "$src"
    cat "$src"
} > docs/src/user/setup.md

echo "docs: manual for $os"
mdbook build docs
