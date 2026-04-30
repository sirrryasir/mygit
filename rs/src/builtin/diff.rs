use crate::core::index::Index;
use crate::core::object::read_object;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

pub fn cmd_diff() {
    let index = Index::load();

    for (path, entry) in &index.entries {
        if !Path::new(path).exists() {
            continue;
        }

        let (_, blob_data) = read_object(&hex::encode(entry.sha1));
        let worktree_data = fs::read(path).unwrap();

        let blob_str = String::from_utf8_lossy(&blob_data);
        let worktree_str = String::from_utf8_lossy(&worktree_data);

        if blob_str == worktree_str {
            continue;
        }

        println!("diff --git a/{} b/{}", path, path);
        println!(
            "index {}..{} 100644",
            &hex::encode(entry.sha1)[..7],
            "0000000"
        );
        println!("--- a/{}", path);
        println!("+++ b/{}", path);

        let diff = TextDiff::from_lines(&blob_str, &worktree_str);
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "\x1b[31m-",
                ChangeTag::Insert => "\x1b[32m+",
                ChangeTag::Equal => " ",
            };
            print!("{}{}\x1b[0m", sign, change);
        }
    }
}
