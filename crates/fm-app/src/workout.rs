//! **A planned Coach session as a watch workout** (`decisions.md` 2026-11-20, *Fitness gets a Coach
//! tab*, item 6; the coach plan's S.7). Reads one planned entry — its steps in the body, as the Coach
//! writes them (`- Warm-up: …`), and its `minutes`, `distance_m` and `target` — and hands
//! `fm_core::fit_write` the steps. The file goes to the watch's *Workouts* list through the person's
//! own watch app; fm never talks to a watch.
//!
//! **Only the person's own numbers are written.** A step takes a time, a distance, a count of
//! repetitions, a weight, a heart rate in bpm or a pace when its line says one; nothing is filled
//! in. A line with no number becomes an **open** step — it lasts until the lap key is pressed — with
//! the line's words as its notes, so an assembled plan (which carries no number of its own) still
//! gives the watch its structure. **A percentage is never sent**: the watch would resolve it against
//! its own settings, so the words keep it and the answer says so.

use fm_core::fit_write::{self, Item, Length, Part, Sport, Step, Target, Workout};
use fm_model::{Object, PropertyValue};
use serde::Serialize;

/// What the page gets: the file, and what it holds, in words.
#[derive(Serialize, Debug)]
pub struct WorkoutFile {
    /// A file name for the person to recognise (`2026-03-14-intervals-run.fit`).
    pub name: String,
    /// The file, standard base64.
    pub data: String,
    /// Steps written, counting each repeat as one.
    pub steps: usize,
    /// No step has a time, distance or count: each lasts until the lap key is pressed.
    pub open: bool,
    /// Sentences to show beside the buttons (what was not sent, and why).
    pub notes: Vec<String>,
}

fn text(o: &Object, k: &str) -> String {
    match o.get(k) {
        PropertyValue::Text(s) => s,
        PropertyValue::Int(n) => n.to_string(),
        _ => String::new(),
    }
}

fn number(o: &Object, k: &str) -> Option<f64> {
    match o.get(k) {
        PropertyValue::Int(n) => Some(n as f64),
        PropertyValue::Text(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite() && *v > 0.0)
}

// ── reading a line ───────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    /// `m:ss`, in seconds.
    Clock(f64),
    Word(String),
    Sym(char),
}

fn tokens(s: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.to_lowercase().chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || matches!(cs[i], '.' | ',' | ':')) {
                i += 1;
            }
            let raw: String = cs[start..i].iter().collect();
            let raw = raw.trim_end_matches(['.', ',', ':']);
            if let Some((m, s)) = raw.split_once(':') {
                match (m.parse::<f64>(), s.parse::<f64>()) {
                    (Ok(m), Ok(s)) if s < 60.0 && s.fract() == 0.0 => {
                        out.push(Tok::Clock(m * 60.0 + s))
                    }
                    _ => out.push(Tok::Word(raw.to_string())),
                }
            } else if let Ok(n) = raw.replace(',', ".").parse::<f64>() {
                out.push(Tok::Num(n));
            }
        } else if c.is_alphabetic() {
            let start = i;
            while i < cs.len() && cs[i].is_alphabetic() {
                i += 1;
            }
            out.push(Tok::Word(cs[start..i].iter().collect()));
        } else {
            i += 1;
            match c {
                '×' => out.push(Tok::Word("x".into())),
                '–' | '—' | '-' => out.push(Tok::Sym('-')),
                c if c.is_whitespace() => {}
                c => out.push(Tok::Sym(c)),
            }
        }
    }
    out
}

/// What one line says, read conservatively.
#[derive(Debug, Default, PartialEq)]
struct Said {
    times: Option<u32>,
    seconds: Option<f64>,
    metres: Option<f64>,
    reps: Option<u32>,
    kg: Option<f64>,
    target: Option<Target>,
    percent: bool,
}

fn is_word(t: Option<&Tok>, words: &[&str]) -> bool {
    matches!(t, Some(Tok::Word(w)) if words.contains(&w.as_str()))
}
fn is_range(t: Option<&Tok>) -> bool {
    matches!(t, Some(Tok::Sym('-'))) || is_word(t, &["to"])
}

