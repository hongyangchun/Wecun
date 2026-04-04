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

/// Count visible text characters (stripping HTML tags) from Weibo HTML content.
pub fn count_plain_text_chars(html: &str) -> usize {
    strip_html_tags(html)
        .replace('\n', "")
        .replace(' ', "")
        .chars()
        .count()
}
