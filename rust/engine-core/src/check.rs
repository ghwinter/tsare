//! Record, type, duplicate and reference checks.

use std::collections::{HashMap, HashSet};

use crate::specs::spec;
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
    pub line: usize,
    pub col: usize,
}

fn diag(code: &'static str, message: String, decl: &Decl) -> Diagnostic {
    Diagnostic { code, message, line: decl.line, col: decl.col }
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

fn is_instrument(value: &Val) -> bool {
    match value {
        Val::Str(_) => true,
        Val::Call(name, args) => {
            (name == "tool" || name == "doc") && args.len() == 1 && matches!(args[0], Val::Str(_))
        }
        _ => false,
    }
}

fn is_evidence(value: &Val) -> bool {
    match value {
        Val::Id(name) => ["user", "model", "observation"].contains(&name.as_str()),
        Val::Call(name, args) => {
            (name == "tool" || name == "doc" || name == "code")
                && args.len() == 1
                && matches!(args[0], Val::Str(_))
        }
        _ => false,
    }
}

fn type_ok(ftype: &str, value: &Val) -> bool {
    if matches!(value, Val::Hole) {
        return true;
    }
    match ftype {
        "str" => matches!(value, Val::Str(_)),
        "num" => matches!(value, Val::Num(_)),
        "attrs" | "rec:any" => matches!(value, Val::Rec(_)),
        "instrument" => is_instrument(value),
        "evidence" => is_evidence(value),
        _ if ftype.starts_with("enum:") => match value {
            Val::Id(name) => ftype[5..].split(',').any(|option| option == name),
            _ => false,
        },
        _ if ftype.starts_with("list:") => matches!(value, Val::List(_)),
        _ if ftype.starts_with("ref:") => matches!(value, Val::Id(_)),
        _ => true,
    }
}

fn check_record(decl: &Decl, out: &mut Vec<Diagnostic>) {
    let Some(fields) = spec(&decl.kind) else {
        out.push(diag("E-KIND-UNKNOWN", format!("unknown kind {}", decl.kind), decl));
        return;
    };
    for (key, value) in &decl.fields {
        match fields.iter().find(|(name, _, _)| name == key) {
            Some((_, ftype, _)) => {
                if !type_ok(ftype, value) {
                    out.push(diag("E-TYPE", format!("{}.{key}: expected {ftype}", decl.name), decl));
                }
            }
            None => out.push(diag("E-FIELD", format!("{} has no field {key}", decl.kind), decl)),
        }
    }
    for &(name, _, required) in fields {
        if required && !decl.fields.iter().any(|(key, _)| key == name) {
            out.push(diag("E-MISSING", format!("{} needs {name}", decl.name), decl));
        }
    }
}

pub fn check(decls: &[Decl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut by_name: HashMap<&str, &str> = HashMap::new();
    for decl in decls {
        if by_name.contains_key(decl.name.as_str()) {
            out.push(diag("E-DUP-ID", format!("{} is declared twice", decl.name), decl));
        } else {
            by_name.insert(decl.name.as_str(), decl.kind.as_str());
        }
    }

    for decl in decls {
        check_record(decl, &mut out);
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
                    out.push(diag("E-UNDEFINED", format!("{}.{key} refers to {name}", decl.name), decl));
                }
            }
        }
    }
    out
}
