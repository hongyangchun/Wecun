#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;
pub mod utils;

use commands::{
    analyze_profile, cancel_download, clear_saved_cookie_cmd, delete_history_entry, export_from_history,
    export_openclaw, export_posts, get_history_entry, has_saved_cookie, list_download_history,
    load_saved_cookie_cmd, open_login_window, start_download,
};
use services::weibo_api::stronghold_password_hash;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_stronghold::Builder::new(stronghold_password_hash).build(),
        )
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_download,
            cancel_download,
            export_posts,
            analyze_profile,
            export_openclaw,
            open_login_window,
            has_saved_cookie,
            load_saved_cookie_cmd,
            clear_saved_cookie_cmd,
            list_download_history,
            delete_history_entry,
            get_history_entry,
            export_from_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
