#!/bin/sh
# **Does git work over HTTPS from iOS?** — rung 4, and the one result that can end the port.
#
# `libgit2-sys` picks its TLS backend by target string: WinHTTP on Windows, **SecureTransport on
# every `apple` target**, OpenSSL elsewhere. So on iOS the vendored OpenSSL this project builds is
# never called for git, and `git_native::add_certs_from_pem` — the entire Android CA workaround — is
# `cfg`'d off (`git_native.rs:386`). That leaves the iOS HTTPS path resting on a claim nothing has
# ever executed: *"SecureTransport uses the system trust store, so it just works."* If it does not,
# formicaria on an iPhone cannot sync, and a notes app whose notes cannot leave the device is not
# worth distributing.
#
# **Why a test binary and not the app.** `crates/fm-core/tests/https_remote.rs` calls the shipping
# path (`git_native::probe`) and is the *same assertion* that passes on Linux, so this run varies the
# platform and nothing else. Driving the app instead would need UI automation we do not have.
#
# **The unknown this script exists to settle** is whether `xcrun simctl spawn` will run a cargo test
# binary at all. It either does — and we get the answer for ~$0.30 — or it does not, and the log says
# so plainly and the fallback is an env-gated probe inside the shell (`simctl launch` passes
# `SIMCTL_CHILD_*`, and the app's own log file works as of ff4b060).
#
#   sh ci/ios-https-probe.sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
OUT="$root/target/ios-https-probe/$(date +%Y%m%d-%H%M%S)"

say()  { echo "ios-https-probe: $*"; }
fail() {
    echo "FAIL: $*" >&2
    if [ -d "$OUT" ]; then echo "  artifacts: $OUT" >&2; fi
    exit 1
}

[ "$(uname -s)" = "Darwin" ] || fail "this needs macOS (xcrun, simctl). There is no Mac in this
  project; the only one it has is a GitHub runner — see .github/workflows/ios.yml."
command -v xcrun >/dev/null 2>&1 || fail "xcrun is not on PATH"
mkdir -p "$OUT"

. "$root/ci/lib/simctl.sh"

# ---------------------------------------------------------------------------
# 1) Build the test binary for the Simulator triple.
#
# `--no-run` builds and prints where it put the executable. The path carries a content hash, so it
# is **parsed, never guessed** — and because `--test https_remote` narrows the build to one target,
# exactly one `Executable` line can match. The same parse is exercised on Linux against the host
# target before this is ever dispatched.
# ---------------------------------------------------------------------------
say "building the test binary for aarch64-apple-ios-sim…"
( set +e
  cargo test --target aarch64-apple-ios-sim -p fm-core --features native-git \
      --test https_remote --no-run
  echo $? > "$OUT/build.rc"
) 2>&1 | tee "$OUT/build.log"
rc=$(cat "$OUT/build.rc" 2>/dev/null || echo 1)
[ "$rc" = 0 ] || fail "could not build the test binary for the Simulator triple (rc=$rc) — $OUT/build.log"

bin=$(sed -n 's|.*Executable tests/https_remote\.rs (\(.*\))$|\1|p' "$OUT/build.log" | tail -1)
[ -n "$bin" ] && [ -f "$bin" ] || fail "no test binary path in cargo's output. Expected a line like
  'Executable tests/https_remote.rs (target/.../deps/https_remote-<hash>)' in $OUT/build.log."
say "built $bin"

# ---------------------------------------------------------------------------
# 2) A booted Simulator, then run the binary inside it.
# ---------------------------------------------------------------------------
udid=$(simctl_device "$OUT/bootstatus.txt")
say "device $udid"

say "running the probe inside the Simulator…"
( set +e
  xcrun simctl spawn "$udid" "$bin" --nocapture
  echo $? > "$OUT/probe.rc"
) 2>&1 | tee "$OUT/probe.log"
prc=$(cat "$OUT/probe.rc" 2>/dev/null || echo 1)

# Only this run's simulator is deleted; see ci/lib/simctl.sh.
if [ "${created_device:-0}" = 1 ]; then
    xcrun simctl shutdown "$udid" >/dev/null 2>&1 || true
    xcrun simctl delete "$udid" >/dev/null 2>&1 || true
fi

# ---------------------------------------------------------------------------
# 3) The verdict — which must never be merely "it did not work".
# ---------------------------------------------------------------------------
if [ "$prc" = 0 ]; then
    if grep -q 'skipping: no TCP route' "$OUT/probe.log"; then
        fail "the probe SKIPPED: the Simulator had no TCP route to github.com:443, so this run
  proves nothing about TLS. The simulator shares the host's network, so this is a runner
  networking problem, not an iOS answer. ($OUT/probe.log)"
    fi
    say "PASS — libgit2 reached an HTTPS remote from iOS. SecureTransport works."
    say "artifacts: $OUT"
    exit 0
fi

echo "--- probe.log ---" >&2; cat "$OUT/probe.log" >&2 2>/dev/null || true
if grep -q 'TCP to github.com:443 succeeded' "$OUT/probe.log"; then
    fail "**TLS, not connectivity.** The Simulator reached github.com:443 and libgit2 still could
  not talk to it — so this is SecureTransport or its system trust store, which is exactly the
  claim rung 4 exists to test. This is the kill criterion for the iPhone port: record it and stop
  rather than iterating. ($OUT/probe.log)"
fi
fail "the probe did not run cleanly (rc=$prc). If the log shows that 'simctl spawn' refused the
  binary, that is the documented unknown and NOT an iOS answer — fall back to the env-gated probe
  inside the app shell (simctl launch passes SIMCTL_CHILD_*, and the app's log file works).
  ($OUT/probe.log)"
