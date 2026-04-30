use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_repo(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("mygit-{name}-{nonce}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn mygit(dir: &PathBuf, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_mygit"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "mygit {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git(dir: &PathBuf, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_with_config(dir: &PathBuf, args: &[&str]) -> String {
    let mut configured = vec![
        "-c",
        "user.name=Yasir",
        "-c",
        "user.email=yasir@example.com",
    ];
    configured.extend_from_slice(args);
    git(dir, &configured)
}

fn commit_file(repo: &PathBuf, path: &str, content: &str, message: &str) -> String {
    fs::write(repo.join(path), content).unwrap();
    mygit(repo, &["add", path]);
    mygit(repo, &["commit", "-m", message]);
    mygit(repo, &["rev-parse", "HEAD"])
}

struct RepoPair {
    ours: PathBuf,
    expected: PathBuf,
}

impl RepoPair {
    fn new(name: &str) -> Self {
        let pair = Self {
            ours: temp_repo(&format!("{name}-ours")),
            expected: temp_repo(&format!("{name}-git")),
        };

        mygit(&pair.ours, &["init"]);
        git(&pair.expected, &["init", "--initial-branch=master"]);
        pair
    }

    fn write_file(&self, path: &str, content: &str) {
        if let Some(parent) = PathBuf::from(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(self.ours.join(parent)).unwrap();
                fs::create_dir_all(self.expected.join(parent)).unwrap();
            }
        }

        fs::write(self.ours.join(path), content).unwrap();
        fs::write(self.expected.join(path), content).unwrap();
    }

    fn mygit(&self, args: &[&str]) -> String {
        mygit(&self.ours, args)
    }

    fn git(&self, args: &[&str]) -> String {
        git(&self.expected, args)
    }

    fn git_with_config(&self, args: &[&str]) -> String {
        git_with_config(&self.expected, args)
    }
}

#[test]
fn init_creates_basic_repository_layout() {
    let dir = temp_repo("init");

    mygit(&dir, &["init"]);

    assert!(dir.join(".git/HEAD").is_file());
    assert!(dir.join(".git/objects").is_dir());
    assert!(dir.join(".git/refs/heads").is_dir());
}

#[test]
fn hash_object_matches_git_for_blob() {
    let dir = temp_repo("hash-object");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();

    let ours = mygit(&dir, &["hash-object", "hello.txt"]);
    let expected = git(&dir, &["hash-object", "hello.txt"]);

    assert_eq!(ours, expected);
}

#[test]
fn add_and_ls_files_records_staged_paths() {
    let dir = temp_repo("add-ls-files");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();

    mygit(&dir, &["add", "hello.txt"]);

    assert_eq!(mygit(&dir, &["ls-files"]), "hello.txt");
}

#[test]
fn write_tree_matches_git_for_staged_file() {
    let repos = RepoPair::new("write-tree");
    repos.write_file("hello.txt", "hello\n");

    repos.mygit(&["add", "hello.txt"]);
    repos.git(&["add", "hello.txt"]);

    assert_eq!(repos.mygit(&["write-tree"]), repos.git(&["write-tree"]));
}

#[test]
fn write_tree_matches_git_for_nested_staged_file() {
    let repos = RepoPair::new("nested-write-tree");
    repos.write_file("src/main.rs", "fn main() {}\n");

    repos.mygit(&["add", "src/main.rs"]);
    repos.git(&["add", "src/main.rs"]);

    assert_eq!(repos.mygit(&["write-tree"]), repos.git(&["write-tree"]));
}

#[test]
fn ls_files_stage_matches_git_for_staged_files() {
    let repos = RepoPair::new("ls-files-stage");
    repos.write_file("hello.txt", "hello\n");
    repos.write_file("src/main.rs", "fn main() {}\n");

    repos.mygit(&["add", "hello.txt", "src/main.rs"]);
    repos.git(&["add", "hello.txt", "src/main.rs"]);

    assert_eq!(
        repos.mygit(&["ls-files", "--stage"]),
        repos.git(&["ls-files", "--stage"])
    );
}

