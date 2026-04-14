use std::path::Path;

use tokio::fs;

use crate::error::AppError;
use crate::services::cache;
use crate::services::profile::models::ProfileResult;

pub struct ProfileExportService;

impl ProfileExportService {
    pub fn new() -> Self {
        Self
    }

    pub async fn export(&self, output_dir: &str, profile: &ProfileResult) -> Result<String, AppError> {
        let bundle = cache::load_cache_bundle(output_dir).await?;
        let first_post = bundle
            .posts
            .first()
            .ok_or_else(|| AppError::ApiError("没有可导出的微博数据".to_string()))?;

        let screen_name = if first_post.author.trim().is_empty() {
            "微博用户"
        } else {
            first_post.author.trim()
        };
        let uid = extract_uid_from_source_url(&first_post.source_url);

        let target_dir = Path::new(output_dir);
        fs::create_dir_all(target_dir).await.map_err(AppError::Io)?;

        // Sanitize filename for filesystem compatibility
        let safe_screen_name = sanitize_filename(screen_name);
        let file_name = format!("{}_画像分析.md", safe_screen_name);
        let profile_path = target_dir.join(&file_name);

        fs::write(&profile_path, build_profile_analysis_markdown(screen_name, &uid, profile))
            .await
            .map_err(AppError::Io)?;

        Ok(profile_path.to_string_lossy().to_string())
    }
}

