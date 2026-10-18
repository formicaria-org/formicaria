#!/usr/bin/env python3
"""Fetch the Italian recipes of Italian Wikibooks' "Libro di cucina", word for word.

    python3 ci/pack-wikibooks.py crates/fm-app/src/shipped/recipes-wikibooks.json

**What it is.** A maintainer runs this by hand when the recipe file is wanted afresh. The app
never runs it, and the build reads only the JSON file it leaves behind — which is committed, so
building formicaria needs neither this script nor the network.

**Where the recipes are from.** Italian Wikibooks, the cookbook "Libro di cucina":

    https://it.wikibooks.org/wiki/Libro_di_cucina/Ricette

Every recipe is a page of its own named `Libro di cucina/Ricette/<Name>`. The script asks the
MediaWiki API (https://it.wikibooks.org/w/api.php) for every page with that prefix
(`list=allpages`, redirects left out), then for each page's wikitext (`prop=revisions`,
`rvslots=main`) and its categories (`prop=categories`), fifty titles to a request, one request
at a time with a pause before each and a User-Agent naming the project.

**Licence.** The site's own words, read 2026-10-18 in the footer of every page (for instance
https://it.wikibooks.org/wiki/Libro_di_cucina/Ricette/Amatriciana): "Il testo è disponibile
secondo la licenza Creative Commons Attribuzione-Condividi allo stesso modo; possono applicarsi
condizioni ulteriori." The licence link is https://creativecommons.org/licenses/by-sa/4.0/deed.it,
and the API's `meta=siteinfo&siprop=rightsinfo` names it "Creative Commons Attribution-Share
Alike 4.0". CC BY-SA 4.0 asks for three things, and the file carries each: who made it (every
recipe's `authors_url` is its page's history, which lists the authors), where it is from (its
`url`), and the licence (`licence`, `licence_url` at the top). Share-alike binds this file:
whoever takes the recipes from it takes them under CC BY-SA 4.0 too — the code that reads it is
not an adaptation of the text and stays MIT.

**Which recipes: the Italian ones, by the page's own categories.** The cookbook sorts recipes by
where they are from under `Categoria:Ricette per provenienza`, which has two branches:
`Categoria:Ricette regionali` — whose subcategories are the twenty Italian regions
("Ricette regionali-Lombardia", …), some with towns below them ("Ricette di Monte Sant'Angelo"
under Puglia) — and `Categoria:Ricette dal mondo` (French, Japanese, …). The script reads both
trees from the API when it runs. A page is kept when one of its own categories is in the
Italian-regions tree; its `region` is the region that category belongs to, when there is exactly
one. A page whose categories name only the rest of the world is rejected, and so is a page that
names no origin at all — many Italian-sounding pages say nothing about where they are from, and
the title is not read to guess. (The templates do not help here: the cookbook's recipe box,
`{{Infobox ricetta}}`, has no country or region field — Nome, Immagine, Categoria, Porzioni,
Tempo, Difficoltà, Calorie — and only sixteen pages use it.)

**How a page is read.** Line by line, top to bottom; each heading, at whatever level, says what
the lines after it are, by its own words:

  - "Ingredienti…" — each list line (`*`, `#`, `:`, `;`) is an ingredient line. A line of prose
    there, or a sub-heading (`=== Per il ripieno ===`), is a line of its own, where the page has
    it — except the section's first line of prose when it is short, opens with "per", "dosi" or
    "porzioni" and says how many it serves ("Per '''4''' persone", "(Dosi per 4 persone)"),
    which is the `yield`. A heading "Ingredienti per 4 persone" gives the `yield` "per 4
    persone".
  - "Preparazione", "Procedimento", "Cottura", "Ricetta", "Impiattare", "Presentazione" (and
    headings containing them) — each list line is a step, and so is each paragraph of prose.
  - "Ingredienti e preparazione" — `*` lines are ingredients, `#` lines and prose are steps.
  - "Consigli", "Note", "Varianti", "Suggerimenti", "Curiosità", "Conservazione", "Utilizzo",
    "Impiego", "Accompagnamento" — each list line or paragraph is a note.
  - "Altri progetti", "Voci correlate", "Ricette correlate", "Pagine correlate", "Vedi anche",
    "Collegamenti esterni", "Bibliografia", "Fonte", "Galleria…" — links away; not taken.
  - A heading deeper than the heading above it that these words do not name, or that names
    the same part ("=== Per la pasta ===" under "== Ingredienti ==", "=== Preparazione
    alternativa ===" under "== Preparazione =="), is a line of its own in that part. A heading
    with an ingredients heading below it ("== Prima ricetta ==", "== Seconda ricetta ==") wraps
    one recipe of several on the page: where a page has two or more, each one's name is a line
    of its own heading its ingredients and its steps.
  - Anything else — the opening paragraphs before the first heading, "Storia", "Attrezzatura",
    a section for one part of the dish — is carried as `text`, paragraph by paragraph, a heading
    being a paragraph of its own, so nothing the recipe says is dropped by not being recognised.

  A second ingredients or steps part on a page is headed by its own heading, a line of its own
  ("Preparazione variante Ninochef"); a notes part is too, unless its heading is only the word
  "Consigli", "Note", "Varianti" or the like.

  A footnote (`<ref>…</ref>`) is taken out of its line; if it says something (not only a
  citation — a `{{cita …}}` template or a bare link), what it says is carried as a note.

**What was changed on the way: markup and whitespace only.** The words are the page's own —
nothing is reworded, reordered, corrected or translated, and typing slips are kept.

  - Links: `[[target|label]]` becomes its label; `[[target]]` its target, less an interwiki
    prefix (`w:`) and the cookbook path (`Libro di cucina/Ricette/`); an anchor (`#…`) goes.
    `[url label]` becomes its label, a bare `[url]` goes, a plain URL stays. Images
    (`[[File:…]]`, `[[Immagine:…]]`), categories and links to other languages go.
  - `'''bold'''` and `''italic''` lose their quotes; HTML tags lose their brackets (their text
    stays; `<br>` is a space); `<gallery>`, comments and tables (`{| … |}`) go; entities are
    decoded.
  - Templates that only print their argument become it: `{{I|x}}` (a link to a recipe) and
    `{{unicode|x}}`; `{{PAGENAME}}`/`{{SUBPAGENAME}}` become the page's
    name. Templates that print only furniture are removed: `{{libro di cucina}}` (the book's
    navigation box), `{{Avanzamento}}` (progress mark), `{{interprogetto}}`, `{{vedi anche}}`,
    `{{categorie della ricetta}}` (it only files the page), `{{Infobox ricetta}}` (read first —
    below), `{{DEFAULTSORT}}`, `{{clear}}`, maintenance banners (`{{da unire}}`,
    `{{da controllare}}`, `{{W}}`, `{{ldcok}}`, `{{citazione necessaria}}`, `{{chiarire}}`,
    `{{...}}` — the "incomplete" mark), `{{references}}` and the citation templates. Any other template is
    removed too and counted by name in the summary line, so a new one is seen.
  - Behaviour switches (`__NOTOC__`) go.
  - Runs of whitespace become one space, and the ends are trimmed. An empty line is not kept.

**Nothing is invented.** A field the page does not have is not in the file.

  - `servings` is there only when the recipe box's "Porzioni" is a whole number ("4"), or —
    failing that — when the yield line is exactly a whole number of people or portions ("Per
    4 persone", "6 porzioni"; not "per 4/6 persone", not "per 24 cannoli").
  - `yield` is the recipe box's "Porzioni" verbatim when that is not a whole number, else the
    page's yield line verbatim (markup gone).
  - `times` holds the recipe box's own time field verbatim, by its own name: `{"tempo": "40
    minuti"}`. The box has no other time field.
  - `image` is the recipe box's "Immagine" file name, only when the API says the file is on
    Wikimedia Commons; it is a name to link to — no image is fetched or carried.
  - A page with no ingredient line and no step (a stub) is left out and counted.

The file is written sorted by id, so the same pages make the same file apart from `seen`, the
day this was run. Fewer than 150 Italian recipes is treated as the site having changed under the
script: nothing is written and the exit status is 1.

Standard library only.
"""
import collections
import html
import json
import re
import sys
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request

