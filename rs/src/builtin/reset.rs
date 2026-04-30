use crate::core::index::Index;
use crate::core::object::{TreeEntry, read_object, read_tree};
use crate::core::refs::{get_current_branch, resolve_ref, update_ref};
use std::fs;
use std::path::Path;

pub fn cmd_reset(target: &str, hard: bool) {
    let sha = resolve_ref(target).unwrap_or_else(|| target.to_string());
    let (obj_type, data) = read_object(&sha);
    if obj_type != "commit" {
        eprintln!("fatal: Could not parse object '{}'.", target);
        std::process::exit(128);
    }

    let content = String::from_utf8_lossy(&data);
    let tree_sha = content.lines().next().unwrap()[5..].to_string();
    let mut entries = Vec::new();
    collect_entries_recursive(&tree_sha, "", &mut entries);

    let mut index = Index::load();
    if hard {
        for path in index.entries.keys() {
            if Path::new(path).exists() {
                fs::remove_file(path).unwrap();
            }
        }
    }
    index.entries.clear();

    for entry in entries {
        if hard {
            let (_, blob_data) = read_object(&entry.sha1);
            if let Some(parent) = Path::new(&entry.path).parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(&entry.path, blob_data).unwrap();
        }

        let metadata = fs::metadata(&entry.path).unwrap();
        index.add_entry(entry.path, entry.sha1, metadata);
    }
    index.write().unwrap();

    if let Some(branch) = get_current_branch() {
        update_ref(&format!("refs/heads/{}", branch), &sha).unwrap();
    } else {
        update_ref("HEAD", &sha).unwrap();
    }
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
            collect_entries_recursive(&entry.sha1, &full_path, entries);
        } else {
            let mut e = entry.clone();
            e.path = full_path;
            entries.push(e);
        }
    }
}
