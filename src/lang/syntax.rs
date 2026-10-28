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

fn lex(src: &str) -> (Vec<Token>, Vec<ParseError>) {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Token> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();
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
            let mut terminated = false;
            while j < n && chars[j] != '\n' {
                if chars[j] == '"' {
                    terminated = true;
                    break;
                }
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
            // an unterminated string ends at the newline: report it and keep lexing, so the
            // rest of the file can still produce declarations and errors of its own
            let width = if terminated { j - i + 1 } else { j - i };
            if !terminated {
                errors.push(ParseError {
                    message: "unterminated string literal".to_string(),
                    line,
                    col: start_col,
                    code: "E-SYNTAX",
                });
            }
            out.push(Token {
                text: format!("\x00{text}"),
                width,
                line,
                col: start_col,
            });
            i = if terminated { j + 1 } else { j };
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
            out.push(Token {
                text,
                width,
                line,
                col,
            });
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
            out.push(Token {
                text,
                width,
                line,
                col,
            });
            col += width;
            continue;
        }
        out.push(Token {
            text: c.to_string(),
            width: 1,
            line,
            col,
        });
        i += 1;
        col += 1;
    }
    (out, errors)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    ref_pos: Vec<(String, usize, usize, usize)>,
    last_end: (usize, usize),
    /// How deep the value currently being parsed is nested.  Recursive descent uses the call
    /// stack as its own stack, so without a bound a hostile document can overflow it before any
    /// diagnostic exists: the process dies instead of saying the document is too deep.
    depth: usize,
}

