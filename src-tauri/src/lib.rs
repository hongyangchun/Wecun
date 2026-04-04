#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;
pub mod utils;

use commands::{
    cancel_download, clear_saved_cookie_cmd, export_posts, has_saved_cookie,
    load_saved_cookie_cmd, open_login_window, start_download,
};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_download,
            cancel_download,
            export_posts,
            open_login_window,
            has_saved_cookie,
            load_saved_cookie_cmd,
            clear_saved_cookie_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
