//! Reading a published calendar — iCalendar today, a feed and embedded JSON-LD next.
//!
//! # Why this sits here, and not in the agent
//!
//! An events digest looks like an assistant feature, and half of it is. The half that turns a
//! published feed into notes is **not**: it is deterministic parsing with no model in it, and it has
//! to keep working with the assistant switched off — which it cannot do if it links the assistant.
//! `fm-fetch`'s own header records the trap in almost these words: reaching a downloader through
//! `fm-agent-run` *"would have linked the whole study-agent pipeline into a notes-only build",*
//! quietly retiring the property `ci/checks.sh` guards — that `fm-serve --no-default-features` is a
//! *provably* agent-free core. So this is a sibling of [`crate::import`], for the same reason
//! `import` is not in the agent either: **a foreign format becoming notes is core work**
//! (`decisions.md`, *An import converts; adoption renders*).
//!
//! Like `import`, this module only ever *reads*. It returns records; it writes nothing, creates
//! nothing, and knows nothing about a `Store`. Fetching is somebody else's job too — there is no
//! HTTP client in `fm-core` and there is not going to be one.
//!
//! # Why there is no timezone database here
//!
//! [`Stamp`] is deliberately naive, and its own doc says why: *"A 14:30 meeting is at 14:30 where you
//! are, and the file is the truth: pinning an offset would make the on-disk text drift with travel
//! and DST."* That decides the hard part of reading a calendar before we start. A
//! `DTSTART;TZID=Asia/Singapore:20260923T183000` is stored as 18:30 on 2026-09-23 — **the wall clock
//! the publisher wrote, carried across unchanged**. No conversion happens, so no conversion can be
//! wrong: the DST bugs that fill other projects' issue trackers (expanding a rule against UTC so it
//! drifts an hour twice a year; a local time that does not exist on a spring-forward day; one that
//! happens twice on a fall-back day) arise when converting *between* zones, and we never do.
//!
//! The one case that genuinely needs an offset is a UTC timestamp — `20260923T103000Z` — because the
//! venue's wall clock is not recoverable from it without knowing where the venue is. So the caller
//! passes the offset it wants applied, explicitly, and **when it passes none the time is dropped and
//! the day is kept**. That is deliberate: reading `Z` as though it were wall-clock time would show a
//! Singapore event eight hours early, which is precisely the failure that sends someone to a venue at
//! the wrong time. A missing time says "sometime that day", which is true. Nothing here reads the
//! machine's local offset — `time`'s `local-offset` feature is not enabled in this workspace, and it
//! is documented as failing in exactly the multi-threaded process `fm-serve` is.
//!
//! # The shape of the work
//!
//! [`unfold`] first, because RFC 5545 wraps long lines and a parser that misses this silently
//! truncates every long title. Then one pass per `VEVENT`, reading only the properties we map, and
//! **never inventing a field that is absent** — an event with no date is returned with no date, for
//! the caller to drop. Guessing here is how a listing acquires a plausible wrong time.

use fm_model::Stamp;
use time::{Date, Time, UtcOffset};

/// One event as its publisher described it — the parser's whole output.
///
/// Every field except `title` and `quote` is optional, and that is the point: a publisher who omits
/// a venue gets a record with no venue, not a guess. The caller decides what is usable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventRecord {
    /// The publisher's own identity for this event — `UID` in iCalendar. The idempotency key: it is
    /// what lets a re-read update an event in place instead of writing a second note for it.
    pub uid: Option<String>,
    /// `SEQUENCE`. A publisher bumps it when the event's details change, which is the only signal
    /// that times should be rewritten on a note somebody may have annotated.
    pub seq: Option<u32>,
    pub title: String,
    /// Wall-clock, naive, as written by the publisher. See the module doc on why there is no offset.
    pub start: Option<Stamp>,
    pub end: Option<Stamp>,
    pub location: Option<String>,
    pub organizer: Option<String>,
    /// The event's own page. Load-bearing for the provenance check the caller applies: an event whose
    /// link is on a different host from the feed that carried it is not that venue's event.
    pub url: Option<String>,
    /// `STATUS` / schema.org `eventStatus`, lowercased. `cancelled` is the only standard signal that
    /// an event is off, and it marks a note rather than deleting one.
    pub status: Option<String>,
    /// Verbatim source text for this event, so a later claim about it can be checked against what the
    /// publisher actually said rather than against a paraphrase.
    pub quote: String,
    /// The `TZID` the start was written in, when one was. Not resolved (see the module doc) — but
    /// carried, so a meeting written in another zone can say so instead of passing as local time.
    pub tzid: Option<String>,
    /// `RECURRENCE-ID`, read as a stamp: this record overrides the occurrence of a repeating event
    /// (same `UID`) that was due on this day. Part of the identity — without it, a moved Tuesday
    /// would overwrite the whole series.
    pub recurrence_id: Option<Stamp>,
    /// The `RRULE` value, verbatim (`FREQ=WEEKLY;BYDAY=TU`). Expanded by `crate::calendar`, which
    /// owns the subset it can read; the parser only carries it.
    pub rrule: Option<String>,
    /// `EXDATE`s: occurrences the organiser removed from the series.
    pub exdates: Vec<Stamp>,
}

