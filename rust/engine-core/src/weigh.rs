//! A quantitative layer that cannot break decidability.
//!
//! This is not probability.  A document states premises and rules; it does not state the
//! independence assumptions a probability needs.  So the number propagated here is a lower
//! bound under "weakest link":
//!
//!     strength(rule)        = min(rule.confidence, min(strength of its premises))
//!     strength(proposition) = max(strength of rules concluding it), or its own number if asserted
//!
//! `min` and `max` over a finite set are decidable and monotone.  Sums, products and
//! conditional probabilities are deliberately absent: they would require a joint distribution
//! the document never declares.

use crate::closure::grounded;
use crate::program::{f_id, f_ids, f_num, has_field, Program};

pub struct Weight {
    pub name: String,
    pub value: Option<f64>,
    pub status: &'static str,
    pub detail: Vec<(String, f64)>,
}

fn confidence(decl: &crate::syntax::Decl, fallback: f64) -> f64 {
    f_num(decl, "confidence").unwrap_or(fallback)
}

pub fn weights(prog: &Program) -> Vec<Weight> {
    let inside = grounded(prog.decls);
    let mut rows = Vec::new();
    for proposition in prog.of_kind("proposition") {
        let rules: Vec<&crate::syntax::Decl> = prog
            .of_kind("warrant")
            .into_iter()
            .filter(|warrant| f_id(warrant, "to") == Some(proposition.name.as_str()))
            .collect();
        if rules.is_empty() {
            rows.push(Weight {
                name: proposition.name.clone(),
                value: if has_field(proposition, "confidence") {
                    f_num(proposition, "confidence")
                } else {
                    None
                },
                status: "asserted",
                detail: Vec::new(),
            });
            continue;
        }
        let admissible: Vec<&&crate::syntax::Decl> =
            rules.iter().filter(|rule| inside.contains(&rule.name)).collect();
        if admissible.is_empty() {
            rows.push(Weight {
                name: proposition.name.clone(),
                value: None,
                status: "defeated",
                detail: rules.iter().map(|rule| (rule.name.clone(), 0.0)).collect(),
            });
            continue;
        }
        let mut per_rule: Vec<(String, f64)> = Vec::new();
        for rule in admissible {
            let mut premises = f_ids(rule, "from");
            premises.extend(f_ids(rule, "basis"));
            let facts: Vec<&crate::syntax::Decl> = premises
                .iter()
                .filter_map(|name| prog.get(name))
                .filter(|decl| decl.kind == "fact")
                .collect();
            let floor = facts
                .iter()
                .map(|fact| confidence(fact, 1.0))
                .fold(None::<f64>, |low, value| Some(match low {
                    None => value,
                    Some(current) => current.min(value),
                }))
                .unwrap_or(1.0);
            per_rule.push((rule.name.clone(), confidence(rule, 1.0).min(floor)));
        }
        let value = per_rule.iter().map(|(_, value)| *value).fold(f64::MIN, f64::max);
        rows.push(Weight {
            name: proposition.name.clone(),
            value: Some(value),
            status: "derived",
            detail: per_rule,
        });
    }
    rows
}
