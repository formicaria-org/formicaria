#!/usr/bin/env python3
"""Build the food reference pack: one read-only SQLite file made from USDA FoodData Central.

    pixi run pack-foods            # writes the list, and the compressed copy the app carries

**What it is.** `decisions.md#toolchain`, 2026-10-11: *the food list is carried in the app*. This
is the script that makes it. It is run by a maintainer, by hand, when a
new list is wanted. The app never runs it, and the build reads only the compressed file it leaves in
`crates/fm-app/packs/` — which is committed, so building formicaria needs neither this script nor
the network.

**What goes in.** Three of FoodData Central's datasets — the generic foods, not the branded ones:

  - Foundation Foods   (laboratory analyses of single foods)
  - SR Legacy          (the Standard Reference database, final release 2018)
  - Survey Foods/FNDDS (foods as people report eating them, with household portions)

Each input is pinned below by URL **and checksum**, so the same script makes the same pack, and a
changed upstream file is a refusal here, not a silently different pack.

**What comes out.** Every value per 100 g, as a whole number in the unit its key names — `kcal`,
`protein_mg`, `iron_ug` — which is the form a logged entry stores (`decisions.md#data`). Nothing is
kept as a decimal, so nothing the app adds up can drift.

Standard library only.
"""
import argparse
import csv
import gzip
import hashlib
import io
import os
import sqlite3
import sys
import urllib.request
import zipfile

BASE = "https://fdc.nal.usda.gov/fdc-datasets/"
#  file, sha256, the `data_type` to keep, a short name for where a food came from
INPUTS = [
    ("FoodData_Central_foundation_food_csv_2026-04-30.zip",
     "70457ee9d9342f43bda2010318c85f04210c689fdeb9cd2da4c513b0e8dbc655", "foundation_food", "foundation"),
    ("FoodData_Central_survey_food_csv_2024-10-31.zip",
     "5ccc25ec2777a8982fbb61378a42f415316173eb11e48c9a8ba4cb19f5a4f29c", "survey_fndds_food", "survey"),
    ("FoodData_Central_sr_legacy_food_csv_2018-04.zip",
     "b80817294b8850530aaedf2e515c02593b1824f763a0ff356e5c2081643e6fd0", "sr_legacy_food", "sr"),
]
# The pack's own version: bump it whenever an input above, the list below or the layout changes.
# An entry records `<name>@<version>/<food>` as where its numbers came from.
NAME = "usda-foods"
VERSION = "2026.10"

# key -> USDA nutrient numbers to take it from, first one present wins.
# The key's ending is the unit stored: `_mg` for what is weighed in grams, `_ug` for what is
# weighed in milligrams or less, `kcal` as itself.
NUTRIENTS = [
    ("kcal", ["208", "958", "957"]),  # Energy; Atwater specific, then general, where 208 is absent
    ("protein_mg", ["203"]),
    ("carbs_mg", ["205"]),
    ("fat_mg", ["204"]),
    ("fiber_mg", ["291"]),
    ("sugars_mg", ["269"]),
    ("saturated_fat_mg", ["606"]),
    ("water_mg", ["255"]),
    ("alcohol_mg", ["221"]),
    ("sodium_ug", ["307"]),
    ("potassium_ug", ["306"]),
    ("calcium_ug", ["301"]),
    ("iron_ug", ["303"]),
    ("magnesium_ug", ["304"]),
    ("phosphorus_ug", ["305"]),
    ("zinc_ug", ["309"]),
    ("copper_ug", ["312"]),
    ("manganese_ug", ["315"]),
    ("selenium_ug", ["317"]),
    ("vitamin_a_ug", ["320"]),  # retinol activity equivalents
    ("vitamin_c_ug", ["401"]),
    ("vitamin_d_ug", ["328"]),
    ("vitamin_e_ug", ["323"]),
    ("vitamin_k_ug", ["430"]),
    ("thiamin_ug", ["404"]),
    ("riboflavin_ug", ["405"]),
    ("niacin_ug", ["406"]),
    ("vitamin_b6_ug", ["415"]),
    ("folate_ug", ["417"]),
    ("vitamin_b12_ug", ["418"]),
    ("choline_ug", ["421"]),
    ("cholesterol_ug", ["601"]),
    ("caffeine_ug", ["262"]),
]
#  how many of a key's unit one of the USDA's unit is
PER = {
    "kcal": {"KCAL": 1},
    "mg": {"G": 1000, "MG": 1},
    "ug": {"G": 1_000_000, "MG": 1000, "UG": 1},
}
MAX_PORTIONS = 6


