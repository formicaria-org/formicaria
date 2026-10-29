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
import re
import sqlite3
import sys
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

# The list's own version: bump it whenever an input, the constituents or the layout changes.
# An entry records `<name>@<version>/<food>` as where its numbers came from.
NAME = "foods"
VERSION = "2026.10.4"

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


# ── borrowing what a table never measures ────────────────────────────────────────────────────────
# `decisions.md#data`, 2026-10-29. Choline is in USDA's table alone, CREA measures no vitamin B12, D,
# A, E, K1, B6 or iodine, the Swiss table no K1, B6, folate, copper or manganese, BLS no selenium —
# so an egg from a European table had no choline at all, and an Italian one none of half the vitamins.
# Where a food's own table carries a nutrient for fewer than BORROW_BELOW of its foods (a gap in the
# table, not one food's missing analysis), the value is taken from USDA's closest food: one that shares
# a word of its name and agrees on the macronutrients, above MATCH_AT. A measured value is never
# replaced, nothing is averaged, and where it came from is kept (`borrowed`).
BORROW_BELOW = 0.05
MATCH_AT = 0.62
# Words that say how a food was handled or sold, not what it is: shared by too many foods to match on.
NAME_NOISE = set("""raw cooked boiled fried baked roasted grilled steamed stewed fresh frozen canned
dried whole average plain with without and or in of the a to for from not ns nfs as on prepared
added salt unsalted salted drained solids liquid solid mixed type made commercial home homemade
regular all purpose including excluding skin skinless boneless bone meat only lean fat""".split())
# Each macronutrient's own scale, so a difference counts as much as it matters: 20 kcal, 1 g.
MACROS = {"kcal": 20, "protein_mg": 1000, "fat_mg": 1000, "carbs_mg": 1000, "water_mg": 5000}


def name_words(title):
    out = set()
    for w in re.findall(r"[a-z]+", title.lower()):
        w = w[:-1] if len(w) > 3 and w.endswith("s") else w
        if len(w) > 2 and w not in NAME_NOISE:
            out.add(w)
    return out


# A dish of several foods (a sandwich, a salad, "with …") shares words with many others; it must
# share more of them to be the same dish.
DISH = set("with sandwich salad pizza burger quiche pie soup dish stew sauce casserole".split())
STATE = set("raw cooked boiled fried baked roasted grilled steamed stewed braised".split())


def likeness(a, b):
    """How alike two foods are, 0–1: half their names' shared words, half their macronutrients —
    or 0 when they are not alike enough to lend a value: too few shared words (half for a dish),
    fewer than three macronutrients to compare, any one of them far apart (fried rice's fat is four
    times wholegrain rice's), or the macronutrients apart on average."""
    na, nb = a["words"], b["words"]
    if not na or not nb or not (na & nb):
        return 0.0
    # The food itself comes first in these names ("Egg, raw", "Tomato sauce, with cheese"): the
    # shorter head must be inside the other — a chicken egg is an egg, a tomato sauce is no cheese
    # sauce.
    ha, hb = sorted((a["head"], b["head"]), key=len)
    if not ha or not ha <= hb:
        return 0.0
    # Fried and not fried are different foods for what is borrowed: the fat and what was fried in it.
    if ("fried" in a["state"]) != ("fried" in b["state"]):
        return 0.0
    names = len(na & nb) / len(na | nb)
    if names < (0.5 if (a["dish"] or b["dish"]) else 0.34):
        return 0.0
    diffs = [abs(a["v"][k] - b["v"][k]) / (max(a["v"][k], b["v"][k]) + scale)
             for k, scale in MACROS.items() if k in a["v"] and k in b["v"]]
    if len(diffs) < 3 or max(diffs) > 0.5:
        return 0.0
    macros = 1 - sum(diffs) / len(diffs)
    if macros < 0.75:
        return 0.0
    # Raw against cooked: the same food, but water and heat move the values; a match in the same
    # state is preferred.
    # Only when both names say: a name that states none ("Chicken Egg, whole") is not a cooked one.
    state = 0.1 if (a["state"] and b["state"] and ("raw" in a["state"]) != ("raw" in b["state"])) else 0.0
    return 0.5 * names + 0.5 * macros - state


def borrow(foods):
    """For every European food, the values its table never measures, from USDA's closest food.
    `foods` is the list of rows the readers yield; returns {food index: {key: (value, from index)}}."""
    by_src = {}
    for i, f in enumerate(foods):
        by_src.setdefault(f[2], []).append(i)
    def shape(i):
        low = set(re.findall(r"[a-z]+", foods[i][1].lower()))
        return {"i": i, "words": name_words(foods[i][1]), "v": foods[i][4],
                "head": name_words(re.split(r"[,(]", foods[i][1])[0]),
                "dish": bool(low & DISH), "state": low & STATE}
    usda = [shape(i) for i in by_src.get("usda", [])]
    index = {}
    for u in usda:
        for w in u["words"]:
            index.setdefault(w, []).append(u)
    out = {}
    for src, idx in by_src.items():
        if src == "usda":
            continue
        gaps = [key for key, *_ in NUTRIENTS
                if sum(key in foods[i][4] for i in idx) < BORROW_BELOW * len(idx)]
        if not gaps:
            continue
        for i in idx:
            title, values = foods[i][1], foods[i][4]
            me = shape(i)
            seen, best, best_score = set(), None, MATCH_AT
            for w in me["words"]:
                for u in index.get(w, []):
                    if u["i"] in seen:
                        continue
                    seen.add(u["i"])
                    s = likeness(me, u)
                    if s > best_score:
                        best, best_score = u, s
            if best is None:
                continue
            got = {k: (foods[best["i"]][4][k], best["i"]) for k in gaps
                   if k not in values and k in foods[best["i"]][4]}
            if got:
                out[i] = got
    return out


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
    counts = {}
    for at, (code, title, src, grp, values, portions, alt) in enumerate(foods):
        fid = at + 1
        db.execute("INSERT INTO food(id, code, name, src, grp, alt) VALUES (?, ?, ?, ?, ?, ?)",
                   (fid, code, title, src, grp, alt))
        mine = dict(values)
        for k, (v, _) in lent.get(at, {}).items():
            mine[k] = v
        db.executemany("INSERT INTO amount VALUES (?, ?, ?)",
                       [(fid, keys[k], v) for k, v in mine.items()])
        db.executemany("INSERT INTO borrowed VALUES (?, ?, ?)",
                       [(fid, keys[k], frm + 1) for k, (_, frm) in lent.get(at, {}).items()])
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
