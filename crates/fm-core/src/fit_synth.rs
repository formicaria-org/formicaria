//! **A test-only FIT writer**: synthetic activity files with invented values, built here so no
//! real person's file is ever needed or committed (the coach plan's T4, T19).
//!
//! Written from the FIT protocol's published description — header, definition and data messages,
//! little-endian, the protocol's CRC-16 — and deliberately **not** with `rustyfit`, the crate that
//! reads them back: a writer and a reader that share code share their mistakes. It carries its own
//! copy of the CRC for the same reason.
//!
//! Included by `fit_tests.rs` (fm-core) and by `crates/fm-app/tests/import_fit.rs` with `#[path]`;
//! it is never compiled into the app. Every serial number here is invented.

#![allow(dead_code)]

/// A field value, by base type.
#[derive(Clone, Debug)]
pub enum V {
    Enum(u8),
    U8(u8),
    U16(u16),
    U32(u32),
    U32z(u32),
    S32(i32),
    Str(String),
    U16s(Vec<u16>),
}

impl V {
    fn base(&self) -> u8 {
        match self {
            V::Enum(_) => 0x00,
            V::U8(_) => 0x02,
            V::U16(_) | V::U16s(_) => 0x84,
            V::U32(_) => 0x86,
            V::U32z(_) => 0x8C,
            V::S32(_) => 0x85,
            V::Str(_) => 0x07,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        match self {
            V::Enum(n) | V::U8(n) => vec![*n],
            V::U16(n) => n.to_le_bytes().to_vec(),
            V::U32(n) | V::U32z(n) => n.to_le_bytes().to_vec(),
            V::S32(n) => n.to_le_bytes().to_vec(),
            V::Str(s) => {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                b
            }
            V::U16s(v) => v.iter().flat_map(|n| n.to_le_bytes()).collect(),
        }
    }
}

const CRC_TABLE: [u16; 16] = [
    0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800, 0xB401,
    0x5000, 0x9C01, 0x8801, 0x4400,
];

pub fn crc(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in bytes {
        let mut tmp = CRC_TABLE[(crc & 0xF) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc = crc ^ tmp ^ CRC_TABLE[(b & 0xF) as usize];
        tmp = CRC_TABLE[(crc & 0xF) as usize];
        crc = (crc >> 4) & 0x0FFF;
        crc = crc ^ tmp ^ CRC_TABLE[((b >> 4) & 0xF) as usize];
    }
    crc
}

/// Messages, written one after another: each with its own definition (local message 0), which
/// the protocol allows and keeps the writer simple.
#[derive(Default)]
pub struct Writer {
    body: Vec<u8>,
}

impl Writer {
    pub fn msg(&mut self, global: u16, fields: &[(u8, V)]) -> &mut Self {
        // Definition: header 0x40 (local 0), reserved, little-endian, global number, field count.
        self.body.extend_from_slice(&[0x40, 0, 0]);
        self.body.extend_from_slice(&global.to_le_bytes());
        self.body.push(fields.len() as u8);
        for (num, v) in fields {
            self.body.extend_from_slice(&[*num, v.bytes().len() as u8, v.base()]);
        }
        // Data: header 0x00 (local 0), then the values in the defined order.
        self.body.push(0);
        for (_, v) in fields {
            self.body.extend(v.bytes());
        }
        self
    }

    /// The file: a 14-byte header with its own checksum, the messages, the file's checksum.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = vec![14u8, 0x20];
        out.extend_from_slice(&2193u16.to_le_bytes()); // profile version 21.93
        out.extend_from_slice(&(self.body.len() as u32).to_le_bytes());
        out.extend_from_slice(b".FIT");
        let h = crc(&out);
        out.extend_from_slice(&h.to_le_bytes());
        out.extend_from_slice(&self.body);
        let c = crc(&out);
        out.extend_from_slice(&c.to_le_bytes());
        out
    }
}

/// Seconds since the FIT epoch (1989-12-31T00:00:00Z) of a UTC civil time.
pub fn fit_time(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> u32 {
    // Howard Hinnant's days-from-civil.
    let y = i64::from(if m <= 2 { y - 1 } else { y });
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let unix = days * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60 + i64::from(s);
    (unix - 631_065_600) as u32
}

#[derive(Clone, Debug, Default)]
pub struct LapSpec {
    pub timer_s: u32,
    pub distance_m: u32,
    pub hr_avg: u8,
    pub hr_max: u8,
    pub wkt_step_index: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct LengthSpec {
    pub timer_s: u32,
    pub strokes: u16,
    /// 0 freestyle, 1 backstroke, 2 breaststroke, 3 butterfly, 4 drill.
    pub stroke: u8,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct SetSpec {
    pub duration_s: u32,
    pub reps: u16,
    /// Kilograms × 16, as the protocol stores them.
    pub weight_x16: u16,
    pub active: bool,
    /// Exercise category and the exercise within it, as numbers.
    pub category: u16,
    pub name: u16,
}

/// One activity, described.
#[derive(Clone, Debug)]
pub struct Spec {
    pub serial: u32,
    pub manufacturer: u16,
    pub sport: u8,
    pub sub_sport: u8,
    /// FIT seconds, UTC.
    pub start: u32,
    /// The watch's offset from UTC, in seconds.
    pub offset_s: Option<i32>,
    pub laps: Vec<LapSpec>,
    /// Pause time added to elapsed beyond the timer.
    pub paused_s: u32,
    pub distance_m: Option<u32>,
    pub hr: Option<(u8, u8)>,
    pub ascent_m: Option<u16>,
    pub descent_m: Option<u16>,
    pub cadence: Option<u8>,
    /// `workout_rpe` as stored, ×10.
    pub rpe_x10: Option<u8>,
    pub training_effect_x10: Option<u8>,
    pub calories: Option<u16>,
    pub pool_length_cm: Option<u16>,
    pub lengths: Vec<LengthSpec>,
    pub sets: Vec<SetSpec>,
    pub steps: Vec<(String, u32)>,
    /// Per-second-ish records: one every ten seconds of the timer.
    pub records: bool,
    pub with_position: bool,
    pub unknown_message: bool,
    pub creator: bool,
    /// Override `time_created` (FIT seconds); default the start.
    pub time_created: Option<u32>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            serial: 3_000_000_001,
            manufacturer: 1,
            sport: 1,
            sub_sport: 0,
            start: fit_time(2026, 3, 14, 7, 0, 0),
            offset_s: Some(3600),
            laps: vec![],
            paused_s: 0,
            distance_m: None,
            hr: None,
            ascent_m: None,
            descent_m: None,
            cadence: None,
            rpe_x10: None,
            training_effect_x10: None,
            calories: None,
            pool_length_cm: None,
            lengths: vec![],
            sets: vec![],
            steps: vec![],
            records: false,
            with_position: false,
            unknown_message: false,
            creator: true,
            time_created: None,
        }
    }
}

impl Spec {
    pub fn timer_s(&self) -> u32 {
        let laps: u32 = self.laps.iter().map(|l| l.timer_s).sum();
        let lengths: u32 = self.lengths.iter().map(|l| l.timer_s).sum();
        let sets: u32 = self.sets.iter().map(|s| s.duration_s).sum();
        laps.max(lengths).max(sets).max(60)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        let timer = self.timer_s();
        let elapsed = timer + self.paused_s;
        let end = self.start + elapsed;
        w.msg(
            0,
            &[
                (0, V::Enum(4)),
                (1, V::U16(self.manufacturer)),
                (2, V::U16(4242)),
                (3, V::U32z(self.serial)),
                (4, V::U32(self.time_created.unwrap_or(self.start))),
            ],
        );
        if self.creator {
            w.msg(49, &[(0, V::U16(1234))]);
        }
        if self.unknown_message {
            // A manufacturer's own message number: no profile names it.
            w.msg(0xFF10, &[(0, V::U32(7)), (1, V::U8(9))]);
        }
        for (i, (name, secs)) in self.steps.iter().enumerate() {
            w.msg(
                27,
                &[
                    (254, V::U16(i as u16)),
                    (0, V::Str(name.clone())),
                    (1, V::Enum(0)),
                    (2, V::U32(secs * 1000)),
                    (7, V::Enum(if i % 2 == 0 { 0 } else { 1 })),
                ],
            );
        }
        if self.records {
            let mut t = self.start;
            let mut i = 0u32;
            while t < self.start + timer {
                let mut f = vec![(253, V::U32(t))];
                if let Some((avg, _)) = self.hr {
                    f.push((3, V::U8(avg)));
                }
                if self.with_position {
                    f.push((0, V::S32(600_000_000 + i as i32)));
                    f.push((1, V::S32(100_000_000 + i as i32)));
                    f.push((2, V::U16(2500 + 5 * (i as u16 % 40))));
                }
                w.msg(20, &f);
                t += 10;
                i += 1;
            }
        }
        let mut t = self.start;
        for (i, l) in self.lengths.iter().enumerate() {
            w.msg(
                101,
                &[
                    (253, V::U32(t + l.timer_s)),
                    (254, V::U16(i as u16)),
                    (2, V::U32(t)),
                    (3, V::U32(l.timer_s * 1000)),
                    (4, V::U32(l.timer_s * 1000)),
                    (5, V::U16(l.strokes)),
                    (7, V::Enum(l.stroke)),
                    (9, V::U8(if l.active { 30 } else { 0 })),
                    (12, V::Enum(u8::from(l.active))),
                ],
            );
            t += l.timer_s;
        }
        let mut t = self.start;
        for (i, s) in self.sets.iter().enumerate() {
            let mut f = vec![
                (254, V::U32(t + s.duration_s)),
                (0, V::U32(s.duration_s * 1000)),
                (5, V::U8(u8::from(s.active))),
                (6, V::U32(t)),
                (10, V::U16(i as u16)),
            ];
            if s.active {
                f.push((3, V::U16(s.reps)));
                f.push((4, V::U16(s.weight_x16)));
                f.push((7, V::U16s(vec![s.category])));
                f.push((8, V::U16s(vec![s.name])));
            }
            w.msg(225, &f);
            t += s.duration_s;
        }
        let mut t = self.start;
        for (i, l) in self.laps.iter().enumerate() {
            let mut f = vec![
                (253, V::U32(t + l.timer_s)),
                (254, V::U16(i as u16)),
                (2, V::U32(t)),
                (7, V::U32(l.timer_s * 1000)),
                (8, V::U32(l.timer_s * 1000)),
                (9, V::U32(l.distance_m * 100)),
                (15, V::U8(l.hr_avg)),
                (16, V::U8(l.hr_max)),
            ];
            if let Some(mm_s) = (l.distance_m * 1000).checked_div(l.timer_s) {
                f.push((13, V::U16(mm_s as u16)));
            }
            if let Some(k) = l.wkt_step_index {
                f.push((71, V::U16(k)));
            }
            w.msg(19, &f);
            t += l.timer_s;
        }
        let distance =
            self.distance_m.unwrap_or_else(|| self.laps.iter().map(|l| l.distance_m).sum());
        let mut s = vec![
            (253, V::U32(end)),
            (2, V::U32(self.start)),
            (5, V::Enum(self.sport)),
            (6, V::Enum(self.sub_sport)),
            (7, V::U32(elapsed * 1000)),
            (8, V::U32(timer * 1000)),
        ];
        if distance > 0 {
            s.push((9, V::U32(distance * 100)));
        }
        if let Some((avg, max)) = self.hr {
            s.push((16, V::U8(avg)));
            s.push((17, V::U8(max)));
        }
        if let Some(c) = self.cadence {
            s.push((18, V::U8(c)));
        }
        if let Some(a) = self.ascent_m {
            s.push((22, V::U16(a)));
        }
        if let Some(d) = self.descent_m {
            s.push((23, V::U16(d)));
        }
        if let Some(te) = self.training_effect_x10 {
            s.push((24, V::U8(te)));
        }
        if let Some(c) = self.calories {
            s.push((11, V::U16(c)));
        }
        if let Some(p) = self.pool_length_cm {
            s.push((44, V::U16(p)));
            let active = self.lengths.iter().filter(|l| l.active).count() as u16;
            s.push((47, V::U16(active)));
        }
        if let Some(r) = self.rpe_x10 {
            s.push((193, V::U8(r)));
        }
        w.msg(18, &s);
        let mut a =
            vec![(253, V::U32(end)), (0, V::U32(timer * 1000)), (1, V::U16(1)), (2, V::Enum(0))];
        if let Some(off) = self.offset_s {
            a.push((5, V::U32((end as i64 + off as i64) as u32)));
        }
        w.msg(34, &a);
        w.bytes()
    }
}

/// A steady run: three laps of a kilometre, heart rate, a climb, a typed effort.
pub fn steady_run() -> Spec {
    Spec {
        laps: vec![
            LapSpec {
                timer_s: 330,
                distance_m: 1000,
                hr_avg: 140,
                hr_max: 150,
                ..Default::default()
            },
            LapSpec {
                timer_s: 325,
                distance_m: 1000,
                hr_avg: 145,
                hr_max: 155,
                ..Default::default()
            },
            LapSpec {
                timer_s: 320,
                distance_m: 1000,
                hr_avg: 150,
                hr_max: 160,
                ..Default::default()
            },
        ],
        hr: Some((145, 160)),
        ascent_m: Some(42),
        descent_m: Some(40),
        cadence: Some(84),
        rpe_x10: Some(50),
        training_effect_x10: Some(32),
        calories: Some(310),
        records: true,
        with_position: true,
        ..Default::default()
    }
}

/// Intervals that followed a workout: warm-up, two hard reps with an easy one between, cool-down.
pub fn intervals() -> Spec {
    Spec {
        serial: 3_000_000_002,
        start: fit_time(2026, 3, 16, 17, 30, 0),
        laps: vec![
            LapSpec {
                timer_s: 600,
                distance_m: 1700,
                hr_avg: 130,
                hr_max: 140,
                wkt_step_index: Some(0),
            },
            LapSpec {
                timer_s: 240,
                distance_m: 1000,
                hr_avg: 170,
                hr_max: 178,
                wkt_step_index: Some(1),
            },
            LapSpec {
                timer_s: 120,
                distance_m: 300,
                hr_avg: 140,
                hr_max: 165,
                wkt_step_index: Some(2),
            },
            LapSpec {
                timer_s: 240,
                distance_m: 1010,
                hr_avg: 172,
                hr_max: 180,
                wkt_step_index: Some(3),
            },
            LapSpec {
                timer_s: 600,
                distance_m: 1600,
                hr_avg: 128,
                hr_max: 140,
                wkt_step_index: Some(4),
            },
        ],
        steps: vec![
            ("Warm up".into(), 600),
            ("Hard".into(), 240),
            ("Easy".into(), 120),
            ("Hard".into(), 240),
            ("Cool down".into(), 600),
        ],
        hr: Some((150, 180)),
        ..Default::default()
    }
}

/// A run that starts at 23:50 on the watch's clock and ends after midnight.
pub fn over_midnight(offset_s: i32) -> Spec {
    // 23:50 local is 23:50 − offset in UTC.
    let local = i64::from(fit_time(2026, 6, 20, 23, 50, 0));
    Spec {
        serial: 3_000_000_003,
        start: (local - i64::from(offset_s)) as u32,
        offset_s: Some(offset_s),
        laps: vec![LapSpec {
            timer_s: 1800,
            distance_m: 5000,
            hr_avg: 140,
            hr_max: 150,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// A pool swim, 25 m: 4 lengths of freestyle, a rest, a medley interval (four strokes), a rest,
/// 2 lengths of drill. The watch's heart rate is there, as a wrist reads it in water.
pub fn pool_swim() -> Spec {
    let len = |stroke, timer_s, strokes| LengthSpec { timer_s, strokes, stroke, active: true };
    let rest = |timer_s| LengthSpec { timer_s, strokes: 0, stroke: 0, active: false };
    Spec {
        serial: 3_000_000_004,
        sport: 5,
        sub_sport: 17,
        start: fit_time(2026, 3, 18, 6, 30, 0),
        pool_length_cm: Some(2500),
        lengths: vec![
            len(0, 25, 18),
            len(0, 26, 18),
            len(0, 26, 19),
            len(0, 27, 19),
            rest(30),
            len(3, 30, 16),
            len(1, 32, 20),
            len(2, 34, 14),
            len(0, 27, 19),
            rest(45),
            len(4, 40, 0),
            len(4, 41, 0),
        ],
        hr: Some((128, 150)),
        ..Default::default()
    }
}

/// Strength: three active sets (two of a mapped exercise, one the app has no card for) with rests
/// between them.
pub fn strength() -> Spec {
    // bench_press (0) / barbell_bench_press (1); squat (28) / goblet_squat (37) is not mapped.
    let work = |category, name, reps, kg_x16| SetSpec {
        duration_s: 40,
        reps,
        weight_x16: kg_x16,
        active: true,
        category,
        name,
    };
    let rest =
        || SetSpec { duration_s: 90, reps: 0, weight_x16: 0, active: false, category: 0, name: 0 };
    Spec {
        serial: 3_000_000_005,
        sport: 10,
        sub_sport: 20,
        start: fit_time(2026, 3, 19, 18, 0, 0),
        sets: vec![
            work(0, 1, 8, 60 * 16),
            rest(),
            work(0, 1, 7, 60 * 16),
            rest(),
            work(28, 37, 12, 20 * 16),
            rest(),
        ],
        hr: Some((110, 140)),
        ..Default::default()
    }
}
