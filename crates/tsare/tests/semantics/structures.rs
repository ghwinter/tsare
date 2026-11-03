use super::*;

#[test]
fn a_document_declares_a_format_only_when_it_must() {
    // no header: the first format, which is every document written before the line existed
    assert!(parse("fact F { claim: \"x\", evidence: user }").is_ok());

    // the header is read and consumed, and it does not shift a single line number
    let declared = parse("tsare 1\nfact F { claim: \"x\", evidence: user }").expect("parse");
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].line, 2);

    // a newer format stops parsing: one true error beats a page of nonsense
    assert!(parse("tsare 2\nfact F { claim: \"x\", evidence: user }").is_ok());
    let error = parse("tsare 3\nfact F { claim: \"x\", evidence: user }").unwrap_err();
    assert_eq!(error.code, "E-VERSION");
    assert!(error.message.contains("format 3"), "{}", error.message);
    assert_eq!((error.line, error.col), (1, 1));

    // a header with no number is still a header
    let error = parse("tsare\nfact F { claim: \"x\", evidence: user }").unwrap_err();
    assert_eq!(error.code, "E-VERSION");

    // and a trailing comment on the line is allowed
    assert!(parse("tsare 1 # the current format\nfact F { claim: \"x\", evidence: user }").is_ok());
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

/// A shared form is a quotient: declarations grouped by their shape, with the positions that vary
/// written down.  Content does not enter the signature -- two claims with completely different text
/// belong to the same class when the rest of the declaration has the same form.
#[test]
fn declarations_of_the_same_form_fall_into_one_class() {
    let source = concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "fact F_B { claim: \"b\", evidence: model }\n",
        "proposition P_ONE { statement: \"one\", from: [F_A] }\n",
        "proposition P_TWO { statement: \"two\", from: [F_B] }\n",
        "warrant W_ONE { claim: \"r\", from: [F_A], to: P_ONE }\n",
        "warrant W_TWO { claim: \"s\", from: [F_B], to: P_TWO }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);
    let groups = tsare::shape::shapes(&program);
    let rules: Vec<&tsare::shape::ShapeGroup> = groups
        .iter()
        .filter(|group| group.kind == "warrant")
        .collect();
    assert_eq!(rules.len(), 1, "the two rules share one form");
    assert_eq!(rules[0].members.len(), 2);
    let fields: Vec<&str> = rules[0]
        .variables
        .iter()
        .map(|(field, _)| field.as_str())
        .collect();
    assert!(
        fields.contains(&"from") && fields.contains(&"to"),
        "{fields:?}"
    );

    // a form with one instance is a declaration, not a structure
    let single = decls("fact F_ONLY { claim: \"x\", evidence: user }\n");
    assert!(tsare::shape::shapes(&Program::new(&single)).is_empty());
}

/// The quotient refines with depth: at depth 0 two rules are alike when their own fields have the
/// same shape; at depth 1 the shape of what they point at counts too.  The classes only get finer,
/// so the sequence is a filtration of the same set, not a different reading of it.
#[test]
fn a_deeper_reading_splits_classes_that_looked_alike() {
    let source = concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "fact F_B { claim: \"b\", evidence: model }\n",
        "proposition P_ONE { statement: \"one\", from: [F_A] }\n",
        "proposition P_TWO { statement: \"two\", from: [F_B] }\n",
        "warrant W_ONE { claim: \"r\", from: [F_A], to: P_ONE }\n",
        "warrant W_TWO { claim: \"s\", from: [F_B], to: P_TWO }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);

    let shallow = tsare::shape::shapes_at(&program, 0);
    assert!(
        shallow
            .iter()
            .any(|group| group.kind == "warrant" && group.members.len() == 2),
        "at depth 0 the two rules share a form"
    );

    let deep = tsare::shape::shapes_at(&program, 1);
    assert!(
        !deep
            .iter()
            .any(|group| group.kind == "warrant" && group.members.len() == 2),
        "at depth 1 they do not: what they point at differs"
    );
}

