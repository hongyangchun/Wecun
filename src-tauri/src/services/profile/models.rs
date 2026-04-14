use serde::{Deserialize, Serialize};

/// Complete profile analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResult {
    pub basic_stats: BasicStats,
    pub content_analysis: ContentAnalysis,
    pub influence_metrics: InfluenceMetrics,
    pub deep_profile: Option<DeepProfile>,
}

/// Basic statistics computed locally from post data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasicStats {
    pub total_posts: usize,
    pub total_original: usize,
    pub total_reposts: usize,
    pub avg_text_length: f64,
    pub max_text_length: usize,
    pub min_text_length: usize,
    pub posts_per_day: f64,
    pub posts_per_week: f64,
    pub posts_per_month: f64,
    pub total_active_days: usize,
    pub most_active_date: String,
    pub active_hours: ActiveHours,
    pub hourly_distribution: Vec<usize>,
    pub weekly_distribution: Vec<usize>,
    /// Month string → post count, sorted chronologically.
    pub monthly_distribution: Vec<(String, usize)>,
}

/// Most active time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveHours {
    /// Hour (0-23) where posting peaks begin.
    pub peak_start: u8,
    /// Hour where posting peaks end.
    pub peak_end: u8,
}

/// Content analysis from local text processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalysis {
    /// Top hashtags with counts.
    pub top_hashtags: Vec<(String, usize)>,
    /// Top @mentions with counts.
    pub top_mentions: Vec<(String, usize)>,
    /// Average hashtags per post.
    pub avg_hashtags_per_post: f64,
    /// Average mentions per post.
    pub avg_mentions_per_post: f64,
    /// Ratio of posts that contain images.
    pub posts_with_images_ratio: f64,
    /// Most frequently used emojis with counts.
    pub emoji_frequency: Vec<(String, usize)>,
}

/// Influence metrics derived from post metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfluenceMetrics {
    /// Post source distribution (e.g. "iPhone客户端", "Android").
    pub source_distribution: Vec<(String, usize)>,
    /// Region distribution from post region tags.
    pub region_distribution: Vec<(String, usize)>,
    /// Ratio of original vs repost content.
    pub original_ratio: f64,
}

/// AI-generated deep profile analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepProfile {
    pub personal_info: PersonalInfo,
    pub personality: PersonalityAnalysis,
    pub interests: InterestAnalysis,
    pub values: ValueAnalysis,
    pub keywords: KeywordAnalysis,
    pub sentiment: AiSentimentAnalysis,
    /// Interesting insights discovered by AI
    pub interesting_insights: Option<InterestingInsights>,
}

/// Interesting and surprising insights about the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestingInsights {
    /// Contradictions between stated beliefs and behaviors
    pub contradictions: Vec<Contradiction>,
    /// Growth trajectory over time
    pub growth_arc: Option<GrowthArc>,
    /// Social role positioning
    pub social_roles: Option<SocialRoles>,
    /// Hidden patterns only AI could discover
    pub hidden_patterns: Vec<String>,
}

/// A contradiction between what someone says and what they do.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contradiction {
    /// Description of the contradiction
    pub what: String,
    /// Evidence from posts supporting this observation
    pub evidence: Vec<String>,
}

/// Growth trajectory over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthArc {
    /// State at an earlier time
    pub then: TimePeriodState,
    /// Current state
    pub now: TimePeriodState,
    /// Narrative summary of the change
    pub narrative: String,
}

/// State description for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimePeriodState {
    /// Time period description
    pub period: String,
    /// Keywords characterizing this period
    pub keywords: Vec<String>,
    /// A representative post from this period
    pub typical_post: Option<String>,
}

/// Social role positioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialRoles {
    /// Role in friend circle
    pub in_friend_circle: String,
    /// Role in comments sections
    pub in_comments: String,
    /// Role during crisis situations
    pub in_crisis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalInfo {
    pub estimated_age_range: String,
    pub zodiac_sign: String,
    pub mbti_type: String,
    pub gender: String,
    pub possible_cities: Vec<String>,
    pub possible_occupation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityAnalysis {
    pub traits: Vec<String>,
    pub communication_style: String,
    pub social_orientation: String,
    pub humor_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestAnalysis {
    pub music: Vec<String>,
    pub books: Vec<String>,
    pub movies: Vec<String>,
    pub hobbies: Vec<String>,
    pub sports: Vec<String>,
    pub food: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueAnalysis {
    pub political_stance: String,
    pub philosophy: String,
    pub worldview: String,
    pub core_values: Vec<String>,
    pub attitude_toward_life: String,
}

/// Keywords grouped by thematic categories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicClusters {
    /// Work/career related keywords
    pub work: Vec<String>,
    /// Daily life related keywords
    pub life: Vec<String>,
    /// Entertainment related keywords
    pub entertainment: Vec<String>,
    /// Opinion/viewpoint related keywords
    pub opinion: Vec<String>,
    /// Emotional expression related keywords
    pub emotion: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordAnalysis {
    /// Top 15-20 most meaningful keywords based on semantic significance
    pub top_keywords: Vec<String>,
    /// Keywords grouped by topic/theme
    pub topic_clusters: TopicClusters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSentimentAnalysis {
    pub overall_sentiment: String,
    pub emotional_stability: String,
    pub emotional_triggers: Vec<String>,
    pub happiness_index: f64,
}
