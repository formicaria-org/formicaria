//! **The app's clock, as this device keeps it** (`decisions.md#data`, 2026-10-24, *one app clock*).
//!
//! One setting per device, never in a vault and never synced: `config_dir()/formicaria/clock.json`,
//! `{"zone": ""}`. Empty is **Automatic** — the zone the system reports to this process; otherwise a
//! zone name (`Asia/Kathmandu`) or an offset (`+05:45`). Every part of the app reads the one
//! [`Clock`] built from it: the store's seam, the calendar and mail readers, reminders, the UI and
//! the assistant (through the `clock` command, [`answer`]).

use fm_core::zone::{Clock, Zone};
use serde_json::{json, Value};
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn path() -> Option<PathBuf> {
    Some(crate::vaults::config_dir()?.join("formicaria").join("clock.json"))
}

/// The calendar panel's old zone setting (`calendars.json`'s `offset`, 2026-10-23), which said what
/// this setting now says: this computer's zone, when it should not be the system's.
fn old_setting() -> Option<String> {
    let p = crate::vaults::config_dir()?.join("formicaria").join("calendars.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    Some(v["offset"].as_str()?.trim().to_string()).filter(|s| !s.is_empty())
}

/// The clock this device chose, as saved: `Some("")` for Automatic, `None` when nothing is saved yet.
fn saved() -> Option<String> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path()?).ok()?).ok()?;
    Some(v["zone"].as_str().unwrap_or("").trim().to_string())
}

/// **This device's clock.** Read once at start; a saved zone that no longer parses is Automatic,
/// said in the log. With nothing saved, the calendar panel's old setting is adopted — once, and
/// written down, so the choice it expressed is kept.
pub fn load() -> Clock {
    let choice = match saved() {
        Some(z) => z,
        None => match old_setting() {
            Some(z) if Zone::parse(&z).is_ok() => {
                let _ = save(&z);
                eprintln!("note: the time zone set for calendars ({z}) is now the app's clock");
                z
            }
            _ => String::new(),
        },
    };
    if choice.is_empty() {
        return Clock::automatic();
    }
    match Zone::parse(&choice) {
        Ok(z) => Clock::chosen(z),
        Err(e) => {
            eprintln!("note: the app's clock is Automatic: {e}");
            Clock::automatic()
        }
    }
}

/// Save a choice: empty for Automatic, else a zone `Zone::parse` accepts. Refused, not guessed.
pub fn save(choice: &str) -> Result<Option<Zone>, String> {
    let choice = choice.trim();
    let zone = if choice.is_empty() { None } else { Some(Zone::parse(choice)?) };
    let p = path().ok_or("this device has nowhere to keep settings")?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text =
        serde_json::to_string_pretty(&json!({ "zone": choice })).map_err(|e| e.to_string())?;
    std::fs::write(&p, text + "\n").map_err(|e| e.to_string())?;
    Ok(zone)
}

/// **A `start`/`due` as a caller may send it** (2026-10-24, contract `instants`): a naive stamp, on
/// the app's clock, as before — or an **RFC 3339 moment** (`2026-11-02T16:30:00+08:00`, `…Z`), which
/// is read onto the app's clock here and kept to the minute. A moment cannot be misread by a clock
/// that changed between the assistant reading it and formicaria writing it.
pub fn stamp_value(raw: &str, zone: Option<&Zone>) -> Result<String, String> {
    let t = raw.trim();
    let has_offset = t.len() > 16 && (t.ends_with('Z') || t[16..].contains(['+', '-']));
    if !has_offset {
        return Ok(raw.to_string());
    }
    let Ok(at) = OffsetDateTime::parse(t, &Rfc3339) else { return Ok(raw.to_string()) };
    let zone =
        zone.ok_or("this device's time zone cannot be read — choose one in Settings → Clock")?;
    let here = at.to_offset(zone.offset_at(at.unix_timestamp()));
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        here.year(),
        u8::from(here.month()),
        here.day(),
        here.hour(),
        here.minute()
    ))
}

