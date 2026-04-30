mod builtin;
mod core;
mod utils;

use crate::core::setup::setup_git_directory;
use clap::{Arg, Command};
use std::env;
use std::path::Path;

fn main() {
    let matches = Command::new("mygit")
        .version("0.1.0")
        .author("Yasir")
        .about("A Git implementation in Rust")
        .arg(
            Arg::new("chdir")
                .short('C')
                .value_name("path")
                .help("Run as if git was started in <path>"),
        )
        .arg(
            Arg::new("git_dir")
                .long("git-dir")
                .value_name("path")
                .help("Set the path to the repository"),
        )
        .subcommand(
            Command::new("init")
                .about("Initialize a new Git repository")
                .arg(Arg::new("directory").index(1)),
        )
        .subcommand(
            Command::new("add")
                .about("Add file contents to the index")
                .arg(Arg::new("files").index(1).num_args(1..).required(true)),
        )
        .subcommand(
            Command::new("rm")
                .about("Remove files from the working tree and from the index")
                .arg(
                    Arg::new("cached")
                        .long("cached")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(Arg::new("files").index(1).num_args(1..).required(true)),
        )
        .subcommand(
            Command::new("mv")
                .about("Move or rename a file")
                .arg(Arg::new("source").index(1).required(true))
                .arg(Arg::new("destination").index(2).required(true)),
        )
        .subcommand(
            Command::new("commit")
                .about("Record changes to the repository")
                .arg(
                    Arg::new("message")
                        .short('m')
                        .long("message")
                        .value_name("msg")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("cat-file")
                .about("Provide content or type and size information for repository objects")
                .arg(
                    Arg::new("type")
                        .index(1)
                        .required(true)
                        .allow_hyphen_values(true),
                )
                .arg(Arg::new("object").index(2).required(true)),
        )
        .subcommand(
            Command::new("ls-files")
                .about("Show information about files in the index and the working tree")
                .arg(
                    Arg::new("stage")
                        .short('s')
                        .long("stage")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("ls-tree")
                .about("List the contents of a tree object")
                .arg(
                    Arg::new("name-only")
                        .long("name-only")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(Arg::new("tree").index(1).required(true)),
        )
        .subcommand(
            Command::new("write-tree").about("Create a tree object from the current working tree"),
        )
        .subcommand(
            Command::new("commit-tree")
                .about("Create a commit object from a tree object")
                .arg(Arg::new("tree").index(1).required(true))
                .arg(Arg::new("parent").short('p').value_name("parent"))
                .arg(
                    Arg::new("message")
                        .short('m')
                        .value_name("message")
                        .required(true),
                ),
        )
        .subcommand(Command::new("log").about("Show commit logs"))
        .subcommand(Command::new("reflog").about("Manage reflog information"))
        .subcommand(
            Command::new("show")
                .about("Show an object")
                .arg(Arg::new("object").index(1).required(true)),
        )
        .subcommand(
            Command::new("rev-parse")
                .about("Pick out and massage parameters")
                .arg(Arg::new("rev").index(1).required(true)),
        )
        .subcommand(
            Command::new("tag")
                .about("Create, list, delete or verify a tag object")
                .arg(Arg::new("name").index(1))
                .arg(Arg::new("object").index(2)),
        )
        .subcommand(
            Command::new("config")
                .about("Get and set repository options")
                .arg(Arg::new("key").index(1).required(true))
                .arg(Arg::new("value").index(2)),
        )
        .subcommand(
            Command::new("reset")
                .about("Reset current HEAD to the specified state")
                .arg(
                    Arg::new("hard")
                        .long("hard")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(Arg::new("commit").index(1).required(true)),
        )
        .subcommand(Command::new("status").about("Show the working tree status"))
        .subcommand(
            Command::new("diff")
                .about("Show changes between commits, commit and working tree, etc"),
        )
        .subcommand(
            Command::new("branch")
                .about("List, create, or delete branches")
                .arg(Arg::new("name").index(1))
                .arg(
                    Arg::new("delete")
                        .short('d')
                        .long("delete")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("checkout")
                .about("Switch branches or restore working tree files")
                .arg(Arg::new("commit").index(1).required(true)),
        )
        .subcommand(
            Command::new("clone")
                .about("Clone a repository into a new directory")
                .arg(Arg::new("url").index(1).required(true))
                .arg(Arg::new("directory").index(2)),
        )
        .subcommand(
            Command::new("fetch")
                .about("Download objects and refs from another repository")
                .arg(Arg::new("remote").index(1)),
        )
        .subcommand(
            Command::new("pull")
                .about("Fetch from and integrate with another repository")
                .arg(Arg::new("remote").index(1))
                .arg(Arg::new("branch").index(2)),
        )
        .subcommand(
            Command::new("push")
                .about("Update remote refs along with associated objects")
                .arg(Arg::new("remote").index(1))
                .arg(Arg::new("branch").index(2)),
        )
        .subcommand(
            Command::new("hash-object")
                .about("Compute object ID and optionally creates a blob from a file")
                .arg(Arg::new("file").index(1).required(true))
                .arg(
                    Arg::new("write")
                        .short('w')
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .get_matches();

    // Handle global -C option
    if let Some(path) = matches.get_one::<String>("chdir") {
        if let Err(e) = env::set_current_dir(Path::new(path)) {
            eprintln!("fatal: cannot change to '{}': {}", path, e);
            std::process::exit(128);
        }
    }

    // Handle global --git-dir option
    if let Some(git_dir) = matches.get_one::<String>("git_dir") {
        unsafe {
            env::set_var("GIT_DIR", git_dir);
        }
    }

    match matches.subcommand() {
        Some(("init", sub_m)) => {
            let dir = sub_m.get_one::<String>("directory").map(|s| s.as_str());
            builtin::init::cmd_init(dir);
        }
        Some(("clone", sub_m)) => {
            let url = sub_m.get_one::<String>("url").unwrap();
            let dir = sub_m.get_one::<String>("directory").map(|s| s.as_str());
            builtin::clone::cmd_clone(url, dir);
        }
        _ => {
            // Require a repository for all other commands
            setup_git_directory();

            match matches.subcommand() {
                Some(("add", sub_m)) => {
                    let files: Vec<String> = sub_m
                        .get_many::<String>("files")
                        .unwrap()
                        .map(|s| s.to_string())
                        .collect();
                    builtin::add::cmd_add(files);
                }
                Some(("rm", sub_m)) => {
                    let files: Vec<String> = sub_m
                        .get_many::<String>("files")
                        .unwrap()
                        .map(|s| s.to_string())
                        .collect();
                    let cached = sub_m.get_flag("cached");
                    builtin::rm::cmd_rm(files, cached);
                }
                Some(("mv", sub_m)) => {
                    let source = sub_m.get_one::<String>("source").unwrap();
                    let destination = sub_m.get_one::<String>("destination").unwrap();
                    builtin::mv::cmd_mv(source, destination);
                }
                Some(("commit", sub_m)) => {
                    let message = sub_m.get_one::<String>("message").unwrap();
                    builtin::commit::cmd_commit(message);
                }
                Some(("cat-file", sub_m)) => {
                    let obj_type = sub_m.get_one::<String>("type").unwrap();
                    let object = sub_m.get_one::<String>("object").unwrap();
                    builtin::cat_file::cmd_cat_file(obj_type, object);
                }
                Some(("ls-files", sub_m)) => {
                    let stage = sub_m.get_flag("stage");
                    builtin::ls_files::cmd_ls_files(stage);
                }
                Some(("ls-tree", sub_m)) => {
                    let tree = sub_m.get_one::<String>("tree").unwrap();
                    let name_only = sub_m.get_flag("name-only");
                    builtin::ls_tree::cmd_ls_tree(tree, name_only);
                }
                Some(("write-tree", _)) => {
                    builtin::write_tree::cmd_write_tree();
                }
                Some(("commit-tree", sub_m)) => {
                    let tree = sub_m.get_one::<String>("tree").unwrap();
                    let parent = sub_m.get_one::<String>("parent").map(|s| s.as_str());
                    let message = sub_m.get_one::<String>("message").unwrap();
                    builtin::commit_tree::cmd_commit_tree(tree, parent, message);
                }
                Some(("log", _)) => {
                    builtin::log::cmd_log();
                }
                Some(("reflog", _)) => {
                    builtin::reflog::cmd_reflog();
                }
                Some(("show", sub_m)) => {
                    let object = sub_m.get_one::<String>("object").unwrap();
                    builtin::show::cmd_show(object);
                }
                Some(("rev-parse", sub_m)) => {
                    let rev = sub_m.get_one::<String>("rev").unwrap();
                    builtin::rev_parse::cmd_rev_parse(rev);
                }
                Some(("tag", sub_m)) => {
                    let name = sub_m.get_one::<String>("name").map(|s| s.as_str());
                    let object = sub_m.get_one::<String>("object").map(|s| s.as_str());
                    builtin::tag::cmd_tag(name, object);
                }
                Some(("config", sub_m)) => {
                    let key = sub_m.get_one::<String>("key").unwrap();
                    let value = sub_m.get_one::<String>("value").map(|s| s.as_str());
                    builtin::config::cmd_config(key, value);
                }
                Some(("reset", sub_m)) => {
                    let commit = sub_m.get_one::<String>("commit").unwrap();
                    let hard = sub_m.get_flag("hard");
                    builtin::reset::cmd_reset(commit, hard);
                }
                Some(("status", _)) => {
                    builtin::status::cmd_status();
                }
                Some(("diff", _)) => {
                    builtin::diff::cmd_diff();
                }
                Some(("branch", sub_m)) => {
                    let name = sub_m.get_one::<String>("name").map(|s| s.as_str());
                    let delete = sub_m.get_flag("delete");
                    builtin::branch::cmd_branch(name, delete);
                }
                Some(("checkout", sub_m)) => {
                    let commit = sub_m.get_one::<String>("commit").unwrap();
                    builtin::checkout::cmd_checkout(commit);
                }
                Some(("fetch", sub_m)) => {
                    let remote = sub_m.get_one::<String>("remote").map(|s| s.as_str());
                    builtin::remote::cmd_fetch(remote);
                }
                Some(("pull", sub_m)) => {
                    let remote = sub_m.get_one::<String>("remote").map(|s| s.as_str());
                    let branch = sub_m.get_one::<String>("branch").map(|s| s.as_str());
                    builtin::remote::cmd_pull(remote, branch);
                }
                Some(("push", sub_m)) => {
                    let remote = sub_m.get_one::<String>("remote").map(|s| s.as_str());
                    let branch = sub_m.get_one::<String>("branch").map(|s| s.as_str());
                    builtin::remote::cmd_push(remote, branch);
                }
                Some(("hash-object", sub_m)) => {
                    let file = sub_m.get_one::<String>("file").unwrap();
                    let write = sub_m.get_flag("write");
                    builtin::hash_object::cmd_hash_object(file, write);
                }
                _ => {}
            }
        }
    }
}