/// A fibre is one form seen at more than one layer: the same structure claimed twice at different
/// heights.  It is not an error -- a refinement looks exactly like this -- but it has to be visible,
/// because a form that appears at two layers either implements the lower one or contradicts it.
#[test]
fn a_form_that_appears_at_two_layers_is_a_fibre() {
    let source = concat!(
        "layer L_ONE { name: \"one\", exposes: [F_A] }\n",
        "layer L_TWO { name: \"two\", exposes: [F_A] }\n",
        "fact F_A { claim: \"a\", evidence: user }\n",
        "proposition P_X { statement: \"x\", from: [F_A], layer: L_ONE }\n",
        "proposition P_Y { statement: \"y\", from: [F_A], layer: L_TWO }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);
    let fibres = tsare::shape::fibres(&program, 0);
    assert_eq!(fibres.len(), 1, "one class spans both layers");
    assert_eq!(fibres[0].kind, "proposition");
    assert_eq!(fibres[0].layers, vec!["L_ONE", "L_TWO"]);

    // the same declarations at one layer are not a fibre
    let single = decls(concat!(
        "layer L_ONE { name: \"one\", exposes: [F_A] }\n",
        "fact F_A { claim: \"a\", evidence: user }\n",
        "proposition P_X { statement: \"x\", from: [F_A], layer: L_ONE }\n",
        "proposition P_Y { statement: \"y\", from: [F_A], layer: L_ONE }\n",
    ));
    assert!(tsare::shape::fibres(&Program::new(&single), 0).is_empty());
}

/// The dual side of the closure: `extent ∘ intent` over sets of facts.  The two directions are
/// computed by different code (one scans the program, the other only its sample), so an axiom that
/// holds on one side says nothing about the other -- which is why both are checked.
#[test]
fn the_object_closure_obeys_the_three_axioms() {
    let source = concat!(
        "fact F_1 { claim: \"1\", evidence: user, attrs: { a: 1, b: 1 } }\n",
        "fact F_2 { claim: \"2\", evidence: user, attrs: { a: 1, c: 1 } }\n",
        "fact F_3 { claim: \"3\", evidence: user, attrs: { a: 1, b: 1, c: 1 } }\n",
        "fact F_4 { claim: \"4\", evidence: user, attrs: { b: 1 } }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);
    let facts = program.of_kind("fact");
    assert!(
        facts.len() <= 12,
        "enumerating every subset must stay cheap"
    );

    let closure = |picked: &[usize]| -> Vec<String> {
        let sample: Vec<&tsare::syntax::Decl> = picked.iter().map(|index| facts[*index]).collect();
        let intent = tsare::fca::intent(&program, &sample);
        tsare::fca::extent(&program, &intent)
            .into_iter()
            .map(|fact| fact.name.clone())
            .collect()
    };
    let subset = |mask: u32| -> Vec<usize> {
        (0..facts.len())
            .filter(|index| mask & (1 << index) != 0)
            .collect()
    };

    for mask in 0u32..(1u32 << facts.len()) {
        let base = subset(mask);
        let closed = closure(&base);
        let closed_names: Vec<String> = closed.clone();

        for index in &base {
            let name = facts[*index].name.clone();
            assert!(closed.contains(&name), "extensive: {base:?} lacks {name}");
        }
        let again = closure(
            &(0..facts.len())
                .filter(|index| closed_names.contains(&facts[*index].name))
                .collect::<Vec<_>>(),
        );
        assert_eq!(again, closed, "idempotent: {base:?}");

        for other in 0u32..(1u32 << facts.len()) {
            if mask & !other != 0 {
                continue;
            }
            let wider = closure(&subset(other));
            for name in &closed {
                assert!(
                    wider.contains(name),
                    "monotone: {base:?} ⊆ {:?} but {name} is not in its closure",
                    subset(other)
                );
            }
        }
    }
}

/// The functor table is the list of structural laws this format claims.  Each entry names the
/// diagnostic that reports its failure, and the two must agree: a law whose code never fires is a
/// slogan, and a code with no law is an unexplained check.
#[test]
fn every_functor_names_a_code_that_actually_fires() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/codes");
    let cases = [("E-HIERARCHY", "Ext"), ("E-LAYER-LEAK", "Expose")];
    for (code, functor_name) in cases {
        let functor = tsare::cat::FUNCTORS
            .iter()
            .find(|item| item.name == functor_name)
            .unwrap_or_else(|| panic!("no functor named {functor_name}"));
        assert_eq!(functor.code, code, "{functor_name} must name its own code");
        let source = std::fs::read_to_string(root.join(format!("{code}.tsare")))
            .unwrap_or_else(|error| panic!("read {code}.tsare: {error}"));
        let declared = decls(&source);
        let program = Program::new(&declared);
        let outcome = (functor.check)(&program);
        assert!(
            !outcome.failures.is_empty(),
            "{functor_name} must fail on the document that violates it"
        );
        assert!(
            check(&declared).iter().any(|item| item.code == code),
            "the checker must report {code} on the same document"
        );
    }
    for functor in tsare::cat::FUNCTORS {
        assert!(!functor.law.is_empty() && !functor.maps.is_empty());
    }
}

/// Subsumption is a preorder and `meet` is its greatest lower bound, so the three laws that define a
/// meet are checked: the meet is below both inputs, and anything below both is below the meet.  The
/// point of stating them is that anti-unification can then be used without trusting the
/// implementation's taste.
#[test]
fn the_meet_of_two_forms_is_a_greatest_lower_bound() {
    let source = concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "fact F_B { claim: \"b\", evidence: model }\n",
        "proposition P_ONE { statement: \"one\", from: [F_A] }\n",
        "proposition P_TWO { statement: \"two\", from: [F_A] }\n",
        "warrant W_ONE { claim: \"r\", from: [F_A], to: P_ONE }\n",
        "warrant W_TWO { claim: \"s\", from: [F_A], to: P_TWO, basis: [F_A] }\n",
    );
    let declared = decls(source);
    let program = Program::new(&declared);
    let one = tsare::generalize::pattern_of(&program, "W_ONE").expect("W_ONE");
    let _ = tsare::generalize::pattern_of(&program, "W_TWO").expect("W_TWO");
    let same = tsare::generalize::pattern_of(&program, "F_A").expect("F_A");
    let other = tsare::generalize::pattern_of(&program, "F_B").expect("F_B");

    // reflexivity: a form subsumes itself
    assert!(tsare::generalize::subsumes(&one, &one));

    // The meet generalises over shapes, not over content: two facts with different sentences still
    // share the shape `claim:text`, while `tool("t")` and `model` are different shapes and become
    // the free position.
    let meet = tsare::generalize::meet(&same, &other).expect("same kind has a meet");
    assert!(tsare::generalize::subsumes(&meet, &same));
    assert!(tsare::generalize::subsumes(&meet, &other));
    assert!(
        matches!(meet.fields.get("claim"), Some(tsare::generalize::Slot::Pinned(shape)) if shape == "text"),
        "text is the shared shape, whatever the sentences say: {}",
        tsare::generalize::text(&meet)
    );
    assert!(
        matches!(
            meet.fields.get("evidence"),
            Some(tsare::generalize::Slot::Variable)
        ),
        "the evidence shapes differ, so the position is free: {}",
        tsare::generalize::text(&meet)
    );

    // greatest lower bound: anything below both is below the meet
    for candidate in [&same, &other, &meet] {
        if tsare::generalize::subsumes(candidate, &same)
            && tsare::generalize::subsumes(candidate, &other)
        {
            assert!(
                tsare::generalize::subsumes(candidate, &meet),
                "{} is not below the meet",
                tsare::generalize::text(candidate)
            );
        }
    }

    // different kinds have no meet: the preorder is partial, and the answer says so
    let rule = one.clone();
    assert!(tsare::generalize::meet(&rule, &same).is_none());
}

