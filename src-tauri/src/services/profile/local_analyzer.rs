use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, FixedOffset, Timelike};

use crate::models::WeiboPost;
use crate::utils::html::strip_html_tags;

use super::models::*;

pub struct LocalAnalyzerResult {
    pub basic_stats: BasicStats,
    pub content_analysis: ContentAnalysis,
    pub influence_metrics: InfluenceMetrics,
}

pub struct LocalAnalyzer;

impl LocalAnalyzer {
    pub fn analyze(posts: &[WeiboPost]) -> LocalAnalyzerResult {
        let total_posts = posts.len();
        let total_original = posts.iter().filter(|post| !post.is_repost).count();
        let total_reposts = total_posts.saturating_sub(total_original);

        let mut plain_texts = Vec::with_capacity(total_posts);
        let mut text_lengths = Vec::with_capacity(total_posts);
        let mut parsed_dates = Vec::with_capacity(total_posts);
        let mut active_date_counts: HashMap<String, usize> = HashMap::new();
        let mut hourly_distribution = vec![0usize; 24];
        let mut weekly_distribution = vec![0usize; 7];
        let mut monthly_counts: BTreeMap<(i32, u32), usize> = BTreeMap::new();
        let mut region_counts: HashMap<String, usize> = HashMap::new();
        let mut posts_with_images = 0usize;
        let mut hashtag_counts: HashMap<String, usize> = HashMap::new();
        let mut mention_counts: HashMap<String, usize> = HashMap::new();
        let mut emoji_counts: HashMap<String, usize> = HashMap::new();
        let mut total_hashtags = 0usize;
        let mut total_mentions = 0usize;

        for post in posts {
            let plain_text = strip_html_tags(&post.text);
            let text_length = plain_text.chars().count();
            text_lengths.push(text_length);

            if !post.images.is_empty() {
                posts_with_images += 1;
            }

            let hashtags = extract_hashtags(&plain_text);
            total_hashtags += hashtags.len();
            for hashtag in hashtags {
                *hashtag_counts.entry(hashtag).or_insert(0) += 1;
            }

            let mentions = extract_mentions(&plain_text);
            total_mentions += mentions.len();
            for mention in mentions {
                *mention_counts.entry(mention).or_insert(0) += 1;
            }

            for emoji in extract_emojis(&plain_text) {
                *emoji_counts.entry(emoji).or_insert(0) += 1;
            }

            if let Some(region) = post
                .region
                .as_deref()
                .map(|value| value.trim().trim_start_matches("发布于 ").trim())
                .filter(|value| !value.is_empty())
            {
                *region_counts.entry(region.to_string()).or_insert(0) += 1;
            }

            if let Some(date_time) = parse_post_datetime(&post.created_at) {
                let date_key = date_time.format("%Y-%m-%d").to_string();
                *active_date_counts.entry(date_key).or_insert(0) += 1;
                hourly_distribution[date_time.hour() as usize] += 1;
                weekly_distribution[date_time.weekday().num_days_from_monday() as usize] += 1;
                *monthly_counts
                    .entry((date_time.year(), date_time.month()))
                    .or_insert(0) += 1;
                parsed_dates.push(date_time);
            }

            plain_texts.push(plain_text);
        }

        let avg_text_length = if total_posts == 0 {
            0.0
        } else {
            text_lengths.iter().sum::<usize>() as f64 / total_posts as f64
        };

        let max_text_length = text_lengths.iter().copied().max().unwrap_or(0);
        let min_text_length = text_lengths.iter().copied().min().unwrap_or(0);

        let (posts_per_day, posts_per_week, posts_per_month) =
            posting_frequency(total_posts, &parsed_dates);

        let most_active_date = active_date_counts
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
            .map(|(date, _)| date.clone())
            .unwrap_or_default();

        let monthly_distribution = monthly_counts
            .into_iter()
            .map(|((year, month), count)| (format!("{year:04}-{month:02}"), count))
            .collect();

        let basic_stats = BasicStats {
            total_posts,
            total_original,
            total_reposts,
            avg_text_length,
            max_text_length,
            min_text_length,
            posts_per_day,
            posts_per_week,
            posts_per_month,
            total_active_days: active_date_counts.len(),
            most_active_date,
            active_hours: detect_active_hours(&hourly_distribution),
            hourly_distribution,
            weekly_distribution,
            monthly_distribution,
        };

        let content_analysis = ContentAnalysis {
            top_hashtags: sorted_frequency(hashtag_counts, 20),
            top_mentions: sorted_frequency(mention_counts, 20),
            avg_hashtags_per_post: ratio(total_hashtags, total_posts),
            avg_mentions_per_post: ratio(total_mentions, total_posts),
            posts_with_images_ratio: ratio(posts_with_images, total_posts),
            emoji_frequency: sorted_frequency(emoji_counts, 20),
        };

        let influence_metrics = InfluenceMetrics {
            source_distribution: Vec::new(),
            region_distribution: sorted_frequency(region_counts, 20),
            original_ratio: ratio(total_original, total_posts),
        };

        LocalAnalyzerResult {
            basic_stats,
            content_analysis,
            influence_metrics,
        }
    }
}

