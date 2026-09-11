//! Only here to tell cargo that `FM_VERSION` is an input to this crate.
//!
//! `Version::running()` reads it with `option_env!`, which is evaluated when the crate compiles. Without
//! this line a cached `fm-update` survives a version bump, and a release named `v0.6.1` compares itself
//! as `v0.6.0` — offering users the very version they are already running, or refusing the newer one.
//! `fm-app/build.rs` exists for exactly the same reason. (The comparison lived in `fm-serve` before
//! this crate, whose build script watches only the UI, so it had this gap too.)
fn main() {
    println!("cargo:rerun-if-env-changed=FM_VERSION");
}
