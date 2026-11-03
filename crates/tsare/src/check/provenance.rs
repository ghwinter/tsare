//! Where a quoted claim sits in its source: the line and column a reader can point at.

use crate::program::{f_id, f_str};
use crate::syntax::{Decl, Val};

/// One quoted claim and the place its quote occupies in the source.
pub struct Row {
    pub name: String,
    pub kind: String,
    pub source: String,
    pub source_name: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub quote: String,
}

/// Every quoted claim in document order.
pub fn rows(decls: &[Decl]) -> Vec<Row> {
    let mut out = Vec::new();
    for decl in decls {
        if decl.kind != "fact" && decl.kind != "proposition" {
            continue;
        }
        let Some(source) = f_id(decl, "source") else {
            continue;
        };
        let Some(quote) = f_str(decl, "quote") else {
            continue;
        };
        let Some(target) = decls.iter().find(|item| item.name == source) else {
            continue;
        };
        let source_name = f_str(target, "name").unwrap_or(source).to_string();
        let found = locate(&lines(target), quote);
        out.push(Row {
            name: decl.name.clone(),
            kind: decl.kind.clone(),
            source: source.to_string(),
            source_name,
            line: found.map(|(line, _)| line),
            col: found.map(|(_, col)| col),
            quote: quote.to_string(),
        });
    }
    out
}

/// The lines of the source's `text` list.
pub fn lines(decl: &Decl) -> Vec<String> {
    match decl
        .fields
        .iter()
        .find(|(name, _)| name == "text")
        .map(|(_, value)| value)
    {
        Some(Val::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Val::Str(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The first line and column (both 1-based) the quote starts at.
pub fn locate(lines: &[String], quote: &str) -> Option<(usize, usize)> {
    if quote.is_empty() {
        return None;
    }
    lines.iter().enumerate().find_map(|(index, line)| {
        line.find(quote)
            .map(|byte| (index + 1, line[..byte].chars().count() + 1))
    })
}