SOURCE = "Wikibooks, Libro di cucina — Ricette (it.wikibooks.org)"
LICENCE = "CC BY-SA 4.0"
LICENCE_URL = "https://creativecommons.org/licenses/by-sa/4.0/"
SITE = "https://it.wikibooks.org"
API = SITE + "/w/api.php"
PREFIX = "Libro di cucina/Ricette/"
HOME = SITE + "/wiki/" + urllib.parse.quote("Libro_di_cucina/Ricette")
ITALY = "Categoria:Ricette regionali"
WORLD = "Categoria:Ricette dal mondo"

USER_AGENT = "formicaria-pack/1.0 (https://github.com/formicaria-org/formicaria)"
PAUSE_S = 1.0  # between any two requests
RETRY_PAUSE_S = 5.0
BATCH = 50  # titles to a request: the API's limit
MIN_RECIPES = 150

# Templates that print their first argument, and those that print nothing worth carrying.
PRINTS_ARGUMENT = {"i", "unicode"}
FURNITURE = {
    "libro di cucina", "avanzamento", "interprogetto", "vedi anche", "categorie della ricetta",
    "infobox ricetta", "defaultsort", "clear", "da unire", "da controllare", "w", "ldcok",
    "citazione necessaria", "chiarire", "...", "references", "cita web", "cita libro", "cita news",
    "cita pubblicazione",
}

