use crate::core::index::Index;
use crate::core::lockfile::LockFile;
use crate::core::object::hash_object_internal;
use std::io::Write;
use std::path::Path;

pub fn cmd_add(files: Vec<String>) {
    if files.is_empty() {
        println!("Nothing specified, nothing added.");
        return;
    }

    let index_path = Path::new(".git/index");
    let mut lock = LockFile::new(index_path);

    if let Err(_) = lock.hold_for_update() {
        eprintln!("fatal: unable to create '.git/index.lock': File exists.");
        return;
    }

    let mut index = Index::load();

    for file in files {
        let path = Path::new(&file);
        if !path.exists() {
            if index.entries.remove(&file).is_some() {
                continue;
            } else {
                eprintln!("fatal: pathspec '{}' did not match any files", file);
                lock.rollback().unwrap();
                return;
            }
        }

        let sha = hash_object_internal(path, true);
        let metadata = std::fs::metadata(path).unwrap();
        index.add_entry(file, sha, metadata);
    }

    // Write new index to lockfile
    let index_data = index.serialize();
    if let Err(e) = lock.write_all(&index_data) {
        eprintln!("error: failed to write index: {}", e);
        lock.rollback().unwrap();
        return;
    }

    // Commit the lockfile (rename to .git/index)
    if let Err(e) = lock.commit() {
        eprintln!("error: failed to commit index: {}", e);
    }
}
