use std::fs;

use tauri_app_lib::models::{RawHistoryMap, RawLongText, RawSearchProfile, RawUserInfo};
use tauri_app_lib::services::weibo_api::WeiboApiClient;

fn load_fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{}.json", env!("CARGO_MANIFEST_DIR"), name);
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read fixture: {}", path))
}

#[test]
fn test_parse_user_info() {
    let json = load_fixture("profile_info");
    let raw: RawUserInfo = serde_json::from_str(&json).expect("Failed to parse profile_info fixture");
    assert_eq!(raw.data.user.id, 1738498871);
    assert_eq!(raw.data.user.screen_name, "测试用户");
}

#[test]
fn test_parse_history_map() {
    let json = load_fixture("mbloghistory");
    let raw: RawHistoryMap = serde_json::from_str(&json).expect("Failed to parse mbloghistory fixture");
    assert_eq!(raw.data.get("2024"), Some(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]));
    assert_eq!(raw.data.get("2023"), Some(&vec![3, 5, 7, 9]));
}

#[test]
fn test_parse_search_profile() {
    let json = load_fixture("search_profile_page");
    let raw: RawSearchProfile = serde_json::from_str(&json).expect("Failed to parse search_profile fixture");
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

    let client = WeiboApiClient::new("test_cookie".to_string());
    let post = client.normalize_post(&list[0], "1738498871");

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

    let client = WeiboApiClient::new("test_cookie".to_string());
    let post = client.normalize_post(&list[1], "1738498871");

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

    let client = WeiboApiClient::new("test_cookie".to_string());
    let post = client.normalize_post(&list[0], "1738498871");

    assert!(post.images[0].original_url.starts_with("http"));
}
