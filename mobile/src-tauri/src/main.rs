// Desktop entry point. The Android build uses `lib.rs`'s `mobile_entry_point` instead, so this
// exists mainly so the crate can be run and debugged on a laptop.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    formicaria_mobile_lib::run()
}