STEP_HEADINGS = re.compile(r"preparazion|procediment|cottura|ricetta|impiattare|presentazione")
NOTE_HEADINGS = re.compile(
    r"^(consigl|note\b|variant|ulteriore variante|suggeriment|curiosit|conservazione|utilizzo"
    r"|impiego|accompagnamento|errori)"
)
LINK_HEADINGS = re.compile(
    r"^(altri progetti|voci correlate|ricette correlate|pagine correlate|vedi anche"
    r"|collegamenti esterni|bibliografia|fonte|galleria)"
)
PLAIN_NOTES = {"consigli", "consiglio", "note", "varianti", "variante", "suggerimenti",
               "suggerimento", "curiosità"}
YIELD_START = re.compile(r"\(?\s*(per|dosi|ingredienti per|porzioni)\b", re.IGNORECASE)
YIELD_LINE = re.compile(r"\b(person[ae]|porzion[ei]|dos[ei])\b", re.IGNORECASE)
WHOLE_SERVINGS = re.compile(r"\(?(?:(?:per|dosi per)\s+)?(\d+)(?:\s+(?:persone|persona|porzioni))?\)?\.?:?",
                            re.IGNORECASE)

_last_request = [0.0]
unknown_templates = collections.Counter()


def api(params):
    """One API answer as JSON: one request at a time, a pause before each, one retry."""
    url = API + "?" + urllib.parse.urlencode(dict(params, format="json", formatversion="2"))
    for attempt in (1, 2):
        wait = PAUSE_S - (time.monotonic() - _last_request[0])
        if wait > 0:
            time.sleep(wait)
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return json.loads(response.read().decode("utf-8"))
        except (urllib.error.URLError, OSError, UnicodeDecodeError, ValueError) as error:
            if attempt == 2:
                raise RuntimeError(f"{url}: {error}") from error
            time.sleep(RETRY_PAUSE_S)
        finally:
            _last_request[0] = time.monotonic()


def continued(params):
    """Every answer of a query the API pages with `continue`."""
    extra = {}
    while True:
        answer = api(dict(params, **extra))
        yield answer
        if "continue" not in answer:
            return
        extra = answer["continue"]


def tidy(text):
    """Whitespace only: runs (non-breaking spaces included) to one space, ends trimmed."""
    return re.sub(r"\s+", " ", text).strip()


# --- what the site says ------------------------------------------------------------------------


def recipe_titles():
    titles = []
    for answer in continued({"action": "query", "list": "allpages", "apprefix": PREFIX,
                             "apnamespace": 0, "apfilterredir": "nonredirects",
                             "aplimit": 500}):
        titles += [page["title"] for page in answer["query"]["allpages"]]
    return titles


