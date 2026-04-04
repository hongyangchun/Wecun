use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AppError;
use crate::models::{CachedPosts, ExportContext, WeiboPost};

const CACHE_DIR_NAME: &str = ".weibo-cache";
const CACHE_FILE_NAME: &str = "posts.json";

fn cache_dir(output_dir: &str) -> PathBuf {
    Path::new(output_dir).join(CACHE_DIR_NAME)
}

pub async fn save_posts_cache(posts: &[WeiboPost], output_dir: &str) -> Result<(), AppError> {
    save_cache_bundle(posts, output_dir, ExportContext::default()).await
}

pub async fn save_cache_bundle(
    posts: &[WeiboPost],
    output_dir: &str,
    export_context: ExportContext,
) -> Result<(), AppError> {
    let dir = cache_dir(output_dir);
    fs::create_dir_all(&dir).await.map_err(AppError::Io)?;
    let json = serde_json::to_string_pretty(&CachedPosts {
        export_context,
        posts: posts.to_vec(),
    })
    .map_err(|e| AppError::Parse(e.to_string()))?;
    let path = dir.join(CACHE_FILE_NAME);
    fs::write(&path, json).await.map_err(AppError::Io)?;
    Ok(())
}

pub fn load_posts_cache_sync(output_dir: &str) -> Result<Vec<WeiboPost>, AppError> {
    Ok(load_cache_bundle_sync(output_dir)?.posts)
}

pub fn load_cache_bundle_sync(output_dir: &str) -> Result<CachedPosts, AppError> {
    let path = cache_dir(output_dir).join(CACHE_FILE_NAME);
    let content = std::fs::read_to_string(&path).map_err(AppError::Io)?;
    parse_cached_posts(&content)
}

pub async fn load_posts_cache(output_dir: &str) -> Result<Vec<WeiboPost>, AppError> {
    Ok(load_cache_bundle(output_dir).await?.posts)
}

pub async fn load_cache_bundle(output_dir: &str) -> Result<CachedPosts, AppError> {
    let path = cache_dir(output_dir).join(CACHE_FILE_NAME);
    let content = fs::read_to_string(&path).await.map_err(AppError::Io)?;
    parse_cached_posts(&content)
}

fn parse_cached_posts(content: &str) -> Result<CachedPosts, AppError> {
    if let Ok(bundle) = serde_json::from_str::<CachedPosts>(content) {
        return Ok(bundle);
    }

    let posts =
        serde_json::from_str::<Vec<WeiboPost>>(content).map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(CachedPosts {
        export_context: ExportContext::default(),
        posts,
    })
}
