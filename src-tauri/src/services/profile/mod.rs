pub mod glm_client;
pub mod local_analyzer;
pub mod models;

use crate::error::AppError;
use crate::services::cache;

use glm_client::GlmClient;
use models::ProfileResult;

pub struct ProfileService;

impl ProfileService {
    pub fn new() -> Self {
        Self
    }

    /// Run the full profile analysis pipeline: local stats + optional AI deep analysis.
    pub async fn analyze(&self, output_dir: &str) -> Result<ProfileResult, AppError> {
        let bundle = cache::load_cache_bundle(output_dir).await?;
        let posts = &bundle.posts;

        if posts.is_empty() {
            return Err(AppError::ApiError("没有可分析的微博数据".to_string()));
        }

        // Step 1: Local analysis
        let local_stats = local_analyzer::LocalAnalyzer::analyze(posts);
        let glm_client = GlmClient::new();
        let deep_profile = if glm_client.is_configured() {
            glm_client.analyze(posts).await.ok()
        } else {
            None
        };

        Ok(ProfileResult {
            basic_stats: local_stats.basic_stats,
            content_analysis: local_stats.content_analysis,
            influence_metrics: local_stats.influence_metrics,
            deep_profile,
        })
    }
}
