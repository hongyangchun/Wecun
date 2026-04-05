use std::path::Path;

use tokio::fs;

use crate::error::AppError;
use crate::models::WeiboPost;
use crate::services::file_naming::sanitize_filename;

pub struct HtmlExportService;

impl HtmlExportService {
    pub fn new() -> Self {
        Self
    }

    pub async fn export(
        &self,
        posts: &[WeiboPost],
        output_dir: impl AsRef<Path>,
    ) -> Result<(), AppError> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir).await.map_err(AppError::Io)?;

        let author_name = posts
            .first()
            .map(|p| p.author.as_str())
            .unwrap_or("微博用户");

        let html = self.render(posts, author_name);
        let filename = format!("{}-微博导出.html", sanitize_filename(author_name));
        let dest = output_dir.join(filename);
        fs::write(&dest, html).await.map_err(AppError::Io)?;
        Ok(())
    }

    fn render(&self, posts: &[WeiboPost], author_name: &str) -> String {
        let mut html = String::with_capacity(64 * 1024);

        html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str(&format!(
            "<title>{} 的微博导出</title>\n",
            html_escape(author_name)
        ));
        html.push_str("<style>\n");
        html.push_str(CSS);
        html.push_str("\n</style>\n");
        html.push_str("</head>\n<body>\n<div class=\"container\">\n");

        html.push_str("<header>\n");
        html.push_str(&format!(
            "<h1>{} 的微博导出</h1>\n",
            html_escape(author_name)
        ));
        html.push_str(&format!("<p class=\"subtitle\">共 {} 条微博</p>\n", posts.len()));
        html.push_str("</header>\n");

        for post in posts {
            html.push_str("<article class=\"post\">\n");

            html.push_str("<div class=\"post-header\">\n");
            html.push_str(&format!(
                "<span class=\"post-author\">{}</span>\n",
                html_escape(&post.author)
            ));
            html.push_str(&format!(
                "<span class=\"post-date\">{}</span>\n",
                html_escape(&post.created_at)
            ));
            html.push_str("</div>\n");

            if let Some(region) = &post.region {
                html.push_str(&format!(
                    "<div class=\"post-region\">{}</div>\n",
                    html_escape(region)
                ));
            }

            if post.is_repost {
                if let Some(repost_user) = &post.repost_user {
                    html.push_str(&format!(
                        "<div class=\"post-repost\">转发自 @{}</div>\n",
                        html_escape(repost_user)
                    ));
                }
            }

            html.push_str(&format!(
                "<div class=\"post-content\">{}</div>\n",
                sanitize_html_content(&post.text)
            ));

            if !post.images.is_empty() {
                html.push_str("<div class=\"post-images\">\n");
                for (i, img) in post.images.iter().enumerate() {
                    let src = img.local_path.as_deref().unwrap_or(&img.original_url);
                    html.push_str(&format!(
                        "<figure><img src=\"{}\" alt=\"图片{}\" loading=\"lazy\" /></figure>\n",
                        html_escape(src),
                        i + 1
                    ));
                }
                html.push_str("</div>\n");
            }

            html.push_str("<div class=\"post-meta\">\n");
            html.push_str(&format!(
                "<a href=\"{}\">原文链接</a>\n",
                html_escape(&post.source_url)
            ));
            if !post.tags.is_empty() {
                html.push_str(&format!(
                    "<span class=\"post-tags\">{}</span>\n",
                    html_escape(&post.tags.join(", "))
                ));
            }
            html.push_str("</div>\n");

            html.push_str("</article>\n");
        }

        html.push_str("</div>\n</body>\n</html>\n");
        html
    }
}

fn sanitize_html_content(html: &str) -> String {
    ammonia::clean(html)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const CSS: &str = r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    background: #f5f5f7;
    color: #1d1d1f;
    line-height: 1.6;
    padding: 20px;
}

.container {
    max-width: 800px;
    margin: 0 auto;
}

header {
    text-align: center;
    margin-bottom: 40px;
    padding: 40px 0;
}

header h1 {
    font-size: 28px;
    font-weight: 600;
    color: #1d1d1f;
    margin-bottom: 8px;
}

.subtitle {
    font-size: 16px;
    color: #86868b;
}

.post {
    background: #fff;
    border-radius: 12px;
    padding: 24px;
    margin-bottom: 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.post-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
}

.post-author {
    font-weight: 600;
    font-size: 16px;
    color: #1d1d1f;
}

.post-date {
    font-size: 14px;
    color: #86868b;
}

.post-region {
    font-size: 13px;
    color: #86868b;
    margin-bottom: 8px;
}

.post-repost {
    font-size: 14px;
    color: #5b5bb4;
    font-style: italic;
    margin-bottom: 8px;
}

.post-content {
    font-size: 16px;
    line-height: 1.7;
    margin-bottom: 12px;
    word-wrap: break-word;
}

.post-content a {
    color: #4F6EF7;
    text-decoration: none;
}

.post-content a:hover {
    text-decoration: underline;
}

.post-images {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    margin-bottom: 12px;
}

.post-images img {
    width: 100%;
    border-radius: 8px;
    object-fit: cover;
}

.post-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 12px;
    border-top: 1px solid #f0f0f0;
    font-size: 13px;
}

.post-meta a {
    color: #4F6EF7;
    text-decoration: none;
}

.post-meta a:hover {
    text-decoration: underline;
}

.post-tags {
    color: #86868b;
}

@media print {
    body {
        background: #fff;
        padding: 0;
    }

    .post {
        box-shadow: none;
        border: 1px solid #e5e5e5;
        break-inside: avoid;
    }
}

@media (prefers-color-scheme: dark) {
    body {
        background: #1a1a1a;
        color: #e5e5ea;
    }

    header h1 {
        color: #f5f5f7;
    }

    .subtitle {
        color: #98989d;
    }

    .post {
        background: #2a2a2a;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    }

    .post-author {
        color: #f5f5f7;
    }

    .post-date {
        color: #98989d;
    }

    .post-region {
        color: #98989d;
    }

    .post-content {
        color: #e5e5ea;
    }

    .post-content a {
        color: #7aa2f7;
    }

    .post-meta {
        border-top-color: #3a3a3a;
    }

    .post-meta a {
        color: #7aa2f7;
    }

    .post-tags {
        color: #98989d;
    }
}
"#;
