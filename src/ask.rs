//! An open list is a question the document has not answered: which of the dimension's examples
//! belong in it.  This names the attribute that would answer that, or says that nothing
//! recorded can -- which is the one answer that tells you to record something new.

use std::collections::HashSet;

use crate::program::{f_id, f_ids, field, Program};
use crate::syntax::{Decl, Val};

pub enum Row {
    /// No dimension: the question is which concept the pile's elements share, and that
    /// question is a declaration, not a choice between recorded attributes.
    Pile {
        partition: String,
        asks: Vec<String>,
    },
    /// A dimension is named and the list does not cover its extent.
    Open {
        partition: String,
        dimension: String,
        listed: usize,
        extent: usize,
        gaps: Vec<Gap>,
    },
}

pub struct Gap {
    pub fact: String,
    /// `attribute = value` that no listed element records, or `attribute = (not recorded)`
    /// when every listed element records it and this one does not.
    pub separates: Vec<(String, String)>,
    /// Listed elements recording exactly the same attributes, so no attribute tells them apart.
    pub twins: Vec<String>,
}

pub fn plan(program: &Program) -> Vec<Row> {
    let mut rows = Vec::new();
    for partition in program.of_kind("partition") {
        match word(partition, "closure").unwrap_or("open") {
            "unknown" => rows.push(Row::Pile {
                partition: partition.name.clone(),
                asks: f_ids(partition, "closes_with"),
            }),
            "open" => {
                let Some(dimension) = f_id(partition, "dimension") else {
                    continue;
                };
                let Some(concept) = program.get(dimension) else {
                    continue;
                };
                let elements = f_ids(partition, "elements");
                let listed: HashSet<&str> = elements.iter().map(String::as_str).collect();
                let extent = f_ids(concept, "grounded_by");
                let gaps = extent
                    .iter()
                    .filter(|name| !listed.contains(name.as_str()))
                    .filter_map(|name| program.get(name))
                    .map(|fact| gap(fact, &elements, program))
                    .collect();
                rows.push(Row::Open {
                    partition: partition.name.clone(),
                    dimension: dimension.to_string(),
                    listed: elements.len(),
                    extent: extent.len(),
                    gaps,
                });
            }
            _ => {}
        }
    }
    rows
}

/// A field written as a bare word or as a string: `closure: unknown` parses as an identifier,
/// while a quoted spelling must behave the same way.
fn word<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    match field(decl, key) {
        Some(Val::Id(text)) | Some(Val::Str(text)) => Some(text.as_str()),
        _ => None,
    }
}

fn gap(fact: &Decl, elements: &[String], program: &Program) -> Gap {
    let mine = attrs(fact);
    let theirs: Vec<Vec<(String, String)>> = elements
        .iter()
        .filter_map(|name| program.get(name))
        .map(attrs)
        .collect();
    let mut separates: Vec<(String, String)> = Vec::new();
    for (key, value) in &mine {
        let shared = theirs
            .iter()
            .any(|record| record.iter().any(|(k, v)| k == key && v == value));
        if !shared {
            separates.push((key.clone(), value.clone()));
        }
    }
    // a key that every listed element records and this one does not is just as decisive
    for (key, _) in theirs.iter().flatten() {
        let universal = theirs
            .iter()
            .all(|record| record.iter().any(|(k, _)| k == key));
        if universal && !mine.iter().any(|(k, _)| k == key) {
            separates.push((key.clone(), "(not recorded)".to_string()));
        }
    }
    separates.sort();
    separates.dedup();
    let twins: Vec<String> = elements
        .iter()
        .filter(|name| {
            program
                .get(name)
                .map(|item| attrs(item) == mine)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    Gap {
        fact: fact.name.clone(),
        separates,
        twins,
    }
}

/// The recorded attributes, sorted, as they will be compared and printed.
pub fn attrs(decl: &Decl) -> Vec<(String, String)> {
    let Some(Val::Rec(fields)) = decl
        .fields
        .iter()
        .find(|(name, _)| name == "attrs")
        .map(|(_, value)| value)
    else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = fields
        .iter()
        .map(|(key, value)| (key.clone(), show(value)))
        .collect();
    out.sort();
    out
}

fn show(value: &Val) -> String {
    match value {
        Val::Id(text) | Val::Str(text) | Val::Num(text) => text.clone(),
        Val::Hole => "?".to_string(),
        Val::List(items) => format!("[{}]", joined(items.iter().map(show))),
        Val::Call(name, args) => format!("{name}({})", joined(args.iter().map(show))),
        Val::Rec(fields) => format!(
            "{{{}}}",
            joined(
                fields
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", show(value)))
            )
        ),
    }
}

fn joined(items: impl Iterator<Item = String>) -> String {
    items.collect::<Vec<String>>().join(", ")
}

/// `a`, `a and b`, `a, b and c` -- the same list shape as the rest of the compiler's prose.
pub fn and_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let head = names[..names.len() - 1].join(", ");
            format!("{head} and {}", names[names.len() - 1])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    fn rows(source: &str) -> Vec<Row> {
        let decls = parse(source).expect("parse");
        plan(&Program::new(&decls))
    }

    #[test]
    fn a_missing_element_names_the_attribute_that_would_decide_it() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first } }
fact F_GAP { claim: "g", evidence: tool("t"), attrs: { kind: second } }
concept C_THINGS { name: "things", grounded_by: [F_A, F_B, F_GAP] }
partition P_SOME { dimension: C_THINGS, elements: [F_A, F_B], closure: open }
"#;
        let rows = rows(source);
        let Row::Open { gaps, .. } = &rows[0] else {
            panic!("an open row");
        };
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].fact, "F_GAP");
        assert_eq!(
            gaps[0].separates,
            vec![("kind".to_string(), "second".to_string())]
        );
    }

    #[test]
    fn a_gap_nothing_separates_reports_its_twins() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_GAP { claim: "g", evidence: tool("t"), attrs: { kind: first } }
concept C_THINGS { name: "things", grounded_by: [F_A, F_GAP] }
partition P_SOME { dimension: C_THINGS, elements: [F_A], closure: open }
"#;
        let rows = rows(source);
        let Row::Open { gaps, .. } = &rows[0] else {
            panic!("an open row");
        };
        assert!(gaps[0].separates.is_empty());
        assert_eq!(gaps[0].twins, vec!["F_A".to_string()]);
    }

    #[test]
    fn a_complete_list_asks_nothing() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t") }
concept C_THINGS { name: "things", grounded_by: [F_A] }
partition P_ALL { dimension: C_THINGS, elements: [F_A], closure: open }
"#;
        let rows = rows(source);
        let Row::Open { gaps, .. } = &rows[0] else {
            panic!("an open row");
        };
        assert!(gaps.is_empty());
    }

    #[test]
    fn a_pile_asks_which_concept_its_elements_share() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t") }
question Q_TYPE { asks: "what do these share?", answers: ["one thing", "nothing"], separates: [] }
partition P_PILE { elements: [F_A], closure: unknown, closes_with: [Q_TYPE] }
"#;
        let rows = rows(source);
        let Row::Pile { asks, .. } = &rows[0] else {
            panic!("a pile row");
        };
        assert_eq!(asks, &vec!["Q_TYPE".to_string()]);
    }
}
