//! Coverage: does the document answer the brief, or only talk near it?
//!
//! The type checker asks whether a declaration is well formed.  It cannot ask whether the
//! document *says anything*, because that is a relation between two artifacts: the brief
//! and the program.  So coverage takes both.
//!
//! A goal counts as addressed only by a declaration that makes a commitment: a value, a
//! question, a hypothesis, a concept, or a proposition.  Attributes never count.

use crate::check::Diagnostic;
use crate::program::Program;
use crate::syntax::{Decl, Val};

pub const CLAIM_KINDS: &[&str] =
    &["value", "question", "hypothesis", "concept", "proposition", "blindspot"];

/// Goal file: one goal per line, `name<TAB>keyword,keyword,...`.
pub fn load_goals(text: &str) -> Vec<(String, Vec<String>)> {
    let mut goals = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, keywords) = match line.split_once('\t') {
            Some((name, rest)) => (name, rest),
            None => (line, ""),
        };
        let groups: Vec<String> = keywords
            .split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect();
        goals.push((name.trim().to_string(), groups));
    }
    goals
}

fn collect(value: &Val, out: &mut Vec<String>) {
    match value {
        Val::Str(text) => out.push(text.clone()),
        Val::List(items) => items.iter().for_each(|item| collect(item, out)),
        Val::Rec(fields) => fields.iter().for_each(|(_, item)| collect(item, out)),
        _ => {}
    }
}

pub fn strings(decl: &Decl) -> Vec<String> {
    let mut out = Vec::new();
    for (key, value) in &decl.fields {
        if key == "attrs" {
            continue;
        }
        collect(value, &mut out);
    }
    out
}

pub struct Goal {
    pub name: String,
    pub hits: Vec<String>,
}

impl Goal {
    pub fn covered(&self) -> bool {
        !self.hits.is_empty()
    }
}

pub fn coverage(prog: &Program, goals: &[(String, Vec<String>)]) -> Vec<Goal> {
    let claims: Vec<&Decl> = prog
        .decls
        .iter()
        .filter(|decl| CLAIM_KINDS.contains(&decl.kind.as_str()))
        .collect();
    let mut out = Vec::new();
    for (name, keywords) in goals {
        let mut hits = Vec::new();
        for decl in &claims {
            let text = strings(decl).join(" ").to_lowercase();
            if keywords.iter().any(|keyword| text.contains(&keyword.to_lowercase())) {
                hits.push(decl.name.clone());
            }
        }
        out.push(Goal { name: name.clone(), hits });
    }
    out
}

pub fn diagnostics(goals: &[Goal]) -> Vec<Diagnostic> {
    let missing: Vec<String> =
        goals.iter().filter(|goal| !goal.covered()).map(|goal| goal.name.clone()).collect();
    if missing.is_empty() {
        return Vec::new();
    }
    vec![Diagnostic {
        code: "E-UNCOVERED",
        message: format!(
            "{} of {} goals are addressed by no value or question: {}",
            missing.len(),
            goals.len(),
            missing.join(", ")
        ),
        line: 1,
        col: 1,
        file: String::new(),
    }]
}
