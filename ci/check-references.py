#!/usr/bin/env python3
"""Open every source the app cites, and read every figure it shows off the document itself.

`pixi run check-references` (`decisions.md#data`, 2026-11-03, *a figure is shown with where it
comes from*). A bar, a ring or a limit in Food says how a person's day compares with a figure, and
a person may act on it, so each figure in `crates/fm-app/src/shipped/reference-sources.json` is
checked against the published text, and each link the food and health cards carry is opened.

It fails when a cited document cannot be opened, when a figure is not where the file says, or when
a link is gone. A site that refuses this client (a bot check, a 403) is not a pass and not a
failure: it was not tested, it is listed, and only a person opening it can settle it. **This client
says what it is and never poses as a browser**: a site's choice to refuse robots is respected.

It needs the network, so it is not part of `pixi run ci` (a gate that fails on a train is a gate
people learn to ignore). `cargo test -p fm-app` is what holds the file to the code offline; this
is what holds the file to the world, and it runs before a release. `--write` records the day and
each link's result in the file, and only when nothing failed. `--by-hand URL ...` records that a
person opened a link this client cannot, and read the page: the one way past a refusal.
"""

import datetime
import html
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request

FILE = "crates/fm-app/src/shipped/reference-sources.json"
CARRIERS = ["crates/fm-app/src/shipped/plans.json", "crates/fm-app/src/shipped/health-cards.json"]
AGENT = "formicaria-check-references/1 (+https://github.com/formicaria-org/formicaria)"
GONE = {404, 410}
NOT_FOUND_PAGE = re.compile(r"<title[^>]*>[^<]*(page not found|not found|404)[^<]*</title>", re.I)


def fetch(url):
    """(HTTP status, bytes), or (None, the error) when no answer came."""
    request = urllib.request.Request(url, headers={"User-Agent": AGENT})
    for attempt in range(2):
        try:
            with urllib.request.urlopen(request, timeout=120) as answer:
                return answer.status, answer.read()
        except urllib.error.HTTPError as e:
            return e.code, b""
        except (urllib.error.URLError, TimeoutError, OSError) as e:
            problem = str(e).encode()
    return None, problem


def verdict(status, body):
    """`opened`, `refused` (no answer a robot can read: a 403, or a 202 or 203 challenge page) or
    `gone`. A 2xx is not enough: PubMed answers robots 203 with a page that is not the article."""
    if status in GONE:
        return "gone"
    if status != 200:
        return "refused"
    if not body.startswith(b"%PDF") and NOT_FOUND_PAGE.search(body.decode("utf-8", "ignore")):
        return "gone"
    return "opened"


def pages_of(body):
    """A document as its pages of text: a PDF's pages keeping their columns, a web page as one."""
    if body.startswith(b"%PDF"):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "doc.pdf")
            with open(path, "wb") as f:
                f.write(body)
            text = subprocess.run(
                ["pdftotext", "-layout", path, "-"], capture_output=True, check=True
            ).stdout.decode("utf-8", "ignore")
        return text.split("\f")
    text = re.sub(r"<(script|style)\b.*?</\1>", " ", body.decode("utf-8", "ignore"), flags=re.S)
    return [" ".join(html.unescape(re.sub(r"<[^>]+>", " ", text)).split())]


def cell(pages, figure):
    """What the document prints in the figure's row and column, without a footnote's mark."""
    for line in pages[figure["page"] - 1].splitlines():
        line = line.strip()
        if line.startswith(figure["row"]):
            cells = re.split(r"\s{2,}", line)
            if figure["col"] < len(cells):
                return re.sub(r"\([a-z]\)$", "", cells[figure["col"]])
            return None
    return None


