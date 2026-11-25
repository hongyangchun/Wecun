use std::path::Path;

use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};

use crate::models::RawPost;

pub(crate) fn needs_long_text(raw: &RawPost) -> bool {
    raw.is_long_text == Some(true) || raw.text.contains("展开") || raw.text.contains("全文")
}

/// Count visible characters in text, excluding HTML tags
/// This is used for the min_text_length filter
pub(crate) fn count_visible_chars(html: &str) -> usize {
    let mut in_tag = false;
    let mut count = 0;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => {
                // Only count non-whitespace visible characters
                if !c.is_whitespace() {
                    count += 1;
                }
            }
            _ => {}
        }
    }

    count
}

pub fn parse_weibo_datetime(created_at: &str) -> Option<DateTime<FixedOffset>> {
    let created_at = created_at.trim();
    if created_at.is_empty() {
        return None;
    }

    let offset = FixedOffset::east_opt(8 * 3600)?;

    // Standard Weibo format: "Wed Mar 19 09:53:32 +0800 2014"
    if let Ok(dt) = DateTime::parse_from_str(created_at, "%a %b %e %H:%M:%S %z %Y") {
        return Some(dt);
    }

    // Format: "2024-03-19 09:53:32"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S") {
        if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
            return Some(dt_with_tz);
        }
    }

    // Relative formats (often used in some API views)
    let now_utc = Utc::now();

    if created_at.contains("秒前") {
        if let Some(n) = created_at.split('秒').next().and_then(|s| s.trim().parse::<i64>().ok()) {
            return Some((now_utc - chrono::Duration::seconds(n)).with_timezone(&offset));
        }
    }
    if created_at.contains("分钟前") {
        if let Some(n) = created_at.split('分').next().and_then(|s| s.trim().parse::<i64>().ok()) {
            return Some((now_utc - chrono::Duration::minutes(n)).with_timezone(&offset));
        }
    }
    if created_at.contains("小时前") {
        if let Some(n) = created_at.split('小').next().and_then(|s| s.trim().parse::<i64>().ok()) {
            return Some((now_utc - chrono::Duration::hours(n)).with_timezone(&offset));
        }
    }
    if created_at.contains("今天") {
        if let Some(hm) = created_at.split_whitespace().nth(1) {
            let mut parts = hm.split(':');
            let h = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let m = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

            let bj_now = now_utc.with_timezone(&offset);
            if let Some(dt) = bj_now.date_naive().and_hms_opt(h, m, 0) {
                if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                    return Some(dt_with_tz);
                }
            }
        }
    }
    if created_at.contains("昨天") {
        if let Some(hm) = created_at.split_whitespace().nth(1) {
            let mut parts = hm.split(':');
            let h = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let m = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

            let bj_yesterday = (now_utc - chrono::Duration::days(1)).with_timezone(&offset);
            if let Some(dt) = bj_yesterday.date_naive().and_hms_opt(h, m, 0) {
                if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                    return Some(dt_with_tz);
                }
            }
        }
    }

    // Format: "04-07 10:00" (Current year)
    if created_at.len() == 11 && created_at.chars().nth(2) == Some('-') && created_at.chars().nth(5) == Some(' ') {
        let bj_now = now_utc.with_timezone(&offset);
        let year = bj_now.year();
        let full_date = format!("{}-{}", year, created_at);
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&full_date, "%Y-%m-%d %H:%M") {
            if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                return Some(dt_with_tz);
            }
        }
    }

    // Fallback for YYYY-MM-DD
    if let Some(date_prefix) = created_at.get(..10) {
        if date_prefix.contains('-') {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_prefix, "%Y-%m-%d") {
                if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                    if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                        return Some(dt_with_tz);
                    }
                }
            }
        }
    }

    None
}

