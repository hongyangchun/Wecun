use std::path::Path;

use tokio::fs;

use crate::error::AppError;
use crate::models::WeiboPost;
use crate::services::file_naming::{post_filename, sanitize_filename};

pub struct MarkdownExportService;

impl MarkdownExportService {
    pub fn new() -> Self {
        Self
    }

    pub async fn export(&self, posts: &[WeiboPost], output_dir: &str, single: bool) -> Result<(), AppError> {
        let output_dir = Path::new(output_dir);
        if single {
            self.export_single(posts, output_dir).await
        } else {
            self.export_per_post(posts, output_dir).await
        }
    }

    pub async fn export_single(&self, posts: &[WeiboPost], output_dir: &Path) -> Result<(), AppError> {
        let mut content = String::new();
        content.push_str(&format!("# {} 的微博导出\n\n", export_author_name(posts)));
        content.push_str(&format!("共 {} 条微博\n\n", posts.len()));
        content.push_str("---\n\n");

        for post in posts {
            content.push_str(&self.format_post(post, false));
            content.push_str("\n---\n\n");
        }

        let dest = output_dir.join(single_export_filename(posts));
        fs::write(&dest, content).await.map_err(AppError::Io)?;
        Ok(())
    }

    pub async fn export_per_post(&self, posts: &[WeiboPost], output_dir: &Path) -> Result<(), AppError> {
        let posts_dir = output_dir.join("posts");
        fs::create_dir_all(&posts_dir).await.map_err(AppError::Io)?;
        let mut index = format!("# {} 的微博目录\n\n", export_author_name(posts));

        for post in posts {
            let content = self.format_post(post, true);
            let filename = post_filename(&post.created_at, &post.mblogid, "md");
            let dest = posts_dir.join(&filename);
            fs::write(&dest, content).await.map_err(AppError::Io)?;
            index.push_str(&format!("- [{} — {}](./{})\n", post.author, post.created_at, filename));
        }

        fs::write(posts_dir.join("index.md"), index)
            .await
            .map_err(AppError::Io)?;

        Ok(())
    }

    fn format_post(&self, post: &WeiboPost, for_per_post_export: bool) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {}\n\n", post.author));

        let plain_text = html_to_markdown(&post.text);
        md.push_str(&plain_text);
        md.push_str("\n\n");

        if let Some(region) = &post.region {
            md.push_str(&format!("> {}\n\n", region));
        }

        if post.is_repost {
            if let Some(repost_user) = &post.repost_user {
                md.push_str(&format!("> 转发自 @{}\n\n", repost_user));
            }
        }

        if !post.images.is_empty() {
            md.push_str("### 图片\n\n");
            for (i, img) in post.images.iter().enumerate() {
                let img_ref = match img.local_path.as_deref() {
                    Some(path) if for_per_post_export => format!("../{path}"),
                    Some(path) => path.to_string(),
                    None => img.original_url.clone(),
                };
                md.push_str(&format!("![图片{}]({})\n\n", i + 1, img_ref));
            }
        }

        md.push_str("### 元数据\n\n");
        md.push_str(&format!("- 发布时间: {}\n", post.created_at));
        md.push_str(&format!("- 作者: {}\n", post.author));
        md.push_str(&format!("- 链接: {}\n", post.source_url));
        if !post.tags.is_empty() {
            md.push_str(&format!("- 标签: {}\n", post.tags.join(", ")));
        }

        md
    }
}

fn export_author_name(posts: &[WeiboPost]) -> String {
    posts.first()
        .map(|post| post.author.clone())
        .filter(|author| !author.trim().is_empty())
        .unwrap_or_else(|| "微博用户".to_string())
}

fn single_export_filename(posts: &[WeiboPost]) -> String {
    format!("{}-微博导出.md", sanitize_filename(&export_author_name(posts)))
}

fn html_to_markdown(html: &str) -> String {
    crate::utils::html::strip_html_tags(&convert_links(html))
}

fn convert_links(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(link_start) = remaining.find("<a ") {
        result.push_str(&remaining[..link_start]);
        remaining = &remaining[link_start..];

        let Some(href_start) = remaining.find("href=") else {
            result.push_str(remaining);
            return result;
        };
        let after_href = &remaining[href_start + 5..];
        let Some(quote) = after_href.chars().next() else {
            result.push_str(remaining);
            return result;
        };
        if quote != '"' && quote != '\'' {
            result.push_str(remaining);
            return result;
        }

        let after_quote = &after_href[1..];
        let Some(url_end) = after_quote.find(quote) else {
            result.push_str(remaining);
            return result;
        };
        let href = &after_quote[..url_end];

        let Some(tag_end) = remaining.find('>') else {
            result.push_str(remaining);
            return result;
        };
        let after_tag = &remaining[tag_end + 1..];
        let Some(close_idx) = after_tag.find("</a>") else {
            result.push_str(remaining);
            return result;
        };
        let text = &after_tag[..close_idx];
        result.push_str(&format!("[{}]({})", crate::utils::html::strip_html_tags(text), href));
        remaining = &after_tag[close_idx + 4..];
    }

    result.push_str(remaining);
    result
}
