use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AppError;
use crate::models::{BackupMeta, CachedPosts, CheckpointMeta, ExportContext, WeiboPost};

const CACHE_DIR_NAME: &str = ".weibo-cache";
const CACHE_FILE_NAME: &str = "posts.json";
const BACKUP_FILE_NAME: &str = "backup.json";

fn cache_dir(output_dir: &str) -> PathBuf {
    Path::new(output_dir).join(CACHE_DIR_NAME)
}

pub async fn save_posts_cache(posts: &[WeiboPost], output_dir: &str) -> Result<(), AppError> {
    save_cache_bundle(posts, output_dir, ExportContext::default()).await
}

async fn write_atomic(path: &Path, content: &str) -> Result<(), AppError> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content).await.map_err(AppError::Io)?;
    if let Err(err) = fs::rename(&tmp_path, path).await {
        // Fallback on platforms where rename over an existing file can fail
        let _ = fs::remove_file(path).await;
        fs::rename(&tmp_path, path).await.map_err(|_| AppError::Io(err))?;
    }
    Ok(())
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
        backup: None,
    })
    .map_err(|e| AppError::Parse(e.to_string()))?;
    let path = dir.join(CACHE_FILE_NAME);
    write_atomic(&path, &json).await
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
        backup: None,
    })
    .map_err(|e| AppError::Parse(e.to_string()))?;
    let path = dir.join(CACHE_FILE_NAME);
    write_atomic(&path, &json).await?;
    // The cache is an interrupted run now, not a finished backup.
    let _ = fs::remove_file(dir.join(BACKUP_FILE_NAME)).await;
    Ok(())
}

/// Saves a finished backup: the posts, and a small file that describes them so the UI can tell an
/// update from a first download without reading every post.
pub async fn save_finished_backup(
    posts: &[WeiboPost],
    output_dir: &str,
    export_context: ExportContext,
    backup: &BackupMeta,
) -> Result<(), AppError> {
    let mut backup = backup.clone();
    backup.post_count = posts.len();

    let dir = cache_dir(output_dir);
    fs::create_dir_all(&dir).await.map_err(AppError::Io)?;
    let json = serde_json::to_string_pretty(&CachedPosts {
        export_context,
        posts: posts.to_vec(),
        checkpoint: None,
        backup: Some(backup.clone()),
    })
    .map_err(|e| AppError::Parse(e.to_string()))?;
    write_atomic(&dir.join(CACHE_FILE_NAME), &json).await?;

    let description = serde_json::to_string_pretty(&backup).map_err(|e| AppError::Parse(e.to_string()))?;
    // Written last: a description is only there when the posts it describes are.
    write_atomic(&dir.join(BACKUP_FILE_NAME), &description).await
}

/// The finished backup in `output_dir`, when there is one.
pub fn inspect_backup(output_dir: &str) -> Option<BackupMeta> {
    let content = std::fs::read_to_string(cache_dir(output_dir).join(BACKUP_FILE_NAME)).ok()?;
    serde_json::from_str(&content).ok()
}

