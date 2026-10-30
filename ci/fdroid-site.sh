#!/bin/sh
# The F-Droid repository on the website: fetch the newest release APKs and their notes from GitHub,
# then build the repository with `ci/fdroid-repo.sh` (`decisions.md` 2026-10-30, *our own F-Droid
# repo*). Run by `pages.yml`; rehearse it on a laptop with a throwaway key.
#
#   sh ci/fdroid-site.sh <out>          (writes <out>/repo and <out>/index.html)
#
# **Gated on the repo key**, like `fdroid-repo.sh`: without `FDROID_KEYSTORE_B64` it fetches
# nothing, writes nothing and exits 0, so the website publishes exactly as before.
#   FDROID_KEEP   how many releases to list (default 3; the newest is what clients install)
#   GH_TOKEN      optional: raises GitHub's rate limit (the releases are public)
set -eu

out=${1:?usage: sh ci/fdroid-site.sh <out>}
if [ -z "${FDROID_KEYSTORE_B64:-}" ]; then
    echo "fdroid-site: no repository key (FDROID_KEYSTORE_B64 is not set) — skipped, nothing fetched"
    exit 0
fi
dl=$(mktemp -d)
trap 'rm -rf "$dl"' EXIT INT TERM

python3 - "$dl" "${FDROID_KEEP:-3}" <<'PY'
import json, os, re, sys, urllib.request
dl, keep = sys.argv[1], int(sys.argv[2])
hdr = {"Accept": "application/vnd.github+json", "User-Agent": "formicaria-fdroid-site"}
if os.environ.get("GH_TOKEN"):
    hdr["Authorization"] = "Bearer " + os.environ["GH_TOKEN"]
api = "https://api.github.com/repos/formicaria-org/formicaria/releases?per_page=30"
releases = json.load(urllib.request.urlopen(urllib.request.Request(api, headers=hdr)))
os.makedirs(os.path.join(dl, "notes"), exist_ok=True)


def what_changed(body, version):
    """The release's own news, without the page's standing introduction and download table.

    `release.yml` writes: an introduction, a line `formicaria <version>`, the tag message, then
    `The assistant in this release…` and `### Get started`. Keep only the tag message, plain (F-Droid
    shows text, 500 characters at most). If the shape ever changes, fall back to the whole body."""
    lines = body.replace("\r", "").split("\n")
    try:
        start = next(i for i, l in enumerate(lines) if l.strip() == f"formicaria {version}") + 1
    except StopIteration:
        return body.strip()
    keep = []
    for l in lines[start:]:
        if l.startswith("###") or l.startswith("The assistant in this release"):
            break
        keep.append(l)
    return "\n".join(keep).replace("**", "").strip() or body.strip()


got = 0
for r in releases:
    if r.get("draft") or r.get("prerelease"):
        continue
    m = re.fullmatch(r"v(\d+)\.(\d+)\.(\d+)", r.get("tag_name", ""))
    if not m:
        continue
    name = f"formicaria-{r['tag_name']}-android-arm64.apk"
    asset = next((a for a in r.get("assets", []) if a["name"] == name), None)
    if not asset:
        continue  # a release whose phone build failed has no APK; skip it, as the updater does
    path = os.path.join(dl, name)
    req = urllib.request.Request(asset["browser_download_url"], headers={"User-Agent": hdr["User-Agent"]})
    with urllib.request.urlopen(req) as src, open(path, "wb") as dst:
        while chunk := src.read(1 << 20):
            dst.write(chunk)
    if os.path.getsize(path) != asset.get("size", -1):
        sys.exit(f"fdroid-site: {name} arrived with the wrong size")
    # The versionCode scheme of `ci/android-release.sh`: major·1000000 + minor·1000 + patch.
    code = int(m[1]) * 1_000_000 + int(m[2]) * 1_000 + int(m[3])
    body = what_changed(r.get("body") or "", f"{m[1]}.{m[2]}.{m[3]}")
    if body:
        with open(os.path.join(dl, "notes", f"{code}.txt"), "w") as f:
            f.write(body + "\n")
    print(f"fdroid-site: {name} (versionCode {code})")
    got += 1
    if got >= keep:
        break
if not got:
    sys.exit("fdroid-site: no release has an Android APK")
PY

FDROID_NOTES_DIR="$dl/notes" pixi run -e fdroid fdroid-repo "$out" "$dl"/*.apk
