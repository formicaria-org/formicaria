//! **How well a note answers a search** — the order search results come in.
//!
//! Search used to come back newest-edited first, so a note *about* a word could sit below a hundred
//! notes that merely mention it (2026-10-29, measured; `known-issues.md`, *Search — what it cannot
//! find*). The order is computed **here, in the pure engine**, not by SQLite's `bm25()`: a store only
//! narrows (`Store::candidates`) and `MultiStore` concatenates every vault's candidates before the
//! engine runs once, so a score that lived inside one vault's index could not be compared with
//! another vault's, and `MemoryStore` has no index at all. Computed from the note itself, the same
//! score ranks every store and every vault alike.
//!
//! The score is an integer and needs no statistics over the collection: for each word of the search,
//! **where** it is found (the title above a tag, above a property, above the body) and **whether it
//! starts a word** there; a few more points for repeats in the body; a bonus when the whole search
//! is found as written in the title or the body. Letters are compared with their accents folded
//! (`légumes` and `legumes` are one word), matching what the index already does. Chinese and Japanese
//! text has no spaces, so there any occurrence counts as the start of a word.

use fm_model::{Object, UNSEARCHED_KEYS};

/// The relevance of `obj` to the search `text`. Zero when no word of it is found (a note the index
/// matched on something this cannot see, such as a word only inside a link, still ranks — last).
pub fn relevance(text: &str, obj: &Object) -> u32 {
    let words = words(text);
    if words.is_empty() {
        return 0;
    }
    let title = fold(obj.title.as_deref().unwrap_or(""));
    let body = fold(&obj.body);
    let tags: Vec<String> = obj.tags.iter().map(|t| fold(t)).collect();
    let props: Vec<String> = obj
        .extra
        .iter()
        .filter(|(k, v)| !v.is_null() && !UNSEARCHED_KEYS.contains(&k.as_str()))
        .map(|(_, v)| fold(&v.display()))
        .collect();

    let mut score = 0u32;
    for w in &words {
        let in_title = found(&title, w).map_or(0, |start| if start { 100 } else { 60 });
        let in_tags = tags
            .iter()
            .map(|t| {
                if t == w {
                    80
                } else {
                    found(t, w).map_or(0, |start| if start { 60 } else { 30 })
                }
            })
            .max()
            .unwrap_or(0);
        let in_props = props
            .iter()
            .map(|p| found(p, w).map_or(0, |start| if start { 40 } else { 20 }))
            .max()
            .unwrap_or(0);
        score += in_title.max(in_tags).max(in_props) + in_body(&body, w);
    }
    // The search as written, word for word, in order.
    if words.len() > 1 {
        let phrase = words.join(" ");
        if spaced(&title).contains(&phrase) {
            score += 150;
        } else if spaced(&body).contains(&phrase) {
            score += 20;
        }
    }
    score
}

/// The words of a search: runs of letters and digits, folded. The same split the index uses
/// (`fm-core`'s `fts_tokens`), so a word scored is a word searched.
pub(crate) fn words(text: &str) -> Vec<String> {
    fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

/// Up to 30 points for the body: 10 for a word that starts there (5 if only inside a word), and 2
/// for each further occurrence up to ten — so a note *about* the word gains on one that names it once,
/// but no body outweighs a title.
fn in_body(body: &str, w: &str) -> u32 {
    let (mut seen, mut any_start, mut from) = (0u32, false, 0);
    while let Some(i) = body[from..].find(w) {
        let at = from + i;
        any_start |= starts_word(body, at, w);
        seen += 1;
        if seen == 11 && any_start {
            break;
        }
        from = at + w.len();
    }
    match seen {
        0 => 0,
        n => (if any_start { 10 } else { 5 }) + 2 * (n - 1).min(10),
    }
}

/// Whether `w` is in `hay`, and if so whether some occurrence starts a word.
fn found(hay: &str, w: &str) -> Option<bool> {
    let mut any = false;
    let mut from = 0;
    while let Some(i) = hay[from..].find(w) {
        let at = from + i;
        if starts_word(hay, at, w) {
            return Some(true);
        }
        any = true;
        from = at + w.len();
    }
    any.then_some(false)
}

fn starts_word(hay: &str, at: usize, w: &str) -> bool {
    w.chars().next().is_some_and(is_cjk)
        || hay[..at].chars().next_back().is_none_or(|c| !c.is_alphanumeric())
}

/// Text with every run of non-letters as one space, for the phrase test.
fn spaced(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Han, kana and hangul: scripts written without spaces between words. Shared with `fm-core`'s
/// index, which matches such text anywhere for the same reason.
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F)
}

