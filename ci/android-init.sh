#!/bin/sh
# Unpack the Android toolchain into `.android/`, verifying every artifact against
# `android/toolchain.lock`.
#
# Nothing here is installed system-wide. Everything Google ships is a standalone zip, so it is
# fetched into a gitignored directory in this tree: nothing to uninstall, two checkouts can hold
# different versions, and the repo can assert what it actually got. A dependency installed with
# `sudo` is one this project can neither pin nor remove, which loses the reproducibility the
# pixi-only rule exists to protect (`docs/context/decisions.md`, the owner's ruling 3).
#
# Idempotent: an artifact whose stamp already matches the locked version is left alone, so this
# is safe to re-run and cheap when there is nothing to do.
#
# NOT part of `pixi run ci`, by ruling: a contributor with no Android toolchain must still get a
# green gate. This is opt-in, and it is the only thing that writes `.android/`.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lock="$root/android/toolchain.lock"
dest="$root/.android"

[ -f "$lock" ] || { echo "android-init: no $lock" >&2; exit 1; }

# Fail on the tool, not on a confusing error 40 lines later. These are coreutils-level and
# present on any machine that could already have run pixi.
for t in curl unzip sha256sum; do
    command -v "$t" >/dev/null 2>&1 || {
        echo "android-init: needs '$t' on PATH" >&2
        exit 1
    }
done

case "$(uname -s)" in
    Linux) host_os=linux ;;
    Darwin) host_os=macos ;;
    *) echo "android-init: unsupported host $(uname -s) — add a line to $lock" >&2; exit 1 ;;
esac

mkdir -p "$dest"
fetched=0
skipped=0

# `read` splits on IFS and `-r` keeps backslashes literal; the lock is whitespace-separated with
# `#` comments, so the parse is one line of shell rather than a dependency.
while read -r name version os sha url || [ -n "${name:-}" ]; do
    case "$name" in ''|\#*) continue ;; esac
    [ "$os" = "$host_os" ] || [ "$os" = any ] || continue

    stamp="$dest/.stamp-$name"
    if [ -d "$dest/$name" ] && [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$version" ]; then
        printf '  %-16s %-10s already present\n' "$name" "$version"
        skipped=$((skipped + 1))
        continue
    fi

    printf '  %-16s %-10s fetching...\n' "$name" "$version"
    tmp="$dest/.$name.zip.part"
    # A progress bar, not curl's table: the NDK is ~1 GB and silence there reads as a hang.
    curl -fL --progress-bar --retry 3 -o "$tmp" "$url" || {
        rm -f "$tmp"
        echo "android-init: download failed: $url" >&2
        exit 1
    }

    got=$(sha256sum "$tmp" | cut -d' ' -f1)
    if [ "$got" != "$sha" ]; then
        # Delete it. Leaving an unverified archive on disk invites someone to unpack it by hand
        # and wonder later why their toolchain does not match anyone else's.
        rm -f "$tmp"
        echo "android-init: CHECKSUM MISMATCH for $name $version" >&2
        echo "  expected $sha" >&2
        echo "  got      $got" >&2
        echo "  url      $url" >&2
        echo "Refusing to unpack. Either the URL now serves different bytes, or the lock is" >&2
        echo "wrong. Do not 'fix' this by pasting the new checksum without knowing why." >&2
        exit 1
    fi

    # Replace rather than merge: an unpack over a previous version leaves both versions' files
    # interleaved, and the stamp would then describe a tree that never existed.
    rm -rf "$dest/$name"
    unzip -q -o "$tmp" -d "$dest"
    rm -f "$tmp"
    printf '%s' "$version" > "$stamp"
    fetched=$((fetched + 1))
done < "$lock"

# `avdmanager` and `sdkmanager` derive the SDK root from **their own path**, expecting to live
# at `$SDK/cmdline-tools/latest/bin/`. Anywhere else they look one directory too high, find no
# system images, and fail with the memorable "Valid system image paths are: null".
# `ANDROID_HOME` does not override it.
#
# A symlink does NOT work: the launcher resolves its own path through the link and lands back
# where it started. So this is a real copy (~165 MB), keyed on the locked version so it is made
# once and refreshed only when cmdline-tools itself changes. Wasteful, and the alternative is
# fighting a tool that has already decided where it lives.
if [ -d "$dest/cmdline-tools" ]; then
    want=$(cat "$dest/.stamp-cmdline-tools" 2>/dev/null || echo unknown)
    have=$(cat "$dest/sdk/cmdline-tools/.stamp" 2>/dev/null || echo none)
    if [ "$want" != "$have" ]; then
        echo "  installing cmdline-tools into the SDK layout the Android tools require..."
        rm -rf "$dest/sdk/cmdline-tools/latest"
        mkdir -p "$dest/sdk/cmdline-tools"
        cp -a "$dest/cmdline-tools" "$dest/sdk/cmdline-tools/latest"
        printf '%s' "$want" > "$dest/sdk/cmdline-tools/.stamp"
    fi
fi

echo "android-init: $fetched fetched, $skipped already present -> $dest"

if [ -x "$dest/platform-tools/adb" ]; then
    cat <<EOF

adb is at .android/platform-tools/adb (not on PATH, and not installed system-wide).

To reach a phone WITHOUT USB — USB adb on Linux needs udev rules or plugdev membership,
which is exactly the system requirement this avoids:

  Phone: Developer options -> Wireless debugging -> Pair device with pairing code
  .android/platform-tools/adb pair <phone-ip>:<pair-port>    # port from the PAIRING DIALOG
  .android/platform-tools/adb devices                        # should already list the phone
  .android/platform-tools/adb reverse tcp:8765 tcp:8765

There is no 'adb connect' step. adb finds the device over mDNS and connects it itself as
soon as pairing succeeds. Running 'connect' by hand races that, and because the pairing
dialog's port is single-use and dies on success, it fails and leaves a dead second entry --
which then makes every later command answer 'more than one device/emulator'. If that
happens: 'adb disconnect' clears the strays, and 'adb devices' shows the survivor.

Then run \`pixi run serve\` and open http://127.0.0.1:8765 in the phone's browser. The
tunnel means the phone sends Host: 127.0.0.1:8765, so fm-serve's guards pass unchanged and
nothing listens on the phone itself.
EOF
fi
