//! A proposed predicate is only new when the current vocabulary cannot express it.
//!
//! `tsare ask` says which observation would decide an open list.  This answers a different
//! question, the one that follows: the model proposes a new attribute, and the document has to
//! say whether that attribute is a distinction the recorded vocabulary already contains.  A
//! predicate that is a function of the existing attributes is a recombination; a predicate that
//! separates two objects with identical attribute records is new, and those two objects are its
//! witness.
//!
//! The decision is a partition comparison, so it is finite and needs no model.

use std::collections::{BTreeMap, BTreeSet};

use crate::ask::attrs;
use crate::program::Program;

/// One fact's recorded attributes, sorted: the object of the context.
type Record = Vec<(String, String)>;

/// The objects that share a record, and whether the proposal is true of each.
type Members<'a> = Vec<(&'a str, bool)>;

#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// No fact records an attribute, so there is nothing for a predicate to be defined over.
    Empty,
    /// One existing attribute records exactly this split.
    Redundant {
        key: String,
        value: String,
        on: Vec<String>,
    },
    /// Two objects share every recorded attribute and disagree on the proposal.
    Novel { witness: (String, String) },
    /// Every block of objects agrees: the proposal is a function of what is recorded.  When
    /// every fact already has its own record this holds for *any* proposal, and says nothing.
    Definable { blocks: usize, objects: usize },
}

/// `holds` lists the facts for which the proposed attribute is taken to be true; every other
/// fact in the context is taken to be false.
pub fn decide(program: &Program, holds: &[String]) -> Verdict {
    let mut objects: Vec<(String, Record)> = program
        .of_kind("fact")
        .into_iter()
        .map(|fact| (fact.name.clone(), attrs(fact)))
        .filter(|(_, record)| !record.is_empty())
        .collect();
    if objects.is_empty() {
        return Verdict::Empty;
    }
    objects.sort_by(|left, right| left.0.cmp(&right.0));
    let true_set: BTreeSet<&str> = holds.iter().map(String::as_str).collect();

    // a constant predicate splits nothing, so it is a function of the record like any other
    let on_count = objects
        .iter()
        .filter(|(name, _)| true_set.contains(name.as_str()))
        .count();
    if on_count == 0 || on_count == objects.len() {
        return Verdict::Definable {
            blocks: 0,
            objects: objects.len(),
        };
    }

    // redundant: one key and one of its values already draws exactly this line
    let mut keys: BTreeSet<&str> = BTreeSet::new();
    for (_, record) in &objects {
        for (key, _) in record {
            keys.insert(key.as_str());
        }
    }
    for key in keys {
        let mut on: Vec<String> = Vec::new();
        let mut values_on: BTreeSet<&str> = BTreeSet::new();
        let mut values_off: BTreeSet<&str> = BTreeSet::new();
        for (name, record) in &objects {
            let value = record
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str());
            match (true_set.contains(name.as_str()), value) {
                (true, Some(value)) => {
                    on.push(name.clone());
                    values_on.insert(value);
                }
                (true, None) => {
                    values_on.insert("(not recorded)");
                }
                (false, Some(value)) => {
                    values_off.insert(value);
                }
                (false, None) => {
                    values_off.insert("(not recorded)");
                }
            };
        }
        if on.is_empty() {
            continue;
        }
        if let Some(value) = values_on.iter().next() {
            if values_on.len() == 1 && !values_off.contains(value) {
                return Verdict::Redundant {
                    key: key.to_string(),
                    value: (*value).to_string(),
                    on,
                };
            }
        }
    }

    // blocks: objects that share an attribute record
    let mut blocks: BTreeMap<&Record, Members> = BTreeMap::new();
    for (name, record) in &objects {
        blocks
            .entry(record)
            .or_default()
            .push((name.as_str(), true_set.contains(name.as_str())));
    }
    let mixed: Vec<&Members> = blocks
        .values()
        .filter(|members| {
            members.iter().any(|(_, holds)| *holds) && members.iter().any(|(_, holds)| !*holds)
        })
        .collect();
    if let Some(members) = mixed.first() {
        let yes = members
            .iter()
            .find(|(_, holds)| *holds)
            .map(|(name, _)| *name);
        let no = members
            .iter()
            .find(|(_, holds)| !*holds)
            .map(|(name, _)| *name);
        if let (Some(yes), Some(no)) = (yes, no) {
            return Verdict::Novel {
                witness: (yes.to_string(), no.to_string()),
            };
        }
    }
    Verdict::Definable {
        blocks: blocks.len(),
        objects: objects.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    fn verdict(source: &str, holds: &[&str]) -> Verdict {
        let decls = parse(source).expect("parse");
        let program = Program::new(&decls);
        let holds: Vec<String> = holds.iter().map(|name| name.to_string()).collect();
        decide(&program, &holds)
    }

    const PAIR: &str = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { size: 3, kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first, size: 3 } }
"#;

    #[test]
    fn a_split_inside_one_block_is_new_and_has_a_witness() {
        let result = verdict(PAIR, &["F_A"]);
        assert_eq!(
            result,
            Verdict::Novel {
                witness: ("F_A".to_string(), "F_B".to_string())
            }
        );
    }

    #[test]
    fn agreeing_inside_every_block_is_a_recombination() {
        let result = verdict(PAIR, &["F_A", "F_B"]);
        assert!(matches!(result, Verdict::Definable { .. }), "{result:?}");
    }

    #[test]
    fn a_line_an_existing_key_already_draws_is_redundant() {
        let result = verdict(PAIR, &["F_A"]);
        assert!(matches!(result, Verdict::Novel { .. }));
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { size: 3 } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { size: 9 } }
"#;
        // saying "size = 3" is not a new predicate, it is a column that is already there
        assert_eq!(
            verdict(source, &["F_A"]),
            Verdict::Redundant {
                key: "size".to_string(),
                value: "3".to_string(),
                on: vec!["F_A".to_string()]
            }
        );
    }

    #[test]
    fn a_proposal_one_object_alone_agrees_with_everything() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { size: 3 } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { size: 9 } }
"#;
        // "size = 3" is not a new distinction, it is a column that is already there
        assert!(matches!(
            verdict(source, &["F_A"]),
            Verdict::Redundant { .. }
        ));
        // and a constant proposal is a function of the record like any other
        assert!(matches!(
            verdict(source, &["F_A", "F_B"]),
            Verdict::Definable { .. }
        ));
        assert!(matches!(verdict(source, &[]), Verdict::Definable { .. }));
    }

    #[test]
    fn a_context_without_attributes_decides_nothing() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t") }
fact F_B { claim: "b", evidence: tool("t") }
"#;
        assert_eq!(verdict(source, &["F_A"]), Verdict::Empty);
    }
}
