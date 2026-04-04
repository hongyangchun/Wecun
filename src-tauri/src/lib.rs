#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;

use commands::{cancel_download, open_login_window, start_download, test_api};
use tauri::{Listener, Manager};

use state::{set_app_handle, ApiFetchResult, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_download,
            cancel_download,
            open_login_window,
            test_api,
        ])
        .setup(|app| {
            set_app_handle(app.handle().clone());

            let app_handle = app.handle().clone();
            app.listen("weibo-api-result", move |event| {
                let payload = event.payload();

                if let Ok(result) = serde_json::from_str::<ApiFetchResult>(payload) {
                    let state = app_handle.state::<AppState>();
                    state.store_api_result(result);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
