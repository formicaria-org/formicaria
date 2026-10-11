-- The layout of a food reference pack. **One definition, two readers:** `ci/pack-foods.py` runs
-- this to make a pack, and `pack.rs` runs it to make the small one its tests read — so the builder
-- and the reader cannot come to disagree about a column.
CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
-- `id` is FoodData Central's own id for the food, so a logged entry's `source` still means
-- something outside this file.
CREATE TABLE food(id INTEGER PRIMARY KEY, name TEXT NOT NULL, src TEXT NOT NULL);
CREATE TABLE nutrient(n INTEGER PRIMARY KEY, key TEXT NOT NULL UNIQUE);
-- Every value per 100 g, as a whole number in the unit the nutrient's key names.
CREATE TABLE amount(food INTEGER NOT NULL, n INTEGER NOT NULL, per100 INTEGER NOT NULL,
                    PRIMARY KEY(food, n)) WITHOUT ROWID;
CREATE TABLE portion(food INTEGER NOT NULL, seq INTEGER NOT NULL, label TEXT NOT NULL,
                     grams INTEGER NOT NULL, PRIMARY KEY(food, seq)) WITHOUT ROWID;
-- Names only, folded like the app's own search, with short prefixes indexed: a food is found by
-- the first letters of a word, typed on a phone.
CREATE VIRTUAL TABLE names USING fts5(name, content='food', content_rowid='id',
                                      tokenize='unicode61 remove_diacritics 2', prefix='1 2 3');
