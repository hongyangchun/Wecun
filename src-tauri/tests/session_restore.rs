use reqwest::StatusCode;
use tauri_app_lib::services::weibo_api::{apply_saved_cookie_to_state, is_auth_invalid_response};
use tauri_app_lib::state::AppState;

#[test]
fn apply_saved_cookie_to_state_stores_non_empty_cookie() {
    let state = AppState::default();

    let restored = apply_saved_cookie_to_state(&state, Some("SUB=restored; ALF=1".to_string()));

    assert_eq!(restored.as_deref(), Some("SUB=restored; ALF=1"));
    assert_eq!(state.get_cookie(), "SUB=restored; ALF=1");
}

#[test]
fn app_state_default_exposes_reusable_http_client() {
    let state = AppState::default();

    let cloned = state.get_client();
    let request = cloned.get("https://example.com/image.jpg").build().unwrap();

    assert_eq!(request.url().as_str(), "https://example.com/image.jpg");
}

#[test]
fn apply_saved_cookie_to_state_clears_state_for_blank_cookie() {
    let state = AppState::default();
    state.set_cookie("SUB=stale".to_string());

    let restored = apply_saved_cookie_to_state(&state, Some("   ".to_string()));

    assert_eq!(restored, None);
    assert!(state.get_cookie().is_empty());
}

#[test]
fn detects_auth_invalid_redirect_body() {
    let body = r#"<html><head><meta http-equiv=\"refresh\" content=\"0; url='https://weibo.com/ajax/profile/info?retcode=6102'\"/></head></html>"#;

    assert!(is_auth_invalid_response(StatusCode::OK, body));
}

#[test]
fn ignores_normal_json_response() {
    let body = r#"{"ok":1,"data":{"user":{"id":1}}}"#;

    assert!(!is_auth_invalid_response(StatusCode::OK, body));
}

#[test]
fn detects_logged_out_html_shell() {
    let body = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="shortcut icon" href="https://weibo.com/favicon.ico" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no, viewport-fit=cover" />
    <meta http-equiv="Content-Security-Policy" content="upgrade-insecure-requests" />
    <script>
      function scriptLoaded() {
        if (window.wbBotDetector) {
          window.wbBotDetector.load({ from: 'weibo' });
        }
      }
    </script>
  </head>
  <body></body>
</html>"#;

    assert!(is_auth_invalid_response(StatusCode::OK, body));
}
