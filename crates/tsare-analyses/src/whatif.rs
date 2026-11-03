//! Counterfactual: remove one assumption and see which conclusions stop standing.
//!
//! Only facts and warrants can be removed; they are referenced from lists, so the edit is
//! `strip every list entry that names it`.  A scalar anchor (a warrant's `to`, a hypothesis's
//! `rival`) is reported instead of silently dangling.
//!
//! This is the mechanical form of "which axiom is load-bearing".

use std::collections::HashSet;

use crate::derive::proposition_statuses;
use tsare_core::check;
use tsare_lang::json::Json as J;
use tsare_lang::program::Program;
use tsare_lang::syntax::{Decl, Val};

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

/// The record of a retraction: the assumption removed, and the conclusions whose status changed.
pub struct RetractReport {
    pub name: String,
    pub counterfactual: Counterfactual,
}

impl RetractReport {
    pub fn text(&self) -> String {
        let result = &self.counterfactual;
        let mut out = format!("retracted {}:\n", self.name);
        out.push_str(&format!(
            "  errors {} -> {}, warnings {} -> {}\n",
            result.errors_before,
            result.errors_after,
            result.warnings_before,
            result.warnings_after
        ));
        if !result.new_codes.is_empty() {
            out.push_str(&format!("  new codes: {}\n", result.new_codes.join(", ")));
        }
        if !result.lost_codes.is_empty() {
            out.push_str(&format!("  lost codes: {}\n", result.lost_codes.join(", ")));
        }
        if result.changed.is_empty() {
            out.push_str("  no proposition changed status\n");
        } else {
            for (name, before, after) in &result.changed {
                out.push_str(&format!("  {name}: {before} -> {after}\n"));
            }
        }
        if !result.newly_asserted.is_empty() {
            out.push_str(&format!(
                "  newly asserted: {}\n",
                result.newly_asserted.join(", ")
            ));
        }
        out
    }

    pub fn json(&self) -> J {
        let result = &self.counterfactual;
        let dependents: Vec<J> = result
            .changed
            .iter()
            .map(|(name, before, after)| {
                J::Obj(vec![
                    ("name".to_string(), J::Str(name.clone())),
                    ("before".to_string(), J::Str(before.clone())),
                    ("after".to_string(), J::Str(after.clone())),
                ])
            })
            .collect();
        let strings =
            |items: &[String]| J::Arr(items.iter().map(|item| J::Str(item.clone())).collect());
        J::Obj(vec![
            ("retracted".to_string(), J::Str(self.name.clone())),
            ("dependents".to_string(), J::Arr(dependents)),
            (
                "newly_asserted".to_string(),
                strings(&result.newly_asserted),
            ),
            ("new_codes".to_string(), strings(&result.new_codes)),
            ("lost_codes".to_string(), strings(&result.lost_codes)),
            (
                "errors_before".to_string(),
                J::Num(result.errors_before as f64),
            ),
            (
                "errors_after".to_string(),
                J::Num(result.errors_after as f64),
            ),
            (
                "warnings_before".to_string(),
                J::Num(result.warnings_before as f64),
            ),
            (
                "warnings_after".to_string(),
                J::Num(result.warnings_after as f64),
            ),
        ])
    }
}

impl tsare_core::out::Report for RetractReport {
    fn text(&self) -> String {
        RetractReport::text(self)
    }

    fn json(&self) -> J {
        RetractReport::json(self)
    }
}