impl EventRecord {
    /// Whether this record says when it happens. A record without a start is not schedulable and the
    /// caller is expected to drop it — but it is *returned*, so the caller can say how many it
    /// dropped and why, rather than silently seeing fewer events than the feed contained.
    pub fn is_dated(&self) -> bool {
        self.start.is_some()
    }
}

/// Undo RFC 5545 line folding: a line beginning with a space or tab continues the one before it.
///
/// First, not last, and not optional. A publisher wraps at 75 octets, so a long `SUMMARY` or a
/// `DESCRIPTION` carrying the event's link is routinely split mid-word — and a parser that reads
/// lines naively does not fail, it just returns a truncated title and loses the URL. That is the
/// quiet kind of wrong this module exists to avoid.
pub fn unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        match line.strip_prefix([' ', '\t']) {
            // A continuation: join with no separator — the fold happened mid-value.
            Some(rest) => {
                if let Some(last) = out.last_mut() {
                    last.push_str(rest);
                    continue;
                }
                // A continuation with nothing to continue is malformed; keep it as its own line
                // rather than dropping text we were given.
                out.push(rest.to_string());
            }
            None => out.push(line.to_string()),
        }
    }
    out
}

/// One parsed content line: its name, its parameters, and its still-escaped value.
type Line<'a> = (String, Vec<(String, String)>, &'a str);

/// Split `NAME;PARAM=V;PARAM=V:value` into its three parts.
///
/// The colon search has to skip anything inside a quoted parameter value, because RFC 5545 allows
/// `TZID="Europe/Berlin"` and a naive `find(':')` would cut a `mailto:` organizer parameter in half.
fn split_line(line: &str) -> Option<Line<'_>> {
    let mut quoted = false;
    let mut colon = None;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => {
                colon = Some(i);
                break;
            }
            _ => {}
        }
    }
    let colon = colon?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);

    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|p| {
            let (k, v) = p.split_once('=')?;
            Some((k.trim().to_ascii_uppercase(), v.trim().trim_matches('"').to_string()))
        })
        .collect();
    Some((name, params, value))
}

/// Unescape an iCalendar TEXT value: `\n` is a newline, and `\, \; \\` are literal.
///
/// Order matters — a backslash-escaped backslash must not then unescape whatever follows it — so this
/// walks the string once rather than running a series of replacements.
fn unescape_text(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') | Some('N') => out.push('\n'),
            Some('\\') => out.push('\\'),
            Some(',') => out.push(','),
            Some(';') => out.push(';'),
            // An escape we do not know: keep both characters rather than eating the text.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Read a `DATE` or `DATE-TIME` value into a naive [`Stamp`].
///
/// Four shapes, and the fourth is the interesting one:
/// - `VALUE=DATE:20260923` → an all-day stamp.
/// - `TZID=Asia/Singapore:20260923T183000` → 18:30, carried across verbatim. The zone name is
///   *deliberately not resolved*: the wall clock is what a person reads off a poster.
/// - `20260923T183000` (floating, no zone) → the same, which is what "floating" means.
/// - `20260923T103000Z` → UTC. Only here is an offset needed, and only here can it be absent: with
///   `offset` given the time is shifted into that offset; with none, the **day is kept and the time
///   dropped**, because a wrong time is worse than no time.
pub(crate) fn parse_dt(
    value: &str,
    params: &[(String, String)],
    offset: Option<UtcOffset>,
) -> Option<Stamp> {
    let v = value.trim();
    let is_date_only =
        params.iter().any(|(k, val)| k == "VALUE" && val.eq_ignore_ascii_case("DATE"))
            || (v.len() == 8 && !v.contains('T'));

    let date_part = v.get(..8)?;
    let date = Date::from_calendar_date(
        date_part.get(..4)?.parse::<i32>().ok()?,
        time::Month::try_from(date_part.get(4..6)?.parse::<u8>().ok()?).ok()?,
        date_part.get(6..8)?.parse::<u8>().ok()?,
    )
    .ok()?;

    if is_date_only {
        return Some(Stamp::day(date));
    }

    let rest = v.get(8..)?.strip_prefix('T')?;
    let utc = rest.ends_with('Z');
    let hms = rest.trim_end_matches('Z');
    let time = Time::from_hms(
        hms.get(..2)?.parse::<u8>().ok()?,
        hms.get(2..4)?.parse::<u8>().ok()?,
        // Seconds are present in practice and `Stamp` truncates them; a value without them is still
        // read rather than refused.
        hms.get(4..6).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0),
    )
    .ok()?;

    if !utc {
        return Some(Stamp::at(date, time));
    }
    match offset {
        // Shift the instant into the offset the caller asked for, letting the date roll.
        Some(off) => {
            let shifted = time::PrimitiveDateTime::new(date, time).assume_utc().to_offset(off);
            Some(Stamp::at(shifted.date(), shifted.time()))
        }
        // No offset to apply: say "that day" rather than a time we cannot justify.
        None => Some(Stamp::day(date)),
    }
}