#[test]
fn cat_file_matches_git_for_written_blob() {
    let dir = temp_repo("cat-file");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();

    let sha = mygit(&dir, &["hash-object", "-w", "hello.txt"]);

    assert_eq!(
        mygit(&dir, &["cat-file", "-t", &sha]),
        git(&dir, &["cat-file", "-t", &sha])
    );
    assert_eq!(
        mygit(&dir, &["cat-file", "-s", &sha]),
        git(&dir, &["cat-file", "-s", &sha])
    );
    assert_eq!(
        mygit(&dir, &["cat-file", "-p", &sha]),
        git(&dir, &["cat-file", "-p", &sha])
    );
}

#[test]
fn git_can_read_blob_written_by_mygit() {
    let dir = temp_repo("git-read-mygit-blob");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();

    let sha = mygit(&dir, &["hash-object", "-w", "hello.txt"]);

    assert_eq!(git(&dir, &["cat-file", "-t", &sha]), "blob");
    assert_eq!(git(&dir, &["cat-file", "-p", &sha]), "hello");
}

#[test]
fn ls_tree_full_output_matches_git() {
    let repos = RepoPair::new("ls-tree-full");
    repos.write_file("hello.txt", "hello\n");
    repos.write_file("src/main.rs", "fn main() {}\n");

    repos.mygit(&["add", "hello.txt", "src/main.rs"]);
    repos.git(&["add", "hello.txt", "src/main.rs"]);
    let ours_tree = repos.mygit(&["write-tree"]);
    let expected_tree = repos.git(&["write-tree"]);

    assert_eq!(ours_tree, expected_tree);
    assert_eq!(
        repos.mygit(&["ls-tree", &ours_tree]),
        repos.git(&["ls-tree", &expected_tree])
    );
}

#[test]
fn ls_tree_name_only_lists_tree_entries() {
    let dir = temp_repo("ls-tree");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);

    let tree = mygit(&dir, &["write-tree"]);

    assert_eq!(mygit(&dir, &["ls-tree", "--name-only", &tree]), "hello.txt");
}

#[test]
fn commit_tree_writes_commit_object() {
    let dir = temp_repo("commit-tree");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    let tree = mygit(&dir, &["write-tree"]);

    let commit = mygit(&dir, &["commit-tree", &tree, "-m", "initial"]);

    assert_eq!(mygit(&dir, &["cat-file", "-t", &commit]), "commit");
}

#[test]
fn commit_updates_head_and_log_shows_commit() {
    let dir = temp_repo("commit-log");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);

    mygit(&dir, &["commit", "-m", "initial"]);
    let head = fs::read_to_string(dir.join(".git/refs/heads/master")).unwrap();
    let log = mygit(&dir, &["log"]);

    assert!(head.trim().len() == 40);
    assert!(log.contains("commit "));
    assert!(log.contains("initial"));
}

#[test]
fn commit_without_changes_does_not_advance_head() {
    let dir = temp_repo("commit-clean");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "initial"]);
    let head_before = fs::read_to_string(dir.join(".git/refs/heads/master")).unwrap();

    let output = mygit(&dir, &["commit", "-m", "again"]);
    let head_after = fs::read_to_string(dir.join(".git/refs/heads/master")).unwrap();

    assert_eq!(head_before, head_after);
    assert!(output.contains("nothing to commit"));
}

#[test]
fn status_is_clean_after_committing_nested_file() {
    let dir = temp_repo("nested-status-clean");
    mygit(&dir, &["init"]);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    mygit(&dir, &["add", "src/main.rs"]);
    mygit(&dir, &["commit", "-m", "nested"]);

    let status = mygit(&dir, &["status"]);

    assert!(status.contains("nothing to commit, working tree clean"));
}

#[test]
fn branch_create_and_list_marks_current_branch() {
    let dir = temp_repo("branch");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "initial"]);

    mygit(&dir, &["branch", "feature"]);
    let branches = mygit(&dir, &["branch"]);

    assert!(branches.contains("* master"));
    assert!(branches.lines().any(|line| line.trim() == "feature"));
}

#[test]
fn branch_list_matches_git_for_basic_branches() {
    let repos = RepoPair::new("branch-list");
    repos.write_file("hello.txt", "hello\n");

    repos.mygit(&["add", "hello.txt"]);
    repos.mygit(&["commit", "-m", "initial"]);
    repos.git(&["add", "hello.txt"]);
    repos.git_with_config(&["commit", "-m", "initial"]);

    repos.mygit(&["branch", "feature"]);
    repos.git(&["branch", "feature"]);

    assert_eq!(repos.mygit(&["branch"]), repos.git(&["branch"]));
}

