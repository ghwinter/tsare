from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = p.read_text(encoding="utf-8")
t = t.replace(
    'const SKIP: &[&str] = &["tool", "doc", "code", "user", "model", "observation"];',
    'const SKIP: &[&str] = &["tool", "doc", "code", "user", "model", "observation", "experiment"];',
)
t = t.replace(
    "    let decls = parse(&src);",
    "    let parsed = std::panic::catch_unwind(|| parse(&src));\n"
    "    let decls = match parsed {\n"
    "        Ok(decls) => decls,\n"
    "        Err(_) => {\n"
    '            println!("parse error");\n'
    "            std::process::exit(2);\n"
    "        }\n"
    "    };",
)
p.write_text(t, encoding="utf-8")
print("patched", "catch_unwind" in t)
