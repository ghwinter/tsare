//! What the attribute table already implies.
//!
//! `W-COLLIDE` says two facts cannot be told apart; `ask` says which observation would decide
//! an open list; this says what the recorded attributes *entail*.  If every fact that records
//! `items = 1` also records `urgent = 0`, then the document already holds that law whether or
//! not anyone wrote it down.
//!
//! The search is bounded on purpose: antecedents are enumerated up to `--max-antecedent`
//! attributes and implications need `--min-support` objects behind them.  A bounded search is
//! not the canonical basis -- a minimal basis may need larger antecedents -- so the output says
//! what it searched instead of pretending to be complete.

use std::collections::{BTreeMap, BTreeSet};

use crate::ask::attrs;
use crate::program::Program;

#[derive(Debug, Clone, PartialEq)]
pub struct Implication {
    /// Sorted `key=value` pairs that jointly imply the consequent.
    pub antecedent: Vec<String>,
    pub consequent: String,
    /// How many facts record every pair of the antecedent.
    pub support: usize,
    pub objects: usize,
    /// A fact that records the consequent but not the whole antecedent, so the law does not run
    /// backwards.  `None` means it does run backwards on this context.
    pub converse_witness: Option<String>,
}

pub struct Search {
    pub max_antecedent: usize,
    pub min_support: usize,
    pub objects: usize,
    pub attributes: usize,
}

/// Objects are the facts that record at least one attribute; an attribute is a `key=value` pair.
pub fn implications(
    program: &Program,
    max_antecedent: usize,
    min_support: usize,
) -> (Vec<Implication>, Search) {
    let mut objects: Vec<(String, BTreeSet<String>)> = program
        .of_kind("fact")
        .into_iter()
        .map(|fact| {
            (
                fact.name.clone(),
                attrs(fact)
                    .into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<BTreeSet<String>>(),
            )
        })
        .filter(|(_, set)| !set.is_empty())
        .collect();
    objects.sort_by(|left, right| left.0.cmp(&right.0));
    let attributes: BTreeSet<String> = objects
        .iter()
        .flat_map(|(_, set)| set.iter().cloned())
        .collect();
    let search = Search {
        max_antecedent: max_antecedent.max(1).min(attributes.len().max(1)),
        min_support,
        objects: objects.len(),
        attributes: attributes.len(),
    };

    let names: Vec<String> = attributes.into_iter().collect();
    let mut found: Vec<Implication> = Vec::new();
    let mut reported: BTreeSet<(Vec<String>, String)> = BTreeSet::new();
    for size in 1..=search.max_antecedent {
        for antecedent in combinations(&names, size) {
            let extent: Vec<&(String, BTreeSet<String>)> = objects
                .iter()
                .filter(|(_, set)| antecedent.iter().all(|pair| set.contains(pair)))
                .collect();
            if extent.len() < search.min_support {
                continue;
            }
            let mut intent: BTreeSet<String> = extent[0].1.clone();
            for (_, set) in &extent {
                intent = intent.intersection(set).cloned().collect();
            }
            for consequent in intent {
                if antecedent.contains(&consequent) {
                    continue;
                }
                if !minimal(&antecedent, &consequent, &objects) {
                    continue;
                }
                if !reported.insert((antecedent.clone(), consequent.clone())) {
                    continue;
                }
                let witness = objects
                    .iter()
                    .find(|(_, set)| {
                        set.contains(&consequent)
                            && !antecedent.iter().all(|pair| set.contains(pair))
                    })
                    .map(|(name, _)| name.clone());
                found.push(Implication {
                    antecedent: antecedent.clone(),
                    consequent,
                    support: extent.len(),
                    objects: objects.len(),
                    converse_witness: witness,
                });
            }
        }
    }
    found.sort_by(|left, right| {
        left.antecedent
            .len()
            .cmp(&right.antecedent.len())
            .then(left.antecedent.cmp(&right.antecedent))
            .then(left.consequent.cmp(&right.consequent))
    });
    (found, search)
}

/// A proper subset of the antecedent that already implies the consequent makes this one
/// redundant, and a redundant law is noise.
fn minimal(
    antecedent: &[String],
    consequent: &str,
    objects: &[(String, BTreeSet<String>)],
) -> bool {
    if antecedent.len() == 1 {
        return true;
    }
    for size in 1..antecedent.len() {
        for subset in combinations(antecedent, size) {
            let holds = objects.iter().all(|(_, set)| {
                !subset.iter().all(|pair| set.contains(pair)) || set.contains(consequent)
            });
            if holds {
                return false;
            }
        }
    }
    true
}

