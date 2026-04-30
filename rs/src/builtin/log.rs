use crate::core::refs::resolve_ref;
use crate::core::revision::RevWalker;

pub fn cmd_log() {
    let head_sha = resolve_ref("HEAD").expect("fatal: Not a valid object name HEAD");
    let walker = RevWalker::new(head_sha);

    for commit in walker {
        println!("commit {}", commit.sha);
        println!("Author: {}", commit.author);
        println!("\n    {}\n", commit.message);
    }
}
