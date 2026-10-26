//! Apply a suggestion: splice over its span, recompile, keep only improvements.

use std::path::Path;

use crate::check::check;
use crate::fix::Suggestion;
use crate::syntax;

pub fn offset(text: &str, line: usize, col: usize) -> usize {
    let mut current_line = 1;
    let mut index = 0;
    for ch in text.chars() {
        if current_line == line {
            break;
        }
        if ch == '\n' {
            current_line += 1;
        }
        index += ch.len_utf8();
    }
    let mut remaining = col.saturating_sub(1);
    let mut byte = index;
    for ch in text[index..].chars() {
        if remaining == 0 {
            break;
        }
        byte += ch.len_utf8();
        remaining -= 1;
    }
    byte
}

pub fn splice(text: &str, suggestion: &Suggestion) -> String {
    let start = offset(text, suggestion.line, suggestion.col);
    let end = offset(text, suggestion.end_line, suggestion.end_col);
    let mut tail = text[end..].to_string();
    if suggestion.replacement.is_empty() && tail.starts_with('\n') {
        tail = tail[1..].to_string();
    }
    format!("{}{}{}", &text[..start], suggestion.replacement, tail)
}

pub fn score(text: &str) -> (usize, usize) {
    match syntax::parse(text) {
        Ok(decls) => {
            let diagnostics = check(&decls);
            let errors = diagnostics.iter().filter(|item| item.code.starts_with("E-")).count();
            let warnings = diagnostics.iter().filter(|item| item.code.starts_with("W-")).count();
            (errors, warnings)
        }
        Err(_) => (usize::MAX, usize::MAX),
    }
}

pub fn apply_best(text: &str, suggestions: &[Suggestion]) -> (String, Vec<String>) {
    let mut current = text.to_string();
    let mut current_score = score(&current);
    let mut applied = Vec::new();
    for suggestion in suggestions {
        if suggestion.applicability != "MachineApplicable" {
            continue;
        }
        let candidate = splice(&current, suggestion);
        let candidate_score = score(&candidate);
        if candidate_score < current_score {
            current = candidate;
            current_score = candidate_score;
            applied.push(suggestion.message.clone());
        }
    }
    (current, applied)
}

pub fn write(path: &Path, text: &str) {
    std::fs::write(path, text).expect("write file");
}
