use glob::Pattern;
use std::fs;
use std::path::Path;

pub struct Ignore {
    patterns: Vec<Pattern>,
}

impl Ignore {
    pub fn load_for_repo(root: &Path) -> Self {
        let mut patterns = Vec::new();

        // 1. Load .git/info/exclude
        let exclude_path = root.join(".git/info/exclude");
        if let Ok(content) = fs::read_to_string(exclude_path) {
            patterns.extend(Self::parse_content(&content));
        }

        // 2. Load root .gitignore
        let gitignore_path = root.join(".gitignore");
        if let Ok(content) = fs::read_to_string(gitignore_path) {
            patterns.extend(Self::parse_content(&content));
        }

        Self { patterns }
    }

    fn parse_content(content: &str) -> Vec<Pattern> {
        content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| Pattern::new(l).ok())
            .collect()
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        // Simple glob matching for parity
        for pattern in &self.patterns {
            if pattern.matches(path) {
                return true;
            }
        }
        false
    }
}
