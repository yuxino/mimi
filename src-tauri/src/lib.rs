#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod audio;
pub mod clients;
pub mod commands;
pub mod core;
pub mod settings_store;

/// Runs the mimi Tauri application.
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            tracing::info!("mimi starting");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
