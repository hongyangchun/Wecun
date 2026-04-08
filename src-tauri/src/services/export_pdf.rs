use std::path::Path;

use tokio::fs;

use crate::error::AppError;
use crate::models::{ExportContext, WeiboPost};
use crate::services::file_naming::sanitize_filename;

pub struct PdfExportService;

impl PdfExportService {
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

        let author_name = posts
            .first()
            .map(|p| p.author.as_str())
            .unwrap_or("微博用户");

        let title = if export_context.type_label == "收藏微博" {
            "我收藏的微博".to_string()
        } else {
            format!("{} 的微博导出", author_name)
        };

        let typst_markup = Self::render_typst(posts, &title, author_name, export_context);

        let pdf_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, AppError> {
            Self::compile_pdf(&typst_markup)
        })
        .await
        .map_err(|e| AppError::ApiError(format!("PDF编译任务失败: {e}")))??;

        let filename = format!(
            "{}-{}.pdf",
            sanitize_filename(&export_context.type_label),
            sanitize_filename(&export_context.date_range_label)
        );
        let dest = output_dir.join(filename);
        fs::write(&dest, pdf_bytes).await.map_err(AppError::Io)?;
        Ok(())
    }

    fn render_typst(
        posts: &[WeiboPost],
        title: &str,
        author_name: &str,
        export_context: &ExportContext,
    ) -> String {
        let mut s = String::with_capacity(64 * 1024);

        s.push_str("#set page(paper: \"a4\", margin: (top: 2.5cm, bottom: 2.5cm, left: 2cm, right: 2cm))\n");
        s.push_str("#set text(font: (\"New Computer Modern\", \"Noto Sans SC\", \"PingFang SC\", \"Microsoft YaHei\", \"SimSun\"), size: 11pt, lang: \"zh\")\n");
        s.push_str("#set par(leading: 0.8em, justify: true)\n");
        s.push_str("#set heading(numbering: none)\n\n");
        s.push_str("#align(center)[\n");
        s.push_str("  #v(3cm)\n");
        s.push_str(&format!(
            "#text(size: 28pt, weight: \"bold\", font: (\"Noto Sans SC\", \"PingFang SC\", \"Microsoft YaHei\"))[{}\n]\n",
            typst_escape(title)
        ));
        s.push_str("  #v(1cm)\n");
        s.push_str(&format!(
            "#text(size: 14pt, fill: luma(100))[{}\n]\n",
            typst_escape(author_name)
        ));
        s.push_str("  #v(0.5cm)\n");
        s.push_str(&format!(
            "#text(size: 11pt, fill: luma(120))[共 {} 条微博 | {}\n]\n",
            posts.len(),
            typst_escape(&export_context.date_range_label)
        ));
        s.push_str(&format!(
            "#text(size: 10pt, fill: luma(140))[由微存 (Wecun) 导出\n]\n"
        ));
        s.push_str("]\n\n");

        s.push_str("#pagebreak()\n\n");

        s.push_str("#outline(title: \"目录\", indent: auto, depth: 1)\n");
        s.push_str("#pagebreak()\n\n");
        for (i, post) in posts.iter().enumerate() {
            let heading = if post.created_at.len() > 10 {
                &post.created_at[..10]
            } else {
                &post.created_at
            };
            let heading = format!("{}. {}", i + 1, heading);

            s.push_str(&format!("= {}\n\n", typst_escape(&heading)));

            if let Some(region) = &post.region {
                s.push_str(&format!(
                    "#text(size: 9pt, fill: luma(120))[{}]\n\n",
                    typst_escape(region)
                ));
            }

            if post.is_repost {
                if let Some(repost_user) = &post.repost_user {
                    s.push_str(&format!(
                        "#text(size: 10pt, fill: rgb(\"#5b5bb4\"), style: \"italic\")[转发自 @{}]\n\n",
                        typst_escape(repost_user)
                    ));
                }
            }

            let plain_text = html_to_plain_text(&post.text);
            for line in plain_text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    s.push('\n');
                } else {
                    s.push_str(&typst_escape(trimmed));
                    s.push('\n');
                }
            }
            s.push('\n');

            if !post.images.is_empty() {
                s.push_str(&format!(
                    "#text(size: 9pt, fill: luma(100))[共 {} 张图片",
                    post.images.len()
                ));
                for (j, img) in post.images.iter().enumerate() {
                    if let Some(local) = &img.local_path {
                        let img_path = std::path::Path::new(local);
                        if img_path.exists() {
                            s.push_str(&format!(
                                "\n#figure(\n  image(\"{}\", width: 80%),\n  caption: [图片{}]\n)",
                                typst_escape(&local.replace('\\', "/")),
                                j + 1
                            ));
                        }
                    }
                }
                s.push_str("]\n\n");
            }

            s.push_str(&format!(
                "#text(size: 9pt, fill: luma(100))[原文链接：{}]\n",
                typst_escape(&post.source_url)
            ));
            if !post.tags.is_empty() {
                s.push_str(&format!(
                    "#text(size: 9pt, fill: luma(100))[标签：{}]\n",
                    typst_escape(&post.tags.join("、"))
                ));
            }
            s.push_str("#line(length: 100%, stroke: 0.5pt + luma(220))\n\n");
        }

        s.push_str("#pagebreak()\n");
        s.push_str("#align(center)[\n");
        s.push_str("  #v(6cm)\n");
        s.push_str("#text(size: 12pt, fill: luma(140))[— 全文完 —]\n");
        s.push_str("  #v(1cm)\n");
        s.push_str("#text(size: 10pt, fill: luma(160))[由微存 (Wecun) 导出\n]\n");
        s.push_str("]\n");

        s
    }

    fn compile_pdf(typst_markup: &str) -> Result<Vec<u8>, AppError> {
        use typst::diag::{FileError, FileResult};
        use typst::foundations::{Bytes, Datetime};
        use typst::layout::PagedDocument;
        use typst::syntax::{FileId, Source};
        use typst::text::{Font, FontBook};
        use typst::utils::LazyHash;
        use typst::{Library, LibraryExt, World};

        struct WecunWorld {
            library: LazyHash<Library>,
            book: LazyHash<FontBook>,
            font_slots: Vec<typst_kit::fonts::FontSlot>,
            source: Source,
        }

        impl World for WecunWorld {
            fn library(&self) -> &LazyHash<Library> {
                &self.library
            }
            fn book(&self) -> &LazyHash<FontBook> {
                &self.book
            }
            fn main(&self) -> FileId {
                self.source.id()
            }
            fn source(&self, id: FileId) -> FileResult<Source> {
                if id == self.source.id() {
                    Ok(self.source.clone())
                } else {
                    Err(FileError::NotFound(
                        id.vpath().as_rootless_path().to_path_buf(),
                    ))
                }
            }
            fn file(&self, id: FileId) -> FileResult<Bytes> {
                let path = id.vpath().as_rooted_path();
                if let Ok(data) = std::fs::read(path) {
                    return Ok(Bytes::new(data));
                }
                Err(FileError::NotFound(
                    id.vpath().as_rootless_path().to_path_buf(),
                ))
            }
            fn font(&self, index: usize) -> Option<Font> {
                self.font_slots.get(index)?.get()
            }
            fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
                None
            }
        }

        let fonts = typst_kit::fonts::FontSearcher::new().search();

        let library = Library::default();
        let source = Source::detached(typst_markup);

        let world = WecunWorld {
            library: LazyHash::new(library),
            book: LazyHash::new(fonts.book),
            font_slots: fonts.fonts,
            source,
        };

        let warned = typst::compile::<PagedDocument>(&world);
        let document = warned
            .output
            .map_err(|errs| AppError::ApiError(format!("Typst编译错误: {:?}", errs)))?;

        let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .map_err(|errs| AppError::ApiError(format!("PDF生成错误: {:?}", errs)))?;

        Ok(pdf_bytes)
    }
}

fn typst_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('#', "\\#")
        .replace('$', "\\$")
        .replace('@', "\\@")
        .replace('~', "\\~")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('^', "\\^")
        .replace('"', "\\\"")
}

fn html_to_plain_text(html: &str) -> String {
    let text = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</li>", "\n")
        .replace("</blockquote>", "\n")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    let mut cleaned = String::with_capacity(result.len());
    let mut last_was_newline = false;
    for line in result.lines() {
        if line.trim().is_empty() {
            if !last_was_newline {
                cleaned.push('\n');
                last_was_newline = true;
            }
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
            last_was_newline = false;
        }
    }

    cleaned.trim().to_string()
}
