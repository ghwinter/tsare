use super::*;

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

/// A retired kernel law stops licensing.  `supersedes` inside the kernel used to be a no-op here:
/// the layer's law list was built from `kind == "law"` alone, so a target could stand on a
/// principle the kernel had withdrawn and still read as licensed.
#[test]
fn a_retired_kernel_law_stops_grounding() {
    let kernel = decls(concat!(
        "fact F_C { claim: \"checked\", evidence: tool(\"t\") }\n",
        "layer L_X { name: \"x\", exposes: [F_C] }\n",
        "law K_OLD { statement: \"old principle\", holds_in: L_X, checked_by: [F_C] }\n",
        "law K_NEW { statement: \"new principle\", holds_in: L_X, checked_by: [F_C], supersedes: [K_OLD], because: [F_C] }\n",
    ));
    let target = decls("warrant W_X { claim: \"r\", from: [F_C], basis: [K_OLD], layer: L_X }\n");
    let found = tsare::kernel::check(&kernel, &target);
    assert!(
        found
            .iter()
            .any(|item| item.code == "W-UNPRINCIPLED" && item.message.contains("K_NEW")),
        "the only live law in L_X is K_NEW: {found:#?}"
    );

    let licensed = decls("warrant W_X { claim: \"r\", from: [F_C], basis: [K_NEW], layer: L_X }\n");
    assert!(
        !tsare::kernel::check(&kernel, &licensed)
            .iter()
            .any(|item| item.code == "W-UNPRINCIPLED"),
        "standing on the live law is licensed"
    );
}

/// `weigh` propagates a lower bound under the weakest link, not a probability: min over premises,
/// max over rules.  The property that makes it a bound rather than a guess is that a rule cannot
/// gain strength by demanding one more premise, so the arithmetic is not what is tested here.
#[test]
fn an_extra_weak_premise_never_raises_a_weight() {
    let alone = concat!(
        "fact F1 { claim: \"a\", evidence: user, confidence: 0.9 }\n",
        "warrant W1 { claim: \"r\", from: [F1], to: P1, confidence: 0.8 }\n",
        "proposition P1 { statement: \"p\", from: [W1] }\n",
    );
    let burdened = concat!(
        "fact F1 { claim: \"a\", evidence: user, confidence: 0.9 }\n",
        "fact F2 { claim: \"b\", evidence: user, confidence: 0.2 }\n",
        "warrant W1 { claim: \"r\", from: [F1, F2], to: P1, confidence: 0.8 }\n",
        "proposition P1 { statement: \"p\", from: [W1] }\n",
    );
    let weight = |source: &str| -> f64 {
        tsare::weigh::weights(&Program::new(&decls(source)))
            .into_iter()
            .find(|row| row.name == "P1")
            .and_then(|row| row.value)
            .expect("P1 carries a weight")
    };
    let (light, heavy) = (weight(alone), weight(burdened));
    assert!(
        heavy < light,
        "the weak premise should lower the bound: {heavy} vs {light}"
    );
    assert!(heavy <= 0.2, "the bound is the weakest link, got {heavy}");
}
