//! The testable core of the desktop app, independent of Tauri.
//!
//! Everything the frontend can ask for is a plain function over the [`Store`]
//! seam ([`commands`]) returning a serializable DTO ([`dto`]). The Tauri binary
//! (`src/bin/fm-app.rs`, behind the `desktop` feature) is a thin wrapper: each
//! `#[tauri::command]` locks the store and calls the matching function here.
//! Because this layer has no `tauri` dependency, `cargo test -p fm-app` exercises
//! the board — group by any property, drop writes the value back to disk — with
//! zero webkit, in the lean environment.
//!
//! [`Store`]: fm_core::Store

pub mod commands;
pub mod dto;
