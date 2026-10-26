//! Proposals from outside the compiler, verified inside it.
//!
//! A model may fill a hole; it may not decide that the fill is right.  This module takes a
//! declaration, a field, and a value, and answers one question: does the document get worse?
//! That is the same gate `apply` uses on its own repairs, so a model's proposal and a
//! diagnostic's repair are judged by one rule.
//!
//! The gate is a guard, not a judge: a well-typed value that contradicts the document passes.

use crate::apply::{score, splice};
use crate::fix::Suggestion;
use crate::syntax::{Decl, Val};

pub struct Proposal {
    pub decl: String,
    pub field: String,
    pub value: String,
}

/// How much a proposal must earn before it counts as admitted.
#[derive(Clone, Copy, PartialEq)]
pub enum Policy {
    /// No new diagnostic. A hole removed is progress by itself.
    Harmless,
    /// No new diagnostic, and an introduced one is a refusal rather than a question.
    Strict,
    /// No new diagnostic, and something apart from the hole must get better.
    Improve,
}

impl Policy {
    pub fn parse(text: &str) -> Policy {
        match text {
            "strict" => Policy::Strict,
            "improve" => Policy::Improve,
            _ => Policy::Harmless,
        }
    }
}

pub enum Verdict {
    /// Nothing got worse and the hole is gone. `errors` below zero means an error went too.
    Admitted { errors: isize, warnings: isize, text: String },
    /// No error was added, but the fill introduced a diagnostic that was not there before,
    /// or (under `Improve`) it changed nothing except the hole.  A human should look: the
    /// compiler can say "this needs attention", not "which of the two is right".
    NeedsReview { reason: String, text: String },
    Rejected { reason: String },
}

fn code_counts(decls: &[Decl]) -> std::collections::HashMap<&'static str, usize> {
    let mut counts = std::collections::HashMap::new();
    for item in crate::check::check(decls) {
        *counts.entry(item.code).or_insert(0) += 1;
    }
    counts
}

pub fn verify(text: &str, decls: &[Decl], proposal: &Proposal, policy: Policy) -> Verdict {
    let Some(decl) = decls.iter().find(|decl| decl.name == proposal.decl) else {
        return Verdict::Rejected { reason: "no such declaration".to_string() };
    };
    let Some(index) = decl.fields.iter().position(|(name, _)| name == &proposal.field) else {
        return Verdict::Rejected { reason: "no such field".to_string() };
    };
    if !matches!(decl.fields[index].1, Val::Hole) {
        return Verdict::Rejected { reason: "the field is not a hole".to_string() };
    }
    let Some(&(line, col, end_line, end_col)) = decl.field_span.get(index) else {
        return Verdict::Rejected { reason: "the hole has no span".to_string() };
    };

    let suggestion = Suggestion {
        applicability: "MachineApplicable",
        message: format!("{}.{}: {}", proposal.decl, proposal.field, proposal.value),
        file: decl.file.clone(),
        line,
        col,
        end_line,
        end_col,
        replacement: proposal.value.clone(),
    };
    let before = score(text);
    let candidate = splice(text, &suggestion);
    let after = score(&candidate);
    if after.0 == usize::MAX {
        return Verdict::Rejected { reason: "the value does not parse".to_string() };
    }
    let errors = after.0 as isize - before.0 as isize;
    let warnings = after.1 as isize - before.1 as isize;
    if errors > 0 {
        return Verdict::Rejected { reason: format!("adds {errors} error(s)") };
    }
    if candidate == text {
        return Verdict::Rejected { reason: "changes nothing".to_string() };
    }

    let Ok(updated) = crate::syntax::parse(&candidate) else {
        return Verdict::Rejected { reason: "the value does not parse".to_string() };
    };
    let old = code_counts(decls);
    let new = code_counts(&updated);
    let introduced: Vec<&'static str> = new
        .iter()
        .filter(|(code, count)| **count > old.get(*code).copied().unwrap_or(0))
        .map(|(code, _)| *code)
        .collect();
    if !introduced.is_empty() {
        return match policy {
            Policy::Strict => {
                Verdict::Rejected { reason: format!("introduces {}", introduced.join(", ")) }
            }
            _ => Verdict::NeedsReview { reason: format!("introduced {}", introduced.join(", ")), text: candidate },
        };
    }
    if policy == Policy::Improve {
        // something other than the hole has to get better: W-HOLE leaving is not progress
        let better = new
            .iter()
            .any(|(code, count)| *code != "W-HOLE" && *count < old.get(*code).copied().unwrap_or(0));
        if !better {
            return Verdict::NeedsReview { reason: "removes only the hole; nothing else improved".to_string(), text: candidate };
        }
    }
    Verdict::Admitted { errors, warnings, text: candidate }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    const DOC: &str = "fact F { claim: \"x\", evidence: ? }\n";

    fn proposal(field: &str, value: &str) -> Proposal {
        Proposal { decl: "F".to_string(), field: field.to_string(), value: value.to_string() }
    }

    #[test]
    fn a_well_typed_value_is_admitted_and_a_type_error_is_not() {
        let decls = parse(DOC).unwrap();
        assert!(matches!(
            verify(DOC, &decls, &proposal("evidence", "tool(\"t\")"), Policy::Harmless),
            Verdict::Admitted { .. }
        ));
        assert!(matches!(
            verify(DOC, &decls, &proposal("evidence", "nonsense"), Policy::Harmless),
            Verdict::Rejected { .. }
        ));
    }

    #[test]
    fn a_field_that_is_not_a_hole_is_refused() {
        let text = "fact F { claim: \"x\", evidence: user }\n";
        let decls = parse(text).unwrap();
        let verdict = verify(text, &decls, &proposal("evidence", "model"), Policy::Harmless);
        assert!(matches!(verdict, Verdict::Rejected { reason } if reason.contains("not a hole")));
    }

    #[test]
    fn a_value_that_leaves_the_document_unchanged_is_refused() {
        let decls = parse(DOC).unwrap();
        let verdict = verify(DOC, &decls, &proposal("evidence", "?"), Policy::Harmless);
        assert!(matches!(verdict, Verdict::Rejected { reason } if reason.contains("changes nothing")));
    }
}