/// Pull a URL out of free text — the fallback for a publisher that ships no `URL` property.
///
/// Luma's city feeds are the reason this exists: they are the best keyless city-wide source found,
/// they carry 48 events for Singapore, and they have **no `URL:` line at all** — the canonical link
/// is a sentence inside `DESCRIPTION` ("Get up-to-date information at: https://luma.com/…"). Without
/// this, every event from the single best source would arrive with nothing to link to, which also
/// means nothing for the provenance check to check.
fn url_in_text(text: &str) -> Option<String> {
    let at = text.find("https://").or_else(|| text.find("http://"))?;
    let tail = &text[at..];
    let end = tail
        .find(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"')
        .unwrap_or(tail.len());
    // Trailing sentence punctuation is not part of the URL.
    let url = tail[..end].trim_end_matches(['.', ',', ')', ';']);
    (url.len() > "https://".len()).then(|| url.to_string())
}

/// Read every `VEVENT` in an iCalendar document.
///
/// `offset` is applied only to UTC (`Z`) timestamps — see [`parse_dt`]. Malformed events are skipped
/// rather than failing the document: a feed with one bad entry is still worth the other forty-seven,
/// and a publisher's mistake is not a reason to show a person nothing.
pub fn parse_ics(text: &str, offset: Option<UtcOffset>) -> Vec<EventRecord> {
    let mut out = Vec::new();
    let mut cur: Option<(EventRecord, Vec<String>)> = None;
    let mut description = String::new();

    for line in unfold(text) {
        let upper = line.trim().to_ascii_uppercase();
        if upper == "BEGIN:VEVENT" {
            cur = Some((EventRecord::default(), Vec::new()));
            description.clear();
            continue;
        }
        if upper == "END:VEVENT" {
            if let Some((mut rec, raw)) = cur.take() {
                if rec.url.is_none() {
                    rec.url = url_in_text(&description);
                }
                rec.quote = raw.join("\n");
                // A record with no title is not an event anybody can read; everything else is kept.
                if !rec.title.trim().is_empty() {
                    out.push(rec);
                }
            }
            continue;
        }
        let Some((rec, raw)) = cur.as_mut() else { continue };
        raw.push(line.clone());
        let Some((name, params, value)) = split_line(&line) else { continue };
        match name.as_str() {
            "SUMMARY" => rec.title = unescape_text(value).trim().to_string(),
            "DTSTART" => {
                rec.start = parse_dt(value, &params, offset);
                rec.tzid = params.iter().find(|(k, _)| k == "TZID").map(|(_, v)| v.clone());
            }
            "RECURRENCE-ID" => rec.recurrence_id = parse_dt(value, &params, offset),
            "RRULE" => {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    rec.rrule = Some(v);
                }
            }
            // A comma-separated list, and the property may also repeat.
            "EXDATE" => {
                rec.exdates.extend(value.split(',').filter_map(|v| parse_dt(v, &params, offset)))
            }
            "DTEND" => rec.end = parse_dt(value, &params, offset),
            "LOCATION" => {
                let v = unescape_text(value).trim().to_string();
                if !v.is_empty() {
                    rec.location = Some(v);
                }
            }
            "UID" => {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    rec.uid = Some(v);
                }
            }
            "SEQUENCE" => rec.seq = value.trim().parse().ok(),
            "STATUS" => rec.status = Some(value.trim().to_ascii_lowercase()),
            "URL" => {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    rec.url = Some(v);
                }
            }
            "ORGANIZER" => {
                // Prefer the human-readable `CN=` where there is one; fall back to the address with
                // its `mailto:` scheme stripped, because a name is what a reader wants.
                let cn = params.iter().find(|(k, _)| k == "CN").map(|(_, v)| v.clone());
                let addr = value.trim().trim_start_matches("mailto:").to_string();
                rec.organizer = cn.or(if addr.is_empty() { None } else { Some(addr) });
            }
            "DESCRIPTION" => description = unescape_text(value),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, offset, time};

    #[test]
    fn a_folded_line_is_rejoined_before_anything_else_reads_it() {
        // RFC 5545 wraps at 75 octets. Reading lines naively truncates the title instead of failing,
        // which is why this is the first thing the parser does.
        let ics = "BEGIN:VEVENT\r\nSUMMARY:A talk with a very long title that the\r\n  publisher had to wrap\r\nDTSTART:20260923T183000\r\nEND:VEVENT\r\n";
        let got = parse_ics(ics, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "A talk with a very long title that the publisher had to wrap");
    }

    #[test]
    fn a_tzid_time_is_carried_across_as_the_wall_clock_the_publisher_wrote() {
        // The zone is deliberately not resolved: 18:30 in Singapore is 18:30 on the poster, and
        // `Stamp` is naive on purpose. No conversion means no conversion bug.
        let ics = "BEGIN:VEVENT\nSUMMARY:GDG Monthly Meetup\nDTSTART;TZID=Asia/Singapore:20260923T183000\nDTEND;TZID=Asia/Singapore:20260923T203000\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].start, Some(Stamp::at(date!(2026 - 09 - 23), time!(18:30))));
        assert_eq!(got[0].end, Some(Stamp::at(date!(2026 - 09 - 23), time!(20:30))));
    }

    #[test]
    fn a_value_date_becomes_an_all_day_stamp_with_no_time() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Exhibition opens\nDTSTART;VALUE=DATE:20260923\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].start, Some(Stamp::day(date!(2026 - 09 - 23))));
        assert!(!got[0].start.unwrap().has_time());
    }

    #[test]
    fn a_utc_time_without_an_offset_keeps_its_day_and_drops_its_time() {
        // The failure this prevents: reading 10:30Z as 10:30 wall-clock shows a Singapore event eight
        // hours early. "Sometime that day" is true; a wrong time sends someone to the wrong place at
        // the wrong time, which is the whole hazard this feature has to avoid.
        let ics = "BEGIN:VEVENT\nSUMMARY:A webinar\nDTSTART:20260923T103000Z\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].start, Some(Stamp::day(date!(2026 - 09 - 23))));
        assert!(
            !got[0].start.unwrap().has_time(),
            "a UTC time must not be passed off as wall-clock"
        );
    }

    #[test]
    fn a_utc_time_with_an_offset_is_shifted_and_may_roll_the_date() {
        // 18:30Z at +08:00 is 02:30 the following day — the roll is the reason this is computed
        // rather than added to the hour.
        let ics = "BEGIN:VEVENT\nSUMMARY:A late webinar\nDTSTART:20260923T183000Z\nEND:VEVENT";
        let got = parse_ics(ics, Some(offset!(+8)));
        assert_eq!(got[0].start, Some(Stamp::at(date!(2026 - 09 - 24), time!(02:30))));
    }

    #[test]
    fn escaped_text_is_unescaped_once_and_a_double_backslash_does_not_cascade() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Jazz\\, blues\\; and a \\\\n that is not a newline\nDTSTART;VALUE=DATE:20260923\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].title, "Jazz, blues; and a \\n that is not a newline");
    }

    #[test]
    fn a_quoted_tzid_does_not_cut_the_line_at_the_wrong_colon() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Concert\nDTSTART;TZID=\"Europe/Berlin\":20260923T200000\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].start, Some(Stamp::at(date!(2026 - 09 - 23), time!(20:00))));
    }

    #[test]
    fn the_link_is_recovered_from_the_description_when_there_is_no_url_property() {
        // Luma's city feeds — the best keyless city source found — ship no URL property at all.
        // Without this, every event from that source would have nothing to link to, and so nothing
        // for the provenance check to verify.
        let ics = "BEGIN:VEVENT\nSUMMARY:GDG Monthly Meetup\nDTSTART;VALUE=DATE:20260923\nDESCRIPTION:Come along.\\n\\nGet up-to-date information at: https://luma.com/gdgsg-sep26\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].url.as_deref(), Some("https://luma.com/gdgsg-sep26"));
    }

    #[test]
    fn an_explicit_url_property_wins_over_one_found_in_prose() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Concert\nDTSTART;VALUE=DATE:20260923\nURL:https://venue.example/concert\nDESCRIPTION:See also https://elsewhere.example/other\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].url.as_deref(), Some("https://venue.example/concert"));
    }

    #[test]
    fn uid_sequence_and_status_are_read_because_updating_in_place_depends_on_them() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Concert\nDTSTART;VALUE=DATE:20260923\nUID:abc-123@venue.example\nSEQUENCE:3\nSTATUS:CANCELLED\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].uid.as_deref(), Some("abc-123@venue.example"));
        assert_eq!(got[0].seq, Some(3));
        assert_eq!(got[0].status.as_deref(), Some("cancelled"));
    }

    #[test]
    fn an_organizer_prefers_its_common_name_over_its_address() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Talk\nDTSTART;VALUE=DATE:20260923\nORGANIZER;CN=The Physics Department:mailto:physics@uni.example\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].organizer.as_deref(), Some("The Physics Department"));
    }

    #[test]
    fn an_organizer_falls_back_to_the_address_with_its_scheme_stripped() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Talk\nDTSTART;VALUE=DATE:20260923\nORGANIZER:mailto:physics@uni.example\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got[0].organizer.as_deref(), Some("physics@uni.example"));
    }

    #[test]
    fn absent_is_absent_and_nothing_is_invented_to_fill_a_field() {
        // The measured failure mode in LLM extraction is inventing values for optional-sounding
        // fields. The deterministic path must not do by omission what we refuse to let a model do.
        let ics = "BEGIN:VEVENT\nSUMMARY:Something\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got.len(), 1);
        let r = &got[0];
        assert!(r.start.is_none() && r.end.is_none());
        assert!(r.location.is_none() && r.organizer.is_none() && r.url.is_none());
        assert!(r.uid.is_none() && r.seq.is_none() && r.status.is_none());
        assert!(!r.is_dated(), "a record with no start is not schedulable and must say so");
    }

    #[test]
    fn one_malformed_event_does_not_cost_the_reader_the_rest_of_the_feed() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Good one\nDTSTART;VALUE=DATE:20260923\nEND:VEVENT\nBEGIN:VEVENT\nDTSTART:garbage\nEND:VEVENT\nBEGIN:VEVENT\nSUMMARY:Another good one\nDTSTART;VALUE=DATE:20260924\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert_eq!(got.len(), 2, "the untitled middle event is skipped, the others survive");
        assert_eq!(got[0].title, "Good one");
        assert_eq!(got[1].title, "Another good one");
    }

    #[test]
    fn calendar_level_properties_outside_an_event_are_ignored() {
        let ics = "BEGIN:VCALENDAR\nX-WR-CALNAME:What's Happening in Singapore\nREFRESH-INTERVAL;VALUE=DURATION:PT12H\nBEGIN:VEVENT\nSUMMARY:Real event\nDTSTART;VALUE=DATE:20260923\nEND:VEVENT\nEND:VCALENDAR";
        let got = parse_ics(ics, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "Real event");
    }

    #[test]
    fn the_quote_keeps_the_publishers_own_lines_so_a_claim_can_be_checked_against_them() {
        let ics = "BEGIN:VEVENT\nSUMMARY:Concert\nDTSTART;VALUE=DATE:20260923\nLOCATION:Esplanade Concert Hall\nEND:VEVENT";
        let got = parse_ics(ics, None);
        assert!(got[0].quote.contains("SUMMARY:Concert"));
        assert!(got[0].quote.contains("LOCATION:Esplanade Concert Hall"));
    }
}

