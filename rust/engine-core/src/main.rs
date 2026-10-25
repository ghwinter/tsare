use std::env;
use std::fs;
use std::path::Path;

use engine_core::check;
use engine_core::syntax;

fn collect(path: &Path) -> Vec<syntax::Decl> {
    let mut decls = Vec::new();
    if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)
            .expect("read directory")
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .filter(|item| item.extension().map(|ext| ext == "mf").unwrap_or(false))
            .collect();
        entries.sort();
        for entry in entries {
            let source = fs::read_to_string(&entry).expect("read file");
            let mut parsed = syntax::parse(&source).unwrap_or_else(|error| {
                println!("parse error in {}: {error}", entry.display());
                std::process::exit(2);
            });
            let name = entry.file_name().unwrap_or_default().to_string_lossy().to_string();
            for decl in &mut parsed {
                decl.file = name.clone();
            }
            decls.extend(parsed);
        }
    } else {
        let source = fs::read_to_string(path).expect("read file");
        decls = syntax::parse(&source).unwrap_or_else(|error| {
            println!("parse error: {error}");
            std::process::exit(2);
        });
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        for decl in &mut decls {
            decl.file = name.clone();
        }
    }
    decls
}

fn main() {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: engine-core FILE|DIR");
            std::process::exit(2);
        }
    };
    let decls = collect(Path::new(&path));
    let diagnostics = check::check(&decls);
    println!("declarations: {}", decls.len());
    println!("diagnostics: {}", diagnostics.len());
    let mut sorted = diagnostics.clone();
    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));
    for item in &sorted {
        let file = if item.file.is_empty() { path.clone() } else { item.file.clone() };
        println!("  {} {}:{} {}", item.code, file, item.line, item.message);
    }
}
