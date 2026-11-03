use super::*;

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

/// The attribute reading claims to be a closure operator, so the three axioms are checked rather
/// than assumed: extensive (A is inside its closure), monotone (a larger set has a larger
/// closure), idempotent (closing twice is closing once).  An implementation that breaks any of them
/// is not a lattice, whatever the output looks like.
#[test]
fn the_attribute_closure_obeys_the_three_axioms() {
    let source = concat!(
        "fact F_1 { claim: \"1\", evidence: user, attrs: { a: 1, b: 1 } }\n",
        "fact F_2 { claim: \"2\", evidence: user, attrs: { a: 1, c: 1 } }\n",
        "fact F_3 { claim: \"3\", evidence: user, attrs: { a: 1, b: 1, c: 1 } }\n",
        "fact F_4 { claim: \"4\", evidence: user, attrs: { b: 1 } }\n",
        "fact F_5 { claim: \"5\", evidence: user, attrs: { c: 1, d: 1 } }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);

    let mut universe: Vec<(String, String)> = program
        .of_kind("fact")
        .iter()
        .flat_map(|fact| tsare::fca::attributes(fact))
        .collect();
    universe.sort();
    universe.dedup();
    assert!(
        universe.len() <= 12,
        "enumerating every subset must stay cheap"
    );

    let closure = |set: &[(String, String)]| -> Vec<(String, String)> {
        let extent = tsare::fca::extent(&program, set);
        tsare::fca::intent(&program, &extent)
    };
    let subset = |mask: u32| -> Vec<(String, String)> {
        (0..universe.len())
            .filter(|index| mask & (1 << index) != 0)
            .map(|index| universe[index].clone())
            .collect()
    };

    for mask in 0u32..(1u32 << universe.len()) {
        let base = subset(mask);
        let closed = closure(&base);

        for pair in &base {
            assert!(closed.contains(pair), "extensive: {base:?} lacks {pair:?}");
        }
        assert_eq!(closure(&closed), closed, "idempotent: {base:?}");

        for other in 0u32..(1u32 << universe.len()) {
            if mask & !other != 0 {
                continue;
            }
            let wider = closure(&subset(other));
            for pair in &closed {
                assert!(
                    wider.contains(pair),
                    "monotone: {base:?} ⊆ {:?} but {pair:?} is not in its closure",
                    subset(other)
                );
            }
        }
    }
}