def subcategories(root):
    """{category: the root's direct subcategory it lies under} for the tree below `root`."""
    under, todo = {}, [(root, None)]
    while todo:
        category, top = todo.pop()
        for answer in continued({"action": "query", "list": "categorymembers",
                                 "cmtitle": category, "cmtype": "subcat", "cmlimit": 500}):
            for member in answer["query"]["categorymembers"]:
                name = member["title"]
                if name not in under and name != root:
                    under[name] = top or name
                    todo.append((name, top or name))
    return under


def pages(titles):
    """{title: (wikitext, [categories])}, fifty titles to a request."""
    found = {}
    for start in range(0, len(titles), BATCH):
        batch = "|".join(titles[start:start + BATCH])
        for answer in continued({"action": "query", "prop": "revisions", "rvprop": "content",
                                 "rvslots": "main", "titles": batch}):
            for page in answer["query"]["pages"]:
                if page.get("revisions"):
                    text = page["revisions"][0]["slots"]["main"]["content"]
                    found.setdefault(page["title"], [text, []])[0] = text
        for answer in continued({"action": "query", "prop": "categories", "cllimit": "max",
                                 "titles": batch}):
            for page in answer["query"]["pages"]:
                entry = found.setdefault(page["title"], [None, []])
                entry[1] += [c["title"] for c in page.get("categories", [])]
    return found


def on_commons(names):
    """The file names among `names` that the API says are kept on Wikimedia Commons."""
    shared, names = set(), sorted(names)
    for start in range(0, len(names), BATCH):
        titles = "|".join("File:" + name for name in names[start:start + BATCH])
        answer = api({"action": "query", "prop": "imageinfo", "iiprop": "url", "titles": titles})
        normal = {n["to"]: n["from"] for n in answer["query"].get("normalized", [])}
        for page in answer["query"]["pages"]:
            if page.get("imagerepository") == "shared":
                asked = normal.get(page["title"], page["title"])
                shared.add(asked.split(":", 1)[1])
    return shared


# --- wikitext to text --------------------------------------------------------------------------


def template_spans(text):
    """(start, end) of each outermost {{…}} in `text`."""
    spans, depth, start, i = [], 0, 0, 0
    while i < len(text) - 1:
        pair = text[i:i + 2]
        if pair == "{{":
            if depth == 0:
                start = i
            depth += 1
            i += 2
        elif pair == "}}" and depth:
            depth -= 1
            if depth == 0:
                spans.append((start, i + 2))
            i += 2
        else:
            i += 1
    return spans


def template_parts(body):
    """A template's name and arguments; a | inside [[…]] does not split."""
    parts, depth, current = [], 0, []
    i = 0
    while i < len(body):
        if body.startswith("[[", i):
            depth += 1
            current.append("[[")
            i += 2
        elif body.startswith("]]", i) and depth:
            depth -= 1
            current.append("]]")
            i += 2
        elif body[i] == "|" and not depth:
            parts.append("".join(current))
            current = []
            i += 1
        else:
            current.append(body[i])
            i += 1
    parts.append("".join(current))
    return parts[0].strip(), parts[1:]


def infobox(text):
    """The {{Infobox ricetta}} fields, verbatim apart from whitespace and markup."""
    for start, end in template_spans(text):
        name, args = template_parts(text[start + 2:end - 2])
        if name.lower() == "infobox ricetta":
            fields = {}
            for arg in args:
                if "=" in arg:
                    key, value = arg.split("=", 1)
                    fields[tidy(key).lower()] = value
            return fields
    return {}


def expand_templates(text, page_name):
    """Innermost first: argument-printing templates become their argument, the rest go."""
    pattern = re.compile(r"\{\{([^{}]*)\}\}")
    while True:
        match = pattern.search(text)
        if not match:
            return text
        name, args = template_parts(match.group(1))
        key = name.lower()
        if key in ("pagename", "subpagename"):
            value = page_name
        elif key in PRINTS_ARGUMENT and args:
            value = args[0]
        else:
            if key.split(":")[0] not in FURNITURE and key not in FURNITURE:
                unknown_templates[name] += 1
            value = ""
        text = text[:match.start()] + value + text[match.end():]


