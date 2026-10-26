//! Grounding closure: a rule only derives its conclusion when every premise is derived.
//!
//! `argue` answers "which rules survive attack".  It does not ask whether a premise is itself
//! grounded, so a proposition can be called supported while leaning on another proposition
//! that nothing supports.  This module computes the least set built from facts and rules whose
//! premises are already in that set, then compares the two answers.

use std::collections::{HashMap, HashSet};

use crate::argue::{conclusions, statuses};
use crate::closure::{applications, grounded};
use crate::program::Program;

pub struct Analysis {
    pub facts: Vec<String>,
    pub grounded: Vec<String>,
    pub conditional: Vec<String>,
    pub reasons: HashMap<String, Vec<String>>,
}

pub fn analyze(prog: &Program) -> Analysis {
    let inside = grounded(prog.decls);
    let mut facts: Vec<String> =
        prog.of_kind("fact").iter().map(|decl| decl.name.clone()).collect();
    facts.sort();
    let mut grounded_names: HashSet<String> = facts.iter().cloned().collect();
    let rules: Vec<crate::closure::Rule> = applications(prog.decls)
        .into_iter()
        .filter(|rule| inside.contains(&rule.warrant))
        .collect();

    let mut applied: HashSet<(String, String)> = HashSet::new();
    let mut changed = true;
    while changed {
        changed = false;
        for rule in &rules {
            if applied.contains(&(rule.warrant.clone(), rule.target.clone())) {
                continue;
            }
            if !rule.premises.is_empty()
                && rule.premises.iter().all(|premise| grounded_names.contains(premise))
            {
                applied.insert((rule.warrant.clone(), rule.target.clone()));
                if !grounded_names.contains(&rule.target) {
                    grounded_names.insert(rule.target.clone());
                    changed = true;
                }
            }
        }
    }

    let state = statuses(prog);
    let mut supported: Vec<String> = state
        .iter()
        .filter(|(_, status)| **status == "supported")
        .map(|(name, _)| name.clone())
        .collect();
    supported.sort();

    let fact_set: HashSet<String> = facts.iter().cloned().collect();
    let mut derived: Vec<String> = grounded_names.difference(&fact_set).cloned().collect();
    derived.sort();
    let mut conditional: Vec<String> =
        supported.iter().filter(|name| !grounded_names.contains(*name)).cloned().collect();
    conditional.sort();

    let mut reasons: HashMap<String, Vec<String>> = HashMap::new();
    for name in &conditional {
        let mut missing: Vec<String> = Vec::new();
        for rule in rules.iter().filter(|rule| &rule.target == name) {
            for premise in &rule.premises {
                if !grounded_names.contains(premise) {
                    let status = state.get(premise).copied().unwrap_or("fact");
                    missing.push(format!("{} needs {premise} ({status})", rule.warrant));
                }
            }
        }
        if missing.is_empty() {
            missing.push("no rule reached the facts".to_string());
        }
        reasons.insert(name.clone(), missing);
    }

    Analysis { facts, grounded: derived, conditional, reasons }
}

/// Kept for `check`, which only needs the warning text.
pub fn conditional_reasons(prog: &Program) -> Vec<(String, String)> {
    let analysis = analyze(prog);
    analysis
        .conditional
        .iter()
        .map(|name| {
            let reasons = analysis.reasons.get(name).cloned().unwrap_or_default();
            let shown: Vec<String> = reasons.iter().take(2).cloned().collect();
            (name.clone(), shown.join("; "))
        })
        .collect()
}

/// The status words `whatif` prints, restricted to propositions.
pub fn proposition_statuses(prog: &Program) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for row in conclusions(prog) {
        if prog.get(&row.name).map(|decl| decl.kind == "proposition").unwrap_or(false) {
            out.insert(row.name, row.status.to_string());
        }
    }
    out
}
