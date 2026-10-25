use std::env;
use std::fs;

use engine_core::check;
use engine_core::syntax;

fn main() {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: engine-core FILE");
            std::process::exit(2);
        }
    };
    let source = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(2);
        }
    };
    let decls = match syntax::parse(&source) {
        Ok(decls) => decls,
        Err(error) => {
            println!("parse error: {error}");
            std::process::exit(2);
        }
    };
    let diagnostics = check::check(&decls);
    println!("declarations: {}", decls.len());
    println!("diagnostics: {}", diagnostics.len());
    for item in &diagnostics {
        println!("  {} {}", item.code, item.message);
    }
}
