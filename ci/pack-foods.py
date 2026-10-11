#!/usr/bin/env python3
"""Build the food list the app carries: one read-only SQLite file, European tables first.

    pixi run pack-foods            # writes the list, and the compressed copy the app carries

**What it is.** `decisions.md#toolchain` and `#data`, 2026-10-11: *the food list is carried in
the app*, and *the food list is European; USDA fills only what Europe's tables lack*. This is the
script that makes it. It is run by a maintainer, by hand, when a new list is wanted. The app never
runs it, and the build reads only the compressed file it leaves in `crates/fm-app/packs/` — which
is committed, so building formicaria needs neither this script nor the network.

**What goes in.**

  - Ciqual 2025        ANSES, France — 3,484 foods. Etalab Open Licence 2.0.
  - BLS 4.0            Max Rubner-Institut, Germany — 7,140 foods. CC BY 4.0.
  - FoodData Central   USDA — Foundation Foods and Survey Foods (FNDDS) only, and only as a
                       fallback: it has the household portions ("1 cup") that neither European
                       table has, and some foods they do not list. U.S. Government work.

Each input is pinned below by checksum, so the same script makes the same list, and a changed
upstream file is a refusal here, not a silently different list.

**What comes out, and what was changed on the way.** Every value per 100 g, as a whole number in
the unit its key names — `kcal`, `protein_mg`, `iron_ug` — the form a logged entry stores. A
subset of each table's constituents is kept (the ones below). Ciqual's *traces* and *< x* are
stored as 0, and its *-* (no value) as no value at all. Nothing is averaged across tables, and no
food in one table is treated as the same food in another.

**One key, one definition.** The tables do not all mean the same thing by the same word, so each
key below is tied to one FAO/INFOODS food component identifier (the *tagname* — the international
vocabulary for exactly this), and a table's column is used only where the table's own
documentation gives it that definition. Where it does not, the table has no value under that key;
nothing is renamed to fit. Read at source: `docs/context/sources-2026-10-11.md`.

  - Carbohydrate is *available* carbohydrate (CHOAVL): sugars and starch, fibre not counted.
    Ciqual and BLS publish it. USDA publishes carbohydrate by difference, fibre included
    (CHOCDF); its value here is that minus its total dietary fibre, which is INFOODS's own
    *available carbohydrate by difference* (CHOAVLDF). A USDA food with no fibre value has no
    carbohydrate value.
  - Vitamin A is retinol activity equivalents (VITA_RAE: retinol + beta-carotene/12), which all
    three publish. Ciqual's column is named "retinol equivalent" but is computed with 1/12 and
    tagged RAE. BLS's separate retinol-equivalent column (1/6) is left out.
  - Folate is dietary folate equivalents (FOLDFE: food folate + 1.7 x folic acid). Each table's
    undifferentiated "total folate" column is left out.
  - Vitamin E is alpha-tocopherol alone (TOCPHA), the definition EFSA sets its reference value
    on. Ciqual's older "vitamin E" column, whose contributors it does not state, is left out.
  - Vitamin K is K1, phylloquinone (VITK1). Niacin is preformed niacin (NIA), not equivalents.
  - Protein is total nitrogen x 6.25 in Ciqual and BLS, the EU labelling definition. USDA's is
    nitrogen x a factor that depends on the food, so a USDA food's protein can differ by a few
    per cent from what a European table would print for it.
  - Energy (ENERC) is calculated by each table from its own figures: with the EU labelling
    factors in Ciqual and BLS (fibre counted at 2 kcal/g), with Atwater factors in USDA.

**Food groups.** A Ciqual food carries Ciqual's own group name. BLS marks its main groups by the
first letter of the code and its 4.0 release does not publish their names, and USDA's two
datasets here carry no group; those foods have none, and none is made up for them.

Standard library only.
"""
import argparse
import csv
import gzip
import hashlib
import html
import io
import os
import re
import sqlite3
import sys
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

# The list's own version: bump it whenever an input, the constituents or the layout changes.
# An entry records `<name>@<version>/<food>` as where its numbers came from.
NAME = "foods"
VERSION = "2026.10.1"

# ── inputs, each pinned by checksum ──────────────────────────────────────────────────────────────

