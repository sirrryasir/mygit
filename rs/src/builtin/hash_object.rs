use crate::core::object::hash_object_internal;
use std::path::Path;

pub fn cmd_hash_object(file: &str, write: bool) {
    println!("{}", hash_object_internal(Path::new(file), write));
}
