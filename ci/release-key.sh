#!/bin/sh
# Make an Ed25519 release-signing key. **Run by the maintainer, on their own machine, once** — never
# by CI, and never inside a session whose transcript is kept.
#
# The key is the whole trust root of the in-app updater: every byte an installed formicaria is
# willing to execute traces back to it. Two consequences are worth reading before running this:
#
#   - **Losing it strands every installed copy.** Nothing signed with another key will be accepted,
#     so no installed formicaria could update itself ever again. Back it up offline.
#   - **A second, offline recovery key is cheap insurance.** The app trusts every key listed in
#     `crates/fm-update/release-keys.txt`, so generating one more — `sh ci/release-key.sh
#     release-recovery` — and listing its public half means a lost signing key costs one release
#     signed with the recovery key, not every user's updates. Keep the recovery key off GitHub.
#
#   sh ci/release-key.sh [name]    (default name: release-signing)
set -eu

name=${1:-release-signing}
case "$name" in *[!A-Za-z0-9_-]*) echo "release-key: '$name' is not a plain name" >&2; exit 1 ;; esac
d="$HOME/.config/formicaria"
key="$d/$name.pem"

if [ -e "$key" ]; then
    echo "release-key: $key already exists. Refusing to overwrite a trust root." >&2
    exit 1
fi
mkdir -p "$d"
chmod 700 "$d"
( umask 177 && openssl genpkey -algorithm ed25519 -out "$key" )
chmod 600 "$key"

# The raw 32-byte public key: the tail of the DER SubjectPublicKeyInfo, as lowercase hex.
pub=$(openssl pkey -in "$key" -pubout -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n')

cat <<EOF
Wrote the private key to $key (mode 600).

1. Back it up somewhere offline, now. If it is lost, no installed formicaria can update itself.

2. Add this line to crates/fm-update/release-keys.txt and commit it:

   $pub  # $name, $(date -u +%Y-%m-%d)

3. If this is the signing key (not a recovery key), store it as the RELEASE_SIGNING_KEY_B64
   repository secret — the command reads the file, so the key never appears on screen:

   base64 -w0 "$key" | gh secret set RELEASE_SIGNING_KEY_B64      (Linux)
   base64 -i "$key" | tr -d '\n' | gh secret set RELEASE_SIGNING_KEY_B64      (macOS)
EOF
