use std::fs;

pub fn cmd_reflog() {
    let Ok(content) = fs::read_to_string(".git/logs/HEAD") else {
        return;
    };

    for (idx, line) in content.lines().rev().enumerate() {
        let mut parts = line.splitn(2, '\t');
        let meta = parts.next().unwrap_or("");
        let message = parts.next().unwrap_or("");
        let new_sha = meta.split_whitespace().nth(1).unwrap_or("");
        println!("{} HEAD@{{{}}}: {}", &new_sha[..7], idx, message);
    }
}