def fetch(cache, name, want):
    """The input file's bytes: from the cache if its checksum is right, else downloaded."""
    path = os.path.join(cache, name)
    if not os.path.exists(path):
        print(f"  downloading {name}", file=sys.stderr)
        os.makedirs(cache, exist_ok=True)
        with urllib.request.urlopen(BASE + name, timeout=300) as r, open(path + ".part", "wb") as f:
            f.write(r.read())
        os.replace(path + ".part", path)
    data = open(path, "rb").read()
    got = hashlib.sha256(data).hexdigest()
    if got != want:
        sys.exit(f"{name}: checksum is {got}, expected {want}. The upstream file changed — "
                 f"look at it, then pin the new checksum and bump VERSION.")
    return data


def table(z, name):
    """One CSV in the archive, as dict rows. The archives nest everything one folder deep."""
    member = next(n for n in z.namelist() if n.endswith("/" + name) or n == name)
    return csv.DictReader(io.TextIOWrapper(z.open(member), encoding="utf-8", newline=""))


def unit_of(key):
    return "kcal" if key == "kcal" else key.rsplit("_", 1)[1]


def portion_label(row, units):
    """A household portion in words: `1 cup, chopped`. Empty when the row does not say."""
    desc = (row.get("portion_description") or "").strip()
    if desc:
        return "" if desc.lower().startswith("quantity not specified") else desc
    amount = (row.get("amount") or "").strip()
    if amount.endswith(".0"):
        amount = amount[:-2]
    unit = units.get(row.get("measure_unit_id", ""), "")
    if unit.lower() == "undetermined":
        unit = ""
    modifier = (row.get("modifier") or "").strip()
    return " ".join(p for p in (amount, unit, modifier) if p).strip()


def read(data, keep, src):
    """Foods of one dataset: (fdc id, name, src, {key: per 100 g}, [(label, grams)])."""
    z = zipfile.ZipFile(io.BytesIO(data))
    nutrients = list(table(z, "nutrient.csv"))
    by_id = {n["id"]: n for n in nutrients}
    by_nbr = {n["nutrient_nbr"].split(".")[0]: n for n in nutrients}
    wanted = {}  # nutrient number -> (key, priority)
    for key, numbers in NUTRIENTS:
        for at, nbr in enumerate(numbers):
            wanted[nbr] = (key, at)

    names = {r["fdc_id"]: r["description"].strip() for r in table(z, "food.csv") if r["data_type"] == keep}

    # **The survey dataset writes the nutrient's *number* where the others write its id.** Tell
    # which by looking: a column whose values are nearly all ids holds ids, otherwise numbers.
    rows = [r for r in table(z, "food_nutrient.csv") if r["fdc_id"] in names]
    # (Mostly: the foundation dataset also uses a few ids its own nutrient table does not list.)
    sample = rows[:2000]
    ids_are_ids = sum(r["nutrient_id"] in by_id for r in sample) > 0.9 * len(sample)
    values, best = {}, {}
    for r in rows:
        n = by_id.get(r["nutrient_id"]) if ids_are_ids else by_nbr.get(r["nutrient_id"])
        if not n or not r["amount"]:
            continue
        hit = wanted.get(n["nutrient_nbr"].split(".")[0])
        if not hit:
            continue
        key, priority = hit
        per = PER[unit_of(key)].get(n["unit_name"])
        if per is None:
            continue
        slot = (r["fdc_id"], key)
        if slot in best and best[slot] <= priority:
            continue
        best[slot] = priority
        values.setdefault(r["fdc_id"], {})[key] = round(float(r["amount"]) * per)

    units = {r["id"]: r["name"] for r in table(z, "measure_unit.csv")}
    portions = {}
    for r in sorted(table(z, "food_portion.csv"), key=lambda r: (r["fdc_id"], int(r["seq_num"] or 0))):
        if r["fdc_id"] not in names:
            continue
        try:
            grams = round(float(r["gram_weight"]))
        except ValueError:
            continue
        label = portion_label(r, units)
        mine = portions.setdefault(r["fdc_id"], [])
        if grams < 1 or not label or len(mine) >= MAX_PORTIONS or any(l == label for l, _ in mine):
            continue
        mine.append((label, grams))

    for fdc, name in names.items():
        v = values.get(fdc, {})
        if "kcal" not in v:
            continue  # a food with no energy value cannot be logged against a target
        yield int(fdc), name, src, v, portions.get(fdc, [])


