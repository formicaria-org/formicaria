//! The command library — everything the frontend can ask for, as plain functions.
//!
//! Each is a function over the [`Store`] seam ([`commands`]) returning a
//! serializable DTO ([`dto`]). The browser server (`fm-serve`) fronts these over
//! HTTP: it locks the store and calls the matching function here. This layer has
//! no window/webkit dependency, so `cargo test -p fm-app` exercises the real
//! logic — group by any property, a drop writes the value back to disk — in the
//! lean environment.
//!
//! [`Store`]: fm_core::Store

pub mod commands;
pub mod dto;
