#!/bin/sh
# Put the website together: the pages in `site/`, the drawing, and the manual under `manual/`.
#
#   sh ci/site-assemble.sh <out> [<dir holding setup-macos.html and setup-windows.html>]
#
# Run by `.github/workflows/pages.yml` after `pixi run docs`, and by hand to rehearse it: the
# result is a folder a browser can open. A script and not YAML, because a step in a workflow can
# only be tried by publishing.
#
# It does three things a copy would not:
#   - **makes the manual's title the way back to the first page.** The owner, 2026-10-13: *"one can
#     easily go back to it"*. Done here and not in the book's theme, because the same book goes
#     into every release archive, where it is opened from a folder and has no first page;
#   - **checks every link from the site's own pages** that stays on the site against what was
#     built — a chapter renamed in `SUMMARY.md` is otherwise a dead link on the first page a
#     newcomer reads, and mdBook checks the book's links, not these;
#   - **refuses to finish if the way back did not land**, so a change in mdBook's markup is a
#     failed build and not a manual nobody can leave.
set -eu

out=${1:?usage: sh ci/site-assemble.sh <out> [<extra setup chapters>]}
extra=${2:-}
[ -s docs/book/index.html ] || { echo "site: build the manual first (pixi run docs)" >&2; exit 1; }

rm -rf "$out"
mkdir -p "$out/manual"
cp -R site/. "$out/"
cp docs/assets/formicaria.svg "$out/formicaria.svg"
cp -R docs/book/. "$out/manual/"
if [ -n "$extra" ]; then
    cp "$extra/setup-macos.html" "$extra/setup-windows.html" "$out/manual/user/"
fi

# The title in the bar at the top of every page of the manual becomes a link to the first page.
python3 - "$out/manual" <<'PY'
import os, sys
root = sys.argv[1]
old = '<h1 class="menu-title">formicaria</h1>'
done = 0
for d, _, files in os.walk(root):
    for f in files:
        if not f.endswith(".html"):
            continue
        p = os.path.join(d, f)
        s = open(p, encoding="utf-8").read()
        if old not in s:
            continue
        up = os.path.relpath(os.path.dirname(root), d).replace(os.sep, "/")
        new = ('<h1 class="menu-title"><a href="%s/index.html" title="Back to the first page" '
               'style="color:inherit;text-decoration:none">← formicaria</a></h1>' % up)
        open(p, "w", encoding="utf-8").write(s.replace(old, new))
        done += 1
print("site: the way back is on %d pages of the manual" % done)
sys.exit(0 if done else 1)
PY
grep -q 'Back to the first page' "$out/manual/index.html" "$out/manual/user/notes.html" \
    || { echo "site: the manual has no link back to the first page" >&2; exit 1; }

bad=0
for target in $(cat "$out"/*.html | grep -oE '(href|src)="[^"#]+' \
                | sed -E 's/^(href|src)="//' | grep -vE '^[a-z]+:' | sort -u); do
    if [ ! -s "$out/$target" ]; then
        echo "site: a page in site/ points at '$target', which was not built" >&2
        bad=1
    fi
done
[ "$bad" -eq 0 ]
echo "site: assembled in $out"
