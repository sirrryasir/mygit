use crate::core::object::read_object;
use crate::core::refs::resolve_ref;
use std::io::{self, Write};

pub fn cmd_show(rev: &str) {
    let sha = resolve_ref(rev).unwrap_or_else(|| rev.to_string());
    let (obj_type, data) = read_object(&sha);

    match obj_type.as_str() {
        "blob" => io::stdout().write_all(&data).unwrap(),
        "commit" | "tag" => {
            println!("{} {}", obj_type, sha);
            println!("{}", String::from_utf8_lossy(&data));
        }
        "tree" => {
            for entry in crate::core::object::read_tree(&sha) {
                println!("{} {}\t{}", entry.mode, entry.sha1, entry.path);
            }
        }
        _ => io::stdout().write_all(&data).unwrap(),
    }
}
