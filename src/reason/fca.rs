//! Formal concept analysis, the part a checker can use: intent, extent, and the gap between
//! what the recorded attributes imply and what the author listed.
//!
//! A concept's positives share an intent — the attribute pairs common to all of them.  Every
//! fact that carries that intent is an instance, whether or not the author wrote it down.  The
//! difference is the answer to "what do these have in common", computed instead of asked.

use std::collections::HashMap;

use crate::fix::render_value;
use crate::program::{neg, pos, Program};
use crate::syntax::{Decl, Val};

pub type Pair = (String, String);

pub fn attributes(decl: &Decl) -> Vec<Pair> {
    match decl
        .fields
        .iter()
        .find(|(key, _)| key == "attrs")
        .map(|(_, value)| value)
    {
        Some(Val::Rec(fields)) => fields
            .iter()
            .map(|(key, value)| (key.clone(), render_value(value)))
            .collect(),
        _ => Vec::new(),
    }
}

/// The attribute pairs every fact in the sample has.
///
/// For an empty sample this is **every attribute in the program**, not none: intent of no objects is
/// vacuously satisfied by all of them.  That convention is what makes `intent ∘ extent` a closure
/// operator -- with the empty intent instead, a larger attribute set can have a smaller closure and
/// monotonicity fails at exactly the sets no fact satisfies.
pub fn intent(prog: &Program, facts: &[&Decl]) -> Vec<Pair> {
    let mut iter = facts.iter();
    let Some(first) = iter.next() else {
        let mut universe: Vec<Pair> = prog
            .of_kind("fact")
            .iter()
            .flat_map(|fact| attributes(fact))
            .collect();
        universe.sort();
        universe.dedup();
        return universe;
    };
    let mut common = attributes(first);
    for fact in iter {
        let attrs = attributes(fact);
        common.retain(|pair| attrs.contains(pair));
    }
    common.sort();
    common
}

/// Every fact that carries all of `pairs`, in declaration order.
pub fn extent<'a>(prog: &Program<'a>, pairs: &[Pair]) -> Vec<&'a Decl> {
    prog.of_kind("fact")
        .into_iter()
        .filter(|fact| {
            let attrs = attributes(fact);
            pairs.iter().all(|pair| attrs.contains(pair))
        })
        .collect()
}

pub struct Concept {
    pub name: String,
    pub intent: Vec<Pair>,
    /// Facts the intent implies that the concept does not list as examples.
    pub implied: Vec<String>,
    pub listed: usize,
}

pub fn concepts(prog: &Program) -> Vec<Concept> {
    let mut out = Vec::new();
    for concept in prog.of_kind("concept") {
        let positives: Vec<&Decl> = pos(concept)
            .iter()
            .filter_map(|name| prog.get(name))
            .collect();
        if positives.is_empty() {
            continue;
        }
        let pairs = intent(prog, &positives);
        let implied = if pairs.is_empty() {
            Vec::new()
        } else {
            extent(prog, &pairs)
                .into_iter()
                .filter(|fact| !positives.iter().any(|known| known.name == fact.name))
                .map(|fact| fact.name.clone())
                .collect()
        };
        out.push(Concept {
            name: concept.name.clone(),
            intent: pairs,
            implied,
            listed: positives.len(),
        });
    }
    out
}

pub struct Pile {
    pub name: String,
    pub shared: Vec<Pair>,
    pub elements: usize,
}

/// Partitions with no dimension: the compiler can still say what their elements share.
pub fn piles(prog: &Program) -> Vec<Pile> {
    let mut out = Vec::new();
    for partition in prog.of_kind("partition") {
        if crate::program::f_id(partition, "closure") != Some("unknown") {
            continue;
        }
        let elements: Vec<&Decl> = crate::program::f_ids(partition, "elements")
            .iter()
            .filter_map(|name| prog.get(name))
            .collect();
        let shared = intent(prog, &elements);
        out.push(Pile {
            name: partition.name.clone(),
            shared,
            elements: elements.len(),
        });
    }
    out
}

/// The concepts whose counterexamples cannot be compared with their positives, because the
/// two share no attribute key at all.
pub fn incomparable(prog: &Program) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for concept in prog.of_kind("concept") {
        let positives: Vec<&Decl> = pos(concept)
            .iter()
            .filter_map(|name| prog.get(name))
            .collect();
        let keys: std::collections::HashSet<String> = positives
            .iter()
            .flat_map(|fact| attributes(fact))
            .map(|pair| pair.0)
            .collect();
        if keys.is_empty() {
            continue;
        }
        for name in neg(concept) {
            let Some(fact) = prog.get(&name) else {
                continue;
            };
            let attrs = attributes(fact);
            if !attrs.is_empty() && !attrs.iter().any(|pair| keys.contains(&pair.0)) {
                out.push((concept.name.clone(), name));
            }
        }
    }
    out
}

pub fn render_pairs(pairs: &[Pair]) -> String {
    if pairs.is_empty() {
        return "(nothing recorded in common)".to_string();
    }
    pairs
        .iter()
        .map(|(key, value)| format!("{key} = {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn kind_counts(prog: &Program) -> HashMap<String, usize> {
    let mut out: HashMap<String, usize> = HashMap::new();
    for decl in prog.decls {
        *out.entry(decl.kind.clone()).or_insert(0) += 1;
    }
    out
}
