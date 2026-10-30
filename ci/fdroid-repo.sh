#!/bin/sh
# Build formicaria's own F-Droid repository from release APKs (`decisions.md` 2026-10-30, *our own
# F-Droid repo*; plan: `docs/context/fdroid-distribution-plan-2026-10-30.md` Phase 2).
#
#   pixi run -e fdroid fdroid-repo <out> <apk>...      (or: sh ci/fdroid-repo.sh <out> <apk>...)
#
# Writes <out>/repo/ — the signed index plus the APKs, exactly what an F-Droid client reads — and
# <out>/index.html, a short page that says how to add the repository. `pages.yml` publishes <out>
# as `/fdroid/` on the website, so the address is
#   https://formicaria-org.github.io/formicaria/fdroid/repo
#
# **The APKs are our release APKs, unchanged** — signed with our key, the same file the website and
# the in-app updater deliver, so a person can move between channels without reinstalling. The repo
# key below signs only the *index* (the list of versions), never an app.
#
# **Gated on the repo key.** With no `FDROID_KEYSTORE_B64` this prints why and exits 0, writing
# nothing: the site publishes as before. Turning it on is the owner's step (see the plan).
#   FDROID_KEYSTORE_B64   base64 of a PKCS12 keystore holding the repo signing key
#   FDROID_KEYSTORE_PASS  its password
#   FDROID_KEY_ALIAS      the key's alias
#   FDROID_KEY_PASS       the key's password (defaults to FDROID_KEYSTORE_PASS)
#   FDROID_NOTES_DIR      optional: <versionCode>.txt release notes (500 characters at most)
#   FDROID_REPO_URL       optional: overrides the address above (for a rehearsal)
#   FDROID_APKSIGNER      optional: path to `apksigner`, which fdroidserver signs the index with
#
# **Rehearse with a throwaway key** (writes to /tmp, publishes nothing):
#   pixi run -e fdroid keytool -genkeypair -keystore /tmp/rehearsal.p12 -storetype PKCS12 \
#       -storepass rehearsal -keypass rehearsal -alias rehearsal -keyalg RSA -keysize 2048 \
#       -validity 30 -dname 'CN=rehearsal only'
#   FDROID_KEYSTORE_B64=$(base64 -w0 /tmp/rehearsal.p12) FDROID_KEYSTORE_PASS=rehearsal \
#       FDROID_KEY_ALIAS=rehearsal sh ci/fdroid-site.sh /tmp/fdroid-rehearsal
#
# `apksigner` comes from Android's build-tools: this repo's own `.android/` on a laptop (`pixi run
# android-init`), the runner's preinstalled SDK (`$ANDROID_HOME`) on GitHub's Ubuntu runners.
set -eu

