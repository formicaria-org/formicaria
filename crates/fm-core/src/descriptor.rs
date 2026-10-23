//! `<vault>/vault.json` — the vault's own description of itself.
//!
//! **Bounded by one rule: every field must be a fact git cannot supply.** That rule is what
//! keeps this from becoming the config file that eats the project. Git already knows the
//! authorship and the history (`git log`), the audience (`git remote`, plus who can clone),
//! and — neatly — `.gitignore` is *already* a truth-versus-cache declaration. So there is no
//! `author`, no `collaborators`, no `remote`, no `created`. Adding one later means answering
//! why git's answer was wrong, which is a bar worth having to clear.
//!
//! Three facts survive it:
//!
//! - **`name`** — the audience label. A vault list entry can already carry one, but a repo
//!   you clone has nobody to type it, and the directory name is whatever git chose.
//! - **`description`** — what this vault is for, in the vault, so it travels.
//! - **`notes`** — where the notes are, relative to the vault root. The reason this exists at
//!   all: adopting a repo you already own means the notes are in `docs/` or `notes/` or
//!   nowhere in particular, and demanding they move is demanding you restructure your project
//!   to suit a notebook.
//!
//! **Every field is optional, and an absent file is exactly today's behaviour** — that is the
//! property that makes this safe to add to a format people already have on disk.
//!
//! **Rejected:** a descriptor declaring *two* locations (read from here, write to there).
//! It reads as flexibility and behaves as a trapdoor: the first edit silently relocates a
//! note, and "where is my file" is the one question a files-as-truth notebook must never
//! make hard to answer.

use crate::StoreError;
use std::path::{Path, PathBuf};

/// **Whether this vault's review record may be collected, and whether it may be published.**
///
/// Two separate answers, because the first does not imply the second. That distinction is not
/// pedantry: it is what froze the only comparable open corpus of AI corrections, whose contributors
/// had agreed to collection and training but never to public release.
///
/// Here rather than in per-device Settings, for exactly the reason [`Descriptor::git_assets_max`]
/// is: publishing relicenses the vault's **content**, which is shared and permanent, so the vault
/// decides — not the loosest machine that happens to hold a clone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Supervision {
    /// Record what a human did with an AI proposal. On by default: it is the user's own history in
    /// their own repository and nothing leaves the machine — but it can be turned off.
    pub collect: bool,
    /// Allow that record to be exported and published under an open licence. **Off by default and
    /// never inferred**, because publication cannot be recalled.
    pub publish: bool,
}

impl Default for Supervision {
    fn default() -> Self {
        Supervision { collect: true, publish: false }
    }
}

/// What `vault.json` can say. Absent file, absent field and empty string are all "no
/// opinion" — the caller keeps its default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Descriptor {
    pub name: Option<String>,
    pub description: Option<String>,
    /// Where the notes live, relative to the vault root. `None` means `notes/`.
    pub notes: Option<PathBuf>,
    /// **The largest attachment this vault will commit to git**, in bytes. `None` — the default —
    /// means *none of them*: notes travel, media does not, which is the two-tier split the whole
    /// backup design rests on.
    ///
    /// It lives here, in the vault's own file, rather than in per-device Settings, because it
    /// decides what enters **shared, permanent history**. A per-device setting would let the
    /// loosest machine decide for everyone, and git history cannot be un-decided: a 50 MB video
    /// committed once is in every clone forever, and removing it means rewriting history that
    /// collaborators have already pulled.
    ///
    /// Written as a number of bytes or a human string (`"2MB"`), because this is a file people
    /// edit by hand.
    pub git_assets_max: Option<u64>,

    /// **Hard guardrails on a proposal's size** (see [`crate::proposal`]). Here, not in per-device
    /// Settings, for the same reason as `git_assets_max`: a proposal enters shared history, so the
    /// vault — not the loosest device — bounds how large a single proposal, and how many at once,
    /// it will hold. Absent config is the conservative built-in [default](crate::proposal::ProposalLimits::default).
    pub proposal_limits: crate::proposal::ProposalLimits,

    /// **What may be done with this vault's review record** — see [`Supervision`]. Absent config is
    /// the conservative default: collect locally, publish nothing.
    pub supervision: Supervision,

    /// **The clock this vault's times are written on, when the vault says so** (`notes_time_zone`:
    /// `Asia/Tokyo`, or an offset such as `+09:00`) — `decisions.md` 2026-10-23, *the vault's
    /// clock is read from its first commit*. A note's `start: 2026-10-27T17:00` is a wall clock
    /// with no zone in it (`Stamp`'s ruling), so something must say which wall. Stated here it
    /// wins; absent — the usual case — every device reads the same answer from the vault's first
    /// commit ([`crate::vcs::founding_offset`]), and nothing is written. Set it by hand for a vault
    /// begun where clocks change in summer, so its times follow daylight saving.
    pub notes_time_zone: Option<String>,
}

/// The largest attachment this app will ever put into git, whatever a vault asks for.
///
/// **There is no git-lfs here, and that is the whole reason this number exists.** Every
/// attachment under a vault's `git_assets_max` is stored as a full blob in permanent history:
/// it is in every clone forever, it cannot be taken back without rewriting history others have
/// pulled, and it is paid for again on every fresh clone. Without LFS there is no pointer to
/// hide that cost behind, so the honest move is a bound.
///
/// **100 MB decimal, deliberately a little stricter than GitHub's 100 MiB** (104,857,600). A
/// file this app accepts should never be one the host refuses over a rounding difference, so
/// the ceiling sits just inside the wall rather than exactly on it. Above it, a push does not
/// merely bloat the repository — it *fails*, and it fails after the commit is already made.
///
/// A self-hosted remote may well accept more, and there is deliberately no way to say so: one
/// documented constant is clearer than a second key in a file format for one audience, and the
/// restic tier already carries attachments of any size.
pub const GIT_ASSETS_CEILING: u64 = 100_000_000;

