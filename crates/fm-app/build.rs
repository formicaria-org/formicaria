// Tauri codegen runs only for the desktop build; the lean lib/test build (no
// `desktop` feature) has an empty build script and never needs tauri-build.
fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::build();
}
