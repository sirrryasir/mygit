use crate::core::index::Index;

pub fn cmd_ls_files(stage: bool) {
    let index = Index::load();
    for entry in index.entries.values() {
        if stage {
            println!(
                "{:o} {} 0\t{}",
                entry.mode,
                hex::encode(entry.sha1),
                entry.path
            );
        } else {
            println!("{}", entry.path);
        }
    }
}
