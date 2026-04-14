use wecun_lib::models::{ExportContext, WeiboImage, WeiboPost};
use wecun_lib::services::export_markdown::MarkdownExportService;
use wecun_lib::services::markdown_export_filename;

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

fn rich_html_post() -> WeiboPost {
    WeiboPost {
        mblogid: "Orich123".to_string(),
        created_at: "Tue Jan 16 08:30:00 +0800 2024".to_string(),
        text: r#"<p><strong>加粗</strong> 和 <em>斜体</em><br>下一行</p><blockquote><p>引用 <a href="https://weibo.com/rich">链接</a></p></blockquote><ul><li>第一项</li><li>第二项</li></ul><p><img src="https://example.com/inline.jpg" alt="内联图片"></p>"#.to_string(),
        images: vec![],
        is_repost: false,
        repost_user: None,
        region: None,
        source_url: "https://weibo.com/123/Orich123".to_string(),
        author: "测试用户".to_string(),
        tags: vec![],
    }
}

fn single_export_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(markdown_export_filename("微博备份", "2024-01-01至2024-01-31"))
}

#[tokio::test]
async fn markdown_export_preserves_link_targets() {
    let dir = temp_dir("links");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

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

    let rendered = std::fs::read_to_string(single_export_path(&dir)).unwrap();
    assert!(rendered.contains("[查看原文](https://weibo.com/example)"));
}

#[tokio::test]
async fn markdown_export_preserves_rich_html_formatting() {
    let dir = temp_dir("rich-formatting");
    let svc = MarkdownExportService::new();
    let posts = vec![rich_html_post()];

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

    let rendered = std::fs::read_to_string(single_export_path(&dir)).unwrap();
    assert!(rendered.contains("**加粗** 和 *斜体*\n下一行"));
    assert!(rendered.contains("> 引用 [链接](https://weibo.com/rich)"));
    assert!(rendered.contains("- 第一项"));
    assert!(rendered.contains("- 第二项"));
    assert!(rendered.contains("![内联图片](https://example.com/inline.jpg)"));
}

#[tokio::test]
async fn markdown_export_time_link_appears_above_body() {
    let dir = temp_dir("time-link");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

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

    let rendered = std::fs::read_to_string(single_export_path(&dir)).unwrap();
    let time_index = rendered.find("Mon Jan 15").unwrap();
    let body_index = rendered.find("正文前半段").unwrap();

    assert!(time_index < body_index);
    assert!(rendered.contains("[原文链接](https://weibo.com/123/Olinked123)"));
}

#[tokio::test]
async fn per_post_export_writes_index_file() {
    let dir = temp_dir("index");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_per_post(
        &posts,
        &dir,
        &ExportContext {
            date_range_label: "2024-01-01至2024-01-31".to_string(),
            type_label: "微博备份".to_string(),
        },
    )
    .await
    .unwrap();

    assert!(dir.join("posts").join("index.md").exists());
}

#[tokio::test]
async fn per_post_export_uses_parent_relative_image_paths() {
    let dir = temp_dir("relative-images");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_per_post(
        &posts,
        &dir,
        &ExportContext {
            date_range_label: "2024-01-01至2024-01-31".to_string(),
            type_label: "微博备份".to_string(),
        },
    )
    .await
    .unwrap();

    let post_file = std::fs::read_dir(dir.join("posts"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md") && path.file_name().and_then(|name| name.to_str()) != Some("index.md"))
        .unwrap();

    let rendered = std::fs::read_to_string(post_file).unwrap();
    assert!(rendered.contains("../images/img.jpg"));
}

#[tokio::test]
async fn per_post_export_writes_obsidian_frontmatter() {
    let dir = temp_dir("frontmatter");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_per_post(
        &posts,
        &dir,
        &ExportContext {
            date_range_label: "2024-01-01至2024-01-31".to_string(),
            type_label: "微博备份".to_string(),
        },
    )
    .await
    .unwrap();

    let post_file = std::fs::read_dir(dir.join("posts"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md") && path.file_name().and_then(|name| name.to_str()) != Some("index.md"))
        .unwrap();

    let rendered = std::fs::read_to_string(post_file).unwrap();
    assert!(rendered.starts_with("---\n"));
    assert!(rendered.contains("author: \"测试用户\""));
    assert!(rendered.contains("source_url: \"https://weibo.com/123/Olinked123\""));
}

#[tokio::test]
async fn per_post_export_uses_time_and_text_filename() {
    let dir = temp_dir("filename");
    let svc = MarkdownExportService::new();
    let posts = vec![linked_post()];

    svc.export_per_post(
        &posts,
        &dir,
        &ExportContext {
            date_range_label: "2024-01-01至2024-01-31".to_string(),
            type_label: "微博备份".to_string(),
        },
    )
    .await
    .unwrap();

    let post_file = std::fs::read_dir(dir.join("posts"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md") && path.file_name().and_then(|name| name.to_str()) != Some("index.md"))
        .unwrap();

    let file_name = post_file.file_name().and_then(|name| name.to_str()).unwrap();
    assert!(file_name.starts_with("2024-01-15-正文前半段"));
}
