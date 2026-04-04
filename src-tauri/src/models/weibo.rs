use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct RawUserInfo {
    pub data: RawUserData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawUserData {
    pub user: RawUser,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawUser {
    pub id: i64,
    pub screen_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawHistoryMap {
    pub data: std::collections::HashMap<String, Vec<i32>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawSearchProfile {
    pub data: RawSearchProfileData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawSearchProfileData {
    pub list: Option<Vec<RawPost>>,
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPost {
    pub mblogid: String,
    pub created_at: String,
    pub text: String,
    #[serde(default, alias = "isLongText")]
    pub is_long_text: Option<bool>,
    #[serde(default)]
    pub region_name: Option<String>,
    #[serde(default)]
    pub user: Option<RawPostUser>,
    #[serde(default)]
    pub pic_infos: Option<serde_json::Value>,
    #[serde(default)]
    pub retweeted_status: Option<Box<RawPost>>,
    #[serde(default)]
    pub page_info: Option<serde_json::Value>,
    #[serde(default)]
    pub tag_struct: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub topic_struct: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPostUser {
    pub id: i64,
    pub screen_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawLongText {
    pub data: RawLongTextData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawLongTextData {
    #[serde(default)]
    pub long_text_content: String,
    #[serde(default)]
    pub raw_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub uid: String,
    pub screen_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeiboImage {
    pub original_url: String,
    pub local_path: Option<String>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeiboPost {
    pub mblogid: String,
    pub created_at: String,
    pub text: String,
    pub images: Vec<WeiboImage>,
    pub is_repost: bool,
    pub repost_user: Option<String>,
    pub region: Option<String>,
    pub source_url: String,
    pub author: String,
    pub tags: Vec<String>,
}
