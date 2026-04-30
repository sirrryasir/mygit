use crate::core::object::hash_object_internal;
use crate::core::object::read_object;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn resolve_ref(ref_name: &str) -> Option<String> {
    if ref_name == "HEAD" {
        let head_content = fs::read_to_string(".git/HEAD").ok()?;
        if head_content.starts_with("ref: ") {
            let full_ref_path = format!(".git/{}", head_content.trim_start_matches("ref: ").trim());
            return fs::read_to_string(full_ref_path)
                .ok()
                .map(|s| s.trim().to_string());
        }
        return Some(head_content.trim().to_string());
    }
    let head_path = format!(".git/refs/heads/{}", ref_name);
    if Path::new(&head_path).exists() {
        return fs::read_to_string(head_path)
            .ok()
            .map(|s| s.trim().to_string());
    }
    if ref_name.len() == 40 {
        return Some(ref_name.to_string());
    }
    None
}

pub fn update_ref(ref_path: &str, sha: &str) {
    let path = format!(".git/{}", ref_path);
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, format!("{}\n", sha)).unwrap();
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) {
    if !src.exists() {
        return;
    }
    fs::create_dir_all(dst).unwrap();
    if let Ok(read_dir) = fs::read_dir(src) {
        for entry in read_dir {
            let entry = entry.unwrap();
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if path.is_dir() {
                copy_dir_recursive(&path, &dest_path);
            } else {
                fs::copy(&path, &dest_path).unwrap();
            }
        }
    }
}

pub fn collect_tree_entries(tree_sha: &str, prefix: &str, entries: &mut HashMap<String, String>) {
    let (_, data) = read_object(tree_sha);
    let mut pos = 0;
    while pos < data.len() {
        let space_pos = data[pos..].iter().position(|&b| b == b' ').unwrap();
        let mode = String::from_utf8_lossy(&data[pos..pos + space_pos]);
        pos += space_pos + 1;
        let null_pos = data[pos..].iter().position(|&b| b == 0).unwrap();
        let name = String::from_utf8_lossy(&data[pos..pos + null_pos]);
        pos += null_pos + 1;
        let sha = hex::encode(&data[pos..pos + 20]);
        pos += 20;

        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", prefix, name)
        };
        if mode == "40000" {
            collect_tree_entries(&sha, &path, entries);
        } else {
            entries.insert(path, sha);
        }
    }
}

pub fn collect_work_entries(dir: &Path, prefix: &str, entries: &mut HashMap<String, String>) {
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            if name == ".git" || name == "target" {
                continue;
            }
            let path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };
            if entry.path().is_dir() {
                collect_work_entries(&entry.path(), &path, entries);
            } else {
                entries.insert(path, hash_object_internal(&entry.path(), false));
            }
        }
    }
}