// ---------------------------------------------------------------------------
// Rung 2: RSS and Atom.
// ---------------------------------------------------------------------------

/// Read an RSS 2.0 or Atom feed.
///
/// # `pubDate` is not the event's date, and this is the trap
///
/// A feed's `pubDate`/`<updated>` says when the *item was published*, which for an events feed is
/// usually the day somebody added the listing — not the day the event happens. Mapping it to `start`
/// would be the single easiest way to fill a calendar with confident, wrong dates, and it would look
/// right in testing because the two are often days apart rather than obviously absurd.
///
/// So this reads a date **only** where the feed states one as an event date: the RSS event module's
/// `<ev:startdate>`/`<ev:enddate>`, or a `<dc:date>`. Otherwise the record comes back with **no
/// start**, [`EventRecord::is_dated`] is false, and the caller reports it as something a person must
/// look at. That is the honest outcome, and it is also a real finding about this rung: a plain events
/// RSS feed is good for *discovering* that something exists and bad at saying when.
pub fn parse_feed(text: &str, offset: Option<UtcOffset>) -> Vec<EventRecord> {
    let atom = text.contains("<entry");
    let (open, close) = if atom { ("<entry", "</entry>") } else { ("<item", "</item>") };
    let mut out = Vec::new();
    for chunk in text.split(open).skip(1) {
        let item = chunk.split(close).next().unwrap_or("");
        let title = strip_tags(&inner(item, "title"));
        if title.trim().is_empty() {
            continue;
        }
        // Atom puts the address in an attribute; RSS puts it in the element's text.
        let url = if atom { link_href(item) } else { non_empty(inner(item, "link")) };
        let start = non_empty(inner(item, "ev:startdate"))
            .or_else(|| non_empty(inner(item, "dc:date")))
            .and_then(|s| parse_iso(&s, offset));
        let end = non_empty(inner(item, "ev:enddate")).and_then(|s| parse_iso(&s, offset));
        let body =
            strip_tags(&inner(item, "description")) + " " + &strip_tags(&inner(item, "summary"));
        out.push(EventRecord {
            uid: non_empty(inner(item, "guid")).or_else(|| non_empty(inner(item, "id"))),
            seq: None,
            title: title.trim().to_string(),
            start,
            end,
            location: non_empty(inner(item, "ev:location")),
            organizer: None,
            url: url.or_else(|| url_in_text(&body)),
            status: None,
            quote: item.trim().to_string(),
            ..Default::default()
        });
    }
    out
}