/// What a vault's `git_assets_max` actually means once [`GIT_ASSETS_CEILING`] is applied.
///
/// A function, not an inlined `min`, for two reasons: exactly one expression defines the rule,
/// and the clamp is testable without writing a 100 MB file to disk. A descriptor asking for more
/// than the ceiling is **not** an error — it may have been written by a hand, by another machine,
/// or by a version that had no ceiling, and a value that was legal when written must not make a
/// vault unopenable. It is simply not honoured.
pub fn effective_git_assets_max(max: u64) -> u64 {
    max.min(GIT_ASSETS_CEILING)
}

/// Parse a size a human would write: `2MB`, `500 kb`, `1.5 GiB`, or plain bytes.
///
/// Decimal units (MB = 10^6) rather than binary, because that is what a file manager shows and
/// this number exists to be compared against what someone sees next to their photo. `MiB`/`GiB`
/// are accepted and mean the binary thing, for anyone who wants to be exact.
pub fn parse_size(s: &str) -> Option<u64> {
    let t = s.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }
    let split = t.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(t.len());
    let (num, unit) = t.split_at(split);
    let n: f64 = num.trim().parse().ok()?;
    if n < 0.0 {
        return None;
    }
    let mult: f64 = match unit.trim() {
        "" | "b" => 1.0,
        "k" | "kb" => 1e3,
        "m" | "mb" => 1e6,
        "g" | "gb" => 1e9,
        "kib" => 1024.0,
        "mib" => 1024.0 * 1024.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((n * mult) as u64)
}

/// A byte count as the shortest string a person would write, round-tripping through [`parse_size`].
pub fn format_size(bytes: u64) -> String {
    for (unit, mult) in [("GB", 1e9), ("MB", 1e6), ("kB", 1e3)] {
        let v = bytes as f64 / mult;
        if v >= 1.0 {
            return if (v.fract()).abs() < 0.05 {
                format!("{}{unit}", v.round() as u64)
            } else {
                format!("{v:.1}{unit}")
            };
        }
    }
    format!("{bytes}B")
}

/// **Where a vault keeps its notes, without needing its descriptor first** — read it, fall back to
/// `notes/`.
///
/// [`Descriptor::notes_dir`] is the rule; this is the *wrapper around a possibly-absent descriptor*,
/// which four call sites had each written for themselves — and one of them (`recoverable_vaults`,
/// 2026-09-08) skipped the descriptor entirely and hardcoded `notes/`, so a vault that keeps its
/// notes elsewhere was invisible to the one screen that offers a vault back.
///
/// Absent and unparseable both fall back, deliberately: a vault whose `vault.json` has a typo still
/// has notes, and this is asked by surfaces that must render rather than fail.
pub fn notes_dir_of(root: &std::path::Path) -> PathBuf {
    Descriptor::read(root).map(|d| d.notes_dir(root)).unwrap_or_else(|_| root.join("notes"))
}

/// How many notes are in that directory. Three call sites counted `*.md` for themselves, and two of
/// them disagreed about *which* directory to count.
pub fn note_count(root: &std::path::Path) -> usize {
    std::fs::read_dir(notes_dir_of(root))
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0)
}

/// **What a vault is called: caller > descriptor > directory.** One function, because it is asked in
/// two places that must not drift — `FileStore::prepare`, which names the open store, and
/// `fm_app::dispatch::infos`, which names the entry in the vault list. Those two answering
/// differently is a vault that appears under one name in the switcher and another everywhere else.
///
/// `given` is the vault *list*'s name — the audience label this user chose — so a repo they cloned
/// never renames it out from under them. The directory is the last resort: it is whatever git
/// called the clone.
pub fn vault_name(root: &std::path::Path, desc: Option<&Descriptor>, given: &str) -> String {
    if !given.is_empty() {
        return given.to_string();
    }
    desc.and_then(|d| d.name.clone())
        .or_else(|| root.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "vault".to_string())
}

/// The key formicaria 0.15.0 stamped at start; see [`Descriptor::strip_legacy_time_zone`].
const LEGACY_TIME_ZONE: &str = "time_zone";

/// **Merge three versions of `vault.json`, key by key.** The engine behind `fm merge-vault` on a
/// desktop and the path routing in `git_native` on a phone, so the two devices write the same file.
///
/// Merged as text, two devices changing the file at once left `<<<<<<<` inside a JSON document,
/// and from then on [`Descriptor::read`] failed: proposals were refused while the app ran, and the
/// vault would not open at the next start (2026-10-23). So **this never conflicts**, for the same
/// reason `fm merge-manifest` never does: a non-zero exit from a driver leaves an unmerged path that
/// stops every commit in the vault.
///
/// - Each key takes the three-way rule: both sides agree, or the side that changed it wins.
/// - **An edit beats a deletion**, as a tag does in the note merge: losing a setting someone chose,
///   silently, is worse than keeping one someone removed.
/// - **Two different edits keep both**: the winner is chosen from content only (the larger
///   serialisation), so every device computes the same file, and the other value is kept under
///   `conflict-<key>` — the note merge's rule (`merge::DEMOTED_PREFIX`). [`Descriptor::read`]
///   ignores keys it does not know, so the vault keeps working and the other value is still there
///   for whoever opens the file.
/// - 0.15.0's `time_zone` is dropped from every side, since nothing else ever wrote it.
/// - A side that is not a JSON object is no information, not a failure; the other side stands.
///
/// The result is pretty-printed with sorted keys, so it is the same bytes on every device.
pub fn merge_texts(base: &str, ours: &str, theirs: &str) -> String {
    use serde_json::{Map, Value};
    let object = |t: &str| -> Option<Map<String, Value>> {
        let mut m = serde_json::from_str::<Value>(t).ok()?.as_object()?.clone();
        m.remove(LEGACY_TIME_ZONE);
        Some(m)
    };
    let (o, t) = match (object(ours), object(theirs)) {
        (Some(o), Some(t)) => (o, t),
        (Some(one), None) | (None, Some(one)) => return pretty(&one),
        (None, None) => return ours.to_string(),
    };
    pretty(&merge_maps(&object(base).unwrap_or_default(), &o, &t))
}

