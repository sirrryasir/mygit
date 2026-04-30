pub struct DiffEntry {
    pub path: String,
    pub status: char, // 'M', 'A', 'D'
}

pub fn diff_index_to_worktree(_index: &crate::core::index::Index) -> Vec<DiffEntry> {
    let diffs = Vec::new();
    // Logic already partly in status, but we should centralize it
    diffs
}

pub fn diff_tree_to_index(tree_sha: &str, index: &crate::core::index::Index) -> Vec<DiffEntry> {
    let mut diffs = Vec::new();

    let mut tree_map = std::collections::HashMap::new();
    collect_tree_entries(tree_sha, "", &mut tree_map);

    // 1. Detect Added (in Index but not in Tree) and Modified
    for (path, index_entry) in &index.entries {
        let index_sha = hex::encode(index_entry.sha1);
        if let Some(tree_sha) = tree_map.get(path) {
            if index_sha != *tree_sha {
                diffs.push(DiffEntry {
                    path: path.clone(),
                    status: 'M',
                });
            }
        } else {
            diffs.push(DiffEntry {
                path: path.clone(),
                status: 'A',
            });
        }
    }

    // 2. Detect Deleted (in Tree but not in Index)
    for path in tree_map.keys() {
        if !index.entries.contains_key(path) {
            diffs.push(DiffEntry {
                path: path.clone(),
                status: 'D',
            });
        }
    }

    diffs
}

fn collect_tree_entries(
    tree_sha: &str,
    prefix: &str,
    entries: &mut std::collections::HashMap<String, String>,
) {
    for entry in crate::core::object::read_tree(tree_sha) {
        let path = if prefix.is_empty() {
            entry.path.clone()
        } else {
            format!("{}/{}", prefix, entry.path)
        };

        if entry.mode == "40000" {
            collect_tree_entries(&entry.sha1, &path, entries);
        } else {
            entries.insert(path, entry.sha1);
        }
    }
}