/// The text between `<tag ...>` and `</tag>`, or empty. Hand-rolled, like `parse_arxiv` next door —
/// this project does not take an XML dependency to read four elements.
fn inner(xml: &str, tag: &str) -> String {
    let Some(at) = xml.find(&format!("<{tag}")) else { return String::new() };
    let rest = &xml[at..];
    // Skip the rest of the opening tag, so attributes do not land in the value.
    let Some(gt) = rest.find('>') else { return String::new() };
    let after = &rest[gt + 1..];
    let end = after.find(&format!("</{tag}>")).unwrap_or(after.len());
    decode_entities(after[..end].trim())
}

/// Atom's `<link href="…"/>`, preferring a rel="alternate" (or absent rel) over an enclosure.
fn link_href(item: &str) -> Option<String> {
    for chunk in item.split("<link").skip(1) {
        let tag = chunk.split('>').next().unwrap_or("");
        if tag.contains("rel=") && !tag.contains("alternate") {
            continue;
        }
        if let Some(at) = tag.find("href=") {
            let v = tag[at + 5..].trim_start_matches(['"', '\'']);
            let end = v.find(['"', '\'']).unwrap_or(v.len());
            return non_empty(decode_entities(&v[..end]));
        }
    }
    None
}

fn non_empty(s: String) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Strip tags and collapse whitespace — feed descriptions carry escaped HTML routinely.
pub(crate) fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The five XML entities, plus the numeric forms feeds use for punctuation.
pub(crate) fn decode_entities(s: &str) -> String {
    let mut out = s
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#8217;", "\u{2019}")
        .replace("&nbsp;", " ");
    // Ampersand last, so an already-decoded `&amp;lt;` does not become a tag.
    out = out.replace("&amp;", "&");
    out
}