def link_text(match):
    target, _, label = match.group(1).partition("|")
    if label:
        return label
    target = target.strip()
    if target.startswith(":"):
        target = target[1:]
    target = re.sub(r"^(w|wikipedia|it|wikt|b):", "", target, flags=re.IGNORECASE)
    if target.startswith(PREFIX):
        target = target[len(PREFIX):]
    return target.split("#", 1)[0] or target


def inline(text, page_name):
    """One line or paragraph of wikitext as plain text."""
    text = expand_templates(text, page_name)
    # Images, categories and links to other languages: gone, captions and all.
    text = re.sub(r"\[\[\s*(?:File|Immagine|Image|Categoria|Category)\s*:[^\[\]]*"
                  r"(?:\[\[[^\[\]]*\]\][^\[\]]*)*\]\]", "", text, flags=re.IGNORECASE)
    text = re.sub(r"\[\[[a-z]{2,3}(?:-[a-z]+)?:[^\[\]|]*\]\]", "", text)
    text = re.sub(r"\[\[([^\[\]]*)\]\]", link_text, text)
    text = re.sub(r"\[(?:https?:)?//[^\s\]]+\s+([^\]]*)\]", r"\1", text)
    text = re.sub(r"\[(?:https?:)?//[^\s\]]+\]", "", text)
    text = re.sub(r"'{2,}", "", text)
    text = re.sub(r"__[A-Z]+__", "", text)
    text = re.sub(r"<br\s*/?>", " ", text, flags=re.IGNORECASE)
    text = re.sub(r"<[^<>]+>", "", text)
    return tidy(html.unescape(text))


# --- one page ----------------------------------------------------------------------------------


def blocks(body):
    """The page's lines as (kind, mark, raw): kind is "list" (mark: its marker), "prose" or
    "heading" (mark: its level); prose lines that follow each other make one paragraph, as
    MediaWiki prints them."""
    out, para = [], []

    def end_para():
        if para:
            out.append(("prose", "", "\n".join(para)))
            para.clear()

    for line in body.split("\n"):
        stripped = line.strip()
        heading = re.fullmatch(r"(={2,6})\s*(.*?)\s*\1", stripped)
        if not stripped:
            end_para()
        elif heading:
            end_para()
            out.append(("heading", len(heading.group(1)), heading.group(2)))
        elif stripped[0] in "*#:;":
            end_para()
            marker = re.match(r"[*#:;]+", stripped).group(0)
            rest = stripped[len(marker):]
            if marker.endswith(";") and ":" in rest:
                term, _, definition = rest.partition(":")
                out.append(("list", marker, term.strip() + ": " + definition.strip()))
            else:
                out.append(("list", marker, rest))
        else:
            para.append(stripped)
    end_para()
    return out


def classify(heading):
    name = heading.lower().strip(" :'")
    if name.startswith("ingredienti e preparazione"):
        return "mixed"
    if name.startswith("ingredienti"):
        return "ingredients"
    if LINK_HEADINGS.match(name):
        return "drop"
    if NOTE_HEADINGS.match(name):
        return "notes"
    if STEP_HEADINGS.search(name):
        return "steps"
    return "text"


def slug(name):
    folded = unicodedata.normalize("NFKD", name).encode("ascii", "ignore").decode("ascii")
    return re.sub(r"[^a-z0-9]+", "-", folded.lower()).strip("-")


def whole_servings(text):
    match = WHOLE_SERVINGS.fullmatch(tidy(text))
    return int(match.group(1)) if match and int(match.group(1)) > 0 else None