/// Deeper than any document a person writes; shallow enough that the recursion cannot exhaust
/// the stack.
const MAX_DEPTH: usize = 128;

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

    fn fail(&self, message: String) -> ParseError {
        // at the end of input the useful position is the end of the last token read
        let (line, col) = if self.pos >= self.toks.len() {
            self.last_end
        } else {
            self.position()
        };
        ParseError {
            message,
            line: line.max(1),
            col: col.max(1),
            code: "E-SYNTAX",
        }
    }

    fn expect(&mut self, want: &str) -> Result<(), ParseError> {
        match self.bump() {
            Some(token) if token.text == want => Ok(()),
            Some(token) => Err(ParseError {
                message: format!("expected {want}, got {:?}", token.text),
                line: token.line,
                col: token.col,
                code: "E-SYNTAX",
            }),
            None => Err(self.fail(format!("expected {want}, reached the end"))),
        }
    }

    fn value(&mut self) -> Result<Val, ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err(self.fail(format!(
                "value nested deeper than {MAX_DEPTH}; a document this deep has no reading"
            )));
        }
        self.depth += 1;
        let parsed = self.value_inner();
        self.depth -= 1;
        parsed
    }

    fn value_inner(&mut self) -> Result<Val, ParseError> {
        match self.peek() {
            Some("{") => {
                self.bump();
                let mut fields = Vec::new();
                while self.peek() != Some("}") {
                    if self.peek().is_none() {
                        return Err(self.fail("unexpected end of input: missing }".to_string()));
                    }
                    let key = self
                        .bump()
                        .ok_or_else(|| self.fail("missing field name".to_string()))?
                        .text;
                    self.expect(":")?;
                    fields.push((key, self.value()?));
                    if self.peek() == Some(",") {
                        self.bump();
                    } else if self.peek() != Some("}") {
                        return Err(self.fail("expected , or } in a record".to_string()));
                    }
                }
                self.expect("}")?;
                Ok(Val::Rec(fields))
            }
            Some("[") => {
                self.bump();
                let mut items = Vec::new();
                while self.peek() != Some("]") {
                    if self.peek().is_none() {
                        return Err(self.fail("unexpected end of input: missing ]".to_string()));
                    }
                    items.push(self.value()?);
                    if self.peek() == Some(",") {
                        self.bump();
                    } else if self.peek() != Some("]") {
                        return Err(self.fail("expected , or ] in a list".to_string()));
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
            Some(text)
                if text
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit() || c == '-')
                    .unwrap_or(false) =>
            {
                Ok(Val::Num(self.bump().unwrap().text))
            }
            Some(_) => {
                let name = self
                    .bump()
                    .ok_or_else(|| self.fail("missing value".to_string()))?
                    .text;
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
                    self.ref_pos
                        .push((name.clone(), line, col, col + name.chars().count()));
                    Ok(Val::Id(name))
                }
            }
            None => Err(self.fail("unexpected end of input".to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
    /// The diagnostic code this error reports.  Almost everything is `E-SYNTAX`; a document
    /// that declares a format this build does not speak is `E-VERSION`, because the document
    /// is well formed -- it is the compiler that cannot read it.
    pub code: &'static str,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl Parser {
    /// One declaration, from its kind token to its closing brace.
    fn declaration(&mut self) -> Result<Decl, ParseError> {
        let (line, col) = self.position();
        self.ref_pos.clear();
        let kind_token = self
            .bump()
            .ok_or_else(|| self.fail("declaration without a kind".to_string()))?;
        let name = self
            .bump()
            .ok_or_else(|| self.fail("declaration without a name".to_string()))?
            .text;
        self.expect("{")?;
        let mut fields = Vec::new();
        let mut field_span = Vec::new();
        while self.peek() != Some("}") {
            if self.peek().is_none() {
                return Err(self.fail("unexpected end of input: missing }".to_string()));
            }
            let key = self
                .bump()
                .ok_or_else(|| self.fail("field without a name".to_string()))?
                .text;
            self.expect(":")?;
            let (value_line, value_col) = self.position();
            let value = self.value()?;
            fields.push((key, value));
            field_span.push((value_line, value_col, self.last_end.0, self.last_end.1));
            if self.peek() == Some(",") {
                self.bump();
            } else if self.peek() != Some("}") {
                // The mistake a model makes most often is separating fields with a space, so the
                // message names the missing comma instead of the two tokens it could be.
                return Err(self.fail(
                    "expected , or } after a field; fields are separated by commas".to_string(),
                ));
            }
        }
        let (end_line, end_col) = self.position();
        self.expect("}")?;
        Ok(Decl {
            kind: kind_token.text,
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
            ref_pos: self.ref_pos.clone(),
        })
    }

    /// Skip to the next line-initial identifier: the place a declaration may begin.  A broken
    /// record cannot be repaired locally, so the parser restarts where the author started.
    fn recover(&mut self) {
        while let Some(token) = self.toks.get(self.pos) {
            let line_initial = token.col == 1;
            let identifier = token
                .text
                .chars()
                .next()
                .map(|c| c.is_alphabetic() || c == '_')
                .unwrap_or(false);
            if line_initial && identifier {
                return;
            }
            self.pos += 1;
        }
    }
}

/// The format this build speaks.  A document declares a version only when it is newer than the
/// first one: an absent header means format 1, so a document written before the header existed
/// is not a document of unknown version.  The number changes only when the grammar does, never
/// when the crate does.
pub const FORMAT_VERSION: u32 = 1;

/// The optional first line, `tsare <n>`.  `None` means no header.  `Ok(n)` is the number of
/// bytes the header occupies, including its newline.  `Err` means the document declares a
/// format this build cannot read, and the caller must stop before parsing: reading a newer
/// syntax as this one would produce a page of nonsense errors instead of one true one.
fn header(src: &str) -> Option<Result<usize, ParseError>> {
    let line_end = src.find('\n').unwrap_or(src.len());
    let line = src[..line_end].trim_end_matches('\r');
    let mut words = line.split_whitespace();
    if words.next()? != "tsare" {
        return None;
    }
    let rest: Vec<&str> = words.collect();
    let Some(first) = rest.first() else {
        return Some(Err(version_error(
            "the first line says `tsare` and no version number",
        )));
    };
    let Ok(version) = first.parse::<u32>() else {
        return Some(Err(version_error(&format!(
            "`{first}` is not a format version; the first line reads `tsare <number>`"
        ))));
    };
    if let Some(trailing) = rest.get(1) {
        if !trailing.starts_with('#') {
            return Some(Err(version_error(&format!(
                "unexpected `{trailing}` after the format version"
            ))));
        }
    }
    if version != FORMAT_VERSION {
        return Some(Err(version_error(&format!(
            "this document is format {version}; this build speaks format {FORMAT_VERSION}, so \
             it cannot read this one"
        ))));
    }
    Some(Ok(line_end + usize::from(line_end < src.len())))
}

fn version_error(message: &str) -> ParseError {
    ParseError {
        message: message.to_string(),
        line: 1,
        col: 1,
        code: "E-VERSION",
    }
}

/// Parse, keeping every error: after one declaration fails the parser restarts at the next
/// line-initial identifier, so one typo does not hide the rest of the file.  The lexer fails
/// only on an unterminated string, which it reports and then ends at the newline.
pub fn parse_recovering(src: &str) -> (Vec<Decl>, Vec<ParseError>) {
    let body = match header(src) {
        None => src.to_string(),
        Some(Err(error)) => return (Vec::new(), vec![error]),
        Some(Ok(consumed)) => {
            // blank the header line but keep its newline, so every line number stays absolute
            let mut text = String::with_capacity(src.len());
            if consumed > 0 {
                text.extend(std::iter::repeat_n(' ', consumed - 1));
                text.push('\n');
            }
            text.push_str(&src[consumed..]);
            text
        }
    };
    let (tokens, lex_errors) = lex(&body);
    let mut parser = Parser {
        toks: tokens,
        pos: 0,
        ref_pos: Vec::new(),
        last_end: (0, 0),
        depth: 0,
    };
    let mut decls = Vec::new();
    let mut errors = lex_errors;
    while let Some(kind) = parser.peek().map(|text| text.to_string()) {
        if kind == "}" || kind == "]" {
            parser.bump();
            continue;
        }
        let start = parser.pos;
        match parser.declaration() {
            Ok(decl) => decls.push(decl),
            Err(error) => {
                errors.push(error);
                parser.recover();
                if parser.pos == start {
                    parser.bump();
                }
            }
        }
    }
    (decls, errors)
}

pub fn parse(src: &str) -> Result<Vec<Decl>, ParseError> {
    let (decls, errors) = parse_recovering(src);
    match errors.into_iter().next() {
        Some(error) => Err(error),
        None => Ok(decls),
    }
}