pub(crate) fn parse_weibo_date_to_timestamp(created_at: &str) -> i64 {
    parse_weibo_datetime(created_at)
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

pub fn normalize_raw_post_for_export(
    mut raw: RawPost,
    top_level_long_text: Option<String>,
    retweeted_long_text: Option<String>,
) -> RawPost {
    if let Some(text) = top_level_long_text.filter(|text| !text.is_empty()) {
        raw.text = text;
    }

    if let Some(retweeted) = raw.retweeted_status.as_mut() {
        if let Some(text) = retweeted_long_text.filter(|text| !text.is_empty()) {
            retweeted.text = text;
        }
    }

    raw
}

pub(crate) fn image_extension(url: &str) -> String {
    let cleaned = url.split(['?', '#']).next().unwrap_or(url);

    Path::new(cleaned)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "jpg".to_string())
}

pub(crate) fn format_date_range_label(start_timestamp: Option<i64>, end_timestamp: Option<i64>) -> String {
    match (
        start_timestamp.and_then(timestamp_to_local_date),
        end_timestamp.and_then(timestamp_to_local_date),
    ) {
        (Some(start), Some(end)) => format!("{start}至{end}"),
        _ => "全部时间".to_string(),
    }
}

fn timestamp_to_local_date(timestamp: i64) -> Option<String> {
    let offset = chrono::FixedOffset::east_opt(8 * 3600)?;
    Some(
        Utc.timestamp_opt(timestamp, 0)
            .single()?
            .with_timezone(&offset)
            .format("%Y-%m-%d")
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_parsing_standard_format() {
        // Standard Weibo format: "Wed Mar 19 09:53:32 +0800 2014"
        let ts = parse_weibo_date_to_timestamp("Wed Mar 19 09:53:32 +0800 2014");
        assert!(ts > 0, "Should parse correctly");
        assert_eq!(ts, 1395194012);
    }

    #[test]
    fn test_date_parsing_iso_format() {
        // Format: "2024-03-19 09:53:32"
        let ts = parse_weibo_date_to_timestamp("2024-03-19 09:53:32");
        assert!(ts > 0, "Should parse ISO-like format");
        assert_eq!(ts, 1710813212);
    }

    #[test]
    fn test_date_parsing_date_only() {
        // Format: "2024-03-19"
        let ts = parse_weibo_date_to_timestamp("2024-03-19");
        assert!(ts > 0, "Should parse YYYY-MM-DD");
        assert_eq!(ts, 1710777600);
    }

    #[test]
    fn test_date_parsing_relative_seconds() {
        let ts = parse_weibo_date_to_timestamp("30秒前");
        let now = Utc::now().timestamp();
        assert!(ts > 0, "Should parse relative seconds");
        assert!(ts <= now && ts >= now - 60, "Should be within last minute");
    }

    #[test]
    fn test_date_parsing_relative_minutes() {
        let ts = parse_weibo_date_to_timestamp("5分钟前");
        let now = Utc::now().timestamp();
        assert!(ts > 0, "Should parse relative minutes");
        assert!(ts <= now - 240 && ts >= now - 360, "Should be about 5 minutes ago");
    }

    #[test]
    fn test_date_parsing_today() {
        let ts = parse_weibo_date_to_timestamp("今天 00:00");
        assert!(ts > 0, "Should parse 'today' format");
        let now = Utc::now().timestamp();
        assert!(ts <= now, "Today 00:00 should not be in the future");
        assert!(ts > now - 86400, "Today 00:00 should be within the last 24 hours");
    }

    #[test]
    fn test_date_parsing_yesterday() {
        let ts = parse_weibo_date_to_timestamp("昨天 10:00");
        assert!(ts > 0, "Should parse 'yesterday' format");
        // Just verify it's in the past
        let now = Utc::now().timestamp();
        assert!(ts < now, "Yesterday should be in the past");
        assert!(ts > now - 172800, "Yesterday should be within last 48 hours");
    }

    #[test]
    fn test_date_parsing_month_day_format() {
        // Format: "04-07 10:00" (Current year)
        let ts = parse_weibo_date_to_timestamp("04-07 10:00");
        assert!(ts > 0, "Should parse MM-DD HH:MM format");
    }

    #[test]
    fn test_date_parsing_empty_string() {
        let ts = parse_weibo_date_to_timestamp("");
        assert_eq!(ts, 0, "Empty string should return 0");
    }

    #[test]
    fn test_date_parsing_invalid_format() {
        let ts = parse_weibo_date_to_timestamp("invalid date format");
        assert_eq!(ts, 0, "Invalid format should return 0");
    }

    #[test]
    fn test_count_visible_chars() {
        let html = "<p>Hello <b>world</b></p>";
        assert_eq!(count_visible_chars(html), 10, "Should count visible characters excluding tags");

        let plain = "Hello world";
        assert_eq!(count_visible_chars(plain), 10, "Should count plain text");

        let with_spaces = "Hello   world";
        assert_eq!(count_visible_chars(with_spaces), 10, "Should not count whitespace");

        let empty = "";
        assert_eq!(count_visible_chars(empty), 0, "Empty string should return 0");
    }

    #[test]
    fn test_needs_long_text() {
        let raw_with_flag = RawPost {
            is_long_text: Some(true),
            text: "Short".to_string(),
            ..Default::default()
        };
        assert!(needs_long_text(&raw_with_flag), "Should need long text when flag is true");

        let raw_with_expand = RawPost {
            is_long_text: Some(false),
            text: "点击展开全文".to_string(),
            ..Default::default()
        };
        assert!(needs_long_text(&raw_with_expand), "Should need long text when text contains '展开'");

        let raw_normal = RawPost {
            is_long_text: Some(false),
            text: "Normal short post".to_string(),
            ..Default::default()
        };
        assert!(!needs_long_text(&raw_normal), "Should not need long text for normal post");
    }

    #[test]
    fn test_normalize_raw_post_for_export() {
        let raw = RawPost {
            text: "Original".to_string(),
            ..Default::default()
        };
        let result = normalize_raw_post_for_export(raw, Some("Long text".to_string()), None);
        assert_eq!(result.text, "Long text", "Should replace text with long text");

        let raw2 = RawPost {
            text: "Original".to_string(),
            ..Default::default()
        };
        let result2 = normalize_raw_post_for_export(raw2, None, None);
        assert_eq!(result2.text, "Original", "Should keep original text when no long text");

        let raw3 = RawPost {
            text: "Original".to_string(),
            retweeted_status: Some(Box::new(RawPost {
                text: "Retweeted".to_string(),
                ..Default::default()
            })),
            ..Default::default()
        };
        let result3 = normalize_raw_post_for_export(raw3, None, Some("Long retweeted".to_string()));
        assert_eq!(result3.retweeted_status.as_ref().unwrap().text, "Long retweeted", "Should replace retweeted text");
    }

    #[test]
    fn test_image_extension_extraction() {
        assert_eq!(image_extension("https://example.com/image.jpg"), "jpg");
        assert_eq!(image_extension("https://example.com/image.png?size=large"), "png");
        assert_eq!(image_extension("https://example.com/image.jpeg#fragment"), "jpeg");
        assert_eq!(image_extension("https://example.com/image"), "jpg", "Should default to jpg when no extension");
        assert_eq!(image_extension(""), "jpg", "Should default to jpg for empty string");
    }

    #[test]
    fn test_format_date_range_label() {
        assert_eq!(
            format_date_range_label(Some(1704067200), Some(1706745600)),
            "2024-01-01至2024-02-01",
            "Should format date range"
        );
        assert_eq!(
            format_date_range_label(None, None),
            "全部时间",
            "Should return default label when no dates"
        );
        assert_eq!(
            format_date_range_label(Some(1704067200), None),
            "全部时间",
            "Should return default when end is missing"
        );
    }
}