def recipe(title, text, region, commons):
    """The file's entry for one page, or None for a stub."""
    name = title[len(PREFIX):]
    box = infobox(text)

    # Take out what never prints: comments, galleries, tables, and the footnotes (kept aside).
    text = re.sub(r"<!--.*?-->", "", text, flags=re.S)
    text = re.sub(r"<gallery.*?</gallery>", "", text, flags=re.S | re.IGNORECASE)
    text = re.sub(r"<ref[^>/]*/>", "", text, flags=re.IGNORECASE)
    footnotes = []

    def keep_footnote(match):
        said = expand_templates(match.group(1), name)
        said = re.sub(r"\[(?:https?:)?//[^\]]*\]|(?:https?:)?//\S+", "", said)
        if inline(said, name).strip(" .,;:"):
            footnotes.append(inline(match.group(1), name))
        return ""

    text = re.sub(r"<ref[^>]*>(.*?)</ref>", keep_footnote, text, flags=re.S | re.IGNORECASE)
    text = re.sub(r"^\{\|.*?^\|\}", "", text, flags=re.S | re.M)
    # Templates spanning lines (the recipe box) go before the text is cut into lines.
    for start, end in reversed(template_spans(text)):
        if "\n" in text[start:end]:
            text = text[:start] + expand_templates(text[start:end], name) + text[end:]

    ingredients, steps, notes, paras = [], [], [], []
    yields = None
    lines = [(kind, mark, inline(raw, name), raw) for kind, mark, raw in blocks(text)]
    headings = [(i, mark, classify(line)) for i, (kind, mark, line, _) in enumerate(lines)
                if kind == "heading"]
    # A heading with an ingredients heading below it ("== Prima ricetta ==" over
    # "=== Ingredienti ===") wraps a recipe; where a page has two or more, each one's name
    # heads its part of the ingredients and of the steps.
    wrappers = set()
    for n, (i, level, _) in enumerate(headings):
        for _, deeper, kind in headings[n + 1:]:
            if deeper <= level:
                break
            if kind in ("ingredients", "mixed"):
                wrappers.add(i)
                break
    labelled = len(wrappers) >= 2
    pending = []  # wrapper names not yet put at the head of the ingredients and the steps

    kind, level, prose_seen = "text", 1, False

    def put(target, line):
        if labelled and pending:
            label = pending[-1]
            if target is ingredients and ("i", label) not in done:
                done.add(("i", label))
                target.append(label)
            if target is steps and ("s", label) not in done:
                done.add(("s", label))
                target.append(label)
        target.append(line)

    done = set()
    for i, (line_kind, mark, line, raw) in enumerate(lines):
        if line_kind == "heading":
            if i in wrappers:
                pending.append(line)
                kind, level = "wrapper", mark
                continue
            heading_kind = classify(line)
            if (heading_kind in ("text", kind) and kind not in ("text", "wrapper")
                    and mark > level):
                if kind != "drop" and line:
                    {"ingredients": ingredients, "mixed": ingredients, "steps": steps,
                     "notes": notes}[kind].append(line)
                continue
            kind, level, prose_seen = heading_kind, mark, False
            if kind == "text" and line:
                paras.append(line)
            # A second ingredients or steps part ("Preparazione variante …") is headed by its
            # heading; so is a note part with a name of its own ("Variante NinoChef").
            if kind in ("ingredients", "steps") and line:
                target = ingredients if kind == "ingredients" else steps
                if target:
                    target.append(line)
            if kind == "notes" and line and line.lower().strip(" :") not in PLAIN_NOTES:
                notes.append(line)
            if kind == "ingredients" and yields is None:
                tail = re.match(r"ingredienti\s*:?\s*(per\s.*\d.*)", line, re.IGNORECASE)
                if tail:
                    yields = tail.group(1).rstrip(":")
            continue
        if not line or kind == "drop":
            continue
        if kind == "ingredients":
            # Prose among the ingredients is mostly one ingredient to a line.
            parts = [inline(part, name) for part in raw.split("\n")]
            parts = [part for part in parts if part] if line_kind == "prose" else [line]
            first = parts[0] if parts else ""
            if (line_kind == "prose" and not prose_seen and not ingredients
                    and yields is None and YIELD_LINE.search(first) and len(first) <= 60
                    and YIELD_START.match(first) and not re.search(r":\s*\S", first)):
                yields = first.rstrip(":")
                parts = parts[1:]
            for part in parts:
                put(ingredients, part)
            prose_seen = prose_seen or line_kind == "prose"
        elif kind == "mixed":
            put(ingredients if mark.startswith("*") else steps, line)
        elif kind == "steps":
            put(steps, line)
        elif kind == "notes":
            notes.append(line)
        else:
            paras.append(line)
    notes += footnotes

    if not ingredients and not steps:
        return None

    entry = {
        "id": "wikibooks-" + slug(name),
        "title": name,
        "url": SITE + "/wiki/" + urllib.parse.quote(title.replace(" ", "_")),
    }
    porzioni = inline(box.get("porzioni", ""), name)
    if porzioni.lower() in ("n/a", "-"):
        porzioni = ""
    if porzioni and whole_servings(porzioni):
        entry["servings"] = whole_servings(porzioni)
    elif porzioni:
        entry["yield"] = porzioni
    if yields and "yield" not in entry:
        entry["yield"] = yields
        if "servings" not in entry and whole_servings(yields):
            entry["servings"] = whole_servings(yields)
    tempo = inline(box.get("tempo", ""), name)
    if tempo:
        entry["times"] = {"tempo": tempo}
    if region:
        entry["region"] = region
    image = tidy(re.sub(r"^(File|Immagine|Image):", "", box.get("immagine", ""), flags=re.I))
    if image and image in commons:
        entry["image"] = image
    if ingredients:
        entry["ingredients"] = ingredients
    if steps:
        entry["steps"] = steps
    if notes:
        entry["notes"] = notes
    if paras:
        entry["text"] = paras
    entry["authors_url"] = (SITE + "/w/index.php?title="
                            + urllib.parse.quote(title.replace(" ", "_")) + "&action=history")
    return entry


