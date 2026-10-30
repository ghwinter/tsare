//! Whether a document is closed, closable, or open.
//!
//! Three words, one reading of the ledger.  A declaration is **closed** when the witness its kind
//! names is recorded at the grade the kind demands.  It is **closable** when the shape is
//! unfulfilled but the document names what could fulfil it: a command, a person, a question, or a
//! declaration that is itself closable.  It is **open** otherwise.  A retired declaration and a
//! note count for nothing; truth is not a question this module asks.

use std::collections::HashSet;

use crate::closure;
use crate::program::{f_id, f_ids, f_num, f_str, grade, instrument_of, Grade};
use crate::syntax::Decl;

/// How far a declaration is from being settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// the witness is in
    Closed,
    /// a witness is missing, and something in the document names what would produce it
    Closable,
    /// a witness is missing, and nothing names what would produce it
    Open,
}

impl Decision {
    pub fn label(self) -> &'static str {
        match self {
            Decision::Closed => "closed",
            Decision::Closable => "closable",
            Decision::Open => "open",
        }
    }
}

/// One declaration that still owes something.
pub struct Item {
    pub name: String,
    pub kind: String,
    pub decision: Decision,
    pub reason: &'static str,
}

/// The document's decision, and the declarations that keep it from being closed.
pub struct Verdict {
    pub decision: Decision,
    pub items: Vec<Item>,
}

/// The document's decision alone.
pub fn decide(decls: &[Decl]) -> Decision {
    report(decls).decision
}

/// Every declaration that still owes a witness, in document order.
pub fn report(decls: &[Decl]) -> Verdict {
    let gone = closure::retired(decls);
    let concluded: HashSet<String> = closure::applications(decls)
        .into_iter()
        .filter(|rule| !gone.contains(&rule.warrant))
        .map(|rule| rule.target)
        .collect();
    let mut items = Vec::new();
    for decl in decls {
        if decl.kind == "note" || gone.contains(&decl.name) {
            continue;
        }
        let (decision, reason) = judge(decl, &concluded, &gone);
        if decision != Decision::Closed {
            items.push(Item {
                name: decl.name.clone(),
                kind: decl.kind.clone(),
                decision,
                reason,
            });
        }
    }
    let decision = if items.iter().any(|item| item.decision == Decision::Open) {
        Decision::Open
    } else if items.iter().any(|item| item.decision == Decision::Closable) {
        Decision::Closable
    } else {
        Decision::Closed
    };
    Verdict { decision, items }
}

/// The witnesses a retired declaration can no longer provide.
fn live(names: Vec<String>, gone: &HashSet<String>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| !gone.contains(name))
        .collect()
}

