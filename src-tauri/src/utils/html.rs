#[derive(Debug, Clone)]
struct HtmlTag {
    name: String,
    attrs: Vec<(String, String)>,
    is_closing: bool,
    self_closing: bool,
    end: usize,
}

#[derive(Debug, Clone)]
enum ListKind {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone)]
struct ListState {
    kind: ListKind,
    next_index: usize,
}

#[derive(Debug, Default)]
struct RenderState {
    list_stack: Vec<ListState>,
}

/// Check if an image URL is likely an emoji icon
/// Weibo emoji images are typically from specific patterns
fn is_emoji_image(src: &str) -> bool {
    // Weibo emoji URLs typically contain specific patterns
    src.contains("/emoji/")
        || src.contains("face/")
        || src.contains("expression")
        || (src.contains("sinaimg.cn") && src.contains("small"))
        || (src.contains("wx4.sinaimg.cn") && !src.contains("large"))
}

/// Strip HTML tags and decode common HTML entities from Weibo HTML content.
pub fn strip_html_tags(html: &str) -> String {
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
            '>' if in_tag => in_tag = false,
            '>' => result.push(ch),
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result = decode_html_entities(&result);

    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

pub fn html_to_markdown_rich(html: &str) -> String {
    // Pre-process: Convert consecutive <br> tags to paragraph breaks
    let preprocessed = html
        .replace("<br />", "<br>")
        .replace("<br/>", "<br>")
        .replace("<br><br>", "</p><p>")
        .replace("<br><br>", "</p><p>");

    let mut pos = 0;
    let mut state = RenderState::default();
    let rendered = render_nodes(&preprocessed, &mut pos, None, &mut state);
    normalize_markdown(&decode_html_entities(&rendered))
}

fn render_nodes(
    html: &str,
    pos: &mut usize,
    until_tag: Option<&str>,
    state: &mut RenderState,
) -> String {
    let mut output = String::new();

    while *pos < html.len() {
        if html[*pos..].starts_with("<!--") {
            if let Some(end) = html[*pos + 4..].find("-->") {
                *pos += end + 7;
            } else {
                *pos = html.len();
            }
            continue;
        }

        if html.as_bytes()[*pos] == b'<' {
            let Some(tag) = parse_tag(html, *pos) else {
                output.push('<');
                *pos += 1;
                continue;
            };
            *pos = tag.end;

            if tag.is_closing {
                if until_tag == Some(tag.name.as_str()) {
                    break;
                }
                continue;
            }

            match tag.name.as_str() {
                "br" => output.push('\n'),
                "p" => {
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some("p"), state)
                    };
                    push_block(&mut output, inner);
                }
                "b" | "strong" => {
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some(tag.name.as_str()), state)
                    };
                    let inner = trim_inline(&inner);
                    if !inner.is_empty() {
                        output.push_str("**");
                        output.push_str(&inner);
                        output.push_str("**");
                    }
                }
                "i" | "em" => {
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some(tag.name.as_str()), state)
                    };
                    let inner = trim_inline(&inner);
                    if !inner.is_empty() {
                        output.push('*');
                        output.push_str(&inner);
                        output.push('*');
                    }
                }
                "a" => {
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some("a"), state)
                    };
                    let href = tag
                        .attr("href")
                        .map(decode_html_entities)
                        .unwrap_or_default();
                    let text = trim_inline(&inner);
                    if href.is_empty() {
                        output.push_str(&inner);
                    } else if text.is_empty() {
                        output.push_str(&format!("[{href}]({href})"));
                    } else {
                        output.push_str(&format!("[{text}]({href})"));
                    }
                }
                "img" => {
                    let src = tag
                        .attr("src")
                        .map(decode_html_entities)
                        .unwrap_or_default();
                    if !src.is_empty() {
                        // Check if this is a small emoji image
                        let is_emoji = is_emoji_image(&src);
                        if is_emoji {
                            // For emoji images, just skip them in Markdown
                            // The emoji should already be in the text content
                        } else {
                            let alt = tag
                                .attr("alt")
                                .map(decode_html_entities)
                                .unwrap_or_default();
                            output.push_str(&format!("![{alt}]({src})"));
                        }
                    }
                }
                "ul" => {
                    state.list_stack.push(ListState {
                        kind: ListKind::Unordered,
                        next_index: 1,
                    });
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some("ul"), state)
                    };
                    state.list_stack.pop();
                    push_block(&mut output, inner);
                }
                "ol" => {
                    state.list_stack.push(ListState {
                        kind: ListKind::Ordered,
                        next_index: 1,
                    });
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some("ol"), state)
                    };
                    state.list_stack.pop();
                    push_block(&mut output, inner);
                }
                "li" => {
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some("li"), state)
                    };
                    let prefix = next_list_prefix(state);
                    let item = format_list_item(&inner, &prefix);
                    if !item.is_empty() {
                        if !output.is_empty() && !output.ends_with('\n') {
                            output.push('\n');
                        }
                        output.push_str(&item);
                        output.push('\n');
                    }
                }
                "blockquote" => {
                    let inner = if tag.self_closing {
                        String::new()
                    } else {
                        render_nodes(html, pos, Some("blockquote"), state)
                    };
                    let level = blockquote_level(&output) + 1;
                    push_block(&mut output, format_blockquote(&inner, level));
                }
                _ => {
                    if !tag.self_closing {
                        output.push_str(&render_nodes(html, pos, Some(tag.name.as_str()), state));
                    }
                }
            }

            continue;
        }

        let next_tag = html[*pos..]
            .find('<')
            .map(|idx| *pos + idx)
            .unwrap_or(html.len());
        output.push_str(&html[*pos..next_tag]);
        *pos = next_tag;
    }

    output
}