#[test]
fn status_reports_modified_deleted_and_untracked_files() {
    let dir = temp_repo("status");
    mygit(&dir, &["init"]);
    fs::write(dir.join("modified.txt"), "before\n").unwrap();
    fs::write(dir.join("deleted.txt"), "gone\n").unwrap();
    mygit(&dir, &["add", "modified.txt", "deleted.txt"]);
    mygit(&dir, &["commit", "-m", "initial"]);

    fs::write(dir.join("modified.txt"), "after\n").unwrap();
    fs::remove_file(dir.join("deleted.txt")).unwrap();
    fs::write(dir.join("new.txt"), "new\n").unwrap();

    let status = mygit(&dir, &["status"]);

    assert!(status.contains("modified:   modified.txt"));
    assert!(status.contains("deleted:    deleted.txt"));
    assert!(status.contains("new.txt"));
}

#[test]
fn diff_reports_tracked_worktree_modification() {
    let dir = temp_repo("diff");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "before\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    fs::write(dir.join("hello.txt"), "after\n").unwrap();

    let diff = mygit(&dir, &["diff"]);

    assert!(diff.contains("diff --git a/hello.txt b/hello.txt"));
    assert!(diff.contains("-before"));
    assert!(diff.contains("+after"));
}

#[test]
fn checkout_branch_updates_worktree_and_removes_old_tracked_files() {
    let dir = temp_repo("checkout");
    mygit(&dir, &["init"]);
    fs::write(dir.join("master.txt"), "master\n").unwrap();
    mygit(&dir, &["add", "master.txt"]);
    mygit(&dir, &["commit", "-m", "master"]);
    mygit(&dir, &["branch", "feature"]);

    mygit(&dir, &["checkout", "feature"]);
    fs::remove_file(dir.join("master.txt")).unwrap();
    fs::write(dir.join("feature.txt"), "feature\n").unwrap();
    mygit(&dir, &["add", "master.txt", "feature.txt"]);
    mygit(&dir, &["commit", "-m", "feature"]);

    mygit(&dir, &["checkout", "master"]);

    assert!(dir.join("master.txt").exists());
    assert!(!dir.join("feature.txt").exists());
    assert_eq!(
        fs::read_to_string(dir.join(".git/HEAD")).unwrap(),
        "ref: refs/heads/master\n"
    );
}

#[test]
fn rm_removes_tracked_file_from_worktree_and_index() {
    let dir = temp_repo("rm");
    mygit(&dir, &["init"]);
    fs::write(dir.join("remove.txt"), "remove\n").unwrap();
    mygit(&dir, &["add", "remove.txt"]);

    mygit(&dir, &["rm", "remove.txt"]);

    assert!(!dir.join("remove.txt").exists());
    assert_eq!(mygit(&dir, &["ls-files"]), "");
}

#[test]
fn mv_renames_file_and_updates_index() {
    let dir = temp_repo("mv");
    mygit(&dir, &["init"]);
    fs::write(dir.join("old.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "old.txt"]);

    mygit(&dir, &["mv", "old.txt", "new.txt"]);

    assert!(!dir.join("old.txt").exists());
    assert!(dir.join("new.txt").exists());
    assert_eq!(mygit(&dir, &["ls-files"]), "new.txt");
}

#[test]
fn rev_parse_resolves_head_and_tags() {
    let dir = temp_repo("rev-parse");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "initial"]);
    let head = mygit(&dir, &["rev-parse", "HEAD"]);

    mygit(&dir, &["tag", "v1"]);

    assert_eq!(head.len(), 40);
    assert_eq!(mygit(&dir, &["rev-parse", "v1"]), head);
}

#[test]
fn tag_lists_created_tags() {
    let dir = temp_repo("tag");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "initial"]);

    mygit(&dir, &["tag", "v1"]);
    mygit(&dir, &["tag", "v2"]);

    assert_eq!(mygit(&dir, &["tag"]), "v1\nv2");
}

#[test]
fn config_sets_and_gets_repository_value() {
    let dir = temp_repo("config");
    mygit(&dir, &["init"]);

    mygit(&dir, &["config", "user.name", "Yasir"]);

    assert_eq!(mygit(&dir, &["config", "user.name"]), "Yasir");
}

