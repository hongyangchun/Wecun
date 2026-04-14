use std::collections::HashSet;

pub fn sanitize_filename(input: &str) -> String {
    let forbidden = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut result = String::with_capacity(input.len());

    for ch in input.chars() {
        if forbidden.contains(&ch) {
            result.push('_');
        } else if !ch.is_control() {
            result.push(ch);
        }
    }

    let trimmed = result.trim().trim_matches('.');
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.chars().take(200).collect()
    }
}

pub fn dedupe_filenames(names: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(names.len());

    for name in names {
        let (base, ext) = match name.rfind('.') {
            Some(dot_pos) => (&name[..dot_pos], &name[dot_pos..]),
            None => (name.as_str(), ""),
        };

        let mut candidate = name.clone();
        let mut counter = 1;
        while seen.contains(&candidate) {
            candidate = if ext.is_empty() {
                format!("{name}_{counter}")
            } else {
                format!("{base}_{counter}{ext}")
            };
            counter += 1;
        }
        seen.insert(candidate.clone());
        result.push(candidate);
    }

    result
}

pub fn post_filename(created_at: &str, mblogid: &str, extension: &str) -> String {
    let date_prefix = parse_date_prefix(created_at);
    format!("{date_prefix}_{mblogid}.{extension}")
}

pub fn obsidian_post_filename(created_at: &str, title_hint: &str, extension: &str) -> String {
    let date_prefix = parse_date_prefix(created_at);
    let title = sanitize_filename(&truncate_chars(&normalize_title_hint(title_hint), 10));
    format!("{date_prefix}-{title}.{extension}")
}

pub fn markdown_export_filename(type_label: &str, date_range_label: &str) -> String {
    format!(
        "{}-{}.md",
        sanitize_filename(type_label),
        sanitize_filename(date_range_label)
    )
}

/// Generate unified export filename with author name
/// Format: "AuthorName_Type-StartDate-EndDate.ext"
pub fn unified_export_filename(author_name: &str, type_label: &str, date_range_label: &str, extension: &str) -> String {
    let author = sanitize_filename(author_name);
    let type_part = sanitize_filename(type_label);
    let date_part = sanitize_filename(date_range_label);
    format!("{}_{}_{}.{}", author, type_part, date_part, extension)
}

/// Parse date range label into start-end format
/// Input: "2024-01-01至2024-12-31" or "全部时间"
/// Output: "2024-01-01-2024-12-31" or "all-time"
pub fn format_date_range_for_filename(date_range_label: &str) -> String {
    if date_range_label == "全部时间" {
        return "全部时间".to_string();
    }
    // Convert "2024-01-01至2024-12-31" to "2024-01-01-2024-12-31"
    date_range_label.replace("至", "-")
}

pub fn parse_date_prefix(created_at: &str) -> String {
    if created_at.len() >= 10 && created_at.chars().nth(4) == Some('-') {
        return created_at[..10].to_string();
    }

    let months = [
        ("Jan", "01"),
        ("Feb", "02"),
        ("Mar", "03"),
        ("Apr", "04"),
        ("May", "05"),
        ("Jun", "06"),
        ("Jul", "07"),
        ("Aug", "08"),
        ("Sep", "09"),
        ("Oct", "10"),
        ("Nov", "11"),
        ("Dec", "12"),
    ];

    let parts: Vec<&str> = created_at.split_whitespace().collect();
    if parts.len() >= 5 {
        let month_str = parts[1];
        let day = parts[2];
        let year = if parts.len() >= 6 { parts[5] } else { "2024" };

        for (abbr, num) in &months {
            if month_str.starts_with(abbr) {
                let day_padded = if day.len() == 1 {
                    format!("0{day}")
                } else {
                    day.to_string()
                };
                return format!("{year}-{num}-{day_padded}");
            }
        }
    }

    "unknown".to_string()
}

fn normalize_title_hint(input: &str) -> String {
    let collapsed = input.split_whitespace().collect::<String>();
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        "微博".to_string()
    } else {
        trimmed.to_string()
    }
}

fn truncate_chars(input: &str, limit: usize) -> String {
    input.chars().take(limit).collect()
}