def main(argv):
    if len(argv) != 2:
        print("usage: pack-wikibooks.py <output.json>", file=sys.stderr)
        return 2
    italy, world = subcategories(ITALY), subcategories(WORLD)
    titles = recipe_titles()
    found = pages(titles)

    chosen, rejected, unstated, both = [], [], [], []
    for title in titles:
        text, categories = found.get(title, [None, []])
        if text is None:
            continue
        regions = sorted({italy[c][len(ITALY) + 1:] for c in categories if c in italy})
        here = regions or ITALY in categories
        abroad = [c for c in categories if c in world or c == WORLD]
        if here:
            chosen.append((title, text, regions[0] if len(regions) == 1 else None))
            if abroad:
                both.append(title[len(PREFIX):])
        elif abroad:
            rejected.append(title)
        else:
            unstated.append(title)

    boxes = {tidy(re.sub(r"^(File|Immagine|Image):", "", infobox(text).get("immagine", ""),
                         flags=re.I)) for _, text, _ in chosen}
    commons = on_commons(name for name in boxes if name)

    recipes, stubs = [], []
    for title, text, region in chosen:
        entry = recipe(title, text, region, commons)
        if entry:
            recipes.append(entry)
        else:
            stubs.append(title[len(PREFIX):])
    recipes.sort(key=lambda entry: entry["id"])
    ids = collections.Counter(entry["id"] for entry in recipes)
    clash = [i for i, n in ids.items() if n > 1]

    summary = (f"{len(recipes)} Italian recipes from {len(titles)} pages; "
               f"{len(rejected)} from elsewhere, {len(unstated)} naming no origin, "
               f"{len(stubs)} Italian stubs left out ({'; '.join(stubs) or 'none'})")
    if both:
        summary += f"; Italian and also filed abroad: {'; '.join(both)}"
    if unknown_templates:
        summary += "; other templates removed: " + ", ".join(
            f"{name} ×{n}" for name, n in sorted(unknown_templates.items()))
    if clash:
        print(f"{summary} — ids made twice: {', '.join(clash)}; nothing written", file=sys.stderr)
        return 1
    if len(recipes) < MIN_RECIPES:
        print(f"{summary} — fewer than {MIN_RECIPES}, nothing written", file=sys.stderr)
        return 1

    pack = {
        # What the app calls this collection: `key` prefixes a logged serving's source
        # (`wikibooks/<id>`), `label` is what each of its rows is labelled with (2026-10-18).
        "key": "wikibooks",
        "label": "Wikibooks",
        "source": SOURCE,
        "licence": LICENCE,
        "licence_url": LICENCE_URL,
        "home": HOME,
        "language": "it",
        "seen": time.strftime("%Y-%m-%d"),
        "recipes": recipes,
    }
    with open(argv[1], "w", encoding="utf-8", newline="\n") as out:
        json.dump(pack, out, ensure_ascii=False, indent=1)
        out.write("\n")
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
