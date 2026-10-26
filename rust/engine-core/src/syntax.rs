//! Lexer and parser for the declaration syntax, with positions.

/// A source span: where a value, a reference or a declaration starts and ends.
pub type Span = (usize, usize, usize, usize);

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
    /// The value's span, parallel to `fields`.
    pub field_span: Vec<Span>,
    pub kind_line: usize,
    pub kind_col: usize,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub file: String,
    /// Every identifier value in the declaration, with the span of its occurrence.
    pub ref_pos: Vec<(String, usize, usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    /// Characters consumed in the source, so `end_col` is the position after the token.
    pub width: usize,
    pub line: usize,
    pub col: usize,
}

fn lex(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        if c == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            col += 1;
            continue;
        }
        if c == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            let start_col = col;
            let mut j = i + 1;
            let mut text = String::new();
            while j < n && chars[j] != '"' {
                if chars[j] == '\\' && j + 1 < n {
                    let escaped = chars[j + 1];
                    text.push(match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        other => other,
                    });
                    j += 2;
                    continue;
                }
                text.push(chars[j]);
                j += 1;
            }
            let width = j - i + 1;
            out.push(Token { text: format!("\x00{text}"), width, line, col: start_col });
            i = j + 1;
            col += width;
            continue;
        }
        if c.is_ascii_digit() || (c == '-' && i + 1 < n && chars[i + 1].is_ascii_digit()) {
            let mut text = String::new();
            if c == '-' {
                text.push('-');
                i += 1;
            }
            while i < n && (chars[i].is_ascii_digit() || chars[i] == '.') {
                text.push(chars[i]);
                i += 1;
            }
            // `16ms` is one number: the unit suffix is part of it, not an identifier
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '%') {
                text.push(chars[i]);
                i += 1;
            }
            let width = text.chars().count();
            out.push(Token { text, width, line, col });
            col += width;
            continue;
        }
        if c.is_alphanumeric() || c == '_' || c == '.' {
            let mut text = String::new();
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                text.push(chars[i]);
                i += 1;
            }
            let width = text.chars().count();
            out.push(Token { text, width, line, col });
            col += width;
            continue;
        }
        out.push(Token { text: c.to_string(), width: 1, line, col });
        i += 1;
        col += 1;
    }
    out
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    ref_pos: Vec<(String, usize, usize, usize)>,
    last_end: (usize, usize),
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
        if let Some(token) = &token {
            self.last_end = (token.line, token.col + token.width);
        }
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
            Some(text) if text.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) => {
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
                    let (line, col) = self
                        .toks
                        .get(self.pos.saturating_sub(1))
                        .map(|token| (token.line, token.col))
                        .unwrap_or((0, 0));
                    self.ref_pos.push((name.clone(), line, col, col + name.chars().count()));
                    Ok(Val::Id(name))
                }
            }
            None => Err("unexpected end of input".into()),
        }
    }
}

pub fn parse(src: &str) -> Result<Vec<Decl>, String> {
    let mut parser = Parser { toks: lex(src), pos: 0, ref_pos: Vec::new(), last_end: (0, 0) };
    let mut out = Vec::new();
    while let Some(kind) = parser.peek().map(|text| text.to_string()) {
        if kind == "}" || kind == "]" {
            parser.bump();
            continue;
        }
        let (line, col) = parser.position();
        parser.ref_pos.clear();
        let kind_token = parser.bump().ok_or("declaration without a kind")?;
        let name = parser.bump().ok_or("declaration without a name")?.text;
        parser.expect("{")?;
        let mut fields = Vec::new();
        let mut field_span = Vec::new();
        while parser.peek() != Some("}") {
            let key = parser.bump().ok_or("field without a name")?.text;
            parser.expect(":")?;
            let (value_line, value_col) = parser.position();
            let value = parser.value()?;
            fields.push((key, value));
            field_span.push((value_line, value_col, parser.last_end.0, parser.last_end.1));
            if parser.peek() == Some(",") {
                parser.bump();
            }
        }
        let (end_line, end_col) = parser.position();
        parser.expect("}")?;
        out.push(Decl {
            kind,
            name,
            fields,
            field_span,
            kind_line: kind_token.line,
            kind_col: kind_token.col,
            line,
            col,
            end_line,
            end_col: end_col + 1,
            file: String::new(),
            ref_pos: parser.ref_pos.clone(),
        });
    }
    Ok(out)
}
