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
  - CREA, Tabelle di composizione degli alimenti, 2019 update
                       Centro di ricerca Alimenti e Nutrizione, Italy — about 830 foods. Its
                       own terms: copying with the source named. Published as a page a food and
                       no file, so it is read from a snapshot of those pages made by
                       `ci/fetch-crea.py` and kept in `ci/data/`.
  - Swiss Food Composition Database 7.1
                       Federal Food Safety and Veterinary Office, Switzerland — its generic
                       foods, about 1,200. Free to use, commercially too, with the source
                       acknowledged. Four workbooks, one a language: the food's names in Italian,
                       German and French are kept beside the English one and searched.
  - FoodData Central   USDA — Foundation Foods and Survey Foods (FNDDS) only, and only as a
                       fallback: it has the household portions ("1 cup") that neither European
                       table has, and some foods they do not list. U.S. Government work.

Each input is pinned below by checksum, so the same script makes the same list, and a changed
upstream file is a refusal here, not a silently different list.

**What comes out, and what was changed on the way.** Every value per 100 g, as a whole number in
the unit its key names — `kcal`, `protein_mg`, `iron_ug` — the form a logged entry stores. A
subset of each table's constituents is kept (the ones below). Ciqual's *traces* are stored as 0,
its *< x* as x/2 (since 2026-10-29), and its *-* (no value) as no value at all. Nothing is averaged
across tables. **One exception, since 2026-10-29:** a nutrient a food's own table carries for almost
none of its foods (choline in every European table; most vitamins in CREA's) is borrowed from USDA's
closest food, and the pack records where it came from (see *borrowing*, below).

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
import random
import re
import sqlite3
import sys
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

# The list's own version: bump it whenever an input, the constituents or the layout changes.
# An entry records `<name>@<version>/<food>` as where its numbers came from.
NAME = "foods"
VERSION = "2026.10.6"

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
# ending is the unit stored: `_mg` for what is weighed in grams, `_ug` for milligrams or less, and
# `_ng` for vitamin B12 and D, whose foods hold fractions of a microgram: in whole micrograms, milk's
# B12 (about 0.4 µg/100 g) was stored as 0 (`decisions.md#data`, 2026-10-29).
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
    ("vitamin_d_ng", "VITD", "52100", "VITD", ["328"]),
    ("vitamin_e_ug", "TOCPHA", "71010", "VITE", ["323"]),
    ("vitamin_k1_ug", "VITK1", "54101", "VITK1", ["430"]),
    ("thiamin_ug", "THIA", "56100", "THIA", ["404"]),
    ("riboflavin_ug", "RIBF", "56200", "RIBF", ["405"]),
    ("niacin_ug", "NIA", "56310", "NIA", ["406"]),
    ("pantothenic_acid_ug", "PANTAC", "56400", "PANTAC", ["410"]),
    ("vitamin_b6_ug", "VITB6", "56500", "VITB6", ["415"]),
    ("vitamin_b12_ng", "VITB12", "56600", "VITB12", ["418"]),
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
    "ng": {"g": 1_000_000_000, "mg": 1_000_000, "ug": 1000, "ng": 1},
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
    """A Ciqual value: a number with a decimal comma; `traces` is 0; `< x` (below the limit of
    detection or quantification) is x/2, the middle bound of EFSA's guidance on left-censored data
    (2010) — small, not nothing (`decisions.md#data`, 2026-10-29: it had been 0, and 46 % of Ciqual's
    selenium values are given this way); `-` is no value."""
    s = s.strip()
    if not s or s == "-":
        return None
    if s == "traces":
        return 0.0
    if s.startswith("<"):
        n = ciqual_number(s[1:])
        return None if n is None else n / 2
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
            # Its French name is its own; the two differ for most foods.
            fr = text(a, "alim_nom_fr")
            yield (f"ciqual:{code}", name, "ciqual", groups.get(text(a, "alim_grp_code"), ""), v, [],
                   fr if fr.lower() != name.lower() else "")


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
    de_col = next(c for c, t in head.items() if t == "Lebensmittelbezeichnung")
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
            de = r.get(de_col, "").strip()
            yield f"bls:{code}", name, "bls", "", v, [], de if de.lower() != name.lower() else ""


# ── the Swiss table ──────────────────────────────────────────────────────────────────────────────
# One workbook a language, the same foods in each by `ID`. The English one gives the name and the
# values; the Italian, German and French ones give the food's other names and synonyms.
SWISS = "https://naehrwertdaten.ch/wp-content/uploads/2026/07/"
SWISS_FILES = {
    "en": ("Swiss_food_composition_database.xlsx",
           "323bae2eecd639d2c3b3bf3797d30b1426ccb0253fb62810f522778b38089196"),
    "it": ("Banca_dati_svizzera_dei_valori_nutritivi.xlsx",
           "7427ce03b3d20a0e676935c5bc6c06152b5e1f204e845afbe45d311abddeae23"),
    "de": ("Schweizer_Nahrwertdatenbank.xlsx",
           "f4bb854944ef811f9b8463984389f8121e73278c4fd24fc3e8d16336cb711270"),
    "fr": ("Base_de_donnees_suisse_des_valeurs_nutritives.xlsx",
           "1692aa241728f25d4823a6c97bb36c393f7f64173a00a8ed4175b1bf926a5405"),
}
# key → the column's heading in the English workbook. Each is used under the INFOODS identifier
# the workbook's own *Nutrient codes* sheet gives it, read 2026-10-13. Left out because the
# definition differs: folate (theirs is total folate, FOL; the key is FOLDFE) and vitamin B6
# (theirs is VITB6-, method unknown). Salt is left out for sodium, which they also publish.
SWISS_COLUMNS = {
    "kcal": "Energy, kilocalories (kcal)",                 # ENERC
    "protein_mg": "Protein (g)",                           # PROCNT (their PROT625)
    "carbs_mg": "Carbohydrates, available (g)",            # CHOAVL
    "fat_mg": "Fat, total (g)",                            # FAT
    "fiber_mg": "Dietary fibres (g)",                      # FIB-
    "sugars_mg": "Sugars (g)",                             # SUGAR
    "saturated_fat_mg": "Fatty acids, saturated (g)",      # FASAT
    "water_mg": "Water (g)",                               # WATER
    "alcohol_mg": "Alcohol (g)",                           # ALC
    "sodium_ug": "Sodium (Na) (mg)",                       # NA
    "potassium_ug": "Potassium (K) (mg)",                  # K
    "calcium_ug": "Calcium (Ca) (mg)",                     # CA
    "iron_ug": "Iron (Fe) (mg)",                           # FE
    "magnesium_ug": "Magnesium (Mg) (mg)",                 # MG
    "phosphorus_ug": "Phosphorus (P) (mg)",                # P
    "zinc_ug": "Zinc (Zn) (mg)",                           # ZN
    "selenium_ug": "Selenium (Se) (µg)",                   # SE
    "iodine_ug": "Iodide (I) (µg)",                        # ID
    "vitamin_a_rae_ug": "Vitamin A activity, RAE (µg)",    # VITA_RAE
    "vitamin_c_ug": "Vitamin C (ascorbic acid) (mg)",      # VITC
    "vitamin_d_ng": "Vitamin D (calciferol) (µg)",         # VITD
    "vitamin_e_ug": "Vitamin E (α-tocopherol) (mg)",       # TOCPHA
    "thiamin_ug": "Vitamin B1 (thiamine) (mg)",            # THIA
    "riboflavin_ug": "Vitamin B2 (riboflavin) (mg)",       # RIBF
    "niacin_ug": "Niacin (mg)",                            # NIA
    "pantothenic_acid_ug": "Pantothenic acid (mg)",        # PANTAC
    "vitamin_b12_ng": "Vitamin B12 (cobalamin) (µg)",      # VITB12
    "cholesterol_ug": "Cholesterol (mg)",                  # CHOL-
}
SWISS_PER_100G = "per 100g edible portion"
# One food's English row leaves the reference unit blank (a four-cheese pizza, in 7.1); the same
# food's Italian row states it. The table is one table: the other workbook's word is taken.
SWISS_PER_100G_IT = "per 100 g di parte edibile"


def squeeze(s):
    return " ".join(html.unescape(s).split())


def swiss_number(s):
    """A Swiss value: `tr.` (traces) and `<x` are 0, as Ciqual's are; `n.d.` and blank are none."""
    s = squeeze(s)
    if not s:
        return None
    if s.startswith("<") or s.lower().startswith("tr"):
        return 0.0
    try:
        return float(s)
    except ValueError:
        return None


def swiss_sheet(cache, lang):
    """The generic foods of one language's workbook: (the heading row, the food rows)."""
    name, sha = SWISS_FILES[lang]
    rows = xlsx_rows(fetch(cache, name, SWISS + name, sha))
    for r in rows:
        if r.get("A") == "ID":
            return r, list(rows)
    sys.exit(f"{name}: no heading row. The workbook's layout changed — look at it.")


def read_swiss(cache):
    head, rows = swiss_sheet(cache, "en")
    col = {squeeze(t): c for c, t in head.items()}
    cols = {}
    for key, title in SWISS_COLUMNS.items():
        if title not in col:
            sys.exit(f"Swiss table: no column headed {title!r}. Its layout changed — look at it.")
        unit = re.search(r"\((kcal|g|mg|µg)\)$", title).group(1).replace("µ", "u")
        cols[col[title]] = (key, PER[unit_of(key)][unit])
    name_c, syn_c, cat_c, ref_c = col["Name"], col["Synonyms"], col["Category"], col["Matrix unit"]
    # The other languages' names and synonyms, by the food's ID. The columns are where the English
    # workbook has them: the four are one table in four languages.
    others, ref_it = {}, {}
    for lang in ("it", "de", "fr"):
        _, lrows = swiss_sheet(cache, lang)
        for r in lrows:
            if lang == "it":
                ref_it[r.get("A", "")] = squeeze(r.get(ref_c, ""))
            others.setdefault(r.get("A", ""), []).extend(
                [squeeze(r.get(name_c, ""))] + [squeeze(x) for x in r.get(syn_c, "").split(";")])
    for r in rows:
        code, name = r.get("A", "").strip(), squeeze(r.get(name_c, ""))
        if not code or not name:
            continue
        ref = squeeze(r.get(ref_c, ""))
        if ref != SWISS_PER_100G and not (ref == "" and ref_it.get(code) == SWISS_PER_100G_IT):
            sys.exit(f"Swiss table: food {code} is {ref!r}, not per 100 g. Look at it.")
        v = {}
        for c, (key, per) in cols.items():
            n = swiss_number(r.get(c, ""))
            if n is not None:
                v[key] = round(n * per)
        if "kcal" not in v:
            continue
        alt, seen = [], {name.lower()}
        for n in [squeeze(x) for x in r.get(syn_c, "").split(";")] + others.get(code, []):
            if n and n.lower() not in seen:
                seen.add(n.lower())
                alt.append(n)
        # The category is `Main/Sub`; the main one is the group.
        yield (f"swiss:{code}", name, "swiss", squeeze(r.get(cat_c, "")).split("/")[0], v, [],
               " · ".join(alt))


# ── CREA, Italy ──────────────────────────────────────────────────────────────────────────────────
# CREA publishes a page a food and no file, so its input is a snapshot of those pages made by
# `ci/fetch-crea.py` and kept in the repository (`decisions.md#data`, 2026-10-13). Pinned like the
# others: a snapshot made again is a different file, and that is a refusal here.
CREA_SNAPSHOT = ("crea-2019.json.gz", "a4e54838e276007a4f20efe977d5a29a5bbcec090a195bbfc1e18ceff40d838e")
# A line of a food's page, by its printed name → (key, the unit it is printed in). Read against
# CREA's *Presentazione dati*: energy by Southgate's factors; protein from nitrogen by the
# FAO/WHO (1973) factor for the food; fibre by the AOAC enzymatic-gravimetric method. Left out
# because the definition is not the key's: available carbohydrate (monosaccharide equivalents —
# see CREA_CARBS), vitamin A (retinol equivalents, β-carotene ÷ 6), vitamin E (α-tocopherol
# equivalents), vitamin B6 (not said). (Folate is kept since 2026-10-29: see its line.)
CREA_LINES = {
    "Energia (kcal)": ("kcal", "kcal"),
    "Proteine (g)": ("protein_mg", "g"),
    "Lipidi (g)": ("fat_mg", "g"),
    "Fibra totale (g)": ("fiber_mg", "g"),
    "Zuccheri solubili (g)": ("sugars_mg", "g"),     # as monosaccharides: see CREA_CARBS
    "Acqua (g)": ("water_mg", "g"),
    "Alcool (g)": ("alcohol_mg", "g"),
    "Colesterolo (mg)": ("cholesterol_ug", "mg"),
    "Sodio (mg)": ("sodium_ug", "mg"),
    "Potassio (mg)": ("potassium_ug", "mg"),
    "Calcio (mg)": ("calcium_ug", "mg"),
    "Ferro (mg)": ("iron_ug", "mg"),
    "Magnesio (mg)": ("magnesium_ug", "mg"),
    "Fosforo (mg)": ("phosphorus_ug", "mg"),
    "Zinco (mg)": ("zinc_ug", "mg"),
    "Rame (mg)": ("copper_ug", "mg"),
    "Manganese (mg)": ("manganese_ug", "mg"),
    "Selenio (μg)": ("selenium_ug", "ug"),
    "Vitamina C (mg)": ("vitamin_c_ug", "mg"),
    "Vitamina D (μg)": ("vitamin_d_ng", "ug"),
    "Tiamina (mg)": ("thiamin_ug", "mg"),
    "Riboflavina (mg)": ("riboflavin_ug", "mg"),
    "Niacina (mg)": ("niacin_ug", "mg"),
    # **Folate, as dietary folate equivalents** (2026-10-29). CREA prints total food folate; DFE adds
    # 1.7 × folic acid, which only a fortified food has, and none of CREA's 89 foods with this line
    # is fortified — so for them the two are equal.
    "Folati (μg)": ("folate_dfe_ug", "ug"),
}
# **Carbohydrate.** CREA's *Carboidrati disponibili* is starch × 1.10 plus sugars, in
# monosaccharide equivalents (CHOAVLM), which is not what `carbs_mg` holds (CHOAVL, by weight).
# Its *Amido* is starch as a polysaccharide — by weight — and its *Zuccheri solubili* are sugars
# as monosaccharides, at most 5 % above their weight (a disaccharide yields 105 g of
# monosaccharides per 100 g). So `carbs_mg` is starch plus soluble sugars, as printed: exact for
# the starch, and for the sugars high by up to a twentieth of the sugars. A food without both
# lines has no carbohydrate here, unless its available carbohydrate is printed as 0.
CREA_CARBS = ("Amido (g)", "Zuccheri solubili (g)")
# **Saturated fat** (corrected 2026-10-29). CREA prints each class of fatty acids as a percentage,
# and its three classes sum to 100.2 % (the median of its 244 foods that have all three) — they are
# shares of the *fatty acids*, not of the fat, which also holds glycerol, phospholipids and sterols.
# So the grams are fat × the fatty-acid conversion factor (XFA) × the share ÷ 100: FAO/INFOODS,
# *Guidelines for Converting Units, Denominators and Expressions* (2012), Tables 4.4-5 and 4.4-6.
# Read as a share of the fat, as it was until then, an egg had 3.96 g where the other tables give
# 2.4–3.1 g. A food no factor in those tables fits has no saturated-fat value: none is guessed.
CREA_SATURATED = "Acidi grassi Saturi (%)"


def crea_xfa(category, name, fat_g):
    """The FAO/INFOODS fatty-acid conversion factor (g fatty acids per g fat) for a CREA food, or
    None when its tables give none for it."""
    n = name.lower()
    has = lambda *ws: any(w in n for w in ws)
    fatty = fat_g >= 20
    if category == "Uova":
        return 0.83
    if category in ("Latte e yogurt", "Formaggi e latticini") or has("burro") and not has("arachid"):
        return 0.945                                     # milk and milk products
    if category == "Oli e grassi":
        if has("cocco"):
            return 0.942                                 # coconut oil
        if has("lardo", "strutto", "sugna"):
            return 0.953                                 # pork, fat
        return 0.956                                     # fats and oils, and peanut butter as nuts
    if category == "Frutta secca a guscio e semi oleaginosi":
        return 0.956                                     # nuts
    if category == "Frattaglie":
        for word, x in (("fegato", 0.741), ("cuore", 0.789), ("rene", 0.747), ("rognon", 0.747),
                        ("cervell", 0.561)):
            if word in n:
                return x
        return None
    if category in ("Carni fresche", "Carni trasformate e conservate"):
        if has("pollo", "tacchino", "faraona", "oca", "anatra", "gallina", "cappone"):
            return 0.945                                 # poultry
        if has("maiale", "suino", "prosciutto", "salam", "mortadella", "coppa", "pancetta",
               "speck", "cotechino", "zampone", "ciccioli", "wurstel"):
            return 0.953 if fatty else 0.910             # pork, fat or lean
        if has("bovino", "vitell", "bufalo", "bresaola", "manzo", "agnell", "pecora", "montone"):
            return 0.953 if fatty else 0.916             # beef (lamb taken as beef), fat or lean
        return None                                      # horse, goat, rabbit, game: no factor
    if category == "Prodotti della pesca":
        if has("calamar", "cozz", "mitil", "vongol", "polpo", "seppi", "ostric", "totan",
               "moscardin", "lumach"):
            return 0.956 - 0.296 / fat_g if fat_g >= 0.55 else 0.417   # molluscs
        if has("gamber", "scamp", "aragost", "astice", "granch", "mazzancoll", "canoc"):
            return 0.956 - 0.237 / fat_g if fat_g >= 0.55 else 0.459   # crustaceans
        return 0.933 - 0.143 / fat_g if fat_g >= 0.55 else 0.673       # finfish
    if category == "Cereali e derivati":
        if has("crusca"):
            return 0.82                                  # bran
        if has("avena"):
            return 0.94                                  # oats
        if has("riso"):
            return 0.85                                  # rice, milled
        if has("farina di frumento", "semola"):
            return 0.67                                  # wheat flour
        if has("frumento", "orzo") and not has("germe"):
            return 0.72                                  # wheat, barley, wholegrain
        return None
    if category in ("Verdure e ortaggi", "Frutta"):
        return 0.956 if has("avocado") else 0.80
    return None


def crea_number(s):
    """A CREA value: `tr` (present in traces) is 0; blank is no value."""
    s = s.strip()
    if not s:
        return None
    if s == "tr":
        return 0.0
    try:
        return float(s.replace(",", "."))
    except ValueError:
        return None


def read_crea(cache):
    import json
    name, sha = CREA_SNAPSHOT
    here = os.path.dirname(os.path.abspath(__file__))
    snapshot = json.loads(gzip.decompress(checked(os.path.join(here, "data", name), sha, name)))
    for f in snapshot["foods"]:
        got = {}
        for line in f["lines"]:
            if line["section"].startswith("AMINO"):
                continue
            n = crea_number(line["per100"])
            if n is not None:
                got.setdefault(line["name"], n)
        v = {}
        for printed, (key, unit) in CREA_LINES.items():
            if printed in got:
                v[key] = round(got[printed] * PER[unit_of(key)][unit])
        if all(x in got for x in CREA_CARBS):
            v["carbs_mg"] = round(sum(got[x] for x in CREA_CARBS) * 1000)
        elif got.get("Carboidrati disponibili (g)") == 0:
            # None is none in any unit.
            v["carbs_mg"] = 0
        if CREA_SATURATED in got and got.get("Lipidi (g)"):
            xfa = crea_xfa(f["category"], f["name"], got["Lipidi (g)"])
            if xfa is not None:
                v["saturated_fat_mg"] = round(got["Lipidi (g)"] * xfa * got[CREA_SATURATED] / 100 * 1000)
        italian, english = f["name"].strip(), f["english"].strip()
        if "kcal" not in v or not (italian or english):
            continue
        title = english or italian
        portion = re.fullmatch(r"(\d+) g", f["portion"].strip())
        yield (f"crea:{f['code']}", title, "crea", f["category"], v,
               [("1 portion", int(portion.group(1)))] if portion else [],
               italian if italian.lower() != title.lower() else "")


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
                yield f"usda:{fdc}", title, "usda", "", v, portions.get(fdc, []), ""


# ── borrowing what a food's table did not measure ────────────────────────────────────────────────
# `decisions.md#data`, 2026-10-29 and (reworked) 2026-10-29. Choline is in USDA's table alone; CREA
# measures no vitamin B12, D, A, E, K1, B6 or iodine, the Swiss table no K1, B6, folate, copper or
# manganese, BLS no selenium; and every table leaves single foods unanalysed (a third of Ciqual's lack
# selenium). **Any value a food lacks** is taken from the closest food of another table that has it:
#
# 1. **USDA lends first**, to every European food it matches.
# 2. **Then the European tables lend each other** what they hold — measured, or lent by USDA in step 1
#    (the value keeps its origin: the food that measured it). A European measurement is nearer a
#    European food than USDA's, and BLS and the Swiss table measure most vitamins for nearly all
#    their foods.
#
# A match is a food of the same kind (the shorter name's head is inside the other's), in the same
# state (fried or not, preserved or fresh, dried or not, smoked or cured or not, a juice or not, a
# dish or not), whose names share enough
# words and whose macronutrients agree. Names are read alike first (`norm`): "Chicken Egg, whole,
# boiled, a la coque or hard-boiled" and USDA's "Egg, whole, boiled or poached" are one food, which
# the first version missed by one percent of name likeness. A measured value is never replaced,
# nothing is averaged, and where each value came from is kept (`borrowed`).
#
# **American fortification is not lent** (`lends`): US enriched grain carries iron, folic acid,
# thiamin, riboflavin and niacin by law, and US milk vitamin D (and A, when reduced in fat); a
# European flour or milk mostly does not.
#
# Measured on the 2026-10-29 build (`docs/context/nutrient-data-audit-2026-10-29.md` §6), and read by
# eye on random samples of the matches.
MATCH_AT = 0.62
# Words that say how a food was handled or sold, not what it is: shared by too many foods to match on.
# (Not "bone": "Pork, bones" is a food of its own.)
NAME_NOISE = set("""raw cooked boiled fried baked roasted grilled steamed stewed fresh frozen canned
dried whole average plain with without and or in of the a to for from not ns nfs as on prepared
added salt unsalted salted drained solids liquid solid mixed type made commercial home homemade
regular all purpose including excluding skin skinless boneless meat only lean fat scrambled omelet
pan pickled""".split())
# Each macronutrient's own scale, so a difference counts as much as it matters: 20 kcal, 1 g.
MACROS = {"kcal": 20, "protein_mg": 1000, "fat_mg": 1000, "carbs_mg": 1000, "water_mg": 5000}
# The tables' names for one thing, made one: their English differs ("Chicken Egg … a la coque",
# "Egg … boiled or poached"; CREA's "scrumbled"). A hen's egg is "egg", as USDA writes it; a duck's
# stays a duck egg.
SAME = [(r"\bfrozen yogh?urts?\b", "frozenyogurt"), (r"\bomelettes?\b", "omelet"),
        (r"\bscrumbled\b", "scrambled"), (r"\byoghurts?\b", "yogurt"),
        (r"\b(hard|soft)[- ]boiled\b", "boiled"), (r"\ba la coque\b", "boiled"), (r"\bhard\b", ""),
        (r"\bpoached\b", "boiled"), (r"\b(chicken|hen'?s?) eggs?\b", "egg"),
        (r"\bwholemeal\b", "whole wheat"), (r"\bwholegrain\b", "whole grain"),
        (r"\baubergines?\b", "eggplant"), (r"\bcourgettes?\b", "zucchini"), (r"\bminced\b", "ground")]
# A dish of several foods (a sandwich, a salad, "with …") shares words with many others; it must
# share more of them to be the same dish.
DISH = set("with sandwich salad pizza burger quiche pie soup dish stew sauce casserole".split())
STATE = set("raw cooked boiled fried baked roasted grilled steamed stewed braised".split())
FRIED = {"fried", "omelet", "scrambled", "sauteed"}
KEPT = {"canned", "pickled", "syrup", "brine", "preserved", "jarred", "tinned"}
DRY = {"dried", "dry", "dehydrated", "powder", "powdered", "flakes"}
JUICE = {"juice", "nectar", "smoothie", "squash", "syrup"}
CURED = {"smoked", "cured"}
GRAIN = set("""bread roll bun bagel flour pasta macaroni spaghetti noodle cereal cracker rice tortilla
muffin biscuit cake cookie pancake waffle pretzel couscous semolina""".split())
ENRICHED = {"folate_dfe_ug", "thiamin_ug", "riboflavin_ug", "niacin_ug", "iron_ug"}


def norm(title):
    t = title.lower()
    for a, b in SAME:
        t = re.sub(a, b, t)
    return t


def name_words(title):
    out = set()
    for w in re.findall(r"[a-z]+", norm(title)):
        w = w[:-1] if len(w) > 3 and w.endswith("s") else w
        if len(w) > 2 and w not in NAME_NOISE:
            out.add(w)
    return out


def shape(i, foods):
    t = norm(foods[i][1])
    low = set(re.findall(r"[a-z]+", t))
    return {"i": i, "words": name_words(foods[i][1]), "v": foods[i][4],
            "head": name_words(re.split(r"[,(]", foods[i][1])[0]), "dish": bool(low & DISH),
            "fried": bool(low & FRIED), "kept": bool(low & KEPT), "dry": bool(low & DRY),
            "juice": bool(low & JUICE), "cured": bool(low & CURED), "raw": "raw" in low,
            "cooked": bool(low & ((STATE - {"raw"}) | FRIED))}


def likeness(a, b):
    """How alike two foods are, 0–1: four tenths their names' shared words, six tenths their
    macronutrients — or 0 when they are not alike enough to lend a value: not the same kind of food,
    not in the same state, too few shared words (half for a dish), fewer than three macronutrients
    to compare, any one far apart (fried rice's fat is four times wholegrain rice's), or apart on
    average. Names may share fewer words (a fifth) only when one holds every word of the other and
    the macronutrients agree closely: no rival variety on either side (a pecorino is not a Swiss
    cheese, an apricot kernel oil not a coconut oil, with the same fat)."""
    na, nb = a["words"], b["words"]
    if not na or not nb or not (na & nb):
        return 0.0
    # The food itself comes first in these names ("Egg, raw", "Tomato sauce, with cheese"): the
    # shorter head must be inside the other — a chicken egg is an egg, a tomato sauce is no cheese
    # sauce.
    ha, hb = sorted((a["head"], b["head"]), key=len)
    if not ha or not ha <= hb:
        return 0.0
    # And a dish lends only to a dish: raw salmon is not a salmon salad.
    if any(a[k] != b[k] for k in ("fried", "kept", "dry", "juice", "cured", "dish")):
        return 0.0
    diffs = [abs(a["v"][k] - b["v"][k]) / (max(a["v"][k], b["v"][k]) + scale)
             for k, scale in MACROS.items() if k in a["v"] and k in b["v"]]
    if len(diffs) < 3 or max(diffs) > 0.5:
        return 0.0
    macros = 1 - sum(diffs) / len(diffs)
    names = len(na & nb) / len(na | nb)
    nested = na <= nb or nb <= na
    if a["dish"] or b["dish"]:
        need = 0.5
    elif nested and macros >= 0.9 and max(diffs) <= 0.25:
        need = 0.2
    else:
        need = 0.34
    if names < need or macros < 0.75:
        return 0.0
    # Raw against cooked: the same food, but water and heat move the values; the same state is
    # preferred. Only when both names say.
    state = 0.1 if ((a["raw"] and b["cooked"]) or (a["cooked"] and b["raw"])) else 0.0
    return 0.4 * names + 0.6 * macros - state


FORTIFIED = re.compile(r"\b(?<!un)(enriched|fortified)\b")


def lends(title, key):
    """Whether a USDA food may lend `key`: not a fortified value a European food would not have."""
    t = norm(title)
    if FORTIFIED.search(t):
        return False
    head = name_words(re.split(r"[,(]", title)[0])
    if head & GRAIN and key in ENRICHED and "unenriched" not in t:
        return False
    if "milk" in head and key in {"vitamin_d_ng", "vitamin_a_rae_ug"}:
        return False
    return True


def matches(foods, i, index, from_srcs):
    """The best match of food `i` in each other table of `from_srcs`, best first: [(j, score)]."""
    me = shape(i, foods)
    seen, best = set(), {}
    for w in me["words"]:
        for u in index.get(w, []):
            src = foods[u["i"]][2]
            if u["i"] in seen or src == foods[i][2] or src not in from_srcs:
                continue
            seen.add(u["i"])
            s = likeness(me, u)
            if s > MATCH_AT and s > best.get(src, (None, 0))[1]:
                best[src] = (u["i"], s)
    return sorted(best.values(), key=lambda x: -x[1])


def borrow(foods):
    """For every European food, each value it lacks, from the closest food that has it (see above).
    `foods` is the list of rows the readers yield; returns {food index: {key: (value, origin index)}}."""
    keys = [key for key, *_ in NUTRIENTS]
    shapes = [shape(i, foods) for i in range(len(foods))]
    index = {}
    for u in shapes:
        for w in u["words"]:
            index.setdefault(w, []).append(u)
    europe = [i for i, f in enumerate(foods) if f[2] != "usda"]
    out = {}
    # 1. USDA lends.
    for i in europe:
        for j, _ in matches(foods, i, index, {"usda"})[:1]:
            got = {k: (foods[j][4][k], j) for k in keys
                   if k not in foods[i][4] and k in foods[j][4] and lends(foods[j][1], k)}
            if got:
                out[i] = got
    # 2. The European tables lend each other what they hold, with its origin.
    held = [{**{k: (v, i) for k, v in f[4].items()}, **out.get(i, {})} for i, f in enumerate(foods)]
    tables = {f[2] for f in foods} - {"usda"}
    for i in europe:
        ranked = matches(foods, i, index, tables)
        for k in keys:
            if k in foods[i][4] or k in out.get(i, {}):
                continue
            for j, _ in ranked:
                # A European food named enriched or fortified (some breakfast cereals) lends what it
                # was given no more than an American one.
                if k in held[j] and not FORTIFIED.search(norm(foods[held[j][k][1]][1])):
                    out.setdefault(i, {})[k] = held[j][k]
                    break
    return out


# ── one food, one set of values ──────────────────────────────────────────────────────────────────
# `decisions.md#data`, 2026-10-29 (later that night). The same food in several tables was several
# foods with different values: macadamia nuts were BLS's, Ciqual's, CREA's and USDA's, with selenium
# 12, 12, 4 and 33 µg, and which one a person got depended on the row they tapped. Now the same food
# is **one group**, and every member holds the group's values: each nutrient from the first table, in
# ORDER, that measured it — one real measurement, never a computed one (the owner's choice, "one table
# first"); where none measured it, what was lent to the first that has a lent value.
#
# A group is strict, because a wrong group gives a food another food's values: same kind, same
# state (raw or cooked as well as the states `likeness` keeps apart), the same numbers in the name
# (wheat flour type 550 is not type 812), the same organ or part (a mutton heart is not mutton meat),
# names one inside the other (the same words for a dish), protein, fat, carbohydrate and water within
# a quarter; at most one food per table, and every member strictly alike every other.
ORDER = ["bls", "swiss", "ciqual", "crea", "usda"]
PARTS = set("""heart liver kidney tongue brain tripe lung stomach gizzard marrow sweetbread yolk white
stalk leaf leaves root seed bran germ skin peel""".split())


def same_food(a, b, foods):
    s = likeness(a, b)
    if s < 0.75 or a["raw"] != b["raw"] or a["cooked"] != b["cooked"]:
        return False
    num = lambda i: set(re.findall(r"\d+(?:[.,]\d+)?", foods[i][1]))
    if num(a["i"]) != num(b["i"]):
        return False
    low = lambda i: set(re.findall(r"[a-z]+", norm(foods[i][1])))
    if (low(a["i"]) ^ low(b["i"])) & PARTS:
        return False
    na, nb = a["words"], b["words"]
    if a["dish"] or b["dish"]:
        if na != nb:
            return False
    elif not (na <= nb or nb <= na):
        return False
    d = [abs(a["v"][k] - b["v"][k]) / (max(a["v"][k], b["v"][k]) + sc)
         for k, sc in MACROS.items() if k in a["v"] and k in b["v"]]
    return len(d) >= 4 and max(d) <= 0.25


def groups(foods):
    """Groups of the same food across tables, as lists of food indexes."""
    shapes = [shape(i, foods) for i in range(len(foods))]
    index = {}
    for u in shapes:
        for w in u["words"]:
            index.setdefault(w, []).append(u)
    pairs = []
    for a in shapes:
        seen = set()
        for w in a["words"]:
            for b in index.get(w, []):
                if b["i"] <= a["i"] or b["i"] in seen or foods[b["i"]][2] == foods[a["i"]][2]:
                    continue
                seen.add(b["i"])
                if same_food(a, b, foods):
                    pairs.append((likeness(a, b), a["i"], b["i"]))
    pairs.sort(reverse=True)
    root = list(range(len(foods)))
    members = {i: [i] for i in range(len(foods))}
    def find(i):
        while root[i] != i:
            root[i] = root[root[i]]
            i = root[i]
        return i
    for _, a, b in pairs:
        ra, rb = find(a), find(b)
        if ra == rb:
            continue
        if {foods[m][2] for m in members[ra]} & {foods[m][2] for m in members[rb]}:
            continue
        if not all(same_food(shapes[x], shapes[y], foods) for x in members[ra] for y in members[rb]):
            continue
        root[rb] = ra
        members[ra] += members.pop(rb)
    return [sorted(m, key=lambda i: ORDER.index(foods[i][2])) for m in members.values() if len(m) > 1]


# ── estimates, only where they were tested ───────────────────────────────────────────────────────
# Same night. Where no table measured a nutrient for a food, nor for a food close enough to lend
# it, the value may be **estimated** from the most similar foods of its kind that have it: the five
# nearest by protein, fat, carbohydrate, water and fibre, their nutrient per gram of protein (for a
# nutrient that goes with protein) or per kilocalorie, times the food's own. **Only for a nutrient
# and a kind of food where the method was tested and held**: at every build, measured values are
# hidden and estimated from other foods — never from a food of the same name — and a pair is used
# only when the typical error is within 20% and three in five estimates are within 30%. On the
# 2026-10-29 sources 35 pairs held, most of them the absence of vitamin B12, vitamin D and
# cholesterol in plant foods; choline only for meat. Everything else stays a gap, said per entry.
# A zero is given only to plant foods (vegetables, fruit, legumes, nuts, drinks) — never to
# mushrooms (vitamin D), algae or yeast (B12), anything fortified, or a plant drink.
KINDS = [
    ("egg", "egg eggs"),
    ("offal", "liver kidney heart tongue brain tripe lung gizzard sweetbread offal"),
    ("fish", """fish salmon tuna cod trout herring mackerel sardine anchovy halibut plaice sole carp
     pike perch hake haddock pollock saithe whiting bream bass turbot eel catfish tilapia swordfish
     monkfish anglerfish ling redfish mullet sprat char whitefish zander shrimp prawn crab lobster
     mussel oyster clam scallop squid octopus cuttlefish crayfish caviar roe seafood shellfish
     sturgeon"""),
    ("cheese", """cheese cheddar gouda emmental emmentaler brie camembert mozzarella parmesan
     parmigiano grana ricotta feta gorgonzola pecorino fontina gruyere comte edam tilsit limburger
     mascarpone provolone taleggio asiago raclette appenzeller stracchino caciotta caciocavallo"""),
    ("dairy", "milk yogurt kefir quark whey buttermilk cream curd skyr"),
    ("fat", "oil butter margarine lard ghee dripping suet"),
    ("nut", """nut nuts almond hazelnut walnut cashew pistachio pecan macadamia peanut seed seeds
     sesame sunflower chia flax linseed coconut pine"""),
    ("legume", "bean beans lentil lentils chickpea chickpeas soy soya tofu lupin pea peas"),
    ("meat", """beef veal pork lamb mutton goat venison deer boar rabbit hare horse ham bacon sausage
     salami chicken turkey duck goose poultry meat steak mince meatball mortadella speck chorizo"""),
    ("grain", """flour bread roll pasta rice oat oats wheat rye barley corn maize millet quinoa cereal
     muesli cracker crackers biscuit cake pastry croissant noodle noodles spaghetti couscous semolina
     bran germ polenta buckwheat spelt"""),
    ("sweet", "sugar chocolate candy honey jam marmalade sweet sweets dessert pudding ice cookie syrup praline"),
    ("drink", "juice tea coffee soda water beer wine spirit liqueur lemonade cola drink nectar smoothie"),
    ("fruit", """apple pear peach apricot plum cherry grape banana orange lemon lime mandarin grapefruit
     strawberry raspberry blueberry blackberry currant kiwi mango papaya pineapple melon watermelon
     fig date fruit berry berries pomegranate quince persimmon lychee passion avocado olive"""),
    ("veg", """vegetable vegetables carrot potato tomato onion garlic leek cabbage broccoli cauliflower
     spinach lettuce salad cucumber zucchini pepper eggplant pumpkin squash celery fennel asparagus
     artichoke beet beetroot radish turnip kale chard mushroom mushrooms sprouts endive chicory rocket
     cress okra parsnip"""),
]
KIND_WORDS = [(k, set(w.split())) for k, w in KINDS]
# Nutrients that go with protein, so scaled by it; the rest by energy.
WITH_PROTEIN = {"choline_ug", "selenium_ug", "vitamin_b12_ng", "zinc_ug", "iron_ug",
                "phosphorus_ug", "niacin_ug", "vitamin_b6_ug", "pantothenic_acid_ug", "iodine_ug",
                "copper_ug", "riboflavin_ug", "thiamin_ug", "cholesterol_ug"}
NOT_ESTIMATED = {"kcal", "protein_mg", "fat_mg", "carbs_mg", "water_mg", "fiber_mg", "sugars_mg",
                 "saturated_fat_mg", "alcohol_mg", "caffeine_ug", "sodium_ug", "total_carbs_mg"}
NEVER_ZERO = re.compile(r"\b(mushrooms?|algae|seaweed|spirulina|nori|yeast|fortified|enriched|"
                        r"(soy|soya|oat|almond|rice|coconut|plant)[- ](drink|milk|beverage))\b")
MACRO_FEATURES = ["protein_mg", "fat_mg", "carbs_mg", "water_mg", "fiber_mg"]
PLANT = {"veg", "fruit", "legume", "nut", "drink"}


def kind_of(f):
    t = set(re.findall(r"[a-z]+", norm(f[1] + " " + (f[3] or ""))))
    if t & DISH:
        return "dish"
    for k, ws in KIND_WORDS:
        if t & ws:
            return k
    return "other"


def estimate(foods, i, key, donors):
    """The estimate for food `i` from `donors`, and how many it rests on; None with fewer than 3."""
    v = foods[i][4]
    x = [v.get(k, 0) / 1000 for k in MACRO_FEATURES]
    near = sorted(donors, key=lambda j: sum((a - foods[j][4].get(k, 0) / 1000) ** 2
                                            for a, k in zip(x, MACRO_FEATURES)))[:5]
    if len(near) < 3:
        return None
    base = (lambda w: max(w.get("protein_mg", 0), 1)) if key in WITH_PROTEIN else \
        (lambda w: max(w.get("kcal", 0), 1))
    ratios = sorted(foods[j][4][key] / base(foods[j][4]) for j in near)
    return round(ratios[len(ratios) // 2] * base(v)), len(near)


def tested(foods, kinds, heads):
    """The (nutrient, kind) pairs the method holds for: hide, estimate, compare (see above)."""
    rng = random.Random(7)
    keys = [k for k, *_ in NUTRIENTS if k not in NOT_ESTIMATED]
    pools = {}
    for i, f in enumerate(foods):
        for k in keys:
            if k in f[4]:
                pools.setdefault((k, kinds[i]), []).append(i)
    held = {}
    for (k, kind), pool in pools.items():
        if kind == "other" or len(pool) < 20:
            continue
        errs = []
        for i in rng.sample(pool, min(60, len(pool))):
            got = estimate(foods, i, k, [j for j in pool if j != i and not heads[j] & heads[i]])
            if got is None:
                continue
            t = foods[i][4][k]
            errs.append(abs(got[0] - t) / t if t > 0 else (0.0 if got[0] == 0 else 1.0))
        if len(errs) < 20:
            continue
        errs.sort()
        typical = errs[len(errs) // 2]
        if typical <= 0.2 and sum(e <= 0.3 for e in errs) >= 0.6 * len(errs):
            held[(k, kind)] = typical
    return held


# Foods whose matches are printed at every build, for a person to look at.
REVIEW = ["egg,", "milk", "yogurt", "butter", "cheese", "chicken", "beef", "pork", "salmon", "tuna",
          "potato", "rice", "bread", "apple", "banana", "spinach", "broccoli", "lentil", "almond",
          "oil"]


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
    foods = [row for source in (read_ciqual, read_bls, read_swiss, read_crea, read_usda)
             for row in source(cache)]
    lent = borrow(foods)
    # Every food's values and where each came from: its own (origin itself), then borrowed.
    held = [{**{k: (v, i) for k, v in f[4].items()}, **lent.get(i, {})} for i, f in enumerate(foods)]
    # One food, one set of values: each nutrient from the first table that measured it, else lent.
    same = groups(foods)
    first_of = {}
    for g in same:
        one = {}
        for k in keys:
            mine = [m for m in g if k in foods[m][4]]
            if mine:
                one[k] = (foods[mine[0]][4][k], mine[0])
            elif any(k in held[m] for m in g):
                one[k] = held[next(m for m in g if k in held[m])][k]
        for m in g:
            held[m] = dict(one)
            first_of[m] = g[0]
    # What was lent from a member of a group carries the group's value: roasted macadamia nuts
    # lent selenium by USDA's "Macadamia nuts" hold the group's, not USDA's own.
    canon = {m: held[m] for g in same for m in g}
    for i in range(len(foods)):
        if i in canon:
            continue
        for k, (v, frm) in list(held[i].items()):
            if frm is not None and frm != i and frm in canon and k in canon[frm]:
                held[i][k] = canon[frm][k]
    print(f"  one food, one set of values: {len(same)} groups of {sum(len(g) for g in same)} foods")
    # Estimates, only where the method was tested and held at this build.
    kinds = [kind_of(f) for f in foods]
    heads = [name_words(re.split(r"[,(]", f[1])[0]) for f in foods]
    held_pairs = tested(foods, kinds, heads)
    print(f"  estimates hold for {len(held_pairs)} nutrient and kind pairs:")
    for (k, kind), typical in sorted(held_pairs.items()):
        print(f"    {k:20} {kind:7} typically {round(100 * typical)}% off")
    donors = {}
    for i, f in enumerate(foods):
        for k in f[4]:
            if (k, kinds[i]) in held_pairs:
                donors.setdefault((k, kinds[i]), []).append(i)
    guessed = {}
    for i, f in enumerate(foods):
        for (k, kind) in held_pairs:
            if kind != kinds[i] or k in held[i]:
                continue
            got = estimate(foods, i, k, [j for j in donors[(k, kind)] if not heads[j] & heads[i]])
            # A zero only for plant foods: a test on typical error cannot see the cakes made with
            # eggs and butter among the grains, or the butter among the fats.
            if got is None or (got[0] == 0 and (kind not in PLANT or NEVER_ZERO.search(norm(f[1])))):
                continue
            held[i][k] = (got[0], None)
            guessed.setdefault(i, {})[k] = f"{got[1]} similar foods ({kind})"
    print(f"  estimated: {sum(len(v) for v in guessed.values())} values for {len(guessed)} foods")
    counts = {}
    for at, (code, title, src, grp, values, portions, alt) in enumerate(foods):
        fid = at + 1
        db.execute("INSERT INTO food(id, code, name, src, grp, alt) VALUES (?, ?, ?, ?, ?, ?)",
                   (fid, code, title, src, grp, alt))
        db.executemany("INSERT INTO amount VALUES (?, ?, ?)",
                       [(fid, keys[k], v) for k, (v, _) in held[at].items()])
        db.executemany("INSERT INTO borrowed VALUES (?, ?, ?)",
                       [(fid, keys[k], frm + 1) for k, (_, frm) in held[at].items()
                        if frm is not None and frm != at])
        db.executemany("INSERT INTO estimated VALUES (?, ?, ?)",
                       [(fid, keys[k], basis) for k, basis in guessed.get(at, {}).items()])
        if at in first_of:
            db.execute("INSERT INTO same VALUES (?, ?)", (fid, first_of[at] + 1))
        db.executemany("INSERT INTO portion VALUES (?, ?, ?, ?)",
                       [(fid, i, label, grams) for i, (label, grams) in enumerate(portions)])
        counts[src] = counts.get(src, 0) + 1
    print(f"  borrowed: {sum(len(v) for v in lent.values())} values for {len(lent)} foods")
    for word in REVIEW:
        shown = 0
        for at, got in lent.items():
            if word in foods[at][1].lower() and shown < 2:
                frm = next(iter(got.values()))[1]
                print(f"    {foods[at][2]:7} {foods[at][1][:44]:44} <- {foods[frm][1][:44]}"
                      f"  [{', '.join(sorted(got))[:60]}]")
                shown += 1
    db.execute("INSERT INTO names(rowid, name, alt) SELECT id, name, alt FROM food")
    meta = {
        "name": NAME,
        "version": VERSION,
        "foods": str(sum(counts.values())),
        "title": ("Foods: Ciqual 2025, BLS 4.0, the Swiss Food Composition Database 7.1 and "
                  "CREA's tables (2019), with USDA FoodData Central as a fallback"),
        "licence": ("Ciqual: Etalab Open Licence 2.0. BLS: CC BY 4.0. Swiss Food Composition "
                    "Database: free to use, commercially too, with the source acknowledged. "
                    "CREA: may be copied with the source named. "
                    "USDA: U.S. Government work"),
        "attribution": (
            "Anses (2025), Ciqual French food composition table, doi:10.57745/RDMHWY. "
            "Max Rubner-Institut (2025), Bundeslebensmittelschlüssel (BLS), Version 4.0, "
            "doi:10.25826/Data20251217-134202-0. "
            "Federal Food Safety and Veterinary Office (2026), Swiss Food Composition Database, "
            "version 7.1, naehrwertdaten.ch. "
            "CREA Centro di ricerca Alimenti e Nutrizione, Tabelle di composizione degli "
            "alimenti, Aggiornamento 2019, www.alimentinutrizione.it. "
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
