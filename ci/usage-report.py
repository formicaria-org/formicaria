#!/usr/bin/env python3
"""**What the usage diary says** (`fm_app::diary`, `decisions.md` 2026-11-20).

`pixi run usage-report` (the last 30 days) or `pixi run usage-report -- --days 7`. Reads this
computer's diary (`<settings>/formicaria/diary/*.jsonl`) and what phones carried into the vaults
(`<vault>/.formicaria/usage/*.jsonl`), and says, per device:

- each part's memory and processor (median, 95th percentile, most) and by day;
- starts and stops and how each ended, runs that ended without saying, the browser's own ends;
- commands by use and by time, the slow ones, the errors;
- pages opened and how long they took, the add-on's gestures, errors seen in pages.

Reads only; prints only what the diary holds (names and figures, never a person's things)."""
from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path


def config_dir() -> Path:
    x = os.environ.get("XDG_CONFIG_HOME")
    return Path(x) if x and os.path.isabs(x) else Path.home() / ".config"


def sources(cfg: Path) -> list[Path]:
    files = sorted((cfg / "formicaria" / "diary").glob("????-??-??.jsonl"))
    try:
        vaults = json.loads((cfg / "formicaria" / "vaults.json").read_text())["vaults"]
    except (OSError, ValueError, KeyError):
        vaults = []
    for v in vaults:
        files += sorted((Path(v["path"]) / ".formicaria" / "usage").glob("*.jsonl"))
    return files


def read(files: list[Path], since_ms: int) -> list[dict]:
    out = []
    for f in files:
        try:
            text = f.read_text()
        except OSError:
            continue
        for line in text.splitlines():
            try:
                v = json.loads(line)
            except ValueError:
                continue
            if isinstance(v, dict) and v.get("t", 0) >= since_ms:
                out.append(v)
    out.sort(key=lambda v: v.get("t", 0))
    return out


def pct(xs: list[float], p: float) -> float:
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(round(p * (len(xs) - 1))))] if xs else 0.0


def day(t: int) -> str:
    return time.strftime("%Y-%m-%d", time.localtime(t / 1000))


