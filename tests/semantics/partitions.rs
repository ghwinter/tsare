use super::*;

#[test]
fn a_closed_partition_must_cover_its_dimension() {
    let source = r#"
fact F_A { claim: "a", evidence: user }
fact F_B { claim: "b", evidence: user }
concept C_ALL { name: "all", grounded_by: [F_A, F_B] }
partition E_ONE { dimension: C_ALL, elements: [F_A], closure: closed, basis: [F_B] }
"#;
    assert!(
        codes(source).contains(&"E-PARTITION-CLOSED-CONTRADICTED"),
        "a closed list that omits an instance of its own dimension is wrong"
    );
}

#[test]
fn a_partition_element_must_be_an_instance_of_its_dimension() {
    let source = r#"
fact F_A { claim: "a", evidence: user }
fact F_OUTSIDE { claim: "outside", evidence: user }
concept C_ALL { name: "all", grounded_by: [F_A] }
partition E_ONE { dimension: C_ALL, elements: [F_A, F_OUTSIDE], closure: open }
"#;
    assert!(codes(source).contains(&"E-PARTITION-ELEMENT"));
}

#[test]
fn a_closed_partition_needs_a_reason() {
    let source = r#"
fact F_A { claim: "a", evidence: user }
concept C_ALL { name: "all", grounded_by: [F_A] }
partition E_ONE { dimension: C_ALL, elements: [F_A], closure: closed }
"#;
    assert!(codes(source).contains(&"W-PARTITION-CLOSED-UNJUSTIFIED"));
}

#[test]
fn an_open_partition_used_as_a_premise_is_flagged() {
    let source = r#"
fact F_A { claim: "a", evidence: user }
concept C_ALL { name: "all", grounded_by: [F_A] }
partition E_ONE { dimension: C_ALL, elements: [F_A], closure: open }
proposition P_ONE { statement: "s", from: [E_ONE] }
"#;
    assert!(
        codes(source).contains(&"W-PARTITION-OPEN-USED"),
        "a list known to be open cannot silently carry a conclusion"
    );
}

const PILE_CONTEXT: &str = r#"
fact F_A { claim: "a", evidence: user }
fact F_B { claim: "b", evidence: user }
hypothesis H_A { assumes: [F_A], predicts: "p", test: "t" }
hypothesis H_B { assumes: [F_A], predicts: "q", rival: H_A }
question Q_X { asks: "which?", answers: ["a", "b"], separates: [H_A, H_B] }
"#;

#[test]
fn a_pile_without_a_type_may_be_written_but_not_used() {
    let named = format!(
        "{PILE_CONTEXT}\npartition E_PILE {{ elements: [F_A, F_B], closure: unknown, closes_with: [Q_X] }}"
    );
    assert!(codes(&named).contains(&"W-PARTITION-UNKNOWN"));
    assert!(!codes(&named)
        .iter()
        .any(|code| code.starts_with("E-PARTITION")));

    let used = format!(
        "{PILE_CONTEXT}\npartition E_PILE {{ elements: [F_A, F_B], closure: unknown, closes_with: [Q_X] }}\n\
         proposition P_USE {{ statement: \"s\", from: [E_PILE] }}"
    );
    assert!(codes(&used).contains(&"E-PARTITION-UNKNOWN-USED"));

    let no_exit =
        format!("{PILE_CONTEXT}\npartition E_PILE {{ elements: [F_A, F_B], closure: unknown }}");
    assert!(codes(&no_exit).contains(&"E-PARTITION-UNKNOWN-NO-EXIT"));
}

#[test]
fn closure_must_agree_with_having_a_dimension() {
    let source = r#"
fact F_A { claim: "a", evidence: user }
concept C_ONE { name: "one", grounded_by: [F_A] }
partition E_ONE { dimension: C_ONE, elements: [F_A], closure: unknown }
"#;
    assert!(codes(source).contains(&"E-PARTITION-CLOSURE"));

    let silent = r#"
fact F_A { claim: "a", evidence: user }
partition E_ONE { elements: [F_A], closure: closed }
"#;
    assert!(codes(silent).contains(&"E-PARTITION-CLOSURE"));
}

