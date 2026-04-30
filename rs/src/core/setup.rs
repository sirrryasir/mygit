use std::env;
use std::path::PathBuf;

pub struct RepoInfo {
    pub git_dir: PathBuf,
    pub work_tree: PathBuf,
    pub prefix: String,
}

pub fn setup_git_directory() -> RepoInfo {
    let cwd = env::current_dir().expect("Could not get current directory");

    // 1. Check environment variables
    let env_git_dir = env::var("GIT_DIR").ok().map(PathBuf::from);
    let env_work_tree = env::var("GIT_WORK_TREE").ok().map(PathBuf::from);

    if let (Some(git_dir), Some(work_tree)) = (env_git_dir.clone(), env_work_tree.clone()) {
        return RepoInfo {
            git_dir,
            work_tree,
            prefix: String::new(),
        };
    }

    // 2. Discover .git directory by walking up
    let mut current = cwd.as_path();
    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            let work_tree = current.to_path_buf();
            let prefix = cwd
                .strip_prefix(&work_tree)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            return RepoInfo {
                git_dir: env_git_dir.unwrap_or(git_dir),
                work_tree: env_work_tree.unwrap_or(work_tree),
                prefix,
            };
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            eprintln!("fatal: not a git repository (or any of the parent directories): .git");
            std::process::exit(128);
        }
    }
}
