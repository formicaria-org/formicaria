#!/usr/bin/env python3
"""Pin the Firefox ESR formicaria's browser is built on — `decisions.md` 2026-11-15 (evening).

A release step, beside `myrme-bump`: asks Mozilla for the newest ESR, checks Mozilla's signature on its
published SHA-512 sums with `gpgv` against the key kept in this repository, and writes
`browser/esr.toml` (the version and the sum per platform), which `fm-serve` compiles in. When Mozilla
rotates its signing subkey under the same primary key, the repository's copy of the key is refreshed.

A new **major** (the yearly ESR) is said loudly: the kit's hooks into Firefox's window must then pass
`pixi run browser-check` before the release goes out.
"""
import re
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PIN = ROOT / "browser" / "esr.toml"
KEY_GPG = ROOT / "browser" / "mozilla-release-key.gpg"
KEY_ASC = ROOT / "browser" / "mozilla-release-key.asc"
FINGERPRINT = "14F26682D0916CDD81E37B6D61B7B526D98F0353"  # Mozilla Software Releases
LATEST = "https://download.mozilla.org/?product=firefox-esr-latest-ssl&os=linux64&lang=en-US"
RELEASES = "https://archive.mozilla.org/pub/firefox/releases/{v}/"
PLATFORMS = {"linux-x86_64": "linux-x86_64", "linux-aarch64": "linux-aarch64"}
UA = {"User-Agent": "formicaria-release-tools"}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


def latest_version() -> str:
    opener = urllib.request.build_opener(NoRedirect)
    try:
        opener.open(urllib.request.Request(LATEST, headers=UA))
    except urllib.error.HTTPError as e:
        loc = e.headers.get("Location", "")
        m = re.search(r"/releases/([0-9.]+esr)/", loc)
        if m:
            return m.group(1)
    sys.exit("browser-bump: Mozilla did not say which ESR is newest")


def get(url: str) -> bytes:
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
        return r.read()


def pinned() -> str | None:
    if not PIN.exists():
        return None
    m = re.search(r'^version = "([^"]+)"', PIN.read_text(), re.M)
    return m.group(1) if m else None


def main() -> None:
    v = latest_version()
    base = RELEASES.format(v=v)
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        # Mozilla's key as published beside the release: refresh ours only under the same primary key.
        key = get(base + "KEY")
        (tmp / "KEY").write_bytes(key)
        shown = subprocess.run(["gpg", "--show-keys", "--with-colons", str(tmp / "KEY")],
                               capture_output=True, text=True).stdout
        fprs = [ln.split(":")[9] for ln in shown.splitlines() if ln.startswith("fpr:")]
        if not fprs or fprs[0] != FINGERPRINT:
            sys.exit(f"browser-bump: the key beside {v} is not Mozilla's release key ({fprs[:1]})")
        dearmored = subprocess.run(["gpg", "--dearmor"], input=key, capture_output=True, check=True).stdout
        if dearmored != KEY_GPG.read_bytes():
            KEY_GPG.write_bytes(dearmored)
            KEY_ASC.write_bytes(key)
            print("browser-bump: Mozilla's signing subkeys changed; the repository's key was refreshed")
        sums = get(base + "SHA512SUMS")
        (tmp / "SHA512SUMS").write_bytes(sums)
        (tmp / "SHA512SUMS.asc").write_bytes(get(base + "SHA512SUMS.asc"))
        ok = subprocess.run(["gpgv", "--keyring", str(KEY_GPG), str(tmp / "SHA512SUMS.asc"),
                             str(tmp / "SHA512SUMS")], capture_output=True, text=True)
        if ok.returncode != 0:
            sys.exit("browser-bump: Mozilla's signature on SHA512SUMS did not verify:\n" + ok.stderr)
    lines = sums.decode().splitlines()
    out = {}
    for name, folder in PLATFORMS.items():
        want = f"{folder}/en-US/firefox-{v}.tar.xz"
        hit = [ln.split()[0] for ln in lines if ln.split()[1:2] == [want]]
        if not hit:
            sys.exit(f"browser-bump: Mozilla lists no {want}")
        out[name] = hit[0]
    was = pinned()
    major = v.split(".")[0]
    body = (
        "# The Firefox ESR formicaria's browser is built on. Written by `pixi run browser-bump`\n"
        "# (Mozilla's signature on the sums checked); compiled into fm-serve (`browser.rs`).\n"
        f'version = "{v}"\n'
        f"major = {major}\n\n[sha512]\n"
        + "".join(f'"{k}" = "{h}"\n' for k, h in out.items())
    )
    PIN.write_text(body)
    if was == v:
        print(f"browser-bump: {v} is already pinned (signature good)")
    else:
        print(f"browser-bump: pinned {v} (was {was})")
        if was and was.split(".")[0] != major:
            print(f"browser-bump: MAJOR CHANGE {was} -> {v}: run `pixi run browser-check` and adapt the kit")


if __name__ == "__main__":
    main()
