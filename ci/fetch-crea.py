#!/usr/bin/env python3
"""Make the snapshot of CREA's food composition tables that `ci/pack-foods.py` reads.

    python3 ci/fetch-crea.py            # reads the pages, writes ci/data/crea-2019.json.gz

**Why this exists.** CREA (Centro di ricerca Alimenti e Nutrizione) publishes its *Tabelle di
composizione degli alimenti* (2019 update) as one web page a food, and as no file. Every other
table the food list is built from is a file pinned by checksum; this one has to be read off its
pages once and kept, or the list could not be rebuilt the same way twice. The snapshot is that
keeping: what each page said on the day it was read, and the day.

**The terms it is read under**, from the foot of every page of www.alimentinutrizione.it:
*"Dati e testi … non possono essere copiati o in altro modo riprodotti o trasmessi … senza una
chiara indicazione della fonte originale. Il riferimento è: CREA Centro di ricerca Alimenti e
Nutrizione"*. Copying with the source named. The owner's decision to carry it is in
`docs/context/decisions.md`, 2026-10-13.

**How it reads.** The site's own alphabetical index lists the foods; each food's page is asked
for once, two seconds apart, saying who is asking. `robots.txt` allows these pages (read
2026-10-13). Pages are kept in a cache, so a run that stops can go on, and a second run asks for
nothing it has. Run by a maintainer, by hand; the app and its build never run it.

**What is kept**, per food: its code, Italian and English names, category, edible part, portion,
and every line of its tables as printed — the nutrient's name, its unit and its value per 100 g.
Nothing is converted here; `pack-foods.py` decides what a line means.

Standard library only.
"""
import argparse
import datetime
import gzip
import html
import json
import os
import re
import sys
import time
import urllib.request

SITE = "https://www.alimentinutrizione.it"
INDEX = "/tabelle-nutrizionali/ricerca-per-ordine-alfabetico"
AGENT = "formicaria-pack-foods (https://github.com/formicaria-org/formicaria)"
PAUSE = 2.0


def get(path, cache):
    name = os.path.join(cache, re.sub(r"[^0-9A-Za-z]+", "_", path).strip("_") + ".html")
    if os.path.exists(name) and os.path.getsize(name) > 2000:
        return open(name, encoding="utf-8", errors="replace").read()
    req = urllib.request.Request(SITE + path, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(req, timeout=60) as r:
        data = r.read()
    with open(name + ".part", "wb") as f:
        f.write(data)
    os.replace(name + ".part", name)
    time.sleep(PAUSE)
    return data.decode("utf-8", "replace")


def words(fragment):
    return " ".join(html.unescape(re.sub(r"<[^>]+>", " ", fragment)).split())


def cells(row):
    return [words(c) for c in re.findall(r"<t[dh][^>]*>(.*?)</t[dh]>", row, flags=re.S)]


def food(page):
    """One food's page as a dict, or None when it is not a food's page."""
    page = re.sub(r"<(script|style)[^>]*>.*?</\1>", " ", page, flags=re.S)
    facts, lines, section = {}, [], ""
    for row in re.findall(r"<tr[^>]*>(.*?)</tr>", page, flags=re.S):
        c = cells(row)
        if len(c) == 1 and c[0]:
            section = c[0]
        elif len(c) == 2:
            facts[c[0]] = c[1]
        elif len(c) >= 3 and c[0] and c[0] != "Descrizione Nutriente":
            lines.append({"section": section, "name": c[0], "unit": c[1], "per100": c[2],
                          "origin": c[6] if len(c) > 6 else ""})
    if "Codice Alimento" not in facts:
        return None
    title = re.search(r"<h1[^>]*>(.*?)</h1>", page, flags=re.S)
    heads = [words(h) for h in re.findall(r"<h[1-4][^>]*>(.*?)</h[1-4]>", page, flags=re.S)]
    name = next((h for h in heads if h and h != "Tabelle di Composizione degli Alimenti"), "")
    return {
        "code": facts["Codice Alimento"],
        "name": name or (words(title.group(1)) if title else ""),
        "english": facts.get("English Name", ""),
        "category": facts.get("Categoria", ""),
        "edible": facts.get("Parte Edibile", ""),
        "portion": facts.get("Porzione", ""),
        "lines": lines,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--cache", default="target/packs/src/crea")
    p.add_argument("--out", default="ci/data/crea-2019.json.gz")
    a = p.parse_args()
    os.makedirs(a.cache, exist_ok=True)
    index = get(INDEX, a.cache)
    paths = sorted(set(re.findall(r'href="(/tabelle-nutrizionali/[0-9A-Za-z_-]+)"', index)))
    paths = [x for x in paths if re.search(r"/\d+$", x)]
    foods, missed = [], []
    for i, path in enumerate(paths):
        try:
            f = food(get(path, a.cache))
        except Exception as e:  # one page that will not come is said, not fatal
            missed.append(f"{path}: {e}")
            continue
        if f:
            foods.append(f)
        else:
            missed.append(f"{path}: not a food's page")
        if (i + 1) % 50 == 0:
            print(f"  {i + 1} of {len(paths)}", file=sys.stderr, flush=True)
    foods.sort(key=lambda f: f["code"])
    snapshot = {
        "source": "CREA Centro di ricerca Alimenti e Nutrizione, Tabelle di composizione degli "
                  "alimenti, Aggiornamento 2019",
        "site": SITE,
        "read": datetime.date.today().isoformat(),
        "foods": foods,
    }
    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    data = json.dumps(snapshot, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    with open(a.out, "wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as z:
        z.write(data)
    print(f"{a.out}: {len(foods)} foods, {os.path.getsize(a.out)} bytes")
    for m in missed:
        print(f"  missed {m}", file=sys.stderr)


if __name__ == "__main__":
    main()
