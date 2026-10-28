//! What the compiler decides, in the smallest form that can be checked by eye.
//!
//! These assertions came from the reference suite and were kept because they name a property,
//! not an implementation: a rule that attacks another defeats it, an ungrounded premise makes
//! a conclusion conditional, a duplicate is repaired only when the repair compiles better.

use std::collections::HashSet;

use tsare::argue::conclusions;
use tsare::build::{learnings_of, verify_predicates};
use tsare::check::check;
use tsare::closure::{engine, grounded_sets};
use tsare::cost::report as cost_report;
use tsare::derive::analyze;
use tsare::egraph::merge_groups;
use tsare::fix::suggestions;
use tsare::induce::induce_concept;
use tsare::program::Program;
use tsare::sugar::expand;
use tsare::syntax::{parse, Decl};

fn decls(source: &str) -> Vec<Decl> {
    parse(source).expect("parse")
}

fn codes(source: &str) -> Vec<&'static str> {
    check(&decls(source)).iter().map(|item| item.code).collect()
}

fn statuses(source: &str) -> std::collections::HashMap<String, &'static str> {
    let decls = decls(source);
    conclusions(&Program::new(&decls))
        .into_iter()
        .map(|row| (row.name, row.status))
        .collect()
}

const ATTACK: &str = r#"
fact F1 { claim: "x", evidence: user }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r1", from: [F1], to: P1 }
warrant W2 { claim: "r2", from: [F1], to: P1, attacks: [W1] }
"#;

#[test]
fn an_attack_defeats_its_target() {
    let parsed = decls(ATTACK);
    let (inside, outside) = grounded_sets(&parsed);
    assert!(inside.contains("W2"), "the attacking rule survives");
    assert!(outside.contains("W1"), "the attacked rule falls");
    assert_eq!(
        statuses(ATTACK)["P1"],
        "supported",
        "the other rule still concludes it"
    );
}

#[test]
fn a_rule_without_premises_licenses_nothing() {
    let source = r#"
fact F1 { claim: "x", evidence: user }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r", from: [], to: P1 }
"#;
    let (inside, outside) = grounded_sets(&decls(source));
    assert!(!inside.contains("W1"));
    assert!(
        outside.contains("W1"),
        "it is out from the start and cannot defeat anything"
    );
}

#[test]
fn a_supported_conclusion_without_facts_is_conditional() {
    let source = r#"
fact F1 { claim: "x", evidence: model }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r", from: [F1], to: P1 }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let result = analyze(&program);
    assert!(
        result.conditional.is_empty(),
        "F1 is a fact, so P1 is grounded"
    );

    let source = r#"
proposition P0 { statement: "unchecked" }
proposition P1 { statement: "a", from: [P0] }
warrant W1 { claim: "r", from: [P0], to: P1 }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let result = analyze(&program);
    assert_eq!(result.conditional, vec!["P1".to_string()]);
    assert!(
        result.reasons["P1"][0].contains("W1 needs P0"),
        "the reason names the missing premise"
    );
}

#[test]
fn engine_separates_held_from_conditional() {
    let source = r#"
fact F1 { claim: "x", evidence: tool("t") }
proposition P1 { statement: "a", from: [F1] }
proposition P2 { statement: "b", from: [P1, P3] }
warrant W1 { claim: "r1", from: [F1], to: P1 }
warrant W2 { claim: "r2", from: [P1, P3], to: P2 }
"#;
    let result = engine(&decls(source));
    assert!(result.held.contains(&"P1".to_string()));
    assert!(result.conditional.iter().any(|(name, _)| name == "P2"));
}

#[test]
fn a_model_fact_costs_one_confirmation() {
    let declared = decls(r#"fact F1 { claim: "x", evidence: model }"#);
    let cost = cost_report(&Program::new(&declared));
    assert_eq!(cost.total, 1);
    assert_eq!(cost.rows[0].0, "F1");
}

#[test]
fn a_measured_support_costs_nothing() {
    let source = r#"
fact F1 { claim: "x", evidence: tool("t") }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r", from: [F1], to: P1, basis: [V1] }
value V1 { metric: "m", direction: min, unit: "ms", threshold: 1, measured_by: tool("t") }
"#;
    let cost = cost_report(&Program::new(&decls(source)));
    assert_eq!(cost.total, 0);
}

#[test]
fn removing_the_only_premise_defeats_the_conclusion() {
    assert_eq!(statuses(ATTACK)["P1"], "supported");
    let declared = decls(ATTACK);
    let reduced = tsare::whatif::remove(&declared, "F1").expect("F1 is removable");
    let result = tsare::whatif::compare(&declared, &reduced);
    assert!(result.changed.iter().any(|(name, before, after)| {
        name == "P1" && before == "supported" && after != "supported"
    }));
}

#[test]
fn identical_facts_merge() {
    let source = r#"
fact F1 { claim: "x", evidence: user }
fact F2 { claim: "x", evidence: user }
"#;
    let declared = decls(source);
    let (_, groups) = merge_groups(&Program::new(&declared));
    let merged: Vec<&Vec<String>> = groups.iter().filter(|group| group.len() > 1).collect();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0], &vec!["F1".to_string(), "F2".to_string()]);
}

#[test]
fn a_duplicate_is_a_machine_repair() {
    let text = "fact F { claim: \"a\", evidence: user }\nfact F { claim: \"b\", evidence: user }\n";
    let declared = decls(text);
    let items = suggestions(&declared);
    let drop = items
        .iter()
        .find(|item| item.message.contains("drop the duplicate"))
        .expect("a duplicate can be dropped");
    assert_eq!(drop.applicability, "MachineApplicable");
    let (updated, applied) = tsare::apply::apply_best(text, &items);
    assert_eq!(applied.len(), 1);
    assert!(
        !updated.contains("\"b\""),
        "the later duplicate is the one that goes"
    );
    let before = tsare::apply::score(text);
    let after = tsare::apply::score(&updated);
    assert!(after < before, "the applied text compiles better");
}

