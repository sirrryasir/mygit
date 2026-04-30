use crate::core::refs::{resolve_ref, update_ref};
use std::fs;
use std::path::Path;

pub fn cmd_tag(name: Option<&str>, target: Option<&str>) {
    if let Some(name) = name {
        let sha = target
            .and_then(resolve_ref)
            .or_else(|| target.map(|s| s.to_string()))
            .or_else(|| resolve_ref("HEAD"))
            .expect("fatal: Failed to resolve HEAD as a valid ref.");
        update_ref(&format!("refs/tags/{}", name), &sha).unwrap();
        return;
    }

    let tags_dir = Path::new(".git/refs/tags");
    let mut tags = Vec::new();
    if let Ok(entries) = fs::read_dir(tags_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                tags.push(name.to_string());
            }
        }
    }
    tags.sort();
    for tag in tags {
        println!("{}", tag);
    }
}
