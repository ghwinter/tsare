from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\syntax.rs")
t = p.read_text(encoding="utf-8")

t = t.replace(
    "    pub file: String,\n}",
    "    pub file: String,\n    pub ref_pos: Vec<(String, usize, usize)>,\n}",
)

t = t.replace(
    "struct Parser {\n    toks: Vec<Token>,\n    pos: usize,\n}",
    "struct Parser {\n    toks: Vec<Token>,\n    pos: usize,\n    ref_pos: Vec<(String, usize, usize)>,\n}",
)

t = t.replace(
    "                } else {\n                    Ok(Val::Id(name))\n                }",
    "                } else {\n                    let (line, col) = position_of(&self.toks, self.pos - 1);\n                    self.ref_pos.push((name.clone(), line, col));\n                    Ok(Val::Id(name))\n                }",
)

t = t.replace(
    "struct Parser {",
    "fn position_of(tokens: &[Token], index: usize) -> (usize, usize) {\n    tokens\n        .get(index)\n        .map(|token| (token.line, token.col))\n        .unwrap_or((0, 0))\n}\n\nstruct Parser {",
)

t = t.replace(
    "    let mut parser = Parser { toks: lex(src), pos: 0 };",
    "    let mut parser = Parser { toks: lex(src), pos: 0, ref_pos: Vec::new() };",
)

t = t.replace(
    "        let (line, col) = parser.position();\n        parser.bump();",
    "        let (line, col) = parser.position();\n        parser.ref_pos.clear();\n        parser.bump();",
)

t = t.replace(
    'out.push(Decl { kind, name, fields, field_pos, line, col, file: String::new() });',
    'out.push(Decl { kind, name, fields, field_pos, line, col, file: String::new(), ref_pos: parser.ref_pos.clone() });',
)

p.write_text(t, encoding="utf-8")
print("reference positions recorded")