#[test]
fn a_repair_that_does_not_improve_is_dropped() {
    let text = "fact F { claim: \"a\", evidence: user }\n";
    let declared = decls(text);
    let noop = tsare::fix::Suggestion {
        applicability: "MachineApplicable",
        message: "no-op".to_string(),
        file: String::new(),
        line: declared[0].line,
        col: declared[0].col,
        end_line: declared[0].end_line,
        end_col: declared[0].end_col,
        replacement: String::new(),
    };
    let (updated, applied) = tsare::apply::apply_best(text, &[noop]);
    assert!(applied.is_empty());
    assert_eq!(updated, text);
}

#[test]
fn the_checks_name_the_problem() {
    assert!(codes("").contains(&"E-EMPTY-DOCUMENT"));
    assert!(codes(
        "fact F1 { claim: \"x\", evidence: user }\nfact F1 { claim: \"y\", evidence: user }"
    )
    .contains(&"E-DUP-ID"));
    assert!(
        codes("proposition P1 { statement: \"a\", from: [F_MISSING] }").contains(&"E-UNDEFINED")
    );
    assert!(codes("fact F1 { claim: \"x\", evidence: nonsense }").contains(&"E-TYPE"));
    assert!(codes("nonsense N { x: 1 }").contains(&"E-KIND-UNKNOWN"));
}

#[test]
fn a_layer_leak_is_reported_where_the_reference_is_made() {
    let source = r#"
layer L_CORE { name: "core", exposes: [] }
layer L_APP { name: "app", exposes: [] }
fact F_INTERNAL { claim: "x", evidence: user, layer: L_CORE }
proposition P_LEAK { statement: "y", from: [F_INTERNAL], layer: L_APP }
"#;
    let diagnostics = check(&decls(source));
    let leak = diagnostics
        .iter()
        .find(|item| item.code == "E-LAYER-LEAK")
        .expect("a leak");
    assert!(
        leak.message.contains("P_LEAK"),
        "the declaration that reaches out is named"
    );
}

#[test]
fn a_concept_learns_a_clause_from_its_examples() {
    let source = r#"
fact F_SLOW { claim: "slow", evidence: tool("t"), attrs: { frame_ms: 900 } }
fact F_FAST { claim: "fast", evidence: tool("t"), attrs: { frame_ms: 8 } }
fact F_MEDIUM { claim: "medium", evidence: tool("t"), attrs: { frame_ms: 40 } }
concept C_FAST { name: "fast", grounded_by: [F_FAST, F_MEDIUM], against: [F_SLOW] }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let concept = program.of_kind("concept")[0];
    let learning = induce_concept(&program, concept);
    let clause = learning.clause.expect("a clause exists");
    let text = tsare::induce::clause_text(&clause);
    assert_eq!(
        text, "frame_ms <= 40",
        "the tightest bound the data supports"
    );
}