fn build_profile_analysis_markdown(screen_name: &str, uid: &str, profile: &ProfileResult) -> String {
    let deep = profile.deep_profile.as_ref();
    let top_hashtags = format_ranked_pairs(&profile.content_analysis.top_hashtags, 10, "暂无话题标签");
    let top_mentions = format_ranked_pairs(&profile.content_analysis.top_mentions, 10, "暂无互动对象");
    let regions = format_ranked_pairs(&profile.influence_metrics.region_distribution, 10, "暂无地域线索");

    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("name: \"{}\"\n", yaml_escape(screen_name)));
    content.push_str(&format!("uid: \"{}\"\n", yaml_escape(uid)));
    content.push_str("---\n\n");
    content.push_str(&format!("# {} 的画像分析报告\n\n", screen_name));

    content.push_str("## 基础信息\n\n");
    content.push_str(&format!("- 微博总数：{}\n", profile.basic_stats.total_posts));
    content.push_str(&format!("- 原创占比：{:.1}%\n", profile.influence_metrics.original_ratio * 100.0));
    content.push_str(&format!("- 活跃天数：{}\n", profile.basic_stats.total_active_days));
    content.push_str(&format!("- 最活跃日期：{}\n", fallback_text(&profile.basic_stats.most_active_date, "未知")));
    content.push_str(&format!(
        "- 活跃时段：{:02}:00 - {:02}:59\n",
        profile.basic_stats.active_hours.peak_start, profile.basic_stats.active_hours.peak_end
    ));
    content.push_str(&format!("- 主要地域：{}\n\n", regions));

    content.push_str("## 写作风格\n\n");
    content.push_str(&format!(
        "- 沟通方式：{}\n",
        deep.map(|value| value.personality.communication_style.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("以微博原文为准")
    ));
    content.push_str(&format!(
        "- 幽默风格：{}\n\n",
        deep.map(|value| value.personality.humor_style.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("暂无明确结论")
    ));

    content.push_str("## 内容特征\n\n");
    content.push_str(&format!("- 热门话题：{}\n", top_hashtags));
    content.push_str(&format!("- 常互动对象：{}\n", top_mentions));
    content.push_str(&format!(
        "- 配图微博占比：{:.1}%\n\n",
        profile.content_analysis.posts_with_images_ratio * 100.0
    ));

    content.push_str("## 个人画像\n\n");
    content.push_str(&format!(
        "- 年龄区间：{}\n",
        deep.map(|value| value.personal_info.estimated_age_range.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("未知")
    ));
    content.push_str(&format!(
        "- 性别倾向：{}\n",
        deep.map(|value| value.personal_info.gender.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("未知")
    ));
    content.push_str(&format!(
        "- 星座：{}\n",
        deep.map(|value| value.personal_info.zodiac_sign.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("未知")
    ));
    content.push_str(&format!(
        "- MBTI：{}\n",
        deep.map(|value| value.personal_info.mbti_type.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("未知")
    ));
    content.push_str(&format!(
        "- 可能城市：{}\n",
        deep.map(|value| format_string_list(&value.personal_info.possible_cities, "未知"))
            .unwrap_or_else(|| "未知".to_string())
    ));
    content.push_str(&format!(
        "- 可能职业：{}\n",
        deep.map(|value| value.personal_info.possible_occupation.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("未知")
    ));
    content.push_str(&format!(
        "- 核心人格特质：{}\n\n",
        deep.map(|value| format_string_list(&value.personality.traits, "待 AI 分析补充"))
            .unwrap_or_else(|| "待 AI 分析补充".to_string())
    ));

    content.push_str("## 兴趣偏好\n\n");
    if let Some(deep) = deep {
        content.push_str(&format!("- 音乐：{}\n", format_string_list(&deep.interests.music, "暂无明显偏好")));
        content.push_str(&format!("- 书籍：{}\n", format_string_list(&deep.interests.books, "暂无明显偏好")));
        content.push_str(&format!("- 电影：{}\n", format_string_list(&deep.interests.movies, "暂无明显偏好")));
        content.push_str(&format!("- 爱好：{}\n", format_string_list(&deep.interests.hobbies, "暂无明显偏好")));
        content.push_str(&format!("- 运动：{}\n", format_string_list(&deep.interests.sports, "暂无明显偏好")));
        content.push_str(&format!("- 食物：{}\n", format_string_list(&deep.interests.food, "暂无明显偏好")));
    } else {
        content.push_str("- 待 AI 分析补充兴趣偏好信息。\n");
    }
    content.push_str("\n");

    content.push_str("## 价值观倾向\n\n");
    content.push_str(&format!(
        "- 核心价值：{}\n",
        deep.map(|value| format_string_list(&value.values.core_values, "待 AI 分析补充"))
            .unwrap_or_else(|| "待 AI 分析补充".to_string())
    ));
    content.push_str(&format!(
        "- 世界观：{}\n",
        deep.map(|value| value.values.worldview.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("待 AI 分析补充")
    ));
    content.push_str(&format!(
        "- 政治倾向：{}\n",
        deep.map(|value| value.values.political_stance.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("未知")
    ));
    content.push_str(&format!(
        "- 人生哲学：{}\n",
        deep.map(|value| value.values.philosophy.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("待 AI 分析补充")
    ));
    content.push_str(&format!(
        "- 生活态度：{}\n\n",
        deep.map(|value| value.values.attitude_toward_life.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("待 AI 分析补充")
    ));

    content.push_str("## 情绪特征\n\n");
    content.push_str(&format!(
        "- 整体情绪：{}\n",
        deep.map(|value| value.sentiment.overall_sentiment.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("待 AI 分析补充")
    ));
    content.push_str(&format!(
        "- 情绪稳定性：{}\n",
        deep.map(|value| value.sentiment.emotional_stability.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("待 AI 分析补充")
    ));
    content.push_str(&format!(
        "- 情绪触发因素：{}\n",
        deep.map(|value| format_string_list(&value.sentiment.emotional_triggers, "暂无明显线索"))
            .unwrap_or_else(|| "暂无明显线索".to_string())
    ));
    content.push_str(&format!(
        "- 幸福指数：{}\n",
        deep.map(|value| format!("{:.1}/100", value.sentiment.happiness_index))
            .unwrap_or_else(|| "待 AI 分析补充".to_string())
    ));
    content.push_str("\n");

    content.push_str("---\n\n");
    content.push_str("*本报告由微存 Wecun 生成，基于微博内容的AI推测分析，仅供参考。*\n");

    content
}

fn format_ranked_pairs(items: &[(String, usize)], limit: usize, fallback: &str) -> String {
    if items.is_empty() {
        return fallback.to_string();
    }

    items.iter()
        .take(limit)
        .map(|(value, count)| format!("{}({})", value, count))
        .collect::<Vec<_>>()
        .join("、")
}

fn format_string_list(items: &[String], fallback: &str) -> String {
    if items.is_empty() {
        fallback.to_string()
    } else {
        items.join("、")
    }
}

fn yaml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn fallback_text<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn extract_uid_from_source_url(source_url: &str) -> String {
    source_url
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .find(|segment| segment.chars().all(|ch| ch.is_ascii_digit()))
        .map(|segment| segment.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Sanitize a screen name for use as a filename.
/// Replaces invalid filesystem characters with underscores.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || matches!(ch, '—'|'-'|'_') {
                ch
            } else if is_cjk_char(ch) {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF | 0x2A700..=0x2B73F
    )
}
