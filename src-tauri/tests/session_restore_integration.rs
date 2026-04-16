use std::fs;
use std::sync::{Mutex, OnceLock};

use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;
use wecun_lib::services::weibo_api::{
    clear_saved_cookie, load_saved_cookie, restore_saved_cookie, save_cookie,
};
use wecun_lib::state::AppState;

const COOKIE_FILE_NAME: &str = "weibo_cookie.dat";
const STRONGHOLD_FILE_NAME: &str = "cookie_vault.tauri";

fn cookie_storage_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_cookie_storage() -> std::sync::MutexGuard<'static, ()> {
    match cookie_storage_lock().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn create_app() -> tauri::App<tauri::test::MockRuntime> {
    let mut context = mock_context(noop_assets());
    // Use a fixed identifier for all tests to ensure they share the same app_data_dir
    context.config_mut().identifier = "com.test.wecun".to_string();

    mock_builder()
        .manage(AppState::default())
        .build(context)
        .expect("failed to build mock app")
}

fn cookie_dir(handle: &tauri::AppHandle<tauri::test::MockRuntime>) -> std::path::PathBuf {
    // Use app_data_dir to match the actual implementation
    handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("wecun"))
}

fn stronghold_dir(handle: &tauri::AppHandle<tauri::test::MockRuntime>) -> std::path::PathBuf {
    // Stronghold uses app_local_data_dir in the actual implementation
    handle
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("wecun"))
}

fn cleanup_cookie_storage(handle: &tauri::AppHandle<tauri::test::MockRuntime>) {
    let dir = cookie_dir(handle);
    let stronghold_dir = stronghold_dir(handle);
    let _ = fs::remove_file(dir.join(COOKIE_FILE_NAME));
    let _ = fs::remove_file(stronghold_dir.join(STRONGHOLD_FILE_NAME));
}

#[test]
fn saved_cookie_restores_across_fresh_app_instances() {
    let _guard = lock_cookie_storage();
    let first_app = create_app();
    let first_handle = first_app.handle().clone();
    cleanup_cookie_storage(&first_handle);

    save_cookie(&first_handle, "SUB=restored-session; ALF=1");

    let second_app = create_app();
    let second_handle = second_app.handle().clone();
    let second_state = second_handle.state::<AppState>();

    let restored = load_saved_cookie(&second_handle);

    assert_eq!(restored.as_deref(), Some("SUB=restored-session; ALF=1"));
    assert!(
        second_state.get_cookie().is_empty(),
        "test should prove explicit restore is still required"
    );

    cleanup_cookie_storage(&second_handle);
}

#[test]
fn restore_saved_cookie_migrates_plaintext_cookie_to_stronghold() {
    let _guard = lock_cookie_storage();
    let app = create_app();
    let handle = app.handle().clone();
    let state = handle.state::<AppState>();
    let dir = cookie_dir(&handle);
    let stronghold_dir = stronghold_dir(&handle);
    let plaintext_path = dir.join(COOKIE_FILE_NAME);
    let stronghold_path = stronghold_dir.join(STRONGHOLD_FILE_NAME);

    cleanup_cookie_storage(&handle);
    fs::create_dir_all(&dir).expect("failed to create cookie dir");
    fs::create_dir_all(&stronghold_dir).expect("failed to create stronghold dir");
    fs::write(&plaintext_path, "SUB=migrate-me; ALF=1").expect("failed to seed plaintext cookie");

    let restored = restore_saved_cookie(&handle, state.inner());

    assert_eq!(restored.as_deref(), Some("SUB=migrate-me; ALF=1"));
    assert_eq!(state.get_cookie(), "SUB=migrate-me; ALF=1");
    assert!(
        !plaintext_path.exists(),
        "plaintext cookie file should be deleted after migration"
    );
    assert!(
        stronghold_path.exists(),
        "stronghold vault should be created during migration"
    );
    assert_eq!(
        load_saved_cookie(&handle).as_deref(),
        Some("SUB=migrate-me; ALF=1")
    );

    cleanup_cookie_storage(&handle);
}

#[test]
fn clear_saved_cookie_removes_stored_cookie() {
    let _guard = lock_cookie_storage();
    let app = create_app();
    let handle = app.handle().clone();

    cleanup_cookie_storage(&handle);
    save_cookie(&handle, "SUB=clear-me; ALF=1");
    clear_saved_cookie(&handle);

    assert_eq!(load_saved_cookie(&handle), None);

    cleanup_cookie_storage(&handle);
}