/// An ISO 8601 date or date-time — `2026-09-23`, `2026-09-23T18:30:00+02:00`, `…Z`.
///
/// **A stated offset is read and then discarded**, deliberately: a publisher writing
/// `2026-09-12T11:00:00+02:00` is telling us the event is at 11:00 where it happens, and 11:00 is what
/// a person reads off the poster. `Stamp` is naive for exactly this reason. `Z` is the one case with
/// no wall clock in it, so it follows the same rule as iCalendar: shifted if the caller gave an
/// offset, otherwise the day is kept and the time dropped.
fn parse_iso(s: &str, offset: Option<UtcOffset>) -> Option<Stamp> {
    let s = s.trim();
    let (d, rest) = s.split_once('T').map_or((s, None), |(d, t)| (d, Some(t)));
    let mut it = d.split('-');
    let date = Date::from_calendar_date(
        it.next()?.parse::<i32>().ok()?,
        time::Month::try_from(it.next()?.parse::<u8>().ok()?).ok()?,
        it.next()?.get(..2)?.parse::<u8>().ok()?,
    )
    .ok()?;
    let Some(rest) = rest else { return Some(Stamp::day(date)) };

    let utc = rest.ends_with('Z') || rest.ends_with('z');
    // Cut a numeric offset off the time; we read the wall clock in front of it.
    let hms = match rest.find(['+', 'Z', 'z']) {
        Some(i) => &rest[..i],
        // A negative offset, but not the `-` inside a time. Times have no `-`.
        None => rest.split('-').next().unwrap_or(rest),
    };
    let mut parts = hms.split(':');
    let time = Time::from_hms(
        parts.next()?.parse::<u8>().ok()?,
        parts.next().and_then(|m| m.parse::<u8>().ok()).unwrap_or(0),
        parts.next().and_then(|s| s.get(..2)?.parse::<u8>().ok()).unwrap_or(0),
    )
    .ok()?;
    if !utc {
        return Some(Stamp::at(date, time));
    }
    match offset {
        Some(off) => {
            let shifted = time::PrimitiveDateTime::new(date, time).assume_utc().to_offset(off);
            Some(Stamp::at(shifted.date(), shifted.time()))
        }
        None => Some(Stamp::day(date)),
    }
}

// ---------------------------------------------------------------------------
// Rung 3: schema.org/Event as JSON-LD embedded in a page.
// ---------------------------------------------------------------------------

/// Read every `schema.org/Event` out of a page's `<script type="application/ld+json">` blocks.
///
/// Four containers, because publishers use all of them: a bare `Event`, an array of them, an
/// `@graph`, and an `ItemList` whose `itemListElement` holds `ListItem`s wrapping the events (which is
/// what a city-wide "what's on" page emits). A block that does not parse is skipped rather than
/// failing the page — a single malformed script is common and is not a reason to show nothing.
pub fn parse_jsonld(html: &str, offset: Option<UtcOffset>) -> Vec<EventRecord> {
    let mut out = Vec::new();
    for chunk in html.split("<script").skip(1) {
        let Some(gt) = chunk.find('>') else { continue };
        if !chunk[..gt].contains("ld+json") {
            continue;
        }
        let body = chunk[gt + 1..].split("</script>").next().unwrap_or("");
        let Ok(v) = serde_json::from_str::<serde_json::Value>(body.trim()) else { continue };
        collect_events(&v, offset, &mut out);
    }
    out
}