#[test]
fn generated_predicates_hold_on_every_example() {
    let source = r#"
fact F_SLOW { claim: "slow", evidence: tool("t"), attrs: { frame_ms: 900 } }
fact F_FAST { claim: "fast", evidence: tool("t"), attrs: { frame_ms: 8 } }
concept C_FAST { name: "fast", grounded_by: [F_FAST], against: [F_SLOW] }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let learnings = learnings_of(&program);
    let mut structurals = Vec::new();
    for concept in program.of_kind("concept") {
        structurals.push((
            concept.name.clone(),
            tsare::patterns::learn(
                &program,
                &tsare::program::pos(concept),
                &tsare::program::neg(concept),
            ),
        ));
    }
    let problems = verify_predicates(
        &program,
        &learnings
            .iter()
            .map(|item| {
                (
                    item.concept.clone(),
                    tsare::induce::Learning {
                        concept: item.concept.clone(),
                        positives: item.positives.clone(),
                        negatives: item.negatives.clone(),
                        clause: item.clause.clone(),
                        witness: item.witness.clone(),
                        reason: item.reason.clone(),
                    },
                )
            })
            .collect(),
        &structurals,
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn sugar_expands_one_line_into_one_declaration() {
    let source = "fact \"the window appears\" by user as F_WINDOW\n\
                  measure \"frame_ms\" min 16ms by tool(\"trace\") as V_FRAME\n\
                  because F_SCROLL rule \"one frame is the budget\" given V_FRAME => P_FAST\n";
    let expanded = expand(source);
    let lines: Vec<&str> = expanded.lines().collect();
    assert_eq!(
        lines[0],
        "fact F_WINDOW { claim: \"the window appears\", evidence: user }"
    );
    assert_eq!(
        lines[1],
        "value V_FRAME { metric: \"frame_ms\", direction: min, unit: \"ms\", threshold: 16, measured_by: tool(\"trace\") }"
    );
    assert_eq!(
        lines[2],
        "warrant W_L3 { claim: \"one frame is the budget\", from: [F_SCROLL], to: P_FAST, basis: [V_FRAME] }"
    );
    // the line count is unchanged, so spans still point at the line the author wrote
    assert_eq!(expanded.lines().count(), source.lines().count());
}

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
fn a_list_checks_its_elements() {
    let source = r#"
hypothesis H_A { assumes: [F], predicts: "a", test: "t" }
hypothesis H_B { assumes: [F], predicts: "b", rival: H_A }
question Q { asks: "q", answers: [H_A, H_B], separates: [H_A, H_B] }
fact F { claim: "x", evidence: user }
"#;
    assert!(
        codes(source).contains(&"E-TYPE"),
        "answers is list:str, so hypotheses are not answers"
    );
}

#[test]
fn an_example_that_shares_no_attribute_cannot_be_compared() {
    let source = r#"
fact F_A { claim: "a", evidence: user, attrs: { contrast: 7.2 } }
fact F_B { claim: "b", evidence: user, attrs: { contrast: 2.8 } }
fact F_C { claim: "c", evidence: user, attrs: { theme_ms: 40 } }
concept C_X { name: "x", grounded_by: [F_A], against: [F_B, F_C] }
"#;
    assert!(codes(source).contains(&"W-EXAMPLE-SHAPE"));
    assert!(!codes(source).is_empty());
}

#[test]
fn the_declared_inference_type_must_match_the_structure() {
    let deductive_without_rule = r#"
fact F { claim: "x", evidence: user }
concept C { name: "c", grounded_by: [F] }
proposition P { statement: "s", from: [F], uses: [C], by: deductive }
"#;
    assert!(codes(deductive_without_rule).contains(&"W-BY-MISMATCH"));

    let inductive_with_rule = r#"
fact F { claim: "x", evidence: user }
proposition P { statement: "s", from: [F], by: inductive }
warrant W { claim: "r", from: [F], to: P, basis: [F] }
"#;
    assert!(codes(inductive_with_rule).contains(&"W-BY-MISMATCH"));
}

#[test]
fn a_number_cannot_exceed_its_support() {
    let proposition = r#"
fact F { claim: "x", evidence: tool("t"), confidence: 0.9 }
proposition P { statement: "s", from: [F], confidence: 0.99 }
"#;
    assert!(codes(proposition).contains(&"W-CONFIDENCE-INFLATED"));

    let warrant = r#"
fact F { claim: "x", evidence: tool("t"), confidence: 0.5 }
proposition P { statement: "s", from: [F] }
warrant W { claim: "r", from: [F], to: P, basis: [F], confidence: 0.9 }
"#;
    assert!(codes(warrant).contains(&"W-CONFIDENCE-INFLATED"));

    let honest = r#"
fact F { claim: "x", evidence: tool("t"), confidence: 0.9 }
proposition P { statement: "s", from: [F], confidence: 0.8 }
"#;
    assert!(!codes(honest).contains(&"W-CONFIDENCE-INFLATED"));
}

#[test]
fn every_kind_owes_an_action() {
    use tsare::program::TABLE;
    assert_eq!(TABLE.len(), 12, "one row per kind");
    for (kind, obligation, discharge) in TABLE {
        assert!(
            tsare::specs::spec(kind).is_some(),
            "{kind} has no field spec"
        );
        assert!(
            !obligation.is_empty() && !discharge.is_empty(),
            "{kind} owes nothing?"
        );
        assert_ne!(
            tsare::build::action_of(kind),
            "?",
            "{kind} has no action in OBLIGATIONS.md"
        );
    }
}

#[test]
fn a_warrant_is_ready_only_with_a_basis() {
    let with = decls(
        "fact F { claim: \"x\", evidence: user }\n\
         proposition P { statement: \"s\", from: [F] }\n\
         warrant W1 { claim: \"r\", from: [F], to: P, basis: [F] }\n",
    );
    let program = Program::new(&with);
    let row = tsare::build::obligation(program.get("W1").unwrap(), &program, &Default::default());
    assert_eq!((row.action, row.state), ("justify", "ready"));

    let without = decls(
        "fact F { claim: \"x\", evidence: user }\n\
         proposition P { statement: \"s\", from: [F] }\n\
         warrant W1 { claim: \"r\", from: [F], to: P }\n",
    );
    let program = Program::new(&without);
    let row = tsare::build::obligation(program.get("W1").unwrap(), &program, &Default::default());
    assert_eq!(row.state, "blocked");
}

#[test]
fn a_document_declares_a_format_only_when_it_must() {
    // no header: the first format, which is every document written before the line existed
    assert!(parse("fact F { claim: \"x\", evidence: user }").is_ok());

    // the header is read and consumed, and it does not shift a single line number
    let declared = parse("tsare 1\nfact F { claim: \"x\", evidence: user }").expect("parse");
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].line, 2);

    // a newer format stops parsing: one true error beats a page of nonsense
    let error = parse("tsare 2\nfact F { claim: \"x\", evidence: user }").unwrap_err();
    assert_eq!(error.code, "E-VERSION");
    assert!(error.message.contains("format 2"), "{}", error.message);
    assert_eq!((error.line, error.col), (1, 1));

    // a header with no number is still a header
    let error = parse("tsare\nfact F { claim: \"x\", evidence: user }").unwrap_err();
    assert_eq!(error.code, "E-VERSION");

    // and a trailing comment on the line is allowed
    assert!(parse("tsare 1 # the current format\nfact F { claim: \"x\", evidence: user }").is_ok());
}

#[test]
fn a_law_with_nothing_refining_its_layer_is_unbacked() {
    let source = r#"
layer L_UP { name: "the interface", exposes: [F_A] }
layer L_DOWN { name: "the implementation", exposes: [F_B] }
fact F_A { claim: "a", evidence: tool("t") }
fact F_B { claim: "b", evidence: tool("t") }
law L_A { statement: "a holds", holds_in: L_UP, checked_by: [F_A] }
"#;
    assert!(codes(source).contains(&"W-UNREFINED"));

    let backed = r#"
layer L_UP { name: "the interface", exposes: [F_A] }
layer L_DOWN { name: "the implementation", exposes: [F_B] }
fact F_A { claim: "a", evidence: tool("t") }
fact F_B { claim: "b", evidence: tool("t") }
law L_A { statement: "a holds", holds_in: L_UP, checked_by: [F_A] }
law L_B { statement: "b implements a", holds_in: L_DOWN, checked_by: [F_B], refines: L_UP }
"#;
    assert!(!codes(backed).contains(&"W-UNREFINED"));
}

