from pathlib import Path

m = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = m.read_text(encoding="utf-8")

t = t.replace(
    "fn main() {\n    let path = match env::args().nth(1) {",
    'fn escape(text: &str) -> String {\n'
    '    let mut out = String::new();\n'
    "    for ch in text.chars() {\n"
    '        match ch {\n'
    '            \'"\' => out.push_str("\\\\\\""),\n'
    '            \'\\\\\' => out.push_str("\\\\\\\\"),\n'
    "            '\\n' => out.push_str(\"\\\\n\"),\n"
    "            '\\t' => out.push_str(\"\\\\t\"),\n"
    '            other => out.push(other),\n'
    "        }\n"
    "    }\n"
    "    out\n"
    "}\n\n"
    "fn main() {\n    let args: Vec<String> = env::args().collect();\n    let json = args.iter().any(|arg| arg == \"--json\");\n    let path = match args.iter().skip(1).find(|arg| !arg.starts_with(\"--\")) {",
)

t = t.replace(
    "        Some(path) => path,\n        None => {",
    "        Some(path) => path.clone(),\n        None => {",
)

t = t.replace(
    '    println!("declarations: {}", decls.len());\n    println!("diagnostics: {}", diagnostics.len());',
    '    if json {\n'
    '        let errors = diagnostics.iter().filter(|item| item.code.starts_with("E-")).count();\n'
    '        let warnings = diagnostics.len() - errors;\n'
    '        println!("{{");\n'
    '        println!("  \\"diagnostics\\": [");\n'
    '        for (index, item) in sorted.iter().enumerate() {\n'
    '            let file = if item.file.is_empty() { path.clone() } else { item.file.clone() };\n'
    '            let severity = if item.code.starts_with("E-") { "error" } else { "warning" };\n'
    '            let comma = if index + 1 == sorted.len() { "" } else { "," };\n'
    '            println!(\n'
    '                "    {{\\"code\\": \\"{}\\", \\"severity\\": \\"{}\\", \\"file\\": \\"{}\\", \\"line\\": {}, \\"col\\": {}, \\"message\\": \\"{}\\"}}{}",\n'
    '                item.code, severity, escape(&file), item.line, item.col, escape(&item.message), comma\n'
    '            );\n'
    '        }\n'
    '        println!("  ],");\n'
    '        println!("  \\"errors\\": {},", errors);\n'
    '        println!("  \\"warnings\\": {}", warnings);\n'
    '        println!("}}");\n'
    '        return;\n'
    '    }\n'
    '    println!("declarations: {}", decls.len());\n    println!("diagnostics: {}", diagnostics.len());',
)

m.write_text(t, encoding="utf-8")
print("json output added")
