//! Canonical printing: one spelling for a document that has many.
//!
//! `check` reads whatever an author wrote, which is the right thing for a checker. `fmt` writes
//! the form a diff and a hash can rely on: one declaration per block, one field per line, list
//! and record separators spelled the same way everywhere.
//!
//! It prints, it does not rewrite. Comments live in the text and not in the syntax tree, so an
//! in-place rewrite would delete them, and deleting a comment is an edit, not a formatting
//! decision.

use crate::syntax::{Decl, Val};

/// The canonical spelling of a whole document, header included.
pub fn document(decls: &[Decl]) -> String {
    let mut out = String::from("tsare 1\n");
    for decl in decls {
        out.push('\n');
        out.push_str(&declaration(decl));
    }
    out
}

pub fn declaration(decl: &Decl) -> String {
    let mut out = format!("{} {} {{\n", decl.kind, decl.name);
    for (key, value) in &decl.fields {
        out.push_str(&format!("  {key}: {},\n", value_of(value)));
    }
    out.push_str("}\n");
    out
}

pub fn value_of(value: &Val) -> String {
    match value {
        Val::Id(text) => text.clone(),
        Val::Str(text) => quote(text),
        Val::Num(text) => text.clone(),
        Val::Hole => "?".to_string(),
        Val::List(items) => {
            let parts: Vec<String> = items.iter().map(value_of).collect();
            format!("[{}]", parts.join(", "))
        }
        Val::Call(name, args) => {
            let parts: Vec<String> = args.iter().map(value_of).collect();
            format!("{name}({})", parts.join(", "))
        }
        Val::Rec(fields) => {
            let parts: Vec<String> = fields
                .iter()
                .map(|(key, item)| format!("{key}: {}", value_of(item)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// The escapes the parser understands, and only those: an escape it would pass through unchanged
/// is not an escape.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
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