#[test]
fn a_kernel_refuses_a_claim_that_stands_on_none_of_its_laws() {
    let kernel = decls(
        r#"
layer L { name: "governed", exposes: [F_A] }
fact F_A { claim: "a", evidence: tool("t") }
law K { statement: "a claim needs a ground", holds_in: L, checked_by: [F_A] }
"#,
    );
    let governed = decls(
        r#"
proposition P { statement: "a claim with no principle behind it", from: [F_A], layer: L }
"#,
    );
    let found = tsare::kernel::check(&kernel, &governed);
    assert!(
        found.iter().any(|item| item.code == "W-UNPRINCIPLED"),
        "{found:?}"
    );

    let standing = decls(
        r#"
proposition P { statement: "a claim on a principle", from: [K], layer: L }
"#,
    );
    let found = tsare::kernel::check(&kernel, &standing);
    assert!(
        !found.iter().any(|item| item.code == "W-UNPRINCIPLED"),
        "{found:?}"
    );

    let shadow =
        decls(r#"law K { statement: "a different rule", holds_in: L, checked_by: [F_A] }"#);
    let found = tsare::kernel::check(&kernel, &shadow);
    assert!(
        found.iter().any(|item| item.code == "E-KERNEL-SHADOW"),
        "{found:?}"
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
fn a_syntax_error_is_a_diagnostic_with_a_span() {
    let error =
        parse("fact F { claim: \"unterminated\nevidence: user }").expect_err("must not parse");
    assert_eq!(error.line, 1);
    assert!(error.col > 10, "the caret points at the opening quote");
    assert!(error.message.contains("unterminated"));

    let error = parse("fact F { claim: \"x\", evidence: user").expect_err("missing brace");
    assert!(!error.message.is_empty(), "the error must say something");
    assert!(error.line >= 1 && error.col >= 1);
}

#[test]
fn parsing_reports_every_error_in_one_run() {
    let source = "fact F_OK { claim: \"x\", evidence: user }\n\
                  proposition P_BROKEN { statement: \"unterminated\n\
                  fact F_AFTER { claim: \"still parses\", evidence: user }\n\
                  concept C_BAD { name: \"c\" grounded_by: [F_OK] }\n";
    let (decls, errors) = tsare::syntax::parse_recovering(source);
    assert_eq!(
        errors.len(),
        3,
        "unterminated string, the field that follows it, the missing comma: {errors:?}"
    );
    let names: Vec<&str> = decls.iter().map(|decl| decl.name.as_str()).collect();
    assert!(
        names.contains(&"F_OK"),
        "a good declaration before the error survives"
    );
    assert!(
        names.contains(&"F_AFTER"),
        "parsing resumes at the next line-initial keyword"
    );
    assert!(
        !names.contains(&"C_BAD"),
        "the broken declaration is dropped, not half-read"
    );
}

#[test]
fn a_missing_separator_is_an_error() {
    let error = parse("fact F { claim: \"x\" evidence: user }").expect_err("no comma");
    assert!(
        error.message.contains(','),
        "the message names the separator: {}",
        error.message
    );
}

#[test]
fn a_session_owns_order_counts_and_shape() {
    let mut session = tsare::session::Session::new("doc.tsare");
    session.emit(tsare::check::Diagnostic {
        code: "W-HOLE",
        message: "later".to_string(),
        line: 9,
        col: 1,
        end_line: 9,
        end_col: 2,
        file: String::new(),
        witness: None,
    });
    session.emit(tsare::check::Diagnostic {
        code: "E-TYPE",
        message: "earlier".to_string(),
        line: 2,
        col: 5,
        end_line: 2,
        end_col: 9,
        file: String::new(),
        witness: None,
    });
    assert_eq!(
        (session.errors(), session.warnings(), session.exit_code()),
        (1, 1, 1)
    );
    assert_eq!(session.summary(), "1 error(s), 1 warning(s)");
    let json = session.json();
    assert!(json.contains("\"code\": \"E-TYPE\"") && json.contains("\"code\": \"W-HOLE\""));
    assert!(json.contains("\"errors\": 1") && json.contains("\"warnings\": 1"));
    assert!(
        json.find("E-TYPE").unwrap() < json.find("W-HOLE").unwrap(),
        "sorted by position"
    );
    // with the source, the text form is the rendered form
    let rendered = tsare::session::Session::new("doc.tsare")
        .with_source("x\nfact F { claim: \"y\" }\n")
        .text();
    assert!(rendered.is_empty(), "an empty session prints nothing");
}

#[test]
fn a_directory_is_rendered_file_by_file() {
    let sources = vec![
        (
            "a.tsare".to_string(),
            "fact F_A { claim: \"x\" }\n".to_string(),
        ),
        (
            "b.tsare".to_string(),
            "fact F_B { claim: \"y\" }\n".to_string(),
        ),
    ];
    let mut session = tsare::session::Session::new("dir").with_sources(&sources);
    session.emit(tsare::check::Diagnostic {
        code: "E-TYPE",
        message: "b is wrong".to_string(),
        line: 1,
        col: 18,
        end_line: 1,
        end_col: 21,
        file: "b.tsare".to_string(),
        witness: None,
    });
    session.emit(tsare::check::Diagnostic {
        code: "E-MISSING",
        message: "a is incomplete".to_string(),
        line: 1,
        col: 1,
        end_line: 1,
        end_col: 4,
        file: "a.tsare".to_string(),
        witness: None,
    });
    let text = session.text();
    assert!(text.contains("b.tsare:1:18: error[E-TYPE]"), "{text}");
    assert!(text.contains("a.tsare:1:1: error[E-MISSING]"), "{text}");
    assert!(
        text.contains("claim"),
        "the source line is printed for each file: {text}"
    );
    assert!(
        text.find("a.tsare").unwrap() < text.find("b.tsare").unwrap(),
        "files in load order"
    );
}

#[test]
fn a_document_can_arrive_on_standard_input() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_tsare"))
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn the compiler");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all("fact F { claim: \"x\", evidence: user }\n".as_bytes())
        .expect("write to stdin");
    let output = child.wait_with_output().expect("wait");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("0 error(s), 0 warning(s)"), "{text}");
    assert_eq!(output.status.code(), Some(0));
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

#[test]
fn a_declaration_keeps_its_positions() {
    let declared = decls("fact F1 { claim: \"x\", evidence: user }\n");
    assert_eq!(declared[0].line, 1);
    assert_eq!(declared[0].col, 1);
    assert_eq!(
        declared[0].field_span[0],
        (1, 18, 1, 21),
        "the claim value runs from col 18 to 21"
    );
    let reference = decls("proposition P1 { statement: \"a\", from: [F_MISSING] }");
    assert_eq!(reference[0].ref_pos[0].0, "F_MISSING");
    assert_eq!(
        reference[0].ref_pos[0].2, 41,
        "the reference is where the name was written"
    );
}

#[test]
fn the_content_hash_is_a_function_of_the_bytes() {
    assert_eq!(
        tsare::hash::hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_ne!(tsare::hash::hex(b"abc"), tsare::hash::hex(b"abd"));
}

#[test]
fn the_store_reader_reads_the_records_it_wrote() {
    let line = r#"{"doc": "a.tsare", "codes": ["E-UNDEFINED", "W-HOLE"], "action": "mechanical", "before": {"errors": 2, "cost": 1}, "after": {"errors": 1, "cost": 1}, "checker": "tsare-0.1"}"#;
    let entry = tsare::json::parse(line).expect("parse");
    assert_eq!(
        entry
            .get("codes")
            .map(|value| value.strings())
            .unwrap_or_default(),
        vec!["E-UNDEFINED", "W-HOLE"]
    );
    assert!(tsare::cases::improved(&entry));
    assert_eq!(
        tsare::json::serialize(&entry),
        r#"{"doc": "a.tsare", "codes": ["E-UNDEFINED", "W-HOLE"], "action": "mechanical", "before": {"errors": 2, "cost": 1}, "after": {"errors": 1, "cost": 1}, "checker": "tsare-0.1"}"#
    );
    assert_eq!(
        tsare::cases::list_repr(&["A".to_string(), "B".to_string()]),
        "['A', 'B']"
    );
}

#[test]
fn names_resolve_once_per_program() {
    let source = "fact F1 { claim: \"x\", evidence: user }\nproposition P1 { statement: \"a\", from: [F1] }\n";
    let declared = decls(source);
    let program = Program::new(&declared);
    let names: HashSet<&str> = program
        .of_kind("fact")
        .iter()
        .map(|decl| decl.name.as_str())
        .collect();
    assert_eq!(names, HashSet::from(["F1"]));
    assert!(program.get("P1").is_some());
    assert!(program.get("P2").is_none());
}

/// The projection quotes the sentence and keeps everything that makes it checkable beside it.
/// A ready obligation owes nothing, so its note is not repeated; the blocked one carries the
/// reason, which is what the margin is for.
#[test]
fn a_projection_quotes_the_sentence_and_keeps_its_ground_in_the_margin() {
    let declared = decls(
        "layer L_X { name: \"shipping\", exposes: [P_A] }\nfact F_A { claim: \"the build is green\", evidence: model }\nproposition P_A { statement: \"it is ready\", from: [F_A], layer: L_X }\n",
    );
    let program = Program::new(&declared);
    let text = tsare::outline::markdown(&program, &check(program.decls), "t.tsare");
    assert!(text.contains("> the build is green"), "{text}");
    assert!(text.contains("## L_X"), "{text}");
    assert!(text.contains("evidence: model"), "{text}");
    assert!(
        text.contains("obligation: check (blocked), model-supplied, not confirmed"),
        "{text}"
    );
    assert!(text.contains("! W-UNCONFIRMED"), "{text}");
    // a field is never paraphrased, and the counted tail is part of the document
    assert!(text.contains("> it is ready"), "{text}");
    assert!(text.contains("obligation(s),"), "{text}");
}

/// A law may name what a citation costs.  K_NUMBER requires an instrument, and no kind that can
/// cite a law even has `measured_by`, so the citation is refused rather than trusted: a principle
/// that cannot be satisfied by pointing at it has to be satisfied by writing the thing down.
/// K_CONF requires a field a warrant does carry, and the warrant that carries it pays nothing.
#[test]
fn a_law_that_names_a_requirement_charges_for_being_cited() {
    let declared = decls(concat!(
        "layer L { name: \"l\", exposes: [P_A] }\n",
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "proposition P_A { statement: \"s\", from: [F_A], layer: L }\n",
        "proposition P_BARE { statement: \"the button is blue\", from: [K_NUMBER] }\n",
        "law K_NUMBER { statement: \"a number needs an instrument\", holds_in: L, checked_by: [F_A], requires: [\"measured_by\"] }\n",
        "law K_CONF { statement: \"a rule carries its confidence\", holds_in: L, checked_by: [F_A], requires: [\"confidence\"] }\n",
        "warrant W_PAID { claim: \"r\", from: [F_A], to: P_A, basis: [K_CONF], confidence: 0.5 }\n",
    ));
    let found = check(&declared);
    let unpaid: Vec<&str> = found
        .iter()
        .filter(|item| item.code == "W-UNSATISFIED")
        .map(|item| item.message.as_str())
        .collect();
    assert_eq!(unpaid.len(), 1, "{unpaid:#?}");
    assert!(unpaid[0].contains("P_BARE") && unpaid[0].contains("measured_by"));
    let text = tsare::explain::explain("W-UNSATISFIED").unwrap_or_default();
    assert!(text.contains("requirement"), "{text}");
}

/// A verdict is a claim about the strength of evidence, so a model's own assessment cannot supply
/// it.  The fact stays legal -- it is still a fact the document holds -- but it cannot settle a
/// hypothesis: deciding needs a witness that is not the thing being judged.
#[test]
fn a_model_cannot_settle_a_hypothesis_by_agreeing_with_itself() {
    let base = concat!(
        "fact F_MODEL { claim: \"the new path is faster\", evidence: model }\n",
        "fact F_TOOL { claim: \"the new path is faster\", evidence: tool(\"bench.sh\") }\n",
        "hypothesis H_ONE { assumes: [F_MODEL], predicts: \"a\", verdict: supported, verdict_by: F_MODEL }\n",
        "hypothesis H_TWO { assumes: [F_TOOL], predicts: \"b\", verdict: supported, verdict_by: F_TOOL }\n",
    );
    let found = check(&decls(base));
    let self_verdict: Vec<&str> = found
        .iter()
        .filter(|item| item.code == "E-SELF-VERDICT")
        .map(|item| item.message.as_str())
        .collect();
    assert_eq!(self_verdict.len(), 1, "{self_verdict:#?}");
    assert!(self_verdict[0].contains("H_ONE") && self_verdict[0].contains("F_MODEL"));
    // a person who looked is a witness
    let confirmed = concat!(
        "fact F_MODEL { claim: \"x\", evidence: model, confirmed_by: \"user\" }\n",
        "hypothesis H_ONE { assumes: [F_MODEL], predicts: \"a\", verdict: refuted, verdict_by: F_MODEL }\n",
    );
    assert!(!check(&decls(confirmed))
        .iter()
        .any(|item| item.code == "E-SELF-VERDICT"));
}

/// A threshold nothing reads cannot change a decision.  It is the number-shaped version of a fact
/// no conclusion depends on: the document pays a reader's attention for it and gets nothing back.
#[test]
fn a_bound_no_rule_reads_is_not_a_decision() {
    let unread = concat!(
        "value V_CONTRAST { metric: \"contrast\", direction: min, threshold: 4.5, measured_by: \"checker\" }\n",
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "proposition P_A { statement: \"it passes\", from: [F_A] }\n",
    );
    let found = check(&decls(unread));
    assert_eq!(
        found.iter().filter(|item| item.code == "W-UNREAD").count(),
        1
    );
    // cited by the rule it constrains: no longer dead
    let read = concat!(
        "value V_CONTRAST { metric: \"contrast\", direction: min, threshold: 4.5, measured_by: \"checker\" }\n",
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "warrant W_A { claim: \"readable text needs 4.5\", from: [F_A], basis: [V_CONTRAST] }\n",
    );
    assert!(!check(&decls(read))
        .iter()
        .any(|item| item.code == "W-UNREAD"));
}

/// `out` is what the instrument produced.  A fact naming an artifact whose bytes hash to something
/// else is not evidence: the run that produced this document is not the run that produced the file.
/// A hash with nothing to compare is unverified, which is a different thing from wrong.
#[test]
fn an_artifact_that_does_not_match_its_hash_is_refused() {
    let dir = std::env::temp_dir().join("tsare-artifact");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("out.log");
    std::fs::write(&path, b"3 runs, all within threshold\n").expect("write artifact");
    let shown = path.display().to_string().replace('\\', "/");
    let actual = tsare::hash::hex(b"3 runs, all within threshold\n");

    let matching = format!(
        "fact F_OK {{ claim: \"three runs\", evidence: tool(\"bench.sh\"), artifact: \"{shown}\", out: \"{actual}\" }}\n"
    );
    let found = check(&decls(&matching));
    assert!(
        !found
            .iter()
            .any(|item| item.code.starts_with('E') || item.code == "W-OUT-UNVERIFIED"),
        "{found:#?}"
    );

    let wrong = matching.replace(
        &actual,
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert!(check(&decls(&wrong))
        .iter()
        .any(|item| item.code == "E-OUT-MISMATCH"));

    let unverified =
        format!("fact F_ONE {{ claim: \"x\", evidence: tool(\"bench.sh\"), out: \"{actual}\" }}\n");
    assert!(check(&decls(&unverified))
        .iter()
        .any(|item| item.code == "W-OUT-UNVERIFIED"));
}

/// A universal claim is bound only when its domain is an extent the engine can list.  Naming the
/// domain in prose is not binding it: `scope:` is a string, and a string cannot be enumerated.
/// `over:` points at something with an extent -- a concept's positives, a closed partition's
/// elements, a layer's declarations -- and anything else is a free variable with a name.
#[test]
fn a_universal_claim_is_bound_only_to_an_enumerable_extent() {
    let bound = concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "concept C_ALL { name: \"everything counted\", grounded_by: [F_A] }\n",
        "fact F_UNIVERSAL { claim: \"all of them hold\", evidence: user, over: [C_ALL] }\n",
    );
    assert!(!check(&decls(bound))
        .iter()
        .any(|item| item.code == "W-OVERCLAIM" || item.code == "E-SCOPE-UNBOUNDED"));

    // a prose scope is not a binding
    let named =
        "fact F_UNIVERSAL { claim: \"all of them hold\", evidence: user, scope: \"everything\" }\n";
    assert!(check(&decls(named))
        .iter()
        .any(|item| item.code == "W-OVERCLAIM"));

    // a fact is not a domain
    let wrong_kind = concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "fact F_UNIVERSAL { claim: \"all of them hold\", evidence: user, over: [F_A] }\n",
    );
    assert!(check(&decls(wrong_kind))
        .iter()
        .any(|item| item.code == "E-SCOPE-UNBOUNDED"));

    // an open list cannot bound a universal; a closed one can
    let open = concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "concept C_ALL { name: \"c\", grounded_by: [F_A] }\n",
        "partition P_OPEN { dimension: C_ALL, elements: [F_A], closure: open }\n",
        "fact F_UNIVERSAL { claim: \"all of them hold\", evidence: user, over: [P_OPEN] }\n",
    );
    assert!(check(&decls(open))
        .iter()
        .any(|item| item.code == "E-SCOPE-UNBOUNDED"));
    let closed = open.replace("closure: open", "closure: closed");
    assert!(!check(&decls(&closed))
        .iter()
        .any(|item| item.code == "E-SCOPE-UNBOUNDED"));

    // a universal proposition has no `scope:` field at all, so `over:` is its only binding
    let proposition = concat!(
        "concept C_ALL { name: \"c\", grounded_by: [F_A] }\n",
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "proposition P_ALL { statement: \"all of them hold\", from: [F_A] }\n",
    );
    assert!(check(&decls(proposition))
        .iter()
        .any(|item| item.code == "W-OVERCLAIM" && item.message.contains("P_ALL")));
}

/// An abstraction is proposed only when it covers two facts, is not already a named concept, and
/// separates one fact from another.  The last condition is what keeps a proposal from being a
/// prettier name for "all of these": a concept whose extent is everything distinguishes nothing.
#[test]
fn an_abstraction_is_proposed_only_when_it_separates_something() {
    let source = concat!(
        "fact F_1 { claim: \"a\", evidence: tool(\"t\"), attrs: { size: 3, kind: first, region: east } }\n",
        "fact F_2 { claim: \"b\", evidence: tool(\"t\"), attrs: { size: 3, kind: first, region: west } }\n",
        "fact F_3 { claim: \"c\", evidence: tool(\"t\"), attrs: { size: 3, kind: second, region: east } }\n",
        "fact F_4 { claim: \"d\", evidence: tool(\"t\"), attrs: { size: 9, kind: first, region: east } }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);
    let (found, truncated) = tsare::abstraction::candidates(&program, 2, 3);
    assert!(!truncated);
    let pair = found
        .iter()
        .find(|candidate| candidate.intent.len() == 2)
        .expect("a two-attribute class exists");
    assert_eq!(pair.extent, vec!["F_1".to_string(), "F_2".to_string()]);
    // the witness is a pair, not a count
    assert!(pair.extent.contains(&pair.inside));
    assert!(!pair.extent.contains(&pair.outside));

    // naming it already removes it from the proposals
    let named = format!("{source}concept C_ONE {{ name: \"one\", grounded_by: [F_1, F_2] }}\n");
    let declared = decls(&named);
    let program = Program::new(&declared);
    let (after, _) = tsare::abstraction::candidates(&program, 2, 3);
    assert!(!after
        .iter()
        .any(|candidate| candidate.extent == vec!["F_1".to_string(), "F_2".to_string()]));

    // the proposal pastes back clean: it has an extent, a counterexample and a clause
    let pasted = format!(
        "{source}concept C_PROPOSED_1 {{ name: \"size = 3 and kind = first\", grounded_by: [F_1, F_2], against: [F_3] }}\n"
    );
    let found = check(&decls(&pasted));
    assert!(
        !found
            .iter()
            .any(|item| item.code == "E-COLLAPSE" || item.code == "W-UNTESTED"),
        "{found:#?}"
    );
}

/// Retirement appends, it does not delete: the old declaration stays, references to it still
/// resolve, and everything that leaned on it loses its ground.  This is the minimal memory --
/// no storage, no query language, just "what was replaced, and what falls with it".
#[test]
fn a_retired_fact_takes_what_leaned_on_it_down_with_it() {
    let source = concat!(
        "fact F_OLD { claim: \"latency is 80 ms\", evidence: tool(\"t\") }\n",
        "fact F_NEW { claim: \"latency is 140 ms after the fix\", evidence: tool(\"t\"), supersedes: [F_OLD] }\n",
        "proposition P_FAST { statement: \"we are under the budget\", from: [F_OLD] }\n",
        "warrant W_FAST { claim: \"80 ms is under budget\", from: [F_OLD], to: P_FAST, basis: [F_OLD] }\n",
    );
    let declared = decls(source);
    // the reference resolves: withdrawal is an append, not a delete
    assert!(!check(&declared)
        .iter()
        .any(|item| item.code == "E-UNDEFINED"));
    // but the ground is gone, and the checker says so
    let found = check(&declared);
    let retired: Vec<&str> = found
        .iter()
        .filter(|item| item.code == "W-RETIRED")
        .map(|item| item.message.as_str())
        .collect();
    assert_eq!(retired.len(), 2, "{retired:#?}");
    assert!(retired.iter().all(|message| message.contains("F_OLD")));

    // the engine drops it: the conclusion no longer holds
    let program = Program::new(&declared);
    let result = tsare::closure::engine(&declared);
    assert!(
        result
            .conditional
            .iter()
            .any(|(name, why)| name == "P_FAST" && why.iter().any(|line| line.contains("retired"))),
        "{:#?}",
        result.conditional
    );
    // and without the retirement the same document holds
    let plain = source.replace(", supersedes: [F_OLD]", "");
    let held = tsare::closure::engine(&decls(&plain));
    assert!(!held.conditional.iter().any(|(name, _)| name == "P_FAST"));
    let _ = program;
}

/// A basis is a licence, not a citation.  Each kind has one criterion -- the same one it owes
/// elsewhere -- and a note, a question or a hypothesis is never a licence.
#[test]
fn a_basis_must_meet_its_own_kind_criterion() {
    let licence = concat!(
        "fact F_MEASURED { claim: \"a\", evidence: tool(\"t\") }\n",
        "fact F_GUESS { claim: \"b\", evidence: model }\n",
        "note N_A { tag: hunch, text: \"not a licence\" }\n",
        "warrant W_OK { claim: \"r\", from: [F_MEASURED], basis: [F_MEASURED] }\n",
    );
    let found = check(&decls(licence));
    assert!(
        !found.iter().any(|item| item.code == "W-BASIS-NO-CRITERION"),
        "{found:#?}"
    );

    for (bad, needle) in [
        (
            "note N_B { tag: hunch, text: \"t\" }",
            "note is not a licence",
        ),
        ("fact F_GUESS { claim: \"b\", evidence: model }", "model"),
    ] {
        let name = if bad.starts_with("note") {
            "N_B"
        } else {
            "F_GUESS"
        };
        let source = format!(
            "fact F_MEASURED {{ claim: \"a\", evidence: tool(\"t\") }}\n{bad}\nwarrant W_BAD {{ claim: \"r\", from: [F_MEASURED], basis: [{name}] }}\n"
        );
        let found = check(&decls(&source));
        assert!(
            found
                .iter()
                .any(|item| item.code == "W-BASIS-NO-CRITERION" && item.message.contains(needle)),
            "{source}\n{found:#?}"
        );
    }
}

/// A verified repair may create the declaration it needs -- adding a rule to connect a
/// proposition creates the rule, and the rule can owe something of its own.  What it may not do is
/// put a new complaint on a declaration that was already there.  A total score cannot tell those
/// two apart, which is the whole reason the judgment moved to (declaration, code) pairs.
#[test]
fn a_verified_repair_does_not_borrow_trouble() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let pairs = |list: &[Decl]| -> HashSet<(String, &'static str)> {
        check(list)
            .into_iter()
            .filter(|item| item.code != "W-HOLE")
            .filter_map(|item| item.witness.map(|name| (name, item.code)))
            .collect()
    };
    for entry in std::fs::read_dir(&root).expect("fixtures") {
        let path = entry.expect("entry").path();
        if path.extension().map(|ext| ext != "tsare").unwrap_or(true) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read a fixture");
        let declared = decls(&source);
        let known: HashSet<&str> = declared.iter().map(|item| item.name.as_str()).collect();
        let before = pairs(&declared);
        let (accepted, _) = tsare::fix::verified(&declared);
        for fix in accepted {
            let after = pairs(&fix.trial);
            for (declaration, code) in after.difference(&before) {
                assert!(
                    !known.contains(declaration.as_str()),
                    "{}: {} adds {code} to the existing {declaration}",
                    path.display(),
                    fix.description
                );
            }
        }
    }
}

/// A quantity with no instrument is an opinion wearing a number's clothes; a quantity that cites
/// the value that measured it is not.  Only digits with a unit count, because `A1` and `0.4.3` are
/// names rather than quantities -- variant A fired on four of those and variant B on none.
#[test]
fn a_stated_quantity_needs_the_value_that_measured_it() {
    let bare =
        "proposition P_TIME { statement: \"frame times stay under 16 ms while scrolling\" }\n";
    assert!(
        check(&decls(bare))
            .iter()
            .any(|item| item.code == "W-BARE-NUMBER"),
        "a quantity with no instrument must warn"
    );

    let cited = concat!(
        "value V_FRAME { metric: \"frame time\", unit: ms, threshold: 16, measured_by: tool(\"trace\") }\n",
        "proposition P_TIME { statement: \"frame times stay under 16 ms while scrolling\", from: [V_FRAME] }\n",
    );
    let found = check(&decls(cited));
    assert!(
        !found.iter().any(|item| item.code == "W-BARE-NUMBER"),
        "citing the measured value must silence it: {found:#?}"
    );

    // a name that happens to contain a digit is not a quantity
    let named = concat!(
        "fact F_A1 { claim: \"a\", evidence: user }\n",
        "proposition P_ONE { statement: \"A1 既支持又担保这条规则\", from: [F_A1] }\n",
    );
    assert!(
        !check(&decls(named))
            .iter()
            .any(|item| item.code == "W-BARE-NUMBER"),
        "an identifier is not a measurement"
    );
}

/// A retirement is a decision, so it says why -- in the same place every other decision does.
/// A warning rather than an error: an old document that withdrew something is still readable.
#[test]
fn a_withdrawal_that_gives_no_reason_is_reported_but_not_refused() {
    let bare = concat!(
        "fact F_OLD { claim: \"old\", evidence: user }\n",
        "fact F_NEW { claim: \"new\", evidence: user, supersedes: [F_OLD] }\n",
    );
    let found = check(&decls(bare));
    assert!(
        found.iter().any(|item| item.code == "W-RETIRED-UNBASED"),
        "{found:#?}"
    );
    assert!(
        !found.iter().any(|item| item.code.starts_with("E-")),
        "a missing reason is not an error: {found:#?}"
    );

    let reasoned = concat!(
        "fact F_OLD { claim: \"old\", evidence: user }\n",
        "fact F_WHY { claim: \"the instrument was wrong\", evidence: user }\n",
        "fact F_NEW { claim: \"new\", evidence: user, supersedes: [F_OLD], because: [F_WHY] }\n",
    );
    assert!(
        !check(&decls(reasoned))
            .iter()
            .any(|item| item.code == "W-RETIRED-UNBASED"),
        "a stated reason silences it"
    );

    let dangling = concat!(
        "fact F_OLD { claim: \"old\", evidence: user }\n",
        "fact F_NEW { claim: \"new\", evidence: user, supersedes: [F_OLD], because: [NOPE] }\n",
    );
    assert!(
        check(&decls(dangling))
            .iter()
            .any(|item| item.code == "E-UNDEFINED"),
        "a reason that names nothing is an error"
    );
}
