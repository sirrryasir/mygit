use crate::builtin::checkout::cmd_checkout;
use crate::builtin::init::cmd_init;
use crate::builtin::remote::set_origin_config;
use crate::core::refs::resolve_ref;
use crate::utils::helpers::copy_dir_recursive;
use std::fs;
use std::path::Path;

pub fn cmd_clone(url: &str, dir: Option<&str>) {
    let dest_dir = dir.unwrap_or_else(|| url.split('/').last().unwrap().trim_end_matches(".git"));
    if Path::new(dest_dir).exists() {
        panic!("Directory {} already exists", dest_dir);
    }
    fs::create_dir_all(dest_dir).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dest_dir).unwrap();

    // Fix: Pass None to cmd_init as we already changed directory
    cmd_init(None);

    let src_path = original_dir.join(url);
    if src_path.exists() {
        let src_git = src_path.join(".git");
        if src_git.exists() {
            copy_dir_recursive(&src_git.join("objects"), &Path::new(".git/objects"));
            copy_dir_recursive(&src_git.join("refs"), &Path::new(".git/refs"));
            fs::copy(src_git.join("HEAD"), ".git/HEAD").unwrap();
            set_origin_config(&src_path);

            if let Some(head) = fs::read_to_string(src_git.join("HEAD"))
                .ok()
                .map(|s| s.trim().to_string())
            {
                if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
                    cmd_checkout(branch.trim());
                } else if let Some(sha) = resolve_ref("HEAD") {
                    cmd_checkout(&sha);
                }
            }
            println!("Cloned {} into {}", url, dest_dir);
            std::env::set_current_dir(original_dir).unwrap();
            return;
        }
    }

    std::env::set_current_dir(original_dir).unwrap();
    println!("Remote cloning not yet implemented for Rust version.");
}