out=${1:?usage: sh ci/fdroid-repo.sh <out> <apk>...}
shift
[ $# -gt 0 ] || { echo "fdroid-repo: no APK given" >&2; exit 1; }

if [ -z "${FDROID_KEYSTORE_B64:-}" ]; then
    echo "fdroid-repo: no repository key (FDROID_KEYSTORE_B64 is not set) — skipped, nothing written"
    exit 0
fi
: "${FDROID_KEYSTORE_PASS:?fdroid-repo: FDROID_KEYSTORE_PASS is not set}"
: "${FDROID_KEY_ALIAS:?fdroid-repo: FDROID_KEY_ALIAS is not set}"
# The passwords stay in the environment: config.yml names them (`{env: …}`), never holds them.
FDROID_KEY_PASS=${FDROID_KEY_PASS:-$FDROID_KEYSTORE_PASS}
export FDROID_KEYSTORE_PASS FDROID_KEY_PASS
repo_url=${FDROID_REPO_URL:-https://formicaria-org.github.io/formicaria/fdroid/repo}
app=dev.formicaria.notes

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
apksigner=${FDROID_APKSIGNER:-}
if [ -z "$apksigner" ]; then
    for c in "$root/.android/sdk/build-tools/35.0.0/apksigner" \
        $(ls -d "${ANDROID_HOME:-/nonexistent}"/build-tools/*/apksigner 2>/dev/null | sort -V | tail -1) \
        $(command -v apksigner 2>/dev/null || true); do
        [ -x "$c" ] && { apksigner=$c; break; }
    done
fi
[ -n "$apksigner" ] || { echo "fdroid-repo: no apksigner found (set FDROID_APKSIGNER)" >&2; exit 1; }
work=$(mktemp -d)
# The keystore never outlives this script, whatever happens.
trap 'rm -rf "$work"' EXIT INT TERM
umask 077

mkdir -p "$work/repo" "$work/metadata"
# The repository's icon is the app's own (F-Droid draws a placeholder otherwise). fdroidserver reads
# `repo_icon` relative to where it runs and copies it into repo/icons itself.
cp "$root/mobile/src-tauri/icons/128x128@2x.png" "$work/icon.png"
for apk in "$@"; do
    [ -s "$apk" ] || { echo "fdroid-repo: $apk is missing or empty" >&2; exit 1; }
    cp "$apk" "$work/repo/"
done
printf '%s' "$FDROID_KEYSTORE_B64" | base64 -d > "$work/keystore.p12"

# The listing: the same fastlane texts IzzyOnDroid and f-droid.org read from the source tree.
cp -R "$root/fastlane/metadata/android" "$work/metadata/$app"
if [ -n "${FDROID_NOTES_DIR:-}" ] && [ -d "$FDROID_NOTES_DIR" ]; then
    mkdir -p "$work/metadata/$app/en-US/changelogs"
    for f in "$FDROID_NOTES_DIR"/*.txt; do
        [ -e "$f" ] || continue
        # F-Droid shows 500 characters at most.
        head -c 500 "$f" > "$work/metadata/$app/en-US/changelogs/$(basename "$f")"
    done
fi
cat > "$work/metadata/$app.yml" <<EOF
License: MIT
Categories:
  - Writing
AuthorName: formicaria contributors
WebSite: https://formicaria-org.github.io/formicaria/
SourceCode: https://github.com/formicaria-org/formicaria
IssueTracker: https://github.com/formicaria-org/formicaria/issues
Changelog: https://github.com/formicaria-org/formicaria/releases
EOF

cat > "$work/config.yml" <<EOF
repo_url: $repo_url
repo_name: formicaria
repo_description: >-
  formicaria's own repository: the same signed APKs as the website, so an F-Droid client keeps
  formicaria up to date.
archive_older: 0
keystore: keystore.p12
keystorepass: {env: FDROID_KEYSTORE_PASS}
keypass: {env: FDROID_KEY_PASS}
repo_keyalias: $FDROID_KEY_ALIAS
make_current_version_link: false
repo_icon: icon.png
apksigner: $apksigner
EOF

(cd "$work" && fdroid update --pretty --use-date-from-apk)

# **Only our signed APK is ever listed.** Every version in the index must carry the certificate
# pinned in `android/signing-certificate.sha256` — the same pin the release build checks.
pin=$(grep -E '^sha256 ' "$root/android/signing-certificate.sha256" | awk '{print $2}')
python3 - "$work/repo/index-v2.json" "$pin" "$app" <<'PY'
import json, sys
index, pin, app = sys.argv[1], sys.argv[2], sys.argv[3]
versions = json.load(open(index))["packages"].get(app, {}).get("versions", {})
if not versions:
    sys.exit("fdroid-repo: the index lists no version of " + app)
for v in versions.values():
    signers = v["manifest"].get("signer", {}).get("sha256", [])
    if signers != [pin]:
        sys.exit(f"fdroid-repo: {v['manifest']['versionName']} is signed by {signers}, not our pin {pin}")
print(f"fdroid-repo: {len(versions)} version(s), all signed by our certificate")
PY

# Fingerprint of the repo key, so a person can check the address they add.
fp=$(cd "$work" && keytool -exportcert -alias "$FDROID_KEY_ALIAS" -keystore keystore.p12 \
    -storetype PKCS12 -storepass:env FDROID_KEYSTORE_PASS | sha256sum | cut -d' ' -f1 | tr 'a-f' 'A-F')

rm -rf "$out"
mkdir -p "$out"
cp -R "$work/repo" "$out/repo"
# Written under umask 077 for the keystore's sake; what is published must be readable.
chmod -R a+rX "$out"
[ -s "$out/repo/index-v2.json" ] && [ -s "$out/repo/index-v1.jar" ] && [ -s "$out/repo/entry.jar" ] \
    || { echo "fdroid-repo: the index was not written" >&2; exit 1; }
umask 022
cat > "$out/index.html" <<EOF
<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>formicaria — F-Droid repository</title><link rel="stylesheet" href="../style.css"></head>
<body><main>
<h1>formicaria in F-Droid</h1>
<p>Add this repository to F-Droid (or another F-Droid client) and it keeps formicaria up to date.
It holds the same signed APK as the website, so a copy installed from the website updates from
here too, without reinstalling.</p>
<p><a href="$repo_url?fingerprint=$fp">$repo_url</a></p>
<p>Repository key fingerprint (SHA-256):<br><code>$fp</code></p>
<p>Open the link on the phone, or in F-Droid: <em>Settings → Repositories → +</em> and paste it.</p>
<p><a href="../index.html">← formicaria</a></p>
</main></body></html>
EOF
echo "fdroid-repo: $(ls "$out/repo"/*.apk | wc -l) APK(s) in $out/repo, key fingerprint $fp"
