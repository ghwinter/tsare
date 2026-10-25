from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\syntax.rs")
t = p.read_text(encoding="utf-8")

t = t.replace(
    "    pub fields: Vec<(String, Val)>,\n    pub line: usize,",
    "    pub fields: Vec<(String, Val)>,\n    pub field_pos: Vec<(usize, usize)>,\n    pub line: usize,",
)

t = t.replace(
    "        let mut fields = Vec::new();\n        while parser.peek() != Some(\"}\") {\n            let key = parser.bump().ok_or(\"field without a name\")?.text;",
    "        let mut fields = Vec::new();\n        let mut field_pos = Vec::new();\n        while parser.peek() != Some(\"}\") {\n            let (field_line, field_col) = parser.position();\n            let key = parser.bump().ok_or(\"field without a name\")?.text;",
)

t = t.replace(
    "            fields.push((key, parser.value()?));",
    "            fields.push((key, parser.value()?));\n            field_pos.push((field_line, field_col));",
)

t = t.replace(
    "        out.push(Decl { kind, name, fields, line, col });",
    "        out.push(Decl { kind, name, fields, field_pos, line, col });",
)

p.write_text(t, encoding="utf-8")
print("field positions added")
