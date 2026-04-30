use crate::core::index::{Index, IndexEntry};
use crate::core::object::write_object;
use std::collections::BTreeMap;

pub fn cmd_write_tree() {
    let index = Index::load();
    println!("{}", write_index_tree(&index));
}

#[derive(Default)]
struct TreeNode {
    files: Vec<IndexEntry>,
    dirs: BTreeMap<String, TreeNode>,
}

pub fn write_index_tree(index: &Index) -> String {
    let mut root = TreeNode::default();

    for entry in index.entries.values() {
        insert_entry(&mut root, &entry.path, entry.clone());
    }

    write_tree_node(&root)
}

fn insert_entry(node: &mut TreeNode, path: &str, entry: IndexEntry) {
    if let Some((dir, rest)) = path.split_once('/') {
        insert_entry(node.dirs.entry(dir.to_string()).or_default(), rest, entry);
    } else {
        node.files.push(entry);
    }
}

fn write_tree_node(node: &TreeNode) -> String {
    let mut entries = Vec::new();

    for (name, child) in &node.dirs {
        let sha = write_tree_node(child);
        let mut entry = Vec::new();
        entry.extend_from_slice(format!("40000 {}\0", name).as_bytes());
        entry.extend_from_slice(&hex::decode(sha).unwrap());
        entries.push((name.clone(), entry));
    }

    for entry in &node.files {
        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
        let mut tree_entry = Vec::new();
        tree_entry.extend_from_slice(format!("{:o} {}\0", entry.mode, name).as_bytes());
        tree_entry.extend_from_slice(&entry.sha1);
        entries.push((name.to_string(), tree_entry));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut body = Vec::new();
    for (_, entry) in entries {
        body.extend_from_slice(&entry);
    }

    write_object("tree", &body)
}
