#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;
pub mod utils;

use commands::{
    analyze_profile, cancel_download, clear_saved_cookie_cmd, export_posts, export_profile, get_current_user_info,
    get_saved_cookie, has_saved_cookie, load_saved_cookie_cmd, open_login_window, set_cookie_cmd, start_download,
};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
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
            export_profile,
            open_login_window,
            has_saved_cookie,
            get_saved_cookie,
            get_current_user_info,
            load_saved_cookie_cmd,
            clear_saved_cookie_cmd,
            set_cookie_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
