from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

t = t.replace(
    "pub struct Diagnostic {\n    pub code: &'static str,\n    pub message: String,\n    pub line: usize,\n    pub col: usize,\n}",
    "pub struct Diagnostic {\n    pub code: &'static str,\n    pub message: String,\n    pub line: usize,\n    pub col: usize,\n    pub file: String,\n}",
)

t = t.replace(
    "Diagnostic { code, message, line, col }",
    "Diagnostic { code, message, line, col, file: decl.file.clone() }",
)

# literal constructions in check_record, cycles, collapse, tension and layer
t = t.replace(
    'code: "E-TYPE",\n                        message: format!("{}.{key}: expected {ftype}", decl.name),\n                        line,\n                        col,\n                    });',
    'code: "E-TYPE",\n                        message: format!("{}.{key}: expected {ftype}", decl.name),\n                        line,\n                        col,\n                        file: decl.file.clone(),\n                    });',
)
t = t.replace(
    'code: "E-FIELD",\n                message: format!("{} has no field {key}", decl.kind),\n                line,\n                col,\n            }),',
    'code: "E-FIELD",\n                message: format!("{} has no field {key}", decl.kind),\n                line,\n                col,\n                file: decl.file.clone(),\n            }),',
)
t = t.replace(
    "out.push(Diagnostic { code, message, line, col, file: decl.file.clone() });",
    "out.push(Diagnostic { code, message, line, col, file: decl.file.clone() });",
)

p.write_text(t, encoding="utf-8")

m = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = m.read_text(encoding="utf-8")
t = t.replace(
    "    let mut origin: std::collections::HashMap<usize, String> = std::collections::HashMap::new();\n"
    "    for decl in &decls {\n"
    "        origin.entry(decl.line).or_insert_with(|| decl.file.clone());\n"
    "    }\n",
    "",
)
t = t.replace(
    '        let file = origin.get(&item.line).cloned().unwrap_or_else(|| path.clone());\n'
    '        println!("  {} {}:{} {}", item.code, file, item.line, item.message);',
    '        let file = if item.file.is_empty() { path.clone() } else { item.file.clone() };\n'
    '        println!("  {} {}:{} {}", item.code, file, item.line, item.message);',
)
t = t.replace(
    '    sorted.sort_by_key(|item| {\n        let file = origin.get(&item.line).cloned().unwrap_or_default();\n        (file, item.line, item.col, item.code)\n    });',
    '    sorted.sort_by_key(|item| (item.file.clone(), item.line, item.col, item.code));',
)
m.write_text(t, encoding="utf-8")
print("diagnostics carry the file")
