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
VERSION = "2026.10.3"

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
    "vitamin_d_ug": "Vitamin D (calciferol) (µg)",         # VITD
    "vitamin_e_ug": "Vitamin E (α-tocopherol) (mg)",       # TOCPHA
    "thiamin_ug": "Vitamin B1 (thiamine) (mg)",            # THIA
    "riboflavin_ug": "Vitamin B2 (riboflavin) (mg)",       # RIBF
    "niacin_ug": "Niacin (mg)",                            # NIA
    "pantothenic_acid_ug": "Pantothenic acid (mg)",        # PANTAC
    "vitamin_b12_ug": "Vitamin B12 (cobalamin) (µg)",      # VITB12
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
# equivalents), folate (total), vitamin B6 (not said).
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
    "Vitamina D (μg)": ("vitamin_d_ug", "ug"),
    "Tiamina (mg)": ("thiamin_ug", "mg"),
    "Riboflavina (mg)": ("riboflavin_ug", "mg"),
    "Niacina (mg)": ("niacin_ug", "mg"),
}
# **Carbohydrate.** CREA's *Carboidrati disponibili* is starch × 1.10 plus sugars, in
# monosaccharide equivalents (CHOAVLM), which is not what `carbs_mg` holds (CHOAVL, by weight).
# Its *Amido* is starch as a polysaccharide — by weight — and its *Zuccheri solubili* are sugars
# as monosaccharides, at most 5 % above their weight (a disaccharide yields 105 g of
# monosaccharides per 100 g). So `carbs_mg` is starch plus soluble sugars, as printed: exact for
# the starch, and for the sugars high by up to a twentieth of the sugars. A food without both
# lines has no carbohydrate here, unless its available carbohydrate is printed as 0.
CREA_CARBS = ("Amido (g)", "Zuccheri solubili (g)")
# **Saturated fat.** *"Il contenuto in acidi grassi è riportato in % di lipidi"*: the saturated
# fatty acids are a percentage of the food's fat, so the grams are fat × that ÷ 100.
CREA_SATURATED = "Acidi grassi Saturi (%)"


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
        if CREA_SATURATED in got and "Lipidi (g)" in got:
            v["saturated_fat_mg"] = round(got["Lipidi (g)"] * got[CREA_SATURATED] / 100 * 1000)
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
    for source in (read_ciqual, read_bls, read_swiss, read_crea, read_usda):
        for code, title, src, grp, values, portions, alt in source(cache):
            fid += 1
            db.execute("INSERT INTO food(id, code, name, src, grp, alt) VALUES (?, ?, ?, ?, ?, ?)",
                       (fid, code, title, src, grp, alt))
            db.executemany("INSERT INTO amount VALUES (?, ?, ?)",
                           [(fid, keys[k], v) for k, v in values.items()])
            db.executemany("INSERT INTO portion VALUES (?, ?, ?, ?)",
                           [(fid, i, label, grams) for i, (label, grams) in enumerate(portions)])
            counts[src] = counts.get(src, 0) + 1
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