CIQUAL = "https://entrepot.recherche.data.gouv.fr/api/access/datafile/"  # doi:10.57745/RDMHWY
CIQUAL_FILES = {
    "alim": ("ciqual-alim_2025_11_03.xml", "666252",
             "e0b1de25b3039028205e9d54a96892e403e1b313c2efeb41180fabe132627478"),
    "grp": ("ciqual-alim_grp_2025_11_03.xml", "666250",
            "e216928be1001aed15ba1b120405b70a98145da2a1a76026d6e33542bd5e39dc"),
    "compo": ("ciqual-compo_2025_11_03.xml", "666249",
              "8c46a9032ece4eab4ffccc9dfcb0c490ec2f416aa664cc1ee013b241c6bdd4af"),
}
# blsdb.de hands the archive out behind a link that carries a token made when the page is drawn.
BLS_PAGE = "https://blsdb.de/download"
BLS_FILE = ("BLS_4_0_2025_DE.zip", "12b7a6ba62807ec9b301eb276f897dc85f99b2292311618dec3749a12d984c91")
USDA = "https://fdc.nal.usda.gov/fdc-datasets/"
USDA_FILES = [
    ("FoodData_Central_survey_food_csv_2024-10-31.zip",
     "5ccc25ec2777a8982fbb61378a42f415316173eb11e48c9a8ba4cb19f5a4f29c", "survey_fndds_food"),
    ("FoodData_Central_foundation_food_csv_2026-04-30.zip",
     "70457ee9d9342f43bda2010318c85f04210c689fdeb9cd2da4c513b0e8dbc655", "foundation_food"),
]

# ── which constituents are kept, and where each table has them ───────────────────────────────────
# (key, INFOODS identifier, Ciqual constituent code, BLS component code, USDA nutrient numbers —
# first present wins). `None` where a table has no constituent of that definition. The key's
# ending is the unit stored: `_mg` for what is weighed in grams, `_ug` for milligrams or less.
NUTRIENTS = [
    ("kcal", "ENERC", "328", "ENERCC", ["208", "958", "957"]),
    ("protein_mg", "PROCNT", "25003", "PROT625", ["203"]),
    ("carbs_mg", "CHOAVL", "31000", "CHO", None),         # USDA: derived, see USDA_TOTAL_CARBS
    ("fat_mg", "FAT", "40000", "FAT", ["204"]),
    ("fiber_mg", "FIB-", "34100", "FIBT", ["291"]),
    ("sugars_mg", "SUGAR", "32000", "SUGAR", ["269", "269.3"]),
    ("saturated_fat_mg", "FASAT", "40302", "FASAT", ["606"]),
    ("water_mg", "WATER", "400", "WATER", ["255"]),
    ("alcohol_mg", "ALC", "60000", "ALC", ["221"]),
    ("sodium_ug", "NA", "10110", "NA", ["307"]),
    ("potassium_ug", "K", "10190", "K", ["306"]),
    ("calcium_ug", "CA", "10200", "CA", ["301"]),
    ("iron_ug", "FE", "10260", "FE", ["303"]),
    ("magnesium_ug", "MG", "10120", "MG", ["304"]),
    ("phosphorus_ug", "P", "10150", "P", ["305"]),
    ("zinc_ug", "ZN", "10300", "ZN", ["309"]),
    ("copper_ug", "CU", "10290", "CU", ["312"]),
    ("manganese_ug", "MN", "10251", "MN", ["315"]),
    ("selenium_ug", "SE", "10340", None, ["317"]),
    ("iodine_ug", "ID", "10530", "ID", None),
    ("vitamin_a_rae_ug", "VITA_RAE", "51104", "VITAA", ["320"]),
    ("vitamin_c_ug", "VITC", "55100", "VITC", ["401"]),
    ("vitamin_d_ug", "VITD", "52100", "VITD", ["328"]),
    ("vitamin_e_ug", "TOCPHA", "71010", "VITE", ["323"]),
    ("vitamin_k1_ug", "VITK1", "54101", "VITK1", ["430"]),
    ("thiamin_ug", "THIA", "56100", "THIA", ["404"]),
    ("riboflavin_ug", "RIBF", "56200", "RIBF", ["405"]),
    ("niacin_ug", "NIA", "56310", "NIA", ["406"]),
    ("pantothenic_acid_ug", "PANTAC", "56400", "PANTAC", ["410"]),
    ("vitamin_b6_ug", "VITB6", "56500", "VITB6", ["415"]),
    ("vitamin_b12_ug", "VITB12", "56600", "VITB12", ["418"]),
    ("folate_dfe_ug", "FOLDFE", "56702", "FOL", ["435"]),
    ("cholesterol_ug", "CHOL-", "75100", "CHORL", ["601"]),
    ("choline_ug", "CHOLN", None, None, ["421"]),
    ("caffeine_ug", "CAFFN", None, None, ["262"]),
]
# USDA's carbohydrate by difference (CHOCDF). Never stored: a food's `carbs_mg` is this minus its
# total dietary fibre — available carbohydrate by difference, CHOAVLDF in the FAO/INFOODS
# guidelines for checking food composition data (2012).
USDA_TOTAL_CARBS = "205"
# How many of a key's unit one of a table's unit is.
PER = {
    "kcal": {"kcal": 1},
    "mg": {"g": 1000, "mg": 1},
    "ug": {"g": 1_000_000, "mg": 1000, "ug": 1},
}
# The unit each Ciqual constituent kept here is published in, read from its English name
# (`Protein (g/100g)`). Written out, not parsed: one of the names has a misprint in its unit.
CIQUAL_UNIT = {
    "328": "kcal", "25003": "g", "31000": "g", "40000": "g", "34100": "g", "32000": "g",
    "40302": "g", "400": "g", "60000": "g", "10110": "mg", "10190": "mg", "10200": "mg",
    "10260": "mg", "10120": "mg", "10150": "mg", "10300": "mg", "10290": "mg", "10251": "mg",
    "10340": "ug", "10530": "ug", "51104": "ug", "55100": "mg", "52100": "ug", "71010": "mg",
    "54101": "ug", "56100": "mg", "56200": "mg", "56310": "mg", "56400": "mg", "56500": "mg",
    "56600": "ug", "56702": "ug", "75100": "mg",
}
MAX_PORTIONS = 6