/// Walk a JSON-LD value, pushing every node that is an `Event`.
///
/// Recurses into arrays, `@graph` and `itemListElement`/`item` rather than matching one fixed shape,
/// because the container is the part publishers disagree about while the `Event` node itself is
/// consistent. Depth is bounded by the document.
fn collect_events(v: &serde_json::Value, offset: Option<UtcOffset>, out: &mut Vec<EventRecord>) {
    match v {
        serde_json::Value::Array(items) => {
            for i in items {
                collect_events(i, offset, out);
            }
        }
        serde_json::Value::Object(map) => {
            if is_event_type(map.get("@type")) {
                if let Some(rec) = event_from_jsonld(map, offset) {
                    out.push(rec);
                }
                return;
            }
            for key in ["@graph", "itemListElement", "item", "subEvent"] {
                if let Some(inner) = map.get(key) {
                    collect_events(inner, offset, out);
                }
            }
        }
        _ => {}
    }
}

/// Whether an `@type` names an event. schema.org has 25+ subtypes (`MusicEvent`, `TheaterEvent`,
/// `ExhibitionEvent`, `Festival`, …) and a page may give `@type` as an array, so this is a suffix test
/// rather than an equality one — the alternative is a hard-coded list that silently drops a subtype
/// nobody thought of.
fn is_event_type(t: Option<&serde_json::Value>) -> bool {
    let one = |s: &str| s == "Event" || s.ends_with("Event");
    match t {
        Some(serde_json::Value::String(s)) => one(s),
        Some(serde_json::Value::Array(a)) => a.iter().any(|v| v.as_str().map(one).unwrap_or(false)),
        _ => false,
    }
}

fn event_from_jsonld(
    map: &serde_json::Map<String, serde_json::Value>,
    offset: Option<UtcOffset>,
) -> Option<EventRecord> {
    let title = map.get("name")?.as_str()?.trim().to_string();
    if title.is_empty() {
        return None;
    }
    let str_at = |k: &str| map.get(k).and_then(|v| v.as_str()).map(|s| s.trim().to_string());
    // A `location` is a Place with a name, or occasionally just a string. An address is not
    // synthesised from parts here: the caller wants what the publisher said.
    let location = match map.get("location") {
        Some(serde_json::Value::String(s)) => non_empty(s.to_string()),
        Some(serde_json::Value::Object(p)) => p
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                p.get("address").and_then(|a| match a {
                    serde_json::Value::String(s) => Some(s.to_string()),
                    serde_json::Value::Object(ad) => {
                        ad.get("streetAddress").and_then(|v| v.as_str()).map(|s| s.to_string())
                    }
                    _ => None,
                })
            })
            .and_then(non_empty),
        _ => None,
    };
    let organizer = match map.get("organizer") {
        Some(serde_json::Value::String(s)) => non_empty(s.to_string()),
        Some(serde_json::Value::Object(o)) => {
            o.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()).and_then(non_empty)
        }
        _ => None,
    };
    Some(EventRecord {
        uid: str_at("@id").or_else(|| str_at("identifier")),
        seq: None,
        title,
        start: str_at("startDate").and_then(|s| parse_iso(&s, offset)),
        end: str_at("endDate").and_then(|s| parse_iso(&s, offset)),
        location,
        organizer,
        url: str_at("url"),
        // `https://schema.org/EventCancelled` -> `cancelled`, matching iCalendar's `STATUS`.
        status: str_at("eventStatus").map(|s| {
            s.rsplit('/').next().unwrap_or(&s).trim_start_matches("Event").to_ascii_lowercase()
        }),
        quote: serde_json::to_string(map).unwrap_or_default(),
        ..Default::default()
    })
}

#[cfg(test)]
mod feed_tests {
    use super::*;
    use time::macros::{date, offset, time};

    #[test]
    fn an_rss_items_pubdate_is_never_read_as_the_events_date() {
        // The trap this rung exists to avoid. pubDate is when the listing was posted; treating it as
        // the event's date fills a calendar with confident wrong days that look plausible in tests.
        let rss = "<rss><channel><item><title>Jazz at the Hall</title><link>https://venue.example/jazz</link><pubDate>Sat, 12 Sep 2026 07:47:10 +0200</pubDate></item></channel></rss>";
        let got = parse_feed(rss, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "Jazz at the Hall");
        assert!(got[0].start.is_none(), "pubDate must not become a start");
        assert!(!got[0].is_dated(), "an undated record must say it is undated");
    }

    #[test]
    fn an_rss_item_that_states_an_event_date_is_read() {
        let rss = "<rss><channel><item><title>Talk</title><link>https://uni.example/talk</link><ev:startdate>2026-09-23T18:30:00+08:00</ev:startdate></item></channel></rss>";
        let got = parse_feed(rss, None);
        assert_eq!(got[0].start, Some(Stamp::at(date!(2026 - 09 - 23), time!(18:30))));
    }

    #[test]
    fn an_atom_entry_takes_its_address_from_the_link_href() {
        let atom = "<feed><entry><title>Exhibition</title><link rel=\"alternate\" href=\"https://museum.example/show\"/><id>tag:museum,2026:1</id></entry></feed>";
        let got = parse_feed(atom, None);
        assert_eq!(got[0].url.as_deref(), Some("https://museum.example/show"));
        assert_eq!(got[0].uid.as_deref(), Some("tag:museum,2026:1"));
    }

