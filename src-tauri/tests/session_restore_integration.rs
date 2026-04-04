use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_app_lib::services::weibo_api::{load_saved_cookie, save_cookie};
use tauri_app_lib::state::AppState;

fn create_app() -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .manage(AppState::default())
        .build(mock_context(noop_assets()))
        .expect("failed to build mock app")
}

#[test]
fn saved_cookie_restores_across_fresh_app_instances() {
    let first_app = create_app();
    let first_handle = first_app.handle().clone();

    save_cookie(&first_handle, "SUB=restored-session; ALF=1");

    let second_app = create_app();
    let second_handle = second_app.handle().clone();
    let second_state = second_handle.state::<AppState>();

    let restored = load_saved_cookie(&second_handle);

    assert_eq!(restored.as_deref(), Some("SUB=restored-session; ALF=1"));
    assert!(second_state.get_cookie().is_empty(), "test should prove explicit restore is still required");
}
