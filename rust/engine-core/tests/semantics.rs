//! What the compiler decides, in the smallest form that can be checked by eye.
//!
//! These assertions came from the reference suite and were kept because they name a property,
//! not an implementation: a rule that attacks another defeats it, an ungrounded premise makes
//! a conclusion conditional, a duplicate is repaired only when the repair compiles better.

use std::collections::HashSet;

use engine_core::argue::conclusions;
use engine_core::build::{learnings_of, verify_predicates};
use engine_core::check::check;
use engine_core::closure::{engine, grounded_sets};
use engine_core::cost::report as cost_report;
use engine_core::derive::analyze;
use engine_core::egraph::merge_groups;
use engine_core::fix::suggestions;
use engine_core::induce::induce_concept;
use engine_core::program::Program;
use engine_core::sugar::expand;
use engine_core::syntax::{parse, Decl};

fn decls(source: &str) -> Vec<Decl> {
    parse(source).expect("parse")
}

fn codes(source: &str) -> Vec<&'static str> {
    check(&decls(source)).iter().map(|item| item.code).collect()
}

fn statuses(source: &str) -> std::collections::HashMap<String, &'static str> {
    let decls = decls(source);
    conclusions(&Program::new(&decls)).into_iter().map(|row| (row.name, row.status)).collect()
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
    assert_eq!(statuses(ATTACK)["P1"], "supported", "the other rule still concludes it");
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
    assert!(outside.contains("W1"), "it is out from the start and cannot defeat anything");
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
    assert!(result.conditional.is_empty(), "F1 is a fact, so P1 is grounded");

    let source = r#"
proposition P0 { statement: "unchecked" }
proposition P1 { statement: "a", from: [P0] }
warrant W1 { claim: "r", from: [P0], to: P1 }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let result = analyze(&program);
    assert_eq!(result.conditional, vec!["P1".to_string()]);
    assert!(result.reasons["P1"][0].contains("W1 needs P0"), "the reason names the missing premise");
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
    let reduced = engine_core::whatif::remove(&declared, "F1").expect("F1 is removable");
    let result = engine_core::whatif::compare(&declared, &reduced);
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
    let (updated, applied) = engine_core::apply::apply_best(text, &items);
    assert_eq!(applied.len(), 1);
    assert!(!updated.contains("\"b\""), "the later duplicate is the one that goes");
    let before = engine_core::apply::score(text);
    let after = engine_core::apply::score(&updated);
    assert!(after < before, "the applied text compiles better");
}

#[test]
fn a_repair_that_does_not_improve_is_dropped() {
    let text = "fact F { claim: \"a\", evidence: user }\n";
    let declared = decls(text);
    let noop = engine_core::fix::Suggestion {
        applicability: "MachineApplicable",
        message: "no-op".to_string(),
        file: String::new(),
        line: declared[0].line,
        col: declared[0].col,
        end_line: declared[0].end_line,
        end_col: declared[0].end_col,
        replacement: String::new(),
    };
    let (updated, applied) = engine_core::apply::apply_best(text, &[noop]);
    assert!(applied.is_empty());
    assert_eq!(updated, text);
}

