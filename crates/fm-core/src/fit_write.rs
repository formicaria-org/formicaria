//! **A planned session as a FIT workout file** — the watch's own format for a workout it can follow
//! (`decisions.md` 2026-11-20, *Fitness gets a Coach tab*, item 6: *the watch receives workouts*;
//! the coach plan's S.7). Plain values in, the file's bytes out; no `Store`, no network, no paths.
//! Which words of a note become which step is `fm_app::workout`'s business, not this module's.
//!
//! # What it writes
//!
//! One FIT file of type *workout* (5): `file_id`, one `workout` message (name, sport, number of
//! steps) and one `workout_step` per step, a repeat written as the protocol's *repeat until steps
//! complete* step after the steps it repeats. **Targets are absolute or open, never a percentage**:
//! a heart rate is written as a custom range in bpm with the protocol's +100 offset (a custom value of
//! 100 or below means "% of maximum heart rate", which the watch would resolve against its own
//! settings), and a speed as a custom range in m/s × 1000. A zone number is never written either —
//! the `target_value` of a custom target is 0.
//!
//! # Provenance
//!
//! Written with the `rustyfit` crate, version 0.11.2 (BSD-3-Clause), the writer the decision named;
//! its message and field numbers, types and scales come from that crate's generated profile (FIT
//! profile 21.221). The scales applied here are the profile's: time in ms, distance in cm, speed in
//! mm/s, weight in kg × 100, heart rate + 100. The same note is in `docs/src/dev/fit-provenance.md`.

use rustyfit::profile::{mesgdef, typedef};
use rustyfit::proto::{Message, FIT};

/// A step's place in the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Warmup,
    Work,
    Recover,
    Rest,
    Cooldown,
}

/// How long a step lasts. `Open`: until the person presses the lap key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Open,
    Seconds(f64),
    Metres(f64),
    Reps(u32),
}

/// What a step aims at. Absolute only; there is no way to say "a percentage" here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Open,
    /// Beats a minute, both ends inclusive.
    HeartRate {
        low: u32,
        high: u32,
    },
    /// Metres a second; `low` is the slower end.
    Speed {
        low: f64,
        high: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub part: Part,
    /// Shown on the watch as the step's name (short).
    pub name: String,
    /// The session's own words for the step.
    pub notes: String,
    pub length: Length,
    pub target: Target,
    /// For a strength step: the weight, kilograms.
    pub weight_kg: Option<f64>,
}

/// A step, or a group of steps done `times` times.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Step(Step),
    Repeat { times: u32, steps: Vec<Step> },
}

/// The sport the watch starts the workout as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sport {
    Running,
    Cycling,
    Swimming,
    Walking,
    Rowing,
    Strength,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Workout {
    pub name: String,
    pub sport: Sport,
    pub items: Vec<Item>,
    /// When the file is made, Unix seconds UTC (the file's identity on the watch).
    pub created: i64,
}

/// The FIT protocol's own offset for a heart rate written as bpm rather than % of maximum.
pub const HR_OFFSET: u32 = 100;
/// More steps than this is not a session a watch takes (watches cap well below the field's range).
pub const MAX_STEPS: usize = 100;
/// The longest name or note written, in bytes (the watch shows far less).
const MAX_TEXT: usize = 120;

/// The text cut to at most `n` bytes on a character boundary.
fn cut(s: &str, n: usize) -> Box<str> {
    let s = s.trim();
    if s.len() <= n {
        return Box::from(s);
    }
    let mut end = n;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    Box::from(&s[..end])
}

fn sport(s: Sport) -> (typedef::Sport, typedef::SubSport) {
    match s {
        Sport::Running => (typedef::Sport::RUNNING, typedef::SubSport::GENERIC),
        Sport::Cycling => (typedef::Sport::CYCLING, typedef::SubSport::GENERIC),
        Sport::Swimming => (typedef::Sport::SWIMMING, typedef::SubSport::LAP_SWIMMING),
        Sport::Walking => (typedef::Sport::WALKING, typedef::SubSport::GENERIC),
        Sport::Rowing => (typedef::Sport::ROWING, typedef::SubSport::GENERIC),
        Sport::Strength => (typedef::Sport::TRAINING, typedef::SubSport::STRENGTH_TRAINING),
        Sport::Other => (typedef::Sport::GENERIC, typedef::SubSport::GENERIC),
    }
}

fn intensity(p: Part) -> typedef::Intensity {
    match p {
        Part::Warmup => typedef::Intensity::WARMUP,
        Part::Work => typedef::Intensity::ACTIVE,
        Part::Recover => typedef::Intensity::RECOVERY,
        Part::Rest => typedef::Intensity::REST,
        Part::Cooldown => typedef::Intensity::COOLDOWN,
    }
}

/// A whole number in a field's range, or why not.
fn scaled(v: f64, scale: f64, what: &str) -> Result<u32, String> {
    let x = (v * scale).round();
    if !(x.is_finite() && x >= 1.0 && x < f64::from(u32::MAX)) {
        return Err(format!("{what} is not a number a watch can follow"));
    }
    Ok(x as u32)
}

