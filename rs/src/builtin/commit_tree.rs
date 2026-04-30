use crate::core::object::write_object;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn cmd_commit_tree(tree_sha: &str, parent_sha: Option<&str>, message: &str) {
    println!("{}", commit_tree(tree_sha, parent_sha, message));
}

pub fn commit_tree(tree_sha: &str, parent_sha: Option<&str>, message: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let author = format!("Yasir <yasir@example.com> {} +0000", now);
    let mut body = format!("tree {}\n", tree_sha);
    if let Some(p) = parent_sha {
        if p.len() == 40 {
            body.push_str(&format!("parent {}\n", p));
        }
    }
    body.push_str(&format!("author {}\n", author));
    body.push_str(&format!("committer {}\n", author));
    body.push_str(&format!("\n{}\n", message));
    write_object("commit", body.as_bytes())
}
