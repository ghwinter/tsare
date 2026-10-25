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

fn escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn engine_mode(args: &[String]) -> ! {
    let path = args
        .iter()
        .skip(2)
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("usage: engine-core engine FILE|DIR");
            std::process::exit(2);
        });
    let decls = collect(Path::new(&path));
    let result = engine_core::closure::engine(&decls);
    let quoted = |items: &[String]| {
        items.iter().map(|item| format!("\"{}\"", escape(item))).collect::<Vec<_>>().join(", ")
    };
    println!("{{");
    println!("  \"held\": [{}],", quoted(&result.held));
    let conditional: Vec<String> = result
        .conditional
        .iter()
        .map(|(name, _)| format!("\"{}\"", escape(name)))
        .collect();
    println!("  \"conditional\": [{}],", conditional.join(", "));
    let blocked: Vec<String> = result
        .blocked
        .iter()
        .map(|(warrant, missing)| {
            format!("{{\"warrant\": \"{}\", \"missing\": [{}]}}", escape(warrant), quoted(missing))
        })
        .collect();
    println!("  \"blocked\": [{}],", blocked.join(", "));
    let chains: Vec<String> = result
        .chains
        .iter()
        .map(|(name, chain)| format!("\"{}\": \"{}\"", escape(name), escape(chain)))
        .collect();
    println!("  \"chains\": {{{}}},", chains.join(", "));
    let gates: Vec<String> = result
        .gates
        .iter()
        .map(|(name, weak)| format!("\"{}\": [{}]", escape(name), quoted(weak)))
        .collect();
    println!("  \"gates\": {{{}}}", gates.join(", "));
    println!("}}");
    std::process::exit(0);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(|arg| arg == "engine").unwrap_or(false) {
        engine_mode(&args);
    }
    if args.get(1).map(|arg| arg == "apply").unwrap_or(false) {
        let dry = args.iter().any(|arg| arg == "--dry-run");
        let path = args.iter().skip(2).find(|arg| !arg.starts_with("--")).cloned().unwrap_or_default();
        let text = std::fs::read_to_string(&path).expect("read file");
        let decls = syntax::parse(&text).expect("parse");
        let items = engine_core::fix::suggestions(&decls);
        let (updated, applied) = engine_core::apply::apply_best(&text, &items);
        if !dry {
            std::fs::write(&path, &updated).expect("write file");
        }
        let list: Vec<String> = applied.iter().map(|item| format!("\"{}\"", escape(item))).collect();
        println!("{{\"applied\": [{}]}}", list.join(", "));
        std::process::exit(0);
    }
    if args.get(1).map(|arg| arg == "suggest").unwrap_or(false) {
        let path = args.iter().skip(2).find(|arg| !arg.starts_with("--")).cloned().unwrap_or_default();
        let decls = collect(Path::new(&path));
        let items = engine_core::fix::suggestions(&decls);
        let rendered: Vec<String> = items
            .iter()
            .map(|item| {
                format!(
                    "{{\"applicability\": \"{}\", \"message\": \"{}\", \"file\": \"{}\", \"span\": {{\"line\": {}, \"col\": {}, \"end_line\": {}, \"end_col\": {}}}, \"replacement\": \"{}\"}}",
                    item.applicability,
                    escape(&item.message),
                    escape(&item.file),
                    item.line,
                    item.col,
                    item.end_line,
                    item.end_col,
                    escape(&item.replacement)
                )
            })
            .collect();
        println!("{{\"suggestions\": [{}]}}", rendered.join(", "));
        std::process::exit(0);
    }
    let json = args.iter().any(|arg| arg == "--json");
    let skip = if args.get(1).map(|arg| arg == "check").unwrap_or(false) { 2 } else { 1 };
    let path = match args.iter().skip(skip).find(|arg| !arg.starts_with("--")) {
        Some(path) => path.clone(),
        None => {
            eprintln!("usage: engine-core FILE|DIR");
            std::process::exit(2);
        }
    };
    let decls = collect(Path::new(&path));
    let diagnostics = check::check(&decls);
    let mut sorted = diagnostics.clone();
    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));
    if json {
        let errors = diagnostics.iter().filter(|item| item.code.starts_with("E-")).count();
        let warnings = diagnostics.len() - errors;
        println!("{{");
        println!("  \"diagnostics\": [");
        for (index, item) in sorted.iter().enumerate() {
            let file = if item.file.is_empty() { path.clone() } else { item.file.clone() };
            let severity = if item.code.starts_with("E-") { "error" } else { "warning" };
            let comma = if index + 1 == sorted.len() { "" } else { "," };
            println!(
                "    {{\"code\": \"{}\", \"severity\": \"{}\", \"file\": \"{}\", \"line\": {}, \"col\": {}, \"message\": \"{}\"}}{}",
                item.code, severity, escape(&file), item.line, item.col, escape(&item.message), comma
            );
        }
        println!("  ],");
        println!("  \"errors\": {},", errors);
        println!("  \"warnings\": {}", warnings);
        println!("}}");
        return;
    }
    println!("declarations: {}", decls.len());
    println!("diagnostics: {}", diagnostics.len());
    for item in &sorted {
        let file = if item.file.is_empty() { path.clone() } else { item.file.clone() };
        println!("  {} {}:{} {}", item.code, file, item.line, item.message);
    }
}
