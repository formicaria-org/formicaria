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

# Resolve the repo root from this script's own location: `release.yml` calls us with a
# staged output path, and the npm tree has to be found regardless of where OUT points.
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

# Emit the npm rows. Kept as a function so the heredoc below stays readable, and so the
# failure modes are loud: this file is a legal notice, and a notice that quietly emits an
# empty table is worse than no notice, because it looks like an answer.
npm_licences() {
    [ -d "$root/ui/node_modules" ] || {
        echo "third-party: $root/ui/node_modules is missing — run 'pnpm -C ui install' first." >&2
        echo "  The npm notices cannot be generated without the installed tree, and emitting" >&2
        echo "  an empty table would silently understate what the binary carries." >&2
        exit 1
    }
    ( cd "$root/ui" && pnpm licenses list --prod --json ) | node -e '
        let raw = "";
        process.stdin.on("data", d => raw += d);
        process.stdin.on("end", () => {
            const byLicence = JSON.parse(raw);

            // OVERRIDES — same reasoning as the crate overrides below, different manifest.
            // `pnpm licenses list` reads the package.json `license` field and nothing else, so a
            // package that ships its licence only as a FILE is reported "Unknown".
            //
            // khroma 2.1.0 (a mermaid dependency) is exactly that: no `license` field, and a
            // `license` file reading "The MIT License (MIT) / Copyright (c) 2019-present Fabio
            // Spampinato, Andrew Maney". Read on 2026-09-03.
            const OVERRIDE = { khroma: "MIT (declared in its `license` file, not package.json)" };

            // dompurify is dual-licensed `(MPL-2.0 OR Apache-2.0)`. We ELECT Apache-2.0, which
            // keeps every notice here permissive-only and matches `deny.toml`s posture for
            // crates. Recording the election is the point: a dual licence is a choice someone
            // made, and an unrecorded choice is one nobody can audit later.
            const ELECT = { dompurify: "Apache-2.0 (elected from `MPL-2.0 OR Apache-2.0`)" };

            const rows = [];
            for (const [licence, pkgs] of Object.entries(byLicence)) {
                for (const p of pkgs) {
                    const lic = OVERRIDE[p.name] || ELECT[p.name] || licence;
                    rows.push([p.name, (p.versions || []).join(", ") || "-", lic]);
                }
            }
            // **Codepoint order, not `localeCompare`** (2026-09-10). ICU collation is
            // locale-dependent: under `en_US.UTF-8` punctuation is largely ignored, so
            // `windows_aarch64_gnullvm` sorts before `windows-link`, while a runner with no locale
            // set uses C collation and puts `-` (0x2D) before `_` (0x5F). The generated file then
            // differed by machine, and `third-party-check` failed on CI because the committed copy
            // carried whichever order the last generating machine happened to have.
            //
            // No apostrophes in this block: the whole script is one single-quoted shell string.
            rows.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));

            const unknown = rows.filter(r => /^Unknown$/i.test(r[2])).map(r => r[0]);
            if (unknown.length) {
                console.error("third-party: no licence could be determined for: " + unknown.join(", "));
                console.error("  Read each package licence file and add it to OVERRIDE in");
                console.error("  ci/third-party.sh. An \"Unknown\" row in a notice is a defect,");
                console.error("  not information.");
                process.exit(1);
            }

            console.log("| Package | Version | Licence |");
            console.log("|---|---|---|");
            for (const r of rows) console.log("| " + r.join(" | ") + " |");
        });
    '
}

