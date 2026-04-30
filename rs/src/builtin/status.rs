use crate::core::diff::diff_tree_to_index;
use crate::core::ignore::Ignore;
use crate::core::index::Index;
use crate::core::object::{hash_object_internal, read_object};
use crate::core::refs::{get_current_branch, resolve_ref};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn cmd_status() {
    let branch = get_current_branch().unwrap_or_else(|| "HEAD (detached)".to_string());
    println!("On branch {}", branch);

    let index = Index::load();
    let ignore = Ignore::load_for_repo(Path::new("."));

    // 1. Changes to be committed (HEAD vs Index)
    let head_sha = resolve_ref("HEAD");
    let staged = if let Some(sha) = head_sha {
        let (obj_type, data) = read_object(&sha);
        let tree_sha = if obj_type == "commit" {
            let content = String::from_utf8_lossy(&data);
            content.lines().next().unwrap()[5..].to_string()
        } else {
            sha
        };
        diff_tree_to_index(&tree_sha, &index)
    } else {
        // Initial commit case
        index
            .entries
            .keys()
            .map(|path| crate::core::diff::DiffEntry {
                path: path.clone(),
                status: 'A',
            })
            .collect()
    };

    if !staged.is_empty() {
        println!("\nChanges to be committed:");
        println!("  (use \"git rm --cached <file>...\" to unstage)");
        for entry in &staged {
            let status_str = match entry.status {
                'A' => "new file:  ",
                'M' => "modified:  ",
                'D' => "deleted:   ",
                _ => "unknown:   ",
            };
            println!("\t\x1b[32m{}{}\x1b[0m", status_str, entry.path);
        }
    }

    // 2. Changes not staged for commit (Index vs Worktree)
    let mut untracked = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for entry in WalkDir::new(".")
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().map(|s| s == ".git").unwrap_or(false))
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path().strip_prefix("./").unwrap_or(entry.path());
        let path_str = path.to_string_lossy().to_string();

        if ignore.is_ignored(&path_str) {
            continue;
        }

        if let Some(index_entry) = index.entries.get(&path_str) {
            let metadata = fs::metadata(path).unwrap();
            if !index_entry.matches_stat(&metadata) {
                let worktree_sha = hash_object_internal(path, false);
                if worktree_sha != hex::encode(index_entry.sha1) {
                    modified.push(path_str);
                }
            }
        } else {
            untracked.push(path_str);
        }
    }

    for path in index.entries.keys() {
        if !Path::new(path).exists() {
            deleted.push(path.clone());
        }
    }

    if !modified.is_empty() {
        println!("\nChanges not staged for commit:");
        println!("  (use \"git add <file>...\" to update what will be committed)");
        for path in &modified {
            println!("\t\x1b[31mmodified:   {}\x1b[0m", path);
        }
    }

    if !deleted.is_empty() {
        if modified.is_empty() {
            println!("\nChanges not staged for commit:");
            println!("  (use \"git add/rm <file>...\" to update what will be committed)");
        }
        for path in &deleted {
            println!("\t\x1b[31mdeleted:    {}\x1b[0m", path);
        }
    }

    if !untracked.is_empty() {
        println!("\nUntracked files:");
        println!("  (use \"git add <file>...\" to include in what will be committed)");
        for path in &untracked {
            println!("\t\x1b[31m{}\x1b[0m", path);
        }
    }

    if staged.is_empty() && modified.is_empty() && deleted.is_empty() && untracked.is_empty() {
        println!("nothing to commit, working tree clean");
    }
}
