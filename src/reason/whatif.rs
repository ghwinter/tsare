//! Counterfactual: remove one assumption and see which conclusions stop standing.
//!
//! Only facts and warrants can be removed; they are referenced from lists, so the edit is
//! `strip every list entry that names it`.  A scalar anchor (a warrant's `to`, a hypothesis's
//! `rival`) is reported instead of silently dangling.
//!
//! This is the mechanical form of "which axiom is load-bearing".

use std::collections::HashSet;

use crate::check::check;
use crate::derive::proposition_statuses;
use crate::program::Program;
use crate::syntax::{Decl, Val};

pub fn strip(value: &mut Val, name: &str) {
    if let Val::List(items) = value {
        items.retain(|item| !matches!(item, Val::Id(text) if text == name));
    }
}

pub fn remove(decls: &[Decl], name: &str) -> Result<Vec<Decl>, String> {
    let Some(target) = decls.iter().find(|decl| decl.name == name) else {
        return Err(format!("{name} is not declared"));
    };
    if target.kind != "fact" && target.kind != "warrant" {
        return Err(format!(
            "refusing to remove a {}: only facts and warrants are assumptions",
            target.kind
        ));
    }
    let mut out = Vec::new();
    for decl in decls {
        if decl.name == name {
            continue;
        }
        let mut copy = decl.clone();
        for (_, value) in copy.fields.iter_mut() {
            strip(value, name);
        }
        out.push(copy);
    }
    Ok(out)
}

pub struct Counterfactual {
    pub errors_before: usize,
    pub errors_after: usize,
    pub warnings_before: usize,
    pub warnings_after: usize,
    pub new_codes: Vec<String>,
    pub lost_codes: Vec<String>,
    pub changed: Vec<(String, String, String)>,
    pub newly_asserted: Vec<String>,
}

pub fn compare(before: &[Decl], after: &[Decl]) -> Counterfactual {
    let diagnostics_before = check(before);
    let diagnostics_after = check(after);
    let errors_before = diagnostics_before
        .iter()
        .filter(|item| item.code.starts_with("E-"))
        .count();
    let errors_after = diagnostics_after
        .iter()
        .filter(|item| item.code.starts_with("E-"))
        .count();
    let warnings_before = diagnostics_before
        .iter()
        .filter(|item| item.code.starts_with("W-"))
        .count();
    let warnings_after = diagnostics_after
        .iter()
        .filter(|item| item.code.starts_with("W-"))
        .count();
    let codes_before: HashSet<&str> = diagnostics_before.iter().map(|item| item.code).collect();
    let codes_after: HashSet<&str> = diagnostics_after.iter().map(|item| item.code).collect();
    let mut new_codes: Vec<String> = codes_after
        .difference(&codes_before)
        .map(|code| code.to_string())
        .collect();
    let mut lost_codes: Vec<String> = codes_before
        .difference(&codes_after)
        .map(|code| code.to_string())
        .collect();
    new_codes.sort();
    lost_codes.sort();

    let status_before = proposition_statuses(&Program::new(before));
    let status_after = proposition_statuses(&Program::new(after));
    let mut changed = Vec::new();
    for (name, status) in &status_before {
        let now = status_after
            .get(name)
            .cloned()
            .unwrap_or_else(|| "gone".to_string());
        if &now != status {
            changed.push((name.clone(), status.clone(), now));
        }
    }
    changed.sort();
    let mut newly_asserted: Vec<String> = status_after
        .iter()
        .filter(|(name, status)| {
            **status == "asserted"
                && status_before
                    .get(*name)
                    .map(|old| old != "asserted")
                    .unwrap_or(true)
        })
        .map(|(name, _)| name.clone())
        .collect();
    newly_asserted.sort();

    Counterfactual {
        errors_before,
        errors_after,
        warnings_before,
        warnings_after,
        new_codes,
        lost_codes,
        changed,
        newly_asserted,
    }
}
