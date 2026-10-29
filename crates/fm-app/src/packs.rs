//! The food list: carried in the app, unpacked where `fm-core` can read it.
//!
//! `decisions.md#toolchain`, 2026-10-11 — *the food list is carried in the app, not fetched.* It
//! is compiled in here, compressed, and written to the packs folder the first time something
//! reads it. So every system has it the same way — a desktop archive, the Android app and the
//! iPhone app are all just the binary — with nothing to host and nothing to download. **No HTTP
//! client enters `fm-app`**, and none is needed.
//!
//! It is still a file outside every vault, still read-only, still read by `fm-core`; and a logged
//! entry still copies the values it used, so nothing already logged depends on this file.

use fm_core::pack::{FoodDetail, FoodHit, FoodPack, PackInfo, FOOD_FILE};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The list, as `ci/pack-foods.py` leaves it: gzip of the SQLite file.
static FOOD_GZ: &[u8] = include_bytes!("../packs/foods.sqlite.gz");

/// What the bytes above unpack to. **These three change together with the file**, and
/// `tests/packs.rs` holds them to it: the version, the size, and the checksum the script prints.
pub const FOOD_VERSION: &str = "2026.10.6";
pub const FOOD_BYTES: u64 = 12_570_624;
pub const FOOD_SHA256: &str = "dd5cd31139b40f354cb7e78acdb5fdf54b9d6a2ad4b1f5aa82b3c1536895ccd4";

/// Where the list is unpacked: beside the assistant's models and runtime, outside every vault.
/// `FM_PACKS_DIR` overrides it, for tests.
pub fn dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FM_PACKS_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    crate::vaults::config_dir().map(|d| d.join("formicaria").join("tools").join("packs"))
}

pub fn food_path() -> Option<PathBuf> {
    dir().map(|d| d.join(FOOD_FILE))
}

/// Is the file at `path` the list this build carries? Asked of its size first, which is free, and
/// of its own `version` second — never of its checksum, which would read 8 MB on every search.
fn is_current(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.len() == FOOD_BYTES)
        && FoodPack::open(path).is_ok_and(|p| p.info().version == FOOD_VERSION)
}

/// Make sure the list this build carries is in the packs folder, and answer where it is.
///
/// Written when the file is missing, or is not the version carried — an older one left by an
/// earlier build, or one cut short. **Unpacked to a sidecar, checked, then renamed into place**, so
/// a reader never opens half a file and a full disk leaves whatever was there before.
pub fn ensure_food() -> Result<PathBuf, String> {
    let path = food_path().ok_or("this device has nowhere to keep the food list")?;
    if is_current(&path) {
        return Ok(path);
    }
    let mut bytes = Vec::with_capacity(FOOD_BYTES as usize);
    flate2::read::GzDecoder::new(FOOD_GZ)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("the food list this app carries could not be unpacked: {e}"))?;
    if fm_core::blob::sha256_hex(&bytes) != FOOD_SHA256 {
        return Err("the food list this app carries is not the one it was built with".into());
    }
    let dir = path.parent().ok_or("the food list has no folder to go in")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let part = path.with_extension("sqlite.part");
    std::fs::write(&part, &bytes).map_err(|e| format!("{}: {e}", part.display()))?;
    std::fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

fn open_food() -> Result<FoodPack, String> {
    FoodPack::open(&ensure_food()?).map_err(|e| e.to_string())
}

pub fn food_search(q: &str, limit: usize) -> Result<Vec<FoodHit>, String> {
    open_food()?.search(q, limit).map_err(|e| e.to_string())
}

pub fn food_get(id: &str) -> Result<Option<FoodDetail>, String> {
    open_food()?.get(id).map_err(|e| e.to_string())
}

/// What Settings says about the list: what it is, how much of the device it takes, whose it is.
#[derive(Serialize)]
pub struct PackStatus {
    /// What the list says about itself, or `None` with `error` when it could not be put in place.
    pub installed: Option<PackInfo>,
    /// What it takes on this device.
    pub bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn food_status() -> PackStatus {
    match open_food() {
        Ok(p) => PackStatus { installed: Some(p.info().clone()), bytes: FOOD_BYTES, error: None },
        Err(e) => PackStatus { installed: None, bytes: FOOD_BYTES, error: Some(e) },
    }
}

/// **A food entry's numbers, from the food list as it is now** (`decisions.md#data`, 2026-10-29,
/// *every view sums from the food list*). An entry of the list keeps its food (`source`) and its
/// amount (`amount_g`); what it holds is the food's values per 100 g times the amount, worked out
/// whenever the entry is read — so a list that improves improves every entry, past and planned, and
/// no view can sum an old copy. Applied to every note a query reads (`fm_core::Scoped::with_resolver`);
/// anything else (a note, a packet from its label, a food typed in by hand) passes untouched, as
/// does an entry whose food the list no longer has, which keeps the numbers it was written with.
pub fn resolve(o: &mut fm_model::Object) {
    use fm_model::PropertyValue;
    let PropertyValue::Text(src) = o.get("source") else { return };
    if !src.starts_with("foods@") {
        return;
    }
    let PropertyValue::Int(grams) = o.get("amount_g") else { return };
    let code = src.rsplit('/').next().unwrap_or("").to_string();
    let Some(food) = cached(&code) else { return };
    for (key, per100) in &food.per100 {
        let v = (*per100 as f64 * grams as f64 / 100.0).round() as i64;
        o.extra.insert(key.clone(), PropertyValue::Int(v));
    }
    // An entry from before vitamin B12 and D were kept in nanograms holds the old key too.
    for (new, old, _) in crate::nutrients::RENAMED {
        if food.per100.contains_key(*new) {
            o.extra.remove(*old);
        }
    }
    for (prop, from) in [("borrowed", &food.borrowed), ("estimated", &food.estimated)] {
        let said: Vec<String> = from.iter().map(|(k, f)| format!("{k}={f}")).collect();
        if said.is_empty() {
            o.extra.remove(prop);
        } else {
            o.extra.insert(prop.into(), PropertyValue::Text(said.join("; ")));
        }
    }
}

/// A food of the list by its code, read once per process: a day's view reads the same few foods
/// again and again.
fn cached(code: &str) -> Option<std::sync::Arc<FoodDetail>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static FOODS: OnceLock<Mutex<HashMap<String, Option<Arc<FoodDetail>>>>> = OnceLock::new();
    let map = FOODS.get_or_init(Default::default);
    if let Some(hit) = map.lock().unwrap_or_else(|e| e.into_inner()).get(code) {
        return hit.clone();
    }
    let found = food_get(code).ok().flatten().map(Arc::new);
    map.lock().unwrap_or_else(|e| e.into_inner()).insert(code.to_string(), found.clone());
    found
}
