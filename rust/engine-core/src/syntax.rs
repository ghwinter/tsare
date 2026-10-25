//! Lexer and parser for the declaration syntax.

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
}

fn lex(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '#' => {
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {
                chars.next();
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(n) = chars.next() {
                    if n == '"' {
                        break;
                    }
                    s.push(n);
                }
                out.push(format!("\x00{}", s));
            }
            c if c.is_alphanumeric() || c == '_' || c == '.' => {
                let mut s = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' || n == '.' {
                        s.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push(s);
            }
            _ => {
                chars.next();
                out.push(c.to_string());
            }
        }
    }
    out
}

struct Parser {
    toks: Vec<String>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.toks.get(self.pos).map(|s| s.as_str())
    }

    fn bump(&mut self) -> Option<String> {
        let token = self.toks.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expect(&mut self, want: &str) -> Result<(), String> {
        match self.bump().as_deref() {
            Some(got) if got == want => Ok(()),
            other => Err(format!("expected {want}, got {other:?}")),
        }
    }

    fn value(&mut self) -> Result<Val, String> {
        match self.peek() {
            Some("{") => {
                self.bump();
                let mut fields = Vec::new();
                while self.peek() != Some("}") {
                    let key = self.bump().ok_or("missing key")?;
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
            Some(t) if t.starts_with('\x00') => {
                let token = self.bump().unwrap();
                Ok(Val::Str(token[1..].to_string()))
            }
            Some(t) if t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                Ok(Val::Num(self.bump().unwrap()))
            }
            Some(_) => {
                let name = self.bump().ok_or("missing value")?;
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
    while let Some(kind) = parser.peek().map(|s| s.to_string()) {
        if kind == "}" || kind == "]" {
            parser.bump();
            continue;
        }
        parser.bump();
        let name = parser.bump().ok_or("declaration without a name")?;
        parser.expect("{")?;
        let mut fields = Vec::new();
        while parser.peek() != Some("}") {
            let key = parser.bump().ok_or("field without a name")?;
            parser.expect(":")?;
            fields.push((key, parser.value()?));
            if parser.peek() == Some(",") {
                parser.bump();
            }
        }
        parser.expect("}")?;
        out.push(Decl { kind, name, fields });
    }
    Ok(out)
}