const MINUTES: &[&str] = &["min", "mins", "minute", "minutes"];
const SECONDS: &[&str] = &["s", "sec", "secs", "second", "seconds"];
const HOURS: &[&str] = &["h", "hr", "hrs", "hour", "hours"];
const KM: &[&str] = &["km", "k", "kms", "kilometre", "kilometres", "kilometer", "kilometers"];
const METRES: &[&str] = &["m", "metre", "metres", "meter", "meters"];

fn read(line: &str) -> Said {
    let t = tokens(line);
    let mut s = Said::default();
    let mut i = 0;
    // The end of the last time read, so `1 h 30 min` is one time and `1 min run, 2 min walk` is not
    // three minutes.
    let mut time_end: Option<usize> = None;
    while i < t.len() {
        let at = |k: usize| t.get(k);
        match &t[i] {
            Tok::Clock(a) => {
                // A pace: `4:30/km`, `4:30-4:45 /km`, `1:50/100 m`, `7:00/mi`.
                let (b, j) = match (is_range(at(i + 1)), at(i + 2)) {
                    (true, Some(Tok::Clock(b))) => (*b, i + 3),
                    _ => (*a, i + 1),
                };
                if matches!(at(j), Some(Tok::Sym('/'))) {
                    let per = if is_word(at(j + 1), KM) {
                        Some((1000.0, j + 2))
                    } else if is_word(at(j + 1), &["mi", "mile"]) {
                        Some((1609.344, j + 2))
                    } else if matches!(at(j + 1), Some(Tok::Num(n)) if *n == 100.0)
                        && is_word(at(j + 2), METRES)
                    {
                        Some((100.0, j + 3))
                    } else {
                        None
                    };
                    if let Some((metres, end)) = per {
                        if *a > 0.0 && b > 0.0 && s.target.is_none() {
                            // The slower pace is the lower speed.
                            s.target = Some(Target::Speed {
                                low: metres / a.max(b),
                                high: metres / a.min(b),
                            });
                        }
                        i = end;
                        continue;
                    }
                }
                i += 1;
            }
            Tok::Num(a) => {
                let a = *a;
                let (b, j) = match (is_range(at(i + 1)), at(i + 2)) {
                    (true, Some(Tok::Num(b))) => (*b, i + 3),
                    _ => (a, i + 1),
                };
                if is_word(at(j), &["bpm"]) {
                    if s.target.is_none() {
                        s.target = Some(Target::HeartRate { low: a as u32, high: b as u32 });
                    }
                    i = j + 1;
                } else if is_word(at(j), KM)
                    && matches!(at(j + 1), Some(Tok::Sym('/')))
                    && is_word(at(j + 2), &["h"])
                {
                    if s.target.is_none() && a > 0.0 && b > 0.0 {
                        s.target =
                            Some(Target::Speed { low: a.min(b) / 3.6, high: a.max(b) / 3.6 });
                    }
                    i = j + 3;
                } else if matches!(at(i + 1), Some(Tok::Sym('%')))
                    || is_word(at(j), &["percent"])
                    || matches!(at(j), Some(Tok::Sym('%')))
                {
                    s.percent = true;
                    i = j + 1;
                } else if is_word(at(i + 1), &["x"])
                    || is_word(at(i + 1), &["sets", "rounds", "times"])
                {
                    s.times = s.times.or(Some(a as u32).filter(|n| *n > 1));
                    i += 2;
                } else if is_word(at(i + 1), &["reps", "rep", "repetitions"]) {
                    s.reps = s.reps.or(Some(a as u32).filter(|n| *n > 0));
                    i += 2;
                } else if is_word(at(i + 1), &["kg", "kgs"]) {
                    s.kg = s.kg.or(Some(a));
                    i += 2;
                } else if let Some(per) = if is_word(at(i + 1), MINUTES) {
                    Some(60.0)
                } else if is_word(at(i + 1), HOURS) {
                    Some(3600.0)
                } else if is_word(at(i + 1), SECONDS) {
                    Some(1.0)
                } else {
                    None
                } {
                    if s.seconds.is_none() || time_end == Some(i) {
                        s.seconds = Some(s.seconds.unwrap_or(0.0) + a * per);
                        time_end = Some(i + 2);
                    }
                    i += 2;
                } else if is_word(at(i + 1), KM) {
                    s.metres = s.metres.or(Some(a * 1000.0));
                    i += 2;
                } else if is_word(at(i + 1), METRES) {
                    s.metres = s.metres.or(Some(a));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    s
}

/// A line's part, from the word before its colon.
fn part_of(label: &str) -> Option<(Part, &'static str)> {
    let l: String = label.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    Some(match l.as_str() {
        "warmup" | "warm" => (Part::Warmup, "Warm-up"),
        "work" | "main" | "interval" | "intervals" | "run" | "effort" => (Part::Work, "Work"),
        "recovery" | "recover" | "easy" => (Part::Recover, "Recovery"),
        "rest" => (Part::Rest, "Rest"),
        "cooldown" | "cool" => (Part::Cooldown, "Cool-down"),
        _ => return None,
    })
}

/// The body's step lines: `- Part: words` (or `* `, or `1. `). Anything else is the person's notes.
fn step_lines(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")).or_else(|| {
                let d = l.find(|c: char| !c.is_ascii_digit())?;
                (d > 0).then(|| l[d..].strip_prefix(". "))?
            })?;
            let rest = rest.trim();
            (!rest.is_empty()).then(|| rest.to_string())
        })
        .collect()
}