#[test]
fn the_checks_name_the_problem() {
    assert!(codes("").contains(&"E-EMPTY-DOCUMENT"));
    assert!(codes("fact F1 { claim: \"x\", evidence: user }\nfact F1 { claim: \"y\", evidence: user }")
        .contains(&"E-DUP-ID"));
    assert!(codes("proposition P1 { statement: \"a\", from: [F_MISSING] }")
        .contains(&"E-UNDEFINED"));
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
    let leak = diagnostics.iter().find(|item| item.code == "E-LAYER-LEAK").expect("a leak");
    assert!(leak.message.contains("P_LEAK"), "the declaration that reaches out is named");
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
    let text = engine_core::induce::clause_text(&clause);
    assert_eq!(text, "frame_ms <= 40", "the tightest bound the data supports");
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
            engine_core::patterns::learn(
                &program,
                &engine_core::program::pos(concept),
                &engine_core::program::neg(concept),
            ),
        ));
    }
    let problems = verify_predicates(&program, &learnings.iter().map(|item| (item.concept.clone(), engine_core::induce::Learning {
        concept: item.concept.clone(),
        positives: item.positives.clone(),
        negatives: item.negatives.clone(),
        clause: item.clause.clone(),
        witness: item.witness.clone(),
        reason: item.reason.clone(),
    })).collect(), &structurals);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn sugar_expands_one_line_into_one_declaration() {
    let source = "fact \"the window appears\" by user as F_WINDOW\n\
                  measure \"frame_ms\" min 16ms by tool(\"trace\") as V_FRAME\n\
                  because F_SCROLL rule \"one frame is the budget\" given V_FRAME => P_FAST\n";
    let expanded = expand(source);
    let lines: Vec<&str> = expanded.lines().collect();
    assert_eq!(lines[0], "fact F_WINDOW { claim: \"the window appears\", evidence: user }");
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
    assert!(!codes(&named).iter().any(|code| code.starts_with("E-PARTITION")));

    let used = format!(
        "{PILE_CONTEXT}\npartition E_PILE {{ elements: [F_A, F_B], closure: unknown, closes_with: [Q_X] }}\n\
         proposition P_USE {{ statement: \"s\", from: [E_PILE] }}"
    );
    assert!(codes(&used).contains(&"E-PARTITION-UNKNOWN-USED"));

    let no_exit = format!("{PILE_CONTEXT}\npartition E_PILE {{ elements: [F_A, F_B], closure: unknown }}");
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
    use engine_core::program::TABLE;
    assert_eq!(TABLE.len(), 12, "one row per kind");
    for (kind, obligation, discharge) in TABLE {
        assert!(engine_core::specs::spec(kind).is_some(), "{kind} has no field spec");
        assert!(!obligation.is_empty() && !discharge.is_empty(), "{kind} owes nothing?");
        assert_ne!(
            engine_core::build::action_of(kind),
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
    let row = engine_core::build::obligation(program.get("W1").unwrap(), &program, &Default::default());
    assert_eq!((row.action, row.state), ("justify", "ready"));

    let without = decls(
        "fact F { claim: \"x\", evidence: user }\n\
         proposition P { statement: \"s\", from: [F] }\n\
         warrant W1 { claim: \"r\", from: [F], to: P }\n",
    );
    let program = Program::new(&without);
    let row = engine_core::build::obligation(program.get("W1").unwrap(), &program, &Default::default());
    assert_eq!(row.state, "blocked");
}

#[test]
fn the_self_document_compiles_without_errors() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("SELF.mf");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{} is missing", path.display()));
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
        learnings.iter().any(|item| item.concept == "C_REFUSE_NOW" && item.clause.is_some()),
        "the self document's concept has no learnable clause"
    );
}

#[test]
fn a_syntax_error_is_a_diagnostic_with_a_span() {
    let error = parse("fact F { claim: \"unterminated\nevidence: user }").expect_err("must not parse");
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
    let (decls, errors) = engine_core::syntax::parse_recovering(source);
    assert_eq!(errors.len(), 3, "unterminated string, the field that follows it, the missing comma: {errors:?}");
    let names: Vec<&str> = decls.iter().map(|decl| decl.name.as_str()).collect();
    assert!(names.contains(&"F_OK"), "a good declaration before the error survives");
    assert!(names.contains(&"F_AFTER"), "parsing resumes at the next line-initial keyword");
    assert!(!names.contains(&"C_BAD"), "the broken declaration is dropped, not half-read");
}

#[test]
fn a_missing_separator_is_an_error() {
    let error = parse("fact F { claim: \"x\" evidence: user }").expect_err("no comma");
    assert!(error.message.contains(','), "the message names the separator: {}", error.message);
}