/// Lower case, with the accents of Latin letters removed — the index's `remove_diacritics 2`, for
/// the letters of the languages the app is used in. A letter without an entry is kept as it is.
pub(crate) fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
            'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => 'c',
            'ď' | 'đ' => 'd',
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
            'ĝ' | 'ğ' | 'ġ' | 'ģ' => 'g',
            'ĥ' | 'ħ' => 'h',
            'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => 'i',
            'ĵ' => 'j',
            'ķ' => 'k',
            'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => 'l',
            'ñ' | 'ń' | 'ņ' | 'ň' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => 'o',
            'ŕ' | 'ŗ' | 'ř' => 'r',
            'ś' | 'ŝ' | 'ş' | 'š' => 's',
            'ţ' | 'ť' | 'ŧ' => 't',
            'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => 'u',
            'ŵ' => 'w',
            'ý' | 'ÿ' | 'ŷ' => 'y',
            'ź' | 'ż' | 'ž' => 'z',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fm_model::{Kind, PropertyValue};

    fn note(title: &str, body: &str, tags: &[&str]) -> Object {
        let mut o = Object::new(Kind::Note, body);
        o.title = Some(title.into());
        o.tags = tags.iter().map(|t| (*t).to_string()).collect();
        o
    }

    #[test]
    fn a_word_in_the_title_outranks_the_same_word_in_the_body() {
        let about = note("Vitamin K", "Found in leafy greens.", &[]);
        let mention = note("Shopping", "Buy greens; they have vitamin K.", &[]);
        assert!(relevance("vitamin k", &about) > relevance("vitamin k", &mention));
    }

    #[test]
    fn a_title_outranks_a_tag_outranks_a_property_outranks_the_body() {
        let mut prop = note("x", "", &[]);
        prop.extra.insert("venue".into(), PropertyValue::Text("Ferritin Society".into()));
        let s = |o: &Object| relevance("ferritin", o);
        let title = note("Ferritin", "", &[]);
        let tag = note("x", "", &["ferritin"]);
        let body = note("x", "ferritin", &[]);
        assert!(
            s(&title) > s(&tag) && s(&tag) > s(&prop) && s(&prop) > s(&body),
            "{} {} {} {}",
            s(&title),
            s(&tag),
            s(&prop),
            s(&body)
        );
    }

    #[test]
    fn a_note_about_a_word_outranks_one_that_names_it_once() {
        let about = note("x", "Iron. Iron absorption. Iron and tea. Iron stores.", &[]);
        let once = note("x", "Bought iron tablets.", &[]);
        assert!(relevance("iron", &about) > relevance("iron", &once));
    }

    #[test]
    fn the_start_of_a_word_counts_more_than_the_inside_of_one() {
        let start = note("Beratung", "", &[]);
        let inside = note("Ernährungsberatung", "", &[]);
        assert!(relevance("beratung", &start) > relevance("beratung", &inside));
        assert!(relevance("beratung", &inside) > 0, "the inside of a compound is still found");
    }

    #[test]
    fn the_search_as_written_scores_above_its_words_apart() {
        let phrase = note("Blood pressure at home", "", &[]);
        let apart = note("Home blood test, pressure cooker", "", &[]);
        assert!(relevance("blood pressure", &phrase) > relevance("blood pressure", &apart));
    }

    #[test]
    fn accents_are_folded_both_ways() {
        let n = note("Légumes verts", "", &[]);
        assert_eq!(relevance("legumes", &n), relevance("légumes", &n));
        assert!(relevance("legumes", &n) >= 100);
    }

    #[test]
    fn chinese_is_found_inside_a_run_without_spaces() {
        let n = note("你好世界", "我喜欢学习", &[]);
        assert!(relevance("世界", &n) >= 100, "in the title");
        assert!(relevance("学习", &n) > 0, "in the body");
    }

    #[test]
    fn bookkeeping_properties_do_not_count() {
        let mut n = note("x", "", &[]);
        n.extra.insert("import_hash".into(), PropertyValue::Text("vitamin".into()));
        assert_eq!(relevance("vitamin", &n), 0);
    }

    #[test]
    fn nothing_to_search_scores_nothing() {
        assert_eq!(relevance(" ?! ", &note("Vitamin", "", &[])), 0);
    }
}
