//! A small JSON reader, for the stores the analyses keep (cases, cache).
//!
//! Values are read, never trusted: an entry that does not answer a question is skipped.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }

    pub fn strings(&self) -> Vec<String> {
        self.as_arr()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(|text| text.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

struct Reader<'a> {
    chars: Vec<char>,
    pos: usize,
    _text: &'a str,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let value = self.peek();
        self.pos += 1;
        value
    }

    fn skip(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bump() != Some('"') {
            return Err("expected a string".into());
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('b') => out.push('\u{8}'),
                    Some('f') => out.push('\u{c}'),
                    Some('u') => {
                        let mut code = String::new();
                        for _ in 0..4 {
                            code.push(self.bump().ok_or("short escape")?);
                        }
                        let value = u32::from_str_radix(&code, 16).map_err(|_| "bad escape")?;
                        out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
                    }
                    Some(other) => out.push(other),
                    None => return Err("unterminated escape".into()),
                },
                Some(c) => out.push(c),
                None => return Err("unterminated string".into()),
            }
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip();
        match self.peek() {
            Some('{') => {
                self.bump();
                let mut fields = Vec::new();
                self.skip();
                if self.peek() == Some('}') {
                    self.bump();
                    return Ok(Json::Obj(fields));
                }
                loop {
                    self.skip();
                    let key = self.string()?;
                    self.skip();
                    if self.bump() != Some(':') {
                        return Err("expected ':'".into());
                    }
                    fields.push((key, self.value()?));
                    self.skip();
                    match self.bump() {
                        Some(',') => continue,
                        Some('}') => return Ok(Json::Obj(fields)),
                        _ => return Err("expected ',' or '}'".into()),
                    }
                }
            }
            Some('[') => {
                self.bump();
                let mut items = Vec::new();
                self.skip();
                if self.peek() == Some(']') {
                    self.bump();
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.skip();
                    match self.bump() {
                        Some(',') => continue,
                        Some(']') => return Ok(Json::Arr(items)),
                        _ => return Err("expected ',' or ']'".into()),
                    }
                }
            }
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('t') => {
                for want in ['t', 'r', 'u', 'e'] {
                    if self.bump() != Some(want) {
                        return Err("bad literal".into());
                    }
                }
                Ok(Json::Bool(true))
            }
            Some('f') => {
                for want in ['f', 'a', 'l', 's', 'e'] {
                    if self.bump() != Some(want) {
                        return Err("bad literal".into());
                    }
                }
                Ok(Json::Bool(false))
            }
            Some('n') => {
                for want in ['n', 'u', 'l', 'l'] {
                    if self.bump() != Some(want) {
                        return Err("bad literal".into());
                    }
                }
                Ok(Json::Null)
            }
            Some(_) => {
                let mut text = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E') {
                        text.push(c);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                text.parse::<f64>()
                    .map(Json::Num)
                    .map_err(|_| format!("bad number {text:?}"))
            }
            None => Err("unexpected end of input".into()),
        }
    }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut reader = Reader {
        chars: text.chars().collect(),
        pos: 0,
        _text: text,
    };
    reader.value()
}

fn quote(text: &str) -> String {
    let mut out = String::from('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

pub fn serialize(value: &Json) -> String {
    match value {
        Json::Null => "null".to_string(),
        Json::Bool(true) => "true".to_string(),
        Json::Bool(false) => "false".to_string(),
        Json::Num(number) => format!("{number}"),
        Json::Str(text) => quote(text),
        Json::Arr(items) => format!(
            "[{}]",
            items.iter().map(serialize).collect::<Vec<_>>().join(", ")
        ),
        Json::Obj(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(key, item)| format!("{}: {}", quote(key), serialize(item)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// One JSON object per line; a malformed line is skipped rather than aborting the store.
pub fn parse_lines(text: &str) -> Vec<Json> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| parse(line).ok())
        .collect()
}
