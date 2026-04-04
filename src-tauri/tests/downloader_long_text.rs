use tauri_app_lib::models::RawSearchProfile;
use tauri_app_lib::services::downloader::normalize_raw_post_for_export;

fn load_fixture() -> RawSearchProfile {
    serde_json::from_str(include_str!("fixtures/search_profile_page.json")).unwrap()
}

#[test]
fn expands_top_level_long_post_before_export() {
    let mut raw = load_fixture().data.list.unwrap().remove(0);
    raw.is_long_text = Some(true);
    raw.text = "<p>展开全文</p>".to_string();

    let normalized = normalize_raw_post_for_export(raw, Some("完整长微博正文".to_string()), None);

    assert_eq!(normalized.text, "完整长微博正文");
}

#[test]
fn expands_retweeted_long_post_before_export() {
    let mut raw = load_fixture().data.list.unwrap().remove(1);
    let retweeted = raw.retweeted_status.as_mut().expect("retweeted post should exist");
    retweeted.is_long_text = Some(true);
    retweeted.text = "<p>全文</p>".to_string();

    let normalized = normalize_raw_post_for_export(raw, None, Some("转发里的完整正文".to_string()));

    let nested = normalized.retweeted_status.expect("retweeted post should remain");
    assert_eq!(nested.text, "转发里的完整正文");
}

#[test]
fn keeps_original_text_when_no_long_text_is_available() {
    let raw = load_fixture().data.list.unwrap().remove(0);

    let normalized = normalize_raw_post_for_export(raw.clone(), None, None);

    assert_eq!(normalized.text, raw.text);
}

#[test]
fn expands_even_when_raw_text_does_not_contain_expand_keyword() {
    let mut raw = load_fixture().data.list.unwrap().remove(0);
    raw.is_long_text = Some(true);
    raw.text = "<p>截断内容但没有关键字</p>".to_string();

    let normalized = normalize_raw_post_for_export(raw, Some("完整正文没有展开关键字也要替换".to_string()), None);

    assert_eq!(normalized.text, "完整正文没有展开关键字也要替换");
}