/// Comparing two documents as structures, not as texts: which forms both use, and which only one of
/// them has.  This is the primitive the "which structure should this be expressed in" question needs,
/// and it is a set difference over signatures, so it cannot be talked into agreeing.
#[test]
fn two_documents_sharing_a_form_and_differing_in_another() {
    let here = decls(concat!(
        "fact F_A { claim: \"a\", evidence: tool(\"t\") }\n",
        "proposition P_X { statement: \"x\", from: [F_A] }\n",
    ));
    let there = decls(concat!(
        "fact F_B { claim: \"b\", evidence: tool(\"t\") }\n",
        "hypothesis H_Y { predicts: \"y\", verdict: open }\n",
    ));
    let difference = tsare::shape::compare(&Program::new(&here), &Program::new(&there), 0);
    // the fact form is shared; the proposition exists only here, the hypothesis only there
    assert!(
        !difference.shared.is_empty(),
        "the two facts have the same form"
    );
    assert!(
        difference.only_here.iter().any(|item| item.contains("P_X")),
        "{:?}",
        difference.only_here
    );
    assert!(
        difference
            .only_there
            .iter()
            .any(|item| item.contains("H_Y")),
        "{:?}",
        difference.only_there
    );
    // the same document compared with itself differs in nothing
    let same = tsare::shape::compare(&Program::new(&here), &Program::new(&here), 0);
    assert!(same.only_here.is_empty() && same.only_there.is_empty());
}

