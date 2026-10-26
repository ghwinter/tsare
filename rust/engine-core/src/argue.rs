//! Which conclusions survive their own rules?
//!
//! Warrants can attack warrants.  The grounded extension gives the unique set of rules that
//! survive every attack they receive:
//!
//!     IN_0 = {}
//!     OUT_i = { a : some b in IN_i attacks a }
//!     IN_{i+1} = { a : every b that attacks a is in OUT_i }
//!
//! A proposition is supported when some rule for it is IN, defeated when every rule for it is
//! OUT, contested when the rules are undecided, and asserted when it has no rule at all.

use std::collections::HashMap;

use crate::closure::{grounded_sets, field_ids, id_field};
use crate::program::Program;

pub struct Conclusion {
    pub name: String,
    pub status: &'static str,
    pub why: String,
}

pub fn conclusions(prog: &Program) -> Vec<Conclusion> {
    let (inside, outside) = grounded_sets(prog.decls);
    let mut by_conclusion: HashMap<String, Vec<String>> = HashMap::new();
    for warrant in prog.of_kind("warrant") {
        if let Some(conclusion) = id_field(warrant, "to") {
            by_conclusion.entry(conclusion.to_string()).or_default().push(warrant.name.clone());
        }
    }

    let mut rows = Vec::new();
    for proposition in prog.of_kind("proposition") {
        let empty = Vec::new();
        let rules = by_conclusion.get(&proposition.name).unwrap_or(&empty);
        if rules.is_empty() {
            rows.push(Conclusion {
                name: proposition.name.clone(),
                status: "asserted",
                why: "no rule".to_string(),
            });
        } else if rules.iter().any(|rule| inside.contains(rule)) {
            let live: Vec<String> =
                rules.iter().filter(|rule| inside.contains(*rule)).cloned().collect();
            rows.push(Conclusion {
                name: proposition.name.clone(),
                status: "supported",
                why: live.join(", "),
            });
        } else if rules.iter().all(|rule| outside.contains(rule)) {
            let dead: Vec<String> =
                rules.iter().map(|rule| format!("{rule} (out)")).collect();
            rows.push(Conclusion {
                name: proposition.name.clone(),
                status: "defeated",
                why: dead.join(", "),
            });
        } else {
            let mixed: Vec<String> = rules
                .iter()
                .map(|rule| {
                    let label = if inside.contains(rule) { "in" } else { "undecided" };
                    format!("{rule} ({label})")
                })
                .collect();
            rows.push(Conclusion {
                name: proposition.name.clone(),
                status: "contested",
                why: mixed.join(", "),
            });
        }
    }

    let mut undecided: Vec<String> = prog
        .of_kind("warrant")
        .iter()
        .filter(|warrant| {
            !inside.contains(&warrant.name) && !outside.contains(&warrant.name)
        })
        .map(|warrant| warrant.name.clone())
        .collect();
    undecided.sort();
    for name in undecided {
        rows.push(Conclusion {
            name,
            status: "undecided",
            why: "rule neither survives nor falls".to_string(),
        });
    }
    rows
}

/// The status of each named item, for the analyses that only need the words.
pub fn statuses(prog: &Program) -> HashMap<String, &'static str> {
    conclusions(prog).into_iter().map(|row| (row.name, row.status)).collect()
}

pub fn premises_of(prog: &Program, warrant: &str) -> Vec<String> {
    prog.get(warrant).map(|decl| field_ids(decl, "from")).unwrap_or_default()
}
