//! How a conclusion is known: verified, inferred, or unverified.
//!
//! The witness order asks who stands behind a claim; this asks how the claim got here.  A fact
//! with a machine or a person behind it is verified; a proposition a live rule concludes, or a
//! hypothesis a fact decided, is inferred; everything else is unverified.  A definition is not a
//! conclusion and has no row; a retired declaration counts for nothing.

use std::collections::HashMap;

use crate::closure;
use crate::program::{f_id, f_ids, grade, Grade};
use crate::syntax::Decl;

/// The three states of a conclusion, as the judgement-level projection of a finer field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Knowing {
    /// a machine or a person stands behind it
    Verified,
    /// a live rule or a deciding fact derives it
    Inferred,
    /// nothing recorded derives it, or only a model's word does
    Unverified,
}

impl Knowing {
    pub fn label(self) -> &'static str {
        match self {
            Knowing::Verified => "verified",
            Knowing::Inferred => "inferred",
            Knowing::Unverified => "unverified",
        }
    }
}

/// One conclusion and how it is known.
pub struct Row {
    pub name: String,
    pub kind: String,
    pub knowing: Knowing,
    /// The rule, fact or witness behind the state, when there is one.
    pub basis: Option<String>,
}

/// Every conclusion in the document, in declaration order.
pub fn report(decls: &[Decl]) -> Vec<Row> {
    let gone = closure::retired(decls);
    let concluded: HashMap<String, String> = closure::applications(decls)
        .into_iter()
        .filter(|rule| !gone.contains(&rule.warrant))
        .map(|rule| (rule.target, rule.warrant))
        .collect();
    let mut rows = Vec::new();
    for decl in decls {
        if gone.contains(&decl.name) {
            continue;
        }
        let (knowing, basis) = match decl.kind.as_str() {
            "fact" => match grade(decl) {
                Grade::Measured | Grade::Confirmed => {
                    (Knowing::Verified, Some(grade(decl).label().to_string()))
                }
                Grade::ModelSupplied | Grade::Asserted => {
                    (Knowing::Unverified, Some(grade(decl).label().to_string()))
                }
            },
            "proposition" => match concluded.get(&decl.name) {
                Some(warrant) => (Knowing::Inferred, Some(warrant.clone())),
                None => (Knowing::Unverified, None),
            },
            "hypothesis" => {
                let verdict = f_id(decl, "verdict").unwrap_or("open");
                let decided = if verdict == "supported" || verdict == "refuted" {
                    f_id(decl, "verdict_by").filter(|name| !gone.contains(*name))
                } else {
                    None
                };
                match decided {
                    Some(fact) => (Knowing::Inferred, Some(fact.to_string())),
                    None => (Knowing::Unverified, None),
                }
            }
            "concept" => {
                let positives = f_ids(decl, "grounded_by")
                    .into_iter()
                    .filter(|name| !gone.contains(name))
                    .count();
                let negatives = f_ids(decl, "against")
                    .into_iter()
                    .filter(|name| !gone.contains(name))
                    .count();
                if positives > 0 && negatives > 0 {
                    (
                        Knowing::Verified,
                        Some("examples on both sides".to_string()),
                    )
                } else {
                    (Knowing::Unverified, None)
                }
            }
            _ => continue,
        };
        rows.push(Row {
            name: decl.name.clone(),
            kind: decl.kind.clone(),
            knowing,
            basis,
        });
    }
    rows
}

/// The projection as one block of text: the counts, then one row per conclusion.
pub fn text(rows: &[Row]) -> String {
    let verified = rows
        .iter()
        .filter(|row| row.knowing == Knowing::Verified)
        .count();
    let inferred = rows
        .iter()
        .filter(|row| row.knowing == Knowing::Inferred)
        .count();
    let unverified = rows.len() - verified - inferred;
    let mut out =
        format!("epistemic: {verified} verified, {inferred} inferred, {unverified} unverified\n");
    for row in rows {
        let line = format!(
            "{:<20} {:<12} {:<11}",
            row.name,
            row.kind,
            row.knowing.label()
        );
        match &row.basis {
            Some(basis) => out.push_str(&format!("{line} {basis}\n")),
            None => out.push_str(&format!("{line}\n")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    fn rows(text: &str) -> Vec<Row> {
        report(&parse(text).expect("the document parses"))
    }

    fn knowing(text: &str, name: &str) -> Knowing {
        rows(text)
            .into_iter()
            .find(|row| row.name == name)
            .expect("a row for the declaration")
            .knowing
    }

    #[test]
    fn a_fact_is_verified_only_with_a_witness() {
        assert_eq!(
            knowing("fact F { claim: \"x\", evidence: user }", "F"),
            Knowing::Verified
        );
        assert_eq!(
            knowing("fact F { claim: \"x\", evidence: model }", "F"),
            Knowing::Unverified
        );
    }

    #[test]
    fn a_rule_makes_a_proposition_inferred() {
        let text = "fact F { claim: \"x\", evidence: user }\n\
                    proposition P { statement: \"s\", from: [F] }\n\
                    warrant W { claim: \"c\", from: [F], to: P, basis: [F] }";
        assert_eq!(knowing(text, "P"), Knowing::Inferred);
        let row = rows(text)
            .into_iter()
            .find(|row| row.name == "P")
            .expect("a row for P");
        assert_eq!(row.basis.as_deref(), Some("W"));
    }

    #[test]
    fn a_verdict_makes_a_hypothesis_inferred() {
        let text = "fact F { claim: \"x\", evidence: user }\n\
                    hypothesis H { assumes: [F], predicts: \"p\", verdict: supported, verdict_by: F }";
        assert_eq!(knowing(text, "H"), Knowing::Inferred);
    }

    #[test]
    fn a_concept_is_verified_with_examples_on_both_sides() {
        let text = "fact F { claim: \"x\", evidence: user }\n\
                    fact G { claim: \"y\", evidence: user }\n\
                    concept C { name: \"c\", grounded_by: [F] }";
        assert_eq!(knowing(text, "C"), Knowing::Unverified);
        let both = format!("{text}\nconcept D {{ name: \"d\", grounded_by: [F], against: [G] }}");
        assert_eq!(knowing(&both, "D"), Knowing::Verified);
    }

    #[test]
    fn a_note_is_not_a_conclusion() {
        assert!(rows("note N { tag: conjecture, text: \"x\" }").is_empty());
    }
}
