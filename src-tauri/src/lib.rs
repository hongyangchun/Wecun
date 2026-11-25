#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;
pub mod utils;

use commands::{
    analyze_profile, cancel_download, clear_saved_cookie_cmd, export_posts, export_profile, get_current_user_info,
    get_log_dir, get_saved_cookie, get_saved_session, has_saved_cookie, load_saved_cookie_cmd, lookup_user_info,
    open_login_window, set_cookie_cmd, start_download,
};
use state::AppState;

/// Runs before an aborted panic takes the process down, so the log file says why the app vanished.
fn log_panic(info: &std::panic::PanicHookInfo<'_>) {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>");
    let location = info
        .location()
        .map(|location| location.to_string())
        .unwrap_or_else(|| "<unknown location>".to_string());

    // stderr keeps `cargo run` readable; the log file is what a user can send in.
    eprintln!("panic at {location}: {message}");
    log::error!("panic at {location}: {message}");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(log_panic));

    tauri::Builder::default()
        // Registered first, so everything the app logs afterwards lands in the log file.
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stderr),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }),
                ])
                // The app's own diagnostics at debug level; dependencies only when they complain.
                .level(log::LevelFilter::Warn)
                .level_for("wecun_lib", log::LevelFilter::Debug)
                .max_file_size(500_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
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
            get_saved_session,
            get_current_user_info,
            lookup_user_info,
            load_saved_cookie_cmd,
            clear_saved_cookie_cmd,
            set_cookie_cmd,
            get_log_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
