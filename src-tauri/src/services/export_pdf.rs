use std::path::{Path, PathBuf};

use genpdf::elements;
use genpdf::style::{Color, Style};
use genpdf::{Alignment, Element as _, SimplePageDecorator};
use tokio::fs;

use crate::error::AppError;
use crate::models::WeiboPost;

pub struct PdfExportService;

impl PdfExportService {
    pub fn new() -> Self {
        Self
    }

    pub async fn export(&self, posts: &[WeiboPost], output_dir: impl AsRef<Path>) -> Result<(), AppError> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir).await.map_err(AppError::Io)?;

        let font_family = load_cjk_font_family()?;
        let mut doc = genpdf::Document::new(font_family);
        doc.set_title("微博导出");
        doc.set_minimal_conformance();
        doc.set_font_size(11);
        doc.set_line_spacing(1.25);

        let post_count = posts.len();
        let mut decorator = SimplePageDecorator::new();
        decorator.set_margins((12, 16));
        decorator.set_header(move |page| {
            let mut header = elements::Paragraph::new(format!(
                "微博导出 — 共 {} 条微博 — 第 {} 页",
                post_count, page
            ));
            header.set_alignment(Alignment::Center);
            header.styled(Style::new().with_font_size(9).with_color(Color::Greyscale(120)))
        });
        doc.set_page_decorator(decorator);

        let mut title = elements::Paragraph::new("微博导出");
        title.set_alignment(Alignment::Center);
        doc.push(title.styled(Style::new().with_font_size(20).bold()));

        let mut subtitle = elements::Paragraph::new(format!("共 {} 条微博", posts.len()));
        subtitle.set_alignment(Alignment::Center);
        doc.push(subtitle.styled(Style::new().with_font_size(12).with_color(Color::Greyscale(120))));
        doc.push(elements::Break::new(1.5));

        for (index, post) in posts.iter().enumerate() {
            if index > 0 {
                doc.push(elements::Break::new(0.8));
                doc.push(
                    elements::Paragraph::new("────────────────────────────────────────")
                        .styled(Style::new().with_color(Color::Greyscale(180))),
                );
                doc.push(elements::Break::new(0.6));
            }

            let header_text = format!("{} — {}", post.author, post.created_at);
            doc.push(elements::Paragraph::new(header_text).styled(Style::new().with_font_size(14).bold()));

            if let Some(region) = &post.region {
                doc.push(
                    elements::Paragraph::new(format!("地区：{}", region))
                        .styled(Style::new().with_font_size(10).with_color(Color::Greyscale(120))),
                );
            }

            if post.is_repost {
                if let Some(repost_user) = &post.repost_user {
                    doc.push(
                        elements::Paragraph::new(format!("转发自 @{}", repost_user))
                            .styled(Style::new().with_font_size(10).italic().with_color(Color::Rgb(90, 90, 180))),
                    );
                }
            }

            doc.push(elements::Break::new(0.3));

            let plain_text = strip_html_tags(&post.text);
            let content = if plain_text.is_empty() {
                "（无正文）".to_string()
            } else {
                plain_text
            };
            doc.push(elements::Paragraph::new(content));

            if !post.images.is_empty() {
                doc.push(elements::Break::new(0.4));
                doc.push(
                    elements::Paragraph::new(format!("图片（{} 张）", post.images.len()))
                        .styled(Style::new().with_font_size(10).bold().with_color(Color::Greyscale(110))),
                );

                for (image_index, image) in post.images.iter().enumerate() {
                    let image_ref = image.local_path.as_deref().unwrap_or(image.original_url.as_str());
                    doc.push(
                        elements::Paragraph::new(format!("- 图片 {}: {}", image_index + 1, image_ref))
                            .styled(Style::new().with_font_size(9).with_color(Color::Greyscale(120))),
                    );
                }
            }

            doc.push(elements::Break::new(0.3));
            doc.push(
                elements::Paragraph::new(format!("原文链接：{}", post.source_url))
                    .styled(Style::new().with_font_size(9).with_color(Color::Greyscale(110))),
            );
        }

        let destination = output_dir.join("微博导出.pdf");
        doc.render_to_file(&destination)
            .map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))?;

        Ok(())
    }
}

fn strip_html_tags(html: &str) -> String {
    let html = html
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");

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

    result = result
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#10;", "\n")
        .replace("&#13;", "\n");

    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

fn load_cjk_font_family() -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, AppError> {
    let mut attempts = Vec::new();

    for directory in candidate_font_directories() {
        for family in [
            "NotoSansSC",
            "SourceHanSansSC",
            "SourceHanSansCN",
            "MicrosoftYaHei",
            "WenQuanYiZenHei",
            "SimHei",
        ] {
            attempts.push(format!("{} in {}", family, directory.display()));
            if let Ok(font_family) = try_load_font_family(&directory, family) {
                return Ok(font_family);
            }
        }
    }

    Err(AppError::Parse(format!(
        "Failed to load a CJK font family. Checked: {}",
        attempts.join(", ")
    )))
}

fn candidate_font_directories() -> Vec<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut directories = vec![manifest_dir.join("assets/fonts")];

    if let Ok(executable_path) = std::env::current_exe() {
        if let Some(executable_dir) = executable_path.parent() {
            directories.push(executable_dir.join("assets/fonts"));
            directories.push(executable_dir.join("../Resources/assets/fonts"));
        }
    }

    directories.extend([
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("C:\\Windows\\Fonts"),
    ]);

    directories
}

fn try_load_font_family(
    directory: &Path,
    family: &str,
) -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, AppError> {
    let regular_path = resolve_font_file(directory, family, &["Regular", ""])
        .ok_or_else(|| AppError::Parse(format!("Missing regular font for {}", family)))?;
    let bold_path = resolve_font_file(directory, family, &["Bold"]).unwrap_or_else(|| regular_path.clone());
    let italic_path = resolve_font_file(directory, family, &["Italic"]).unwrap_or_else(|| regular_path.clone());
    let bold_italic_path = resolve_font_file(directory, family, &["BoldItalic", "BoldOblique", "Bold-Italic"])
        .unwrap_or_else(|| bold_path.clone());

    Ok(genpdf::fonts::FontFamily {
        regular: load_font_data(&regular_path)?,
        bold: load_font_data(&bold_path)?,
        italic: load_font_data(&italic_path)?,
        bold_italic: load_font_data(&bold_italic_path)?,
    })
}

fn resolve_font_file(directory: &Path, family: &str, styles: &[&str]) -> Option<PathBuf> {
    if !directory.exists() {
        return None;
    }

    let mut candidates = Vec::new();
    for style in styles {
        if style.is_empty() {
            candidates.push(format!("{}.ttf", family));
            candidates.push(format!("{}.otf", family));
        } else {
            candidates.push(format!("{}-{}.ttf", family, style));
            candidates.push(format!("{}-{}.otf", family, style));
        }
    }

    candidates
        .into_iter()
        .map(|candidate| directory.join(candidate))
        .find(|candidate| candidate.is_file())
}

fn load_font_data(path: &Path) -> Result<genpdf::fonts::FontData, AppError> {
    genpdf::fonts::FontData::load(path, None)
        .map_err(|error| AppError::Parse(format!("Failed to load font {}: {error}", path.display())))
}