fn combinations(items: &[String], size: usize) -> Vec<Vec<String>> {
    if size == 0 || size > items.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut index: Vec<usize> = (0..size).collect();
    loop {
        out.push(index.iter().map(|i| items[*i].clone()).collect());
        let mut position = size;
        loop {
            if position == 0 {
                return out;
            }
            position -= 1;
            if index[position] != position + items.len() - size {
                break;
            }
        }
        index[position] += 1;
        for next in position + 1..size {
            index[next] = index[next - 1] + 1;
        }
    }
}

/// Group implications for printing: one block per antecedent size, so a reader can see how much
/// the search had to combine.
pub fn by_size(found: &[Implication]) -> BTreeMap<usize, Vec<&Implication>> {
    let mut out: BTreeMap<usize, Vec<&Implication>> = BTreeMap::new();
    for item in found {
        out.entry(item.antecedent.len()).or_default().push(item);
    }
    out
}

/// The text form, shared by the command line and the MCP server so they cannot drift.
pub fn text(found: &[Implication], search: &Search) -> String {
    if found.is_empty() {
        return format!(
            "no implication holds: {} fact(s), {} attribute(s), antecedents up to {}, support at least {}\n",
            search.objects, search.attributes, search.max_antecedent, search.min_support
        );
    }
    let mut out = format!(
        "{} implication(s) over {} fact(s) and {} attribute(s); antecedents up to {}, support at least {}\n",
        found.len(),
        search.objects,
        search.attributes,
        search.max_antecedent,
        search.min_support
    );
    for item in found {
        let converse = match &item.converse_witness {
            Some(name) => format!("converse fails: {name}"),
            None => "converse also holds".to_string(),
        };
        out.push_str(&format!(
            "  {} -> {}   ({}/{} facts; {converse})\n",
            item.antecedent.join(" & "),
            item.consequent,
            item.support,
            item.objects
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    fn found(source: &str, max: usize, support: usize) -> Vec<Implication> {
        let decls = parse(source).expect("parse");
        implications(&Program::new(&decls), max, support).0
    }

    const FOUR: &str = r#"
fact F_1 { claim: "1", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_2 { claim: "2", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_3 { claim: "3", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_4 { claim: "4", evidence: tool("t"), attrs: { items: 5, urgent: 1 } }
fact F_5 { claim: "5", evidence: tool("t"), attrs: { items: 5, urgent: 0 } }
"#;

    #[test]
    fn an_implication_holds_when_every_witness_agrees() {
        let out = found(FOUR, 1, 2);
        let law = out
            .iter()
            .find(|item| item.antecedent == vec!["items=1".to_string()])
            .expect("items=1 implies something");
        assert_eq!(law.consequent, "urgent=0");
        assert_eq!(law.support, 3);
    }

    #[test]
    fn the_converse_is_not_reported_when_a_counterexample_exists() {
        let out = found(FOUR, 1, 2);
        assert!(
            !out.iter()
                .any(|item| item.antecedent == vec!["urgent=0".to_string()]
                    && item.consequent == "items=1"),
            "urgent=0 does not imply items=1"
        );
        // the law does not run backwards either: the fifth fact has urgent=0 and no items=1
        let law = out
            .iter()
            .find(|item| item.antecedent == vec!["items=1".to_string()])
            .expect("items=1 implies urgent=0");
        assert_eq!(law.converse_witness.as_deref(), Some("F_5"));
    }

    #[test]
    fn a_weak_law_is_not_reported_below_the_support_threshold() {
        let out = found(FOUR, 1, 4);
        assert!(
            !out.iter()
                .any(|item| item.antecedent == vec!["items=1".to_string()]),
            "three facts are below a support of four"
        );
    }

    #[test]
    fn a_larger_antecedent_that_a_smaller_one_already_covers_is_dropped() {
        let out = found(FOUR, 2, 2);
        assert!(
            !out.iter().any(|item| item.antecedent.len() == 2
                && item.antecedent.contains(&"items=1".to_string())
                && item.consequent == "urgent=0"),
            "items=1 already implies urgent=0"
        );
    }

    #[test]
    fn combinations_are_complete() {
        let items: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert_eq!(combinations(&items, 1).len(), 3);
        assert_eq!(combinations(&items, 2).len(), 3);
        assert_eq!(combinations(&items, 3).len(), 1);
        assert_eq!(combinations(&items, 4).len(), 0);
    }
}
