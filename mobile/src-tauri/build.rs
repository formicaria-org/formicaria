//! Tauri's codegen, plus the one compile-time fact this shell cannot express in Cargo.
//!
//! **`agent_shell` means "the study agent is compiled into this build".** It is the `agent`
//! feature **and** Android, and it needs a build script because a Cargo feature cannot be made
//! conditional on a target: `default = ["agent"]` turns the feature on for every target, and an
//! iOS build then tries to compile `src/agent.rs`, whose model runner reaches for
//! `fm_agent_run::nativelib::native_lib_dir` — an Android-only symbol (an APK's
//! `nativeLibraryDir` has no iOS counterpart), so it is a *compile error*, not a runtime one.
//!
//! That iOS ships without the assistant is not a workaround, it is the ruling: `decisions.md`
//! `#track-m`, *iOS ships agent-free* (Route C). iOS forbids `fork`/`exec` outright, so the
//! out-of-process model runner this shell uses on Android has no iOS form at all — and neither
//! candidate that would replace it can be evaluated in a Simulator. Expressing the ruling here
//! makes it the compiler's job instead of a flag someone has to remember on the command line.
//!
//! Naming it once also keeps the two-sided logic honest: `lib.rs` has both `#[cfg(agent_shell)]`
//! and `#[cfg(not(agent_shell))]` arms, and a hand-written `all(feature = "agent", target_os =
//! "android"))` repeated at nine sites is a `not(all(…))` waiting to be got wrong.
fn main() {
    // Declares the cfg so `--cfg` typos become warnings rather than silently-dead code.
    println!("cargo::rustc-check-cfg=cfg(agent_shell)");
    // **`update_shell` means "this build can update itself"**: the `update` feature, on Android. The
    // same two-sided shape as `agent_shell`, and for the same reason — iOS cannot install an app and has
    // no installer bridge, so it must compile the *other* arm, which answers a status that hides the rows.
    println!("cargo::rustc-check-cfg=cfg(update_shell)");
    if std::env::var_os("CARGO_FEATURE_UPDATE").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android")
    {
        println!("cargo::rustc-cfg=update_shell");
    }
    if std::env::var_os("CARGO_FEATURE_AGENT").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android")
    {
        println!("cargo::rustc-cfg=agent_shell");
    }
    tauri_build::build()
}