def unit_of(key):
    return "kcal" if key == "kcal" else key.rsplit("_", 1)[1]


def checked(path, want, what):
    data = open(path, "rb").read()
    got = hashlib.sha256(data).hexdigest()
    if got != want:
        sys.exit(f"{what}: checksum is {got}, expected {want}. The upstream file changed — "
                 f"look at it, then pin the new checksum and bump VERSION.")
    return data


def fetch(cache, name, url, want):
    """An input's bytes: from the cache if it is there, else downloaded; always checked."""
    path = os.path.join(cache, name)
    if not os.path.exists(path):
        print(f"  downloading {name}", file=sys.stderr)
        os.makedirs(cache, exist_ok=True)
        with urllib.request.urlopen(url, timeout=600) as r, open(path + ".part", "wb") as f:
            f.write(r.read())
        os.replace(path + ".part", path)
    return checked(path, want, name)


def fetch_bls(cache):
    name, want = BLS_FILE
    path = os.path.join(cache, name)
    if not os.path.exists(path):
        page = html.unescape(urllib.request.urlopen(BLS_PAGE, timeout=60).read().decode("utf-8", "replace"))
        link = re.search(r'href="([^"]+\.zip\?token=[^"]+)"', page)
        if not link:
            sys.exit(f"{name}: no download link on {BLS_PAGE}. Fetch it by hand into {cache}/.")
        return fetch(cache, name, "https://blsdb.de" + link.group(1), want)
    return checked(path, want, name)


# ── Ciqual ───────────────────────────────────────────────────────────────────────────────────────

def text(el, tag):
    v = el.findtext(tag)
    return v.strip() if v else ""


def ciqual_number(s):
    """A Ciqual value: a number with a decimal comma; `traces` and `< x` are 0; `-` is no value."""
    s = s.strip()
    if not s or s == "-":
        return None
    if s == "traces" or s.startswith("<"):
        return 0.0
    try:
        return float(s.replace(",", "."))
    except ValueError:
        return None


def read_ciqual(cache):
    files = {k: fetch(cache, n, CIQUAL + fid, sha) for k, (n, fid, sha) in CIQUAL_FILES.items()}
    groups = {}
    for g in ET.fromstring(files["grp"].decode("utf-8-sig")).iter("ALIM_GRP"):
        groups[text(g, "alim_grp_code")] = text(g, "alim_grp_nom_eng")
    wanted = {c: key for key, _, c, _, _ in NUTRIENTS if c}
    values = {}
    for ev, el in ET.iterparse(io.BytesIO(files["compo"])):
        if el.tag != "COMPO":
            continue
        key = wanted.get(text(el, "const_code"))
        if key:
            n = ciqual_number(text(el, "teneur"))
            if n is not None:
                per = PER[unit_of(key)][CIQUAL_UNIT[text(el, "const_code")]]
                values.setdefault(text(el, "alim_code"), {})[key] = round(n * per)
        el.clear()
    for a in ET.fromstring(files["alim"].decode("utf-8-sig")).iter("ALIM"):
        code = text(a, "alim_code")
        v = values.get(code, {})
        name = text(a, "alim_nom_eng")
        if "kcal" in v and name:
            yield f"ciqual:{code}", name, "ciqual", groups.get(text(a, "alim_grp_code"), ""), v, []


