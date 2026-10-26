//! Cases: what happened, in a form that can be retrieved and audited.
//!
//! A case is a situation, an action, and an outcome:
//!
//! ```text
//!     situation  the diagnostic codes and the kinds they landed on
//!     action     mechanical repair, model rewrite, or a dropped declaration
//!     outcome    errors and confirmations before and after
//! ```
//!
//! `recall` ranks cases by how much of the situation they share with the current one.  `audit`
//! checks the store itself: a memory that cannot say which of its entries are stale is a rumour.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::cache::SALT;
use crate::json::{self, Json};

pub fn store_path() -> PathBuf {
    PathBuf::from(".metaform").join("cases.jsonl")
}

pub fn load() -> Vec<Json> {
    std::fs::read_to_string(store_path())
        .map(|text| json::parse_lines(&text))
        .unwrap_or_default()
}

fn pair(entry: &Json, side: &str) -> (f64, f64) {
    let block = entry.get(side);
    let errors = block
        .and_then(|item| item.get("errors"))
        .and_then(|item| item.as_f64())
        .unwrap_or(0.0);
    let cost = block
        .and_then(|item| item.get("cost"))
        .and_then(|item| item.as_f64())
        .unwrap_or(0.0);
    (errors, cost)
}

/// Fewer errors, then less cost: the pair is compared the way the reference compares tuples.
pub fn improved(entry: &Json) -> bool {
    let before = pair(entry, "before");
    let after = pair(entry, "after");
    after.0 < before.0 || (after.0 == before.0 && after.1 < before.1)
}

fn codes_of(entry: &Json) -> Vec<String> {
    entry
        .get("codes")
        .map(|value| value.strings())
        .unwrap_or_default()
}

fn action_of(entry: &Json) -> String {
    entry
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn overlap(left: &[String], right: &[String]) -> f64 {
    let a: HashSet<&String> = left.iter().collect();
    let b: HashSet<&String> = right.iter().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    a.intersection(&b).count() as f64 / a.union(&b).count() as f64
}

pub struct Recalled {
    pub entry: Json,
    pub improved: bool,
}

pub fn recall(codes: &[String], k: usize) -> Vec<Recalled> {
    let mut scored: Vec<(f64, bool, Json)> = Vec::new();
    for entry in load() {
        let score = overlap(codes, &codes_of(&entry));
        if score > 0.0 {
            scored.push((score, improved(&entry), entry));
        }
    }
    scored.sort_by(|left, right| {
        // best overlap first; a case that improved outranks one that did not
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.1.cmp(&left.1))
    });
    scored
        .into_iter()
        .take(k)
        .map(|(_, improved, entry)| Recalled { entry, improved })
        .collect()
}

pub struct Audit {
    pub entries: usize,
    pub expired: usize,
    pub conflicting: usize,
    pub noop: usize,
    pub unchanged: usize,
    pub orphans: usize,
}

pub fn audit() -> Audit {
    let entries = load();
    let expired = entries
        .iter()
        .filter(|entry| entry.get("checker").and_then(|value| value.as_str()) != Some(SALT))
        .count();
    let noop = entries
        .iter()
        .filter(|entry| !improved(entry) && pair(entry, "after") != pair(entry, "before"))
        .count();
    let unchanged = entries
        .iter()
        .filter(|entry| pair(entry, "after") == pair(entry, "before"))
        .count();
    let orphans = entries
        .iter()
        .filter(|entry| {
            entry
                .get("doc")
                .and_then(|value| value.as_str())
                .map(|doc| !PathBuf::from(doc).exists())
                .unwrap_or(true)
        })
        .count();

    type GroupKey = (Vec<String>, String);
    let mut groups: Vec<(GroupKey, HashSet<bool>)> = Vec::new();
    for entry in &entries {
        let key = (codes_of(entry), action_of(entry));
        let improved = improved(entry);
        match groups.iter_mut().find(|(existing, _)| existing == &key) {
            Some((_, set)) => {
                set.insert(improved);
            }
            None => {
                let mut set = HashSet::new();
                set.insert(improved);
                groups.push((key, set));
            }
        }
    }
    let conflicting = groups.iter().filter(|(_, set)| set.len() > 1).count();

    Audit {
        entries: entries.len(),
        expired,
        conflicting,
        noop,
        unchanged,
        orphans,
    }
}

/// Python's list repr, because the store prints its code list the way it was written.
pub fn list_repr(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| format!("'{}'", item.replace('\'', "\\'")))
        .collect();
    format!("[{}]", quoted.join(", "))
}
