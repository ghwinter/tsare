//! A quantitative layer that cannot break decidability.
//!
//! This is not probability.  A document states premises and rules; it does not state the
//! independence assumptions a probability needs.  So the number propagated here is a lower
//! bound under "weakest link":
//!
//! ```text
//!     strength(rule)        = min(rule.confidence, min(strength of its premises))
//!     strength(proposition) = max(strength of rules concluding it), or its own number if asserted
//! ```
//!
//! `min` and `max` over a finite set are decidable and monotone.  Sums, products and
//! conditional probabilities are deliberately absent: they would require a joint distribution
//! the document never declares.

use crate::closure::grounded;
use crate::program::{f_ids, f_num, has_field, Program};
use std::collections::{HashMap, HashSet};

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
    let propositions = prog.of_kind("proposition");
    // A warrant reaches a proposition by `to:` or by belonging to the class `concludes:`
    // names; both are in closure::applications, and weighing must read the same list.
    let mut rules_of: HashMap<String, Vec<&crate::syntax::Decl>> = HashMap::new();
    for rule in crate::closure::applications(prog.decls) {
        if let Some(warrant) = prog.get(&rule.warrant) {
            rules_of.entry(rule.target).or_default().push(warrant);
        }
    }
    let live: HashSet<&str> = propositions
        .iter()
        .filter(|proposition| {
            rules_of
                .get(&proposition.name)
                .map(|rules| rules.iter().any(|rule| inside.contains(&rule.name)))
                .unwrap_or(false)
        })
        .map(|proposition| proposition.name.as_str())
        .collect();

    // A proposition premise contributes its own weight, and the weights form a fixpoint: the
    // value of a derived proposition is the weakest link of the rules that conclude it.  A
    // proposition outside the live set contributes its declared number (or 1.0), because the
    // document states the assumption and nothing derives it.
    let premise = |name: &str, current: &HashMap<String, f64>| -> f64 {
        match prog.get(name) {
            Some(decl) if decl.kind == "fact" => confidence(decl, 1.0),
            Some(decl) if decl.kind == "proposition" => {
                if live.contains(decl.name.as_str()) {
                    current.get(name).copied().unwrap_or(1.0)
                } else {
                    confidence(decl, 1.0)
                }
            }
            _ => 1.0,
        }
    };
    let rule_value = |rule: &crate::syntax::Decl, current: &HashMap<String, f64>| -> f64 {
        let mut premises = f_ids(rule, "from");
        premises.extend(f_ids(rule, "basis"));
        let floor = premises
            .iter()
            .map(|name| premise(name, current))
            .fold(1.0f64, f64::min);
        confidence(rule, 1.0).min(floor)
    };

    let mut strength: HashMap<String, f64> = propositions
        .iter()
        .map(|proposition| (proposition.name.clone(), 1.0))
        .collect();
    // Values only fall, so a document without support cycles settles in at most one pass per
    // proposition.  A cycle is already E-CYCLE; the cap keeps weigh terminating on one.
    let mut rounds = 0;
    loop {
        let mut changed = false;
        for proposition in propositions
            .iter()
            .filter(|proposition| live.contains(proposition.name.as_str()))
        {
            let rules = rules_of
                .get(&proposition.name)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let best = rules
                .iter()
                .filter(|rule| inside.contains(&rule.name))
                .map(|rule| rule_value(rule, &strength))
                .fold(f64::MIN, f64::max);
            let now = strength[&proposition.name];
            if best < now - 1e-12 {
                strength.insert(proposition.name.clone(), best);
                changed = true;
            }
        }
        rounds += 1;
        if !changed || rounds > propositions.len() + 1 {
            break;
        }
    }

    let mut rows = Vec::new();
    for proposition in propositions {
        let rules: Vec<&crate::syntax::Decl> =
            rules_of.get(&proposition.name).cloned().unwrap_or_default();
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
        let admissible: Vec<&&crate::syntax::Decl> = rules
            .iter()
            .filter(|rule| inside.contains(&rule.name))
            .collect();
        if admissible.is_empty() {
            rows.push(Weight {
                name: proposition.name.clone(),
                value: None,
                status: "defeated",
                detail: rules.iter().map(|rule| (rule.name.clone(), 0.0)).collect(),
            });
            continue;
        }
        let per_rule: Vec<(String, f64)> = admissible
            .iter()
            .map(|rule| (rule.name.clone(), rule_value(rule, &strength)))
            .collect();
        rows.push(Weight {
            name: proposition.name.clone(),
            value: Some(strength[&proposition.name]),
            status: "derived",
            detail: per_rule,
        });
    }
    rows
}