#[test]
fn a_partition_cannot_lean_on_a_dimension_whose_examples_collide() {
    let source = r#"
fact F_A { claim: "a", evidence: user }
fact F_B { claim: "b", evidence: user }
concept C_COLLIDE { name: "one", grounded_by: [F_A], against: [F_B] }
partition E_ONE { dimension: C_COLLIDE, elements: [F_A], closure: open }
"#;
    assert!(
        codes(source).contains(&"W-PARTITION-UNLEARNED"),
        "a type whose own examples no attribute separates is a name, not a type"
    );
}

#[test]
fn the_self_document_compiles_without_errors() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("SELF.tsare");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is missing", path.display()));
    let declared = parse(&text).expect("the self document parses");
    let errors: Vec<&str> = check(&declared)
        .iter()
        .filter(|item| item.code.starts_with("E-"))
        .map(|item| item.code)
        .collect();
    assert!(errors.is_empty(), "the self document reports {errors:?}");
    // and the concept it argues about must be learnable, not merely declared
    let program = Program::new(&declared);
    let learnings = learnings_of(&program);
    assert!(
        learnings
            .iter()
            .any(|item| item.concept == "C_REFUSE_NOW" && item.clause.is_some()),
        "the self document's concept has no learnable clause"
    );
}

#[test]
fn the_repository_document_compiles_and_learns() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("REPO.tsare");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is missing", path.display()));
    let declared = parse(&text).expect("docs/REPO.tsare parses");
    let errors: Vec<&str> = check(&declared)
        .iter()
        .filter(|item| item.code.starts_with("E-"))
        .map(|item| item.code)
        .collect();
    assert!(errors.is_empty(), "docs/REPO.tsare reports {errors:?}");
    let program = Program::new(&declared);
    let learnings = learnings_of(&program);
    assert!(
        learnings
            .iter()
            .any(|item| item.concept == "C_PROOF_CARRIER" && item.clause.is_some()),
        "the repository document concept has no learnable clause"
    );
}

#[test]
fn every_code_the_compiler_prints_has_an_explanation() {
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden");
    let mut codes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(&golden).expect("golden directory") {
        let text = std::fs::read_to_string(entry.expect("entry").path()).expect("golden file");
        // only diagnostic positions count: `error[CODE]:` or `warning[CODE]:`, never prose
        for marker in ["error[", "warning["] {
            let mut rest = text.as_str();
            while let Some(position) = rest.find(marker) {
                rest = &rest[position + marker.len()..];
                let Some(end) = rest.find(']') else { break };
                codes.insert(rest[..end].to_string());
            }
        }
    }
    assert!(
        codes.len() > 5,
        "the corpus should exercise more than a handful of codes"
    );
    let missing: Vec<&String> = codes
        .iter()
        .filter(|code| tsare::explain::explain(code).is_none())
        .collect();
    assert!(missing.is_empty(), "no explanation for {missing:?}");
    // and the table itself must answer for every code it lists
    for code in tsare::explain::CODES {
        assert!(
            tsare::explain::explain(code).is_some(),
            "{code} is listed but unexplained"
        );
    }
}

/// A form whose members differ in a position a concept could name, and no concept in the
/// document names it: the equation is there, the fill is missing.
#[test]
fn a_form_with_an_unnamed_dimension_is_a_hint() {
    let source = concat!(
        "fact F_A { claim: \"a\", evidence: user }\n",
        "fact F_B { claim: \"b\", evidence: model }\n",
    );
    let found = tsare::hint::hints(&Program::new(&decls(source)));
    assert_eq!(found.len(), 1);
    assert!(found[0].free.iter().any(|(field, _)| field == "evidence"));
    assert!(found[0].unnamed);
    assert!(
        found[0].template.contains("evidence: ?"),
        "{}",
        found[0].template
    );
    // a concept that names one member means the vocabulary has noticed the class
    let named = format!("{source}concept C {{ name: \"c\", grounded_by: [F_A] }}\n");
    let found = tsare::hint::hints(&Program::new(&decls(&named)));
    assert!(!found[0].unnamed);
}
