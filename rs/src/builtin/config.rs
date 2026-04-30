use crate::core::config::GitConfig;
use std::fs;
use std::path::Path;

pub fn cmd_config(key: &str, value: Option<&str>) {
    if let Some(value) = value {
        set_config(key, value);
    } else if let Some(value) = GitConfig::load_default().get(key) {
        println!("{}", value);
    }
}

fn set_config(key: &str, value: &str) {
    let mut parts = key.splitn(2, '.');
    let section = parts.next().unwrap();
    let name = parts.next().unwrap_or("");
    let config_path = Path::new(".git/config");
    let mut lines: Vec<String> = fs::read_to_string(config_path)
        .unwrap_or_default()
        .lines()
        .map(|line| line.to_string())
        .collect();

    let section_header = format!("[{}]", section);
    let mut section_start = None;
    let mut section_end = lines.len();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == section_header {
            section_start = Some(idx);
        } else if section_start.is_some() && trimmed.starts_with('[') && trimmed.ends_with(']') {
            section_end = idx;
            break;
        }
    }

    if let Some(start) = section_start {
        for line in lines.iter_mut().take(section_end).skip(start + 1) {
            if line.trim_start().starts_with(&format!("{} ", name))
                || line.trim_start().starts_with(&format!("{}=", name))
            {
                *line = format!("\t{} = {}", name, value);
                fs::write(config_path, lines.join("\n") + "\n").unwrap();
                return;
            }
        }
        lines.insert(section_end, format!("\t{} = {}", name, value));
    } else {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(section_header);
        lines.push(format!("\t{} = {}", name, value));
    }

    fs::write(config_path, lines.join("\n") + "\n").unwrap();
}
