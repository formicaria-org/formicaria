-- The layout of a food reference pack. **One definition, two readers:** `ci/pack-foods.py` runs
-- this to make a pack, and `pack.rs` runs it to make the small one its tests read — so the builder
-- and the reader cannot come to disagree about a column.
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
-- `code` is the food's own identifier in the table it came from, prefixed with that table
-- (`ciqual:1000`, `bls:C133000`, `usda:2709224`) — so a logged entry's `source` still means
-- something outside this file. `src` is the table; `grp` its food group there, by the name that
-- table publishes (empty where it publishes none). `id` is only a row number, for the name index.
-- `alt` is the food's other names: what its own table calls it in its own language or languages,
-- and its synonyms, joined by ` · ` (`decisions.md#data`, 2026-10-13). Published names only —
-- nothing here is a translation made by the app.
CREATE TABLE food(id INTEGER PRIMARY KEY, code TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
                  src TEXT NOT NULL, grp TEXT NOT NULL DEFAULT '', alt TEXT NOT NULL DEFAULT '');
CREATE TABLE nutrient(n INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE);
-- Every value per 100 g, as a whole number in the unit the nutrient's key names.
CREATE TABLE amount(food INTEGER NOT NULL, n INTEGER NOT NULL, per100 INTEGER NOT NULL,
                    PRIMARY KEY(food, n)) WITHOUT ROWID;
CREATE TABLE portion(food INTEGER NOT NULL, seq INTEGER NOT NULL, label TEXT NOT NULL,
                     grams INTEGER NOT NULL, PRIMARY KEY(food, seq)) WITHOUT ROWID;
-- Names only, folded like the app's own search, with short prefixes indexed: a food is found by
-- the first letters of a word, typed on a phone.
CREATE VIRTUAL TABLE names USING fts5(name, alt, content='food', content_rowid='id',
                                      tokenize='unicode61 remove_diacritics 2', prefix='1 2 3');
-- **A value a food's own table never measures, borrowed** (`decisions.md#data`, 2026-10-29): from
-- another table's closest food, where the food's table carries that nutrient for almost none of its
-- foods. The value is in `amount` like any other; this says where it came from, so the app can say
-- it. A pack built before has no such table, and every value in it is the food's own.
CREATE TABLE borrowed(food INTEGER NOT NULL, n INTEGER NOT NULL, from_food INTEGER NOT NULL,
                      PRIMARY KEY(food, n)) WITHOUT ROWID;
-- **One food, one set of values** (`decisions.md#data`, 2026-10-29, later that night): the same food
-- in several tables — macadamia nuts in BLS, Ciqual, CREA and USDA — is one group, and every member
-- holds the group's values (each nutrient from the first table, in a fixed order, that measured
-- it). `first` is the group's first member; a search shows a group once. A food in no group has
-- no row. A pack built before has no such table.
CREATE TABLE same(food INTEGER PRIMARY KEY, first INTEGER NOT NULL);
-- **A value estimated from similar foods** (same night): where no table measured a nutrient for a
-- food or one close enough, and only for a nutrient and a kind of food where hiding measured values
-- and estimating them came out within 20% typically. The value is in `amount`; `basis` says from
-- what ("12 similar meats"). A pack built before has no such table.
CREATE TABLE estimated(food INTEGER NOT NULL, n INTEGER NOT NULL, basis TEXT NOT NULL,
                       PRIMARY KEY(food, n)) WITHOUT ROWID;
