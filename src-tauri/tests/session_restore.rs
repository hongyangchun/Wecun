use reqwest::StatusCode;
use wecun_lib::services::weibo_api::{apply_saved_cookie_to_state, is_auth_invalid_response};
use wecun_lib::state::AppState;

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

#[test]
fn a_bare_forbidden_response_is_not_treated_as_a_lost_login() {
    // Weibo also answers 403 and 418 when it throttles or wants a captcha. Logging the user out for
    // that throws away a session that is still valid.
    assert!(!is_auth_invalid_response(
        StatusCode::FORBIDDEN,
        r#"{"code":10000,"message":"访问过于频繁"}"#
    ));
    assert!(!is_auth_invalid_response(
        StatusCode::from_u16(418).unwrap(),
        "<html>访问过于频繁</html>"
    ));

    // The body still decides: a 403 that carries the sign-in redirect is a lost login.
    assert!(is_auth_invalid_response(
        StatusCode::FORBIDDEN,
        r#"<html><head><meta http-equiv=\"refresh\" content=\"0; url='https://passport.weibo.com/sso/signin?retcode=6102'\"/></head></html>"#
    ));
}