fn sport_of(o: &Object) -> Sport {
    match text(o, "sport").as_str() {
        "run" => Sport::Running,
        "ride" => Sport::Cycling,
        "swim" => Sport::Swimming,
        "walk" => Sport::Walking,
        "row" => Sport::Rowing,
        "strength" => Sport::Strength,
        _ if text(o, "counts_as") == "strength" => Sport::Strength,
        _ => Sport::Other,
    }
}

fn build(said: &Said, part: Part, name: &str, words: &str) -> Step {
    let length = match (said.reps, said.seconds, said.metres) {
        (Some(n), _, _) => Length::Reps(n),
        (_, Some(t), _) => Length::Seconds(t),
        (_, _, Some(d)) => Length::Metres(d),
        _ => Length::Open,
    };
    Step {
        part,
        name: name.to_string(),
        notes: words.to_string(),
        length,
        target: said.target.unwrap_or(Target::Open),
        weight_kg: said.kg,
    }
}

/// The workout a planned entry describes, or one sentence saying why it cannot be one.
pub fn workout_of(o: &Object, created: i64) -> Result<(Workout, Vec<String>), String> {
    let planned = !matches!(
        o.get("planned"),
        PropertyValue::Null | PropertyValue::Int(0) | PropertyValue::Bool(false)
    ) && text(o, "log") == "workout";
    if !planned {
        return Err("Only a planned training session can be saved as a workout for a watch.".into());
    }
    let title = o.title.clone().unwrap_or_default();
    let mut notes = Vec::new();
    let mut percent = false;
    // The session's own target, for work steps whose line names none.
    let session_target = read(&text(o, "target"));
    percent |= session_target.percent;

    let lines = step_lines(&o.body);
    let mut items: Vec<Item> = Vec::new();
    if lines.is_empty() {
        // No steps written: one step, the session's own minutes or distance, if it has them.
        let said = Said {
            seconds: number(o, "minutes").map(|m| m * 60.0),
            metres: number(o, "distance_m"),
            target: session_target.target,
            ..Default::default()
        };
        let words = text(o, "said");
        items.push(Item::Step(build(
            &said,
            Part::Work,
            if title.is_empty() { "Work" } else { &title },
            &words,
        )));
    } else {
        let mut k = 0;
        while k < lines.len() {
            let line = &lines[k];
            let (part, name, words) = match line.split_once(':') {
                Some((label, words)) => match part_of(label) {
                    Some((p, n)) => (p, n.to_string(), words.trim()),
                    None => (Part::Work, label.trim().to_string(), words.trim()),
                },
                None => (Part::Work, "Work".to_string(), line.as_str()),
            };
            let mut said = read(words);
            percent |= said.percent;
            if part == Part::Work && said.target.is_none() {
                said.target = session_target.target;
            }
            let step = build(&said, part, &name, words);
            k += 1;
            match said.times {
                Some(times) => {
                    // A repeat takes its line and the recovery or rest lines straight after it.
                    let mut group = vec![step];
                    while k < lines.len() {
                        let next = &lines[k];
                        let Some((label, words)) = next.split_once(':') else { break };
                        let Some((p, n)) =
                            part_of(label).filter(|(p, _)| matches!(p, Part::Recover | Part::Rest))
                        else {
                            break;
                        };
                        let s = read(words.trim());
                        if s.times.is_some() {
                            break;
                        }
                        percent |= s.percent;
                        group.push(build(&s, p, n, words.trim()));
                        k += 1;
                    }
                    items.push(Item::Repeat { times, steps: group });
                }
                None => items.push(Item::Step(step)),
            }
        }
    }

    let steps: Vec<&Step> = items
        .iter()
        .flat_map(|i| match i {
            Item::Step(s) => vec![s],
            Item::Repeat { steps, .. } => steps.iter().collect(),
        })
        .collect();
    if steps.iter().all(|s| s.length == Length::Open) {
        notes.push(
            "No step has a time, distance or count yet, so each lasts until you press the lap \
             key. Write the numbers into the session's steps (\"Work: 4 x 800 m at 4:30/km\") for \
             the watch to time them."
                .into(),
        );
    }
    if percent {
        notes.push(
            "A percentage is not sent as a target: the watch would read it against its own \
             settings. Write a heart rate in bpm or a pace for the watch to follow."
                .into(),
        );
    }
    let name = if title.is_empty() { "Workout".to_string() } else { title };
    Ok((Workout { name, sport: sport_of(o), items, created }, notes))
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(40).collect()
}

