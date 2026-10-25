from pathlib import Path

m = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = m.read_text(encoding="utf-8")

t = t.replace(
    "fn main() {\n    let args: Vec<String> = env::args().collect();\n    let json = args.iter().any(|arg| arg == \"--json\");",
    "fn engine_mode(args: &[String]) -> ! {\n"
    "    let path = args\n"
    "        .iter()\n"
    "        .skip(2)\n"
    "        .find(|arg| !arg.starts_with(\"--\"))\n"
    "        .cloned()\n"
    "        .unwrap_or_else(|| {\n"
    "            eprintln!(\"usage: engine-core engine FILE|DIR\");\n"
    "            std::process::exit(2);\n"
    "        });\n"
    "    let decls = collect(Path::new(&path));\n"
    "    let result = engine_core::closure::engine(&decls);\n"
    "    let quoted = |items: &[String]| {\n"
    "        items.iter().map(|item| format!(\"\\\"{}\\\"\", escape(item))).collect::<Vec<_>>().join(\", \")\n"
    "    };\n"
    "    println!(\"{{\");\n"
    "    println!(\"  \\\"held\\\": [{}],\", quoted(&result.held));\n"
    "    let conditional: Vec<String> = result\n"
    "        .conditional\n"
    "        .iter()\n"
    "        .map(|(name, _)| format!(\"\\\"{}\\\"\", escape(name)))\n"
    "        .collect();\n"
    "    println!(\"  \\\"conditional\\\": [{}],\", conditional.join(\", \"));\n"
    "    let blocked: Vec<String> = result\n"
    "        .blocked\n"
    "        .iter()\n"
    "        .map(|(warrant, missing)| {\n"
    "            format!(\"{{\\\"warrant\\\": \\\"{}\\\", \\\"missing\\\": [{}]}}\", escape(warrant), quoted(missing))\n"
    "        })\n"
    "        .collect();\n"
    "    println!(\"  \\\"blocked\\\": [{}],\", blocked.join(\", \"));\n"
    "    let chains: Vec<String> = result\n"
    "        .chains\n"
    "        .iter()\n"
    "        .map(|(name, chain)| format!(\"\\\"{}\\\": \\\"{}\\\"\", escape(name), escape(chain)))\n"
    "        .collect();\n"
    "    println!(\"  \\\"chains\\\": {{{}}},\", chains.join(\", \"));\n"
    "    let gates: Vec<String> = result\n"
    "        .gates\n"
    "        .iter()\n"
    "        .map(|(name, weak)| format!(\"\\\"{}\\\": [{}]\", escape(name), quoted(weak)))\n"
    "        .collect();\n"
    "    println!(\"  \\\"gates\\\": {{{}}}\", gates.join(\", \"));\n"
    "    println!(\"}}\");\n"
    "    std::process::exit(0);\n"
    "}\n\n"
    "fn main() {\n    let args: Vec<String> = env::args().collect();\n    if args.get(1).map(|arg| arg == \"engine\").unwrap_or(false) {\n        engine_mode(&args);\n    }\n    let json = args.iter().any(|arg| arg == \"--json\");",
)

m.write_text(t, encoding="utf-8")
print("engine command added")
