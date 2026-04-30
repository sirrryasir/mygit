use crate::core::index::Index;
use std::fs;
use std::path::Path;

pub fn cmd_rm(files: Vec<String>, cached: bool) {
    let mut index = Index::load();

    for file in files {
        if index.entries.remove(&file).is_none() {
            eprintln!("fatal: pathspec '{}' did not match any files", file);
            continue;
        }

        if !cached && Path::new(&file).exists() {
            fs::remove_file(&file).unwrap();
        }
    }

    index.write().unwrap();
}
