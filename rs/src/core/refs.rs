use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn resolve_ref(ref_name: &str) -> Option<String> {
    let path = Path::new(".git").join(ref_name);
    if !path.exists() {
        // Try shorthand
        let heads_path = Path::new(".git/refs/heads").join(ref_name);
        if heads_path.exists() {
            return resolve_ref_path(&heads_path);
        }
        let tags_path = Path::new(".git/refs/tags").join(ref_name);
        if tags_path.exists() {
            return resolve_ref_path(&tags_path);
        }
        return None;
    }
    resolve_ref_path(&path)
}

fn resolve_ref_path(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let content = content.trim();
    if content.starts_with("ref: ") {
        resolve_ref(&content[5..])
    } else {
        Some(content.to_string())
    }
}

pub fn update_ref(ref_name: &str, sha: &str) -> std::io::Result<()> {
    let path = if ref_name.starts_with("refs/") || ref_name == "HEAD" {
        Path::new(".git").join(ref_name)
    } else {
        Path::new(".git/refs/heads").join(ref_name)
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let old_sha = resolve_ref(ref_name).unwrap_or_else(|| "0".repeat(40));
    fs::write(path, format!("{}\n", sha))
        .and_then(|_| append_reflog(ref_name, &old_sha, sha, "update by mygit"))
}

pub fn get_current_branch() -> Option<String> {
    let head_content = fs::read_to_string(".git/HEAD").ok()?;
    if head_content.starts_with("ref: refs/heads/") {
        Some(head_content[16..].trim().to_string())
    } else {
        None // Detached HEAD
    }
}

pub fn list_branches() -> Vec<String> {
    let mut branches = Vec::new();
    let heads_dir = Path::new(".git/refs/heads");
    if let Ok(entries) = fs::read_dir(heads_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                branches.push(name.to_string());
            }
        }
    }
    branches.sort();
    branches
}

pub fn delete_ref(ref_name: &str) -> std::io::Result<()> {
    let path = Path::new(".git/refs/heads").join(ref_name);
    if !path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "branch not found",
        ));
    }
    fs::remove_file(path)
}

fn append_reflog(
    ref_name: &str,
    old_sha: &str,
    new_sha: &str,
    message: &str,
) -> std::io::Result<()> {
    let log_path = Path::new(".git/logs").join(ref_name);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let line = format!(
        "{} {} Yasir <yasir@example.com> {} +0000\t{}\n",
        old_sha, new_sha, timestamp, message
    );

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    file.write_all(line.as_bytes())?;

    if ref_name != "HEAD" {
        if let Some(current_branch) = get_current_branch() {
            if ref_name == format!("refs/heads/{}", current_branch) {
                append_reflog("HEAD", old_sha, new_sha, message)?;
            }
        }
    }

    Ok(())
}