#[test]
fn a_session_owns_order_counts_and_shape() {
    let mut session = engine_core::session::Session::new("doc.mf");
    session.emit(engine_core::check::Diagnostic {
        code: "W-HOLE",
        message: "later".to_string(),
        line: 9,
        col: 1,
        end_line: 9,
        end_col: 2,
        file: String::new(),
    });
    session.emit(engine_core::check::Diagnostic {
        code: "E-TYPE",
        message: "earlier".to_string(),
        line: 2,
        col: 5,
        end_line: 2,
        end_col: 9,
        file: String::new(),
    });
    assert_eq!((session.errors(), session.warnings(), session.exit_code()), (1, 1, 1));
    assert_eq!(session.summary(), "1 error(s), 1 warning(s)");
    let json = session.json();
    assert!(json.contains("\"code\": \"E-TYPE\"") && json.contains("\"code\": \"W-HOLE\""));
    assert!(json.contains("\"errors\": 1") && json.contains("\"warnings\": 1"));
    assert!(json.find("E-TYPE").unwrap() < json.find("W-HOLE").unwrap(), "sorted by position");
    // with the source, the text form is the rendered form
    let rendered = engine_core::session::Session::new("doc.mf")
        .with_source("x\nfact F { claim: \"y\" }\n")
        .text();
    assert!(rendered.is_empty(), "an empty session prints nothing");
}

#[test]
fn every_code_the_compiler_prints_has_an_explanation() {
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("golden");
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
    assert!(codes.len() > 5, "the corpus should exercise more than a handful of codes");
    let missing: Vec<&String> =
        codes.iter().filter(|code| engine_core::explain::explain(code).is_none()).collect();
    assert!(missing.is_empty(), "no explanation for {missing:?}");
    // and the table itself must answer for every code it lists
    for code in engine_core::explain::CODES {
        assert!(engine_core::explain::explain(code).is_some(), "{code} is listed but unexplained");
    }
}

#[test]
fn a_declaration_keeps_its_positions() {
    let declared = decls("fact F1 { claim: \"x\", evidence: user }\n");
    assert_eq!(declared[0].line, 1);
    assert_eq!(declared[0].col, 1);
    assert_eq!(declared[0].field_span[0], (1, 18, 1, 21), "the claim value runs from col 18 to 21");
    let reference = decls("proposition P1 { statement: \"a\", from: [F_MISSING] }");
    assert_eq!(reference[0].ref_pos[0].0, "F_MISSING");
    assert_eq!(reference[0].ref_pos[0].2, 41, "the reference is where the name was written");
}

#[test]
fn the_content_hash_is_a_function_of_the_bytes() {
    assert_eq!(
        engine_core::hash::hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_ne!(engine_core::hash::hex(b"abc"), engine_core::hash::hex(b"abd"));
}

#[test]
fn the_store_reader_reads_the_records_it_wrote() {
    let line = r#"{"doc": "a.mf", "codes": ["E-UNDEFINED", "W-HOLE"], "action": "mechanical", "before": {"errors": 2, "cost": 1}, "after": {"errors": 1, "cost": 1}, "checker": "engine-core-0.1"}"#;
    let entry = engine_core::json::parse(line).expect("parse");
    assert_eq!(entry.get("codes").map(|value| value.strings()).unwrap_or_default(), vec!["E-UNDEFINED", "W-HOLE"]);
    assert!(engine_core::cases::improved(&entry));
    assert_eq!(
        engine_core::json::serialize(&entry),
        r#"{"doc": "a.mf", "codes": ["E-UNDEFINED", "W-HOLE"], "action": "mechanical", "before": {"errors": 2, "cost": 1}, "after": {"errors": 1, "cost": 1}, "checker": "engine-core-0.1"}"#
    );
    assert_eq!(engine_core::cases::list_repr(&["A".to_string(), "B".to_string()]), "['A', 'B']");
}

#[test]
fn names_resolve_once_per_program() {
    let source = "fact F1 { claim: \"x\", evidence: user }\nproposition P1 { statement: \"a\", from: [F1] }\n";
    let declared = decls(source);
    let program = Program::new(&declared);
    let names: HashSet<&str> = program.of_kind("fact").iter().map(|decl| decl.name.as_str()).collect();
    assert_eq!(names, HashSet::from(["F1"]));
    assert!(program.get("P1").is_some());
    assert!(program.get("P2").is_none());
}

