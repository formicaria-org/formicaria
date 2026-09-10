#!/bin/sh
# Run the gate the way a fresh machine would, so this class of bug is found here and not on CI.
#
# **Why this exists.** `ci.yml` last passed 2026-07-17 and was not looked at again until
# 2026-09-10. In between the suite roughly doubled, and when it finally ran it failed **eight
# times in a row, for eight different reasons** — every one of them a check that passed only
# because of something this machine already had:
#
#   1. a global `git config user.name`, inherited by fixtures that never set one
#   2. a leftover `target/debug/deps/fm`, hiding a race in the fixture that copies it
#   3. a fast CPU, hiding a 100 ms budget that needs 113 ms on a 4-core runner
#   4. a `/usr/include` that happens to agree with conda's compiler, where the runner's does not
#   5. a warm `rust-cache`, which meant `libgit2-sys` was never actually compiled
#   6. a single timing sample, read as signal when it was scheduling noise
#   7. a locale — `en_US.UTF-8` sorts `windows_aarch64` before `windows-link`, C collation does not
#
# None was a regression. All were written after the last remote run, and all were green here.
# **Verifying the input is not verifying the output**, and "it passes locally" was the input.
#
# So this reproduces what a runner does not have. It is not a sandbox and makes no claim to
# isolate anything — it removes the specific things that were found to be doing the hiding.
#
#   sh ci/like-a-runner.sh
#
# Run it before pushing something the gate has to survive, and after touching anything that reads
# the environment: a test that shells out to git, one that times something, one that sorts.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

# **A home with the caches but without the git config.** The first version of this script pointed
# `HOME` at an empty directory, and `third-party-check` promptly died with "pkgs is not iterable" —
# `pnpm` had lost its store, which a runner *does* have. Blinding too much invents failures; that
# is the same mistake as blinding `git(1)` without `libgit2`, made twice in one day.
#
# So the caches are linked through and only the git configuration is withheld: no `.gitconfig`, and
# no `.config` (which is where the XDG git config would be). `.local` and `.cache` carry the pnpm
# store and the rattler cache, neither of which is what differs between here and a runner.
tmp_home=$(mktemp -d)
trap 'rm -rf "$tmp_home"' EXIT INT TERM
for d in .local .cache; do
    if [ -e "$HOME/$d" ]; then ln -s "$HOME/$d" "$tmp_home/$d"; fi
done

echo "like-a-runner: HOME=$tmp_home (caches linked; no git config, for git AND libgit2)"

# **The leftover that hid the merge-driver race.** Three `fm-cli` suites copy the built `fm` beside
# their test binary; a stamped copy from an earlier run makes every thread return early, so the
# race in that copy never happens. A runner has no such copy.
rm -f target/debug/deps/fm target/debug/deps/fm.stamp target/debug/deps/fm.*.tmp
echo "like-a-runner: cleared the fm driver leftovers in target/debug/deps"

# `CARGO_HOME` deliberately keeps pointing at the real one. Moving it would re-download the whole
# registry to prove nothing — the registry is not what differs between here and a runner.
#
# `GIT_CONFIG_GLOBAL` is a git(1) feature that **libgit2 ignores**, which is why `HOME` moves too.
# Blinding only one of them makes the two backends disagree about the world and produces a *false*
# failure in `git_differential`, which is correct and was nearly "fixed" on the strength of it.
#
# `LC_ALL=C` is the runner's collation. `RUST_TEST_THREADS=4` is its core count, which is what
# turns a fixture race from theoretical into reproducible.
exec env \
    HOME="$tmp_home" \
    CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
    GIT_CONFIG_GLOBAL=/dev/null \
    GIT_CONFIG_SYSTEM=/dev/null \
    LC_ALL=C \
    RUST_TEST_THREADS=4 \
    pixi run ci
