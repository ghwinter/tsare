from pathlib import Path

s = Path(r"D:\GitHub\engine\rust\engine-core\src\syntax.rs")
t = s.read_text(encoding="utf-8")

t = t.replace(
    "    pub line: usize,\n    pub col: usize,\n    pub file: String,",
    "    pub line: usize,\n    pub col: usize,\n    pub end_line: usize,\n    pub end_col: usize,\n    pub file: String,",
)

t = t.replace(
    '        parser.expect("}")?;\n        out.push(Decl { kind, name, fields, field_pos, line, col, file: String::new(), ref_pos: parser.ref_pos.clone() });',
    '        let (end_line, end_col) = parser.position();\n        parser.expect("}")?;\n        out.push(Decl {\n            kind,\n            name,\n            fields,\n            field_pos,\n            line,\n            col,\n            end_line,\n            end_col: end_col + 1,\n            file: String::new(),\n            ref_pos: parser.ref_pos.clone(),\n        });',
)

s.write_text(t, encoding="utf-8")
print("declaration end positions recorded")
