//! The next question a document needs answered.
//!
//! Three families, and they are one defect in three places: somewhere the document compares
//! objects, and the recorded attributes cannot tell two of them apart.  A partition that does
//! not cover its dimension's extent, two facts that record the same evidence and the same
//! attributes, and a concept whose positive and negative examples look identical all ask the
//! same thing -- which attribute would separate them?  A question names that attribute, or says
//! plainly that no recorded attribute can, which is the answer that tells the author to record
//! something new.

use std::collections::{BTreeMap, HashSet};

use crate::check::collide::signature;
use crate::program::{f_id, f_ids, field, Program};
use crate::syntax::{Decl, Val};

#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    /// `gap`, `collision`, `concept` or `pile`.
    pub kind: &'static str,
    /// What is being asked about: a list, a concept, or a pair of facts.
    pub subject: String,
    pub fact: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    /// How many blocks one answer would settle.
    pub decides: usize,
    pub question: String,
}

/// Every open question, ranked by how much one answer would settle.
pub fn questions(program: &Program) -> Vec<Question> {
    let mut out = Vec::new();
    out.extend(piles(program));
    out.extend(gaps(program));
    out.extend(collisions(program));
    out.extend(concept_collisions(program));
    out.sort_by(|left, right| {
        right
            .decides
            .cmp(&left.decides)
            .then(left.kind.cmp(right.kind))
            .then(left.subject.cmp(&right.subject))
            .then(left.fact.cmp(&right.fact))
    });
    out
}

fn piles(program: &Program) -> Vec<Question> {
    let mut out = Vec::new();
    for partition in program.of_kind("partition") {
        if word(partition, "closure") != Some("unknown") {
            continue;
        }
        let asks = f_ids(partition, "closes_with");
        let question = if asks.is_empty() {
            format!(
                "{} has no dimension; which concept do its elements share? nothing is attached that would type it",
                partition.name
            )
        } else {
            format!(
                "{} has no dimension; which concept do its elements share? -> {}",
                partition.name,
                and_list(&asks)
            )
        };
        out.push(Question {
            kind: "pile",
            subject: partition.name.clone(),
            fact: None,
            key: None,
            value: None,
            decides: 1,
            question,
        });
    }
    out
}

fn gaps(program: &Program) -> Vec<Question> {
    struct Draft {
        partition: String,
        fact: String,
        separates: Vec<(String, String)>,
        twins: Vec<String>,
    }
    // a fact can be missing from more than one list; one measurement settles all of them
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    let mut drafts: Vec<Draft> = Vec::new();
    for partition in program.of_kind("partition") {
        if word(partition, "closure") != Some("open") {
            continue;
        }
        let Some(dimension) = f_id(partition, "dimension") else {
            continue;
        };
        let Some(concept) = program.get(dimension) else {
            continue;
        };
        let elements = f_ids(partition, "elements");
        let listed: HashSet<&str> = elements.iter().map(String::as_str).collect();
        let records: Vec<(String, Vec<(String, String)>)> = elements
            .iter()
            .filter_map(|name| program.get(name).map(|item| (name.clone(), attrs(item))))
            .collect();
        for name in f_ids(concept, "grounded_by") {
            if listed.contains(name.as_str()) {
                continue;
            }
            let Some(fact) = program.get(&name) else {
                continue;
            };
            *missing.entry(name.clone()).or_insert(0) += 1;
            let mine = attrs(fact);
            let mut separates: Vec<(String, String)> = Vec::new();
            for (key, value) in &mine {
                if !records
                    .iter()
                    .any(|(_, record)| record.iter().any(|(k, v)| k == key && v == value))
                {
                    separates.push((key.clone(), value.clone()));
                }
            }
            for (_, record) in &records {
                for (key, _) in record {
                    let universal = records
                        .iter()
                        .all(|(_, other)| other.iter().any(|(k, _)| k == key));
                    if universal && !mine.iter().any(|(k, _)| k == key) {
                        separates.push((key.clone(), "(not recorded)".to_string()));
                    }
                }
            }
            separates.sort();
            separates.dedup();
            let twins = records
                .iter()
                .filter(|(_, record)| *record == mine)
                .map(|(name, _)| name.clone())
                .collect();
            drafts.push(Draft {
                partition: partition.name.clone(),
                fact: name,
                separates,
                twins,
            });
        }
    }
    drafts
        .into_iter()
        .map(|draft| {
            let decides = *missing.get(&draft.fact).unwrap_or(&1);
            let question = match draft.separates.first() {
                Some((key, value)) if value == "(not recorded)" => format!(
                    "{} does not list {}; does it record {key}? every listed element does",
                    draft.partition, draft.fact
                ),
                Some((key, value)) => format!(
                    "{} does not list {}; does it record {key} = {value}? no listed element does",
                    draft.partition, draft.fact
                ),
                None => format!(
                    "{} does not list {}; no recorded attribute separates it from {}",
                    draft.partition,
                    draft.fact,
                    and_list(&draft.twins)
                ),
            };
            Question {
                kind: "gap",
                subject: draft.partition,
                fact: Some(draft.fact),
                key: draft.separates.first().map(|(key, _)| key.clone()),
                value: draft.separates.first().map(|(_, value)| value.clone()),
                decides,
                question,
            }
        })
        .collect()
}

fn collisions(program: &Program) -> Vec<Question> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for fact in program.of_kind("fact") {
        if let Some(key) = signature(fact) {
            groups.entry(key).or_default().push(fact.name.clone());
        }
    }
    let mut out = Vec::new();
    for (_, mut names) in groups {
        if names.len() < 2 {
            continue;
        }
        names.sort();
        let first = names[0].clone();
        out.push(Question {
            kind: "collision",
            subject: format!("{first} ~ {}", and_list(&names[1..])),
            fact: Some(first),
            key: None,
            value: None,
            decides: names.len() - 1,
            question: format!(
                "{} record the same evidence and the same attributes; which recorded attribute differs?",
                and_list(&names)
            ),
        });
    }
    out
}

