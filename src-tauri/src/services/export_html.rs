use std::collections::{HashMap, HashSet};
use std::path::Path;

use tokio::fs;

use crate::error::AppError;
use crate::models::{ExportContext, WeiboPost};
use crate::services::file_naming::{format_date_range_for_filename, unified_export_filename};

// Pre-computed sets for ammonia configuration
fn get_allowed_tags() -> HashSet<&'static str> {
    HashSet::from([
        "a", "b", "blockquote", "br", "code", "div", "em", "h1", "h2", "h3", "h4", "h5", "h6",
        "hr", "i", "img", "li", "ol", "p", "pre", "span", "strong", "table", "tbody", "td",
        "th", "thead", "tr", "ul",
    ])
}

fn get_tag_attributes() -> HashMap<&'static str, HashSet<&'static str>> {
    HashMap::from([
        ("a", HashSet::from(["href", "title"])),
        ("img", HashSet::from(["src", "alt", "title", "width", "height"])),
        ("th", HashSet::from(["colspan", "rowspan"])),
        ("td", HashSet::from(["colspan", "rowspan"])),
    ])
}

pub struct HtmlExportService;

impl HtmlExportService {
    pub fn new() -> Self {
        Self
    }

    pub async fn export(
        &self,
        posts: &[WeiboPost],
        output_dir: impl AsRef<Path>,
        export_context: &ExportContext,
    ) -> Result<(), AppError> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir).await.map_err(AppError::Io)?;

        // For favorites, use "我的收藏" as author name; otherwise use first post's author
        let author_name = if export_context.type_label == "收藏微博" {
            "我的收藏"
        } else {
            posts.first()
                .map(|p| p.author.as_str())
                .unwrap_or("微博用户")
        };

        let html = self.render(posts, author_name, export_context);
        let date_part = format_date_range_for_filename(&export_context.date_range_label);
        let filename = unified_export_filename(author_name, &export_context.type_label, &date_part, "html");
        let dest = output_dir.join(filename);
        fs::write(&dest, html).await.map_err(AppError::Io)?;
        Ok(())
    }

    fn render(&self, posts: &[WeiboPost], author_name: &str, export_context: &ExportContext) -> String {
        let mut html = String::with_capacity(64 * 1024);

        let title = if export_context.type_label == "收藏微博" {
            "我收藏的微博".to_string()
        } else {
            format!("{} 的微博导出", author_name)
        };

        html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str(&format!(
            "<title>{}</title>\n",
            html_escape(&title)
        ));
        html.push_str("<style>\n");
        html.push_str(CSS);
        html.push_str("\n</style>\n");
        html.push_str("</head>\n<body>\n<div class=\"container\">\n");

        html.push_str("<header>\n");
        html.push_str(&format!(
            "<h1>{}</h1>\n",
            html_escape(&title)
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
    use ammonia::Builder;

    // Pre-process: normalize br tags but preserve existing structure
    let preprocessed = html
        .replace("<br />", "<br>")
        .replace("<br/>", "<br>")
        // Convert consecutive br tags to paragraph breaks only for content without p tags
        .replace("<br><br>", "</p><p>");

    // Don't wrap content that already has block-level structure
    let has_structure = preprocessed.contains("<p>") || preprocessed.contains("<div>")
        || preprocessed.contains("<blockquote>") || preprocessed.contains("<ul")
        || preprocessed.contains("<ol") || preprocessed.contains("<h");

    let content_to_clean = if has_structure {
        preprocessed
    } else if preprocessed.contains("<br>") {
        // Content with br tags but no structure - wrap in p and convert br
        format!("<p>{}</p>", preprocessed.replace("<br>", "<br/>"))
    } else {
        // Plain text content - wrap in p
        format!("<p>{}</p>", preprocessed)
    };

    // Configure ammonia to preserve structure
    let clean = Builder::default()
        .tags(get_allowed_tags())
        .tag_attributes(get_tag_attributes())
        .link_rel(None)
        .url_relative(ammonia::UrlRelative::PassThrough)
        .clean(&content_to_clean);

    clean.to_string()
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
    line-height: 1.8;
    margin-bottom: 12px;
    word-wrap: break-word;
    /* Preserve line breaks in text content */
    white-space: pre-line;
}

/* Ensure all block-level elements have proper spacing */
.post-content p,
.post-content div,
.post-content blockquote,
.post-content ul,
.post-content ol,
.post-content h1,
.post-content h2,
.post-content h3,
.post-content h4,
.post-content h5,
.post-content h6,
.post-content pre {
    margin-top: 0.8em;
    margin-bottom: 0.8em;
}

/* First element shouldn't have top margin */
.post-content > *:first-child {
    margin-top: 0;
}

/* Last element shouldn't have bottom margin */
.post-content > *:last-child {
    margin-bottom: 0;
}

/* Small inline images (emojis) */
.post-content img[width="20"],
.post-content img[height="20"],
.post-content img[width="21"],
.post-content img[height="21"],
.post-content img[width="22"],
.post-content img[height="22"],
.post-content img[width="23"],
.post-content img[height="23"] {
    max-width: 1.2em !important;
    max-height: 1.2em !important;
    vertical-align: middle !important;
    display: inline !important;
}

/* For emoji images without explicit dimensions but small src URLs */
.post-content img:not([src*="wx4.sinaimg"]):not([src*="large"]) {
    max-width: 1.4em;
    max-height: 1.4em;
    vertical-align: middle;
    display: inline;
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

    /* Ensure all block-level elements have proper spacing in dark mode */
    .post-content p,
    .post-content div,
    .post-content blockquote,
    .post-content ul,
    .post-content ol,
    .post-content h1,
    .post-content h2,
    .post-content h3,
    .post-content h4,
    .post-content h5,
    .post-content h6,
    .post-content pre {
        margin-top: 0.8em;
        margin-bottom: 0.8em;
    }

    .post-content > *:first-child {
        margin-top: 0;
    }

    .post-content > *:last-child {
        margin-bottom: 0;
    }

    .post-content img[width="20"],
    .post-content img[height="20"],
    .post-content img[width="21"],
    .post-content img[height="21"],
    .post-content img[width="22"],
    .post-content img[height="22"],
    .post-content img[width="23"],
    .post-content img[height="23"] {
        max-width: 1.2em !important;
        max-height: 1.2em !important;
        vertical-align: middle !important;
        display: inline !important;
    }

    .post-content img:not([src*="wx4.sinaimg"]):not([src*="large"]) {
        max-width: 1.4em;
        max-height: 1.4em;
        vertical-align: middle;
        display: inline;
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
