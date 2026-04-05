use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AppError;
use crate::models::{CachedPosts, CheckpointMeta, ExportContext, WeiboPost};

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
        checkpoint: None,
    })
    .map_err(|e| AppError::Parse(e.to_string()))?;
    let path = dir.join(CACHE_FILE_NAME);
    fs::write(&path, json).await.map_err(AppError::Io)?;
    Ok(())
}

pub async fn save_cache_checkpoint(
    posts: &[WeiboPost],
    output_dir: &str,
    checkpoint: CheckpointMeta,
    export_context: &ExportContext,
) -> Result<(), AppError> {
    let dir = cache_dir(output_dir);
    fs::create_dir_all(&dir).await.map_err(AppError::Io)?;
    let json = serde_json::to_string(&CachedPosts {
        export_context: export_context.clone(),
        posts: posts.to_vec(),
        checkpoint: Some(checkpoint),
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
        checkpoint: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_cached_posts, save_cache_checkpoint, CACHE_DIR_NAME, CACHE_FILE_NAME};
    use crate::models::{CheckpointMeta, ExportContext, WeiboImage, WeiboPost};

    fn sample_post(id: &str) -> WeiboPost {
        WeiboPost {
            mblogid: id.to_string(),
            created_at: "Mon Jan 15 12:00:00 +0800 2024".to_string(),
            text: "<p>测试微博内容</p>".to_string(),
            images: vec![WeiboImage {
                original_url: "https://example.com/img.jpg".to_string(),
                local_path: Some("images/img.jpg".to_string()),
                width: 1080,
                height: 1080,
            }],
            is_repost: false,
            repost_user: None,
            region: Some("发布于 北京".to_string()),
            source_url: format!("https://weibo.com/123/{id}"),
            author: "测试用户".to_string(),
            tags: vec![],
        }
    }

    fn temp_output_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "weibo-cache-test-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_cached_posts_accepts_legacy_cache_without_checkpoint() {
        let post = sample_post("legacy-post");
        let content = serde_json::to_string(&serde_json::json!({
            "export_context": {
                "date_range_label": "全部时间"
            },
            "posts": [post]
        }))
        .unwrap();

        let bundle = parse_cached_posts(&content).expect("legacy cache should deserialize");

        assert!(bundle.checkpoint.is_none());
        assert_eq!(bundle.posts.len(), 1);
        assert_eq!(bundle.posts[0].mblogid, "legacy-post");
    }

    #[tokio::test]
    async fn save_cache_checkpoint_writes_checkpoint_metadata() {
        let output_dir = temp_output_dir("checkpoint");
        let output_dir_str = output_dir.to_string_lossy().to_string();
        let posts = vec![sample_post("checkpoint-post")];

        save_cache_checkpoint(
            &posts,
            &output_dir_str,
            CheckpointMeta {
                uid: "123456".to_string(),
                last_page: 3,
                total_fetched: 42,
                total_posts: 99,
            },
            &ExportContext {
                date_range_label: "2024-01-01至2024-01-31".to_string(),
            },
        )
        .await
        .expect("checkpoint cache should be written");

        let cache_path = output_dir.join(CACHE_DIR_NAME).join(CACHE_FILE_NAME);
        let content = std::fs::read_to_string(&cache_path).expect("checkpoint cache file should exist");
        let bundle = parse_cached_posts(&content).expect("checkpoint cache should deserialize");

        let checkpoint = bundle.checkpoint.expect("checkpoint metadata should exist");
        assert_eq!(checkpoint.uid, "123456");
        assert_eq!(checkpoint.last_page, 3);
        assert_eq!(checkpoint.total_fetched, 42);
        assert_eq!(checkpoint.total_posts, 99);
        assert_eq!(bundle.posts.len(), 1);

        std::fs::remove_dir_all(&output_dir).ok();
    }
}
