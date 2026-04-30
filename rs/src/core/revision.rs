use crate::core::object::read_object;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct Commit {
    pub sha: String,
    pub parents: Vec<String>,
    pub author: String,
    pub message: String,
    pub tree: String,
}

pub struct RevWalker {
    pending: VecDeque<String>,
}

impl RevWalker {
    pub fn new(start_sha: String) -> Self {
        let mut pending = VecDeque::new();
        pending.push_back(start_sha);
        Self { pending }
    }
}

impl Iterator for RevWalker {
    type Item = Commit;

    fn next(&mut self) -> Option<Self::Item> {
        let sha = self.pending.pop_front()?;
        let (obj_type, data) = read_object(&sha);
        if obj_type != "commit" {
            return None;
        }

        let content = String::from_utf8_lossy(&data);
        let mut parents = Vec::new();
        let mut author = String::new();
        let mut message = String::new();
        let mut tree = String::new();

        let mut lines = content.lines();
        while let Some(line) = lines.next() {
            if line.is_empty() {
                message = lines.collect::<Vec<_>>().join("\n");
                break;
            }
            if line.starts_with("tree ") {
                tree = line[5..].to_string();
            } else if line.starts_with("parent ") {
                let parent_sha = line[7..].to_string();
                parents.push(parent_sha.clone());
                self.pending.push_back(parent_sha);
            } else if line.starts_with("author ") {
                author = line[7..].to_string();
            }
        }

        Some(Commit {
            sha,
            parents,
            author,
            message,
            tree,
        })
    }
}
