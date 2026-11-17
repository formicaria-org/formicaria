#!/usr/bin/env python3
"""Pin the media tools formicaria fetches on a computer — `decisions.md` 2026-11-17 (*media from any
site*). A release step, beside `browser-bump`.

- **yt-dlp**: its newest release; the maintainers' signature on the published SHA-256 sums checked
  with `gpgv` against the key kept here (`media/yt-dlp-release-key.asc`, fingerprint below). Between
  formicaria releases yt-dlp keeps itself current with its own updater.
- **ffmpeg** (and ffprobe): the newest release build, from builders that publish a SHA-256 for each
  file: Martin Riedl's single-binary builds for Linux and macOS, gyan.dev's essentials for Windows.

Writes `media/tools.json`, which `fm-serve` compiles in. Only the sums are fetched, not the tools.
"""
import json
import re
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PIN = ROOT / "media" / "tools.json"
KEY = ROOT / "media" / "yt-dlp-release-key.gpg"
FINGERPRINT = "AC0CBBE6848D6A873464AF4E57CF65933B5A7581"  # yt-dlp signing key
UA = {"User-Agent": "formicaria-release-tools"}
YTDLP = {
    "linux-x86_64": "yt-dlp_linux",
    "linux-aarch64": "yt-dlp_linux_aarch64",
    "win64": "yt-dlp.exe",
    "mac": "yt-dlp_macos",
}
RIEDL = {"linux-x86_64": "linux/amd64", "linux-aarch64": "linux/arm64", "mac-arm64": "macos/arm64", "mac-x86_64": "macos/amd64"}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *a, **k):
        return None


def get(url: str) -> bytes:
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
        return r.read()


def where(url: str) -> str:
    """The address a redirect points to, without following it."""
    op = urllib.request.build_opener(NoRedirect)
    try:
        op.open(urllib.request.Request(url, headers=UA), timeout=60)
    except urllib.error.HTTPError as e:
        if e.code in (301, 302, 303, 307, 308):
            loc = e.headers["Location"]
            return loc if loc.startswith("http") else urllib.parse.urljoin(url, loc)
        raise
    raise SystemExit(f"media-bump: {url} did not redirect")


def ytdlp() -> dict:
    tag = where("https://github.com/yt-dlp/yt-dlp/releases/latest").rstrip("/").rsplit("/", 1)[1]
    base = f"https://github.com/yt-dlp/yt-dlp/releases/download/{tag}/"
    sums, sig = get(base + "SHA2-256SUMS"), get(base + "SHA2-256SUMS.sig")
    with tempfile.TemporaryDirectory() as t:
        (Path(t) / "s").write_bytes(sums)
        (Path(t) / "s.sig").write_bytes(sig)
        r = subprocess.run(["gpgv", "--status-fd", "1", "--keyring", str(KEY), str(Path(t) / "s.sig"), str(Path(t) / "s")],
                           capture_output=True, text=True)
    # `VALIDSIG` names the signing key and, last, its primary key: the one kept here.
    if r.returncode != 0 or not any(l.startswith("[GNUPG:] VALIDSIG") and FINGERPRINT in l for l in r.stdout.splitlines()):
        raise SystemExit(f"media-bump: yt-dlp {tag}: the signature on its sums does not check\n{r.stdout}{r.stderr}")
    table = dict(reversed(l.split(None, 1)) for l in sums.decode().splitlines() if l.strip())
    table = {k.strip().lstrip("*"): v for k, v in table.items()}
    files = {}
    for plat, name in YTDLP.items():
        if name not in table:
            raise SystemExit(f"media-bump: yt-dlp {tag} has no {name}")
        files[plat] = {"url": base + name, "sha256": table[name]}
    return {"version": tag, "files": files}


def sha_of(url: str) -> str:
    s = get(url + ".sha256").decode().split()[0]
    if not re.fullmatch(r"[0-9a-f]{64}", s):
        raise SystemExit(f"media-bump: no SHA-256 for {url}")
    return s


def ffmpeg() -> dict:
    files, version = {}, None
    for plat, path in RIEDL.items():
        url = where(f"https://ffmpeg.martin-riedl.de/redirect/latest/{path}/release/ffmpeg.zip")
        probe = url.rsplit("/", 1)[0] + "/ffprobe.zip"
        v = url.rstrip("/").split("/")[-2].split("_", 1)[-1]
        version = version or v
        files[plat] = {"url": url, "sha256": sha_of(url), "probe_url": probe, "probe_sha256": sha_of(probe)}
    win = where("https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip")
    folder = win.rsplit("/", 1)[1].removesuffix(".zip")
    files["win64"] = {"url": win, "sha256": sha_of(win), "inner": f"{folder}/bin/"}
    return {"version": version, "files": files}


def main() -> int:
    old = json.loads(PIN.read_text()) if PIN.exists() else {}
    new = {
        "_": "Written by `pixi run media-bump` (decisions.md 2026-11-17); compiled into fm-serve (media.rs).",
        "yt-dlp": ytdlp(),
        "ffmpeg": ffmpeg(),
    }
    if {k: v for k, v in old.items() if k != "_"} == {k: v for k, v in new.items() if k != "_"}:
        print(f"media-bump: yt-dlp {new['yt-dlp']['version']}, ffmpeg {new['ffmpeg']['version']} already pinned")
        return 0
    PIN.write_text(json.dumps(new, indent=2) + "\n")
    print(f"media-bump: pinned yt-dlp {new['yt-dlp']['version']} (signature good), ffmpeg {new['ffmpeg']['version']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
