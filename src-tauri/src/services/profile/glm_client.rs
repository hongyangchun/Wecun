use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::WeiboPost;
use crate::utils::html::strip_html_tags;

use super::models::*;

const API_URL: &str = "https://api.deepseek.com/chat/completions";
const MODEL: &str = "deepseek-chat";
const API_KEY_FILE: &str = ".ai-api-key";
const MAX_SAMPLE_POSTS: usize = 200;
const MAX_CONTENT_CHARS: usize = 40_000;
const SYSTEM_PROMPT: &str = r#"你是一名擅长做社交媒体人物画像的分析师，擅长发现有趣、细腻、让人会心一笑的洞察。你会根据微博样本输出一个严格合法的 JSON 对象，字段必须完整且与下面结构完全一致，字段名必须使用 camelCase，不要输出 Markdown、解释或代码块。

返回 JSON 结构：
{
  "personalInfo": {
    "estimatedAgeRange": "",
    "zodiacSign": "",
    "mbtiType": "",
    "gender": "",
    "possibleCities": [],
    "possibleOccupation": ""
  },
  "personality": {
    "traits": [],
    "communicationStyle": "",
    "socialOrientation": "",
    "humorStyle": ""
  },
  "interests": {
    "music": [],
    "books": [],
    "movies": [],
    "hobbies": [],
    "sports": [],
    "food": []
  },
  "values": {
    "politicalStance": "",
    "philosophy": "",
    "worldview": "",
    "coreValues": [],
    "attitudeTowardLife": ""
  },
  "keywords": {
    "topKeywords": [],
    "topicClusters": {
      "work": [],
      "life": [],
      "entertainment": [],
      "opinion": [],
      "emotion": []
    }
  },
  "sentiment": {
    "overallSentiment": "",
    "emotionalStability": "",
    "emotionalTriggers": [],
    "happinessIndex": 0.0
  },
  "interestingInsights": {
    "contradictions": [
      {
        "what": "矛盾点的简短描述，比如'自称社恐但频繁组局'",
        "evidence": ["支持这一观察的具体微博内容引用1", "引用2"]
      }
    ],
    "growthArc": {
      "then": {
        "period": "时间描述，如'三年前'或'早期'",
        "keywords": ["关键词1", "关键词2", "关键词3"],
        "typicalPost": "该时期的一条代表性微博原文"
      },
      "now": {
        "period": "时间描述，如'最近'或'现在'",
        "keywords": ["关键词1", "关键词2", "关键词3"],
        "typicalPost": "该时期的一条代表性微博原文"
      },
      "narrative": "用一句话概括这个人的成长变化，要有故事感，比如'从锐利的观察者变成温柔的记录者'"
    },
    "socialRoles": {
      "inFriendCircle": "在朋友圈里的角色，比如'被大家当成百科全书'或'组局担当'",
      "inComments": "在别人评论区的风格，比如'神回复担当'或'总是暖心的鼓励者'",
      "inCrisis": "出事时的反应模式，比如'最先站出来组织的人'或'默默提供帮助的人'"
    },
    "hiddenPatterns": [
      "只有仔细观察才能发现的模式1，比如'只在周二晚上发深度思考'",
      "模式2，比如'提到某个特定话题时语气会明显变化'",
      "模式3，比如'字数超过80字时一定是情绪到了'"
    ]
  }
}

分析要求：
1. 所有字段必须存在，不确定时使用"未知"、空数组或保守描述。
2. happinessIndex 返回 0 到 100 的浮点数。
3. 推断必须基于样本内容，避免夸张结论。
4. contradictions 要找真实的矛盾，不要编造，如果找不到明显的矛盾就返回空数组。
5. growthArc 要能看出时间上的变化，如果样本时间跨度太短或变化不明显，narrative 可以写"时间跨度不足以判断成长轨迹"。
6. socialRoles 要基于实际的社交行为模式，比如转发/评论/@他人的频率和内容。
7. hiddenPatterns 要找那些只有 AI 通过大量数据才能发现的模式，比如特定时间、特定话题、特定表达方式的相关性。
8. 关键词提取要求：
   - topKeywords 返回 15-20 个最有意义的关键词，基于语义重要性而非简单词频
   - 过滤掉所有停用词、语气词、助词、代词、连词等
   - 优先选择名词、有实际意义的词汇
   - topicClusters 按主题分组，每组 3-5 个关键词
   - 工作类：职业、技能、项目相关
   - 生活类：日常、家庭、健康相关
   - 娱乐类：游戏、影视、音乐相关
   - 观点类：价值观、看法、态度相关
   - 情感类：情绪表达、心理状态相关
