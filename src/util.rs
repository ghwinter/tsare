//! Shared helpers with no opinion about a document.
//!
//! `escape` turns the characters a JSON string cannot carry into their spellings.  It lives here,
//! not in the CLI, so a core module can quote a string without depending on the presentation layer.

pub fn escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
