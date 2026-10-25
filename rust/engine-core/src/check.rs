//! The checks that do not need the graph yet: duplicate names and unknown references.

use std::collections::{HashMap, HashSet};

use crate::syntax::{Decl, Val};

pub const REF_KEYS: &[&str] = &[
    "from", "to", "grounded_by", "against", "uses", "assumes", "answers", "separates",
    "rival", "witness", "basis", "attacks", "checked_by", "holds_in", "exposes",
    "closes_with", "parent", "questions",
];

/// Values that name a procedure rather than a declaration.
pub const LITERALS: &[&str] = &["tool", "doc", "code", "user", "model", "observation", "experiment"];

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
}

pub fn refs(value: &Val, out: &mut Vec<String>) {
    match value {
        Val::Id(name) => out.push(name.clone()),
        Val::List(items) => items.iter().for_each(|item| refs(item, out)),
        Val::Call(_, args) => args.iter().for_each(|item| refs(item, out)),
        Val::Rec(fields) => fields.iter().for_each(|(_, item)| refs(item, out)),
        _ => {}
    }
}

pub fn check(decls: &[Decl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut by_name: HashMap<&str, &str> = HashMap::new();
    for decl in decls {
        if by_name.contains_key(decl.name.as_str()) {
            out.push(Diagnostic {
                code: "E-DUP-ID",
                message: format!("{} is declared twice", decl.name),
            });
        } else {
            by_name.insert(decl.name.as_str(), decl.kind.as_str());
        }
    }

    let mut seen: HashSet<(String, String)> = HashSet::new();
    for decl in decls {
        for (key, value) in &decl.fields {
            if !REF_KEYS.contains(&key.as_str()) {
                continue;
            }
            let mut found = Vec::new();
            refs(value, &mut found);
            for name in found {
                if LITERALS.contains(&name.as_str()) || by_name.contains_key(name.as_str()) {
                    continue;
                }
                if seen.insert((decl.name.clone(), name.clone())) {
                    out.push(Diagnostic {
                        code: "E-UNDEFINED",
                        message: format!("{}.{key} refers to {name}", decl.name),
                    });
                }
            }
        }
    }
    out
}
