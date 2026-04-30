use crate::core::refs::{delete_ref, get_current_branch, list_branches, resolve_ref, update_ref};

pub fn cmd_branch(name: Option<&str>, delete: bool) {
    if let Some(branch_name) = name {
        if delete {
            if delete_ref(branch_name).is_err() {
                eprintln!("error: branch '{}' not found.", branch_name);
            } else {
                println!("Deleted branch {}.", branch_name);
            }
        } else {
            // Create branch
            let head_sha = resolve_ref("HEAD").expect("fatal: Not a valid object name HEAD");
            update_ref(branch_name, &head_sha).unwrap();
        }
    } else {
        // List branches
        let branches = list_branches();
        let current = get_current_branch();
        for branch in branches {
            if Some(&branch) == current.as_ref() {
                println!("* {}", branch);
            } else {
                println!("  {}", branch);
            }
        }
    }
}