fn posting_frequency(
    total_posts: usize,
    parsed_dates: &[DateTime<FixedOffset>],
) -> (f64, f64, f64) {
    if total_posts == 0 || parsed_dates.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let earliest = parsed_dates.iter().min().copied();
    let latest = parsed_dates.iter().max().copied();

    match (earliest, latest) {
        (Some(start), Some(end)) => {
            let days = (end.date_naive() - start.date_naive()).num_days().max(0) + 1;
            let span_days = days.max(1) as f64;
            (
                total_posts as f64 / span_days,
                total_posts as f64 / (span_days / 7.0).max(1.0),
                total_posts as f64 / (span_days / 30.0).max(1.0),
            )
        }
        _ => (0.0, 0.0, 0.0),
    }
}

fn parse_post_datetime(created_at: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(created_at, "%a %b %e %H:%M:%S %z %Y").ok()
}

fn detect_active_hours(hourly_distribution: &[usize]) -> ActiveHours {
    if hourly_distribution.len() != 24 || hourly_distribution.iter().all(|count| *count == 0) {
        return ActiveHours {
            peak_start: 0,
            peak_end: 0,
        };
    }

    let window_size = 3usize;
    let mut best_start = 0usize;
    let mut best_score = 0usize;

    for start in 0..24usize {
        let score = (0..window_size)
            .map(|offset| hourly_distribution[(start + offset) % 24])
            .sum();
        if score > best_score {
            best_score = score;
            best_start = start;
        }
    }

    ActiveHours {
        peak_start: best_start as u8,
        peak_end: ((best_start + window_size - 1) % 24) as u8,
    }
}

fn extract_hashtags(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut hashtags = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] == '#' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '#' {
                end += 1;
            }
            if end < chars.len() {
                let value: String = chars[start..end]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string();
                if !value.is_empty() {
                    hashtags.push(value);
                }
                index = end + 1;
                continue;
            }
        }
        index += 1;
    }

    hashtags
}

fn extract_mentions(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut mentions = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] == '@' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && !is_mention_delimiter(chars[end]) {
                end += 1;
            }
            let mention = chars[start..end].iter().collect::<String>();
            let mention = mention.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && !is_cjk_char(ch) && ch != '_' && ch != '-'
            });
            if !mention.is_empty() {
                mentions.push(mention.to_string());
            }
            index = end;
            continue;
        }
        index += 1;
    }

    mentions
}

fn extract_emojis(text: &str) -> Vec<String> {
    text.chars()
        .filter(|ch| is_emoji_char(*ch))
        .map(|ch| ch.to_string())
        .collect()
}

fn sorted_frequency(counts: HashMap<String, usize>, limit: usize) -> Vec<(String, usize)> {
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    pairs.truncate(limit);
    pairs
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn is_mention_delimiter(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '@' | '#'
                | ','
                | '.'
                | '!'
                | '?'
                | ';'
                | ':'
                | '，'
                | '。'
                | '！'
                | '？'
                | '；'
                | '：'
                | '（'
                | '）'
                | '('
                | ')'
                | '['
                | ']'
                | '【'
                | '】'
                | '<'
                | '>'
                | '《'
                | '》'
                | '/'
                | '\\'
                | '"'
                | '\''
                | '“'
                | '”'
                | '‘'
                | '’'
        )
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF | 0x2A700..=0x2B73F
    )
}

fn is_emoji_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1F300..=0x1F5FF
            | 0x1F600..=0x1F64F
            | 0x1F680..=0x1F6FF
            | 0x1F700..=0x1F77F
            | 0x1F780..=0x1F7FF
            | 0x1F800..=0x1F8FF
            | 0x1F900..=0x1F9FF
            | 0x1FA70..=0x1FAFF
            | 0x2600..=0x26FF
            | 0x2700..=0x27BF
    )
}
