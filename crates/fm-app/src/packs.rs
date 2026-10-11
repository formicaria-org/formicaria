//! Reference packs: what this installation can fetch, where it keeps them, and what is there.
//!
//! `decisions.md#toolchain`, 2026-10-11 — *a reference pack is fetched on request, like a model,
//! and is not the assistant's.* So none of this is in an agent crate, and none of it fetches:
//! **no HTTP client enters `fm-app`.** A transport shell downloads the file named here
//! (`fm_fetch`); this module says what to download, where it goes, and reads what arrived.

use fm_core::pack::{FoodDetail, FoodHit, FoodPack, PackInfo, FOOD_FILE};
use serde::Serialize;
use std::path::PathBuf;

/// A pack this version of the app knows how to use. **The catalogue follows the app**
/// (`decisions.md#agent`, 2026-10-10): it is compiled in, so a build always offers the pack whose
/// layout it can read.
pub struct Offer {
    pub title: &'static str,
    pub version: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    /// Where it is fetched from. `None` until a pack is published: the app then says the list is
    /// not available yet, and offers nothing it cannot deliver (*advice that cannot succeed is
    /// worse than none*).
    pub url: Option<&'static str>,
    pub licence: &'static str,
}

/// The generic-foods pack `ci/pack-foods.py` builds. Its `bytes` and `sha256` are that script's
/// output and change with it.
pub const FOOD: Offer = Offer {
    title: "Generic foods (USDA FoodData Central)",
    version: "2026.10",
    bytes: 8_134_656,
    sha256: "daef8e92574b80bca40343a4db79845699dcf19c6f978973de13601a9bea850f",
    url: None,
    licence: "U.S. Government work, public domain",
};

/// Where packs live: beside the assistant's models and runtime, outside every vault.
/// `FM_PACKS_DIR` overrides it — for tests, and for trying a pack built by hand.
pub fn dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("FM_PACKS_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    crate::vaults::config_dir().map(|d| d.join("formicaria").join("tools").join("packs"))
}

pub fn food_path() -> Option<PathBuf> {
    dir().map(|d| d.join(FOOD_FILE))
}

/// Where the food pack is fetched from and what it must hash to. `FM_PACK_URL` (with
/// `FM_PACK_SHA256`) overrides the compiled-in offer, so a pack can be tried from a local server
/// before one is published.
pub fn food_source() -> Option<(String, String)> {
    if let Ok(url) = std::env::var("FM_PACK_URL") {
        if !url.trim().is_empty() {
            let sha = std::env::var("FM_PACK_SHA256").unwrap_or_else(|_| FOOD.sha256.to_string());
            return Some((url, sha));
        }
    }
    FOOD.url.map(|u| (u.to_string(), FOOD.sha256.to_string()))
}

const MISSING: &str =
    "The food list is not on this device yet. Get it in Settings, or add a food by name.";

fn open_food() -> Result<FoodPack, String> {
    let path = food_path().filter(|p| p.exists()).ok_or(MISSING)?;
    FoodPack::open(&path).map_err(|e| e.to_string())
}

pub fn food_search(q: &str, limit: usize) -> Result<Vec<FoodHit>, String> {
    open_food()?.search(q, limit).map_err(|e| e.to_string())
}

pub fn food_get(id: &str) -> Result<Option<FoodDetail>, String> {
    open_food()?.get(id).map_err(|e| e.to_string())
}

/// What Settings shows: what is offered, whether it can be fetched, and what is on the device.
#[derive(Serialize)]
pub struct PackStatus {
    pub title: &'static str,
    pub version: &'static str,
    /// What fetching it will spend, stated before it is spent.
    pub bytes: u64,
    pub licence: &'static str,
    /// There is somewhere to fetch it from.
    pub available: bool,
    /// What is on this device, read from the file itself.
    pub installed: Option<PackInfo>,
    /// What removing it hands back.
    pub installed_bytes: u64,
}

pub fn food_status() -> PackStatus {
    let path = food_path().filter(|p| p.exists());
    let installed = path.as_ref().and_then(|p| FoodPack::open(p).ok()).map(|p| p.info().clone());
    let installed_bytes =
        path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0);
    PackStatus {
        title: FOOD.title,
        version: FOOD.version,
        bytes: FOOD.bytes,
        licence: FOOD.licence,
        available: food_source().is_some(),
        installed,
        installed_bytes,
    }
}

/// Remove the food pack and anything half-fetched beside it. Entries already logged keep their
/// numbers — each carries its own.
pub fn food_remove() -> Result<(), String> {
    let Some(path) = food_path() else { return Ok(()) };
    for p in [path.clone(), path.with_extension("sqlite.part")] {
        match std::fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("could not remove {}: {e}", p.display())),
        }
    }
    Ok(())
}
