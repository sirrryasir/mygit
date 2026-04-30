use crate::core::object::read_object;

pub fn cmd_ls_tree(sha: &str, name_only: bool) {
    let (_, data) = read_object(sha);
    let mut pos = 0;
    while pos < data.len() {
        let space_pos = data[pos..].iter().position(|&b| b == b' ').unwrap();
        let mode = String::from_utf8_lossy(&data[pos..pos + space_pos]);
        pos += space_pos + 1;
        let null_pos = data[pos..].iter().position(|&b| b == 0).unwrap();
        let name = String::from_utf8_lossy(&data[pos..pos + null_pos]);
        pos += null_pos + 1;
        let sha_bytes = &data[pos..pos + 20];
        let sha_hex = hex::encode(sha_bytes);
        pos += 20;

        if name_only {
            println!("{}", name);
        } else {
            let (entry_type, _) = read_object(&sha_hex);
            let display_mode = if mode == "40000" { "040000" } else { &mode };
            println!("{} {} {}\t{}", display_mode, entry_type, sha_hex, name);
        }
    }
}