fn parse_tag(html: &str, start: usize) -> Option<HtmlTag> {
    if html.as_bytes().get(start) != Some(&b'<') {
        return None;
    }

    let bytes = html.as_bytes();
    let len = html.len();
    let mut i = start + 1;
    let mut is_closing = false;

    if bytes.get(i) == Some(&b'/') {
        is_closing = true;
        i += 1;
    }

    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    let name_start = i;
    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
        i += 1;
    }
    if i == name_start {
        return None;
    }

    let name = html[name_start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let mut self_closing = false;

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        if bytes[i] == b'>' {
            i += 1;
            break;
        }

        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>') {
            self_closing = true;
            i += 2;
            break;
        }

        let attr_start = i;
        while i < len
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
            && !(bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>'))
        {
            i += 1;
        }

        if i == attr_start {
            i += 1;
            continue;
        }

        let attr_name = html[attr_start..i].to_ascii_lowercase();
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let mut value = String::new();
        if bytes.get(i) == Some(&b'=') {
            i += 1;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            if let Some(quote) = bytes.get(i).copied() {
                if quote == b'"' || quote == b'\'' {
                    i += 1;
                    let value_start = i;
                    while i < len && bytes[i] != quote {
                        i += 1;
                    }
                    value = html[value_start..i.min(len)].to_string();
                    if i < len {
                        i += 1;
                    }
                } else {
                    let value_start = i;
                    while i < len
                        && !bytes[i].is_ascii_whitespace()
                        && bytes[i] != b'>'
                        && !(bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>'))
                    {
                        i += 1;
                    }
                    value = html[value_start..i].to_string();
                }
            }
        }

        attrs.push((attr_name, value));
    }

    if matches!(name.as_str(), "br" | "img" | "hr") {
        self_closing = true;
    }

    Some(HtmlTag {
        name,
        attrs,
        is_closing,
        self_closing,
        end: i,
    })
}

impl HtmlTag {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }
}

fn trim_inline(value: &str) -> String {
    value
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_block(output: &mut String, block: String) {
    let block = block.trim();
    if block.is_empty() {
        return;
    }

    if !output.is_empty() && !output.ends_with("\n\n") {
        if output.ends_with('\n') {
            output.push('\n');
        } else {
            output.push_str("\n\n");
        }
    }

    output.push_str(block);
    output.push_str("\n\n");
}

fn next_list_prefix(state: &mut RenderState) -> String {
    let depth = state.list_stack.len().saturating_sub(1);
    let indent = "  ".repeat(depth);

    match state.list_stack.last_mut() {
        Some(list) => match list.kind {
            ListKind::Unordered => format!("{indent}- "),
            ListKind::Ordered => {
                let index = list.next_index;
                list.next_index += 1;
                format!("{indent}{index}. ")
            }
        },
        None => "- ".to_string(),
    }
}

fn format_list_item(inner: &str, prefix: &str) -> String {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let continuation = " ".repeat(prefix.chars().count());
    let mut lines = trimmed.lines();
    let Some(first) = lines.next() else {
        return String::new();
    };

    let mut result = format!("{prefix}{}", first.trim());
    for line in lines {
        result.push('\n');
        if line.trim().is_empty() {
            continue;
        }
        result.push_str(&continuation);
        result.push_str(line.trim());
    }

    result
}

fn format_blockquote(inner: &str, level: usize) -> String {
    let prefix = "> ".repeat(level);
    let mut result = String::new();

    for line in inner.trim().lines() {
        if !result.is_empty() {
            result.push('\n');
        }
        if line.trim().is_empty() {
            result.push_str(prefix.trim_end());
        } else {
            result.push_str(&prefix);
            result.push_str(line.trim());
        }
    }

    result
}

fn blockquote_level(output: &str) -> usize {
    output
        .lines()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('>') {
                Some(trimmed.chars().take_while(|ch| *ch == '>').count())
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#10;", "\n")
        .replace("&#13;", "\n")
}

fn normalize_markdown(value: &str) -> String {
    let mut result = value.replace("\r\n", "\n").replace('\r', "\n");
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result.trim().to_string()
}

/// Count visible text characters (stripping HTML tags) from Weibo HTML content.
pub fn count_plain_text_chars(html: &str) -> usize {
    strip_html_tags(html)
        .replace('\n', "")
        .replace(' ', "")
        .chars()
        .count()
}

#[cfg(test)]
mod tests {
    use super::html_to_markdown_rich;

    #[test]
    fn rich_converter_preserves_supported_html_formatting() {
        let html = r#"<p><strong>加粗</strong> <em>斜体</em> <a href="https://weibo.com/example">链接</a><br>下一行</p><blockquote><p>引用内容</p></blockquote><ol><li>第一项</li><li>第二项</li></ol><p><img src="https://example.com/a.jpg" alt="配图"></p>"#;

        assert_eq!(
            html_to_markdown_rich(html),
            "**加粗** *斜体* [链接](https://weibo.com/example)\n下一行\n\n> 引用内容\n\n1. 第一项\n2. 第二项\n\n![配图](https://example.com/a.jpg)"
        );
    }
}
