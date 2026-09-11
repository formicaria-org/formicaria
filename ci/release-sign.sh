#!/bin/sh
# Sign a release manifest with the Ed25519 release key: a raw 64-byte detached signature written
# beside it as `<manifest>.sig`, which is exactly what `fm_update::verify` checks with `ring`.
#
# **`openssl`, not a new tool and not a compiler.** The job that holds the key runs this and nothing
# else — the same reasoning that split `android-build` from `android-sign`: every program that runs on
# a machine holding a key is a program that can read it. OpenSSL is already on every runner and in
# the pixi environment, and `pkeyutl -rawin` makes a pure Ed25519 signature that `ring` verifies.
#
#   FM_RELEASE_KEY=<pem> sh ci/release-sign.sh <manifest>
#   (default key: ~/.config/formicaria/release-signing.pem, as written by ci/release-key.sh)
set -eu

manifest=${1:?usage: release-sign.sh <manifest>}
key=${FM_RELEASE_KEY:-$HOME/.config/formicaria/release-signing.pem}
[ -f "$manifest" ] || { echo "release-sign: no manifest at $manifest" >&2; exit 1; }
[ -f "$key" ] || { echo "release-sign: no key at $key — see ci/release-key.sh" >&2; exit 1; }

openssl pkeyutl -sign -inkey "$key" -rawin -in "$manifest" -out "$manifest.sig"

# **Verify what was just written, with the public half, before anything publishes it.** A signature
# that does not verify reaches users as "the update did not come from formicaria" on every device at
# once; catching it here costs one command.
pub=$(mktemp)
trap 'rm -f "$pub"' EXIT
openssl pkey -in "$key" -pubout -out "$pub"
if ! openssl pkeyutl -verify -pubin -inkey "$pub" -rawin -in "$manifest" -sigfile "$manifest.sig" >/dev/null; then
    echo "release-sign: the signature just made does not verify" >&2
    rm -f "$manifest.sig"; exit 1
fi
if [ "$(wc -c < "$manifest.sig" | tr -d ' ')" != 64 ]; then
    echo "release-sign: an Ed25519 signature is 64 bytes, and this one is not" >&2
    rm -f "$manifest.sig"; exit 1
fi
echo "release-sign: signed and verified -> $manifest.sig"
