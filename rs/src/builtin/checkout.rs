use crate::core::index::Index;
use crate::core::object::{TreeEntry, read_object, read_tree};
use crate::core::refs::resolve_ref;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn cmd_checkout(target: &str) {
    // 1. Resolve target to a commit
    let sha = resolve_ref(target).unwrap_or_else(|| target.to_string());
    let (obj_type, data) = read_object(&sha);

    let tree_sha = if obj_type == "commit" {
        let content = String::from_utf8_lossy(&data);
        content.lines().next().unwrap()[5..].to_string()
    } else if obj_type == "tree" {
        sha.clone()
    } else {
        panic!("fatal: Not a valid commit or tree: {}", target);
    };

    // 2. Load the tree recursively
    let mut entries = Vec::new();
    collect_entries_recursive(&tree_sha, "", &mut entries);

    // 3. Update working tree and index
    let mut index = Index::load();
    let previous_paths: Vec<String> = index.entries.keys().cloned().collect();
    let target_paths: std::collections::HashSet<String> =
        entries.iter().map(|entry| entry.path.clone()).collect();

    index.entries.clear();

    for path in previous_paths {
        if !target_paths.contains(&path) && Path::new(&path).exists() {
            fs::remove_file(&path).unwrap();
        }
    }

    for entry in entries {
        let (_, blob_data) = read_object(&entry.sha1);
        if let Some(parent) = Path::new(&entry.path).parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&entry.path, blob_data).unwrap();

        let metadata = fs::metadata(&entry.path).unwrap();
        index.add_entry(entry.path, entry.sha1, metadata);
    }

    // 4. Update index file
    let mut lock = crate::core::lockfile::LockFile::new(Path::new(".git/index"));
    lock.hold_for_update().unwrap();
    lock.file
        .as_mut()
        .unwrap()
        .write_all(&index.serialize())
        .unwrap();
    lock.commit().unwrap();

    // 5. Update HEAD
    if list_branches_internal().contains(&target.to_string()) {
        fs::write(".git/HEAD", format!("ref: refs/heads/{}\n", target)).unwrap();
    } else {
        fs::write(".git/HEAD", format!("{}\n", sha)).unwrap();
    }

    println!("Switched to branch '{}'", target);
}

fn collect_entries_recursive(tree_sha: &str, prefix: &str, entries: &mut Vec<TreeEntry>) {
    let tree = read_tree(tree_sha);
    for entry in tree {
        let full_path = if prefix.is_empty() {
            entry.path.clone()
        } else {
            format!("{}/{}", prefix, entry.path)
        };

        if entry.mode == "40000" {
            // Directory
            collect_entries_recursive(&entry.sha1, &full_path, entries);
        } else {
            let mut e = entry.clone();
            e.path = full_path;
            entries.push(e);
        }
    }
}

fn list_branches_internal() -> Vec<String> {
    let mut branches = Vec::new();
    if let Ok(entries) = fs::read_dir(".git/refs/heads") {
        for entry in entries.flatten() {
            branches.push(entry.file_name().to_str().unwrap().to_string());
        }
    }
    branches
}
