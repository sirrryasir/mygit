use crate::core::index::Index;
use crate::core::object::hash_object_internal;
use std::fs;
use std::path::Path;

pub fn cmd_mv(source: &str, destination: &str) {
    fs::rename(source, destination).unwrap();

    let mut index = Index::load();
    index.entries.remove(source);

    let sha = hash_object_internal(Path::new(destination), true);
    let metadata = fs::metadata(destination).unwrap();
    index.add_entry(destination.to_string(), sha, metadata);
    index.write().unwrap();
}
