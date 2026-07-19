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
            name = p[1];
            lic  = ($2 == "" ? "(unstated)" : $2);
            # OVERRIDES — crates whose declared licence is not the whole truth.
            #
            # `cargo tree` reports the `license` field, and so does `cargo deny`. A crate that
            # vendors third-party source under a different licence declares only its own, and
            # both gates believe it. That is not a hypothetical: it is why the entry below
            # exists, and it is the exact gap `deny.toml` documents at length.
            #
            # This file generates the notices that MUST travel with a binary, so a wrong row
            # here is a licence violation shipped in the artifact. Overriding is therefore not
            # "being helpful" — it is the only place the truth gets recorded.
            #
            # libgit2-sys declares MIT OR Apache-2.0, which covers its Rust wrapper. It also
            # vendors libgit2 itself: GPL-2.0-only WITH a linking exception (read from the
            # vendored COPYING). Both ship, so both are stated.
            if (name == "libgit2-sys")
                lic = "(MIT OR Apache-2.0) AND (GPL-2.0-only WITH linking exception) — vendors libgit2";
            printf "| %s | %s | %s |\n", name, (n>1 ? p[2] : "-"), lic
        }' \
      | sort -u
    echo
    echo "Full licence texts: <https://spdx.org/licenses/>. formicaria's own licence is MIT,"
    echo "in \`LICENSE\`."
} > "$OUT"

echo "wrote $OUT ($(grep -c '^| ' "$OUT") crates)"