/// Two documents whose facts differ in one field: the exact difference calls the two forms
/// unrelated, which is true of the set and useless to a reader.  The near miss names the pair and
/// the share of positions they have in common, and says it is an approximation, not an
/// equivalence.
#[test]
fn two_forms_that_differ_in_one_field_are_near_misses() {
    let here = decls("fact F_A { claim: \"a\", evidence: user, scope: \"one\" }\n");
    let there =
        decls("fact F_B { claim: \"b\", evidence: user, scope: \"two\", artifact: \"run.log\" }\n");
    let (left, right) = (Program::new(&here), Program::new(&there));
    let difference = tsare::shape::compare(&left, &right, 0);
    assert!(difference.shared.is_empty());
    let misses = tsare::shape::near_misses(&left, &right, 0, 4);
    assert_eq!(misses.len(), 1, "one unshared form on each side");
    assert!((misses[0].score - 0.8).abs() < 1e-9, "{}", misses[0].score);
    assert!(misses[0].here.contains("F_A"));
    assert!(misses[0].there.contains("F_B"));
    // a shared form is not a near miss, and a limit of zero asks for nothing
    let same = tsare::shape::near_misses(&left, &left, 0, 4);
    assert!(same.is_empty(), "every form is shared with itself");
    assert!(tsare::shape::near_misses(&left, &right, 0, 0).is_empty());
    assert!(tsare::shape::near_text(&misses).contains("approximation"));
}

/// One spelling for a document with many: the formatted text must parse back to the same
/// document, and formatting it again must not change anything.
#[test]
fn formatting_a_document_is_stable_and_keeps_it_checkable() {
    let source = r#"fact F_A { claim: "a \"quoted\" line", evidence: user }
proposition P { statement: "p", from: [F_A] }
fact F_B { claim: "nested", evidence: tool("t"), attrs: {size: 3, kind: first} }
"#;
    let decls = parse(source).expect("parse");
    let once = tsare::format::document(&decls);
    let again = parse(&once).expect("formatted output parses");
    assert_eq!(once, tsare::format::document(&again), "fmt is idempotent");
    let before: Vec<&str> = check(&decls).iter().map(|item| item.code).collect();
    let after: Vec<&str> = check(&again).iter().map(|item| item.code).collect();
    assert_eq!(
        before, after,
        "the document means the same after formatting"
    );
    assert!(once.contains(r#"\"quoted\""#), "{once}");
    assert!(once.contains("attrs: {size: 3, kind: first}"), "{once}");
}