    #[test]
    fn feed_titles_are_entity_decoded_and_stripped_of_markup() {
        let rss = "<rss><channel><item><title>Jazz &amp; blues</title><description>&lt;p&gt;Come   along&lt;/p&gt;</description><link>https://v.example/x</link></item></channel></rss>";
        let got = parse_feed(rss, None);
        assert_eq!(got[0].title, "Jazz & blues");
    }

    #[test]
    fn an_iso_datetime_keeps_its_wall_clock_and_discards_the_stated_offset() {
        // Museumsportal Berlin publishes `2026-09-12T11:00:00+02:00`. 11:00 is what a visitor reads;
        // storing an offset is what `Stamp` exists to avoid.
        assert_eq!(
            parse_iso("2026-09-12T11:00:00+02:00", None),
            Some(Stamp::at(date!(2026 - 09 - 12), time!(11:00)))
        );
        assert_eq!(
            parse_iso("2026-09-12T11:00:00-05:00", None),
            Some(Stamp::at(date!(2026 - 09 - 12), time!(11:00)))
        );
    }

    #[test]
    fn an_iso_utc_datetime_follows_the_same_rule_as_icalendar() {
        assert_eq!(
            parse_iso("2026-09-23T18:30:00Z", None),
            Some(Stamp::day(date!(2026 - 09 - 23)))
        );
        assert_eq!(
            parse_iso("2026-09-23T18:30:00Z", Some(offset!(+8))),
            Some(Stamp::at(date!(2026 - 09 - 24), time!(02:30)))
        );
    }

    #[test]
    fn a_bare_event_in_a_script_block_is_read_with_its_place_and_status() {
        let html = r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@type":"Event","@id":"https://luma.com/gdgsg-sep26",
         "url":"https://luma.com/gdgsg-sep26","name":"GDG Monthly Meetup #2609",
         "startDate":"2026-09-23T18:30:00+08:00","endDate":"2026-09-23T20:30:00+08:00",
         "eventStatus":"https://schema.org/EventScheduled",
         "location":{"@type":"Place","name":"80 Pasir Panjang Rd"}}
        </script></head></html>"#;
        let got = parse_jsonld(html, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "GDG Monthly Meetup #2609");
        assert_eq!(got[0].start, Some(Stamp::at(date!(2026 - 09 - 23), time!(18:30))));
        assert_eq!(got[0].location.as_deref(), Some("80 Pasir Panjang Rd"));
        assert_eq!(got[0].status.as_deref(), Some("scheduled"));
        assert_eq!(got[0].url.as_deref(), Some("https://luma.com/gdgsg-sep26"));
    }

    #[test]
    fn events_wrapped_in_an_itemlist_are_found_because_city_pages_emit_that_shape() {
        let html = r#"<script type="application/ld+json">
        {"@type":"ItemList","itemListElement":[
          {"@type":"ListItem","item":{"@type":"MusicEvent","name":"One","startDate":"2026-09-23"}},
          {"@type":"ListItem","item":{"@type":"TheaterEvent","name":"Two","startDate":"2026-09-24"}}]}
        </script>"#;
        let got = parse_jsonld(html, None);
        assert_eq!(got.len(), 2, "a subtype of Event is still an Event");
        assert_eq!(got[0].title, "One");
        assert_eq!(got[1].start, Some(Stamp::day(date!(2026 - 09 - 24))));
    }

    #[test]
    fn a_cancelled_event_is_reported_as_cancelled_rather_than_dropped() {
        // Insertion-only: a check may mark a note, never delete one.
        let html = r#"<script type="application/ld+json">
        {"@type":"Event","name":"Off","startDate":"2026-09-23",
         "eventStatus":"https://schema.org/EventCancelled"}</script>"#;
        let got = parse_jsonld(html, None);
        assert_eq!(got[0].status.as_deref(), Some("cancelled"));
    }

    #[test]
    fn a_malformed_script_block_does_not_cost_the_page_its_other_events() {
        let html = r#"<script type="application/ld+json">{ not json </script>
        <script type="application/ld+json">{"@type":"Event","name":"Survivor","startDate":"2026-09-23"}</script>"#;
        let got = parse_jsonld(html, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "Survivor");
    }

    #[test]
    fn a_script_block_that_is_not_json_ld_is_ignored() {
        let html = r#"<script type="text/javascript">var Event = {"name":"not an event"}</script>"#;
        assert!(parse_jsonld(html, None).is_empty());
    }

    #[test]
    fn a_graph_wrapper_is_walked_and_non_event_nodes_are_left_alone() {
        let html = r#"<script type="application/ld+json">
        {"@context":"https://schema.org","@graph":[
          {"@type":"WebPage","name":"What's on"},
          {"@type":"Organization","name":"The Venue"},
          {"@type":"Event","name":"The only event","startDate":"2026-09-23"}]}
        </script>"#;
        let got = parse_jsonld(html, None);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "The only event");
    }
}