def check_figure(pages, figure):
    """Why the figure is not what the document prints, or None when it is."""
    name = f'{figure["key"]} {figure["what"]} ({figure["for"]})'
    if "contains" in figure:
        if figure["contains"] not in " ".join(" ".join(pages).split()):
            return f'{name}: the words "{figure["contains"]}" are not in the document'
        return None
    if figure["page"] > len(pages):
        return f'{name}: the document has no page {figure["page"]}'
    got = cell(pages, figure)
    if got != figure["value"]:
        return f'{name}: page {figure["page"]}, row "{figure["row"]}", column {figure["col"]} prints {got!r}, the file says {figure["value"]!r}'
    return None


def carried_links():
    found = set()
    for path in CARRIERS:
        with open(path, encoding="utf-8") as f:
            found |= set(re.findall(r"https?://[^\"\s<>]+", f.read()))
    return found


def main():
    args = sys.argv[1:]
    with open(FILE, encoding="utf-8") as f:
        data = json.load(f)
    if args[:1] == ["--by-hand"]:
        for url in args[1:]:
            if url not in data["links"]:
                print(f"not a link the cards carry: {url}")
                return 1
            data["links"][url] = {"result": "by hand", "seen": datetime.date.today().isoformat()}
        with open(FILE, "w", encoding="utf-8") as f:
            write_file(f, data)
        return 0
    write = "--write" in args
    before = data.get("links", {})
    failures, refused, fell_back = [], [], []
    today = datetime.date.today().isoformat()

    opened = {}
    for doc_id, doc in data["documents"].items():
        status, body = fetch(doc["url"])
        if verdict(status, body) != "opened":
            failures.append(f'{doc_id}: {doc["url"]} did not open ({status})')
            continue
        opened[doc_id] = pages_of(body)
        doc["seen"] = today
    for figure in data["figures"]:
        if figure["doc"] in opened:
            why = check_figure(opened[figure["doc"]], figure)
            if why:
                failures.append(why)

    links = {}
    for url in sorted(carried_links()):
        status, body = fetch(url)
        result = verdict(status, body)
        print(f"{result:8} {status} {url}")
        if result == "gone":
            failures.append(f"{url}: gone ({status})")
        elif result == "refused":
            # A bot wall changes its mind: a link it let through last week may be refused today.
            # That is no news about the link, so the record of when it last opened stands.
            earlier = before.get(url, {})
            if earlier.get("result") in ("opened", "by hand"):
                links[url] = earlier
                fell_back.append(f'{url} ({status}), last opened {earlier["seen"]}')
                continue
            refused.append(f"{url} ({status})")
        links[url] = {"result": result, "seen": today}

    if fell_back:
        print(f"\nREFUSED TODAY, opened before: {len(fell_back)}")
        print("\n".join(f"  {r}" for r in fell_back))
    if refused:
        print(f"\nNOT TESTED: {len(refused)} refused this client, so a person has to open them:")
        print("\n".join(f"  {r}" for r in refused))
    if failures:
        print(f"\nFAILED: {len(failures)}")
        print("\n".join(f"  {f}" for f in failures))
        return 1
    n = len(data["figures"])
    print(f"\nOK: {len(opened)} documents opened, {n} figures read off them, {len(links)} links.")
    if write:
        data["links"] = links
        with open(FILE, "w", encoding="utf-8") as f:
            write_file(f, data)
    return 0


def write_file(f, data):
    """One figure or link to a line, so a change reads as one line in a diff."""
    dump = lambda v: json.dumps(v, ensure_ascii=False)
    f.write('{\n  "documents": {\n')
    f.write(",\n".join(f"    {dump(k)}: {dump(v)}" for k, v in data["documents"].items()))
    f.write('\n  },\n  "figures": [\n')
    f.write(",\n".join(f"    {dump(x)}" for x in data["figures"]))
    f.write('\n  ],\n  "links": {\n')
    f.write(",\n".join(f"    {dump(k)}: {dump(v)}" for k, v in sorted(data["links"].items())))
    f.write("\n  }\n}\n")


if __name__ == "__main__":
    sys.exit(main())
