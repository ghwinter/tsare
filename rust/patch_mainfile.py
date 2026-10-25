from pathlib import Path

m = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = m.read_text(encoding="utf-8")

t = t.replace(
    "    let decls = collect(Path::new(&path));\n    let diagnostics = check::check(&decls);",
    "    let decls = collect(Path::new(&path));\n"
    "    let mut origin: std::collections::HashMap<usize, String> = std::collections::HashMap::new();\n"
    "    for decl in &decls {\n"
    "        origin.entry(decl.line).or_insert_with(|| decl.file.clone());\n"
    "    }\n"
    "    let diagnostics = check::check(&decls);",
)

t = t.replace(
    '    let mut sorted = diagnostics.clone();\n    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));',
    '    let mut sorted = diagnostics.clone();\n'
    '    sorted.sort_by_key(|item| {\n'
    '        let file = origin.get(&item.line).cloned().unwrap_or_default();\n'
    '        (file, item.line, item.col, item.code)\n'
    '    });',
)

t = t.replace(
    '        if item.file.is_empty() {\n'
    '            println!("  {} {}:{} {}", item.code, path, item.line, item.message);\n'
    '        } else {\n'
    '            println!("  {} {}:{} {}", item.code, item.file, item.line, item.message);\n'
    '        }',
    '        let file = origin.get(&item.line).cloned().unwrap_or_else(|| path.clone());\n'
    '        println!("  {} {}:{} {}", item.code, file, item.line, item.message);',
)

m.write_text(t, encoding="utf-8")
print("main uses a line to file map")