/// [`merge_texts`]'s rule over one JSON object. **A nested object both sides changed is merged the
/// same way, key by key** — `supervision` and `proposals` are objects, and one device changing
/// `supervision.collect` while another changes `supervision.publish` is two edits to two settings,
/// not one disagreement about all of them. A `conflict-<key>` from inside one stays inside it, where
/// the reader of that object ignores it like any key it does not know.
fn merge_maps(
    b: &serde_json::Map<String, serde_json::Value>,
    o: &serde_json::Map<String, serde_json::Value>,
    t: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    use serde_json::{Map, Value};
    let demoted = |k: &String| k.starts_with(crate::merge::DEMOTED_PREFIX);
    let keys: std::collections::BTreeSet<&String> =
        b.keys().chain(o.keys()).chain(t.keys()).collect();
    let mut out = Map::new();
    let mut losers: Vec<(String, Value)> = Vec::new();
    for key in keys.iter().copied().filter(|k| !demoted(k)) {
        let (bv, ov, tv) = (b.get(key), o.get(key), t.get(key));
        let kept = if ov == tv || tv == bv {
            ov
        } else if ov == bv {
            tv
        } else {
            match (ov, tv) {
                (Some(Value::Object(x)), Some(Value::Object(y))) => {
                    let under = match bv {
                        Some(Value::Object(m)) => m.clone(),
                        _ => Map::new(),
                    };
                    out.insert(key.clone(), Value::Object(merge_maps(&under, x, y)));
                    continue;
                }
                (Some(x), Some(y)) => {
                    let (sx, sy) = (x.to_string(), y.to_string());
                    let (win, lose) = if sx >= sy { (x, y) } else { (y, x) };
                    losers.push((key.clone(), lose.clone()));
                    Some(win)
                }
                (edit, None) | (None, edit) => edit,
            }
        };
        if let Some(v) = kept {
            out.insert(key.clone(), v.clone());
        }
    }

    // A `conflict-<key>` is a set of values, and merges as one: kept unless a side that had it
    // removed it, plus whatever this merge demoted, minus a value equal to the winner.
    let as_set = |m: &Map<String, Value>, k: &str| -> Vec<Value> {
        match m.get(k) {
            Some(Value::Array(a)) => a.clone(),
            Some(v) => vec![v.clone()],
            None => Vec::new(),
        }
    };
    let mut sets: std::collections::BTreeMap<String, Vec<Value>> = Default::default();
    for key in keys.iter().copied().filter(|k| demoted(k)) {
        let (bs, os, ts) = (as_set(b, key), as_set(o, key), as_set(t, key));
        let kept = os
            .iter()
            .chain(ts.iter())
            .filter(|v| !(bs.contains(v) && (!os.contains(v) || !ts.contains(v))))
            .cloned()
            .collect();
        sets.insert(key.clone(), kept);
    }
    for (key, v) in losers {
        sets.entry(format!("{}{key}", crate::merge::DEMOTED_PREFIX)).or_default().push(v);
    }
    for (key, values) in sets {
        let field = &key[crate::merge::DEMOTED_PREFIX.len()..];
        let mut unique: Vec<Value> = Vec::new();
        for v in values {
            if Some(&v) != out.get(field) && !unique.contains(&v) {
                unique.push(v);
            }
        }
        unique.sort_by_key(|v| v.to_string());
        if !unique.is_empty() {
            out.insert(key, Value::Array(unique));
        }
    }
    out
}

fn pretty(m: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut text = serde_json::to_string_pretty(m).unwrap_or_else(|_| "{}".into());
    text.push('\n');
    text
}

/// **The versions inside a file git's text merge left markers in**, as `(base, ours, theirs)`.
/// Lines outside a conflict belong to all three. A diff3-style `|||||||` section is the base; without
/// one the base has nothing for that conflict, so [`merge_texts`] keeps what either side has there.
/// `None` when a marker is out of order, which is not a file this can read.
pub fn split_conflict(text: &str) -> Option<(String, String, String)> {
    #[derive(PartialEq)]
    enum In {
        Outside,
        Ours,
        Base,
        Theirs,
    }
    let (mut base, mut ours, mut theirs) = (String::new(), String::new(), String::new());
    let mut at = In::Outside;
    for line in text.split_inclusive('\n') {
        at = match (&at, line) {
            (In::Outside, l) if l.starts_with("<<<<<<<") => In::Ours,
            (In::Ours, l) if l.starts_with("|||||||") => In::Base,
            (In::Ours | In::Base, l) if l.starts_with("=======") => In::Theirs,
            (In::Theirs, l) if l.starts_with(">>>>>>>") => In::Outside,
            (_, l)
                if ["<<<<<<<", "|||||||", "=======", ">>>>>>>"]
                    .iter()
                    .any(|m| l.starts_with(m)) =>
            {
                return None
            }
            (In::Outside, l) => {
                base.push_str(l);
                ours.push_str(l);
                theirs.push_str(l);
                continue;
            }
            (In::Ours, l) => {
                ours.push_str(l);
                continue;
            }
            (In::Base, l) => {
                base.push_str(l);
                continue;
            }
            (In::Theirs, l) => {
                theirs.push_str(l);
                continue;
            }
        };
    }
    (at == In::Outside).then_some((base, ours, theirs))
}

