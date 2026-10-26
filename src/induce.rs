//! Back end, part 1: learn a concept from its examples, or report why it cannot be learned.
//!
//! The learner is a least-generalisation search over attribute literals:
//!
//! ```text
//!     candidates  key = value  (categorical)   key <= t / key >= t  (numeric, t at observed values)
//!     search      subsets of size 1..3, smallest consistent clause wins
//!     consistent  covers every positive, covers no negative
//! ```
//!
//! When no clause exists, the failure is informative: a positive and a negative with identical
//! attributes is a *collision*, and a collision is exactly a blind spot in the current
//! signature.  The backend turns that into a question, which is the only move that can extend
//! the signature.

use std::collections::{HashMap, HashSet};

use crate::program::{neg, pos, Program};
use crate::syntax::{Decl, Val};

#[derive(Debug, Clone, PartialEq)]
pub enum Rhs {
    Num(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lit {
    pub key: String,
    pub op: &'static str,
    pub rhs: Rhs,
}

impl Lit {
    pub fn holds(&self, attrs: &HashMap<String, Rhs>) -> bool {
        let Some(value) = attrs.get(&self.key) else {
            return false;
        };
        match self.op {
            "=" => value == &self.rhs,
            _ => {
                let (Rhs::Num(left), Rhs::Num(right)) = (value, &self.rhs) else {
                    return false;
                };
                if self.op == "<=" {
                    left <= right
                } else {
                    left >= right
                }
            }
        }
    }

    pub fn text(&self) -> String {
        match (&self.rhs, self.op) {
            (Rhs::Text(text), "=") => format!("{} = {text}", self.key),
            (Rhs::Num(value), "=") => format!("{} = {value}", self.key),
            (Rhs::Num(value), _) => {
                format!("{} {} {}", self.key, self.op, crate::program::fmt_g(*value))
            }
            (Rhs::Text(text), _) => format!("{} {} {text}", self.key, self.op),
        }
    }
}

pub struct Learning {
    pub concept: String,
    pub positives: Vec<String>,
    pub negatives: Vec<String>,
    pub clause: Option<Vec<Lit>>,
    pub witness: Option<(String, String)>,
    pub reason: String,
}

pub fn attrs_of(fact: &Decl) -> HashMap<String, Rhs> {
    let mut out = HashMap::new();
    let Some(Val::Rec(fields)) = fact
        .fields
        .iter()
        .find(|(key, _)| key == "attrs")
        .map(|(_, value)| value)
    else {
        return out;
    };
    for (key, value) in fields {
        match value {
            Val::Num(raw) => {
                if let Some(number) = crate::program::num_of(value) {
                    out.insert(key.clone(), Rhs::Num(number));
                } else {
                    let _ = raw;
                }
            }
            Val::Str(text) => {
                out.insert(key.clone(), Rhs::Text(text.clone()));
            }
            Val::Id(name) => {
                out.insert(key.clone(), Rhs::Text(name.clone()));
            }
            _ => {}
        }
    }
    out
}

fn examples<'a>(prog: &Program<'a>, decl: &Decl) -> (Vec<&'a Decl>, Vec<&'a Decl>) {
    let lookup = |names: Vec<String>| -> Vec<&'a Decl> {
        names
            .iter()
            .filter_map(|name| prog.get(name))
            .filter(|target| target.kind == "fact")
            .collect()
    };
    (lookup(pos(decl)), lookup(neg(decl)))
}

/// Ordered from most specific to most general, per key.
pub fn candidates(positive: &[&Decl], negative: &[&Decl]) -> Vec<Lit> {
    let facts: Vec<HashMap<String, Rhs>> = positive
        .iter()
        .chain(negative.iter())
        .map(|fact| attrs_of(fact))
        .collect();
    let mut keys: Vec<String> = facts
        .iter()
        .flat_map(|attrs| attrs.keys().cloned())
        .collect();
    keys.sort();
    keys.dedup();

    let mut raw: Vec<Lit> = Vec::new();
    for key in &keys {
        let values: Vec<Rhs> = facts
            .iter()
            .filter_map(|attrs| attrs.get(key).cloned())
            .collect();
        let numeric = values.iter().all(|value| matches!(value, Rhs::Num(_)));
        if numeric {
            let mut numbers: Vec<f64> = values
                .iter()
                .filter_map(|value| match value {
                    Rhs::Num(number) => Some(*number),
                    _ => None,
                })
                .collect();
            numbers.sort_by(|left, right| {
                left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
            });
            numbers.dedup();
            for value in &numbers {
                raw.push(Lit {
                    key: key.clone(),
                    op: "<=",
                    rhs: Rhs::Num(*value),
                });
            }
            for value in numbers.iter().rev() {
                raw.push(Lit {
                    key: key.clone(),
                    op: ">=",
                    rhs: Rhs::Num(*value),
                });
            }
        } else {
            let mut texts: Vec<String> = values
                .iter()
                .map(|value| match value {
                    Rhs::Num(number) => crate::program::repr_float(*number),
                    Rhs::Text(text) => text.clone(),
                })
                .collect();
            texts.sort();
            texts.dedup();
            for text in texts {
                raw.push(Lit {
                    key: key.clone(),
                    op: "=",
                    rhs: Rhs::Text(text),
                });
            }
        }
    }

    // keep only literals that can do work: cover a positive and exclude a negative
    let positive_attrs: Vec<HashMap<String, Rhs>> =
        positive.iter().map(|fact| attrs_of(fact)).collect();
    let negative_attrs: Vec<HashMap<String, Rhs>> =
        negative.iter().map(|fact| attrs_of(fact)).collect();
    raw.into_iter()
        .filter(|lit| {
            let covers = positive_attrs
                .iter()
                .filter(|attrs| lit.holds(attrs))
                .count();
            let excludes = negative_attrs
                .iter()
                .filter(|attrs| !lit.holds(attrs))
                .count();
            covers > 0 && excludes > 0
        })
        .collect()
}

