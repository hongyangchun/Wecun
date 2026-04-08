use std::fs;

use tauri_app_lib::models::{
    RawFavProfile, RawHistoryMap, RawLongText, RawSearchProfile, RawUserInfo,
};
use tauri_app_lib::services::weibo_api::WeiboApiClient;

fn load_fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read fixture: {}", path))
}

#[test]
fn test_parse_user_info() {
    let json = load_fixture("profile_info");
    let raw: RawUserInfo =
        serde_json::from_str(&json).expect("Failed to parse profile_info fixture");
    assert_eq!(raw.data.user.id, 1738498871);
    assert_eq!(raw.data.user.screen_name, "测试用户");
}

#[test]
fn test_parse_history_map() {
    let json = load_fixture("mbloghistory");
    let raw: RawHistoryMap =
        serde_json::from_str(&json).expect("Failed to parse mbloghistory fixture");
    assert_eq!(
        raw.data.get("2024"),
        Some(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])
    );
    assert_eq!(raw.data.get("2023"), Some(&vec![3, 5, 7, 9]));
}

#[test]
fn test_parse_search_profile() {
    let json = load_fixture("search_profile_page");
    let raw: RawSearchProfile =
        serde_json::from_str(&json).expect("Failed to parse search_profile fixture");
    assert_eq!(raw.data.total, Some(2));
    let list = raw.data.list.expect("list should exist");
    assert_eq!(list.len(), 2);

    assert_eq!(list[0].mblogid, "Oabc123def");
    assert!(list[0].retweeted_status.is_none());

    assert!(list[1].retweeted_status.is_some());
    assert_eq!(
        list[1]
            .retweeted_status
            .as_ref()
            .unwrap()
            .user
            .as_ref()
            .unwrap()
            .screen_name,
        "原作者"
    );
}

#[test]
fn test_normalize_post() {
    let json = load_fixture("search_profile_page");
    let raw: RawSearchProfile = serde_json::from_str(&json).unwrap();
    let list = raw.data.list.unwrap();

    let post = WeiboApiClient::<tauri::Wry>::normalize_post(&list[0], "1738498871");

    assert_eq!(post.mblogid, "Oabc123def");
    assert_eq!(post.author, "测试用户");
    assert!(!post.is_repost);
    assert_eq!(post.images.len(), 1);
    assert_eq!(post.images[0].width, 1080);
    assert_eq!(post.images[0].height, 1080);
    assert!(post.source_url.contains("weibo.com"));
}

#[test]
fn test_normalize_repost() {
    let json = load_fixture("search_profile_page");
    let raw: RawSearchProfile = serde_json::from_str(&json).unwrap();
    let list = raw.data.list.unwrap();

    let post = WeiboApiClient::<tauri::Wry>::normalize_post(&list[1], "1738498871");

    assert!(post.is_repost);
    assert_eq!(post.repost_user, Some("原作者".to_string()));
}

#[test]
fn test_parse_long_text() {
    let json = load_fixture("longtext");
    let raw: RawLongText = serde_json::from_str(&json).expect("Failed to parse longtext fixture");
    assert!(raw.data.long_text_content.contains("长微博"));
}

#[test]
fn test_image_url_https_prefix() {
    let json = load_fixture("search_profile_page");
    let raw: RawSearchProfile = serde_json::from_str(&json).unwrap();
    let list = raw.data.list.unwrap();

    let post = WeiboApiClient::<tauri::Wry>::normalize_post(&list[0], "1738498871");

    assert!(post.images[0].original_url.starts_with("http"));
}

#[test]
fn test_parse_camelcase_is_long_text_field() {
    let raw: tauri_app_lib::models::RawPost = serde_json::from_str(
        r#"{
            "mblogid": "Qdemo123",
            "created_at": "Thu Apr 02 09:30:36 +0800 2026",
            "text": "<p>截断内容</p>",
            "isLongText": true,
            "user": { "id": 1, "screen_name": "测试用户" }
        }"#,
    )
    .unwrap();

    assert_eq!(raw.is_long_text, Some(true));
}

#[test]
fn test_parse_favorites_page() {
    let json = load_fixture("favorites_page");
    let raw: RawFavProfile =
        serde_json::from_str(&json).expect("Failed to parse favorites_page fixture");
    assert_eq!(raw.ok, Some(1));
    let list = raw.data.expect("data should exist");
    assert_eq!(list.len(), 2);

    assert_eq!(list[0].mblogid, "Ofav001abc");
    assert_eq!(list[0].user.as_ref().unwrap().screen_name, "被收藏的博主");
    assert!(list[0].retweeted_status.is_none());

    assert!(list[1].retweeted_status.is_some());
    assert_eq!(
        list[1]
            .retweeted_status
            .as_ref()
            .unwrap()
            .user
            .as_ref()
            .unwrap()
            .screen_name,
        "原作者"
    );
}

#[test]
fn test_normalize_favorites_post_with_empty_uid() {
    let json = load_fixture("favorites_page");
    let raw: RawFavProfile = serde_json::from_str(&json).unwrap();
    let list = raw.data.unwrap();

    let post = WeiboApiClient::<tauri::Wry>::normalize_post(&list[0], "");

    assert_eq!(post.mblogid, "Ofav001abc");
    assert_eq!(post.author, "被收藏的博主");
    assert!(post.source_url.contains("2166767661"));
    assert!(post.source_url.contains("Ofav001abc"));
}