# ── BLS ──────────────────────────────────────────────────────────────────────────────────────────

SS = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"


def xlsx_rows(data):
    """The first sheet of a workbook, row by row, as {column letters: text}."""
    z = zipfile.ZipFile(io.BytesIO(data))
    strings = []
    if "xl/sharedStrings.xml" in z.namelist():
        for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall(SS + "si"):
            strings.append("".join(t.text or "" for t in si.iter(SS + "t")))
    for ev, el in ET.iterparse(z.open("xl/worksheets/sheet1.xml")):
        if el.tag != SS + "row":
            continue
        row = {}
        for c in el:
            v = c.find(SS + "v")
            if v is None or v.text is None:
                continue
            col = re.match(r"[A-Z]+", c.get("r")).group(0)
            row[col] = strings[int(v.text)] if c.get("t") == "s" else v.text
        yield row
        el.clear()


def read_bls(cache):
    z = zipfile.ZipFile(io.BytesIO(fetch_bls(cache)))
    member = lambda end: z.read(next(n for n in z.namelist() if n.endswith(end)))
    # The unit of each component, from the table that comes with the data.
    units = {}
    for r in xlsx_rows(member("Components_DE_EN.xlsx")):
        if r.get("B") and r.get("E"):
            units[r["B"]] = r["E"].replace("µ", "u")
    wanted = {c: key for key, _, _, c, _ in NUTRIENTS if c}
    rows = xlsx_rows(member("Daten_2025_DE.xlsx"))
    head = next(rows)
    # A value column is headed `<CODE> <name> [<unit>/100g]`; the two columns after it say where
    # the value came from, and are not values.
    cols = {}
    for col, title in head.items():
        code = title.split(" ", 1)[0]
        if code in wanted and "/100g]" in title:
            cols[col] = (wanted[code], PER[unit_of(wanted[code])][units[code]])
    name_col = next(c for c, t in head.items() if t == "Food name")
    code_col = next(c for c, t in head.items() if t == "BLS Code")
    for r in rows:
        code, name = r.get(code_col, "").strip(), r.get(name_col, "").strip()
        if not code or not name:
            continue
        v = {}
        for col, (key, per) in cols.items():
            try:
                v[key] = round(float(r[col]) * per)
            except (KeyError, ValueError):
                pass
        if "kcal" in v:
            # The first letter of the code is the food's main group, but BLS 4.0 does not publish
            # the groups' names, so the food carries none.
            yield f"bls:{code}", name, "bls", "", v, []


# ── USDA, the fallback ───────────────────────────────────────────────────────────────────────────

def usda_table(z, name):
    member = next(n for n in z.namelist() if n.endswith("/" + name) or n == name)
    return csv.DictReader(io.TextIOWrapper(z.open(member), encoding="utf-8", newline=""))


def portion_label(row, units):
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