fn first_collision(positive: &[&Decl], negative: &[&Decl]) -> Option<(String, String)> {
    for fact in positive {
        for other in negative {
            if attrs_of(fact) == attrs_of(other) {
                return Some((fact.name.clone(), other.name.clone()));
            }
        }
    }
    None
}

/// Index combinations of size `k`, in lexicographic order.
pub fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn walk(start: usize, n: usize, k: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if current.len() == k {
            out.push(current.clone());
            return;
        }
        for index in start..n {
            if n - index < k - current.len() {
                break;
            }
            current.push(index);
            walk(index + 1, n, k, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    walk(0, n, k, &mut Vec::new(), &mut out);
    out
}

pub fn induce_concept(prog: &Program, decl: &Decl) -> Learning {
    let (positive, negative) = examples(prog, decl);
    let mut result = Learning {
        concept: decl.name.clone(),
        positives: positive.iter().map(|fact| fact.name.clone()).collect(),
        negatives: negative.iter().map(|fact| fact.name.clone()).collect(),
        clause: None,
        witness: None,
        reason: String::new(),
    };
    if positive.is_empty() {
        result.reason = "no positive examples".to_string();
        return result;
    }
    if negative.is_empty() {
        result.reason = "no negative examples, so nothing constrains the clause".to_string();
        return result;
    }
    let lits = candidates(&positive, &negative);
    if lits.is_empty() {
        result.reason =
            "no attribute distinguishes the examples; they are indistinguishable as recorded"
                .to_string();
        result.witness = first_collision(&positive, &negative);
        return result;
    }

    let positive_attrs: Vec<HashMap<String, Rhs>> =
        positive.iter().map(|fact| attrs_of(fact)).collect();
    let negative_attrs: Vec<HashMap<String, Rhs>> =
        negative.iter().map(|fact| attrs_of(fact)).collect();
    for size in 1..=3 {
        if size > lits.len() {
            continue;
        }
        let mut best: Option<(usize, Vec<Lit>)> = None;
        for indices in combinations(lits.len(), size) {
            let combo: Vec<Lit> = indices.iter().map(|index| lits[*index].clone()).collect();
            let covers = positive_attrs
                .iter()
                .all(|attrs| combo.iter().all(|lit| lit.holds(attrs)));
            let excludes = negative_attrs
                .iter()
                .all(|attrs| !combo.iter().all(|lit| lit.holds(attrs)));
            if covers && excludes {
                let score: usize = indices.iter().sum();
                if best
                    .as_ref()
                    .map(|(best_score, _)| score < *best_score)
                    .unwrap_or(true)
                {
                    best = Some((score, combo));
                }
            }
        }
        if let Some((_, clause)) = best {
            result.clause = Some(clause);
            return result;
        }
    }

    result.reason = "no consistent conjunction of at most three literals exists".to_string();
    result.witness = first_collision(&positive, &negative);
    if result.witness.is_some() {
        result.reason = "a positive and a negative have identical attributes".to_string();
    }
    result
}

pub fn clause_text(clause: &[Lit]) -> String {
    clause
        .iter()
        .map(|lit| lit.text())
        .collect::<Vec<_>>()
        .join(" and ")
}

pub fn auto_question(learning: &Learning) -> (String, Vec<String>) {
    let asks = match &learning.witness {
        Some((a, b)) => format!(
            "{a} and {b} have identical recorded attributes, yet {b} is a counterexample to {}. What distinguishes them?",
            learning.concept
        ),
        None => format!("which attribute separates the examples of {}?", learning.concept),
    };
    (
        asks,
        vec![
            "a dimension that is not recorded yet (add an attribute)".to_string(),
            "one of the two examples is mislabelled (relabel)".to_string(),
            "the concept mixes two contexts and must be split (split)".to_string(),
        ],
    )
}

/// Names of the facts a concept's examples resolve to.
pub fn example_names(prog: &Program, decl: &Decl) -> (Vec<String>, Vec<String>) {
    let (positive, negative) = examples(prog, decl);
    (
        positive.iter().map(|fact| fact.name.clone()).collect(),
        negative.iter().map(|fact| fact.name.clone()).collect(),
    )
}

pub fn learned_names(learnings: &[Learning]) -> HashSet<&str> {
    learnings
        .iter()
        .filter(|item| item.clause.is_some())
        .map(|item| item.concept.as_str())
        .collect()
}
