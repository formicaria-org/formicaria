#!/bin/sh
# Generate THIRD-PARTY.md — the notices that MUST travel with a shipped binary.
#
# MIT and Apache-2.0 both require their copyright notice to be included in binary
# distributions, and `fm-serve` statically links ~65 crates under them. Shipping the
# binaries without this is a licence violation, however permissive the licences are.
#
# Generated from the real dependency tree at build time, never hand-maintained: a
# hand-written list is a list that silently goes stale the first time someone adds a crate.
set -eu
OUT="${1:-THIRD-PARTY.md}"

{
    echo "# Third-party notices"
    echo
    echo "formicaria's binaries statically link the crates below. Both MIT and Apache-2.0"
    echo "require their notices to travel with a binary, so they travel here."
    echo
    echo "Generated from the dependency tree by \`ci/third-party.sh\`; do not edit."
    echo
    echo "| Crate | Version | Licence |"
    echo "|---|---|---|"
    # One row per crate actually linked into the shipped binaries. `--prefix none` flattens
    # the tree; sort -u collapses the many diamond dependencies.
    for pkg in fm-serve fm-cli; do
        cargo tree -p "$pkg" --prefix none --format '{p}|{l}' --no-dedupe 2>/dev/null || true
    done \
      | grep -v '^$' \
      | sed 's/ (proc-macro)//; s| ([^)]*)||' \
      | sort -u \
      | awk -F'|' 'NF==2 {
            n = split($1, p, " ");
            printf "| %s | %s | %s |\n", p[1], (n>1 ? p[2] : "-"), ($2 == "" ? "(unstated)" : $2)
        }' \
      | sort -u
    echo
    echo "Full licence texts: <https://spdx.org/licenses/>. formicaria's own licence is in"
    echo "\`LICENSE-MIT\` / \`LICENSE-APACHE\`."
} > "$OUT"

echo "wrote $OUT ($(grep -c '^| ' "$OUT") crates)"
