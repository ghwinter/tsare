//! What changed between two reports, keyed by the declaration rather than by position.
//!
//! A line number is true for one revision of one file: an edit above a diagnostic moves it, so two
//! reports of the same document can disagree about where a complaint is while agreeing about what
//! it is.  The name is the part that survives the edit, which is why the report carries one.
//!
//! This is the quantity a loop needs and cannot compute from a single run: whether an edit removed
//! complaints without adding new ones.  It reports the change; it does not call the change an
//! improvement, because fewer warnings can also mean a weaker document.

use std::collections::BTreeMap;

use crate::json::{self, Json};

pub struct Change {
    pub name: String,
    pub code: String,
    pub count: usize,
}

pub struct Difference {
    pub before: usize,
    pub after: usize,
    pub gone: Vec<Change>,
    pub appeared: Vec<Change>,
}

/// The key a complaint is remembered by.  A diagnostic about no declaration (a parse error, a
/// file-level complaint) is keyed by its code, which is the most it has.
fn key(item: &Json) -> (String, String) {
    let code = item.get("code").and_then(Json::as_str).unwrap_or("");
    let name = item.get("name").and_then(Json::as_str).unwrap_or("");
    (name.to_string(), code.to_string())
}

/// Complaints counted by the pair that identifies them: the declaration and the code.
type Counts = BTreeMap<(String, String), usize>;

fn counts(report: &str) -> Result<(Counts, usize), String> {
    let parsed = json::parse(report).map_err(|error| format!("not a report: {error}"))?;
    let items = parsed
        .get("diagnostics")
        .and_then(Json::as_arr)
        .ok_or_else(|| "not a report: no `diagnostics` array".to_string())?;
    let mut out: Counts = BTreeMap::new();
    for item in items {
        *out.entry(key(item)).or_insert(0) += 1;
    }
    Ok((out, items.len()))
}

pub fn compare(before: &str, after: &str) -> Result<Difference, String> {
    let (left, left_total) = counts(before)?;
    let (right, right_total) = counts(after)?;
    let mut difference = Difference {
        before: left_total,
        after: right_total,
        gone: Vec::new(),
        appeared: Vec::new(),
    };
    for ((name, code), count) in &left {
        let now = right
            .get(&(name.clone(), code.clone()))
            .copied()
            .unwrap_or(0);
        if now < *count {
            difference.gone.push(Change {
                name: name.clone(),
                code: code.clone(),
                count: count - now,
            });
        }
    }
    for ((name, code), count) in &right {
        let was = left
            .get(&(name.clone(), code.clone()))
            .copied()
            .unwrap_or(0);
        if was < *count {
            difference.appeared.push(Change {
                name: name.clone(),
                code: code.clone(),
                count: count - was,
            });
        }
    }
    Ok(difference)
}

fn list(items: &[Change]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    let shown: Vec<String> = items
        .iter()
        .map(|item| {
            let times = if item.count > 1 {
                format!(" (x{})", item.count)
            } else {
                String::new()
            };
            if item.name.is_empty() {
                format!("{}{times}", item.code)
            } else {
                format!("{} {}{times}", item.code, item.name)
            }
        })
        .collect();
    shown.join(", ")
}

pub fn text(difference: &Difference) -> String {
    let change = difference.after as isize - difference.before as isize;
    let mut out = format!(
        "{} -> {} diagnostic(s) ({change:+})\n",
        difference.before, difference.after
    );
    out.push_str(&format!("  gone:     {}\n", list(&difference.gone)));
    out.push_str(&format!("  appeared: {}\n", list(&difference.appeared)));
    out
}

pub fn to_json(difference: &Difference, recorded: Option<&str>) -> String {
    let render = |items: &[Change]| -> String {
        let rows: Vec<String> = items
            .iter()
            .map(|item| {
                format!(
                    "{{\"name\": \"{}\", \"code\": \"{}\", \"count\": {}}}",
                    crate::cli::escape(&item.name),
                    crate::cli::escape(&item.code),
                    item.count
                )
            })
            .collect();
        format!("[{}]", rows.join(", "))
    };
    let mut out = format!(
        "{{\"before\": {}, \"after\": {}, \"gone\": {}, \"appeared\": {}",
        difference.before,
        difference.after,
        render(&difference.gone),
        render(&difference.appeared)
    );
    if let Some(store) = recorded {
        out.push_str(&format!(
            ", \"recorded\": \"{}\"",
            crate::cli::escape(store)
        ));
    }
    out.push('}');
    out
}
