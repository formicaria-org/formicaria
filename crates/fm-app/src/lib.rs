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

// A CA trust store for the vendored OpenSSL, which has none on Android.
pub mod ca_bundle;
pub mod commands;
pub mod dispatch;
pub mod dto;
pub mod refs;
pub mod scope;
// A git token, only on platforms with no credential helper to delegate to (i.e. Android).
pub mod secrets;
pub mod thread;
pub mod vaults;
pub mod views;

pub use dispatch::{dispatch, dispatch_as, App, Host, Output};
pub use scope::Scope;
