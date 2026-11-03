use super::*;

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
fn a_reading_is_supported_by_what_it_says_it_read() {
    // The frame capture: the claim names the user's sentence, but the only rule that concludes
    // it draws on the previous round's frame.  The support is real, and it is borrowed.
    let borrowed = r#"
fact F_SAID { claim: "x", evidence: user }
fact F_EARLIER { claim: "y", evidence: user }
proposition P_READ { statement: "s", from: [F_SAID] }
warrant W_FRAME { claim: "r", from: [F_EARLIER], to: P_READ, basis: [F_EARLIER] }
"#;
    assert!(codes(borrowed).contains(&"W-UNFAITHFUL"));

    // One premise in common anchors the rule, even when it also draws on the frame.
    let anchored = r#"
fact F_SAID { claim: "x", evidence: user }
fact F_EARLIER { claim: "y", evidence: user }
proposition P_READ { statement: "s", from: [F_SAID] }
warrant W_READS { claim: "r", from: [F_SAID, F_EARLIER], to: P_READ, basis: [F_SAID] }
"#;
    assert!(!codes(anchored).contains(&"W-UNFAITHFUL"));
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

/// The search used to stop at three literals, so a consistent clause that needed four was reported
/// as none.  The literals of any covering clause must hold on every positive, so the search space
/// is exactly that set, and the full set is always checked.
#[test]
fn a_clause_may_need_more_than_three_literals() {
    let source = r#"
concept C_FOUR { name: "four at once", grounded_by: [F_P1, F_P2], against: [F_N1, F_N2, F_N3, F_N4] }
fact F_P1 { claim: "p1", evidence: user, attrs: { a: 1, b: 1, c: 1, d: 1, e: 1 } }
fact F_P2 { claim: "p2", evidence: user, attrs: { a: 1, b: 1, c: 1, d: 1, e: 2 } }
fact F_N1 { claim: "n1", evidence: user, attrs: { a: 2, b: 1, c: 1, d: 1, e: 1 } }
fact F_N2 { claim: "n2", evidence: user, attrs: { a: 1, b: 2, c: 1, d: 1, e: 1 } }
fact F_N3 { claim: "n3", evidence: user, attrs: { a: 1, b: 1, c: 2, d: 1, e: 1 } }
fact F_N4 { claim: "n4", evidence: user, attrs: { a: 1, b: 1, c: 1, d: 2, e: 2 } }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let concept = program.of_kind("concept")[0];
    let learning = induce_concept(&program, concept);
    let clause = learning.clause.expect("a four-literal clause exists");
    assert_eq!(clause.len(), 4, "{}", tsare::induce::clause_text(&clause));
    assert_eq!(
        tsare::induce::clause_text(&clause),
        "a <= 1 and b <= 1 and c <= 1 and d <= 1"
    );
}

/// When the most specific conjunction still covers a negative, no conjunction does: everything more
/// general covers strictly more.  Saying that is different from saying "the search gave up".
#[test]
fn an_impossible_clause_says_why_it_is_impossible() {
    let source = r#"
concept C_TWIN { name: "twins", grounded_by: [F_A], against: [F_B, F_C] }
fact F_A { claim: "a", evidence: user, attrs: { x: 1 } }
fact F_B { claim: "b", evidence: user, attrs: { x: 2 } }
fact F_C { claim: "c", evidence: user, attrs: { x: 1 } }
"#;
    let declared = decls(source);
    let program = Program::new(&declared);
    let concept = program.of_kind("concept")[0];
    let learning = induce_concept(&program, concept);
    assert!(
        learning.clause.is_none(),
        "a negative with the same attributes cannot be excluded"
    );
    assert!(
        learning.reason.contains("no consistent conjunction"),
        "{}",
        learning.reason
    );
}

/// Two reports of the same document, and an edit that moved a complaint without changing it.  A
/// change keyed by line would call that a change; keyed by name it is not one.  This is the
/// quantity a loop needs to know whether its last edit helped, and no single run can produce it.
#[test]
fn a_delta_keys_on_the_declaration_not_the_line() {
    let before = r#"{"diagnostics": [
        {"code": "W-UNCONFIRMED", "name": "F_A", "line": 3},
        {"code": "E-UNDEFINED", "name": "P_B", "line": 9}]}"#;
    let after = r#"{"diagnostics": [{"code": "W-UNCONFIRMED", "name": "F_A", "line": 7}]}"#;
    let difference = tsare::delta::compare(before, after).expect("two reports");
    assert_eq!((difference.before, difference.after), (2, 1));
    assert!(difference.appeared.is_empty());
    assert_eq!(difference.gone.len(), 1, "one complaint was removed");
    assert_eq!(difference.gone[0].code, "E-UNDEFINED");
    assert_eq!(difference.gone[0].name, "P_B");
    assert!(
        difference
            .gone
            .iter()
            .all(|item| item.code != "W-UNCONFIRMED"),
        "the warning kept its name while its line moved: that is not a change"
    );
    let text = tsare::delta::text(&difference);
    assert!(text.contains("2 -> 1"), "{text}");
    assert!(text.contains("E-UNDEFINED P_B"), "{text}");
}

/// A document nested past the parser's cap is a diagnostic, not a crash: recursive descent uses
/// the call stack as its own stack, and a hostile document used to kill the process at ~2100 deep.
#[test]
fn a_value_nested_past_the_cap_is_refused_with_a_diagnostic() {
    let deep = format!(
        "fact F {{ claim: {}1{} }}\n",
        "{a: ".repeat(200),
        "}".repeat(200)
    );
    let error =
        parse(&deep).expect_err("a value deep enough to overflow the stack must be refused");
    assert_eq!(error.code, "E-SYNTAX");
    assert!(error.message.contains("nested deeper"), "{}", error.message);
    let ordinary = format!(
        "fact F {{ claim: {}1{} }}\n",
        "{a: ".repeat(20),
        "}".repeat(20)
    );
    assert!(
        parse(&ordinary).is_ok(),
        "twenty levels is an ordinary document"
    );
}