fn concept_collisions(program: &Program) -> Vec<Question> {
    let mut out = Vec::new();
    for concept in program.of_kind("concept") {
        let positives = f_ids(concept, "grounded_by");
        let negatives = f_ids(concept, "against");
        for positive in &positives {
            let Some(yes) = program.get(positive) else {
                continue;
            };
            let record = attrs(yes);
            if record.is_empty() {
                continue;
            }
            for negative in &negatives {
                let Some(no) = program.get(negative) else {
                    continue;
                };
                if record != attrs(no) {
                    continue;
                }
                out.push(Question {
                    kind: "concept",
                    subject: concept.name.clone(),
                    fact: Some(negative.clone()),
                    key: None,
                    value: None,
                    decides: 1,
                    question: format!(
                        "{} is a negative example of {} with the same record as the positive {}; which attribute separates them?",
                        negative, concept.name, positive
                    ),
                });
            }
        }
    }
    out
}

/// A field written as a bare word or as a string: `closure: unknown` parses as an identifier,
/// while a quoted spelling must behave the same way.
fn word<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    match field(decl, key) {
        Some(Val::Id(text)) | Some(Val::Str(text)) => Some(text.as_str()),
        _ => None,
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

/// The text form, shared by the command line and the MCP server so they cannot drift.
pub fn text(program: &Program, one: bool) -> String {
    let asked = questions(program);
    if asked.is_empty() {
        return "nothing to ask: every list covers its extent and nothing collides\n".to_string();
    }
    if one {
        return format!("{}\n", asked[0].question);
    }
    let mut out = format!(
        "{} question(s), ranked by what one answer would settle\n",
        asked.len()
    );
    for item in &asked {
        out.push_str(&format!(
            "  {:>2}  {:9}  {}\n",
            item.decides, item.kind, item.question
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;

    fn all(source: &str) -> Vec<Question> {
        let decls = parse(source).expect("parse");
        questions(&Program::new(&decls))
    }

    const GAP: &str = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first } }
fact F_GAP { claim: "g", evidence: tool("t"), attrs: { kind: second } }
concept C_THINGS { name: "things", grounded_by: [F_A, F_B, F_GAP] }
partition P_SOME { dimension: C_THINGS, elements: [F_A, F_B], closure: open }
"#;

    #[test]
    fn a_missing_element_names_the_attribute_that_would_decide_it() {
        let asked = all(GAP);
        let gap = asked
            .iter()
            .find(|item| item.kind == "gap")
            .expect("a gap question");
        assert_eq!(gap.fact.as_deref(), Some("F_GAP"));
        assert_eq!(gap.key.as_deref(), Some("kind"));
        assert_eq!(gap.value.as_deref(), Some("second"));
        assert!(
            gap.question.contains("no listed element does"),
            "{}",
            gap.question
        );
    }

    #[test]
    fn a_gap_nothing_separates_says_so_and_names_the_twins() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_GAP { claim: "g", evidence: tool("t"), attrs: { kind: first } }
concept C_THINGS { name: "things", grounded_by: [F_A, F_GAP] }
partition P_SOME { dimension: C_THINGS, elements: [F_A], closure: open }
"#;
        let asked = all(source);
        let gap = asked.iter().find(|item| item.kind == "gap").unwrap();
        assert!(gap.key.is_none());
        assert!(
            gap.question.contains("no recorded attribute separates"),
            "{}",
            gap.question
        );
    }

    #[test]
    fn two_facts_nothing_tells_apart_are_a_collision() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first } }
"#;
        let asked = all(source);
        let collision = asked.iter().find(|item| item.kind == "collision").unwrap();
        assert_eq!(collision.decides, 1);
        assert!(collision
            .question
            .contains("which recorded attribute differs"));
    }

    #[test]
    fn a_concept_whose_examples_look_alike_asks_what_separates_them() {
        let source = r#"
fact F_P { claim: "p", evidence: tool("t"), attrs: { kind: first } }
fact F_N { claim: "n", evidence: tool("t"), attrs: { kind: first } }
concept C_X { name: "x", grounded_by: [F_P], against: [F_N] }
"#;
        let asked = all(source);
        let concept = asked.iter().find(|item| item.kind == "concept").unwrap();
        assert_eq!(concept.subject, "C_X");
        assert!(concept.question.contains("which attribute separates them"));
    }

    #[test]
    fn a_pile_asks_which_concept_its_elements_share() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t") }
question Q_TYPE { asks: "what do these share?", answers: ["one thing", "nothing"] }
partition P_PILE { elements: [F_A], closure: unknown, closes_with: [Q_TYPE] }
"#;
        let asked = all(source);
        let pile = asked.iter().find(|item| item.kind == "pile").unwrap();
        assert!(pile.question.contains("Q_TYPE"));
    }

    #[test]
    fn the_bigger_settlement_comes_first() {
        let source = r#"
fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first } }
fact F_C { claim: "c", evidence: tool("t"), attrs: { kind: first } }
fact F_N { claim: "n", evidence: tool("t"), attrs: { kind: second } }
concept C_X { name: "x", grounded_by: [F_N], against: [F_A] }
"#;
        let asked = all(source);
        assert_eq!(asked[0].kind, "collision");
        assert!(asked[0].decides > asked.last().unwrap().decides || asked.len() == 1);
    }
}
