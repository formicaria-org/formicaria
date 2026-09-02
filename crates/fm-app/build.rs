//! Only here to tell cargo that `FM_VERSION` is an input to this crate.
//!
//! `dispatch.rs` reads it with `option_env!`, which is resolved **at compile time** and then baked
//! into the binary. Cargo has no way to know that on its own: change the variable and it sees the
//! same sources, the same features and the same profile, so it reuses the cached object and the
//! binary keeps reporting whatever version it was first built with.
//!
//! That is not a theoretical worry. The release workflow restores `target/` from
//! `Swatinem/rust-cache`, so without this line a cached `fm-app` could survive a version bump and
//! ship an archive named `formicaria-v0.2.3-…` whose Settings screen says `v0.2.2` — a version
//! display that lies is worse than none, because the whole reason it exists is to tell two
//! unpacked folders apart.
fn main() {
    println!("cargo:rerun-if-env-changed=FM_VERSION");
}
