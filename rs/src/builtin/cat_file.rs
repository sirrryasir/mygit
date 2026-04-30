use crate::core::object::read_object;
use std::io::{self, Write};

pub fn cmd_cat_file(obj_type_or_pretty: &str, object: &str) {
    let (obj_type, data) = read_object(object);

    if obj_type_or_pretty == "-p" {
        io::stdout().write_all(&data).unwrap();
    } else if obj_type_or_pretty == "-t" {
        println!("{}", obj_type);
    } else if obj_type_or_pretty == "-s" {
        println!("{}", data.len());
    } else {
        // Assume obj_type_or_pretty is the type
        if obj_type == obj_type_or_pretty {
            io::stdout().write_all(&data).unwrap();
        } else {
            eprintln!("fatal: cat-file {}: bad tree object", object);
        }
    }
}