/// Removes the cache, so that a run which starts over is never mistaken for a continuation.
pub fn clear(output_dir: &str) {
    let dir = cache_dir(output_dir);
    for name in [CACHE_FILE_NAME, BACKUP_FILE_NAME] {
        let _ = std::fs::remove_file(dir.join(name));
    }
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
        backup: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_cached_posts, save_cache_checkpoint, save_cache_bundle, load_cache_bundle_sync, load_posts_cache_sync, CACHE_DIR_NAME, CACHE_FILE_NAME};
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
            repost_text: None,
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

    #[test]
    fn parse_cached_posts_accepts_plain_array_format() {
        // Old format: just an array of posts
        let posts = vec![sample_post("array-post-1"), sample_post("array-post-2")];
        let content = serde_json::to_string(&posts).unwrap();

        let bundle = parse_cached_posts(&content).expect("array format should deserialize");

        assert!(bundle.checkpoint.is_none());
        assert_eq!(bundle.posts.len(), 2);
        assert_eq!(bundle.posts[0].mblogid, "array-post-1");
        assert_eq!(bundle.posts[1].mblogid, "array-post-2");
    }

    #[test]
    fn parse_cached_posts_handles_invalid_json() {
        let content = "not valid json";
        let result = parse_cached_posts(content);
        assert!(result.is_err(), "Should error on invalid JSON");
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
                source_type: None,
                last_page: 3,
                total_fetched: 42,
                total_posts: 99,
                filter: None,
                start_timestamp: None,
                end_timestamp: None,
                schema: 0,
                min_text_length: 0,
                ignore_deleted: false,
                long_text_failed: 0,
                base_posts: 0,
            },
            &ExportContext {
                date_range_label: "2024-01-01至2024-01-31".to_string(),
                type_label: "微博备份".to_string(),
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

    #[tokio::test]
    async fn save_cache_bundle_without_checkpoint() {
        let output_dir = temp_output_dir("bundle");
        let output_dir_str = output_dir.to_string_lossy().to_string();
        let posts = vec![sample_post("bundle-post")];

        save_cache_bundle(
            &posts,
            &output_dir_str,
            ExportContext {
                date_range_label: "2024-01-01至2024-01-31".to_string(),
                type_label: "微博备份".to_string(),
            },
        )
        .await
        .expect("cache bundle should be written");

        let cache_path = output_dir.join(CACHE_DIR_NAME).join(CACHE_FILE_NAME);
        assert!(cache_path.exists(), "Cache file should exist");

        let content = std::fs::read_to_string(&cache_path).expect("cache file should be readable");
        let bundle = parse_cached_posts(&content).expect("cache should deserialize");

        assert!(bundle.checkpoint.is_none(), "Bundle should not have checkpoint");
        assert_eq!(bundle.posts.len(), 1);
        assert_eq!(bundle.posts[0].mblogid, "bundle-post");

        std::fs::remove_dir_all(&output_dir).ok();
    }

    #[test]
    fn load_cache_bundle_sync_reads_correctly() {
        let output_dir = temp_output_dir("sync-load");
        let output_dir_str = output_dir.to_string_lossy().to_string();
        let posts = vec![sample_post("sync-post")];

        // Write cache using sync method for test setup
        // Note: source_type is Option<String> in the model, not a string directly
        let cache_content = serde_json::to_string(&serde_json::json!({
            "export_context": {
                "date_range_label": "2024-01-01至2024-01-31",
                "type_label": "微博备份"
            },
            "posts": posts,
            "checkpoint": {
                "uid": "789",
                "source_type": null,
                "last_page": 5,
                "total_fetched": 100,
                "total_posts": 200
            }
        })).unwrap();

        let cache_file = output_dir.join(CACHE_DIR_NAME).join(CACHE_FILE_NAME);
        std::fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        std::fs::write(&cache_file, cache_content).unwrap();

        let bundle = load_cache_bundle_sync(&output_dir_str).expect("Should load cache bundle");

        assert_eq!(bundle.posts.len(), 1);
        assert_eq!(bundle.posts[0].mblogid, "sync-post");
        let checkpoint = bundle.checkpoint.expect("Should have checkpoint");
        assert_eq!(checkpoint.uid, "789");
        assert_eq!(checkpoint.last_page, 5);

        std::fs::remove_dir_all(&output_dir).ok();
    }

    #[test]
    fn load_posts_cache_sync_returns_posts_only() {
        let output_dir = temp_output_dir("posts-only");
        let output_dir_str = output_dir.to_string_lossy().to_string();
        let posts = vec![sample_post("post-only-1"), sample_post("post-only-2")];

        let cache_content = serde_json::to_string(&serde_json::json!({
            "export_context": {
                "date_range_label": "全部时间",
                "type_label": "微博备份"
            },
            "posts": posts
        })).unwrap();

        let cache_file = output_dir.join(CACHE_DIR_NAME).join(CACHE_FILE_NAME);
        std::fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        std::fs::write(&cache_file, cache_content).unwrap();

        let loaded_posts = load_posts_cache_sync(&output_dir_str).expect("Should load posts");

        assert_eq!(loaded_posts.len(), 2);
        assert_eq!(loaded_posts[0].mblogid, "post-only-1");
        assert_eq!(loaded_posts[1].mblogid, "post-only-2");

        std::fs::remove_dir_all(&output_dir).ok();
    }

    #[test]
    fn load_cache_handles_missing_file() {
        let output_dir = temp_output_dir("missing");
        let output_dir_str = output_dir.to_string_lossy().to_string();

        let result = load_cache_bundle_sync(&output_dir_str);
        assert!(result.is_err(), "Should error when cache file doesn't exist");

        std::fs::remove_dir_all(&output_dir).ok();
    }

    #[tokio::test]
    async fn save_and_load_roundtrip() {
        let output_dir = temp_output_dir("roundtrip");
        let output_dir_str = output_dir.to_string_lossy().to_string();
        let posts = vec![
            sample_post("roundtrip-1"),
            sample_post("roundtrip-2"),
        ];

        save_cache_bundle(
            &posts,
            &output_dir_str,
            ExportContext {
                date_range_label: "2024-01-01至2024-12-31".to_string(),
                type_label: "收藏微博".to_string(),
            },
        )
        .await
        .expect("Should save cache");

        let loaded_posts = load_posts_cache_sync(&output_dir_str).expect("Should load posts");

        assert_eq!(loaded_posts.len(), 2);
        assert_eq!(loaded_posts[0].mblogid, "roundtrip-1");
        assert_eq!(loaded_posts[1].mblogid, "roundtrip-2");

        std::fs::remove_dir_all(&output_dir).ok();
    }

    #[tokio::test]
    async fn a_finished_backup_is_described_next_to_the_posts_and_interrupting_it_removes_that() {
        use crate::models::{BackupMeta, PostFilter, SourceType};

        let output_dir = temp_output_dir("finished-backup");
        let dir = output_dir.to_string_lossy().to_string();
        assert!(super::inspect_backup(&dir).is_none(), "nothing is there yet");

        let posts = vec![sample_post("finished-post")];
        let described = BackupMeta {
            uid: "123456".to_string(),
            source_type: SourceType::Profile,
            post_count: 999,
            author: "测试用户".to_string(),
            filter: PostFilter::Original,
            min_text_length: 20,
            ignore_deleted: true,
            start_timestamp: None,
            end_timestamp: None,
        };
        super::save_finished_backup(&posts, &dir, ExportContext::default(), &described)
            .await
            .expect("a finished backup is saved");

        let inspected = super::inspect_backup(&dir).expect("the backup can be inspected");
        assert_eq!(inspected.post_count, 1, "the count comes from the posts, not from the caller");
        let bundle = super::load_cache_bundle_sync(&dir).unwrap();
        assert!(bundle.checkpoint.is_none());
        assert_eq!(bundle.backup.map(|backup| backup.author), Some("测试用户".to_string()));

        // An interrupted run writes a checkpoint instead; it is not a finished backup any more.
        super::save_cache_checkpoint(
            &posts,
            &dir,
            CheckpointMeta {
                uid: "123456".to_string(),
                source_type: None,
                last_page: 4,
                total_fetched: 1,
                total_posts: 1,
                filter: None,
                start_timestamp: None,
                end_timestamp: None,
                schema: 0,
                min_text_length: 0,
                ignore_deleted: false,
                long_text_failed: 0,
                base_posts: 0,
            },
            &ExportContext::default(),
        )
        .await
        .unwrap();
        assert!(super::inspect_backup(&dir).is_none());

        super::save_finished_backup(&posts, &dir, ExportContext::default(), &described)
            .await
            .unwrap();
        super::clear(&dir);
        assert!(super::inspect_backup(&dir).is_none());
        assert!(super::load_cache_bundle_sync(&dir).is_err(), "a run that starts over is not mistaken for a continuation");

        std::fs::remove_dir_all(&output_dir).ok();
    }
}