/// **Where the internet's clock comes from** — a shell that talks to the internet registers it
/// (`fm_fetch::clock_skew`); `fm-app` makes no request of its own. `(host, seconds this machine is
/// ahead)`.
/// The shell's answer: `(host, seconds this machine is ahead)`.
pub type SkewSource = fn() -> Option<(String, i64)>;

static SKEW: std::sync::OnceLock<SkewSource> = std::sync::OnceLock::new();

/// Register the shell's skew source, once.
pub fn set_skew_source(f: SkewSource) {
    let _ = SKEW.set(f);
}

/// The date of an instant on `zone` — never its UTC date (F10).
pub fn date_of(t: OffsetDateTime, zone: &Zone) -> time::Date {
    t.to_offset(zone.offset_at(t.unix_timestamp())).date()
}

/// **The `clock` command's answer** — what the UI and the assistant read (contract `v: 1`; fields are
/// only ever added). `zone` is `null` when neither a choice nor the system's zone can be read: a
/// reader must then say so, never assume UTC.
pub fn answer(clock: &Clock) -> Value {
    let now = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap_or(OffsetDateTime::now_utc());
    let zone = clock.zone();
    let offset = zone.as_ref().map(Zone::offset_now);
    json!({
        "v": 1,
        "zone": zone.as_ref().map(Zone::label),
        "iana": zone.as_ref().and_then(|z| z.name().map(str::to_string)),
        "offset": offset.map(fm_core::zone::offset_label),
        "offset_seconds": offset,
        "now": now.format(&Rfc3339).unwrap_or_default(),
        "today": zone.as_ref().map(|z| z.today().to_string()),
        "source": clock.source(),
        "chosen": clock.choice().map(|z| z.label()).unwrap_or_default(),
        "system": Zone::system().map(|z| z.label()),
        "instants": true,
        // **The internet check, a warning only** (2026-10-24): how far this machine's clock was from
        // the last answer of Google or GitHub. Nothing is ever adjusted; `null` until one answered.
        "skew_seconds": SKEW.get().and_then(|f| f()).map(|(_, s)| s),
        "skew_from": SKEW.get().and_then(|f| f()).map(|(h, _)| h),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_names_its_zone_and_its_day_on_that_zone() {
        let c = Clock::chosen(Zone::parse("Asia/Kathmandu").unwrap());
        let a = answer(&c);
        assert_eq!(a["zone"], "Asia/Kathmandu");
        assert_eq!(a["offset"], "+05:45");
        assert_eq!(a["offset_seconds"], 20700);
        assert_eq!(a["source"], "chosen");
        assert_eq!(a["v"], 1);
        let today = Zone::parse("Asia/Kathmandu").unwrap().today().to_string();
        assert_eq!(a["today"], today);
    }

    #[test]
    fn a_moment_is_read_onto_the_app_clock_and_a_stamp_is_kept() {
        let ktm = Zone::parse("Asia/Kathmandu").unwrap();
        assert_eq!(stamp_value("2026-11-02T08:30:00Z", Some(&ktm)).unwrap(), "2026-11-02T14:15");
        assert_eq!(
            stamp_value("2026-11-02T16:30:00+08:00", Some(&ktm)).unwrap(),
            "2026-11-02T14:15"
        );
        assert_eq!(stamp_value("2026-11-02T16:30", Some(&ktm)).unwrap(), "2026-11-02T16:30");
        assert_eq!(stamp_value("2026-11-02", None).unwrap(), "2026-11-02");
        assert!(
            stamp_value("2026-11-02T08:30:00Z", None).is_err(),
            "no clock: refused, not guessed"
        );
    }

    #[test]
    fn an_instant_s_date_is_on_the_clock_not_utc() {
        let late = time::macros::datetime!(2026-10-23 22:30 UTC);
        let ktm = Zone::parse("+05:45").unwrap();
        assert_eq!(date_of(late, &ktm).to_string(), "2026-10-24");
        let stj = Zone::parse("-03:30").unwrap();
        assert_eq!(date_of(late, &stj).to_string(), "2026-10-23");
    }
}
