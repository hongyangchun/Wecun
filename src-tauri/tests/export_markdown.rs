use tauri_app_lib::models::{ExportContext, WeiboImage, WeiboPost};
use tauri_app_lib::services::export_markdown::MarkdownExportService;
use tauri_app_lib::services::markdown_export_filename;

fn sample_posts() -> Vec<WeiboPost> {
    vec![WeiboPost {
        mblogid: "Otest123".to_string(),
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
        source_url: "https://weibo.com/123/Otest123".to_string(),
        author: "测试用户".to_string(),
        tags: vec![],
    }]
}

#[tokio::test]
async fn test_markdown_single() {
    let posts = sample_posts();
    let dir = std::env::temp_dir().join("weibo_test_single");
    std::fs::create_dir_all(&dir).unwrap();

    let svc = MarkdownExportService::new();
    svc.export_single(
        &posts,
        &dir,
        &ExportContext {
            date_range_label: "2024-01-01至2024-01-31".to_string(),
            type_label: "微博备份".to_string(),
        },
    )
    .await
    .unwrap();

    let output = dir.join(markdown_export_filename("微博备份", "2024-01-01至2024-01-31"));
    assert!(output.exists());

    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("# 微博备份"));
    assert!(content.contains("测试微博内容"));
    assert!(content.contains("images/img.jpg"));

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_markdown_per_post() {
    let posts = sample_posts();
    let dir = std::env::temp_dir().join("weibo_test_multi");
    std::fs::create_dir_all(&dir).unwrap();

    let svc = MarkdownExportService::new();
    svc.export_per_post(&posts, &dir).await.unwrap();

    let posts_dir = dir.join("posts");
    assert!(posts_dir.exists());

    let entries: Vec<_> = std::fs::read_dir(&posts_dir).unwrap().collect();
    assert!(!entries.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}
