use crate::core::refs::resolve_ref;

pub fn cmd_rev_parse(rev: &str) {
    if rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit()) {
        println!("{}", rev);
        return;
    }

    match resolve_ref(rev) {
        Some(sha) => println!("{}", sha),
        None => {
            eprintln!("fatal: ambiguous argument '{}': unknown revision", rev);
            std::process::exit(128);
        }
    }
}
