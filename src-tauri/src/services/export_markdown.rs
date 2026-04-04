use std::path::Path;

use tokio::fs;

use crate::error::AppError;
use crate::models::WeiboPost;
use crate::services::file_naming::post_filename;

pub struct MarkdownExportService;

impl MarkdownExportService {
    pub fn new() -> Self {
        Self
    }

    pub async fn export(&self, posts: &[WeiboPost], output_dir: &str) -> Result<(), AppError> {
        let output_dir = Path::new(output_dir);
        self.export_single(posts, output_dir).await?;
        self.export_per_post(posts, output_dir).await
    }

    pub async fn export_single(&self, posts: &[WeiboPost], output_dir: &Path) -> Result<(), AppError> {
        let mut content = String::new();
        content.push_str("# 微博导出\n\n");
        content.push_str(&format!("共 {} 条微博\n\n", posts.len()));
        content.push_str("---\n\n");

        for post in posts {
            content.push_str(&self.format_post(post));
            content.push_str("\n---\n\n");
        }

        let dest = output_dir.join("微博导出.md");
        fs::write(&dest, content).await.map_err(AppError::Io)?;
        Ok(())
    }

    pub async fn export_per_post(&self, posts: &[WeiboPost], output_dir: &Path) -> Result<(), AppError> {
        let posts_dir = output_dir.join("posts");
        fs::create_dir_all(&posts_dir).await.map_err(AppError::Io)?;

        for post in posts {
            let content = self.format_post(post);
            let filename = post_filename(&post.created_at, &post.mblogid, "md");
            let dest = posts_dir.join(&filename);
            fs::write(&dest, content).await.map_err(AppError::Io)?;
        }

        Ok(())
    }

    fn format_post(&self, post: &WeiboPost) -> String {
        let mut md = String::new();

        md.push_str(&format!("## {} — @{}\n\n", post.author, post.created_at));

        if let Some(region) = &post.region {
            md.push_str(&format!("📍 {}\n\n", region));
        }

        if post.is_repost {
            if let Some(repost_user) = &post.repost_user {
                md.push_str(&format!("> 转发自 @{}\n\n", repost_user));
            }
        }

        md.push_str(&format!("[查看原微博]({})\n\n", post.source_url));

        let plain_text = strip_html_tags(&post.text);
        md.push_str(&plain_text);
        md.push_str("\n\n");

        if !post.images.is_empty() {
            md.push_str("### 图片\n\n");
            for (i, img) in post.images.iter().enumerate() {
                let img_ref = img.local_path.as_deref().unwrap_or(&img.original_url);
                md.push_str(&format!("![图片{}]({})\n\n", i + 1, img_ref));
            }
        }

        md.push_str(&format!("\n*微博ID: {}*\n", post.mblogid));

        md
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result = result.replace("&nbsp;", " ");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&amp;", "&");
    result = result.replace("&quot;", "\"");

    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}