#[test]
fn reflog_records_commit_updates() {
    let dir = temp_repo("reflog");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "initial"]);

    let reflog = mygit(&dir, &["reflog"]);

    assert!(reflog.contains("HEAD@{0}: update by mygit"));
}

#[test]
fn show_prints_blob_content() {
    let dir = temp_repo("show");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "hello\n").unwrap();
    let sha = mygit(&dir, &["hash-object", "-w", "hello.txt"]);

    assert_eq!(mygit(&dir, &["show", &sha]), "hello");
}

#[test]
fn reset_hard_moves_head_index_and_worktree() {
    let dir = temp_repo("reset-hard");
    mygit(&dir, &["init"]);
    fs::write(dir.join("hello.txt"), "first\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "first"]);
    let first = mygit(&dir, &["rev-parse", "HEAD"]);

    fs::write(dir.join("hello.txt"), "second\n").unwrap();
    mygit(&dir, &["add", "hello.txt"]);
    mygit(&dir, &["commit", "-m", "second"]);

    mygit(&dir, &["reset", "--hard", &first]);

    assert_eq!(mygit(&dir, &["rev-parse", "HEAD"]), first);
    assert_eq!(
        fs::read_to_string(dir.join("hello.txt")).unwrap(),
        "first\n"
    );
    assert!(mygit(&dir, &["status"]).contains("nothing to commit"));
}

#[test]
fn clone_local_repository_checks_out_branch_and_sets_origin() {
    let source = temp_repo("remote-source");
    mygit(&source, &["init"]);
    commit_file(&source, "hello.txt", "hello\n", "initial");

    let clone_dir = temp_repo("remote-clone");
    mygit(&clone_dir, &["clone", source.to_str().unwrap()]);

    assert!(clone_dir.join("hello.txt").exists());
    assert_eq!(
        mygit(&clone_dir, &["rev-parse", "HEAD"]),
        mygit(&source, &["rev-parse", "HEAD"])
    );
    assert!(
        fs::read_to_string(clone_dir.join(".git/config"))
            .unwrap()
            .contains(source.to_str().unwrap())
    );
}

#[test]
fn fetch_local_remote_updates_tracking_ref() {
    let source = temp_repo("remote-fetch-source");
    mygit(&source, &["init"]);
    commit_file(&source, "hello.txt", "one\n", "one");

    let clone_dir = temp_repo("remote-fetch-clone");
    mygit(&clone_dir, &["clone", source.to_str().unwrap()]);

    let second = commit_file(&source, "hello.txt", "two\n", "two");
    mygit(&clone_dir, &["fetch"]);

    assert_eq!(
        fs::read_to_string(clone_dir.join(".git/refs/remotes/origin/master"))
            .unwrap()
            .trim(),
        second
    );
}

#[test]
fn push_local_remote_updates_remote_branch() {
    let remote = temp_repo("remote-push-remote");
    mygit(&remote, &["init"]);

    let local = temp_repo("remote-push-local");
    mygit(&local, &["init"]);
    commit_file(&local, "hello.txt", "hello\n", "initial");
    mygit(&local, &["config", "remote.origin.url", remote.to_str().unwrap()]);

    mygit(&local, &["push"]);

    assert_eq!(
        fs::read_to_string(remote.join(".git/refs/heads/master"))
            .unwrap()
            .trim(),
        mygit(&local, &["rev-parse", "HEAD"])
    );
}

#[test]
fn pull_fast_forwards_local_branch_from_remote() {
    let source = temp_repo("remote-pull-source");
    mygit(&source, &["init"]);
    commit_file(&source, "hello.txt", "one\n", "one");

    let clone_dir = temp_repo("remote-pull-clone");
    mygit(&clone_dir, &["clone", source.to_str().unwrap()]);

    commit_file(&source, "hello.txt", "two\n", "two");
    mygit(&clone_dir, &["pull"]);

    assert_eq!(fs::read_to_string(clone_dir.join("hello.txt")).unwrap(), "two\n");
    assert_eq!(
        mygit(&clone_dir, &["rev-parse", "HEAD"]),
        mygit(&source, &["rev-parse", "HEAD"])
    );
}