def report(lines: list[dict], out=sys.stdout) -> None:
    say = lambda s="": print(s, file=out)  # noqa: E731
    if not lines:
        say("usage-report: the diary is empty for this period.")
        return
    devices = sorted({v.get("dev", "?") for v in lines})
    say(f"usage-report: {len(lines)} lines, {day(lines[0]['t'])} to {day(lines[-1]['t'])}, devices: {', '.join(devices)}")
    for dev in devices:
        mine = [v for v in lines if v.get("dev") == dev]
        by = defaultdict(list)
        for v in mine:
            by[v.get("kind")].append(v)
        say()
        say(f"== {dev}")

        # Starts and stops.
        versions = Counter(v.get("version") for v in by["start"])
        say(f"starts {len(by['start'])} ({', '.join(f'{k} ×{n}' for k, n in versions.most_common())}); "
            f"stops said {len(by['stop'])}; ended without saying {len(by['ended_unsaid'])}")
        reasons = Counter(v.get("reason") for v in by["stop"])
        if reasons:
            say("  stops: " + "; ".join(f"{k} ×{n}" for k, n in reasons.most_common()))
        if by["browser_exit"]:
            ends = Counter(
                f"signal {v['signal']}" if v.get("signal") else f"code {v.get('code')}" for v in by["browser_exit"]
            )
            runs = [v.get("ran_s", 0) / 3600 for v in by["browser_exit"]]
            say(f"  browser ended {len(runs)}×: " + "; ".join(f"{k} ×{n}" for k, n in ends.most_common())
                + f"; ran median {statistics.median(runs):.1f} h")
        quits = Counter(v.get("what") for v in by["browser"])
        starts = [v["ms"] for v in by["browser"] if v.get("what") == "started" and "ms" in v]
        if quits or starts:
            say("  browser: " + "; ".join(f"{k} ×{n}" for k, n in quits.most_common())
                + (f"; session back in median {statistics.median(starts):.0f} ms" if starts else ""))

        # Memory and processor.
        parts = defaultdict(lambda: {"mb": [], "cpu": [], "days": defaultdict(list)})
        for v in by["sample"]:
            for part, s in (v.get("parts") or {}).items():
                parts[part]["mb"].append(s.get("mb", 0))
                parts[part]["cpu"].append(s.get("cpu_pct", 0))
                parts[part]["days"][day(v["t"])].append(s.get("mb", 0))
        if parts:
            say(f"memory and processor ({len(by['sample'])} samples, one each 5 min while running):")
            say(f"  {'part':<10} {'MB median':>9} {'p95':>6} {'most':>6}   {'CPU % median':>12} {'p95':>6}")
            for part, s in parts.items():
                say(f"  {part:<10} {statistics.median(s['mb']):>9.0f} {pct(s['mb'], .95):>6.0f} {max(s['mb']):>6.0f}"
                    f"   {statistics.median(s['cpu']):>12.1f} {pct(s['cpu'], .95):>6.1f}")
            say("  by day, most MB: " + "; ".join(
                f"{d} " + " ".join(f"{p} {max(parts[p]['days'][d]):.0f}" for p in parts if d in parts[p]['days'])
                for d in sorted({d for s in parts.values() for d in s["days"]})[-10:]))

        # Commands.
        cmds = defaultdict(lambda: {"n": 0, "ms": 0, "max": 0, "errors": 0})
        for v in by["use"]:
            for c, t in (v.get("commands") or {}).items():
                a = cmds[c]
                a["n"] += t.get("n", 0)
                a["ms"] += t.get("ms", 0)
                a["max"] = max(a["max"], t.get("max_ms", 0))
                a["errors"] += t.get("errors", 0)
        if cmds:
            say(f"commands ({sum(a['n'] for a in cmds.values())} answered, {len(cmds)} kinds):")
            say("  most used: " + ", ".join(f"{c} {a['n']}" for c, a in sorted(cmds.items(), key=lambda x: -x[1]["n"])[:12]))
            say("  most time: " + ", ".join(
                f"{c} {a['ms'] / 1000:.1f} s (avg {a['ms'] / max(a['n'], 1):.0f} ms, most {a['max']} ms)"
                for c, a in sorted(cmds.items(), key=lambda x: -x[1]["ms"])[:8]))
            errs = {c: a["errors"] for c, a in cmds.items() if a["errors"]}
            if errs:
                say("  errors: " + ", ".join(f"{c} {n}" for c, n in sorted(errs.items(), key=lambda x: -x[1])[:10]))
        if by["slow"]:
            slow = Counter(v.get("cmd") for v in by["slow"])
            say(f"  slow (over 1 s) {len(by['slow'])}×: " + ", ".join(f"{c} ×{n}" for c, n in slow.most_common(10)))

        # Pages, gestures, errors.
        if by["page"]:
            pages = defaultdict(list)
            for v in by["page"]:
                pages[v.get("page")].append(v.get("ms", 0))
            say("pages opened: " + ", ".join(
                f"{p} {len(ms)}× (median {statistics.median(ms):.0f} ms)" for p, ms in sorted(pages.items(), key=lambda x: -len(x[1]))[:12]))
        if by["act"]:
            say("gestures: " + ", ".join(f"{k} {n}" for k, n in Counter(v.get("what") for v in by["act"]).most_common()))
        if by["error"]:
            say(f"errors in pages {len(by['error'])}×: " + "; ".join(
                f"{m} ×{n}" for m, n in Counter(v.get("message") for v in by["error"]).most_common(8)))


def main() -> None:
    a = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    a.add_argument("--days", type=int, default=30)
    a.add_argument("--dir", type=Path, help="a settings folder other than this user's (for tests)")
    args = a.parse_args()
    cfg = args.dir or config_dir()
    since = int((time.time() - args.days * 86400) * 1000)
    report(read(sources(cfg), since))


if __name__ == "__main__":
    main()