/// The workout file for a planned entry.
pub fn for_note(o: &Object) -> Result<WorkoutFile, String> {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let (w, notes) = workout_of(o, now)?;
    let bytes = fit_write::encode(&w)?;
    let open = w.items.iter().all(|i| match i {
        Item::Step(s) => s.length == Length::Open,
        Item::Repeat { steps, .. } => steps.iter().all(|s| s.length == Length::Open),
    });
    let steps = w
        .items
        .iter()
        .map(|i| match i {
            Item::Step(_) => 1,
            Item::Repeat { steps, .. } => steps.len() + 1,
        })
        .sum();
    let day: String = o.start.map(|s| s.to_string()).unwrap_or_default().chars().take(10).collect();
    let stem =
        [day, slug(&w.name)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    Ok(WorkoutFile {
        name: format!("{}.fit", if stem.is_empty() { "workout".into() } else { stem }),
        data: crate::wire::b64_encode(&bytes),
        steps,
        open,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planned(body: &str, props: &[(&str, PropertyValue)]) -> Object {
        let mut o = Object::new(fm_model::Kind::Note, body);
        o.title = Some("Intervals · Run".into());
        o.start = "2031-03-14".parse().ok();
        o.extra.insert("log".into(), PropertyValue::Text("workout".into()));
        o.extra.insert("planned".into(), PropertyValue::Int(1));
        o.extra.insert("sport".into(), PropertyValue::Text("run".into()));
        for (k, v) in props {
            o.extra.insert(k.to_string(), v.clone());
        }
        o
    }

    #[test]
    fn a_line_says_what_it_says() {
        let s = read("4 x 800 m at 4:30-4:15/km");
        assert_eq!(s.times, Some(4));
        assert_eq!(s.metres, Some(800.0));
        let Some(Target::Speed { low, high }) = s.target else { panic!("{s:?}") };
        assert!((low - 1000.0 / 270.0).abs() < 1e-9 && (high - 1000.0 / 255.0).abs() < 1e-9);
        assert_eq!(read("10 min easy").seconds, Some(600.0));
        assert_eq!(read("1 h 30 min").seconds, Some(5400.0));
        assert_eq!(read("1 min run, 2 min walk").seconds, Some(60.0));
        assert_eq!(
            read("2 min under 120-135 bpm").target,
            Some(Target::HeartRate { low: 120, high: 135 })
        );
        assert_eq!(read("5 km").metres, Some(5000.0));
        assert_eq!(
            read("Run at 25-28 km/h").target,
            Some(Target::Speed { low: 25.0 / 3.6, high: 28.0 / 3.6 })
        );
        let st = read("3 sets of 8 reps at 42,5 kg");
        assert_eq!((st.times, st.reps, st.kg), (Some(3), Some(8), Some(42.5)));
        let p = read("20 min at 75 % of maximum heart rate");
        assert!(p.percent && p.target.is_none(), "{p:?}");
        assert_eq!(p.seconds, Some(1200.0));
        assert_eq!(read("A warm-up walk, as the page says"), Said::default());
    }

    #[test]
    fn steps_with_numbers_become_timed_steps_and_a_repeat() {
        let o = planned(
            "- Warm-up: 10 min easy\n- Work: 4 x 800 m at 4:30/km\n- Recovery: 2 min walk\n- Cool-down: easy\n",
            &[],
        );
        let (w, notes) = workout_of(&o, 1_900_000_000).unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(w.items.len(), 3);
        let Item::Repeat { times, steps } = &w.items[1] else { panic!("{:?}", w.items) };
        assert_eq!(*times, 4);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[1].part, Part::Recover);
        assert_eq!(steps[1].length, Length::Seconds(120.0));
        let Item::Step(cool) = &w.items[2] else { panic!() };
        assert_eq!(cool.length, Length::Open);
        let f = for_note(&o).unwrap();
        assert_eq!(f.name, "2031-03-14-intervals-run.fit");
        assert_eq!((f.steps, f.open), (5, false));
    }

    #[test]
    fn a_session_without_numbers_is_open_steps_with_its_words() {
        let o = planned(
            "- Warm-up: A warm-up walk, as the page says\n- Work: Run and walk in turn\n- Cool-down: A cool-down walk\n",
            &[],
        );
        let f = for_note(&o).unwrap();
        assert!(f.open);
        assert_eq!(f.steps, 3);
        assert!(f.notes[0].contains("lap key"));
        let (w, _) = workout_of(&o, 1_900_000_000).unwrap();
        let Item::Step(s) = &w.items[1] else { panic!() };
        assert_eq!(s.notes, "Run and walk in turn");
        assert_eq!(s.target, Target::Open);
    }

    #[test]
    fn no_steps_written_uses_the_session_minutes_and_target() {
        let o = planned(
            "",
            &[
                ("minutes", PropertyValue::Int(40)),
                ("target", PropertyValue::Text("140-150 bpm".into())),
            ],
        );
        let (w, _) = workout_of(&o, 1_900_000_000).unwrap();
        let Item::Step(s) = &w.items[0] else { panic!() };
        assert_eq!(s.length, Length::Seconds(2400.0));
        assert_eq!(s.target, Target::HeartRate { low: 140, high: 150 });
    }

    #[test]
    fn a_percentage_is_said_and_never_sent() {
        let o = planned("- Work: 20 min at 80 % effort\n", &[]);
        let f = for_note(&o).unwrap();
        assert!(f.notes.iter().any(|n| n.contains("percentage")), "{:?}", f.notes);
        let bytes = crate::wire::b64_decode(&f.data).unwrap();
        let back = fm_core::fit::decode(&bytes).unwrap();
        assert!(back.steps.iter().all(|s| s.target_type.as_deref() == Some("open")));
    }

    #[test]
    fn only_a_planned_session() {
        let mut o = planned("", &[]);
        o.extra.remove("planned");
        assert!(for_note(&o).unwrap_err().contains("planned"));
    }
}
