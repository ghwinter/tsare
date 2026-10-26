//! Two facts can carry different claims and the same recorded attributes, which means this
//! vocabulary cannot tell them apart: the document believes it has two observations and the
//! predicate table has one.

use super::*;

pub(crate) fn check_collide(decls: &[Decl], out: &mut Vec<Diagnostic>) {
    let mut groups: HashMap<String, Vec<&Decl>> = HashMap::new();
    for decl in decls.iter().filter(|decl| decl.kind == "fact") {
        if let Some(signature) = signature(decl) {
            groups.entry(signature).or_default().push(decl);
        }
    }
    // deterministic order: the group whose first member appears first in the document
    let mut collided: Vec<Vec<&Decl>> = groups
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();
    collided.sort_by_key(|group| group[0].line);
    for group in collided {
        for decl in &group {
            let others: Vec<&str> = group
                .iter()
                .map(|item| item.name.as_str())
                .filter(|name| *name != decl.name)
                .collect();
            sem(
                decl,
                "W-COLLIDE",
                format!(
                    "{} and {} record the same evidence and the same attributes, so this vocabulary cannot tell them apart",
                    decl.name,
                    join(&others)
                ),
                Some("attrs"),
                out,
            );
        }
    }
}

/// What the machine-readable part of a fact records: its evidence and its attributes.
/// `None` when the fact records no attributes -- every unrecorded fact would collide with
/// every other one, and the attribute table says nothing about either.  Evidence is part of
/// the signature because two facts citing different commands are distinguished even when their
/// numbers are identical.
fn signature(decl: &Decl) -> Option<String> {
    let attrs = decl
        .fields
        .iter()
        .find(|(name, _)| name == "attrs")
        .map(|(_, value)| value)?;
    let Val::Rec(fields) = attrs else {
        return None;
    };
    let mut pairs: Vec<String> = fields
        .iter()
        .map(|(key, value)| format!("{key}={value:?}"))
        .collect();
    if pairs.is_empty() {
        return None;
    }
    pairs.sort();
    let evidence = decl
        .fields
        .iter()
        .find(|(name, _)| name == "evidence")
        .map(|(_, value)| format!("{value:?}"))
        .unwrap_or_default();
    Some(format!("evidence={evidence}|{}", pairs.join(",")))
}

fn join(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_string(),
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

    fn warns(source: &str) -> usize {
        let decls = parse(source).expect("parse");
        let mut out = Vec::new();
        check_collide(&decls, &mut out);
        out.len()
    }

    #[test]
    fn the_same_attributes_are_one_observation() {
        let source = r#"
fact F_A { claim: "32 of 40 asked for dark mode", evidence: tool("survey"), attrs: { asked: 32, total: 40 } }
fact F_B { claim: "12 of 40 asked for multi-select", evidence: tool("survey"), attrs: { asked: 12, total: 40 } }
"#;
        // the claims differ and so does one number, so nothing collides
        assert_eq!(warns(source), 0);
    }

    #[test]
    fn indistinguishable_facts_collide() {
        let source = r#"
fact F_A { claim: "32 people named dark mode", evidence: tool("survey"), attrs: { asked: 32, total: 40 } }
fact F_B { claim: "the dark mode request came from 32 people", evidence: tool("survey"), attrs: { total: 40, asked: 32 } }
"#;
        assert_eq!(warns(source), 2);
    }

    #[test]
    fn an_attribute_only_one_of_them_has_ends_the_collision() {
        let source = r#"
fact F_A { claim: "one", evidence: tool("survey"), attrs: { asked: 32, total: 40 } }
fact F_B { claim: "two", evidence: tool("survey"), attrs: { asked: 32, total: 40, returning: 9 } }
"#;
        assert_eq!(warns(source), 0);
    }

    #[test]
    fn a_fact_without_attributes_never_collides() {
        let source = r#"
fact F_A { claim: "one", evidence: user }
fact F_B { claim: "two", evidence: user }
"#;
        assert_eq!(warns(source), 0);
    }
}