fn judge(
    decl: &Decl,
    concluded: &HashSet<String>,
    gone: &HashSet<String>,
) -> (Decision, &'static str) {
    match decl.kind.as_str() {
        "fact" => match grade(decl) {
            Grade::Measured | Grade::Confirmed => (Decision::Closed, "the witness is recorded"),
            Grade::ModelSupplied => (Decision::Open, "a model's word, with no confirmation named"),
            Grade::Asserted => (Decision::Open, "no witness named"),
        },
        "concept" => {
            if live(f_ids(decl, "grounded_by"), gone).is_empty() {
                (Decision::Open, "no positive example")
            } else if live(f_ids(decl, "against"), gone).is_empty() {
                (Decision::Open, "no counterexample named")
            } else {
                (Decision::Closed, "examples on both sides")
            }
        }
        "proposition" => {
            if concluded.contains(&decl.name) {
                (Decision::Closed, "a live rule concludes it")
            } else if f_ids(decl, "from").is_empty() {
                (Decision::Open, "no premise and no rule")
            } else {
                (Decision::Open, "premises, but no live rule connects them")
            }
        }
        "hypothesis" => {
            let verdict = f_id(decl, "verdict").unwrap_or("open");
            if verdict != "supported" && verdict != "refuted" {
                let test = f_str(decl, "test").is_some_and(|text| !text.trim().is_empty());
                let rival = f_id(decl, "rival").is_some_and(|name| !gone.contains(name));
                if test || rival {
                    (Decision::Closable, "a test or a rival is named")
                } else {
                    (Decision::Open, "no test and no rival")
                }
            } else if f_id(decl, "verdict_by").is_some_and(|name| !gone.contains(name)) {
                (Decision::Closed, "the deciding fact is named")
            } else {
                (Decision::Open, "a verdict with no deciding fact")
            }
        }
        "question" => {
            if f_id(decl, "to").unwrap_or("user") == "user" {
                (Decision::Closable, "the user is named")
            } else {
                (Decision::Open, "no respondent is named")
            }
        }
        "blindspot" => {
            if live(f_ids(decl, "closes_with"), gone).is_empty() {
                (Decision::Open, "no question would close it")
            } else {
                (Decision::Closable, "the closing question is named")
            }
        }
        "value" => {
            if instrument_of(decl, "measured_by").is_none() {
                (Decision::Open, "no instrument")
            } else if f_num(decl, "threshold").is_none() {
                (Decision::Open, "no threshold")
            } else if f_str(decl, "unit").is_none() {
                (Decision::Open, "no unit")
            } else {
                (
                    Decision::Closed,
                    "the instrument, the threshold and the unit are named",
                )
            }
        }
        "warrant" => {
            if live(f_ids(decl, "basis"), gone).is_empty() {
                (Decision::Open, "no basis")
            } else {
                (Decision::Closed, "the basis is named")
            }
        }
        "law" => {
            if live(f_ids(decl, "checked_by"), gone).is_empty() {
                (Decision::Open, "no checker")
            } else {
                (Decision::Closed, "the checker is named")
            }
        }
        "layer" => {
            if f_ids(decl, "exposes").is_empty() {
                (Decision::Open, "exposes nothing")
            } else {
                (Decision::Closed, "the interface is the witness")
            }
        }
        "partition" => {
            let state = f_id(decl, "closure").unwrap_or("unknown");
            let dimension = f_id(decl, "dimension").is_some_and(|name| !gone.contains(name));
            let closes = live(f_ids(decl, "closes_with"), gone);
            match state {
                "closed" if !dimension => (Decision::Open, "closed with no dimension"),
                "closed" if live(f_ids(decl, "basis"), gone).is_empty() => {
                    (Decision::Open, "closed with no basis")
                }
                "closed" => (Decision::Closed, "the dimension and the basis are named"),
                _ if !closes.is_empty() => (
                    Decision::Closable,
                    "the question that would close it is named",
                ),
                "unknown" => (Decision::Open, "no dimension and no question"),
                _ => (Decision::Open, "open, and no question closes it"),
            }
        }
        _ => (Decision::Open, "unknown kind"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    fn decision(text: &str) -> Decision {
        decide(&parse(text).expect("the document parses"))
    }

    #[test]
    fn a_fact_is_closed_only_when_a_witness_stands_behind_it() {
        assert_eq!(
            decision("fact F { claim: \"x\", evidence: user }"),
            Decision::Closed
        );
        assert_eq!(
            decision("fact F { claim: \"x\", evidence: model }"),
            Decision::Open
        );
        assert_eq!(
            decision("fact F { claim: \"x\", evidence: model, confirmed_by: \"p\" }"),
            Decision::Closed
        );
    }

    #[test]
    fn a_concept_needs_a_counterexample() {
        let only_positive = "fact F { claim: \"x\", evidence: user }\n\
                             concept C { name: \"c\", grounded_by: [F] }";
        assert_eq!(decision(only_positive), Decision::Open);
        let both = "fact F { claim: \"x\", evidence: user }\n\
                    fact G { claim: \"y\", evidence: user }\n\
                    concept C { name: \"c\", grounded_by: [F], against: [G] }";
        assert_eq!(decision(both), Decision::Closed);
    }

    #[test]
    fn a_test_makes_a_hypothesis_closable_and_a_verdict_closes_it() {
        let base = "fact F { claim: \"x\", evidence: user }\n";
        let tested =
            format!("{base}hypothesis H {{ assumes: [F], predicts: \"p\", test: \"cargo test\" }}");
        assert_eq!(decision(&tested), Decision::Closable);
        let decided = format!(
            "{base}hypothesis H {{ assumes: [F], predicts: \"p\", verdict: supported, verdict_by: F }}"
        );
        assert_eq!(decision(&decided), Decision::Closed);
    }

    #[test]
    fn a_value_needs_an_instrument_a_threshold_and_a_unit() {
        let closed = "value V { metric: \"m\", direction: max, unit: \"ms\", threshold: 1, measured_by: \"trace\" }";
        assert_eq!(decision(closed), Decision::Closed);
        let open = "value V { metric: \"m\", direction: max, unit: \"ms\", threshold: 1 }";
        assert_eq!(decision(open), Decision::Open);
    }

    #[test]
    fn a_partition_unknown_is_closable_only_with_its_question() {
        let open = "fact F { claim: \"x\", evidence: user }\n\
                    partition P { elements: [F], closure: unknown }";
        assert_eq!(decision(open), Decision::Open);
        let closable = "fact F { claim: \"x\", evidence: user }\n\
                        hypothesis H { assumes: [F], predicts: \"p\", test: \"t\" }\n\
                        hypothesis I { assumes: [F], predicts: \"q\", test: \"t\" }\n\
                        question Q { asks: \"which?\", answers: [\"p\", \"q\"], separates: [H, I] }\n\
                        partition P { elements: [F], closure: unknown, closes_with: [Q] }";
        assert_eq!(decision(closable), Decision::Closable);
    }
}
