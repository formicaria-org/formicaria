//! The command library — everything the frontend can ask for, as plain functions,
//! plus **the one door they are reached through**.
//!
//! Each command is a function over the [`Store`] seam ([`commands`]) returning a
//! serializable DTO ([`dto`]). [`dispatch`] is what turns that pile of functions into a
//! *surface*: one `match` from command name to function, holding the vault list and the
//! lock discipline that makes it safe from more than one thread.
//!
//! A frontend therefore supplies only its framing — parse a request, call
//! [`dispatch::dispatch`], encode the [`Output`]. `fm-serve` is that shell for HTTP; it
//! used to *be* the surface, which meant a second frontend could only re-implement it.
//!
//! This layer has no window/webkit dependency, so `cargo test -p fm-app` exercises the real
//! logic — group by any property, a drop writes the value back to disk — in the lean
//! environment.
//!
//! [`Store`]: fm_core::Store
//! [`Output`]: dispatch::Output

/// **The Myrme contract this app is written for** (Myrme's `docs/context/contract.md`, *Versions*):
/// the major it speaks and the least minor it needs. Any Myrme release of this major with at least
/// this minor links unchanged — moving to it is a tag in `Cargo.toml` and nothing else
/// (`pixi run myrme-bump`). Stated once, here, for the desktop and the phone alike; each crate that
/// links Myrme checks it with [`check_myrme_contract!`]. `decisions.md#agent`, 2026-10-18.
/// 1.4: the manual's assistant chapter and the phone's runtime pins are read from Myrme.
/// 1.5: the phone's model choice (`Manifest::phone_choices`, "Faster" / "More accurate, slower").
/// 1.6: the OCR's ONNX Runtime, staged into the APK from Myrme's catalogue
/// (`ci/android-stage-onnxruntime.sh`, `host::BUNDLED_ONNXRUNTIME`).
pub const MYRME_CONTRACT: (u32, u32) = (1, 6);

/// The compile-time check of [`MYRME_CONTRACT`] against the Myrme a crate links: another major, or
/// an older minor, does not build, and says why rather than failing on a missing function. A macro
/// because this crate does not link Myrme; the crates that do expand it.
#[macro_export]
macro_rules! check_myrme_contract {
    () => {
        const _: () = assert!(
            fm_agent_run::CONTRACT_MAJOR == $crate::MYRME_CONTRACT.0
                && fm_agent_run::CONTRACT_MINOR >= $crate::MYRME_CONTRACT.1,
            "the pinned Myrme speaks another contract than this app was written for: pin a Myrme \
             release of the major and at least the minor in fm_app::MYRME_CONTRACT (`pixi run \
             myrme-bump`; Myrme's docs/context/releases.md lists each release's contract)"
        );
    };
}

// A CA trust store for the vendored OpenSSL, which has none on Android.
pub mod ca_bundle;
pub mod commands;
pub mod cycle;
pub mod dispatch;
pub mod dto;
pub mod guidance;
pub mod health;
pub mod log;
pub mod nutrients;
pub mod packs;
pub mod paper;
pub mod plan;
pub mod product;
pub mod refs;
pub mod reminders;
pub mod scope;
// A git token, only on platforms with no credential helper to delegate to (i.e. Android).
pub mod secrets;
pub mod themes;
pub mod thread;
pub mod tutorials;
pub mod vaultfile;
pub mod vaults;
pub mod views;
pub mod web_recipe;
// Encodings a shell needs on the way in — here rather than in the (workspace-excluded, untested)
// mobile crate, so `cargo test --workspace` reaches them.
pub mod wire;

pub use dispatch::{dispatch, dispatch_as, App, Host, Output};
pub use scope::Scope;
