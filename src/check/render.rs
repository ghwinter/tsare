//! Rendered diagnostics: the source line, a caret under the span, and the hint.
//!
//! A diagnostic is an edit suggestion, so the reader must see the text it points at.  The
//! layout follows the convention a compiler user already knows: `file:line:col: severity[code]:
//! message`, then the line, then the caret.

use crate::check::Diagnostic;

pub fn render(source: &str, file: &str, diagnostics: &[Diagnostic]) -> String {
    let mut sorted: Vec<&Diagnostic> = diagnostics.iter().collect();
    sorted.sort_by(|left, right| {
        (left.line, left.col, left.code, &left.message).cmp(&(
            right.line,
            right.col,
            right.code,
            &right.message,
        ))
    });
    let lines: Vec<&str> = source.lines().collect();
    let mut out = String::new();
    for item in sorted {
        let severity = if item.code.starts_with('E') {
            "error"
        } else {
            "warning"
        };
        out.push_str(&format!(
            "{file}:{}:{}: {severity}[{}]: {}\n",
            item.line, item.col, item.code, item.message
        ));
        let gutter = " ".repeat(item.line.to_string().len());
        if item.line >= 1 && item.line <= lines.len() {
            let text = lines[item.line - 1];
            out.push_str(&format!(" {} | {text}\n", item.line));
            let start = item.col.clamp(1, text.chars().count() + 1);
            let width = if item.end_line == item.line {
                item.end_col.saturating_sub(item.col).max(1)
            } else {
                1
            };
            out.push_str(&format!(
                " {gutter} | {}{}\n",
                " ".repeat(start - 1),
                "^".repeat(width)
            ));
        }
        out.push('\n');
    }
    out
}
