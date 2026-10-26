//! Surface sugar: one line in, one declaration out, same line number.
//!
//! Writing the record form by hand is the main friction of the language, and a model writes
//! the long form badly.  These three forms are expanded line-by-line before parsing, so spans
//! and diagnostics still point at the line the author wrote.
//!
//! ```text
//!     fact "the window appears at launch" by user as F_WINDOW
//!     measure "frame_ms" min 16ms by tool("trace") as V_FRAME
//!     because F_SCROLL rule "one frame is the budget" given V_FRAME => P_FAST
//! ```

fn quoted(text: &str) -> Option<(String, &str)> {
    let start = text.find('"')?;
    let rest = &text[start + 1..];
    let end = rest.find('"')?;
    Some((rest[..end].to_string(), &rest[end + 1..]))
}

fn identifier(text: &str) -> Option<&str> {
    let text = text.trim();
    let mut chars = text.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some(text)
}

fn identifiers(text: &str) -> Vec<String> {
    text.split(',').map(|part| part.trim()).filter(|part| !part.is_empty()).map(|part| part.to_string()).collect()
}

/// `16ms` -> ("16", "ms", rest of the line).
fn split_number(text: &str) -> Option<(String, String, &str)> {
    let mut end = 0;
    let mut seen_dot = false;
    for (index, ch) in text.char_indices() {
        if ch.is_ascii_digit() {
            end = index + ch.len_utf8();
        } else if ch == '.' && !seen_dot {
            seen_dot = true;
            end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let number = text[..end].to_string();
    let mut unit_end = end;
    for (index, ch) in text[end..].char_indices() {
        if ch.is_ascii_alphabetic() || ch == '%' {
            unit_end = end + index + ch.len_utf8();
        } else {
            break;
        }
    }
    Some((number, text[end..unit_end].to_string(), &text[unit_end..]))
}

pub fn expand_line(line: &str, number: usize) -> String {
    let trimmed = line.trim_end();

    if let Some(rest) = trimmed.strip_prefix("fact ") {
        if rest.starts_with('"') {
            if let Some((claim, tail)) = quoted(rest) {
                if let Some(tail) = tail.trim_start().strip_prefix("by ") {
                    if let Some((evidence, name)) = tail.rsplit_once(" as ") {
                        if let Some(name) = identifier(name) {
                            return format!(
                                "fact {name} {{ claim: \"{claim}\", evidence: {} }}",
                                evidence.trim()
                            );
                        }
                    }
                }
            }
        }
    }

    if let Some(rest) = trimmed.strip_prefix("measure ") {
        if rest.starts_with('"') {
            if let Some((metric, tail)) = quoted(rest) {
                let tail = tail.trim_start();
                let (direction, body) = if let Some(body) = tail.strip_prefix("min ") {
                    ("min", body)
                } else if let Some(body) = tail.strip_prefix("max ") {
                    ("max", body)
                } else {
                    ("", "")
                };
                if !direction.is_empty() {
                    if let Some((number, unit, rest)) = split_number(body.trim_start()) {
                        if let Some(rest) = rest.trim_start().strip_prefix("by ") {
                            if let Some((instrument, name)) = rest.rsplit_once(" as ") {
                                if let Some(name) = identifier(name) {
                                    return format!(
                                        "value {name} {{ metric: \"{metric}\", direction: {direction}, unit: \"{unit}\", threshold: {number}, measured_by: {} }}",
                                        instrument.trim()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(rest) = trimmed.strip_prefix("because ") {
        if let Some((premises, tail)) = rest.split_once(" rule ") {
            if let Some((claim, after)) = quoted(tail) {
                let after = after.trim();
                let (basis, conclusion) = match after.strip_prefix("given ") {
                    Some(rest) => match rest.split_once("=>") {
                        Some((basis, conclusion)) => (Some(basis.trim().to_string()), conclusion),
                        None => (None, rest),
                    },
                    None => (None, after),
                };
                let conclusion = conclusion.trim();
                if let Some(conclusion) = identifier(conclusion) {
                    let froms = identifiers(premises).join(", ");
                    let extra = match &basis {
                        Some(basis) if !basis.is_empty() => format!(", basis: [{basis}]"),
                        _ => String::new(),
                    };
                    return format!(
                        "warrant W_L{number} {{ claim: \"{claim}\", from: [{froms}], to: {conclusion}{extra} }}"
                    );
                }
            }
        }
    }

    line.to_string()
}

pub fn expand(source: &str) -> String {
    source.lines().enumerate().map(|(index, line)| expand_line(line, index + 1)).collect::<Vec<_>>().join("\n")
}
