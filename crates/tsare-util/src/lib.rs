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

/// Index combinations of size `k`, in lexicographic order.
pub fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn walk(start: usize, n: usize, k: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if current.len() == k {
            out.push(current.clone());
            return;
        }
        for index in start..n {
            if n - index < k - current.len() {
                break;
            }
            current.push(index);
            walk(index + 1, n, k, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    walk(0, n, k, &mut Vec::new(), &mut out);
    out
}
