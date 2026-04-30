use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Default)]
pub struct GitConfig {
    pub entries: HashMap<String, String>,
}

impl GitConfig {
    pub fn load_default() -> Self {
        let mut config = Self::default();

        // System config (placeholder)
        // Global config (~/.gitconfig)
        if let Some(home) = dirs::home_dir() {
            config.load_file(&home.join(".gitconfig"));
        }

        // Repository config (.git/config)
        config.load_file(Path::new(".git/config"));

        config
    }

    pub fn load_file(&mut self, path: &Path) {
        if let Ok(content) = fs::read_to_string(path) {
            let mut current_section = String::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                    continue;
                }

                if line.starts_with('[') && line.ends_with(']') {
                    current_section = line[1..line.len() - 1].to_string();
                    continue;
                }

                if let Some(pos) = line.find('=') {
                    let key = line[..pos].trim();
                    let value = line[pos + 1..].trim();
                    let full_key = if current_section.is_empty() {
                        key.to_string()
                    } else {
                        format!("{}.{}", current_section, key)
                    };
                    self.entries
                        .insert(full_key.to_lowercase(), value.to_string());
                }
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.entries.get(&key.to_lowercase())
    }
}
