use std::fs;
use std::path::Path;

pub fn cmd_init(directory: Option<&str>) {
    let root = directory.unwrap_or(".");
    let git_dir = Path::new(root).join(".git");

    if git_dir.exists() {
        println!(
            "Reinitialized existing Git repository in {}",
            git_dir.display()
        );
    } else {
        fs::create_dir_all(&git_dir).expect("Could not create .git directory");
        fs::create_dir_all(git_dir.join("objects")).expect("Could not create objects directory");
        fs::create_dir_all(git_dir.join("refs/heads"))
            .expect("Could not create refs/heads directory");
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/master\n").expect("Could not create HEAD");

        // Initialize basic config
        let config_content = "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n";
        fs::write(git_dir.join("config"), config_content).expect("Could not create config");

        println!("Initialized empty Git repository in {}", git_dir.display());
    }
}