fn step_message(index: u16, s: &Step) -> Result<Message, String> {
    let mut m = mesgdef::WorkoutStep::new();
    m.message_index = typedef::MessageIndex(index);
    m.wkt_step_name = cut(&s.name, MAX_TEXT);
    m.notes = cut(&s.notes, MAX_TEXT);
    m.intensity = intensity(s.part);
    match s.length {
        Length::Open => m.duration_type = typedef::WktStepDuration::OPEN,
        Length::Seconds(t) => {
            m.duration_type = typedef::WktStepDuration::TIME;
            m.duration_value = scaled(t, 1000.0, "a step's time")?;
        }
        Length::Metres(d) => {
            m.duration_type = typedef::WktStepDuration::DISTANCE;
            m.duration_value = scaled(d, 100.0, "a step's distance")?;
        }
        Length::Reps(n) => {
            m.duration_type = typedef::WktStepDuration::REPS;
            m.duration_value = n.max(1);
        }
    }
    match s.target {
        Target::Open => m.target_type = typedef::WktStepTarget::OPEN,
        Target::HeartRate { low, high } => {
            let (low, high) = (low.min(high), low.max(high));
            if !(25..=250).contains(&low) || !(25..=250).contains(&high) {
                return Err(format!("{low}–{high} bpm is not a heart rate"));
            }
            m.target_type = typedef::WktStepTarget::HEART_RATE;
            // A custom range, not a zone; the offset says "bpm", not "% of maximum".
            m.target_value = 0;
            m.custom_target_value_low = low + HR_OFFSET;
            m.custom_target_value_high = high + HR_OFFSET;
        }
        Target::Speed { low, high } => {
            let (low, high) = (low.min(high), low.max(high));
            m.target_type = typedef::WktStepTarget::SPEED;
            m.target_value = 0;
            m.custom_target_value_low = scaled(low, 1000.0, "a pace")?;
            m.custom_target_value_high = scaled(high, 1000.0, "a pace")?;
        }
    }
    if let Some(w) = s.weight_kg {
        m.exercise_weight = u16::try_from(scaled(w, 100.0, "a weight")?)
            .map_err(|_| format!("{w} kg is more than a watch records"))?;
        m.weight_display_unit = typedef::FitBaseUnit::KILOGRAM;
    }
    Ok(Message::from(m))
}

/// The workout's bytes, or one sentence saying what stops it.
pub fn encode(w: &Workout) -> Result<Vec<u8>, String> {
    let mut steps: Vec<Message> = Vec::new();
    let mut index: u16 = 0;
    let next = |index: &mut u16| -> Result<u16, String> {
        let i = *index;
        if usize::from(i) >= MAX_STEPS {
            return Err(format!("A watch takes at most {MAX_STEPS} steps in one workout."));
        }
        *index += 1;
        Ok(i)
    };
    for item in &w.items {
        match item {
            Item::Step(s) => {
                let i = next(&mut index)?;
                steps.push(step_message(i, s)?);
            }
            Item::Repeat { times, steps: group } => {
                if group.is_empty() {
                    continue;
                }
                let first = index;
                for s in group {
                    let i = next(&mut index)?;
                    steps.push(step_message(i, s)?);
                }
                let mut r = mesgdef::WorkoutStep::new();
                r.message_index = typedef::MessageIndex(next(&mut index)?);
                r.duration_type = typedef::WktStepDuration::REPEAT_UNTIL_STEPS_CMPLT;
                // The step to go back to, and how many times in all.
                r.duration_value = u32::from(first);
                r.target_value = (*times).max(1);
                steps.push(Message::from(r));
            }
        }
    }
    if steps.is_empty() {
        return Err("The session has no steps to send.".into());
    }

    let created = (w.created - crate::fit::FIT_EPOCH).clamp(0x1000_0000, i64::from(u32::MAX) - 1);
    let mut id = mesgdef::FileId::new();
    id.r#type = typedef::File::WORKOUT;
    id.manufacturer = typedef::Manufacturer::DEVELOPMENT;
    id.product = 0;
    id.time_created = typedef::DateTime(created as u32);

    let (sp, sub) = sport(w.sport);
    let mut wk = mesgdef::Workout::new();
    wk.wkt_name = cut(if w.name.trim().is_empty() { "Workout" } else { &w.name }, MAX_TEXT);
    wk.sport = sp;
    wk.sub_sport = sub;
    wk.num_valid_steps = index;

    let mut messages = vec![Message::from(id), Message::from(wk)];
    messages.extend(steps);
    let mut fit = FIT { messages, ..Default::default() };
    let mut out = Cursor::default();
    rustyfit::Encoder::new()
        .encode(&mut out, &mut fit)
        .map_err(|e| format!("The workout file could not be written ({e:?})."))?;
    Ok(out.bytes)
}

/// Bytes in memory that the encoder can write into and go back over (it fills the header last).
#[derive(Default)]
struct Cursor {
    bytes: Vec<u8>,
    at: usize,
}

impl embedded_io::ErrorType for Cursor {
    type Error = core::convert::Infallible;
}

impl embedded_io::Write for Cursor {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let end = self.at + buf.len();
        if end > self.bytes.len() {
            self.bytes.resize(end, 0);
        }
        self.bytes[self.at..end].copy_from_slice(buf);
        self.at = end;
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl embedded_io::Seek for Cursor {
    fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
        let len = self.bytes.len() as i64;
        let at = match pos {
            embedded_io::SeekFrom::Start(n) => n as i64,
            embedded_io::SeekFrom::End(n) => len + n,
            embedded_io::SeekFrom::Current(n) => self.at as i64 + n,
        };
        self.at = at.max(0) as usize;
        Ok(self.at as u64)
    }
}

#[cfg(test)]
#[path = "fit_write_tests.rs"]
mod tests;
