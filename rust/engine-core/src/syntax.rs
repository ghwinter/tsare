//! Lexer and parser for the declaration syntax, with positions.

#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Id(String),
    Str(String),
    Num(String),
    List(Vec<Val>),
    Call(String, Vec<Val>),
    Rec(Vec<(String, Val)>),
    Hole,
}

#[derive(Debug, Clone)]
pub struct Decl {
    pub kind: String,
    pub name: String,
    pub fields: Vec<(String, Val)>,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub line: usize,
    pub col: usize,
}

fn lex(src: &str) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut chars = src.chars().peekable();
    let mut line = 1usize;
    let mut col = 1usize;
    while let Some(&c) = chars.peek() {
        match c {
            '#' => {
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n == '\n' {
                        line += 1;
                        col = 1;
                        break;
                    }
                    col += 1;
                }
            }
            c if c.is_whitespace() => {
                chars.next();
                if c == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
            }
            '"' => {
                let start_line = line;
                let start_col = col;
                chars.next();
                col += 1;
                let mut text = String::new();
                while let Some(n) = chars.next() {
                    col += 1;
                    if n == '"' {
                        break;
                    }
                    if n == '\\' {
                        if let Some(next) = chars.next() {
                            col += 1;
                            text.push(next);
                            continue;
                        }
                    }
                    text.push(n);
                }
                out.push(Token { text: format!("\x00{}", text), line: start_line, col: start_col });
            }
            c if c.is_alphanumeric() || c == '_' || c == '.' => {
                let start_line = line;
                let start_col = col;
                let mut text = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' || n == '.' {
                        text.push(n);
                        chars.next();
                        col += 1;
                    } else {
                        break;
                    }
                }
                out.push(Token { text, line: start_line, col: start_col });
            }
            _ => {
                let start_line = line;
                let start_col = col;
                chars.next();
                col += 1;
                out.push(Token { text: c.to_string(), line: start_line, col: start_col });
            }
        }
    }
    out
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.toks.get(self.pos).map(|token| token.text.as_str())
    }

    fn position(&self) -> (usize, usize) {
        self.toks
            .get(self.pos)
            .map(|token| (token.line, token.col))
            .unwrap_or((0, 0))
    }

    fn bump(&mut self) -> Option<Token> {
        let token = self.toks.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expect(&mut self, want: &str) -> Result<(), String> {
        match self.bump() {
            Some(token) if token.text == want => Ok(()),
            Some(token) => Err(format!("line {}: expected {want}, got {:?}", token.line, token.text)),
            None => Err(format!("expected {want}, reached the end")),
        }
    }

    fn value(&mut self) -> Result<Val, String> {
        match self.peek() {
            Some("{") => {
                self.bump();
                let mut fields = Vec::new();
                while self.peek() != Some("}") {
                    let key = self.bump().ok_or("missing key")?.text;
                    self.expect(":")?;
                    fields.push((key, self.value()?));
                    if self.peek() == Some(",") {
                        self.bump();
                    }
                }
                self.expect("}")?;
                Ok(Val::Rec(fields))
            }
            Some("[") => {
                self.bump();
                let mut items = Vec::new();
                while self.peek() != Some("]") {
                    items.push(self.value()?);
                    if self.peek() == Some(",") {
                        self.bump();
                    }
                }
                self.expect("]")?;
                Ok(Val::List(items))
            }
            Some("?") => {
                self.bump();
                Ok(Val::Hole)
            }
            Some(text) if text.starts_with('\x00') => {
                let token = self.bump().unwrap();
                Ok(Val::Str(token.text[1..].to_string()))
            }
            Some(text) if text.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                Ok(Val::Num(self.bump().unwrap().text))
            }
            Some(_) => {
                let name = self.bump().ok_or("missing value")?.text;
                if self.peek() == Some("(") {
                    self.bump();
                    let mut args = Vec::new();
                    while self.peek() != Some(")") {
                        args.push(self.value()?);
                        if self.peek() == Some(",") {
                            self.bump();
                        }
                    }
                    self.expect(")")?;
                    Ok(Val::Call(name, args))
                } else {
                    Ok(Val::Id(name))
                }
            }
            None => Err("unexpected end of input".into()),
        }
    }
}

pub fn parse(src: &str) -> Result<Vec<Decl>, String> {
    let mut parser = Parser { toks: lex(src), pos: 0 };
    let mut out = Vec::new();
    while let Some(kind) = parser.peek().map(|text| text.to_string()) {
        if kind == "}" || kind == "]" {
            parser.bump();
            continue;
        }
        let (line, col) = parser.position();
        parser.bump();
        let name = parser.bump().ok_or("declaration without a name")?.text;
        parser.expect("{")?;
        let mut fields = Vec::new();
        while parser.peek() != Some("}") {
            let key = parser.bump().ok_or("field without a name")?.text;
            parser.expect(":")?;
            fields.push((key, parser.value()?));
            if parser.peek() == Some(",") {
                parser.bump();
            }
        }
        parser.expect("}")?;
        out.push(Decl { kind, name, fields, line, col });
    }
    Ok(out)
}
