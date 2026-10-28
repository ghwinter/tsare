//! Content-addressed cache for diagnostics.
//!
//! A file whose bytes have not changed cannot produce different diagnostics, so the second
//! check of the same file is pure waste.  The cache is keyed by sha256 of the source (salted
//! with the checker identity) and stores the diagnostic records; a hit replays them verbatim.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::check::Diagnostic;
use crate::hash::hex;
use crate::json::{self, Json};

/// Part of every key: a cache built by another checker must never be replayed.  The shape of a
/// record is part of the checker's identity, so a report that gained a field gets a new salt rather
/// than replaying entries that would silently carry an empty name.
pub const SALT: &str = "tsare-0.3";

pub fn store_path() -> PathBuf {
    PathBuf::from(".tsare").join("check-cache.json")
}

pub fn key(text: &str) -> String {
    hex(format!("{SALT}{text}").as_bytes())
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

pub fn dump(diagnostics: &[Diagnostic]) -> String {
    let records: Vec<String> = diagnostics
        .iter()
        .map(|item| {
            format!(
                "{{\"code\": \"{}\", \"message\": \"{}\", \"name\": \"{}\", \"file\": \"{}\", \"line\": {}, \"col\": {}, \"end_line\": {}, \"end_col\": {}}}",
                escape(item.code),
                escape(&item.message),
                escape(item.witness.as_deref().unwrap_or("")),
                escape(&item.file),
                item.line,
                item.col,
                item.end_line,
                item.end_col
            )
        })
        .collect();
    format!("[{}]", records.join(", "))
}

pub fn restore(payload: &Json) -> Vec<Diagnostic> {
    payload
        .as_arr()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let code = item.get("code")?.as_str()?;
                    let message = item
                        .get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let line = item
                        .get("line")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(1.0) as usize;
                    let col = item
                        .get("col")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(1.0) as usize;
                    // The record has always carried the name; the reader used to drop it, and a
                    // cache hit then reported an empty `name` for a diagnostic that has one.
                    let name = item
                        .get("name")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let file = item
                        .get("file")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let end_line = item
                        .get("end_line")
                        .and_then(|value| value.as_f64())
                        .map(|value| value as usize)
                        .unwrap_or(line);
                    let end_col = item
                        .get("end_col")
                        .and_then(|value| value.as_f64())
                        .map(|value| value as usize)
                        .unwrap_or(col);
                    Some(Diagnostic {
                        code: crate::check::code_of(code),
                        message: message.to_string(),
                        line,
                        col,
                        end_line,
                        end_col,
                        file: file.to_string(),
                        witness: if name.is_empty() {
                            None
                        } else {
                            Some(name.to_string())
                        },
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn load() -> HashMap<String, Json> {
    let Ok(text) = std::fs::read_to_string(store_path()) else {
        return HashMap::new();
    };
    match json::parse(&text) {
        Ok(Json::Obj(fields)) => fields.into_iter().collect(),
        _ => HashMap::new(),
    }
}

pub fn save(entries: &[(String, Json)]) -> std::io::Result<()> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let fields: Vec<(String, Json)> = entries.to_vec();
    std::fs::write(path, json::serialize(&Json::Obj(fields)))
}
