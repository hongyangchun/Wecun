pub mod ai_key;
pub mod glm_client;
pub mod local_analyzer;
pub mod models;

use tauri::{AppHandle, Runtime};

use crate::error::AppError;
use crate::services::cache;

use glm_client::GlmClient;
use models::ProfileResult;

pub struct ProfileService;

impl ProfileService {
    pub fn new() -> Self {
        Self
    }

    /// Local statistics over the saved posts, plus the AI deep profile when the user has set a key.
    ///
    /// A failed AI call does not fail the analysis: the report shows the local statistics and says
    /// what is missing and why.
    pub async fn analyze<R: Runtime>(&self, output_dir: &str, app: &AppHandle<R>) -> Result<ProfileResult, AppError> {
        let bundle = cache::load_cache_bundle(output_dir).await?;
        let posts = &bundle.posts;

        if posts.is_empty() {
            return Err(AppError::ApiError("没有可分析的微博数据".to_string()));
        }

        let local_stats = local_analyzer::LocalAnalyzer::analyze(posts);
        let api_key = ai_key::read(app);
        let ai_configured = api_key.is_some();
        let (deep_profile, ai_error) = match api_key {
            Some(key) => match GlmClient::new(key).analyze(posts).await {
                Ok(profile) => (Some(profile), None),
                Err(error) => {
                    log::warn!("AI profile analysis failed: {error}");
                    (None, Some(error))
                }
            },
            None => (None, None),
        };

        let author_name = posts
            .iter()
            .map(|p| p.author.trim())
            .find(|a| !a.is_empty())
            .map(str::to_string);

        Ok(ProfileResult {
            author_name,
            basic_stats: local_stats.basic_stats,
            content_analysis: local_stats.content_analysis,
            influence_metrics: local_stats.influence_metrics,
            deep_profile,
            ai_configured,
            ai_error,
        })
    }
}
