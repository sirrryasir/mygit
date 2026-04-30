use crate::builtin::checkout::cmd_checkout;
use crate::core::refs::{get_current_branch, resolve_ref, update_ref};
use crate::utils::helpers::copy_dir_recursive;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn cmd_fetch(remote: Option<&str>) {
    let remote_name = remote.unwrap_or("origin");
    let remote_git = resolve_remote_git(remote_name);

    copy_dir_recursive(&remote_git.join("objects"), Path::new(".git/objects"));

    let remote_heads = remote_git.join("refs/heads");
    let local_remote_refs = Path::new(".git/refs/remotes").join(remote_name);
    copy_dir_recursive(&remote_heads, &local_remote_refs);
    copy_dir_recursive(&remote_git.join("refs/tags"), Path::new(".git/refs/tags"));

    println!("Fetched from {}", remote_name);
}

pub fn cmd_pull(remote: Option<&str>, branch: Option<&str>) {
    let remote_name = remote.unwrap_or("origin");
    cmd_fetch(Some(remote_name));

    let branch_name = branch
        .map(|s| s.to_string())
        .or_else(get_current_branch)
        .unwrap_or_else(|| "master".to_string());

    let remote_ref = Path::new(".git/refs/remotes")
        .join(remote_name)
        .join(&branch_name);
    let sha = fs::read_to_string(&remote_ref)
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            eprintln!("fatal: couldn't find remote ref {}", branch_name);
            std::process::exit(128);
        });

    update_ref(&format!("refs/heads/{}", branch_name), &sha).unwrap();
    cmd_checkout(&branch_name);
}

pub fn cmd_push(remote: Option<&str>, branch: Option<&str>) {
    let remote_name = remote.unwrap_or("origin");
    let branch_name = branch
        .map(|s| s.to_string())
        .or_else(get_current_branch)
        .unwrap_or_else(|| "master".to_string());

    let sha = resolve_ref(&format!("refs/heads/{}", branch_name))
        .or_else(|| resolve_ref("HEAD"))
        .unwrap_or_else(|| {
            eprintln!("fatal: The current branch {} has no commits yet", branch_name);
            std::process::exit(128);
        });

    let remote_git = resolve_remote_git(remote_name);
    copy_dir_recursive(Path::new(".git/objects"), &remote_git.join("objects"));

    let ref_path = remote_git.join("refs/heads").join(&branch_name);
    if let Some(parent) = ref_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(ref_path, format!("{}\n", sha)).unwrap();

    println!("Pushed {} to {}", branch_name, remote_name);
}

pub fn set_origin_config(remote_path: &Path) {
    let config_path = Path::new(".git/config");
    let mut config = fs::read_to_string(config_path).unwrap_or_default();
    if !config.contains("[remote \"origin\"]") {
        if !config.ends_with('\n') {
            config.push('\n');
        }
        config.push_str(&format!(
            "[remote \"origin\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
            remote_path.display()
        ));
        fs::write(config_path, config).unwrap();
    }
}

fn resolve_remote_git(remote: &str) -> PathBuf {
    let path = Path::new(remote);
    if path.join(".git").is_dir() {
        return path.join(".git");
    }
    if path.is_dir() && path.join("objects").is_dir() && path.join("refs").is_dir() {
        return path.to_path_buf();
    }

    remote_url(remote)
        .map(PathBuf::from)
        .map(|p| {
            if p.join(".git").is_dir() {
                p.join(".git")
            } else {
                p
            }
        })
        .unwrap_or_else(|| {
            eprintln!("fatal: remote '{}' not found", remote);
            std::process::exit(128);
        })
}

fn remote_url(remote_name: &str) -> Option<String> {
    let content = fs::read_to_string(".git/config").ok()?;
    let wanted = format!("remote \"{}\"", remote_name);
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = &trimmed[1..trimmed.len() - 1] == wanted;
            continue;
        }

        if in_section {
            if let Some((key, value)) = trimmed.split_once('=') {
                if key.trim() == "url" {
                    return Some(value.trim().to_string());
                }
            }
        }
    }

    None
}