def build(cache, out, embed):
    if os.path.exists(out):
        os.remove(out)
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    db = sqlite3.connect(out)
    # The layout is the reader's as much as this script's, so it lives with the reader.
    here = os.path.dirname(os.path.abspath(__file__))
    layout = open(os.path.join(here, "..", "crates", "fm-core", "src", "pack.sql"), encoding="utf-8").read()
    db.executescript("PRAGMA page_size = 4096;\n" + layout)
    keys = {key: i for i, (key, _) in enumerate(NUTRIENTS)}
    db.executemany("INSERT INTO nutrient(n, key) VALUES (?, ?)", [(i, k) for k, i in keys.items()])
    counts = {}
    for name, sha, keep, src in INPUTS:
        for fdc, title, src, values, portions in read(fetch(cache, name, sha), keep, src):
            db.execute("INSERT INTO food(id, name, src) VALUES (?, ?, ?)", (fdc, title, src))
            db.executemany("INSERT INTO amount VALUES (?, ?, ?)", [(fdc, keys[k], v) for k, v in values.items()])
            db.executemany("INSERT INTO portion VALUES (?, ?, ?, ?)",
                           [(fdc, i, label, grams) for i, (label, grams) in enumerate(portions)])
            counts[src] = counts.get(src, 0) + 1
    db.execute("INSERT INTO names(rowid, name) SELECT id, name FROM food")
    meta = {
        "name": NAME,
        "version": VERSION,
        "foods": str(sum(counts.values())),
        "title": "Generic foods (USDA FoodData Central)",
        "licence": "U.S. Government work, public domain",
        "sources": "; ".join(n for n, *_ in INPUTS),
        "attribution": "U.S. Department of Agriculture, Agricultural Research Service. "
                       "FoodData Central. fdc.nal.usda.gov.",
    }
    db.executemany("INSERT INTO meta VALUES (?, ?)", meta.items())
    db.commit()
    db.execute("INSERT INTO names(names) VALUES ('optimize')")
    db.commit()
    db.execute("VACUUM")
    db.close()
    data = open(out, "rb").read()
    print(f"{out}")
    print(f"  foods:   {sum(counts.values())}  {counts}")
    print(f"  bytes:   {len(data)}")
    print(f"  sha256:  {hashlib.sha256(data).hexdigest()}")
    print(f"  version: {NAME}@{VERSION}")
    if embed:
        # What the app carries (`decisions.md#toolchain`, 2026-10-11). `mtime=0` and a fixed name,
        # so the same list always compresses to the same bytes.
        os.makedirs(os.path.dirname(embed), exist_ok=True)
        with open(embed, "wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as z:
            z.write(data)
        print(f"{embed}")
        print(f"  bytes:   {os.path.getsize(embed)}")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--cache", default="target/packs/src", help="where the USDA archives are kept")
    p.add_argument("--out", default="target/packs/usda-foods.sqlite")
    p.add_argument("--embed", default="crates/fm-app/packs/usda-foods.sqlite.gz",
                   help="the compressed copy the app carries; empty to skip")
    a = p.parse_args()
    build(a.cache, a.out, a.embed)
