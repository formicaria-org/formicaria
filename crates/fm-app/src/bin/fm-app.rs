//! The Tauri desktop shell. Every command here is three lines: lock the store,
//! call the matching function in `fm_app::commands`, map the error to a string.
//! All behavior lives in the (webkit-free, unit-tested) library; this file only
//! wires it to a window. Built only with `--features desktop`, in the `gui` env.

use fm_app::dto::{Board, ObjectMeta};
use fm_core::FileStore;
use std::sync::Mutex;
use tauri::State;

/// The store is opened once at startup and shared behind a mutex — file writes
/// and index updates happen in one place, serialized.
struct AppState {
    store: Mutex<FileStore>,
}

#[tauri::command]
fn board(state: State<AppState>, group_by: String) -> Result<Board, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    fm_app::commands::board(&*store, &group_by).map_err(|e| e.to_string())
}

#[tauri::command]
fn capture(state: State<AppState>, body: String) -> Result<ObjectMeta, String> {
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    fm_app::commands::capture(&mut *store, &body).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_property(
    state: State<AppState>,
    id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    fm_app::commands::set_property(&mut *store, &id, &key, &value).map_err(|e| e.to_string())
}

fn main() {
    // Opening the vault rebuilds the index from the files on disk — same "reindex
    // on reload" every entry point uses.
    let vault = std::env::var("FM_VAULT").unwrap_or_else(|_| "vault".to_string());
    let store = FileStore::open(&vault).expect("open vault");

    tauri::Builder::default()
        .manage(AppState { store: Mutex::new(store) })
        .invoke_handler(tauri::generate_handler![board, capture, set_property])
        .run(tauri::generate_context!())
        .expect("error while running the formicarium window");
}
