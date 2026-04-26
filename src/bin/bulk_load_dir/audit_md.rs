use std::collections::BTreeMap;

pub fn parse_context_id(content: &str) -> Option<i32> {
    for line in content.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("- context_id:") else {
            continue;
        };
        let value = rest.trim();
        if value == "-" {
            return None;
        }
        if let Ok(parsed) = value.parse::<i32>()
            && parsed > 0
        {
            return Some(parsed);
        }
    }
    None
}

pub fn parse_done_entries(content: &str) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for line in content.lines() {
        if !line.starts_with('|') {
            continue;
        }
        let cols: Vec<String> = line
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if cols.len() < 3 {
            continue;
        }
        let done = cols.get(1).is_some_and(|s| s == "[x]");
        let path = cols.get(2).map_or(String::new(), Clone::clone);
        let size_bytes = cols
            .get(3)
            .and_then(|s| s.parse::<u64>().ok())
            .map_or(0, |value| value);
        if done && !path.is_empty() && path != "path" {
            out.insert(path, size_bytes);
        }
    }
    out
}

pub fn parse_max_row_index(content: &str) -> usize {
    let mut max_index = 0usize;
    for line in content.lines() {
        if !line.starts_with('|') {
            continue;
        }
        let cols: Vec<&str> = line
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if cols.is_empty() {
            continue;
        }
        if cols[0] == "#" {
            continue;
        }
        if let Ok(value) = cols[0].parse::<usize>()
            && value > max_index
        {
            max_index = value;
        }
    }
    max_index
}

pub fn escape_md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

pub fn rewrite_context_id(content: &str, context_id: i32) -> String {
    let mut out = String::new();
    for line in content.lines() {
        if line.trim_start().starts_with("- context_id:") {
            out.push_str(&format!("- context_id: {context_id}\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