{
    echo "# Third-party notices"
    echo
    echo "formicaria's binaries statically link the crates below. Both MIT and Apache-2.0"
    echo "require their notices to travel with a binary, so they travel here."
    echo
    echo "The crate table covers **every supported platform**, not just the one this file was"
    echo "generated on: some crates are reached only on Windows or macOS, and a notice listing"
    echo "one host's arms would understate what those builds carry."
    echo
    echo "Generated from the dependency tree by \`ci/third-party.sh\`; do not edit."
    echo
    echo "| Crate | Version | Licence |"
    echo "|---|---|---|"
    # One row per crate linked into the shipped binaries, on ANY supported platform.
    # `--prefix none` flattens the tree; sort -u collapses the many diamond dependencies.
    #
    # **`--target all`, not the host.** Without it `cargo tree` resolves for whatever machine
    # happens to be running, and this file is committed at the repo root where a stranger reads
    # it — so a Linux-generated notice would silently omit every Windows-only arm, `libgit2-sys`
    # among them, which is the one crate here carrying GPL-2.0-only code (see its override
    # below). It also makes the notice REPRODUCIBLE: `ci/third-party-check.sh` diffs this file
    # against a fresh run, and a host-dependent table would fail that check on every OS but the
    # one it was generated on. A superset over-attributes, which is the safe direction for a
    # legal notice; under-attributing is the one that is a violation.
    for pkg in fm-serve fm-cli; do
        cargo tree -p "$pkg" --target all --prefix none --format '{p}|{l}' --no-dedupe 2>/dev/null || true
    done \
      | grep -v '^$' \
      | sed 's/ (proc-macro)//; s| ([^)]*)||' \
      | LC_ALL=C sort -u \
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
            # openssl-src vendors OpenSSL itself. Apache-2.0 is already on the allowlist so this
            # is not a policy question, but the notice must still say whose code ships.
            if (name == "openssl-src")
                lic = "(MIT OR Apache-2.0) AND Apache-2.0 — vendors OpenSSL";
            printf "| %s | %s | %s |\n", name, (n>1 ? p[2] : "-"), lic
        }' \
      | LC_ALL=C sort -u
    echo
    # ------------------------------------------------------------------------------------
    # **What `cargo tree` cannot see, part one: the user interface.**
    #
    # `crates/fm-serve/build.rs` bakes `ui/dist` into the binary. So `marked`, `katex`,
    # `mermaid`, `dompurify`, Excalidraw, React and Svelte ship inside `fm-serve` exactly as
    # the crates above do — and MIT and Apache-2.0 ask the same of them. This section existed
    # nowhere until 2026-09-03, because every gate in this repo is keyed on `Cargo.lock`:
    # `cargo tree` cannot see npm and neither can `cargo deny`. A gate keyed on one manifest
    # is structurally blind to everything the binary carries that is not in it.
    #
    # `--prod` is the shipped closure and not a convenience: `devDependencies` are the
    # toolchain (vite, typescript, vitest, jsdom) and are not in the bundle. `svelte` and
    # `@tauri-apps/api` were moved into `dependencies` on the same day for this reason —
    # Svelte 5 compiles components against its own client runtime and `@tauri-apps/api/core`
    # is imported by `ipc.ts`, so both execute in shipped code and calling them "dev" was
    # simply wrong.
    echo "## Bundled into the user interface (npm)"
    echo
    echo "\`ui/dist\` is compiled into \`fm-serve\` by \`crates/fm-serve/build.rs\`, so these"
    echo "ship inside the binary. Generated from the production dependency closure"
    echo "(\`pnpm licenses list --prod\`); build-only tooling is deliberately absent."
    echo
    npm_licences
    echo
    echo
    # ------------------------------------------------------------------------------------
    # **What `cargo tree` cannot see, part two: the fonts.**
    #
    # `ui/scripts/copy-excalidraw-fonts.mjs` copies eight font families out of the
    # `@excalidraw/excalidraw` package into `ui/public/fonts/`, which Vite emits and
    # `build.rs` bakes in — so the font *binaries* ship. The OFL requires its notice to
    # travel with them.
    #
    # **This table is typed, not generated, and that is not laziness.** The npm package
    # ships the `.woff2` files with no licence metadata whatsoever — no `license` field, no
    # LICENSE file beside them — and upstream Excalidraw does not carry one in its own
    # `fonts/` directory either. There is nothing on disk to read. Each row below was
    # verified against that font's own upstream project on 2026-09-03, and `ci/checks.sh`
    # fails if the set of families this table names ever stops matching the set the copier
    # actually copies. So the licences are typed; the *list* cannot drift silently.
    #
    # ComicShanns is the reason to check rather than assume: everything else here is OFL and
    # it is MIT.
    echo "## Fonts bundled with the whiteboard"
    echo
    echo "Copied out of \`@excalidraw/excalidraw\` by \`ui/scripts/copy-excalidraw-fonts.mjs\` and"
    echo "baked into the binary, so the whiteboard never fetches a font from the internet."
    echo "Xiaolai (CJK) is deliberately **not** bundled — 13 of the 14 MB — so CJK whiteboard"
    echo "text falls back to a system font."
    echo
    echo "| Family | Licence | Copyright | Upstream |"
    echo "|---|---|---|---|"
    echo "| Excalifont | OFL-1.1 | Excalidraw | <https://plus.excalidraw.com/excalifont> |"
    echo "| Virgil | OFL-1.1 | Excalidraw (Ellinor Rapp) | <https://github.com/excalidraw/virgil> |"
    echo "| ComicShanns | **MIT** | Copyright (c) 2018 Shannon Miwa | <https://github.com/shannpersand/comic-shanns> |"
    echo "| Nunito | OFL-1.1 | Copyright 2014 The Nunito Project Authors | <https://github.com/googlefonts/nunito> |"
    echo "| Assistant | OFL-1.1 | Copyright 2020 The Assistant Project Authors | <https://github.com/hafontia/Assistant> |"
    echo "| Lilita One | OFL-1.1 | Copyright (c) 2011 Juan Montoreano, with Reserved Font Name Lilita | <https://fonts.google.com/specimen/Lilita+One> |"
    echo "| Cascadia Code | OFL-1.1 | Copyright (c) 2019-Present Microsoft Corporation, with Reserved Font Name Cascadia Code | <https://github.com/microsoft/cascadia-code> |"
    echo "| Liberation Sans | OFL-1.1 | Red Hat, Inc. | <https://github.com/liberationfonts/liberation-fonts> |"
    echo
    echo "OFL-1.1 in full: <https://openfontlicense.org/open-font-license-official-text/>."
    echo
    # **What `cargo tree` cannot see, part three: the model runtime.** The study assistant's model runtime is not linked into any
    # binary here — it is downloaded onto the user's machine on first enable and executed as a
    # separate process. `cargo tree` therefore knows nothing about it, and this notice would be
    # silently incomplete about software this project chose, pinned and put on their disk.
    #
    # `ensure_runtime` also extracts each archive's own LICENSE next to the binary, so the notice
    # travels with the bytes as well as appearing here. Both, deliberately: this file is what a
    # reader checks before downloading, and the extracted copy is what survives the app.
    echo "## Downloaded at runtime (the optional study assistant)"
    echo
    echo "Not part of these binaries and not present unless the assistant is turned on. Pinned by"
    echo "URL **and SHA-256** in \`models.toml\`, fetched once, and kept under this machine's"
    echo "configuration directory."
    echo
    echo "| Component | Licence | Source |"
    echo "|---|---|---|"
    echo "| llama.cpp (\`llama-server\` + \`ggml\`) | MIT | <https://github.com/ggml-org/llama.cpp> |"
    echo "| whisper.cpp (\`whisper-server\`) | MIT | <https://github.com/ggml-org/whisper.cpp> |"
    echo
    echo "Model **weights** are downloaded from Hugging Face and carry their own terms, stated in"
    echo "the app before anything is fetched and recorded per model in \`models.toml\`."
    echo
    echo "Full licence texts: <https://spdx.org/licenses/>. formicaria's own licence is MIT,"
    echo "in \`LICENSE\`."
} > "$OUT"

# Report each table separately. One total over every `| ` line was wrong the moment this file
# grew a second table, and a wrong count in the one line a human reads is how nobody notices
# that a whole section stopped being emitted.
crates=$(awk '/^## Bundled into the user interface/{exit} /^\| /{n++} END{print n+0}' "$OUT")
npm=$(awk '/^## Bundled into the user interface/{i=1} /^## Fonts bundled/{i=0} i && /^\| /{n++} END{print n+0}' "$OUT")
fonts=$(awk '/^## Fonts bundled/{i=1} /^## Downloaded at runtime/{i=0} i && /^\| /{n++} END{print n+0}' "$OUT")
# Each table contributes one `| ` header line; the `|---|` separator does not match.
echo "wrote $OUT ($((crates - 1)) crates, $((npm - 1)) npm packages, $((fonts - 1)) font families)"
