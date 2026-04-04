use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::TimeZone;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_app_lib::models::{DateRange, DownloadRequest, ExportFormat, PostFilter};
use tauri_app_lib::services::downloader::DownloadService;
use tauri_app_lib::state::AppState;

fn create_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .manage(AppState::default())
        .build(mock_context(noop_assets()))
        .expect("failed to build mock app")
}

fn temp_output_dir() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis();
    std::env::temp_dir().join(format!("weibo-live-export-smoke-{unique}"))
}

fn actual_saved_cookie() -> String {
    let home = std::env::var("HOME").expect("HOME should be set");
    let path = PathBuf::from(home)
        .join("Library/Application Support/com.hongyangchun.weibo-downloader/weibo_cookie.dat");
    fs::read_to_string(path).expect("expected saved live Weibo cookie file")
}

#[tokio::test]
async fn live_session_exports_real_long_post_content() {
    let app = create_app();
    let handle = app.handle().clone();
    let state = handle.state::<AppState>();

    let cookie = actual_saved_cookie();
    assert!(!cookie.trim().is_empty(), "saved live Weibo cookie should not be blank");
    state.set_cookie(cookie);

    let output_dir = temp_output_dir();
    fs::create_dir_all(&output_dir).expect("failed to create output dir");

    let request = DownloadRequest {
        uid: "1195242865".to_string(),
        cookie: String::new(),
        filter: PostFilter::All,
        include_images: false,
        date_range: DateRange {
            start_timestamp: Some(chrono::FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2026, 4, 1, 0, 0, 0)
                .single()
                .unwrap()
                .timestamp()),
            end_timestamp: Some(chrono::FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2026, 4, 3, 23, 59, 59)
                .single()
                .unwrap()
                .timestamp()),
        },
        export_format: ExportFormat::MarkdownSingle,
        output_dir: output_dir.to_string_lossy().to_string(),
        min_text_length: 0,
    };

    DownloadService::new()
        .run(&request, state.inner(), &handle)
        .await
        .expect("live export smoke test failed");

    let export_file = output_dir.join("杨幂-微博导出.md");
    let content = fs::read_to_string(&export_file).expect("expected markdown export file");

    assert!(
        content.contains("为了融入社区，孤独症人士和Ta们的家人都在尝试探索新的身份"),
        "export should contain expanded long-text content"
    );

    let _ = fs::remove_dir_all(&output_dir);
}