9. 只返回 JSON 对象本身。"#;

pub struct GlmClient {
    client: Client,
    api_key: String,
}

impl GlmClient {
    pub fn new() -> Self {
        let api_key = load_api_key();

        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    pub async fn analyze(&self, posts: &[WeiboPost]) -> Result<DeepProfile, String> {
        let sampled = self.sample_posts(posts, MAX_SAMPLE_POSTS);
        let content = self.build_content(&sampled);
        let response = self.call_api(&content).await?;
        let normalized = normalize_json_response(&response);

        serde_json::from_str::<DeepProfile>(&normalized)
            .map_err(|error| format!("解析AI分析结果失败: {error}"))
    }

    fn sample_posts<'a>(&self, posts: &'a [WeiboPost], max: usize) -> Vec<&'a WeiboPost> {
        if posts.len() <= max {
            posts.iter().collect()
        } else {
            let step = posts.len() as f64 / max as f64;
            (0..max)
                .map(|index| {
                    let sample_index = ((index as f64 * step).floor() as usize).min(posts.len() - 1);
                    &posts[sample_index]
                })
                .collect()
        }
    }

    fn build_content(&self, posts: &[&WeiboPost]) -> String {
        let mut content = String::from("微博内容样本：\n---\n");

        for (index, post) in posts.iter().enumerate() {
            let plain_text = strip_html_tags(&post.text).replace('\n', " ");
            let date_label = format_post_date(&post.created_at);
            let entry = format!("[{}] {}: {}\n", index + 1, date_label, plain_text);

            if content.chars().count() + entry.chars().count() > MAX_CONTENT_CHARS {
                break;
            }

            content.push_str(&entry);
        }

        content.push_str("---\n请基于这些样本完成博主画像分析。\n");
        content
    }

    async fn call_api(&self, content: &str) -> Result<String, String> {
        let request = GlmRequest {
            model: MODEL.to_string(),
            messages: vec![
                GlmMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                GlmMessage {
                    role: "user".to_string(),
                    content: content.to_string(),
                },
            ],
            temperature: 0.3,
        };

        let response = self
            .client
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|error| format!("调用 AI 接口失败: {error}"))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("读取 AI 响应失败: {error}"))?;

        if !status.is_success() {
            return Err(format!("AI 接口返回错误 ({status}): {body}"));
        }

        let parsed: GlmResponse =
            serde_json::from_str(&body).map_err(|error| format!("解析 AI 响应失败: {error}"))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| "AI 响应中没有可用内容".to_string())
    }
}

fn load_api_key() -> String {
    // 环境变量优先（开发调试用）
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        return key;
    }

    // 尝试从可执行文件目录读取（允许用户覆盖）
    let exe_dir = std::env::current_exe().unwrap_or_default();
    let config_path = exe_dir.parent().map(|p| p.join(API_KEY_FILE));

    if let Some(path) = config_path {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let key = content.trim().to_string();
            if !key.is_empty() {
                return key;
            }
        }
    }

    // 内置 Key（生产环境）
    "sk-22de1e879d7245c397d87582490bee2a".to_string()
}

fn format_post_date(created_at: &str) -> String {
    chrono::DateTime::parse_from_str(created_at, "%a %b %e %H:%M:%S %z %Y")
        .map(|date_time| date_time.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| created_at.to_string())
}

fn normalize_json_response(content: &str) -> String {
    let trimmed = content.trim().trim_matches('`').trim();
    let without_json_prefix = trimmed.strip_prefix("json").map(str::trim).unwrap_or(trimmed);

    match (
        without_json_prefix.find('{'),
        without_json_prefix.rfind('}'),
    ) {
        (Some(start), Some(end)) if start <= end => without_json_prefix[start..=end].to_string(),
        _ => without_json_prefix.to_string(),
    }
}

#[derive(Debug, Serialize)]
struct GlmRequest {
    model: String,
    messages: Vec<GlmMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct GlmMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GlmResponse {
    choices: Vec<GlmChoice>,
}

#[derive(Debug, Deserialize)]
struct GlmChoice {
    message: GlmMessage,
}
