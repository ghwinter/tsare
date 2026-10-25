from pathlib import Path

syntax = Path(r"D:\GitHub\engine\rust\engine-core\src\syntax.rs")
t = syntax.read_text(encoding="utf-8")

t = t.replace(
    "#[derive(Debug, Clone)]\npub struct Decl {\n    pub kind: String,\n    pub name: String,\n    pub fields: Vec<(String, Val)>,\n}",
    "#[derive(Debug, Clone)]\npub struct Decl {\n    pub kind: String,\n    pub name: String,\n    pub fields: Vec<(String, Val)>,\n    pub line: usize,\n    pub col: usize,\n}",
)

t = t.replace(
    "fn lex(src: &str) -> Vec<String> {\n    let mut out = Vec::new();\n    let mut chars = src.chars().peekable();",
    "#[derive(Debug, Clone)]\npub struct Token {\n    pub text: String,\n    pub line: usize,\n    pub col: usize,\n}\n\nfn lex(src: &str) -> Vec<Token> {\n    let mut out: Vec<Token> = Vec::new();\n    let mut chars = src.chars().peekable();\n    let mut line = 1usize;\n    let mut col = 1usize;",
)

# every token push becomes a Token with the position at the time of reading
t = t.replace('                out.push(format!("\\x00{}", s));', '                out.push(Token { text: format!("\\x00{}", s), line, col });')
t = t.replace('                out.push(s);', '                out.push(Token { text: s, line, col });')
t = t.replace('                out.push(c.to_string());', '                out.push(Token { text: c.to_string(), line, col });')

# track newlines: replace the whitespace arm to update line/col
t = t.replace(
    "            c if c.is_whitespace() => {\n                chars.next();\n            }",
    "            c if c.is_whitespace() => {\n                chars.next();\n                if c == '\\n' {\n                    line += 1;\n                    col = 1;\n                } else {\n                    col += 1;\n                }\n            }",
)

syntax.write_text(t, encoding="utf-8")
print("lexer positions added")
