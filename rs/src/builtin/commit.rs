use crate::builtin::write_tree::write_index_tree;
use crate::core::index::Index;
use crate::core::object::{read_object, write_object};
use crate::core::refs::{get_current_branch, resolve_ref, update_ref};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn cmd_commit(message: &str) {
    let index = Index::load();

    // 1. Create Tree Object from the index.
    let tree_sha = write_index_tree(&index);
    let parent_sha = resolve_ref("HEAD");

    if let Some(parent) = &parent_sha {
        if commit_tree_sha(parent).as_deref() == Some(tree_sha.as_str()) {
            println!("nothing to commit, working tree clean");
            return;
        }
    } else if index.entries.is_empty() {
        println!("nothing to commit, working tree clean");
        return;
    }

    // 2. Create Commit Object
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut commit_content = format!("tree {}\n", tree_sha);
    if let Some(parent) = parent_sha {
        commit_content.push_str(&format!("parent {}\n", parent));
    }

    commit_content.push_str(&format!(
        "author Yasir <yasir@example.com> {} +0000\n",
        timestamp
    ));
    commit_content.push_str(&format!(
        "committer Yasir <yasir@example.com> {} +0000\n",
        timestamp
    ));
    commit_content.push_str(&format!("\n{}\n", message));

    let commit_sha = write_object("commit", commit_content.as_bytes());

    // 3. Update Current Branch or HEAD
    if let Some(branch) = get_current_branch() {
        update_ref(&format!("refs/heads/{}", branch), &commit_sha).unwrap();
        println!("[{} {}] {}", branch, &commit_sha[..7], message);
    } else {
        update_ref("HEAD", &commit_sha).unwrap();
        println!("[detached HEAD {}] {}", &commit_sha[..7], message);
    }
}

fn commit_tree_sha(commit_sha: &str) -> Option<String> {
    let (obj_type, data) = read_object(commit_sha);
    if obj_type != "commit" {
        return None;
    }

    let content = String::from_utf8_lossy(&data);
    content
        .lines()
        .find_map(|line| line.strip_prefix("tree ").map(|sha| sha.to_string()))
}
