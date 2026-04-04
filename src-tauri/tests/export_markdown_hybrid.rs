use tauri_app_lib::models::{WeiboImage, WeiboPost};
use tauri_app_lib::services::export_markdown::MarkdownExportService;
use tauri_app_lib::services::sanitize_filename;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "weibo_markdown_hybrid_{}_{}",
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn linked_post() -> WeiboPost {
    WeiboPost {
        mblogid: "Olinked123".to_string(),
        created_at: "Mon Jan 15 12:00:00 +0800 2024".to_string(),
        text: r#"<p>正文前半段</p><p><a href="https://weibo.com/example">查看原文</a></p>"#.to_string(),
        images: vec![WeiboImage {
            original_url: "https://example.com/img.jpg".to_string(),
            local_path: Some("images/img.jpg".to_string()),
            width: 1080,
            height: 1080,
        }],
        is_repost: false,
        repost_user: None,
        region: Some("发布于 北京".to_string()),
        source_url: "https://weibo.com/123/Olinked123".to_string(),
        author: "测试用户".to_string(),
        tags: vec![],
    }
}

fn single_export_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(format!("{}-微博导出.md", sanitize_filename("测试用户")))
}

#[tokio::test]
async fn markdown_export_preserves_link_targets() {
    let dir = temp_dir("links");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_single(&posts, &dir).await.unwrap();

    let rendered = std::fs::read_to_string(single_export_path(&dir)).unwrap();
    assert!(rendered.contains("[查看原文](https://weibo.com/example)"));
}

#[tokio::test]
async fn markdown_export_places_metadata_below_body() {
    let dir = temp_dir("metadata");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_single(&posts, &dir).await.unwrap();

    let rendered = std::fs::read_to_string(single_export_path(&dir)).unwrap();
    let body_index = rendered.find("正文前半段").unwrap();
    let metadata_index = rendered.find("- 发布时间:").unwrap();

    assert!(body_index < metadata_index);
}

#[tokio::test]
async fn per_post_export_writes_index_file() {
    let dir = temp_dir("index");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_per_post(&posts, &dir).await.unwrap();

    assert!(dir.join("posts").join("index.md").exists());
}

#[tokio::test]
async fn per_post_export_uses_parent_relative_image_paths() {
    let dir = temp_dir("relative-images");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_per_post(&posts, &dir).await.unwrap();

    let post_file = std::fs::read_dir(dir.join("posts"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md") && path.file_name().and_then(|name| name.to_str()) != Some("index.md"))
        .unwrap();

    let rendered = std::fs::read_to_string(post_file).unwrap();
    assert!(rendered.contains("../images/img.jpg"));
}
