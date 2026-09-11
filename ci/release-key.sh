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

# How to store it depends on what this machine has. **Never by a command whose output is recorded**: the
# key must not land in a terminal log, a shell history or a transcript — so with `gh` it is piped
# straight into the secret, and without it the instruction is a terminal of the person's own.
if [ "$name" = release-signing ]; then
    if command -v gh >/dev/null 2>&1; then
        store="3. Store the private key as the RELEASE_SIGNING_KEY_B64 repository secret. This reads the file,
   so the key never appears on screen:

   base64 -w0 \"$key\" | gh secret set RELEASE_SIGNING_KEY_B64"
    else
        store="3. Store the private key as a repository secret. \`gh\` is not installed here, so use the website:

   - In a terminal of your own — not one whose output is being recorded or shared — run:
         base64 -w0 \"$key\"
   - Copy the one long line it prints into GitHub: the repository → Settings → Secrets and variables
     → Actions → New repository secret, named RELEASE_SIGNING_KEY_B64.
   - Close that terminal window afterwards."
    fi
else
    store="3. This is a recovery key: do NOT store it as a GitHub secret. Its whole value is being somewhere
   CI cannot reach. Copy the file offline — a USB stick in a drawer is the right shape — and remove it
   from this machine once the copy is safe."
fi

cat <<EOF
Wrote the private key to $key (mode 600).

1. Back it up somewhere offline, now. If it is lost, no installed formicaria can update itself.

2. Add this line to crates/fm-update/release-keys.txt:

   $pub  # $name, $(date -u +%Y-%m-%d)

$store
EOF