def read_usda(cache):
    wanted = {}
    for key, _, _, _, numbers in NUTRIENTS:
        for at, nbr in enumerate(numbers or []):
            wanted[nbr] = (key, at)
    wanted[USDA_TOTAL_CARBS] = ("total_carbs_mg", 0)
    unit = {"KCAL": "kcal", "G": "g", "MG": "mg", "UG": "ug"}
    for name, sha, keep in USDA_FILES:
        z = zipfile.ZipFile(io.BytesIO(fetch(cache, name, USDA + name, sha)))
        nutrients = list(usda_table(z, "nutrient.csv"))
        by_id = {n["id"]: n for n in nutrients}
        by_nbr = {n["nutrient_nbr"]: n for n in nutrients}
        names = {r["fdc_id"]: r["description"].strip()
                 for r in usda_table(z, "food.csv") if r["data_type"] == keep}
        # **The survey dataset writes the nutrient's *number* where the others write its id.**
        # Tell which by looking: a column whose values are nearly all ids holds ids.
        rows = [r for r in usda_table(z, "food_nutrient.csv") if r["fdc_id"] in names]
        sample = rows[:2000]
        ids_are_ids = sum(r["nutrient_id"] in by_id for r in sample) > 0.9 * len(sample)
        values, best = {}, {}
        for r in rows:
            n = by_id.get(r["nutrient_id"]) if ids_are_ids else by_nbr.get(r["nutrient_id"])
            if not n or not r["amount"]:
                continue
            # The number exactly: `323.99` (all tocopherols) is not `323` (alpha-tocopherol).
            hit = wanted.get(n["nutrient_nbr"])
            if not hit:
                continue
            key, priority = hit
            per = PER[unit_of(key)].get(unit.get(n["unit_name"], ""))
            if per is None:
                continue
            slot = (r["fdc_id"], key)
            if slot in best and best[slot] <= priority:
                continue
            best[slot] = priority
            values.setdefault(r["fdc_id"], {})[key] = round(float(r["amount"]) * per)
        measures = {r["id"]: r["name"] for r in usda_table(z, "measure_unit.csv")}
        portions = {}
        for r in sorted(usda_table(z, "food_portion.csv"),
                        key=lambda r: (r["fdc_id"], int(r["seq_num"] or 0))):
            if r["fdc_id"] not in names:
                continue
            try:
                grams = round(float(r["gram_weight"]))
            except ValueError:
                continue
            label = portion_label(r, measures)
            mine = portions.setdefault(r["fdc_id"], [])
            if grams < 1 or not label or len(mine) >= MAX_PORTIONS or any(l == label for l, _ in mine):
                continue
            mine.append((label, grams))
        for fdc, title in names.items():
            v = values.get(fdc, {})
            total = v.pop("total_carbs_mg", None)
            if total is not None and "fiber_mg" in v:
                v["carbs_mg"] = max(0, total - v["fiber_mg"])
            if "kcal" in v:
                yield f"usda:{fdc}", title, "usda", "", v, portions.get(fdc, [])


# ── the list ─────────────────────────────────────────────────────────────────────────────────────

def build(cache, out, embed):
    if os.path.exists(out):
        os.remove(out)
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    db = sqlite3.connect(out)
    # The layout is the reader's as much as this script's, so it lives with the reader.
    here = os.path.dirname(os.path.abspath(__file__))
    layout = open(os.path.join(here, "..", "crates", "fm-core", "src", "pack.sql"), encoding="utf-8").read()
    db.executescript("PRAGMA page_size = 4096;\n" + layout)
    keys = {key: i for i, (key, *_) in enumerate(NUTRIENTS)}
    db.executemany("INSERT INTO nutrient(n, key) VALUES (?, ?)", [(i, k) for k, i in keys.items()])
    counts, fid = {}, 0
    for source in (read_ciqual, read_bls, read_usda):
        for code, title, src, grp, values, portions in source(cache):
            fid += 1
            db.execute("INSERT INTO food(id, code, name, src, grp) VALUES (?, ?, ?, ?, ?)",
                       (fid, code, title, src, grp))
            db.executemany("INSERT INTO amount VALUES (?, ?, ?)",
                           [(fid, keys[k], v) for k, v in values.items()])
            db.executemany("INSERT INTO portion VALUES (?, ?, ?, ?)",
                           [(fid, i, label, grams) for i, (label, grams) in enumerate(portions)])
            counts[src] = counts.get(src, 0) + 1
    db.execute("INSERT INTO names(rowid, name) SELECT id, name FROM food")
    meta = {
        "name": NAME,
        "version": VERSION,
        "foods": str(sum(counts.values())),
        "title": "Foods: Ciqual 2025 and BLS 4.0, with USDA FoodData Central as a fallback",
        "licence": "Ciqual: Etalab Open Licence 2.0. BLS: CC BY 4.0. USDA: U.S. Government work",
        "attribution": (
            "Anses (2025), Ciqual French food composition table, doi:10.57745/RDMHWY. "
            "Max Rubner-Institut (2025), Bundeslebensmittelschlüssel (BLS), Version 4.0, "
            "doi:10.25826/Data20251217-134202-0. "
            "U.S. Department of Agriculture, Agricultural Research Service, FoodData Central. "
            "A subset of each table's constituents, converted to whole numbers per 100 g."
        ),
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
        # What the app carries. `mtime=0` and no name, so the same list always compresses to the
        # same bytes.
        os.makedirs(os.path.dirname(embed), exist_ok=True)
        with open(embed, "wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as z:
            z.write(data)
        print(f"{embed}")
        print(f"  bytes:   {os.path.getsize(embed)}")


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--cache", default="target/packs/src", help="where the source files are kept")
    p.add_argument("--out", default="target/packs/foods.sqlite")
    p.add_argument("--embed", default="crates/fm-app/packs/foods.sqlite.gz",
                   help="the compressed copy the app carries; empty to skip")
    a = p.parse_args()
    build(a.cache, a.out, a.embed)