impl Descriptor {
    /// Read `<root>/vault.json`. **An absent file is `Ok(default)`, never an error** — the
    /// overwhelming majority of vaults will not have one, and a notebook that refused to
    /// open a folder because it lacked a config file would have missed the point entirely.
    ///
    /// A file that exists but does not parse *is* an error, and deliberately so: it was put
    /// there on purpose, so silently ignoring it would apply settings the user believes are
    /// in force. Same stance as `vaults::load`'s malformed-JSON warning.
    pub fn read(root: &Path) -> Result<Self, StoreError> {
        let path = root.join("vault.json");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(StoreError::Io(format!("{}: {e}", path.display()))),
        };
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| StoreError::Parse(format!("{} is not valid JSON: {e}", path.display())))?;

        let field = |k: &str| {
            v.get(k)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
        };

        let notes = match field("notes") {
            None => None,
            Some(rel) => {
                // **The one thing worth refusing.** `notes` is joined onto the vault root, so
                // an absolute path or a `..` would put the notes directory outside the vault
                // — which means outside the repo that defines the audience, and audience is
                // decided by *location* in this design. A vault that could point its notes
                // into another vault's tree would make "which audience is this note in"
                // unanswerable.
                let p = PathBuf::from(&rel);
                if p.is_absolute() || p.components().any(|c| c.as_os_str() == "..") {
                    return Err(StoreError::Parse(format!(
                        "{}: `notes` must stay inside the vault — {rel:?} does not",
                        path.display()
                    )));
                }
                Some(p)
            }
        };

        // Accepts `"2MB"` or a raw byte count, because both are things a person writes. A value
        // that is present but unparseable is an error for the same reason a malformed file is:
        // silently ignoring it would apply a limit the user believes is in force — and here the
        // consequence of getting it wrong is media in permanent history.
        let git_assets_max = match v.get("git_assets_max") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::Number(n)) => Some(n.as_u64().ok_or_else(|| {
                StoreError::Parse(format!(
                    "{}: `git_assets_max` must not be negative",
                    path.display()
                ))
            })?),
            Some(serde_json::Value::String(t)) => Some(parse_size(t).ok_or_else(|| {
                StoreError::Parse(format!(
                    "{}: `git_assets_max` — {t:?} is not a size (try \"2MB\")",
                    path.display()
                ))
            })?),
            Some(_) => {
                return Err(StoreError::Parse(format!(
                    "{}: `git_assets_max` must be a size like \"2MB\"",
                    path.display()
                )))
            }
        };

        // Proposal guardrails — the vault's blast-radius policy. A nested object, so the keys read
        // as one concern; any missing key keeps its conservative default, and a present-but-
        // unparseable value is an error (silently ignoring it would apply a limit the user believes
        // is in force — the same stance as `git_assets_max`).
        let proposal_limits = {
            let mut limits = crate::proposal::ProposalLimits::default();
            match v.get("proposals") {
                None | Some(serde_json::Value::Null) => {}
                Some(serde_json::Value::Object(p)) => {
                    let size = |key: &str| -> Result<Option<u64>, StoreError> {
                        match p.get(key) {
                            None | Some(serde_json::Value::Null) => Ok(None),
                            Some(serde_json::Value::Number(n)) => Ok(Some(n.as_u64().ok_or_else(|| {
                                StoreError::Parse(format!(
                                    "{}: `proposals.{key}` must not be negative",
                                    path.display()
                                ))
                            })?)),
                            Some(serde_json::Value::String(t)) => Ok(Some(parse_size(t).ok_or_else(
                                || {
                                    StoreError::Parse(format!(
                                        "{}: `proposals.{key}` — {t:?} is not a size (try \"256kB\")",
                                        path.display()
                                    ))
                                },
                            )?)),
                            Some(_) => Err(StoreError::Parse(format!(
                                "{}: `proposals.{key}` must be a size like \"256kB\"",
                                path.display()
                            ))),
                        }
                    };
                    let count = |key: &str| -> Result<Option<usize>, StoreError> {
                        match p.get(key) {
                            None | Some(serde_json::Value::Null) => Ok(None),
                            Some(serde_json::Value::Number(n)) => Ok(Some(
                                n.as_u64().and_then(|x| usize::try_from(x).ok()).ok_or_else(
                                    || {
                                        StoreError::Parse(format!(
                                        "{}: `proposals.{key}` must be a non-negative whole number",
                                        path.display()
                                    ))
                                    },
                                )?,
                            )),
                            Some(_) => Err(StoreError::Parse(format!(
                                "{}: `proposals.{key}` must be a whole number",
                                path.display()
                            ))),
                        }
                    };
                    if let Some(n) = count("max_files")? {
                        limits.max_files = n;
                    }
                    if let Some(n) = size("max_change")? {
                        limits.max_change_bytes = n;
                    }
                    if let Some(n) = count("max_open")? {
                        limits.max_open = n;
                    }
                    if let Some(n) = size("max_total")? {
                        limits.max_open_bytes = n;
                    }
                }
                Some(_) => {
                    return Err(StoreError::Parse(format!(
                        "{}: `proposals` must be an object of limits",
                        path.display()
                    )))
                }
            }
            limits
        };

        // Consent. A nested object so the two answers read as one concern, and a wrong type is an
        // error rather than a shrug: someone who wrote `"publish": "yes"` believes they granted it.
        let supervision = {
            let mut sup = Supervision::default();
            match v.get("supervision") {
                None | Some(serde_json::Value::Null) => {}
                Some(serde_json::Value::Object(o)) => {
                    for (key, slot) in
                        [("collect", &mut sup.collect), ("publish", &mut sup.publish)]
                    {
                        match o.get(key) {
                            None | Some(serde_json::Value::Null) => {}
                            Some(serde_json::Value::Bool(b)) => *slot = *b,
                            Some(_) => {
                                return Err(StoreError::Parse(format!(
                                    "{}: `supervision.{key}` must be true or false",
                                    path.display()
                                )))
                            }
                        }
                    }
                }
                Some(_) => {
                    return Err(StoreError::Parse(format!(
                        "{}: `supervision` must be an object like {{\"collect\": true}}",
                        path.display()
                    )))
                }
            }
            sup
        };

        Ok(Descriptor {
            name: field("name"),
            description: field("description"),
            notes,
            git_assets_max,
            proposal_limits,
            supervision,
            notes_time_zone: field("notes_time_zone"),
        })
    }

    /// **Take out the `time_zone` key formicaria 0.15.0 wrote at start.** 0.15.0 stamped each
    /// vault with the clock of whichever device opened it first — a device in another zone stamped its
    /// own zone on a vault written elsewhere, and two devices stamped two zones that would have
    /// collided on sync (2026-10-23). Nothing else ever wrote that key, so it is removed wherever it
    /// is found; an object left empty takes the file with it. Returns whether it changed anything,
    /// so the caller can record the change for the next commit.
    pub fn strip_legacy_time_zone(root: &Path) -> Result<bool, StoreError> {
        let path = root.join("vault.json");
        let Ok(text) = std::fs::read_to_string(&path) else { return Ok(false) };
        let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) else { return Ok(false) };
        let Some(obj) = v.as_object_mut() else { return Ok(false) };
        if obj.remove("time_zone").is_none() {
            return Ok(false);
        }
        if obj.is_empty() {
            std::fs::remove_file(&path)
                .map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
            return Ok(true);
        }
        let mut text =
            serde_json::to_string_pretty(&v).map_err(|e| StoreError::Io(e.to_string()))?;
        text.push('\n');
        std::fs::write(&path, text)
            .map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
        Ok(true)
    }

    /// **Settle a `vault.json` that a text merge left with conflict markers in it** — the file a
    /// version without [`merge_texts`] produced when two devices changed it at once (2026-10-23:
    /// 0.15.0's start-up stamp, written by one device and removed by another). Both versions are
    /// still in the file between the markers, so the answer is the same key-by-key merge a
    /// pull now makes, computed from them. Returns whether it wrote, so the caller can mark the
    /// path resolved and record it for the next commit.
    ///
    /// Run at open because that is the only moment it can help: a `vault.json` that does not parse
    /// makes [`Descriptor::read`] fail, which keeps the vault from opening at all. A file whose
    /// sides cannot be read as JSON is left exactly as it is — a person has to look at it.
    pub fn repair_conflict_markers(root: &Path) -> Result<bool, StoreError> {
        let path = root.join("vault.json");
        let Ok(text) = std::fs::read_to_string(&path) else { return Ok(false) };
        if !crate::merge::has_conflict_markers(&text) {
            return Ok(false);
        }
        let Some((base, ours, theirs)) = split_conflict(&text) else { return Ok(false) };
        let merged = merge_texts(&base, &ours, &theirs);
        if serde_json::from_str::<serde_json::Value>(&merged).is_err() {
            return Ok(false);
        }
        std::fs::write(&path, merged)
            .map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
        Ok(true)
    }

    /// [`Descriptor::repair_conflict_markers`], then **the merge it came from marked resolved**.
    /// Clean text over an unmerged path settles nothing as far as git is concerned (see
    /// [`crate::git::Keep::Edited`]): the path stays unmerged and `commit_all` keeps refusing every
    /// commit in the vault. So when `vault.json` is one of the merge's conflicts, the repaired file is
    /// recorded as its answer, through whichever backend this device has.
    pub fn settle_conflicted(root: &Path) -> Result<bool, StoreError> {
        if !Self::repair_conflict_markers(root)? {
            return Ok(false);
        }
        if crate::vcs::conflicts(root)?.iter().any(|p| p == "vault.json") {
            crate::vcs::resolve_conflict(root, "vault.json", crate::git::Keep::Edited)?;
        }
        Ok(true)
    }

    /// **Change one key in `vault.json`, leaving every other byte alone.**
    ///
    /// Separate from [`write_new`] and deliberately so. That function refuses to overwrite because
    /// the descriptor is the *user's* file: it may carry a description they wrote and keys this
    /// version has never heard of, and `read` keeps none of them. This one is the narrow exception
    /// a *setting* requires — it re-reads the file as raw JSON, replaces a single key, and writes
    /// it back, so unknown keys and hand-written formatting choices survive.
    ///
    /// `None` removes the key rather than writing `null`, so "off" looks like a vault that never
    /// had the setting — which is what it is.
    /// **Record this vault's supervision consent**, leaving every other byte of `vault.json` alone.
    ///
    /// Same narrow-rewrite discipline as [`set_git_assets_max`], and here for the same reason: this
    /// is a *setting* on a file the user owns and may have hand-edited.
    ///
    /// Both answers are always written together, explicitly, even when one is the default. Consent
    /// is the one setting where an absent key and a deliberate "no" must not look identical to the
    /// person who has to answer for it later.
    pub fn set_supervision(root: &Path, sup: Supervision) -> Result<(), StoreError> {
        let path = root.join("vault.json");
        let mut v: serde_json::Value = match std::fs::read_to_string(&path) {
            Ok(t) => serde_json::from_str(&t).map_err(|e| {
                StoreError::Parse(format!("{} is not valid JSON: {e}", path.display()))
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                serde_json::Value::Object(serde_json::Map::new())
            }
            Err(e) => return Err(StoreError::Io(format!("{}: {e}", path.display()))),
        };
        let Some(obj) = v.as_object_mut() else {
            return Err(StoreError::Parse(format!("{}: not a JSON object", path.display())));
        };
        let mut m = serde_json::Map::new();
        m.insert("collect".into(), serde_json::Value::Bool(sup.collect));
        m.insert("publish".into(), serde_json::Value::Bool(sup.publish));
        obj.insert("supervision".into(), serde_json::Value::Object(m));
        let mut text =
            serde_json::to_string_pretty(&v).map_err(|e| StoreError::Io(e.to_string()))?;
        text.push('\n');
        std::fs::write(&path, text).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))
    }

    pub fn set_git_assets_max(root: &Path, max: Option<u64>) -> Result<(), StoreError> {
        // **Refused here, not in the dispatch arm**, so `fm-cli` and every future caller inherit
        // the bound rather than each re-deriving it. Writing a limit we would then decline to
        // honour is the dishonest half of a clamp: the file would say one thing and the push do
        // another. See [`GIT_ASSETS_CEILING`] for why the number is what it is.
        if let Some(n) = max {
            if n > GIT_ASSETS_CEILING {
                return Err(StoreError::Parse(format!(
                    "{} is larger than {} — without git-lfs an attachment that size is permanent \
                     history and most hosts refuse the push outright. Leave the heavy ones to the \
                     backup snapshot.",
                    format_size(n),
                    format_size(GIT_ASSETS_CEILING)
                )));
            }
        }
        let path = root.join("vault.json");
        let mut v: serde_json::Value = match std::fs::read_to_string(&path) {
            Ok(t) => serde_json::from_str(&t).map_err(|e| {
                StoreError::Parse(format!("{} is not valid JSON: {e}", path.display()))
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                serde_json::Value::Object(serde_json::Map::new())
            }
            Err(e) => return Err(StoreError::Io(format!("{}: {e}", path.display()))),
        };
        let Some(obj) = v.as_object_mut() else {
            return Err(StoreError::Parse(format!("{}: not a JSON object", path.display())));
        };
        match max {
            Some(n) => {
                obj.insert("git_assets_max".into(), serde_json::Value::String(format_size(n)));
            }
            None => {
                obj.remove("git_assets_max");
            }
        }
        if obj.is_empty() {
            // Nothing left to say: do not leave `{}` behind for someone to wonder about.
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
            }
            return Ok(());
        }
        let mut text =
            serde_json::to_string_pretty(&v).map_err(|e| StoreError::Io(e.to_string()))?;
        text.push('\n');
        std::fs::write(&path, text).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))
    }

    /// Where this vault's notes live, given its root. The single question `FileStore` asks.
    pub fn notes_dir(&self, root: &Path) -> PathBuf {
        root.join(self.notes.clone().unwrap_or_else(|| PathBuf::from("notes")))
    }

    /// Write `vault.json` for a vault that does not have one. Returns whether it wrote.
    ///
    /// **Never overwrites**, which is why it is `write_new` and not `write`. A descriptor is
    /// the user's file — it may hold a `description` they wrote and keys this version has
    /// never heard of ([`Descriptor::read`] preserves neither, because it only reads the
    /// three it knows) — so a writer that rewrote it would silently delete both. The only
    /// safe write is the one that creates a file where none exists.
    ///
    /// The reason it exists at all: restoring from a backup. `backup` snapshots the vault's
    /// own directories and not its root, so `vault.json` is not in the snapshot — and a vault
    /// whose notes were in `docs/` would come back with its notes intact, be opened looking in
    /// `notes/`, and show nothing at all. The snapshot's recorded paths are the last surviving
    /// record of that name, so this is how it gets written back down.
    ///
    /// Only fields with an opinion are emitted; a descriptor with nothing to say writes
    /// nothing and returns false, rather than leaving `{}` behind for someone to wonder about.
    pub fn write_new(&self, root: &Path) -> Result<bool, StoreError> {
        let path = root.join("vault.json");
        if path.exists() {
            return Ok(false);
        }
        let mut obj = serde_json::Map::new();
        if let Some(n) = &self.name {
            obj.insert("name".into(), serde_json::Value::String(n.clone()));
        }
        if let Some(d) = &self.description {
            obj.insert("description".into(), serde_json::Value::String(d.clone()));
        }
        if let Some(n) = &self.notes {
            obj.insert("notes".into(), serde_json::Value::String(n.to_string_lossy().into_owned()));
        }
        if obj.is_empty() {
            return Ok(false);
        }
        let mut text = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
            .map_err(|e| StoreError::Io(e.to_string()))?;
        text.push('\n');
        std::fs::write(&path, text)
            .map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn with(json: &str) -> (tempfile::TempDir, Descriptor) {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), json).unwrap();
        let desc = Descriptor::read(d.path()).unwrap();
        (d, desc)
    }

    /// The common case by a wide margin, and it must be silent.
    #[test]
    fn an_absent_descriptor_is_todays_behaviour() {
        let d = tempdir().unwrap();
        let desc = Descriptor::read(d.path()).unwrap();

        assert_eq!(desc, Descriptor::default());
        assert_eq!(desc.notes_dir(d.path()), d.path().join("notes"));
    }

    #[test]
    fn it_reads_the_three_facts_git_cannot_supply() {
        let (d, desc) = with(r#"{"name":"lab","description":"Ravi's group","notes":"docs"}"#);

        assert_eq!(desc.name.as_deref(), Some("lab"));
        assert_eq!(desc.description.as_deref(), Some("Ravi's group"));
        assert_eq!(desc.notes_dir(d.path()), d.path().join("docs"));
    }

    /// Absent, null and empty all mean "no opinion" — a half-filled file must not turn into
    /// a vault named "".
    #[test]
    fn empty_and_missing_fields_are_both_no_opinion() {
        let (d, desc) = with(r#"{"name":"","description":null}"#);

        assert_eq!(desc.name, None);
        assert_eq!(desc.description, None);
        assert_eq!(desc.notes_dir(d.path()), d.path().join("notes"), "still the default");
    }

    /// Unknown keys are the user's, or a newer version's. Neither is ours to reject.
    #[test]
    fn an_unknown_key_is_not_an_error() {
        let (_d, desc) = with(r#"{"name":"lab","somethingElse":{"nested":true}}"#);
        assert_eq!(desc.name.as_deref(), Some("lab"));
    }

    /// Audience is decided by *location*, so notes escaping the vault would make "who can
    /// see this note" unanswerable.
    #[test]
    fn notes_may_not_escape_the_vault() {
        for bad in
            [r#"{"notes":"/etc"}"#, r#"{"notes":"../elsewhere"}"#, r#"{"notes":"a/../../b"}"#]
        {
            let d = tempdir().unwrap();
            std::fs::write(d.path().join("vault.json"), bad).unwrap();
            assert!(Descriptor::read(d.path()).is_err(), "must refuse: {bad}");
        }
    }

    /// The restore case: the notes were in `docs/`, and without this they come back invisible.
    #[test]
    fn write_new_records_a_non_default_notes_dir_and_reads_back() {
        let d = tempdir().unwrap();
        let desc = Descriptor { notes: Some(PathBuf::from("docs")), ..Default::default() };

        assert!(desc.write_new(d.path()).unwrap());
        assert_eq!(Descriptor::read(d.path()).unwrap().notes_dir(d.path()), d.path().join("docs"));
    }

    /// A descriptor is the user's file, and `read` keeps only the three keys it knows — so a
    /// writer that overwrote would delete their description and any key a newer version added.
    #[test]
    fn write_new_never_overwrites_what_is_already_there() {
        let d = tempdir().unwrap();
        let theirs = r#"{"description":"Ravi's group","somethingElse":true}"#;
        std::fs::write(d.path().join("vault.json"), theirs).unwrap();

        let wrote = Descriptor { notes: Some(PathBuf::from("docs")), ..Default::default() }
            .write_new(d.path())
            .unwrap();

        assert!(!wrote);
        assert_eq!(std::fs::read_to_string(d.path().join("vault.json")).unwrap(), theirs);
    }

    /// Nothing to say writes nothing, rather than leaving `{}` for someone to wonder about.
    #[test]
    fn write_new_with_no_opinion_writes_no_file() {
        let d = tempdir().unwrap();
        assert!(!Descriptor::default().write_new(d.path()).unwrap());
        assert!(!d.path().join("vault.json").exists());
    }

    /// It was put there on purpose, so applying "defaults" the user thinks are overridden is
    /// worse than saying the file is broken.
    #[test]
    fn a_malformed_descriptor_is_an_error_not_a_shrug() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), "{ not json").unwrap();
        assert!(Descriptor::read(d.path()).is_err());
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}:\n{text}"))
    }

    /// The case that broke a vault: one device wrote 0.15.0's stamp, the other took it out. The key
    /// is dropped from every side, so the two agree and nothing is left to conflict.
    #[test]
    fn the_legacy_time_zone_cannot_make_a_merge_disagree() {
        let base = r#"{"git_assets_max":"50MB","time_zone":"America/Denver"}"#;
        let ours = r#"{"git_assets_max":"50MB"}"#;
        let theirs = r#"{"git_assets_max":"50MB","time_zone":"Africa/Accra"}"#;
        assert_eq!(json(&merge_texts(base, ours, theirs)), json(r#"{"git_assets_max":"50MB"}"#));
    }

    /// Each key takes the side that changed it, so two devices changing different settings both win.
    #[test]
    fn different_keys_changed_on_each_side_both_land() {
        let base = r#"{"name":"lab","git_assets_max":"50MB"}"#;
        let ours = r#"{"name":"lab","git_assets_max":"20MB"}"#;
        let theirs = r#"{"name":"lab","git_assets_max":"50MB","description":"shared notes"}"#;
        assert_eq!(
            json(&merge_texts(base, ours, theirs)),
            json(r#"{"name":"lab","git_assets_max":"20MB","description":"shared notes"}"#)
        );
    }

    /// A key one side removed and the other left alone is removed; one the other side changed is kept.
    #[test]
    fn a_deletion_stands_unless_the_other_side_edited_the_key() {
        let base = r#"{"name":"lab","description":"old"}"#;
        let ours = r#"{"description":"old"}"#;
        let theirs = r#"{"name":"lab","description":"new"}"#;
        assert_eq!(json(&merge_texts(base, ours, theirs)), json(r#"{"description":"new"}"#));
        let theirs = r#"{"name":"lab 2","description":"old"}"#;
        assert_eq!(
            json(&merge_texts(base, ours, theirs)),
            json(r#"{"name":"lab 2","description":"old"}"#),
            "an edit beats a deletion"
        );
    }

    /// Two different values: both kept, the same file from either side, and it still reads.
    #[test]
    fn a_real_disagreement_keeps_both_and_both_devices_write_the_same_bytes() {
        let base = r#"{"name":"lab"}"#;
        let (a, b) = (r#"{"name":"lab A"}"#, r#"{"name":"lab B"}"#);
        let one = merge_texts(base, a, b);
        assert_eq!(one, merge_texts(base, b, a), "the side a device is on must not matter");
        assert_eq!(json(&one), json(r#"{"name":"lab B","conflict-name":["lab A"]}"#));

        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), &one).unwrap();
        assert_eq!(Descriptor::read(d.path()).unwrap().name.as_deref(), Some("lab B"));
    }

    /// Merging again after that adds nothing: the record of the disagreement does not grow.
    #[test]
    fn a_kept_disagreement_merges_as_a_fixed_point() {
        let settled =
            merge_texts(r#"{"name":"lab"}"#, r#"{"name":"lab A"}"#, r#"{"name":"lab B"}"#);
        assert_eq!(merge_texts(&settled, &settled, &settled), settled);
        let edited = r#"{"name":"lab C"}"#;
        assert_eq!(
            json(&merge_texts(&settled, edited, &settled)),
            json(edited),
            "a side that settled it by hand settles it everywhere"
        );
    }

    /// Two devices changing two different consent answers made two edits, not one disagreement.
    #[test]
    fn a_nested_setting_is_merged_key_by_key() {
        let base =
            r#"{"supervision":{"collect":true,"publish":false},"proposals":{"max_files":5}}"#;
        let ours =
            r#"{"supervision":{"collect":false,"publish":false},"proposals":{"max_files":5}}"#;
        let theirs =
            r#"{"supervision":{"collect":true,"publish":true},"proposals":{"max_files":9}}"#;
        let merged = merge_texts(base, ours, theirs);
        assert_eq!(
            json(&merged),
            json(r#"{"supervision":{"collect":false,"publish":true},"proposals":{"max_files":9}}"#)
        );
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), &merged).unwrap();
        let desc = Descriptor::read(d.path()).unwrap();
        assert_eq!(desc.supervision, Supervision { collect: false, publish: true });
        assert_eq!(desc.proposal_limits.max_files, 9);
    }

    /// The same sub-key set two ways keeps both inside its object, and the object still reads.
    #[test]
    fn a_nested_disagreement_stays_inside_its_object() {
        let base = r#"{"proposals":{"max_files":5,"max_open":3}}"#;
        let ours = r#"{"proposals":{"max_files":7,"max_open":3}}"#;
        let theirs = r#"{"proposals":{"max_files":9,"max_open":4}}"#;
        let merged = merge_texts(base, ours, theirs);
        assert_eq!(merged, merge_texts(base, theirs, ours), "the same bytes from either side");
        assert_eq!(
            json(&merged),
            json(r#"{"proposals":{"max_files":9,"conflict-max_files":[7],"max_open":4}}"#)
        );
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("vault.json"), &merged).unwrap();
        assert_eq!(Descriptor::read(d.path()).unwrap().proposal_limits.max_files, 9);
    }

    /// A side that is not JSON is no information, never a reason to fail or to write markers.
    #[test]
    fn an_unreadable_side_leaves_the_other_standing() {
        let ok = r#"{"name":"lab"}"#;
        assert_eq!(json(&merge_texts("", "<<<<<<< broken", ok)), json(ok));
        assert_eq!(json(&merge_texts("", ok, "not json")), json(ok));
    }

    /// The file a text merge already left behind: both versions are between the markers.
    #[test]
    fn a_file_with_conflict_markers_is_repaired_from_its_two_sides() {
        let d = tempdir().unwrap();
        let conflicted = "{\n<<<<<<< HEAD\n  \"git_assets_max\": \"50MB\"\n=======\n  \"git_assets_max\": \"50MB\",\n  \"time_zone\": \"Africa/Accra\"\n>>>>>>> origin/main\n}\n";
        std::fs::write(d.path().join("vault.json"), conflicted).unwrap();
        assert!(Descriptor::read(d.path()).is_err(), "the state that kept the vault shut");

        assert!(Descriptor::repair_conflict_markers(d.path()).unwrap());
        let text = std::fs::read_to_string(d.path().join("vault.json")).unwrap();
        assert_eq!(json(&text), json(r#"{"git_assets_max":"50MB"}"#));
        assert_eq!(Descriptor::read(d.path()).unwrap().git_assets_max, Some(50_000_000));
        assert!(!Descriptor::repair_conflict_markers(d.path()).unwrap(), "nothing left to do");
    }

    /// The diff3 style carries the base too, which lets a deletion stand.
    #[test]
    fn a_diff3_conflict_uses_its_base() {
        let text = "<<<<<<< ours\n{\"description\":\"old\"}\n||||||| base\n{\"name\":\"lab\",\"description\":\"old\"}\n=======\n{\"name\":\"lab\",\"description\":\"new\"}\n>>>>>>> theirs\n";
        let (base, ours, theirs) = split_conflict(text).unwrap();
        assert_eq!(json(&merge_texts(&base, &ours, &theirs)), json(r#"{"description":"new"}"#));
    }

    /// Markers out of order are not a file this can read; it is left for a person.
    #[test]
    fn markers_out_of_order_are_left_alone() {
        assert!(split_conflict("=======\n{}\n<<<<<<< a\n>>>>>>> b\n").is_none());
        let d = tempdir().unwrap();
        let odd = "<<<<<<< a\n=======\n<<<<<<< b\n>>>>>>> c\n";
        std::fs::write(d.path().join("vault.json"), odd).unwrap();
        assert!(!Descriptor::repair_conflict_markers(d.path()).unwrap());
        assert_eq!(std::fs::read_to_string(d.path().join("vault.json")).unwrap(), odd);
    }
}
