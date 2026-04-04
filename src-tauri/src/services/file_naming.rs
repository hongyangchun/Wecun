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
        let mut candidate = name.clone();
        let mut counter = 1;
        while seen.contains(&candidate) {
            if let Some(dot_pos) = candidate.rfind('.') {
                let base = &name[..dot_pos];
                let ext = &name[dot_pos..];
                candidate = format!("{base}_{counter}{ext}");
            } else {
                candidate = format!("{name}_{counter}");
            }
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

fn parse_date_prefix(created_at: &str) -> String {
    if created_at.len() >= 10 && created_at.chars().nth(4) == Some('-') {
        return created_at[..10].to_string();
    }

    let months = [
        ("Jan", "01"), ("Feb", "02"), ("Mar", "03"), ("Apr", "04"),
        ("May", "05"), ("Jun", "06"), ("Jul", "07"), ("Aug", "08"),
        ("Sep", "09"), ("Oct", "10"), ("Nov", "11"), ("Dec", "12"),
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
