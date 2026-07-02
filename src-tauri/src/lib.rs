//! Tauri IPC commands (PLAN.md §4.3). Owned by Agent D.
//!
//! `commands.rs` implements every command in the IPC surface as a thin
//! wrapper: no business logic lives here. Where a sibling crate
//! (`pcf-codec`/`pcf-images`/`pcf-manager`, owned by Agents A/B/C) doesn't
//! exist yet, the wrapper calls a schema-correct mock in `mock.rs` — see
//! the `TODO(D): swap for ...` comments in `commands.rs` for the exact
//! swap points once those crates land.

pub mod commands;
mod mock;

use commands::{
    charmap_status, detect_game_dir, export_dbdat, import_crest, import_photo, load_pkf, new_dbc,
    open_dbc, patch_manager, save_dbc,
};

/// Build and run the Tauri app. Called from `main.rs`.
///
/// Tauri config lives in `tauri.conf.json` — see the comment in
/// `dist/index.html` for what Agent G still needs to repoint for the real
/// packaged frontend/bundle.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_pkf,
            open_dbc,
            save_dbc,
            new_dbc,
            import_crest,
            import_photo,
            export_dbdat,
            detect_game_dir,
            patch_manager,
            charmap_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running pcf-editor");
}
